use super::*;
struct Attribute(f32);
impl SkateboardSteeringAttributes for Attribute {
    type Error = ();
    fn blend_coefficient(&mut self) -> Result<f32, ()> {
        Ok(self.0)
    }
}
struct Input {
    flipped: bool,
    fail: bool,
    events: Vec<&'static str>,
}
impl SkateboardSteeringInput for Input {
    type Processed = ();
    type Error = &'static str;
    fn capture_processed(&mut self) -> Result<(), Self::Error> {
        self.events.push("capture");
        Ok(())
    }
    fn flags_8c4(&mut self, (): &()) -> Result<u32, Self::Error> {
        self.events.push("flip");
        if self.fail {
            Err("flags")
        } else {
            Ok(if self.flipped { 0x0010_0000 } else { 0 })
        }
    }
    fn flags_8c8(&mut self, (): &()) -> Result<u32, Self::Error> {
        self.events.push("contacts");
        Ok(0x1000_0000)
    }
}
#[test]
fn current_flags_swap_contacts_and_zero_target_still_invokes_body() {
    for flipped in [false, true] {
        let mut state = SkateboardSteeringState::new();
        let mut input = Input {
            flipped,
            fail: false,
            events: vec![],
        };
        let mut call = None;
        state
            .try_update(
                0.0,
                &mut Attribute(0.25),
                &mut input,
                |angle, front, back| call = Some((angle.to_bits(), front, back)),
            )
            .unwrap();
        assert_eq!(call, Some((0, !flipped, flipped)));
        assert_eq!(input.events, ["capture", "flip", "contacts"]);
    }
}
#[test]
fn late_flag_failure_retains_fused_tilt_store_before_body_call() {
    let mut state = SkateboardSteeringState {
        deck_tilt: 2.0,
        ..SkateboardSteeringState::new()
    };
    let mut input = Input {
        flipped: false,
        fail: true,
        events: vec![],
    };
    assert!(
        state
            .try_update(4.0, &mut Attribute(0.25), &mut input, |_, _, _| panic!(
                "not reached"
            ))
            .is_err()
    );
    assert_eq!(state.deck_tilt().to_bits(), 2.5_f32.to_bits());
    assert_eq!(input.events, ["capture", "flip"]);
}
#[test]
fn runtime_class_and_missing_default_are_source_exact() {
    assert_eq!(
        skate2_formats::attrib::string_hash64("physics_steering"),
        STEERING_CLASS
    );
    let vault = Arc::new(crate::attribute_runtime::tests::graph(
        STEERING_CLASS,
        vec![],
    ));
    let mut attributes = SkateboardSteeringVaultAttributes::bind(&vault);
    assert_eq!(attributes.blend_coefficient().unwrap().to_bits(), 0);
}
