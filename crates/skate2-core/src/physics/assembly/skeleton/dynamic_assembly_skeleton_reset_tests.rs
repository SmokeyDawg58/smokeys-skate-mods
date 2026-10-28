//! Controlled reset scheduling over actual admitted bodies and retained owners.
#![expect(
    clippy::float_cmp,
    reason = "unchanged retained fixture fields are exact stores"
)]
use super::*;
use crate::{
    dynamic_assembly_session::skeleton_collision_publication::tests::fixture,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_physics_history::{HostSkeletonPhysicsHistoryNumeric, SkeletonHistoryInput},
};
use std::sync::Arc;

fn map(owner: &mut SkeletonNormalAssembly, count: usize) {
    let data = Arc::clone(&owner.state.binding.body().data);
    let mut pose = *data.physics_pose();
    for matrix in &mut pose {
        matrix[12] += 100.0;
        matrix[13] += 20.0;
    }
    let result = owner
        .animation_targets
        .try_map_current_pose(&data, &pose[..count]);
    assert_eq!(result.is_ok(), count == 24);
}

struct HistoryInput;
impl SkeletonHistoryInput for HistoryInput {
    type Error = Infallible;
    type Up = ();
    fn acquire_up_vector(&mut self) -> Result<(), Infallible> {
        Ok(())
    }
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Infallible> {
        Ok(1)
    }
    fn read_up_vector(&mut self, (): &()) -> Result<[f32; 4], Infallible> {
        panic!("XYZ COM does not sample Up")
    }
}

fn seed_history(owner: &mut SkeletonNormalAssembly) {
    owner.physics_history.try_initialize_masses().unwrap();
    let mut matrix = FRESH_VOLUME_TRANSFORM;
    matrix[12..].copy_from_slice(&[17.0, 23.0, 31.0, 41.0]);
    owner
        .physics_history
        .try_update(
            matrix,
            &(),
            &mut |_| Ok::<_, Infallible>(matrix),
            &mut HistoryInput,
            &mut HostSkeletonPhysicsHistoryNumeric,
        )
        .unwrap();
    owner.state.proxy_volume_target = Some(
        crate::skeleton_root_transforms::SkeletonRootVector::known([101.0, 102.0, 103.0, 104.0]),
    );
}

#[test]
fn missing_first_mapping_leaves_bodies_history_and_proxy_tail_untouched() {
    let mut owner = fixture(None);
    let mut backend = SkateboardSimulationBackend::try_new(24, 22, 0).unwrap();
    owner.attach(&mut backend).unwrap();
    seed_history(&mut owner);
    let bodies = backend.rigid_body_snapshot();
    let transforms = *owner.physics_history.record().transforms();
    let proxy = owner.state.proxy_volume_target;
    assert_eq!(
        owner.try_reset_skeleton(&mut backend),
        Err(SkeletonResetError::MissingMappedVolume { part: 0 })
    );
    assert_eq!(backend.rigid_body_snapshot(), bodies);
    assert_eq!(*owner.physics_history.record().transforms(), transforms);
    assert_eq!(owner.state.proxy_volume_target, proxy);
    owner.detach(&mut backend).unwrap();
}

#[test]
fn detached_parts_skip_body_motion_and_history_reads_the_updated_caches() {
    let mut owner = fixture(None);
    let mut backend = SkateboardSimulationBackend::try_new(24, 22, 0).unwrap();
    seed_history(&mut owner);
    map(&mut owner, 24);
    let before = backend.rigid_body_snapshot();
    owner.try_reset_skeleton(&mut backend).unwrap();
    assert_eq!(backend.rigid_body_snapshot(), before);
    for part in 0..24 {
        let mapped = owner.animation_targets.volume_row(part).unwrap();
        let actual = owner.physics_history.record().transforms()[part];
        for column in 0..4 {
            assert_eq!(
                actual[column * 4..column * 4 + 3],
                mapped[column * 4..column * 4 + 3]
            );
            assert_eq!(actual[column * 4 + 3].to_bits(), 0);
        }
        assert_eq!(
            owner.physics_history.record().velocities()[part].map(f32::to_bits),
            [0; 4]
        );
    }
    assert_eq!(
        owner
            .state
            .proxy_volume_target
            .unwrap()
            .known_lanes()
            .expect("known controlled proxy lanes")
            .map(f32::to_bits),
        [0; 4]
    );
}

#[test]
fn partial_mapping_resets_earlier_bodies_before_missing_row_but_never_starts_history() {
    let mut owner = fixture(None);
    let mut backend = SkateboardSimulationBackend::try_new(24, 22, 0).unwrap();
    owner.attach(&mut backend).unwrap();
    seed_history(&mut owner);
    map(&mut owner, 2);
    backend.set_gravity([3.0, -5.0, 7.0]);
    let before = backend.rigid_body_snapshot();
    let transforms = *owner.physics_history.record().transforms();
    let proxy = owner.state.proxy_volume_target;
    assert_eq!(
        owner.try_reset_skeleton(&mut backend),
        Err(SkeletonResetError::MissingMappedVolume { part: 2 })
    );
    let after = backend.rigid_body_snapshot();
    for (part, attachment) in owner.session.part_attachment.parts.iter().enumerate() {
        let index = usize::try_from(attachment.body.unwrap().id().index()).unwrap();
        if part < 2 {
            assert_ne!(
                after.slots[index].center_of_mass,
                before.slots[index].center_of_mass
            );
            assert_eq!(
                after.slots[index].force.map(f32::to_bits),
                [3.0, -5.0, 7.0].map(f32::to_bits)
            );
        } else {
            assert_eq!(after.slots[index], before.slots[index]);
        }
    }
    assert_eq!(*owner.physics_history.record().transforms(), transforms);
    assert_eq!(owner.state.proxy_volume_target, proxy);
    owner.detach(&mut backend).unwrap();
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one actual reset and retained-owner comparison"
)]
fn complete_reset_rereads_live_parts_and_preserves_nonreset_owners_and_history() {
    for hat in [None, Some(false)] {
        let mut owner = fixture(hat);
        let mut backend = SkateboardSimulationBackend::try_new(24, 22, 0).unwrap();
        owner.attach(&mut backend).unwrap();
        seed_history(&mut owner);
        owner.initialize_jump_adjustments_constructor().unwrap();
        owner
            .store_jump_adjustment_x(f32::from_bits(0x8000_0000))
            .unwrap();
        owner
            .store_jump_adjustment_z(f32::from_bits(0x7fc1_2345))
            .unwrap();
        map(&mut owner, 24);
        backend.set_gravity([3.0, -5.0, 7.0]);
        backend.begin_dynamic_update_frame(0.25);
        backend
            .try_integrate_active_bodies_ordinary_finite()
            .unwrap();
        let before = backend.rigid_body_snapshot();
        let joints = backend.joint_snapshot();
        let drives = backend.drive_snapshot();
        let deltas = *owner.physics_history.record().velocity_deltas();
        let com = owner.physics_history.record().centre_of_mass();
        let com_velocity = owner.physics_history.record().centre_of_mass_velocity();
        let proxy_position = owner.physics_history.record().proxy_volume_position();
        let masses = *owner.physics_history.part_masses();
        let fractions = *owner.physics_history.fractional_part_masses();
        let timestep = owner.physics_history.time_delta();
        let poses = *owner.current_animation.record().poses();
        owner.try_reset_skeleton(&mut backend).unwrap();
        assert_eq!(owner.jump_adjustment_x().unwrap().to_bits(), 0x8000_0000);
        assert_eq!(owner.jump_adjustment_z().unwrap().to_bits(), 0x7fc1_2345);
        let after = backend.rigid_body_snapshot();
        for (part, attachment) in owner.session.part_attachment.parts.iter().enumerate() {
            let index = usize::try_from(attachment.body.unwrap().id().index()).unwrap();
            let old = &before.slots[index];
            let next = &after.slots[index];
            assert_eq!(next.velocity.map(f32::to_bits), [0; 3]);
            assert_eq!(next.omega.map(f32::to_bits), [0; 3]);
            assert_eq!(
                next.force.map(f32::to_bits),
                [3.0, -5.0, 7.0].map(f32::to_bits)
            );
            assert_eq!(next.torque.map(f32::to_bits), [0; 3]);
            assert_eq!(
                (
                    next.id,
                    next.occurrence,
                    next.right_link,
                    next.left_link,
                    next.stasis
                ),
                (
                    old.id,
                    old.occurrence,
                    old.right_link,
                    old.left_link,
                    old.stasis
                )
            );
            assert_eq!(
                next.kinematic_value.to_bits(),
                old.kinematic_value.to_bits()
            );
            assert_eq!(
                (next.cooldown, next.state, next.tag, next.membership),
                (old.cooldown, old.state, old.tag, old.membership)
            );
            let actual = owner
                .session
                .try_live_part_transform_host(&backend, part)
                .unwrap();
            assert_eq!(
                owner.physics_history.record().transforms()[part].map(f32::to_bits),
                actual.map(f32::to_bits)
            );
            assert_eq!(
                owner.physics_history.record().vector_typed(
                    crate::skeleton_physics_history::lane_knowledge::RecordVectorField::Position(
                        part
                    )
                ),
                owner
                    .session
                    .try_live_part_transform_typed(&backend, part)
                    .unwrap()
                    .columns()[3]
            );
            assert_eq!(
                owner.physics_history.record().velocities()[part].map(f32::to_bits),
                [0; 4]
            );
        }
        assert_eq!(*owner.physics_history.record().velocity_deltas(), deltas);
        assert_eq!(owner.physics_history.record().centre_of_mass(), com);
        assert_eq!(
            owner.physics_history.record().centre_of_mass_velocity(),
            com_velocity
        );
        assert_eq!(
            owner.physics_history.record().proxy_volume_position(),
            proxy_position
        );
        assert_eq!(*owner.physics_history.part_masses(), masses);
        assert_eq!(*owner.physics_history.fractional_part_masses(), fractions);
        assert_eq!(
            owner.physics_history.time_delta().to_bits(),
            timestep.to_bits()
        );
        assert_eq!(*owner.current_animation.record().poses(), poses);
        assert_eq!(
            owner
                .state
                .proxy_volume_target
                .unwrap()
                .known_lanes()
                .expect("known controlled proxy lanes")
                .map(f32::to_bits),
            [0; 4]
        );
        assert_eq!(backend.joint_snapshot(), joints);
        assert_eq!(backend.drive_snapshot(), drives);
        owner.detach(&mut backend).unwrap();
    }
}

#[test]
fn foreign_body_on_later_part_preserves_previous_resets_and_blocks_history_and_tail() {
    let mut owner = fixture(None);
    let mut backend = SkateboardSimulationBackend::try_new(24, 22, 0).unwrap();
    owner.attach(&mut backend).unwrap();
    seed_history(&mut owner);
    map(&mut owner, 24);
    let mut other = fixture(None);
    let mut foreign = SkateboardSimulationBackend::try_new(24, 22, 0).unwrap();
    other.attach(&mut foreign).unwrap();
    let saved = owner.session.part_attachment.parts[2].body;
    owner.session.part_attachment.parts[2].body = other.session.part_attachment.parts[2].body;
    backend.set_gravity([13.0, 17.0, 19.0]);
    let before = backend.rigid_body_snapshot();
    let history = *owner.physics_history.record().transforms();
    let proxy = owner.state.proxy_volume_target;
    assert!(matches!(
        owner.try_reset_skeleton(&mut backend),
        Err(SkeletonResetError::Part {
            part: 2,
            source: DynamicAssemblyPartTransformError::Body(
                SimulationRigidBodyPoolError::ForeignHandle
            ),
        })
    ));
    owner.session.part_attachment.parts[2].body = saved;
    let after = backend.rigid_body_snapshot();
    for (part, attachment) in owner.session.part_attachment.parts.iter().enumerate() {
        let index = usize::try_from(attachment.body.unwrap().id().index()).unwrap();
        if part < 2 {
            assert_eq!(
                after.slots[index].force.map(f32::to_bits),
                [13.0, 17.0, 19.0].map(f32::to_bits)
            );
        } else {
            assert_eq!(after.slots[index], before.slots[index]);
        }
    }
    assert_eq!(*owner.physics_history.record().transforms(), history);
    assert_eq!(owner.state.proxy_volume_target, proxy);
    owner.detach(&mut backend).unwrap();
    other.detach(&mut foreign).unwrap();
}
