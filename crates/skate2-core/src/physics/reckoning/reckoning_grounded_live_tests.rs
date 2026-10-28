//! Late-read failures preserve source-completed work on the original algorithm.
use super::{Attributes, Cvars, input};
use crate::{
    reckoning::{
        Reckoning, ReckoningError, ReckoningGroundInput,
        grounded_input::{
            GroundDirectionsRead, GroundInputError, GroundInputRead, GroundScalar, GroundVector,
        },
    },
    reckoning_numeric::HostReckoningNumeric,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::cell::RefCell;
struct Read {
    value: ReckoningGroundInput,
    missing_heading: bool,
    missing_com: bool,
    unknown_w: bool,
    events: RefCell<Vec<&'static str>>,
}
impl Read {
    fn new() -> Self {
        Self {
            value: input(),
            missing_heading: false,
            missing_com: false,
            unknown_w: false,
            events: RefCell::new(Vec::new()),
        }
    }
}
impl GroundDirectionsRead for Read {
    fn dynamic_up(&self) -> Result<SkeletonRootVector, GroundInputError> {
        self.events.borrow_mut().push("dynamic");
        Ok(SkeletonRootVector::known([0.0, 1.0, 0.0, 77.0]))
    }
    fn heading(&self) -> Result<SkeletonRootVector, GroundInputError> {
        self.events.borrow_mut().push("heading");
        if self.missing_heading {
            Err(GroundInputError::Missing("heading"))
        } else {
            Ok(SkeletonRootVector {
                xyz: [0.0, 0.0, 1.0],
                w: if self.unknown_w { None } else { Some(0.0) },
            })
        }
    }
    fn ground_y(&self) -> Result<f32, GroundInputError> {
        Ok(1.0)
    }
    fn ground(&self) -> Result<SkeletonRootVector, GroundInputError> {
        Ok(SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]))
    }
}
impl GroundInputRead for Read {
    fn scalar(&self, f: GroundScalar) -> Result<f32, GroundInputError> {
        if f == GroundScalar::BodySpin {
            self.events.borrow_mut().push("spin");
        }
        self.value.scalar(f)
    }
    fn vector(&self, f: GroundVector) -> Result<SkeletonRootVector, GroundInputError> {
        if f == GroundVector::ComToDeck {
            self.events.borrow_mut().push("com");
            if self.missing_com {
                return Err(GroundInputError::Missing("com"));
            }
        }
        let mut value = self.value.vector(f)?;
        if self.unknown_w
            && matches!(
                f,
                GroundVector::DeckY | GroundVector::DeckZ | GroundVector::EffectiveZ
            )
        {
            value.w = None;
        }
        Ok(value)
    }
    fn wheels(&self) -> Result<i32, GroundInputError> {
        self.value.wheels()
    }
    fn flag8c4(&self, m: u32) -> Result<bool, GroundInputError> {
        self.value.flag8c4(m)
    }
    fn flag8c8(&self, m: u32) -> Result<bool, GroundInputError> {
        self.value.flag8c8(m)
    }
}
#[test]
fn missing_heading_keeps_wrapper_dynamic_store_without_entering_body_spin() {
    let mut r = Reckoning::try_new(Attributes::new()).unwrap();
    let old = r.heading;
    let mut read = Read::new();
    read.missing_heading = true;
    let mut c = Cvars::new();
    assert!(matches!(
        r.try_update_grounded_live(&read, &mut c, &mut HostReckoningNumeric),
        Err(ReckoningError::Input(GroundInputError::Missing("heading")))
    ));
    assert_eq!(
        r.dynamic_up
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        [0.0, 1.0, 0.0, 77.0].map(f32::to_bits)
    );
    assert_eq!(
        r.heading
            .known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits),
        old.known_lanes()
            .expect("controlled known lanes")
            .map(f32::to_bits)
    );
    assert_eq!(*read.events.borrow(), ["dynamic", "heading"]);
}
#[test]
fn missing_com_is_late_to_the_real_body_spin_history_update() {
    let mut r = Reckoning::try_new(Attributes::new()).unwrap();
    let mut read = Read::new();
    read.missing_com = true;
    read.value.body_spin = 0.75;
    let mut c = Cvars::new();
    assert!(matches!(
        r.try_update_grounded_live(&read, &mut c, &mut HostReckoningNumeric),
        Err(ReckoningError::Input(GroundInputError::Missing("com")))
    ));
    assert_eq!(r.body_spin.state_words().1[0], 0.75f32.to_bits());
    assert_eq!(r.body_spin.state_words().2, 1);
    assert_eq!(*read.events.borrow(), ["dynamic", "heading", "spin", "com"]);
}

#[test]
fn unknown_deck_w_completes_xyz_grounded_work_and_remains_unknown_in_retained_state() {
    let mut read = Read::new();
    read.unknown_w = true;
    let mut r = Reckoning::try_new(Attributes::new()).unwrap();
    let mut c = Cvars::new();
    let mut n = HostReckoningNumeric;
    r.try_update_grounded_live(&read, &mut c, &mut n).unwrap();
    assert!(r.transform_no_flip.columns()[0].w.is_none());
    assert!(r.transform().columns().iter().all(|v| v.w.is_some()));
    assert!(r.heading.w.is_none());
    assert!(r.slow_filter.last_target().w.is_none());
    assert!(r.up().xyz.iter().all(|v| v.is_finite()));
}

#[test]
fn normalization_uses_actual_fallback_knowledge_without_promoting_successful_unknown_w() {
    use crate::reckoning_numeric::try_normalize_safe;
    let mut n = HostReckoningNumeric;
    let fallback = SkeletonRootVector::known([0.0, 1.0, 0.0, 77.0]);
    let degenerate = SkeletonRootVector {
        xyz: [0.0; 3],
        w: None,
    };
    let result = try_normalize_safe(degenerate, fallback, &mut n).unwrap();
    assert_eq!(result.w.map(f32::to_bits), Some(77.0_f32.to_bits()));
    let result = try_normalize_safe(
        SkeletonRootVector {
            xyz: [1.0, 0.0, 0.0],
            w: None,
        },
        fallback,
        &mut n,
    )
    .unwrap();
    assert!(result.w.is_none());
}
