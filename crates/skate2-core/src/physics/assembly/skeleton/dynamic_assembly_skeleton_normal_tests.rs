//! Controlled fixtures over real admitted owners, not a captured native Tick.
use super::*;
use crate::{
    skeleton_body_attributes::SkeletonBodyDebugRead,
    skeleton_culling::{SkeletonCullingInventory, SkeletonCullingKey},
    skeleton_runtime_state::{SkeletonPartMaterialSelection, tests::CollisionReader},
};

struct Reader {
    result: Result<u8, &'static str>,
    calls: usize,
}
impl SkeletonBodyDebugRead for Reader {
    type Error = &'static str;
    fn mode(&mut self) -> Result<u32, Self::Error> {
        panic!("normal setup creates no debug instance")
    }
    fn factor(&mut self) -> Result<f32, Self::Error> {
        panic!("normal setup fetches no debug factor")
    }
}
impl SkeletonBodyAttributeRead for Reader {
    type Error = &'static str;
    type DebugInstance = Self;
    fn float(&mut self, _: SkeletonBodyAttributeRoute, _: u64) -> Result<f32, Self::Error> {
        panic!("normal setup has no new float FETCH")
    }
    fn byte(&mut self, route: SkeletonBodyAttributeRoute, key: u64) -> Result<u8, Self::Error> {
        assert_eq!(route, SkeletonBodyAttributeRoute::LocalSkeleton);
        assert_eq!(key, NORMAL_COLLISION_ENABLED);
        self.calls += 1;
        self.result
    }
    fn debug_instance(&mut self) -> Result<Self, Self::Error> {
        panic!("no debug instance")
    }
}

pub(crate) fn fixture() -> SkeletonNormalAssembly {
    let (mut body, joints) = crate::skeleton_assembly::tests::fixture();
    // Explicit source-field discriminators, not original Vault tuning.
    let definition = Arc::get_mut(&mut body).unwrap();
    definition.parts[1].animated.inverse_mass = 0.125;
    definition.parts[1].animated.inverse_tensor = [2.0, 4.0, 8.0, -0.0];
    let attributes = Arc::get_mut(&mut definition.attributes).unwrap();
    attributes.drives[5].collision_compliant = 0xfe;
    attributes.drives[5].collision_priority = 17.0;
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

#[expect(
    clippy::unnecessary_wraps,
    reason = "implements the fallible numeric service signature"
)]
fn cosine(value: f32) -> Result<f32, std::convert::Infallible> {
    Ok(value.cos())
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one actual admission, normal setup and teardown ownership proof"
)]
fn normal_setup_writes_actual_owners_without_refreshing_cached_body_state() {
    let mut skeleton = fixture();
    let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    skeleton.attach(&mut backend).unwrap();
    assert_eq!(skeleton.state.assembly_group, 1);
    assert_eq!(skeleton.state.part_user_data, [1; 24]);
    skeleton
        .try_set_pre_normal_properties(&mut backend)
        .unwrap();
    assert!(
        backend
            .joint_snapshot()
            .slots
            .iter()
            .all(|slot| slot.spy == 1 && slot.tag == 0)
    );
    assert_eq!(
        skeleton.state.material_selection[0],
        SkeletonPartMaterialSelection::Zero
    );
    let before_bodies = backend.rigid_body_snapshot();
    let before_joints = backend.joint_snapshot();
    let binding = Arc::clone(skeleton.session.template().collision());
    let root = binding.inertias()[0].try_read_packed().unwrap().unwrap();
    let mut reader = Reader {
        result: Ok(0xfe),
        calls: 0,
    };
    skeleton
        .try_set_up_normal(&mut backend, &mut reader, &mut cosine)
        .unwrap();
    assert_eq!(reader.calls, 1);
    assert_eq!(skeleton.state.assembly_group, 6);
    assert_eq!(skeleton.state.part_user_data, [6; 24]);
    assert_eq!(skeleton.state.volume_headers[0].group_id, 4);
    assert_eq!(skeleton.state.volume_headers[0].flags & 1, 0);
    assert_eq!(skeleton.state.disable_counts[0], Some(0));
    assert!(
        skeleton.state.disable_counts[1..]
            .iter()
            .all(Option::is_none)
    );
    assert_eq!(skeleton.state.volume_disable_count_active, None);
    assert_eq!(skeleton.state.collision.is_compliant[0], 1);
    assert_eq!(skeleton.state.collision.is_compliant[5], 0xfe);
    assert_eq!(
        skeleton.state.collision.priority[5].to_bits(),
        17.0_f32.to_bits()
    );
    assert!(!skeleton.state.is_ragdoll);
    assert_eq!(
        binding.inertias()[0]
            .try_read_packed()
            .unwrap()
            .unwrap()
            .map(f32::to_bits),
        root.map(f32::to_bits)
    );
    let mut expected = before_bodies;
    for (part, owner) in binding.inertias().iter().enumerate() {
        let current = owner.try_read().unwrap();
        assert_eq!(current.angular_drag.to_bits(), 0);
        assert_eq!(current.linear_drag.to_bits(), 0);
        if part != 0 {
            let cache = binding.body().parts[part].animated;
            assert_eq!(current.inverse_mass.to_bits(), cache.inverse_mass.to_bits());
            assert_eq!(
                owner.try_read_packed().unwrap().unwrap().map(f32::to_bits),
                cache.inverse_tensor.map(f32::to_bits)
            );
            assert_eq!(
                skeleton.state.material_selection[part],
                SkeletonPartMaterialSelection::Normal
            );
            assert!(std::ptr::eq(
                skeleton.state.material(part),
                skeleton.state.material(1)
            ));
        }
        expected.slots[part].inertia = Some(current);
    }
    assert_eq!(backend.rigid_body_snapshot(), expected);
    assert_eq!(backend.joint_snapshot(), before_joints);
    assert_eq!(
        binding.inertias()[1]
            .try_read()
            .unwrap()
            .spherical
            .to_bits(),
        0.5_f32.to_bits()
    );
    assert_eq!(
        skeleton.attach(&mut backend),
        Err(DynamicAssemblyAttachError::AlreadyAttached)
    );
    assert_eq!(skeleton.state.assembly_group, 6);
    let compliance = skeleton.state.collision.is_compliant;
    let headers = skeleton.state.volume_headers;
    skeleton.teleport_reset();
    assert_eq!(skeleton.state.collision.is_compliant, compliance);
    assert_eq!(skeleton.state.collision.priority.map(f32::to_bits), [0; 24]);
    assert_eq!(skeleton.state.disable_counts, [Some(0); 24]);
    assert_eq!(skeleton.state.volume_disable_count_active, Some(false));
    assert_eq!(skeleton.state.volume_headers, headers);
    skeleton.detach(&mut backend).unwrap();
    assert_eq!(backend.rigid_body_snapshot().counts.free, 24);
    assert_eq!(backend.joint_snapshot().counts.free, 22);
}

#[test]
fn cosine_failure_keeps_limit_prefix_before_any_normal_state_effects() {
    let mut skeleton = fixture();
    let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    skeleton.attach(&mut backend).unwrap();
    let before_collision = skeleton.state.collision.clone();
    let before_bodies = backend.rigid_body_snapshot();
    let mut reader = Reader {
        result: Ok(1),
        calls: 0,
    };
    let error = skeleton
        .try_set_up_normal(&mut backend, &mut reader, &mut |_| Err::<f32, _>("cosine"))
        .unwrap_err();
    assert!(matches!(
        error,
        SkeletonNormalSetupError::Limits(SkeletonNormalJointLimitsError::Cosine {
            row: 0,
            source: "cosine",
            ..
        })
    ));
    assert_eq!(reader.calls, 0);
    assert_eq!(skeleton.state.assembly_group, 1);
    assert_eq!(skeleton.state.collision, before_collision);
    assert_eq!(backend.rigid_body_snapshot(), before_bodies);
    skeleton.detach(&mut backend).unwrap();
}

#[test]
fn late_attribute_failure_retains_mass_material_compliance_prefix() {
    let mut skeleton = fixture();
    let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    skeleton.attach(&mut backend).unwrap();
    skeleton.state.is_ragdoll = true;
    let before_headers = skeleton.state.volume_headers;
    let mut reader = Reader {
        result: Err("late fetch"),
        calls: 0,
    };
    assert!(matches!(
        skeleton.try_set_up_normal(&mut backend, &mut reader, &mut cosine),
        Err(SkeletonNormalSetupError::Attribute("late fetch"))
    ));
    assert_eq!(reader.calls, 1);
    assert!(skeleton.state.is_ragdoll);
    assert_eq!(skeleton.state.volume_headers, before_headers);
    assert_eq!(skeleton.state.assembly_group, 6);
    assert_eq!(skeleton.state.collision.is_compliant[0], 1);
    assert_eq!(skeleton.state.collision.is_compliant[5], 0xfe);
    assert_eq!(
        skeleton.state.collision.priority[5].to_bits(),
        17.0_f32.to_bits()
    );
    assert_eq!(
        skeleton.state.binding.inertias()[1]
            .try_read_packed()
            .unwrap()
            .unwrap()
            .map(f32::to_bits),
        [2.0, 4.0, 8.0, -0.0].map(f32::to_bits)
    );
    skeleton.detach(&mut backend).unwrap();
}

#[test]
fn detached_and_foreign_backend_refuse_before_callbacks_and_state_changes() {
    let mut skeleton = fixture();
    let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    let mut foreign = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    let mut reader = Reader {
        result: Ok(1),
        calls: 0,
    };
    assert!(matches!(
        skeleton.try_set_up_normal(&mut backend, &mut reader, &mut cosine),
        Err(SkeletonNormalSetupError::Detached)
    ));
    assert_eq!(skeleton.state.assembly_group, 0);
    skeleton.attach(&mut backend).unwrap();
    let before = backend.rigid_body_snapshot();
    assert!(matches!(
        skeleton.try_set_up_normal(&mut foreign, &mut reader, &mut cosine),
        Err(SkeletonNormalSetupError::ForeignBackend)
    ));
    assert_eq!(skeleton.state.assembly_group, 1);
    assert_eq!(reader.calls, 0);
    assert_eq!(backend.rigid_body_snapshot(), before);
    skeleton.detach(&mut backend).unwrap();
}

#[test]
fn zero_collision_byte_resets_counters_after_all_mass_stores() {
    let mut skeleton = fixture();
    let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    skeleton.attach(&mut backend).unwrap();
    let mut reader = Reader {
        result: Ok(0),
        calls: 0,
    };
    skeleton
        .try_set_up_normal(&mut backend, &mut reader, &mut cosine)
        .unwrap();
    assert_eq!(skeleton.state.disable_counts, [Some(0); 24]);
    assert!(
        skeleton
            .state
            .volume_headers
            .iter()
            .all(|header| header.flags & 1 == 0)
    );
    assert_eq!(skeleton.state.volume_disable_count_active, None);
    assert_eq!(
        skeleton.state.collision.priority[5].to_bits(),
        17.0_f32.to_bits()
    );
    skeleton.detach(&mut backend).unwrap();
}

#[test]
fn actual_packed_stores_preserve_w_and_spherical_right_ties_and_unordered_selection() {
    let mut skeleton = fixture();
    let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    skeleton.attach(&mut backend).unwrap();
    let body = skeleton.session.part_attachment.parts[1].body.unwrap();
    let owner = &skeleton.state.binding.inertias()[1];
    let before = backend.rigid_body_snapshot().slots[1].clone();
    let w = f32::from_bits(0x7fc1_2345);
    for (xyz, expected) in [
        ([-0.0, 0.0, 0.0], f32::INFINITY),
        ([0.0, -0.0, -0.0], f32::NEG_INFINITY),
        ([f32::NAN, 2.0, 3.0], 0.5),
        ([2.0, f32::NAN, 3.0], 1.0_f32 / 3.0),
    ] {
        let [x, y, z] = xyz;
        backend
            .try_set_skeleton_animated_inertia(
                body,
                owner,
                crate::skeleton_body_definition::SkeletonPartMassCache {
                    inverse_mass: 0.25,
                    inverse_tensor: [x, y, z, w],
                },
            )
            .unwrap();
        assert_eq!(
            owner.try_read().unwrap().spherical.to_bits(),
            expected.to_bits()
        );
        assert_eq!(
            owner.try_read_packed().unwrap().unwrap().map(f32::to_bits),
            [x, y, z, w].map(f32::to_bits)
        );
        let current = backend.rigid_body_snapshot().slots[1].clone();
        assert_eq!(
            current.inverse_mass.to_bits(),
            before.inverse_mass.to_bits()
        );
        assert_eq!(
            current.inverse_inertia_full.map(f32::to_bits),
            before.inverse_inertia_full.map(f32::to_bits)
        );
        assert_eq!(
            current.inverse_inertia_split.map(f32::to_bits),
            before.inverse_inertia_split.map(f32::to_bits)
        );
    }
    skeleton.detach(&mut backend).unwrap();
}

pub(crate) fn retained_animation_threshold(normal: &SkeletonNormalAssembly) -> f32 {
    normal
        .current_animation_attributes
        .as_ref()
        .unwrap()
        .skateboard_animation_threshold()
        .unwrap()
}
