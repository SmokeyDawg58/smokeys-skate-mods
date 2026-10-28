use super::*;
use crate::{
    actor_face_playback::FaceAnimationManagers,
    animation_job_owners::AnimationJobManager,
    attribute_runtime::tests::graph,
    players_backend_settings::PlayersBackendSettings,
    reckoning_numeric::HostReckoningNumeric,
    simulation_animation_startup::mounted_source::MountedAnimationSource,
    simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo,
    skater_manager_local_identity::{LocalBackendOwner, SkaterManagerLocalIdentity},
};

#[test]
fn missing_input_mount_retains_selection_and_does_not_start_actor_or_retry() {
    let settings = PlayersBackendSettings::construct_fields();
    settings
        .try_set_character(2, 0xaaaa_bbbb_cccc_dddd)
        .unwrap();
    let mut loader = PlayerLoaderSlot::new(2);
    loader.set_marquee(0xaaaa_bbbb_cccc_dddd, crate::player::NO_PEER_ID);
    let cvars = InputCvarReferences::new_static_initialized();
    let mounts = Arc::new(MountSet::new());
    let mut retained = SelectedPlayerStartup::new(SelectedPlayerStartupOwners {
        mounts: Arc::clone(&mounts),
        focus: Arc::new(Mutex::new(
            crate::selected_sim_command_queue::focus::SelectedPlayerFocus::construct_fields(2),
        )),
        juice: crate::juice_trick_input::JuiceTrickInput::new_static_initialized(),
        pause: crate::selected_pause_manager::SelectedPauseManager::construct(),
        recipe_registry: None,
        recipe_load: None,
        living_world_recipes: Arc::new(Mutex::new(crate::selected_living_world_recipes::SelectedLivingWorldRecipes::new())),
        presentation: None,
        pedestrian_animation: Default::default(),
        loader,
        services: PlayerAttributeServices {
            profile: crate::players_backend_settings::create::ProfileNameOwner::construct_name(),
            vault: Arc::new(graph(0x91bc_a669_3efc_9aa7, vec![])),
            settings: Arc::clone(&settings),
            skaters: SkaterManagerLocalIdentity::construct_local_fields(
                LocalBackendOwner::construct_static_fields(
                    PhysicalGameInfo::construct_network_field(),
                ),
            ),
        },
        animation: SimulationAnimationStartup::new(FaceAnimationManagers::new()),
        faces: CharacterFaceManager::new(),
        cvars: cvars.clone(),
        graphs: None,
        loaded_graphs: None,
        formation: None,
        map: None,
    });
    let mut source = MountedAnimationSource::new(
        Arc::clone(&mounts),
        Arc::new(AnimationJobManager::new()),
        cvars,
        || Err::<u64, _>("must not sample"),
    );
    let world = Arc::new(Mutex::new(
        PhysicalAssemblyWorld::try_new(35, 28, 51, 0).unwrap(),
    ));
    let request = || SelectedPlayerConstructionRequest {
        spawn: SkeletonRootMatrix::known([0.0; 16]),
        actor_flags: 1,
    };
    assert!(matches!(
        retained.try_initialize(
            &world,
            &mounts,
            request(),
            &mut source,
            &mut HostReckoningNumeric
        ),
        Err(SelectedPlayerStartupError::Mount(
            MountError::EntryNotFound(_)
        ))
    ));
    assert!(retained.actor.is_none());
    assert!(retained.physical_animation.is_none());
    assert!(retained.owners.formation.is_none());
    assert!(Arc::ptr_eq(&settings, &retained.owners.services.settings));
    assert_eq!(retained.owners.loader.player_index(), 2);
    assert_eq!(settings.try_character(2).unwrap(), 0xaaaa_bbbb_cccc_dddd);
    assert!(matches!(
        retained.try_initialize(
            &world,
            &mounts,
            request(),
            &mut source,
            &mut HostReckoningNumeric
        ),
        Err(SelectedPlayerStartupError::AlreadyAttempted)
    ));
}
