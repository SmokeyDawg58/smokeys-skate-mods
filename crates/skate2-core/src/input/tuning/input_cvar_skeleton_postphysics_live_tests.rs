use super::*;
use crate::{
    input_cvar_runtime::InputCvarManager,
    physical_player_assembly::tests::issued_owner_for_grind_tests,
};

#[test]
fn default_and_live_ini_updates_use_the_same_linked_targets() {
    let references = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&references).unwrap();
    let overlay = DebugOverlaySlot::default();
    let mut cvars = SkeletonPostPhysicsCvars::new(&references, &overlay);
    assert_eq!(cvars.draw_body_part_collision().unwrap(), 0);
    assert_eq!(
        cvars.maximum_ground_distance_offboard().unwrap().to_bits(),
        0x3e99_999a
    );
    assert_eq!(
        cvars.small_object_force_scalar().unwrap().to_bits(),
        0x3f00_0000
    );
    manager.try_deserialize_startup(Some(b"[Config]\r\nkAllowAnyContact=1\r\nkSCMaxDistOffBoard=0.75\r\nkSmallObjectForceScalarHack=-0\r\nkDrawSkateboardError=1\r\n")).unwrap();
    assert_eq!(cvars.allow_any_contact().unwrap(), 1);
    assert_eq!(
        cvars.maximum_ground_distance_offboard().unwrap().to_bits(),
        0x3f40_0000
    );
    assert_eq!(
        cvars.small_object_force_scalar().unwrap().to_bits(),
        0x8000_0000
    );
    assert!(cvars.draw_skateboard_error().unwrap());
    assert!(matches!(
        cvars.require_debug_overlay(),
        Err(SkeletonPostPhysicsServiceError::Overlay(_))
    ));
}

#[test]
fn unlinked_target_is_not_replaced_with_initializer_default() {
    let references = InputCvarReferences::new_static_initialized();
    let overlay = DebugOverlaySlot::default();
    let mut cvars = SkeletonPostPhysicsCvars::new(&references, &overlay);
    assert!(matches!(
        cvars.show_ground_plane(),
        Err(SkeletonPostPhysicsServiceError::Cvar(_))
    ));
}

#[test]
fn post_record_cvar_read_does_not_acquire_reckoning_early() {
    let (_world, owner) = issued_owner_for_grind_tests();
    let references = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&references).unwrap();
    let overlay = DebugOverlaySlot::default();
    let held = owner.reckoning().lock().unwrap();
    let mut services = SkeletonPostRecordServices::new(&references, &overlay, &owner);
    assert_eq!(services.draw_hook_to_hips().unwrap(), 0);
    assert_eq!(services.use_normals_for_proxy_offset().unwrap(), 0);
    assert_eq!(
        services.scale_hook_to_hips_y().unwrap().to_bits(),
        0x3f00_0000
    );
    drop(held);
    // The fixture's constructed/unconstructed status is not replaced with a
    // test normal: compare the actual retained owner's state at the read site.
    let expected = owner
        .reckoning()
        .lock()
        .unwrap()
        .reckoning()
        .map(crate::reckoning::Reckoning::ground_normal);
    match expected {
        Some(value) => {
            let actual = services.ground_normal().unwrap();
            assert_eq!(actual.xyz.map(f32::to_bits), value.xyz.map(f32::to_bits));
            assert_eq!(actual.w.map(f32::to_bits), value.w.map(f32::to_bits));
        }
        None => assert!(matches!(
            services.ground_normal(),
            Err(SkeletonPostPhysicsServiceError::ReckoningUnconstructed)
        )),
    }
}

#[test]
fn fence_targets_capture_before_values_and_retain_live_updates() {
    let references = InputCvarReferences::new_static_initialized();
    let overlay = DebugOverlaySlot::default();
    let mut cvars = SkeletonPostPhysicsCvars::new(&references, &overlay);
    assert!(
        cvars
            .capture_fence_threshold(FenceThreshold::Distance)
            .is_err()
    );
    let mut manager = InputCvarManager::new();
    manager.try_link(&references).unwrap();
    let distance = cvars
        .capture_fence_threshold(FenceThreshold::Distance)
        .unwrap();
    let dot = cvars
        .capture_fence_threshold(FenceThreshold::OpposingDot)
        .unwrap();
    assert_eq!(
        cvars.read_fence_threshold(&distance).unwrap().to_bits(),
        0x3cf5_c28f
    );
    assert_eq!(
        cvars.read_fence_threshold(&dot).unwrap().to_bits(),
        0xbf7d_70a4
    );
    manager
        .try_deserialize_startup(Some(
            b"[Config]\r\nkSCDistToBeOnSamePlane=-0\r\nkSCDotValueToBeOpposing=-0.5\r\n",
        ))
        .unwrap();
    assert_eq!(
        cvars.read_fence_threshold(&distance).unwrap().to_bits(),
        0x8000_0000
    );
    assert_eq!(
        cvars.read_fence_threshold(&dot).unwrap().to_bits(),
        0xbf00_0000
    );
}
