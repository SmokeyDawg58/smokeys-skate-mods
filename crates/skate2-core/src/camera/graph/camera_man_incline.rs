//! `CameraMan::UpdateInclineAngle` 0x82e41420, same retained Subject and angles.
use super::{CameraTrajectoryError, is_on_ground, math::flat_basis};
use crate::{
    camera_compass::runtime::math::{camera_angle, camera_atan2_pending, euler_angles},
    camera_man::CameraMan,
    debug_overlay::DebugOverlaySlot,
    input_cvar_runtime::InputCvarReferences,
    reckoning_numeric::{
        ReckoningNumeric, cross_typed as cross, try_asin, try_normalize_unsafe, try_unsigned_angle,
    },
    skeleton_root_transforms::SkeletonRootVector as V,
    skeleton_two_bone::try_inverse_basis_from_cofactors,
};
const UP: V = V::known([0.0, 1.0, 0.0, 0.0]);

fn stationary(velocity: V) -> bool {
    //41570..58c substitutes abs(X) for W, then CR6 all-false from >epsilon.
    !velocity
        .xyz
        .into_iter()
        .any(|v| v.abs() > f32::from_bits(0x3400_0000))
}
fn madd(v: V, scalar: f32, add: V) -> V {
    v.zip(add, |a, b| a.mul_add(scalar, b))
}
fn inverse<N: ReckoningNumeric>([x, y, z]: [V; 3], numeric: &mut N) -> Result<[V; 3], N::Error> {
    try_inverse_basis_from_cofactors(x, [cross(y, z), cross(z, x), cross(x, y)], numeric)
}
fn transform([x, y, z]: [V; 3], value: V) -> V {
    let first = x.map(|v| v * value.xyz[0]);
    let second = madd(y, value.xyz[1], first);
    madd(z, value.xyz[2], second)
}
fn slope_inverse<N: ReckoningNumeric>(
    side: V,
    normal: V,
    slope_up: V,
    numeric: &mut N,
) -> Result<[V; 3], N::Error> {
    //41770 uses dot(side,side) directly, not a recomputed cross(normal,slopeUp).
    try_inverse_basis_from_cofactors(
        side,
        [side, cross(slope_up, side), cross(side, normal)],
        numeric,
    )
}
fn frontside<N: ReckoningNumeric>(
    basis: [V; 3],
    velocity: V,
    still: bool,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    // Negative-zero affine translation executes before optional normalization.
    let translated = basis[0].map(|v| -0.0 * v);
    let translated = madd(basis[1], -0.0, translated);
    let translated = madd(basis[2], -0.0, translated);
    let direction = if still {
        UP
    } else {
        try_normalize_unsafe(velocity, numeric)?
    };
    let first = madd(basis[0], direction.xyz[0], translated);
    let second = madd(basis[1], direction.xyz[1], first);
    let value = madd(basis[2], direction.xyz[2], second);
    Ok(camera_atan2_pending(value.xyz[0], value.xyz[2], numeric)?.finish())
}

impl CameraMan {
    pub(crate) fn try_update_incline<N: ReckoningNumeric>(
        &mut self,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), CameraTrajectoryError<N::Error>> {
        use CameraTrajectoryError as E;
        let owner = self.subject.as_ref().ok_or(E::MissingSubject)?;
        let subject = owner.lock().map_err(|_| E::OwnerLock)?;
        self.incline_normal = subject.ground_normal; //41454
        if subject.last_valid_ground_normal.xyz[1].partial_cmp(&f32::from_bits(0x3f7f_ff58))
            == Some(std::cmp::Ordering::Greater)
            && !subject.flags.grinding
        {
            self.incline_angle = 0.0;
            self.absolute_incline_angle = 0.0;
            self.frontside_angle = 0.0;
            self.incline_alignment = 0.0;
            self.flags &= !0x10;
            self.incline_downhill_heading = 0.0;
            self.grind_incline = 0.0;
        } else {
            let flat = flat_basis(&subject, numeric).map_err(E::Numeric)?; //41514
            let normal = subject.last_valid_ground_normal;
            let component = numeric.dot_vector(UP, normal).map_err(E::Numeric)?; //41564
            let still = stationary(self.anchor_velocity);
            let projected = normal.map(|v| v * component);
            let slope_up = try_normalize_unsafe(UP.zip(projected, |a, b| a - b), numeric)
                .map_err(E::Numeric)?;
            let side = cross(normal, slope_up);
            let flat_inverse = inverse(flat, numeric).map_err(E::Numeric)?;
            let local_normal = transform(flat_inverse, subject.last_valid_ground_normal);
            let angle = -try_asin(local_normal.xyz[2], numeric).map_err(E::Numeric)?;
            self.incline_angle = angle; //41700
            self.absolute_incline_angle = if angle < 0.0 { -angle } else { angle }; //41728
            self.incline_downhill_heading = euler_angles(subject.ground_normal, numeric)
                .map_err(E::Numeric)?
                .1;
            if is_on_ground(&subject, self.rig.subject_state) {
                let basis = slope_inverse(side, normal, slope_up, numeric).map_err(E::Numeric)?;
                self.frontside_angle =
                    frontside(basis, self.anchor_velocity, still, numeric).map_err(E::Numeric)?;
                self.incline_alignment = self.frontside_angle.abs();
                self.flags = (self.flags & !0x10) | (u8::from(self.frontside_angle > 0.0) << 4);
                if self.flags & 2 != 0 {
                    self.frontside_angle *= -1.0;
                }
            }
            if !still {
                self.grind_incline =
                    try_unsigned_angle(subject.grind_direction, UP, numeric).map_err(E::Numeric)?;
                self.grind_incline -= f32::from_bits(0x3fc9_0fdb);
                self.grind_incline = -self.grind_incline;
                self.grind_incline =
                    camera_angle(self.grind_incline, numeric).map_err(E::Numeric)?;
            }
        }
        if cvars.try_camera_show_frontside().map_err(E::Cvar)? {
            overlay.require().map_err(E::DebugOverlay)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;
    #[test]
    fn velocity_gate_ignores_w_and_preserves_unordered_lanes() {
        assert!(stationary(V::known([0.0, -0.0, 0.0, f32::INFINITY])));
        assert!(stationary(V::known([f32::NAN, 0.0, 0.0, 0.0])));
        assert!(stationary(V::known([
            f32::from_bits(0x3400_0000),
            0.0,
            0.0,
            0.0
        ])));
        assert!(!stationary(V::known([0.0, 0.0001, 0.0, 0.0])));
    }
    #[test]
    fn slope_inverse_retains_uphill_and_sideways_heading() {
        let normal = V::known([0.0, 0.8, -0.6, 0.0]);
        let slope_up = V::known([0.0, 0.6, 0.8, 0.0]);
        let side = cross(normal, slope_up);
        let basis = slope_inverse(side, normal, slope_up, &mut HostReckoningNumeric).unwrap();
        let still = frontside(basis, V::known([0.0; 4]), true, &mut HostReckoningNumeric).unwrap();
        assert!(still.abs() < 0.0001);
        let sideways = frontside(
            basis,
            V::known([3.0, 0.0, 0.0, 0.0]),
            false,
            &mut HostReckoningNumeric,
        )
        .unwrap();
        assert_eq!(sideways.to_bits(), 0x3fc9_0fdb);
    }
    #[test]
    fn incline_normal_uses_full_inverse_without_affine_translation() {
        let flat = [
            V::known([2.0, 0.0, 0.0, 0.0]),
            UP,
            V::known([0.0, 0.0, 4.0, 0.0]),
        ];
        let value = transform(
            inverse(flat, &mut HostReckoningNumeric).unwrap(),
            V::known([0.0, 0.0, 2.0, 0.0]),
        );
        assert_eq!(value.xyz[2], 0.5);
    }
}
