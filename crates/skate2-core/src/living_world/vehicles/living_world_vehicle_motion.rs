//! Vehicle acceleration/speed fields and original update graphs. These are the
//! fields of the eventual Vehicle entity, not another traffic/world owner.
//! UpdateAcceleration82d18310, UpdateSpeed82d18700, GetCurvePosUpdate82d171a8.
use crate::attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance};
#[path = "living_world_vehicle_action_decisions.rs"]
mod action_decisions;
#[path = "living_world_vehicle_attributes.rs"]
pub(crate) mod attributes;
#[path = "living_world_vehicle_driving.rs"]
pub(crate) mod driving;
#[path = "living_world_vehicle_hooks.rs"]
mod hooks;
#[path = "living_world_vehicle_maneuvers.rs"]
mod maneuvers;
#[path = "living_world_vehicle_nav_rig.rs"]
pub(crate) mod nav_rig;
#[path = "living_world_vehicle_network_lifecycle.rs"]
mod network_lifecycle;
#[path = "living_world_vehicle_obstacles.rs"]
pub(crate) mod obstacles;
#[path = "living_world_vehicle_passing.rs"]
mod passing;
#[path = "living_world_vehicle_route_update.rs"]
pub(crate) mod route_update;
#[path = "living_world_vehicle_query.rs"]
mod query;
#[path = "living_world_vehicle_signals.rs"]
pub(crate) mod signals;
#[path = "living_world_vehicle_target_transform.rs"]
pub(crate) mod target_transform;
#[path = "living_world_vehicle_vision.rs"]
pub(crate) mod vision;
#[path = "living_world_vehicle_wheels.rs"]
pub(crate) mod wheels;

pub(crate) const CHARACTERISTICS_CLASS: u64 = 0x62b1_c1b3_7ed2_e2ca;
const SKID_START_DECELERATION: u64 = 0xdd49_e3c2_b833_a723;
const SKITCH_ACTIVATION_SPEED: u64 = 0xd208_26f1_5fb1_5a2e;
const SKITCH_DECELERATION_DISTANCE: u64 = 0x3ab7_fc7c_f7a1_7c81;
const SKITCH_GRAB_DECELERATION_DISTANCE: u64 = 0xf682_d359_cdbc_4d12;
const SKITCH_RADIUS: u64 = 0x3346_6832_d817_8eaf;
const SKITCH_GRAB_RADIUS: u64 = 0xd49f_c490_1918_1ee5;
const SKITCH_SKATER_SPEED_DELTA: u64 = 0x256a_412e_350a_2659;
const METRES_TO_KM: f32 = f32::from_bits(0x4066_6666);
const KM_TO_METRES: f32 = f32::from_bits(0x3e8e_38e4);
const DISTANCE_EPSILON: f32 = f32::from_bits(0x3a83_126f);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum StopReason {
    None = 0,
    Intersection = 1,
    Parking = 2,
    Vehicle = 3,
    Entity = 4,
    RuleBreakingVehicle = 5,
}
pub(crate) trait VehicleCharacteristics {
    type Error;
    fn acceleration_increment(&self) -> Result<f32, Self::Error>;
    fn scalar(&self, key: u64) -> Result<f32, Self::Error>;
}
impl VehicleCharacteristics for AttributeRuntimeInstance {
    type Error = AttributeRuntimeError;
    fn acceleration_increment(&self) -> Result<f32, Self::Error> {
        Ok(self.try_cached_layout_field(0x3c, 4)?.map_or(0., |data| {
            f32::from_be_bytes(data.bytes().try_into().expect("four-byte layout field"))
        }))
    }
    fn scalar(&self, key: u64) -> Result<f32, Self::Error> {
        Ok(f32::from_bits(self.try_scalar_word(key, 0)?.unwrap_or(0)))
    }
}
#[derive(Debug)]
pub(crate) struct VehicleMotion {
    pub(crate) acceleration: f32,
    pub(crate) speed: f32,
    pub(crate) default_acceleration: f32,
    pub(crate) speed_limit_delta: f32,
    /// Constructor leaves this unwritten; UpdateAttributes supplies it.
    pub(crate) skitching_speed_limit_delta: Option<f32>,
    pub(crate) current_speed_limit: f32,
    pub(crate) minimum_intersection_distance: f32,
    pub(crate) minimum_vehicle_distance: f32,
    pub(crate) minimum_entity_distance: f32,
    pub(crate) minimum_parking_distance: f32,
    pub(crate) minimum_lane_change_distance_factor: f32,
    pub(crate) skitch_helper_timer: f32,
    pub(crate) rear_skater_speed: f32,
    pub(crate) rear_skater_distance: f32,
    pub(crate) stopping_reason: StopReason,
    pub(crate) front_vehicle_distance: f32,
    pub(crate) vehicle_waiting_timer: f32,
    pub(crate) entity_waiting_timer: f32,
    pub(crate) parking_timer: f32,
    pub(crate) alarm_timer: f32,
    pub(crate) travel_intention: bool,
    pub(crate) alarming: bool,
    pub(crate) parked: bool,
    pub(crate) cornering: Option<f32>,
    pub(crate) breaking_rules: Option<bool>,
    pub(crate) willing_to_run_red_light: bool,
    pub(crate) local_player_grabbing: bool,
    pub(crate) skidding: bool,
    pub(crate) backing_up: bool,
    pub(crate) force_stopping: bool,
    pub(crate) being_skitched: bool,
    pub(crate) rear_skater_skitch_potential: bool,
    pub(crate) rear_skater_tends_to_skitch: bool,
    pub(crate) parking_started: bool,
}
impl VehicleMotion {
    /// Selected scalar/flag stores in Vehicle ctor82d1cc38..cd80 only.
    pub(crate) fn new() -> Self {
        Self {
            acceleration: 0.,
            speed: 0.,
            default_acceleration: 0.,
            speed_limit_delta: 0.,
            skitching_speed_limit_delta: None,
            current_speed_limit: 0.,
            minimum_intersection_distance: 0.,
            minimum_vehicle_distance: 0.,
            minimum_entity_distance: 0.,
            minimum_parking_distance: 0.,
            minimum_lane_change_distance_factor: 0.,
            skitch_helper_timer: 0.,
            rear_skater_speed: 0.,
            rear_skater_distance: -1.,
            stopping_reason: StopReason::None,
            front_vehicle_distance: -1.,
            vehicle_waiting_timer: 0.,
            entity_waiting_timer: 0.,
            parking_timer: 0.,
            alarm_timer: 0.,
            travel_intention: true,
            alarming: false,
            parked: false,
            cornering: None,      // Not written by VehicleState constructor.
            breaking_rules: None, // ctor preserves low3 bits; later Init supplies it.
            willing_to_run_red_light: false,
            local_player_grabbing: false,
            skidding: false,
            backing_up: false,
            force_stopping: false,
            being_skitched: false,
            rear_skater_skitch_potential: false,
            rear_skater_tends_to_skitch: false,
            parking_started: false,
        }
    }
    pub(crate) fn curve_position_update(&self, position: f32, dt: f32) -> f32 {
        let signed_speed = self.speed * if self.backing_up { -1. } else { 1. };
        signed_speed.mul_add(dt, position) // raw fmadds82d17204
    }
    /// WillRunRedLight82d1a768: grab overrides the driver preference, but the
    /// authored speed test runs first when willing-to-run is set.
    pub(crate) fn will_run_red_light<A: VehicleCharacteristics>(
        &self,
        driver: &A,
    ) -> Result<bool, A::Error> {
        if self.willing_to_run_red_light {
            let speed = self.speed * METRES_TO_KM;
            if speed > driver.scalar(0xf142_abbf_beda_71e2)? {
                return Ok(true);
            }
        }
        Ok(self.local_player_grabbing)
    }
    /// The original accepts dT but does not use it for the acceleration ramp.
    /// Read attributes only on their original branches, preserving fault prefixes.
    pub(crate) fn update_acceleration<A: VehicleCharacteristics>(
        &mut self,
        _dt: f32,
        destination_distance: f32,
        minimum_distance: f32,
        attributes: &A,
    ) -> Result<(), A::Error> {
        if !self.skidding {
            if self.stopping_reason == StopReason::None {
                if self.acceleration < 0. {
                    self.acceleration = 0.;
                } else {
                    self.acceleration += attributes.acceleration_increment()?;
                }
                let mut maximum = self.default_acceleration;
                if self.rear_skater_skitch_potential {
                    let activation = attributes.scalar(SKITCH_ACTIVATION_SPEED)?;
                    if self.speed * METRES_TO_KM > activation
                        && self.rear_skater_speed * METRES_TO_KM > activation
                    {
                        maximum = 0.;
                        let delta = attributes.scalar(SKITCH_SKATER_SPEED_DELTA)? * KM_TO_METRES;
                        if self.skitch_helper_timer < 0.
                            && self.rear_skater_distance < attributes.scalar(SKITCH_RADIUS)?
                        {
                            let target = self.rear_skater_speed - delta;
                            let target = if target >= 0. { target } else { 0. };
                            self.acceleration = deceleration(
                                self.speed,
                                target,
                                attributes.scalar(SKITCH_DECELERATION_DISTANCE)?,
                            );
                        }
                        if self.rear_skater_tends_to_skitch
                            && self.rear_skater_distance < attributes.scalar(SKITCH_GRAB_RADIUS)?
                        {
                            let target = self.rear_skater_speed - delta;
                            let target = if target >= 0. { target } else { 0. };
                            self.acceleration = deceleration(
                                self.speed,
                                target,
                                attributes.scalar(SKITCH_GRAB_DECELERATION_DISTANCE)?,
                            );
                        }
                    }
                }
                if self.acceleration > maximum {
                    self.acceleration = maximum;
                }
            }
            // bgt skips this branch; retain its unordered behavior too.
            if minimum_distance < f32::MAX
                && !(destination_distance > self.speed + minimum_distance)
            {
                if destination_distance > minimum_distance {
                    self.acceleration = -((self.speed * self.speed)
                        / (destination_distance - minimum_distance).mul_add(2., DISTANCE_EPSILON));
                } else if self.stopping_reason == StopReason::Entity {
                    self.acceleration = -self.speed;
                    if self.acceleration < -attributes.scalar(SKID_START_DECELERATION)? {
                        self.skidding = true;
                    }
                } else {
                    self.acceleration = 0.;
                    self.speed = 0.;
                }
            }
            if self.force_stopping && self.acceleration > -self.speed {
                self.acceleration = -self.speed;
                if self.acceleration < -attributes.scalar(SKID_START_DECELERATION)? {
                    self.skidding = true;
                }
            }
        }
        if self.stopping_reason == StopReason::Parking {
            self.parking_started = true;
        }
        Ok(())
    }
    /// Returns false only for a missing host initialization proof; does not
    /// substitute an invented skitch delta. Original speed is NOT clamped to limit.
    pub(crate) fn update_speed(&mut self, dt: f32, limit: f32) -> bool {
        let skitch_delta = if self.being_skitched {
            let Some(delta) = self.skitching_speed_limit_delta else {
                return false;
            };
            delta
        } else {
            0.
        };
        self.current_speed_limit = ((self.speed_limit_delta + skitch_delta) + 1.) * limit;
        let increment = self.acceleration * dt;
        self.speed += increment; // separately rounded fmuls then fadds
        // Raw blt/bgt branches (82d187b4/7fc/820), not Hex-Rays' >=/<=.
        if !(self.acceleration < 0.) && self.speed > self.current_speed_limit {
            self.acceleration = 0.;
        }
        if !(self.acceleration > DISTANCE_EPSILON) && !(self.speed > f32::from_bits(0x3c23_d70a)) {
            self.acceleration = 0.;
            self.speed = 0.;
            self.skidding = false;
            self.force_stopping = false;
        }
        true
    }
}
fn deceleration(speed: f32, target: f32, distance: f32) -> f32 {
    target.mul_add(target, -(speed * speed)) / distance.mul_add(2., DISTANCE_EPSILON)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Attributes;
    impl VehicleCharacteristics for Attributes {
        type Error = u64;
        fn acceleration_increment(&self) -> Result<f32, u64> {
            Ok(0.25)
        }
        fn scalar(&self, key: u64) -> Result<f32, u64> {
            match key {
                SKID_START_DECELERATION => Ok(3.),
                _ => Err(key), // Unexpected branch reads must fail.
            }
        }
    }
    #[test]
    fn crossing_limit_stops_acceleration_without_teleporting_speed() {
        let mut motion = VehicleMotion::new();
        motion.speed = 9.5;
        motion.acceleration = 2.;
        assert!(motion.update_speed(0.5, 10.));
        assert_eq!(motion.speed, 10.5);
        assert_eq!(motion.acceleration, 0.);
    }
    #[test]
    fn forced_stop_skids_then_clears_both_flags_at_rest() {
        let mut motion = VehicleMotion::new();
        motion.speed = 5.;
        motion.default_acceleration = 1.;
        motion.force_stopping = true;
        motion
            .update_acceleration(1., 100., f32::MAX, &Attributes)
            .unwrap();
        assert_eq!(motion.acceleration, -5.);
        assert!(motion.skidding);
        assert!(motion.update_speed(1., 10.));
        assert_eq!(motion.speed, 0.);
        assert!(!motion.skidding);
        assert!(!motion.force_stopping);
    }
    #[test]
    fn acceleration_ramp_is_per_update_and_recovers_negative_first() {
        let mut motion = VehicleMotion::new();
        motion.default_acceleration = 2.;
        motion.acceleration = -1.;
        motion
            .update_acceleration(0.001, 100., f32::MAX, &Attributes)
            .unwrap();
        assert_eq!(motion.acceleration, 0.);
        motion
            .update_acceleration(0.001, 100., f32::MAX, &Attributes)
            .unwrap();
        assert_eq!(motion.acceleration, 0.25);
        motion
            .update_acceleration(10., 100., f32::MAX, &Attributes)
            .unwrap();
        assert_eq!(motion.acceleration, 0.5);
    }
}
