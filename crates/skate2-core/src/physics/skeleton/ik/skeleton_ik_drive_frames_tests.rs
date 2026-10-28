#![expect(
    clippy::float_cmp,
    reason = "exact finite fixture values; numerical discriminators use bit assertions"
)]
use super::super::initial::tests::{data, matrix};
use super::*;
use crate::{ragdoll::SkeletonPartIndex, skeleton_data_tests};
use std::{cell::RefCell, rc::Rc, sync::Arc};

type DotArguments = Rc<RefCell<Vec<([f32; 4], [f32; 4])>>>;

type Error = SkeletonIkDriveFramesError<
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
>;
#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Enable,
    Leg,
    Arm,
    Animation(usize),
    Read(usize),
    Write(usize),
    Draw,
    Overlay,
    Minimum,
    Maximum,
    Dot,
    Seed,
    Reciprocal,
    Round,
}
#[derive(Clone, Default)]
struct Trace {
    events: Rc<RefCell<Vec<Event>>>,
    dots: DotArguments,
    fail: Option<usize>,
}
impl Trace {
    fn event(&self, event: Event) -> Result<(), &'static str> {
        let index = self.events.borrow().len();
        self.events.borrow_mut().push(event);
        if self.fail == Some(index) {
            Err("refused")
        } else {
            Ok(())
        }
    }
}
impl ReckoningNumeric for Trace {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.dots.borrow_mut().push((a, b));
        self.event(Event::Dot)?;
        Ok((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event(Event::Seed)?;
        Ok(1.0 / value.sqrt())
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event(Event::Reciprocal)?;
        Ok(1.0 / value)
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no integer conversion")
    }
}
impl ReckoningSinCosNumeric for Trace {
    type Error = &'static str;
    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event(Event::Round)?;
        Ok(value.round_ties_even())
    }
}
struct Services {
    trace: Trace,
    enabled: bool,
    leg: f32,
    arm: f32,
}
impl SkeletonIkPrePhysicsServices for Services {
    type Error = &'static str;
    fn draw_pre_feet_ik(&mut self) -> Result<bool, Self::Error> {
        self.trace.event(Event::Draw)?;
        Ok(false)
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.trace.event(Event::Overlay)
    }
}
impl SkeletonIkDriveFramesServices for Services {
    fn enable_prephysics(&mut self) -> Result<bool, Self::Error> {
        self.trace.event(Event::Enable)?;
        Ok(self.enabled)
    }
    fn maximum_leg_length(&mut self) -> Result<f32, Self::Error> {
        self.trace.event(Event::Leg)?;
        Ok(self.leg)
    }
    fn maximum_arm_length(&mut self) -> Result<f32, Self::Error> {
        self.trace.event(Event::Arm)?;
        Ok(self.arm)
    }
}
impl SkeletonIkPrePhysicsAttributes for Trace {
    type Error = &'static str;
    fn minimum_angle_degrees(&mut self) -> Result<f32, Self::Error> {
        self.event(Event::Minimum)?;
        Ok(0.0)
    }
    fn maximum_angle_degrees(&mut self) -> Result<f32, Self::Error> {
        self.event(Event::Maximum)?;
        Ok(180.0)
    }
}
struct Rows {
    trace: Trace,
    raw: [SkeletonRootVector; 24],
    mapped: [SkeletonRootMatrix; 24],
    refuse_unknown: bool,
}
impl SkeletonIkPrePhysicsRows for Rows {
    type Error = &'static str;
    fn animation_position(
        &mut self,
        part: SkeletonPartIndex,
    ) -> Result<SkeletonRootVector, Self::Error> {
        self.trace.event(Event::Animation(part.index()))?;
        Ok(self.raw[part.index()])
    }
    fn mapped_transform(
        &mut self,
        part: SkeletonPartIndex,
    ) -> Result<SkeletonRootMatrix, Self::Error> {
        self.trace.event(Event::Read(part.index()))?;
        Ok(self.mapped[part.index()])
    }
    fn set_mapped_transform(
        &mut self,
        part: SkeletonPartIndex,
        value: &SkeletonRootMatrix,
    ) -> Result<(), Self::Error> {
        self.trace.event(Event::Write(part.index()))?;
        if self.refuse_unknown && value.known_lanes().is_none() {
            return Err("unknown W");
        }
        self.mapped[part.index()] = *value;
        Ok(())
    }
}
struct Fixture {
    state: SkeletonIkState,
    targets: SkeletonIkTargets,
    data: SkeletonIkDataIn,
    rows: Rows,
    services: Services,
    trace: Trace,
}
impl Fixture {
    fn new() -> Self {
        Self::with_bone_offset([0.0; 4])
    }
    fn with_bone_offset(offset: [f32; 4]) -> Self {
        let (mut hierarchy, mut parameters, pose) = skeleton_data_tests::fixture();
        hierarchy.parents.fill(-1);
        for (child, parent) in [
            (16, 8),
            (8, 12),
            (20, 9),
            (9, 13),
            (3, 8),
            (7, 10),
            (10, 14),
        ] {
            hierarchy.parents[child] = parent;
        }
        parameters.bones[3].position = offset;
        let mut targets = SkeletonIkTargets::new(Arc::new(
            skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap(),
        ));
        targets.initialize();
        let mut state = SkeletonIkState::new(&mut targets);
        state.modes[2] = 1;
        state.part_to_world[2] = SkeletonRootMatrix::known(matrix([1.5, 0.0, 0.0, 0.0]));
        let trace = Trace::default();
        let mut raw = [SkeletonRootVector::known([0.0; 4]); 24];
        for selected in [16, 20, 3, 7] {
            raw[selected] = SkeletonRootVector::known([2.0, 0.0, 0.0, 0.0]);
        }
        for middle in [8, 9, 10] {
            raw[middle] = SkeletonRootVector::known([1.0, 1.0, 0.0, 0.0]);
        }
        let mapped =
            std::array::from_fn(|_| SkeletonRootMatrix::known(matrix([7.0, 8.0, 9.0, 2.0])));
        Self {
            state,
            targets,
            data: data(),
            rows: Rows {
                trace: trace.clone(),
                raw,
                mapped,
                refuse_unknown: false,
            },
            services: Services {
                trace: trace.clone(),
                enabled: true,
                leg: 100.0,
                arm: 100.0,
            },
            trace,
        }
    }
    fn fail_at(&mut self, index: usize) {
        self.trace.fail = Some(index);
        self.rows.trace = self.trace.clone();
        self.services.trace = self.trace.clone();
    }
    fn run(&mut self) -> Result<(), Error> {
        self.state.try_update_drive_frames(
            &self.data,
            &self.targets,
            &mut self.rows,
            &mut self.trace.clone(),
            &mut self.services,
            &mut self.trace.clone(),
            &mut self.trace.clone(),
        )
    }
    fn events(&self) -> Vec<Event> {
        self.trace.events.borrow().clone()
    }
    fn feet(&mut self) {
        self.state.modes = [1, 0, 0, 0];
        self.state.part_to_world[0] = SkeletonRootMatrix::known(matrix([1.75, 0.125, 0.25, 0.0]));
        self.state.parent_to_world[0] = SkeletonRootMatrix::known(matrix([1.5, 0.0, 0.0, 0.0]));
        self.services.leg = 1.25;
    }
}

#[test]
fn disabled_service_returns_before_lengths_and_any_pose_read() {
    let mut f = Fixture::new();
    f.services.enabled = false;
    let before = f.rows.mapped;
    assert_eq!(f.run(), Ok(()));
    assert_eq!(f.events(), [Event::Enable]);
    assert_eq!(f.rows.mapped, before);
}

#[test]
fn both_lengths_are_sampled_once_even_when_all_modes_are_disabled_or_unknown() {
    let mut f = Fixture::new();
    f.state.modes = [0, 4, u32::MAX, 0];
    assert_eq!(f.run(), Ok(()));
    assert_eq!(f.events(), [Event::Enable, Event::Leg, Event::Arm]);
    for index in 0..3 {
        let mut f = Fixture::new();
        f.fail_at(index);
        assert_eq!(f.run(), Err(Error::Service("refused")));
        assert_eq!(
            f.events(),
            [Event::Enable, Event::Leg, Event::Arm][..=index]
        );
    }
}

#[test]
fn actual_selected_animation_is_read_before_helper_rereads_it_and_ancestors() {
    let prefix = [
        Event::Enable,
        Event::Leg,
        Event::Arm,
        Event::Animation(3),
        Event::Animation(3),
        Event::Animation(8),
        Event::Animation(12),
    ];
    for failure in 3..7 {
        let mut f = Fixture::new();
        let before = f.rows.mapped;
        f.fail_at(failure);
        let error = if failure == 3 {
            Error::Rows("refused")
        } else {
            Error::PrePhysics(SkeletonIkPrePhysicsError::Rows("refused"))
        };
        assert_eq!(f.run(), Err(error));
        assert_eq!(f.events(), prefix[..=failure]);
        assert_eq!(f.rows.mapped, before);
    }
}

#[test]
fn unknown_preinit_part_ids_refuse_at_conversion_after_all_three_services() {
    let mut f = Fixture::new();
    f.targets = SkeletonIkTargets::new(Arc::clone(f.targets.data()));
    assert_eq!(f.run(), Err(Error::MissingSelectedPart { entry: 2 }));
    assert_eq!(f.events(), [Event::Enable, Event::Leg, Event::Arm]);
}

#[test]
fn volume_to_bone_is_the_actual_inner_frame_before_requested_delta() {
    let mut f = Fixture::with_bone_offset([0.5, 0.0, 0.0, 0.0]);
    let mut target = matrix([2.5, 0.0, 0.0, 0.0]);
    target[..4].copy_from_slice(&[0.0, 1.0, 0.0, 0.0]);
    target[4..8].copy_from_slice(&[-1.0, 0.0, 0.0, 0.0]);
    f.state.part_to_world[2] = SkeletonRootMatrix::known(target);
    f.fail_at(7); // first clamp dot, after all actual animation reads
    assert!(f.run().is_err());
    let (left, right) = f.trace.dots.borrow()[0];
    assert_eq!(left, [2.5, -0.5, 0.0, 0.0]);
    assert_eq!(right, left);
}

#[test]
fn successful_hand_path_writes_ancestors_then_actual_selected_target_without_remapping() {
    let mut f = Fixture::new();
    let raw = f.rows.raw;
    let old = f.rows.mapped;
    assert_eq!(f.run(), Ok(()));
    assert_eq!(f.rows.raw, raw);
    let rows: Vec<_> = f
        .events()
        .into_iter()
        .filter(|event| matches!(event, Event::Read(_) | Event::Write(_)))
        .collect();
    assert_eq!(
        rows,
        [
            Event::Read(12),
            Event::Write(12),
            Event::Read(8),
            Event::Write(8),
            Event::Write(3)
        ]
    );
    assert_eq!(
        f.rows.mapped[3],
        SkeletonRootMatrix::known(matrix([1.5, 0.0, 0.0, 0.0]))
    );
    for (part, before) in old.iter().enumerate() {
        if ![3, 8, 12].contains(&part) {
            assert_eq!(f.rows.mapped[part], *before);
        }
    }
}

#[test]
fn foot_solver_selects_parent_then_commits_primary_and_parent_with_adjustment() {
    let mut f = Fixture::new();
    f.feet();
    assert_eq!(f.run(), Ok(()));
    assert_eq!(
        &f.events()[3..7],
        [
            Event::Animation(16),
            Event::Animation(16),
            Event::Animation(8),
            Event::Animation(12)
        ]
    );
    let writes: Vec<_> = f
        .events()
        .into_iter()
        .filter(|event| matches!(event, Event::Write(_)))
        .collect();
    assert_eq!(
        writes,
        [
            Event::Write(12),
            Event::Write(8),
            Event::Write(15),
            Event::Write(16)
        ]
    );
    assert_eq!(f.rows.mapped[15].columns()[3].xyz, [1.5, 0.125, 0.25]);
    assert_eq!(f.rows.mapped[16].columns()[3].xyz, [1.25, 0.0, 0.0]);
    assert_eq!(
        f.state.part_to_world[0].columns()[3].xyz,
        [1.5, 0.125, 0.25]
    );
    assert_eq!(
        f.state.parent_to_world[0].columns()[3].xyz,
        [1.25, 0.0, 0.0]
    );
}

#[test]
fn target_setter_failures_retain_exact_primary_and_world_position_prefixes() {
    let mut complete = Fixture::new();
    complete.feet();
    complete.run().unwrap();
    let events = complete.events();
    for failing_part in [15, 16] {
        let failure = events
            .iter()
            .position(|event| *event == Event::Write(failing_part))
            .unwrap();
        let mut f = Fixture::new();
        f.feet();
        let before = f.rows.mapped;
        let world = f.state.part_to_world[0];
        let parent_world = f.state.parent_to_world[0];
        f.fail_at(failure);
        assert_eq!(f.run(), Err(Error::Rows("refused")));
        assert_eq!(f.events(), events[..=failure]);
        assert_eq!(f.rows.mapped[12], complete.rows.mapped[12]);
        assert_eq!(f.rows.mapped[8], complete.rows.mapped[8]);
        assert_eq!(f.rows.mapped[16], before[16]);
        assert_eq!(f.state.parent_to_world[0], parent_world);
        if failing_part == 15 {
            assert_eq!(f.rows.mapped[15], before[15]);
            assert_eq!(f.state.part_to_world[0], world);
        } else {
            assert_eq!(f.rows.mapped[15], complete.rows.mapped[15]);
            assert_eq!(f.state.part_to_world[0], complete.state.part_to_world[0]);
        }
    }
}

#[test]
fn final_helper_debug_refusal_leaves_ancestor_writes_but_no_caller_target_commit() {
    let mut complete = Fixture::new();
    complete.feet();
    complete.run().unwrap();
    let failure = complete
        .events()
        .iter()
        .rposition(|event| *event == Event::Draw)
        .unwrap();
    let mut f = Fixture::new();
    f.feet();
    let before = f.rows.mapped;
    let world = f.state.part_to_world[0];
    f.fail_at(failure);
    assert_eq!(
        f.run(),
        Err(Error::PrePhysics(SkeletonIkPrePhysicsError::Service(
            "refused"
        )))
    );
    assert_eq!(f.rows.mapped[12], complete.rows.mapped[12]);
    assert_eq!(f.rows.mapped[8], complete.rows.mapped[8]);
    assert_eq!(f.rows.mapped[15], before[15]);
    assert_eq!(f.rows.mapped[16], before[16]);
    assert_eq!(f.state.part_to_world[0], world);
}

#[test]
fn world_positions_receive_same_animation_adjustment_without_rotating_it() {
    let mut f = Fixture::new();
    f.feet();
    let mut rotation = matrix([0.0; 4]);
    rotation[..4].copy_from_slice(&[0.0, 1.0, 0.0, 0.0]);
    rotation[4..8].copy_from_slice(&[-1.0, 0.0, 0.0, 0.0]);
    f.data.world_to_animation = SkeletonRootMatrix::known(rotation);
    f.state.part_to_world[0] = SkeletonRootMatrix::known(matrix([0.0, -1.75, 0.25, 0.0]));
    f.state.parent_to_world[0] = SkeletonRootMatrix::known(matrix([0.0, -1.5, 0.0, 0.0]));
    f.run().unwrap();
    assert_eq!(f.rows.mapped[15].columns()[3].xyz, [1.5, 0.0, 0.25]);
    assert_eq!(
        f.state.part_to_world[0].columns()[3].xyz,
        [-0.25, -1.75, 0.25]
    );
    assert_eq!(
        f.state.parent_to_world[0].columns()[3].xyz,
        [-0.25, -1.5, 0.0]
    );
}

#[test]
fn known_inverse_outer_recovers_target_row_w_but_retained_unknown_world_w_survives() {
    let mut f = Fixture::new();
    let mut columns = *f.state.part_to_world[2].columns();
    for column in &mut columns {
        column.w = None;
    }
    f.state.part_to_world[2] = SkeletonRootMatrix::from_columns(columns);
    f.rows.refuse_unknown = true;
    f.run().unwrap();
    assert!(f.rows.mapped[3].known_lanes().is_some());
    assert!(
        f.state.part_to_world[2]
            .columns()
            .iter()
            .all(|column| column.w.is_none())
    );
}

#[test]
fn invalid_helper_outcome_skips_target_writes_and_continues_next_active_entry() {
    let mut f = Fixture::new();
    f.state.modes = [0, 0, 1, 3];
    f.rows.raw[8] = SkeletonRootVector::known([0.0; 4]);
    f.state.part_to_world[3] = SkeletonRootMatrix::known(matrix([1.5, 0.0, 0.0, 0.0]));
    let old = f.rows.mapped[3];
    f.run().unwrap();
    assert_eq!(f.rows.mapped[3], old);
    assert_eq!(f.rows.mapped[7].columns()[3].xyz, [1.5, 0.0, 0.0]);
    assert!(!f.events().contains(&Event::Write(3)));
    assert!(f.events().contains(&Event::Write(7)));
    assert_eq!(
        f.events()
            .iter()
            .filter(|event| **event == Event::Arm)
            .count(),
        1
    );
}

#[test]
fn leg_and_arm_limits_are_selected_from_the_single_read_pair() {
    for (entry, selected, maximum) in [(0, 16, 0.75_f32), (1, 20, 0.75), (2, 3, 1.25), (3, 7, 1.25)]
    {
        let mut f = Fixture::new();
        f.state.modes = [0; 4];
        f.state.modes[entry] = 2;
        f.services.leg = 0.75;
        f.services.arm = 1.25;
        f.state.part_to_world[entry] = SkeletonRootMatrix::known(matrix([1.5, 0.0, 0.0, 0.0]));
        f.state.parent_to_world[entry] = SkeletonRootMatrix::known(matrix([1.5, 0.0, 0.0, 0.0]));
        f.run().unwrap();
        assert_eq!(
            f.rows.mapped[selected].columns()[3].xyz,
            [maximum, 0.0, 0.0]
        );
        assert_eq!(
            &f.events()[..4],
            [
                Event::Enable,
                Event::Leg,
                Event::Arm,
                Event::Animation(selected)
            ]
        );
    }
}
