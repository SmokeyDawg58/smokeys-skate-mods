use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;

#[derive(Default)]
struct SourceFixture {
    correction_graph: bool,
    debug_calls: usize,
    fail_final_debug: bool,
    unknown_normal_w: bool,
    delta: Option<V>,
}
impl Source for SourceFixture {
    type Error = &'static str;
    fn vector(&mut self, field: Vector) -> Result<V, Self::Error> {
        Ok(match field {
            Vector::Normal if self.unknown_normal_w => V {
                xyz: [0.0, 1.0, 0.0],
                w: None,
            },
            Vector::Normal | Vector::ReckoningUp => UP,
            Vector::Prepared | Vector::Ground | Vector::Com => ZERO,
            Vector::Velocity => V::known([2.0, 0.0, 0.0, 0.0]),
            Vector::BodyZ => V::known([0.0, 0.0, 1.0, 0.0]),
        })
    }
    fn scalar(&mut self, field: Scalar) -> Result<f32, Self::Error> {
        Ok(match field {
            Scalar::Strength => 1.0,
            Scalar::Gravity => -10.0,
            Scalar::SurfaceSpeed | Scalar::AdjustX | Scalar::AdjustZ => 0.0,
        })
    }
    fn attribute(&mut self, field: A) -> Result<f32, Self::Error> {
        Ok(match field {
            A::ReferenceSpeed => 10.0,
            A::BaseHeight | A::MinimumHeight => 1.0,
            A::MaximumHeight => 2.0,
            A::MinimumVerticalResponse
            | A::MaximumYCompensation
            | A::SidewaysScale
            | A::ForwardScale => 0.0,
            A::VerticalResponseEnabled => panic!("byte attribute has its own accessor"),
        })
    }
    fn graph(&mut self, field: G, _input: f32) -> Result<f32, Self::Error> {
        match field {
            G::TangentLimit => self.correction_graph = true,
            G::VerticalResponse => panic!("disabled vertical response must not read its graph"),
            _ => {}
        }
        Ok(1.0)
    }
    fn vertical_response_enabled(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn publish_delta(&mut self, value: V) -> Result<(), Self::Error> {
        self.delta = Some(value);
        Ok(())
    }
    fn debug(&mut self) -> Result<(), Self::Error> {
        self.debug_calls += 1;
        if self.fail_final_debug && self.delta.is_some() {
            Err("late debug")
        } else {
            Ok(())
        }
    }
}

#[test]
fn correction_compares_squared_error_to_unsquared_maximum_with_ordered_gt() {
    for (maximum, correction) in [(3.0, true), (4.0, false), (f32::NAN, false)] {
        let mut source = SourceFixture::default();
        calculate(maximum, &mut source, &mut HostReckoningNumeric).unwrap();
        assert_eq!(source.correction_graph, correction);
        assert_eq!(source.debug_calls, if correction { 2 } else { 1 });
    }
}

#[test]
fn board_delta_store_survives_late_debug_failure() {
    let mut source = SourceFixture {
        fail_final_debug: true,
        ..SourceFixture::default()
    };
    assert!(matches!(
        calculate(4.0, &mut source, &mut HostReckoningNumeric),
        Err(Error::Source("late debug"))
    ));
    let delta = source.delta.unwrap();
    assert_eq!(delta.xyz[0].to_bits(), (-2.0_f32).to_bits());
    assert!(delta.xyz[1] > 6.0);
}

#[test]
fn unknown_normal_w_is_retained_without_blocking_xyz_jump_math() {
    let mut source = SourceFixture {
        unknown_normal_w: true,
        ..SourceFixture::default()
    };
    let velocity = calculate(4.0, &mut source, &mut HostReckoningNumeric).unwrap();
    assert!(velocity.xyz.iter().all(|v| v.is_finite()));
    assert!(velocity.w.is_none());
    assert!(source.delta.unwrap().w.is_none());
}
