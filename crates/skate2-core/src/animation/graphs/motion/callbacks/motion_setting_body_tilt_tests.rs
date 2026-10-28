use super::{FastString30, TiltInstance, TiltSource, limited, update};
struct Source {
    applying: bool,
    fail: Option<&'static str>,
    calls: Vec<&'static str>,
    published: Option<f32>,
}
impl Source {
    fn call(&mut self, name: &'static str) -> Result<(), &'static str> {
        self.calls.push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl TiltSource for Source {
    type Error = &'static str;
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error> {
        self.call("interfaces")
    }
    fn applying(&mut self) -> Result<bool, Self::Error> {
        self.call("applying")?;
        Ok(self.applying)
    }
    fn missing_instance(&mut self) -> Self::Error {
        "instance"
    }
    fn capture_animation_output(&mut self) -> Result<(), Self::Error> {
        self.call("output")
    }
    fn mirrored(&mut self) -> Result<bool, Self::Error> {
        self.call("mirror")?;
        Ok(true)
    }
    fn animation_tilt_x(&mut self) -> Result<f32, Self::Error> {
        self.call("tilt")?;
        Ok(10.0)
    }
    fn body_spin_speed(&mut self) -> Result<f32, Self::Error> {
        self.call("spin")?;
        Ok(-2.0)
    }
    fn evaluate_spin_graph(&mut self, speed: f32) -> Result<f32, Self::Error> {
        assert_eq!(speed.to_bits(), 2.0f32.to_bits());
        self.call("graph")?;
        Ok(1.0)
    }
    fn filtered_is_air(&mut self) -> Result<bool, Self::Error> {
        self.call("filtered")?;
        Ok(false)
    }
    fn clamp_acceleration(&mut self, air: bool) -> Result<f32, Self::Error> {
        assert!(!air);
        self.call("acc")?;
        Ok(1.0)
    }
    fn clamp_velocity(&mut self, air: bool) -> Result<f32, Self::Error> {
        assert!(!air);
        self.call("vel")?;
        Ok(3.0)
    }
    fn use_tilt(&mut self) -> Result<bool, Self::Error> {
        self.call("use")?;
        Ok(false)
    }
    fn set_tilt(&mut self) -> Result<bool, Self::Error> {
        panic!("disabled use must skip override")
    }
    fn override_x(&mut self) -> Result<f32, Self::Error> {
        panic!("disabled use must skip override")
    }
    fn publish(&mut self, _: FastString30, value: f32) -> Result<(), Self::Error> {
        self.call("publish")?;
        self.published = Some(value);
        Ok(())
    }
}
fn source(applying: bool, fail: Option<&'static str>) -> Source {
    Source {
        applying,
        fail,
        calls: vec![],
        published: None,
    }
}
const NAME: FastString30 = FastString30 { words: [0; 5] };
#[test]
fn inactive_retains_history_and_null_still_fails_after_gate() {
    let mut s = source(false, None);
    let mut i = TiltInstance {
        angle: 8.0,
        delta: 9.0,
        was_applying: true,
    };
    update(&mut s, Some(&mut i), NAME).unwrap();
    assert_eq!(i.angle.to_bits(), 8.0f32.to_bits());
    assert!(!i.was_applying);
    assert_eq!(s.calls, ["interfaces", "applying"]);
    assert_eq!(update(&mut s, None, NAME), Err("instance"));
    assert!(TiltInstance::after_allocation(false).is_none());
}
#[test]
fn rising_edge_reset_precedes_failed_output() {
    let mut s = source(true, Some("output"));
    let mut i = TiltInstance {
        angle: 8.0,
        delta: 9.0,
        was_applying: false,
    };
    assert_eq!(update(&mut s, Some(&mut i), NAME), Err("output"));
    assert!(i.was_applying);
    assert_eq!(i.angle.to_bits(), 0);
    assert_eq!(i.delta.to_bits(), 0);
}
#[test]
fn cvar_failure_preserves_integrated_state_and_disabled_draw_still_integrates() {
    let mut s = source(true, Some("use"));
    let mut i = TiltInstance::after_allocation(true).unwrap();
    assert_eq!(update(&mut s, Some(&mut i), NAME), Err("use"));
    assert_eq!(i.angle.to_bits(), 1.0f32.to_bits());
    assert_eq!(i.delta.to_bits(), 1.0f32.to_bits());
    s.fail = None;
    update(&mut s, Some(&mut i), NAME).unwrap();
    assert_eq!(i.angle.to_bits(), 3.0f32.to_bits());
    assert_eq!(i.delta.to_bits(), 2.0f32.to_bits());
    assert_eq!(s.published.unwrap().to_bits(), 0);
}
#[test]
fn native_fsel_nan_and_negative_bounds_are_not_rust_clamp() {
    assert_eq!(limited(f32::NAN, 3.0).to_bits(), 3.0f32.to_bits());
    assert_eq!(limited(0.0, -2.0).to_bits(), (-2.0f32).to_bits());
    assert_eq!(super::select(-0.0, -0.0, 1.0).to_bits(), 0x8000_0000);
}
