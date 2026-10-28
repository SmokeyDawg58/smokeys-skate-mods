use super::{
    HeadingGlobal, HeadingScalar as F, HeadingTorqueError, HeadingTorqueSource, HeadingVector as V,
    Matrix, SkateboardEntryState, Vector,
};
use crate::reckoning_numeric::HostReckoningNumeric;
struct Source {
    balance: f32,
    missing: Option<&'static str>,
    nan_feedback: bool,
    draw: bool,
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
impl HeadingTorqueSource for Source {
    type Error = &'static str;
    type Graph = usize;
    fn scalar(&mut self, f: F) -> Result<f32, Self::Error> {
        let (e, v) = match f {
            F::Balance => ("balance", self.balance),
            F::ManualTurn => ("manual_turn", 2.0),
            F::DeckSpeed => ("deck_speed", 4.0),
            F::TimeDelta => ("dt", 0.5),
            F::ForwardSpeed => ("speed", 3.0),
            F::VertTurn => ("vert_turn", 1.0),
            F::HeadingFactor => ("factor", 1.0),
        };
        self.event(e)?;
        Ok(v)
    }
    fn vector(&mut self, f: V) -> Result<Vector, Self::Error> {
        let (e, v) = match f {
            V::GroundNormal => ("normal", [0.0, 1.0, 0.0]),
            V::ZAxisVelocity => ("velocity", [2.0, 0.0, 3.0]),
            V::AngularVelocity => ("angular", [0.0, 1.0, 0.0]),
        };
        self.event(e)?;
        Ok(Vector { xyz: v, w: None })
    }
    fn flags_8c8(&mut self) -> Result<u32, Self::Error> {
        self.event("flags")?;
        Ok(0)
    }
    fn attribute(&mut self, _: HeadingGlobal, key: u64) -> Result<f32, Self::Error> {
        self.event("attribute")?;
        Ok(if key == 0xe56a_c6f5_81f2_799c {
            0.25
        } else {
            1.0
        })
    }
    fn capture_graph(&mut self, _: HeadingGlobal, o: usize) -> Result<usize, Self::Error> {
        self.event("capture")?;
        Ok(o)
    }
    fn evaluate_graph(&mut self, g: &usize, _: f32) -> Result<f32, Self::Error> {
        self.event("graph")?;
        Ok(if *g == 0x110 && self.nan_feedback {
            f32::NAN
        } else {
            1.0
        })
    }
    fn ground_transform(&mut self) -> Result<Matrix, Self::Error> {
        self.event("transform")?;
        Ok(Matrix::from_columns([
            Vector::known([1.0, 0.0, 0.0, 0.0]),
            Vector::known([0.0, 1.0, 0.0, 0.0]),
            Vector::known([0.0, 0.0, 1.0, 0.0]),
            Vector::known([0.0; 4]),
        ]))
    }
    fn draw_vert_turning(&mut self) -> Result<bool, Self::Error> {
        self.event("draw")?;
        Ok(self.draw)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.event("overlay")
    }
}
fn source(balance: f32) -> Source {
    Source {
        balance,
        missing: None,
        nan_feedback: false,
        draw: false,
        events: Vec::new(),
    }
}
#[test]
fn manual_smoothing_publishes_before_late_dt_failure() {
    let mut state = SkateboardEntryState::new();
    state.manual_spin_angle = 4.0;
    state.manual_torque = 13.0;
    let mut source = source(1.0);
    source.missing = Some("dt");
    let mut output = Vector::known([9.0; 4]);
    assert!(matches!(
        state.try_heading_torque(&mut source, &mut HostReckoningNumeric, &mut output),
        Err(HeadingTorqueError::Source("dt"))
    ));
    let expected = 0.75_f32.mul_add(4.0, f32::from_bits(0x3c8e_fa35) * 0.25);
    assert_eq!(state.manual_spin_angle.to_bits(), expected.to_bits());
    assert_eq!(state.manual_torque.to_bits(), 13.0_f32.to_bits());
    assert_eq!(output.xyz.map(f32::to_bits), [9.0_f32.to_bits(); 3]);
    assert_eq!(
        source.events,
        [
            "balance",
            "manual_turn",
            "attribute",
            "flags",
            "manual_turn",
            "attribute",
            "capture",
            "deck_speed",
            "graph",
            "dt"
        ]
    );
}
#[test]
fn nonmanual_clears_same_state_before_transform_and_short_circuits_manual_turn() {
    let mut state = SkateboardEntryState::new();
    state.manual_spin_angle = 8.0;
    state.manual_torque = 17.0;
    let mut source = source(0.0);
    source.missing = Some("transform");
    let mut output = Vector::known([9.0; 4]);
    assert!(
        state
            .try_heading_torque(&mut source, &mut HostReckoningNumeric, &mut output)
            .is_err()
    );
    assert_eq!(state.manual_spin_angle.to_bits(), 0);
    assert_eq!(state.manual_torque.to_bits(), 17.0_f32.to_bits());
    assert_eq!(source.events, ["balance", "transform"]);
    assert_eq!(output.xyz.map(f32::to_bits), [9.0_f32.to_bits(); 3]);
}
#[test]
fn unordered_candidate_selects_existing_torque_and_overlay_precedes_output() {
    let mut state = SkateboardEntryState::new();
    let mut source = source(0.0);
    source.nan_feedback = true;
    let mut output = Vector::known([9.0; 4]);
    state
        .try_heading_torque(&mut source, &mut HostReckoningNumeric, &mut output)
        .unwrap();
    assert!(output.xyz.iter().all(|v| v.is_finite()));
    assert_eq!(output.w, None);
    source.draw = true;
    source.missing = Some("overlay");
    output = Vector::known([9.0; 4]);
    assert!(
        state
            .try_heading_torque(&mut source, &mut HostReckoningNumeric, &mut output)
            .is_err()
    );
    assert_eq!(output.xyz.map(f32::to_bits), [9.0_f32.to_bits(); 3]);
}

#[test]
fn manual_reset_and_force_scale_preserve_distinct_spin_angle() {
    let mut state = SkateboardEntryState::new();
    assert_eq!(state.manual_spin_angle.to_bits(), 0);
    state.manual_spin_angle = -3.0;
    state.manual_torque = 5.0;
    state.scale_manual_force(2.0);
    assert_eq!(state.manual_torque.to_bits(), 10.0_f32.to_bits());
    assert_eq!(state.manual_spin_angle.to_bits(), (-3.0_f32).to_bits());
    state.reset_manual();
    assert_eq!(state.manual_torque.to_bits(), 0);
    assert_eq!(state.manual_spin_angle.to_bits(), (-3.0_f32).to_bits());
}
