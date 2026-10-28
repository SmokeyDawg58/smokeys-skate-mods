//! Original DynCurve/DynPassage. The native length is uninitialized until
//! ParameterizeByArclength; keep that distinction while sharing Hermite math.
use super::curve::{CurveError, arc_parameter, hermite};
use crate::reckoning_numeric::{ReckoningNumeric, host_dot4, length_from_squared_and_estimate};

pub(crate) struct DynCurve {
    pub(crate) rows: [[f32; 4]; 4],
    pub(crate) length: Option<f32>,
    pub(crate) parameters: Vec<f32>,
}
impl DynCurve {
    fn new() -> Self {
        Self {
            rows: [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
            length: None,
            parameters: Vec::new(),
        }
    }
    /// SetParams82e4df60 stores from/to points followed by from/to tangents.
    pub(crate) fn set_params(
        &mut self,
        from: [f32; 4],
        from_tangent: [f32; 4],
        to: [f32; 4],
        to_tangent: [f32; 4],
    ) {
        self.rows = [from, to, from_tangent, to_tangent];
    }
    pub(crate) fn point(&self, t: f32) -> [f32; 4] {
        hermite(self.rows, [t * t * t, t * t, t, 1.])
    }
    pub(crate) fn tangent(&self, t: f32) -> [f32; 4] {
        hermite(self.rows, [(3. * t) * t, 2. * t, 1., 0.])
    }
    pub(crate) fn parameter<N: ReckoningNumeric>(
        &self,
        arc: f32,
        numeric: &mut N,
    ) -> Result<f32, CurveError<N::Error>> {
        arc_parameter(&self.parameters, arc, numeric)
    }
    ///82e5fcb8. `points` is the number of intervals: result has points+1
    /// samples. Invalid count returns0 without changing length or old samples.
    pub(crate) fn parameterize<N: ReckoningNumeric>(
        &mut self,
        points: i32,
        numeric: &mut N,
    ) -> Result<f32, N::Error> {
        if points < 2 {
            return Ok(0.);
        }
        let mut previous = self.rows[0];
        self.length = Some(0.);
        self.parameters.clear();
        let mut total = 0.;
        for i in 1..=points {
            self.parameters.push(total);
            let point = self.point(i as f32 / points as f32);
            let delta: [f32; 4] = std::array::from_fn(|lane| point[lane] - previous[lane]);
            previous = point;
            // vmsum4fp82e5fe08, including W; SAME selected host dot4 policy as
            // quaternion/camera math, not a claim of Xenon reduction bit parity.
            let squared = host_dot4(delta, delta);
            let estimate = numeric.reciprocal_sqrt_estimate(squared)?;
            total += length_from_squared_and_estimate(squared, estimate);
            self.length = Some(total);
        }
        self.parameters.push(total);
        Ok(total)
    }
}
pub(crate) struct DynPassage {
    pub(crate) curve: DynCurve,
    pub(crate) maximum_speed: f32,
    pub(crate) parameter_start_position: f32,
    pub(crate) parameter_end_position: f32,
    pub(crate) origin_pin: u32,
    pub(crate) origin_lane: u32,
    pub(crate) destination_pin: u32,
    pub(crate) destination_lane: u32,
    pub(crate) initialized: bool,
    pub(crate) valid: bool,
}
impl DynPassage {
    ///82e5bbb0: identity rows, empty samples; curve length is NOT written.
    pub(crate) fn new() -> Self {
        Self {
            curve: DynCurve::new(),
            maximum_speed: 0.,
            parameter_start_position: 0.,
            parameter_end_position: 0.,
            origin_pin: 0,
            origin_lane: 0,
            destination_pin: 0,
            destination_lane: 0,
            initialized: false,
            valid: false,
        }
    }
    pub(crate) fn init(&mut self) {
        self.valid = true;
        self.initialized = true;
    } //82e4df98
    ///82e591f0 leaves curve rows/length and allocated sample capacity intact.
    pub(crate) fn deinit(&mut self) {
        self.maximum_speed = 0.;
        self.parameter_start_position = 0.;
        self.parameter_end_position = 0.;
        self.origin_pin = u32::MAX;
        self.origin_lane = u32::MAX;
        self.destination_pin = u32::MAX;
        self.destination_lane = u32::MAX;
        self.curve.parameters.clear();
        self.valid = false;
        self.initialized = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;
    #[test]
    fn invalid_count_keeps_old_curve_and_deinit_keeps_length() {
        let mut p = DynPassage::new();
        let mut n = HostReckoningNumeric;
        assert_eq!(p.curve.length, None);
        p.curve.set_params(
            [0.; 4],
            [0., 0., 2., 0.],
            [0., 0., 2., 0.],
            [0., 0., 2., 0.],
        );
        let length = p.curve.parameterize(4, &mut n).unwrap();
        assert_eq!(p.curve.parameters.len(), 5);
        assert!((length - 2.).abs() < 0.0001);
        assert_eq!(p.curve.parameterize(1, &mut n).unwrap(), 0.);
        assert_eq!(p.curve.parameters.len(), 5);
        p.init();
        p.deinit();
        assert_eq!(p.curve.length, Some(length));
        assert!(p.curve.parameters.is_empty());
        assert_eq!(p.origin_lane, u32::MAX);
    }
}
