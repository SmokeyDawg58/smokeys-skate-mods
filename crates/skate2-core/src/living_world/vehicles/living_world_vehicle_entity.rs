//! Vehicle fields of the existing LivingWorld EntityMan, not a second roster.
//! Selected ctor82d1cab0. Publication still requires the complete factory,
//! collision/model admission and AttachToManagers sequence.
use super::{
    character_models::CharacterModel,
    dmo_objects::ENTITIES_CLASS,
    entities::{EntityMan, SpatialEntity},
    entity_core::CommonEntityCore,
    vehicle_graph::VehicleGraph,
    vehicle_motion::{
        CHARACTERISTICS_CLASS, StopReason, VehicleMotion,
        attributes::DRIVERS_CLASS,
        driving::{DrivingVehicles, VehicleRoute},
        nav_rig::VehicleNavRigValidator,
        signals::VehicleSignals,
        vision::VehicleVision,
        wheels::VehicleWheelState,
    },
};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    living_world_rain::{
        intersection_vehicles::IntersectionVehicles, spatial::RainItem, vehicle_lanes::RainVehicles,
    },
};
use std::{collections::BTreeMap, sync::Arc};

#[path = "living_world_vehicle_collision.rs"]
pub(crate) mod collision;
#[path = "living_world_vehicle_execution.rs"]
pub(crate) mod execution;
#[path = "living_world_vehicle_factory.rs"]
pub(crate) mod factory;
#[path = "living_world_vehicle_frame.rs"]
pub(crate) mod frame;
#[path = "living_world_vehicle_model_init.rs"]
pub(crate) mod model_init;
#[path = "living_world_vehicle_perception.rs"]
pub(crate) mod perception;
#[path = "living_world_vehicle_skitch_perception.rs"]
pub(crate) mod skitch_perception;
#[path = "living_world_vehicle_spawn.rs"]
pub(crate) mod spawn;
#[path = "living_world_vehicle_spawn_overlap.rs"]
pub(crate) mod spawn_overlap;
#[path = "living_world_vehicle_spawn_road.rs"]
pub(crate) mod spawn_road;
#[path = "living_world_vehicle_state_frame.rs"]
pub(crate) mod state_frame;
#[path = "living_world_vehicle_target_frame.rs"]
pub(crate) mod target_frame;

pub(crate) struct Vehicle {
    pub(crate) id: u32,
    pub(super) allocation: factory::VehicleAllocation,
    pub(crate) core: CommonEntityCore,
    pub(crate) rain_data: Option<RainItem>,
    /// LWBasicEntity.mpAttribData+54 retains the Manager's model-key cache.
    pub(crate) attrib_cache: Arc<super::attrib_cache::LwAttribCache>,
    pub(crate) model: Option<Arc<CharacterModel>>,
    /// Created by Collideable.Init during InitModels, not by these ctor fields.
    pub(crate) collision: Option<collision::VehicleCollision>,
    pub(crate) graph: Option<VehicleGraph>,
    pub(crate) grab_splines: BTreeMap<u32, super::dmo_models::DmoGrabSpline>,
    pub(crate) grind_splines: BTreeMap<u32, super::dmo_models::DmoGrindSpline>,
    pub(crate) motion: VehicleMotion,
    pub(crate) route: VehicleRoute,
    pub(crate) signals: VehicleSignals,
    pub(crate) wheels: VehicleWheelState,
    pub(crate) vision: VehicleVision,
    pub(crate) nav_rig: Option<VehicleNavRigValidator>,
    pub(crate) characteristics: AttributeRuntimeInstance,
    pub(crate) driver: AttributeRuntimeInstance,
    pub(crate) force_new_action: bool,
    pub(crate) being_skitched_previous_frame: bool,
    pub(crate) local_player_grabbing_previous_frame: bool,
    pub(crate) skater_collision_flags: u32,
    pub(crate) entity_in_vision: bool,
    pub(crate) nearest_vision_position: [f32; 4],
    pub(crate) nearest_vision_radius: [f32; 4],
    pub(crate) nearest_compulsory_stop: bool,
    pub(crate) nearest_pedestrian: Option<u32>,
}
impl Vehicle {
    /// UpdatePositionalData82e50040. Vehicle.GetPosition82d36140 reads core
    /// transform translation; both trailing RAIN/WorldPainter virtuals in
    /// vtable82326bc8 (+dc/+e0) are the real no-op825187c0 for this category.
    pub(crate) fn update_positional_data<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self,
        player_position: [f32; 4],
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        self.core
            .player_distance
            .update(player_position, self.core.transform[3], numeric)
    }

    /// Does not publish an incomplete vehicle or claim the constructor's
    /// collision/grab/grind registration side effects have already run.
    pub(crate) fn construct_fields(
        id: u32,
        allocation: factory::VehicleAllocation,
        entity_key: u64,
        model_key: u64,
        attrib_cache: Arc<super::attrib_cache::LwAttribCache>,
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        // Factory has consumed the ID, then requested the core-data block.
        let mut core = CommonEntityCore::new();
        core.entity_type = 3;
        core.entity_key = entity_key;
        core.model_key = model_key;
        let entity_attributes = vault.instance(ENTITIES_CLASS, entity_key);
        let characteristic_key = entity_attributes
            .try_ref_spec_collection(0x92a0_43b4_a11f_1a2a)?
            .unwrap_or(0);
        let characteristics = vault.instance(CHARACTERISTICS_CLASS, characteristic_key);
        let driver_key = entity_attributes
            .try_ref_spec_collection(0x023e_f929_823a_3c50)?
            .unwrap_or(0);
        let driver = vault.instance(DRIVERS_CLASS, driver_key);
        // This entity comes from MemoryMan's cleared MemStore block:
        // Init82a6dd98 and FreeBlock82a6df20 both zero the whole allocation.
        // Vehicle's constructor preserves these bytes; they are known zero
        // here, including before its first intersection transition.
        let mut motion = VehicleMotion::new();
        motion.breaking_rules = Some(false);
        motion.cornering = Some(0.);
        motion.skitching_speed_limit_delta = Some(0.);
        Ok(Self {
            id,
            allocation,
            core,
            rain_data: None,
            attrib_cache,
            model: None,
            collision: None,
            graph: None,
            grab_splines: BTreeMap::new(),
            grind_splines: BTreeMap::new(),
            motion,
            route: VehicleRoute::new(),
            signals: VehicleSignals::new(),
            wheels: VehicleWheelState::new(),
            vision: VehicleVision::new(),
            nav_rig: None,
            characteristics,
            driver,
            force_new_action: true,
            being_skitched_previous_frame: false,
            local_player_grabbing_previous_frame: false,
            skater_collision_flags: 0,
            entity_in_vision: false,
            nearest_vision_position: [0.; 4],
            nearest_vision_radius: [0.; 4],
            nearest_compulsory_stop: false,
            nearest_pedestrian: None,
        })
    }
    /// LWBasicEntity.GetLength82e4fec0, GetDimensions82e4bfe0: zero when
    /// mpEntityModel is null; otherwise the model's actual authored dimensions.
    pub(crate) fn dimensions(&self) -> [f32; 4] {
        self.model
            .as_ref()
            .map_or([0.; 4], |model| model.dimensions)
    }
    pub(crate) fn length(&self) -> f32 {
        self.dimensions()[2]
    }
}
impl SpatialEntity for Vehicle {
    fn core_position_xz(&self) -> [f32; 2] {
        self.core.position_xz
    }
    fn core_data_valid(&self) -> bool {
        true
    } // owned non-null core, source proxy IsValid
}
#[derive(Debug)]
pub(crate) enum VehicleLookupError {
    CullPending(u32),
    Missing(u32),
    UninitializedBreakingRules(u32),
}
/// Read-only borrow of the fields used by RAIN. No frame snapshot: each query
/// reads the original live entity, or the updating entity's currently borrowed
/// route/motion inside a read-only RAIN call.
pub(crate) struct VehicleRead<'a> {
    pub(crate) length: f32,
    pub(crate) route: &'a VehicleRoute,
    pub(crate) motion: &'a VehicleMotion,
}
pub(crate) trait VehicleReader {
    fn vehicle(&self, id: u32) -> Result<VehicleRead<'_>, VehicleLookupError>;
}
impl VehicleReader for EntityMan<24, Vehicle> {
    fn vehicle(&self, id: u32) -> Result<VehicleRead<'_>, VehicleLookupError> {
        let vehicle = self.get(id).ok_or(VehicleLookupError::Missing(id))?;
        Ok(VehicleRead {
            length: vehicle.length(),
            route: &vehicle.route,
            motion: &vehicle.motion,
        })
    }
}
pub(crate) struct OtherVehicles<'a>(pub(crate) Vec<(u32, &'a Vehicle)>);
impl VehicleReader for OtherVehicles<'_> {
    fn vehicle(&self, id: u32) -> Result<VehicleRead<'_>, VehicleLookupError> {
        let vehicle = self
            .0
            .iter()
            .find(|(key, _)| *key == id)
            .map(|(_, vehicle)| *vehicle)
            .ok_or(VehicleLookupError::Missing(id))?;
        Ok(VehicleRead {
            length: vehicle.length(),
            route: &vehicle.route,
            motion: &vehicle.motion,
        })
    }
}
impl<T: VehicleReader> RainVehicles for T {
    type Error = VehicleLookupError;
    fn require_vehicle(&mut self, id: u32) -> Result<(), Self::Error> {
        self.vehicle(id).map(|_| ())
    }
    fn position(&mut self, id: u32) -> Result<f32, Self::Error> {
        Ok(self.vehicle(id)?.route.super_road_position)
    }
    fn length(&mut self, id: u32) -> Result<f32, Self::Error> {
        Ok(self.vehicle(id)?.length)
    }
    fn speed(&mut self, id: u32) -> Result<f32, Self::Error> {
        Ok(self.vehicle(id)?.motion.speed)
    }
}
impl<T: VehicleReader> IntersectionVehicles for T {
    fn rain_item_type(&mut self, id: u32) -> Result<u32, Self::Error> {
        Ok(self.vehicle(id)?.route.item_type)
    }
    fn super_road_id(&mut self, id: u32) -> Result<u64, Self::Error> {
        Ok(self.vehicle(id)?.route.super_road_id)
    }
    fn current_position(&mut self, id: u32) -> Result<f32, Self::Error> {
        Ok(self.vehicle(id)?.route.item_position)
    }
    fn breaking_rules(&mut self, id: u32) -> Result<bool, Self::Error> {
        self.vehicle(id)?
            .motion
            .breaking_rules
            .ok_or(VehicleLookupError::UninitializedBreakingRules(id))
    }
    fn minimum_intersection_distance(&mut self, id: u32) -> Result<f32, Self::Error> {
        Ok(self.vehicle(id)?.motion.minimum_intersection_distance)
    }
    fn minimum_vehicle_distance(&mut self, id: u32) -> Result<f32, Self::Error> {
        Ok(self.vehicle(id)?.motion.minimum_vehicle_distance)
    }
}
impl<T: VehicleReader> DrivingVehicles for T {
    fn stopping_reason(&mut self, id: u32) -> Result<StopReason, Self::Error> {
        Ok(self.vehicle(id)?.motion.stopping_reason)
    }
    fn lane_change_passage_valid(&mut self, id: u32) -> Result<bool, Self::Error> {
        Ok(self.vehicle(id)?.route.lane_change_passage.valid)
    }
    fn backup_position(&mut self, id: u32) -> Result<f32, Self::Error> {
        Ok(self.vehicle(id)?.route.backup_position)
    }
}
