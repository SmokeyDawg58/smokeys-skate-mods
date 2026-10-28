#![expect(
    clippy::float_cmp,
    reason = "exact finite fixture values; numerical discriminators use bit assertions"
)]
//! Synthetic primitive/query services test source ordering. They are not an
//! alternate production island or a claim of completed World line traversal.
use super::*;
use crate::{reckoning_numeric::HostReckoningNumeric, skeleton_data_tests};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

type Error = SkeletonIkContactError<&'static str, &'static str>;
const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];
#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Flags,
    Bone,
    Effective,
    Draw,
    Overlay,
    Maximum,
    Threshold,
    Penetration,
    Sideways,
    SideDraw,
    Dot,
    Query,
}
#[derive(Clone, Default)]
struct Trace {
    events: Rc<RefCell<Vec<Event>>>,
    fail_at: Option<usize>,
}
impl Trace {
    fn event(&self, event: Event) -> Result<(), &'static str> {
        let index = self.events.borrow().len();
        self.events.borrow_mut().push(event);
        if self.fail_at == Some(index) {
            Err("refused")
        } else {
            Ok(())
        }
    }
}
impl ReckoningNumeric for Trace {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.event(Event::Dot)?;
        Ok(HostReckoningNumeric.dot3(a, b).unwrap())
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("contact has no rsqrt")
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("contact has no reciprocal")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("contact has no integer conversion")
    }
}
struct Processed {
    trace: Trace,
    flags: Rc<Cell<u32>>,
    bone: Option<usize>,
    effective: SkeletonRootMatrix,
}
impl SkeletonIkContactProcessed for Processed {
    fn flags_8c4(&self) -> u32 {
        self.trace.events.borrow_mut().push(Event::Flags);
        self.flags.get()
    }
    fn contact_bone_id(&self) -> Option<usize> {
        self.trace.events.borrow_mut().push(Event::Bone);
        self.bone
    }
    fn effective_transform(&self) -> &SkeletonRootMatrix {
        self.trace.events.borrow_mut().push(Event::Effective);
        &self.effective
    }
}
struct Services {
    trace: Trace,
    draw: bool,
    side_draw: bool,
    maximum: f32,
    threshold: f32,
    penetration: f32,
    sideways: f32,
    result: SkeletonIkContactLineResult,
    queries: Vec<SkeletonIkContactLineQuery>,
    flags: Rc<Cell<u32>>,
    flags_on_maximum: Option<u32>,
}
impl SkeletonIkContactServices for Services {
    type Error = &'static str;
    fn draw_ik_contact_debug(&mut self) -> Result<bool, Self::Error> {
        self.trace.event(Event::Draw)?;
        Ok(self.draw)
    }
    fn max_ik_contact_penetration_offset(&mut self) -> Result<f32, Self::Error> {
        self.trace.event(Event::Penetration)?;
        Ok(self.penetration)
    }
    fn sideways_offset(&mut self) -> Result<f32, Self::Error> {
        self.trace.event(Event::Sideways)?;
        Ok(self.sideways)
    }
    fn sideways_offset_draw(&mut self) -> Result<bool, Self::Error> {
        self.trace.event(Event::SideDraw)?;
        Ok(self.side_draw)
    }
    fn max_ik_contact_offset(&mut self) -> Result<f32, Self::Error> {
        self.trace.event(Event::Maximum)?;
        if let Some(flags) = self.flags_on_maximum {
            self.flags.set(flags);
        }
        Ok(self.maximum)
    }
    fn toe_contact_threshold(&mut self) -> Result<f32, Self::Error> {
        self.trace.event(Event::Threshold)?;
        Ok(self.threshold)
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.trace.event(Event::Overlay)
    }
    fn try_line_query(
        &mut self,
        query: &SkeletonIkContactLineQuery,
    ) -> Result<SkeletonIkContactLineResult, Self::Error> {
        self.trace.event(Event::Query)?;
        self.queries.push(*query);
        Ok(self.result)
    }
}
struct Fixture {
    state: SkeletonIkState,
    targets: SkeletonIkTargets,
    data: SkeletonIkDataIn,
    processed: Processed,
    services: Services,
    trace: Trace,
}
fn vector(values: [f32; 4]) -> SkeletonRootVector {
    SkeletonRootVector::known(values)
}
fn matrix_at(values: [f32; 4]) -> SkeletonRootMatrix {
    let mut matrix = IDENTITY;
    matrix[12..16].copy_from_slice(&values);
    SkeletonRootMatrix::known(matrix)
}
fn set_position(matrix: &mut SkeletonRootMatrix, value: SkeletonRootVector) {
    let mut columns = *matrix.columns();
    columns[3] = value;
    *matrix = SkeletonRootMatrix::from_columns(columns);
}
impl Fixture {
    fn new() -> Self {
        let (mut hierarchy, parameters, pose) = skeleton_data_tests::fixture();
        hierarchy.parents.fill(-1);
        for (a, b) in [(15, 22), (19, 11)] {
            hierarchy.bone_names.swap(a, b);
            hierarchy.packed_bone_names.swap(a, b);
        }
        let data = skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap();
        let mut targets = SkeletonIkTargets::new(Arc::new(data));
        targets.initialize();
        assert_eq!(targets.bone_ids()[..2], [Some(22), Some(11)]);
        let mut state = SkeletonIkState::new(&mut targets);
        state.modes = [1, 1, 2, 3];
        state.contact_states = [0, 3, 1, 2];
        state.contact_positions = [vector([0.0, -0.1, 0.0, 7.0]); 4];
        state.part_to_world = [matrix_at([0.0, 0.0, 0.0, 2.0]); 4];
        state.parent_to_world = [matrix_at([3.0, 4.0, 5.0, 6.0]); 4];
        let identity = SkeletonRootMatrix::known(IDENTITY);
        let data = SkeletonIkDataIn {
            animation_to_world: identity,
            world_to_animation: identity,
            physics_skateboard: identity,
            inverse_skateboard: identity,
            hips: identity,
            skateboard: identity,
            toe_contact_normal: vector([0.0, 1.0, 0.0, 0.0]),
            hips_offset: vector([0.0; 4]),
        };
        let trace = Trace::default();
        let flags = Rc::new(Cell::new(0));
        Self {
            state,
            targets,
            data,
            processed: Processed {
                trace: trace.clone(),
                flags: Rc::clone(&flags),
                bone: Some(22),
                effective: identity,
            },
            services: Services {
                trace: trace.clone(),
                draw: false,
                side_draw: false,
                maximum: 0.3,
                threshold: 0.7,
                penetration: 0.3,
                sideways: 0.28,
                result: SkeletonIkContactLineResult::Miss,
                queries: Vec::new(),
                flags,
                flags_on_maximum: None,
            },
            trace,
        }
    }
    fn run(&mut self) -> Result<(), Error> {
        self.state.try_update_contact(
            SkeletonIkContactInput {
                data_in: &self.data,
                targets: &self.targets,
                processed: &self.processed,
            },
            &mut self.services,
            &mut self.trace.clone(),
        )
    }
    fn line(&mut self, entry: usize) -> Result<(), Error> {
        self.state.try_contact_line_test(
            entry,
            &self.processed,
            &mut self.services,
            &mut self.trace.clone(),
        )
    }
    fn fail(&mut self, index: usize) {
        self.trace.fail_at = Some(index);
        self.services.trace.fail_at = Some(index);
    }
    fn events(&self) -> Vec<Event> {
        self.trace.events.borrow().clone()
    }
    fn valid(&mut self, height: f32) {
        self.state.contact_states[0] = 1;
        self.state.contact_positions[0] = vector([0.0, height, 0.0, 7.0]);
    }
}

#[test]
fn mode_mismatch_skips_bone_and_query_and_preserves_skateboard_flag_and_hands() {
    let mut f = Fixture::new();
    f.state.modes[0] = 0;
    f.state.modes[1] = 2;
    f.state.tell_anim_dont_push_or_brake = true;
    f.state.skateboard_dont_push = true;
    f.state.contact_target_offsets = [0.08; 4];
    f.state.contact_offsets = [0.05; 4];
    let contacts = f.state.contact_positions;
    let hand = f.state.part_to_world[2];
    assert_eq!(f.run(), Ok(()));
    assert_eq!(
        f.events(),
        [
            Event::Flags,
            Event::Draw,
            Event::Draw,
            Event::Draw,
            Event::Draw
        ]
    );
    assert!(!f.state.tell_anim_dont_push_or_brake);
    assert!(f.state.skateboard_dont_push);
    assert_eq!(f.state.contact_states, [0, 0, 1, 2]);
    assert_eq!(f.state.contact_positions, contacts);
    assert_eq!(f.state.contact_target_offsets, [0.0, 0.0, 0.08, 0.08]);
    assert_eq!(
        f.state.contact_offsets[0].to_bits(),
        (0.05_f32 - STEP_MAX).to_bits()
    );
    assert_eq!(f.state.part_to_world[2], hand);
}

#[test]
fn matching_uses_actual_hierarchy_ids_and_second_foot_starts_after_first_suffix() {
    for (bone, query_count) in [(Some(15), 0), (Some(22), 1), (Some(11), 1)] {
        let mut f = Fixture::new();
        f.processed.bone = bone;
        assert_eq!(f.run(), Ok(()));
        assert_eq!(f.services.queries.len(), query_count);
        if bone == Some(11) {
            assert_eq!(
                &f.events()[..5],
                [
                    Event::Flags,
                    Event::Bone,
                    Event::Draw,
                    Event::Draw,
                    Event::Bone
                ]
            );
            assert_eq!(f.state.contact_states[..2], [0, 2]);
            assert!(f.services.queries[0].start.xyz[0] < 0.0);
        }
    }
}

#[test]
fn waiting_clamps_old_target_slews_then_queries_without_consuming_fresh_hit() {
    let mut f = Fixture::new();
    f.state.contact_target_offsets[0] = 0.4;
    let hit = SkeletonRootVector {
        xyz: [8.0, 9.0, 10.0],
        w: None,
    };
    f.services.result = SkeletonIkContactLineResult::Hit { position: hit };
    assert_eq!(f.run(), Ok(()));
    assert_eq!(
        f.state.contact_target_offsets[0].to_bits(),
        TARGET_MAX.to_bits()
    );
    assert_eq!(f.state.contact_offsets[0].to_bits(), STEP_MAX.to_bits());
    assert_eq!(
        f.state.part_to_world[0].columns()[3].xyz[1].to_bits(),
        STEP_MAX.to_bits()
    );
    assert_eq!(f.state.contact_positions[0], hit);
    assert_eq!(f.state.contact_states[0], 1);
    assert_eq!(
        &f.events()[..8],
        [
            Event::Flags,
            Event::Bone,
            Event::Draw,
            Event::Draw,
            Event::Effective,
            Event::Sideways,
            Event::Dot,
            Event::SideDraw
        ]
    );
}

#[test]
fn unnamed_states_refresh_directly_without_offset_application() {
    for state in [3, u32::MAX] {
        let mut f = Fixture::new();
        f.state.contact_states[0] = state;
        f.state.contact_target_offsets[0] = 0.08;
        f.state.contact_offsets[0] = 0.06;
        let old = f.state.part_to_world[0];
        assert_eq!(f.run(), Ok(()));
        assert_eq!(f.state.part_to_world[0], old);
        assert_eq!(f.state.contact_offsets[0].to_bits(), 0.06_f32.to_bits());
        assert_eq!(
            &f.events()[..7],
            [
                Event::Flags,
                Event::Bone,
                Event::Effective,
                Event::Sideways,
                Event::Dot,
                Event::SideDraw,
                Event::Query
            ]
        );
    }
}

#[test]
fn no_contact_sets_both_flags_before_debug_and_acquires_two_branch_singletons() {
    let mut f = Fixture::new();
    f.state.contact_states[0] = 2;
    f.processed.flags.set(0x0200_0000);
    f.services.draw = true;
    f.fail(6); // second branch singleton, after AddIn's own singleton
    assert_eq!(f.run(), Err(Error::Service("refused")));
    assert_eq!(
        f.events(),
        [
            Event::Flags,
            Event::Bone,
            Event::Draw,
            Event::Overlay,
            Event::Draw,
            Event::Overlay,
            Event::Overlay
        ]
    );
    assert!(f.state.tell_anim_dont_push_or_brake);
    assert!(f.state.skateboard_dont_push);
    assert_eq!(f.state.contact_states[0], 2); // refresh has not started
}

#[test]
fn valid_debug_and_maximum_are_lazy_before_any_contact_scalar_store() {
    let mut f = Fixture::new();
    f.valid(0.01);
    f.services.draw = true;
    f.state.contact_target_offsets[0] = 0.08;
    f.state.contact_offsets[0] = 0.04;
    f.fail(4);
    assert_eq!(f.run(), Err(Error::Service("refused")));
    assert_eq!(
        f.events(),
        [
            Event::Flags,
            Event::Bone,
            Event::Draw,
            Event::Overlay,
            Event::Maximum
        ]
    );
    assert_eq!(f.state.contact_offsets[0].to_bits(), 0.04_f32.to_bits());
    assert_eq!(
        f.state.contact_target_offsets[0].to_bits(),
        0.08_f32.to_bits()
    );
    assert_eq!(f.state.contact_states[0], 1);
}

#[test]
fn excessive_height_skips_threshold_for_request_and_tests_strict_toe_normal_otherwise() {
    for (flags, normal, threshold, abort, threshold_read) in [
        (0x4000_0000, 0.0, 0.7, true, false),
        (0, 0.8, 0.7, true, true),
        (0, 0.7, 0.7, false, true),
        (0, f32::NAN, 0.7, false, true),
        (0, 1.0, f32::NAN, false, true),
    ] {
        let mut f = Fixture::new();
        f.valid(1.0);
        f.processed.flags.set(flags);
        f.data.toe_contact_normal.xyz[1] = normal;
        f.services.threshold = threshold;
        assert_eq!(f.run(), Ok(()));
        assert_eq!(f.state.tell_anim_dont_push_or_brake, abort);
        assert_eq!(f.state.skateboard_dont_push, abort);
        assert_eq!(f.events().contains(&Event::Threshold), threshold_read);
        assert_eq!(
            f.state.contact_target_offsets[0].to_bits(),
            0.0_f32.to_bits()
        );
    }
}

#[test]
fn initial_request_and_late_brake_flags_are_distinct_reads() {
    let mut f = Fixture::new();
    f.valid(-0.1);
    f.state.contact_offsets[0] = -0.05;
    f.state.contact_target_offsets[0] = 0.09;
    f.processed.flags.set(0x0200_0000);
    f.services.flags_on_maximum = Some(0x0100_0000);
    assert_eq!(f.run(), Ok(()));
    assert_eq!(
        f.state.contact_target_offsets[0].to_bits(),
        0.0_f32.to_bits()
    );
    assert_eq!(
        f.state.contact_offsets[0].to_bits(),
        (-0.05_f32 + STEP_MAX).to_bits()
    );
    assert!(f.state.tell_anim_dont_push_or_brake);
    assert!(f.state.skateboard_dont_push);
    assert_eq!(f.events().iter().filter(|e| **e == Event::Flags).count(), 2);
}

#[test]
fn unordered_valid_height_uses_fsel_upper_bound_and_still_refreshes() {
    let mut f = Fixture::new();
    f.valid(f32::NAN);
    assert_eq!(f.run(), Ok(()));
    assert_eq!(
        f.state.contact_target_offsets[0].to_bits(),
        TARGET_MAX.to_bits()
    );
    assert_eq!(f.state.contact_offsets[0].to_bits(), STEP_MAX.to_bits());
    assert!(!f.events().contains(&Event::Threshold));
    assert!(!f.events().contains(&Event::Penetration));
    assert_eq!(f.services.queries.len(), 1);
}

#[test]
fn penetration_uses_open_interval_and_keeps_correction_on_late_debug_failure() {
    for (distance, reads, corrected) in [
        (0.0, 0, false),
        (-0.0, 0, false),
        (f32::NAN, 0, false),
        (-0.3, 1, false),
        (-0.4, 1, false),
        (-0.2, 2, true),
    ] {
        let mut f = Fixture::new();
        f.state.contact_offsets[0] = 0.02;
        assert_eq!(
            f.state
                .try_force_offset_for_penetration(0, distance, &mut f.services),
            Ok(())
        );
        assert_eq!(f.events().len(), reads);
        assert_eq!(
            f.state.contact_offsets[0].to_bits(),
            if corrected {
                (0.02_f32 - distance).to_bits()
            } else {
                0.02_f32.to_bits()
            }
        );
    }
    let mut f = Fixture::new();
    f.state.contact_offsets[0] = 0.02;
    f.services.draw = true;
    f.fail(2);
    assert_eq!(
        f.state
            .try_force_offset_for_penetration(0, -0.2, &mut f.services),
        Err("refused")
    );
    assert_eq!(
        f.events(),
        [Event::Penetration, Event::Draw, Event::Overlay]
    );
    assert_eq!(f.state.contact_offsets[0].to_bits(), 0.22_f32.to_bits());
}

#[test]
fn add_offset_uses_skateboard_direction_full4_and_unconditional_parent() {
    let mut f = Fixture::new();
    f.state.contact_offsets[2] = 0.125;
    f.data.skateboard = SkeletonRootMatrix::known([
        1.0, 0.0, 0.0, 2.0, 2.0, 3.0, 4.0, 5.0, 0.0, 0.0, 1.0, 6.0, 99.0, 98.0, 97.0, 96.0,
    ]);
    assert_eq!(f.targets.parents()[2], None);
    let primary = f.state.part_to_world[2];
    let parent = f.state.parent_to_world[2];
    assert_eq!(
        f.state
            .try_add_in_contact_offset(2, &f.data, &mut f.services),
        Ok(())
    );
    assert_eq!(
        f.state.part_to_world[2].columns()[3],
        vector([0.25, 0.375, 0.5, 2.625])
    );
    assert_eq!(
        f.state.parent_to_world[2].columns()[3],
        vector([3.25, 4.375, 5.5, 6.625])
    );
    assert_eq!(
        f.state.part_to_world[2].columns()[..3],
        primary.columns()[..3]
    );
    assert_eq!(
        f.state.parent_to_world[2].columns()[..3],
        parent.columns()[..3]
    );
}

#[test]
fn add_offset_debug_failure_precedes_both_positions_and_unknown_zero_product_is_retained() {
    let mut f = Fixture::new();
    f.services.draw = true;
    f.fail(1);
    let old = f.state.part_to_world;
    let parents = f.state.parent_to_world;
    assert_eq!(
        f.state
            .try_add_in_contact_offset(0, &f.data, &mut f.services),
        Err("refused")
    );
    assert_eq!(f.state.part_to_world, old);
    assert_eq!(f.state.parent_to_world, parents);
    let mut f = Fixture::new();
    let mut columns = *f.data.skateboard.columns();
    columns[0].w = None;
    f.data.skateboard = SkeletonRootMatrix::from_columns(columns);
    assert_eq!(
        f.state
            .try_add_in_contact_offset(0, &f.data, &mut f.services),
        Ok(())
    );
    assert_eq!(f.state.part_to_world[0].columns()[3].w, None);
    assert_eq!(f.state.parent_to_world[0].columns()[3].w, None);
}

#[test]
fn distance_assertion_and_prospective_transform_do_not_require_known_w() {
    let mut f = Fixture::new();
    assert_eq!(
        f.state.get_dist_to_contact::<&str, &str>(0, &f.data),
        Err(Error::ContactNotValid { entry: 0, state: 0 })
    );
    f.valid(-0.1);
    f.state.contact_offsets[0] = 0.02;
    let old = f.state.part_to_world[0];
    let mut columns = *f.data.inverse_skateboard.columns();
    for column in &mut columns {
        column.w = None;
    }
    f.data.inverse_skateboard = SkeletonRootMatrix::from_columns(columns);
    let actual = f
        .state
        .get_dist_to_contact::<&str, &str>(0, &f.data)
        .unwrap();
    assert_eq!(
        actual.to_bits(),
        ((0.02_f32 - CONTACT_DISTANCE) - (-0.1_f32)).to_bits()
    );
    assert_eq!(f.state.part_to_world[0], old);
    assert!(f.events().is_empty());
}

#[test]
fn line_query_preserves_full4_frozen_projection_and_both_side_signs() {
    // Independent explicit-f32/ucrt fmaf oracle, host dot reduction, not Xenon.
    for (entry, start, end) in [
        (
            0,
            [0xbd13_74f0, 0xbe1b_a5f0, 0x4080_0000, 0x3ca3_d6b0],
            [0x3f10_624b, 0xbf73_b649, 0x4080_0000, 0x3f05_1eb6],
        ),
        (
            1,
            [0xbef7_cede, 0xbef9_db28, 0x4080_0000, 0xbdf5_c294],
            [0x3ded_9158, 0xbfa4_dd30, 0x4080_0000, 0x3ec2_8f5b],
        ),
    ] {
        let mut f = Fixture::new();
        f.processed.effective = SkeletonRootMatrix::known([
            0.8, 0.6, 0.0, 0.25, -0.6, 0.8, 0.0, -0.5, 0.0, 0.0, 1.0, 0.0, 1.0, -2.0, 0.5, 3.0,
        ]);
        set_position(
            &mut f.state.part_to_world[entry],
            vector([2.0, 1.0, 4.0, 0.75]),
        );
        assert_eq!(f.line(entry), Ok(()));
        let query = &f.services.queries[0];
        assert_eq!(scratch(query.start).map(f32::to_bits), start);
        assert_eq!(scratch(query.end).map(f32::to_bits), end);
        assert_eq!(query.label, "SkeletonIK::ContactLineTest");
        assert_eq!(query.fatness.to_bits(), 0);
        assert_eq!(
            f.events(),
            [
                Event::Effective,
                Event::Sideways,
                Event::Dot,
                Event::SideDraw,
                Event::Query
            ]
        );
    }
}

#[test]
fn segment_end_is_start_plus_down_and_unknown_w_is_not_preflighted() {
    let mut f = Fixture::new();
    let mut columns = *f.processed.effective.columns();
    columns[1].w = Some(-f32::from_bits(0x4019_ae9b));
    f.processed.effective = SkeletonRootMatrix::from_columns(columns);
    set_position(
        &mut f.state.part_to_world[0],
        vector([0.0, 0.0, 0.0, f32::from_bits(0xbf97_328b)]),
    );
    assert_eq!(f.line(0), Ok(()));
    let query = &f.services.queries[0];
    assert_eq!(query.start.w.unwrap().to_bits(), 0xbff3_681c);
    assert_eq!(query.end.w.unwrap().to_bits(), 0x3eff_d468);
    let shortcut = f32::from_bits(0xbf97_328b) + f32::from_bits(0x4019_ae9b) * 0.7;
    assert_eq!(shortcut.to_bits(), 0x3eff_d46c);
    let mut f = Fixture::new();
    let mut columns = *f.processed.effective.columns();
    columns[0].w = None;
    f.processed.effective = SkeletonRootMatrix::from_columns(columns);
    assert_eq!(f.line(0), Ok(()));
    assert_eq!(f.services.queries[0].start.w, None);
    assert_eq!(f.services.queries[0].end.w, None);
}

#[test]
fn query_hit_miss_and_refusal_are_distinct_and_dot_precedes_draw_value() {
    let mut f = Fixture::new();
    let old = f.state.contact_positions[0];
    f.state.contact_states[0] = 0;
    f.fail(2);
    assert_eq!(f.line(0), Err(Error::Numeric("refused")));
    assert_eq!(f.events(), [Event::Effective, Event::Sideways, Event::Dot]);
    assert_eq!(f.state.contact_positions[0], old);
    let mut f = Fixture::new();
    f.state.contact_states[0] = 0;
    let old = f.state.contact_positions[0];
    f.fail(4);
    assert_eq!(f.line(0), Err(Error::Service("refused")));
    assert_eq!(f.state.contact_states[0], 0);
    assert_eq!(f.state.contact_positions[0], old);
    let mut f = Fixture::new();
    let old = f.state.contact_positions[0];
    assert_eq!(f.line(0), Ok(()));
    assert_eq!(f.state.contact_states[0], 2);
    assert_eq!(f.state.contact_positions[0], old);
    let mut f = Fixture::new();
    let hit = SkeletonRootVector {
        xyz: [4.0, 5.0, 6.0],
        w: None,
    };
    f.services.result = SkeletonIkContactLineResult::Hit { position: hit };
    assert_eq!(f.line(0), Ok(()));
    assert_eq!(f.state.contact_positions[0], hit);
    assert_eq!(f.state.contact_states[0], 1);
}

#[test]
fn query_failure_keeps_caller_state_clear_and_all_preceding_offset_and_flag_stores() {
    let mut complete = Fixture::new();
    complete.state.contact_states[0] = 2;
    complete.state.contact_target_offsets[0] = 0.07;
    complete.processed.flags.set(0x0200_0000);
    assert_eq!(complete.run(), Ok(()));
    let events = complete.events();
    let query = events.iter().position(|e| *e == Event::Query).unwrap();
    let mut f = Fixture::new();
    f.state.contact_states[0] = 2;
    f.state.contact_target_offsets[0] = 0.07;
    f.processed.flags.set(0x0200_0000);
    let old_contact = f.state.contact_positions[0];
    let second = f.state.contact_states[1];
    f.fail(query);
    assert_eq!(f.run(), Err(Error::Service("refused")));
    assert_eq!(f.state.contact_states[0], 0);
    assert_eq!(f.state.contact_states[1], second);
    assert_eq!(f.state.contact_positions[0], old_contact);
    assert!(f.state.tell_anim_dont_push_or_brake);
    assert!(f.state.skateboard_dont_push);
    assert_eq!(f.state.part_to_world[0], complete.state.part_to_world[0]);
    assert_eq!(
        f.state.parent_to_world[0],
        complete.state.parent_to_world[0]
    );
    assert_eq!(f.events(), events[..=query]);
}

#[test]
fn common_debug_failure_retains_first_hit_before_second_foot_starts() {
    let mut complete = Fixture::new();
    assert_eq!(complete.run(), Ok(()));
    let events = complete.events();
    let query = events.iter().position(|e| *e == Event::Query).unwrap();
    let mut f = Fixture::new();
    let hit = vector([8.0, 9.0, 10.0, 11.0]);
    f.services.result = SkeletonIkContactLineResult::Hit { position: hit };
    f.fail(query + 1);
    assert_eq!(f.run(), Err(Error::Service("refused")));
    assert_eq!(f.events().last(), Some(&Event::Draw));
    assert_eq!(f.state.contact_positions[0], hit);
    assert_eq!(f.state.contact_states[0], 1);
    assert_eq!(f.state.contact_states[1], 3);
}

#[test]
fn scalar_bounds_preserve_unordered_and_fneg_decay_graphs() {
    assert_eq!(
        bound(f32::NAN, TARGET_MIN, TARGET_MAX).to_bits(),
        TARGET_MAX.to_bits()
    );
    for value in [-0.0, 0.0, -0.2, 0.2] {
        let mut f = Fixture::new();
        f.state.modes[0] = 0;
        f.state.contact_offsets[0] = value;
        assert_eq!(f.run(), Ok(()));
        let expected = if value < -STEP_MAX {
            value + STEP_MAX
        } else if value > STEP_MAX {
            value + STEP_MIN
        } else {
            -value + value
        };
        assert_eq!(f.state.contact_offsets[0].to_bits(), expected.to_bits());
    }
}
