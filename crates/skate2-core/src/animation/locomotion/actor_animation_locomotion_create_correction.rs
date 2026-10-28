//! CreateLocomotionTrajectory foot/collision correction82cadc80..82cade0c.
use super::physics_update_data::trajectory::AnimationLocomotionError;
use super::{ActorAnimationConstruction, locomotion_producer::TrajectoryProducerError};
use crate::{
    input_cvar_runtime::InputCvarReferences,
    reckoning_numeric::{ReckoningNumeric, try_normalize_safe},
    skeleton_root_transforms::SkeletonRootVector as V,
};
#[derive(Debug)]
pub(super) enum CreateCorrectionError<N> {
    Missing(&'static str),
    UnknownFlags(u32),
    Foot(i32),
    Trajectory(TrajectoryProducerError<std::convert::Infallible>),
    CurrentCom(AnimationLocomotionError<N>),
    Numeric(N),
}
impl<K, W> ActorAnimationConstruction<K, W> {
    pub(super) fn try_correct_create_locomotion<N: ReckoningNumeric>(
        &mut self,
        collision_updated: bool,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), CreateCorrectionError<N::Error>> {
        use CreateCorrectionError as E;
        let player = self.player.as_mut().ok_or(E::Missing("player"))?;
        let fields = self
            .before_sprint
            .as_ref()
            .ok_or(E::Missing("locomotion fields"))?;
        let suffix = self
            .before_face
            .as_mut()
            .ok_or(E::Missing("locomotion suffix"))?;
        let trajectory = &mut self
            .skater
            .as_mut()
            .ok_or(E::Missing("trajectory"))?
            .trajectory;
        if fields.foot_down != fields.last_foot_down && fields.foot_down != -1 {
            trajectory
                .try_set_foot_world_position(
                    fields.foot_down,
                    suffix.feet[(fields.foot_down & 1) as usize],
                )
                .map_err(E::Trajectory)?;
            player.flags_bits &= !0x1e0;
            player.flags_known_mask |= 0x1e0;
        }
        let zero = V::known([0.0; 4]);
        if !collision_updated {
            suffix.collision_trajectory_displacement = zero;
            return Ok(());
        }
        if player.flags_known_mask & 8 == 0 {
            return Err(E::UnknownFlags(8));
        }
        if player.flags_bits & 8 == 0 {
            suffix.collision_trajectory_displacement = zero;
            return Ok(());
        }
        let mut current = zero;
        trajectory
            .try_current_com(&mut current, true, refs, numeric)
            .map_err(E::CurrentCom)?;
        let mut delta = current.zip(suffix.last_centre_of_mass, |a, b| a - b);
        delta.xyz[1] = 0.0;
        let normal =
            try_normalize_safe(suffix.collision_displacement, zero, numeric).map_err(E::Numeric)?;
        let dot = numeric.dot_vector(delta, normal).map_err(E::Numeric)?;
        // 82cadd60=40980060 branches to zero when LT is clear (NaN included).
        if dot < 0.0 {
            let negative = f32::from_bits(dot.to_bits() ^ 0x8000_0000);
            suffix.collision_trajectory_displacement = normal
                .zip(suffix.collision_trajectory_displacement, |n, old| {
                    n.mul_add(negative, old)
                });
            let offset = suffix.collision_trajectory_displacement;
            let position = suffix.world_position.columns()[3].zip(offset, |p, d| p + d);
            suffix.world_position.set_column(3, position);
            let foot = usize::try_from(trajectory.foot_locked)
                .ok()
                .filter(|i| *i < 4)
                .ok_or(E::Foot(trajectory.foot_locked))?;
            trajectory.input_feet[foot].data.position = trajectory.input_feet[foot]
                .data
                .position
                .zip(offset, |p, d| p + d);
        } else {
            suffix.collision_trajectory_displacement = zero;
        }
        Ok(())
    }
}
