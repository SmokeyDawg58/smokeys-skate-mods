//! Original selected inputs through initial Ground entry on one retained pair.
//! Spawn remains controlled; pose/physics data come from the same Actor animation.
//! Full Actor physical admission and
//! continuous state execution are not claimed by this controlled schedule.
use super::*;
use crate::physical_player_physics_state::{CurrentPhysicalState, PhysicalStateStage};
use crate::simulation_assembly_backend::constraint_session::physical_state::ground_to_air::GroundDestination;
use std::sync::Mutex;
#[path = "../runtime/physical_player_camera_stream_corpus.rs"]
mod camera_stream;
#[path = "../grind/physical_player_grind_corpus.rs"]
mod grind_corpus;
#[path = "../runtime/physical_player_three_frame_corpus.rs"]
mod three_frame;
#[path = "../lifecycle/physical_player_update1_corpus.rs"]
mod update1_corpus;

pub(super) fn check(inputs: PhysicalPlayerAssemblyInputs) {
    check_schedule_mode(inputs.clone(), None, true, true, false);
    check_schedule(inputs.clone(), None, true);
    check_schedule(inputs.clone(), None, false);
    check_schedule(inputs.clone(), Some(GroundDestination::Air), false);
    check_schedule(inputs.clone(), Some(GroundDestination::Wipeout), false);
    // Each schedule constructs fresh owners. Exercise established modes before
    // the unfinished repeated-frame continuation, so its failure cannot mask
    // their original-data regressions.
    check_schedule_mode(inputs, None, true, true, true);
}

pub(super) fn check_queued_ticks(inputs: PhysicalPlayerAssemblyInputs) {
    check_schedule_policy(
        inputs,
        None,
        true,
        true,
        true,
        false,
        Some(super::queued_input::QueuedRemapPolicy::ExistingFixture),
    );
}

pub(super) fn check_queued_actual_roster_ticks(inputs: PhysicalPlayerAssemblyInputs) {
    check_schedule_policy(
        inputs,
        None,
        true,
        true,
        true,
        true,
        Some(super::queued_input::QueuedRemapPolicy::ExistingFixture),
    );
}

pub(super) fn check_queued_scripted_ticks(inputs: PhysicalPlayerAssemblyInputs) {
    check_schedule_policy(
        inputs,
        None,
        true,
        true,
        true,
        false,
        Some(super::queued_input::QueuedRemapPolicy::ScriptedKickflip),
    );
}

pub(super) fn check_queued_scripted_y_ticks(inputs: PhysicalPlayerAssemblyInputs) {
    check_schedule_policy(
        inputs,
        None,
        true,
        true,
        true,
        false,
        Some(super::queued_input::QueuedRemapPolicy::ScriptedY),
    );
}

pub(super) fn check_queued_juice_ticks(inputs: PhysicalPlayerAssemblyInputs) {
    check_schedule_policy(
        inputs,
        None,
        true,
        true,
        true,
        false,
        Some(super::queued_input::QueuedRemapPolicy::NativeDisabledJuice),
    );
}

pub(super) fn check_wipeout_ticks(inputs: PhysicalPlayerAssemblyInputs) {
    check_schedule_mode(inputs, Some(GroundDestination::Wipeout), true, true, true);
}

fn check_schedule(
    inputs: PhysicalPlayerAssemblyInputs,
    destination: Option<GroundDestination>,
    finish_constructor: bool,
) {
    check_schedule_mode(inputs, destination, finish_constructor, false, false);
}

pub(super) fn check_actual_roster_startup(inputs: PhysicalPlayerAssemblyInputs) {
    check_schedule_policy(inputs, None, true, true, true, true, None);
}
fn check_schedule_mode(
    inputs: PhysicalPlayerAssemblyInputs,
    destination: Option<GroundDestination>,
    finish_constructor: bool,
    solved_fill: bool,
    repeated_frames: bool,
) {
    check_schedule_policy(
        inputs,
        destination,
        finish_constructor,
        solved_fill,
        repeated_frames,
        false,
        None,
    );
}
#[expect(
    clippy::too_many_lines,
    reason = "one actual Actor-animation to physical pair integration fixture"
)]
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "explicit legacy component and source startup corpus policies"
)]
fn check_schedule_policy(
    mut inputs: PhysicalPlayerAssemblyInputs,
    destination: Option<GroundDestination>,
    finish_constructor: bool,
    solved_fill: bool,
    repeated_frames: bool,
    native_roster: bool,
    queued_input: Option<super::queued_input::QueuedRemapPolicy>,
) {
    if repeated_frames {
        // Host-selected authored location, not the native spawn-manager chain.
        // Set all lanes before either Actor or physical construction so their
        // retained transforms and histories share the same original placement.
        let mounts =
            crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_corpus();
        let locations = skate2_formats::location::LocationTable::parse(
            &mounts
                .read_path("data\\content\\world\\stream\\BAM\\BAM_Sim.loc")
                .unwrap(),
        )
        .unwrap();
        inputs.spawn = locations.find("A_RobsPark").unwrap().transform;
    }
    let enter_air = matches!(destination, Some(GroundDestination::Air));
    let shared = if repeated_frames && native_roster {
        let mounts = Arc::new(crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_corpus());
        let startup = crate::selected_session_world_startup::SelectedSessionWorldStartup::try_new(
            &inputs.cvars,
            &inputs.vault,
            &mounts,
        )
        .unwrap();
        assert!(Arc::ptr_eq(
            startup
                .world
                .lock()
                .unwrap()
                .local_skater_identity
                .as_ref()
                .unwrap(),
            &startup.local,
        ));
        startup.world
    } else {
        // Retain the same actual global fields across World and SkaterManager.
        let game_info = crate::simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo::construct_network_field();
        let backend =
            crate::skater_manager_local_identity::LocalBackendOwner::construct_static_fields(
                Arc::clone(&game_info),
            );
        if finish_constructor {
            backend
                .try_apply_debug_menu_ghost_sample(
                    inputs.cvars.try_enable_ghost_by_default().unwrap(),
                )
                .unwrap();
        }
        let mut world = if repeated_frames {
            PhysicalAssemblyWorld::try_new_selected_main(&backend).unwrap()
        } else {
            PhysicalAssemblyWorld::try_new(35, 28, 51, if solved_fill { 256 } else { 0 }).unwrap()
        };
        if repeated_frames {
            world.try_initialize_world_feedback_parameters().unwrap();
        }
        world
            .try_initialize_prediction_owner(&inputs.vault, true)
            .unwrap();
        world
            .try_initialize_step_selector(true, &inputs.cvars)
            .unwrap();
        world.try_initialize_grab_query_manager(true).unwrap();
        world.try_initialize_debug_overlay().unwrap();
        // Selected source startup subsequence: World.Init, LW.Init, provider install.
        world.grind_services.try_initialize_static().unwrap();
        world
            .grind_services
            .try_initialize_entities(&inputs.vault, &inputs.cvars)
            .unwrap();
        world.grind_services.try_install_dynamic_provider().unwrap();
        if finish_constructor {
            // World and local identity share the backend sampled above.
            world.entity_roster = Some(
                crate::simulation_entity_roster::SimulationEntityRoster::construct(
                    world.physical_manager_authority(),
                ),
            );
            let manager = crate::skater_manager_local_identity::SkaterManagerLocalIdentity::construct_local_fields(backend);
            world.local_skater_identity = Some(Arc::clone(&manager));
            if native_roster {
                // Selected EventSystem queue7 construction occurs before callbacks;
                // the actual observer manager retains this SAME queue thereafter.
                let queue =
                    crate::hermes_challenge_queue::ChallengeEventQueue::try_construct().unwrap();
                manager.try_register_startup_observers(queue).unwrap();
            }

            manager.try_reset_game_before_first_spawn().unwrap();
            world
                .try_initialize_physical_world_services(game_info, true)
                .unwrap();
        }
        world
            .try_initialize_skater_pool(
                world.island_flags(),
                inputs.cvars.try_skater_pool_parallel().unwrap(),
            )
            .unwrap();
        Arc::new(Mutex::new(world))
    };
    let input_vault = Arc::clone(&inputs.vault);
    let (actor, mut physical_animation, mut input_session, mut selected_players) =
        if repeated_frames {
            let (actor, animation, session, players) =
                crate::actor_construction::player_startup::corpus::construct(
                    &inputs.vault,
                    &shared,
                    inputs.spawn,
                    &inputs.cvars,
                    native_roster,
                );
            (actor, animation, session, Some(players))
        } else {
            let (actor, animation, session) = crate::game_input_recognizer_corpus::check(
                &inputs.vault,
                &shared,
                inputs.spawn,
                inputs.cvars.clone(),
            );
            (Arc::new(actor), animation, session, None)
        };
    if let Some(players) = &selected_players {
        inputs = crate::selected_session_physical_pair_startup::try_selected_physical_inputs(
            players,
            actor.as_ref(),
            &physical_animation,
            inputs.spawn,
        )
        .unwrap();
        assert!(matches!(
            players.loader.phase(),
            crate::player::PlayerLoadPhase::Registering { .. }
        ));
    } else {
        physical_animation.try_apply_to(&mut inputs).unwrap();
    }
    let mut action_services =
        crate::actor_construction::action_runtime_session::ActionRuntimeServices::try_bind(
            &inputs.vault,
            shared.lock().unwrap().debug_overlay.clone(),
        )
        .unwrap();
    let mut mapping = crate::gesture_trick_mapping::GestureTrickMapping::new();
    mapping
        .try_initialize::<std::convert::Infallible>(|_, _, _| {
            Ok(crate::gesture_trick_mapping::MappingAllocation::Host)
        })
        .unwrap();
    action_services.gesture = Some(Arc::new(
        crate::actor_construction::action_motion_intent_bridge::gesture_runtime::GestureTrickRuntimeServices::try_capture_mapping_member(
            Arc::new(mapping), inputs.cvars.clone(), shared.lock().unwrap().debug_overlay.clone(),
            crate::gesture_trick_mapping::debug::GestureMappingDebugPosition::new(),
        ).unwrap(),
    ));
    actor
        .try_install_action_services(Arc::new(action_services))
        .unwrap();
    let mut refs = inputs.cvars.clone();
    let mut world = shared.lock().unwrap();
    if repeated_frames {
        let completed =
            crate::selected_session_physical_pair_startup::try_construct_selected_physical_pair(
                &mut world,
                actor.as_ref(),
                inputs,
                &mut refs,
                &mut crate::reckoning_numeric::HostReckoningNumeric,
                true,
                &mut || panic!("native spawn orthogonality assertion"),
                &mut || panic!("original hat0"),
            )
            .unwrap();
        let pair = &completed.pair;
        // Actor::Actor 0x825ff300..444 constructs this one breadcrumb owner
        // immediately after PhysicalPlayerHiLOD returns, before later Actor
        // auxiliaries. Use the same published animation, Vault and World/LW/
        // MeshStore identities as the selected production boot.
        let breadcrumb_owners = crate::actor_construction::breadcrumbs::ActorBreadCrumbConstructorOwners::try_capture_selected(&shared, &world).unwrap();
        actor
            .try_construct_breadcrumbs_from_animation(&input_vault, breadcrumb_owners)
            .unwrap();
        let mut camera_residency = None;
        // Keep the actual full accepted stream owner alive through camera ticks.
        let mut _camera_stream = None;
        if native_roster {
            let players = selected_players.as_mut().expect("actual selected startup");
            players.try_ensure_player_graph_owners().unwrap();
            let stream = camera_stream::load(Arc::clone(&players.mounts), &refs);
            let active_world = &mut world;
            let residency = grind_corpus::mount_from(
                &active_world.grind_services,
                &refs,
                &players.mounts,
                &stream.catalog,
            );
            let capture = residency.camera_inputs(active_world, &shared, players, &stream.bounds);
            players
                .services
                .skaters
                .try_capture_selected_camera(capture)
                .unwrap();
            let entities = Arc::clone(active_world.entity_roster.as_ref().unwrap());
            players.try_admit_first_actor(&actor, &entities).unwrap();
            assert!(entities.contains(&actor).unwrap());
            assert_eq!(players.services.skaters.try_count().unwrap(), 1);
            assert!(Arc::ptr_eq(
                &players.services.skaters.try_actor(0).unwrap().unwrap(),
                &actor
            ));
            assert_eq!(players.loaded_graphs, Some([true, true]));
            assert!(Arc::ptr_eq(
                &players.services.skaters.try_local_actor().unwrap().unwrap(),
                &actor
            ));
            let (census, queue, camera_null) =
                players.services.skaters.observer_evidence().unwrap();
            assert_eq!(census, 0xff);
            assert!(!camera_null);
            let (creation, deletion, entries) = queue.snapshot().unwrap();
            assert_eq!((creation, deletion), (96, 0));
            assert_eq!(entries.len(), 2);
            assert!(
                entries
                    .iter()
                    .all(|entry| entry.message_published && entry.event_size == 48)
            );
            let created = entries[0].copied_message.unwrap();
            let local_changed = entries[1].copied_message.unwrap();
            assert_eq!(
                created.kind,
                u32::from_ne_bytes((-340_057_085_i32).to_ne_bytes())
            );
            assert_eq!(local_changed.kind, 2_104_924_600);
            use crate::hermes_challenge_queue::{ChallengePayload, SkaterNotification};
            let player_id = players.services.skaters.try_local_index().unwrap();
            assert_eq!(created.payload, ChallengePayload::Skater {
                notification: SkaterNotification::Created, player_id,
            });
            assert_eq!(local_changed.payload, ChallengePayload::Skater {
                notification: SkaterNotification::LocalChanged, player_id,
            });
            assert_eq!(
                created.port,
                skate2_formats::state_graph::state_graph_string_hash32(b"Challenge")
            );
            assert_eq!(local_changed.port, created.port);
            eprintln!(
                "actual same-Actor roster/local callback constructed retained camera; proceeding to explicit selected camera schedule"
            );
            camera_residency = Some(residency);
            _camera_stream = Some(stream);
        }
        let registered = actor.registered_physical_allocation().unwrap();
        let cached = actor.cached_physical_allocation().unwrap();
        assert!(Arc::ptr_eq(&registered, &cached));
        assert!(Arc::ptr_eq(cached.owner(), &completed.owner));
        assert_eq!(
            cached.owner().physics_state().lock().unwrap().current(),
            CurrentPhysicalState::Ground
        );
        if let Some(policy) = queued_input {
            three_frame::check_queued(
                &mut world,
                &shared,
                &pair.board,
                &pair.skeleton,
                actor.as_ref(),
                &mut physical_animation,
                &mut input_session,
                &refs,
                &input_vault,
                selected_players.as_ref().expect("same startup owners"),
                camera_residency,
                policy,
            );
            return;
        }
        let run = if matches!(destination, Some(GroundDestination::Wipeout)) {
            three_frame::check_wipeout
        } else {
            three_frame::check
        };
        run(
            &mut world,
            &shared,
            &pair.board,
            &pair.skeleton,
            actor.as_ref(),
            &mut physical_animation,
            &mut input_session,
            &refs,
            &input_vault,
            &selected_players
                .as_ref()
                .expect("fresh repeated-frame startup")
                .services,
            selected_players
                .as_ref()
                .unwrap()
                .presentation
                .as_ref()
                .unwrap(),
            Arc::clone(
                selected_players
                    .as_ref()
                    .unwrap()
                    .recipe_load
                    .as_ref()
                    .unwrap()
                    .scene()
                    .unwrap(),
            ),
            camera_residency,
        );
        return;
    }
    let mut factory = actor.try_allocate_physical(&mut world, inputs).unwrap();
    let registered = actor.registered_physical_allocation().unwrap();
    assert!(Arc::ptr_eq(registered.owner(), factory.owner()));
    assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::Fresh);
    {
        let prefix = actor.prefix().unwrap();
        let mut services = constructor_services::PhysicalConstructorServices {
            fields: prefix.fields().expect("actual constructed Actor fields"),
            references: &refs,
            assertion: || panic!("native spawn orthogonality assertion"),
        };
        factory.try_construct_board(&mut services).unwrap();
        factory
            .try_construct_skeleton(&mut services, &mut || panic!("original hat0"))
            .unwrap();
    }
    factory.try_finish_skeleton_from_live_services().unwrap();
    let owner = Arc::clone(factory.owner());
    {
        let animation = owner.anim_out.lock().unwrap();
        let animation = animation.as_ref().unwrap();
        assert_eq!(
            animation.grind_name,
            crate::play_animation::ANDALE_NULL_STRING_NAME_30
        );
        assert_eq!(animation.flags_known_mask, 0xf800_0000);
        assert_eq!(animation.flags_bits & animation.flags_known_mask, 0);
        let grab = owner.grab_spline.lock().unwrap();
        let grab = grab.as_ref().unwrap();
        assert!(grab.returned);
        assert!(
            !grab.data.lock().unwrap().ready
                && !grab.alternate.lock().unwrap().ready
                && !grab.interactable.lock().unwrap().ready
        );
        assert_eq!(grab.data.lock().unwrap().result.length_unit, Some([0.0; 4]));
        assert_eq!(grab.data.lock().unwrap().result.status_known_mask, u8::MAX);
        assert_eq!(grab.query.lock().unwrap().result.result_count, 0);
        assert!(!grab.query.lock().unwrap().ready);
        assert!(Arc::ptr_eq(
            grab.manager.as_ref().unwrap(),
            factory.world().grab_query_manager().unwrap()
        ));
    }
    crate::actor_construction::character_gesture::check_unpublished(&actor, &refs);
    crate::actor_construction::action_runtime_session::check_unpublished(&actor);
    {
        let output = owner.phys_out().lock().unwrap();
        assert!(!output.try_ground_output().unwrap().is_hippy_jumping);
        assert!(!output.try_offboard_output().unwrap().is_grabbing_object);
    }
    crate::physical_player_trajectory_owners::check_original(&owner);
    crate::physical_player_grind_manager::check_original(
        owner.grind.lock().unwrap().as_ref().unwrap(),
        &owner,
    );
    {
        let adjust = owner.grind_air().lock().unwrap();
        let adjust = adjust.as_ref().unwrap();
        assert!((2..=12).contains(&adjust.num_frames));
        assert!(!adjust.started && !adjust.activated && !adjust.adjusting);
        assert_eq!(
            adjust.targets.map(|target| target.contact),
            [1, 4, 0, 2, 2, 3]
        );
        assert!(adjust.contact_offsets[1][1] > 0.0);
        assert_eq!(
            adjust.contact_offsets[1][1].to_bits(),
            adjust.contact_offsets[4][1].to_bits()
        );
        assert_eq!(
            adjust.contact_offsets[1][2].to_bits(),
            (-adjust.contact_offsets[4][2]).to_bits()
        );
    }
    crate::physical_player_handplant::check_original(
        owner.handplant.lock().unwrap().as_ref().unwrap(),
        factory.world().prediction_owner().unwrap(),
    );
    let board = factory.board().unwrap().clone();
    let skeleton = factory.skeleton().unwrap().clone();
    let hook = factory.state.hook.as_ref().unwrap().clone();
    let processed = Arc::clone(owner.reckoning().lock().unwrap().processed());
    assert_eq!(
        processed.lock().unwrap().physics_state_fields().category,
        Some(0)
    );
    let world = factory.completed_world_mut().unwrap();
    world
        .try_construct_pair_skateboard_controller(&board, &skeleton, &mut refs)
        .unwrap();
    crate::physical_player_remaining_constructor_owners::check_original(
        &owner.remaining().lock().unwrap(),
        &owner,
        &processed,
        &skeleton,
    );
    world
        .try_construct_pair_skeleton_controller(&board, &skeleton)
        .unwrap();
    world
        .try_construct_pair_physics_states(&board, &skeleton)
        .unwrap();
    remaining_states::check_original(&owner.remaining_states().lock().unwrap(), &owner);
    if finish_constructor {
        assert!(actor.try_publish_physical_constructor_return().is_err());
        world
            .try_finish_pair_hilod_constructor(
                &board,
                &skeleton,
                actor.as_ref(),
                &mut crate::reckoning_numeric::HostReckoningNumeric,
                true,
            )
            .unwrap();
        assert!(actor.cached_physical_allocation().is_err());
        actor.try_publish_physical_constructor_return().unwrap();
        assert!(Arc::ptr_eq(
            &actor.cached_physical_allocation().unwrap(),
            &registered
        ));
        let objects = world
            .physical_world_services()
            .unwrap()
            .try_physical_objects()
            .unwrap();
        assert_eq!(objects.len(), 1);
        assert!(Arc::ptr_eq(&objects[0], &registered));
        let tail = owner.tail().lock().unwrap();
        assert!(tail.returned && tail.world_added && tail.collide_with_world_only);
        assert_eq!(
            tail.intended_jump_velocity,
            Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
                [0.0; 4]
            ))
        );
        let capsule = tail.intersection_capsule.as_ref().unwrap();
        assert_eq!(
            capsule.transform.map(f32::to_bits),
            [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ]
            .map(f32::to_bits)
        );
        assert_eq!((capsule.radius, capsule.half_height), (1.0, 1.0));
        assert_eq!(
            (capsule.group_id, capsule.surface_id, capsule.flags),
            (0, 0, 1)
        );
        for line in tail.line_records.as_ref().unwrap() {
            assert_eq!(
                line.result.position,
                crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4])
            );
            assert_eq!(
                line.result.normal,
                crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4])
            );
            assert_eq!(line.result.surface_id, 0);
            assert!(!line.result.valid);
            assert_eq!(
                line.start_position,
                crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4])
            );
            assert!(line.trajectory_batch.is_none());
        }
        drop(tail);
        assert_eq!(
            owner.physics_state().lock().unwrap().current(),
            CurrentPhysicalState::Ground
        );
        assert!(actor.try_publish_physical_constructor_return().is_err());
        crate::actor_construction::action_runtime_session::corpus::check_published(
            &actor,
            &refs,
            &mut physical_animation.body_services,
            &mut physical_animation.input_services,
            &mut crate::simulation_assembly_backend::constraint_session::motion_force_physics::MotionForcePhysicsWriter::new(world, &skeleton),
        );
        if solved_fill {
            update1_corpus::check_mode(world, &board, &skeleton, actor.as_ref(), true);
            let selected = input_session.check_original_kickflip(
                &actor,
                &mut physical_animation.input_services,
                &refs,
                &input_vault,
                &world.debug_overlay,
            );
            assert_eq!(
                selected,
                [52, 52, 94],
                "actual conditioned Ground -> codec Kickflip/hold/release"
            );
            return;
        }
        update1_corpus::check(world, &board, &skeleton, actor.as_ref());
        input_session.check_original_kickflip(
            &actor,
            &mut physical_animation.input_services,
            &refs,
            &input_vault,
            &world.debug_overlay,
        );
        println!(
            "original same Actor/Skeleton Init/26 state constructors -> capsule/Ground/network collision -> actual World collection -> HiLOD suffix return -> SAME cached physical publication"
        );
        return;
    }
    world
        .try_start_pair_ground_state(
            &board,
            &skeleton,
            &mut actor.as_ref(),
            &refs,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .unwrap();
    {
        let state = owner.physics_state().lock().unwrap();
        assert_eq!(state.current(), CurrentPhysicalState::Ground);
        assert_eq!(state.stage(), PhysicalStateStage::EntryComplete);
    }
    let fields = *processed.lock().unwrap().physics_state_fields();
    assert_eq!(
        (
            fields.last,
            fields.last_category,
            fields.next,
            fields.current
        ),
        (Some(700), Some(700), Some(100), Some(100))
    );
    assert_eq!((fields.physics, fields.category), (Some(0), Some(0)));
    assert_eq!(
        owner.wipeout().lock().unwrap().state_bits(),
        Some((1, Some(0), 0, 0, 0))
    );
    assert_eq!(
        owner.skateboard_controller().lock().unwrap().system_on(),
        Some(false)
    );
    assert_eq!(
        owner
            .skeleton_controller()
            .lock()
            .unwrap()
            .current()
            .unwrap()
            .value,
        6
    );
    let normal = world.skeleton(&skeleton).unwrap();
    assert_eq!(normal.use_physics_truck_tilt(), Some(false));
    assert!(
        normal
            .current_animation()
            .ik_runtime()
            .unwrap()
            .state()
            .do_ik()
    );
    crate::simulation_assembly_backend::constraint_session::physical_state::manual_velocity::check_original_part_velocity(world, &board);
    crate::simulation_assembly_backend::constraint_session::system_logic::check_original_system_logic_prefix(world, &board, &skeleton);
    crate::simulation_assembly_backend::constraint_session::system_logic::original_corpus::check(
        world,
        &board,
        &skeleton,
        &mut refs,
        if enter_air {
            Some(actor.as_ref())
        } else {
            None
        },
    );
    if enter_air {
        // Selected first Air.Update collaborator, after actual Air.Enter.
        // This does not claim the remaining Air update/trajectory path.
        world
            .try_update_pair_air_collision(&board, &skeleton)
            .unwrap();
    }
    if !enter_air {
        crate::simulation_assembly_backend::constraint_session::system_logic::original_corpus::grounded::check(world, &board, &skeleton, &hook, &mut refs);
        if matches!(destination, Some(GroundDestination::Wipeout)) {
            // Exercise the actual SetPhysicsState API after a genuine grounded
            // Update2 initialized the request ledger. Destination is supplied
            // for this selected transition test; no live bail-input claim.
            world
                .try_transition_pair_ground(
                    &board,
                    &skeleton,
                    actor.as_ref(),
                    GroundDestination::Wipeout,
                    &refs,
                    None,
                    &mut crate::reckoning_numeric::HostReckoningNumeric,
                    &mut |angle: f32| Ok(angle.cos()),
                )
                .unwrap();
            let state = owner.physics_state().lock().unwrap();
            assert_eq!(state.current, CurrentPhysicalState::WipeoutGround);
            assert_eq!(state.stage, PhysicalStateStage::EntryComplete);
            let wipeout = &state.wipeout_ground.as_ref().unwrap().fields;
            assert_eq!(wipeout.is_impulse, Some(false));
            assert_eq!(wipeout.impulse_frames, Some(0));
            assert!(!wipeout.prediction.ready);
            drop(state);
            assert!(
                !world
                    .skeleton(&skeleton)
                    .unwrap()
                    .current_animation()
                    .ik_runtime()
                    .unwrap()
                    .state()
                    .do_ik()
            );
            let fields = *processed.lock().unwrap().physics_state_fields();
            assert_eq!(
                (
                    fields.last,
                    fields.last_category,
                    fields.next,
                    fields.current
                ),
                (Some(100), Some(100), Some(300), Some(300))
            );
            println!(
                "original grounded pair -> SAME registered Actor -> selected SetPhysicsState300 -> Ground.Exit/Wipeout.Enter complete; actual request ledger, body owners and source cvars, no live bail-input/frame-loop claim"
            );
        } else {
            crate::simulation_assembly_backend::constraint_session::physical_skeleton_controller::hip_hook_soft::check_original(world, &board, &skeleton);
            crate::simulation_assembly_backend::constraint_session::physical_skeleton_controller::ragdoll_corpus::check(world, &board, &skeleton, &refs);
            crate::simulation_assembly_backend::constraint_session::physical_board_forces::board_wipeout::check_original(world, &board);
            crate::simulation_assembly_backend::constraint_session::physical_state::ground_exit::check_original(world, &board, &skeleton);
        }
    }
    world
        .try_remove_pair_skateboard_controller_drives(&board, &skeleton, &mut refs)
        .unwrap();
    factory.try_teardown().unwrap();
    if destination.is_none() {
        crate::actor_animation_construction::channel_runtime::check_initialized_actor(&actor);
        crate::actor_animation_construction::channel_runtime::blend_tests::check_disposable_initialized_actor(&actor);
    }
    println!(
        "original Vault + SAME Actor evaluated bind pose + startup.ini retained cvars -> SAME board/skeleton/controllers -> actual Actor ID queries -> initial Sleeping700/Ground100 entry complete; controlled spawn/schedule, no Actor physical admission or continuous tick"
    );
}
