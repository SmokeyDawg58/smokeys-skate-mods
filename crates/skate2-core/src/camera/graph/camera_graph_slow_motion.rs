//! `Sk8::Behaviours::SlowMotionController`: CreateInstance82ca2738,
//! Begin82ca8e20, Update82ca8f48 and End82ca9170. Each callback only posts a
//! `cMsgSetSimRate`; the Simulation handles it at the next Update1 dispatch.
use super::{CameraGraphBackend, CameraGraphLeafError};
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    camera_graph::CameraContext,
    physics::PointGraph,
    reckoning_numeric::ReckoningNumeric,
    simulation_sim_rate::{SetSimRate, UPDATE_TIME},
};

/// `SlowMotionController::Data`, 0x18 bytes after the RefCounter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SlowMotionInstance {
    /// +0x08 `mTimeInSlowMotion`.
    time_in_slow_motion: f32,
    /// +0x0c `mFirstLookAtTimeInAir`.
    first_look_at_time_in_air: f32,
    /// +0x10 `mTimeBeforeInAir`.
    time_before_in_air: f32,
    /// +0x14 `mWasInAir`.
    was_in_air: bool,
}
impl SlowMotionInstance {
    /// CreateInstance82ca2798..27c0: false, 0, 1 and 0.
    pub(crate) fn construct() -> Self {
        Self {
            time_in_slow_motion: 0.0,
            first_look_at_time_in_air: 1.0,
            time_before_in_air: 0.0,
            was_in_air: false,
        }
    }

    /// Update82ca9058..90d8 after the elapsed store that precedes the PhysOut
    /// read; an unordered trajectory time takes the grounded `ble` path.
    fn advance(&mut self, dt: f32, traj_collision_time: f32) -> f32 {
        if traj_collision_time > 0.0 {
            if !self.was_in_air {
                self.first_look_at_time_in_air = traj_collision_time;
                self.was_in_air = true;
            }
            self.time_in_slow_motion / (self.time_before_in_air + self.first_look_at_time_in_air)
        } else {
            if !self.was_in_air {
                self.time_before_in_air += dt;
            }
            f32::from_bits(0x3dcc_cccd)
        }
    }
}

/// `slowmotion_controller::_LayoutStruct` (0x84 bytes; zeros when absent).
struct SlowMotionAttributes {
    /// +0x00 `timescale`, PointGraphData16.
    timescale: PointGraph<16>,
    /// +0x80 `fps_at_scale_one`.
    fps_at_scale_one: f32,
}
impl SlowMotionAttributes {
    fn read<E>(attributes: &AttributeRuntimeInstance) -> Result<Self, CameraGraphLeafError<E>> {
        let layout = attributes
            .try_cached_layout(0x84)
            .map_err(CameraGraphLeafError::Attribute)?;
        let word = |at: usize| {
            layout.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(
                    data.bytes()[at..at + 4]
                        .try_into()
                        .expect("0x84-byte slowmotion_controller layout"),
                )
            })
        };
        Ok(Self {
            timescale: PointGraph {
                x: std::array::from_fn(|i| word(i * 4)),
                y: std::array::from_fn(|i| word(0x40 + i * 4)),
            },
            fps_at_scale_one: word(0x80),
        })
    }

    /// Begin82ca8ebc..8f14 / Update82ca9100..914c: fmuls, fsubs, fused
    /// fmadds and fdivs, posted with one tick of slow motion.
    fn sim_rate(&self, time: f32, scale: f32) -> SetSimRate {
        let product = self.timescale.evaluate(time) * scale;
        let fps = (self.fps_at_scale_one - 60.0).mul_add(product, 60.0);
        SetSimRate {
            sim_time: 1.0 / fps,
            ticks_of_slow_motion: 1,
        }
    }
}

impl<N: ReckoningNumeric> CameraGraphBackend<'_, N> {
    fn post_sim_rate(&mut self, message: SetSimRate) -> Result<(), CameraGraphLeafError<N::Error>> {
        self.inputs
            .sim_rate
            .lock()
            .map_err(|_| CameraGraphLeafError::OwnerLock)?
            .send(message).map_err(CameraGraphLeafError::SimRate)?;
        Ok(())
    }

    fn in_an_online_game(&self) -> Result<bool, CameraGraphLeafError<N::Error>> {
        self.inputs
            .sim_rate
            .lock()
            .map_err(|_| CameraGraphLeafError::OwnerLock)?
            .try_is_network_game()
            .map_err(CameraGraphLeafError::SimRate)
    }

    pub(super) fn slow_motion_begin(
        &mut self,
        attributes: &AttributeRuntimeInstance,
        instance: Option<&mut SlowMotionInstance>,
    ) -> Result<(), CameraGraphLeafError<N::Error>> {
        if self.in_an_online_game()? {
            return Ok(());
        }
        let data = instance.ok_or(CameraGraphLeafError::MissingInstance)?;
        data.was_in_air = false;
        data.time_in_slow_motion = 0.0;
        data.time_before_in_air = 0.0;
        let scale = self
            .inputs
            .cvars
            .try_slow_motion_scale()
            .map_err(CameraGraphLeafError::Cvar)?;
        let message = SlowMotionAttributes::read::<N::Error>(attributes)?.sim_rate(0.0, scale);
        self.post_sim_rate(message)
    }

    pub(super) fn slow_motion_update(
        &mut self,
        attributes: &AttributeRuntimeInstance,
        context: &CameraContext,
        dt: f32,
        instance: Option<&mut SlowMotionInstance>,
    ) -> Result<(), CameraGraphLeafError<N::Error>> {
        if self.in_an_online_game()? {
            return Ok(());
        }
        // 82ca8fa8..9044: a null Entity has no IPhysical and posts nothing.
        let Some(actor) = context.entity() else {
            return Ok(());
        };
        let physical = actor
            .0
            .cached_physical_allocation()
            .map_err(CameraGraphLeafError::Physical)?;
        let data = instance.ok_or(CameraGraphLeafError::MissingInstance)?;
        data.time_in_slow_motion += dt;
        // IPhysical+0x28 GetPhysOut, PhysOut+8 Air, +0xa8 mTrajCollisionTime.
        let time = {
            let output = physical
                .owner()
                .phys_out()
                .lock()
                .map_err(|_| CameraGraphLeafError::OwnerLock)?;
            let traj_collision_time = output
                .try_air_output()
                .map_err(CameraGraphLeafError::Component)?
                .traj_collision_time;
            data.advance(dt, traj_collision_time)
        };
        let scale = self
            .inputs
            .cvars
            .try_slow_motion_scale()
            .map_err(CameraGraphLeafError::Cvar)?;
        let message = SlowMotionAttributes::read::<N::Error>(attributes)?.sim_rate(time, scale);
        self.post_sim_rate(message)
    }

    /// End82ca91a4..91bc: `kUpdateTime` with no slow-motion ticks, no online gate.
    pub(super) fn slow_motion_end(&mut self) -> Result<(), CameraGraphLeafError<N::Error>> {
        self.post_sim_rate(SetSimRate {
            sim_time: UPDATE_TIME,
            ticks_of_slow_motion: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grounded_updates_accumulate_until_the_first_airborne_look_latches() {
        let dt = f32::from_bits(0x3c88_8889);
        let mut data = SlowMotionInstance::construct();
        assert_eq!(data.advance(dt, 0.0).to_bits(), 0x3dcc_cccd);
        assert_eq!(data.advance(dt, f32::NAN).to_bits(), 0x3dcc_cccd);
        assert_eq!(data.time_before_in_air.to_bits(), (dt + dt).to_bits());
        data.time_in_slow_motion = 0.25;
        let first = data.advance(dt, 0.5);
        assert!(data.was_in_air);
        assert_eq!(data.first_look_at_time_in_air.to_bits(), 0.5_f32.to_bits());
        assert_eq!(first.to_bits(), (0.25 / (dt + dt + 0.5)).to_bits());
        // Later trajectory times and grounded frames never move the latch.
        data.advance(dt, 0.2);
        data.advance(dt, 0.0);
        assert_eq!(data.first_look_at_time_in_air.to_bits(), 0.5_f32.to_bits());
        assert_eq!(data.time_before_in_air.to_bits(), (dt + dt).to_bits());
    }

    #[test]
    fn unit_timescale_at_scale_one_posts_the_authored_rate() {
        let attributes = SlowMotionAttributes {
            timescale: PointGraph {
                x: std::array::from_fn(|i| f32::from(u16::try_from(i).unwrap())),
                y: [1.0; 16],
            },
            fps_at_scale_one: 30.0,
        };
        let message = attributes.sim_rate(0.0, 1.0);
        assert_eq!(message.sim_time.to_bits(), (1.0_f32 / 30.0).to_bits());
        assert_eq!(message.ticks_of_slow_motion, 1);
        let unscaled = attributes.sim_rate(0.0, 0.0);
        assert_eq!(unscaled.sim_time.to_bits(), (1.0_f32 / 60.0).to_bits());
    }
}
