use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;
#[derive(Default)]
struct Source {
    calls: Vec<&'static str>,
    wants: bool,
    clear_fails: bool,
}
impl BoardFillFinalSource for Source {
    type Error = &'static str;
    type Body = ();
    fn capture_body(&mut self) -> Result<(), Self::Error> {
        self.calls.push("body");
        Ok(())
    }
    fn deck_slide_speed(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.calls.push("slide");
        Ok(3.0)
    }
    fn deck_to_wheel_base(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.calls.push("base");
        Ok(0.25)
    }
    fn wheel_hardness(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("hardness");
        Ok(-0.0)
    }
    fn wants_respawn(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("wants");
        Ok(self.wants)
    }
    fn clear_wants_respawn(&mut self) -> Result<(), Self::Error> {
        self.calls.push("clear");
        if self.clear_fails {
            Err("clear")
        } else {
            self.wants = false;
            Ok(())
        }
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out
}
#[test]
fn locked_output_calls_slide_before_assertion() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut source = Source::default();
    assert!(matches!(
        out.try_fill_board_final(&mut source),
        Err(BoardFinalOutputError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    ));
    assert_eq!(source.calls, ["body", "slide"]);
}
#[test]
fn request_publication_precedes_clear_and_survives_clear_failure() {
    let mut out = output();
    let mut source = Source {
        wants: true,
        clear_fails: true,
        ..Source::default()
    };
    assert!(matches!(
        out.try_fill_board_final(&mut source),
        Err(BoardFinalOutputError::Source("clear"))
    ));
    assert!(
        out.concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requested_teleport
    );
    assert!(source.wants);
    assert_eq!(
        out.concrete
            .motion
            .as_ref()
            .unwrap()
            .wheel_hardness
            .to_bits(),
        0x8000_0000
    );
    assert_eq!(
        source.calls,
        [
            "body", "slide", "body", "base", "hardness", "wants", "clear"
        ]
    );
}
#[test]
fn false_request_neither_clears_input_nor_overwrites_previous_output_request() {
    let mut out = output();
    out.concrete
        .state
        .as_mut()
        .unwrap()
        .physics_requested_teleport = true;
    let mut source = Source::default();
    out.try_fill_board_final(&mut source).unwrap();
    assert!(
        out.concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requested_teleport
    );
    assert_eq!(source.calls.last(), Some(&"wants"));
}
