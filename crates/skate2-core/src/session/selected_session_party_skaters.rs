//! `SkaterManager::SpawnPlayer` 0x82600630 (command 3) for players after the first. A later
//! skater owns its `PlayerLoader` member, recipe, Actor, physical pair and `SkaterPresEntity`
//! presentation; it shares the first player's input manager, animation banks, face manager,
//! recipe registry, graphs, chase formation, action services and back-end rows.
use super::SelectedSession;
use crate::{
    actor_construction::{
        ActorConstruction, breadcrumbs::ActorBreadCrumbConstructorOwners,
        input_generation::runtime::ActorInputRuntimeServices,
        physical_animation_inputs::{ActorPhysicalAnimationInputs, PhysicalAnimationInputsError},
        player_animation::ActorAnimationPlayerServices,
    },
    actor_face_playback::presentation::{
        frame::selected_runtime::{SelectedPresentationPolicy, SelectedPresentationRuntime},
        startup::SelectedPresentationStartup,
    },
    player::{
        CharacterRegistrationRequest, MarqueeDefinition, PlayerLoadAction, PlayerLoaderSlot,
        attributes_runtime::PlayerAttributeServices,
        mounted_recipe::{MountedPlayerRecipeLoad, PreparedRecipe},
    },
    reckoning_numeric::HostReckoningNumeric,
    selected_session_physical_pair_startup::{
        SelectedSessionPhysicalPair, try_construct_selected_physical_pair,
    },
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::sync::Arc;

/// `Character::CharactersMarquee`.
const CHARACTERS_MARQUEE: u64 = 0x91bc_a669_3efc_9aa7;

/// The marquee collection's definition: its schema is the layout's text 0.
pub(crate) fn marquee_definition(services: &PlayerAttributeServices, marquee: u64) -> Result<MarqueeDefinition, String> {
    let instance = services.vault.instance(CHARACTERS_MARQUEE, marquee);
    let name = instance
        .try_cached_layout_text(0)
        .map_err(|e| error("marquee", e))?
        .ok_or("party skater: marquee schema name absent")?;
    Ok(MarqueeDefinition {
        id: marquee,
        schema_name: std::str::from_utf8(name.bytes()).map_err(|e| error("marquee", e))?.to_owned(),
    })
}

/// The first player's boot answers every later spawn repeats.
#[derive(Clone, Copy)]
pub(crate) struct SkaterSpawnPolicy {
    pub(crate) actor_flags: u32,
    pub(crate) capsule_resource_allocated: bool,
    pub(crate) presentation: SelectedPresentationPolicy,
}

/// The skater a later player's `PlayerLoader` member loads.
pub(crate) enum PartyCharacter<'a> {
    Marquee(u64),
    /// The profile's created skater, whose parts record `PlayerLoaderImpl::Load` 0x8281aa80's
    /// CAS branch reads.
    Created(&'a skate2_formats::skater_profile::CacRecord),
}

/// One later skater's own owners.
pub(crate) struct PartySkater {
    pub(crate) loader: PlayerLoaderSlot,
    pub(crate) recipe_load: MountedPlayerRecipeLoad,
    pub(crate) actor: Arc<ActorConstruction>,
    pub(crate) animation: ActorPhysicalAnimationInputs,
    pub(crate) physical: SelectedSessionPhysicalPair,
    pub(crate) presentation: SelectedPresentationStartup,
    pub(crate) presentation_runtime: SelectedPresentationRuntime,
    pub(crate) registration: CharacterRegistrationRequest,
}

impl PartySkater {
    pub(crate) fn player_index(&self) -> u32 {
        self.loader.player_index()
    }
}

fn error(stage: &str, cause: impl std::fmt::Debug) -> String {
    format!("party skater {stage}: {cause:?}")
}

impl<T, C> SelectedSession<T, C> {
    /// The skaters after the first.
    pub(crate) fn party_skaters(&self) -> &[PartySkater] {
        &self.party
    }

    /// The later skater the local index (`SkaterManager::GetLocalIndex`) names on its turn;
    /// none while the first player's skater is local.
    pub(crate) fn local_party_skater(&self) -> Result<Option<&PartySkater>, String> {
        if self.party.is_empty() {
            return Ok(None);
        }
        self.local_skater_among(&self.party)
    }

    /// `SkaterPresentation::GetLocalIndex` 0x827d0090: the skater whose presentation carries
    /// the local flag, the actor at `SkaterManager::GetLocalIndex` (`IsLocal`). A challenge's
    /// `SetLocalIndex` hands it to the AI skater it spectates (a S.K.A.T.E. turn); none while
    /// the first player's skater is local.
    pub(crate) fn local_presented_skater(&self) -> Result<Option<&PartySkater>, String> {
        if self.party.is_empty() && self.ai_skaters.is_empty() {
            return Ok(None);
        }
        if let Some(skater) = self.local_skater_among(&self.party)? {
            return Ok(Some(skater));
        }
        self.local_skater_among(&self.ai_skaters)
    }

    /// The presentation `SkaterPresentation::GetLocalIndex` names, which the ICE camera anchors
    /// read (`User_ICECameraAnchor::GetPosition` 0x826cfd04): the local presented skater's, else
    /// the first player's.
    pub(crate) fn local_index_presentation(&self) -> Result<Option<&SelectedPresentationRuntime>, String> {
        Ok(match self.local_presented_skater()? {
            Some(skater) => Some(&skater.presentation_runtime),
            None => self.presentation_runtime.as_ref(),
        })
    }

    fn local_skater_among<'a>(&self, skaters: &'a [PartySkater]) -> Result<Option<&'a PartySkater>, String> {
        let identity = self
            .world
            .as_ref()
            .ok_or("party skater: world absent")?
            .world
            .lock()
            .map_err(|_| "party skater: world lock poisoned")?
            .local_skater_identity
            .clone();
        let Some(identity) = identity else { return Ok(None) };
        let index = identity.try_local_index().map_err(|e| error("local index", e))?;
        let Some(actor) = identity.try_find_by_index(index).map_err(|e| error("local skater", e))? else {
            return Ok(None);
        };
        Ok(skaters.iter().find(|skater| Arc::ptr_eq(&skater.actor, &actor)))
    }

    /// `StateGameUnload::UnloadGameState` 0x82757cc8 for the later skaters. `ResetGame`'s
    /// `SkaterManager::Reset` 0x82600410 destroys each player (`DestroyPlayer` 0x825fff80),
    /// whose Actor's destructor 0x825f9668 releases its input listener and its physical
    /// player: the skateboard controller's drives, then `PhysicalPlayerHiLOD`'s skeleton,
    /// hook and board, and its place in the World's physical collection (`Simulation::Remove`).
    /// `PlayerLoader::UnloadPartyMember` 0x828182f0 then unloads each member's character.
    pub(crate) fn try_destroy_party_skaters(&mut self) -> Result<(), String> {
        let party = std::mem::take(&mut self.party);
        self.try_destroy_later_skaters(party)
    }

    /// `DestroyPlayer` and `UnloadPartyMember` for later skaters taken from the list.
    pub(crate) fn try_destroy_later_skaters(&mut self, skaters: Vec<PartySkater>) -> Result<(), String> {
        if skaters.is_empty() {
            return Ok(());
        }
        let assets = self.assets.as_ref().ok_or("party skater: assets absent")?;
        let world_owner = &self.world.as_ref().ok_or("party skater: world absent")?.world;
        let primary = self.player.as_mut().ok_or("party skater: first player absent")?;
        let roster = Arc::clone(
            world_owner
                .lock()
                .map_err(|_| "party skater: world lock poisoned")?
                .entity_roster
                .as_ref()
                .ok_or("party skater: entity roster absent")?,
        );
        let mut members = Vec::with_capacity(skaters.len());
        for skater in skaters {
            let player = i32::try_from(skater.player_index()).map_err(|e| error("player", e))?;
            primary
                .selected
                .owners
                .services
                .skaters
                .try_destroy_player(player, &roster)
                .map_err(|e| error("destroy", e))?;
            if let Some(listener) = skater.actor.retained_listener().map_err(|e| error("listener", e))? {
                let registration = listener.lock().map_err(|_| "party skater: listener lock poisoned")?.registration.clone();
                if let Some(registration) = registration {
                    primary
                        .selected
                        .input
                        .gameplay
                        .route_mut()
                        .try_unregister(&registration)
                        .map_err(|e| error("listener route", e))?;
                }
            }
            let SelectedSessionPhysicalPair { pair, _retained: retained, .. } = skater.physical;
            {
                let mut world = world_owner.lock().map_err(|_| "party skater: world lock poisoned")?;
                let mut references = assets.cvars.clone();
                world
                    .try_remove_pair_skateboard_controller_drives(&pair.board, &pair.skeleton, &mut references)
                    .map_err(|e| error("controller drives", e))?;
                retained
                    .try_bind(&mut world)
                    .map_err(|(e, _)| error("physical bind", e))?
                    .try_teardown()
                    .map_err(|e| error("physical teardown", e))?;
                let physical = skater.actor.cached_physical_allocation().map_err(|e| error("physical", e))?;
                physical
                    .owner()
                    .world_services()
                    .ok_or("party skater: physical World services absent")?
                    .try_remove(&physical)
                    .map_err(|e| error("physical World services", e))?;
                physical.owner().tail().lock().map_err(|_| "party skater: HiLOD tail lock poisoned")?.world_added = false;
            }
            if let Some(animation) = skater.actor.retained_animation().map_err(|e| error("animation", e))? {
                animation
                    .lock()
                    .map_err(|_| "party skater: animation lock poisoned")?
                    .try_purge_face_pose()
                    .map_err(|e| error("face pose", e))?;
            }
            let mut presentation = skater.presentation;
            presentation.animation.try_uninit().map_err(|e| error("presentation animation", e))?;
            members.push((skater.loader, skater.recipe_load));
            if std::env::var_os("SKATE2_DIAGNOSTIC_PARTY_PLAY").is_some() {
                eprintln!("diagnostic party play destroyed player {player}");
            }
        }
        let registry = primary.selected.owners.recipe_registry.as_mut().ok_or("party skater: recipe registry absent")?;
        for (mut loader, mut recipe_load) in members {
            recipe_load.try_unload(&mut loader, registry).map_err(|e| error("unload", e))?;
        }
        Ok(())
    }

    /// `PresCharacterManager::Update` 0x82818f2c for each later skater: a registering loader
    /// completes once its presentation published a frame with its peer.
    pub(crate) fn try_complete_party_registration(&mut self) -> Result<(), String> {
        for skater in self.party.iter_mut().chain(self.ai_skaters.iter_mut()) {
            if !matches!(skater.loader.phase(), crate::player::PlayerLoadPhase::Registering { .. }) {
                continue;
            }
            let Some(peer) = skater.presentation_runtime.published_peer() else { continue };
            let scene = Arc::clone(
                skater.presentation_runtime.palette().map_err(|e| error("palette", e))?.scene(),
            );
            skater
                .recipe_load
                .complete_presentation_registration(&mut skater.loader, &scene, peer)
                .map_err(|e| error("registration", e))?;
        }
        Ok(())
    }

    /// The later skaters' presentation entities after `DoSimRestore`; one not yet restored
    /// draws nothing.
    /// `SkaterPresentation::GetPlayerRecipeID` (`0x827dd270`): the first player's loaded recipe.
    pub(crate) fn player_recipe_id(&self) -> Option<u64> {
        self.player
            .as_ref()
            .and_then(|player| player.selected.owners.recipe_load.as_ref())
            .and_then(MountedPlayerRecipeLoad::asset_id)
    }

    pub(crate) fn party_skater_visuals(&self) -> Result<Vec<crate::SelectedSkaterVisual<'_>>, String> {
        let player = self.player_recipe_id();
        let mut visuals = Vec::with_capacity(self.party.len());
        for skater in &self.party {
            let runtime = &skater.presentation_runtime;
            let borrowed = runtime.borrowed();
            let Some(render) = runtime.render_opacity() else { continue };
            visuals.push(crate::SelectedSkaterVisual {
                name_tag: None,
                palette: runtime.palette().map_err(|e| error("palette", e))?,
                materials: Arc::clone(runtime.materials().map_err(|e| error("materials", e))?),
                hips_position: (if borrowed { runtime.borrowed_hips_position() } else { runtime.published_hips_position() })
                    .ok_or("party skater hips unpublished")?,
                world_bounds: runtime.published_bounds().ok_or("party skater bounds unpublished")?,
                opacity: if borrowed { 1.0 } else { render.opacity },
                ghosted: render.ghosted && !borrowed,
                entity_type: if borrowed { 0x822 } else { 0x122 },
                players_recipe: skater.recipe_load.asset_id() == player,
                lobby_ghost: false,
            });
        }
        Ok(visuals)
    }

    /// `SpawnPlayer` for `player` at `spawn` with the skater its `PlayerLoader` member loads,
    /// in the first player's boot order: recipe and registration request, Actor, shared
    /// Skater animation data, physical pair and breadcrumbs, presentation, then the skater
    /// list, entity list, sort and `InitPlayer` (graphs, Created, `UpdateLocalPlayer`).
    pub(crate) fn try_spawn_party_skater<E: std::fmt::Debug>(
        &mut self,
        player: u32,
        character: PartyCharacter<'_>,
        spawn: [f32; 16],
    ) -> Result<(), String>
    where
        T: FnMut() -> Result<u64, E>,
    {
        let skater = self.try_build_later_skater(player, character, spawn, None)?;
        let at = self.party.partition_point(|skater| skater.player_index() < player);
        self.party.insert(at, skater);
        Ok(())
    }

    /// The spawn's owners, published to the skater and entity lists but kept by the caller.
    #[expect(clippy::too_many_lines, reason = "one ordered spawn over the first player's boot stages")]
    pub(crate) fn try_build_later_skater<E: std::fmt::Debug>(
        &mut self,
        player: u32,
        character: PartyCharacter<'_>,
        spawn: [f32; 16],
        prepared: Option<(String, Arc<PreparedRecipe>)>,
    ) -> Result<PartySkater, String>
    where
        T: FnMut() -> Result<u64, E>,
    {
        let SkaterSpawnPolicy {
            actor_flags,
            capsule_resource_allocated,
            presentation: presentation_policy,
        } = self.spawn_policy.ok_or("party skater: first player's spawn policy absent")?;
        let assets = self.assets.as_ref().ok_or("party skater: assets absent")?;
        let world_owner = &self.world.as_ref().ok_or("party skater: world absent")?.world;
        let primary = self.player.as_mut().ok_or("party skater: first player absent")?;
        let selected = &mut primary.selected;
        let owners = &mut selected.owners;
        // PartySetMarqueeCharacter: the member's peer is its PlayersBackEnd row's.
        let peer = owners.services.settings.try_peer_id(player).map_err(|e| error("peer", e))?;
        let mut loader = PlayerLoaderSlot::new(player);
        let mut recipe_load = MountedPlayerRecipeLoad::new();
        let registry = owners.recipe_registry.as_mut().ok_or("party skater: recipe registry absent")?;
        let action = match character {
            PartyCharacter::Marquee(marquee) => {
                loader.set_marquee(marquee, peer);
                let definition = marquee_definition(&owners.services, marquee)?;
                let prepared = prepared
                    .filter(|(schema_name, _)| {
                        MountedPlayerRecipeLoad::try_actual_marquee_definition(&owners.services, &definition)
                            .is_ok_and(|actual| actual.schema_name == *schema_name)
                    })
                    .map(|(_, scene)| scene);
                if let Some(prepared) = prepared {
                    recipe_load.try_load_selected_prepared(
                        &mut loader,
                        &definition,
                        &owners.services,
                        &mut owners.faces,
                        &owners.mounts,
                        &owners.cvars,
                        registry,
                        &prepared,
                    )
                } else {
                    recipe_load.try_load_selected(
                        &mut loader,
                        &definition,
                        &owners.services,
                        &mut owners.faces,
                        &owners.mounts,
                        &owners.cvars,
                        registry,
                        None,
                    )
                }
            }
            PartyCharacter::Created(record) => {
                loader.set_create_a_character(peer);
                recipe_load.try_load_cac(
                    &mut loader,
                    &owners.services,
                    &mut owners.faces,
                    &owners.mounts,
                    &owners.cvars,
                    registry,
                    record,
                )
            }
        }
        .map_err(|e| error("recipe", e))?;
        let PlayerLoadAction::Register(registration) = action else {
            return Err(error("recipe action", action));
        };
        let input_player = Arc::clone(
            selected
                .input
                .gameplay
                .player(player as usize)
                .ok_or("party skater: input player absent")?,
        );
        let formation = Arc::clone(owners.formation.as_ref().ok_or("party skater: chase formation absent")?);
        let authority = world_owner
            .lock()
            .map_err(|_| "party skater: world lock poisoned")?
            .physical_manager_authority();
        let actor = Arc::new(
            ActorConstruction::new(input_player, authority, actor_flags, SkeletonRootMatrix::known(spawn))
                .map_err(|e| error("Actor", e))?,
        );
        actor
            .try_construct_prefix(
                &*formation.lock().map_err(|_| "party skater: chase formation lock poisoned")?,
                &mut HostReckoningNumeric,
            )
            .map_err(|e| error("Actor prefix", e))?;
        actor
            .try_construct_listener(selected.input.gameplay.route_mut())
            .map_err(|e| error("Actor listener", e))?;
        actor.try_construct_graphs().map_err(|e| error("Actor graphs", e))?;
        let owners = &mut selected.owners;
        let input_services = ActorInputRuntimeServices {
            attributes: crate::action_graph_input_listener::fill::ActionInputFillAttributes::new(
                &owners.services.vault,
            ),
            cvars: owners.cvars.fill_provider(),
            juice: owners.juice.clone(),
        };
        let animation = {
            let mut adapter =
                ActorAnimationPlayerServices::try_bind(&owners.services, &owners.animation, &owners.cvars)
                    .map_err(|e| error("animation binding", e))?;
            actor
                .try_complete_player_animation(
                    world_owner,
                    &mut adapter,
                    &owners.faces,
                    &mut primary.source,
                    input_services,
                )
                .map_err(|e| error("animation", e))?
        };
        actor
            .try_install_push_shared_attributes(Arc::clone(
                self.push_shared_attributes.as_ref().ok_or("party skater: push attributes absent")?,
            ))
            .map_err(|e| error("push attributes", e))?;
        let first = selected.actor.as_ref().ok_or("party skater: first Actor absent")?;
        let handplant = first
            .handplant_shared
            .lock()
            .map_err(|_| "party skater: HandPlant shared lock poisoned")?
            .clone();
        *actor
            .handplant_shared
            .lock()
            .map_err(|_| "party skater: HandPlant shared lock poisoned")? = handplant;
        actor
            .try_install_wipeout_recovery_graph(Arc::clone(
                self.wipeout_recovery_graph.as_ref().ok_or("party skater: recovery graph absent")?,
            ))
            .map_err(|e| error("recovery graph", e))?;
        let owners = &mut selected.owners;
        let inputs = animation
            .try_assembly_inputs(owners.cvars.clone(), Arc::clone(&owners.services.vault), spawn, || {
                owners.services.settings.try_hat(player).map_err(PhysicalAnimationInputsError::Settings)
            })
            .map_err(|e| error("physical inputs", e))?;
        actor
            .try_install_action_services(Arc::clone(
                self.action_services.as_ref().ok_or("party skater: Action services absent")?,
            ))
            .map_err(|e| error("Action services", e))?;
        let hat_collision = assets.cvars.try_enable_hat_collision().map_err(|e| error("hat", e))?;
        let mut asserted = false;
        let physical = {
            let mut world = world_owner.lock().map_err(|_| "party skater: world lock poisoned")?;
            world.backend_mut().ensure_publication_frame();
            let mut references = assets.cvars.clone();
            let physical = try_construct_selected_physical_pair(
                &mut world,
                &actor,
                inputs,
                &mut references,
                &mut HostReckoningNumeric,
                capsule_resource_allocated,
                &mut || asserted = true,
                &mut || hat_collision,
            )
            .map_err(|e| error("physical pair", e))?;
            let breadcrumbs = ActorBreadCrumbConstructorOwners::try_capture_selected(world_owner, &world)
                .map_err(|e| error("breadcrumb owners", e))?;
            actor
                .try_construct_breadcrumbs_from_animation(&assets.vault, breadcrumbs)
                .map_err(|e| error("breadcrumbs", e))?;
            physical
        };
        if asserted {
            return Err("party skater: spawn orientation assertion".into());
        }
        let recipe = Arc::clone(recipe_load.recipe().map_err(|e| error("recipe", e))?);
        let finalized = Arc::clone(recipe_load.finalized().map_err(|e| error("recipe", e))?);
        let presentation = owners
            .try_new_skater_presentation(recipe, finalized, &mut primary.source)
            .map_err(|e| error("presentation", e))?;
        actor
            .try_construct_physics_auxiliary()
            .map_err(|e| error("physics auxiliary", e))?;
        let roster = Arc::clone(
            world_owner
                .lock()
                .map_err(|_| "party skater: world lock poisoned")?
                .entity_roster
                .as_ref()
                .ok_or("party skater: entity roster absent")?,
        );
        owners
            .services
            .skaters
            .try_publish_actor(&actor, &roster)
            .map_err(|e| error("roster", e))?;
        owners.try_load_skater_graphs(&actor).map_err(|e| error("graphs", e))?;
        owners
            .services
            .skaters
            .try_finish_player_observers(&actor)
            .map_err(|e| error("observers", e))?;
        let scene = Arc::clone(recipe_load.scene().map_err(|e| error("scene", e))?);
        let presentation_runtime = SelectedPresentationRuntime::try_new(&presentation, scene, presentation_policy)
            .map_err(|e| error("presentation runtime", e))?;
        Ok(PartySkater {
            loader,
            recipe_load,
            actor,
            animation,
            physical,
            presentation,
            presentation_runtime,
            registration,
        })
    }
}
