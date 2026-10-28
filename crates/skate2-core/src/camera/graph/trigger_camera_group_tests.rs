use super::*;

#[test]
fn constructed_group_is_not_an_initialized_or_admitted_empty_query() {
    let mut group = CameraTriggerGroup::construct();
    let origin = SkeletonRootVector {
        xyz: [0.0; 3],
        w: None,
    };
    assert_eq!(
        group.try_subject_box_is_empty(origin),
        Err(CameraTriggerError::MissingAdmission)
    );
    assert_eq!(
        group.try_finish_admission(&[]),
        Err(CameraTriggerError::MissingInitialization)
    );
    let assets = SelectedSimulationAssetServices::construct();
    group
        .try_initialize(
            assets,
            OctreeBounds {
                minimum: [0.0; 4],
                maximum: [0.0; 4],
            },
        )
        .unwrap();
    assert_eq!(
        group.bounds.unwrap().minimum.map(f32::to_bits),
        [-1000.0_f32, -1000.0, -1000.0, 0.0].map(f32::to_bits)
    );
    assert_eq!(
        group.try_subject_box_is_empty(origin),
        Err(CameraTriggerError::MissingAdmission)
    );
    group.try_finish_admission(&[]).unwrap();
    assert!(group.try_subject_box_is_empty(origin).unwrap());
    let tree = group.group.as_ref().unwrap();
    assert_eq!(tree.entry_fill_index, 0);
    assert!(tree.free_indices.is_empty());
    assert!(tree.invalid_entry.is_none());
    let volume = tree.collision_volume.as_ref().unwrap();
    assert_eq!(volume.volume_type(), Some(VolumeType::Box));
    assert_eq!(volume.shape_words, [1.0_f32.to_bits(); 3]);
}

#[test]
fn invalid_bounds_keep_published_group_and_prior_unit_box() {
    let mut group = CameraTriggerGroup::construct();
    let assets = SelectedSimulationAssetServices::construct();
    assert_eq!(
        group.try_initialize(
            Arc::clone(&assets),
            OctreeBounds {
                minimum: [2.0, 0.0, 0.0, 0.0],
                maximum: [1.0, 0.0, 0.0, 0.0],
            }
        ),
        Err(CameraTriggerError::InvalidBounds)
    );
    assert!(Arc::ptr_eq(group.assets.as_ref().unwrap(), &assets));
    let tree = group.other_groups[0].as_ref().unwrap();
    assert!(group.group.is_none());
    assert!(tree.collision_volume.is_some());
    assert!(tree.tree.is_none());
    assert!(tree.bounds.is_none());
}
