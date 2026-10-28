//! Wipeout `CreateInstance82cb0098`, Begin82cafe80 and Update82cc5c88.
use crate::play_animation::FastString30;
#[cfg(test)]
#[path = "motion_wipeout_tests.rs"]
mod tests;

#[derive(Debug)]
pub(crate) struct WipeoutInstance {
    pub(crate) updates: u32,
    pub(crate) lean: f32,
    pub(crate) twist: f32,
    pub(crate) delta_lean: f32,
    pub(crate) delta_twist: f32,
    pub(crate) gesture: [f32; 2],
}
impl WipeoutInstance {
    /// Actual allocation24/alignment10 initializes every payload field to zero.
    pub(crate) fn after_allocation(allocated: bool) -> Option<Self> {
        allocated.then_some(Self {
            updates: 0,
            lean: 0.0,
            twist: 0.0,
            delta_lean: 0.0,
            delta_twist: 0.0,
            gesture: [0.0; 2],
        })
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Names {
    pub(crate) lean: FastString30,
    pub(crate) twist: FastString30,
    pub(crate) gesture_x: FastString30,
    pub(crate) gesture_y: FastString30,
}
pub(crate) trait Source {
    type Error;
    fn missing_instance(&mut self) -> Self::Error;
    fn resolve(&mut self, update: bool) -> Result<(), Self::Error>;
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn lean(&mut self) -> Result<f32, Self::Error>;
    fn twist(&mut self) -> Result<f32, Self::Error>;
    fn mirrored(&mut self) -> Result<bool, Self::Error>;
    fn settable(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error>;
    fn capture_intents(&mut self) -> Result<(), Self::Error>;
    fn gesture(&mut self, x: bool) -> Result<Option<f32>, Self::Error>;
    fn controlled(&mut self) -> Result<(), Self::Error>;
    fn publish_gesture(&mut self, value: [f32; 2]) -> Result<(), Self::Error>;
}
fn select(test: f32, yes: f32, no: f32) -> f32 {
    if test >= 0.0 { yes } else { no }
}
fn clamp_delta(value: f32, maximum: f32) -> f32 {
    let negative = -maximum;
    let lower = select(negative - value, negative, value);
    select(maximum - lower, lower, maximum)
}
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
const PI: f32 = f32::from_bits(0x4049_0fdb);
pub(crate) fn begin<S: Source>(
    s: &mut S,
    instance: Option<&mut WipeoutInstance>,
) -> Result<(), S::Error> {
    let data = instance.ok_or_else(|| s.missing_instance())?;
    data.updates = 0;
    s.resolve(false)?;
    let lean = s.lean()?;
    let mut twist = s.twist()?;
    if s.mirrored()? {
        twist = -twist;
    }
    data.lean = lean;
    let wrapped = twist + TWO_PI;
    data.delta_twist = 0.0;
    data.gesture = [0.0; 2];
    data.twist = select(twist, twist, wrapped);
    Ok(())
}
#[expect(
    clippy::assign_op_pattern,
    reason = "retain source clamp-result plus previous-lean operand order"
)]
pub(crate) fn update<S: Source>(
    s: &mut S,
    instance: Option<&mut WipeoutInstance>,
    names: Names,
) -> Result<(), S::Error> {
    s.resolve(true)?;
    let lean_blend = s.scalar(0xe7cc_6d49_4e1e_cc67)?;
    let twist_blend = s.scalar(0x5b35_2375_1e22_f6f0)?;
    let lean_maximum = s.scalar(0x1e1a_632b_229d_ae28)?;
    let twist_maximum = s.scalar(0x2c35_05e3_7b8a_f858)?;
    let twist_acceleration = s.scalar(0x6a57_e32d_b7a5_261b)?;
    let lean = s.lean()?;
    let mut twist = s.twist()?;
    if s.mirrored()? {
        twist = -twist;
    }
    let weighted_lean = lean * lean_blend;
    let data = instance.ok_or_else(|| s.missing_instance())?;
    let lean_delta = (1.0 - lean_blend).mul_add(data.lean, weighted_lean) - data.lean;
    data.lean = clamp_delta(lean_delta, lean_maximum) + data.lean;
    s.settable(names.lean, data.lean)?;
    let wrapped_twist = select(twist, twist, twist + TWO_PI);
    let mut difference = wrapped_twist - data.twist;
    // Raw BO4 BI25/24: unordered skips both arithmetic wrap branches.
    if difference > PI {
        difference -= TWO_PI;
    } else if difference < -PI {
        difference += TWO_PI;
    }
    let inverse_blend = 1.0 - twist_blend;
    let adjusted = data.twist + difference;
    let weighted_old = inverse_blend * data.twist;
    let delta = adjusted.mul_add(twist_blend, weighted_old) - data.twist;
    let mut delta = clamp_delta(delta, twist_maximum);
    if data.updates >= 2 {
        delta = clamp_delta(delta - data.delta_twist, twist_acceleration) + data.delta_twist;
    }
    let mut twist = data.twist + delta;
    if twist > TWO_PI {
        twist -= TWO_PI;
    }
    let wrapped = twist + TWO_PI;
    data.delta_twist = delta;
    data.twist = select(twist, twist, wrapped);
    s.settable(names.twist, data.twist)?;
    s.capture_intents()?;
    let x_blend = s.scalar(0x4385_8810_d1cd_e1d7)?;
    let y_blend = s.scalar(0x303b_7fcd_b850_0dcd)?;
    let threshold = s.scalar(0x2f48_759c_d4b3_6916)?;
    let x = s.gesture(true)?.unwrap_or(0.0);
    let weighted_x = x * x_blend;
    data.gesture[0] = (1.0 - x_blend).mul_add(data.gesture[0], weighted_x);
    s.settable(names.gesture_x, data.gesture[0])?;
    let y = s.gesture(false)?.unwrap_or(0.0);
    let weighted_y = y * y_blend;
    data.gesture[1] = (1.0 - y_blend).mul_add(data.gesture[1], weighted_y);
    s.settable(names.gesture_y, data.gesture[1])?;
    let x_squared = data.gesture[0] * data.gesture[0];
    let threshold_squared = threshold * threshold;
    let magnitude = data.gesture[1].mul_add(data.gesture[1], x_squared);
    // 64ac=40990024 branches on NOT GT: NaN does not publish the attribute.
    if magnitude > threshold_squared {
        s.controlled()?;
    }
    s.publish_gesture(data.gesture)?;
    data.updates = data.updates.wrapping_add(1);
    Ok(())
}
