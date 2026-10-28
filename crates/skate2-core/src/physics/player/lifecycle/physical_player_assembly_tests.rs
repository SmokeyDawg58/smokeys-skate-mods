//! Controlled hierarchy/Vault inputs exercise the actual paired factory and
//! manager. No fabricated Entity, member handle or successful body definition.
use super::*;
use crate::{
    attribute_runtime::tests::{collection, definition, graph, node},
    skateboard_attributes::{
        SkateboardAttributeClass, SkateboardAttributeError, SkateboardAttributes,
    },
    skateboard_body_construction::SkateboardBodyConstructionPhase,
    skeleton_assembly_construction::SkeletonAssemblyConstructionPhase,
    skeleton_data::SkeletonDataError,
};

struct Services {
    flags: u32,
    reads: usize,
}
impl SkateboardSpawnServices for Services {
    fn current_orthogonal_tolerance(&mut self) -> f32 {
        panic!("failed definition cannot reach placement")
    }
    fn orthogonality_assertion_failed(&mut self) {
        panic!("no placement assertion")
    }
}
impl PhysicalPlayerAssemblyServices for Services {
    fn current_entity_flags(&mut self) -> u32 {
        self.reads += 1;
        self.flags
    }
}

fn malformed_vault() -> Arc<AttributeRuntimeVault> {
    // Deliberately wrong-size joint-enable value. GrindAir reads truck geometry
    // earlier now; this independent byte belongs to the later board constructor.
    let class = SkateboardAttributeClass::Trucks.key();
    let key = 0xb1cb_e3e9_29af_c8a0;
    let mut vault = graph(class, vec![definition(key, 4, 4)]);
    collection(
        &mut vault,
        class,
        SkateboardAttributes::DEFAULT_COLLECTION,
        0,
        None,
        vec![node(key, 0x40, 0x7f00_0000)],
    );
    Arc::new(vault)
}
fn inputs(vault: Arc<AttributeRuntimeVault>) -> PhysicalPlayerAssemblyInputs {
    // Supply only the constructor's required frame count to these controlled
    // failure fixtures. This is not original runtime tuning.
    let mut vault = Arc::try_unwrap(vault).expect("unbound fixture Vault");
    let class = crate::physical_player_grind_air_adjust::GRIND_AIR_CLASS;
    let key = 0x9759_3d0d_4f34_2621;
    crate::attribute_runtime::tests::add_fixture_class(
        &mut vault,
        class,
        vec![definition(key, 4, 4)],
    );
    collection(
        &mut vault,
        class,
        0xd7ed_bd36_2d7d_2152,
        0,
        None,
        vec![node(key, 0x40, 4)],
    );
    let vault = Arc::new(vault);
    let (hierarchy, parameters, matrices) = crate::skeleton_data_tests::fixture();
    let cvars = crate::input_cvar_runtime::InputCvarReferences::new_static_initialized();
    crate::input_cvar_runtime::InputCvarManager::new()
        .try_link(&cvars)
        .unwrap();
    PhysicalPlayerAssemblyInputs {
        cvars,
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
fn same_assets(input: &PhysicalPlayerAssemblyInputs) -> PhysicalPlayerAssemblyInputs {
    PhysicalPlayerAssemblyInputs {
        cvars: input.cvars.clone(),
        vault: Arc::clone(&input.vault),
        hierarchy: Arc::clone(&input.hierarchy),
        parameters: Arc::clone(&input.parameters),
        pose_hierarchy: Arc::clone(&input.pose_hierarchy),
        pose_matrices: Arc::clone(&input.pose_matrices),
        spawn: input.spawn,
        hat_type: input.hat_type,
        bouncy_ragdoll: input.bouncy_ragdoll,
        is_human: input.is_human,
    }
}
fn world() -> PhysicalAssemblyWorld {
    PhysicalAssemblyWorld::try_new(1, 0, 0, 0).unwrap()
}

#[test]
fn explicit_cleanup_of_failed_definition_retains_owner_and_forbids_construction_restart() {
    let mut world = world();
    let before = world.backend().rigid_body_snapshot();
    let mut factory =
        PhysicalPlayerAssemblyConstruction::new(&mut world, inputs(malformed_vault()));
    let owner = Arc::clone(factory.owner());
    let mut services = Services { flags: 1, reads: 0 };
    assert!(matches!(
        factory.try_construct_board(&mut services),
        Err(PhysicalPlayerAssemblyError::BoardDefinition(_))
    ));
    factory.try_teardown().unwrap();
    assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::Removed);
    assert_eq!(factory.world().backend().rigid_body_snapshot(), before);
    assert_eq!(factory.world().physical_body_registry().next_body_id(), 1);
    assert!(Arc::ptr_eq(factory.owner(), &owner));
    assert_eq!(
        factory.state.board_construction.as_ref().unwrap().phase(),
        SkateboardBodyConstructionPhase::EarlyAttributes
    );
    factory.try_teardown().unwrap();
    assert!(matches!(
        factory.try_construct_board(&mut services),
        Err(PhysicalPlayerAssemblyError::WrongPhase(
            PhysicalPlayerAssemblyPhase::Removed
        ))
    ));
    assert!(matches!(
        factory.try_construct_skeleton(&mut services, &mut || panic!("no hat")),
        Err(PhysicalPlayerAssemblyError::WrongPhase(
            PhysicalPlayerAssemblyPhase::Removed
        ))
    ));
    assert_eq!(services.reads, 1);
}

#[test]
fn actual_outer_constructor_entry_fields_survive_later_body_definition_failure() {
    let mut world = world();
    let mut factory =
        PhysicalPlayerAssemblyConstruction::new(&mut world, inputs(malformed_vault()));
    assert!(factory.state.board_force_runtime.is_none());
    let mut services = Services { flags: 1, reads: 0 };
    assert!(matches!(
        factory.try_construct_board(&mut services),
        Err(PhysicalPlayerAssemblyError::BoardDefinition(_))
    ));
    let outer = factory.state.board_force_runtime.as_ref().unwrap();
    assert!(!outer.entry.animated());
    assert_eq!(outer.entry.target_speed().to_bits(), 0);
    assert!(outer.hook_dynamics.is_none());
    assert!(factory.state.hook_construction.is_none());
    assert_eq!(
        outer.speed_wobble.phase(),
        crate::skateboard_force_runtime::speed_wobble::SkateboardSpeedWobblePhase::Constructed
    );
    assert_eq!(
        outer.speed_wobble.value().unwrap().state_bits(),
        (false, [0; 6], None)
    );
}

#[test]
fn selected_reckoning_failure_precedes_outer_board_and_retains_factory_attempt() {
    let class = crate::reckoning_attributes::RECKONING_CLASS;
    let mut data = graph(class, vec![]);
    collection(&mut data, class, 0xd7ed_bd36_2d7d_2152, 0, Some(64), vec![]);
    crate::attribute_runtime::tests::set_dependency_bytes(&mut data, 1, vec![0; 68]);
    let mut world = world();
    let mut factory = PhysicalPlayerAssemblyConstruction::new(&mut world, inputs(Arc::new(data)));
    let mut services = Services { flags: 1, reads: 0 };
    let processed = Arc::clone(factory.owner().reckoning().lock().unwrap().processed());
    assert!(matches!(
        factory.try_construct_board(&mut services),
        Err(PhysicalPlayerAssemblyError::Reckoning(_))
    ));
    assert_eq!(
        factory.phase(),
        PhysicalPlayerAssemblyPhase::SelectedReckoning
    );
    assert_eq!(services.reads, 0);
    assert!(factory.state.board_force_runtime.is_none());
    assert!(factory.owner().surface_physics().lock().unwrap().is_some());
    assert!(
        factory
            .owner()
            .reckoning()
            .lock()
            .unwrap()
            .reckoning()
            .is_none()
    );
    // PhysOut82e03f20 precedes the failing Reckoning constructor. Its nineteen
    // completed children survive on the same owner despite no physical Body.
    {
        let output = factory.owner().phys_out().lock().unwrap();
        assert!(output.complete_construction());
        assert!(output.component_statuses().iter().all(|status| {
            *status == crate::physical_player_phys_out::construct::ComponentStatus::Constructed
        }));
        assert_eq!(
            output
                .try_animation_output()
                .unwrap()
                .time_since_last_input
                .to_bits(),
            0
        );
    }
    assert!(Arc::ptr_eq(
        &processed,
        factory.owner().reckoning().lock().unwrap().processed()
    ));
    assert!(matches!(
        factory.try_construct_board(&mut services),
        Err(PhysicalPlayerAssemblyError::WrongPhase(
            PhysicalPlayerAssemblyPhase::SelectedReckoning
        ))
    ));
    factory.try_teardown().unwrap();
}

#[test]
fn surface_constructor_failure_retains_material_prefix_before_reckoning_and_body() {
    let class = crate::skateboard_slide_collision_attributes::SkateboardSlideCollisionVaultAttributes::SURFACE_CLASS;
    let mut vault = graph(class, vec![]);
    // Actual smooth collection, with a deliberately truncated cached layout.
    collection(
        &mut vault,
        class,
        0x8e61_e92e_8922_5fbe,
        0,
        Some(240),
        vec![],
    );
    let mut world = world();
    let mut factory = PhysicalPlayerAssemblyConstruction::new(&mut world, inputs(Arc::new(vault)));
    let mut services = Services { flags: 1, reads: 0 };
    assert!(matches!(
        factory.try_construct_board(&mut services),
        Err(PhysicalPlayerAssemblyError::SurfacePhysics(_))
    ));
    assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::SurfacePhysics);
    assert_eq!(services.reads, 0);
    assert!(factory.state.board_force_runtime.is_none());
    assert!(factory.state.surface_attributes.is_some());
    assert!(
        factory
            .owner()
            .phys_out()
            .lock()
            .unwrap()
            .complete_construction()
    );
    let materials = factory
        .owner()
        .surface_physics()
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .material_prefix();
    // All five restitution writes precede the failing first friction-layout read.
    for material in materials {
        assert_eq!(material.restitution.to_bits(), 0);
        assert_eq!(material.static_friction.to_bits(), 0.5_f32.to_bits());
        assert_eq!(material.dynamic_friction.to_bits(), 0.3_f32.to_bits());
    }
    assert!(matches!(
        factory.try_construct_board(&mut services),
        Err(PhysicalPlayerAssemblyError::WrongPhase(
            PhysicalPlayerAssemblyPhase::SurfacePhysics
        ))
    ));
}

#[test]
fn visual_routing_precedes_malformed_board_attribute_reads_and_cannot_replay() {
    for flags in [0, 2, 0x8000_0000, 0xffff_fffe] {
        let mut world = world();
        let before = world.backend().rigid_body_snapshot();
        let mut factory =
            PhysicalPlayerAssemblyConstruction::new(&mut world, inputs(malformed_vault()));
        assert!(factory.state.board_force_runtime.is_none());
        assert!(
            factory
                .owner()
                .reckoning()
                .lock()
                .unwrap()
                .reckoning()
                .is_none()
        );
        let mut services = Services { flags, reads: 0 };
        assert!(matches!(factory.try_construct_board(&mut services),
            Err(PhysicalPlayerAssemblyError::VisualIslandUnsupported { flags: actual }) if actual == flags));
        assert_eq!(services.reads, 1);
        let collision = Arc::clone(
            &factory
                .state
                .board_force_runtime
                .as_ref()
                .unwrap()
                .contact_surface,
        );
        assert!(
            factory
                .owner()
                .reckoning()
                .lock()
                .unwrap()
                .reckoning()
                .is_some()
        );
        {
            let state = collision.lock().unwrap();
            assert!(state.collision_state.reset_completed());
            assert_eq!(
                state
                    .collision_state
                    .last_valid_normal
                    .unwrap()
                    .xyz
                    .map(f32::to_bits),
                [0, 0x3f80_0000, 0]
            );
            assert_eq!(state.collision_state.relative_velocity, [None; 7]);
        }
        assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::BoardRouting);
        assert_eq!(
            factory.state.board_construction.as_ref().unwrap().phase(),
            SkateboardBodyConstructionPhase::Fresh
        );
        assert!(factory.state.board_candidate.is_none());
        assert!(factory.board().is_none());
        assert_eq!(factory.world().backend().rigid_body_snapshot(), before);
        assert_eq!(factory.world().physical_body_registry().next_body_id(), 1);
        services.flags = 1;
        assert!(matches!(
            factory.try_construct_board(&mut services),
            Err(PhysicalPlayerAssemblyError::WrongPhase(
                PhysicalPlayerAssemblyPhase::BoardRouting
            ))
        ));
        assert_eq!(services.reads, 1);
        assert!(matches!(
            factory.completed_world_mut(),
            Err(PhysicalPlayerAssemblyPhase::BoardRouting)
        ));
    }
}

#[test]
fn only_low_flag_bit_selects_simulation_and_real_scalar_error_keeps_definition_prefix() {
    for flags in [1, 3, 0x8000_0001, u32::MAX] {
        let mut world = world();
        let before = world.backend().rigid_body_snapshot();
        let mut factory =
            PhysicalPlayerAssemblyConstruction::new(&mut world, inputs(malformed_vault()));
        let mut services = Services { flags, reads: 0 };
        assert!(matches!(
            factory.try_construct_board(&mut services),
            Err(PhysicalPlayerAssemblyError::BoardDefinition(
                SkateboardBodyConstructionError::Attribute(SkateboardAttributeError::ValueSize {
                    expected: 1,
                    actual: 4
                })
            ))
        ));
        assert_eq!(services.reads, 1);
        assert_eq!(
            factory.phase(),
            PhysicalPlayerAssemblyPhase::BoardDefinition
        );
        let construction = factory.state.board_construction.as_ref().unwrap();
        assert_eq!(
            construction.phase(),
            SkateboardBodyConstructionPhase::EarlyAttributes
        );
        assert!(Arc::ptr_eq(
            construction.physical_owner().unwrap(),
            factory.owner()
        ));
        assert!(construction.island_body_id().is_none());
        assert!(factory.state.board_candidate.is_none());
        assert_eq!(factory.world().backend().rigid_body_snapshot(), before);
        services.flags = 0;
        assert!(matches!(
            factory.try_construct_board(&mut services),
            Err(PhysicalPlayerAssemblyError::WrongPhase(
                PhysicalPlayerAssemblyPhase::BoardDefinition
            ))
        ));
        assert_eq!(services.reads, 1);
        assert!(matches!(
            factory.completed_world_mut(),
            Err(PhysicalPlayerAssemblyPhase::BoardDefinition)
        ));
    }
}

#[test]
fn skeleton_before_completed_board_refuses_before_flags_hat_or_data() {
    let mut world = world();
    let mut input = inputs(Arc::new(graph(0, vec![])));
    Arc::get_mut(&mut input.parameters).unwrap().bones.clear();
    let mut factory = PhysicalPlayerAssemblyConstruction::new(&mut world, input);
    let mut services = Services { flags: 1, reads: 0 };
    assert!(matches!(
        factory.try_construct_skeleton(&mut services, &mut || panic!("no hat sampling")),
        Err(PhysicalPlayerAssemblyError::WrongPhase(
            PhysicalPlayerAssemblyPhase::Fresh
        ))
    ));
    assert_eq!(services.reads, 0);
    assert!(!factory.state.skeleton_started);
    assert!(factory.state.skeleton_construction.is_none());
    assert!(factory.skeleton().is_none());
    assert_eq!(factory.world().physical_body_registry().member_count(), 0);
    assert!(matches!(
        factory.completed_world_mut(),
        Err(PhysicalPlayerAssemblyPhase::Fresh)
    ));
}

#[test]
fn identical_asset_arcs_and_same_manager_still_issue_distinct_real_construction_owners() {
    let mut world = world();
    let before = world.backend().rigid_body_snapshot();
    let input = inputs(malformed_vault());
    let first = {
        let factory = PhysicalPlayerAssemblyConstruction::new(&mut world, same_assets(&input));
        Arc::clone(factory.owner())
    };
    let second = {
        let factory = PhysicalPlayerAssemblyConstruction::new(&mut world, same_assets(&input));
        Arc::clone(factory.owner())
    };
    assert!(!Arc::ptr_eq(&first, &second));
    assert_eq!(first.manager(), second.manager());
    assert_eq!(first.manager(), world.physical_manager_authority());
    assert!(Arc::ptr_eq(&first.inputs().vault, &second.inputs().vault));
    assert!(Arc::ptr_eq(
        &first.inputs().hierarchy,
        &second.inputs().hierarchy
    ));
    assert!(Arc::ptr_eq(
        &first.inputs().parameters,
        &second.inputs().parameters
    ));
    assert!(Arc::ptr_eq(
        &first.inputs().pose_hierarchy,
        &second.inputs().pose_hierarchy
    ));
    assert!(Arc::ptr_eq(
        &first.inputs().pose_matrices,
        &second.inputs().pose_matrices
    ));
    assert_eq!(world.backend().rigid_body_snapshot(), before);
    assert_eq!(world.physical_body_registry().member_count(), 0);
    assert_eq!(world.physical_body_registry().next_body_id(), 1);
}

#[test]
fn foreign_manager_skeleton_bridge_refuses_before_data_and_original_manager_reaches_it() {
    let mut original = world();
    let mut foreign = world();
    let mut input = inputs(malformed_vault());
    Arc::get_mut(&mut input.parameters).unwrap().bones.pop();
    let owner = {
        let factory = PhysicalPlayerAssemblyConstruction::new(&mut original, input);
        Arc::clone(factory.owner())
    };
    let mut construction = SkeletonAssemblyConstruction::new(owner);
    let before = foreign.backend().rigid_body_snapshot();
    assert!(matches!(
        foreign.try_construct_physical_skeleton(&mut construction, &mut || panic!("no hat read")),
        Err(PhysicalPlayerAssemblyError::Pool(
            SimulationSkaterPoolError::ForeignManager
        ))
    ));
    assert_eq!(
        construction.phase(),
        SkeletonAssemblyConstructionPhase::Fresh
    );
    assert!(construction.data().is_none());
    assert_eq!(foreign.backend().rigid_body_snapshot(), before);
    assert_eq!(foreign.physical_body_registry().next_body_id(), 1);
    assert!(matches!(
        original.try_construct_physical_skeleton(&mut construction, &mut || panic!("no hat read")),
        Err(PhysicalPlayerAssemblyError::SkeletonDefinition(
            SkeletonAssemblyConstructionError::Data(SkeletonDataError::PhysicsBoneCount {
                actual: 23
            })
        ))
    ));
    assert_eq!(
        construction.phase(),
        SkeletonAssemblyConstructionPhase::Data
    );
}

/// Actual factory-issued owner for focused `StateChanger` read-order tests.
pub(crate) fn issued_owner_for_grind_tests()
-> (PhysicalAssemblyWorld, Arc<PhysicalPlayerAssemblyOwner>) {
    let mut world = world();
    let owner = {
        let factory =
            PhysicalPlayerAssemblyConstruction::new(&mut world, inputs(malformed_vault()));
        Arc::clone(factory.owner())
    };
    (world, owner)
}

#[test]
fn partial_constructor_detaches_rejects_foreign_world_and_recovers_without_restart() {
    let mut original = world();
    let mut foreign = world();
    let mut factory =
        PhysicalPlayerAssemblyConstruction::new(&mut original, inputs(malformed_vault()));
    let owner = Arc::clone(factory.owner());
    let mut services = Services { flags: 1, reads: 0 };
    assert!(matches!(
        factory.try_construct_board(&mut services),
        Err(PhysicalPlayerAssemblyError::BoardDefinition(_))
    ));
    let phase = factory.phase();
    let board_prefix = factory.state.board_construction.as_ref().unwrap().phase();
    assert!(factory.state.board_started);
    let retained = factory.into_retained();
    let original_snapshot = original.backend().rigid_body_snapshot();
    let foreign_snapshot = foreign.backend().rigid_body_snapshot();
    let Err((PhysicalConstructionBindError::ForeignWorld, retained)) =
        retained.try_bind(&mut foreign)
    else {
        panic!("foreign world admitted");
    };
    assert_eq!(foreign.backend().rigid_body_snapshot(), foreign_snapshot);
    assert_eq!(original.backend().rigid_body_snapshot(), original_snapshot);
    let mut factory = retained
        .try_bind(&mut original)
        .unwrap_or_else(|_| panic!("same world rejected"));
    assert!(Arc::ptr_eq(factory.owner(), &owner));
    assert_eq!(factory.phase(), phase);
    assert_eq!(
        factory.state.board_construction.as_ref().unwrap().phase(),
        board_prefix
    );
    assert!(factory.state.board_started);
    assert!(matches!(
        factory.try_construct_board(&mut services),
        Err(PhysicalPlayerAssemblyError::WrongPhase(_))
    ));
    assert_eq!(services.reads, 1);
    factory.try_teardown().unwrap();
    let retained = factory.into_retained();
    let mut factory = retained
        .try_bind(&mut original)
        .unwrap_or_else(|_| panic!("removed state lost world identity"));
    assert_eq!(factory.phase(), PhysicalPlayerAssemblyPhase::Removed);
    factory.try_teardown().unwrap();
    assert!(Arc::ptr_eq(factory.owner(), &owner));
}
