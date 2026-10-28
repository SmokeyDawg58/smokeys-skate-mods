//! Remaining LandingOnDeckManager trajectory and accurate-IK children.
use super::{GRAVITY, LandingError, LandingOnDeck};
use crate::{
    debug_overlay::DebugOverlaySlot,
    dynamic_assembly_session::skeleton_normal::SkeletonNormalAssembly,
    input_cvar_runtime::InputCvarReferences,
    physical_player_trajectory_owners::{
        offboard_runtime::{position_at, velocity_at},
        selector_numeric,
    },
    reckoning_numeric::ReckoningNumeric,
    skeleton_root_transforms::SkeletonRootVector as V,
};

impl LandingOnDeck {
    /// InitTrajectory82dc8d38. The flags read follows the trajectory stores.
    pub(crate) fn init_trajectory<N>(
        &mut self,
        position: V,
        velocity: V,
    ) -> Result<(), LandingError<N>> {
        self.trajectory.position = position;
        self.trajectory.velocity = velocity;
        self.trajectory.gravity = GRAVITY;
        self.trajectory.time = [-1.0; 4];
        self.trajectory_valid = true;
        self.trajectory_time = 0.0;
        self.landing_hippy_jump = self.flags()? & 0x8000 != 0;
        Ok(())
    }

    /// GetCurrentVelocity82dc8db8 has no validity gate.
    pub(crate) fn current_velocity(&self) -> V {
        velocity_at(&self.trajectory, self.trajectory_time)
    }

    /// CanLandOnDeck82dc8ee8.
    pub(crate) fn can_land_on_deck(&self) -> bool {
        self.can_land && !self.something_in_the_way
    }

    /// CorrectTrajectory82dce588: position is committed before AdjustTrajectory.
    pub(crate) fn correct_trajectory<N: ReckoningNumeric>(
        &mut self,
        position: V,
        cvars: &InputCvarReferences,
        numeric: &mut N,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), LandingError<N::Error>> {
        let predicted = position_at(&self.trajectory, self.trajectory_time);
        let frames = numeric
            .truncate_i32(self.time_to_land * f32::from_bits(0x426f_ffff))
            .map_err(LandingError::Numeric)?;
        let delta = predicted.zip(position, |a, b| a - b);
        if frames > 0 {
            self.trajectory.position = self.trajectory.position.zip(delta, |a, b| a + b);
            let correction = V::known([0.0; 4]).zip(delta, |a, b| a - b);
            selector_numeric::adjust::<std::convert::Infallible, N>(
                &mut self.trajectory,
                frames,
                correction,
                0.5,
                numeric,
            )
            .map_err(LandingError::Adjustment)?;
        }
        if cvars.try_landing_debug().map_err(LandingError::Cvar)? {
            overlay.require().map_err(LandingError::Overlay)?;
        }
        Ok(())
    }

    /// CalculateAccurateIKOffset82dceab0, bound to the actual Skeleton owner.
    /// The quadratic product is added separately here, unlike position_at's FMA.
    pub(crate) fn calculate_accurate_ik_offset<N: ReckoningNumeric>(
        &mut self,
        skeleton: &SkeletonNormalAssembly,
        time_to_land: f32,
        cvars: &InputCvarReferences,
        numeric: &mut N,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), LandingError<N::Error>> {
        use LandingError as E;
        if cvars.try_landing_accurate_ik().map_err(E::Cvar)? {
            let time = self.trajectory_time + time_to_land;
            let linear = self
                .trajectory
                .velocity
                .zip(self.trajectory.position, |v, p| v.mul_add(time, p));
            let quadratic = self.trajectory.gravity.map(|g| (g * 0.5) * (time * time));
            let predicted = linear.zip(quadratic, |a, b| a + b);
            let raw = skeleton.current_animation().raw_skateboard();
            let board = V::known([raw[12], raw[13], raw[14], raw[15]]);
            let com = V::known(skeleton.current_animation().record().centre_of_mass());
            let delta = board.zip(com, |a, b| a - b);
            let (up, velocity) = {
                let p = self.skeleton.processed.lock().map_err(|_| E::Poisoned)?;
                (
                    p.prior_output()
                        .reckoning_up()
                        .ok_or(E::Missing("Processed up+220"))?
                        .value,
                    p.prior_output()
                        .velocity()
                        .ok_or(E::Missing("Processed velocity+190"))?
                        .value,
                )
            };
            let projection = numeric.dot_vector(up, velocity).map_err(E::Numeric)?;
            let basis = skeleton.root_transforms().animation_to_world().columns();
            let world_delta = basis[0].map(|a| a * delta.xyz[0]);
            let world_delta = basis[1].zip(world_delta, |a, b| a.mul_add(delta.xyz[1], b));
            let world_delta = basis[2].zip(world_delta, |a, b| a.mul_add(delta.xyz[2], b));
            let deck = self
                .skeleton
                .processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .deck_transform()
                .ok_or(E::Missing("Processed deck+40"))?
                .columns()[3];
            let normal_velocity = up.map(|v| v * projection);
            let tangent = velocity.zip(normal_velocity, |a, b| a - b);
            let expected = predicted.zip(world_delta, |a, b| a + b);
            let board_predicted = tangent.zip(deck, |v, p| {
                v.mul_add(time_to_land + f32::from_bits(0x3c88_8889), p)
            });
            self.landing_error = board_predicted.zip(expected, |a, b| a - b);
            if cvars.try_landing_zero_ik_y().map_err(E::Cvar)? {
                self.landing_error.xyz[1] = 0.0;
            }
            if cvars.try_landing_debug().map_err(E::Cvar)? {
                overlay.require().map_err(E::Overlay)?;
            }
        }
        if cvars.try_landing_debug().map_err(E::Cvar)? {
            overlay.require().map_err(E::Overlay)?;
        }
        Ok(())
    }
}
