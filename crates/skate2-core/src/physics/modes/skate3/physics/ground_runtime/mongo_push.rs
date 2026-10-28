//! Local stationary-mongo propulsion preference, separate from retail timing.
//! Feed the adjusted target through the original82D948B8 force calculation.
const STATIONARY_SPEED: f32 = 0.5;
// Owner preference: match a normal first push rather than the rejected extra
// strong stroke. This local calibration is checked against the normal push in
// the same recorded scene; it is not a recovered retail Skate3 constant.
const STROKE_GAIN: f32 = 1.2;

#[derive(Debug, Default)]
pub(crate) struct StationaryMongoPush {
    contact_active: bool,
    origin_speed: Option<f32>,
}
impl StationaryMongoPush {
    pub(crate) fn reset(&mut self) { *self = Self::default(); }

    pub(crate) fn target(
        &mut self, mongo_animation: bool, contact: bool, body_speed: f32,
        signed_speed: f32, native_target: f32,
    ) -> f32 {
        if !mongo_animation || !contact || !body_speed.is_finite() || body_speed < 0.0
            || !signed_speed.is_finite() || !native_target.is_finite() {
            self.reset();
            return native_target;
        }
        if !self.contact_active {
            self.origin_speed = (body_speed < STATIONARY_SPEED).then_some(signed_speed);
            self.contact_active = true;
        }
        // Anchor the gain at the stroke's starting velocity. Scaling only the
        // instantaneous target/current gap still converges to the old terminal
        // speed and does not give the requested additional speed per stroke.
        match self.origin_speed {
            Some(origin) if native_target > origin =>
                origin + (native_target - origin) * STROKE_GAIN,
            _ => native_target,
        }
    }
}
