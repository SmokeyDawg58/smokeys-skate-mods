//! `SetTurning1413`: Create82cb27b8, Begin82cb2798, Update82cc8408.
use crate::play_animation::FastString30;
#[path = "motion_set_turning_reshape.rs"]
pub(crate) mod reshape;
#[cfg(test)]
#[path = "motion_set_turning_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TurningInstance {
    pub(crate) time: f32,
    pub(crate) value: f32,
    pub(crate) state: i32,
}
impl TurningInstance {
    pub(crate) fn try_create<E>(
        mut allocate: impl FnMut(usize, usize) -> Result<bool, E>,
    ) -> Result<Option<Self>, E> {
        Ok(allocate(0x14, 0x10)?.then_some(Self {
            time: 0.0,
            value: 0.0,
            state: 2,
        }))
    }
    pub(crate) fn begin(&mut self) {
        self.state = 2;
        self.time = 0.0;
        self.value = 0.0;
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Names {
    pub(crate) angle: FastString30,
    pub(crate) direction: FastString30,
    pub(crate) quickness: FastString30,
    pub(crate) holding: FastString30,
    pub(crate) speed: FastString30,
}
#[derive(Clone, Copy)]
pub(crate) enum Output {
    TargetLean,
    Turn,
    Direction,
    Quickness,
    Holding,
}
pub(crate) trait TurningSource: reshape::ReshapeSource {
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error>;
    fn riding_fakie(&mut self) -> Result<bool, Self::Error>;
    fn capture_intents(&mut self) -> Result<(), Self::Error>;
    fn output(&mut self, field: Output) -> Result<f32, Self::Error>;
    fn missing_instance(&mut self) -> Self::Error;
    fn slide_flags(&mut self, side: usize, required: u32) -> Result<u32, Self::Error>;
    fn clear_force(&mut self, side: usize) -> Result<(), Self::Error>;
    fn combined(&mut self, required: u32) -> Result<u32, Self::Error>;
    fn reset_slide_time(&mut self, side: usize) -> Result<(), Self::Error>;
    fn set_slide_fakie(&mut self, fakie: bool) -> Result<(), Self::Error>;
    fn mirrored(&mut self) -> Result<bool, Self::Error>;
    fn debug(&mut self) -> Result<(), Self::Error>;
    fn dt(&mut self) -> Result<f32, Self::Error>;
    fn speed(&mut self) -> Result<f32, Self::Error>;
    fn settable(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error>;
    fn slide_turn(&mut self) -> Result<f32, Self::Error>;
    fn intent(&mut self, name: &str) -> Result<Option<f32>, Self::Error>;
    fn motion_attribute(&mut self, name: &str, value: f32) -> Result<(), Self::Error>;
}
#[expect(
    clippy::assign_op_pattern,
    reason = "native elapsed-time addition uses dt as left operand"
)]
pub(crate) fn update<S: TurningSource>(
    source: &mut S,
    instance: Option<&mut TurningInstance>,
    names: Names,
) -> Result<(), S::Error> {
    source.resolve_interfaces()?;
    let fakie = source.riding_fakie()?;
    source.capture_intents()?;
    let target_lean = source.output(Output::TargetLean)?;
    let turn = source.output(Output::Turn)?;
    let direction = source.output(Output::Direction)?;
    let quickness = source.output(Output::Quickness)?;
    let holding = source.output(Output::Holding)?;
    let quick = reshape::reshape(source, [turn, direction], false)?;
    let _ = reshape::reshape(source, [target_lean, direction], false)?;
    let slow = reshape::reshape(source, [turn, direction], true)?;
    let _ = reshape::reshape(source, [target_lean, direction], true)?;
    let blended = std::array::from_fn::<_, 2, _>(|i| {
        let value = quick[i].mul_add(quickness, slow[i] * (1.0 - quickness));
        f32::from_bits(value.to_bits() ^ 0x8000_0000)
    });
    let instance = instance.ok_or_else(|| source.missing_instance())?;
    let old_state = instance.state;
    // Native captures both flag words before clearing either force bit.
    let left = source.slide_flags(0, 0x2000_0000)?;
    let right = source.slide_flags(1, 0x2000_0000)?;
    source.clear_force(0)?;
    source.clear_force(1)?;
    let mut state = old_state;
    if state == 2 {
        let enabled = source.combined(0x4000_0000)? & 0x4000_0000 != 0;
        let right_start = enabled
            && (source.slide_flags(1, 0x8000_0000)? & 0x8000_0000 != 0 || right & 0x2000_0000 != 0);
        let left_start = enabled
            && (source.slide_flags(0, 0x8000_0000)? & 0x8000_0000 != 0 || left & 0x2000_0000 != 0);
        if right_start || left_start {
            if (left | right) & 0x2000_0000 == 0 {
                source.reset_slide_time(0)?;
                source.reset_slide_time(1)?;
                source.set_slide_fakie(fakie)?;
            }
            state = i32::from(right_start);
        }
        source.debug()?;
    } else if (state == 1 || state == 0)
        && (source.combined(0x4000_0000)? & 0x4000_0000 == 0
            || source.slide_flags(usize::from(state == 1), 0x4000_0000)? & 0x4000_0000 != 0)
    {
        state = 2;
        source.debug()?;
    }
    let mut target = blended[0];
    if state == 0 || state == 1 {
        let mirrored = source.mirrored()?;
        target = if (state == 1) == mirrored { -1.0 } else { 1.0 };
        if source.combined(0x8000_0000)? & 0x8000_0000 != 0 {
            target = -target;
        }
    }
    if state != old_state {
        instance.time = 0.0;
    }
    instance.state = state;
    let curve = source.graph(0x290, instance.time)?;
    let value = smooth(curve, target, instance.value);
    let maximum_step = source.scalar(0xd35c_a989_8b86_827a)?;
    let lower = instance.value - maximum_step;
    let upper = instance.value + maximum_step;
    let value = reshape::select(lower - value, lower, value);
    let dt = source.dt()?;
    instance.time = dt + instance.time;
    instance.value = reshape::select(upper - value, value, upper);
    let speed = source.speed()?.abs();
    let threshold = source.scalar(0x5339_323f_7417_10db)?;
    let tuck = if speed > threshold {
        source.graph(0x240, speed - threshold)?
    } else {
        0.0
    };
    if std::env::var_os("SKATE2_TRACE_SPEED").is_some() && speed > 8.0 {
        eprintln!("speed tuck: z_speed={speed} threshold={threshold} speedtuck={tuck}");
    }
    source.settable(names.angle, instance.value)?;
    crate::diagnostics::lean_trace::record(
        "turning",
        std::ptr::from_ref(instance) as usize,
        0,
        &[
            target_lean,
            turn,
            direction,
            quickness,
            holding,
            blended[0],
            blended[1],
            target,
            instance.value,
            instance.time,
            state as f32,
            old_state as f32,
            speed,
            tuck,
            f32::from(u8::from(fakie)),
            maximum_step,
        ],
    );
    source.settable(names.direction, blended[1])?;
    source.settable(names.quickness, quickness)?;
    source.settable(names.speed, tuck)?;
    source.settable(names.holding, holding)?;
    let turn = source.slide_turn()?; // unconditionally fetched, including state2
    if state == 1 || state == 0 {
        let fakie = source.combined(0x8000_0000)? & 0x8000_0000 != 0;
        let turn = if (state == 1) == fakie { -turn } else { turn };
        source.motion_attribute("turn", turn)?;
        let slide = source
            .intent(if state == 1 {
                "RightSlide"
            } else {
                "LeftSlide"
            })?
            .unwrap_or(0.0);
        let fakie = source.combined(0x8000_0000)? & 0x8000_0000 != 0;
        source.motion_attribute("slide", adjust_slide(slide, fakie))?;
    } else if let Some(turn) = source.intent("fakieturn")? {
        source.motion_attribute("turn", turn)?;
    }
    source.debug()
}
/// `SlidingState::AdjustSlideForFakie82c8a5c0`; no clamp or modulo.
pub(crate) fn adjust_slide(value: f32, fakie: bool) -> f32 {
    if !fakie {
        return value;
    }
    let negative = -value;
    let magnitude = negative.abs();
    let magnitude = if magnitude > 0.5 {
        1.0 - magnitude
    } else {
        magnitude
    };
    reshape::select(negative, magnitude, -magnitude)
}
fn smooth(curve: f32, target: f32, previous: f32) -> f32 {
    // 8b40 rounds curve*target, 8b50 fuses (1-curve)*previous + that product.
    (1.0 - curve).mul_add(previous, curve * target)
}
