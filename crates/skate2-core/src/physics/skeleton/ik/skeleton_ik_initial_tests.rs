#![expect(
    clippy::float_cmp,
    reason = "exact finite fixture values; numerical discriminators use bit assertions"
)]
use super::*;
use crate::{
    skeleton_data_tests::{build, fixture},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub(crate) fn targets() -> SkeletonIkTargets {
    let (hierarchy, parameters, pose) = fixture();
    let mut result = SkeletonIkTargets::new(Arc::new(build(hierarchy, parameters, &pose).unwrap()));
    result.initialize();
    result
}
pub(crate) fn matrix(position: [f32; 4]) -> AnimationMatrix {
    let mut result = super::super::IDENTITY;
    result[12..16].copy_from_slice(&position);
    result
}
pub(crate) fn data() -> SkeletonIkDataIn {
    let identity = SkeletonRootMatrix::known(matrix([0.0; 4]));
    SkeletonIkDataIn {
        animation_to_world: identity,
        world_to_animation: identity,
        physics_skateboard: identity,
        inverse_skateboard: identity,
        hips: identity,
        skateboard: identity,
        toe_contact_normal: SkeletonRootVector::known([0.0; 4]),
        hips_offset: SkeletonRootVector::known([0.0; 4]),
    }
}
pub(crate) fn state(targets: &mut SkeletonIkTargets) -> SkeletonIkState {
    let mut result = SkeletonIkState::new(targets);
    result.external_blends = [1.0; 4];
    result
}

#[derive(Clone)]
pub(crate) struct Services {
    pub(crate) events: Rc<RefCell<Vec<&'static str>>>,
    pub(crate) fail_at: Option<usize>,
    pub(crate) extent: [f32; 4],
    pub(crate) draw: u8,
    pub(crate) maximum_angle: f32,
    pub(crate) decay: f32,
    pub(crate) local_maximum: f32,
    pub(crate) world_maximum: f32,
}
impl Default for Services {
    fn default() -> Self {
        Self {
            events: Rc::default(),
            fail_at: None,
            extent: [10.0; 4],
            draw: 0,
            maximum_angle: 0.71,
            decay: 0.2,
            local_maximum: 0.25,
            world_maximum: 100.0,
        }
    }
}
impl Services {
    fn event(&mut self, name: &'static str) -> Result<(), &'static str> {
        let mut events = self.events.borrow_mut();
        events.push(name);
        if self.fail_at == Some(events.len()) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl SkeletonIkAttributeRead for Services {
    type Error = &'static str;
    fn scalar(&mut self, _: u64) -> Result<f32, Self::Error> {
        panic!("initial has no scalar attribute")
    }
    fn vector(&mut self, key: u64) -> Result<[f32; 4], Self::Error> {
        assert_eq!(key, DECK_EXTENT_KEY);
        self.event("extent")?;
        Ok(self.extent)
    }
}
impl SkeletonIkInitialCvars for Services {
    type Error = &'static str;
    fn draw_feet_on_deck(&mut self) -> Result<u8, Self::Error> {
        self.event("draw")?;
        Ok(self.draw)
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.event("overlay")
    }
    fn maximum_external_normal_angle(&mut self) -> Result<f32, Self::Error> {
        self.event("angle")?;
        Ok(self.maximum_angle)
    }
    fn target_normal_blend_speed(&mut self) -> Result<f32, Self::Error> {
        self.event("decay")?;
        Ok(self.decay)
    }
    fn maximum_local_delta(&mut self) -> Result<f32, Self::Error> {
        self.event("local")?;
        Ok(self.local_maximum)
    }
    fn maximum_world_offset(&mut self) -> Result<f32, Self::Error> {
        self.event("world")?;
        Ok(self.world_maximum)
    }
}
impl ReckoningNumeric for Services {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.event("dot")?;
        Ok((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event("rsqrt")?;
        Ok(1.0 / value.sqrt())
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event("reciprocal")?;
        Ok(1.0 / value)
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no integer conversion in these stages")
    }
}
impl ReckoningSinCosNumeric for Services {
    type Error = &'static str;
    fn round_nearest_even(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.event("round")?;
        Ok(value.round_ties_even())
    }
}
type Error = SkeletonIkInitialError<&'static str, &'static str, &'static str, &'static str>;
fn run(
    state: &mut SkeletonIkState,
    data: &SkeletonIkDataIn,
    targets: &SkeletonIkTargets,
    rows: &[AnimationMatrix],
    services: &Services,
) -> Result<(), Error> {
    state.try_calculate_initial_transforms(
        data,
        targets,
        rows,
        &mut services.clone(),
        &mut services.clone(),
        &mut services.clone(),
        &mut services.clone(),
    )
}

#[test]
fn actual_previous_record_and_adjusted_targets_produce_distinct_deck_and_world_frames() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes[0] = 1;
    targets.add_offset_transform(&matrix([12.0, 2.0, 3.0, 7.0]));
    let mut rows = [matrix([0.0; 4]); 24];
    rows[0] = matrix([10.0, 0.0, 0.0, 99.0]);
    rows[15] = matrix([20.0, 3.0, 4.0, 88.0]);
    rows[16] = matrix([21.0, 5.0, 7.0, 77.0]);
    let mut data = data();
    data.animation_to_world = SkeletonRootMatrix::known(matrix([100.0, 0.0, 0.0, 6.0]));
    run(&mut state, &data, &targets, &rows, &Services::default()).unwrap();
    assert_eq!(
        state.part_positions[0],
        SkeletonRootVector::known([2.0, 2.0, 3.0, 0.0])
    );
    assert_eq!(
        state.part_to_world[0].columns()[3],
        SkeletonRootVector::known([120.0, 3.0, 4.0, 6.0])
    );
    assert_eq!(
        state.parent_to_deck[0].columns()[3],
        SkeletonRootVector::known([3.0, 4.0, 6.0, 0.0])
    );
    assert_eq!(
        state.parent_to_world[0].columns()[3],
        SkeletonRootVector::known([121.0, 5.0, 7.0, 6.0])
    );
    assert_eq!(state.parts_on_deck, [1, 0, 0, 0]);
    assert_eq!(
        state.parent_to_animation[0],
        SkeletonRootMatrix::known(super::super::IDENTITY)
    );
}

#[test]
fn extent_and_first_draw_fail_before_any_on_deck_clear() {
    for fail in [1, 2] {
        let mut targets = targets();
        let mut state = state(&mut targets);
        state.parts_on_deck = [9; 4];
        let services = Services {
            fail_at: Some(fail),
            ..Services::default()
        };
        assert!(
            run(
                &mut state,
                &data(),
                &targets,
                &[matrix([0.0; 4]); 24],
                &services
            )
            .is_err()
        );
        assert_eq!(state.parts_on_deck, [9; 4]);
        assert_eq!(services.events.borrow().len(), fail);
    }
}

#[test]
fn deck_read_precedes_attribute_service() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    let services = Services::default();
    assert_eq!(
        run(&mut state, &data(), &targets, &[], &services),
        Err(Error::RecordUnavailable(SkeletonIkRecordRead::Deck))
    );
    assert!(services.events.borrow().is_empty());
}

#[test]
fn missing_primary_part_is_late_and_clears_only_current_on_deck_entry() {
    let (hierarchy, parameters, pose) = fixture();
    let mut targets =
        SkeletonIkTargets::new(Arc::new(build(hierarchy, parameters, &pose).unwrap()));
    let mut state = state(&mut targets);
    state.modes[0] = 1;
    state.parts_on_deck = [9; 4];
    let services = Services::default();
    assert_eq!(
        run(
            &mut state,
            &data(),
            &targets,
            &[matrix([0.0; 4]); 24],
            &services
        ),
        Err(Error::MissingPrimaryPart { entry: 0 })
    );
    assert_eq!(state.parts_on_deck, [0, 9, 9, 9]);
    assert_eq!(*services.events.borrow(), ["extent", "draw"]);
}

#[test]
fn per_entry_debug_failure_preserves_current_primary_and_prior_parent() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes = [1; 4];
    state.parts_on_deck = [9; 4];
    let mut rows = [matrix([0.0; 4]); 24];
    rows[15] = matrix([4.0, 0.0, 0.0, 0.0]);
    rows[19] = matrix([8.0, 0.0, 0.0, 0.0]);
    let services = Services {
        draw: 1,
        fail_at: Some(6),
        ..Services::default()
    };
    assert_eq!(
        run(&mut state, &data(), &targets, &rows, &services),
        Err(Error::Cvar("overlay"))
    );
    assert_eq!(
        *services.events.borrow(),
        ["extent", "draw", "draw", "overlay", "draw", "overlay"]
    );
    assert_eq!(state.part_to_world[1].columns()[3].xyz, [8.0, 0.0, 0.0]);
    assert_eq!(state.parts_on_deck, [1, 1, 9, 9]);
    assert_eq!(
        state.parent_to_world[1],
        SkeletonRootMatrix::known(super::super::IDENTITY)
    );
}

#[test]
fn optional_parent_record_failure_occurs_after_primary_debug_and_stores() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes[0] = 1;
    let mut rows = [matrix([0.0; 4]); 16];
    rows[15] = matrix([4.0, 0.0, 0.0, 0.0]);
    let services = Services {
        draw: 1,
        ..Services::default()
    };
    assert_eq!(
        run(&mut state, &data(), &targets, &rows, &services),
        Err(Error::RecordUnavailable(SkeletonIkRecordRead::Parent {
            entry: 0,
            part: 16
        }))
    );
    assert_eq!(state.part_to_world[0].columns()[3].xyz, [4.0, 0.0, 0.0]);
    assert_eq!(state.parts_on_deck[0], 1);
    assert_eq!(
        *services.events.borrow(),
        ["extent", "draw", "draw", "overlay"]
    );
}

#[test]
fn unordered_blends_skip_common_stage_unless_mode_is_on_deck() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.on_deck_blends = [f32::NAN; 4];
    state.external_blends = [f32::NAN; 4];
    state.parts_on_deck = [9; 4];
    let services = Services::default();
    run(
        &mut state,
        &data(),
        &targets,
        &[matrix([0.0; 4])],
        &services,
    )
    .unwrap();
    assert_eq!(state.parts_on_deck, [0; 4]);
    assert_eq!(*services.events.borrow(), ["extent", "draw"]);
    state.modes[2] = 1;
    assert_eq!(
        run(
            &mut state,
            &data(),
            &targets,
            &[matrix([0.0; 4])],
            &Services::default()
        ),
        Err(Error::RecordUnavailable(SkeletonIkRecordRead::Primary {
            entry: 2,
            part: 3
        }))
    );
}

#[test]
fn bounds_are_inclusive_drop_w_and_nan_extent_does_not_make_an_outside_bit() {
    for x in [-2.0, 2.0] {
        let mut targets = targets();
        let mut state = state(&mut targets);
        state.modes[2] = 1;
        targets.add_offset_transform(&matrix([x, 0.0, 0.0, f32::NAN]));
        let services = Services {
            extent: [2.0, f32::NAN, 0.0, f32::NAN],
            ..Services::default()
        };
        run(
            &mut state,
            &data(),
            &targets,
            &[matrix([0.0; 4]); 24],
            &services,
        )
        .unwrap();
        assert_eq!(state.parts_on_deck[2], 1);
    }
}

#[test]
fn partial_world_w_remains_unknown_while_deck_inverse_recovers_w() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes[0] = 1;
    let mut data = data();
    let mut columns = *data.animation_to_world.columns();
    columns[0].w = None;
    data.animation_to_world = SkeletonRootMatrix::from_columns(columns);
    run(
        &mut state,
        &data,
        &targets,
        &[matrix([0.0; 4]); 24],
        &Services::default(),
    )
    .unwrap();
    assert!(
        state.part_to_world[0]
            .columns()
            .iter()
            .all(|v| v.w.is_none())
    );
    assert!(
        state.parent_to_world[0]
            .columns()
            .iter()
            .all(|v| v.w.is_none())
    );
    assert!(state.part_to_deck[0].known_lanes().is_some());
}

#[test]
fn external_failure_precedes_common_gate_and_preserves_on_deck_clear() {
    let mut targets = targets();
    let mut state = state(&mut targets);
    state.modes[0] = 2;
    state.parts_on_deck = [9; 4];
    let services = Services {
        fail_at: Some(3),
        ..Services::default()
    };
    assert_eq!(
        run(
            &mut state,
            &data(),
            &targets,
            &[matrix([0.0; 4]); 24],
            &services
        ),
        Err(Error::External(SkeletonIkExternalError::Cvar("world")))
    );
    assert_eq!(state.parts_on_deck, [0, 9, 9, 9]);
    assert_eq!(*services.events.borrow(), ["extent", "draw", "world"]);
}
