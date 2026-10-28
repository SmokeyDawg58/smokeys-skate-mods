//! Retained pedestrian construction parts, Pedestrian82e97748. This object is
//! not a published entity: original intent/motion/adjust owners must be attached
//! before the factory may admit it to the existing EntityMan roster.
#[path = "pedestrian_big_black.rs"]
pub(crate) mod big_black;
use crate::{
    actor_face_playback::{AnimationPoseRef, FacePlaybackData},
    attribute_runtime::AttributeRuntimeVault,
    living_world_grind_services::{
        attrib_cache::LwAttribCache, character_models::CharacterModel,
        entity_core::CommonEntityCore,
    },
    living_world_pedestrian_memory::{PedestrianMemory, PedestrianStore},
    living_world_pedestrian_resources::PedestrianResources,
    living_world_pedestrian_state::PedestrianStateStorage,
    motion_graph_entity_authority::ActorGraphAuthority,
    pedestrian_animation::PedestrianAnimation,
    pedestrian_graph::{PedestrianGraph, PedestrianGraphBehaviours},
    pedestrian_intent_packet::queue::IntentQueue,
    pedestrian_navigation::{PedestrianNavigation, mesh::MeshNavigation},
    physical_player_assembly::PhysicalAssemblyWorld,
    simulation_animation_startup::AnimationAttributePool,
};
use std::sync::{Arc, Mutex};
#[path = "pedestrian_ground_dmo_receive.rs"]
mod ground_dmo_receive;
#[path = "pedestrian_entity_jump.rs"]
mod jump;
#[path = "pedestrian_entity_roster.rs"]
mod roster;
#[path = "pedestrian_factory.rs"]
pub(crate) mod factory;
#[path = "pedestrian_targeting.rs"]
pub(crate) mod targeting;
#[path = "pedestrian_patrol.rs"]
pub(crate) mod patrol;
#[path = "pedestrian_model_init.rs"]
pub(crate) mod model_init;
#[path = "pedestrian_hand_prop.rs"]
mod hand_prop;
#[path = "pedestrian_takedown_descriptor.rs"]
pub(crate) mod takedown_descriptor;
#[path = "pedestrian_entity_intent.rs"]
mod intent;
#[path = "pedestrian_perception.rs"]
pub(crate) mod perception;

pub(crate) struct PedestrianIdentity {
    pub(crate) id: u32,
}
pub(crate) type EntityAnimation = PedestrianAnimation<PedestrianIdentity, Mutex<PhysicalAssemblyWorld>>;

#[derive(Clone, Copy)]
enum ActiveNavigation {
    Mesh,
    Physics,
}

/// Partial construction remains owned across a failure, including native slot
/// reservations. The factory retains this until completion or explicit teardown.
pub(crate) struct PedestrianConstruction {
    pub(crate) id: u32,
    pub(crate) allocations: Vec<(PedestrianStore, u32)>,
    pub(crate) authority: ActorGraphAuthority<PedestrianIdentity>,
    pub(crate) core: Option<CommonEntityCore>,
    pub(crate) rain_data: Option<crate::living_world_rain::spatial::RainItem>,
    pub(crate) model: Option<Arc<CharacterModel>>,
    /// Native +1a00 is written only when reverse category lookup matches.
    pub(crate) entity_category_key: Option<u64>,
    /// Pedestrian+0x1a08: activity assigned by the challenge character manager.
    pub(crate) challenge_owner: Option<usize>,
    pub(crate) hand_prop_attributes: Option<crate::attribute_runtime::AttributeRuntimeInstance>,
    pub(crate) hand_prop_usage: hand_prop::HandPropUsage,
    pub(crate) positioning_test_start_height: Option<f32>,
    pub(crate) target_iterators: targeting::TargetIterators,
    pub(crate) current_waypoint: Option<crate::living_world_grind_services::waypoints::WaypointRef>,
    /// LWBasicEntity::InitContainedProps82e615a0, native capacity eight.
    pub(crate) contained_props: Vec<u32>,
    pub(crate) attrib_cache: Arc<LwAttribCache>,
    pub(crate) resources: Option<PedestrianResources>,
    pub(crate) mesh_navigation: Option<MeshNavigation>,
    pub(crate) physics_navigation: Option<PedestrianNavigation>,
    active_navigation: ActiveNavigation,
    pub(crate) state: Option<PedestrianStateStorage>,
    pub(crate) big_black: Option<big_black::BigBlackData>,
    pub(crate) security_guard: Option<crate::living_world_security_guard::SecurityGuardData>,
    /// Entity byte6724 bit80, cleared by base ctor82e97d48 and set by guard ctor.
    pub(crate) navigation_rig_enabled: bool,
    /// mbSmoothingEnabled, byte6724 bit20; native ctor inserts7<<4.
    pub(crate) navigation_smoothing_enabled: bool,
    /// IsNavRigOverrideEnabled82e94e70: adjusted IPedestrian+1998 bit10.
    pub(crate) navigation_rig_override: bool,
    /// LWBasicEntity+96 bit80; constructor82e614b4 clears it.
    pub(crate) renderable: bool,
    /// Pedestrian+6724 bit40: ctor82e97d44 inserts7<<4, setting bits70.
    pub(crate) adjust_world_collisions: bool,
    /// IsIgnoreNavMeshInGroundTest82e94e80: entity+6724 bit08, ctor clear.
    pub(crate) ignore_nav_mesh_in_ground_test: bool,
    /// mNavRigIgnoreEntity; Set/ClearNavRigIgnoreVolume82e94c78/80.
    /// Retains target identity rather than a host address truncated to32bits.
    pub(crate) nav_rig_ignore_entity: Option<crate::living_world_chase::TargetableId>,
    /// Pedestrian+1950, initialized invalid at82e97c48/50.
    pub(crate) ignore_collision_target: crate::living_world_chase::TargetableId,
    /// Pedestrian+1958, distinct from collision suppression; ctor82e97c54/58.
    pub(crate) ignore_avoid_target: crate::living_world_chase::TargetableId,
    pub(crate) chasee: Option<crate::living_world_chase::ChaseeState>,
    /// Chaser+8 (complete+c8); allocation zeroing precedes the inline ctor.
    pub(crate) chaser_should_block: bool,
    /// Chaser+16 block-prediction vector; folded getter82a90dc8 adds16.
    pub(crate) chaser_block_position: [f32;4],
    /// LWPatrolZoneOwner+30 (complete+2a0), explicit null82e978bc.
    pub(crate) patrol_entity_of_interest: Option<crate::living_world_chase::TargetableId>,
    pub(crate) patrol_zone_parameters: Option<crate::attribute_runtime::AttributeRuntimeInstance>,
    /// Sole LWPatrolZoneOwner start point, native+280; initially MemStore zero.
    pub(crate) patrol_zone_start_point: [f32; 4],
    /// LWPatrolZoneOwner+20 (complete+290), independent of the start point.
    pub(crate) patrol_point_of_interest: [f32; 4],
    pub(crate) patrol_returning: bool,
    /// mpInteractingObject+1a30, ctor82e97d2c clears the pointer.
    pub(crate) interacting_object: Option<u32>,
    /// mpHandProp+1a34, cleared by82e97d30; distinct from selected spawn metadata.
    pub(crate) held_hand_prop: Option<u32>,
    /// mLastFrameHandPosition; native MemStore zero, updated only by prediction.
    pub(crate) last_frame_hand_position: [f32;4],
    /// LWBehaviourSink+60: actual per-sink controller copy and source identity.
    pub(crate) plugin: Option<Arc<crate::living_world_grind_services::waypoints::plugin::attached::AttachedPlugin>>,
    pub(crate) plugin_removal_requested: bool,
    pub(crate) intent_queue: Option<IntentQueue>,
    pub(crate) navigation_intent: crate::pedestrian_navigation_modifiers::intent::NavigationIntentState,
    pub(crate) perception: perception::PedestrianPerception,
    pub(crate) state_out: crate::pedestrian_motion_delegate::frame::PedStateOut,
    pub(crate) collision_info: crate::pedestrian_graph::colliding::PedestrianCollisionInfo,
    pub(crate) navigation_modifiers: Option<crate::pedestrian_navigation_modifiers::NavigationModifiers>,
    pub(crate) animation: Option<EntityAnimation>,
    pub(crate) nav_rig: Option<crate::pedestrian_navigation::rig::NavRig>,
    pub(crate) collision: Option<crate::pedestrian_collision::PedestrianCollision>,
    pub(crate) post_adjust: Option<crate::pedestrian_adjust_owners::PostAdjust>,
    pub(crate) head_tracking: Option<crate::pedestrian_adjust_owners::HeadTracking>,
    pub(crate) spatial_adjust: Option<crate::pedestrian_adjust_owners::SpatialAdjust>,
    pub(crate) graph: PedestrianGraph,
    pub(crate) ai_bindings: Option<crate::pedestrian_graph::PedestrianAiBindings>,
    pub(crate) behaviours: PedestrianGraphBehaviours,
}
impl PedestrianConstruction {
    pub(crate) fn new(
        id: u32,
        entity_allocation: u32,
        attrib_cache: Arc<LwAttribCache>,
    ) -> Self {
        Self {
            id,
            allocations: vec![(PedestrianStore::Entity, entity_allocation)],
            authority: ActorGraphAuthority::new(PedestrianIdentity { id }),
            core: None,
            rain_data: None,
            model: None,
            entity_category_key: None,
            hand_prop_attributes: None,
            hand_prop_usage: hand_prop::HandPropUsage::default(),
            positioning_test_start_height: None,
            target_iterators: targeting::TargetIterators::new(),
            current_waypoint: None,
            contained_props: Vec::new(),
            attrib_cache,
            resources: None,
            mesh_navigation: None,
            physics_navigation: None,
            active_navigation: ActiveNavigation::Physics,
            state: None,
            security_guard: None,
            big_black: None,
            challenge_owner: None,
            navigation_rig_enabled: false,
            navigation_smoothing_enabled: true,
            navigation_rig_override: true,
            renderable: false,
            adjust_world_collisions: true,
            ignore_nav_mesh_in_ground_test: false,
            nav_rig_ignore_entity: None,
            ignore_collision_target: crate::living_world_chase::TargetableId::INVALID,
            ignore_avoid_target: crate::living_world_chase::TargetableId::INVALID,
            chasee: None,
            chaser_should_block: false,
            chaser_block_position: [0.;4],
            patrol_entity_of_interest: None,
            patrol_zone_parameters: None,
            patrol_zone_start_point: [0.; 4],
            patrol_point_of_interest: [0.; 4],
            patrol_returning: false,
            interacting_object: None,
            held_hand_prop: None,
            last_frame_hand_position: [0.;4],
            plugin: None,
            plugin_removal_requested: false,
            intent_queue: None,
            navigation_intent: crate::pedestrian_navigation_modifiers::intent::NavigationIntentState::new(),
            perception: perception::PedestrianPerception::new(),
            state_out: crate::pedestrian_motion_delegate::frame::PedStateOut::new(),
            collision_info: crate::pedestrian_graph::colliding::PedestrianCollisionInfo::new(),
            navigation_modifiers: None,
            animation: None,
            nav_rig: None,
            collision: None,
            post_adjust: None,
            head_tracking: None,
            spatial_adjust: None,
            graph: PedestrianGraph::default(),
            ai_bindings: None,
            behaviours: PedestrianGraphBehaviours::default(),
        }
    }
    /// SwitchNavigationMode82e6d7d8: types0/1 select the retained mesh
    /// owner, type2 selects physics, other values leave the pointer unchanged.
    /// No navigation settings are copied or reset when switching.
    pub(crate) fn switch_navigation_mode(&mut self, kind: u32) {
        match kind {
            0 | 1 => self.active_navigation = ActiveNavigation::Mesh,
            2 => self.active_navigation = ActiveNavigation::Physics,
            _ => {}
        }
    }

    /// SecurityGuard82e98778 extends this same pedestrian after its base
    /// constructor. Its store10 and patrol-zone data are not a second entity.
    pub(crate) fn initialize_security_guard(&mut self, memory: &mut PedestrianMemory)
        -> Result<(), String> {
        if self.nav_rig.is_none() || self.animation.is_none() {
            return Err("security guard requires completed pedestrian base construction".into());
        }
        self.reserve(memory, PedestrianStore::SecurityGuard)?;
        self.security_guard = Some(crate::living_world_security_guard::SecurityGuardData::default());
        self.navigation_rig_enabled = true;
        Ok(())
    }

    /// Guard PostInitialize82e70330, after the factory places the entity.
    pub(crate) fn post_initialize_security_guard(
        &mut self, painter: &crate::world_painter_layer::WorldPainterLayerMan,
    ) -> Result<(), String> {
        let position = self.core.as_ref().ok_or("guard core absent")?.transform[3];
        self.patrol_zone_start_point = position;
        let state = self.state.as_mut().ok_or("guard AI state absent")?;
        self.security_guard.as_mut().ok_or("guard data absent")?
            .post_initialize(painter, [position[0], position[1], position[2]])
            .map_err(|e| e.to_string())?;
        state.flags[0] |= 8;
        Ok(())
    }

    /// InitModels82e78580 prefix, before model lookup/collision initialization.
    pub(crate) fn initialize_model_identity(
        &mut self, recipe_id: u64, vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<(), String> {
        let core = self.core.as_mut().ok_or("pedestrian core absent")?;
        let model = vault.instance(crate::living_world_recipe_manager::LIVING_WORLD_MODELS_CLASS_ID,
            core.model_key);
        core.recipe_id = recipe_id;
        core.initialize_model_colours(&model)
            .map_err(|e| format!("pedestrian model colours: {e:?}"))
    }

    /// CreateLivingWorldCollision82e78718, called by InitModels.
    pub(crate) fn create_collision(&mut self, memory: &mut PedestrianMemory,
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<(), String> {
        self.reserve(memory, PedestrianStore::Collision)?;
        self.collision = Some(crate::pedestrian_collision::PedestrianCollision::new(vault));
        Ok(())
    }

    pub(crate) fn navigation_mut(&mut self) -> Result<&mut PedestrianNavigation, String> {
        self.navigation_if_present_mut()
            .ok_or_else(|| "pedestrian navigation not constructed".into())
    }
    /// Native GetNavigation can return null during retained construction.
    pub(crate) fn navigation_if_present_mut(&mut self) -> Option<&mut PedestrianNavigation> {
        match self.active_navigation {
            ActiveNavigation::Mesh => self.mesh_navigation.as_mut().map(|nav| &mut nav.base),
            ActiveNavigation::Physics => self.physics_navigation.as_mut(),
        }
    }

    pub(crate) fn uses_mesh_navigation(&self) -> bool {
        matches!(self.active_navigation, ActiveNavigation::Mesh)
    }

    /// Pedestrian82e97748: retained stores7/8/9 follow motion graph loading
    /// and precede NavRig. Bone mappings use the actual animation hierarchy.
    pub(crate) fn initialize_adjust_owners(
        &mut self, memory: &mut PedestrianMemory,
        bind_pose: AnimationPoseRef,
        hierarchy: Arc<skate2_formats::andale::AndaleHierarchy>,
    ) -> Result<(), String> {
        use crate::pedestrian_adjust_owners::{
            PostAdjust, HeadTracking, SpatialAdjust,
            POST_ADJUST_REFERENCE, HEAD_TRACKING_REFERENCE,
        };
        self.reserve(memory, PedestrianStore::PostAdjust)?;
        let attributes = self.resources.as_ref().ok_or("pedestrian resources absent")?
            .entity.try_ref_spec_instance(POST_ADJUST_REFERENCE)
            .map_err(|e| e.to_string())?.ok_or("pedestrian post-adjust reference absent")?;
        self.post_adjust = Some(PostAdjust::before_constructor(attributes, bind_pose, hierarchy));
        self.post_adjust.as_mut().expect("post-adjust retained").construct()?;
        self.reserve(memory, PedestrianStore::HeadTracking)?;
        let attributes = self.resources.as_ref().ok_or("pedestrian resources absent")?
            .entity.try_ref_spec_instance(HEAD_TRACKING_REFERENCE)
            .map_err(|e| e.to_string())?.ok_or("pedestrian head-tracking reference absent")?;
        self.head_tracking = Some(HeadTracking::new(attributes));
        self.reserve(memory, PedestrianStore::SpatialAdjust)?;
        self.spatial_adjust = Some(SpatialAdjust::new());
        Ok(())
    }

    /// Constructor82e98068 reserves store3 after animation/adjust construction.
    pub(crate) fn initialize_nav_rig(&mut self, memory: &mut PedestrianMemory) -> Result<(), String> {
        self.reserve(memory, PedestrianStore::NavRig)?;
        self.nav_rig = Some(crate::pedestrian_navigation::rig::NavRig::new());
        let modifiers = self.navigation_modifiers.as_mut().ok_or("pedestrian navigation modifiers absent")?;
        let mesh = self.mesh_navigation.as_ref().ok_or("pedestrian mesh navigation absent")?;
        modifiers.initialize_with_mesh(&mesh.base);
        let active = match self.active_navigation {
            ActiveNavigation::Mesh => &mesh.base,
            ActiveNavigation::Physics => self.physics_navigation.as_ref().ok_or("pedestrian physics navigation absent")?,
        };
        modifiers.reset_enabled_to_attributes(&active.parameters)?;
        Ok(())
    }

    /// Pedestrian::TransformNavRig82e6d848 first normalizes the actual Z axis.
    pub(crate) fn transform_nav_rig<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self, tuning: &crate::pedestrian_navigation::rig::NavRigTuning,
        rotations: &[(f32, f32); 7], numeric: &mut N,
    ) -> Result<(), String>
    where N::Error: std::fmt::Debug,
    {
        let transform = self.core.as_ref().ok_or("pedestrian core not constructed")?.transform;
        let heading = crate::reckoning_numeric::try_normalize_safe(transform[2], [0.; 4], numeric)
            .map_err(|e| format!("pedestrian rig heading: {e:?}"))?;
        self.nav_rig.as_mut().ok_or("pedestrian rig not constructed")?
            .transform(transform[3], heading, tuning.distance, tuning.physics_caching,
                tuning.cache_distance, rotations, numeric)
            .map_err(|e| format!("pedestrian rig transform: {e:?}"))
    }

    /// Task_ValidateNavRigs82e73628 brackets even disabled validation with the
    /// batch count. `enabled` is the entity's native byte6724 bit80 producer.
    pub(crate) fn validate_nav_rig<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self, enabled: bool, tuning: &crate::pedestrian_navigation::rig::NavRigTuning,
        batch: &mut crate::pedestrian_navigation::rig::NavRigBatch, numeric: &mut N,
    ) -> Result<(), String>
    where N::Error: std::fmt::Debug,
    {
        let rig = self.nav_rig.as_mut().ok_or("pedestrian rig not constructed")?;
        rig.batch_range.start = batch.count();
        if enabled && tuning.path_validation {
            // PhysicsNavigation vtable823a4390+104/+108 both resolve to blr.
            if let ActiveNavigation::Mesh = self.active_navigation {
                let navigation = self.mesh_navigation.as_ref().ok_or("pedestrian mesh navigation absent")?;
                if tuning.batch_tests {
                    navigation.add_nav_rig_tests(&rig.directions, &rig.distances, self.id, batch)?;
                } else {
                    navigation.validate_nav_rig(&rig.directions, &mut rig.distances, numeric)?;
                }
            }
        }
        rig.batch_range.end = batch.count();
        Ok(())
    }

    pub(crate) fn receive_nav_rig_batch(
        &mut self, batch: &crate::pedestrian_navigation::rig::NavRigBatch,
    ) -> Result<(), String> {
        let rig = self.nav_rig.as_mut().ok_or("pedestrian rig not constructed")?;
        batch.apply(rig.batch_range.clone(), &mut rig.distances)?;
        rig.batch_range = 0..0;
        Ok(())
    }

    /// AddTestsToGroundBatch82e78818 reads this entity's current position.
    /// The execution manager supplies the shared batch (native capacity80).
    pub(crate) fn add_ground_probes<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self, enabled: bool, ignore_volume_id: u32,
        tuning: &crate::pedestrian_navigation::rig::NavRigProbeTuning,
        requests: &mut Vec<crate::pedestrian_navigation::rig::NavRigPhysicsRequest>,
        capacity: usize, numeric: &mut N,
    ) -> Result<(), String>
    where N::Error: std::fmt::Debug,
    {
        let rig = self.nav_rig.as_mut().ok_or("pedestrian rig not constructed")?;
        if !enabled || rig.reuse_physics { return Ok(()); }
        let position = self.core.as_ref().ok_or("pedestrian core not constructed")?.transform[3];
        rig.add_ground_probes(enabled, self.id, ignore_volume_id, position,
            tuning, requests, capacity, numeric)
    }

    /// AddTestsToVolumeBatch82e79478: same owner/position, without ground cache gate.
    pub(crate) fn add_volume_probes<N: crate::reckoning_numeric::ReckoningNumeric>(
        &self, enabled: bool, ignore_volume_id: u32,
        fatness: f32, y_clearance: f32, start_offset: f32,
        requests: &mut Vec<crate::pedestrian_navigation::rig::NavRigPhysicsRequest>,
        capacity: usize, numeric: &mut N,
    ) -> Result<(), String>
    where N::Error: std::fmt::Debug,
    {
        if !enabled { return Ok(()); }
        let position = self.core.as_ref().ok_or("pedestrian core not constructed")?.transform[3];
        self.nav_rig.as_ref().ok_or("pedestrian rig not constructed")?
            .add_volume_probes(enabled, self.id, ignore_volume_id, position,
                fatness, y_clearance, start_offset, requests, capacity, numeric)
    }

    /// Pedestrian::OrientNavRigToWorldTopology82e6d960 uses actual core position.
    pub(crate) fn orient_nav_rig<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self, enabled: bool, tuning: &crate::pedestrian_navigation::rig::NavRigTuning,
        mesh: &crate::living_world_mesh_store::MeshStore, numeric: &mut N,
    ) -> Result<(), String>
    where N::Error: std::fmt::Debug,
    {
        if !enabled { return Ok(()); }
        let position = self.core.as_ref().ok_or("pedestrian core not constructed")?.transform[3];
        self.nav_rig.as_mut().ok_or("pedestrian rig not constructed")?
            .orient_to_topology(position, tuning.topology_angle, tuning.topology_distance, mesh, numeric)
    }

    fn reserve(
        &mut self,
        memory: &mut PedestrianMemory,
        store: PedestrianStore,
    ) -> Result<(), String> {
        if self
            .allocations
            .iter()
            .any(|(existing, _)| *existing == store)
        {
            return Err(format!("pedestrian construction repeated {store:?}"));
        }
        let slot = memory
            .request(store)
            .ok_or_else(|| format!("pedestrian memory exhausted {store:?}"))?;
        self.allocations.push((store, slot));
        Ok(())
    }
    /// Constructor prefix through the state and animation allocation. Both
    /// navigation bases read the same authored entity RefSpec. No transform is
    /// guessed here: placement belongs to the factory's later Initialize call.
    pub(crate) fn initialize_core(
        &mut self,
        memory: &mut PedestrianMemory,
        vault: &Arc<AttributeRuntimeVault>,
        entity_key: u64,
        model_key: u64,
    ) -> Result<(), String> {
        if self.resources.is_some() {
            return Err("pedestrian construction already attempted".into());
        }
        //Base Chasee825f6990 calls ChaseeReset82de2310 before the entity
        //attribute/navigation allocations. Reuse the authored global owner.
        let chase = crate::living_world_chase::attributes::ChaseSettings::load(vault)?;
        self.chasee = Some(chase.chasee());
        //Base LWPatrolZoneOwnerReset82e51780 follows Chasee construction.
        self.patrol_zone_parameters=Some(vault.instance(0xb5de_32c9_7060_5efa,
            skate2_formats::attrib::string_hash64("default")));
        self.resources = Some(
            PedestrianResources::load(vault, entity_key, model_key).map_err(|e| e.to_string())?,
        );
        self.reserve(memory, PedestrianStore::Core)?;
        let mut core = CommonEntityCore::new();
        core.entity_type = 2; // 82e97dcc..dd4, PEDESTRIANTYPE
        core.entity_key = entity_key;
        core.model_key = model_key;
        let position = core.transform[3];
        self.core = Some(core);
        let resources = self.resources.as_ref().expect("resources assigned above");
        self.mesh_navigation = Some(MeshNavigation::new(
            PedestrianNavigation::initialize(&resources.entity, position, 0.)
                .map_err(|e| format!("pedestrian mesh navigation: {e:?}"))?,
        ));
        self.physics_navigation = Some(
            PedestrianNavigation::initialize(&resources.entity, position, 0.)
                .map_err(|e| format!("pedestrian physics navigation: {e:?}"))?,
        );
        self.reserve(memory, PedestrianStore::Intent)?;
        self.intent_queue = Some(IntentQueue::new());
        self.navigation_modifiers = Some(crate::pedestrian_navigation_modifiers::NavigationModifiers::new(
            self.intent_queue.as_mut().expect("intent owner constructed"))?);
        self.reserve(memory, PedestrianStore::AiState)?;
        self.state = Some(PedestrianStateStorage::default());
        Ok(())
    }
    pub(crate) fn initialize_animation(
        &mut self,
        memory: &mut PedestrianMemory,
        world: Arc<Mutex<PhysicalAssemblyWorld>>,
        playback: Arc<Mutex<FacePlaybackData>>,
        bind_pose: AnimationPoseRef,
        attributes: Arc<Mutex<AnimationAttributePool>>,
        motion_pool: u32,
        registered_sim_lod_matrix_id: u32,
    ) -> Result<(), String> {
        if self.state.is_none() {
            return Err("pedestrian state not constructed".into());
        }
        self.reserve(memory, PedestrianStore::Animation)?;
        self.animation = Some(
            EntityAnimation::new(
                &self.authority,
                world,
                playback,
                bind_pose,
                attributes,
                motion_pool,
            )
            .map_err(|e| format!("pedestrian animation: {e:?}"))?,
        );
        let data = self.resources.as_ref().ok_or("pedestrian animation RefSpec absent")?.animation.clone();
        let animation = self.animation.as_mut().expect("retained animation");
        animation.entity_animation_data = Some(data.clone());
        animation.override_animation_data = Some(data);
        animation.initialize_sim_lod_pose(registered_sim_lod_matrix_id)?;
        Ok(())
    }
}
