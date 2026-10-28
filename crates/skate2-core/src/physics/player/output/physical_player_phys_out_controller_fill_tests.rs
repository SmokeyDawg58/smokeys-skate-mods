use super::*;
use crate::{
    physical_player_phys_out::construct::PhysOutAllocation,
    reckoning_numeric::{HostReckoningNumeric, host_dot3},
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};
struct Source {
    state: u32,
    flags: u32,
    fail_spine: bool,
    calls: Vec<String>,
}
impl Source {
    fn new(state: u32, flags: u32) -> Self {
        Self {
            state,
            flags,
            fail_spine: false,
            calls: Vec::new(),
        }
    }
}
impl ControllerFillSource for Source {
    type Error = &'static str;
    fn state(&mut self) -> Result<u32, Self::Error> {
        self.calls.push("state".into());
        Ok(self.state)
    }
    fn spine_world(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.calls.push("spine".into());
        if self.fail_spine {
            return Err("spine");
        }
        let mut matrix = FRESH_VOLUME_TRANSFORM;
        matrix[13] = 2.0;
        Ok(SkeletonRootMatrix::known(matrix))
    }
    fn effective_animation(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.calls.push("effective".into());
        Ok(SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM))
    }
    fn retrieval_start_position(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("retrieval".into());
        Ok(SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]))
    }
    fn deck_position(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("deck".into());
        Ok(SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]))
    }
    fn flags_8cc(&mut self, mask: u32) -> Result<u32, Self::Error> {
        self.calls.push(format!("flags:{mask:x}"));
        Ok(self.flags)
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut value = PhysicalPlayerPhysOut::new();
    value
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    value
}
#[test]
fn holding_store_precedes_mandatory_spine_read_even_while_on_board() {
    let mut out = output();
    let mut source = Source::new(1, 0);
    source.fail_spine = true;
    assert!(matches!(
        out.try_fill_controller(&mut source, &mut HostReckoningNumeric),
        Err(ControllerFillError::Source("spine"))
    ));
    assert!(
        out.concrete
            .offboard
            .as_ref()
            .unwrap()
            .is_holding_skateboard
    );
    assert_eq!(source.calls, ["state", "spine"]);
    out.lock_after_update1().unwrap();
    source.calls.clear();
    assert!(matches!(
        out.try_fill_controller(&mut source, &mut HostReckoningNumeric),
        Err(ControllerFillError::Access(_))
    ));
    assert_eq!(source.calls, ["state"]);
}
#[test]
fn hiding_selects_retained_retrieval_and_zero_denominator_keeps_signed_half_pi() {
    let mut out = output();
    let mut source = Source::new(3, 0);
    out.try_fill_controller(&mut source, &mut HostReckoningNumeric)
        .unwrap();
    let value = out.concrete.offboard.as_ref().unwrap();
    assert_eq!(value.yaw_to_free_board.to_bits(), 0x3fc9_0fdb);
    assert_eq!(value.pitch_to_free_board.to_bits(), 0);
    assert!(value.is_hiding_board);
    assert!(source.calls.iter().any(|s| s == "retrieval"));
    assert!(!source.calls.iter().any(|s| s == "deck"));
    assert_eq!(select_yaw(-0.0, -0.0, f32::NAN).to_bits(), 0xbfc9_0fdb);
    assert!(select_yaw(1.0, f32::NAN, f32::NAN).is_nan());
}
#[test]
fn returning_player_request_uses_short_circuit_source_order() {
    let mut out = output();
    let mut source = Source::new(4, 0x1000);
    out.try_fill_controller(&mut source, &mut HostReckoningNumeric)
        .unwrap();
    let value = out.concrete.offboard.as_ref().unwrap();
    assert!(value.is_retrieving_board && value.is_player_retrieving_board);
    assert_eq!(
        source.calls,
        [
            "state",
            "spine",
            "state",
            "effective",
            "deck",
            "state",
            "flags:400",
            "flags:800",
            "state",
            "flags:1800",
            "state"
        ]
    );
    source.calls.clear();
    source.flags = 0x800 | 0x400;
    out.try_fill_controller(&mut source, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(
        source.calls,
        [
            "state",
            "spine",
            "state",
            "effective",
            "deck",
            "state",
            "flags:400",
            "flags:800",
            "flags:1800"
        ]
    );
    assert!(out.concrete.offboard.as_ref().unwrap().is_dropping_board);
}
struct FailProjection;
impl ReckoningNumeric for FailProjection {
    type Error = &'static str;
    fn dot3(&mut self, _a: [f32; 4], _b: [f32; 4]) -> Result<f32, Self::Error> {
        Err("projection")
    }
    fn reciprocal_sqrt_estimate(&mut self, _v: f32) -> Result<f32, Self::Error> {
        panic!("not reached")
    }
    fn reciprocal_estimate(&mut self, _v: f32) -> Result<f32, Self::Error> {
        panic!("not reached")
    }
    fn truncate_i32(&mut self, _v: f32) -> Result<i32, Self::Error> {
        panic!("not reached")
    }
}
#[test]
fn projection_failure_does_not_read_selected_board_position() {
    let mut out = output();
    let mut source = Source::new(3, 0);
    assert!(matches!(
        out.try_fill_controller(&mut source, &mut FailProjection),
        Err(ControllerFillError::Numeric("projection"))
    ));
    assert_eq!(source.calls, ["state", "spine", "state", "effective"]);
}
#[test]
fn yaw_projection_retains_source_nonunit_up_and_unknown_w() {
    let mut effective = SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM);
    effective.set_column(
        1,
        SkeletonRootVector {
            xyz: [0.0, 2.0, 0.0],
            w: None,
        },
    );
    let spine = SkeletonRootMatrix::translation(SkeletonRootVector::known([0.0, 3.0, 0.0, 0.0]));
    let projection = yaw_projection(&spine, &effective, &mut HostReckoningNumeric).unwrap();
    assert_eq!(
        projection.xyz.map(f32::to_bits),
        [0.0_f32, 12.0, 0.0].map(f32::to_bits)
    );
    assert_eq!(projection.w, None);
    assert_eq!(
        host_dot3([1.0, 2.0, 3.0, f32::NAN], [1.0; 4]).to_bits(),
        6.0_f32.to_bits()
    );
}
