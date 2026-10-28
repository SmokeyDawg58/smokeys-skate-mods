use super::{RetainedTrajectoryLaunchInfo, Vector, finish_com_launch};
use crate::input_cvar_runtime::{InputCvarManager, InputCvarReferences};

#[test]
fn unlinked_delta_failure_keeps_completed_velocity_and_jump_flag_only() {
    let mut out = RetainedTrajectoryLaunchInfo::constructed();
    out.start_velocity = Vector::known([100.0; 4]);
    out.trajectory_start_override = Vector::known([19.0; 4]);
    out.board_position_override = Vector::known([23.0; 4]);
    let refs = InputCvarReferences::new_static_initialized();
    assert!(finish_com_launch(&mut out, Vector::known([2.0, 3.0, 4.0, 5.0]), &refs).is_err());
    assert!(out.player_jumped);
    assert_eq!(out.start_velocity.xyz[0].to_bits(), 2.0_f32.to_bits());
    assert_eq!(out.start_velocity.xyz[2].to_bits(), 4.0_f32.to_bits());
    assert_eq!(out.start_velocity.w, Some(5.0));
    assert_eq!(out.trajectory_start_override, Vector::known([19.0; 4]));
    assert_eq!(out.board_position_override, Vector::known([23.0; 4]));
    assert!(!out.use_start_override);
}

#[test]
fn actual_linked_values_drive_offsets_cones_and_low_halfword_without_clamping() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    manager.try_deserialize_startup(Some(b"[Config]\r\nkOBA_BoardPosDelta=-7\r\nkOBA_TrajPosDelta=9\r\nkTrajectoryConeAngleScalar=2\r\nkOBA_NumTrajForMount=-2\r\n")).unwrap();
    let mut out = RetainedTrajectoryLaunchInfo::constructed();
    out.com_position = Vector {
        xyz: [2.0, 20.0, 3.0],
        w: None,
    };
    out.board_position = Vector::known([999.0; 4]);
    out.cone_angle_x = 10.0;
    out.cone_angle_z = 40.0;
    finish_com_launch(
        &mut out,
        Vector {
            xyz: [1.0, 5.0, 2.0],
            w: None,
        },
        &refs,
    )
    .unwrap();
    assert_eq!(
        out.trajectory_start_override,
        Vector {
            xyz: [2.0, 29.0, 3.0],
            w: None
        }
    );
    assert_eq!(
        out.board_position_override,
        Vector {
            xyz: [2.0, 13.0, 3.0],
            w: None
        }
    );
    assert_eq!((out.cone_angle_x, out.cone_angle_z), (20.0, 80.0));
    assert_eq!(out.num_trajectories, 65534);
    assert!(out.use_start_override && out.player_jumped);
    assert!(out.start_velocity.w.is_none());
    assert_eq!(out.start_velocity.xyz[1].to_bits(), 0x409a_c5f9);
}

#[test]
fn captured_delta_target_survives_relink_and_reads_latest_original_value() {
    use crate::input_cvar_runtime::air_com_launch::ComLaunchFloat;
    let refs = InputCvarReferences::new_static_initialized();
    let mut original = InputCvarManager::new();
    original.try_link(&refs).unwrap();
    let captured = refs
        .try_capture_com_launch_float(ComLaunchFloat::BoardDelta)
        .unwrap();
    assert_eq!(captured.read().unwrap().to_bits(), 0xbf4c_cccd);
    original
        .try_deserialize_startup(Some(b"[Config]\r\nkOBA_BoardPosDelta=-7\r\n"))
        .unwrap();
    let mut replacement = InputCvarManager::new();
    replacement.try_link(&refs).unwrap();
    assert_eq!(captured.read().unwrap().to_bits(), (-7.0_f32).to_bits());
    assert_eq!(
        refs.try_capture_com_launch_float(ComLaunchFloat::BoardDelta)
            .unwrap()
            .read()
            .unwrap()
            .to_bits(),
        0xbf4c_cccd
    );
    assert_eq!(
        refs.try_capture_com_launch_float(ComLaunchFloat::TrajectoryDelta)
            .unwrap()
            .read()
            .unwrap()
            .to_bits(),
        0xbf26_6666
    );
    assert_eq!(
        refs.try_capture_com_launch_float(ComLaunchFloat::ConeScalar)
            .unwrap()
            .read()
            .unwrap()
            .to_bits(),
        0x3ecc_cccd
    );
    assert_eq!(refs.try_com_launch_count().unwrap(), 5);
}
