//! `Sk8::Behaviours::LandingOffBoardAir`: Create82cb4730, Begin82cb4610,
//! Update82ca6340; End is the shared BLR825187c0.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::{input_cvar_runtime::InputCvarReferences, play_animation::FastString30};

/// `kUpdateTime_56`822f2204 (0x3c888889); Update never reads the frame dt.
const UPDATE_TIME: f32 = f32::from_bits(0x3c88_8889);

/// Create82cb4730: RefCounter plus four zeroed floats (0x18 bytes, align 0x10).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct LandingOffBoardAirInstance {
    /// +0x08, written to the `Impact` settable attribute.
    pub(crate) impact: f32,
    /// +0x0c, raised to 1 by Begin only; never cleared afterwards.
    pub(crate) impact_target: f32,
    /// +0x10, written to the `CloseToLand` settable attribute.
    pub(crate) close_to_land: f32,
    /// +0x14, advanced by the fixed update time.
    pub(crate) time: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LandingTuning {
    pub(crate) min_blend_delta: f32,
    pub(crate) max_blend_delta: f32,
    pub(crate) time_to_wait_to_blend_to_two_foot: f32,
    pub(crate) time_to_blend_to_two_foot: f32,
    pub(crate) time_to_blend_to_land_pose: f32,
    pub(crate) land_pose_blend_delta: f32,
}

fn fsel(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}

/// 82cb4654..4694: the three stores precede GetPhysOut; `ble` skips an
/// unordered comparison, so only an ordered greater-than raises the target.
pub(crate) fn begin(data: &mut LandingOffBoardAirInstance, landing_velocity: f32, threshold: f32) {
    if landing_velocity.abs() > threshold {
        data.impact_target = 1.0;
    }
}

/// 82ca6488..64a4: the timer store precedes the GetPhysOut call.
pub(crate) fn advance(data: &mut LandingOffBoardAirInstance) {
    data.time += UPDATE_TIME;
}

/// 82ca64b0..658c. The blend delta stays 0 unless the ordered target span is
/// positive; the two `fsel` clamp time-to-land/span into [min, max].
pub(crate) fn blend(data: &mut LandingOffBoardAirInstance, time_to_land: f32, t: &LandingTuning) {
    let span = data.impact_target - data.impact;
    let mut delta = 0.0;
    if span > 0.0 {
        let rate = time_to_land / span;
        let raised = fsel(t.min_blend_delta - rate, t.min_blend_delta, rate);
        delta = fsel(t.max_blend_delta - raised, raised, t.max_blend_delta);
    }
    if (data.time > t.time_to_wait_to_blend_to_two_foot
        || time_to_land < t.time_to_blend_to_two_foot)
        && data.impact < 1.0
    {
        data.impact += delta;
    }
    if time_to_land < t.time_to_blend_to_land_pose && data.close_to_land < 1.0 {
        data.close_to_land = t.land_pose_blend_delta + data.close_to_land;
    }
}

fn tuning(cvars: &InputCvarReferences) -> Result<LandingTuning, E> {
    Ok(LandingTuning {
        min_blend_delta: cvars
            .try_landing_impact_min_blend_delta()
            .map_err(E::Cvar)?,
        max_blend_delta: cvars
            .try_landing_impact_max_blend_delta()
            .map_err(E::Cvar)?,
        time_to_wait_to_blend_to_two_foot: cvars
            .try_landing_time_to_wait_to_blend_to_two_foot()
            .map_err(E::Cvar)?,
        time_to_blend_to_two_foot: cvars
            .try_landing_time_to_blend_to_two_foot()
            .map_err(E::Cvar)?,
        time_to_blend_to_land_pose: cvars
            .try_landing_time_to_blend_to_land_pose()
            .map_err(E::Cvar)?,
        land_pose_blend_delta: cvars.try_landing_land_pose_blend_delta().map_err(E::Cvar)?,
    })
}

impl ActorConstruction {
    fn landing_offboard_air_debug_text(&self, cvars: &InputCvarReferences) -> Result<(), E> {
        if cvars.try_show_skater_in_air_debug_text().map_err(E::Cvar)? {
            self.action_runtime_services()
                .map_err(E::Actor)?
                .as_ref()
                .ok_or(E::MissingOwner("LandingOffBoardAir debug overlay"))?
                .overlay
                .require()
                .map_err(E::LandingOffBoardAirOverlay)?;
        }
        Ok(())
    }

    /// Begin82cb4610: IPhysical, three stores, landing-velocity threshold, debug text.
    pub(crate) fn motion_landing_offboard_air_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        data: Option<&mut LandingOffBoardAirInstance>,
        cvars: Option<&InputCvarReferences>,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let data = data.ok_or(E::MissingOwner("LandingOffBoardAir instance"))?;
        data.impact = 0.0;
        data.close_to_land = 0.0;
        data.time = 0.0;
        let landing_velocity = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .landing_velocity;
        let cvars = cvars.ok_or(E::MissingOwner("LandingOffBoardAir cvars"))?;
        let threshold = cvars
            .try_landing_velocity_two_foot_threshold()
            .map_err(E::Cvar)?;
        begin(data, landing_velocity, threshold);
        self.landing_offboard_air_debug_text(cvars)
    }

    /// Update82ca6340: IPhysical and IAnimatable are both required; the two
    /// `AddSettableAtt`82d7f5f0 calls (vtable +0x48) pass normalized true, sequence -1.
    pub(crate) fn motion_landing_offboard_air_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        data: Option<&mut LandingOffBoardAirInstance>,
        impact: FastString30,
        close_to_land: FastString30,
        cvars: Option<&InputCvarReferences>,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let animation = self.lifecycle_animation(0)?;
        let data = data.ok_or(E::MissingOwner("LandingOffBoardAir instance"))?;
        let cvars = cvars.ok_or(E::MissingOwner("LandingOffBoardAir cvars"))?;
        let tuning = tuning(cvars)?;
        advance(data);
        let time_to_land = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .time_to_land;
        blend(data, time_to_land, &tuning);
        if std::env::var_os("SKATE2_TRACE_OFFBOARD_AIR").is_some() {
            eprintln!(
                "offboard-air landing: time_to_land={time_to_land} impact={} impact_target={} close_to_land={} time={}",
                data.impact, data.impact_target, data.close_to_land, data.time
            );
        }
        {
            let mut animation = animation.lock().map_err(|_| E::Lock)?;
            animation
                .try_add_gesture_attribute(impact, data.impact, true, -1)
                .map_err(E::AnimationWrite)?;
            animation
                .try_add_gesture_attribute(close_to_land, data.close_to_land, true, -1)
                .map_err(E::AnimationWrite)?;
        }
        self.landing_offboard_air_debug_text(cvars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(data: &mut LandingOffBoardAirInstance, time_to_land: f32, t: &LandingTuning) {
        advance(data);
        blend(data, time_to_land, t);
    }

    fn tuning() -> LandingTuning {
        LandingTuning {
            min_blend_delta: f32::from_bits(0x3ca3_d70a),
            max_blend_delta: f32::from_bits(0x3d4c_cccd),
            time_to_wait_to_blend_to_two_foot: 0.5,
            time_to_blend_to_two_foot: 0.5,
            time_to_blend_to_land_pose: f32::from_bits(0x3e4c_cccd),
            land_pose_blend_delta: f32::from_bits(0x3d23_d70a),
        }
    }

    #[test]
    fn begin_raises_only_on_ordered_greater_magnitude() {
        let mut data = LandingOffBoardAirInstance::default();
        begin(&mut data, -5.0, 5.0);
        assert_eq!(data.impact_target, 0.0);
        begin(&mut data, f32::NAN, 5.0);
        assert_eq!(data.impact_target, 0.0);
        begin(&mut data, -5.5, 5.0);
        assert_eq!(data.impact_target, 1.0);
    }

    #[test]
    fn impact_blend_clamps_rate_and_waits_for_gates() {
        let t = tuning();
        let mut data = LandingOffBoardAirInstance {
            impact_target: 1.0,
            ..LandingOffBoardAirInstance::default()
        };
        // Far from landing and before the wait time: no impact blend.
        step(&mut data, 2.0, &t);
        assert_eq!(data.impact.to_bits(), 0);
        assert_eq!(data.time.to_bits(), UPDATE_TIME.to_bits());
        // Inside the two-foot window: rate 0.3/1 clamps to the 0.05 maximum.
        step(&mut data, 0.3, &t);
        assert_eq!(data.impact.to_bits(), t.max_blend_delta.to_bits());
        // Tiny rate clamps up to the 0.02 minimum.
        let before = data.impact;
        step(&mut data, 0.001, &t);
        assert_eq!(
            data.impact.to_bits(),
            (before + t.min_blend_delta).to_bits()
        );
    }

    #[test]
    fn nan_time_to_land_clamps_delta_to_max_but_fails_both_gates() {
        let t = tuning();
        let mut data = LandingOffBoardAirInstance {
            impact_target: 1.0,
            ..LandingOffBoardAirInstance::default()
        };
        step(&mut data, f32::NAN, &t);
        assert_eq!(data.impact.to_bits(), 0);
        assert_eq!(data.close_to_land.to_bits(), 0);
    }

    #[test]
    fn close_to_land_steps_until_one() {
        let t = tuning();
        let mut data = LandingOffBoardAirInstance::default();
        step(&mut data, 0.1, &t);
        assert_eq!(
            data.close_to_land.to_bits(),
            t.land_pose_blend_delta.to_bits()
        );
        data.close_to_land = 1.0;
        step(&mut data, 0.1, &t);
        assert_eq!(data.close_to_land, 1.0);
    }
}
