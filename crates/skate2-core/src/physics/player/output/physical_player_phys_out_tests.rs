use super::*;

#[derive(Default)]
struct Remainder {
    stages: Vec<PhysOutRemainderStage>,
    fail: Option<PhysOutRemainderStage>,
}
impl PhysOutRemainder for Remainder {
    type Error = &'static str;
    fn run(&mut self, stage: PhysOutRemainderStage) -> Result<(), Self::Error> {
        self.stages.push(stage);
        if self.fail == Some(stage) {
            Err("stage")
        } else {
            Ok(())
        }
    }
}
fn constructed() -> PhysicalPlayerPhysOut {
    let mut output = PhysicalPlayerPhysOut::new();
    output.try_construct(&mut Remainder::default()).unwrap();
    output
}

#[test]
fn ground_up_copies_read_twice_and_preserve_first_store_on_second_failure() {
    let mut output = constructed();
    let prior = output
        .ground_vector(PhysOutGroundVector::LastValidGroundUp)
        .unwrap();
    let first = SkeletonRootVector {
        xyz: [0.25, 0.75, -0.0],
        w: None,
    };
    let mut calls = 0;
    assert_eq!(
        output.try_publish_last_ground_up(|| {
            calls += 1;
            if calls == 1 { Ok(first) } else { Err("second") }
        }),
        Err(PhysOutPublicationError::Source("second"))
    );
    assert_eq!(calls, 2);
    assert_eq!(
        output
            .ground_vector(PhysOutGroundVector::GroundUp)
            .unwrap()
            .value,
        first
    );
    assert_eq!(
        output
            .ground_vector(PhysOutGroundVector::LastValidGroundUp)
            .unwrap(),
        prior
    );
    output.lock_after_update1().unwrap();
    calls = 0;
    assert_eq!(
        output.try_publish_last_ground_up(|| {
            calls += 1;
            Ok::<_, &str>(first)
        }),
        Err(PhysOutPublicationError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    );
    assert_eq!(calls, 1, "source read precedes sampled latch assertion");
}

#[test]
fn unknown_owner_refuses_reads_and_constructor_failure_retains_prefix() {
    let mut output = PhysicalPlayerPhysOut::new();
    assert_eq!(
        output.collision_physics_type(),
        Err(PhysOutAccessError::NotConstructed)
    );
    let mut remainder = Remainder {
        fail: Some(PhysOutRemainderStage::Allocate(PhysOutComponent::Ground)),
        ..Remainder::default()
    };
    assert_eq!(
        output.try_construct(&mut remainder),
        Err(PhysOutLifecycleError::Remainder("stage"))
    );
    assert_eq!(
        output.motion_vectors[1].unwrap().origin,
        PhysOutOrigin::Constructor
    );
    assert_eq!(output.collision_type.unwrap().value, 0);
    assert!(output.ground_transform.is_none());
    assert_eq!(output.locked_for_write(), None);
    assert!(!output.selected_construction_complete());
    assert_eq!(
        output.try_construct(&mut Remainder::default()),
        Err(PhysOutLifecycleError::AlreadyStarted)
    );
}

#[test]
fn constructor_defaults_are_full_lane_values_without_publication_claim() {
    let output = constructed();
    let velocity = output
        .motion_vector(PhysOutMotionVector::DeckVelocity)
        .unwrap();
    assert_eq!(velocity.value.xyz.map(f32::to_bits), [0; 3]);
    assert_eq!(velocity.value.w.map(f32::to_bits), Some(0));
    assert_eq!(velocity.origin, PhysOutOrigin::Constructor);
    assert_eq!(
        output
            .ground_vector(PhysOutGroundVector::LastValidGroundUp)
            .unwrap()
            .value,
        UP
    );
    assert_eq!(
        output
            .system_vector(PhysOutSystemVector::Heading)
            .unwrap()
            .value,
        FORWARD
    );
    assert!(!output.selected_reset_complete());
}

#[test]
fn collision_constructor_precedes_reset_and_failure_preserves_zero_type() {
    let mut output = PhysicalPlayerPhysOut::new();
    let mut remainder = Remainder {
        fail: Some(PhysOutRemainderStage::CollisionInfoReset),
        ..Remainder::default()
    };
    output.try_construct(&mut remainder).unwrap_err();
    assert_eq!(
        remainder.stages.last(),
        Some(&PhysOutRemainderStage::CollisionInfoReset)
    );
    assert!(
        remainder
            .stages
            .contains(&PhysOutRemainderStage::CollisionInfoConstructor)
    );
    assert_eq!(output.collision_type.unwrap().value, 0);
    assert!(output.ground_transform.is_none());
}

#[test]
fn late_reset_failure_keeps_lock_and_selected_reset_prefix() {
    let mut output = constructed();
    output
        .try_publish_collision_type(|| Ok::<_, ()>(5))
        .unwrap();
    output.lock_after_update1().unwrap();
    let mut remainder = Remainder {
        fail: Some(PhysOutRemainderStage::ResetOmitted(
            PhysOutComponent::OffBoard,
        )),
        ..Remainder::default()
    };
    output.try_reset(&mut remainder).unwrap_err();
    assert_eq!(output.locked_for_write(), Some(true));
    assert!(!output.selected_reset_complete());
    assert_eq!(output.collision_type.unwrap().value, 0);
    assert_eq!(output.collision_type.unwrap().origin, PhysOutOrigin::Reset);
    assert_eq!(
        output.motion_vector(PhysOutMotionVector::DeckVelocity),
        Err(PhysOutAccessError::LockedForWrite)
    );
    assert_eq!(
        remainder.stages.last(),
        Some(&PhysOutRemainderStage::ResetOmitted(
            PhysOutComponent::OffBoard
        ))
    );
}

#[test]
fn reset_order_includes_nested_collision_before_state_and_final_unlock() {
    use PhysOutComponent::{
        Air, Animation, Audio, Camera, Filtered, Grinds, Intents, OffBoard, PathRecording, Physics,
        Scoring2, SkateboardReckoning, Skeleton, State, Trick,
    };
    use PhysOutRemainderStage::{CollisionInfoReset, ResetOmitted};
    let mut output = constructed();
    output.lock_after_update1().unwrap();
    let mut remainder = Remainder::default();
    output.try_reset(&mut remainder).unwrap();
    assert_eq!(
        remainder.stages,
        [
            ResetOmitted(SkateboardReckoning),
            ResetOmitted(Air),
            ResetOmitted(Physics),
            ResetOmitted(Grinds),
            ResetOmitted(Skeleton),
            CollisionInfoReset,
            ResetOmitted(State),
            ResetOmitted(Audio),
            ResetOmitted(Camera),
            ResetOmitted(Trick),
            ResetOmitted(Intents),
            ResetOmitted(Animation),
            ResetOmitted(Scoring2),
            ResetOmitted(Filtered),
            ResetOmitted(PathRecording),
            ResetOmitted(OffBoard)
        ]
    );
    assert_eq!(output.locked_for_write(), Some(false));
    assert!(output.selected_reset_complete());
}

#[test]
fn reset_collision_failure_does_not_touch_later_ground_component() {
    let mut output = constructed();
    let guard = output.sample_write_guard().unwrap();
    let old = SkeletonRootVector {
        xyz: [2.0, 3.0, 4.0],
        w: None,
    };
    output
        .publish_ground_vector(&guard, PhysOutGroundVector::LastValidGroundUp, old)
        .unwrap();
    output.lock_after_update1().unwrap();
    let mut remainder = Remainder {
        fail: Some(PhysOutRemainderStage::CollisionInfoReset),
        ..Remainder::default()
    };
    output.try_reset(&mut remainder).unwrap_err();
    assert_eq!(output.collision_type.unwrap().origin, PhysOutOrigin::Reset);
    assert_eq!(output.ground_vectors[2].unwrap().value, old);
    assert_eq!(
        output.ground_vectors[2].unwrap().origin,
        PhysOutOrigin::Publication
    );
}

#[test]
fn collision_source_callback_runs_after_guard_and_preserves_failed_write() {
    let mut output = constructed();
    output
        .try_publish_collision_type(|| Ok::<_, ()>(4))
        .unwrap();
    assert_eq!(
        output.try_publish_collision_type(|| Err("source")),
        Err(PhysOutPublicationError::Source("source"))
    );
    assert_eq!(output.collision_physics_type().unwrap().value, 4);
    output.lock_after_update1().unwrap();
    assert_eq!(
        output.try_publish_collision_type(|| -> Result<u32, ()> { panic!("source before guard") }),
        Err(PhysOutPublicationError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    );
}

#[test]
fn latch_observation_is_occurrence_bound_and_retains_sampled_condition() {
    let mut first = constructed();
    let second = constructed();
    let guard = first.sample_write_guard().unwrap();
    assert_eq!(
        second.check_write_guard(&guard),
        Err(PhysOutAccessError::ForeignGuard)
    );
    first.lock_after_update1().unwrap();
    // Source arithmetic may intervene between sample and assertion branch.
    assert_eq!(first.check_write_guard(&guard), Ok(()));
    assert_eq!(
        first.check_write_guard(&first.sample_write_guard().unwrap()),
        Err(PhysOutAccessError::LockedForWrite)
    );
}

#[test]
fn publication_preserves_unknown_w_and_scalar_bits() {
    let mut output = constructed();
    let guard = output.sample_write_guard().unwrap();
    let value = SkeletonRootVector {
        xyz: [-0.0, 3.0, 4.0],
        w: None,
    };
    output
        .publish_motion_vector(&guard, PhysOutMotionVector::DeckVelocity, value)
        .unwrap();
    output
        .publish_motion_scalar(&guard, PhysOutMotionScalar::ZSpeed, -0.0)
        .unwrap();
    assert_eq!(
        output
            .motion_vector(PhysOutMotionVector::DeckVelocity)
            .unwrap()
            .value
            .w,
        None
    );
    assert_eq!(
        output
            .motion_scalar(PhysOutMotionScalar::ZSpeed)
            .unwrap()
            .value
            .to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(
        output
            .motion_scalar(PhysOutMotionScalar::ZSpeed)
            .unwrap()
            .origin,
        PhysOutOrigin::Publication
    );
}

#[test]
fn selected_reset_replaces_published_ground_system_and_angle_fields() {
    let mut output = constructed();
    assert_eq!(
        output
            .system_vector(PhysOutSystemVector::Right)
            .unwrap()
            .value,
        RIGHT
    );
    let guard = output.sample_write_guard().unwrap();
    let changed = SkeletonRootVector {
        xyz: [2.0, 3.0, 4.0],
        w: None,
    };
    for field in [
        PhysOutGroundVector::DynamicUp,
        PhysOutGroundVector::GroundUp,
    ] {
        output
            .publish_ground_vector(&guard, field, changed)
            .unwrap();
        assert_eq!(output.ground_vector(field).unwrap().value.w, None);
    }
    output
        .publish_system_vector(&guard, PhysOutSystemVector::Up, changed)
        .unwrap();
    output
        .publish_motion_scalar(&guard, PhysOutMotionScalar::SurfaceAngle, -0.0)
        .unwrap();
    assert_eq!(
        output
            .motion_scalar(PhysOutMotionScalar::SurfaceAngle)
            .unwrap()
            .value
            .to_bits(),
        (-0.0_f32).to_bits()
    );
    output.try_reset(&mut Remainder::default()).unwrap();
    for field in [
        PhysOutGroundVector::DynamicUp,
        PhysOutGroundVector::GroundUp,
    ] {
        let value = output.ground_vector(field).unwrap();
        assert_eq!(value.value, UP);
        assert_eq!(value.origin, PhysOutOrigin::Reset);
    }
    assert_eq!(
        output.system_vector(PhysOutSystemVector::Up).unwrap().value,
        UP
    );
    assert_eq!(
        output
            .motion_scalar(PhysOutMotionScalar::SurfaceAngle)
            .unwrap()
            .value
            .to_bits(),
        0
    );
}
