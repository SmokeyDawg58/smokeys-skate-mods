//! Controlled original-shaped inputs test failure prefixes. Owner identity is
//! issued by the actual paired factory and its real manager, never fabricated.
use super::*;
use crate::{
    attribute_runtime::{
        AttributeRuntimeVault,
        tests::{collection, definition, graph, node},
    },
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyConstruction, PhysicalPlayerAssemblyInputs,
    },
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_attributes::SkeletonAttributeClass,
};

fn inputs(vault: Arc<AttributeRuntimeVault>) -> PhysicalPlayerAssemblyInputs {
    let (mut hierarchy, mut parameters, matrices) = crate::skeleton_data_tests::fixture();
    // These downstream failure fixtures must pass the preceding native
    // Skeleton::Init Head lookup and mapped-parent assertion.
    let head = skate2_formats::andale::PackedFastString::encode("Head").unwrap();
    hierarchy.bone_names[1] = "Head".into();
    hierarchy.packed_bone_names[1] = head;
    parameters.bones[1].name = "Head".into();
    parameters.bones[1].packed_name = head;
    PhysicalPlayerAssemblyInputs {
        cvars: crate::input_cvar_runtime::InputCvarReferences::new_static_initialized(),
        vault,
        pose_hierarchy: Arc::new(hierarchy.clone()),
        hierarchy: Arc::new(hierarchy),
        parameters: Arc::new(parameters),
        pose_matrices: matrices.into(),
        spawn: FRESH_VOLUME_TRANSFORM,
        hat_type: 0,
        bouncy_ragdoll: 0x80,
        is_human: 0xff,
    }
}

fn issue_owner(
    inputs: PhysicalPlayerAssemblyInputs,
) -> (PhysicalAssemblyWorld, Arc<PhysicalPlayerAssemblyOwner>) {
    // Actual manager StaticWorld body, no invented board or humanoid member.
    let mut world = PhysicalAssemblyWorld::try_new(1, 0, 0, 0).unwrap();
    let owner = {
        let factory = PhysicalPlayerAssemblyConstruction::new(&mut world, inputs);
        Arc::clone(factory.owner())
    };
    (world, owner)
}

fn malformed_scalar_vault(class: u64, key: u64) -> Arc<AttributeRuntimeVault> {
    let mut vault = graph(class, vec![definition(key, 4, 1)]);
    collection(
        &mut vault,
        class,
        DEFAULT_COLLECTION,
        0,
        None,
        vec![node(key, 0x40, 0x7f00_0000)],
    );
    Arc::new(vault)
}

fn assert_no_replay_or_handoff(
    construction: &mut SkeletonAssemblyConstruction,
    inventory: &mut SkeletonCullingInventory,
    phase: SkeletonAssemblyConstructionPhase,
) {
    assert_eq!(construction.phase(), phase);
    assert!(
        matches!(construction.try_construct(inventory, &mut || panic!("no replay")),
        Err(SkeletonAssemblyConstructionError::WrongPhase(actual)) if actual == phase)
    );
    assert!(matches!(construction.take_assembly(),
        Err(SkeletonAssemblyConstructionError::WrongPhase(actual)) if actual == phase));
    assert!(construction.assembly().is_none());
}

#[test]
fn invalid_physics_count_stops_before_any_collision_or_attribute_provider() {
    let vault = malformed_scalar_vault(
        SkeletonCollisionAttributes::CLASS,
        crate::skeleton_collision_attributes::GROIN_RADIUS,
    );
    let mut input = inputs(vault);
    Arc::get_mut(&mut input.parameters).unwrap().bones.pop();
    let (_world, owner) = issue_owner(input);
    let owner_lifetime = Arc::downgrade(&owner);
    let mut construction = SkeletonAssemblyConstruction::new(Arc::clone(&owner));
    assert!(Arc::ptr_eq(construction.owner(), &owner));
    drop(owner);
    let mut inventory = SkeletonCullingInventory::default();
    assert!(matches!(
        construction.try_construct(&mut inventory, &mut || panic!("no hat read")),
        Err(SkeletonAssemblyConstructionError::Data(
            SkeletonDataError::PhysicsBoneCount { actual: 23 }
        ))
    ));
    assert!(construction.data().is_none());
    assert!(construction.collision_reader.is_none());
    assert!(construction.attribute_reader.is_none());
    assert!(construction.body_reader_mut().is_none());
    assert!(inventory.find(SkeletonCullingKey::Normal).is_none());
    assert!(inventory.find(SkeletonCullingKey::Ragdoll).is_none());
    assert_no_replay_or_handoff(
        &mut construction,
        &mut inventory,
        SkeletonAssemblyConstructionPhase::Data,
    );
    assert!(owner_lifetime.upgrade().is_some());
    drop(construction);
    assert!(owner_lifetime.upgrade().is_none());
}

#[test]
fn collision_failure_retains_real_mapping_and_separate_pose_owner() {
    let vault = malformed_scalar_vault(
        SkeletonCollisionAttributes::CLASS,
        crate::skeleton_collision_attributes::GROIN_RADIUS,
    );
    let mut input = inputs(vault);
    Arc::get_mut(&mut input.pose_matrices).unwrap()[0][15] = f32::from_bits(0x7fc0_1234);
    let (_world, owner) = issue_owner(input);
    let mut construction = SkeletonAssemblyConstruction::new(Arc::clone(&owner));
    let mut inventory = SkeletonCullingInventory::default();
    assert!(matches!(
        construction.try_construct(&mut inventory, &mut || panic!("no hat read")),
        Err(SkeletonAssemblyConstructionError::CollisionData(
            AttributeRuntimeError::ScalarSize { size: 1 }
        ))
    ));
    let data = construction.data().unwrap();
    assert!(Arc::ptr_eq(data.hierarchy(), &owner.inputs().hierarchy));
    assert!(Arc::ptr_eq(data.parameters(), &owner.inputs().parameters));
    assert!(!Arc::ptr_eq(
        &owner.inputs().hierarchy,
        &owner.inputs().pose_hierarchy
    ));
    assert_eq!(data.physics_pose()[0][15].to_bits(), 0x7fc0_1234);
    assert_eq!(
        (owner.inputs().bouncy_ragdoll, owner.inputs().is_human),
        (0x80, 0xff)
    );
    assert!(construction.collision_reader.is_some());
    assert!(construction.collision_data.is_none());
    assert!(construction.attribute_reader.is_none());
    assert!(construction.attributes().is_none());
    assert!(construction.body().is_none());
    assert_no_replay_or_handoff(
        &mut construction,
        &mut inventory,
        SkeletonAssemblyConstructionPhase::CollisionData,
    );
}

#[test]
fn init_attributes_failure_preserves_collision_constructor_data() {
    let vault = malformed_scalar_vault(
        SkeletonAttributeClass::Skeleton.key(),
        0x8a43_3894_d234_21e6,
    );
    let (_world, owner) = issue_owner(inputs(vault));
    let mut construction = SkeletonAssemblyConstruction::new(owner);
    let mut inventory = SkeletonCullingInventory::default();
    assert!(matches!(
        construction.try_construct(&mut inventory, &mut || panic!("no hat read")),
        Err(SkeletonAssemblyConstructionError::Attributes(
            SkeletonAttributeLoadError::Read(AttributeRuntimeError::ScalarSize { size: 1 })
        ))
    ));
    assert!(construction.data().is_some());
    let collision = construction.collision_data.as_ref().unwrap();
    assert_eq!(collision.is_compliant, [0; 24]);
    assert_eq!(collision.priority.map(f32::to_bits), [0; 24]);
    assert_eq!(
        collision.specific_parts.radius_squared.map(f32::to_bits),
        [0; 2]
    );
    assert_eq!(collision.specific_parts.contact, [None; 2]);
    assert!(construction.attribute_reader.is_some());
    assert!(construction.attributes().is_none());
    assert!(construction.body_reader_mut().is_none());
    assert_no_replay_or_handoff(
        &mut construction,
        &mut inventory,
        SkeletonAssemblyConstructionPhase::Attributes,
    );
}

#[test]
fn zero_vault_defaults_reach_body_refusal_without_fabricated_geometry_or_culling() {
    let vault = Arc::new(graph(SkeletonAttributeClass::Skeleton.key(), vec![]));
    let vault_lifetime = Arc::downgrade(&vault);
    let (_world, owner) = issue_owner(inputs(vault));
    let mut construction = SkeletonAssemblyConstruction::new(owner);
    let mut inventory = SkeletonCullingInventory::default();
    assert!(matches!(
        construction.try_construct(&mut inventory, &mut || panic!("hat0 is not sampled")),
        Err(SkeletonAssemblyConstructionError::Body(
            SkeletonBodyDefinitionError::InvalidRagdollNumerator { bits: 0 }
        ))
    ));
    assert!(construction.data().is_some());
    assert!(construction.collision_data.is_some());
    let attributes = construction.attributes().unwrap();
    assert_eq!(attributes.normal.static_friction.to_bits(), 0);
    assert_eq!(
        attributes.bones.map(|bone| bone.mass_factor.to_bits()),
        [0; 24]
    );
    assert!(construction.body_reader_mut().is_some());
    assert!(construction.joint_reader.is_some());
    assert!(construction.hat_reader.is_some());
    assert!(construction.hat_layout.is_none());
    assert!(construction.body().is_none());
    assert!(construction.joints.is_none());
    assert!(construction.initial_culling().is_none());
    assert!(construction.ragdoll_culling().is_none());
    assert!(inventory.find(SkeletonCullingKey::Normal).is_none());
    assert!(inventory.find(SkeletonCullingKey::Ragdoll).is_none());
    assert_no_replay_or_handoff(
        &mut construction,
        &mut inventory,
        SkeletonAssemblyConstructionPhase::Body,
    );
    assert!(vault_lifetime.upgrade().is_some());
    drop(construction);
    assert!(vault_lifetime.upgrade().is_none());
}
