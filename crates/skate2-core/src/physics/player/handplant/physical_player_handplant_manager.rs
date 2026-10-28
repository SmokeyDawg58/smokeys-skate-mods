//! HandPlantManager launch/orientation/time, original82dc4140/82dbd910.
use super::{
    HandPlantManager, geometry,
    position::{PositionSelection, scalar},
};
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    input_cvar_runtime::{InputCvarReferences, handplant::HandPlantFloat},
    physical_player_footplant::RetainedTrajectory,
    physical_player_grind_manager::{
        math::{GrindNum, I, J, K, V4, ZERO, add, cross, madd, neg, neg1, scale, sub},
        types::GrindPrim,
        view::{GrindError, error, lanes, root},
    },
    physical_player_trajectory_owners::offboard_runtime::position_at,
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate, try_normalize_safe},
};
pub(crate) type E = GrindError;
pub(crate) const DT: f32 = f32::from_bits(0x3c88_8889);
pub(crate) const GRAVITY: V4 = [0., f32::from_bits(0xc11c_cccd), 0., 0.];

#[derive(Clone, Copy)]
pub(crate) struct LaunchInfo {
    pub(crate) position: V4,
    pub(crate) velocity: V4,
    pub(crate) surface_normal: V4,
    pub(crate) look: V4,
    pub(crate) heading: V4,
    pub(crate) clamp_direction: i32,
}
pub(crate) fn graph(a: &AttributeRuntimeInstance, key: u64, input: f32) -> Result<f32, E> {
    let data = a.try_get_data(key, 0).map_err(error)?;
    let mut g = crate::physics::PointGraph::<8>::default();
    if let Some(data) = data {
        let bytes = data.bytes();
        for i in 0..8 {
            for (offset, target) in [(16, &mut g.x[i]), (48, &mut g.y[i])] {
                *target = f32::from_be_bytes(
                    bytes
                        .get(offset + i * 4..offset + i * 4 + 4)
                        .ok_or_else(|| error("HandPlant graph extent"))?
                        .try_into()
                        .unwrap(),
                );
            }
        }
    }
    Ok(g.evaluate(input))
}
pub(crate) fn normalize(v: V4, n: &mut GrindNum<'_>) -> Result<V4, E> {
    try_normalize_safe(v, ZERO, n.n).map_err(error)
}
pub(crate) fn sqrt(value: f32, n: &mut GrindNum<'_>) -> Result<f32, E> {
    let estimate = n.n.reciprocal_sqrt_estimate(value).map_err(error)?;
    Ok(length_from_squared_and_estimate(value, estimate))
}
/// tTrajectory::_GetTimeAtHighestPos82db7198.
pub(crate) fn apex_time(t: &RetainedTrajectory, n: &mut GrindNum<'_>) -> Result<f32, E> {
    let vy = t.velocity.xyz[1];
    let down = neg1(t.gravity.xyz[1]);
    if vy < 0. || down <= 0. {
        return Ok(0.);
    }
    Ok(n.reciprocal(down)? * vy)
}
/// InitializeTrajectoryUsingApex82db6a58: separate products before subtract,
/// two-refinement sqrt/reciprocals, supplied duration independent of apex time.
pub(crate) fn trajectory_using_apex(
    p: V4,
    apex: V4,
    g: V4,
    duration: f32,
    n: &mut GrindNum<'_>,
) -> Result<RetainedTrajectory, E> {
    if !(apex[1] > p[1]) || g[1] == 0. {
        return Err(error("HandPlant apex trajectory assertion"));
    }
    let speed = sqrt((-2. * g[1]) * (apex[1] - p[1]), n)?;
    let t = n.reciprocal(g[1])? * neg1(speed);
    let half_g = scale(g, n.reciprocal(2.)?);
    let travelled = scale(scale(half_g, t), t);
    let v = scale(sub(sub(apex, p), travelled), n.reciprocal(t)?);
    Ok(RetainedTrajectory {
        position: root(p),
        velocity: root(v),
        gravity: root(g),
        time: [duration; 4],
    })
}
/// CreateMtxFromHeadingAndUp82db5878. Native translation is the unnormalized
/// second cross-product scratch; callers overwrite it after orientation setup.
pub(crate) fn matrix(mut heading: V4, up: V4, n: &mut GrindNum<'_>) -> Result<[f32; 16], E> {
    if n.dot(heading, up)?.abs() > 0.99999 {
        heading = I;
        if n.dot(heading, up)?.abs() > 0.99999 {
            heading = K;
        }
    }
    let x = normalize(cross(up, heading), n)?;
    let p = cross(x, up);
    let z = normalize(p, n)?;
    let y = n.normalize_unsafe(cross(z, x))?;
    Ok(std::array::from_fn(|i| [x, y, z, p][i / 4][i % 4]))
}
pub(crate) fn direction(info: &LaunchInfo, n: &mut GrindNum<'_>) -> Result<i32, E> {
    let m = geometry::vertical_plane(info.surface_normal, info.position, n)?;
    let v = geometry::transform_vector(geometry::inverse(m), info.velocity);
    Ok(if v[0] > 0. { 1 } else { 2 })
}
impl HandPlantManager {
    /// Called after the original Reset and ShouldLaunch gates, with the actual
    /// world GrindData result, not an invented target or a second spline owner.
    pub(crate) fn launch_selected(
        &mut self,
        info: &LaunchInfo,
        found: PositionSelection,
        previous: V4,
        refs: &InputCvarReferences,
        n: &mut GrindNum<'_>,
    ) -> Result<(), E> {
        let distance = scalar(&self.attributes, 0xa046_d051_6c0b_d62b)?;
        let angle = scalar(&self.attributes, 0x90cd_d541_098a_b4b5)?;
        let (sine, cosine) = n.sin_cos(angle)?;
        let tangent = sub(found.primitive.end, found.primitive.start);
        let normal = cross(tangent, J);
        // Source vsel chooses -1 when dot>0, +1 otherwise.
        let sign = if n.dot(normal, info.surface_normal)? > 0. {
            -1.
        } else {
            1.
        };
        let inward = scale(normalize(normal, n)?, sign);
        self.grind_normal_out = neg(inward);
        if !(n.dot(self.grind_normal_out, self.grind_normal_out)? > 0.9) {
            return Err(error("HandPlant grind normal assertion"));
        }
        let offset = madd(inward, distance * sine, scale(J, distance * cosine));
        let apex = add(found.point, offset);
        self.trajectory_into = trajectory_using_apex(info.position, apex, GRAVITY, -1., n)?;
        self.parametric_time = [0.; 4];
        self.parametric_time_at_apex = [apex_time(&self.trajectory_into, n)?; 4];
        self.grind_spline = GrindPrim::from_query(&found.primitive);
        self.position_result.found = true;
        self.position_result.position = found.point;
        self.position_result.direction_vs_up = found.direction;
        self.position_result.primitive = self.grind_spline.clone();
        self.trajectory_out = self.trajectory_into;
        self.trajectory_out_min = self.trajectory_into;
        let down = [0., -1., 0., 0.];
        self.up_vectors = [info.surface_normal, down, info.surface_normal];
        let apex = lanes(position_at(
            &self.trajectory_into,
            self.parametric_time_at_apex[0],
        ));
        self.matrices[0] = matrix(info.heading, info.surface_normal, n)?;
        let flip = if n.dot(info.heading, info.velocity)? > 0. {
            1.
        } else {
            -1.
        };
        self.heading_flip_scalar = [flip; 4];
        let tangent = normalize(neg(tangent), n)?;
        let mut flat = tangent;
        flat[1] = 0.;
        let facing = if n.dot(flat, sub(apex, info.position))? > 0. {
            1.
        } else {
            -1.
        };
        let blend = refs
            .try_handplant_float(HandPlantFloat::ApexUpBlend)
            .map_err(error)?;
        let up = normalize(
            madd(sub(found.point, apex), blend, scale(down, 1. - blend)),
            n,
        )?;
        self.matrices[1] = matrix(scale(tangent, facing * flip), up, n)?;
        self.matrices[2] = matrix(scale(down, flip), info.surface_normal, n)?;
        // Native computes and discards a ground-normal candidate here. The
        // ground matrix is produced only when the landing prediction returns.
        let mut ground = info.surface_normal;
        ground[1] = refs
            .try_handplant_float(HandPlantFloat::GroundNormalY)
            .map_err(error)?;
        let _ = normalize(ground, n)?;
        self.matrices[1][12..16].copy_from_slice(&apex);
        self.matrices[0][12..16].copy_from_slice(&lanes(position_at(&self.trajectory_into, 0.)));
        let estimated = self.estimate_time_to_apex()?;
        self.animation.position = found.point;
        self.animation.estimated_time_to_apex = estimated;
        let changed = n.length(sub(previous, found.point))? > 0.15;
        let behind = n.dot(sub(found.point, info.position), info.look)? < 0.;
        let imminent = estimated < scalar(&self.attributes, 0xb5ec_5a09_41b4_07a4)?;
        self.animation.flags = (self.animation.flags & 0x0fff_ffff)
            | 0x8000_0000
            | ((changed as u32) << 30)
            | ((behind as u32) << 29)
            | ((imminent as u32) << 28);
        Ok(())
    }
    pub(crate) fn estimate_time_to_apex(&mut self) -> Result<f32, E> {
        let delta = self.parametric_time[0] - self.parametric_time_at_apex[0];
        let mut t = delta;
        let step = (delta * f32::from_bits(0x3eaa_aaab)).abs();
        let mut sum = 0.;
        for _ in 0..4 {
            sum = (graph(&self.attributes, 0xf551_df32_8912_4fb0, t)? + 1.) + sum;
            t += step;
        }
        let mean = sum * 0.25;
        if mean != 0. {
            self.estimated_time_to_apex = [(-1. / mean) * delta; 4];
        }
        Ok(self.estimated_time_to_apex[0])
    }
    pub(crate) fn advance_time(&mut self) -> Result<f32, E> {
        let rate = graph(
            &self.attributes,
            0xf551_df32_8912_4fb0,
            self.parametric_time[0] - self.parametric_time_at_apex[0],
        )? + 1.;
        self.parametric_time = self.parametric_time.map(|t| t + rate * DT);
        let estimated = self.estimate_time_to_apex()?;
        self.animation.estimated_time_to_apex = estimated;
        // Raw82dbd9e0 reloads kUpdateTime: real time is NOT graph-scaled.
        self.real_time = self.real_time.map(|t| t + DT);
        Ok(estimated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn solved_arc_hits_apex_with_zero_vertical_speed() {
        use crate::{
            physical_player_trajectory_owners::offboard_runtime::velocity_at,
            reckoning_numeric::HostReckoningNumeric, reckoning_sin_cos::HostReckoningSinCosNumeric,
        };
        let mut host = HostReckoningNumeric;
        let mut trig = HostReckoningSinCosNumeric;
        let mut n = GrindNum {
            n: &mut host,
            s: &mut trig,
        };
        let p = [4., 1., -3., 0.];
        let top = [7., 3., -2., 0.];
        let trajectory = trajectory_using_apex(p, top, GRAVITY, -1., &mut n).unwrap();
        let time = apex_time(&trajectory, &mut n).unwrap();
        let actual = lanes(position_at(&trajectory, time));
        for i in 0..3 {
            assert!((actual[i] - top[i]).abs() < 1e-4);
        }
        assert!(velocity_at(&trajectory, time).xyz[1].abs() < 1e-4);
        assert!(time > 0.);
    }
}
