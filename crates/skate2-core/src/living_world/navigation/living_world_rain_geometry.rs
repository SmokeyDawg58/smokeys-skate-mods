//! RAIN Road/Segment/SuperRoad evaluators. They consume the existing streamed
//! records and CurveData arc tables; no independent road representation.
use super::{
    RainMan,
    asset::RainAssetError,
    curve::CurveError,
    network::{Road, Segment, SuperRoad},
};
use crate::{
    reckoning_numeric::{
        ReckoningNumeric, cross_full4, try_length, try_look_at_up_eye, try_normalize_unsafe,
    },
    simulation_drive_jacobian::refine_drive_reciprocal,
};
pub(crate) type Transform = [[f32; 4]; 4];
const IDENTITY: Transform = [
    [1., 0., 0., 0.],
    [0., 1., 0., 0.],
    [0., 0., 1., 0.],
    [0.; 4],
];
#[derive(Debug)]
pub(crate) enum GeometryError<E> {
    Asset(RainAssetError),
    Curve(CurveError<E>),
    Numeric(E),
    MissingRoad(u64),
}
impl<E> From<RainAssetError> for GeometryError<E> {
    fn from(error: RainAssetError) -> Self {
        Self::Asset(error)
    }
}
impl Road {
    /// CompareSegmentArclengths upper_bound82e5b0b8. Exact segment endpoint
    /// selects the next segment; end-of-road deliberately has no selected segment.
    fn segment_at(&self, distance: f32) -> Result<Option<&Segment>, RainAssetError> {
        let mut first = 0;
        let mut count = self.segments.len();
        while count != 0 {
            let half = count / 2;
            let middle = first + half;
            if distance < self.segments[middle].record.scalar(0xd0)? {
                count = half;
            } else {
                first = middle + 1;
                count -= half + 1;
            }
        }
        Ok(self.segments.get(first))
    }
    /// EvaluateWidthDistance82e4e0c0; source offset and lane count are NOT
    /// clamped. Reciprocal has the original two refinement steps.
    pub(crate) fn width_distance<N: ReckoningNumeric>(
        &self,
        lane: i32,
        offset: f32,
        numeric: &mut N,
    ) -> Result<f32, GeometryError<N::Error>> {
        let lanes = self.record.word(0x84)? as f32;
        let width = self.record.scalar(0x88)?;
        let denominator = 2. * lanes;
        let numerator = offset.mul_add(width, 2.0_f32.mul_add(lane as f32, 1.));
        let inverse = refine_drive_reciprocal(
            denominator,
            numeric
                .reciprocal_estimate(denominator)
                .map_err(GeometryError::Numeric)?,
        );
        Ok(inverse * numerator)
    }
    /// EvaluateNormal82e5bf08 does not read lane/offset or interpolate normals.
    pub(crate) fn evaluate_normal(&self, distance: f32) -> Result<[f32; 4], RainAssetError> {
        Ok(self
            .segment_at(distance)?
            .map_or([0., 1., 0., 0.], |s| s.normal))
    }
    /// EvaluateTransform82e5d7d8; missing segment returns source identity.
    pub(crate) fn evaluate_transform<N: ReckoningNumeric>(
        &self,
        distance: f32,
        lane: i32,
        offset: f32,
        numeric: &mut N,
    ) -> Result<Transform, GeometryError<N::Error>> {
        let Some(segment) = self.segment_at(distance)? else {
            return Ok(IDENTITY);
        };
        let arc = segment.arc_distance(distance)?;
        let width = self.width_distance(lane, offset, numeric)?;
        segment.evaluate_transform(arc, width, numeric)
    }
    /// EvaluatePosition82e5eeb8; no segment returns zero, not last endpoint.
    pub(crate) fn evaluate_position<N: ReckoningNumeric>(
        &self,
        distance: f32,
        lane: i32,
        offset: f32,
        numeric: &mut N,
    ) -> Result<[f32; 4], GeometryError<N::Error>> {
        let Some(segment) = self.segment_at(distance)? else {
            return Ok([0.; 4]);
        };
        let arc = segment.arc_distance(distance)?;
        let width = self.width_distance(lane, offset, numeric)?;
        segment.evaluate_position(arc, width, numeric)
    }
    /// EvaluateTangent82e5efb0 still evaluates width even though Segment's
    /// tangent ignores it. Source fallback is +X, NOT the transform's +Z.
    pub(crate) fn evaluate_tangent<N: ReckoningNumeric>(
        &self,
        distance: f32,
        lane: i32,
        offset: f32,
        numeric: &mut N,
    ) -> Result<[f32; 4], GeometryError<N::Error>> {
        let Some(segment) = self.segment_at(distance)? else {
            return Ok([1., 0., 0., 0.]);
        };
        let arc = segment.arc_distance(distance)?;
        let _width = self.width_distance(lane, offset, numeric)?;
        let t = segment
            .curve
            .parameter(arc, numeric)
            .map_err(GeometryError::Curve)?;
        Ok(segment.curve.tangent(t))
    }
}
impl Segment {
    fn arc_distance(&self, distance: f32) -> Result<f32, RainAssetError> {
        let arc = self.curve.length - (self.record.scalar(0xd0)? - distance);
        // fsel82e5d878: unordered selects zero; signed zero retained.
        Ok(if arc >= 0. { arc } else { 0. })
    }
    /// Linear edge interpolation uses arc/length, not Hermite parameter t.
    fn width_vector(&self, arc: f32) -> Result<[f32; 4], RainAssetError> {
        let fraction = arc / self.curve.length; // source scalar fdivs
        let a = self.record.vector(0x50)?;
        let b = self.record.vector(0x60)?;
        let c = self.record.vector(0x70)?;
        let d = self.record.vector(0x80)?;
        let left: [f32; 4] = std::array::from_fn(|i| (c[i] - a[i]).mul_add(fraction, a[i]));
        let right: [f32; 4] = std::array::from_fn(|i| (d[i] - b[i]).mul_add(fraction, b[i]));
        Ok(std::array::from_fn(|i| right[i] - left[i]))
    }
    /// Segment.EvaluateTransform82e5bfa8. Centerline orientation plus interpolated
    /// full road width; subtract along local X with two rounded multiplies.
    pub(crate) fn evaluate_transform<N: ReckoningNumeric>(
        &self,
        arc: f32,
        width: f32,
        numeric: &mut N,
    ) -> Result<Transform, GeometryError<N::Error>> {
        let t = self
            .curve
            .parameter(arc, numeric)
            .map_err(GeometryError::Curve)?;
        let point = self.curve.point(t);
        let tangent = self.curve.tangent(t);
        let width_vector = self.width_vector(arc)?;
        let width_length = try_length(width_vector, numeric).map_err(GeometryError::Numeric)?;
        let look = std::array::from_fn(|i| point[i] + tangent[i]);
        let mut result = try_look_at_up_eye(look, self.normal, point, numeric)
            .map_err(GeometryError::Numeric)?;
        let offset = width - 0.5;
        result[3] = std::array::from_fn(|i| point[i] - (result[0][i] * width_length) * offset);
        Ok(result)
    }
    /// Segment.EvaluateBinormal82e5c150. Normalize up cross raw tangent,
    /// multiply by interpolated width; no degenerate-direction substitute.
    pub(crate) fn evaluate_binormal<N: ReckoningNumeric>(
        &self,
        arc: f32,
        numeric: &mut N,
    ) -> Result<[f32; 4], GeometryError<N::Error>> {
        let t = self
            .curve
            .parameter(arc, numeric)
            .map_err(GeometryError::Curve)?;
        let tangent = self.curve.tangent(t);
        let direction = try_normalize_unsafe(cross_full4(self.normal, tangent), numeric)
            .map_err(GeometryError::Numeric)?;
        let width = try_length(self.width_vector(arc)?, numeric).map_err(GeometryError::Numeric)?;
        Ok(direction.map(|v| v * width))
    }
    /// Segment.EvaluatePosition82e5d8f0. Preserve separate binormal evaluation.
    pub(crate) fn evaluate_position<N: ReckoningNumeric>(
        &self,
        arc: f32,
        width: f32,
        numeric: &mut N,
    ) -> Result<[f32; 4], GeometryError<N::Error>> {
        let t = self
            .curve
            .parameter(arc, numeric)
            .map_err(GeometryError::Curve)?;
        let point = self.curve.point(t);
        let binormal = self.evaluate_binormal(arc, numeric)?;
        Ok(std::array::from_fn(|i| {
            point[i] - binormal[i] * (width - 0.5)
        }))
    }
}
impl SuperRoad {
    /// FindRoadFromDistance82e5c2c8. Stream gaps reject the upper-bound candidate
    /// if its start is still ahead of the query; never bridge a missing chunk.
    pub(crate) fn find_road<'a, E>(
        &self,
        distance: f32,
        rain: &'a RainMan,
    ) -> Result<Option<&'a Road>, GeometryError<E>> {
        let mut first = 0;
        let mut count = self.roads.len();
        while count != 0 {
            let half = count / 2;
            let middle = first + half;
            let id = self.roads[middle].1;
            let road = rain.roads.get(&id).ok_or(GeometryError::MissingRoad(id))?;
            if distance < road.record.scalar(0x90)? {
                count = half;
            } else {
                first = middle + 1;
                count -= half + 1;
            }
        }
        let Some((_, id)) = self.roads.get(first) else {
            return Ok(None);
        };
        let road = rain.roads.get(id).ok_or(GeometryError::MissingRoad(*id))?;
        let start = road.record.scalar(0x90)? - road.record.scalar(0x8c)?;
        Ok(if distance < start { None } else { Some(road) })
    }
    /// EvaluateRoadPointTransform82e5da28.
    pub(crate) fn evaluate_transform<N: ReckoningNumeric>(
        &self,
        distance: f32,
        lane: u32,
        offset: f32,
        rain: &RainMan,
        numeric: &mut N,
    ) -> Result<Transform, GeometryError<N::Error>> {
        let Some(road) = self.find_road(distance, rain)? else {
            return Ok(IDENTITY);
        };
        let local = distance - (road.record.scalar(0x90)? - road.record.scalar(0x8c)?);
        road.evaluate_transform(local, lane as i32, offset, numeric)
    }
}
