//! Animation.Update third callback82e2b470. Child of `conditioners::animation`.
use super::{Animation, AnimationAttributes, ScalarPid};
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    physics::PointGraph,
    reckoning_pid::PhysicsControllerData,
};

// Full original u64 attribute keys, recovered from insrdi argument assembly.
pub(crate) const LEAN_ANGLE_SCALE: u64 = 0xb061_790e_1040_392c;
pub(crate) const SPEED_SCALE: u64 = 0x7525_f3b3_cd5d_bea8;
pub(crate) const WOBBLE_LEAN_SCALE: u64 = 0x8b4f_1a08_951d_c1d5;
pub(crate) const ACTUAL_LEAN_BLEND: u64 = 0x198a_fc5b_8490_cfc4;
pub(crate) const DIRECTION_SCALE: u64 = 0x9659_dfa3_e06d_48c5;
pub(crate) const QUICKNESS_MAX_DELTA: u64 = 0x742d_5536_361a_5cd1;
pub(crate) const QUICKNESS_BLEND: u64 = 0xc59c_a518_c35f_0a0d;
pub(crate) const QUICKNESS_SCALE: u64 = 0x8f78_3a5a_b642_3a2d;
pub(crate) const HOLDING_BLEND: u64 = 0x8cda_9e8d_6d31_f331;
pub(crate) const HOLDING_THRESHOLD: u64 = 0xf87f_5225_afc9_b077;
pub(crate) const HOLDING_ATTACK: u64 = 0x0af8_487d_6c70_4c52;
pub(crate) const HOLDING_RELEASE: u64 = 0x725b_ba72_4929_7352;
pub(crate) const HOLDING_MAX_DELTA: u64 = 0x3850_e0aa_166a_0e17;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TurnField {
    Turn,
    TargetLean,
    InputLean,
    Quickness,
    Direction,
    Holding,
    CameraTurn,
    CameraInputLean,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TurnFilter {
    SlowLean,
    Lean,
    Direction,
}

/// Each output read/store is a separate native checked accessor. Attributes
/// delegate to the retained global `physics_animation` instance below.
pub(crate) trait AnimationTurnSource {
    type Error;
    type Scalar;
    fn mirrored(&mut self) -> Result<bool, Self::Error>;
    fn deck_speed(&mut self) -> Result<f32, Self::Error>;
    fn target_lean(&mut self) -> Result<f32, Self::Error>;
    fn ground_turn(&mut self) -> Result<f32, Self::Error>;
    fn wobble(&mut self) -> Result<f32, Self::Error>;
    fn carving(&mut self) -> Result<bool, Self::Error>;
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn capture_scalar(&mut self, key: u64) -> Result<Self::Scalar, Self::Error>;
    fn read_scalar(&mut self, value: &Self::Scalar) -> Result<f32, Self::Error>;
    fn control(&mut self, filter: TurnFilter) -> Result<PhysicsControllerData, Self::Error>;
    fn graph(&mut self, offset: usize, points: usize, value: f32) -> Result<f32, Self::Error>;
    fn store(&mut self, field: TurnField, value: f32) -> Result<(), Self::Error>;
    fn draw_carve_enabled(&mut self) -> Result<bool, Self::Error>;
    /// All three final draw calls are BLR; actual singleton is still required.
    fn require_carve_overlay(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub(crate) enum AnimationTurnError<E> {
    Source(E),
    MissingFilter(TurnFilter),
}

#[derive(Debug)]
pub(crate) enum AnimationTurnLayoutError {
    Attribute(AttributeRuntimeError),
    MissingLayout,
}

impl Animation {
    #[expect(
        clippy::too_many_lines,
        reason = "source-ordered three filter updates and interleaved retained history writes"
    )]
    pub(crate) fn try_update_animation_turn<S: AnimationTurnSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), AnimationTurnError<S::Error>> {
        use AnimationTurnError::{MissingFilter, Source};
        let mirror = if source.mirrored().map_err(Source)? {
            -1.0
        } else {
            1.0
        };
        let speed = source.deck_speed().map_err(Source)?;
        let lean_scale = source.scalar(LEAN_ANGLE_SCALE).map_err(Source)?;
        let lean = source.target_lean().map_err(Source)?;
        let normalized_lean = signed_unit((lean / lean_scale) * mirror);
        let speed_scale = source.scalar(SPEED_SCALE).map_err(Source)?;
        let normalized_speed = unit(speed / speed_scale);
        let turn = source.ground_turn().map_err(Source)? * mirror;
        let turn_curve = source.graph(0xd0, 8, normalized_speed).map_err(Source)?;
        let input_lean = turn_curve * turn;
        let wobble_scale = source.scalar(WOBBLE_LEAN_SCALE).map_err(Source)?;
        let wobble = source.wobble().map_err(Source)?;
        // fnmsubs negates the rounded fused result; retain signed zero.
        let wobble_weight = -wobble.mul_add(1.0 - wobble_scale, -1.0);
        let actual_blend = source.scalar(ACTUAL_LEAN_BLEND).map_err(Source)?;
        let weighted_actual = (actual_blend * wobble_weight) * normalized_lean;
        let target = (1.0 - actual_blend).mul_add(input_lean, weighted_actual);

        let slow_control = source.control(TurnFilter::SlowLean).map_err(Source)?;
        let slow = self
            .slow_lean_filter
            .as_mut()
            .ok_or(MissingFilter(TurnFilter::SlowLean))?;
        slow.control = slow_control;
        let slow_lean = dead_zone(update_pid(slow, target, false));
        let lean_control = source.control(TurnFilter::Lean).map_err(Source)?;
        let fast = self
            .lean_filter
            .as_mut()
            .ok_or(MissingFilter(TurnFilter::Lean))?;
        fast.control = lean_control;
        let fast_lean = dead_zone(update_pid(fast, target, false));
        let direction_control = source.control(TurnFilter::Direction).map_err(Source)?;
        let direction = self
            .direction_filter
            .as_mut()
            .ok_or(MissingFilter(TurnFilter::Direction))?;
        direction.control = direction_control;
        // This lookup occurs AFTER direction control copies, before its histories.
        let direction_scale = source.scalar(DIRECTION_SCALE).map_err(Source)?;
        let direction_value = update_pid(direction, (target - fast_lean) / direction_scale, true);

        let anim_lean = if source.carving().map_err(Source)? {
            turn
        } else {
            0.0
        };
        let anim_delta = anim_lean - self.last_animation_lean;
        self.last_animation_lean = anim_lean;
        let quickness_delta = source.scalar(QUICKNESS_MAX_DELTA).map_err(Source)?;
        let quickness_blend = source.scalar(QUICKNESS_BLEND).map_err(Source)?;
        self.input_quickness = (1.0 - quickness_blend)
            .mul_add(self.input_quickness, quickness_blend * anim_delta.abs());
        let quickness_scale = source.scalar(QUICKNESS_SCALE).map_err(Source)?;
        let q = self.input_quickness / quickness_scale;
        let squared = q * q;
        let capped = select(squared - 1.0, 1.0, squared);
        let quickness = limit_delta(capped, self.last_normalized_quickness, quickness_delta);
        self.last_normalized_quickness = quickness;
        let speed_quickness = source.graph(0x10, 8, speed).map_err(Source)?;
        let quickness_curve = source.graph(0x90, 8, quickness).map_err(Source)?;
        let output_quickness = unit(quickness_curve) * speed_quickness;

        let actual_lean = if source.carving().map_err(Source)? {
            fast_lean
        } else {
            0.0
        };
        let old_actual_lean = self.last_actual_lean;
        self.last_actual_lean = actual_lean;
        let actual_delta = (actual_lean - old_actual_lean).abs();
        let holding_blend = source.scalar(HOLDING_BLEND).map_err(Source)?;
        self.holding_integral =
            (1.0 - holding_blend).mul_add(self.holding_integral, holding_blend * actual_delta);
        let threshold = source.scalar(HOLDING_THRESHOLD).map_err(Source)?;
        // Raw4098000c BO4/BI24 branches when LT is clear, including unordered.
        let holding: f32 = if self.holding_integral < threshold {
            1.0
        } else {
            0.0
        };
        let attack = source.capture_scalar(HOLDING_ATTACK).map_err(Source)?;
        let release = source.capture_scalar(HOLDING_RELEASE).map_err(Source)?;
        let attack = source.read_scalar(&attack).map_err(Source)?;
        let release = source.read_scalar(&release).map_err(Source)?;
        let holding_rate = (1.0 - holding).mul_add(release, attack * holding);
        let desired_holding =
            (1.0 - holding_rate).mul_add(self.last_holding, holding_rate * holding);
        let desired_delta = desired_holding - self.last_holding;
        let max_delta = source.scalar(HOLDING_MAX_DELTA).map_err(Source)?;
        let delta = limit_delta(desired_delta, self.last_holding_delta, max_delta);
        self.last_holding_delta = delta;
        self.last_holding += delta;
        self.last_holding = unit(self.last_holding);
        let wobble = source.wobble().map_err(Source)?;
        let output_blend = source.graph(0x120, 4, wobble).map_err(Source)?;
        self.last_output_lean =
            (1.0 - output_blend).mul_add(self.last_output_lean, output_blend * slow_lean);
        let output_turn = signed_unit(self.last_output_lean);
        let target_lean = signed_unit(target);
        let output_holding = unit(self.last_holding);
        let output_direction = signed_unit(direction_value);
        // Eight separate guard/store boundaries: preceding writes survive failure.
        source.store(TurnField::Turn, output_turn).map_err(Source)?;
        source
            .store(TurnField::TargetLean, target_lean)
            .map_err(Source)?;
        source
            .store(TurnField::InputLean, input_lean)
            .map_err(Source)?;
        source
            .store(TurnField::Quickness, output_quickness)
            .map_err(Source)?;
        source
            .store(TurnField::Direction, output_direction)
            .map_err(Source)?;
        source
            .store(TurnField::Holding, output_holding)
            .map_err(Source)?;
        source
            .store(TurnField::CameraTurn, output_turn * mirror)
            .map_err(Source)?;
        source
            .store(TurnField::CameraInputLean, input_lean * mirror)
            .map_err(Source)?;
        if source.draw_carve_enabled().map_err(Source)? {
            source.require_carve_overlay().map_err(Source)?;
        }
        Ok(())
    }
}

fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}
fn signed_unit(value: f32) -> f32 {
    let low = select(-1.0 - value, -1.0, value);
    select(1.0 - low, low, 1.0)
}
fn unit(value: f32) -> f32 {
    let low = select(-value, 0.0, value);
    select(1.0 - low, low, 1.0)
}
fn limit_delta(value: f32, previous: f32, delta: f32) -> f32 {
    let lower = previous - delta;
    let upper = previous + delta;
    let low = select(lower - value, lower, value);
    select(upper - low, low, upper)
}
fn dead_zone(value: f32) -> f32 {
    // Raw4099000c BO4/BI25 branches when GT is clear, including unordered.
    if value.abs() > f32::from_bits(0x3c23_d70a) {
        value
    } else {
        0.0
    }
}
fn update_pid(filter: &mut ScalarPid, target: f32, direction_order: bool) -> f32 {
    let control = &filter.control;
    let old_value = filter.value;
    let error = target - old_value;
    let delta = target - filter.last_target;
    let inverse_blend = 1.0 - control.integral_blend;
    let integral_tail = inverse_blend * filter.integral;
    let derivative_tail = inverse_blend * filter.derivative_input;
    filter.last_target = target;
    let integral = control.integral_blend.mul_add(error, integral_tail);
    let derivative_input = delta.mul_add(control.integral_blend, derivative_tail);
    if direction_order {
        filter.derivative_input = derivative_input;
        filter.integral = integral;
    } else {
        filter.integral = integral;
        filter.derivative_input = derivative_input;
    }
    let derivative = derivative_input - filter.derivative;
    let correction = derivative.mul_add(control.derivative, control.integral * integral);
    let change = control.proportional.mul_add(error, correction);
    filter.value = change + old_value;
    filter.derivative = filter.value - old_value;
    filter.value
}

impl AnimationAttributes {
    pub(crate) fn turn_capture_scalar(
        &self,
        key: u64,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        self.global.try_get_data(key, 0)
    }
    pub(crate) fn turn_read_scalar(
        value: Option<&AttributeRuntimeData>,
    ) -> Result<f32, AttributeRuntimeError> {
        let Some(value) = value else {
            return Ok(0.0);
        };
        let bytes = value.bytes();
        Ok(f32::from_be_bytes(bytes.try_into().map_err(|_| {
            AttributeRuntimeError::ScalarSize { size: bytes.len() }
        })?))
    }
    pub(crate) fn turn_scalar(&self, key: u64) -> Result<f32, AttributeRuntimeError> {
        Self::turn_read_scalar(self.turn_capture_scalar(key)?.as_ref())
    }
    pub(crate) fn turn_control(
        &self,
        filter: TurnFilter,
    ) -> Result<PhysicsControllerData, AnimationTurnLayoutError> {
        let layout = self
            .global
            .try_cached_layout(0x220)
            .map_err(AnimationTurnLayoutError::Attribute)?
            .ok_or(AnimationTurnLayoutError::MissingLayout)?;
        let offset = match filter {
            TurnFilter::SlowLean => 0x1f0,
            TurnFilter::Lean => 0x200,
            TurnFilter::Direction => 0x210,
        };
        let word = |index| {
            f32::from_be_bytes(
                layout.bytes()[index..index + 4]
                    .try_into()
                    .expect("source control offset"),
            )
        };
        Ok(PhysicsControllerData {
            proportional: word(offset),
            integral: word(offset + 4),
            derivative: word(offset + 8),
            integral_blend: word(offset + 12),
        })
    }
    pub(crate) fn turn_graph<const N: usize>(
        &self,
        offset: usize,
        value: f32,
    ) -> Result<f32, AnimationTurnLayoutError> {
        let layout = self
            .global
            .try_cached_layout(0x220)
            .map_err(AnimationTurnLayoutError::Attribute)?
            .ok_or(AnimationTurnLayoutError::MissingLayout)?;
        let word = |index: usize| {
            f32::from_be_bytes(
                layout.bytes()[index..index + 4]
                    .try_into()
                    .expect("source graph offset"),
            )
        };
        let graph = PointGraph {
            x: std::array::from_fn::<_, N, _>(|i| word(offset + i * 4)),
            y: std::array::from_fn::<_, N, _>(|i| word(offset + (N + i) * 4)),
        };
        Ok(graph.evaluate(value))
    }
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "exact retained stores, FSEL arms and two-tick fixture outputs"
)]
mod tests {
    use super::*;

    fn control() -> PhysicsControllerData {
        PhysicsControllerData {
            proportional: 1.0,
            integral: 0.0,
            derivative: 0.0,
            integral_blend: 1.0,
        }
    }
    fn retained() -> Animation {
        let mut animation = Animation::before_filters();
        animation.slow_lean_filter = Some(ScalarPid::new(control()));
        animation.lean_filter = Some(ScalarPid::new(control()));
        animation.direction_filter = Some(ScalarPid::new(control()));
        animation
    }
    struct Source {
        events: Vec<String>,
        stores: Vec<(TurnField, f32)>,
        carving_reads: usize,
        fail: Option<TurnField>,
        threshold: f32,
    }
    impl Source {
        fn new() -> Self {
            Self {
                events: Vec::new(),
                stores: Vec::new(),
                carving_reads: 0,
                fail: None,
                threshold: 0.1,
            }
        }
        fn event(&mut self, value: impl Into<String>) {
            self.events.push(value.into());
        }
    }
    impl AnimationTurnSource for Source {
        type Error = TurnField;
        type Scalar = u64;
        fn mirrored(&mut self) -> Result<bool, Self::Error> {
            self.event("mirrored");
            Ok(false)
        }
        fn deck_speed(&mut self) -> Result<f32, Self::Error> {
            self.event("speed");
            Ok(2.0)
        }
        fn target_lean(&mut self) -> Result<f32, Self::Error> {
            self.event("lean");
            Ok(0.6)
        }
        fn ground_turn(&mut self) -> Result<f32, Self::Error> {
            self.event("turn");
            Ok(0.4)
        }
        fn wobble(&mut self) -> Result<f32, Self::Error> {
            self.event("wobble");
            Ok(0.0)
        }
        fn carving(&mut self) -> Result<bool, Self::Error> {
            self.event("carving");
            self.carving_reads += 1;
            Ok(self.carving_reads % 2 == 1)
        }
        fn scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
            self.event(format!("scalar:{key:016x}"));
            Ok(match key {
                ACTUAL_LEAN_BLEND | QUICKNESS_BLEND | HOLDING_BLEND => 0.5,
                HOLDING_THRESHOLD => self.threshold,
                _ => 1.0,
            })
        }
        fn capture_scalar(&mut self, key: u64) -> Result<u64, Self::Error> {
            self.event(format!("capture:{key:016x}"));
            Ok(key)
        }
        fn read_scalar(&mut self, key: &u64) -> Result<f32, Self::Error> {
            self.event(format!("read:{key:016x}"));
            Ok(if *key == HOLDING_ATTACK { 0.5 } else { 0.25 })
        }
        fn control(&mut self, filter: TurnFilter) -> Result<PhysicsControllerData, Self::Error> {
            self.event(format!("control:{filter:?}"));
            Ok(control())
        }
        fn graph(&mut self, offset: usize, _: usize, _: f32) -> Result<f32, Self::Error> {
            self.event(format!("graph:{offset:x}"));
            Ok(match offset {
                0xd0 | 0x90 => 0.5,
                0x10 => 0.8,
                0x120 => 1.0,
                _ => unreachable!(),
            })
        }
        fn store(&mut self, field: TurnField, value: f32) -> Result<(), Self::Error> {
            self.event(format!("store:{field:?}"));
            if self.fail == Some(field) {
                return Err(field);
            }
            self.stores.push((field, value));
            Ok(())
        }
        fn draw_carve_enabled(&mut self) -> Result<bool, Self::Error> {
            self.event("draw");
            Ok(true)
        }
        fn require_carve_overlay(&mut self) -> Result<(), Self::Error> {
            self.event("overlay");
            Ok(())
        }
    }

    #[test]
    fn live_read_order_two_carving_samples_and_second_tick_history() {
        let mut animation = retained();
        let mut source = Source::new();
        animation.try_update_animation_turn(&mut source).unwrap();
        assert_eq!(source.carving_reads, 2);
        let expected = [
            (TurnField::Turn, 0.4),
            (TurnField::TargetLean, 0.4),
            (TurnField::InputLean, 0.2),
            (TurnField::Quickness, 0.4),
            (TurnField::Direction, 0.0),
            (TurnField::Holding, 0.5),
            (TurnField::CameraTurn, 0.4),
            (TurnField::CameraInputLean, 0.2),
        ];
        assert_eq!(source.stores, expected);
        assert_eq!(animation.last_animation_lean, 0.4);
        assert_eq!(animation.last_actual_lean, 0.0);
        let capture = source
            .events
            .iter()
            .position(|s| s == &format!("capture:{HOLDING_ATTACK:016x}"))
            .unwrap();
        assert_eq!(
            &source.events[capture..capture + 4],
            [
                format!("capture:{HOLDING_ATTACK:016x}"),
                format!("capture:{HOLDING_RELEASE:016x}"),
                format!("read:{HOLDING_ATTACK:016x}"),
                format!("read:{HOLDING_RELEASE:016x}"),
            ]
        );
        assert_eq!(
            &source.events[source.events.len() - 2..],
            ["draw", "overlay"]
        );
        animation.try_update_animation_turn(&mut source).unwrap();
        assert_eq!(animation.input_quickness, 0.1);
        assert_eq!(animation.last_holding, 0.75);
    }

    #[test]
    fn failed_store_keeps_preceding_stores_and_all_filter_histories() {
        let mut animation = retained();
        let mut source = Source::new();
        source.fail = Some(TurnField::InputLean);
        assert!(matches!(
            animation.try_update_animation_turn(&mut source),
            Err(AnimationTurnError::Source(TurnField::InputLean))
        ));
        assert_eq!(source.stores.len(), 2);
        assert_eq!(animation.last_output_lean, 0.4);
        assert_eq!(animation.slow_lean_filter.unwrap().value, 0.4);
        assert!(!source.events.iter().any(|s| s == "draw"));
    }

    #[test]
    fn raw_cr_branches_and_fsel_have_distinct_nan_and_zero_results() {
        assert_eq!(dead_zone(f32::NAN).to_bits(), 0);
        assert_eq!(dead_zone(f32::from_bits(0x3c23_d70a)).to_bits(), 0);
        assert!(dead_zone(f32::from_bits(0x3c23_d70b)) > 0.0);
        assert_eq!(signed_unit(f32::NAN), 1.0);
        assert_eq!(unit(-0.0).to_bits(), 0);
        assert_eq!(limit_delta(f32::NAN, 0.5, 0.1), 0.6);
        let mut animation = retained();
        let mut source = Source::new();
        source.threshold = f32::NAN;
        animation.try_update_animation_turn(&mut source).unwrap();
        assert_eq!(animation.last_holding, 0.0);
    }
}
