//! PowerSliding.Update82cc7478, typed live operations and native store order.
#[cfg(test)]
#[path = "motion_power_sliding_tests.rs"]
mod tests;
#[derive(Clone, Copy, Debug)]
pub(crate) enum Side {
    Left,
    Right,
}
impl Side {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}
#[derive(Debug)]
pub(crate) struct SlidingInstance {
    pub(crate) heading_time: f32,
    pub(crate) previous_right: f32,
    pub(crate) previous_left: f32,
    pub(crate) right_started: bool,
    pub(crate) left_started: bool,
    pub(crate) was_ground: bool,
}
impl SlidingInstance {
    /// Create82ca3088: allocation18 hex/alignment10. Only three high flags
    /// are initialized; unrelated low29 bits are not reconstructed fields.
    pub(crate) fn after_allocation(allocated: bool) -> Option<Self> {
        allocated.then_some(Self {
            heading_time: 0.0,
            previous_right: 0.0,
            previous_left: 0.0,
            right_started: false,
            left_started: false,
            was_ground: false,
        })
    }
}
pub(crate) trait SlidingSource {
    type Error;
    fn resolve_interfaces_and_sliding_state(&mut self) -> Result<(), Self::Error>;
    fn intent(&mut self, side: Side) -> Result<Option<f32>, Self::Error>;
    fn is_ground(&mut self) -> Result<bool, Self::Error>;
    fn missing_instance(&mut self) -> Self::Error;
    fn deck_speed(&mut self) -> Result<f32, Self::Error>;
    fn speed_limit(&mut self) -> Result<f32, Self::Error>;
    fn start_intent(&mut self, side: Side) -> Result<bool, Self::Error>;
    fn write_start(&mut self, side: Side, value: bool) -> Result<(), Self::Error>;
    fn debug(&mut self, calls: usize) -> Result<(), Self::Error>;
    fn heading_cosine(&mut self) -> Result<f32, Self::Error>;
    fn graph(&mut self, offset: usize, value: f32) -> Result<f32, Self::Error>;
    fn dt(&mut self) -> Result<f32, Self::Error>;
    fn minimum_time(&mut self) -> Result<f32, Self::Error>;
    fn increment_time(&mut self, side: Side, dt: f32) -> Result<f32, Self::Error>;
    fn write_end(&mut self, side: Side, value: bool) -> Result<(), Self::Error>;
}

pub(crate) fn update<S: SlidingSource>(
    source: &mut S,
    instance: Option<&mut SlidingInstance>,
) -> Result<(), S::Error> {
    source.resolve_interfaces_and_sliding_state()?;
    let right = source.intent(Side::Right)?;
    let left = source.intent(Side::Left)?;
    let ground = source.is_ground()?;
    let state = instance.ok_or_else(|| source.missing_instance())?;
    let r = right.unwrap_or(0.0);
    let l = left.unwrap_or(0.0);
    if ground && !state.was_ground {
        state.heading_time = 0.0;
        state.previous_right = r;
        state.previous_left = l;
        state.right_started = false;
        state.left_started = false;
    }
    state.was_ground = ground;
    if !ground {
        return Ok(());
    }
    let speed = source.deck_speed()?;
    let limit = source.speed_limit()?;
    // Raw7858 BGT: unordered speed comparison is false, reused for end flags.
    let fast = speed > limit;
    let right_start = source.start_intent(Side::Right)?;
    source.write_start(
        Side::Right,
        fast && right_start && !state.right_started && state.previous_right < r,
    )?;
    state.previous_right = r;
    state.right_started = right_start;
    let left_start = source.start_intent(Side::Left)?;
    source.write_start(
        Side::Left,
        fast && left_start && !state.left_started && state.previous_left > l,
    )?;
    state.previous_left = l;
    state.left_started = left_start;
    source.debug(2)?;
    let heading = source.heading_cosine()?;
    let speed_graph = source.graph(0x4a0, speed)?;
    let threshold = speed_graph * (1.0 - heading.abs());
    let heading_threshold = source.graph(0x500, heading)?;
    let heading_duration = source.graph(0x4d0, heading)?;
    // Raw7bbc BO4/BI24 skips on !LT, including unordered.
    state.heading_time = if threshold < heading_threshold {
        source.dt()? + state.heading_time
    } else {
        0.0
    };
    let heading_expired = state.heading_time > heading_duration;
    let minimum = source.minimum_time()?;
    let dt = source.dt()?;
    let right_time = source.increment_time(Side::Right, dt)?;
    let dt = source.dt()?;
    let left_time = source.increment_time(Side::Left, dt)?;
    // Raw7c7c/7cbc BO4/BI25 skips on !GT; NaN times do not end slides.
    source.write_end(
        Side::Right,
        right_time > minimum && (right.is_none() || !fast || heading_expired),
    )?;
    source.write_end(
        Side::Left,
        left_time > minimum && (left.is_none() || !fast || heading_expired),
    )?;
    source.debug(3)
}
