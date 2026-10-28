//! Borrow the updating car during read-only RAIN approval/parking queries.
//! The other-car provider still reads the live EntityMan payloads. The borrow
//! ends before route/motion writes resume, so no previous-frame self snapshot
//! or temporary removal from the road/entity maps is needed.
use super::{VehicleMotion, driving::DrivingVehicles};
use crate::living_world_rain::{
    intersection_vehicles::IntersectionVehicles, vehicle_lanes::RainVehicles,
};

pub(crate) struct CurrentVehicle<'a, V> {
    pub(crate) id: u32,
    pub(crate) length: f32,
    pub(crate) position: f32,
    pub(crate) motion: &'a VehicleMotion,
    pub(crate) others: &'a mut V,
}
impl<V: DrivingVehicles> RainVehicles for CurrentVehicle<'_, V> {
    type Error = V::Error;
    fn require_vehicle(&mut self, id: u32) -> Result<(), Self::Error> {
        if id == self.id {
            Ok(())
        } else {
            self.others.require_vehicle(id)
        }
    }
    fn position(&mut self, id: u32) -> Result<f32, Self::Error> {
        if id == self.id {
            Ok(self.position)
        } else {
            self.others.position(id)
        }
    }
    fn length(&mut self, id: u32) -> Result<f32, Self::Error> {
        if id == self.id {
            Ok(self.length)
        } else {
            self.others.length(id)
        }
    }
    fn speed(&mut self, id: u32) -> Result<f32, Self::Error> {
        if id == self.id {
            Ok(self.motion.speed)
        } else {
            self.others.speed(id)
        }
    }
}
impl<V: DrivingVehicles> IntersectionVehicles for CurrentVehicle<'_, V> {
    // These four queries are for other cars only: CheckPassage and the
    // destination/passage queue loops exclude the requesting vehicle.
    fn rain_item_type(&mut self, id: u32) -> Result<u32, Self::Error> {
        self.others.rain_item_type(id)
    }
    fn super_road_id(&mut self, id: u32) -> Result<u64, Self::Error> {
        self.others.super_road_id(id)
    }
    fn current_position(&mut self, id: u32) -> Result<f32, Self::Error> {
        self.others.current_position(id)
    }
    fn breaking_rules(&mut self, id: u32) -> Result<bool, Self::Error> {
        self.others.breaking_rules(id)
    }
    fn minimum_intersection_distance(&mut self, id: u32) -> Result<f32, Self::Error> {
        if id == self.id {
            Ok(self.motion.minimum_intersection_distance)
        } else {
            self.others.minimum_intersection_distance(id)
        }
    }
    fn minimum_vehicle_distance(&mut self, id: u32) -> Result<f32, Self::Error> {
        if id == self.id {
            Ok(self.motion.minimum_vehicle_distance)
        } else {
            self.others.minimum_vehicle_distance(id)
        }
    }
}
