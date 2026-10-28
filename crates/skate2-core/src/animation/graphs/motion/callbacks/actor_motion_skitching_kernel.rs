//! EnterSkitching Update82cb4b10 and Skitching Update82cb48d0 arithmetic.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct EnterInstance {
    pub(crate) initial_crouch: f32,
    pub(crate) elapsed: f32,
    pub(crate) reach_blend: f32,
}
impl EnterInstance {
    pub(crate) fn begin(&mut self, crouch: f32) {
        self.initial_crouch = crouch;
        self.elapsed = 0.0;
        self.reach_blend = -1.0;
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct HoldingInstance {
    pub(crate) initial_crouch: f32,
    pub(crate) absorb_speed: f32,
}
fn fsel(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
pub(crate) fn limit(value: f32, previous: f32, delta: f32) -> f32 {
    let minimum = previous - delta;
    let maximum = previous + delta;
    let raised = fsel(minimum - value, minimum, value);
    fsel(maximum - raised, raised, maximum)
}
pub(crate) fn reach(
    data: &mut EnterInstance,
    time: f32,
    length: f32,
    contact_time: f32,
    short: f32,
    long: f32,
) -> f32 {
    let remaining = 1.0 - time / length;
    let reach_time = if remaining > 0.0 {
        contact_time / remaining
    } else {
        length
    };
    let span = long - short;
    let value = reach_time - short;
    let raised = fsel(-value, 0.0, value);
    let mut blend = fsel(span - raised, raised, span) / span;
    // Native blt skips only an ordered negative previous value, including -1
    // from Initialize; do not impose the delta cap on the first update.
    if !(data.reach_blend < 0.0) {
        blend = limit(blend, data.reach_blend, f32::from_bits(0x3dcc_cccd));
    }
    data.reach_blend = blend;
    1.0 - blend
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_reach_is_unrestricted_then_changes_by_point_one() {
        let mut data = EnterInstance::default();
        data.begin(0.75);
        assert_eq!(reach(&mut data, 0.0, 1.0, 1.0, 0.0, 1.0), 0.0);
        assert_eq!(reach(&mut data, 0.0, 1.0, 0.0, 0.0, 1.0), 1.0 - 0.9);
        assert_eq!(data.initial_crouch, 0.75);
        assert_eq!(data.elapsed, 0.0);
    }
    #[test]
    fn reach_uses_remaining_animation_fraction_and_completed_length() {
        let mut data = EnterInstance::default();
        data.begin(0.0);
        assert_eq!(reach(&mut data, 0.5, 1.0, 0.25, 0.0, 1.0), 0.5);
        data.begin(0.0);
        assert_eq!(reach(&mut data, 1.0, 1.0, 0.0, 0.0, 2.0), 0.5);
    }
    #[test]
    fn absorb_cap_is_per_update_in_both_directions() {
        let delta = f32::from_bits(0x3d23_d70a);
        assert_eq!(limit(1.0, 0.5, delta), 0.5 + delta);
        assert_eq!(limit(0.0, 0.5, delta), 0.5 - delta);
        assert_eq!(limit(0.51, 0.5, delta), 0.51);
    }
}
