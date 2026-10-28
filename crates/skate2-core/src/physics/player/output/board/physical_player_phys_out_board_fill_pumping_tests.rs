use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;

struct Source {
    calls: Vec<&'static str>,
    jump: bool,
}
impl BoardFillPumpingSource for Source {
    type Error = &'static str;
    type Pumping = ();
    fn capture_pumping(&mut self) -> Result<(), Self::Error> {
        self.calls.push("capture pump");
        Ok(())
    }
    fn pump_amount(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.calls.push("pump");
        Ok(0.75)
    }
    fn intentional_pump(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("intentional");
        Ok(true)
    }
    fn abort_push(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("abort");
        Ok(true)
    }
    fn captured_body_position(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("position");
        Ok(SkeletonRootVector {
            xyz: [1.0, 2.0, 3.0],
            w: None,
        })
    }
    fn board_flipped(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("flip");
        Ok(true)
    }
    fn has_jump_delta(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("jump bit");
        Ok(self.jump)
    }
    fn jump_delta(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("jump delta");
        Err("pending jump producer")
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out
}

#[test]
fn locked_output_reads_pumping_after_latch_sample_before_assertion() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut source = Source {
        calls: vec![],
        jump: false,
    };
    assert!(matches!(
        out.try_fill_board_pumping(&mut source),
        Err(BoardFillPumpingOutputError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    ));
    assert_eq!(source.calls, ["capture pump", "pump"]);
    assert_eq!(
        out.concrete
            .motion
            .as_ref()
            .unwrap()
            .pump_potential
            .to_bits(),
        0
    );
}

#[test]
fn selected_missing_jump_producer_retains_all_preceding_stores() {
    let mut out = output();
    out.concrete.air.as_mut().unwrap().jump_dv = SkeletonRootVector::known([99.0; 4]);
    let mut source = Source {
        calls: vec![],
        jump: true,
    };
    assert!(matches!(
        out.try_fill_board_pumping(&mut source),
        Err(BoardFillPumpingOutputError::Source("pending jump producer"))
    ));
    assert_eq!(
        source.calls,
        [
            "capture pump",
            "pump",
            "intentional",
            "abort",
            "position",
            "flip",
            "jump bit",
            "jump delta"
        ]
    );
    assert_eq!(
        out.concrete
            .motion
            .as_ref()
            .unwrap()
            .pump_potential
            .to_bits(),
        0.75_f32.to_bits()
    );
    assert!(out.concrete.motion.as_ref().unwrap().is_intentional_pump);
    assert!(out.concrete.skeleton.as_ref().unwrap().abort_push);
    assert_eq!(
        out.concrete
            .skateboard_reckoning
            .as_ref()
            .unwrap()
            .deck_position
            .w,
        None
    );
    assert!(
        out.concrete
            .skateboard_reckoning
            .as_ref()
            .unwrap()
            .is_transform_flipped
    );
    assert_eq!(
        out.concrete.air.as_ref().unwrap().jump_dv,
        SkeletonRootVector::known([99.0; 4])
    );
}

#[test]
fn unselected_jump_copies_native_full_zero_without_querying_missing_producer() {
    let mut out = output();
    out.concrete.air.as_mut().unwrap().jump_dv = SkeletonRootVector::known([99.0; 4]);
    let mut source = Source {
        calls: vec![],
        jump: false,
    };
    out.try_fill_board_pumping(&mut source).unwrap();
    assert_eq!(source.calls.last(), Some(&"jump bit"));
    assert_eq!(
        out.concrete.air.as_ref().unwrap().jump_dv,
        SkeletonRootVector::known([0.0; 4])
    );
}
