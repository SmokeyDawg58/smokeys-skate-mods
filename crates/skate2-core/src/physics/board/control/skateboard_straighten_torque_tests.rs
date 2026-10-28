use super::{StraightenError, StraightenSource, Vector, try_straighten};
use crate::reckoning_numeric::HostReckoningNumeric;
struct Source {
    minimum: f32,
    turn: f32,
    missing: Option<&'static str>,
    events: Vec<&'static str>,
}
impl Source {
    fn event(&mut self, e: &'static str) -> Result<(), &'static str> {
        self.events.push(e);
        if self.missing == Some(e) {
            Err(e)
        } else {
            Ok(())
        }
    }
}
impl StraightenSource for Source {
    type Error = &'static str;
    type Surface = ();
    fn wheel_hardness(&mut self) -> Result<f32, Self::Error> {
        self.event("hardness")?;
        Ok(0.5)
    }
    fn wheel_minimum(&mut self) -> Result<f32, Self::Error> {
        self.event("minimum")?;
        Ok(self.minimum)
    }
    fn capture_surface(&mut self) -> Result<(), Self::Error> {
        self.event("surface")
    }
    fn surface_duration(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.event("duration")?;
        Ok(4.0)
    }
    fn heading_graph(&mut self, _: f32) -> Result<f32, Self::Error> {
        self.event("graph")?;
        Ok(1.0)
    }
    fn body_z(&mut self) -> Result<Vector, Self::Error> {
        self.event("body_z")?;
        Ok(Vector::known([0.0, 0.0, 1.0, 3.0]))
    }
    fn velocity(&mut self) -> Result<Vector, Self::Error> {
        self.event("velocity")?;
        Ok(Vector::known([1.0, 0.0, 2.0, 0.0]))
    }
    fn surface_strength(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.event("strength")?;
        Ok(2.0)
    }
    fn ground_normal(&mut self) -> Result<Vector, Self::Error> {
        self.event("normal")?;
        Ok(Vector {
            xyz: [0.0, 1.0, 0.0],
            w: None,
        })
    }
    fn turn(&mut self) -> Result<f32, Self::Error> {
        self.event("turn")?;
        Ok(self.turn)
    }
    fn maximum_turn(&mut self) -> Result<f32, Self::Error> {
        self.event("maximum_turn")?;
        Ok(4.0)
    }
}
fn source() -> Source {
    Source {
        minimum: 0.5,
        turn: -1.0,
        missing: None,
        events: Vec::new(),
    }
}
#[test]
fn retained_first_output_survives_late_opposing_turn_attribute_failure() {
    let mut s = source();
    s.missing = Some("maximum_turn");
    let mut out = Vector::known([9.0; 4]);
    assert!(matches!(
        try_straighten(&mut s, 1.0, &mut HostReckoningNumeric, &mut out),
        Err(StraightenError::Source("maximum_turn"))
    ));
    assert_eq!(out.xyz.map(f32::to_bits), [0, 2.0_f32.to_bits(), 0]);
    assert_eq!(out.w, None);
    assert_eq!(
        s.events,
        [
            "hardness",
            "minimum",
            "surface",
            "duration",
            "graph",
            "surface",
            "body_z",
            "velocity",
            "strength",
            "normal",
            "turn",
            "maximum_turn"
        ]
    );
    s.missing = None;
    try_straighten(&mut s, 1.0, &mut HostReckoningNumeric, &mut out).unwrap();
    assert_eq!(out.xyz[1].to_bits(), 1.5_f32.to_bits());
}
#[test]
fn invalid_minimum_stops_before_output_but_nan_minimum_and_turn_take_source_branches() {
    let mut s = source();
    s.minimum = 2.0;
    let mut out = Vector::known([9.0; 4]);
    assert!(matches!(
        try_straighten(&mut s, 1.0, &mut HostReckoningNumeric, &mut out),
        Err(StraightenError::WheelMinimumAssertion { .. })
    ));
    assert_eq!(out.xyz.map(f32::to_bits), [9.0_f32.to_bits(); 3]);
    s.minimum = f32::NAN;
    s.turn = f32::NAN;
    s.events.clear();
    try_straighten(&mut s, 1.0, &mut HostReckoningNumeric, &mut out).unwrap();
    assert!(!s.events.contains(&"maximum_turn"));
    assert_eq!(out.xyz[1].to_bits(), 2.0_f32.to_bits());
}
