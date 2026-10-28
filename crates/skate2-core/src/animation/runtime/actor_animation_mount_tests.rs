use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;
use crate::{
    actor_construction::ActorChaseFormationOwner,
    game_input_player::recognizer::event_route::GameInputEventRoute,
    skeleton_root_transforms::SkeletonRootMatrix,
};

struct Selection {
    local: u32,
    network: bool,
    ghost: bool,
    marquee: u64,
    calls: Vec<&'static str>,
    fail: Option<&'static str>,
    managers: FaceAnimationManagers,
}
impl Selection {
    fn call(&mut self, name: &'static str) -> Result<(), &'static str> {
        self.calls.push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl ActorAnimationSelection for Selection {
    type Error = &'static str;
    fn local_player_index(&mut self) -> Result<u32, Self::Error> {
        self.call("local")?;
        Ok(self.local)
    }
    fn is_network_game(&mut self) -> Result<bool, Self::Error> {
        self.call("network")?;
        Ok(self.network)
    }
    fn ghost_system_enabled(&mut self) -> Result<bool, Self::Error> {
        self.call("ghost")?;
        Ok(self.ghost)
    }
    fn marquee_id(&mut self, _: u32) -> Result<u64, Self::Error> {
        self.call("marquee")?;
        Ok(self.marquee)
    }
    fn character_is_male(&mut self, _: u32) -> Result<bool, Self::Error> {
        self.call("male")?;
        Ok(true)
    }
    fn command_manager(&mut self) -> Result<Arc<Mutex<FaceCommandManager>>, Self::Error> {
        self.call("command")?;
        Ok(Arc::clone(&self.managers.command))
    }
    fn playback_manager(&mut self) -> Result<Arc<Mutex<FacePlaybackManager>>, Self::Error> {
        self.call("playback")?;
        Ok(Arc::clone(&self.managers.playback))
    }
    fn database_manager(&mut self) -> Result<Arc<Mutex<FaceDatabaseManager>>, Self::Error> {
        self.call("database")?;
        Ok(Arc::clone(&self.managers.database))
    }
}
fn fixture() -> (
    ActorConstruction,
    Arc<Mutex<PhysicalAssemblyWorld>>,
    Arc<AttributeRuntimeVault>,
) {
    let world = Arc::new(Mutex::new(
        PhysicalAssemblyWorld::try_new(2, 0, 0, 0).unwrap(),
    ));
    let (player, _, _manager) = crate::action_graph_input_listener::tests::gameplay();
    let actor = ActorConstruction::new(
        player,
        world.lock().unwrap().physical_manager_authority(),
        1,
        SkeletonRootMatrix::known(crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM),
    )
    .unwrap();
    let vault = super::super::tests::vault(4);
    let mut formation = ActorChaseFormationOwner::new(Arc::clone(&vault));
    formation.try_bind_selected_reset().unwrap();
    actor
        .try_construct_prefix(&formation, &mut HostReckoningNumeric)
        .unwrap();
    let mut route = GameInputEventRoute::new();
    actor.try_construct_listener(&mut route).unwrap();
    actor.try_construct_graphs().unwrap();
    (actor, world, vault)
}
#[test]
fn source_selection_short_circuits_and_registers_same_actor_animation_world() {
    for (local, network, ghost, calls, low) in [
        (99, false, false, vec!["local", "marquee", "male"], false),
        (
            2,
            true,
            false,
            vec!["local", "network", "marquee", "male"],
            false,
        ),
        (
            2,
            false,
            true,
            vec!["local", "network", "ghost", "marquee", "male"],
            false,
        ),
        (
            2,
            false,
            false,
            vec!["local", "network", "ghost", "marquee", "male"],
            true,
        ),
    ] {
        let (actor, world, vault) = fixture();
        let mut selection = Selection {
            local,
            network,
            ghost,
            marquee: u64::MAX,
            calls: vec![],
            fail: None,
            managers: FaceAnimationManagers::new(),
        };
        let retained = actor
            .try_construct_animation_prefix(&world, &vault, &mut selection)
            .unwrap();
        let mut calls = calls;
        calls.extend(["command", "playback", "database"]);
        assert_eq!(selection.calls, calls);
        let animation = retained.lock().unwrap();
        assert!(Arc::ptr_eq(&animation.inputs().physics_world, &world));
        assert!(
            animation
                .entity()
                .is_same_issuance(&actor.authority.entity_handle())
        );
        assert_eq!(
            animation.inputs().face_name.as_deref(),
            Some(&b"CAC_male"[..])
        );
        assert_eq!(animation.inputs().use_low_blend_cull_threshold, low);
        assert_eq!(animation.phase(), ActorAnimationPhase::VaultRequired);
        let prefix = actor.prefix().unwrap();
        assert!(Arc::ptr_eq(prefix.animation.as_ref().unwrap(), &retained));
        for occurrence in &prefix.animation_interfaces {
            assert!(Arc::ptr_eq(
                occurrence.as_ref().unwrap(),
                animation.occurrence()
            ));
        }
        assert!(!prefix.animation_published);
        assert!(prefix.fields().unwrap().animation_null);
        drop(prefix);
        drop(animation);
        assert!(matches!(
            actor.try_publish_animation_constructor(),
            Err(ActorAnimationMountError::ConstructorIncomplete)
        ));
    }
}
#[test]
fn missing_backend_retains_reset_allocation_but_foreign_world_has_no_effects() {
    let (actor, world, vault) = fixture();
    let mut selection = Selection {
        local: 2,
        network: false,
        ghost: false,
        marquee: 0,
        calls: vec![],
        fail: Some("marquee"),
        managers: FaceAnimationManagers::new(),
    };
    let foreign = Arc::new(Mutex::new(
        PhysicalAssemblyWorld::try_new(2, 0, 0, 0).unwrap(),
    ));
    assert!(matches!(
        actor.try_construct_animation_prefix(&foreign, &vault, &mut selection),
        Err(ActorAnimationMountError::ForeignWorld)
    ));
    assert!(selection.calls.is_empty());
    assert!(actor.prefix().unwrap().external_player_reset.is_none());
    assert!(matches!(
        actor.try_construct_animation_prefix(&world, &vault, &mut selection),
        Err(ActorAnimationMountError::Selection("marquee"))
    ));
    let prefix = actor.prefix().unwrap();
    assert!(prefix.external_player_reset.is_some());
    assert!(prefix.animation.is_none());
    assert_eq!(prefix.phase(), ActorPrefixPhase::AnimationSelecting);
}

#[test]
fn each_missing_animation_manager_retains_allocation_before_any_interface_effect() {
    for (failure, expected) in [
        (
            "command",
            vec!["local", "network", "ghost", "marquee", "male", "command"],
        ),
        (
            "playback",
            vec![
                "local", "network", "ghost", "marquee", "male", "command", "playback",
            ],
        ),
        (
            "database",
            vec![
                "local", "network", "ghost", "marquee", "male", "command", "playback", "database",
            ],
        ),
    ] {
        let (actor, world, vault) = fixture();
        let mut selection = Selection {
            local: 2,
            network: false,
            ghost: false,
            marquee: u64::MAX,
            calls: Vec::new(),
            fail: Some(failure),
            managers: FaceAnimationManagers::new(),
        };
        assert!(
            matches!(actor.try_construct_animation_prefix(&world, &vault, &mut selection),
            Err(ActorAnimationMountError::Selection(actual)) if actual == failure)
        );
        assert_eq!(selection.calls, expected);
        let prefix = actor.prefix().unwrap();
        assert!(prefix.external_player_reset.is_some());
        assert!(prefix.animation_interfaces.iter().all(Option::is_none));
        assert!(!prefix.animation_published);
        let animation = prefix.animation.as_ref().unwrap().lock().unwrap();
        assert_eq!(animation.phase(), ActorAnimationPhase::Unattempted);
        assert!(animation.base().is_none());
        assert!(animation.channel().is_none());
        assert!(animation.skater().is_none());
    }
}
