//! Vehicle target geometry82d17e60/18010/1db38/26848. Borrows actual core
//! transform and model state through the entity frame; no presentation owner.
use super::{
    VehicleMotion,
    driving::{DrivingError, VehicleRoute},
    route_update::VehicleRouteControl,
    wheels::{self, WheelCurve},
};
use crate::{
    living_world_mesh_store::MeshStore,
    living_world_rain::{RainMan, geometry::Transform},
    reckoning_numeric::{
        ReckoningNumeric, cross_full4, try_length, try_look_at_up_eye, try_normalize_unsafe,
    },
};

pub(crate) trait VehicleTargetControl: VehicleRouteControl {
    fn target_transform(&self) -> Transform;
    /// Original SetTargetTransform82e4cbf8, including validity assertion.
    fn set_target_transform(
        &mut self,
        value: Transform,
    ) -> Result<
        (),
        crate::living_world_grind_services::entity_core::CoreError<std::convert::Infallible>,
    >;
    fn set_target_wheel_heading(&mut self, value: [f32; 4]);
    fn update_vision_area<N: ReckoningNumeric>(
        &mut self,
        motion: &VehicleMotion,
        registered_in_intersection: bool,
        numeric: &mut N,
    ) -> Result<
        (),
        super::vision::VisionError<crate::attribute_runtime::AttributeRuntimeError, N::Error>,
    >;
    fn update_nav_rig_validator<N: ReckoningNumeric>(
        &mut self,
        numeric: &mut N,
    ) -> Result<(), DrivingError<Self::Error, N::Error>>;
}

impl VehicleRoute {
    pub(crate) fn update_target_transform<C: VehicleTargetControl, N: ReckoningNumeric>(
        &mut self,
        motion: &VehicleMotion,
        colliding: bool,
        vehicle_length: f32,
        rain: &mut RainMan,
        mesh: &MeshStore,
        control: &mut C,
        numeric: &mut N,
    ) -> Result<(), DrivingError<C::Error, N::Error>> {
        let heading;
        if colliding {
            heading = control.target_transform()[2];
        } else {
            let wheel_info;
            match self.item_type {
                1 => {
                    let road = rain
                        .roads
                        .get(&self.item_id)
                        .ok_or(DrivingError::Road(self.item_id))?;
                    if self.lane_change_passage.valid {
                        // UpdateOnLaneChangeTargetTransform82d18010.
                        let curve = &self.lane_change_passage.curve;
                        let t = curve
                            .parameter(self.lane_change_position, numeric)
                            .map_err(DrivingError::Curve)?;
                        let position = curve.point(t);
                        let tangent = curve.tangent(t);
                        let normal = road.evaluate_normal(self.item_position)?;
                        let look = std::array::from_fn(|i| position[i] + tangent[i]);
                        let transform = try_look_at_up_eye(look, normal, position, numeric)
                            .map_err(DrivingError::Numeric)?;
                        control
                            .set_target_transform(transform)
                            .map_err(DrivingError::Core)?;
                        wheel_info = wheels::on_curve(
                            WheelCurve::Dynamic(curve),
                            self.lane_change_position,
                            vehicle_length,
                            numeric,
                        )
                        .map_err(DrivingError::Wheel)?;
                    } else {
                        // UpdateOnFollowLaneTargetTransform82d17e60.
                        let super_id = road.record.id(0x98)?;
                        let super_road = rain
                            .super_roads
                            .get(&super_id)
                            .ok_or(DrivingError::SuperRoad(super_id))?;
                        let previous_position = control.target_transform()[3];
                        let transform = road
                            .evaluate_transform(
                                self.item_position,
                                self.road_lane as i32,
                                self.offset_from_lane,
                                numeric,
                            )
                            .map_err(DrivingError::Geometry)?;
                        control
                            .set_target_transform(transform)
                            .map_err(DrivingError::Core)?;
                        wheel_info = wheels::on_super_road(
                            super_road,
                            control.target_transform(),
                            self.super_road_position,
                            self.road_lane,
                            self.offset_from_lane,
                            vehicle_length,
                            rain,
                            numeric,
                        )
                        .map_err(DrivingError::Wheel)?;
                        let position = control.target_transform()[3];
                        let displacement: [f32; 4] =
                            std::array::from_fn(|i| position[i] - previous_position[i]);
                        // Raw fadds82d17fdc adds the distance to prior mileage;
                        // Hex-Rays loses this VMX-written stack operand.
                        self.action_decision_mileage +=
                            try_length(displacement, numeric).map_err(DrivingError::Numeric)?;
                    }
                }
                0 => {
                    // UpdateOnIntersectionTargetTransform82d1db38.
                    let intersection = rain
                        .intersection(self.item_id, mesh)
                        .map_err(DrivingError::Rain)?
                        .ok_or(DrivingError::Intersection(self.item_id))?;
                    let passage = intersection
                        .passages
                        .get(self.passage_index as usize)
                        .ok_or(DrivingError::Passage(self.passage_index))?;
                    let curve = &passage.curve;
                    let t = curve
                        .parameter(self.item_position, numeric)
                        .map_err(DrivingError::Curve)?;
                    let position = curve.point(t);
                    let tangent = curve.tangent(t);
                    // Inner quadrilateral diagonals (vertex3-1) cross (2-0),
                    // checked raw VMX82d1dc18..4c; no assumed world-up vector.
                    let v0 = intersection.record.vector(0)?;
                    let v1 = intersection.record.vector(0x10)?;
                    let v2 = intersection.record.vector(0x20)?;
                    let v3 = intersection.record.vector(0x30)?;
                    let normal = cross_full4(
                        std::array::from_fn(|i| v3[i] - v1[i]),
                        std::array::from_fn(|i| v2[i] - v0[i]),
                    );
                    let look = std::array::from_fn(|i| position[i] + tangent[i]);
                    let transform = try_look_at_up_eye(look, normal, position, numeric)
                        .map_err(DrivingError::Numeric)?;
                    control
                        .set_target_transform(transform)
                        .map_err(DrivingError::Core)?;
                    wheel_info = wheels::on_curve(
                        WheelCurve::Authored(curve),
                        self.item_position,
                        vehicle_length,
                        numeric,
                    )
                    .map_err(DrivingError::Wheel)?;
                }
                _ => {
                    return control
                        .request_explicit_cull()
                        .map_err(DrivingError::Vehicle);
                }
            }
            let (transform, wheel_heading) =
                wheels::adjusted(&wheel_info, control.target_transform(), numeric)
                    .map_err(DrivingError::Numeric)?;
            control
                .set_target_transform(transform)
                .map_err(DrivingError::Core)?;
            heading = wheel_heading;
        }
        control.set_target_wheel_heading(
            try_normalize_unsafe(heading, numeric).map_err(DrivingError::Numeric)?,
        );
        control
            .update_vision_area(motion, self.registered_in_intersection, numeric)
            .map_err(DrivingError::Vision)?;
        control.update_nav_rig_validator(numeric)?;
        Ok(())
    }
}
