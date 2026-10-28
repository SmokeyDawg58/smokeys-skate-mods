//! Concrete callback bridge for the original Vehicle.xml graph. Borrows a
//! vehicle's fields and existing world services; spawning/physics remain the
//! original EntityMan/Vehicle owner responsibility, not a second traffic owner.
use super::{
    VehicleCondition, VehicleGraphRuntime, VehicleGraphView, VehicleHook, VehicleState,
    conditions::VehicleDecisionView,
};
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::vehicle_motion::{
        VehicleMotion,
        driving::{DrivingError, DrivingVehicles, VehicleRoute},
        obstacles::FrontEntityGeometry,
    },
    living_world_mesh_store::MeshStore,
    living_world_rain::RainMan,
    native_skater_random::{NativeRandomGate, NativeSkaterRandom},
    reckoning_numeric::ReckoningNumeric,
};
pub(crate) struct VehicleBehaviourFrame<'a, V, N> {
    pub(crate) motion: &'a mut VehicleMotion,
    pub(crate) route: &'a mut VehicleRoute,
    pub(crate) entity_id: u32,
    pub(crate) length: f32,
    pub(crate) rain: &'a mut RainMan,
    pub(crate) mesh: &'a MeshStore,
    pub(crate) vehicles: &'a mut V,
    pub(crate) characteristics: &'a AttributeRuntimeInstance,
    pub(crate) driver: &'a AttributeRuntimeInstance,
    /// Actual perception observation, consumed by the one selected update.
    pub(crate) front_entity: Option<FrontEntityGeometry>,
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) random: &'a mut NativeSkaterRandom,
    pub(crate) random_gate: &'a NativeRandomGate,
    pub(crate) numeric: &'a mut N,
}
impl<V: DrivingVehicles, N: ReckoningNumeric> VehicleGraphRuntime
    for VehicleBehaviourFrame<'_, V, N>
{
    type Error = DrivingError<V::Error, N::Error>;
    fn condition(
        &mut self,
        condition: VehicleCondition,
        _view: VehicleGraphView<'_>,
    ) -> Result<bool, Self::Error> {
        let route = &*self.route;
        let decision = VehicleDecisionView {
            entity_id: self.entity_id,
            rain_item_type: route.item_type,
            rain_item_id: route.item_id,
            intersection_id: route.intersection_id,
            road_lane: route.road_lane,
            change_to_lane: route.change_to_lane,
            super_road_position: route.super_road_position,
            pull_over_position: route.pull_over_position,
            backup_position: route.backup_position,
            lane_change_position: route.lane_change_position,
            lane_change_length: route.lane_change_passage.curve.length,
            action: route.action,
            registered_in_intersection: route.registered_in_intersection,
            length: self.length,
            motion: self.motion,
            characteristics: self.characteristics,
            driver: self.driver,
        };
        decision
            .evaluate(condition, self.rain, self.vehicles)
            .map_err(DrivingError::Decision)
    }
    fn hook(&mut self, hook: VehicleHook, _view: VehicleGraphView<'_>) -> Result<(), Self::Error> {
        self.route.on_transition(
            hook,
            self.motion,
            self.entity_id,
            self.length,
            self.rain,
            self.mesh,
            self.vehicles,
            self.driver,
            self.cvars,
            self.numeric,
        )
    }
    fn begin(
        &mut self,
        state: VehicleState,
        _view: VehicleGraphView<'_>,
    ) -> Result<(), Self::Error> {
        match state {
            VehicleState::Impatience => self.motion.backing_up = true, //82d16288
            VehicleState::StayingParked => self.motion.parked = true,  //82d162b8
            _ => {} // Original Begin pointers all shared no-op825187c0.
        }
        Ok(())
    }
    fn end(&mut self, state: VehicleState, _view: VehicleGraphView<'_>) -> Result<(), Self::Error> {
        match state {
            VehicleState::Impatience => self.motion.backing_up = false, //82d162a0
            VehicleState::StayingParked => self.motion.parked = false,  //82d163a8
            VehicleState::PassingIntersection => self.motion.cornering = Some(0.), //82d163c0
            _ => {} // Other original End pointers no-op825187c0.
        }
        Ok(())
    }
    fn update(
        &mut self,
        state: VehicleState,
        view: VehicleGraphView<'_>,
    ) -> Result<(), Self::Error> {
        let dt = view.delta_time;
        match state {
            VehicleState::FollowingLane => self.route.continue_following_lane(
                self.motion,
                self.entity_id,
                self.length,
                dt,
                self.rain,
                self.mesh,
                self.vehicles,
                self.characteristics,
                self.driver,
                self.front_entity.take(),
                self.cvars,
                self.random,
                self.random_gate,
                self.numeric,
            ),
            VehicleState::Impatience => self.route.continue_impatience(
                self.motion,
                self.entity_id,
                self.length,
                dt,
                self.rain,
                self.vehicles,
                self.characteristics,
                self.driver,
                self.front_entity.take(),
                self.numeric,
            ),
            VehicleState::ChangingLane | VehicleState::PullingOver => {
                self.route.continue_lane_maneuver(
                    state == VehicleState::PullingOver,
                    self.motion,
                    self.entity_id,
                    self.length,
                    dt,
                    self.rain,
                    self.vehicles,
                    self.characteristics,
                    self.driver,
                    self.front_entity.take(),
                    self.cvars,
                    self.numeric,
                )
            }
            VehicleState::StayingParked => {
                self.motion.continue_staying_parked(dt);
                Ok(())
            }
            VehicleState::PullingOut => self.route.continue_pulling_out(
                self.motion,
                self.entity_id,
                self.length,
                dt,
                self.rain,
                self.vehicles,
                self.characteristics,
                self.driver,
                self.front_entity.take(),
                self.numeric,
            ),
            VehicleState::PassingIntersection => self.route.continue_passing_intersection(
                self.motion,
                self.entity_id,
                self.length,
                dt,
                self.rain,
                self.mesh,
                self.vehicles,
                self.characteristics,
                self.driver,
                self.front_entity.take(),
                self.numeric,
            ),
        }
    }
}
