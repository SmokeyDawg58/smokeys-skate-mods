//! Concrete bridge from Vehicle's retained fields to the source RAIN/target
//! operations. No copied motion/route snapshot, private census, or second pose.
use super::Vehicle;
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    living_world_grind_services::{
        entity_core::{CommonEntityCore, CoreError},
        vehicle_motion::{
            VehicleMotion,
            driving::{DrivingError, DrivingVehicles},
            nav_rig::VehicleNavRigValidator,
            route_update::VehicleRouteControl,
            target_transform::VehicleTargetControl,
            vision::{VehicleVision, VisionError},
            wheels::VehicleWheelState,
        },
    },
    living_world_mesh_store::MeshStore,
    living_world_rain::{RainMan, geometry::Transform},
    reckoning_numeric::ReckoningNumeric,
};
use std::convert::Infallible;

/// Motion and route are borrowed by the invoking method, not retained here.
/// The target method supplies their current values AFTER its preceding writes.
struct TargetFrame<'a, F> {
    id: u32,
    core: &'a mut CommonEntityCore,
    wheels: &'a mut VehicleWheelState,
    vision: &'a mut VehicleVision,
    nav_rig: &'a mut Option<VehicleNavRigValidator>,
    characteristics: &'a AttributeRuntimeInstance,
    collision_dimensions: Option<[f32; 4]>,
    request_cull: &'a mut F,
}

impl<E, F: FnMut(u32) -> Result<(), E>> VehicleRouteControl for TargetFrame<'_, F> {
    type Error = E;
    fn request_explicit_update(&mut self) {
        self.core.explicit_update_requested = true;
    }
    fn request_explicit_cull(&mut self) -> Result<(), E> {
        // RequestExplicitCull82d25920 sets AllowCulling before Census.RequestCull.
        self.core.allow_culling = true;
        (self.request_cull)(self.id)
    }
}
impl<E, F: FnMut(u32) -> Result<(), E>> VehicleTargetControl for TargetFrame<'_, F> {
    fn target_transform(&self) -> Transform {
        self.core.target_transform
    }
    fn set_target_transform(&mut self, value: Transform) -> Result<(), CoreError<Infallible>> {
        self.core.set_target_transform(value)
    }
    fn set_target_wheel_heading(&mut self, value: [f32; 4]) {
        self.wheels.target_heading = value;
    }
    fn update_vision_area<N: ReckoningNumeric>(
        &mut self,
        motion: &VehicleMotion,
        registered_in_intersection: bool,
        numeric: &mut N,
    ) -> Result<(), VisionError<AttributeRuntimeError, N::Error>> {
        // UpdateTargetTransform82d26b24 calls this after target-wheel heading.
        self.vision.update(
            motion,
            self.core.target_transform,
            self.collision_dimensions,
            registered_in_intersection,
            self.wheels.turn_angle,
            self.characteristics,
            numeric,
        )
    }
    fn update_nav_rig_validator<N: ReckoningNumeric>(
        &mut self,
        numeric: &mut N,
    ) -> Result<(), DrivingError<E, N::Error>> {
        // 82d20f18 dereferences the collision model unconditionally. Source
        // vision's null-model early return does NOT skip this later call.
        let dimensions = self
            .collision_dimensions
            .ok_or(DrivingError::MissingCollisionModel)?;
        *self.nav_rig = Some(
            VehicleNavRigValidator::from_vehicle(
                self.core.target_transform,
                dimensions,
                self.core.velocity,
                numeric,
            )
            .map_err(DrivingError::Numeric)?,
        );
        Ok(())
    }
}

impl Vehicle {
    /// VehicleMan.UpdateTargetRAINInfo82d27840 invokes this for every vehicle,
    /// independently of the less frequent behaviour execution groups.
    pub(crate) fn update_target_rain_info<V: DrivingVehicles, N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        rain: &mut RainMan,
        mesh: &MeshStore,
        vehicles: &mut V,
        request_cull: &mut impl FnMut(u32) -> Result<(), V::Error>,
        numeric: &mut N,
    ) -> Result<(), DrivingError<V::Error, N::Error>> {
        let length = self.length();
        let collision = self
            .collision
            .as_ref()
            .ok_or(DrivingError::MissingCollisionModel)?;
        let colliding = collision.colliding;
        let mut control = TargetFrame {
            id: self.id,
            core: &mut self.core,
            wheels: &mut self.wheels,
            vision: &mut self.vision,
            nav_rig: &mut self.nav_rig,
            characteristics: &self.characteristics,
            collision_dimensions: collision.model_dimensions(),
            request_cull,
        };
        self.route.update_target_rain_info(
            &self.motion,
            self.id,
            length,
            dt,
            colliding,
            rain,
            mesh,
            vehicles,
            &mut control,
            numeric,
        )
    }
    /// AddToRoadNetwork82d17428: initial position/limit, lane membership,
    /// target/wheel/vision/nav update, then registered flag. Callback must be
    /// the caller's real Census.RequestCull; this bridge owns no cull queue.
    pub(crate) fn add_to_road_network<V, N>(
        &mut self,
        super_position: f32,
        road_id: u64,
        lane: u32,
        rain: &mut RainMan,
        mesh: &MeshStore,
        vehicles: &mut V,
        request_cull: &mut impl FnMut(u32) -> Result<(), V::Error>,
        numeric: &mut N,
    ) -> Result<(), DrivingError<V::Error, N::Error>>
    where
        V: DrivingVehicles,
        N: ReckoningNumeric,
    {
        let length = self.length();
        let collision = self
            .collision
            .as_ref()
            .ok_or(DrivingError::MissingCollisionModel)?;
        let colliding = collision.colliding;
        let mut control = TargetFrame {
            id: self.id,
            core: &mut self.core,
            wheels: &mut self.wheels,
            vision: &mut self.vision,
            nav_rig: &mut self.nav_rig,
            characteristics: &self.characteristics,
            collision_dimensions: collision.model_dimensions(),
            request_cull,
        };
        self.route.add_to_road_network(
            &mut self.motion,
            self.id,
            length,
            super_position,
            road_id,
            lane,
            colliding,
            rain,
            mesh,
            vehicles,
            &mut control,
            numeric,
        )
    }

    /// Same target update used after the route's per-frame advancement. The
    /// source dT parameter is unused; no integration is added by this bridge.
    pub(crate) fn update_target_transform<E, N: ReckoningNumeric>(
        &mut self,
        rain: &mut RainMan,
        mesh: &MeshStore,
        request_cull: &mut impl FnMut(u32) -> Result<(), E>,
        numeric: &mut N,
    ) -> Result<(), DrivingError<E, N::Error>> {
        let length = self.length();
        let collision = self
            .collision
            .as_ref()
            .ok_or(DrivingError::MissingCollisionModel)?;
        let colliding = collision.colliding;
        let mut control = TargetFrame {
            id: self.id,
            core: &mut self.core,
            wheels: &mut self.wheels,
            vision: &mut self.vision,
            nav_rig: &mut self.nav_rig,
            characteristics: &self.characteristics,
            collision_dimensions: collision.model_dimensions(),
            request_cull,
        };
        self.route.update_target_transform(
            &self.motion,
            colliding,
            length,
            rain,
            mesh,
            &mut control,
            numeric,
        )
    }
}
