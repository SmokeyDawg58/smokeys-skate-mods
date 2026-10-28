//! Actual shared-pool ownership tests. Unit attributes deliberately exercise
//! verified missing defaults; the existing mounted case supplies original Vault
//! values. Cvars1/10 are explicit harness samples, not a native global service.
use super::*;
use crate::{
    attribute_runtime::tests::graph,
    dynamic_assembly_session::{
        DynamicAssemblyDetachError, DynamicAssemblySession,
        skater_pool::SimulationSkaterAssemblyHandle,
        skeleton_collision_publication::tests::fixture,
    },
    simulation_assembly_backend::constraint_session::SimulationAssemblyConstraintSession,
    skateboard_hook::{SkateboardHookConstruction, SkateboardHookDefinition},
    skateboard_spawn_math::skateboard_spawn_compose,
    skateboard_truck_drives::HostSkateboardTruckDriveNumeric,
    skateboard_wheels::HostSkateboardWheelMassNumeric,
    skeleton_drive_attributes::{ANIMATION_DRIVES_CLASS, SkeletonDriveAttributes},
    skeleton_drives::HostSkeletonDriveNumeric,
    world::orthonormalize_collision_affine,
};

#[derive(Default)]
struct Cvars {
    samples: Vec<&'static str>,
}
impl SkeletonDriveCvars for Cvars {
    type Error = std::convert::Infallible;
    fn hard_drive_displacement(&mut self) -> Result<f32, Self::Error> {
        self.samples.push("displacement");
        Ok(1.0)
    }
    fn hard_drive_strength(&mut self) -> Result<f32, Self::Error> {
        self.samples.push("strength");
        Ok(10.0)
    }
}

fn attributes() -> SkeletonDriveAttributes {
    // No collection means actual DefaultDataArea16/4, not guessed drive tuning.
    SkeletonDriveAttributes::bind(&Arc::new(graph(ANIMATION_DRIVES_CLASS, vec![])))
}

fn board_hook() -> SkateboardHookConstruction {
    let mut hook = SkateboardHookConstruction::new();
    hook.try_prepare(
        &mut HostSkateboardTruckDriveNumeric,
        &mut HostSkateboardWheelMassNumeric,
    )
    .unwrap();
    hook
}

fn board_hook_candidate(
    hook: &SkateboardHookConstruction,
) -> SimulationHookAssemblyCandidate<AssemblyPartMaterial, SkateboardHookDefinition> {
    SimulationHookAssemblyCandidate::try_new(
        DynamicAssemblySession::try_new(
            Arc::clone(hook.template().unwrap()),
            vec![FRESH_VOLUME_TRANSFORM],
        )
        .unwrap(),
    )
    .unwrap()
}

fn expected_hook_transforms(owner: &SkeletonNormalAssembly) -> [[f32; 16]; 2] {
    let data = &owner.session().template().collision().body().data;
    let volume = |part| {
        orthonormalize_collision_affine(skateboard_spawn_compose(
            &data.physics_pose()[part],
            &data.bone_to_volume()[part],
        ))
    };
    let hips = volume(23);
    let root = volume(0);
    let mut outer = FRESH_VOLUME_TRANSFORM;
    for lane in 0..4 {
        outer[12 + lane] = 0.0 - root[12 + lane];
    }
    [
        skateboard_spawn_compose(&outer, &hips),
        skateboard_spawn_compose(&outer, &root),
    ]
}

fn source_release_indices(first: u32) -> Vec<u32> {
    let mut expected = vec![first, first + 1];
    for part in (1..23).rev() {
        let local = first + 2 + 2 * (part - 1);
        expected.extend([local, local + 1]);
    }
    expected
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "single actual-owner lifecycle and source-order cleanup oracle"
)]
fn complete_drive_owner_shares_hook_pool_preserves_proxy_w_and_releases_source_order() {
    let mut backend = SkateboardSimulationBackend::try_new(28, 22, 46).unwrap();
    let mut manager = backend.try_manager_for_test(0).unwrap();
    let mut hooks =
        SimulationHookAssemblyPool::<AssemblyPartMaterial, SkateboardHookDefinition>::try_new(
            &manager, &backend, 3,
        )
        .unwrap();
    let mut skeleton = fixture(None);
    skeleton.attach(&mut backend).unwrap();
    let board_hook = board_hook();
    let board = hooks
        .try_insert(
            &mut backend,
            &mut manager,
            board_hook_candidate(&board_hook),
        )
        .unwrap();
    let mut cvars = Cvars::default();
    skeleton
        .try_initialize_drives(
            &mut backend,
            &mut manager,
            &mut hooks,
            &mut attributes(),
            &mut HostSkeletonDriveNumeric,
            &mut HostSkateboardWheelMassNumeric,
            &mut cvars,
        )
        .unwrap();
    assert_eq!(
        cvars.samples,
        ["displacement", "strength", "displacement", "strength"]
    );
    let runtime = skeleton.drive_owner().unwrap();
    assert_eq!(runtime.stage(), SkeletonDriveOwnerStage::Complete);
    let hook = runtime.hook().unwrap().clone();
    let geometry_weak = Arc::downgrade(runtime.prepared().hook_definition().unwrap());
    assert_eq!(hooks.len(), 2);
    assert_eq!(hooks.try_snapshot(&board).unwrap().world_index, 0);
    assert_eq!(hooks.try_snapshot(&hook).unwrap().world_index, 1);
    assert_eq!(hooks.try_snapshot(&hook).unwrap().parts.len(), 2);
    assert!(matches!(
        hooks.skeleton_hook_session(&board),
        Err(SimulationHookPoolError::WrongMemberKind)
    ));
    let drives = backend.drive_snapshot();
    assert_eq!(drives.counts.active, 46);
    assert_eq!(drives.slots[0].spy, Some(0));
    assert_eq!(drives.slots[1].spy, Some(0));
    for (ordinal, emission) in runtime.prepared().eligible_bone_drives().enumerate() {
        let slot = &drives.slots[ordinal + 2];
        assert_eq!(slot.spy, Some(1));
        assert_eq!(
            slot.body_a.unwrap().id().index(),
            u32::try_from(emission.part.index() + 1).unwrap()
        );
        assert_eq!(
            slot.body_b.unwrap().id().index(),
            u32::try_from(emission.parent.index() + 1).unwrap()
        );
    }
    assert_eq!(drives.slots[0].body_a.unwrap().id().index(), 24);
    assert_eq!(drives.slots[1].body_a.unwrap().id().index(), 1);
    assert_eq!(drives.slots[0].body_b.unwrap().id().index(), 26);
    assert_eq!(drives.slots[1].body_b.unwrap().id().index(), 27);
    assert!(matches!(
        skeleton.detach(&mut backend),
        Err(DynamicAssemblyDetachError::SkeletonDrivesPresent)
    ));
    let expected = expected_hook_transforms(&skeleton);
    skeleton
        .try_set_retained_pose_with_hooks(&mut backend, &mut hooks)
        .unwrap();
    let session = hooks.skeleton_hook_session(&hook).unwrap();
    for (part, matrix) in expected.into_iter().enumerate() {
        assert_eq!(
            session
                .part_transforms()
                .map(|matrix| matrix.expect("known fixture Part cache"))
                .nth(part)
                .unwrap()
                .map(f32::to_bits),
            matrix.map(f32::to_bits)
        );
    }
    let proxy = skeleton
        .state()
        .proxy_volume_target
        .unwrap()
        .known_lanes()
        .expect("known controlled proxy lanes");
    assert_eq!(
        proxy.map(f32::to_bits),
        expected[1][12..16]
            .try_into()
            .map(|value: [f32; 4]| value.map(f32::to_bits))
            .unwrap()
    );
    assert_eq!(
        proxy[3].to_bits(),
        (-1.0_f32).to_bits(),
        "full source affine W is not canonicalized or subtracted from itself"
    );
    assert_eq!(backend.drive_snapshot(), drives);
    skeleton
        .try_release_drives(&mut backend, &mut hooks)
        .unwrap();
    assert!(skeleton.drive_owner().is_none());
    assert_eq!(
        skeleton
            .state()
            .proxy_volume_target
            .unwrap()
            .known_lanes()
            .expect("known controlled proxy lanes")
            .map(f32::to_bits),
        proxy.map(f32::to_bits)
    );
    assert!(geometry_weak.upgrade().is_none());
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks.try_snapshot(&board).unwrap().world_index, 0);
    assert_eq!(
        backend
            .drive_snapshot()
            .free_order
            .iter()
            .map(|id| id.index())
            .collect::<Vec<_>>(),
        source_release_indices(0)
    );
    assert_eq!(backend.drive_snapshot().counts.active, 0);
    hooks.try_remove(&mut backend, &board).unwrap();
    skeleton.detach(&mut backend).unwrap();
    assert_eq!(backend.rigid_body_snapshot().counts.free, 27);
}

#[test]
fn capacity_null_bone_spy_retains_partial_owner_until_explicit_forward_hook_then_reverse_bone_cleanup()
 {
    for capacity in [0, 1, 3] {
        let mut backend = SkateboardSimulationBackend::try_new(27, 22, capacity).unwrap();
        let mut manager = backend.try_manager_for_test(0).unwrap();
        let mut hooks =
            SimulationHookAssemblyPool::<AssemblyPartMaterial, SkateboardHookDefinition>::try_new(
                &manager, &backend, 3,
            )
            .unwrap();
        let mut skeleton = fixture(None);
        skeleton.attach(&mut backend).unwrap();
        let failed = skeleton.try_initialize_drives(
            &mut backend,
            &mut manager,
            &mut hooks,
            &mut attributes(),
            &mut HostSkeletonDriveNumeric,
            &mut HostSkateboardWheelMassNumeric,
            &mut Cvars::default(),
        );
        assert!(matches!(
            failed,
            Err(SkeletonDriveConstructionError::Owner(
                SkeletonDriveOwnerError::Drive(DynamicAssemblyPostDriveError::NullDrive { .. })
            ))
        ));
        assert_eq!(
            backend.drive_snapshot().counts.active,
            u32::try_from(capacity).unwrap()
        );
        let runtime = skeleton.drive_owner().unwrap();
        assert_eq!(
            runtime.stage(),
            SkeletonDriveOwnerStage::BoneDrive {
                part: 1,
                kind: if capacity == 3 {
                    SkeletonDriveType::LocalRoot
                } else {
                    SkeletonDriveType::Local
                },
            }
        );
        assert_eq!(hooks.len(), 1);
        assert!(runtime.hook().is_some());
        assert!(matches!(
            skeleton.detach(&mut backend),
            Err(DynamicAssemblyDetachError::SkeletonDrivesPresent)
        ));
        assert!(matches!(
            skeleton.try_set_retained_pose_with_hooks(&mut backend, &mut hooks),
            Err(SkeletonDriveOwnerError::Incomplete)
        ));
        skeleton
            .try_release_drives(&mut backend, &mut hooks)
            .unwrap();
        assert!(skeleton.drive_owner().is_none());
        assert_eq!(hooks.len(), 0);
        assert_eq!(backend.drive_snapshot().counts.active, 0);
        assert_eq!(
            backend
                .drive_snapshot()
                .free_order
                .iter()
                .map(|id| id.index())
                .collect::<Vec<_>>(),
            (0..u32::try_from(capacity).unwrap()).collect::<Vec<_>>()
        );
        skeleton.detach(&mut backend).unwrap();
        assert_eq!(backend.rigid_body_snapshot().counts.free, 26);
    }
}

type MountedWorld =
    SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition>;

pub(crate) struct MountedDriveCheck {
    board_hook: SimulationHookAssemblyHandle,
    skeleton_hook: SimulationHookAssemblyHandle,
    proxy: [u32; 4],
    history: Vec<u32>,
}

/// Existing original board/skeleton corpus supplies both actual owners. The
/// cvars are explicit controlled samples; Vault reads are original mounted data.
pub(crate) fn initialize_mounted_drives(
    world: &mut MountedWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) -> MountedDriveCheck {
    let payload = board_hook();
    let board_hook = world
        .try_insert_hook(board_hook_candidate(&payload))
        .unwrap();
    assert!(
        world
            .try_connect_registered_hook(board, &board_hook, payload.frames(), payload.dynamics())
            .unwrap()
    );
    assert_eq!(world.try_hook_snapshot(&board_hook).unwrap().world_index, 2);
    let mut attributes = crate::skeleton_drives_corpus::mounted_skeleton_drive_attributes();
    let mut cvars = Cvars::default();
    world
        .try_initialize_skeleton_drives(
            skeleton,
            &mut attributes,
            &mut HostSkeletonDriveNumeric,
            &mut HostSkateboardWheelMassNumeric,
            &mut cvars,
        )
        .unwrap();
    let owner = world.skeleton(skeleton).unwrap();
    let runtime = owner.drive_owner().unwrap();
    attributes.assert_constructor_reads(runtime.prepared().data());
    assert_eq!(
        cvars.samples,
        ["displacement", "strength", "displacement", "strength"]
    );
    assert_eq!(runtime.stage(), SkeletonDriveOwnerStage::Complete);
    assert_eq!(runtime.prepared().eligible_bone_drives().count(), 44);
    let skeleton_hook = runtime.hook().unwrap().clone();
    assert_eq!(world.hook_count(), 2);
    assert_eq!(
        world.try_hook_snapshot(&skeleton_hook).unwrap().world_index,
        3
    );
    assert_eq!(
        world.try_hook_snapshot(&skeleton_hook).unwrap().parts.len(),
        2
    );
    let drives = world.backend().drive_snapshot();
    assert_eq!(drives.counts.active, 47);
    for (ordinal, slot) in drives.slots.iter().enumerate() {
        assert_eq!(slot.spy, Some(u32::from(ordinal >= 3)));
    }
    // Static manager0, original board1..7, skeleton8..31, board hook32,
    // actual two-Part skeleton hook33/34: these are observed real allocations.
    assert_eq!(drives.slots[0].body_a.unwrap().id().index(), 32);
    assert_eq!(drives.slots[0].body_b.unwrap().id().index(), 7);
    assert_eq!(drives.slots[1].body_a.unwrap().id().index(), 31);
    assert_eq!(drives.slots[1].body_b.unwrap().id().index(), 33);
    assert_eq!(drives.slots[2].body_a.unwrap().id().index(), 8);
    assert_eq!(drives.slots[2].body_b.unwrap().id().index(), 34);
    for (ordinal, emission) in runtime.prepared().eligible_bone_drives().enumerate() {
        let slot = &drives.slots[ordinal + 3];
        assert_eq!(
            slot.body_a.unwrap().id().index(),
            u32::try_from(emission.part.index() + 8).unwrap()
        );
        assert_eq!(
            slot.body_b.unwrap().id().index(),
            u32::try_from(emission.parent.index() + 8).unwrap()
        );
    }
    let expected = expected_hook_transforms(owner);
    assert!(world.try_remove_skeleton(skeleton).is_err());
    assert!(world.try_remove_hook(&board_hook).is_err());
    assert!(matches!(
        world.try_remove_hook(&skeleton_hook),
        Err(SimulationHookPoolError::WrongMemberKind)
    ));
    world.try_set_skeleton_pose_with_hooks(skeleton).unwrap();
    let proxy = world
        .skeleton(skeleton)
        .unwrap()
        .state()
        .proxy_volume_target
        .unwrap()
        .known_lanes()
        .expect("known controlled proxy lanes")
        .map(f32::to_bits);
    assert_eq!(
        proxy,
        std::array::from_fn(|lane| expected[1][12 + lane].to_bits())
    );
    assert_eq!(world.backend().drive_snapshot(), drives);
    let history = super::super::history_owner::tests::initialize_mounted_history(world, skeleton);
    MountedDriveCheck {
        board_hook,
        skeleton_hook,
        proxy,
        history,
    }
}

/// Runs after the existing shared-ground preparation, leaving board/skeleton
/// members admitted for their existing swap-remove/whole-owner checks.
pub(crate) fn finish_mounted_drives(
    world: &mut MountedWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    check: &MountedDriveCheck,
) {
    assert_eq!(
        super::super::history_owner::tests::history_words(
            world.skeleton(skeleton).unwrap().physics_history()
        ),
        check.history
    );
    world.try_release_skeleton_drives(skeleton).unwrap();
    let owner = world.skeleton(skeleton).unwrap();
    assert!(owner.drive_owner().is_none());
    assert_eq!(
        super::super::history_owner::tests::history_words(owner.physics_history()),
        check.history
    );
    assert_eq!(
        owner
            .state()
            .proxy_volume_target
            .unwrap()
            .known_lanes()
            .expect("known controlled proxy lanes")
            .map(f32::to_bits),
        check.proxy
    );
    assert_eq!(world.hook_count(), 1);
    assert!(world.try_hook_snapshot(&check.skeleton_hook).is_err());
    assert_eq!(
        world
            .try_hook_snapshot(&check.board_hook)
            .unwrap()
            .world_index,
        2
    );
    let snapshot = world.backend().drive_snapshot();
    assert_eq!(snapshot.counts.active, 1);
    assert_eq!(
        snapshot
            .active_order
            .iter()
            .map(|id| id.index())
            .collect::<Vec<_>>(),
        [0]
    );
    let mut expected = source_release_indices(1);
    assert_eq!(
        snapshot
            .free_order
            .iter()
            .map(|id| id.index())
            .collect::<Vec<_>>(),
        expected
    );
    world.try_remove_board_drives(board).unwrap();
    expected.push(0);
    assert_eq!(
        world
            .backend()
            .drive_snapshot()
            .free_order
            .iter()
            .map(|id| id.index())
            .collect::<Vec<_>>(),
        expected
    );
    world.try_remove_hook(&check.board_hook).unwrap();
    assert_eq!(world.hook_count(), 0);
    assert_eq!(world.backend().drive_snapshot().counts.active, 0);
}
