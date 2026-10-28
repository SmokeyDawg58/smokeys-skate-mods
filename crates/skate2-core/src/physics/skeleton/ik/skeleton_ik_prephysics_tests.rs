//! Controlled primitives use the declared host graph. Geometric row checks
//! complement the frozen TwoBone/line-frame graphs; they are not a native trace.
use super::*;
use crate::{reckoning_numeric::HostReckoningNumeric, skeleton_data_tests};
use std::{cell::RefCell, rc::Rc};

type Error =
    SkeletonIkPrePhysicsError<&'static str, &'static str, &'static str, &'static str, &'static str>;
#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
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
    fail: Option<usize>,
    dot_override: Option<(usize, f32)>,
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
        let index = self.events.borrow().len();
        self.event(Event::Dot)?;
        if let Some((target, result)) = self.dot_override {
            if index == target {
                return Ok(result);
            }
        }
        Ok(HostReckoningNumeric.dot3(a, b).unwrap())
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
struct Rows {
    trace: Trace,
    raw: [SkeletonRootVector; 24],
    mapped: [SkeletonRootMatrix; 24],
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
        self.mapped[part.index()] = *value;
        Ok(())
    }
}
struct Attributes {
    trace: Trace,
    minimum: f32,
    maximum: f32,
}
impl SkeletonIkPrePhysicsAttributes for Attributes {
    type Error = &'static str;
    fn minimum_angle_degrees(&mut self) -> Result<f32, Self::Error> {
        self.trace.event(Event::Minimum)?;
        Ok(self.minimum)
    }
    fn maximum_angle_degrees(&mut self) -> Result<f32, Self::Error> {
        self.trace.event(Event::Maximum)?;
        Ok(self.maximum)
    }
}
struct Services {
    trace: Trace,
    draw: [bool; 2],
    next: usize,
}
impl SkeletonIkPrePhysicsServices for Services {
    type Error = &'static str;
    fn draw_pre_feet_ik(&mut self) -> Result<bool, Self::Error> {
        self.trace.event(Event::Draw)?;
        let value = self.draw[self.next];
        self.next += 1;
        Ok(value)
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.trace.event(Event::Overlay)
    }
}
struct Fixture {
    data: SkeletonData,
    rows: Rows,
    attributes: Attributes,
    services: Services,
    trace: Trace,
    selected: Option<SkeletonPartIndex>,
    delta: SkeletonRootVector,
    range: f32,
    adjust: SkeletonRootVector,
}
fn sentinel() -> SkeletonRootVector {
    SkeletonRootVector::known([31.0, 32.0, 33.0, 34.0])
}
fn data(parent: bool, grandparent: bool) -> SkeletonData {
    let (mut hierarchy, parameters, pose) = skeleton_data_tests::fixture();
    hierarchy.parents.fill(-1);
    if parent {
        hierarchy.parents[3] = 8;
    }
    if grandparent {
        hierarchy.parents[8] = 12;
    }
    skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap()
}
impl Fixture {
    fn new() -> Self {
        let trace = Trace::default();
        let mut raw = [sentinel(); 24];
        raw[12] = ZERO;
        raw[8] = SkeletonRootVector::known([1.0, 1.0, 0.0, 0.0]);
        raw[3] = SkeletonRootVector::known([2.0, 0.0, 0.0, 0.0]);
        let mapped = std::array::from_fn(|index| {
            let mut lanes = IDENTITY;
            lanes[0] = 2.0;
            lanes[5] = 3.0;
            lanes[10] = 4.0;
            lanes[12..16].copy_from_slice(&[
                f32::from(u16::try_from(index).unwrap()) * 0.25,
                0.5,
                0.75,
                2.0,
            ]);
            SkeletonRootMatrix::known(lanes)
        });
        Self {
            data: data(true, true),
            rows: Rows {
                trace: trace.clone(),
                raw,
                mapped,
            },
            attributes: Attributes {
                trace: trace.clone(),
                minimum: 0.0,
                maximum: 180.0,
            },
            services: Services {
                trace: trace.clone(),
                draw: [false; 2],
                next: 0,
            },
            trace,
            selected: SkeletonPartIndex::from_index(3),
            delta: SkeletonRootVector::known([-0.5, 0.0, 0.0, 0.0]),
            range: 100.0,
            adjust: sentinel(),
        }
    }
    fn fail_at(&mut self, index: usize) {
        self.trace.fail = Some(index);
        self.rows.trace.fail = Some(index);
        self.attributes.trace.fail = Some(index);
        self.services.trace.fail = Some(index);
    }
    fn run(&mut self) -> Result<bool, Error> {
        try_do_two_bone_prephysics(
            SkeletonIkPrePhysicsInput {
                data: &self.data,
                selected_part: self.selected,
                delta: self.delta,
                max_length: self.range,
            },
            &mut self.rows,
            &mut self.attributes,
            &mut self.services,
            &mut self.adjust,
            &mut self.trace.clone(),
            &mut self.trace.clone(),
        )
    }
    fn events(&self) -> Vec<Event> {
        self.trace.events.borrow().clone()
    }
}

#[test]
fn ancestry_assertions_and_raw_pose_accesses_follow_actual_part_tables() {
    let mut f = Fixture::new();
    f.selected = None;
    assert_eq!(f.run(), Err(Error::MissingSelectedPart));
    assert!(f.events().is_empty());
    let mut f = Fixture::new();
    f.data = data(false, true);
    assert_eq!(
        f.run(),
        Err(Error::MissingParent {
            selected: f.selected.unwrap()
        })
    );
    assert!(f.events().is_empty());
    let mut f = Fixture::new();
    f.data = data(true, false);
    assert_eq!(
        f.run(),
        Err(Error::MissingGrandparent {
            parent: SkeletonPartIndex::from_index(8).unwrap()
        })
    );
    assert_eq!(f.events(), [Event::Animation(3), Event::Animation(8)]);
    assert_eq!(f.adjust, sentinel());
    for index in 0..3 {
        let mut f = Fixture::new();
        let original = f.rows.mapped;
        f.fail_at(index);
        assert_eq!(f.run(), Err(Error::Rows("refused")));
        assert_eq!(
            f.events(),
            [
                Event::Animation(3),
                Event::Animation(8),
                Event::Animation(12)
            ][..=index]
        );
        assert_eq!(f.adjust, sentinel());
        assert_eq!(f.rows.mapped, original);
    }
}

#[test]
fn clamp_and_both_lazy_global_reads_precede_any_adjustment() {
    let prefix = [
        Event::Animation(3),
        Event::Animation(8),
        Event::Animation(12),
        Event::Dot,
        Event::Seed,
        Event::Draw,
        Event::Minimum,
        Event::Maximum,
    ];
    for index in 3..8 {
        let mut f = Fixture::new();
        let original = f.rows.mapped;
        f.fail_at(index);
        let expected = match index {
            3 | 4 => Error::Clamp(SkeletonIkClampError::Numeric("refused")),
            5 => Error::Service("refused"),
            _ => Error::Attribute("refused"),
        };
        assert_eq!(f.run(), Err(expected));
        assert_eq!(f.events(), prefix[..=index]);
        assert_eq!(f.adjust, sentinel());
        assert_eq!(f.rows.mapped, original);
    }
}

#[test]
fn initial_debug_batches_dots_then_seeds_and_only_clamping_requires_overlay() {
    let mut f = Fixture::new();
    f.services.draw[0] = true;
    f.range = 1.0;
    f.fail_at(10);
    assert_eq!(f.run(), Err(Error::Service("refused")));
    assert_eq!(
        &f.events()[5..],
        [
            Event::Draw,
            Event::Dot,
            Event::Dot,
            Event::Seed,
            Event::Seed,
            Event::Overlay
        ]
    );
    assert_eq!(f.adjust, sentinel());
    let mut f = Fixture::new();
    f.services.draw[0] = true;
    f.fail_at(10);
    assert_eq!(f.run(), Err(Error::Attribute("refused")));
    assert_eq!(f.events()[10], Event::Minimum);
}

#[test]
fn invalid_solver_zeroes_adjustment_and_skips_final_debug_and_all_rows() {
    let mut f = Fixture::new();
    f.rows.raw[8] = ZERO;
    f.services.draw[1] = true;
    let original = f.rows.mapped;
    assert_eq!(f.run(), Ok(false));
    assert_eq!(f.adjust, ZERO);
    assert_eq!(
        &f.events()[8..],
        [Event::Dot, Event::Dot, Event::Seed, Event::Seed]
    );
    assert_eq!(f.rows.mapped, original);
    assert_eq!(f.services.next, 1);
}

#[test]
fn unreachable_publishes_adjustment_before_outcome_assertion() {
    let mut f = Fixture::new();
    f.delta = SkeletonRootVector::known([3.0, 0.0, 0.0, 0.0]);
    let original = f.rows.mapped;
    assert_eq!(
        f.run(),
        Err(Error::UnexpectedOutcome(TwoBoneIkOutcome::Unreachable))
    );
    assert_eq!(
        f.adjust.xyz[0].to_bits(),
        (f32::from_bits(0x4035_04f3) - 5.0).to_bits()
    );
    assert_eq!(f.adjust.xyz[1..], [0.0, 0.0]);
    assert_eq!(f.adjust.w, Some(0.0));
    assert_eq!(f.rows.mapped, original);
    assert_eq!(f.services.next, 1);
}

fn close(actual: f32, expected: f64) {
    assert!(
        (f64::from(actual) - expected).abs() < 0.00003,
        "{actual:?} versus {expected:?}"
    );
}

#[test]
fn real_solver_changes_only_ancestors_using_independent_geometric_rotation_oracle() {
    let mut f = Fixture::new();
    let original = f.rows.mapped;
    let raw = f.rows.raw;
    assert_eq!(f.run(), Ok(true));
    assert_eq!(f.adjust, ZERO);
    // Equal sqrt(2) bones, requested end (1.5,0): midpoint x=.75,
    // positive bend y=sqrt(2-.75²). Independent Euclidean geometry, f64.
    let bend = (2.0_f64 - 0.75 * 0.75).sqrt();
    for (part, old_start, new_start, old_angle, new_angle) in [
        (
            12,
            [0.0, 0.0],
            [0.0, 0.0],
            std::f64::consts::FRAC_PI_4,
            bend.atan2(0.75),
        ),
        (
            8,
            [1.0, 1.0],
            [0.75, bend],
            -std::f64::consts::FRAC_PI_4,
            (-bend).atan2(0.75),
        ),
    ] {
        let (sine, cosine) = (new_angle - old_angle).sin_cos();
        let result = f.rows.mapped[part].known_lanes().unwrap();
        for (index, expected) in [(0, cosine), (1, sine), (4, -sine), (5, cosine), (10, 1.0)] {
            close(result[index], expected);
        }
        for index in [2, 3, 6, 7, 8, 9, 11, 15] {
            close(result[index], 0.0);
        }
        let p = original[part].columns()[3].xyz;
        let x = f64::from(p[0]) - old_start[0];
        let y = f64::from(p[1]) - old_start[1];
        close(result[12], new_start[0] + cosine * x - sine * y);
        close(result[13], new_start[1] + sine * x + cosine * y);
        close(result[14], f64::from(p[2]));
    }
    for (index, before) in original.iter().enumerate() {
        if index != 8 && index != 12 {
            assert_eq!(f.rows.mapped[index], *before);
        }
    }
    assert_eq!(f.rows.raw, raw);
    let row_events: Vec<_> = f
        .events()
        .into_iter()
        .filter(|event| matches!(event, Event::Read(_) | Event::Write(_)))
        .collect();
    assert_eq!(
        row_events,
        [
            Event::Read(12),
            Event::Write(12),
            Event::Read(8),
            Event::Write(8)
        ]
    );
}

#[test]
fn grandparent_commit_survives_later_parent_read_normalization_and_store_failures() {
    let mut complete = Fixture::new();
    assert_eq!(complete.run(), Ok(true));
    let events = complete.events();
    let parent_read = events
        .iter()
        .position(|event| *event == Event::Read(8))
        .unwrap();
    let parent_store = events
        .iter()
        .position(|event| *event == Event::Write(8))
        .unwrap();
    assert_eq!(parent_store - parent_read, 10); // nine numeric boundaries
    for index in parent_read..=parent_store {
        let mut f = Fixture::new();
        let old_parent = f.rows.mapped[8];
        f.fail_at(index);
        let expected = if index == parent_read || index == parent_store {
            Error::Rows("refused")
        } else {
            Error::Numeric("refused")
        };
        assert_eq!(f.run(), Err(expected), "callback {index}");
        assert_eq!(f.events(), events[..=index]);
        assert_eq!(f.rows.mapped[12], complete.rows.mapped[12]);
        assert_eq!(f.rows.mapped[8], old_parent);
        assert_eq!(f.adjust, ZERO);
    }
}

#[test]
fn final_debug_reads_again_and_acquires_overlay_before_length_after_both_writes() {
    let mut complete = Fixture::new();
    complete.services.draw[1] = true;
    assert_eq!(complete.run(), Ok(true));
    let events = complete.events();
    assert_eq!(
        &events[events.len() - 4..],
        [Event::Draw, Event::Overlay, Event::Dot, Event::Seed]
    );
    for index in events.len() - 4..events.len() {
        let mut f = Fixture::new();
        f.services.draw[1] = true;
        f.fail_at(index);
        let expected = if index < events.len() - 2 {
            Error::Service("refused")
        } else {
            Error::Numeric("refused")
        };
        assert_eq!(f.run(), Err(expected));
        assert_eq!(f.rows.mapped, complete.rows.mapped);
        assert_eq!(f.adjust, ZERO);
        assert_eq!(f.events(), events[..=index]);
    }
}

#[test]
fn solver_attribute_assertion_and_first_numeric_failure_preserve_adjustment() {
    let mut f = Fixture::new();
    f.attributes.minimum = f32::NAN;
    assert_eq!(
        f.run(),
        Err(Error::TwoBone(TwoBoneIkError::MinimumAngleAssertion {
            bits: f32::NAN.to_bits()
        }))
    );
    assert_eq!(f.events().last(), Some(&Event::Maximum));
    assert_eq!(f.adjust, sentinel());
    let mut f = Fixture::new();
    f.fail_at(8);
    assert_eq!(
        f.run(),
        Err(Error::TwoBone(TwoBoneIkError::Numeric("refused")))
    );
    assert_eq!(f.adjust, sentinel());
}

#[test]
fn typed_orthonormalizer_has_nine_ordered_boundaries_and_preserves_unknown_w() {
    let input = SkeletonRootMatrix::from_columns([
        SkeletonRootVector {
            xyz: [2.0, 1.0, 0.5],
            w: None,
        },
        SkeletonRootVector::known([0.5, 3.0, 1.0, 7.0]),
        SkeletonRootVector::known([0.25, 0.5, 4.0, 9.0]),
        SkeletonRootVector {
            xyz: [5.0, 6.0, 7.0],
            w: None,
        },
    ]);
    let mut trace = Trace::default();
    let result = try_normalize_rotation(&input, &mut trace).unwrap();
    assert_eq!(
        *trace.events.borrow(),
        [
            Event::Dot,
            Event::Seed,
            Event::Dot,
            Event::Dot,
            Event::Seed,
            Event::Dot,
            Event::Dot,
            Event::Dot,
            Event::Seed
        ]
    );
    assert_eq!(result.columns()[0].w, None);
    assert_eq!(result.columns()[3], input.columns()[3]);
    assert!(result.columns()[1].w.is_some());
    for index in 0..9 {
        let mut trace = Trace {
            fail: Some(index),
            ..Trace::default()
        };
        assert_eq!(try_normalize_rotation(&input, &mut trace), Err("refused"));
        assert_eq!(trace.events.borrow().len(), index + 1);
    }
}

#[test]
fn normal_fallback_and_upper_frame_false_keep_adjustment_and_still_sample_final_debug() {
    let mut complete = Fixture::new();
    assert_eq!(complete.run(), Ok(true));
    let events = complete.events();
    let first_row = events
        .iter()
        .position(|event| *event == Event::Read(12))
        .unwrap();
    // Two normal callbacks, then two mappings of eighteen callbacks each.
    // A zero normal takes the lower old-frame fallback. An unordered final
    // quality dot in the upper old frame skips only the upper new frame.
    // Lower false path: Z/X fallback dots, Y dot+seed, three quality dots,
    // then the final draw read. The overridden normal dot is counted separately.
    for (index, replacement, remaining) in [(first_row - 38, 0.0, 8), (first_row - 10, f32::NAN, 1)]
    {
        assert_eq!(events[index], Event::Dot);
        let mut f = Fixture::new();
        let original = f.rows.mapped;
        f.trace.dot_override = Some((index, replacement));
        assert_eq!(f.run(), Ok(false));
        assert_eq!(f.adjust, ZERO);
        assert_eq!(f.rows.mapped, original);
        assert_eq!(f.services.next, 2);
        assert_eq!(f.events().len(), index + 1 + remaining);
        assert_eq!(f.events().last(), Some(&Event::Draw));
    }
}

#[test]
fn first_ancestor_failures_have_published_adjustment_but_no_row_commit() {
    let mut complete = Fixture::new();
    assert_eq!(complete.run(), Ok(true));
    let events = complete.events();
    let read = events
        .iter()
        .position(|event| *event == Event::Read(12))
        .unwrap();
    let write = events
        .iter()
        .position(|event| *event == Event::Write(12))
        .unwrap();
    for index in read..=write {
        let mut f = Fixture::new();
        let original = f.rows.mapped;
        f.fail_at(index);
        let expected = if index == read || index == write {
            Error::Rows("refused")
        } else {
            Error::Numeric("refused")
        };
        assert_eq!(f.run(), Err(expected));
        assert_eq!(f.rows.mapped, original);
        assert_eq!(f.adjust, ZERO);
        assert_eq!(f.events(), events[..=index]);
    }
}

#[test]
fn unknown_requested_w_remains_unknown_in_adjust_before_unreachable_assertion() {
    let mut f = Fixture::new();
    f.delta = SkeletonRootVector {
        xyz: [3.0, 0.0, 0.0],
        w: None,
    };
    assert_eq!(
        f.run(),
        Err(Error::UnexpectedOutcome(TwoBoneIkOutcome::Unreachable))
    );
    assert_eq!(f.adjust.w, None);
    assert_eq!(
        f.adjust.xyz[0].to_bits(),
        (f32::from_bits(0x4035_04f3) - 5.0).to_bits()
    );
}
