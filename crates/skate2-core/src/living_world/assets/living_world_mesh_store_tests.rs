use super::*;

#[test]
fn bound_registry_absence_is_a_native_noop_and_score_thresholds_are_strict() {
    let mesh = MeshStore::construct();
    mesh.bind_assets(SelectedSimulationAssetServices::construct())
        .unwrap();
    mesh.try_add_tile(42).unwrap();
    mesh.try_remove_tile(42).unwrap();
    assert_eq!(
        mesh.try_get_cell(SkeletonRootVector::known([0.; 4]))
            .unwrap()
            .score(),
        10_000_000.
    );
    assert_eq!(cell_score(0., 0.), 0.);
    assert_eq!(cell_score(0.25, 0.), 250.);
    assert_eq!(cell_score(0.25, 0.25), 0.);
    assert_eq!(cell_score(f32::from_bits(0x3f7d70a4), 0.), 10_000_000.);
    assert_eq!(cell_score(1., 2.), 10_000_000.);
    assert!(cell_score(f32::NAN, 0.).is_nan());
}

#[test]
fn real_empty_map_uses_invalid_cell_score_without_reading_position_lanes() {
    let mesh = MeshMan::construct_fields();
    let position = SkeletonRootVector {
        xyz: [f32::NAN, f32::INFINITY, -0.0],
        w: None,
    };
    assert_eq!(
        mesh.store.try_get_cell(position).unwrap().score().to_bits(),
        10_000_000.0_f32.to_bits()
    );
    assert!(!mesh.store.try_point_on_road_network(position).unwrap());
    let validator = mesh.nav_validator.lock().unwrap();
    assert_eq!((validator.capacity(), validator.count()), (0, 0));
    assert!(validator.is_unallocated());
}

#[test]
fn unadmitted_tile_operation_prevents_later_empty_map_success() {
    let mesh = MeshStore::construct();
    let id = 0xfedc_ba98_7654_3210;
    assert_eq!(
        mesh.try_add_tile(id),
        Err(MeshStoreError::PendingTileAdmission { asset_id: id })
    );
    assert!(
        matches!(mesh.try_point_on_road_network(SkeletonRootVector::known([0.0; 4])), Err(MeshStoreError::PendingTileAdmission { asset_id }) if asset_id == id)
    );
    assert_eq!(
        mesh.try_remove_tile(4),
        Err(MeshStoreError::PendingTileAdmission { asset_id: id })
    );
}
