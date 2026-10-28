//! Observe actual post-wipeout caller pose publication on the retained Actor.
use crate::{
    actor_construction::anim_out_phys_in::pose_lanes::PoseBuffer,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
};
use std::sync::Arc;

pub(super) fn check(
    world: &mut PhysicalAssemblyWorld,
    skeleton: &SimulationSkaterAssemblyHandle,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    old_global: &[crate::animation::AnimationMatrix],
) {
    let input = Arc::clone(
        owner
            .animation_physics_input
            .lock()
            .unwrap()
            .as_ref()
            .unwrap(),
    );
    let normal = world.skeleton(skeleton).unwrap();
    let data = &normal.state().binding.body().data;
    let bones = *data.bone_ids();
    let expected = std::array::from_fn::<_, 24, _>(|part| {
        let record = normal
            .physics_history()
            .record()
            .transform_typed(part)
            .unwrap();
        let volume =
            crate::skeleton_root_transforms::SkeletonRootMatrix::known(data.volume_to_bone()[part]);
        normal
            .root_transforms()
            .world_to_animation()
            .compose(&record.compose(&volume))
    });
    assert!(Arc::ptr_eq(
        &input,
        owner
            .animation_physics_input
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
    ));
    let pose = input.lock().unwrap();
    for (part, bone) in bones.into_iter().enumerate() {
        let actual = pose.pose_matrix_typed(PoseBuffer::Global, bone).unwrap();
        for (a, e) in actual.columns().iter().zip(expected[part].columns()) {
            assert_eq!(a.xyz.map(f32::to_bits), e.xyz.map(f32::to_bits));
            assert_eq!(a.w.map(f32::to_bits), e.w.map(f32::to_bits));
        }
    }
    for (bone, old) in old_global.iter().enumerate() {
        if !bones.contains(&bone) {
            assert_eq!(
                pose.global_pose.as_ref().unwrap()[bone].map(f32::to_bits),
                old.map(f32::to_bits)
            );
        }
    }
    assert!(
        pose.global_pose
            .as_ref()
            .unwrap()
            .iter()
            .zip(old_global)
            .any(|(a, b)| a.map(f32::to_bits) != b.map(f32::to_bits)),
        "real solved pose must replace animation input rows"
    );
    assert!(
        pose.pose_known_lanes(PoseBuffer::Global).is_ok(),
        "actual WorldToAnim inverse establishes result W independently of packed inner W"
    );
    eprintln!(
        "same-Actor post-wipeout caller completed IK/head and dispatched UpdatePoseBuffer; mapped globals and retained owner verified, native tick trace remains pending"
    );
}
