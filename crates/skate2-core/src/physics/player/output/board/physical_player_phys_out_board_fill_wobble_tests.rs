use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;
#[derive(Default)]
struct Source {
    captures: u8,
    calls: Vec<&'static str>,
    fail_tilt: bool,
}
impl BoardFillWobbleSource for Source {
    type Error = &'static str;
    type Wobble = u8;
    fn capture_wobble(&mut self) -> Result<u8, Self::Error> {
        self.calls.push("capture");
        self.captures += 1;
        Ok(self.captures)
    }
    fn effect(&mut self, captured: &u8) -> Result<f32, Self::Error> {
        self.calls.push("effect");
        assert_eq!(*captured, 1);
        Ok(-0.0)
    }
    fn tilt(&mut self, captured: &u8) -> Result<f32, Self::Error> {
        self.calls.push("tilt");
        assert_eq!(*captured, 2);
        if self.fail_tilt {
            Err("tilt unavailable")
        } else {
            Ok(f32::from_bits(0x7fc0_1234))
        }
    }
    fn deck_tilt(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("deck");
        Ok(-3.0)
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out
}
#[test]
fn locked_output_still_captures_and_reads_effect_before_assertion() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut source = Source::default();
    assert!(matches!(
        out.try_fill_board_wobble(&mut source),
        Err(BoardFillWobbleOutputError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    ));
    assert_eq!(source.calls, ["capture", "effect"]);
    assert_eq!(
        out.concrete
            .motion
            .as_ref()
            .unwrap()
            .speed_wobble_effect
            .to_bits(),
        0
    );
}
#[test]
fn late_tilt_failure_preserves_effect_and_does_not_read_deck() {
    let mut out = output();
    let mut source = Source {
        fail_tilt: true,
        ..Source::default()
    };
    assert!(matches!(
        out.try_fill_board_wobble(&mut source),
        Err(BoardFillWobbleOutputError::Source("tilt unavailable"))
    ));
    assert_eq!(source.calls, ["capture", "effect", "capture", "tilt"]);
    let motion = out.concrete.motion.as_ref().unwrap();
    assert_eq!(motion.speed_wobble_effect.to_bits(), 0x8000_0000);
    assert_eq!(motion.speed_wobble_tilt.to_bits(), 0);
    assert_eq!(motion.deck_tilt.to_bits(), 0);
}
#[test]
fn fresh_pointer_capture_and_scalar_bits_survive_without_sanitization() {
    let mut out = output();
    let mut source = Source::default();
    out.try_fill_board_wobble(&mut source).unwrap();
    assert_eq!(
        source.calls,
        ["capture", "effect", "capture", "tilt", "deck"]
    );
    let motion = out.concrete.motion.as_ref().unwrap();
    assert_eq!(motion.speed_wobble_effect.to_bits(), 0x8000_0000);
    assert_eq!(motion.speed_wobble_tilt.to_bits(), 0x7fc0_1234);
    assert_eq!(motion.deck_tilt.to_bits(), (-3.0_f32).to_bits());
}
