//! Original retained pair, selected controller entry and checked failure prefix.
//! This deliberate controller-only schedule does not stand in for `WipeoutGround`.
use super::*;
use crate::{
    input_cvar_runtime::InputCvarReferences, skeleton_runtime_state::SkeletonPartMaterialSelection,
};
use std::sync::Arc;

#[expect(
    clippy::too_many_lines,
    reason = "one original same-pair controller failure and completion proof"
)]
pub(crate) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    refs: &InputCvarReferences,
) {
    let owner = world
        .skaters
        .as_mut()
        .unwrap()
        .try_with_jump_pair(&world.backend, board, skeleton, |_, owner| {
            Arc::clone(owner)
        })
        .unwrap();
    let requests_before = owner.wipeout().lock().unwrap().requests_initialized();
    let ledger_before = owner.wipeout().lock().unwrap().request_snapshot();
    let wipeout_before = owner.wipeout().lock().unwrap().state_bits();
    let binding = Arc::clone(&world.skeleton(skeleton).unwrap().state().binding);
    let culling = Arc::clone(
        world
            .skaters
            .as_ref()
            .unwrap()
            .skeleton_construction(skeleton)
            .unwrap()
            .unwrap()
            .ragdoll_culling()
            .unwrap(),
    );
    let before = world.backend.rigid_body_snapshot();
    let before_materials = world.skeleton(skeleton).unwrap().state().material_selection;
    let before_inertias: [_; 24] =
        std::array::from_fn(|part| binding.inertias()[part].try_read().unwrap());
    assert!(refs.try_use_ragdoll_joint_limits().unwrap());
    let failure = world
        .try_pair_skeleton_ragdoll_collision(board, skeleton, refs, &mut |_| {
            Err::<f32, _>("cosine discriminator")
        })
        .unwrap_err();
    assert!(failure.to_string().contains("cosine discriminator"));
    assert_eq!(
        owner
            .skeleton_controller()
            .lock()
            .unwrap()
            .current()
            .unwrap()
            .value,
        8
    );
    assert_eq!(world.backend.rigid_body_snapshot(), before);
    assert_eq!(
        world.skeleton(skeleton).unwrap().state().material_selection,
        before_materials
    );
    world
        .try_pair_skeleton_ragdoll_collision(board, skeleton, refs, &mut |angle| {
            Ok::<f32, std::convert::Infallible>(angle.cos())
        })
        .unwrap();
    let normal = world.skeleton(skeleton).unwrap();
    let state = normal.state();
    assert!(Arc::ptr_eq(&state.binding, &binding));
    assert!(Arc::ptr_eq(&state.current_culling, &culling));
    assert_eq!(state.assembly_group, 7);
    assert_eq!(state.part_user_data, [7; 24]);
    assert!(state.is_ragdoll);
    assert_eq!(state.volume_disable_count_active, Some(false));
    assert_eq!(state.material_selection[0], before_materials[0]);
    for part in 1..24 {
        assert_eq!(state.disable_counts[part], Some(0));
        assert_eq!(state.volume_headers[part].flags & 1, 1);
        assert_eq!(
            state.material_selection[part],
            if [1, 3, 7].contains(&part) {
                SkeletonPartMaterialSelection::RagdollHead
            } else {
                SkeletonPartMaterialSelection::RagdollBody
            }
        );
    }
    let attributes = owner.inputs().vault.instance(
        crate::skeleton_body_attributes::WIPEOUT_CLASS,
        crate::skeleton_body_attributes::DEFAULT_COLLECTION,
    );
    let enabled = |key| {
        attributes
            .try_get_data(key, 0)
            .unwrap()
            .is_some_and(|data| data.bytes()[0] != 0)
    };
    let mass_enabled = enabled(0xf189_9bbc_1605_0b02);
    let tensor_enabled = enabled(0xd6de_5267_3515_1b3f);
    let drag = |key| {
        f32::from_bits(attributes.try_scalar_word(key, 0).unwrap().unwrap_or(0))
            * f32::from_bits(0x426f_ffff)
    };
    for (part, old) in before_inertias.iter().enumerate() {
        let inertia = binding.inertias()[part].try_read().unwrap();
        let cache = binding.body().parts[part].ragdoll;
        let expected_mass = if part != 0 && mass_enabled {
            cache.inverse_mass
        } else {
            old.inverse_mass
        };
        let expected_tensor = if part != 0 && tensor_enabled {
            [
                cache.inverse_tensor[0],
                cache.inverse_tensor[1],
                cache.inverse_tensor[2],
            ]
        } else {
            old.inverse_tensor
        };
        assert_eq!(inertia.inverse_mass.to_bits(), expected_mass.to_bits());
        assert_eq!(
            inertia.inverse_tensor.map(f32::to_bits),
            expected_tensor.map(f32::to_bits)
        );
        assert_eq!(
            inertia.angular_drag.to_bits(),
            drag(0xdc15_94a6_5b48_6273).to_bits()
        );
        assert_eq!(
            inertia.linear_drag.to_bits(),
            drag(0x8a35_2071_b894_3245).to_bits()
        );
    }
    // Controller-only ragdoll does not initialize/clear the request ledger or
    // advance Wipeout state, regardless of the preceding Update2 schedule.
    assert_eq!(
        owner.wipeout().lock().unwrap().requests_initialized(),
        requests_before
    );
    assert_eq!(owner.wipeout().lock().unwrap().state_bits(), wipeout_before);
    assert_eq!(
        owner.wipeout().lock().unwrap().request_snapshot(),
        ledger_before
    );
}
