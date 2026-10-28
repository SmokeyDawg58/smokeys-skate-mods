//! RAIN Curve methods and shared Hermite math, traced through raw VMX operands.
use super::asset::CurveData;
use crate::{
    reckoning_numeric::ReckoningNumeric, simulation_drive_jacobian::refine_drive_reciprocal,
};

#[derive(Debug)]
pub(crate) enum CurveError<E> {
    Numeric(E),
    BeforeFirstArcSample,
}
impl CurveData {
    /// CalculateParamFromArcLength82e5b8d0. upper_bound, not lower_bound;
    /// an endpoint or a value beyond the final sample returns exactly1.
    pub(crate) fn parameter<N: ReckoningNumeric>(
        &self,
        arc: f32,
        numeric: &mut N,
    ) -> Result<f32, CurveError<N::Error>> {
        arc_parameter(&self.parameters, arc, numeric)
    }
    /// Curve.CalculatePointFromParam82e4de80 -> Hermite.Evaluate829b0f70.
    pub(crate) fn point(&self, t: f32) -> [f32; 4] {
        hermite(self.rows, [t * t * t, t * t, t, 1.0])
    }
    /// Curve.CalculateTangentFromParam82e4deb8 -> Hermite829b10b0.
    /// Native multiplies (3*t)*t, not 3*(t*t), and does not normalize.
    pub(crate) fn tangent(&self, t: f32) -> [f32; 4] {
        hermite(self.rows, [(3.0 * t) * t, 2.0 * t, 1.0, 0.0])
    }
}
/// Static Curve82e5b8d0 and DynCurve82e5ba48 share this scalar graph.
pub(super) fn arc_parameter<N: ReckoningNumeric>(
    parameters: &[f32],
    arc: f32,
    numeric: &mut N,
) -> Result<f32, CurveError<N::Error>> {
    let mut first = 0;
    let mut count = parameters.len();
    while count != 0 {
        let half = count / 2;
        let middle = first + half;
        if arc < parameters[middle] {
            count = half;
        } else {
            first = middle + 1;
            count -= half + 1;
        }
    }
    if first == parameters.len() {
        return Ok(1.0);
    }
    let previous = first
        .checked_sub(1)
        .ok_or(CurveError::BeforeFirstArcSample)?;
    let span = parameters[first] - parameters[previous];
    let inv_span = refine_drive_reciprocal(
        span,
        numeric
            .reciprocal_estimate(span)
            .map_err(CurveError::Numeric)?,
    );
    let intervals = (parameters.len() - 1) as f32;
    let inv_intervals = refine_drive_reciprocal(
        intervals,
        numeric
            .reciprocal_estimate(intervals)
            .map_err(CurveError::Numeric)?,
    );
    let numerator = inv_span.mul_add(arc - parameters[previous], first as f32 - 1.0);
    Ok(inv_intervals * numerator)
}
pub(super) fn hermite(rows: [[f32; 4]; 4], weights: [f32; 4]) -> [f32; 4] {
    // Dynamic initializer82f76448, four row vectors in hermiteBasisMat.
    const BASIS: [[f32; 4]; 4] = [
        [2., -2., 1., 1.],
        [-3., 3., -2., -1.],
        [0., 0., 1., 0.],
        [1., 0., 0., 0.],
    ];
    let coefficients: [[f32; 4]; 4] = std::array::from_fn(|i| {
        std::array::from_fn(|lane| {
            let v = rows[0][lane] * BASIS[i][0];
            let v = rows[1][lane].mul_add(BASIS[i][1], v);
            let v = rows[2][lane].mul_add(BASIS[i][2], v);
            rows[3][lane].mul_add(BASIS[i][3], v)
        })
    });
    let mut result: [f32; 4] = std::array::from_fn(|lane| {
        let v = coefficients[0][lane] * weights[0];
        let v = coefficients[1][lane].mul_add(weights[1], v);
        let v = coefficients[2][lane].mul_add(weights[2], v);
        coefficients[3][lane].mul_add(weights[3], v)
    });
    // Final permute82fd6e90 + vrlimi laneZ publishes (x,y,z,x).
    result[3] = result[0];
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;
    #[test]
    fn authored_arc_samples_control_speed_and_endpoint_is_exact() {
        let curve = CurveData {
            rows: [[0.; 4]; 4],
            length: 10.,
            parameters: vec![0., 2., 10.],
        };
        let mut n = HostReckoningNumeric;
        assert_eq!(curve.parameter(0., &mut n).unwrap(), 0.);
        assert_eq!(curve.parameter(2., &mut n).unwrap(), 0.5);
        assert_eq!(curve.parameter(6., &mut n).unwrap(), 0.75);
        assert_eq!(curve.parameter(10., &mut n).unwrap(), 1.);
        assert_eq!(curve.parameter(15., &mut n).unwrap(), 1.);
        assert!(matches!(
            curve.parameter(-1., &mut n),
            Err(CurveError::BeforeFirstArcSample)
        ));
    }
    #[test]
    fn hermite_preserves_authored_endpoints_tangents_and_native_w_lane() {
        let curve = CurveData {
            rows: [
                [1., 2., 3., 0.],
                [9., 4., 5., 0.],
                [2., 0., 1., 0.],
                [3., 1., 0., 0.],
            ],
            length: 0.,
            parameters: vec![],
        };
        assert_eq!(curve.point(0.), [1., 2., 3., 1.]);
        assert_eq!(curve.point(1.), [9., 4., 5., 9.]);
        assert_eq!(curve.tangent(0.), [2., 0., 1., 2.]);
        assert_eq!(curve.tangent(1.), [3., 1., 0., 3.]);
    }
}
