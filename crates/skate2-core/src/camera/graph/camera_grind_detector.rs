//! `Sk8::Camera::GrindDetector` (vtable 0x82386c80), run by `CameraMan::UpdateGrindDetection`
//! 0x82e3e8b0 for Grind Transparency: the nearest rail ahead inside a frustum along the
//! skater's travel, and the anticipation that fades the skater out while their hips block the
//! camera's view of it.
use super::{CameraGrindUpdateError, CameraPreUpdateContinuationError as E};
use crate::{
    ai::vmx::{self, V4},
    camera_attributes::{CameraAttributes, CameraParameterPublication},
    camera_compass::runtime::math::{
        camera_angle, camera_atan2_pending, euler_angles, unit_fraction,
    },
    camera_drop::CameraGrindDetector,
    camera_frame::composition_math::framing_basis,
    camera_parameters::CameraParameters,
    grind_data_octree::OctreeBounds,
    reckoning_numeric::{
        ReckoningNumeric, ReckoningVector, normalize_from_squared_and_estimate, try_asin,
        try_length, try_normalize_safe, try_normalize_unsafe,
    },
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::cmp::Ordering;

const PI: f32 = f32::from_bits(0x4049_0fdb);
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);
/// `UpdateGrindAnticipation`'s radius about the camera's line to the rail (0.3).
const OBSCURED_RADIUS: f32 = f32::from_bits(0x3e99_999a);

/// `GrindData::BoxTest` without vertical primitives ignored: the start and end of each primitive
/// in the box, at most 40.
pub(crate) type GrindBoxTest<'a> =
    dyn FnMut(OctreeBounds) -> Result<Vec<[V4; 2]>, CameraGrindUpdateError> + 'a;

/// The `GameCameraParameters` fields the detector reads (`mParameters`, +4), angles in radians.
struct Parameters {
    min_lookahead: f32,
    max_lookahead: f32,
    lookahead_time: f32,
    horizontal: f32,
    vertical: f32,
    max_relative_angle: f32,
    skateboard_offset: f32,
    max_relative_height: f32,
}

pub(super) fn fields(
    parameters: &CameraAttributes,
) -> Result<&CameraParameters, CameraGrindUpdateError> {
    match &parameters.parameters {
        CameraParameterPublication::Fields(fields) => Ok(fields),
        CameraParameterPublication::Unwritten => Err(CameraGrindUpdateError::Unwritten("camera")),
    }
}

pub(super) fn field(value: Option<f32>, name: &'static str) -> Result<f32, CameraGrindUpdateError> {
    value.ok_or(CameraGrindUpdateError::Unwritten(name))
}

impl CameraGrindDetector {
    fn try_parameters(&self) -> Result<Parameters, CameraGrindUpdateError> {
        let parameters = self
            .parameters
            .lock()
            .map_err(|_| CameraGrindUpdateError::OwnerLock)?;
        let p = fields(&parameters)?;
        Ok(Parameters {
            min_lookahead: field(
                p.grind_min_lookahead_distance,
                "grind_min_lookahead_distance",
            )?,
            max_lookahead: field(
                p.grind_max_lookahead_distance,
                "grind_max_lookahead_distance",
            )?,
            lookahead_time: field(p.grind_lookahead_time, "grind_lookahead_time")?,
            horizontal: field(
                p.grind_frustum_angle_horizontal,
                "grind_frustum_angle_horizontal",
            )? * DEGREES_TO_RADIANS,
            vertical: field(
                p.grind_frustum_angle_vertical,
                "grind_frustum_angle_vertical",
            )? * DEGREES_TO_RADIANS,
            max_relative_angle: field(p.grind_max_relative_angle, "grind_max_relative_angle")?
                * DEGREES_TO_RADIANS,
            skateboard_offset: field(p.grind_skateboard_offset, "grind_skateboard_offset")?,
            max_relative_height: field(p.grind_max_relative_height, "grind_max_relative_height")?,
        })
    }

    /// `GrindDetector::Update` (0x82e385e8): forgets the last rail, then, unless the skater is
    /// grinding or Grind Transparency is off, looks `mGrindLookaheadTime` of `speed` ahead
    /// (within `mGrindMin/MaxLookaheadDistance`) from the frustum raised `mGrindSkateboardOffset`,
    /// and turns the rail it finds into the angle about the frustum's Y to it.
    pub(super) fn try_update<N: ReckoningNumeric>(
        &mut self,
        mut frustum: [V4; 4],
        speed: f32,
        is_grinding: bool,
        box_test: &mut GrindBoxTest<'_>,
        numeric: &mut N,
    ) -> Result<(), E<N::Error>> {
        self.position_valid = false;
        self.position_angle = PI;
        if is_grinding || !self.active {
            return Ok(());
        }
        let p = self.try_parameters().map_err(E::Grind)?;
        // fsel clamps: an unordered lookahead takes the maximum.
        let lookahead = p.lookahead_time * speed;
        let lower = if p.min_lookahead - lookahead >= 0.0 {
            p.min_lookahead
        } else {
            lookahead
        };
        let radius = if p.max_lookahead - lower >= 0.0 {
            lower
        } else {
            p.max_lookahead
        };
        frustum[3][1] += p.skateboard_offset;
        self.position_valid =
            self.try_find_best_candidate(&frustum, radius, &p, box_test, numeric)?;
        if self.position_valid {
            let inverse = affine_inverse(&frustum, numeric).map_err(E::Numeric)?;
            let local = transform_point(&inverse, self.nearest_position.operand());
            let angle = camera_atan2_pending(local[0], local[2], numeric).map_err(E::Numeric)?;
            self.position_angle = -angle.finish();
        }
        Ok(())
    }

    /// `FindBestGrindCandidate` (0x82e37430): the box spanned by the frustum's far point and its
    /// four edge rays (`mGrindFrustumAngleHorizontal` wide, `mGrindFrustumAngleVertical` up),
    /// at most `mGrindMaxRelativeHeight` tall. Of the primitives in it, the one whose nearer end
    /// is nearest, inside the box, between `mGrindMinLookaheadDistance` and `radius` away,
    /// within the frustum's angles and running within `mGrindMaxRelativeAngle` of its Z (either
    /// way) is the candidate: its nearer end and its heading are kept.
    fn try_find_best_candidate<N: ReckoningNumeric>(
        &mut self,
        frustum: &[V4; 4],
        radius: f32,
        p: &Parameters,
        box_test: &mut GrindBoxTest<'_>,
        numeric: &mut N,
    ) -> Result<bool, E<N::Error>> {
        let origin = frustum[3];
        let mut bounds = OctreeBounds {
            minimum: origin,
            maximum: origin,
        };
        add_point(
            &mut bounds,
            std::array::from_fn(|lane| frustum[2][lane].mul_add(radius, origin[lane])),
        );
        let half = p.horizontal * 0.5;
        for (pitch, yaw) in [
            (0.0, -half),
            (-p.vertical, -half),
            (0.0, half),
            (-p.vertical, half),
        ] {
            add_point(&mut bounds, frustum_ray(frustum, radius, pitch, yaw));
        }
        if bounds.maximum[1] - bounds.minimum[1] > p.max_relative_height {
            bounds.maximum[1] = bounds.minimum[1] + p.max_relative_height;
        }
        let segments = box_test(bounds).map_err(E::Grind)?;
        if segments.is_empty() {
            return Ok(false);
        }
        let facing = vmx::cos(p.max_relative_angle);
        let inverse = affine_inverse(frustum, numeric).map_err(E::Numeric)?;
        let mut best = None;
        let mut nearest = f32::MAX;
        for segment in &segments {
            let end = nearer_end(segment, origin, numeric).map_err(E::Numeric)?;
            if !inside(&bounds, end) {
                continue;
            }
            let distance = try_length(vmx::sub(end, origin), numeric).map_err(E::Numeric)?;
            if distance > radius {
                continue;
            }
            if distance < p.min_lookahead {
                continue;
            }
            if angle_to_grind(segment, &inverse, 0, numeric).map_err(E::Numeric)? > p.horizontal {
                continue;
            }
            if angle_to_grind(segment, &inverse, 1, numeric).map_err(E::Numeric)? > p.vertical {
                continue;
            }
            let mut run = vmx::sub(segment[1], segment[0]);
            run[1] = 0.0;
            let run = try_normalize_safe(run, [0.0; 4], numeric).map_err(E::Numeric)?;
            let alignment = numeric.dot3(run, frustum[2]).map_err(E::Numeric)?.abs();
            if facing > alignment {
                continue;
            }
            if distance < nearest {
                nearest = distance;
                best = Some(segment);
            }
        }
        let Some(best) = best else {
            return Ok(false);
        };
        let end = nearer_end(best, origin, numeric).map_err(E::Numeric)?;
        self.nearest_position = SkeletonRootVector::known(end);
        let run = try_normalize_unsafe(vmx::sub(best[1], best[0]), numeric).map_err(E::Numeric)?;
        let (_, heading) =
            euler_angles(SkeletonRootVector::known(run), numeric).map_err(E::Numeric)?;
        self.heading = heading;
        Ok(true)
    }

    /// `GrindDetector::UpdateGrindAnticipation` (0x82e37338): with a rail found (and Grind
    /// Transparency on) the anticipation rises at `mGrindFadeInSpeed` while the hips block the
    /// camera's line to it, otherwise it falls at `mGrindFadeOutSpeed`; clamped to 0..1 and
    /// eased by a half sine (`GetGrindAnticipation`).
    pub(super) fn try_update_anticipation<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        camera: V4,
        hips: V4,
        numeric: &mut N,
    ) -> Result<(), E<N::Error>> {
        // Both speeds load before the arm is chosen.
        let (fade_out, fade_in) = {
            let parameters = self
                .parameters
                .lock()
                .map_err(|_| E::Grind(CameraGrindUpdateError::OwnerLock))?;
            let p = fields(&parameters).map_err(E::Grind)?;
            (
                field(p.grind_fade_out_speed, "grind_fade_out_speed").map_err(E::Grind)?,
                field(p.grind_fade_in_speed, "grind_fade_in_speed").map_err(E::Grind)?,
            )
        };
        let obscured = self.position_valid
            && self.active
            && camera_to_grind_is_obscured(self.nearest_position.operand(), camera, hips, numeric)
                .map_err(E::Numeric)?;
        let raw = if obscured {
            fade_in.mul_add(dt, self.anticipation)
        } else {
            (-fade_out).mul_add(dt, self.anticipation)
        };
        self.anticipation = unit_fraction(raw);
        let angle = self.anticipation.mul_add(PI, -HALF_PI);
        self.shaped_anticipation = vmx::sin(angle).mul_add(0.5, 0.5);
        Ok(())
    }
}

/// `AddPointToBox` (0x82e36758): each of X, Y and Z of the box reaches the point unless the
/// point lies inside that side; an unordered lane takes the point.
fn add_point(bounds: &mut OctreeBounds, point: V4) {
    for lane in 0..3 {
        if point[lane].partial_cmp(&bounds.minimum[lane]) != Some(Ordering::Greater) {
            bounds.minimum[lane] = point[lane];
        }
        if bounds.maximum[lane].partial_cmp(&point[lane]) != Some(Ordering::Greater) {
            bounds.maximum[lane] = point[lane];
        }
    }
}

/// The candidate test of a point against the box: the X, Y and Z excesses past either side,
/// through `vmaxfp`, all zero. An unordered lane is outside.
fn inside(bounds: &OctreeBounds, point: V4) -> bool {
    (0..3).all(|lane| {
        bounds.minimum[lane] - point[lane] <= 0.0 && point[lane] - bounds.maximum[lane] <= 0.0
    })
}

/// `AddFrustumRayToBox` (0x82e36900): the point `radius` along the frustum's Z turned by
/// `pitch` about X and `yaw` about Y (`framing_basis`'s three `SinCos` rotation, roll 0).
fn frustum_ray(frustum: &[V4; 4], radius: f32, pitch: f32, yaw: f32) -> V4 {
    let rotation = framing_basis(pitch, yaw, &mut HostReckoningSinCosNumeric)
        .unwrap_or_else(|never| match never {});
    let turned = rotation.map(|row| transform_vector(frustum, row));
    transform_point(
        &[turned[0], turned[1], turned[2], frustum[3]],
        [0.0, 0.0, radius, 0.0],
    )
}

/// The primitive's end nearer `origin`; the end on a tie or unordered distance.
fn nearer_end<N: ReckoningNumeric>(
    segment: &[V4; 2],
    origin: V4,
    numeric: &mut N,
) -> Result<V4, N::Error> {
    let [start, end] = *segment;
    let to_start = vmx::sub(start, origin);
    let to_end = vmx::sub(end, origin);
    let start_squared = numeric.dot3(to_start, to_start)?;
    let end_squared = numeric.dot3(to_end, to_end)?;
    Ok(if start_squared < end_squared {
        start
    } else {
        end
    })
}

/// `GetYawToGrind` (0x82e35420, `lane` 0) and `GetPitchToGrind` (0x82e35250, `lane` 1): the
/// lesser angle off the frustum's Z in that lane of the primitive's two ends, a `CameraAngle`.
fn angle_to_grind<N: ReckoningNumeric>(
    segment: &[V4; 2],
    inverse: &[V4; 4],
    lane: usize,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let start = transform_point(inverse, segment[0]);
    let end = transform_point(inverse, segment[1]);
    let start_squared = numeric.dot3(start, start)?;
    let end_squared = numeric.dot3(end, end)?;
    let start_estimate = numeric.reciprocal_sqrt_estimate(start_squared)?;
    let end_estimate = numeric.reciprocal_sqrt_estimate(end_squared)?;
    let start = normalize_from_squared_and_estimate(start, start_squared, start_estimate);
    let end = normalize_from_squared_and_estimate(end, end_squared, end_estimate);
    let end_angle = try_asin(end[lane], numeric)?.abs();
    let start_angle = try_asin(start[lane], numeric)?.abs();
    camera_angle(vmx::vminfp(start_angle, end_angle), numeric)
}

/// `GetCameraToGrindIsObscured` (0x82e36b60): the hips within [`OBSCURED_RADIUS`] of the
/// camera's line through the rail, in X and Z.
fn camera_to_grind_is_obscured<N: ReckoningNumeric>(
    grind: V4,
    camera: V4,
    hips: V4,
    numeric: &mut N,
) -> Result<bool, N::Error> {
    let line = [grind[0] - camera[0], grind[2] - camera[2]];
    let to_hips = [hips[0] - camera[0], hips[2] - camera[2]];
    let along = to_hips[0].mul_add(line[0], to_hips[1] * line[1]);
    let squared = line[0] * line[0] + line[1] * line[1];
    let inverse = refine_drive_reciprocal(squared, numeric.reciprocal_estimate(squared)?);
    let t = inverse * along;
    let offset = [
        line[0].mul_add(t, camera[0]) - hips[0],
        line[1].mul_add(t, camera[2]) - hips[2],
    ];
    Ok(offset[0] * offset[0] + offset[1] * offset[1] < OBSCURED_RADIUS * OBSCURED_RADIUS)
}

/// The `Matrix44Affine` inverse `FindBestGrindCandidate` and `Update` inline: the cofactor rows
/// over the determinant (`vrefp` and two Newton rounds), and `-w` through them.
fn affine_inverse<N: ReckoningNumeric>(
    matrix: &[V4; 4],
    numeric: &mut N,
) -> Result<[V4; 4], N::Error> {
    let [x, y, z, w] = *matrix;
    let (yz, zx, xy) = (vmx::cross(y, z), vmx::cross(z, x), vmx::cross(x, y));
    let determinant = numeric.dot3(x, yz)?;
    let reciprocal =
        refine_drive_reciprocal(determinant, numeric.reciprocal_estimate(determinant)?);
    let rows: [V4; 3] = std::array::from_fn(|lane| {
        [yz[lane], zx[lane], xy[lane], zx[lane]].map(|value| value * reciprocal)
    });
    let translation = std::array::from_fn(|lane| {
        (-w[2]).mul_add(
            rows[2][lane],
            (-w[1]).mul_add(rows[1][lane], -w[0] * rows[0][lane]),
        )
    });
    Ok([rows[0], rows[1], rows[2], translation])
}

/// `point.x * x + point.y * y + point.z * z + w`, each step fused.
fn transform_point(matrix: &[V4; 4], point: V4) -> V4 {
    std::array::from_fn(|lane| {
        let x = matrix[0][lane].mul_add(point[0], matrix[3][lane]);
        let y = matrix[1][lane].mul_add(point[1], x);
        matrix[2][lane].mul_add(point[2], y)
    })
}

/// `vector.x * x + vector.y * y + vector.z * z`, the first product rounded.
fn transform_vector(matrix: &[V4; 4], vector: V4) -> V4 {
    std::array::from_fn(|lane| {
        let x = matrix[0][lane] * vector[0];
        let y = matrix[1][lane].mul_add(vector[1], x);
        matrix[2][lane].mul_add(vector[2], y)
    })
}

#[cfg(test)]
#[path = "camera_grind_detector_tests.rs"]
mod tests;
