use super::*;
use crate::{
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
    skeleton_data_tests::{build, fixture},
    skeleton_ik_targets::SkeletonIkTargets,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

const IDENTITY: AnimationMatrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];
type Events = Rc<RefCell<Vec<&'static str>>>;
struct Attributes {
    result: Result<f32, &'static str>,
    events: Events,
    times: Vec<f32>,
}
impl SkeletonLandingOnBoardAttributes for Attributes {
    type Error = &'static str;
    fn blend(&mut self, time: f32) -> Result<f32, Self::Error> {
        self.events.borrow_mut().push("blend");
        self.times.push(time);
        self.result
    }
}
struct Cvars {
    show: Result<u8, &'static str>,
    overlay: Result<(), &'static str>,
    negate: Result<u8, &'static str>,
    frames: [Result<f32, &'static str>; 2],
    reads: usize,
    events: Events,
}
impl SkeletonLandingOnBoardCvars for Cvars {
    type Error = &'static str;
    fn show_landing_adjust(&mut self) -> Result<u8, Self::Error> {
        self.events.borrow_mut().push("show");
        self.show
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.events.borrow_mut().push("overlay");
        self.overlay
    }
    fn negate_y(&mut self) -> Result<u8, Self::Error> {
        self.events.borrow_mut().push("negate");
        self.negate
    }
    fn frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        let index = self.reads;
        self.reads += 1;
        self.events
            .borrow_mut()
            .push(if index == 0 { "full count" } else { "Y count" });
        self.frames[index]
    }
}
struct Fixture {
    offset: SkeletonSkateboardOffset,
    input: SkeletonLandingOnBoardInput,
    mapped: AnimationMatrix,
    world: AnimationMatrix,
    attributes: Attributes,
    cvars: Cvars,
    events: Events,
}
type Error = SkeletonLandingOnBoardError<
    &'static str,
    &'static str,
    crate::reckoning_numeric::ReckoningHostNumericError,
    std::convert::Infallible,
>;
impl Fixture {
    fn new() -> Self {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut offset = SkeletonSkateboardOffset::new();
        offset.transform[12..].copy_from_slice(&[101.0, 103.0, 107.0, 109.0]);
        offset.full_count = 11.0;
        offset.vertical_count = 13.0;
        let mut mapped = IDENTITY;
        mapped[12..].copy_from_slice(&[2.0, 3.0, 5.0, 7.0]);
        let mut world = IDENTITY;
        world[12..].copy_from_slice(&[11.0, 13.0, 17.0, 19.0]);
        let mut deck_transform = IDENTITY;
        deck_transform[12..].copy_from_slice(&[31.0, 37.0, 41.0, 43.0]);
        Self {
            offset,
            mapped,
            world,
            input: SkeletonLandingOnBoardInput {
                flags_8d0: 0,
                deck_transform,
                landing_on_deck_adjust: [1.0, 2.0, 4.0, f32::NAN],
                time_to_land: 2.5,
            },
            attributes: Attributes {
                result: Ok(1.0),
                events: Rc::clone(&events),
                times: Vec::new(),
            },
            cvars: Cvars {
                show: Ok(0),
                overlay: Ok(()),
                negate: Ok(1),
                frames: [Ok(3.0), Ok(5.0)],
                reads: 0,
                events: Rc::clone(&events),
            },
            events,
        }
    }
    fn run(&mut self, com_y: f32) -> Result<(), Error> {
        try_update_landing_on_board(
            &mut self.offset,
            &self.mapped,
            &self.world,
            com_y,
            &self.input,
            &mut self.attributes,
            &mut self.cvars,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
        )
    }
    fn snapshot(&self) -> ([u32; 16], u32, u32, bool, bool) {
        (
            self.offset.transform.map(f32::to_bits),
            self.offset.full_count.to_bits(),
            self.offset.vertical_count.to_bits(),
            self.offset.full_just_set,
            self.offset.vertical_just_set,
        )
    }
}

#[test]
fn ordered_com_gate_and_flip_flag_skip_every_service_and_preserve_offset() {
    for com_y in [0.0, -0.0, 1.0, f32::INFINITY, f32::NAN] {
        let mut f = Fixture::new();
        let before = f.snapshot();
        f.input.deck_transform = [f32::NAN; 16];
        f.run(com_y).unwrap();
        assert_eq!(f.snapshot(), before);
        assert!(f.events.borrow().is_empty());
    }
    let mut f = Fixture::new();
    f.input.flags_8d0 = 0x10;
    let before = f.snapshot();
    f.run(-1.0).unwrap();
    assert_eq!(f.snapshot(), before);
    assert!(f.events.borrow().is_empty());
}

#[test]
fn active_producer_retains_full_w_then_actual_offset_consumer_updates_deck_and_ik() {
    for negate in [0, 3] {
        let mut f = Fixture::new();
        f.cvars.negate = Ok(negate);
        f.run(f32::NEG_INFINITY).unwrap();
        let y = if negate == 0 { 47.0_f32 } else { -47.0_f32 };
        let mut expected = IDENTITY;
        expected[12..].copy_from_slice(&[1.0, y, 4.0, 7.0]);
        assert_eq!(
            f.offset.transform.map(f32::to_bits),
            expected.map(f32::to_bits)
        );
        assert_eq!(f.offset.full_count.to_bits(), 3.0_f32.to_bits());
        assert_eq!(f.offset.vertical_count.to_bits(), 5.0_f32.to_bits());
        assert!(f.offset.full_just_set && f.offset.vertical_just_set);
        assert_eq!(
            &*f.events.borrow(),
            &["blend", "show", "negate", "full count", "Y count"]
        );
        assert_eq!(f.attributes.times[0].to_bits(), 2.5_f32.to_bits());
        let (hierarchy, parameters, pose) = fixture();
        let mut ik = SkeletonIkTargets::new(Arc::new(build(hierarchy, parameters, &pose).unwrap()));
        let mut deck = IDENTITY;
        f.offset
            .update(&mut deck, &mut ik, &mut HostSkateboardSpawnMathNumeric);
        assert_eq!(deck.map(f32::to_bits), expected.map(f32::to_bits));
        for target in ik.targets() {
            assert_eq!(target.map(f32::to_bits), expected.map(f32::to_bits));
        }
        assert!(!f.offset.full_just_set && !f.offset.vertical_just_set);
    }
}

#[test]
fn desired_height_only_raises_mapped_deck_and_ignores_adjustment_w() {
    for deck_y in [-20.0, -10.0, f32::NAN] {
        let mut f = Fixture::new();
        f.input.deck_transform[13] = deck_y;
        f.input.landing_on_deck_adjust[3] = f32::INFINITY;
        f.run(-1.0).unwrap();
        assert_eq!(f.offset.transform[13].to_bits(), (-0.0_f32).to_bits());
        assert_eq!(f.offset.transform[12].to_bits(), 1.0_f32.to_bits());
        assert_eq!(f.offset.transform[14].to_bits(), 4.0_f32.to_bits());
        assert_eq!(f.offset.transform[15].to_bits(), 7.0_f32.to_bits());
    }
}

#[test]
fn attributes_debug_singleton_and_negate_refusals_precede_every_offset_store() {
    for stage in 0..4 {
        let mut f = Fixture::new();
        let before = f.snapshot();
        match stage {
            0 => f.attributes.result = Err("attribute"),
            1 => f.cvars.show = Err("show"),
            2 => {
                f.cvars.show = Ok(2);
                f.cvars.overlay = Err("missing singleton");
            }
            _ => f.cvars.negate = Err("negate"),
        }
        let error = f.run(-1.0).unwrap_err();
        match stage {
            0 => assert!(matches!(error, Error::Attribute("attribute"))),
            1 => assert!(matches!(error, Error::Cvar("show"))),
            2 => assert!(matches!(error, Error::DebugOverlay("missing singleton"))),
            _ => assert!(matches!(error, Error::Cvar("negate"))),
        }
        assert_eq!(f.snapshot(), before);
        assert_eq!(f.cvars.reads, 0);
        let expected: &[&str] = match stage {
            0 => &["blend"],
            1 => &["blend", "show"],
            2 => &["blend", "show", "overlay"],
            _ => &["blend", "show", "negate"],
        };
        assert_eq!(&*f.events.borrow(), expected);
    }
}

#[test]
fn debug_enabled_only_requires_singleton_between_show_and_negate() {
    let mut f = Fixture::new();
    f.cvars.show = Ok(0x80);
    f.run(-1.0).unwrap();
    assert_eq!(
        &*f.events.borrow(),
        &[
            "blend",
            "show",
            "overlay",
            "negate",
            "full count",
            "Y count"
        ]
    );
}

#[test]
fn first_and_second_blend_count_refusals_preserve_their_distinct_zero_y_prefixes() {
    for failure in 0..2 {
        let mut f = Fixture::new();
        f.cvars.frames[failure] = Err("blend count");
        assert!(matches!(f.run(-1.0), Err(Error::Cvar("blend count"))));
        let mut expected = IDENTITY;
        expected[12..].copy_from_slice(&[1.0, 0.0, 4.0, 7.0]);
        assert_eq!(
            f.offset.transform.map(f32::to_bits),
            expected.map(f32::to_bits)
        );
        assert!(f.offset.full_just_set);
        assert!(!f.offset.vertical_just_set);
        assert_eq!(
            f.offset.full_count.to_bits(),
            if failure == 0 {
                11.0_f32.to_bits()
            } else {
                3.0_f32.to_bits()
            }
        );
        assert_eq!(f.offset.vertical_count.to_bits(), 13.0_f32.to_bits());
        assert_eq!(f.cvars.reads, failure + 1);
    }
}

#[test]
fn independent_count_reads_preserve_nonfinite_bits_without_changing_production() {
    let mut f = Fixture::new();
    f.cvars.frames = [Ok(f32::from_bits(0x7fc0_1234)), Ok(-0.0)];
    f.run(-1.0).unwrap();
    assert_eq!(f.offset.full_count.to_bits(), 0x7fc0_1234);
    assert_eq!(f.offset.vertical_count.to_bits(), (-0.0_f32).to_bits());
    assert_eq!(f.offset.transform[13].to_bits(), (-47.0_f32).to_bits());
    assert!(f.offset.full_just_set && f.offset.vertical_just_set);
}

struct RefusingNumeric;
impl ReckoningNumeric for RefusingNumeric {
    type Error = &'static str;
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        Err("query dot")
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("first dot refused")
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("first dot refused")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no integer conversion")
    }
}

#[test]
fn interpolation_refusal_occurs_after_attribute_before_cvars_or_offset_writes() {
    let mut f = Fixture::new();
    f.attributes.result = Ok(0.5);
    let before = f.snapshot();
    assert_eq!(
        try_update_landing_on_board(
            &mut f.offset,
            &f.mapped,
            &f.world,
            -1.0,
            &f.input,
            &mut f.attributes,
            &mut f.cvars,
            &mut RefusingNumeric,
            &mut HostReckoningSinCosNumeric
        ),
        Err(SkeletonLandingOnBoardError::Interpolation(
            SkeletonAffineInterpolationError::Numeric("query dot")
        ))
    );
    assert_eq!(f.snapshot(), before);
    assert_eq!(&*f.events.borrow(), &["blend"]);
}
