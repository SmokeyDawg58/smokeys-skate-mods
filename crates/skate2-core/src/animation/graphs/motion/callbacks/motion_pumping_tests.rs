#![expect(
    clippy::float_cmp,
    reason = "exact channel influence and retained scalar outputs"
)]
use super::*;
struct Source {
    allowed: bool,
    enabled: bool,
    curve: f32,
    present: [bool; 5],
    events: Vec<String>,
    fail: Option<String>,
}
impl Default for Source {
    fn default() -> Self {
        Self {
            allowed: true,
            enabled: true,
            curve: 0.75,
            present: [false; 5],
            events: Vec::new(),
            fail: None,
        }
    }
}
impl Source {
    fn event(&mut self, value: String) -> Result<(), String> {
        self.events.push(value.clone());
        if self.fail.as_ref() == Some(&value) {
            Err(value)
        } else {
            Ok(())
        }
    }
}
impl PumpingSource for Source {
    type Error = String;
    fn resolve(&mut self, update: bool) -> Result<(), String> {
        self.event(format!("resolve {update}"))
    }
    fn allowed(&mut self) -> Result<bool, String> {
        self.event("allowed".into())?;
        Ok(self.allowed)
    }
    fn missing_instance(&mut self) -> String {
        "instance".into()
    }
    fn ground_acceleration(&mut self) -> Result<f32, String> {
        self.event("ground".into())?;
        Ok(0.8)
    }
    fn scalar(&mut self, key: u64) -> Result<f32, String> {
        self.event(format!("scalar {key:x}"))?;
        Ok(match key {
            0x57bc_b525_1cc1_ed5c => 1.0,
            0xc7e4_895c_512a_8550 => 0.5,
            0xc47d_82b1_fb49_e9d7 => 0.25,
            0x7e08_4051_45cc_275d => 0.2,
            0x0ff8_3496_1814_e5f9 => 0.4,
            _ => panic!("unexpected key"),
        })
    }
    fn graph(&mut self, _: f32) -> Result<f32, String> {
        self.event("graph".into())?;
        Ok(self.curve)
    }
    fn has_channel(&mut self, index: usize) -> Result<bool, String> {
        self.event(format!("has {index}"))?;
        Ok(self.present[index])
    }
    fn enabled(&mut self) -> Result<bool, String> {
        self.event("enabled".into())?;
        Ok(self.enabled)
    }
    fn new_channel(&mut self, request: PumpingChannel) -> Result<(), String> {
        assert_eq!(request.blend_in, 0.2);
        assert_eq!(request.blend_out, 0.4);
        self.event(format!("new {}", request.index))
    }
    fn influence(&mut self, index: usize, _: f32) -> Result<(), String> {
        self.event(format!("influence {index}"))
    }
    fn end_channel(&mut self, index: usize) -> Result<(), String> {
        self.event(format!("end {index}"))
    }
}
#[test]
fn disallowed_still_resolves_all_update_interfaces_but_never_dereferences_instance() {
    let mut s = Source {
        allowed: false,
        ..Source::default()
    };
    update(&mut s, None).unwrap();
    assert_eq!(s.events, ["resolve true", "allowed"]);
}
#[test]
fn disabled_creation_still_records_first_free_channel_and_publishes_influence() {
    let mut s = Source {
        enabled: false,
        present: [true, true, false, false, false],
        ..Source::default()
    };
    let mut v = PumpingInstance::after_allocation(true).unwrap();
    update(&mut s, Some(&mut v)).unwrap();
    assert_eq!(v.channel, 2);
    assert_eq!(v.peak, 0.75);
    assert_eq!(v.smoothed, 0.4);
    assert_eq!(
        &s.events[s.events.len() - 6..],
        [
            "has 1",
            "has 2",
            "scalar 7e08405145cc275d",
            "scalar ff834961814e5f9",
            "enabled",
            "influence 2"
        ]
    );
}
#[test]
fn failed_new_channel_preserves_filter_but_not_channel_index_or_peak() {
    let mut s = Source {
        fail: Some("new 0".into()),
        ..Source::default()
    };
    let mut v = PumpingInstance::after_allocation(true).unwrap();
    assert!(update(&mut s, Some(&mut v)).is_err());
    assert_eq!(v.smoothed, 0.4);
    assert_eq!(v.influence, 0.75);
    assert_eq!(v.channel, -1);
    assert_eq!(v.peak, 0.0);
}
#[test]
fn old_nan_can_cross_threshold_and_current_nan_can_publish_existing_channel() {
    let mut s = Source::default();
    let mut v = PumpingInstance {
        influence: f32::NAN,
        smoothed: 0.0,
        channel: -1,
        peak: 0.0,
    };
    update(&mut s, Some(&mut v)).unwrap();
    assert!(s.events.iter().any(|v| v == "new 0"));
    let mut s = Source {
        curve: f32::NAN,
        ..Source::default()
    };
    update(&mut s, Some(&mut v)).unwrap();
    assert!(v.peak.is_nan());
    assert_eq!(s.events.last().unwrap(), "influence 0");
    assert!(!s.events.iter().any(|v| v == "enabled"));
}
#[test]
fn influence_failure_keeps_peak_store_and_end_stops_at_failed_channel() {
    let mut s = Source {
        fail: Some("influence 0".into()),
        ..Source::default()
    };
    let mut v = PumpingInstance {
        influence: 0.5,
        smoothed: 0.0,
        channel: 0,
        peak: 0.5,
    };
    assert!(update(&mut s, Some(&mut v)).is_err());
    assert_eq!(v.peak, 0.75);
    let mut s = Source {
        fail: Some("end 2".into()),
        ..Source::default()
    };
    assert!(end(&mut s).is_err());
    assert_eq!(s.events, ["resolve false", "end 0", "end 1", "end 2"]);
}
#[test]
fn full_channel_set_skips_lengths_and_cvar_without_resetting_retained_index() {
    let mut s = Source {
        present: [true; 5],
        ..Source::default()
    };
    let mut v = PumpingInstance {
        influence: 0.0,
        smoothed: 0.0,
        channel: 4,
        peak: 0.5,
    };
    update(&mut s, Some(&mut v)).unwrap();
    assert_eq!(v.channel, 4);
    assert_eq!(s.events.last().unwrap(), "influence 4");
    assert!(!s.events.iter().any(|v| v == "enabled"));
}
