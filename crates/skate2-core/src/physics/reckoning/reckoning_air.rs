//! Retained Reckoning.UpdateAirStates82ddfde8..82de0244.
//! Uses the actual common filters/BodySpin/transform, with Air tilt graph offsets.
use super::{
    Reckoning, ReckoningCvars, ReckoningDebug, ReckoningError, Scope, Vector, ZERO, blend_fma,
    debug_off, grounded_input, numeric, scale, sub,
};
use crate::reckoning_numeric::ReckoningNumeric;
use crate::{
    reckoning_body_spin::air::{BodySpinAirAttributes, BodySpinAirCvars, BodySpinAirError},
    reckoning_sin_cos::{ReckoningSinCosNumeric, try_sin_cos},
};
use grounded_input::{GroundInputRead, GroundScalar, GroundVector};
#[derive(Debug)]
pub(crate) enum ReckoningAirError<A, C, N, S> {
    Reckoning(ReckoningError<A, C, N>),
    BodySpin(BodySpinAirError<A, C>),
    Vector(AirVectorError<N, S>),
    SinCos(S),
}
#[derive(Debug)]
pub(crate) enum AirVectorError<N, S> {
    Numeric(N),
    SinCos(S),
}

impl<A: BodySpinAirAttributes> Reckoning<A> {
    /// UpdateHandPlant82de0248. Uses the same BodySpin, normal filter,
    /// ClampVectorWithinAngleRange and Air tilt graphs as the original caller.
    pub(crate) fn try_update_handplant_live<
        C: ReckoningCvars + BodySpinAirCvars,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
    >(
        &mut self,
        input: &impl GroundInputRead,
        normal: Vector,
        blend: f32,
        heading: Vector,
        maximum_delta: f32,
        c: &mut C,
        n: &mut N,
        s: &mut S,
    ) -> Result<(), ReckoningAirError<A::Error, C::Error, N::Error, S::Error>> {
        use ReckoningAirError as E;
        self.heading = heading;
        self.body_spin
            .try_update_air(0., 0., &mut self.attributes, c)
            .map_err(E::BodySpin)?;
        self.dynamic_up = self.up;
        self.dynamic_lean = 0.;
        self.ground_blend = 0.;
        let old = self.up;
        self.update_ground_normal::<C::Error, N>(normal, n)
            .map_err(E::Reckoning)?;
        let blended = blend_fma(old, 1. - blend, scale(self.ground_normal, blend));
        let squared = n
            .dot_vector(blended, blended)
            .map_err(|e| E::Reckoning(ReckoningError::Numeric(e)))?;
        let seed = n
            .reciprocal_sqrt_estimate(squared)
            .map_err(|e| E::Reckoning(ReckoningError::Numeric(e)))?;
        let target = numeric::normalize_from_squared_and_estimate(blended, squared, seed);
        let bounded =
            try_clamp_air_angle(target, self.up, maximum_delta, n, s).map_err(E::Vector)?;
        let next = numeric::try_normalize_safe(bounded, self.up, n)
            .map_err(|e| E::Reckoning(ReckoningError::Numeric(e)))?;
        self.up_speed = sub(next, self.up);
        self.up = next;
        self.up_target = next;
        self.maintain_filters();
        self.calculate_transform(n)
            .map_err(|e| E::Reckoning(ReckoningError::Numeric(e)))?;
        self.calculate_tilt_with_graphs(input, c, n, 0x360, 0x2e0)
            .map_err(E::Reckoning)
    }
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "source call operands and independent checked service errors"
    )]
    pub(crate) fn try_update_air_live<
        C: ReckoningCvars + BodySpinAirCvars,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
    >(
        &mut self,
        input: &impl GroundInputRead,
        ground_normal: Vector,
        normal_blend: f32,
        target_spin: f32,
        target_flip: f32,
        c: &mut C,
        n: &mut N,
        s: &mut S,
        #[cfg(feature = "skate3-runtime")]
        mod_tuning: Option<&crate::skate3_native::physics::player::tuning::Tuning>,
    ) -> Result<(), ReckoningAirError<A::Error, C::Error, N::Error, S::Error>> {
        use ReckoningAirError as E;
        self.dynamic_up = self.up; //fe28..34 before either scalar clear.
        self.dynamic_lean = 0.0;
        self.ground_blend = 0.0;
        let old_up = self.up;
        self.update_ground_normal::<C::Error, N>(ground_normal, n)
            .map_err(E::Reckoning)?;
        let spin_input = if input
            .flag8c8(0x4000_0000)
            .map_err(|e| E::Reckoning(ReckoningError::Input(e)))?
        {
            self.spin_speed = target_spin;
            self.spin_angle = target_spin.mul_add(f32::from_bits(0x3c88_8889), self.spin_angle);
            None
        } else {
            let value = input
                .scalar(GroundScalar::BodySpin)
                .map_err(|e| E::Reckoning(ReckoningError::Input(e)))?;
            #[cfg(feature = "skate3-runtime")]
            self.body_spin
                .try_update_selected_air(value, target_spin, &mut self.attributes, c, mod_tuning)
                .map_err(E::BodySpin)?;
            #[cfg(not(feature = "skate3-runtime"))]
            self.body_spin
                .try_update_air(value, target_spin, &mut self.attributes, c)
                .map_err(E::BodySpin)?;
            let speed = self.body_spin.speed();
            let angle = speed.mul_add(f32::from_bits(0x3c88_8889), self.spin_angle);
            self.spin_speed = speed;
            self.spin_angle = angle;
            Some(value)
        };
        #[cfg(feature = "skate3-runtime")]
        let source_flip = self.is_flipping && mod_tuning.is_some_and(|t| t.perfect_body_flips);
        #[cfg(not(feature = "skate3-runtime"))]
        let source_flip = false;
        #[cfg(feature = "skate3-runtime")]
        if source_flip {
            self.update_source_body_flip(target_flip, mod_tuning.unwrap());
        }
        if self.is_flipping && !source_flip {
            // UpdateBodyFlip82dd8410: live FETCH order, then two retained rotations.
            self.target_flip_speed = target_flip;
            let pointer = self
                .attributes
                .lookup(Scope::Own, 0x26e7_322c_cdc3_fe23)
                .map_err(|e| E::Reckoning(ReckoningError::Attribute(e)))?;
            let blend = self
                .attributes
                .float(&pointer)
                .map_err(|e| E::Reckoning(ReckoningError::Attribute(e)))?;
            self.flip_speed = (target_flip - self.flip_speed).mul_add(blend, self.flip_speed);
            let pointer = self
                .attributes
                .lookup(Scope::Own, 0x3d95_c06d_c6e3_a002)
                .map_err(|e| E::Reckoning(ReckoningError::Attribute(e)))?;
            let maximum = self
                .attributes
                .float(&pointer)
                .map_err(|e| E::Reckoning(ReckoningError::Attribute(e)))?;
            let pointer = self
                .attributes
                .lookup(Scope::Own, 0x8976_3e47_7128_71c3)
                .map_err(|e| E::Reckoning(ReckoningError::Attribute(e)))?;
            let coefficient = self
                .attributes
                .float(&pointer)
                .map_err(|e| E::Reckoning(ReckoningError::Attribute(e)))?;
            let spin_angle = self.spin_angle * coefficient;
            let speed = if -maximum - self.flip_speed >= 0.0 {
                -maximum
            } else {
                self.flip_speed
            };
            self.flip_speed = if maximum - speed >= 0.0 {
                speed
            } else {
                maximum
            };
            let dt = input
                .scalar(GroundScalar::TimeDelta)
                .map_err(|e| E::Reckoning(ReckoningError::Input(e)))?;
            self.flip_angle = dt.mul_add(self.flip_speed, self.flip_angle);
            let (sin, cos) = try_sin_cos(spin_angle, s).map_err(E::SinCos)?;
            self.body_spin_transform = axis_rotation(self.up, sin, cos);
            if self.flip_angle != 0.0 {
                let (sin, cos) = try_sin_cos(self.flip_angle, s).map_err(E::SinCos)?;
                self.flip_transform =
                    axis_rotation(self.flip_axis, sin, cos).compose(&self.body_spin_transform);
            }
        }
        let blended = blend_fma(
            old_up,
            1.0 - normal_blend,
            scale(self.ground_normal, normal_blend),
        );
        let q = n
            .dot_vector(blended, blended)
            .map_err(|e| E::Reckoning(ReckoningError::Numeric(e)))?;
        let seed = n
            .reciprocal_sqrt_estimate(q)
            .map_err(|e| E::Reckoning(ReckoningError::Numeric(e)))?;
        let target = numeric::normalize_from_squared_and_estimate(blended, q, seed);
        let com = input
            .vector(GroundVector::ComToDeck)
            .map_err(|e| E::Reckoning(ReckoningError::Input(e)))?;
        let length =
            numeric::try_length(com, n).map_err(|e| E::Reckoning(ReckoningError::Numeric(e)))?;
        let limit = self
            .graph(Scope::Own, 0x160, length)
            .map_err(|e| E::Reckoning(ReckoningError::Attribute(e)))?;
        let angle = numeric::try_unsigned_angle(target, self.up, n)
            .map_err(|e| E::Reckoning(ReckoningError::Numeric(e)))?;
        debug_off::<A::Error, C, N::Error>(c, ReckoningDebug::Reckoning).map_err(E::Reckoning)?;
        let bounded = if angle > limit {
            try_clamp_air_angle(target, self.up, limit, n, s).map_err(E::Vector)?
        } else {
            target
        };
        let next = numeric::try_normalize_safe(bounded, self.up, n)
            .map_err(|e| E::Reckoning(ReckoningError::Numeric(e)))?;
        self.up_speed = sub(next, self.up); //010c before up and target stores.
        self.up = next;
        self.up_target = next;
        // Source rereads flipping here. The ordinary retained false branch rotates.
        if !self.is_flipping {
            let dt = input
                .scalar(GroundScalar::TimeDelta)
                .map_err(|e| E::Reckoning(ReckoningError::Input(e)))?;
            let (sin, cos) = try_sin_cos(dt * self.spin_speed, s).map_err(E::SinCos)?;
            self.heading = rotate_heading(self.heading, self.up, sin, cos);
        }
        if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
            eprintln!(
                "known air: reckoning spin anim_body_spin={spin_input:?} target={target_spin} speed={} angle={} flipping={} heading={:?}",
                self.spin_speed, self.spin_angle, self.is_flipping, self.heading.xyz
            );
        }
        // MaintainFilters8248 updates PID histories BEFORE overriding their values.
        self.maintain_filters();
        self.calculate_transform(n)
            .map_err(|e| E::Reckoning(ReckoningError::Numeric(e)))?;
        self.calculate_tilt_with_graphs(input, c, n, 0x360, 0x2e0)
            .map_err(E::Reckoning)?;
        Ok(())
    }
}

/// Raw0158..0208 constructs three VMX columns then XYZ multiply/FMA. Its W
/// comes from permute<0,5,0,0>, not an invented affine zero.
fn rotate_heading(heading: Vector, axis: Vector, sin: f32, cos: f32) -> Vector {
    let rotation = axis_rotation(axis, sin, cos);
    let [column_x, column_y, column_z, _] = *rotation.columns();
    blend_fma(
        column_z,
        heading.xyz[2],
        blend_fma(column_y, heading.xyz[1], scale(column_x, heading.xyz[0])),
    )
}
fn axis_rotation(axis: Vector, sin: f32, cos: f32) -> super::Matrix {
    let [x, y, z] = axis.xyz;
    let one_minus_cosine = 1.0 - cos;
    let tx = one_minus_cosine * x;
    let ty = one_minus_cosine * y;
    let tz = one_minus_cosine * z;
    let sx = sin * x;
    let sy = sin * y;
    let sz = sin * z;
    let xx = tx.mul_add(x, cos);
    let xy = tx.mul_add(y, sz);
    let xz = tx * z - sy;
    let yx = ty * x - sz;
    let yy = ty.mul_add(y, cos);
    let yz = ty.mul_add(z, sx);
    let zx = tz.mul_add(x, sy);
    let zy = tz * y - sx;
    let zz = tz.mul_add(z, cos);
    let column_x = Vector::known([xx, xy, xz, xx]);
    let column_y = Vector::known([yx, yy, yz, yx]);
    let column_z = Vector::known([zx, zy, zz, zx]);
    super::Matrix::from_columns([column_x, column_y, column_z, Vector::known(ZERO)])
}

impl<A: crate::reckoning_attributes::ReckoningAttributeRead> Reckoning<A> {
    /// StartFlipping82dd87d0; flip axis is transform.X plus authored Y/Z terms.
    pub(crate) fn try_start_flipping<N: ReckoningNumeric>(
        &mut self,
        input: &crate::skeleton_processed_contact::SkeletonProcessedContact,
        forward: bool,
        n: &mut N,
    ) -> Result<(), ReckoningError<A::Error, std::convert::Infallible, N::Error>> {
        if self.is_flipping {
            return Ok(());
        }
        self.flip_transform = super::Matrix::known(super::IDENTITY);
        self.flip_speed = 0.0;
        self.flip_angle = 0.0;
        let mut basis = *self.transform.columns();
        let flipped =
            input
                .toolkit_state()
                .flipped_controls_scalar()
                .ok_or(ReckoningError::Input(
                    grounded_input::GroundInputError::Missing("flipped controls scalar"),
                ))?;
        if flipped < 0.0 {
            basis[0] = basis[0].map(|v| -v);
            basis[2] = basis[2].map(|v| -v);
        }
        let layout = self
            .attributes
            .capture_layout(Scope::Own)
            .map_err(ReckoningError::Attribute)?;
        let mut authored = [0.0; 4];
        for (i, v) in authored.iter_mut().enumerate() {
            *v = self
                .attributes
                .layout_float(&layout, i * 4)
                .map_err(ReckoningError::Attribute)?;
        }
        // 82dd8880 reads mBodySpin (+960), not the air states' mPhysBodySpin (+9f8).
        let amount = input
            .animation_attribute_fields()
            .body_spin
            .ok_or(ReckoningError::Input(grounded_input::GroundInputError::Missing(
                "Processed body spin+960",
            )))?
            * 4.0;
        let positive = if -amount >= 0.0 { 0.0 } else { amount };
        let amount = if 1.0 - positive >= 0.0 { positive } else { 1.0 };
        let authored = authored.map(|v| v * amount);
        self.flip_axis = blend_fma(
            basis[2],
            authored[2],
            blend_fma(basis[1], authored[1], basis[0]),
        );
        self.flip_axis = numeric::try_normalize_safe(self.flip_axis, Vector::known(ZERO), n)
            .map_err(ReckoningError::Numeric)?;
        if forward {
            self.flip_axis = self.flip_axis.map(|v| -v);
        }
        self.is_front_flipping = forward;
        self.is_flipping = true;
        Ok(())
    }
}

/// PhysicsUtility.ClampVectorWithinAngleRange82e49fd0. Reuses source safe
/// normalize/ACos/angle normalization/rsqrt and polynomial sincos primitives.
pub(crate) fn try_clamp_air_angle<N: ReckoningNumeric, S: ReckoningSinCosNumeric>(
    vector: Vector,
    target: Vector,
    range: f32,
    numeric_service: &mut N,
    rounding: &mut S,
) -> Result<Vector, AirVectorError<N::Error, S::Error>> {
    use AirVectorError as E;
    let unit_vector = numeric::try_normalize_safe(vector, Vector::known(ZERO), numeric_service)
        .map_err(E::Numeric)?;
    let unit_target = numeric::try_normalize_safe(target, Vector::known(ZERO), numeric_service)
        .map_err(E::Numeric)?;
    let dot = numeric_service
        .dot_vector(unit_vector, unit_target)
        .map_err(E::Numeric)?;
    let lower = if dot < -1.0 { -1.0 } else { dot };
    let bounded = if lower > 1.0 { 1.0 } else { lower };
    let axis = numeric::cross_typed(unit_vector, unit_target);
    let angle = numeric::try_acos(bounded, numeric_service).map_err(E::Numeric)?;
    let angle = numeric::try_normalize_angle(angle, numeric_service).map_err(E::Numeric)?;
    if angle.abs() < range {
        return Ok(vector);
    }
    let axis_squared = numeric_service.dot_vector(axis, axis).map_err(E::Numeric)?;
    if f32::from_bits(0x3780_0000) > axis_squared {
        return Ok(vector);
    }
    let seed = numeric_service
        .reciprocal_sqrt_estimate(axis_squared)
        .map_err(E::Numeric)?;
    let (sin, cos) = try_sin_cos((-range) * 0.5, rounding).map_err(E::SinCos)?;
    let magnitude_squared = numeric_service
        .dot_vector(vector, vector)
        .map_err(E::Numeric)?;
    let magnitude_seed = numeric_service
        .reciprocal_sqrt_estimate(magnitude_squared)
        .map_err(E::Numeric)?;
    let unit = numeric::normalize_from_squared_and_estimate(axis, axis_squared, seed);
    let mut quaternion = scale(unit, sin);
    quaternion.w = Some(cos);
    let cross = numeric::cross_typed(quaternion, target);
    let inner = blend_fma(target, cos, cross);
    let outer = numeric::cross_typed(quaternion, inner);
    let rotated = blend_fma(outer, 2.0, target);
    let magnitude = numeric::length_from_squared_and_estimate(magnitude_squared, magnitude_seed);
    Ok(scale(rotated, magnitude))
}

#[cfg(test)]
#[path = "reckoning_air_tests.rs"]
mod tests;

#[cfg(feature = "skate3-runtime")]
#[path = "reckoning_air_skate3.rs"]
mod skate3_assists;
