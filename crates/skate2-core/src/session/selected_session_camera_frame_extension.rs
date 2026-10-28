//! Bounded camera admission and queued-frame borrow of the retained session.
//!
//! This follows the terrain/input extension. The host still chooses the
//! location, registration schedule, camera set, gravity, timing and services.
use super::SelectedSession;
use crate::{
    actor_construction::{
        physics_auxiliary::ActorPhysicsAuxiliaryError,
        selected_startup::roster_startup::SelectedRosterStartupError,
        selected_tick::{SelectedPlayerTick, SelectedQueuedTickInput},
    },
    actor_face_playback::presentation::frame::selected_runtime::{
        SelectedPresentationConstructionError, SelectedPresentationPolicy,
        SelectedPresentationRuntime,
    },
    camera_session::selected_runtime::CameraSelectedInputs,
    player::mounted_recipe::MountedRecipeError,
    selected_session_frame::SelectedSessionFrame,
    selected_session_terrain::SelectedSessionTerrainError,
    selected_terrain_startup::camera::CameraTerrainError,
    skater_manager_local_identity::observers::SkaterObserverError,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum SelectedSessionCameraAdmissionError {
    AlreadyAttempted,
    Missing(&'static str),
    Terrain(SelectedSessionTerrainError),
    WorldLock,
    PhysicsAuxiliary(ActorPhysicsAuxiliaryError),
    Graphs(SelectedRosterStartupError),
    Camera(CameraTerrainError),
    Observer(SkaterObserverError),
    Roster(SelectedRosterStartupError),
}

#[derive(Debug)]
pub(crate) enum SelectedSessionFramePrepareError {
    AlreadyAttempted,
    Missing(&'static str),
    Terrain(SelectedSessionTerrainError),
    Recipe(MountedRecipeError),
    Presentation(SelectedPresentationConstructionError),
}

#[derive(Debug)]
pub(crate) enum SelectedSessionFrameBorrowError {
    Missing(&'static str),
    Terrain(SelectedSessionTerrainError),
    WorldLock,
}

impl<T, C> SelectedSession<T, C> {
    /// After the physical pair and completed selected terrain, capture the
    /// camera capability on the already registered observer before publishing
    /// this Actor to the existing roster. A failure retains every earlier
    /// world/player/camera write and makes this admission non-retryable.
    pub(crate) fn try_admit_selected_camera_and_roster(
        &mut self,
        camera_set: &str,
    ) -> Result<(), SelectedSessionCameraAdmissionError> {
        use SelectedSessionCameraAdmissionError as E;
        if self.camera_attempted {
            return Err(E::AlreadyAttempted);
        }
        if self.physical.is_none() {
            return Err(E::Missing("selected physical pair"));
        }
        let world_owner = self.world.as_ref().ok_or(E::Missing("selected world"))?;
        let terrain = self
            .terrain
            .as_ref()
            .ok_or(E::Missing("selected terrain"))?;
        let terrain = terrain
            .terrain_for(&world_owner.world)
            .map_err(E::Terrain)?;
        let bounds = self.terrain.as_ref().expect("checked terrain").bounds();
        let player = self.player.as_mut().ok_or(E::Missing("selected player"))?;
        let actor = player
            .selected
            .actor
            .as_ref()
            .ok_or(E::Missing("selected Actor"))?;
        self.camera_attempted = true;
        actor
            .try_construct_physics_auxiliary()
            .map_err(E::PhysicsAuxiliary)?;
        player
            .selected
            .owners
            .try_ensure_player_graph_owners()
            .map_err(E::Graphs)?;
        let world = world_owner.world.lock().map_err(|_| E::WorldLock)?;
        let inputs: CameraSelectedInputs = terrain
            .try_camera_inputs(
                &world,
                &world_owner.world,
                &player.selected.owners,
                bounds,
                camera_set.to_owned(),
            )
            .map_err(E::Camera)?;
        player
            .selected
            .owners
            .services
            .skaters
            .try_capture_selected_camera(inputs)
            .map_err(E::Observer)?;
        let roster = Arc::clone(
            world
                .entity_roster
                .as_ref()
                .ok_or(E::Missing("world entity roster"))?,
        );
        player
            .selected
            .owners
            .try_admit_first_actor(actor, &roster)
            .map_err(E::Roster)?;
        self.camera_complete = true;
        Ok(())
    }

    /// Construct the only presentation runtime from this player's finalized
    /// scene. The caller supplies explicit presentation and camera-submit
    /// policies; world gravity was installed before the physical bodies.
    /// It runs once after same-world terrain/camera/roster admission and input
    /// binding.
    pub(crate) fn try_prepare_selected_queued_frame(
        &mut self,
        presentation_policy: SelectedPresentationPolicy,
        camera_submit_mask: u32,
    ) -> Result<(), SelectedSessionFramePrepareError> {
        use SelectedSessionFramePrepareError as E;
        if self.frame_attempted {
            return Err(E::AlreadyAttempted);
        }
        if !self.camera_complete {
            return Err(E::Missing("camera and roster admission"));
        }
        if !self.presentation_complete {
            return Err(E::Missing("completed selected presentation"));
        }
        if self.bound_input.is_none() {
            return Err(E::Missing("bound selected input"));
        }
        let world_owner = self.world.as_ref().ok_or(E::Missing("selected world"))?;
        self.terrain
            .as_ref()
            .ok_or(E::Missing("selected terrain"))?
            .ground_pool_for(&world_owner.world)
            .map_err(E::Terrain)?;
        let player = self.player.as_ref().ok_or(E::Missing("selected player"))?;
        player
            .selected
            .actor
            .as_ref()
            .ok_or(E::Missing("selected Actor"))?;
        let owners = &player.selected.owners;
        let presentation = owners
            .presentation
            .as_ref()
            .ok_or(E::Missing("selected presentation"))?;
        let recipe = owners
            .recipe_load
            .as_ref()
            .ok_or(E::Missing("mounted recipe load"))?;
        self.frame_attempted = true;
        let scene = Arc::clone(recipe.scene().map_err(E::Recipe)?);
        self.presentation_runtime = Some(
            SelectedPresentationRuntime::try_new(presentation, scene, presentation_policy)
                .map_err(E::Presentation)?,
        );
        let mut tick = SelectedPlayerTick::new();
        tick.select_camera_subsequence(camera_submit_mask);
        self.selected_tick = Some(tick);
        Ok(())
    }

    /// Lend the same selected world, normal-arena ground pool, queued input,
    /// Actor, pair, animation and presentation runtime for one frame. The
    /// closure owns its source-supplied dt/LOD, physics/grind policy, numeric,
    /// sync, recognizer, feedback and observers. The host schedules one call
    /// to registered `SelectedSessionFrame::try_step_queued` per selected
    /// frame. No queue replay or presentation update occurs in this adapter.
    pub(crate) fn try_with_selected_queued_frame<R>(
        &mut self,
        run: impl FnOnce(SelectedSessionFrame<'_>, &StaticGroundPool, SelectedQueuedTickInput<'_>) -> R,
    ) -> Result<R, SelectedSessionFrameBorrowError> {
        self.try_with_selected_queued_frame_challenge(None, run)
    }

    pub(crate) fn try_with_selected_queued_frame_challenge<R>(
        &mut self,
        challenge: Option<&mut crate::challenge::system::World>,
        run: impl FnOnce(SelectedSessionFrame<'_>, &StaticGroundPool, SelectedQueuedTickInput<'_>) -> R,
    ) -> Result<R, SelectedSessionFrameBorrowError> {
        use SelectedSessionFrameBorrowError as E;
        if !self.camera_complete {
            return Err(E::Missing("camera and roster admission"));
        }
        let world_owner = self.world.as_ref().ok_or(E::Missing("selected world"))?;
        let ground = self
            .terrain
            .as_ref()
            .ok_or(E::Missing("selected terrain"))?
            .ground_pool_for(&world_owner.world)
            .map_err(E::Terrain)?;
        let pair = &self
            .physical
            .as_ref()
            .ok_or(E::Missing("physical pair"))?
            .pair;
        let player = self.player.as_mut().ok_or(E::Missing("selected player"))?;
        let actor = player
            .selected
            .actor
            .as_ref()
            .ok_or(E::Missing("selected Actor"))?;
        let animation = player
            .selected
            .physical_animation
            .as_mut()
            .ok_or(E::Missing("physical animation"))?;
        let owners = &mut player.selected.owners;
        let presentation = owners
            .presentation
            .as_ref()
            .ok_or(E::Missing("selected presentation"))?;
        let recipe_owner=Arc::clone(&owners.living_world_recipes);
        let mut recipe_runtime=recipe_owner.lock().map_err(|_|E::Missing("RecipeMan owner lock"))?;
        let graph_owner=Arc::clone(owners.graphs.as_ref().ok_or(E::Missing("graph owners"))?);
        let graph_guard=graph_owner.lock().map_err(|_|E::Missing("graph owner lock"))?;
        let recipe_frame = crate::selected_living_world_recipes::SelectedRecipeFrame {
            challenge,
            triggers: &self.terrain.as_ref().ok_or(E::Missing("selected terrain"))?
                .terrain_for(&world_owner.world).map_err(E::Terrain)?.triggers,
            pedestrian_animation: crate::selected_living_world_recipes::pedestrian_animation::PedestrianSessionAnimation {
                startup: &owners.pedestrian_animation, shared: &owners.animation, world: &world_owner.world,
            },
            runtime: &mut recipe_runtime,
            registry: owners
                .recipe_registry
                .as_mut()
                .ok_or(E::Missing("mounted recipe registry"))?,
            requests: owners
                .recipe_load
                .as_mut()
                .ok_or(E::Missing("mounted recipe request owner"))?
                .requests_mut(),
            mounts: &owners.mounts,
            vault: &owners.services.vault,
            graphs: &graph_guard,
            painter: &self
                .terrain
                .as_ref()
                .ok_or(E::Missing("selected terrain"))?
                .terrain_for(&world_owner.world)
                .map_err(E::Terrain)?
                .layers,
        };
        let input_owner = self
            .bound_input
            .as_mut()
            .ok_or(E::Missing("bound selected input"))?;
        let tick = self
            .selected_tick
            .as_mut()
            .ok_or(E::Missing("selected tick"))?;
        let presentation_runtime = self
            .presentation_runtime
            .as_mut()
            .ok_or(E::Missing("presentation runtime"))?;
        let living_world = self.living_world_dmo.as_mut();
        let local_index = owners
            .services
            .skaters
            .try_local_index()
            .map_err(|_| E::Missing("local skater index"))?;
        // SkaterManager's list holds the later players and the AI skaters in player order.
        let mut others: Vec<_> = self
            .party
            .iter_mut()
            .chain(self.ai_skaters.iter_mut())
            .map(|skater| {
                (
                    skater.player_index(),
                    crate::selected_session_frame::SelectedFrameSkater {
                        board: &skater.physical.pair.board,
                        skeleton: &skater.physical.pair.skeleton,
                        actor: &skater.actor,
                        animation: &mut skater.animation,
                        presentation: &skater.presentation,
                        presentation_runtime: &mut skater.presentation_runtime,
                    },
                )
            })
            .collect();
        others.sort_by_key(|(player, _)| *player);
        let local = if local_index == 0 {
            0
        } else {
            1 + others
                .iter()
                .position(|(player, _)| i64::from(*player) == i64::from(local_index))
                .ok_or(E::Missing("local party skater"))?
        };
        let others = others.into_iter().map(|(_, skater)| skater).collect();
        let fade_out_time = owners
            .cvars
            .try_skater_fade_out_time()
            .map_err(|_| E::Missing("kSkaterFadeOutTime"))?;
        let mut ai = self.ai.as_mut().map(|ai| crate::selected_session_boot_owner::ai_skaters::SessionAiFrame {
            ai,
            settings: &owners.services.settings,
            vault: &owners.services.vault,
            fade_out_time,
        });
        let mut world = world_owner.world.lock().map_err(|_| E::WorldLock)?;
        Ok(run(
            SelectedSessionFrame {
                tick,
                world_owner: &world_owner.world,
                world: &mut world,
                board: &pair.board,
                skeleton: &pair.skeleton,
                actor,
                animation,
                input: &mut player.selected.input,
                cvars: &owners.cvars,
                settings: &owners.services.settings,
                services: &owners.services,
                presentation,
                presentation_runtime,
                others,
                local,
                challenge_scoring: Some(&self.challenge_scoring),
                living_world,
                living_world_recipes: Some(recipe_frame),
                ai: ai
                    .as_mut()
                    .map(|ai| ai as &mut dyn crate::actor_construction::selected_tick::SelectedTickAi),
            },
            ground,
            input_owner.queued_tick_input(),
        ))
    }
}
