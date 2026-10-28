#![expect(
    clippy::float_cmp,
    reason = "exact source branch selections and publications"
)]
use super::*;
#[test]
fn smoothing_fuses_previous_term_after_rounding_target_product() {
    assert_eq!(
        smooth(
            f32::from_bits(0x3e16_b117),
            f32::from_bits(0xbf0c_b772),
            f32::from_bits(0x3eef_a3e2)
        )
        .to_bits(),
        0x3ea2_f584
    );
}
#[derive(Default)]
struct Probe {
    log: Vec<String>,
    flags: [u32; 2],
    combined: u32,
    fail_set: Option<usize>,
    written: Vec<(FastString30, f32)>,
    motion: Vec<(String, f32)>,
    reset: Vec<usize>,
}
impl reshape::ReshapeSource for Probe {
    type Error = &'static str;
    fn graph(&mut self, offset: usize, _: f32) -> Result<f32, Self::Error> {
        self.log.push(format!("graph{offset:x}"));
        Ok(if offset == 0x290 { 1.0 } else { 0.0 })
    }
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
        self.log.push(format!("scalar{key:x}"));
        Ok(if key == 0xd35c_a989_8b86_827a {
            1.0
        } else {
            0.0
        })
    }
    fn reshape_cvar(&mut self, index: usize) -> Result<bool, Self::Error> {
        self.log.push(format!("cvar{index}"));
        Ok(false)
    }
}
impl TurningSource for Probe {
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error> {
        self.log.push("resolve".into());
        Ok(())
    }
    fn riding_fakie(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    fn capture_intents(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn output(&mut self, _: Output) -> Result<f32, Self::Error> {
        Ok(0.0)
    }
    fn missing_instance(&mut self) -> Self::Error {
        "instance"
    }
    fn slide_flags(&mut self, side: usize, _: u32) -> Result<u32, Self::Error> {
        Ok(self.flags[side])
    }
    fn clear_force(&mut self, side: usize) -> Result<(), Self::Error> {
        self.flags[side] &= !0x2000_0000;
        self.log.push(format!("clear{side}"));
        Ok(())
    }
    fn combined(&mut self, _: u32) -> Result<u32, Self::Error> {
        Ok(self.combined)
    }
    fn reset_slide_time(&mut self, side: usize) -> Result<(), Self::Error> {
        self.reset.push(side);
        Ok(())
    }
    fn set_slide_fakie(&mut self, fakie: bool) -> Result<(), Self::Error> {
        self.combined = (self.combined & !0x8000_0000) | if fakie { 0x8000_0000 } else { 0 };
        Ok(())
    }
    fn mirrored(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    fn debug(&mut self) -> Result<(), Self::Error> {
        self.log.push("debug".into());
        Ok(())
    }
    fn dt(&mut self) -> Result<f32, Self::Error> {
        Ok(0.25)
    }
    fn speed(&mut self) -> Result<f32, Self::Error> {
        Ok(f32::NAN)
    }
    fn settable(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error> {
        if self.fail_set == Some(self.written.len()) {
            return Err("settable");
        }
        self.written.push((name, value));
        Ok(())
    }
    fn slide_turn(&mut self) -> Result<f32, Self::Error> {
        self.log.push("slide_turn".into());
        Ok(0.75)
    }
    fn intent(&mut self, name: &str) -> Result<Option<f32>, Self::Error> {
        self.log.push(name.into());
        Ok(if name == "fakieturn" {
            None
        } else {
            Some(0.25)
        })
    }
    fn motion_attribute(&mut self, name: &str, value: f32) -> Result<(), Self::Error> {
        self.motion.push((name.into(), value));
        Ok(())
    }
}
fn names() -> Names {
    Names {
        angle: FastString30::encode("angle").unwrap(),
        direction: FastString30::encode("dir").unwrap(),
        quickness: FastString30::encode("quickness").unwrap(),
        holding: FastString30::encode("holding").unwrap(),
        speed: FastString30::encode("speedtuck").unwrap(),
    }
}
fn instance() -> TurningInstance {
    TurningInstance {
        time: 5.0,
        value: 0.5,
        state: 2,
    }
}
#[test]
fn neutral_executes_four_reshapes_and_all_late_reads() {
    let mut probe = Probe::default();
    let mut instance = instance();
    update(&mut probe, Some(&mut instance), names()).unwrap();
    let graphs: Vec<_> = probe
        .log
        .iter()
        .filter(|s| s.starts_with("graph"))
        .map(String::as_str)
        .collect();
    assert_eq!(
        graphs,
        [
            "graph120", "graph1b0", "graph120", "graph1b0", "graph0", "graph90", "graph0",
            "graph90", "graph290"
        ]
    );
    for index in [234, 235, 236] {
        assert_eq!(
            probe
                .log
                .iter()
                .filter(|s| **s == format!("cvar{index}"))
                .count(),
            4
        );
    }
    assert_eq!(
        probe.written.iter().map(|p| p.0).collect::<Vec<_>>(),
        [
            names().angle,
            names().direction,
            names().quickness,
            names().speed,
            names().holding
        ]
    );
    assert_eq!(probe.written[3].1, 0.0); // unordered speed does not evaluate speed graph
    assert_eq!(
        &probe.log[probe.log.len() - 3..],
        ["slide_turn", "fakieturn", "debug"]
    );
    assert!(probe.motion.is_empty());
    assert_eq!(instance.time, 5.25);
}
#[test]
fn right_priority_forced_start_preserves_time_and_fakie() {
    let mut probe = Probe {
        flags: [0xa000_0000, 0x8000_0000],
        combined: 0x4000_0000,
        ..Probe::default()
    };
    let mut instance = instance();
    update(&mut probe, Some(&mut instance), names()).unwrap();
    assert_eq!(instance.state, 1);
    assert_eq!(instance.time, 0.25);
    assert_eq!(instance.value, 1.0);
    assert!(probe.reset.is_empty());
    assert_eq!(probe.combined, 0x4000_0000);
    assert_eq!(
        probe.motion,
        [("turn".into(), 0.75), ("slide".into(), 0.25)]
    );
    assert_eq!(probe.flags[0], 0x8000_0000);
}
#[test]
fn publication_failure_keeps_prefix_and_instance_writes() {
    let mut probe = Probe {
        fail_set: Some(2),
        ..Probe::default()
    };
    let mut instance = instance();
    assert_eq!(
        update(&mut probe, Some(&mut instance), names()),
        Err("settable")
    );
    assert_eq!(probe.written.len(), 2);
    assert_eq!(instance.time, 5.25);
    assert!(!probe.log.iter().any(|s| s == "slide_turn"));
}
#[test]
fn source_fakie_fold_is_not_a_clamp_and_begin_resets() {
    assert_eq!(adjust_slide(1.25, true), 0.25);
    assert_eq!(adjust_slide(-0.75, true), 0.25);
    assert_eq!(adjust_slide(0.0, true).to_bits(), 0);
    assert!(adjust_slide(f32::NAN, true).is_nan());
    let mut state = instance();
    state.begin();
    assert_eq!(
        state,
        TurningInstance {
            time: 0.0,
            value: 0.0,
            state: 2
        }
    );
    assert_eq!(
        TurningInstance::try_create(|size, alignment| {
            assert_eq!((size, alignment), (0x14, 0x10));
            Ok::<_, ()>(false)
        })
        .unwrap(),
        None
    );
}
