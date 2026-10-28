use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;
fn output() -> PhysicalPlayerPhysOut {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    output
}
#[derive(Default)]
struct Source {
    calls: Vec<String>,
    fail: Option<&'static str>,
    teleport: bool,
}
impl Source {
    fn record(&mut self, name: String) -> Result<(), &'static str> {
        let fail = self.fail.is_some_and(|expected| name == expected);
        self.calls.push(name);
        if fail { Err("source failure") } else { Ok(()) }
    }
}
impl ManagerFillSource for Source {
    type Error = &'static str;
    fn flag(&mut self, field: ManagerFlag) -> Result<bool, Self::Error> {
        self.record(format!("{field:?}"))?;
        Ok(field != ManagerFlag::RequestTeleport || self.teleport)
    }
    fn vector(&mut self, field: ManagerVector) -> Result<Vector, Self::Error> {
        self.record(format!("{field:?}"))?;
        Ok(Vector::known([2.0, -3.0, 4.0, -0.0]))
    }
    fn scalar(&mut self, field: ManagerScalar) -> Result<f32, Self::Error> {
        self.record(format!("{field:?}"))?;
        Ok(-0.0)
    }
    fn ground_transform(&mut self) -> Result<Matrix, Self::Error> {
        self.record("GroundTransform".into())?;
        Ok(Matrix::known([2.0; 16]))
    }
}
#[test]
fn locked_trajectory_and_reckoning_still_read_first_source_but_state_does_not() {
    let mut output = output();
    output.lock_after_update1().unwrap();
    let mut source = Source::default();
    assert!(matches!(
        output.try_fill_trajectory_selector(&mut source),
        Err(ManagerFillError::Access(_))
    ));
    assert_eq!(source.calls, ["TargetGrind"]);
    source.calls.clear();
    assert!(matches!(
        output.try_fill_reckoning(&mut source),
        Err(ManagerFillError::Access(_))
    ));
    assert_eq!(source.calls, ["Up"]);
    source.calls.clear();
    assert!(matches!(
        output.try_fill_state_changer(&mut source),
        Err(ManagerFillError::Access(_))
    ));
    assert!(source.calls.is_empty());
}
#[test]
fn failed_wallride_source_preserves_target_grind_store() {
    let mut output = output();
    let mut source = Source {
        fail: Some("TargetWallride"),
        ..Source::default()
    };
    assert!(matches!(
        output.try_fill_trajectory_selector(&mut source),
        Err(ManagerFillError::Source("source failure"))
    ));
    let air = output.concrete.air.as_ref().unwrap();
    assert!(air.is_targeting_grind);
    assert!(!air.is_targeting_wall_ride);
    assert_eq!(source.calls, ["TargetGrind", "TargetWallride"]);
}
#[test]
fn reckoning_preserves_full_vectors_signed_zero_and_last_source_failure() {
    let mut output = output();
    let mut source = Source {
        fail: Some("GroundTransform"),
        ..Source::default()
    };
    assert!(matches!(
        output.try_fill_reckoning(&mut source),
        Err(ManagerFillError::Source("source failure"))
    ));
    assert_eq!(
        source.calls,
        [
            "Up",
            "Heading",
            "SignedLean",
            "SpinSpeed",
            "Flipping",
            "FrontFlipping",
            "AnimationTilt",
            "GroundTransform"
        ]
    );
    assert_eq!(
        output
            .concrete
            .system
            .as_ref()
            .unwrap()
            .target_lean_angle
            .to_bits(),
        0x8000_0000
    );
    let air = output.concrete.air.as_ref().unwrap();
    assert_eq!(air.body_spin_speed.to_bits(), 0x8000_0000);
    assert!(air.is_flipping && air.is_front_flipping);
    assert_eq!(
        output
            .concrete
            .animation
            .as_ref()
            .unwrap()
            .anim_tilt
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [
            2.0f32.to_bits(),
            (-3.0f32).to_bits(),
            4.0f32.to_bits(),
            0x8000_0000
        ]
    );
}
#[test]
fn state_teleport_false_preserves_prior_true_and_failure_preserves_revert() {
    let mut output = output();
    output
        .concrete
        .state
        .as_mut()
        .unwrap()
        .physics_requested_teleport = true;
    output
        .try_fill_state_changer(&mut Source::default())
        .unwrap();
    assert!(
        output
            .concrete
            .state
            .as_ref()
            .unwrap()
            .physics_requested_teleport
    );
    output
        .concrete
        .state
        .as_mut()
        .unwrap()
        .revert_exited_normally = false;
    let mut source = Source {
        fail: Some("RequestTeleport"),
        ..Source::default()
    };
    assert!(matches!(
        output.try_fill_state_changer(&mut source),
        Err(ManagerFillError::Source("source failure"))
    ));
    assert!(
        output
            .concrete
            .state
            .as_ref()
            .unwrap()
            .revert_exited_normally
    );
    assert_eq!(source.calls, ["RevertExited", "RequestTeleport"]);
}
