use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;
struct Source {
    captures: usize,
    calls: Vec<String>,
    fail: Option<usize>,
    current_bodies: [u16; 4],
}
impl Source {
    fn new() -> Self {
        Self {
            captures: 0,
            calls: vec![],
            fail: None,
            current_bodies: [10, 11, 12, 13],
        }
    }
}
impl BoardFillWheelPositionSource for Source {
    type Error = &'static str;
    type Parts = usize;
    type Body = (usize, u16);
    fn capture_parts(&mut self) -> Result<usize, Self::Error> {
        let captured = self.captures;
        self.captures += 1;
        self.calls.push(format!("parts{captured}"));
        // A change in the current Part body within the same descriptor must
        // be observed at the later selected read, not from an entry snapshot.
        if captured == 1 {
            self.current_bodies[1] = 77;
        }
        Ok(captured)
    }
    fn body(&mut self, parts: &usize, wheel: usize) -> Result<Self::Body, Self::Error> {
        assert_eq!(*parts, wheel);
        self.calls.push(format!("body{wheel}"));
        Ok((wheel, self.current_bodies[wheel]))
    }
    fn position(&mut self, body: &Self::Body) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push(format!("position{}", body.0));
        if self.fail == Some(body.0) {
            return Err("position unavailable");
        }
        Ok(SkeletonRootVector {
            xyz: [f32::from(body.1), -0.0, 3.0],
            w: None,
        })
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out
}
#[test]
fn fresh_current_body_within_same_descriptor_and_unknown_w_survive() {
    let mut out = output();
    let mut source = Source::new();
    out.try_fill_board_wheel_positions(&mut source).unwrap();
    let positions = out
        .concrete
        .skateboard_reckoning
        .as_ref()
        .unwrap()
        .wheel_pos;
    assert_eq!(
        positions.map(|p| p.xyz[0].to_bits()),
        [10.0_f32, 77.0, 12.0, 13.0].map(f32::to_bits)
    );
    assert!(
        positions
            .iter()
            .all(|p| p.w.is_none() && p.xyz[1].to_bits() == 0x8000_0000)
    );
    assert_eq!(source.captures, 4);
}
#[test]
fn locked_output_still_reads_first_body_and_position_before_assertion() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut source = Source::new();
    assert!(matches!(
        out.try_fill_board_wheel_positions(&mut source),
        Err(BoardWheelPositionOutputError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    ));
    assert_eq!(source.calls, ["parts0", "body0", "position0"]);
}
#[test]
fn late_failure_keeps_earlier_wheels_and_does_not_read_later_parts() {
    let mut out = output();
    let mut source = Source::new();
    source.fail = Some(2);
    assert!(matches!(
        out.try_fill_board_wheel_positions(&mut source),
        Err(BoardWheelPositionOutputError::Source(
            "position unavailable"
        ))
    ));
    let positions = out
        .concrete
        .skateboard_reckoning
        .as_ref()
        .unwrap()
        .wheel_pos;
    assert_eq!(positions[1].xyz[0].to_bits(), 77.0_f32.to_bits());
    assert_eq!(positions[2].xyz.map(f32::to_bits), [0; 3]);
    assert_eq!(source.captures, 3);
}
