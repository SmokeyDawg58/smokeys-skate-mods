use super::*;
use crate::{
    input_cvar_runtime::InputCvarManager,
    skateboard_spawn::{SkateboardSpawnError, SkateboardSpawnParts, try_set_skateboard_transform},
    skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_drives::SkeletonDriveCvars,
};

#[test]
fn constructor_cvars_require_link_and_follow_changes_between_each_sample() {
    let references = InputCvarReferences::new_static_initialized();
    let mut drives = ConstructorDriveCvars(&references);
    assert_eq!(
        references.try_orthogonal_tolerance(),
        Err(InputCvarError::Unlinked("kOrthonalTolerance"))
    );
    assert_eq!(
        drives.hard_drive_displacement(),
        Err(InputCvarError::Unlinked("kALC_HardDriveDisp"))
    );
    assert_eq!(
        drives.hard_drive_strength(),
        Err(InputCvarError::Unlinked("kALC_HardDriveStr"))
    );
    let mut manager = InputCvarManager::new();
    manager.try_link(&references).unwrap();
    assert_eq!(
        references.try_orthogonal_tolerance().unwrap().to_bits(),
        0x3c23_d70a
    );
    assert_eq!(
        drives.hard_drive_displacement().unwrap().to_bits(),
        1.0_f32.to_bits()
    );
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkALC_HardDriveStr=13\r\n"))
        .unwrap();
    assert_eq!(
        drives.hard_drive_strength().unwrap().to_bits(),
        13.0_f32.to_bits()
    );
    manager.try_deserialize_startup(Some(b"[Config]\r\nkALC_HardDriveDisp=-2\r\nkALC_HardDriveStr=21\r\nkOrthonalTolerance=-0\r\n")).unwrap();
    assert_eq!(
        drives.hard_drive_displacement().unwrap().to_bits(),
        (-2.0_f32).to_bits()
    );
    assert_eq!(
        drives.hard_drive_strength().unwrap().to_bits(),
        21.0_f32.to_bits()
    );
    assert_eq!(
        references.try_orthogonal_tolerance().unwrap().to_bits(),
        (-0.0_f32).to_bits()
    );
    InputCvarManager::new().try_link(&references).unwrap();
    assert_eq!(
        drives.hard_drive_strength().unwrap().to_bits(),
        10.0_f32.to_bits()
    );
}

// Only the failure chronology is controlled here; production tolerance lookup
// uses the same actual references as PhysicalConstructorServices.
struct Spawn<'a>(&'a InputCvarReferences);
impl FallibleSkateboardSpawnServices for Spawn<'_> {
    fn try_current_orthogonal_tolerance(&mut self) -> Result<f32, InputCvarError> {
        self.0.try_orthogonal_tolerance()
    }
    fn orthogonality_assertion_failed(&mut self) {
        panic!("unexpected assertion");
    }
}
struct Parts;
impl SkateboardSpawnParts for Parts {
    type Error = std::convert::Infallible;
    fn live_board_part(&mut self, _: usize) -> Result<[f32; 16], Self::Error> {
        panic!("Part read before linked tolerance");
    }
    fn set_board_part(&mut self, _: usize, _: [f32; 16]) -> Result<(), Self::Error> {
        panic!("Part write before linked tolerance");
    }
    fn set_hook_part(&mut self, _: [f32; 16]) -> Result<(), Self::Error> {
        panic!("hook write before linked tolerance");
    }
}
#[test]
fn missing_spawn_target_stops_before_assertion_or_parts() {
    let references = InputCvarReferences::new_static_initialized();
    assert_eq!(
        try_set_skateboard_transform(
            &mut Parts,
            FRESH_VOLUME_TRANSFORM,
            &mut Spawn(&references),
            &mut HostSkateboardSpawnMathNumeric
        ),
        Err(SkateboardSpawnError::Cvar(InputCvarError::Unlinked(
            "kOrthonalTolerance"
        )))
    );
}
