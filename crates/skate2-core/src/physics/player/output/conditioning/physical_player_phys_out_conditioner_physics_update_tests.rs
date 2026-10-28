use crate::physics_state_ids::{
    PHYSICS_CATEGORY_AIR, PHYSICS_CATEGORY_GROUND, PHYSICS_STATE_KNOWN_AIR,
    PHYSICS_STATE_NONSPECIFIC, PHYSICS_STATE_PHYSICS_AIR, PHYSICS_STATE_PHYSICS_GROUND,
};
use super::*;
use crate::{
    attribute_runtime::tests::graph,
    input_cvar_runtime::InputCvarManager,
    physical_player_phys_out::{
        PhysOutComponent, PhysOutOrigin, PhysOutValue,
        construct::{ComponentStatus, ConcretePhysOutError, PhysOutAllocation},
    },
};

fn fixture() -> (Conditioners, PhysicalPlayerPhysOut, InputCvarReferences) {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    let refs = InputCvarReferences::new_static_initialized();
    InputCvarManager::new().try_link(&refs).unwrap();
    let mut owner = Conditioners::before_constructor(&output);
    owner
        .try_construct(&mut output, &Arc::new(graph(0, vec![])), &refs, &mut |_| {
            Ok::<_, ()>(true)
        })
        .unwrap();
    (owner, output, refs)
}
fn physics(owner: &mut Conditioners) -> &mut Physics {
    &mut owner.physics.published_value_mut().unwrap().payload
}
fn set_state(output: &mut PhysicalPlayerPhysOut, state: i32, category: i32) {
    let value = output.concrete.state.as_mut().unwrap();
    value.physics_state = state;
    value.physics_state_category = category;
}
fn numeric() -> Estimate {
    Estimate {
        calls: Vec::new(),
        fail: None,
    }
}
struct Estimate {
    calls: Vec<u32>,
    fail: Option<usize>,
}
impl ReckoningNumeric for Estimate {
    type Error = &'static str;
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.calls.push(value.to_bits());
        if self.fail == Some(self.calls.len()) {
            Err("selected estimate")
        } else {
            Ok(1.0 / value)
        }
    }
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        panic!("no dot instruction")
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("no rsqrt instruction")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no conversion instruction")
    }
}
fn filter(
    owner: &mut Conditioners,
    output: &mut PhysicalPlayerPhysOut,
    refs: &InputCvarReferences,
) -> Result<(), PhysicsConditionerUpdateError<()>> {
    physics(owner).update_filtered_state(output, refs)
}

#[test]
fn same_owner_first_three_calls_publish_full_lanes_and_preserve_identity() {
    let (mut owner, mut output, refs) = fixture();
    set_state(&mut output, PHYSICS_STATE_PHYSICS_GROUND, PHYSICS_CATEGORY_GROUND);
    output.system_vectors[PhysOutSystemVector::CentreOfMassVelocity as usize] =
        Some(PhysOutValue {
            value: SkeletonRootVector::known([4.0, 8.0, 12.0, 16.0]),
            origin: PhysOutOrigin::Publication,
        });
    output
        .concrete
        .skateboard_reckoning
        .as_mut()
        .unwrap()
        .deck_position = SkeletonRootVector::known([1.0, 2.0, 3.0, 7.0]);
    let mut numeric = numeric();
    let continuation = owner
        .try_update_physics_prefix(&mut output, &refs, &mut numeric)
        .unwrap();
    assert!(continuation.matches_output(&output));
    assert!(numeric.calls.is_empty());
    assert_eq!(
        output
            .concrete
            .system
            .as_ref()
            .unwrap()
            .damped_centre_of_mass_velocity,
        SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0])
    );
    assert_eq!(
        output
            .concrete
            .motion
            .as_ref()
            .unwrap()
            .emperical_deck_velocity,
        ZERO
    );
    assert_eq!(
        output.concrete.state.as_ref().unwrap().frames_since_reset,
        1
    );
    let filtered = output.try_filtered_output().unwrap();
    assert_eq!(filtered.filtered_physics_state, State::Ground);
    assert_eq!(filtered.last_filtered_physics_state, State::Invalid);
    assert_eq!(filtered.last_grind_dist.to_bits(), 0.0_f32.to_bits());
    assert_eq!(filtered.grind_pathed_guid, u64::MAX);
    assert_eq!(
        physics(&mut owner).deck_position.w.unwrap().to_bits(),
        7.0_f32.to_bits()
    );
    let (_, mut foreign, _) = fixture();
    assert!(matches!(
        owner.try_update_physics_prefix(&mut foreign, &refs, &mut numeric),
        Err(PhysicsConditionerUpdateError::ForeignOutput)
    ));
}

#[test]
fn empirical_velocity_acceleration_use_distinct_estimates_and_all_four_lanes() {
    let (mut owner, mut output, _) = fixture();
    output.concrete.physics.as_mut().unwrap().timestep = 0.5;
    let p = physics(&mut owner);
    p.frames_since_reset = 2;
    p.deck_position = SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0]);
    p.velocity = SkeletonRootVector::known([1.0, 1.0, 1.0, 1.0]);
    output
        .concrete
        .skateboard_reckoning
        .as_mut()
        .unwrap()
        .deck_position = SkeletonRootVector::known([2.0, 4.0, 6.0, 8.0]);
    let mut n = numeric();
    p.update_empirical_measurements(&mut output, &mut n)
        .unwrap();
    assert_eq!(n.calls, [0.5_f32.to_bits(); 2]);
    assert_eq!(
        output
            .concrete
            .motion
            .as_ref()
            .unwrap()
            .emperical_deck_velocity,
        SkeletonRootVector::known([2.0, 4.0, 6.0, 8.0])
    );
    assert_eq!(
        output
            .concrete
            .motion
            .as_ref()
            .unwrap()
            .emperical_deck_acceleration,
        SkeletonRootVector::known([2.0, 6.0, 10.0, 14.0])
    );
    assert_eq!(
        p.last_deck_position,
        SkeletonRootVector::known([1.0, 2.0, 3.0, 4.0])
    );
    assert_eq!(residual(2.0, 0.5).to_bits(), (-0.0_f32).to_bits());
    assert_eq!((-0.5_f32).mul_add(2.0, 1.0).to_bits(), 0.0_f32.to_bits());
}

#[test]
fn second_estimate_failure_keeps_previous_history_but_prior_system_write_survives() {
    let (mut owner, mut output, refs) = fixture();
    physics(&mut owner).frames_since_reset = 2;
    output.system_vectors[PhysOutSystemVector::CentreOfMassVelocity as usize] =
        Some(PhysOutValue {
            value: SkeletonRootVector::known([8.0; 4]),
            origin: PhysOutOrigin::Publication,
        });
    let before = physics(&mut owner).clone();
    let mut n = Estimate {
        fail: Some(2),
        ..numeric()
    };
    assert!(matches!(
        owner.try_update_physics_prefix(&mut output, &refs, &mut n),
        Err(PhysicsConditionerUpdateError::Numeric("selected estimate"))
    ));
    let after = physics(&mut owner);
    assert_eq!(after.frames_since_reset, before.frames_since_reset);
    assert_eq!(after.velocity, before.velocity);
    assert_eq!(after.deck_position, before.deck_position);
    assert_eq!(
        after.damped_system_velocity,
        SkeletonRootVector::known([2.0; 4])
    );
    assert_eq!(
        output
            .concrete
            .system
            .as_ref()
            .unwrap()
            .damped_centre_of_mass_velocity,
        after.damped_system_velocity
    );
}

#[test]
fn late_component_failure_retains_empirical_histories_and_does_not_run_filter() {
    let (mut owner, mut output, refs) = fixture();
    output.concrete.status[PhysOutComponent::Motion as usize] = ComponentStatus::AllocationNull;
    let mut n = numeric();
    assert!(matches!(
        owner.try_update_physics_prefix(&mut output, &refs, &mut n),
        Err(PhysicsConditionerUpdateError::Read(
            PriorInputReadError::Component(ConcretePhysOutError::MissingComponent(
                PhysOutComponent::Motion
            ))
        ))
    ));
    assert_eq!(physics(&mut owner).frames_since_reset, 1);
    assert_eq!(
        output.concrete.state.as_ref().unwrap().frames_since_reset,
        0
    );
    assert_eq!(physics(&mut owner).filtered_state, State::Invalid);
}

#[test]
fn air_hysteresis_reads_live_stairs_cvar_only_after_four_air_frames() {
    let (mut owner, mut output, _) = fixture();
    let unlinked = InputCvarReferences::new_static_initialized();
    set_state(&mut output, PHYSICS_STATE_PHYSICS_AIR, PHYSICS_CATEGORY_AIR);
    physics(&mut owner).filtered_state = State::Ground;
    for _ in 0..3 {
        filter(&mut owner, &mut output, &unlinked).unwrap();
    }
    assert!(matches!(
        filter(&mut owner, &mut output, &unlinked),
        Err(PhysicsConditionerUpdateError::Cvar(
            InputCvarError::Unlinked("kGroundStairsFrames")
        ))
    ));
    assert_eq!(physics(&mut owner).air_count, 4);
    assert_eq!(physics(&mut owner).frames_since_ground_stairs, 4);
    assert_eq!(physics(&mut owner).filtered_state, State::Ground);
    let mut manager = InputCvarManager::new();
    manager.try_link(&unlinked).unwrap();
    assert_eq!(unlinked.try_ground_stairs_frames().unwrap(), 9);
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkGroundStairsFrames=4\r\n"))
        .unwrap();
    filter(&mut owner, &mut output, &unlinked).unwrap();
    assert_eq!(physics(&mut owner).filtered_state, State::Air);
}

#[test]
fn known_air_targeting_is_lazy_and_unknown_constructor_state_is_not_ground() {
    let (mut owner, mut output, refs) = fixture();
    assert!(matches!(
        filter(&mut owner, &mut output, &refs),
        Err(PhysicsConditionerUpdateError::InvalidPhysicsState(0))
    ));
    assert_eq!(physics(&mut owner).frames_since_ground_stairs, 1);
    set_state(&mut output, PHYSICS_STATE_KNOWN_AIR, PHYSICS_CATEGORY_AIR);
    output.concrete.status[PhysOutComponent::Air as usize] = ComponentStatus::AllocationNull;
    filter(&mut owner, &mut output, &refs).unwrap(); // no contact, no Air read
    assert_eq!(physics(&mut owner).filtered_state, State::Air);
    output
        .concrete
        .collision
        .as_mut()
        .unwrap()
        .anything_in_contact = true;
    assert!(matches!(
        filter(&mut owner, &mut output, &refs),
        Err(PhysicsConditionerUpdateError::Read(_))
    ));
    output.concrete.status[PhysOutComponent::Air as usize] = ComponentStatus::Constructed;
    output.concrete.air.as_mut().unwrap().is_targeting_grind = false;
    filter(&mut owner, &mut output, &refs).unwrap();
    assert_eq!(physics(&mut owner).filtered_state, State::Ground);
    output.concrete.air.as_mut().unwrap().is_targeting_grind = true;
    filter(&mut owner, &mut output, &refs).unwrap();
    assert_eq!(physics(&mut owner).filtered_state, State::Air);
}

#[test]
fn counter_wrap_and_early_latch_failure_preserve_native_prefix() {
    let (mut owner, mut output, refs) = fixture();
    physics(&mut owner).filtered_state = State::Ground;
    output.lock_after_update1().unwrap();
    assert!(matches!(
        filter(&mut owner, &mut output, &refs),
        Err(PhysicsConditionerUpdateError::Read(_))
    ));
    assert_eq!(physics(&mut owner).last_filtered_state, State::Ground);
    let (mut owner, mut output, refs) = fixture();
    set_state(&mut output, PHYSICS_STATE_NONSPECIFIC, PHYSICS_CATEGORY_AIR);
    let p = physics(&mut owner);
    p.air_count = i32::MAX;
    p.nonspecific_count = i32::MAX;
    p.nonspecific_collision_free_count = i32::MAX;
    p.frames_since_ground_stairs = i32::MAX;
    filter(&mut owner, &mut output, &refs).unwrap();
    let p = physics(&mut owner);
    assert_eq!(
        [
            p.air_count,
            p.nonspecific_count,
            p.nonspecific_collision_free_count,
            p.frames_since_ground_stairs
        ],
        [i32::MIN; 4]
    );
}

#[test]
fn zero_time_is_not_clamped_and_unknown_w_is_not_fabricated() {
    let (mut owner, mut output, _) = fixture();
    physics(&mut owner).frames_since_reset = 1;
    output.concrete.physics.as_mut().unwrap().timestep = 0.0;
    output
        .concrete
        .skateboard_reckoning
        .as_mut()
        .unwrap()
        .deck_position = SkeletonRootVector {
        xyz: [1.0; 3],
        w: None,
    };
    let mut n = numeric();
    physics(&mut owner)
        .update_empirical_measurements(&mut output, &mut n)
        .unwrap();
    let v = output
        .concrete
        .motion
        .as_ref()
        .unwrap()
        .emperical_deck_velocity;
    assert!(v.xyz.into_iter().all(f32::is_nan));
    assert_eq!(v.w, None);
    assert_eq!(n.calls, [0]);
}
