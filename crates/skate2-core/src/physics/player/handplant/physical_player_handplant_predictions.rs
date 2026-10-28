//! HandPlantTrajectorySelector82dc26d8/82dbf050/82dbf6a0.
use super::HandPlantTrajectorySelector;
use crate::{
    input_cvar_runtime::{InputCvarReferences, handplant::HandPlantFloat as F},
    physical_player_footplant::RetainedTrajectory,
    physical_player_grind_manager::{
        math::{GrindNum, J, V4, cross, madd, neg1, scale, sub},
        types::GrindPrim,
        view::{error, lanes, root},
    },
    physical_player_handplant::manager::{E, GRAVITY, apex_time, normalize, sqrt},
    physical_player_trajectory_owners::offboard_runtime::{position_at, velocity_at},
};
impl HandPlantTrajectorySelector {
    /// CalculateReflectNormal82db8ec8 and LaunchPrediction82dbf050. Returns
    /// inputs for the SAME six retained future slots; no ready bit is fabricated.
    pub(crate) fn prepare(
        &mut self,
        into: RetainedTrajectory,
        spline: &GrindPrim,
        hand: V4,
        refs: &InputCvarReferences,
        n: &mut GrindNum<'_>,
    ) -> Result<[RetainedTrajectory; 6], E> {
        self.grind_primitive = spline.clone();
        self.trajectory_into = into;
        self.trajectory_out = into;
        self.hand_position = hand;
        self.ready = false;
        let normal = normalize(cross(sub(spline.end, spline.start), J), n)?;
        let sign = if n.dot(normal, lanes(into.velocity))? > 0. {
            -1.
        } else {
            1.
        };
        self.reflect_normal = scale(normal, sign);
        let t = apex_time(&into, n)?;
        let apex = lanes(position_at(&into, t));
        let v = lanes(velocity_at(&into, t));
        let speed = n.length(v)?;
        let along = n.dot(v, self.reflect_normal)?;
        let tangent = normalize(sub(v, scale(self.reflect_normal, along)), n)?;
        let offset = refs
            .try_handplant_float(F::TrajectoryStartDistance)
            .map_err(error)?;
        let mut p = madd(self.reflect_normal, offset, hand);
        p[1] = apex[1];
        let mut out = [RetainedTrajectory::constructed(); 6];
        out[0] = RetainedTrajectory {
            position: root(p),
            velocity: root(scale(tangent, speed)),
            gravity: root(GRAVITY),
            time: [-1.; 4],
        };
        let delta = refs.try_handplant_float(F::AngleDelta).map_err(error)?;
        let mut angle = delta;
        let first = out[0];
        for target in &mut out[1..] {
            let (sin, cos) = n.sin_cos(angle)?;
            // Two products before the final fused add, matching source.
            let v = madd(
                scale(self.reflect_normal, sin),
                speed,
                scale(scale(tangent, cos), speed),
            );
            *target = RetainedTrajectory {
                velocity: root(v),
                ..first
            };
            angle = delta + angle;
        }
        Ok(out)
    }
    /// SurfaceQuery completion is the synchronization gate. Negative collision
    /// time means no hit, not an unready job. Original selects the first steep
    /// landing or the smallest normal Y, then solves a new arc from its apex.
    pub(crate) fn update(
        &mut self,
        refs: &InputCvarReferences,
        n: &mut GrindNum<'_>,
    ) -> Result<bool, E> {
        let surface = self
            .surface_query
            .as_ref()
            .ok_or_else(|| error("HandPlant surface query"))?;
        if !surface.query.ready {
            return Ok(self.ready);
        }
        let time = surface
            .query
            .collision_time
            .ok_or_else(|| error("HandPlant surface time"))?[0];
        if time >= 0. {
            self.surface_id = surface
                .query
                .surface
                .ok_or_else(|| error("HandPlant surface ID"))? as i32;
        }
        let minimum = refs
            .try_handplant_float(F::MinDistanceToLanding)
            .map_err(error)?;
        let mut found = false;
        let mut best_y = 1.1;
        for query in &self.queries {
            if !query.ready {
                return Err(error("HandPlant prediction synchronization assertion"));
            }
            let result = &query.result;
            if !(result.time[0] >= 0.) {
                continue;
            }
            let position = lanes(result.position);
            if !(self.hand_position[1] - position[1] > minimum) {
                continue;
            }
            let normal = lanes(result.ground_normal);
            if normal[1] < 0.71 || best_y > normal[1] {
                self.landing_normal = normal;
                self.landing_position = position;
                found = true;
                if normal[1] < 0.71 {
                    break;
                }
                best_y = normal[1];
            }
        }
        if !found {
            self.landing_normal = J;
            self.landing_position = lanes(position_at(&self.queries[5].result.trajectory, 3.));
        }
        let t = apex_time(&self.trajectory_into, n)?;
        let apex = lanes(position_at(&self.trajectory_into, t));
        let offset = refs.try_handplant_float(F::LandingOffset).map_err(error)?;
        let end = madd(self.landing_normal, offset, self.landing_position);
        // InitializeTrajectoryStartingAtApex82db6c60, separate solve order.
        if !(apex[1] > end[1]) {
            return Err(error("HandPlant landing below apex assertion"));
        }
        let speed = sqrt((-2. * GRAVITY[1]) * (apex[1] - end[1]), n)?;
        let duration = n.reciprocal(GRAVITY[1])? * neg1(speed);
        let travelled = scale(scale(GRAVITY, duration), duration);
        let travelled = scale(travelled, n.reciprocal(2.)?);
        let velocity = scale(sub(sub(end, apex), travelled), n.reciprocal(duration)?);
        let apex_trajectory = RetainedTrajectory {
            position: root(apex),
            velocity: root(velocity),
            gravity: root(GRAVITY),
            time: [-1.; 4],
        };
        // Move the time origin backwards to the original approach's origin.
        self.trajectory_out = RetainedTrajectory {
            position: position_at(&apex_trajectory, neg1(t)),
            velocity: velocity_at(&apex_trajectory, neg1(t)),
            ..apex_trajectory
        };
        self.time_to_land = t + duration;
        self.ready = true;
        Ok(true)
    }
}
