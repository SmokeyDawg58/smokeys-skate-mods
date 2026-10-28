//! Source-producing synthetic fixtures, not original tuning or a whole Init.
use super::*;
use crate::{
    dynamic_assembly_session::DynamicAssemblyAttachError,
    simulation_volume_integral::HostSimulationVolumeIntegralNumeric,
    skeleton_body_attributes::{
        self as body_keys, SkeletonBodyAttributeRead, SkeletonBodyAttributeRoute,
        SkeletonBodyDebugRead,
    },
    skeleton_body_joints::HostSkeletonJointNumeric,
    skeleton_body_mass::HostSkeletonBodyMassNumeric,
    skeleton_culling::SkeletonCullingInventory,
    skeleton_hat::HostHatNumeric,
    skeleton_joint_attributes::{SkeletonJointAttributeRead, SkeletonJointByteRoute},
};

struct BodyReader;
struct DebugReader;
impl SkeletonBodyDebugRead for DebugReader {
    type Error = std::convert::Infallible;
    fn mode(&mut self) -> Result<u32, Self::Error> {
        Ok(1)
    }
    fn factor(&mut self) -> Result<f32, Self::Error> {
        Ok(2.0)
    }
}
impl SkeletonBodyAttributeRead for BodyReader {
    type Error = std::convert::Infallible;
    type DebugInstance = DebugReader;
    fn float(&mut self, _route: SkeletonBodyAttributeRoute, key: u64) -> Result<f32, Self::Error> {
        Ok(match key {
            body_keys::ROOT_RADIUS => 0.125,
            body_keys::ROOT_HALF_LENGTH => 0.25,
            _ => 1.0,
        })
    }
    fn byte(&mut self, _route: SkeletonBodyAttributeRoute, key: u64) -> Result<u8, Self::Error> {
        assert_eq!(key, body_keys::ENABLE_JOINTS);
        Ok(1)
    }
    fn debug_instance(&mut self) -> Result<Self::DebugInstance, Self::Error> {
        Ok(DebugReader)
    }
}
struct JointReader;
impl SkeletonJointAttributeRead for JointReader {
    type Error = std::convert::Infallible;
    fn cached_prism_velocity(&mut self) -> Result<[f32; 4], Self::Error> {
        Ok([0.01, 0.02, 0.03, -0.0])
    }
    fn byte(&mut self, _route: SkeletonJointByteRoute, key: u64) -> Result<u8, Self::Error> {
        Ok(u8::from(key == body_keys::ENABLE_JOINTS))
    }
    fn skeleton_float(&mut self, _key: u64) -> Result<f32, Self::Error> {
        Ok(10.0)
    }
}

pub(crate) fn fixture() -> (Arc<SkeletonBodyDefinition>, SkeletonBodyJoints) {
    let (data, attributes) = crate::skeleton_body_definition::tests::inputs();
    let mut pose = *data.physics_pose();
    pose[0][12] = 7.5;
    let data = Arc::new(
        crate::skeleton_data::SkeletonData::try_new(
            Arc::clone(data.hierarchy()),
            Arc::clone(data.parameters()),
            crate::skeleton_data::SkeletonPoseView {
                hierarchy: data.hierarchy(),
                matrices: &pose,
            },
        )
        .unwrap(),
    );
    let body = Arc::new(
        SkeletonBodyDefinition::try_create(
            data,
            Arc::new(attributes),
            &mut BodyReader,
            0,
            None,
            &mut || panic!("no hat geometry in this controlled fixture"),
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
        &mut JointReader,
        &mut HostSkeletonJointNumeric,
    )
    .unwrap();
    (body, joints)
}

#[test]
fn binding_retains_current_mass_cells_real_geometry_and_constructor_placement() {
    let (mut body, joints) = fixture();
    // Synthetic decoy caches discriminate a wrong cache-based binding. Native
    // Init actually saves the animated tensor AFTER debug adjustment, so its
    // unmodified value is not a valid discriminator against current inertia.
    let part = &mut Arc::get_mut(&mut body).unwrap().parts[1];
    part.animated.inverse_tensor = [41.0; 4];
    part.animated.inverse_mass = 43.0;
    part.ragdoll.inverse_tensor = [47.0; 4];
    part.ragdoll.inverse_mass = 53.0;
    let expected_limits: Vec<_> = joints.joints.iter().map(|joint| joint.limits).collect();
    let expected_mapping = joints.emitted_for_part;
    let session =
        try_create_pre_normal_skeleton_assembly(Arc::clone(&body), joints, normal_culling())
            .unwrap();
    let template = session.template();
    let binding = template.collision();
    assert!(Arc::ptr_eq(binding.body(), &body));
    assert_eq!(binding.emitted_for_part(), &expected_mapping);
    assert_eq!(template.drives().len(), 0);
    assert!(!session.is_attached());
    assert_eq!(body.data.physics_pose()[0][12].to_bits(), 7.5_f32.to_bits());
    for transform in session
        .part_transforms()
        .map(|matrix| matrix.expect("known fixture Part cache"))
    {
        assert_eq!(
            transform.map(f32::to_bits),
            FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
        );
    }
    for (ordinal, payload) in template.part_payloads().iter().enumerate() {
        assert!(payload.inertia.as_immutable().is_none());
        assert_eq!(
            payload.inertia.try_read().unwrap(),
            body.parts[ordinal].mass.inertia
        );
        assert_eq!(
            binding.inertias()[ordinal].try_read().unwrap(),
            body.parts[ordinal].mass.inertia
        );
        assert_eq!(payload.material, body.parts[ordinal].material);
    }
    // The decoy cache is never the live Part inertia owner.
    let animated_xyz: [u32; 3] =
        std::array::from_fn(|lane| body.parts[1].animated.inverse_tensor[lane].to_bits());
    assert_ne!(
        body.parts[1].mass.inertia.inverse_tensor.map(f32::to_bits),
        animated_xyz
    );
    for (ordinal, limits) in expected_limits.iter().enumerate() {
        assert!(template.joints()[ordinal].limits.as_immutable().is_none());
        assert_eq!(
            template.joints()[ordinal].limits.try_read().unwrap(),
            *limits
        );
        assert_eq!(binding.joint_limits()[ordinal].try_read().unwrap(), *limits);
    }
}

#[test]
fn actual_pool_admission_has_24_active_bodies_22_joints_and_forward_cleanup() {
    let (body, joints) = fixture();
    let weak = Arc::downgrade(&body);
    let mut session =
        try_create_pre_normal_skeleton_assembly(body, joints, normal_culling()).unwrap();
    let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    session.attach(&mut backend, 4).unwrap();
    assert!(session.is_attached());
    assert_eq!(backend.rigid_body_snapshot().counts.active, 24);
    assert_eq!(backend.joint_snapshot().counts.active, 22);
    assert!(backend.drive_snapshot().active_order.is_empty());
    assert!(weak.upgrade().is_some());
    session.detach(&mut backend).unwrap();
    assert_eq!(backend.rigid_body_snapshot().counts.active, 0);
    assert_eq!(backend.joint_snapshot().counts.active, 0);
    drop(session);
    assert!(weak.upgrade().is_none());
}

#[test]
fn actual_part_and_joint_capacity_failures_roll_back_the_same_session_transaction() {
    for (body_capacity, joint_capacity, expected) in [
        (
            23,
            22,
            DynamicAssemblyAttachError::PartUnavailable { ordinal: 23 },
        ),
        (
            24,
            21,
            DynamicAssemblyAttachError::JointUnavailable { ordinal: 21 },
        ),
    ] {
        let (body, joints) = fixture();
        let mut session =
            try_create_pre_normal_skeleton_assembly(body, joints, normal_culling()).unwrap();
        let mut backend =
            SkeletonPreNormalSimulationBackend::try_new(body_capacity, joint_capacity, 0).unwrap();
        assert_eq!(session.attach(&mut backend, 4).unwrap_err(), expected);
        assert!(!session.is_attached());
        assert_eq!(backend.rigid_body_snapshot().counts.active, 0);
        assert_eq!(backend.joint_snapshot().counts.active, 0);
    }
}

#[test]
fn malformed_definition_state_is_refused_without_filling_native_unwritten_storage() {
    let (body, mut joints) = fixture();
    joints.joints.pop();
    assert!(matches!(
        try_create_pre_normal_skeleton_assembly(body, joints, normal_culling()),
        Err(SkeletonAssemblyBindingError::UninitializedJointAllocation {
            allocated: 22,
            initialized: 21
        })
    ));
    let (body, mut joints) = fixture();
    joints.allocated_count = 0;
    assert!(matches!(
        try_create_pre_normal_skeleton_assembly(body, joints, normal_culling()),
        Err(SkeletonAssemblyBindingError::JointAllocationMismatch {
            body: 22,
            joints: 0
        })
    ));
    let (mut body, joints) = fixture();
    let mass = &mut Arc::get_mut(&mut body).unwrap().parts[3].mass;
    mass.has_inverse_body_transform = true;
    mass.inverse_body_transform_storage = None;
    assert!(matches!(
        try_create_pre_normal_skeleton_assembly(body, joints, normal_culling()),
        Err(SkeletonAssemblyBindingError::MissingInverseBodyFrame { part: 3 })
    ));
}

fn normal_culling() -> Arc<SkeletonCullingResource> {
    SkeletonCullingInventory::default()
        .try_get_or_create(SkeletonCullingKey::Normal)
        .unwrap()
}

#[test]
fn binding_retains_exact_normal_inventory_resource_and_refuses_ragdoll() {
    let mut inventory = SkeletonCullingInventory::default();
    let normal = inventory
        .try_get_or_create(SkeletonCullingKey::Normal)
        .unwrap();
    let weak = Arc::downgrade(&normal);
    let (body, joints) = fixture();
    let session =
        try_create_pre_normal_skeleton_assembly(body, joints, Arc::clone(&normal)).unwrap();
    assert!(Arc::ptr_eq(
        session.template().collision().initial_culling(),
        &normal
    ));
    let ragdoll = inventory
        .try_get_or_create(SkeletonCullingKey::Ragdoll)
        .unwrap();
    let (body, joints) = fixture();
    assert!(matches!(
        try_create_pre_normal_skeleton_assembly(body, joints, ragdoll),
        Err(SkeletonAssemblyBindingError::WrongCullingResource {
            actual: SkeletonCullingKey::Ragdoll
        })
    ));
    drop(inventory);
    drop(normal);
    assert!(weak.upgrade().is_some());
    drop(session);
    assert!(weak.upgrade().is_none());
}
