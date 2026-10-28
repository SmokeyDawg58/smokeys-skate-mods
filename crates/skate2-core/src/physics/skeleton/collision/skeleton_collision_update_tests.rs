//! Controlled empty-contact calls of the actual Update implementation. These
//! state/history fixtures are kernel inputs, not admitted bodies or a live world.
//! No `ContactInfo`, Spy frame, observation or component identity is fabricated.
use super::*;
use crate::{
    skeleton_collision_attributes::{SkeletonCollisionAttributeRead, SkeletonCollisionOffset},
    skeleton_physics_history::{HostSkeletonPhysicsHistoryNumeric, SkeletonHistoryInput},
};
use std::{cell::RefCell, collections::VecDeque, convert::Infallible, rc::Rc};

struct Constructor;
impl SkeletonCollisionAttributeRead for Constructor {
    type Pointer = ();
    type Error = Infallible;
    fn cached_offset(&mut self, _: SkeletonCollisionOffset) -> Result<[f32; 4], Infallible> {
        Ok([1.0, 2.0, 3.0, 99.0])
    }
    fn lookup(&mut self, _: u64) -> Result<(), Infallible> {
        Ok(())
    }
    fn float(&mut self, (): &()) -> Result<f32, Infallible> {
        Ok(1.0)
    }
}

struct HistoryInput;
impl SkeletonHistoryInput for HistoryInput {
    type Error = Infallible;
    type Up = ();
    fn acquire_up_vector(&mut self) -> Result<(), Infallible> {
        Ok(())
    }
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Infallible> {
        Ok(1)
    }
    fn read_up_vector(&mut self, (): &()) -> Result<[f32; 4], Infallible> {
        panic!("XYZ COM does not load Up")
    }
}

fn history(matrix: [f32; 16]) -> SkeletonPhysicsHistory {
    let mut history = SkeletonPhysicsHistory::new([1.0; 24]);
    history.try_initialize_masses().unwrap();
    history
        .try_update(
            matrix,
            &(),
            &mut |_| Ok::<_, Infallible>(matrix),
            &mut HistoryInput,
            &mut HostSkeletonPhysicsHistoryNumeric,
        )
        .unwrap();
    history
}

fn input() -> SkeletonCollisionUpdateInput {
    SkeletonCollisionUpdateInput {
        ignore_rigid_body: None,
        ground_position: [0.0; 3],
        ground_normal: [0.0, 1.0, 0.0],
        system_velocity: [0.0; 3],
        centre_of_mass_velocity: [0.0; 3],
        timestep: 0.25,
        danger_zone: 0,
        ai_skater: 0,
        offboard: 0,
        offboard_air: 0,
        ground_trick: 0,
        request_partial_ragdoll: 0,
    }
}

fn seeded_data() -> SkeletonCollisionData {
    let mut data = SkeletonCollisionData::try_new(&mut Constructor).unwrap();
    data.collision_timer = 2.0;
    data.time_in_wipeout = 3.0;
    data.is_wiping_out = 0x81;
    data.compliant_just_collided = 0x82;
    data.non_compliant_just_collided = 0x83;
    data.num_normals = 17;
    data.hit_vehicle = 0x84;
    data.hit_world = 0x85;
    data.time_since_collision = [0.125; 24];
    data.just_collided = [0x86; 24];
    data.drive_strength = 0.75;
    data.compliant_active = 0x87;
    data.is_hook_to_hips_valid = 0x88;
    data.specific_parts.world_positions = [[9.0; 4]; 2];
    data
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Dot,
    Root,
    Lookup(Attribute),
    Float(Attribute),
    DrawBody,
    DebugAlways,
    Overlay,
    CaptureFence(FenceThreshold),
    ReadFence(FenceThreshold),
}

#[derive(Clone)]
struct Services {
    events: Rc<RefCell<Vec<Event>>>,
    fail_at: Option<usize>,
    duration: f32,
    draw: VecDeque<u8>,
    debug_always: u8,
}
impl Services {
    fn new() -> Self {
        Self {
            events: Rc::default(),
            fail_at: None,
            duration: 4.0,
            draw: VecDeque::new(),
            debug_always: 0,
        }
    }
    fn event(&mut self, event: Event) -> Result<(), usize> {
        let mut events = self.events.borrow_mut();
        let index = events.len();
        events.push(event);
        if self.fail_at == Some(index) {
            Err(index)
        } else {
            Ok(())
        }
    }
}
impl SkeletonCollisionNumeric for Services {
    type Error = usize;
    fn dot3(&mut self, a: [f32; 3], b: [f32; 3]) -> Result<f32, usize> {
        self.event(Event::Dot)?;
        assert_eq!(a.map(f32::to_bits), [0; 3]);
        assert_eq!(b.map(f32::to_bits), [0; 3]);
        Ok(0.0)
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, usize> {
        self.event(Event::Root)?;
        assert_eq!(value.to_bits(), 0);
        Ok(f32::NAN) // Source zero selection must still finish after refinements.
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, usize> {
        panic!("no contact clamp")
    }
}
impl SkeletonCollisionUpdateAttributeRead for Services {
    type Pointer = Attribute;
    type Error = usize;
    fn lookup(&mut self, route: Attribute) -> Result<Attribute, usize> {
        self.event(Event::Lookup(route))?;
        assert_eq!(route, Attribute::TimerDuration);
        Ok(route)
    }
    fn float(&mut self, pointer: &Attribute) -> Result<f32, usize> {
        self.event(Event::Float(*pointer))?;
        Ok(self.duration)
    }
}
impl SkeletonCollisionCvars for Services {
    type Error = usize;
    type FenceTarget = FenceThreshold;
    fn capture_fence_threshold(&mut self, field: FenceThreshold) -> Result<FenceThreshold, usize> {
        self.event(Event::CaptureFence(field))?;
        Ok(field)
    }
    fn read_fence_threshold(&mut self, field: &FenceThreshold) -> Result<f32, usize> {
        self.event(Event::ReadFence(*field))?;
        Ok(match field {
            FenceThreshold::Distance => f32::from_bits(0x3cf5_c28f),
            FenceThreshold::OpposingDot => f32::from_bits(0xbf7d_70a4),
        })
    }

    fn draw_body_part_collision(&mut self) -> Result<u8, usize> {
        self.event(Event::DrawBody)?;
        Ok(self.draw.pop_front().unwrap_or(0))
    }
    fn debug_always_collide(&mut self) -> Result<u8, usize> {
        self.event(Event::DebugAlways)?;
        Ok(self.debug_always)
    }
    fn require_debug_overlay(&mut self) -> Result<(), usize> {
        self.event(Event::Overlay)
    }
    fn draw_collision_update(&mut self) -> Result<u8, usize> {
        panic!("no ground contact")
    }
    fn show_ground_plane(&mut self) -> Result<u8, usize> {
        panic!("no ground predicate")
    }
    fn allow_any_contact(&mut self) -> Result<u8, usize> {
        panic!("no region contact")
    }
    fn maximum_ground_distance_offboard(&mut self) -> Result<f32, usize> {
        panic!("no ground predicate")
    }
    fn small_object_force_scalar(&mut self) -> Result<f32, usize> {
        panic!("no contact force")
    }
}

struct NoLiveReads;
impl SkeletonCollisionLiveRead for NoLiveReads {
    type Error = Infallible;
    fn other_body_group(&mut self, _: &SimulationIslandBodyHandle) -> Result<u32, Infallible> {
        panic!("empty contacts")
    }
    fn raw_state(&mut self, _: SimulationRigidBodyObservation) -> Result<u32, Infallible> {
        panic!("empty contacts")
    }
    fn cached_inverse_mass(
        &mut self,
        _: SimulationRigidBodyObservation,
    ) -> Result<f32, Infallible> {
        panic!("empty contacts")
    }
    fn velocity(&mut self, _: SimulationRigidBodyObservation) -> Result<[f32; 3], Infallible> {
        panic!("empty contacts")
    }
    fn part_transform(&mut self, _: usize) -> Result<[f32; 16], Infallible> {
        panic!("cleared contact groups")
    }
}

fn update(
    data: &mut SkeletonCollisionData,
    info: &SkeletonCollisionUpdateInput,
    history: &SkeletonPhysicsHistory,
    services: &Services,
) -> Result<(), SkeletonCollisionUpdateError<usize, usize, usize, Infallible>> {
    data.try_update_normal(
        false,
        info,
        &[],
        history,
        &mut services.clone(),
        &mut services.clone(),
        &mut services.clone(),
        &mut NoLiveReads,
    )
}

fn expected_events() -> Vec<Event> {
    vec![
        Event::Dot,
        Event::Dot,
        Event::Root,
        Event::Lookup(Attribute::TimerDuration),
        Event::Float(Attribute::TimerDuration),
        Event::DrawBody,
        Event::DrawBody,
        Event::DrawBody,
        Event::DebugAlways,
    ]
}

#[test]
fn empty_update_preserves_selective_fields_and_clears_actual_response_state() {
    let mut data = seeded_data();
    data.priority = [7.0; 24];
    data.is_compliant = [0x80; 24];
    data.normal_info[3].normal = SkeletonCollisionVector::xyz_only([1.0, 2.0, 3.0]);
    data.audio_surface_type = [123; 8];
    data.contact_force = [8.0; 8];
    data.contact_bone = [Some(SkeletonPartIndex::Hips); 8];
    data.bone_data[4].contact_force = 10.0;
    data.bone_data[4].hit_vehicle = 1;
    let services = Services::new();
    update(&mut data, &input(), &history([0.0; 16]), &services).unwrap();
    assert_eq!(*services.events.borrow(), expected_events());
    assert_eq!(data.priority.map(f32::to_bits), [7.0_f32.to_bits(); 24]);
    assert_eq!(data.is_compliant, [0x80; 24]);
    assert!(data.normal_info[3].normal.known_lanes().is_none());
    assert_eq!(data.audio_surface_type, [123; 8]);
    assert_eq!(
        data.time_since_collision.map(f32::to_bits),
        [(-0.125_f32).to_bits(); 24]
    );
    assert_eq!(data.just_collided, [0; 24]);
    assert_eq!(data.contact_force.map(f32::to_bits), [0; 8]);
    assert_eq!(data.contact_bone, [None; 8]);
    assert_eq!(data.bone_data[4].contact_force.to_bits(), 0);
    assert_eq!(data.bone_data[4].hit_vehicle, 0);
    assert_eq!(
        data.most_vertical_contact_normal
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0x8000_0000, 0xbf80_0000, 0x8000_0000, 0x8000_0000]
    );
    assert_eq!(
        data.most_vertical_toe_contact_normal
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0; 4]
    );
    assert_eq!(data.time_in_wipeout.to_bits(), 0);
    assert_eq!(data.num_normals, 0);
    assert_eq!(data.collision_timer.to_bits(), 1.75_f32.to_bits());
    assert_eq!(data.drive_strength.to_bits(), 0.5625_f32.to_bits());
    assert_eq!(data.compliant_active, 1);
    assert_eq!(data.is_hook_to_hips_valid, 0);
}

#[test]
fn region_transform_is_full_four_lane_x_y_z_fma_not_rounded_products() {
    let mut data = seeded_data();
    let one_plus = f32::from_bits(0x3f80_0001);
    data.specific_parts.offsets = [[one_plus, 1.0, 1.0, f32::NAN]; 2];
    let mut matrix = [0.0; 16];
    matrix[0] = one_plus;
    matrix[12] = -f32::from_bits(0x3f80_0002);
    matrix[5] = 2.0;
    matrix[10] = 3.0;
    matrix[3] = 2.0;
    matrix[7] = 4.0;
    matrix[11] = 8.0;
    matrix[15] = 16.0;
    update(&mut data, &input(), &history(matrix), &Services::new()).unwrap();
    // (1+2^-23)^2 -(1+2^-22) is exactly2^-46 only when fused.
    // W participates in the same three FMAs; offsetW is never sampled.
    let expected = [
        0x2880_0000,
        2.0_f32.to_bits(),
        3.0_f32.to_bits(),
        30.0_f32.to_bits(),
    ];
    assert_eq!(
        data.specific_parts
            .world_positions
            .map(|v| v.map(f32::to_bits)),
        [expected; 2]
    );
    assert_eq!(data.specific_parts.contact, [Some(0); 2]);
    assert_eq!(data.specific_parts.recent_contact, [Some(0); 2]);
}

#[test]
fn numeric_failures_retain_only_source_prefix_before_wipeout_store() {
    for failed in 0..3 {
        let mut data = seeded_data();
        let before = data.clone();
        let mut services = Services::new();
        services.fail_at = Some(failed);
        assert!(
            matches!(update(&mut data, &input(), &history([0.0; 16]), &services), Err(SkeletonCollisionUpdateError::Numeric(index)) if index == failed)
        );
        assert_eq!(*services.events.borrow(), expected_events()[..=failed]);
        if failed < 2 {
            assert_eq!(data, before);
        } else {
            assert_eq!(data.collision_timer.to_bits(), 1.75_f32.to_bits());
            assert_eq!(
                [
                    data.compliant_just_collided,
                    data.non_compliant_just_collided,
                    data.hit_vehicle,
                    data.hit_world
                ],
                [0; 4]
            );
            assert_eq!(data.num_normals, 0);
            assert_eq!(data.time_in_wipeout.to_bits(), 3.0_f32.to_bits());
            assert_eq!(data.is_wiping_out, 0x81);
            assert_eq!(
                data.time_since_collision.map(f32::to_bits),
                before.time_since_collision.map(f32::to_bits)
            );
        }
    }
}

#[test]
fn duration_lookup_and_load_fail_after_speed_but_before_bulk_reset() {
    for failed in [3, 4] {
        let mut data = seeded_data();
        let mut services = Services::new();
        services.fail_at = Some(failed);
        assert!(
            matches!(update(&mut data, &input(), &history([0.0; 16]), &services), Err(SkeletonCollisionUpdateError::Attribute(index)) if index == failed)
        );
        assert_eq!(*services.events.borrow(), expected_events()[..=failed]);
        assert_eq!(data.time_in_wipeout.to_bits(), 3.25_f32.to_bits());
        assert_eq!(data.is_wiping_out, 0);
        assert_eq!(
            data.time_since_collision.map(f32::to_bits),
            [0.125_f32.to_bits(); 24]
        );
        assert_eq!(data.just_collided, [0x86; 24]);
        assert_eq!(data.is_hook_to_hips_valid, 0x88);
    }
}

#[test]
fn debug_region_entry_refusal_and_later_overlay_failure_keep_distinct_prefixes() {
    let mut data = seeded_data();
    let mut services = Services::new();
    services.draw = VecDeque::from([0x80]);
    assert!(matches!(
        update(&mut data, &input(), &history([0.0; 16]), &services),
        Err(SkeletonCollisionUpdateError::DebugRegionReloadUnsupported)
    ));
    assert_eq!(*services.events.borrow(), expected_events()[..6]);
    assert_eq!(
        data.time_since_collision.map(f32::to_bits),
        [(-0.125_f32).to_bits(); 24]
    );
    assert_eq!(
        data.specific_parts
            .world_positions
            .map(|v| v.map(f32::to_bits)),
        [[9.0_f32.to_bits(); 4]; 2]
    );
    assert_eq!(data.specific_parts.contact, [None; 2]);
    let mut data = seeded_data();
    let mut services = Services::new();
    services.draw = VecDeque::from([0, 1]);
    services.fail_at = Some(7);
    assert!(matches!(
        update(&mut data, &input(), &history([0.0; 16]), &services),
        Err(SkeletonCollisionUpdateError::Cvar(7))
    ));
    assert_eq!(services.events.borrow()[7], Event::Overlay);
    assert_eq!(
        data.specific_parts.world_positions[0].map(f32::to_bits),
        [0; 4]
    );
    assert_eq!(
        data.specific_parts.world_positions[1].map(f32::to_bits),
        [9.0_f32.to_bits(); 4]
    );
    assert_eq!(data.specific_parts.contact, [Some(0), None]);
}

#[test]
fn partial_request_and_final_fsel_keep_zero_nan_and_outside_interval_semantics() {
    for (timer, duration, expected) in [
        (-2.0, 4.0, 1.0_f32),
        (8.0, 4.0, 0.0),
        (1.0, 0.0, 1.0),
        (1.0, -0.0, 1.0),
        (f32::NAN, 4.0, 0.0),
        (1.0, f32::NAN, 0.0),
    ] {
        let mut data = seeded_data();
        data.collision_timer = timer;
        let mut services = Services::new();
        services.duration = duration;
        let mut info = input();
        info.timestep = 0.0;
        update(&mut data, &info, &history([0.0; 16]), &services).unwrap();
        assert_eq!(data.drive_strength.to_bits(), expected.to_bits());
        assert_eq!(data.compliant_active, u8::from(expected < 1.0));
    }
    let mut data = seeded_data();
    let mut info = input();
    info.request_partial_ragdoll = 0x80;
    update(&mut data, &info, &history([0.0; 16]), &Services::new()).unwrap();
    assert_eq!(data.collision_timer.to_bits(), 4.0_f32.to_bits());
    assert_eq!(data.drive_strength.to_bits(), 0);
    assert_eq!(data.compliant_just_collided, 0);
    assert_eq!(data.is_hook_to_hips_valid, 0);
}

#[test]
fn final_debug_override_does_not_recompute_hook_validity_and_failure_keeps_strength() {
    let mut data = seeded_data();
    let mut services = Services::new();
    services.debug_always = 0x80;
    let mut info = input();
    info.offboard = 1;
    update(&mut data, &info, &history([0.0; 16]), &services).unwrap();
    assert_eq!(data.drive_strength.to_bits(), 0);
    assert_eq!(data.compliant_just_collided, 1);
    assert_eq!(data.compliant_active, 1);
    assert_eq!(data.is_hook_to_hips_valid, 0);
    let mut data = seeded_data();
    let mut services = Services::new();
    services.fail_at = Some(8);
    assert!(matches!(
        update(&mut data, &input(), &history([0.0; 16]), &services),
        Err(SkeletonCollisionUpdateError::Cvar(8))
    ));
    assert_eq!(data.drive_strength.to_bits(), 0.5625_f32.to_bits());
    assert_eq!(data.compliant_active, 0x87);
    assert_eq!(data.compliant_just_collided, 0);
}

fn ragdoll_update(
    data: &mut SkeletonCollisionData,
    services: &Services,
) -> Result<(), SkeletonCollisionUpdateError<usize, usize, usize, Infallible>> {
    data.try_update_normal(
        true,
        &input(),
        &[],
        &history([0.0; 16]),
        &mut services.clone(),
        &mut services.clone(),
        &mut services.clone(),
        &mut NoLiveReads,
    )
}

fn fence_events() -> Vec<Event> {
    vec![
        Event::CaptureFence(FenceThreshold::Distance),
        Event::CaptureFence(FenceThreshold::OpposingDot),
        Event::ReadFence(FenceThreshold::Distance),
        Event::ReadFence(FenceThreshold::OpposingDot),
    ]
}

#[test]
fn ragdoll_preserves_accumulators_and_recent_unknown_while_running_common_tail() {
    let mut data = seeded_data();
    data.contact_force_total_during_wipeout = 7.0;
    data.slide_force_total_during_wipeout = 9.0;
    data.specific_parts.recent_contact = [Some(0x80), None];
    let services = Services::new();
    ragdoll_update(&mut data, &services).unwrap();
    assert_eq!(data.time_in_wipeout.to_bits(), 3.25f32.to_bits());
    assert_eq!(
        data.contact_force_total_during_wipeout.to_bits(),
        7.0f32.to_bits()
    );
    assert_eq!(
        data.slide_force_total_during_wipeout.to_bits(),
        9.0f32.to_bits()
    );
    assert_eq!(data.specific_parts.recent_contact, [Some(1), None]);
    assert_eq!(data.specific_parts.contact, [Some(0); 2]);
    assert_eq!(data.is_wiping_out, 1);
    assert_eq!(data.opposing_contacts, 0);
    assert_eq!(data.opposing_contacts_on_plane, 0);
    assert_eq!(data.drive_strength.to_bits(), 0.5625f32.to_bits());
    assert_eq!(data.compliant_active, 1);
    let mut expected = expected_events();
    expected.pop();
    expected.extend(fence_events());
    expected.push(Event::DebugAlways);
    assert_eq!(*services.events.borrow(), expected);
}

#[test]
fn fence_target_or_value_failure_retains_update_prefix_before_drive_tail() {
    for failure in 8..12 {
        let mut data = seeded_data();
        let mut services = Services::new();
        services.fail_at = Some(failure);
        assert!(
            matches!(ragdoll_update(&mut data,&services),Err(SkeletonCollisionUpdateError::Cvar(index)) if index==failure)
        );
        assert_eq!(data.time_in_wipeout.to_bits(), 3.25f32.to_bits());
        assert_eq!(data.is_wiping_out, 1);
        assert_eq!(data.just_collided, [0; 24]);
        assert_eq!(data.drive_strength.to_bits(), 0.75f32.to_bits());
        assert_eq!(data.compliant_active, 0x87);
        assert_eq!(services.events.borrow().len(), failure + 1);
    }
}

fn fence(data: &SkeletonCollisionData) -> bool {
    data.check_impaled_on_fence::<Infallible, _, _, Infallible>(
        &mut Services::new(),
        &mut crate::skeleton_collision_numeric::HostSkeletonCollisionNumeric,
    )
    .unwrap()
}

fn opposing_pair(i: usize, j: usize) -> SkeletonCollisionData {
    let mut data = seeded_data();
    data.bone_data[i].contact_force = 1.0;
    data.bone_data[j].contact_force = 1.0;
    data.bone_data[i].contact_normal = SkeletonCollisionVector::xyz_only([1.0, 0.0, 0.0]);
    data.bone_data[j].contact_normal = SkeletonCollisionVector::xyz_only([-1.0, 0.0, 0.0]);
    data.bone_data[i].contact_position = SkeletonCollisionVector::xyz_only([0.01, 99.0, 0.0]);
    data.bone_data[j].contact_position = SkeletonCollisionVector::xyz_only([0.0; 3]);
    data
}

#[test]
fn fence_uses_bones_one_through_twenty_three_and_strict_plane_distance() {
    assert!(fence(&opposing_pair(1, 23)));
    assert!(fence(&opposing_pair(22, 23)));
    assert!(!fence(&opposing_pair(0, 23)));
    let mut data = opposing_pair(1, 23);
    data.bone_data[1].contact_position =
        SkeletonCollisionVector::xyz_only([f32::from_bits(0x3cf5_c28f), 0.0, 0.0]);
    assert!(!fence(&data));
    data.bone_data[1].contact_position = SkeletonCollisionVector::xyz_only([-0.01, 0.0, 0.0]);
    assert!(fence(&data));
}

#[test]
fn fence_skips_nonpositive_and_unordered_forces_and_rejects_unordered_dot() {
    for force in [0.0, -0.0, -1.0, f32::NAN] {
        for index in [1, 23] {
            let mut data = opposing_pair(1, 23);
            data.bone_data[index].contact_force = force;
            assert!(!fence(&data));
        }
    }
    let mut data = opposing_pair(1, 23);
    data.bone_data[1].contact_normal = SkeletonCollisionVector::xyz_only([f32::NAN, 0.0, 0.0]);
    assert!(!fence(&data));
    data = opposing_pair(1, 23);
    data.bone_data[23].contact_position = SkeletonCollisionVector::xyz_only([f32::NAN, 0.0, 0.0]);
    assert!(!fence(&data));
}
