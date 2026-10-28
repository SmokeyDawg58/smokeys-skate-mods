use super::*;
#[derive(Default)]
struct Source {
    calls: Vec<&'static str>,
    value: Option<f32>,
    motion_values: Vec<f32>,
    fail_motion: bool,
}
impl AttachIntentSource for Source {
    type Error = &'static str;
    fn resolve_animatable(&mut self) -> Result<(), Self::Error> {
        self.calls.push("animation");
        Ok(())
    }
    fn resolve_motion(&mut self) -> Result<(), Self::Error> {
        self.calls.push("motion");
        Ok(())
    }
    fn resolve_skater_motion(&mut self) -> Result<(), Self::Error> {
        self.calls.push("skater_motion");
        Ok(())
    }
    fn require_interfaces(&mut self) -> Result<(), Self::Error> {
        self.calls.push("require");
        Ok(())
    }
    fn intent(&mut self, _: &FastString36) -> Result<Option<f32>, Self::Error> {
        self.calls.push("intent");
        Ok(self.value)
    }
    fn append_motion_attribute(&mut self, _: FastString30, value: f32) -> Result<(), Self::Error> {
        self.calls.push("append");
        if self.fail_motion {
            return Err("capacity");
        }
        self.motion_values.push(value);
        Ok(())
    }
    fn settable_attribute(&mut self, _: FastString30, value: f32) -> Result<(), Self::Error> {
        self.calls.push("settable");
        if value.is_nan() {
            return Err("animation_nan");
        }
        Ok(())
    }
}
fn run(source: &mut Source, settable: bool) -> Result<(), &'static str> {
    update(
        source,
        &FastString36::encode("Trick").unwrap(),
        FastString30::encode("Trick").unwrap(),
        settable,
    )
}
#[test]
fn absent_intent_still_resolves_all_interfaces_but_writes_nothing() {
    let mut source = Source::default();
    run(&mut source, true).unwrap();
    assert_eq!(
        source.calls,
        ["animation", "motion", "skater_motion", "require", "intent"]
    );
    assert!(source.motion_values.is_empty());
}
#[test]
fn animation_nan_failure_preserves_prior_motion_append() {
    let bits = 0x7fc0_0123;
    let mut source = Source {
        value: Some(f32::from_bits(bits)),
        ..Source::default()
    };
    assert_eq!(run(&mut source, true), Err("animation_nan"));
    assert_eq!(source.motion_values[0].to_bits(), bits);
    assert_eq!(&source.calls[5..], ["append", "settable"]);
}
#[test]
fn motion_capacity_failure_precedes_optional_animation_write() {
    let mut source = Source {
        value: Some(0.0),
        fail_motion: true,
        ..Source::default()
    };
    assert_eq!(run(&mut source, true), Err("capacity"));
    assert_eq!(source.calls.last(), Some(&"append"));
}
#[test]
fn nonsettable_preserves_negative_zero_and_does_not_call_animation() {
    let mut source = Source {
        value: Some(-0.0),
        ..Source::default()
    };
    run(&mut source, false).unwrap();
    assert_eq!(source.motion_values[0].to_bits(), 0x8000_0000);
    assert_eq!(source.calls.last(), Some(&"append"));
}
