use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;

struct Source {
    calls: Vec<&'static str>,
    validity: [bool; 2],
    reads: usize,
    fail_acceleration: bool,
}
impl BoardFillDeckSource for Source {
    type Error = &'static str;
    type Line = ();
    fn capture_line(&mut self) -> Result<(), Self::Error> {
        self.calls.push("capture");
        Ok(())
    }
    fn valid(&mut self, (): &()) -> Result<bool, Self::Error> {
        self.calls.push("valid");
        let value = self.validity[self.reads];
        self.reads += 1;
        Ok(value)
    }
    fn start(&mut self, (): &()) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("start");
        Ok(SkeletonRootVector {
            xyz: [1.0, 2.0, 3.0],
            w: None,
        })
    }
    fn position(&mut self, (): &()) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("position");
        Ok(SkeletonRootVector::known([4.0, 5.0, 6.0, -0.0]))
    }
    fn normal(&mut self, (): &()) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("normal");
        Ok(SkeletonRootVector::known([7.0, 8.0, 9.0, 10.0]))
    }
    fn surface(&mut self, (): &()) -> Result<u32, Self::Error> {
        self.calls.push("surface");
        Ok(u32::MAX)
    }
    fn acceleration(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("acceleration");
        if self.fail_acceleration {
            Err("acceleration")
        } else {
            Ok(SkeletonRootVector {
                xyz: [11.0, 12.0, 13.0],
                w: None,
            })
        }
    }
}
fn source(validity: [bool; 2]) -> Source {
    Source {
        calls: vec![],
        validity,
        reads: 0,
        fail_acceleration: false,
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
fn fresh_validity_controls_payload_independently_of_published_validity() {
    let mut output = output();
    let mut source = source([false, true]);
    output.try_fill_board_deck(&mut source).unwrap();
    let ground = output.concrete.ground.as_ref().unwrap();
    assert!(!ground.below_deck_ground_data_is_valid);
    assert_eq!(
        ground.below_deck_ground_data_position.w.unwrap().to_bits(),
        0x8000_0000
    );
    assert_eq!(ground.below_deck_ground_data_surface_id, u32::MAX);
    assert_eq!(ground.below_deck_ground_data_start_pos.w, None);
    assert_eq!(
        output.concrete.motion.as_ref().unwrap().deck_acceleration.w,
        None
    );
    assert_eq!(
        source.calls,
        [
            "capture",
            "valid",
            "start",
            "valid",
            "position",
            "normal",
            "surface",
            "acceleration"
        ]
    );
}
#[test]
fn second_false_validity_skips_payload_and_only_nan_x_replaces_position() {
    for x in [f32::INFINITY, -0.0, f32::NAN] {
        let mut output = output();
        output
            .concrete
            .skateboard_reckoning
            .as_mut()
            .unwrap()
            .deck_position = SkeletonRootVector {
            xyz: [x, f32::NAN, 2.0],
            w: None,
        };
        let mut source = source([true, false]);
        output.try_fill_board_deck(&mut source).unwrap();
        assert_eq!(
            source.calls,
            ["capture", "valid", "start", "valid", "acceleration"]
        );
        assert!(
            output
                .concrete
                .ground
                .as_ref()
                .unwrap()
                .below_deck_ground_data_is_valid
        );
        let position = output
            .concrete
            .skateboard_reckoning
            .as_ref()
            .unwrap()
            .deck_position;
        if x.is_nan() {
            assert_eq!(position, SkeletonRootVector::known([0.0; 4]));
        } else {
            assert_eq!(position.xyz[0].to_bits(), x.to_bits());
            assert!(position.xyz[1].is_nan());
            assert_eq!(position.w, None);
        }
    }
}
#[test]
fn late_acceleration_failure_keeps_deck_record_and_native_nan_replacement() {
    let mut output = output();
    output
        .concrete
        .skateboard_reckoning
        .as_mut()
        .unwrap()
        .deck_position = SkeletonRootVector::known([f32::NAN; 4]);
    output.concrete.motion.as_mut().unwrap().deck_acceleration =
        SkeletonRootVector::known([99.0; 4]);
    let mut source = Source {
        fail_acceleration: true,
        ..source([true, true])
    };
    assert!(matches!(
        output.try_fill_board_deck(&mut source),
        Err(BoardFillDeckError::Source("acceleration"))
    ));
    assert_eq!(
        output
            .concrete
            .ground
            .as_ref()
            .unwrap()
            .below_deck_ground_data_surface_id,
        u32::MAX
    );
    assert_eq!(
        output
            .concrete
            .skateboard_reckoning
            .as_ref()
            .unwrap()
            .deck_position,
        SkeletonRootVector::known([0.0; 4])
    );
    assert_eq!(
        output.concrete.motion.as_ref().unwrap().deck_acceleration,
        SkeletonRootVector::known([99.0; 4])
    );
}
#[test]
fn locked_output_captures_line_but_does_not_read_validity() {
    let mut output = output();
    output.lock_after_update1().unwrap();
    let mut source = source([true, true]);
    assert!(matches!(
        output.try_fill_board_deck(&mut source),
        Err(BoardFillDeckError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    ));
    assert_eq!(source.calls, ["capture"]);
}
