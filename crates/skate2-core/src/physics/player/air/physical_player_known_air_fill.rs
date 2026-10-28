//! PhysState_KnownAir::FillPhysOut82d8c348; preserve the existing Air output.
use crate::{
    physical_player_assembly::remaining_states::{
        KnownAir, known_air_trajectory::KnownAirTrajectoryError,
    },
    physical_player_phys_out::PhysicalPlayerPhysOut,
    physical_player_trajectory_owners::{OnboardTrajectoryOwner, offboard_runtime::position_at},
    reckoning_numeric::ReckoningNumeric,
    skeleton_root_transforms::SkeletonRootMatrix,
};
#[derive(Debug)]
pub(crate) enum KnownAirFillError<N> {
    Output(super::prior_input::PriorInputReadError),
    Source(KnownAirTrajectoryError<N>),
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_known_air_state<N: ReckoningNumeric>(
        &mut self,
        s: &KnownAir,
        selector: &OnboardTrajectoryOwner,
        orientation: &SkeletonRootMatrix,
        refs: &crate::input_cvar_runtime::InputCvarReferences,
        n: &mut N,
    ) -> Result<(), KnownAirFillError<N::Error>> {
        use KnownAirFillError as E;
        macro_rules! need {
            ($field:ident) => {
                s.$field
                    .ok_or(E::Source(KnownAirTrajectoryError::Missing(stringify!(
                        $field
                    ))))?
            };
        }
        macro_rules! put {
            ($field:ident, $value:expr) => {{
                let value = $value;
                self.air_fill_guard().map_err(E::Output)?;
                self.concrete.air.as_mut().unwrap().$field = value;
            }};
        }
        put!(is_at_apex, need!(reached_apex));
        put!(jump_height, need!(max_y) - need!(start_y));
        put!(traj_apex, need!(apex_pos));
        put!(total_time_to_apex, need!(time_to_apex));
        put!(traj_coll_position, need!(coll_pos));
        put!(traj_coll_normal, need!(landing_normal));
        put!(landing_normal, need!(landing_normal));
        let fix_board = refs
            .try_known_air_bool(
                crate::input_cvar_runtime::known_air::KnownAirCvar::FixSkateboardInAir,
            )
            .map_err(|e| E::Source(KnownAirTrajectoryError::Cvar(e)))?;
        put!(
            traj_collision_time,
            if fix_board {
                f32::MAX
            } else {
                need!(coll_time) - need!(time_in_state)
            }
        );
        put!(time_in_known_air, need!(time_in_state));
        put!(traj_start_position, need!(start_pos));
        let trajectory = &selector.winning_buffer.trajectory;
        let index = need!(traj_index);
        put!(
            traj_current_pos,
            position_at(trajectory, index as f32 * f32::from_bits(0x3c888889))
        );
        put!(traj_target_heading, need!(landing_heading));
        put!(traj_collision_speed, need!(coll_speed));
        put!(is_traj_valid, true);
        put!(
            trajectory_struct,
            super::simple::Trajectory {
                start: trajectory.position,
                velocity: trajectory.velocity,
                acceleration: trajectory.gravity,
                duration: trajectory.time,
            }
        );
        put!(traj_index, index);
        put!(trajectory_offset, selector.com_position_local);
        put!(traj_total_time, need!(coll_time));
        // GetLocalPosOfCOM82db6410 transforms the local point including W-axis.
        let columns = orientation.columns();
        let local = selector.com_position_local;
        let mut offset = columns[0].zip(columns[3], |a, b| a.mul_add(local.xyz[0], b));
        offset = columns[1].zip(offset, |a, b| a.mul_add(local.xyz[1], b));
        offset = columns[2].zip(offset, |a, b| a.mul_add(local.xyz[2], b));
        let start = index.wrapping_sub(1).max(0);
        for i in 0..25 {
            let position = position_at(
                trajectory,
                start.wrapping_add(i as i32) as f32 * f32::from_bits(0x3c888889),
            );
            let relative = position
                .zip(offset, |a, b| a + b)
                .zip(need!(coll_pos), |a, b| a - b);
            // Height is distance along landing normal, not world Y.
            let height = n
                .dot_vector(relative, need!(landing_normal))
                .map_err(|e| E::Source(KnownAirTrajectoryError::Numeric(e)))?;
            self.air_fill_guard().map_err(E::Output)?;
            self.concrete.air.as_mut().unwrap().height_of_com[i] = height;
        }
        Ok(())
    }
}
