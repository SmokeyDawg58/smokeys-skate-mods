//! CalcValues82dc1970, CalcComPos82db76c8, CalcUpAndHeading82dbd5a8.
use super::{
    HandPlantManager,
    manager::{DT, E, graph, matrix, normalize},
    position::scalar,
};
use crate::{
    input_cvar_runtime::{InputCvarReferences, handplant::HandPlantFloat as F},
    physical_player_grind_manager::{
        math::{GrindNum, V4, madd, scale, sub},
        view::{error, lanes},
    },
    physical_player_trajectory_owners::offboard_runtime::{position_at, velocity_at},
    skeleton_root_transforms::SkeletonRootMatrix,
};
fn interpolate_orientation(
    a: &[f32; 16],
    b: &[f32; 16],
    t: f32,
    n: &mut GrindNum<'_>,
) -> Result<[f32; 16], E> {
    let (matrix, _) = crate::skeleton_affine_interpolation::try_slerp_typed(
        &SkeletonRootMatrix::known(*a),
        &SkeletonRootMatrix::known(*b),
        t,
        n.n,
        n.s,
    )
    .map_err(error)?;
    Ok(matrix.xyz_scratch())
}
impl HandPlantManager {
    fn calculate_bezier(&mut self, refs: &InputCvarReferences) -> Result<(), E> {
        let window = scalar(&self.attributes, 0x2460_d654_4c3b_64b7)?;
        let frames = refs.try_handplant_float(F::ControlFrames).map_err(error)?;
        let before = self.parametric_time_at_apex[0] - window;
        let after = self.parametric_time_at_apex[0] + window;
        self.bezier[0] = lanes(position_at(&self.trajectory_into, before));
        let forward = lanes(position_at(&self.trajectory_into, before + DT));
        self.bezier[1] = madd(sub(forward, self.bezier[0]), frames, self.bezier[0]);
        self.bezier[3] = lanes(position_at(&self.trajectory_out_min, after));
        let backward = lanes(position_at(&self.trajectory_out_min, after - DT));
        self.bezier[2] = madd(sub(backward, self.bezier[3]), frames, self.bezier[3]);
        Ok(())
    }
    pub(crate) fn calc_values(
        &mut self,
        refs: &InputCvarReferences,
        n: &mut GrindNum<'_>,
    ) -> Result<(V4, V4, V4), E> {
        if !self.trajectory_ready && self.trajectory_selector.update(refs, n)? {
            let selector = &self.trajectory_selector;
            self.trajectory_result.trajectory = selector.trajectory_out;
            self.trajectory_result.landing_normal = selector.landing_normal;
            self.trajectory_result.time_to_land = selector.time_to_land;
            self.trajectory_result.result_valid = selector.ready;
            self.trajectory_result.surface_id = selector.surface_id;
            self.trajectory_out = selector.trajectory_out;
            self.trajectory_out_min = self.trajectory_out;
            if !(n.dot(self.grind_normal_out, self.grind_normal_out)? > 0.9) {
                return Err(error("HandPlant exit normal assertion"));
            }
            let out = lanes(self.trajectory_out_min.velocity);
            let speed = n.dot(self.grind_normal_out, out)?;
            let minimum = scalar(&self.attributes, 0x49c9_1803_479b_9bed)?;
            if speed < minimum {
                self.trajectory_out_min.velocity =
                    madd(self.grind_normal_out, minimum - speed, out).into();
            }
            self.surface_id = self.trajectory_result.surface_id as u32;
            let exit = lanes(position_at(
                &self.trajectory_out,
                self.parametric_time_at_apex[0] * 2.,
            ));
            self.matrices[2][12..16].copy_from_slice(&exit);
            self.calculate_bezier(refs)?;
            let land_time = self.trajectory_result.time_to_land;
            let heading = scale(
                normalize(lanes(velocity_at(&self.trajectory_out, land_time)), n)?,
                self.heading_flip_scalar[0],
            );
            self.matrices[3] = matrix(heading, self.trajectory_result.landing_normal, n)?;
            self.matrices[3][12..16]
                .copy_from_slice(&lanes(position_at(&self.trajectory_out, land_time)));
            let time = land_time - self.estimated_time_to_apex[0];
            let minimum = refs
                .try_handplant_float(F::MinTimeToGround)
                .map_err(error)?;
            self.time_from_apex_to_ground = [if time - minimum >= 0. { time } else { minimum }; 4];
            self.trajectory_ready = true;
        }
        let position = self.calc_com(n)?;
        let (up, heading) = self.calc_orientation(n)?;
        Ok((position, up, heading))
    }
    fn calc_com(&mut self, n: &mut GrindNum<'_>) -> Result<V4, E> {
        let t = self.parametric_time[0];
        let window = scalar(&self.attributes, 0x2460_d654_4c3b_64b7)?;
        let delta = t - self.parametric_time_at_apex[0];
        let u = n.reciprocal(window)?.mul_add(delta, 1.) * 0.5;
        let ideal = if 0. > u {
            lanes(position_at(&self.trajectory_into, t))
        } else if u > 1. {
            let minimum = lanes(position_at(&self.trajectory_out_min, t));
            let actual = lanes(position_at(&self.trajectory_out, t));
            let blend = (n.reciprocal(self.time_from_apex_to_ground[0])? * (delta - window))
                .max(0.)
                .min(1.);
            madd(actual, blend, scale(minimum, 1. - blend))
        } else {
            let v = 1. - u;
            let u3 = (u * u) * u;
            let v3 = (v * v) * v;
            let three = 3. * u;
            let b1 = (three * v) * v;
            let b2 = (three * u) * v;
            let [p0, p1, p2, p3] = self.bezier;
            madd(p3, u3, madd(p2, b2, madd(p0, v3, scale(p1, b1))))
        };
        let correction = scalar(&self.attributes, 0xccd7_a82c_e1bb_fff2)?;
        let actual = lanes(position_at(
            &self.trajectory_physics_start,
            self.real_time[0],
        ));
        let blend = (1. - n.reciprocal(correction)? * self.real_time[0]).max(0.);
        let result = madd(ideal, 1. - blend, scale(actual, blend));
        // Adjust approach clock only while the physics-start correction is
        // active and before the Bezier window. Clamp to half a simulation tick.
        if blend > 0. && 0. > u {
            let velocity = lanes(velocity_at(&self.trajectory_into, t));
            let direction = normalize(velocity, n)?;
            let along = n.dot(sub(result, ideal), direction)?;
            let speed = n.length(velocity)?;
            let adjustment = (n.reciprocal(speed)? * along)
                .min(f32::from_bits(0x3c08_8889))
                .max(f32::from_bits(0xbc08_8889));
            self.parametric_time = self.parametric_time.map(|t| t + adjustment);
        }
        Ok(result)
    }
    fn calc_orientation(&self, n: &mut GrindNum<'_>) -> Result<(V4, V4), E> {
        let t = self.parametric_time[0];
        let apex = self.parametric_time_at_apex[0];
        let window = scalar(&self.attributes, 0xd595_a32c_5f02_178f)?;
        let m = if apex > t {
            // Source chooses min(window, apex) before its reciprocal.
            let denominator = if window > apex { apex } else { window };
            let u = (1. - n.reciprocal(denominator)? * (apex - t)).max(0.);
            let u = graph(&self.attributes, 0xf61b_0fb7_3c39_311b, u)?;
            interpolate_orientation(&self.matrices[0], &self.matrices[1], u, n)?
        } else {
            let delta = t - apex;
            let exit = (n.reciprocal(window)? * delta).max(0.).min(1.);
            let ground = (n.reciprocal(self.time_from_apex_to_ground[0])? * delta)
                .max(0.)
                .min(1.);
            let ground = graph(&self.attributes, 0x8fc9_a4f9_002c_f9e0, ground)?;
            let exit = graph(&self.attributes, 0xd4ef_764a_f4d5_f1ca, exit)?;
            let m = interpolate_orientation(&self.matrices[2], &self.matrices[3], ground, n)?;
            interpolate_orientation(&self.matrices[1], &m, exit, n)?
        };
        Ok((m[4..8].try_into().unwrap(), m[8..12].try_into().unwrap()))
    }
}
