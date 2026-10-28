//! GrabSpline curve operations and State_Toolkit::GetBestSpline.
//! Source: 827ab710, 82d83780, 82d89ef0, 82d97d68. All source points stay
//! in model space; the endpoints and returned positions are world space.
use super::{GrabQueryBuffer, GrabSpline};
use crate::{
    physical_player_grind_manager::{
        math::{GrindNum, HALF_PI, I, J, K, PI, V4, abs1, cross, madd, neg, scale, sub},
        view::{GrindError, error},
    },
    physical_player_trajectory_owners::selector_grind::{closest_point_on_line, vmax, vmin},
    reckoning_numeric::{try_normalize_safe, try_unsigned_angle, wrap_turns_to_radians},
};

/// The original PhysBox: affine transform followed by half dimensions.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GrabBox {
    pub(crate) transform: [V4; 4],
    pub(crate) half_dimensions: V4,
}
impl GrabBox {
    /// InitializeGrabbableBoxTransform 82d8cc28. Source intentionally uses
    /// offset Y/Z only, and moves the centre forward by halfDimensions.z.
    pub(crate) fn from_player(
        transform: [V4; 4],
        offset: V4,
        half_dimensions: V4,
        use_pitch: bool,
        num: &mut GrindNum<'_>,
    ) -> Result<Self, GrindError> {
        let (x, y, z) = if use_pitch {
            let mut flat_x = transform[0];
            flat_x[1] = 0.0;
            let x = try_normalize_safe(flat_x, I, num.n).map_err(error)?;
            (x, cross(transform[2], x), transform[2])
        } else {
            let mut flat_z = transform[2];
            flat_z[1] = 0.0;
            let z = try_normalize_safe(flat_z, K, num.n).map_err(error)?;
            (cross(J, z), J, z)
        };
        let position = madd(
            y,
            offset[1],
            madd(z, offset[2] + half_dimensions[2], transform[3]),
        );
        Ok(Self {
            transform: [x, y, z, position],
            half_dimensions,
        })
    }

    /// DoesPointIntersectBox 82e49950: inclusive bounds in box-local space;
    /// unordered coordinates fail the comparisons, rather than passing.
    pub(crate) fn contains(&self, point: V4) -> bool {
        let local = inverse_transform_point(&self.transform, point);
        (0..3).all(|i| local[i] >= -self.half_dimensions[i])
            && (0..3).all(|i| self.half_dimensions[i] >= local[i])
    }
}

/// Matrix44Affine point transform: x FMA translation, y FMA, z FMA.
pub(crate) fn transform_point(rows: &[V4; 4], point: V4) -> V4 {
    madd(
        rows[2],
        point[2],
        madd(rows[1], point[1], madd(rows[0], point[0], rows[3])),
    )
}

/// Raw 82d83884..82d83908: transpose the axes, then form inverse
/// translation in Z,Y,X order and apply the point in X,Y,Z order.
/// Keeping these FMAs separate from (point - translation) matches the code.
pub(crate) fn inverse_transform_point(rows: &[V4; 4], point: V4) -> V4 {
    let x = [rows[0][0], rows[1][0], rows[2][0], 0.0];
    let y = [rows[0][1], rows[1][1], rows[2][1], 0.0];
    let z = [rows[0][2], rows[1][2], rows[2][2], 0.0];
    let translation = sub([0.0; 4], rows[3]);
    let inverse_translation = madd(
        x,
        translation[0],
        madd(y, translation[1], scale(z, translation[2])),
    );
    madd(
        z,
        point[2],
        madd(y, point[1], madd(x, point[0], inverse_translation)),
    )
}

/// SplineUtils::CalculateWorldHorizontalNormal82e48ac0. Near vertical,
/// blend toward the authored normal; the original does not normalize the blend.
pub(crate) fn world_horizontal_normal(
    p1: V4,
    p2: V4,
    general: V4,
    num: &mut GrindNum<'_>,
) -> Result<V4, GrindError> {
    let direction = try_normalize_safe(sub(p2, p1), [0.0; 4], num.n).map_err(error)?;
    let vertical = abs1(num.dot(direction, J)?);
    let close = f32::from_bits(0x3f7f_be77); // 0.999
    let too_close = f32::from_bits(0x3f7f_f972); // 0.9999
    if vertical > close && vertical > too_close {
        return Ok(general);
    }
    let blend = if vertical > close {
        Some(vmin(
            1.0,
            vmax(
                0.0,
                num.reciprocal(too_close - close)? * (too_close - vertical),
            ),
        ))
    } else {
        None
    };
    let mut horizontal = try_normalize_safe(cross(direction, J), [0.0; 4], num.n).map_err(error)?;
    if 0.0 > num.dot(general, horizontal)? {
        horizontal = neg(horizontal);
    }
    Ok(match blend {
        Some(weight) => madd(horizontal, weight, scale(general, 1.0 - weight)),
        None => horizontal,
    })
}

impl GrabSpline {
    /// SplineUtils::CanGrabSpline 82e4b4f8, including the by-value exclusion
    /// clamp. The polyline's nearest point determines distance along it.
    pub(crate) fn can_grab(
        &self,
        reach_start: V4,
        bounds: &GrabBox,
        mut end_exclusion: f32,
        angle_limit: f32,
        horizontal_limit: f32,
        num: &mut GrindNum<'_>,
    ) -> Result<bool, GrindError> {
        if end_exclusion > self.length * 0.5 {
            end_exclusion = self.length * 0.5;
        }
        let mut along = 0.0;
        self.nearest_point(reach_start, Some(&mut along), num)?;
        along = vmin(self.length - end_exclusion, vmax(end_exclusion, along));
        self.can_grab_at(
            reach_start,
            along,
            bounds,
            angle_limit,
            horizontal_limit,
            num,
        )
    }

    /// CanGrabSplineAtPoint 82e4a9a8. Test reach angle, sample the nearby
    /// tangent, reject vertical/degenerate tangents, then test the box.
    pub(crate) fn can_grab_at(
        &self,
        reach_start: V4,
        along: f32,
        bounds: &GrabBox,
        angle_limit: f32,
        horizontal_limit: f32,
        num: &mut GrindNum<'_>,
    ) -> Result<bool, GrindError> {
        let point = self.position_at_spline_length(along, num)?;
        let angle =
            try_unsigned_angle(sub(point, reach_start), neg(self.normal), num.n).map_err(error)?;
        if !(angle_limit > angle) {
            return Ok(false);
        }
        let mut step = self.length * 0.5;
        let maximum_step = f32::from_bits(0x3c23_d70a);
        if step > maximum_step {
            step = maximum_step;
        }
        let next = along + step;
        let sample = if next > self.length {
            along - step
        } else {
            next
        };
        let tangent = sub(self.position_at_spline_length(sample, num)?, point);
        let mut horizontal = tangent;
        horizontal[1] = 0.0;
        if !(num.dot(horizontal, horizontal)? > f32::from_bits(0x3780_0000)) {
            return Ok(false);
        }
        let angle = try_unsigned_angle(tangent, horizontal, num.n).map_err(error)?;
        let mut angle = abs1(wrap_turns_to_radians(angle * f32::from_bits(0x3e22_f983)));
        if angle > HALF_PI {
            angle = PI - angle;
        }
        if !(horizontal_limit > angle) {
            return Ok(false);
        }
        Ok(bounds.contains(point))
    }

    /// GrabSpline::Init(tGrab) 82d1a988. The supplied assembly/snapshot are
    /// from the existing Simulation DMO pool, not a second physics object.
    pub(crate) fn init_dmo(
        &mut self,
        spline_id: u32,
        source: crate::living_world_grind_services::dmo_models::DmoGrabSpline,
        transform: [V4; 4],
        velocity: V4,
        assembly: crate::simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyHandle,
        part: Option<crate::dynamic_assembly_session::source_parts::SourcePartGrabSnapshot>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        self.init_living_world(
            2, spline_id, source, transform, velocity, assembly, part, num,
        )
    }

    /// SAME native Init82d1a988 for vehicle type1 and DMO type2. Only the
    /// descriptor differs; callers supply their real assembly and Part inertia.
    pub(crate) fn init_living_world(
        &mut self,
        kind: i32,
        spline_id: u32,
        source: crate::living_world_grind_services::dmo_models::DmoGrabSpline,
        transform: [V4; 4],
        velocity: V4,
        assembly: crate::simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyHandle,
        part: Option<crate::dynamic_assembly_session::source_parts::SourcePartGrabSnapshot>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        if source.grab().spline_points.len() < 2 {
            return Err(GrindError(
                "grab Init needs at least two spline points".into(),
            ));
        }
        self.body_position = [0.0; 4];
        self.descriptor_type = kind;
        self.descriptor_object = spline_id as i32;
        self.status = (self.status & 0x1f) | 0x80;
        self.status_known_mask |= 0xe0;
        self.transform = transform;
        let data = source.grab();
        self.p1 = transform_point(&transform, data.spline_points[0]);
        self.p2 = transform_point(&transform, *data.spline_points.last().expect("two points"));
        let approach = data
            .approach_vectors
            .first()
            .copied()
            .unwrap_or([0.0, 1.0, 0.0, 0.0]);
        self.normal = madd(
            transform[2],
            approach[2],
            madd(transform[1], approach[1], scale(transform[0], approach[0])),
        );
        self.flags = data.flags;
        self.velocity = velocity;
        self.spline_data = Some(source);
        self.assembly = Some(assembly);
        self.part = None;
        self.rigid_body = None;
        self.inertia = crate::simulation_rigid_body_pool::SimulationInertia::native_default();
        self.inverse_tensor_w = 1.0;
        if let Some(part) = part {
            self.part = Some(0);
            self.rigid_body = part.body;
            self.inertia = part.inertia;
            self.inverse_tensor_w = part.inverse_tensor_w;
            if part.body.is_some() {
                let pose = part
                    .body_transform
                    .ok_or_else(|| GrindError("grab body has no native body transform".into()))?;
                self.body_position.copy_from_slice(&pose[12..16]);
                self.status |= 0x40;
            }
        }
        let delta = sub(self.p2, self.p1);
        let chord_length = num.length(delta)?;
        // Native has no zero guard here beyond the length calculation.
        self.length_unit = Some(scale(delta, num.reciprocal(chord_length)?));
        self.half_length = chord_length * 0.5;
        self.length = self.compute_spline_length(num)?;
        Ok(())
    }

    fn source_points(&self) -> Result<&[V4], GrindError> {
        let source = self
            .spline_data
            .as_ref()
            .ok_or_else(|| GrindError("grab curve has no tGrab source".into()))?;
        let points = &source.grab().spline_points;
        if points.is_empty() {
            return Err(GrindError(
                "grab curve has zero points (native assertion)".into(),
            ));
        }
        Ok(points)
    }

    /// ComputeSplineLength 827ab710 sums the local segments, not the
    /// distance between the transformed first and last point.
    pub(crate) fn compute_spline_length(&self, num: &mut GrindNum<'_>) -> Result<f32, GrindError> {
        let mut total = 0.0;
        for segment in self.source_points()?.windows(2) {
            total += num.length(sub(segment[1], segment[0]))?;
        }
        Ok(total)
    }

    /// GetPosAlongCurveBySplineLength 82d89ef0. Bit 0x20 reverses local
    /// traversal; P1/P2 already reflect the same reversal. At a segment
    /// boundary the strict `total > offset` advances to the next segment.
    pub(crate) fn position_at_spline_length(
        &self,
        offset: f32,
        num: &mut GrindNum<'_>,
    ) -> Result<V4, GrindError> {
        let points = self.source_points()?;
        if points.len() <= 1 || 0.0 >= offset {
            return Ok(self.p1);
        }
        let reverse = self.status & 0x20 != 0;
        let at = |i: usize| points[if reverse { points.len() - 1 - i } else { i }];
        let mut total = 0.0;
        for i in 1..points.len() {
            let end = at(i);
            let delta = sub(end, at(i - 1));
            let length = num.length(delta)?;
            total += length;
            if total > offset {
                // The debug assert continues to this nonpositive-length
                // fallback; don't normalize a degenerate segment.
                let local = if 0.0 >= length {
                    end
                } else {
                    let fraction = num.reciprocal(length)? * (total - offset);
                    sub(end, scale(delta, fraction))
                };
                return Ok(transform_point(&self.transform, local));
            }
        }
        Ok(self.p2)
    }

    /// GetPosAlongCurveByStraightLength82d8a218. The offset is measured from
    /// the chord centre, while segment progress is projected onto the local
    /// first-to-last chord. It is deliberately not arc-length traversal.
    pub(crate) fn position_at_straight_length(
        &self,
        offset: f32,
        num: &mut GrindNum<'_>,
    ) -> Result<V4, GrindError> {
        if -self.half_length >= offset {
            return Ok(self.p1);
        }
        if offset >= self.half_length {
            return Ok(self.p2);
        }
        let points = self.source_points()?;
        if points.len() <= 1 {
            return Ok(self.p1);
        }
        let offset = if self.status & 0x20 != 0 {
            self.half_length - offset
        } else {
            offset + self.half_length
        };
        let chord = num.normalize_unsafe(sub(points[points.len() - 1], points[0]))?;
        let mut total = 0.0;
        for segment in points.windows(2) {
            let delta = sub(segment[1], segment[0]);
            let projected = vmax(num.dot(delta, chord)?, f32::from_bits(0x38d1_b717));
            total += projected;
            if total > offset {
                let local = if 0.0 >= projected {
                    segment[1]
                } else {
                    sub(
                        segment[1],
                        scale(delta, num.reciprocal(projected)? * (total - offset)),
                    )
                };
                return Ok(transform_point(&self.transform, local));
            }
        }
        Ok(self.p2)
    }

    /// GetNearestPointOnSpline 82d83780: closest local segment point,
    /// first winner on ties. Optional along-distance is untouched for a
    /// single-point source, exactly as the early return in the reference.
    pub(crate) fn nearest_point(
        &self,
        point: V4,
        mut distance_along: Option<&mut f32>,
        num: &mut GrindNum<'_>,
    ) -> Result<V4, GrindError> {
        let points = self.source_points()?;
        if points.len() <= 1 {
            return Ok(self.p1);
        }
        let reverse = self.status & 0x20 != 0;
        let at = |i: usize| points[if reverse { points.len() - 1 - i } else { i }];
        if let Some(distance) = distance_along.as_deref_mut() {
            *distance = 0.0;
        }
        let local_point = inverse_transform_point(&self.transform, point);
        let mut best = at(0);
        let mut best_squared = f32::MAX;
        let mut total = 0.0;
        for i in 1..points.len() {
            let start = at(i - 1);
            let end = at(i);
            let candidate = closest_point_on_line(local_point, start, end, num)?;
            let delta = sub(local_point, candidate);
            let squared = num.dot(delta, delta)?;
            if best_squared > squared {
                best_squared = squared;
                best = candidate;
                if let Some(distance) = distance_along.as_deref_mut() {
                    *distance = total + num.length(sub(candidate, start))?;
                }
            }
            if distance_along.is_some() {
                total += num.length(sub(end, start))?;
            }
        }
        Ok(transform_point(&self.transform, best))
    }
}

impl<const N: usize> GrabQueryBuffer<N> {
    /// SortByDistanceFromPoint 827b1e20 calls _Sort 827b14d0. At its actual
    /// capacity (32), that always selects _Insertion_sort 827afe70. Keep
    /// its strict-less comparison and rotate semantics, including ties.
    pub(crate) fn sort_by_distance(
        &mut self,
        point: V4,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        let count = self.result_count as usize;
        if count > N || count > 32 {
            return Err(GrindError("grab sort exceeds native capacity".into()));
        }
        for spline in &mut self.splines[..count] {
            let closest = closest_point_on_line(point, spline.p1, spline.p2, num)?;
            let delta = sub(point, closest);
            spline.sort_parameter = num.dot(delta, delta)?;
        }
        for index in 1..count {
            let value = self.splines[index].sort_parameter;
            let mut destination = index;
            if value < self.splines[0].sort_parameter {
                destination = 0;
            } else {
                while destination > 0 && value < self.splines[destination - 1].sort_parameter {
                    destination -= 1;
                }
            }
            self.splines[destination..=index].rotate_right(1);
        }
        Ok(())
    }

    /// State_Toolkit::GetBestSpline 82d97d68 scans validated splines by
    /// distance to their endpoint chord (not the curved source polyline).
    /// A missing winner leaves the caller's current spline untouched.
    pub(crate) fn best_spline(
        &mut self,
        reach_start: V4,
        excluded: Option<(i32, i32)>,
        num: &mut GrindNum<'_>,
    ) -> Result<Option<GrabSpline>, GrindError> {
        if self.result_count as usize > N {
            return Err(GrindError("grab query result count exceeds storage".into()));
        }
        self.cursor = 0;
        let mut distance = f32::from_bits(0x47c3_4ff3);
        let mut best = None;
        while self.cursor < self.result_count {
            let spline = &self.splines[self.cursor as usize];
            self.cursor += 1;
            if excluded == Some((spline.descriptor_type, spline.descriptor_object)) {
                continue;
            }
            let nearest = closest_point_on_line(reach_start, spline.p1, spline.p2, num)?;
            let delta = sub(nearest, reach_start);
            let squared = num.dot(delta, delta)?;
            if squared < distance {
                distance = squared;
                best = Some(spline.clone());
            }
        }
        Ok(best)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        living_world_grind_services::dmo_models::DmoGrabSpline,
        physical_player_grind_manager::math::ZERO, reckoning_numeric::HostReckoningNumeric,
        reckoning_sin_cos::HostReckoningSinCosNumeric,
    };
    use skate2_formats::pegasus::{AxisAlignedBox, Grab, GrabData};
    use std::sync::Arc;

    fn curve() -> GrabSpline {
        let mut spline = GrabSpline::in_uninitialized_allocation();
        // A corner: local (0,0,0) -> (2,0,0) -> (2,0,3), rotated
        // and translated into world space. Chord length != curve length.
        spline.transform = [
            [0.0, 0.0, -1.0, 0.0],
            J,
            [1.0, 0.0, 0.0, 0.0],
            [10.0, 2.0, 20.0, 0.0],
        ];
        spline.p1 = [10.0, 2.0, 20.0, 0.0];
        spline.p2 = [13.0, 2.0, 18.0, 0.0];
        spline.length = 5.0;
        spline.spline_data = Some(DmoGrabSpline {
            entry_index: 7,
            header_offset: 0x20,
            data: Arc::new(GrabData {
                grabs_offset: 0x20,
                grabs: vec![Grab {
                    bounds: AxisAlignedBox {
                        minimum: [0.0; 4],
                        maximum: [2.0, 0.0, 3.0, 0.0],
                    },
                    attribute_class: 0,
                    attribute_collection: 0,
                    spline_points: vec![[0.0; 4], [2.0, 0.0, 0.0, 0.0], [2.0, 0.0, 3.0, 0.0]],
                    approach_points: vec![],
                    approach_vectors: vec![J],
                    flags: 0,
                    kind: 0,
                }],
            }),
        });
        spline
    }
    fn near(a: V4, b: V4) {
        for i in 0..4 {
            assert!((a[i] - b[i]).abs() < 0.0001, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn straight_length_projects_curved_segments_and_preserves_reversed_world_point() {
        let mut n = HostReckoningNumeric;
        let mut s = HostReckoningSinCosNumeric;
        let mut num = GrindNum {
            n: &mut n,
            s: &mut s,
        };
        let mut spline = curve();
        spline.half_length = 13.0_f32.sqrt() * 0.5;
        // Chord projection at the centre lies 5/6 units into the second
        // segment, rather than the arc midpoint (1/2 unit into it).
        near(
            spline.position_at_straight_length(0.0, &mut num).unwrap(),
            [10.0 + 5.0 / 6.0, 2.0, 18.0, 0.0],
        );
        let forward = spline.position_at_straight_length(0.4, &mut num).unwrap();
        std::mem::swap(&mut spline.p1, &mut spline.p2);
        spline.status |= 0x20;
        near(
            spline.position_at_straight_length(-0.4, &mut num).unwrap(),
            forward,
        );
        assert_eq!(
            spline
                .position_at_straight_length(-100.0, &mut num)
                .unwrap(),
            spline.p1
        );
        assert_eq!(
            spline.position_at_straight_length(100.0, &mut num).unwrap(),
            spline.p2
        );
    }

    #[test]
    fn curved_grab_walks_local_segments_and_reverses_distance_from_swapped_end() {
        let mut n = HostReckoningNumeric;
        let mut s = HostReckoningSinCosNumeric;
        let mut num = GrindNum {
            n: &mut n,
            s: &mut s,
        };
        let mut spline = curve();
        assert!((spline.compute_spline_length(&mut num).unwrap() - 5.0).abs() < 0.0001);
        near(
            spline.position_at_spline_length(3.0, &mut num).unwrap(),
            [11.0, 2.0, 18.0, 0.0],
        );
        assert_eq!(
            spline.position_at_spline_length(-1.0, &mut num).unwrap(),
            spline.p1
        );
        assert_eq!(
            spline.position_at_spline_length(8.0, &mut num).unwrap(),
            spline.p2
        );
        let mut distance = -1.0;
        near(
            spline
                .nearest_point([11.0, 3.0, 18.0, 0.0], Some(&mut distance), &mut num)
                .unwrap(),
            [11.0, 2.0, 18.0, 0.0],
        );
        assert!((distance - 3.0).abs() < 0.0001);
        std::mem::swap(&mut spline.p1, &mut spline.p2);
        spline.status |= 0x20;
        near(
            spline.position_at_spline_length(2.0, &mut num).unwrap(),
            [11.0, 2.0, 18.0, 0.0],
        );
        spline
            .nearest_point([11.0, 3.0, 18.0, 0.0], Some(&mut distance), &mut num)
            .unwrap();
        assert!((distance - 2.0).abs() < 0.0001);
    }

    #[test]
    fn native_sort_preserves_ties_and_best_exclusion_keeps_the_cursor() {
        let mut n = HostReckoningNumeric;
        let mut s = HostReckoningSinCosNumeric;
        let mut num = GrindNum {
            n: &mut n,
            s: &mut s,
        };
        let mut buffer = GrabQueryBuffer::<5>::in_sim_allocation();
        buffer.result_count = 3;
        for (index, z) in [2.0, 1.0, -1.0].into_iter().enumerate() {
            buffer.splines[index].p1 = [-2.0, 0.0, z, 0.0];
            buffer.splines[index].p2 = [2.0, 0.0, z, 0.0];
            buffer.splines[index].descriptor_type = 2;
            buffer.splines[index].descriptor_object = index as i32 + 10;
        }
        buffer.sort_by_distance([0.0; 4], &mut num).unwrap();
        assert_eq!(
            buffer.splines[..3]
                .iter()
                .map(|s| s.descriptor_object)
                .collect::<Vec<_>>(),
            vec![11, 12, 10]
        );
        assert_eq!(
            buffer
                .best_spline([0.0; 4], Some((2, 11)), &mut num)
                .unwrap()
                .unwrap()
                .descriptor_object,
            12
        );
        assert_eq!(buffer.cursor, 3);
    }

    #[test]
    fn reach_box_uses_forward_extent_and_inclusive_rotated_bounds() {
        let mut n = HostReckoningNumeric;
        let mut s = HostReckoningSinCosNumeric;
        let mut num = GrindNum {
            n: &mut n,
            s: &mut s,
        };
        let b = GrabBox::from_player(
            curve().transform,
            [99.0, 1.0, 0.5, 0.0],
            [1.0, 2.0, 3.0, 0.0],
            false,
            &mut num,
        )
        .unwrap();
        near(b.transform[3], [13.5, 3.0, 20.0, 0.0]);
        assert!(b.contains([16.5, 5.0, 21.0, 0.0]));
        assert!(!b.contains([16.6, 5.0, 21.0, 0.0]));
        assert!(!b.contains([f32::NAN, 3.0, 20.0, 0.0]));
    }

    #[test]
    fn horizontal_grab_normal_preserves_facing_and_vertical_fallback() {
        let mut n = HostReckoningNumeric;
        let mut s = HostReckoningSinCosNumeric;
        let mut num = GrindNum {
            n: &mut n,
            s: &mut s,
        };
        let facing = [0.0, 0.0, -1.0, 0.0];
        near(
            world_horizontal_normal(ZERO, [1.0, 0.0, 0.0, 0.0], facing, &mut num).unwrap(),
            facing,
        );
        let fallback = [0.25, 0.5, 0.75, -0.0];
        let vertical =
            world_horizontal_normal(ZERO, [0.0, 1.0, 0.0, 0.0], fallback, &mut num).unwrap();
        assert_eq!(vertical.map(f32::to_bits), fallback.map(f32::to_bits));
    }

    #[test]
    fn near_vertical_normal_blends_without_renormalizing() {
        let mut n = HostReckoningNumeric;
        let mut s = HostReckoningSinCosNumeric;
        let mut num = GrindNum {
            n: &mut n,
            s: &mut s,
        };
        let y = 0.9995_f32;
        let x = (1.0 - y * y).sqrt();
        let result =
            world_horizontal_normal(ZERO, [x, y, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], &mut num)
                .unwrap();
        assert!(result[0] > 0.3 && result[0] < 0.6);
        assert!(result[2] > 0.4 && result[2] < 0.7);
        assert!((result[0] + result[2] - 1.0).abs() < 0.0001);
        assert!(num.length(result).unwrap() < 0.8);
    }
}
