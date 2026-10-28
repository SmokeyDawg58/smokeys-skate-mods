use super::*;
use crate::{
    simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameInput,
    simulation_drive_jacobian::try_prepare_active_drive_islands_host,
    simulation_horus_pipeline::try_complete_prepared_horus_islands_serial,
    simulation_joint_preparation::try_prepare_active_joint_islands_host,
    simulation_rigid_body_pool::{SimulationInertia, SimulationRigidBodyHandle},
    skateboard_hook::SkateboardHookDefinition,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    static_ground_pool::StaticGroundPool,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::sync::Arc;

struct CurrentInput {
    enable: Result<u8, &'static str>,
    threshold: Result<u32, &'static str>,
    calls: Vec<&'static str>,
}

impl CurrentInput {
    fn new(enable: u8, threshold: u32) -> Self {
        Self {
            enable: Ok(enable),
            threshold: Ok(threshold),
            calls: Vec::new(),
        }
    }
}

impl SimulationContactFeedbackInput for CurrentInput {
    type Error = &'static str;
    fn current_enable_freezing(&mut self) -> Result<u8, Self::Error> {
        self.calls.push("enable");
        self.enable
    }
    fn current_frames_before_freezing(&mut self) -> Result<u32, Self::Error> {
        self.calls.push("threshold");
        self.threshold
    }
}

type World = SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition>;

fn static_rf(world: &World) -> [[u32; 4]; 4] {
    world
        .backend
        .rigid_bodies
        .try_with_solver_delta_row(world.contacts.static_body_observation(), |rows| {
            rows.map(|row| row.map(f32::to_bits))
        })
        .unwrap()
}

fn seed_static_rf(world: &mut World) {
    world
        .backend
        .rigid_bodies
        .try_with_solver_delta_row_mut(world.contacts.static_body_observation(), |rows| {
            *rows = [[1.0, -0.0, f32::from_bits(0x7fc0_5678), -2.0]; 4];
        })
        .unwrap();
}

/// Real empty compiler backing and completion receipt; no arbitrary `SpyFrame`
/// constructor. The extra body/RF is a controlled component fixture, not a
/// source Skater assembly or proof that world collision found no contacts.
fn completed_world() -> (World, SimulationRigidBodyHandle) {
    completed_world_with_freezing(false)
}
fn completed_world_with_freezing(warm_stationary_body: bool) -> (World, SimulationRigidBodyHandle) {
    let mut world = World::try_new(2, 0, 0, 0).unwrap();
    if warm_stationary_body {
        world.try_initialize_world_feedback_parameters().unwrap();
        world.backend.rigid_bodies.set_gravity([0.; 3]);
    }
    world.try_initialize_skater_pool(3, false).unwrap();
    let body = world
        .backend
        .rigid_bodies
        .try_add_rigid_body(
            FRESH_VOLUME_TRANSFORM,
            Some(&Arc::new(SimulationInertia::native_default())),
            4,
        )
        .unwrap()
        .unwrap();
    world
        .backend
        .rigid_bodies
        .try_register_contact_spy(body)
        .unwrap();
    world.backend.rigid_bodies.set_cooldown(73);
    world
        .backend
        .rigid_bodies
        .begin_dynamic_update_frame(1.0 / 60.0);
    if warm_stationary_body {
        // Controlled stationary component input; native DynamicUpdate produces
        // the qualifying cooldown, with no fabricated body/header stores.
        for _ in 0..2 {
            world
                .backend
                .rigid_bodies
                .try_integrate_active_bodies_ordinary_finite()
                .unwrap();
        }
    }
    // A controlled predecessor marker in actual Static RF, before compilation
    // and integration; the Active DynamicUpdate must not clear this row.
    seed_static_rf(&mut world);
    world
        .contacts
        .try_start_island_frame_labels(&world.backend.rigid_bodies)
        .unwrap();
    let map = world
        .contacts
        .try_island_map_snapshot(&world.backend.rigid_bodies)
        .unwrap();
    let phase = world
        .backend
        .rigid_bodies
        .try_prepare_dynamic_update_phase()
        .unwrap();
    let mut contacts = world
        .contacts
        .try_prepare_empty_contact_islands_host(&phase, &map)
        .unwrap();
    let mut joints =
        try_prepare_active_joint_islands_host(&world.backend.joints, &phase, &map).unwrap();
    let mut drives =
        try_prepare_active_drive_islands_host(&world.backend.drives, &phase, &map).unwrap();
    let completed = try_complete_prepared_horus_islands_serial(
        &mut contacts,
        &mut joints,
        &mut drives,
        10,
        0,
        &world.backend.drives,
        phase,
    )
    .unwrap();
    world.contact_spies = Some(std::sync::Arc::new(
        contacts.try_export_contact_spies(completed, 4).unwrap(),
    ));
    world.try_refresh_skaters().unwrap();
    assert!(world.skaters.as_ref().unwrap().active_frame().is_some());
    assert_ne!(static_rf(&world), [[0; 4]; 4]);
    (world, body)
}

#[test]
fn actual_empty_list_creation_requires_completed_feedback_and_refuses_replay() {
    use crate::simulation_assembly_backend::constraint_session::physical_contact_list::SimulationPhysicalContactListProgress as Progress;
    let (mut world, _) = completed_world();
    assert!(world.try_create_physical_contact_lists().is_err());
    assert_eq!(world.physical_contact_list_progress(), Progress::NotStarted);
    let mut input = CurrentInput::new(0, u32::MAX);
    world.try_prepare_contact_feedback(&mut input).unwrap();
    world.try_create_physical_contact_lists().unwrap();
    assert_eq!(
        world.physical_contact_list_progress(),
        Progress::Complete {
            completed_spies: 0,
            appended_contacts: 0
        }
    );
    assert_eq!(world.physical_contacts.retained_entry_count(), 0);
    assert!(world.contact_spies().unwrap().unwrap().records().is_empty());
    assert!(world.try_create_physical_contact_lists().is_err());
    assert_eq!(input.calls, ["enable"]);
}

#[test]
fn stale_completed_export_cannot_create_a_new_physical_contact_list() {
    let (mut world, _) = completed_world();
    world
        .try_prepare_contact_feedback(&mut CurrentInput::new(0, 0))
        .unwrap();
    world
        .backend
        .rigid_bodies
        .begin_dynamic_update_frame(1.0 / 60.0);
    assert!(world.try_create_physical_contact_lists().is_err());
    assert_eq!(world.physical_contacts.retained_entry_count(), 0);
}

#[test]
fn feedback_disabled_current_byte_resets_static_rf_without_threshold_or_freeze() {
    let (mut world, body) = completed_world();
    let before = world.backend.rigid_body_snapshot();
    let refreshed = world.skaters.as_ref().unwrap().active_frame();
    let mut input = CurrentInput::new(0, 0);
    input.threshold = Err("must not read");
    world.try_prepare_contact_feedback(&mut input).unwrap();
    assert_eq!(input.calls, ["enable"]);
    assert_eq!(static_rf(&world), [[0; 4]; 4]);
    assert_eq!(world.backend.rigid_body_snapshot(), before);
    assert_eq!(world.skaters.as_ref().unwrap().active_frame(), refreshed);
    assert_eq!(
        world.contact_feedback_progress,
        SimulationContactFeedbackProgress::Complete { frozen_bodies: 0 }
    );
    world
        .backend
        .rigid_bodies
        .try_validate_observation(body.observation())
        .unwrap();
    assert!(world.contact_spies().unwrap().unwrap().records().is_empty());
    let mut retry = CurrentInput::new(1, 0);
    assert!(matches!(
        world.try_prepare_contact_feedback(&mut retry),
        Err(SimulationContactFeedbackError::AlreadyAttempted)
    ));
    assert!(retry.calls.is_empty());
}

#[test]
fn feedback_noncanonical_true_reads_current_threshold_and_keeps_spy_occurrences() {
    let (mut world, body) = completed_world();
    let before = world.backend.rigid_body_snapshot();
    let mut input = CurrentInput::new(0x80, 0);
    world.try_prepare_contact_feedback(&mut input).unwrap();
    assert_eq!(input.calls, ["enable", "threshold"]);
    assert_eq!(static_rf(&world), [[0; 4]; 4]);
    let after = world.backend.rigid_body_snapshot();
    assert_eq!(after.active_order.len(), 0);
    assert_eq!(after.frozen_order, vec![body.id()]);
    assert_eq!(after.slots[body.id().index() as usize].state, 10);
    assert_eq!(after.slots[body.id().index() as usize].cooldown, 73);
    assert_eq!(
        after.slots[body.id().index() as usize].occurrence,
        before.slots[body.id().index() as usize].occurrence
    );
    assert_eq!(
        after.slots[body.id().index() as usize]
            .velocity
            .map(f32::to_bits),
        before.slots[body.id().index() as usize]
            .velocity
            .map(f32::to_bits)
    );
    assert_eq!(
        world.contact_feedback_progress,
        SimulationContactFeedbackProgress::Complete { frozen_bodies: 1 }
    );
    assert!(world.skaters.as_ref().unwrap().active_frame().is_none());
    world
        .backend
        .rigid_bodies
        .try_validate_observation(body.observation())
        .unwrap();
    world
        .contact_spies()
        .unwrap()
        .expect("same retained completed frame");
}

#[test]
fn feedback_enabled_without_qualifying_bodies_keeps_refresh_stamp() {
    let (mut world, _) = completed_world();
    let refreshed = world.skaters.as_ref().unwrap().active_frame();
    let before = world.backend.rigid_body_snapshot();
    let mut input = CurrentInput::new(1, u32::MAX);
    world.try_prepare_contact_feedback(&mut input).unwrap();
    assert_eq!(input.calls, ["enable", "threshold"]);
    assert_eq!(world.backend.rigid_body_snapshot(), before);
    assert_eq!(world.skaters.as_ref().unwrap().active_frame(), refreshed);
    assert_eq!(
        world.contact_feedback_progress,
        SimulationContactFeedbackProgress::Complete { frozen_bodies: 0 }
    );
    assert_eq!(static_rf(&world), [[0; 4]; 4]);
}

#[test]
fn feedback_missing_or_stale_spy_refusal_has_no_config_or_reset_effects() {
    let mut missing = World::try_new(1, 0, 0, 0).unwrap();
    seed_static_rf(&mut missing);
    let missing_rows = static_rf(&missing);
    let mut input = CurrentInput::new(1, 0);
    assert!(matches!(
        missing.try_prepare_contact_feedback(&mut input),
        Err(SimulationContactFeedbackError::MissingSpyFrame)
    ));
    assert_eq!(static_rf(&missing), missing_rows);
    assert_eq!(
        missing.contact_feedback_progress,
        SimulationContactFeedbackProgress::NotStarted
    );
    assert!(input.calls.is_empty());

    let (mut stale, _) = completed_world();
    let stale_rows = static_rf(&stale);
    stale
        .backend
        .rigid_bodies
        .begin_dynamic_update_frame(1.0 / 60.0);
    assert!(matches!(
        stale.try_prepare_contact_feedback(&mut input),
        Err(SimulationContactFeedbackError::Spy(
            SimulationContactSpyError::FrameMismatch
        ))
    ));
    assert_eq!(static_rf(&stale), stale_rows);
    assert_eq!(
        stale.contact_feedback_progress,
        SimulationContactFeedbackProgress::NotStarted
    );
    assert!(input.calls.is_empty());
}

#[test]
fn feedback_config_failure_keeps_reset_prefix_and_refuses_replay() {
    for fail_threshold in [false, true] {
        let (mut world, _) = completed_world();
        let before = world.backend.rigid_body_snapshot();
        let mut input = CurrentInput::new(1, 0);
        if fail_threshold {
            input.threshold = Err("threshold unavailable");
        } else {
            input.enable = Err("enable unavailable");
        }
        assert!(matches!(
            world.try_prepare_contact_feedback(&mut input),
            Err(SimulationContactFeedbackError::Input(_))
        ));
        assert_eq!(
            input.calls,
            if fail_threshold {
                vec!["enable", "threshold"]
            } else {
                vec!["enable"]
            }
        );
        // This also proves reset precedes the failed callback, rather than a
        // prefetch of either setting before the first source mutation.
        assert_eq!(static_rf(&world), [[0; 4]; 4]);
        assert_eq!(world.backend.rigid_body_snapshot(), before);
        assert_eq!(
            world.contact_feedback_progress,
            SimulationContactFeedbackProgress::StaticForcesReset
        );
        world
            .contact_spies()
            .unwrap()
            .expect("integration/export prefix retained");
        let mut retry = CurrentInput::new(1, 0);
        assert!(matches!(
            world.try_prepare_contact_feedback(&mut retry),
            Err(SimulationContactFeedbackError::AlreadyAttempted)
        ));
        assert!(retry.calls.is_empty());
    }
}

#[test]
fn authored_ground_empty_members_automatically_complete_feedback_each_frame() {
    let mut world = World::try_new(1, 0, 0, 0).unwrap();
    world.try_initialize_skater_pool(3, false).unwrap();
    let ground = StaticGroundPool::try_new(0).unwrap();
    let mut input = CurrentInput::new(0, 0);
    for enabled in [0, 1] {
        seed_static_rf(&mut world);
        input.enable = Ok(enabled);
        input.calls.clear();
        let (counts, _) = world
            .try_step_authored_ground(
                SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                    ground_pool: &ground,
                    ground_group: 0,
                    pipeline_culling: None,
                    allow_multiple_writes: true,
                    spy_flags: 4,
                },
                1.0 / 60.0,
                2,
                &mut input,
            )
            .unwrap();
        assert_eq!((counts.contacts, counts.joints, counts.drives), (0, 0, 0));
        assert_eq!(
            input.calls,
            if enabled == 0 {
                vec!["enable"]
            } else {
                vec!["enable", "threshold"]
            }
        );
        assert_eq!(static_rf(&world), [[0; 4]; 4]);
        assert_eq!(
            world.contact_feedback_progress,
            SimulationContactFeedbackProgress::Complete { frozen_bodies: 0 }
        );
        assert!(world.contact_spies().unwrap().unwrap().records().is_empty());
    }
}

#[test]
fn native_default_live_freezing_uses_manager_threshold_and_completed_frame() {
    let (mut world, body) = completed_world_with_freezing(true);
    let refs = crate::input_cvar_runtime::InputCvarReferences::new_static_initialized();
    let mut cvars = crate::input_cvar_runtime::InputCvarManager::new();
    cvars.try_link(&refs).unwrap();
    let mut feedback = world.try_live_contact_feedback(&refs).unwrap();
    assert_eq!(feedback.current_frames_before_freezing().unwrap(), 2);
    assert_eq!(world.backend.rigid_bodies.cooldown(), 73);
    world.try_prepare_contact_feedback(&mut feedback).unwrap();
    let snapshot = world.backend.rigid_body_snapshot();
    assert_eq!(snapshot.frozen_order, vec![body.id()]);
    assert_eq!(snapshot.slots[body.id().index() as usize].cooldown, 73);
    assert_eq!(static_rf(&world), [[0; 4]; 4]);
    assert_eq!(
        world.contact_feedback_progress,
        SimulationContactFeedbackProgress::Complete { frozen_bodies: 1 }
    );
}
