use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;
#[derive(Default)]
struct Source {
    calls: Vec<&'static str>,
    fail_y: bool,
}
impl Update5TrickPayloadSource for Source {
    type Error = &'static str;
    fn capture_animation(&mut self) -> Result<(), Self::Error> {
        self.calls.push("capture");
        Ok(())
    }
    fn trick(&mut self) -> Result<AnimOutTrickData, Self::Error> {
        self.calls.push("trick");
        let mut data = AnimOutTrickData::reset_values();
        data.bits = 0x8123_4567;
        data.tweak_x = -0.0;
        data.tweak_y = f32::from_bits(0x7fc0_0123);
        Ok(data)
    }
    fn gesture(&mut self) -> Result<u32, Self::Error> {
        self.calls.push("gesture");
        Ok(0xffff_fffe)
    }
    fn gesturing(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("gesturing");
        Ok(true)
    }
    fn wipeout_x(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("x");
        Ok(-0.0)
    }
    fn wipeout_y(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("y");
        if self.fail_y { Err("y") } else { Ok(7.0) }
    }
    fn flags_8d0(&mut self) -> Result<u32, Self::Error> {
        self.calls.push("flags");
        Ok(0x4000)
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    output
}
#[test]
fn reads_actual_output_substate_including_two_and_excluding_four() {
    for (state, expected) in [(0, false), (1, true), (2, true), (4, false), (-1, false)] {
        let mut out = output();
        out.concrete.grinds.as_mut().unwrap().grind_substate = state;
        out.try_publish_update5_trick_payload(&mut Source::default())
            .unwrap();
        assert_eq!(out.concrete.grinds.as_ref().unwrap().is_grinding, expected);
        assert_eq!(out.concrete.grinds.as_ref().unwrap().grind_substate, state);
    }
}
#[test]
fn payload_copy_preserves_bits_and_native_field_order() {
    let mut out = output();
    let mut source = Source::default();
    out.try_publish_update5_trick_payload(&mut source).unwrap();
    assert_eq!(
        source.calls,
        [
            "trick",
            "capture",
            "gesture",
            "gesturing",
            "capture",
            "x",
            "y",
            "flags"
        ]
    );
    let trick = out.concrete.trick.as_ref().unwrap();
    assert_eq!(trick.current_animation.bits, 0x8123_4567);
    assert_eq!(trick.current_animation.tweak_x.to_bits(), 0x8000_0000);
    assert_eq!(trick.current_animation.tweak_y.to_bits(), 0x7fc0_0123);
    assert_eq!(trick.gesture, -2);
    assert!(trick.is_gesturing && trick.is_push_tricking);
    assert_eq!(trick.wipeout_gesture_x.to_bits(), 0x8000_0000);
    assert_eq!(trick.wipeout_gesture_y.to_bits(), 7.0_f32.to_bits());
}
#[test]
fn second_wipeout_read_failure_keeps_first_axis_and_earlier_copies() {
    let mut out = output();
    out.concrete.grinds.as_mut().unwrap().grind_substate = 2;
    out.concrete.trick.as_mut().unwrap().wipeout_gesture_y = 19.0;
    let mut source = Source {
        fail_y: true,
        ..Source::default()
    };
    assert!(matches!(
        out.try_publish_update5_trick_payload(&mut source),
        Err(Update5TrickPayloadError::Source("y"))
    ));
    assert!(out.concrete.grinds.as_ref().unwrap().is_grinding);
    let trick = out.concrete.trick.as_ref().unwrap();
    assert_eq!(trick.gesture, -2);
    assert_eq!(trick.wipeout_gesture_x.to_bits(), 0x8000_0000);
    assert_eq!(trick.wipeout_gesture_y.to_bits(), 19.0_f32.to_bits());
    assert!(!trick.is_push_tricking);
    assert!(!source.calls.contains(&"flags"));
}
#[test]
fn locked_output_stops_before_animation_payload_reads() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut source = Source::default();
    assert!(out.try_publish_update5_trick_payload(&mut source).is_err());
    assert!(source.calls.is_empty());
}
