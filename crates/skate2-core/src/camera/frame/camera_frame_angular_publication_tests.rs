use super::*;

#[derive(Default)]
struct Numeric {
    truncate_calls: usize,
    fail_on: Option<usize>,
}
impl ReckoningNumeric for Numeric {
    type Error = ();
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, ()> {
        unreachable!()
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, ()> {
        unreachable!()
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, ()> {
        unreachable!()
    }
    fn truncate_i32(&mut self, value: f32) -> Result<i32, ()> {
        self.truncate_calls += 1;
        if self.fail_on == Some(self.truncate_calls) {
            return Err(());
        }
        Ok(value as i32)
    }
}

#[test]
fn normalizes_xyz_in_order_but_scales_raw_w() {
    let old = Vector::known([0.0; 4]);
    let new = Vector::known([4.0, 5.0, 6.0, 4.0]);
    let mut published = None;
    let mut numeric = Numeric::default();
    publish_angular_from_eulers(&mut published, old, new, 2.0, &mut numeric).unwrap();
    let result = published.unwrap().known_lanes().unwrap();
    assert_eq!(numeric.truncate_calls, 3);
    assert_eq!(result[3], 8.0); // not normalized X times inverse_dt
    assert!(result[0] < 0.0 && result[1] < 0.0 && result[2] < 0.0);
}

#[test]
fn normalization_failure_preserves_the_last_source_store() {
    let old = Vector::known([0.0; 4]);
    let new = Vector::known([4.0, 5.0, 6.0, 4.0]);
    let mut published = None;
    let mut numeric = Numeric {
        fail_on: Some(2),
        ..Numeric::default()
    };
    assert!(publish_angular_from_eulers(&mut published, old, new, 2.0, &mut numeric).is_err());
    assert_eq!(numeric.truncate_calls, 2);
    let partial = published.unwrap().known_lanes().unwrap();
    assert!(partial[0] < 0.0); // X normalized before Y failed.
    assert_eq!(partial[1..], [5.0, 6.0, 4.0]);
}
