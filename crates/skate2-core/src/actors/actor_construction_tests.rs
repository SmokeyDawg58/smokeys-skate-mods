use super::*;
use crate::{
    attribute_runtime::tests::{collection, definition, graph, node},
    reckoning_numeric::{HostReckoningNumeric, host_dot3},
    simulation_contact_jacobian::network_preparation::SimulationManagerContactDomain,
    simulation_rigid_body_pool::SimulationRigidBodyPool,
    skeleton_root_transforms::SkeletonRootVector,
};

pub(super) fn vault(max_size: u16) -> Arc<AttributeRuntimeVault> {
    let mut vault = graph(
        CHASE_CLASS,
        vec![
            definition(MIN_RADIUS, 4, 4),
            definition(MAX_RADIUS, 4, max_size),
            definition(VELOCITY_SCALE, 4, 4),
        ],
    );
    collection(
        &mut vault,
        CHASE_CLASS,
        skate2_formats::attrib::string_hash64("global"),
        0,
        None,
        vec![
            node(MIN_RADIUS, 0x40, 2.0_f32.to_bits()),
            node(MAX_RADIUS, 0x40, 7.0_f32.to_bits()),
            node(VELOCITY_SCALE, 0x40, 3.0_f32.to_bits()),
        ],
    );
    Arc::new(vault)
}
fn formation(size: u16) -> ActorChaseFormationOwner {
    let mut result = ActorChaseFormationOwner::new(vault(size));
    result.try_bind_selected_reset().unwrap();
    result
}
fn spawn(position: [f32; 3]) -> SkeletonRootMatrix {
    SkeletonRootMatrix::from_columns([
        SkeletonRootVector::known([1.0, 0.0, 0.0, f32::NAN]),
        SkeletonRootVector::known([0.0, 1.0, 0.0, f32::NAN]),
        SkeletonRootVector::known([0.0, 0.0, 1.0, f32::NAN]),
        SkeletonRootVector {
            xyz: position,
            w: None,
        },
    ])
}
fn actor(matrix: SkeletonRootMatrix) -> ActorConstruction {
    let (player, _, _command_manager) = crate::action_graph_input_listener::tests::gameplay();
    let mut bodies = SimulationRigidBodyPool::try_new(2).unwrap();
    let manager = SimulationManagerContactDomain::try_new(&mut bodies, 2).unwrap();
    ActorConstruction::new(player, manager.assembly_pool_authority(), 1, matrix).unwrap()
}

pub(crate) fn actor_with_listener() -> (ActorConstruction, GameInputEventRoute<ActorIdentity>) {
    let actor = actor(spawn([0.0; 3]));
    actor
        .try_construct_prefix(&formation(4), &mut HostReckoningNumeric)
        .unwrap();
    let mut route = GameInputEventRoute::new();
    actor.try_construct_listener(&mut route).unwrap();
    (actor, route)
}

#[test]
fn graph_facades_mount_on_same_actor_registry_and_preserve_listener() {
    let (actor, route) = actor_with_listener();
    let listener = actor.retained_listener().unwrap().unwrap();
    actor.try_construct_graphs().unwrap();
    let prefix = actor.prefix().unwrap();
    assert_eq!(prefix.phase(), ActorPrefixPhase::GraphFacadesConstructed);
    assert_eq!(
        prefix.interface_names(),
        [
            0x825e_be28,
            0x825e_be38,
            0x825e_be58,
            0x825e_be68,
            0x825e_be78,
            0x825e_be88,
            0x825e_c090,
            0x825e_c0a0
        ]
    );
    let graphs = prefix.graphs().unwrap();
    let action = graphs.action().unwrap();
    let motion = graphs.motion().unwrap();
    assert!(Arc::ptr_eq(
        prefix.graph_interfaces[0].as_ref().unwrap(),
        action.occurrence()
    ));
    assert!(Arc::ptr_eq(
        prefix.graph_interfaces[1].as_ref().unwrap(),
        motion.occurrence()
    ));
    assert!(Arc::ptr_eq(
        prefix.graph_interfaces[2].as_ref().unwrap(),
        motion.occurrence()
    ));
    assert!(!Arc::ptr_eq(action.occurrence(), motion.occurrence()));
    assert!(action.controller.as_ref().unwrap().is_unloaded());
    assert!(motion.controller.as_ref().unwrap().is_unloaded());
    assert!(prefix.action_graph_published && prefix.motion_graph_published);
    assert!(!prefix.fields().unwrap().action_graph_null);
    assert!(!prefix.fields().unwrap().motion_graph_null);
    assert!(prefix.fields().unwrap().animation_null);
    drop(prefix);
    assert!(Arc::ptr_eq(
        &listener,
        &actor.retained_listener().unwrap().unwrap()
    ));
    assert_eq!(route.registered_handler_count_for_test(), 3);
    assert!(matches!(
        actor.try_construct_graphs(),
        Err(ActorConstructionError::AlreadyAttempted)
    ));
}

#[test]
fn graph_outer_null_is_published_and_does_not_skip_later_motion_constructor() {
    let (actor, _route) = actor_with_listener();
    actor
        .try_construct_graphs_with_allocations([
            ActorGraphAllocation::Null,
            ActorGraphAllocation::Host,
            ActorGraphAllocation::Host,
            ActorGraphAllocation::Null,
        ])
        .unwrap();
    let prefix = actor.prefix().unwrap();
    let graphs = prefix.graphs().unwrap();
    assert!(graphs.action().is_none());
    assert!(graphs.motion().unwrap().controller.is_none());
    assert!(prefix.fields().unwrap().action_graph_null);
    assert!(!prefix.fields().unwrap().motion_graph_null);
    assert!(prefix.action_graph_published && prefix.motion_graph_published);
    assert!(
        !prefix
            .interface_names()
            .contains(&ActorGraphInterface::Action.source_name())
    );
    assert!(
        prefix
            .interface_names()
            .contains(&ActorGraphInterface::SkaterMotion.source_name())
    );
}

#[test]
fn late_motion_registration_failure_retains_action_publication_and_motion_base() {
    let (actor, _route) = actor_with_listener();
    let duplicate = ActorGraphInterface::SkaterMotion.source_name();
    actor
        .prefix()
        .unwrap()
        .register::<std::convert::Infallible>(duplicate)
        .unwrap();
    assert!(
        matches!(actor.try_construct_graphs(), Err(ActorConstructionError::Graph(
        ActorGraphConstructionError::Registry(ActorGraphRegistrationError::DuplicateInterface(name))
    )) if name == duplicate)
    );
    let prefix = actor.prefix().unwrap();
    assert_eq!(prefix.phase(), ActorPrefixPhase::GraphConstructing);
    assert!(prefix.action_graph_published);
    assert!(!prefix.motion_graph_published);
    assert!(!prefix.fields().unwrap().action_graph_null);
    assert!(prefix.fields().unwrap().motion_graph_null);
    let graphs = prefix.graphs().unwrap();
    assert!(graphs.action().unwrap().is_complete());
    assert!(!graphs.motion().unwrap().is_complete());
    assert!(graphs.motion().unwrap().controller.is_some());
    assert!(graphs.motion().unwrap().filtered_intents.is_none());
    assert!(prefix.graph_interfaces[1].is_some());
    assert!(prefix.graph_interfaces[2].is_none());
    drop(prefix);
    assert!(matches!(
        actor.try_construct_graphs(),
        Err(ActorConstructionError::AlreadyAttempted)
    ));
}

#[test]
fn graph_registry_rejects_foreign_entity_before_any_insertion() {
    let (actor, _route) = actor_with_listener();
    let (foreign, _foreign_route) = actor_with_listener();
    let mut prefix = actor.prefix().unwrap();
    let prior = prefix.interface_names().to_vec();
    let mut registry = ActorGraphRegistration {
        authority: &actor.authority,
        state: &mut prefix,
    };
    assert_eq!(
        registry.register_graph_interface(
            &foreign.authority.entity_handle(),
            ActorGraphInterface::Action,
            &Arc::new(())
        ),
        Err(ActorGraphRegistrationError::ForeignEntity)
    );
    assert_eq!(prefix.interface_names(), prior);
    assert!(prefix.graph_interfaces.iter().all(Option::is_none));
}

#[test]
fn graph_construction_requires_published_listener() {
    let actor = actor(spawn([0.0; 3]));
    actor
        .try_construct_prefix(&formation(4), &mut HostReckoningNumeric)
        .unwrap();
    assert!(matches!(
        actor.try_construct_graphs(),
        Err(ActorConstructionError::GraphPrefixIncomplete)
    ));
    assert!(actor.prefix().unwrap().graphs().is_none());
}

#[test]
fn reset_binding_failure_retains_assigned_null_instance_and_consumes_attempt() {
    let mut owner = ActorChaseFormationOwner::new(Arc::new(graph(CHASE_CLASS, vec![])));
    assert!(matches!(
        owner.try_bind_selected_reset(),
        Err(ActorConstructionError::MissingChaseCollection)
    ));
    assert!(owner.instance.as_ref().unwrap().is_null());
    assert!(matches!(
        owner.try_bind_selected_reset(),
        Err(ActorConstructionError::AlreadyAttempted)
    ));
}

#[test]
fn chase_failure_retains_registered_prefix_and_first_scalar_without_actor_fields() {
    let actor = actor(spawn([0.0; 3]));
    assert!(matches!(
        actor.try_construct_prefix(&formation(1), &mut HostReckoningNumeric),
        Err(ActorConstructionError::Attribute(
            AttributeRuntimeError::ScalarSize { size: 1 }
        ))
    ));
    let prefix = actor.prefix().unwrap();
    assert_eq!(
        prefix.interface_names(),
        [0x825e_be28, 0x825e_be38, 0x825e_be58, 0x825e_be68]
    );
    assert_eq!(prefix.min_radius.unwrap().to_bits(), 2.0_f32.to_bits());
    assert!(prefix.max_radius.is_none());
    assert!(prefix.block_radius.is_none());
    assert!(prefix.fields().is_none());
    assert!(prefix.own_chase.is_none());
    drop(prefix);
    assert!(matches!(
        actor.authority_for_listener(),
        Err(ActorConstructionError::ListenerPrefixIncomplete)
    ));
    assert!(matches!(
        actor.try_construct_prefix(&formation(4), &mut HostReckoningNumeric),
        Err(ActorConstructionError::AlreadyAttempted)
    ));
}

#[test]
fn successful_prefix_retains_real_owner_one_authority_and_source_fields() {
    let formation = formation(4);
    let actor = actor(spawn([2.0, -3.0, 4.0]));
    actor
        .try_construct_prefix(&formation, &mut HostReckoningNumeric)
        .unwrap();
    let authority = actor.authority_for_listener().unwrap();
    assert!(std::ptr::eq(
        authority,
        actor.authority_for_listener().unwrap()
    ));
    assert_eq!(
        authority
            .entity_key()
            .gameplay()
            .lock()
            .unwrap()
            .player_index(),
        2
    );
    assert_eq!(
        authority.entity_key().manager(),
        actor.authority.entity_key().manager()
    );
    let prefix = actor.prefix().unwrap();
    assert_eq!(prefix.phase(), ActorPrefixPhase::ListenerRequired);
    assert_eq!(
        prefix.interface_names(),
        [
            0x825e_be28,
            0x825e_be38,
            0x825e_be58,
            0x825e_be68,
            0x825e_be78
        ]
    );
    let chasee=prefix.chasee.as_ref().unwrap();
    assert!(chasee.chasers.is_empty());
    assert!(prefix.cached_interfaces_null);
    assert_eq!(prefix.flank, Some(-1));
    assert_eq!(chasee.block_radius.to_bits(), 2.0_f32.to_bits());
    assert_eq!(chasee.formation_max_radius.to_bits(), 7.0_f32.to_bits());
    assert_eq!(chasee.formation_velocity_scalar.to_bits(), 3.0_f32.to_bits());
    assert_eq!([chasee.scared_timer,chasee.nearby_event_timer].map(f32::to_bits), [0; 2]);
    assert_eq!(chasee.group_end_reason, -1);
    assert!(!std::ptr::eq(
        prefix.own_chase.as_ref().unwrap(),
        formation.instance.as_ref().unwrap()
    ));
    let fields = prefix.fields().unwrap();
    assert_eq!(
        (
            fields.player_index,
            fields.flags,
            fields.set_transform_count,
            fields.teleport_count
        ),
        (2, 1, 0, 0)
    );
    assert_eq!(fields.time_since_fade_out.to_bits(), f32::MAX.to_bits());
    assert_eq!(
        [
            fields.time_since_fade_in,
            fields.sound_level,
            fields.sound_timer,
            fields.grind_sound_timer
        ]
        .map(f32::to_bits),
        [0; 4]
    );
    assert_eq!(fields.packed_known_zero_mask, 0xfe00_0000);
    assert!(fields.last_foot_forward_right && fields.protector_invalid);
    assert!(
        fields.action_graph_null
            && fields.motion_graph_null
            && fields.animation_null
            && fields.breadcrumbs_null
            && fields.ai_controller_null
    );
}

struct Numeric {
    calls: Vec<&'static str>,
    fail: bool,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.calls.push("dot");
        if self.fail {
            Err("dot")
        } else {
            Ok(host_dot3(a, b))
        }
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.calls.push("rsqrt");
        Ok(1.0 / value.sqrt())
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("not source")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("not source")
    }
}
#[test]
fn zero_position_still_calls_estimate_and_unknown_or_nan_w_is_ignored() {
    let mut numeric = Numeric {
        calls: vec![],
        fail: false,
    };
    validate_spawn(&spawn([0.0, -0.0, 0.0]), &mut numeric).unwrap();
    assert_eq!(numeric.calls, ["dot", "rsqrt"]);
    let mut columns = *spawn([0.0; 3]).columns();
    columns[0].xyz[0] = f32::INFINITY;
    validate_spawn(&SkeletonRootMatrix::from_columns(columns), &mut numeric).unwrap();
    columns[1].xyz[2] = f32::NAN;
    numeric.calls.clear();
    assert!(matches!(
        validate_spawn(&SkeletonRootMatrix::from_columns(columns), &mut numeric),
        Err(ActorConstructionError::MatrixNanAssertion)
    ));
    assert!(numeric.calls.is_empty());
}
#[test]
fn position_bound_is_strict_and_numeric_failure_retains_late_prefix() {
    assert!(matches!(
        validate_spawn(&spawn([100_000.0, 0.0, 0.0]), &mut HostReckoningNumeric),
        Err(ActorConstructionError::PositionLengthAssertion)
    ));
    assert!(matches!(
        validate_spawn(&spawn([f32::INFINITY, 0.0, 0.0]), &mut HostReckoningNumeric),
        Err(ActorConstructionError::PositionLengthAssertion)
    ));
    let actor = actor(spawn([0.0; 3]));
    let mut numeric = Numeric {
        calls: vec![],
        fail: true,
    };
    assert!(matches!(
        actor.try_construct_prefix(&formation(4), &mut numeric),
        Err(ActorConstructionError::Numeric("dot"))
    ));
    assert_eq!(
        actor.prefix().unwrap().phase(),
        ActorPrefixPhase::MatrixValidation
    );
    assert!(actor.prefix().unwrap().fields().is_some());
    assert!(matches!(
        actor.authority_for_listener(),
        Err(ActorConstructionError::ListenerPrefixIncomplete)
    ));
}

#[test]
fn native_id_assertion_is_after_chase_reset_and_fields_but_before_numeric() {
    let mut actor = actor(spawn([0.0; 3]));
    // Inject the constructor argument only to discriminate the native stage;
    // production captures this immutable value from its actual gameplay owner.
    actor.player_index = 9;
    let mut numeric = Numeric {
        calls: vec![],
        fail: false,
    };
    assert!(matches!(
        actor.try_construct_prefix(&formation(4), &mut numeric),
        Err(ActorConstructionError::PlayerIndexAssertion(9))
    ));
    let prefix = actor.prefix().unwrap();
    assert_eq!(prefix.chasee.as_ref().unwrap().group_end_reason, -1);
    assert_eq!(prefix.fields().unwrap().player_index, 9);
    assert_eq!(prefix.interface_names().len(), 5);
    assert!(numeric.calls.is_empty());
}

#[test]
fn actor_retains_same_listener_and_issuance_through_move_and_publication() {
    let actor = actor(spawn([0.0; 3]));
    actor
        .try_construct_prefix(&formation(4), &mut HostReckoningNumeric)
        .unwrap();
    let identity = actor.authority_for_listener().unwrap().entity_handle();
    let mut route = GameInputEventRoute::new();
    let listener = actor.try_construct_listener(&mut route).unwrap();
    let weak = Arc::downgrade(&listener);
    assert!(Arc::ptr_eq(
        &listener,
        &actor.retained_listener().unwrap().unwrap()
    ));
    assert!(listener.lock().unwrap().actor().is_same_issuance(&identity));
    assert!(Arc::ptr_eq(
        listener.lock().unwrap().player(),
        actor.authority.entity_key().gameplay()
    ));
    assert_eq!(
        actor.prefix().unwrap().phase(),
        ActorPrefixPhase::GraphRequired
    );
    assert!(actor.prefix().unwrap().listener_published);
    assert_eq!(route.registered_handler_count_for_test(), 3);
    assert!(matches!(
        actor.try_construct_listener(&mut route),
        Err(ActorConstructionError::AlreadyAttempted)
    ));
    drop(listener);
    let moved_actor = Box::new(actor);
    assert!(
        weak.upgrade()
            .unwrap()
            .lock()
            .unwrap()
            .actor()
            .is_same_issuance(&identity)
    );
    let registration = moved_actor
        .retained_listener()
        .unwrap()
        .unwrap()
        .lock()
        .unwrap()
        .registration()
        .unwrap()
        .clone();
    route.try_unregister(&registration).unwrap();
    drop(moved_actor);
    assert!(
        weak.upgrade().is_none(),
        "route is weak and authority does not cycle back to Actor"
    );
}

#[test]
fn late_registration_failure_retains_actor_owned_prefix_for_explicit_cleanup() {
    let actor = actor(spawn([0.0; 3]));
    let mut route = GameInputEventRoute::new();
    assert!(matches!(
        actor.try_construct_listener(&mut route),
        Err(ActorConstructionError::ListenerPrefixIncomplete)
    ));
    actor
        .try_construct_prefix(&formation(4), &mut HostReckoningNumeric)
        .unwrap();
    route.fail_registration_at_for_test(2);
    assert!(matches!(
        actor.try_construct_listener(&mut route),
        Err(ActorConstructionError::ListenerRoute(
            GameInputRouteError::Allocation
        ))
    ));
    assert_eq!(
        actor.prefix().unwrap().phase(),
        ActorPrefixPhase::ListenerConstructing
    );
    assert!(!actor.prefix().unwrap().listener_published);
    let partial = actor.retained_listener().unwrap().unwrap();
    assert!(!partial.lock().unwrap().is_constructed());
    assert_eq!(route.registered_handler_count_for_test(), 2);
    let registration = partial.lock().unwrap().registration().unwrap().clone();
    route.try_unregister(&registration).unwrap();
    assert_eq!(route.registered_handler_count_for_test(), 0);
    assert!(matches!(
        actor.try_construct_listener(&mut route),
        Err(ActorConstructionError::AlreadyAttempted)
    ));
    assert!(Arc::ptr_eq(
        &partial,
        &actor.retained_listener().unwrap().unwrap()
    ));
}
