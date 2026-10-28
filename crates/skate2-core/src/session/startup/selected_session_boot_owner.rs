//! Bounded selected-session boot owner. Each attempt retains the completed
//! prefix and stops at a named source-service boundary.
#![allow(dead_code, reason = "awaits selected session terrain and frame caller")]

use crate::{
    actor_construction::{
        action_runtime_session::{ActionRuntimeServices, ActorActionRuntimeError},
        breadcrumbs::{ActorBreadCrumbConstructorOwners, ActorBreadCrumbMountError},
        physical_animation_inputs::PhysicalAnimationInputsError,
        selected_startup::SelectedPlayerConstructionRequest,
    },
    actor_face_playback::presentation::startup::PresentationStartupError,
    attribute_runtime::AttributeRuntimeError,
    gesture_trick_mapping::{
        GestureTrickMapping, MappingAllocation, MappingError, debug::GestureMappingDebugPosition,
    },
    input_pad::device::PadPlatform,
    juice_trick_input::{
        JuiceTrickInput,
        boot::{JuiceBootPlatform, JuiceHostOption},
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::ReckoningNumeric,
    selected_original_assets_bootstrap::{SelectedOriginalAssets, SelectedOriginalAssetsError},
    selected_production_input_owner::{
        ProductionInputBindFailure, ProductionInputStartError, SelectedInputBootstrap,
        SelectedProductionInputOwner,
    },
    selected_session_physical_pair_startup::{
        SelectedSessionPhysicalPair, SelectedSessionPhysicalPairFailure,
        try_construct_selected_physical_pair, try_selected_physical_inputs,
    },
    selected_session_player_startup::{
        SelectedSessionPlayerStartup, SelectedSessionPlayerStartupError,
    },
    selected_session_world_startup::{
        SelectedSessionWorldStartup, SelectedSessionWorldStartupError,
    },
    simulation_animation_startup::mounted_source::MountedAnimationSourceError,
    skeleton_root_transforms::SkeletonRootMatrix,
};
use skate2_formats::mount::MountSet;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// This is an ordered host composition, not native `Simulation::Init` or an
/// automatic choice of player, location, thread, or resource allocation.
#[expect(
    clippy::struct_excessive_bools,
    reason = "separate one-shot boot, player, presentation and physical attempt gates retain partial owners"
)]
pub(crate) struct SelectedSession<T, C> {
    mounts: Arc<MountSet>,
    data_root: PathBuf,
    pub(crate) assets: Option<SelectedOriginalAssets>,
    pub(crate) input: Option<SelectedInputBootstrap<C>>,
    pub(crate) bound_input: Option<SelectedProductionInputOwner<C>>,
    pub(crate) failed_input_bind: Option<ProductionInputBindFailure<C>>,
    pub(crate) world: Option<SelectedSessionWorldStartup>,
    pub(crate) terrain: Option<Box<dyn terrain_input::SelectedSessionTerrainSlot + Send>>,
    pub(crate) terrain_start_failure:
        Option<crate::selected_session_terrain::SelectedSessionTerrainError>,
    pub(crate) player: Option<SelectedSessionPlayerStartup<T>>,
    pub(crate) physical: Option<SelectedSessionPhysicalPair>,
    pub(crate) selected_tick: Option<crate::actor_construction::selected_tick::SelectedPlayerTick>,
    pub(crate) presentation_runtime: Option<
        crate::actor_face_playback::presentation::frame::selected_runtime::SelectedPresentationRuntime,
    >,
    pub(crate) failed_physical:
        Option<crate::physical_player_assembly::PhysicalPlayerAssemblyConstructionState>,
    pub(crate) failed_mapping: Option<GestureTrickMapping>,
    pub(crate) gesture_mapping: Option<Arc<GestureTrickMapping>>,
    pub(crate) action_services: Option<Arc<ActionRuntimeServices>>,
    pub(crate) wipeout_recovery_graph:
        Option<Arc<crate::skater_wipeout_recovery_graph::SkaterWipeoutRecoveryGraph>>,
    pub(crate) push_shared_attributes:
        Option<Arc<crate::skater_push_shared_attributes::SkaterPushSharedAttributes>>,
    /// Created with the controlled frame, as `GameModule::ResetGame` does.
    pub(crate) session_marker: Option<crate::selected_session_marker::SelectedSessionMarker>,
    /// The main DMO bank, its model registration and the LivingWorld DMO
    /// schedule over the World's Manager occurrence.
    pub(crate) living_world_dmo: Option<crate::selected_living_world_dmo::SelectedLivingWorldDmo>,
    /// `SkaterManager`'s skaters after the first, in player index order.
    pub(crate) party: Vec<party_skaters::PartySkater>,
    /// `SkaterManager`'s AI skaters, in player index order.
    pub(crate) ai_skaters: Vec<party_skaters::PartySkater>,
    /// `AI::AISkaterManager` and the AI skaters' controllers.
    pub(crate) ai: Option<ai_skaters::SessionAi>,
    /// `cTrickPatternManager`'s statics and the challenge system's scoring manager, which
    /// `World::UpdateConditioners` reads for every skater.
    pub(crate) challenge_scoring: Arc<std::sync::Mutex<crate::challenge::skate::ChallengeScoring>>,
    pub(crate) spawn_policy: Option<party_skaters::SkaterSpawnPolicy>,
    spawn: Option<[f32; 16]>,
    boot_attempted: bool,
    player_attempted: bool,
    presentation_attempted: bool,
    presentation_complete: bool,
    terrain_attempted: bool,
    input_bind_attempted: bool,
    camera_attempted: bool,
    camera_complete: bool,
    frame_attempted: bool,
    physical_attempted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SelectedSessionNextGate {
    OriginalAssets,
    InputManagerAndJuice,
    InputBindFailed,
    WorldAndLocal,
    NativeSpawnScheduleAndRegistrationRequest,
    ControlledPresentationAfterPair,
    ActionServicesAndPhysicalPair,
    TerrainStartFailed,
    TerrainCameraInputAndFrameServices,
}

#[derive(Debug)]
pub(crate) enum SelectedSessionBootError<P, J> {
    AlreadyAttempted,
    Assets(SelectedOriginalAssetsError),
    #[cfg(feature = "skate3-runtime")]
    EmbeddedModData(String),
    Input(ProductionInputStartError<P, J>),
    World(SelectedSessionWorldStartupError),
}

/// Explicit host-selected spawn for the first test. The selected mounted
/// constructor issues a Register request and constructs Actor while the loader
/// is Registering. Native command-3 scheduling remains to be established.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ControlledPlayerPolicy {
    HostSpawnWhileRegistrationPending,
}

#[derive(Debug)]
pub(crate) enum SelectedSessionControlledPlayerError<E> {
    AlreadyAttempted,
    MissingPrefix(SelectedSessionNextGate),
    MissingInputMap,
    Player(SelectedSessionPlayerStartupError<E>),
}

#[derive(Debug)]
pub(crate) enum SelectedSessionPresentationError<E> {
    WrongStage(SelectedSessionNextGate),
    AlreadyAttempted,
    Startup(
        PresentationStartupError<
            MountedAnimationSourceError<E>,
            crate::actor_construction::player_animation::PlayerAnimationError,
        >,
    ),
}

#[derive(Debug)]
pub(crate) enum SelectedSessionPhysicalError<N, G> {
    WrongStage(SelectedSessionNextGate),
    AlreadyAttempted,
    WorldLock,
    MissingActor,
    MissingAnimation,
    Inputs(PhysicalAnimationInputsError),
    ActionBind(AttributeRuntimeError),
    Mapping(MappingError<G>),
    GestureCapture(MappingError<std::convert::Infallible>),
    Action(ActorActionRuntimeError),
    Pair(Box<SelectedSessionPhysicalPairFailure<N>>),
    Breadcrumb(ActorBreadCrumbMountError),
}

impl<T, C> SelectedSession<T, C> {
    pub(crate) fn new(mounts: Arc<MountSet>, data_root: &Path) -> Self {
        Self {
            mounts,
            data_root: data_root.to_owned(),
            assets: None,
            input: None,
            bound_input: None,
            failed_input_bind: None,
            world: None,
            terrain: None,
            terrain_start_failure: None,
            player: None,
            physical: None,
            selected_tick: None,
            presentation_runtime: None,
            failed_physical: None,
            failed_mapping: None,
            gesture_mapping: None,
            action_services: None,
            wipeout_recovery_graph: None,
            push_shared_attributes: None,
            session_marker: None,
            living_world_dmo: None,
            party: Vec::new(),
            ai_skaters: Vec::new(),
            ai: None,
            challenge_scoring: Arc::default(),
            spawn_policy: None,
            spawn: None,
            boot_attempted: false,
            player_attempted: false,
            presentation_attempted: false,
            presentation_complete: false,
            terrain_attempted: false,
            input_bind_attempted: false,
            camera_attempted: false,
            camera_complete: false,
            frame_attempted: false,
            physical_attempted: false,
        }
    }

    pub(crate) fn next_gate(&self) -> SelectedSessionNextGate {
        if self.assets.is_none() {
            SelectedSessionNextGate::OriginalAssets
        } else if self.terrain_start_failure.is_some() {
            SelectedSessionNextGate::TerrainStartFailed
        } else if self.failed_input_bind.is_some() {
            SelectedSessionNextGate::InputBindFailed
        } else if self.bound_input.is_none()
            && self
                .input
                .as_ref()
                .and_then(SelectedInputBootstrap::shared_map)
                .is_none()
        {
            SelectedSessionNextGate::InputManagerAndJuice
        } else if self.world.is_none() {
            SelectedSessionNextGate::WorldAndLocal
        } else {
            // Command-3 timing and Registration status remain unresolved.
            // A controlled player continuation retains its exact Register
            // request, but that request is not a native completion receipt.
            SelectedSessionNextGate::NativeSpawnScheduleAndRegistrationRequest
        }
    }

    pub(crate) fn controlled_next_gate(&self) -> SelectedSessionNextGate {
        if self.terrain_start_failure.is_some() {
            SelectedSessionNextGate::TerrainStartFailed
        } else if self.failed_input_bind.is_some() {
            SelectedSessionNextGate::InputBindFailed
        } else if self.player.is_none() {
            self.next_gate()
        } else if self.physical.is_none() {
            SelectedSessionNextGate::ActionServicesAndPhysicalPair
        } else if !self.presentation_complete {
            SelectedSessionNextGate::ControlledPresentationAfterPair
        } else {
            SelectedSessionNextGate::TerrainCameraInputAndFrameServices
        }
    }

    /// Source-ordered available prefix through the original asset, input,
    /// Juice, world and local owners. Spawn scheduling and retention of the
    /// actual Register request are the next source gates.
    pub(crate) fn try_boot_prefix<P: PadPlatform, J: JuiceBootPlatform<Connection = C>>(
        &mut self,
        pad: &mut P,
        juice_platform: &mut J,
        juice_ini: Option<&[u8]>,
        juice_policy: JuiceHostOption,
    ) -> Result<(), SelectedSessionBootError<P::Error, J::Error>> {
        use SelectedSessionBootError as Error;
        if self.boot_attempted {
            return Err(Error::AlreadyAttempted);
        }
        self.boot_attempted = true;
        // This shared loading boundary serves both the PC facade and native
        // headless/session hosts. Never decode embedded recipes in a trick's
        // first physics update.
        #[cfg(feature = "skate3-runtime")]
        crate::physical_player_footplant::skate3::SourceFootplant::preload()
            .map_err(Error::EmbeddedModData)?;
        self.assets = Some(
            SelectedOriginalAssets::try_new(Arc::clone(&self.mounts), &self.data_root)
                .map_err(Error::Assets)?,
        );
        let assets = self.assets.as_ref().expect("retained assets");
        self.input = Some(SelectedInputBootstrap::new(
            JuiceTrickInput::new_static_initialized(),
        ));
        self.input
            .as_mut()
            .expect("retained input")
            .try_initialize(
                pad,
                juice_platform,
                &assets.mounts,
                &assets.cvars,
                juice_ini,
                juice_policy,
            )
            .map_err(Error::Input)?;
        self.world = Some(
            SelectedSessionWorldStartup::try_new(&assets.cvars, &assets.vault, &assets.mounts)
                .map_err(Error::World)?,
        );
        Ok(())
    }

    /// Explicit first-test continuation of the existing mounted constructor.
    /// It constructs Actor while registration remains pending. The native
    /// command-3 receiver checks location-spawner readiness, but when its
    /// message is sent relative to registration is not yet established.
    pub(crate) fn try_continue_controlled_player<E>(
        &mut self,
        policy: ControlledPlayerPolicy,
        spawn: [f32; 16],
        marquee: u64,
        actor_flags: u32,
        current_thread: T,
    ) -> Result<(), SelectedSessionControlledPlayerError<E>>
    where
        T: FnMut() -> Result<u64, E>,
    {
        let request = self.try_prepare_controlled_player(
            policy,
            spawn,
            marquee,
            actor_flags,
            current_thread,
        )?;
        self.try_finish_controlled_player(request)
    }

    /// Retain the selected registration request and initialized Simulation
    /// animation before Actor construction. The facade builds the recovery
    /// graph from this owner's original Skater playback at this seam.
    pub(crate) fn try_prepare_controlled_player<E>(
        &mut self,
        _policy: ControlledPlayerPolicy,
        spawn: [f32; 16],
        marquee: u64,
        actor_flags: u32,
        current_thread: T,
    ) -> Result<SelectedPlayerConstructionRequest, SelectedSessionControlledPlayerError<E>>
    where
        T: FnMut() -> Result<u64, E>,
    {
        use SelectedSessionControlledPlayerError as Error;
        if self.player_attempted {
            return Err(Error::AlreadyAttempted);
        }
        let gate = self.controlled_next_gate();
        if gate != SelectedSessionNextGate::NativeSpawnScheduleAndRegistrationRequest {
            return Err(Error::MissingPrefix(gate));
        }
        self.player_attempted = true;
        let assets = self.assets.as_ref().expect("selected assets");
        let world = self.world.as_ref().expect("retained world");
        let input = self.input.as_ref().expect("retained input");
        let map = input.shared_map().ok_or(Error::MissingInputMap)?;
        self.player = Some(
            SelectedSessionPlayerStartup::try_prepare_with_input_owner(
                Arc::clone(&assets.mounts),
                &world.world,
                Arc::clone(&assets.vault),
                assets.cvars.clone(),
                map,
                input.trick_input(),
                marquee,
                current_thread,
            )
            .map_err(Error::Player)?,
        );
        self.spawn = Some(spawn);
        Ok(SelectedPlayerConstructionRequest {
            spawn: SkeletonRootMatrix::known(spawn),
            actor_flags,
        })
    }

    /// Continue the prepared player through the existing input and Actor
    /// constructor once. Failed construction remains owned by this session.
    pub(crate) fn try_finish_controlled_player<E>(
        &mut self,
        request: SelectedPlayerConstructionRequest,
    ) -> Result<(), SelectedSessionControlledPlayerError<E>>
    where
        T: FnMut() -> Result<u64, E>,
    {
        use SelectedSessionControlledPlayerError as Error;
        let gate = self.controlled_next_gate();
        let player = self.player.as_mut().ok_or(Error::MissingPrefix(gate))?;
        let world = &self.world.as_ref().expect("retained world").world;
        player
            .try_finish_actor(world, request)
            .map_err(Error::Player)
    }

    /// Diagnostic first-test continuation after the pair. This initializes
    /// retained presentation resources, not native player registration.
    pub(crate) fn try_continue_controlled_presentation<E>(
        &mut self,
    ) -> Result<(), SelectedSessionPresentationError<E>>
    where
        T: FnMut() -> Result<u64, E>,
    {
        use SelectedSessionPresentationError as Error;
        if self.presentation_attempted {
            return Err(Error::AlreadyAttempted);
        }
        let gate = self.controlled_next_gate();
        if self.physical.is_none() {
            return Err(Error::WrongStage(gate));
        }
        let player = self.player.as_mut().ok_or(Error::WrongStage(gate))?;
        self.presentation_attempted = true;
        player
            .selected
            .try_initialize_mounted_presentation(&mut player.source)
            .map_err(Error::Startup)?;
        self.presentation_complete = true;
        Ok(())
    }

    /// Construct one pair from this owner's exact Actor, animation, world and
    /// spawn. It builds Action services from this session's Vault and world
    /// overlay only after physical inputs are captured. The host supplies the
    /// mapping allocation and source-visible capsule/assertion/hat answers.
    /// Failed mapping/factory state remains owned here.
    pub(crate) fn try_continue_physical<N: ReckoningNumeric, G>(
        &mut self,
        mapping_allocate: &mut impl FnMut(usize, usize, u32) -> Result<MappingAllocation, G>,
        numeric: &mut N,
        capsule_resource_allocated: bool,
        assertion: &mut impl FnMut(),
        hat_enabled: &mut impl FnMut() -> bool,
    ) -> Result<(), SelectedSessionPhysicalError<N::Error, G>> {
        use SelectedSessionPhysicalError as Error;
        if self.physical_attempted {
            return Err(Error::AlreadyAttempted);
        }
        if self.player.is_none() {
            return Err(Error::WrongStage(self.controlled_next_gate()));
        }
        self.physical_attempted = true;
        let player = self.player.as_ref().expect("checked player");
        let actor = player.selected.actor.as_ref().ok_or(Error::MissingActor)?;
        let animation = player
            .selected
            .physical_animation
            .as_ref()
            .ok_or(Error::MissingAnimation)?;
        let inputs = try_selected_physical_inputs(
            &player.selected.owners,
            actor,
            animation,
            self.spawn.expect("selected spawn"),
        )
        .map_err(Error::Inputs)?;
        let assets = self.assets.as_ref().expect("selected assets");
        let world_handle = &self.world.as_ref().expect("selected world").world;
        let overlay = world_handle
            .lock()
            .map_err(|_| Error::WorldLock)?
            .debug_overlay
            .clone();
        self.action_services = Some(Arc::new(
            ActionRuntimeServices::try_bind(&assets.vault, overlay.clone())
                .map_err(Error::ActionBind)?,
        ));
        let mut mapping = GestureTrickMapping::new();
        if let Err(error) = mapping.try_initialize(mapping_allocate) {
            self.failed_mapping = Some(mapping);
            return Err(Error::Mapping(error));
        }
        let mapping = Arc::new(mapping);
        self.gesture_mapping = Some(Arc::clone(&mapping));
        Arc::get_mut(self.action_services.as_mut().expect("retained Action services"))
            .expect("Action services are unique before Actor installation")
            .gesture = Some(Arc::new(
            crate::actor_construction::action_motion_intent_bridge::gesture_runtime::GestureTrickRuntimeServices::try_capture_mapping_member(
                mapping,
                assets.cvars.clone(),
                overlay,
                GestureMappingDebugPosition::new(),
            ).map_err(Error::GestureCapture)?,
        ));
        actor
            .try_install_action_services(Arc::clone(
                self.action_services
                    .as_ref()
                    .expect("retained Action services"),
            ))
            .map_err(Error::Action)?;
        let mut world = world_handle.lock().map_err(|_| Error::WorldLock)?;
        let mut refs = assets.cvars.clone();
        match try_construct_selected_physical_pair(
            &mut world,
            actor,
            inputs,
            &mut refs,
            numeric,
            capsule_resource_allocated,
            assertion,
            hat_enabled,
        ) {
            Ok(pair) => {
                self.physical = Some(pair);
                let breadcrumb_owners =
                    ActorBreadCrumbConstructorOwners::try_capture_selected(world_handle, &world)
                        .map_err(|error| {
                            Error::Breadcrumb(ActorBreadCrumbMountError::Owners(error))
                        })?;
                actor
                    .try_construct_breadcrumbs_from_animation(&assets.vault, breadcrumb_owners)
                    .map_err(Error::Breadcrumb)?;
                Ok(())
            }
            Err(mut failure) => {
                self.failed_physical = failure.retained.take();
                Err(Error::Pair(Box::new(failure)))
            }
        }
    }

    pub(crate) fn world_handle(&self) -> Option<&Arc<Mutex<PhysicalAssemblyWorld>>> {
        self.world.as_ref().map(|owner| &owner.world)
    }
}

#[path = "../selected_session_terrain_input_extension.rs"]
pub(crate) mod terrain_input;

#[path = "../selected_session_camera_frame_extension.rs"]
pub(crate) mod camera_frame;

#[path = "../selected_session_party_skaters.rs"]
pub(crate) mod party_skaters;

#[path = "../selected_session_ai.rs"]
pub(crate) mod ai_skaters;

#[path = "../selected_session_public_facade.rs"]
pub(crate) mod public_facade;
