//! `SettingBodyTilt` Create82caf1e0 and Update82c9e288.
use crate::play_animation::FastString30;

#[derive(Debug)]
pub(crate) struct TiltInstance {
    pub(crate) angle: f32,
    pub(crate) delta: f32,
    pub(crate) was_applying: bool,
}
impl TiltInstance {
    /// The caller performs the native allocation request (size14, alignment10).
    pub(crate) fn after_allocation(allocated: bool) -> Option<Self> {
        allocated.then_some(Self {
            angle: 0.0,
            delta: 0.0,
            was_applying: false,
        })
    }
}

pub(crate) trait TiltSource {
    type Error;
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error>;
    fn applying(&mut self) -> Result<bool, Self::Error>;
    fn missing_instance(&mut self) -> Self::Error;
    fn capture_animation_output(&mut self) -> Result<(), Self::Error>;
    fn mirrored(&mut self) -> Result<bool, Self::Error>;
    fn animation_tilt_x(&mut self) -> Result<f32, Self::Error>;
    fn body_spin_speed(&mut self) -> Result<f32, Self::Error>;
    fn evaluate_spin_graph(&mut self, speed: f32) -> Result<f32, Self::Error>;
    fn filtered_is_air(&mut self) -> Result<bool, Self::Error>;
    fn clamp_acceleration(&mut self, air: bool) -> Result<f32, Self::Error>;
    fn clamp_velocity(&mut self, air: bool) -> Result<f32, Self::Error>;
    fn use_tilt(&mut self) -> Result<bool, Self::Error>;
    fn set_tilt(&mut self) -> Result<bool, Self::Error>;
    fn override_x(&mut self) -> Result<f32, Self::Error>;
    fn publish(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error>;
}

// Native fsel takes its negative arm for unordered; ±0 take nonnegative.
fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}
fn limited(value: f32, bound: f32) -> f32 {
    let low = -bound;
    let lower = select(low - value, low, value);
    select(bound - lower, lower, bound)
}
#[expect(
    clippy::assign_op_pattern,
    reason = "native delta addition preserves acceleration as left operand"
)]
pub(crate) fn update<S: TiltSource>(
    source: &mut S,
    instance: Option<&mut TiltInstance>,
    name: FastString30,
) -> Result<(), S::Error> {
    source.resolve_interfaces()?;
    let applying = source.applying()?;
    let state = instance.ok_or_else(|| source.missing_instance())?;
    if !state.was_applying && applying {
        state.angle = 0.0;
        state.delta = 0.0;
    }
    state.was_applying = applying;
    if !applying {
        return Ok(());
    }
    source.capture_animation_output()?;
    let mirrored = source.mirrored()?;
    let x = source.animation_tilt_x()?;
    let signed_x = if mirrored {
        x
    } else {
        f32::from_bits(x.to_bits() ^ 0x8000_0000)
    };
    let spin = source.body_spin_speed()?.abs();
    let target = source.evaluate_spin_graph(spin)? * signed_x;
    let air = source.filtered_is_air()?;
    let acc = source.clamp_acceleration(air)?;
    let vel = source.clamp_velocity(air)?;
    let difference = limited(target - state.angle, vel);
    let acceleration = limited(difference - state.delta, acc);
    state.delta = acceleration + state.delta;
    state.angle += state.delta;
    let value = if source.use_tilt()? {
        if source.set_tilt()? {
            source.override_x()?
        } else {
            state.angle
        }
    } else {
        0.0
    };
    crate::diagnostics::lean_trace::record(
        "body_tilt",
        std::ptr::from_ref(state) as usize,
        0,
        &[
            x,
            signed_x,
            spin,
            target,
            acc,
            vel,
            state.angle,
            state.delta,
            value,
            f32::from(u8::from(mirrored)),
            f32::from(u8::from(air)),
        ],
    );
    source.publish(name, value)
}

#[cfg(test)]
#[path = "motion_setting_body_tilt_tests.rs"]
mod tests;
