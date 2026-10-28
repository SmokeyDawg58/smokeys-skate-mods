use super::{AntiFlipError, AntiFlipSource, Vector, positive_cosine, try_antiflip};
use crate::reckoning_numeric::HostReckoningNumeric;

struct Source {
    manual: bool,
    balance: f32,
    draw: bool,
    missing: Option<&'static str>,
    events: Vec<&'static str>,
    z: Vector,
    x: Vector,
}
impl Source {
    fn new() -> Self {
        Self {
            manual: false,
            balance: 0.0,
            draw: false,
            missing: None,
            events: Vec::new(),
            z: Vector {
                xyz: [0.0, 0.0, 1.0],
                w: None,
            },
            x: Vector::known([1.0, 0.0, 0.0, 7.0]),
        }
    }
    fn event(&mut self, e: &'static str) -> Result<(), &'static str> {
        self.events.push(e);
        if self.missing == Some(e) {
            Err(e)
        } else {
            Ok(())
        }
    }
}
impl AntiFlipSource for Source {
    type Error = &'static str;
    fn manual_flag(&mut self) -> Result<bool, Self::Error> {
        self.event("manual")?;
        Ok(self.manual)
    }
    fn balance(&mut self) -> Result<f32, Self::Error> {
        self.event("balance")?;
        Ok(self.balance)
    }
    fn deck_z(&mut self) -> Result<Vector, Self::Error> {
        self.event("z")?;
        Ok(self.z)
    }
    fn up(&mut self) -> Result<Vector, Self::Error> {
        self.event("up")?;
        Ok(Vector::known([0.0, 1.0, 0.0, 0.0]))
    }
    fn deck_x(&mut self) -> Result<Vector, Self::Error> {
        self.event("x")?;
        Ok(self.x)
    }
    fn feet_graph(&mut self, o: usize, _: f32) -> Result<f32, Self::Error> {
        self.event(if o == 0xb0 {
            "pitch"
        } else {
            assert_eq!(o, 0x60);
            "roll"
        })?;
        Ok(if o == 0xb0 { 2.0 } else { 3.0 })
    }
    fn draw(&mut self) -> Result<bool, Self::Error> {
        self.event("draw")?;
        Ok(self.draw)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.event("overlay")
    }
}
#[test]
fn manual_read_precedes_zero_and_manual_branch_is_lazy() {
    let mut source = Source::new();
    source.missing = Some("manual");
    let mut output = Vector::known([9.0; 4]);
    assert!(try_antiflip(&mut source, &mut HostReckoningNumeric, &mut output).is_err());
    assert_eq!(output.xyz.map(f32::to_bits), [9.0_f32.to_bits(); 3]);
    source.missing = None;
    source.manual = true;
    source.events.clear();
    try_antiflip(&mut source, &mut HostReckoningNumeric, &mut output).unwrap();
    assert_eq!(source.events, ["manual"]);
    assert_eq!(output.xyz.map(f32::to_bits), [0; 3]);
    assert_eq!(output.w.map(f32::to_bits), Some(0));
}
#[test]
fn debug_read_precedes_output_but_overlay_failure_preserves_final_value() {
    let mut source = Source::new();
    source.missing = Some("draw");
    let mut output = Vector::known([9.0; 4]);
    assert!(try_antiflip(&mut source, &mut HostReckoningNumeric, &mut output).is_err());
    assert_eq!(output.xyz.map(f32::to_bits), [0; 3]);
    source.missing = Some("overlay");
    source.draw = true;
    source.events.clear();
    assert!(matches!(
        try_antiflip(&mut source, &mut HostReckoningNumeric, &mut output),
        Err(AntiFlipError::Source("overlay"))
    ));
    assert_eq!(
        output.xyz.map(f32::to_bits),
        [2.0_f32.to_bits(), 0, 3.0_f32.to_bits()]
    );
    assert_eq!(output.w, None);
    assert_eq!(
        source.events,
        [
            "manual", "balance", "z", "up", "x", "pitch", "roll", "draw", "overlay"
        ]
    );
}
#[test]
fn nonzero_balance_disables_only_pitch_and_cosine_clamp_preserves_exceptional_input() {
    let mut source = Source::new();
    source.balance = f32::NAN;
    let mut output = Vector::known([9.0; 4]);
    try_antiflip(&mut source, &mut HostReckoningNumeric, &mut output).unwrap();
    assert_eq!(output.xyz.map(f32::to_bits), [0, 0, 3.0_f32.to_bits()]);
    assert!(source.events.contains(&"pitch")); // Still calculates its graph.
    assert_eq!(positive_cosine(-0.5).to_bits(), 0);
    assert_eq!(positive_cosine(2.0).to_bits(), 1.0_f32.to_bits());
    assert_eq!(positive_cosine(-0.0).to_bits(), (-0.0_f32).to_bits());
    assert!(positive_cosine(f32::NAN).is_nan());
}
