#![expect(
    clippy::float_cmp,
    reason = "exact branch values and retained native scalar outputs"
)]
use super::*;
#[derive(Default)]
struct Source {
    events: Vec<String>,
    fail: Option<String>,
    player: bool,
    potential: f32,
}
impl Source {
    fn event(&mut self, event: String) -> Result<(), String> {
        self.events.push(event.clone());
        if self.fail.as_ref() == Some(&event) {
            Err(event)
        } else {
            Ok(())
        }
    }
}
impl CrouchingSource for Source {
    type Error = String;
    type Scalar = u64;
    type Skeleton = ();
    fn resolve(&mut self, update: bool) -> Result<(), String> {
        self.event(format!("resolve {update}"))
    }
    fn scalar_capture(&mut self, _: Collection, key: u64) -> Result<u64, String> {
        self.event(format!("capture {key:x}"))?;
        Ok(key)
    }
    fn scalar_read(&mut self, key: &u64) -> Result<f32, String> {
        self.event(format!("read {key:x}"))?;
        Ok(match *key {
            0x74ea_9539_2d41_5a5d | 0x9b4f_5525_98bb_121a => 0.0,
            0x1056_991e_714f_6fa3 => 0.25,
            _ => 1.0,
        })
    }
    fn graph(&mut self, _: Collection, offset: usize, _: f32) -> Result<f32, String> {
        self.event(format!("graph {offset:x}"))?;
        Ok(1.0)
    }
    fn physics(&mut self, field: PhysicsValue) -> Result<f32, String> {
        Ok(match field {
            PhysicsValue::VelocityY => 2.0,
            PhysicsValue::CentreOfMassToDeck => 0.25,
            PhysicsValue::PumpPotential => self.potential,
            _ => 0.0,
        })
    }
    fn skeleton_capture(&mut self) -> Result<(), String> {
        self.event("skeleton capture".into())
    }
    fn deck_compression(&mut self, (): &()) -> Result<f32, String> {
        self.event("deck compression".into())?;
        Ok(0.0)
    }
    fn intent(&mut self, name: &str) -> Result<Option<f32>, String> {
        self.event(format!("intent {name}"))?;
        Ok((self.player && name == "Crouch").then_some(1.0))
    }
    fn power_sliding(&mut self) -> Result<bool, String> {
        self.event("power sliding".into())?;
        Ok(false)
    }
    fn use_auto_pump(&mut self) -> Result<bool, String> {
        self.event("auto enabled".into())?;
        Ok(false)
    }
    fn pumping_time(&mut self) -> Result<f32, String> {
        self.event("pumping time".into())?;
        Ok(0.5)
    }
    fn dt(&mut self) -> Result<f32, String> {
        self.event("dt".into())?;
        Ok(0.1)
    }
    fn publish_motion(&mut self, name: &str) -> Result<(), String> {
        self.event(format!("publish {name}"))
    }
    fn publish_animation(&mut self, _: f32) -> Result<(), String> {
        self.event("animation".into())
    }
}
#[test]
fn failed_allocation_never_samples_live_cvar() {
    let mut s = Source::default();
    assert!(create(&mut s, false).unwrap().is_none());
    assert!(s.events.is_empty());
}
#[test]
fn begin_cvar_failure_keeps_resets_and_previous_timer() {
    let mut s = Source {
        fail: Some("pumping time".into()),
        ..Source::default()
    };
    let mut v = CrouchingInstance {
        crouch: 0.8,
        mode: 3,
        since_player: 4.0,
        ..CrouchingInstance::default()
    };
    assert!(begin(&mut s, &mut v).is_err());
    assert_eq!(v.crouch, 0.0);
    assert_eq!(v.mode, 0);
    assert_eq!(v.since_player, 4.0);
    assert_eq!(s.events, ["pumping time"]);
}
#[test]
fn begin_reads_animation_then_live_speed_curve_and_baseline() {
    let mut s = Source::default();
    let mut v = CrouchingInstance::default();
    begin(&mut s, &mut v).unwrap();
    assert_eq!(v.crouch, 0.75);
    assert_eq!(
        s.events,
        [
            "pumping time",
            "resolve false",
            "graph 190",
            "capture 74ea95392d415a5d",
            "read 74ea95392d415a5d"
        ]
    );
}
#[test]
fn velocity_sum_survives_late_scalar_failure() {
    let mut s = Source {
        fail: Some("capture 4d4bac56989f5f2b".into()),
        ..Source::default()
    };
    let mut v = CrouchingInstance {
        vertical_velocity: 3.0,
        ..CrouchingInstance::default()
    };
    assert!(update(&mut s, &mut v).is_err());
    assert_eq!(v.vertical_velocity, 5.0);
    assert!(!s.events.iter().any(|v| v.starts_with("intent")));
}
#[test]
fn delayed_delta_scalar_is_not_read_when_intervening_graph_fails() {
    let mut s = Source {
        fail: Some("graph 390".into()),
        ..Source::default()
    };
    let mut v = CrouchingInstance::default();
    assert!(update(&mut s, &mut v).is_err());
    assert!(s.events.iter().any(|v| v == "capture 29b4474528c0a68c"));
    assert!(!s.events.iter().any(|v| v == "read 29b4474528c0a68c"));
    assert_eq!(v.since_player, f32::from_bits(0x3c88_8889));
}
#[test]
fn active_player_flag_precedes_late_limit_failure() {
    let mut s = Source {
        player: true,
        fail: Some("capture 3176b692cfd4664e".into()),
        ..Source::default()
    };
    let mut v = CrouchingInstance {
        since_player: 2.0,
        ..CrouchingInstance::default()
    };
    assert!(update(&mut s, &mut v).is_err());
    assert!(v.player_controlled);
    assert_eq!(v.since_player, 2.0);
    assert_eq!(v.smoothed_input, 1.0);
}
#[test]
fn unordered_potential_reaches_native_auto_pump_publication_even_if_disabled() {
    let mut s = Source {
        potential: f32::NAN,
        ..Source::default()
    };
    let mut v = CrouchingInstance {
        mode: 1,
        crouch: 0.5,
        ..CrouchingInstance::default()
    };
    update(&mut s, &mut v).unwrap();
    assert!(s.events.iter().any(|v| v == "publish NewAutoPump"));
    assert_eq!(s.events.last().unwrap(), "animation");
}
