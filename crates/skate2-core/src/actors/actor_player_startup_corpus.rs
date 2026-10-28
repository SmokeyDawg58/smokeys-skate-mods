//! Original-data driver for the shared mounted startup composition.
//! Test selection, thread identity and component-only graph loading stay explicit.
use crate::{
    actor_construction::{
        ActorConstruction, ActorIdentity,
        physical_animation_inputs::ActorPhysicalAnimationInputs,
        selected_startup::{SelectedPlayerConstructionRequest, SelectedPlayerStartupOwners},
    },
    attribute_runtime::AttributeRuntimeVault,
    game_input_frame_session::GameInputFrameSession,
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalAssemblyWorld,
    player::{DEFAULT_MARQUEE_ID, MarqueeDefinition},
    selected_session_player_startup::SelectedSessionPlayerStartup,
};
use std::sync::{Arc, Mutex};
pub(crate) type SelectedStartupPlayers = SelectedPlayerStartupOwners;

#[expect(
    clippy::too_many_lines,
    reason = "mounted driver retains original constructor and identity assertions"
)]
pub(crate) fn construct(
    vault: &Arc<AttributeRuntimeVault>,
    world: &Arc<Mutex<PhysicalAssemblyWorld>>,
    spawn: [f32; 16],
    cvars: &InputCvarReferences,
    native_roster: bool,
) -> (
    Arc<ActorConstruction>,
    ActorPhysicalAnimationInputs,
    GameInputFrameSession<ActorIdentity>,
    SelectedStartupPlayers,
) {
    let mounts =
        Arc::new(crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_corpus());
    // This is still a corpus-selected spawn and fixture thread answer. The
    // production constructor only retains the answer supplied by its host.
    let SelectedSessionPlayerStartup {
        selected: mut retained,
        source: mut startup_source,
        registration_request: _, // This controlled corpus does not run registration status.
    } = SelectedSessionPlayerStartup::try_new(
        Arc::clone(&mounts),
        world,
        Arc::clone(vault),
        cvars.clone(),
        DEFAULT_MARQUEE_ID,
        SelectedPlayerConstructionRequest {
            spawn: crate::skeleton_root_transforms::SkeletonRootMatrix::known(spawn),
            actor_flags: 1,
        },
        || Ok::<u64, std::convert::Infallible>(1),
    )
    .unwrap();
    assert!(Arc::ptr_eq(&mounts, &retained.owners.mounts));
    let player_index =
        u32::try_from(retained.owners.services.skaters.try_local_index().unwrap()).unwrap();
    assert_eq!(
        retained
            .owners
            .services
            .settings
            .try_player_type(player_index)
            .unwrap(),
        1
    );
    assert_eq!(
        retained
            .owners
            .services
            .settings
            .try_peer_id(player_index)
            .unwrap(),
        -1
    );
    let instance = vault.instance(0x91bc_a669_3efc_9aa7, DEFAULT_MARQUEE_ID);
    let name = instance.try_cached_layout_text(0).unwrap().unwrap();
    let definition = MarqueeDefinition {
        id: DEFAULT_MARQUEE_ID,
        schema_name: std::str::from_utf8(name.bytes()).unwrap().to_owned(),
    };
    assert!(matches!(retained.owners.loader.phase(),
        crate::player::PlayerLoadPhase::Registering {request_id:1,character_id}
        if character_id==definition.schema_asset_id()));
    let recipe_load = retained.owners.recipe_load.as_ref().unwrap();
    let scene = recipe_load.scene().unwrap();
    let native_instances = recipe_load
        .finalized()
        .unwrap()
        .instances(0)
        .unwrap()
        .collect::<Vec<_>>();
    assert!(!native_instances.is_empty());
    assert!(native_instances.iter().all(|instance| {
        scene
            .components
            .iter()
            .flat_map(|c| &c.models)
            .flat_map(|m| &m.lods)
            .flat_map(|l| &l.arena.instances)
            .any(|same| std::ptr::eq(*instance, same))
    }));
    assert!(retained.owners.loaded_graphs.is_none());
    if !native_roster {
        // Explicit component fixture: retain existing graph/tick coverage,
        // without claiming SpawnPlayer/observer or asset registration completion.
        retained
            .owners
            .try_load_player_graphs(retained.actor.as_ref().unwrap())
            .unwrap();
        assert_eq!(retained.owners.loaded_graphs, Some([true, true]));
    }
    assert!(Arc::ptr_eq(
        retained
            .owners
            .recipe_registry
            .as_ref()
            .unwrap()
            .scene(definition.schema_asset_id())
            .unwrap(),
        retained
            .owners
            .recipe_load
            .as_ref()
            .unwrap()
            .scene()
            .unwrap()
    ));
    let recipe = Arc::clone(
        retained
            .owners
            .recipe_load
            .as_ref()
            .unwrap()
            .recipe()
            .unwrap(),
    );
    assert_eq!(recipe.schema_name, definition.schema_name);
    retained
        .try_initialize_mounted_presentation(&mut startup_source)
        .unwrap();
    crate::actor_face_playback::presentation::startup::corpus::check(&retained.owners);
    let actor = retained.actor.take().unwrap();
    assert!(
        actor
            .retained_listener()
            .unwrap()
            .unwrap()
            .lock()
            .unwrap()
            .events()
            .is_empty()
    );
    (
        actor,
        retained.physical_animation.take().unwrap(),
        retained.input,
        retained.owners,
    )
}
