//! `ControlAirLegExtension` state and native ordered updates.
#[derive(Debug, Default)]
pub(crate) struct AirLegInstance {
    pub(crate) phase: u32,
    pub(crate) distance: f32,
    pub(crate) down_time: f32,
}
impl AirLegInstance {
    pub(crate) fn after_allocation(allocated: bool) -> Option<Self> {
        allocated.then(Self::default)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tuning {
    UpSpeed,
    DownSpeed,
    Minimum,
    PrelandSpeed,
    FinalDistance,
    OffboardMultiplier,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cvar {
    Debug,
    Up,
    Down,
    Preland,
    Custom,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Resolve {
    Update,
    Up,
    Down,
    Preland,
    Prepare,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    VelocityY,
    ComToDeck,
    CollisionTime,
}
pub(crate) enum FullValue {
    Known(f32),
    Unavailable,
}
pub(crate) trait Source {
    type Error;
    fn resolve(&mut self, stage: Resolve) -> Result<(), Self::Error>;
    fn value(&mut self, field: Value) -> Result<f32, Self::Error>;
    fn drive_attribute(&mut self) -> Result<Option<f32>, Self::Error>;
    fn initial_distance(&mut self, data: &mut AirLegInstance) -> Result<(), Self::Error>;
    fn tuning(&mut self, field: Tuning) -> Result<f32, Self::Error>;
    fn cvar(&mut self, field: Cvar) -> Result<bool, Self::Error>;
    fn delta_time(&mut self) -> Result<f32, Self::Error>;
    fn downward_projection(&mut self) -> Result<(f32, f32), Self::Error>;
    fn prepare_land(&mut self) -> Result<bool, Self::Error>;
    fn full_extension(&mut self) -> Result<Option<FullValue>, Self::Error>;
    fn custom_earlier(&mut self) -> Result<f32, Self::Error>;
    fn offboard_bone(&self) -> bool;
    fn write_drive(&mut self, value: f32) -> Result<(), Self::Error>;
    fn extend_graph(&mut self, time: f32) -> Result<f32, Self::Error>;
    fn write_extend(&mut self, value: f32) -> Result<(), Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
    fn missing_instance(&self) -> Self::Error;
    fn missing_attribute_word(&self) -> Self::Error;
}
fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}
fn debug<S: Source>(s: &mut S) -> Result<(), S::Error> {
    if s.cvar(Cvar::Debug)? {
        s.require_overlay()?;
        s.require_overlay()?;
    }
    Ok(())
}
#[expect(
    clippy::assign_op_pattern,
    reason = "preserve native multiplier-first operand order"
)]
fn preland<S: Source>(d: &mut AirLegInstance, s: &mut S) -> Result<(), S::Error> {
    s.resolve(Resolve::Preland)?;
    let speed = s.tuning(Tuning::PrelandSpeed)?;
    if s.cvar(Cvar::Preland)? {
        d.distance = s.delta_time()?.mul_add(speed, d.distance);
        if s.offboard_bone() {
            d.distance = s.tuning(Tuning::OffboardMultiplier)? * d.distance;
        }
    }
    debug(s)
}
fn should_prepare<S: Source>(s: &mut S) -> Result<bool, S::Error> {
    s.resolve(Resolve::Prepare)?;
    if !s.prepare_land()? {
        return Ok(false);
    }
    let full = s.full_extension()?;
    if !s.cvar(Cvar::Custom)? {
        return Ok(true);
    }
    let Some(full) = full else { return Ok(true) };
    let full = match full {
        FullValue::Known(value) => value,
        FullValue::Unavailable => return Err(s.missing_attribute_word()),
    };
    if full < s.tuning(Tuning::FinalDistance)? - 0.1 {
        return Ok(true);
    }
    let speed = s.tuning(Tuning::PrelandSpeed)?;
    let duration = (full - s.value(Value::ComToDeck)?) / speed;
    let available = s.value(Value::CollisionTime)? - s.custom_earlier()?;
    if available.partial_cmp(&duration) != Some(std::cmp::Ordering::Less) {
        return Ok(false);
    }
    s.require_overlay()?;
    Ok(true)
}
fn down<S: Source>(d: &mut AirLegInstance, s: &mut S) -> Result<(), S::Error> {
    s.resolve(Resolve::Down)?;
    if should_prepare(s)? {
        d.phase = 3;
        return preland(d, s);
    }
    let given = s.tuning(Tuning::DownSpeed)?;
    let (neg_dot, upright) = s.downward_projection()?;
    let downward = select(-neg_dot, 0.0, neg_dot);
    let mixed = (1.0 - upright).mul_add(given, upright * downward);
    let speed = select(given - mixed, mixed, given);
    let minimum = s.tuning(Tuning::Minimum)?;
    if s.cvar(Cvar::Down)? {
        let candidate = (-s.delta_time()?).mul_add(speed, d.distance);
        d.distance = select(candidate - minimum, candidate, minimum);
    }
    debug(s)
}
fn up<S: Source>(d: &mut AirLegInstance, s: &mut S) -> Result<(), S::Error> {
    s.resolve(Resolve::Up)?;
    if s.value(Value::VelocityY)? < 0.0 {
        d.phase = 2;
        return down(d, s);
    }
    let speed = s.tuning(Tuning::UpSpeed)?;
    let minimum = s.tuning(Tuning::Minimum)?;
    if s.cvar(Cvar::Up)? {
        let candidate = (-s.delta_time()?).mul_add(speed, d.distance);
        d.distance = select(candidate - minimum, candidate, minimum);
    }
    debug(s)
}
pub(crate) fn update<S: Source>(
    data: Option<&mut AirLegInstance>,
    s: &mut S,
) -> Result<(), S::Error> {
    s.resolve(Resolve::Update)?;
    let going_up = s.value(Value::VelocityY)? > 0.0;
    let d = data.ok_or_else(|| s.missing_instance())?;
    match d.phase {
        0 => {
            d.phase = if going_up { 1 } else { 2 };
            if let Some(value) = s.drive_attribute()? {
                d.distance = value;
            } else {
                s.initial_distance(d)?;
            }
        }
        1 => up(d, s)?,
        2 => down(d, s)?,
        3 => preland(d, s)?,
        _ => {}
    }
    s.write_drive(d.distance)?;
    d.down_time = if going_up {
        0.0
    } else {
        s.delta_time()? + d.down_time
    };
    let extend = s.extend_graph(d.down_time)?;
    s.write_extend(extend)
}
#[cfg(test)]
#[path = "motion_air_leg_tests.rs"]
mod tests;
