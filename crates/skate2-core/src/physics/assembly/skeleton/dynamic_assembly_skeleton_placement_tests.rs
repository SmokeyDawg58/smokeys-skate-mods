//! Retained source-producer pose placement into actual Part/body owners.
//! Kernel routing checks reuse separately source-tested host math, not Xenon
//! reference trace words or a substitute live animation/spawn implementation.
use super::*;
use crate::{
    dynamic_assembly_session::{
        part_transform::DynamicAssemblyPartTransformError,
        skeleton_collision_publication::tests::fixture,
    },
    skateboard_spawn_math::skateboard_spawn_compose,
    world::{
        orthonormalize_collision_affine, part_attachment_transform, part_get_transform,
        simulation_quaternion_from_orthonormalized_matrix_host,
        simulation_rotated_inverse_inertia_host,
    },
};

fn requested_parts(owner: &SkeletonNormalAssembly) -> [[f32; 16]; 24] {
    let data = &owner.session().template().collision().body().data;
    std::array::from_fn(|part| {
        orthonormalize_collision_affine(skateboard_spawn_compose(
            &data.physics_pose()[part],
            &data.bone_to_volume()[part],
        ))
    })
}

/// Shared by mounted hats0/4/1: actual source data and current inertia owners,
/// not regenerated bodies or guessed frame defaults. Placement refreshes body
/// pose/derived tensor caches while preserving all other snapshot fields.
#[expect(
    clippy::too_many_lines,
    reason = "one actual owner placement and complete unchanged-field comparison"
)]
pub(crate) fn check_retained_pose_placement(
    owner: &mut SkeletonNormalAssembly,
    backend: &mut SkeletonPreNormalSimulationBackend,
) {
    let requested = requested_parts(owner);
    let binding = Arc::clone(owner.session().template().collision());
    let handles: Vec<_> = owner
        .session
        .part_attachment
        .parts
        .iter()
        .map(|part| part.body.unwrap())
        .collect();
    let before_bodies = backend.rigid_body_snapshot();
    let before_joints = backend.joint_snapshot();
    let before_drives = backend.drive_snapshot();
    let packed: Vec<_> = binding
        .inertias()
        .iter()
        .map(|owner| owner.try_read_packed().unwrap().unwrap().map(f32::to_bits))
        .collect();
    let rf: Vec<_> = handles
        .iter()
        .map(|handle| {
            backend
                .rigid_body_pool_for_test()
                .try_with_solver_delta_row(handle.observation(), |row| *row)
                .unwrap()
        })
        .collect();
    owner
        .try_place_parts_from_retained_physics_pose(backend)
        .unwrap();
    let after_bodies = backend.rigid_body_snapshot();
    let mut expected = before_bodies;
    for (part, handle) in handles.iter().enumerate() {
        let mass = &binding.body().parts[part].mass;
        let inverse = mass
            .has_inverse_body_transform
            .then(|| mass.inverse_body_transform_storage.as_ref().unwrap());
        let normalized =
            orthonormalize_collision_affine(part_attachment_transform(requested[part], inverse));
        let value = binding.inertias()[part].try_read().unwrap();
        let tensor = simulation_rotated_inverse_inertia_host(
            normalized,
            value.inverse_tensor,
            value.inverse_mass,
        );
        let slot = &mut expected.slots[usize::try_from(handle.id().index()).unwrap()];
        slot.right = [normalized[0], normalized[1], normalized[2]];
        slot.up = [normalized[4], normalized[5], normalized[6]];
        slot.at = [normalized[8], normalized[9], normalized[10]];
        slot.center_of_mass = [normalized[12], normalized[13], normalized[14]];
        slot.quaternion = simulation_quaternion_from_orthonormalized_matrix_host(normalized);
        slot.inverse_inertia_full = tensor.full;
        slot.inverse_inertia_split = tensor.split;
        slot.inverse_mass = tensor.inverse_mass;
        assert_eq!(
            owner
                .session()
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .nth(part)
                .unwrap()
                .map(f32::to_bits),
            requested[part].map(f32::to_bits)
        );
        let mut canonical = normalized;
        canonical[3] = 0.0;
        canonical[7] = 0.0;
        canonical[11] = 0.0;
        canonical[15] = 1.0;
        assert_eq!(
            backend
                .try_part_body_pose(*handle)
                .unwrap()
                .map(f32::to_bits),
            canonical.map(f32::to_bits)
        );
        assert_eq!(
            owner
                .session()
                .try_live_part_transform_host(backend, part)
                .unwrap()
                .map(f32::to_bits),
            part_get_transform(requested[part], Some(&canonical), inverse).map(f32::to_bits)
        );
        assert_eq!(
            binding.inertias()[part]
                .try_read_packed()
                .unwrap()
                .unwrap()
                .map(f32::to_bits),
            packed[part]
        );
        assert_eq!(
            backend
                .rigid_body_pool_for_test()
                .try_with_solver_delta_row(handle.observation(), |row| *row)
                .unwrap(),
            rf[part]
        );
    }
    assert_eq!(
        after_bodies, expected,
        "no changed motion/force/tag/occurrence/rings/inertia owner"
    );
    assert_eq!(backend.joint_snapshot(), before_joints);
    assert_eq!(backend.drive_snapshot(), before_drives);
    assert!(Arc::ptr_eq(
        owner.session().template().collision(),
        &binding
    ));
}

#[test]
fn retained_pose_places_all_live_parts_with_and_without_hat_inverse_mapping() {
    for hat in [None, Some(false)] {
        let mut owner = fixture(hat);
        let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
        owner.attach(&mut backend).unwrap();
        // Existing real integration supplies nonzero motion for preservation,
        // not a replacement pose or a claimed constrained physics tick.
        backend.set_gravity([3.0, -5.0, 7.0]);
        backend.begin_dynamic_update_frame(0.25);
        backend
            .try_integrate_active_bodies_ordinary_finite()
            .unwrap();
        assert!(
            backend
                .rigid_body_snapshot()
                .slots
                .iter()
                .any(|slot| slot.velocity.iter().any(|value| value.abs() > 0.0))
        );
        let initial: Vec<_> = owner
            .session()
            .part_transforms()
            .map(|matrix| matrix.expect("known fixture Part cache"))
            .map(|matrix| matrix.map(f32::to_bits))
            .collect();
        check_retained_pose_placement(&mut owner, &mut backend);
        assert_ne!(
            owner
                .session()
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .next()
                .unwrap()
                .map(f32::to_bits),
            initial[0]
        );
        assert_eq!(
            owner
                .session()
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .next()
                .unwrap()[12]
                .to_bits(),
            7.5_f32.to_bits()
        );
        if hat.is_some() {
            assert!(
                owner.session().template().collision().body().parts[1]
                    .mass
                    .has_inverse_body_transform
            );
        }
        owner.detach(&mut backend).unwrap();
    }
}

#[test]
fn detached_and_foreign_placement_refuse_without_any_cache_or_body_writes() {
    let mut owner = fixture(None);
    let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    let initial: Vec<_> = owner
        .session()
        .part_transforms()
        .map(|matrix| matrix.expect("known fixture Part cache"))
        .copied()
        .collect();
    let before = backend.rigid_body_snapshot();
    assert!(matches!(
        owner.try_place_parts_from_retained_physics_pose(&mut backend),
        Err(SkeletonPlacementError::Detached)
    ));
    assert_eq!(
        owner
            .session()
            .part_transforms()
            .map(|matrix| matrix.expect("known fixture Part cache"))
            .copied()
            .collect::<Vec<_>>(),
        initial
    );
    assert_eq!(backend.rigid_body_snapshot(), before);
    owner.attach(&mut backend).unwrap();
    let before = backend.rigid_body_snapshot();
    let mut foreign = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    let foreign_before = foreign.rigid_body_snapshot();
    assert!(matches!(
        owner.try_place_parts_from_retained_physics_pose(&mut foreign),
        Err(SkeletonPlacementError::ForeignBackend)
    ));
    assert_eq!(
        owner
            .session()
            .part_transforms()
            .map(|matrix| matrix.expect("known fixture Part cache"))
            .copied()
            .collect::<Vec<_>>(),
        initial
    );
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!(foreign.rigid_body_snapshot(), foreign_before);
    owner.detach(&mut backend).unwrap();
}

#[test]
fn later_foreign_body_refusal_retains_only_completed_ascending_part_prefix() {
    let mut owner = fixture(None);
    let mut backend = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    owner.attach(&mut backend).unwrap();
    let mut other = fixture(None);
    let mut foreign = SkeletonPreNormalSimulationBackend::try_new(24, 22, 0).unwrap();
    other.attach(&mut foreign).unwrap();
    let requested = requested_parts(&owner);
    let before = backend.rigid_body_snapshot();
    let caches: Vec<_> = owner
        .session()
        .part_transforms()
        .map(|matrix| matrix.expect("known fixture Part cache"))
        .map(|matrix| matrix.map(f32::to_bits))
        .collect();
    let joints = backend.joint_snapshot();
    // Test-only corruption uses another actually issued body; no production
    // capability fabrication or public handle setter is introduced.
    let original = owner.session.part_attachment.parts[2].body;
    owner.session.part_attachment.parts[2].body = other.session.part_attachment.parts[2].body;
    assert!(matches!(
        owner.try_place_parts_from_retained_physics_pose(&mut backend),
        Err(SkeletonPlacementError::Part {
            part: 2,
            source: DynamicAssemblyPartTransformError::Body(
                SimulationRigidBodyPoolError::ForeignHandle
            )
        })
    ));
    owner.session.part_attachment.parts[2].body = original;
    for part in 0..24 {
        let expected = if part < 2 {
            requested[part].map(f32::to_bits)
        } else {
            caches[part]
        };
        assert_eq!(
            owner
                .session()
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .nth(part)
                .unwrap()
                .map(f32::to_bits),
            expected
        );
    }
    assert_eq!(
        &backend.rigid_body_snapshot().slots[2..],
        &before.slots[2..]
    );
    assert_eq!(backend.joint_snapshot(), joints);
    check_retained_pose_placement(&mut owner, &mut backend);
    owner.detach(&mut backend).unwrap();
    other.detach(&mut foreign).unwrap();
}
