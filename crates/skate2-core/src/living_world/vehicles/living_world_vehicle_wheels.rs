//! Vehicle wheel-adjusted RAIN geometry; source82d17228..82d182fc/82d1dc80.
use crate::{
    attribute_runtime::AttributeRuntimeError,
    living_world_rain::{
        RainMan,
        dynamic_passage::DynPassage,
        geometry::{GeometryError, Transform},
        network::SuperRoad,
    },
    reckoning_numeric::{ReckoningNumeric, cross_full4, try_length, try_normalize_unsafe},
};
#[derive(Debug)]
pub(crate) enum WheelError<E> {
    Geometry(GeometryError<E>),
    Numeric(E),
    Attribute(AttributeRuntimeError),
    AttributeByte,
    Curve(crate::living_world_rain::curve::CurveError<E>),
    MissingCurveLength,
}
/// Both native FillOnCurve82d17cc8 and FillOnDynCurve82d17b38 have identical
/// evaluation order. Borrow their real authored/dynamic curve members.
pub(crate) enum WheelCurve<'a> {
    Authored(&'a crate::living_world_rain::asset::CurveData),
    Dynamic(&'a crate::living_world_rain::dynamic_passage::DynCurve),
}
impl WheelCurve<'_> {
    fn length<E>(&self) -> Result<f32, WheelError<E>> {
        match self {
            Self::Authored(c) => Ok(c.length),
            Self::Dynamic(c) => c.length.ok_or(WheelError::MissingCurveLength),
        }
    }
    fn evaluate<N: ReckoningNumeric>(
        &self,
        arc: f32,
        numeric: &mut N,
    ) -> Result<([f32; 4], [f32; 4]), WheelError<N::Error>> {
        match self {
            Self::Authored(c) => {
                let t = c.parameter(arc, numeric).map_err(WheelError::Curve)?;
                Ok((c.point(t), c.tangent(t)))
            }
            Self::Dynamic(c) => {
                let t = c.parameter(arc, numeric).map_err(WheelError::Curve)?;
                Ok((c.point(t), c.tangent(t)))
            }
        }
    }
}
pub(crate) fn on_curve<N: ReckoningNumeric>(
    curve: WheelCurve<'_>,
    position: f32,
    vehicle_length: f32,
    numeric: &mut N,
) -> Result<WheelTransformInfo, WheelError<N::Error>> {
    let length = curve.length()?;
    let front = front_wheel_position(position, length, vehicle_length);
    let rear = rear_wheel_position(position, length, vehicle_length);
    let (front_position, front_tangent) = curve.evaluate(front, numeric)?;
    let (rear_position, rear_tangent) = curve.evaluate(rear, numeric)?;
    Ok(WheelTransformInfo {
        front_position,
        front_tangent,
        rear_position,
        rear_tangent,
    })
}
pub(crate) struct WheelTransformInfo {
    pub(crate) front_position: [f32; 4],
    pub(crate) front_tangent: [f32; 4],
    pub(crate) rear_position: [f32; 4],
    pub(crate) rear_tangent: [f32; 4],
}
pub(crate) struct VehicleWheelState {
    pub(crate) target_heading: [f32; 4],
    pub(crate) radius: f32,
    pub(crate) spin_angle: f32,
    pub(crate) turn_angle: f32,
}
impl VehicleWheelState {
    pub(crate) fn new() -> Self {
        Self {
            target_heading: [0.; 4],
            radius: 0.,
            spin_angle: 0.,
            turn_angle: 0.,
        }
    }
    /// UpdateWheelAdjustments82d215d0. Only spin is frozen when skidding;
    /// angle normalization corrects one revolution, not an arbitrary modulo.
    pub(crate) fn update<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        velocity: [f32; 4],
        target_transform: Transform,
        skidding: bool,
        backing_up: bool,
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        if !skidding {
            let inverse_radius = 1. / self.radius;
            let displacement = velocity.map(|v| v * dt);
            let distance = try_length(displacement, numeric)?;
            let angle = inverse_radius * distance;
            let angle = if backing_up { -angle } else { angle };
            let sum = angle + self.spin_angle;
            let two_pi = f32::from_bits(0x40c9_0fdb);
            let lower = if sum >= 0. { sum } else { sum + two_pi };
            self.spin_angle = if sum - two_pi >= 0. {
                sum - two_pi
            } else {
                lower
            };
        }
        let forward = target_transform[2];
        let dot = numeric.dot3(forward, self.target_heading)?;
        let cross = cross_full4(forward, self.target_heading);
        // Same selected VMX min/max NaN policy as shared angle math. Ordinary
        // finite-input graph exact; Xenon exceptional selection not certified.
        let lower = if dot < -1. { -1. } else { dot };
        let bounded = if lower > 1. { 1. } else { lower };
        let angle = crate::reckoning_numeric::try_acos(bounded, numeric)?;
        self.turn_angle = if cross[1] >= 0. {
            angle
        } else {
            f32::from_bits(angle.to_bits() ^ 0x8000_0000)
        };
        Ok(())
    }
}
///82d17228. Raw bge means !LT (including unordered), unlike Rust >=.
pub(crate) fn front_wheel_position(position: f32, curve_length: f32, vehicle_length: f32) -> f32 {
    let half = vehicle_length * 0.5;
    if position < half {
        position * 2.
    } else {
        let remaining = curve_length - position;
        if remaining < vehicle_length {
            remaining.mul_add(0.5, position)
        } else {
            half + position
        }
    }
}
pub(crate) fn rear_wheel_position(position: f32, curve_length: f32, vehicle_length: f32) -> f32 {
    if position < vehicle_length {
        position * 0.5
    } else {
        let remaining = curve_length - position;
        if remaining < vehicle_length {
            -(remaining.mul_add(0.5, -position))
        } else {
            -(vehicle_length.mul_add(0.5, -position))
        }
    }
}
pub(crate) fn on_super_road<N: ReckoningNumeric>(
    road: &SuperRoad,
    transform: Transform,
    position: f32,
    lane: u32,
    offset: f32,
    vehicle_length: f32,
    rain: &RainMan,
    numeric: &mut N,
) -> Result<WheelTransformInfo, WheelError<N::Error>> {
    use WheelError::Geometry;
    let front = front_wheel_position(position, road.data.length, vehicle_length);
    let rear = rear_wheel_position(position, road.data.length, vehicle_length);
    let front_road = road.find_road(front, rain).map_err(Geometry)?;
    let rear_road = road.find_road(rear, rain).map_err(Geometry)?;
    let mut result = WheelTransformInfo {
        front_position: transform[3],
        front_tangent: transform[2],
        rear_position: transform[3],
        rear_tangent: transform[2],
    };
    if let Some(r) = front_road {
        let at = front
            - (r.record
                .scalar(0x90)
                .map_err(|e| Geometry(GeometryError::Asset(e)))?
                - r.record
                    .scalar(0x8c)
                    .map_err(|e| Geometry(GeometryError::Asset(e)))?);
        result.front_position = r
            .evaluate_position(at, lane as i32, offset, numeric)
            .map_err(Geometry)?;
        result.front_tangent = r
            .evaluate_tangent(at, lane as i32, offset, numeric)
            .map_err(Geometry)?;
    }
    if let Some(r) = rear_road {
        let at = rear
            - (r.record
                .scalar(0x90)
                .map_err(|e| Geometry(GeometryError::Asset(e)))?
                - r.record
                    .scalar(0x8c)
                    .map_err(|e| Geometry(GeometryError::Asset(e)))?);
        result.rear_position = r
            .evaluate_position(at, lane as i32, offset, numeric)
            .map_err(Geometry)?;
        result.rear_tangent = r
            .evaluate_tangent(at, lane as i32, offset, numeric)
            .map_err(Geometry)?;
    }
    Ok(result)
}
pub(crate) fn adjusted<N: ReckoningNumeric>(
    wheels: &WheelTransformInfo,
    transform: Transform,
    numeric: &mut N,
) -> Result<(Transform, [f32; 4]), N::Error> {
    let mut result = transform;
    let mut heading = wheels.front_tangent;
    let forward: [f32; 4] = std::array::from_fn(|i| wheels.front_position[i] - transform[3][i]);
    if try_length(forward, numeric)? > f32::from_bits(0x3dcc_cccd) {
        heading = transform[2];
    }
    let back: [f32; 4] = std::array::from_fn(|i| transform[3][i] - wheels.rear_position[i]);
    if try_length(back, numeric)? > f32::from_bits(0x3dcc_cccd) {
        result[2] = try_normalize_unsafe(back, numeric)?;
    }
    result[0] = try_normalize_unsafe(cross_full4(transform[1], result[2]), numeric)?;
    // Source keeps original Y and translation, does NOT orthogonalize Y again.
    Ok((result, heading))
}
///FillLaneChangePassageCurveParams82d1dc80. Uses adjusted chassis Z axes (not
///returned wheelHeading) for curve tangents, and the existing RAIN attributes.
pub(crate) fn fill_lane_curve<N: ReckoningNumeric>(
    passage: &mut DynPassage,
    road: &SuperRoad,
    offset: f32,
    vehicle_length: f32,
    rain: &RainMan,
    numeric: &mut N,
) -> Result<(), WheelError<N::Error>> {
    let start = passage.parameter_start_position;
    let end = passage.parameter_end_position;
    let from = road
        .evaluate_transform(start, passage.origin_lane, offset, rain, numeric)
        .map_err(WheelError::Geometry)?;
    let to = road
        .evaluate_transform(end, passage.destination_lane, offset, rain, numeric)
        .map_err(WheelError::Geometry)?;
    let factor = f32::from_bits(
        rain.rain_attributes
            .try_scalar_word(0xd2c1_1cc1_0c9e_0e49, 0)
            .map_err(WheelError::Attribute)?
            .unwrap_or(0),
    );
    let tangent_length = (end - start) * factor;
    let front_wheels = on_super_road(
        road,
        from,
        start,
        passage.origin_lane,
        offset,
        vehicle_length,
        rain,
        numeric,
    )?;
    let (from, _) = adjusted(&front_wheels, from, numeric).map_err(WheelError::Numeric)?;
    let rear_wheels = on_super_road(
        road,
        to,
        end,
        passage.destination_lane,
        offset,
        vehicle_length,
        rain,
        numeric,
    )?;
    let (to, _) = adjusted(&rear_wheels, to, numeric).map_err(WheelError::Numeric)?;
    let from_tangent = try_normalize_unsafe(from[2], numeric)
        .map_err(WheelError::Numeric)?
        .map(|v| v * tangent_length);
    let to_tangent = try_normalize_unsafe(to[2], numeric)
        .map_err(WheelError::Numeric)?
        .map(|v| v * tangent_length);
    passage
        .curve
        .set_params(from[3], from_tangent, to[3], to_tangent);
    let data = rain
        .rain_attributes
        .try_get_data(0xe350_79bd_d5ba_e286, 0)
        .map_err(WheelError::Attribute)?;
    let count = match data {
        Some(d) => *d.bytes().first().ok_or(WheelError::AttributeByte)? as i8 as i32,
        None => 0,
    };
    passage
        .curve
        .parameterize(count, numeric)
        .map_err(WheelError::Numeric)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wheels_shorten_baseline_at_both_curve_ends() {
        assert_eq!(front_wheel_position(1., 20., 4.), 2.);
        assert_eq!(rear_wheel_position(1., 20., 4.), 0.5);
        assert_eq!(front_wheel_position(10., 20., 4.), 12.);
        assert_eq!(rear_wheel_position(10., 20., 4.), 8.);
        assert_eq!(front_wheel_position(19., 20., 4.), 19.5);
        assert_eq!(rear_wheel_position(19., 20., 4.), 18.5);
    }
}
