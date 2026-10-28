use super::*;
use crate::{
    reckoning_numeric::HostReckoningNumeric, reckoning_sin_cos::HostReckoningSinCosNumeric,
};
struct Source {
    balance: f32,
    manual: bool,
    flags: u32,
    fail: Option<&'static str>,
    events: Vec<&'static str>,
    keys: Vec<u64>,
}
fn matrix() -> Matrix {
    Matrix::from_columns([
        Vector {
            xyz: [1.0, 0.0, 0.0],
            w: None,
        },
        Vector::known([0.0, 1.0, 0.0, 0.0]),
        Vector::known([0.0, 0.0, 1.0, 0.0]),
        Vector::known([0.0, 0.0, 0.0, 1.0]),
    ])
}
impl Source {
    fn event(&mut self, s: &'static str) -> Result<(), &'static str> {
        self.events.push(s);
        if self.fail == Some(s) { Err(s) } else { Ok(()) }
    }
}
impl ManualEffectSource for Source {
    type Error = &'static str;
    type Graph = ();
    fn scalar(&mut self, f: ManualScalar) -> Result<f32, Self::Error> {
        let (s, v) = match f {
            ManualScalar::FlippedControls => ("flip", 1.0),
            ManualScalar::Balance => ("balance", self.balance),
            ManualScalar::DeckZyDelta => ("delta", 0.0),
            ManualScalar::StateTimer => ("timer", 0.5),
            ManualScalar::AbsoluteSpeed => ("speed", 2.0),
            ManualScalar::TimeDelta => ("dt", 0.125),
        };
        self.event(s)?;
        Ok(v)
    }
    fn attribute(&mut self, k: u64) -> Result<f32, Self::Error> {
        self.keys.push(k);
        self.event(if k == 0xc990_6592_1011_63bd {
            "final_scale"
        } else {
            "attribute"
        })?;
        Ok(match k {
            0xe8c9_db10_77d1_d889 => 0.5,
            0xc990_6592_1011_63bd => 3.0,
            0x1bd8_1d97_e88e_2232 => 30.0,
            _ => 1.0,
        })
    }
    fn capture_speed_graph(&mut self) -> Result<(), Self::Error> {
        self.event("capture")
    }
    fn evaluate_graph(&mut self, (): &(), _: f32) -> Result<f32, Self::Error> {
        self.event("graph")?;
        Ok(1.0)
    }
    fn ground_transform(&mut self) -> Result<Matrix, Self::Error> {
        self.event("ground")?;
        Ok(matrix())
    }
    fn part_six_transform(&mut self) -> Result<Matrix, Self::Error> {
        self.event("part6")?;
        Ok(matrix())
    }
    fn manual_flag(&mut self) -> Result<bool, Self::Error> {
        self.event("manual")?;
        Ok(self.manual)
    }
    fn flags_8c8(&mut self) -> Result<u32, Self::Error> {
        self.event("flags")?;
        Ok(self.flags)
    }
    fn show_manual(&mut self) -> Result<bool, Self::Error> {
        self.event("debug")?;
        Ok(false)
    }
    fn wrong_wheel_scalar(&mut self) -> Result<f32, Self::Error> {
        self.event("wrong")?;
        Ok(0.25)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.event("overlay")
    }
}
fn source(balance: f32) -> Source {
    Source {
        balance,
        manual: true,
        flags: 0x0800_0000,
        fail: None,
        events: vec![],
        keys: vec![],
    }
}
fn output() -> ManualEffect {
    ManualEffect {
        angular_displacement: Vector::known([9.0; 4]),
        corrective_force: SkateboardFootForce {
            world_space_force: Vector::known([8.0; 4]),
            local_space_position: Vector::known([7.0; 4]),
        },
    }
}
#[test]
fn zero_balance_retains_prefix_reads_and_preserves_spin_history_and_unknown_w() {
    let mut entry = SkateboardEntryState::new();
    entry.manual_spin_angle = 17.0;
    entry.manual_torque = 4.0;
    let mut s = source(0.0);
    let mut out = output();
    entry
        .try_manual_effect(
            &mut s,
            false,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
            &mut out,
        )
        .unwrap();
    assert_eq!(
        s.events,
        [
            "flip",
            "balance",
            "attribute",
            "attribute",
            "attribute",
            "ground",
            "debug"
        ]
    );
    assert_eq!(entry.manual_torque.to_bits(), 0);
    assert_eq!(entry.manual_spin_angle.to_bits(), 17.0_f32.to_bits());
    assert_eq!(
        out.angular_displacement.xyz.map(f32::to_bits),
        [0x8000_0000; 3]
    );
    assert!(out.angular_displacement.w.is_none());
    assert_eq!(
        out.corrective_force.world_space_force.xyz.map(f32::to_bits),
        [0; 3]
    );
}
#[test]
fn early_failure_only_initializes_corrective_vectors() {
    let mut entry = SkateboardEntryState::new();
    entry.manual_torque = 4.0;
    let mut s = source(0.0);
    s.fail = Some("ground");
    let mut out = output();
    assert!(
        entry
            .try_manual_effect(
                &mut s,
                false,
                &mut HostReckoningNumeric,
                &mut HostReckoningSinCosNumeric,
                &mut out
            )
            .is_err()
    );
    assert_eq!(entry.manual_torque.to_bits(), 4.0_f32.to_bits());
    assert_eq!(
        out.angular_displacement.xyz.map(f32::to_bits),
        [9.0_f32.to_bits(); 3]
    );
    assert_eq!(
        out.corrective_force
            .local_space_position
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0; 4]
    );
}
#[test]
fn active_wrong_wheel_has_distinct_history_decay_and_final_output_scale() {
    let mut entry = SkateboardEntryState::new();
    entry.manual_torque = 8.0;
    entry.manual_time_since_reset = 1.0;
    entry.manual_spin_angle = 11.0;
    let mut s = source(1.0);
    let mut out = output();
    entry
        .try_manual_effect(
            &mut s,
            false,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
            &mut out,
        )
        .unwrap();
    assert_eq!(entry.manual_torque.to_bits(), 1.0_f32.to_bits()); //8*.5*.25
    assert_eq!(
        out.angular_displacement.xyz[0].to_bits(),
        (-0.75_f32).to_bits()
    ); //torque*(.25*3)
    assert_eq!(entry.manual_time_since_reset.to_bits(), 1.125_f32.to_bits());
    assert_eq!(entry.manual_spin_angle.to_bits(), 11.0_f32.to_bits());
    assert_eq!(s.events.iter().filter(|s| **s == "debug").count(), 2);
}
#[test]
fn missing_final_scale_preserves_decayed_history_but_not_angular_publication() {
    let mut entry = SkateboardEntryState::new();
    entry.manual_torque = 8.0;
    entry.manual_time_since_reset = 1.0;
    let mut s = source(1.0);
    s.fail = Some("final_scale");
    let mut out = output();
    assert!(
        entry
            .try_manual_effect(
                &mut s,
                false,
                &mut HostReckoningNumeric,
                &mut HostReckoningSinCosNumeric,
                &mut out
            )
            .is_err()
    );
    assert_eq!(entry.manual_torque.to_bits(), 1.0_f32.to_bits());
    assert_eq!(entry.manual_time_since_reset.to_bits(), 1.0_f32.to_bits());
    assert_eq!(
        out.angular_displacement.xyz.map(f32::to_bits),
        [9.0_f32.to_bits(); 3]
    );
}
#[test]
fn active_nonmanual_samples_sine_inputs_and_capture_before_speed() {
    let mut entry = SkateboardEntryState::new();
    let mut s = source(1.0);
    s.manual = false;
    let mut out = output();
    entry
        .try_manual_effect(
            &mut s,
            true,
            &mut HostReckoningNumeric,
            &mut HostReckoningSinCosNumeric,
            &mut out,
        )
        .unwrap();
    assert_eq!(
        &s.keys[..3],
        &[
            0x7ed1_8259_85b2_679a,
            0xee15_7c8b_8459_aeac,
            0xb1fb_fff9_8936_b84a
        ]
    );
    let capture = s.events.iter().position(|s| *s == "capture").unwrap();
    assert_eq!(
        &s.events[capture..capture + 3],
        &["capture", "speed", "graph"]
    );
    assert!(s.events.contains(&"timer"));
    assert!(s.events.contains(&"delta"));
}
