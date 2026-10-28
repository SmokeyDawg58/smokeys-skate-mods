//! Actual source-producing fixtures and real pool occurrences, not Tick traces.
use super::*;
use crate::{
    simulation_volume_integral::HostSimulationVolumeIntegralNumeric,
    skeleton_body_attributes::{
        self as keys, SkeletonBodyAttributeRead, SkeletonBodyAttributeRoute, SkeletonBodyDebugRead,
    },
    skeleton_body_definition::SkeletonBodyDefinition,
    skeleton_body_joints::{HostSkeletonJointNumeric, SkeletonBodyJoints},
    skeleton_body_mass::HostSkeletonBodyMassNumeric,
    skeleton_collision_data::SkeletonCollisionData,
    skeleton_culling::{SkeletonCullingInventory, SkeletonCullingKey},
    skeleton_hat::{HatLayout, HostHatNumeric},
    skeleton_joint_attributes::{SkeletonJointAttributeRead, SkeletonJointByteRoute},
    skeleton_runtime_state::tests::CollisionReader,
};

struct Reader {
    normal_enabled: u8,
}
impl SkeletonBodyDebugRead for Reader {
    type Error = Infallible;
    fn mode(&mut self) -> Result<u32, Self::Error> {
        Ok(1)
    }
    fn factor(&mut self) -> Result<f32, Self::Error> {
        Ok(2.0)
    }
}
impl SkeletonBodyAttributeRead for Reader {
    type Error = Infallible;
    type DebugInstance = Self;
    fn float(&mut self, _route: SkeletonBodyAttributeRoute, key: u64) -> Result<f32, Self::Error> {
        Ok(match key {
            keys::ROOT_RADIUS => 0.125,
            keys::ROOT_HALF_LENGTH => 0.25,
            _ => 1.0,
        })
    }
    fn byte(&mut self, route: SkeletonBodyAttributeRoute, key: u64) -> Result<u8, Self::Error> {
        if route == SkeletonBodyAttributeRoute::LocalSkeleton {
            assert_eq!(key, super::super::skeleton_normal::NORMAL_COLLISION_ENABLED);
            Ok(self.normal_enabled)
        } else {
            assert_eq!(key, keys::ENABLE_JOINTS);
            Ok(1)
        }
    }
    fn debug_instance(&mut self) -> Result<Self, Self::Error> {
        Ok(Self {
            normal_enabled: self.normal_enabled,
        })
    }
}
impl SkeletonJointAttributeRead for Reader {
    type Error = Infallible;
    fn cached_prism_velocity(&mut self) -> Result<[f32; 4], Self::Error> {
        Ok([0.01, 0.02, 0.03, -0.0])
    }
    fn byte(&mut self, _route: SkeletonJointByteRoute, key: u64) -> Result<u8, Self::Error> {
        Ok(u8::from(key == keys::ENABLE_JOINTS))
    }
    fn skeleton_float(&mut self, _key: u64) -> Result<f32, Self::Error> {
        Ok(10.0)
    }
}

/// Synthetic 24-bone source inputs plus actual hat constructor. This creates no
/// hand-assembled production Volumes, body observations or publication records.
pub(crate) fn fixture(hat_collision: Option<bool>) -> SkeletonNormalAssembly {
    let (base, _) = crate::skeleton_assembly::tests::fixture();
    // Original other-hat BIN 9ccb0 layout, already independently documented.
    let words = [
        0x3cf5_c28f_u32,
        0,
        0xbca3_d70a,
        0,
        0x3e99_999a,
        0x3fcc_cccd,
        0x3e80_0000,
        0,
        0x3ca3_d70a,
        0x3e38_51ec,
        0,
        0,
    ];
    let bytes = std::array::from_fn(|byte| words[byte / 4].to_be_bytes()[byte % 4]);
    let layout = hat_collision.map(|_| HatLayout::from_bytes_for_test(bytes));
    let mut reader = Reader { normal_enabled: 1 };
    let body = Arc::new(
        SkeletonBodyDefinition::try_create(
            Arc::clone(&base.data),
            Arc::clone(&base.attributes),
            &mut reader,
            u32::from(hat_collision.is_some()),
            layout.as_ref(),
            &mut || hat_collision.expect("only a present hat samples collision enable"),
            &mut HostSkeletonBodyMassNumeric,
            &mut HostSimulationVolumeIntegralNumeric,
            &mut HostHatNumeric,
        )
        .unwrap(),
    );
    let joints = SkeletonBodyJoints::try_create(
        &body.data,
        &body.attributes,
        body.allocated_joint_count,
        &mut reader,
        &mut HostSkeletonJointNumeric,
    )
    .unwrap();
    let normal = SkeletonCullingInventory::default()
        .try_get_or_create(SkeletonCullingKey::Normal)
        .unwrap();
    SkeletonNormalAssembly::try_create(
        body,
        joints,
        normal,
        SkeletonCollisionData::try_new(&mut CollisionReader).unwrap(),
    )
    .unwrap()
}

pub(crate) fn set_normal(
    owner: &mut SkeletonNormalAssembly,
    backend: &mut SkeletonPreNormalSimulationBackend,
    enabled: u8,
) {
    owner.try_set_pre_normal_properties(backend).unwrap();
    owner
        .try_set_up_normal(
            backend,
            &mut Reader {
                normal_enabled: enabled,
            },
            &mut |value| Ok::<_, Infallible>(value.cos()),
        )
        .unwrap();
}

#[test]
fn creation_and_refresh_require_the_exact_attached_owner_and_body_occurrences() {
    let mut owner = fixture(None);
    let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    assert!(matches!(
        SkeletonCollisionPublication::try_create(&owner, &backend, -1.0),
        Err(SkeletonCollisionPublicationError::Detached)
    ));
    owner.attach(&mut backend).unwrap();
    let mut publication = SkeletonCollisionPublication::try_create(&owner, &backend, -1.0).unwrap();
    let frame = backend.collision_body_pool().frame_identity().unwrap();
    assert!(publication.has_bounds_for_frame(frame));
    let foreign = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    assert!(matches!(
        SkeletonCollisionPublication::try_create(&owner, &foreign, -1.0),
        Err(SkeletonCollisionPublicationError::ForeignPool)
    ));
    assert_eq!(
        publication.try_refresh(&owner, &foreign),
        Err(SkeletonCollisionPublicationError::ForeignPool)
    );
    assert!(!publication.has_bounds_for_frame(frame));
    publication.try_refresh(&owner, &backend).unwrap();
    let other = fixture(None);
    assert_eq!(
        publication.try_refresh(&other, &backend),
        Err(SkeletonCollisionPublicationError::DefinitionChanged)
    );
    assert!(!publication.has_bounds_for_frame(frame));
    owner.detach(&mut backend).unwrap();
    assert_eq!(
        publication.try_refresh(&owner, &backend),
        Err(SkeletonCollisionPublicationError::Detached)
    );
    owner.attach(&mut backend).unwrap();
    assert_eq!(
        publication.try_refresh(&owner, &backend),
        Err(SkeletonCollisionPublicationError::BodyOccurrenceChanged { part: 0 })
    );
    owner.detach(&mut backend).unwrap();
}

#[test]
fn normal_refresh_changes_live_fields_not_create_inline_words_and_stages_bounds() {
    let mut owner = fixture(None);
    let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    owner.attach(&mut backend).unwrap();
    let mut publication = SkeletonCollisionPublication::try_create(&owner, &backend, -1.0).unwrap();
    assert_eq!(publication.records().len(), 24);
    assert_eq!(publication.rigid_bodies().len(), 24);
    assert_eq!(publication.group(), 1);
    assert_eq!(publication.num_static(), 0);
    assert!(!publication.all_parts_frozen());
    assert!(!publication.try_all_parts_frozen(&owner, &backend).unwrap());
    assert_eq!(publication.bend_threshold().to_bits(), (-1.0_f32).to_bits());
    assert_eq!(publication.body_pool_authority(), backend.authority());
    assert_eq!(
        (
            publication.culling_table().rows,
            publication.culling_table().columns
        ),
        (24, 24)
    );
    let initial = publication.records().to_vec();
    let initial_bodies = publication.rigid_bodies().to_vec();
    let initial_bounds = publication.bounds();
    let frame = backend.collision_body_pool().frame_identity().unwrap();
    publication.invalidate_refresh();
    set_normal(&mut owner, &mut backend, 0);
    assert_eq!(owner.state().volume_headers[0].group_id, 4);
    publication.try_update_volumes(&owner, &backend).unwrap();
    assert!(!publication.has_bounds_for_frame(frame));
    assert_eq!(publication.group(), 6);
    assert_eq!(publication.rigid_bodies(), initial_bodies);
    for (part, (record, previous)) in publication.records().iter().zip(&initial).enumerate() {
        assert!(!record.enabled);
        assert_eq!(record.material, *owner.state().material(part));
        assert_eq!(record.group, 6);
        assert_eq!(record.volume, previous.volume);
        assert_eq!(record.surface_id, previous.surface_id);
        assert_eq!(record.rigid_body_index, previous.rigid_body_index);
        assert_eq!(record.padding.to_bits(), previous.padding.to_bits());
    }
    assert_eq!(publication.records()[0].volume.group_id, 0);
    assert_eq!(publication.records()[1].volume.flags & 1, 1);
    assert_ne!(
        publication.records()[1].material.static_friction.to_bits(),
        initial[1].material.static_friction.to_bits()
    );
    let bounds = publication.try_stage_bounds(&owner, &backend).unwrap();
    assert_eq!(bounds, initial_bounds); // Disabled records STILL contribute.
    publication.publish_bounds(bounds, frame);
    assert!(publication.has_bounds_for_frame(frame));
    publication.invalidate_refresh();
    assert_eq!(
        publication.try_stage_bounds(&owner, &backend),
        Err(SkeletonCollisionPublicationError::StaleVolumes)
    );
    owner.detach(&mut backend).unwrap();
}

#[test]
fn hat_children_are_flattened_without_outer_flag_inheritance_or_inline_rewrites() {
    for enabled in [false, true] {
        let mut owner = fixture(Some(enabled));
        let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
        owner.attach(&mut backend).unwrap();
        let mut publication =
            SkeletonCollisionPublication::try_create(&owner, &backend, -1.0).unwrap();
        assert_eq!(publication.records().len(), 25);
        assert_eq!(publication.rigid_bodies().len(), 24);
        assert_eq!(publication.records()[1].volume_type, VolumeType::Cylinder);
        assert_eq!(publication.records()[2].volume_type, VolumeType::Sphere);
        assert_eq!(publication.records()[1].rigid_body_index, 1);
        assert_eq!(publication.records()[2].rigid_body_index, 1);
        assert_eq!(publication.records()[3].rigid_body_index, 2);
        assert_eq!(publication.records()[1].enabled, enabled);
        assert!(publication.records()[2].enabled);
        let child_volumes = [
            publication.records()[1].volume.clone(),
            publication.records()[2].volume.clone(),
        ];
        publication.invalidate_refresh();
        set_normal(&mut owner, &mut backend, 0);
        assert_eq!(owner.state().volume_headers[1].flags & 1, 0);
        publication.try_refresh(&owner, &backend).unwrap();
        assert_eq!(publication.records()[1].enabled, enabled);
        assert!(publication.records()[2].enabled);
        for (record, child) in publication.records()[1..3].iter().zip(&child_volumes) {
            assert_eq!(&record.volume, child);
            assert_eq!(record.material, *owner.state().material(1));
            assert_eq!(record.group, 6);
        }
        assert!(!publication.records()[3].enabled);
        owner.detach(&mut backend).unwrap();
    }
}
