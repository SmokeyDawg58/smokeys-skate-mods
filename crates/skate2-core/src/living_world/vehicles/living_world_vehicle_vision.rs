//! Vehicle::UpdateVisionArea82d1deb0. Source front obstacle and rear skitch
//! quadrilaterals; authored characteristics and actual collision-model dimensions.
use super::{VehicleCharacteristics, VehicleMotion};
#[path = "living_world_vehicle_vision_geometry.rs"]
pub(crate) mod geometry;
use crate::{
    living_world_rain::geometry::Transform,
    reckoning_numeric::{ReckoningNumeric, try_normalize_unsafe},
    simulation_drive_jacobian::refine_drive_reciprocal,
};

#[derive(Debug)]
pub(crate) enum VisionError<A, N> {
    Attribute(A),
    Numeric(N),
    InvalidSpeedLimit,
}
pub(crate) struct VehicleVision {
    // Constructor does not initialize these members. No collision model means
    // UpdateVisionArea returns without changing any of them.
    pub(crate) area: Option<[[f32; 4]; 4]>,
    pub(crate) skitch_helper: Option<[[f32; 4]; 4]>,
    pub(crate) extension_max_scale: Option<f32>,
    pub(crate) standard_scale: Option<f32>,
}
impl VehicleVision {
    pub(crate) fn new() -> Self {
        Self {
            area: None,
            skitch_helper: None,
            extension_max_scale: None,
            standard_scale: None,
        }
    }
    pub(crate) fn update<A: VehicleCharacteristics, N: ReckoningNumeric>(
        &mut self,
        motion: &VehicleMotion,
        transform: Transform,
        collision_model_dimensions: Option<[f32; 4]>,
        registered_in_intersection: bool,
        wheel_turn_angle: f32,
        attributes: &A,
        numeric: &mut N,
    ) -> Result<(), VisionError<A::Error, N::Error>> {
        let Some(dimensions) = collision_model_dimensions else {
            return Ok(());
        };
        let max_scale = attributes
            .scalar(0x02a6_8a53_be0c_9ef6)
            .map_err(VisionError::Attribute)?;
        self.extension_max_scale = Some(max_scale);
        let standard = attributes
            .scalar(0xe88c_9a50_820c_c422)
            .map_err(VisionError::Attribute)?;
        self.standard_scale = Some(standard);
        let half: [f32; 4] = dimensions.map(|v| v * 0.5);
        let forward: [f32; 4] = transform[2].map(|v| v * half[2]);
        let side: [f32; 4] = transform[0].map(|v| v * half[0]);
        if motion.current_speed_limit.is_nan() {
            return Err(VisionError::InvalidSpeedLimit);
        }
        let speed_fraction = if motion.current_speed_limit > 0. {
            let inverse = refine_drive_reciprocal(
                motion.current_speed_limit,
                numeric
                    .reciprocal_estimate(motion.current_speed_limit)
                    .map_err(VisionError::Numeric)?,
            );
            inverse * motion.speed
        } else {
            0.
        };
        let skitch_radius = attributes
            .scalar(0x3346_6832_d817_8eaf)
            .map_err(VisionError::Attribute)?;
        let center = transform[3];
        let front = add(center, forward);
        let rear = sub(center, forward);
        let standard_side = side.map(|v| v * (1. + standard));
        let extended_side = side.map(|v| v * speed_fraction.mul_add(max_scale, 1.));
        let mut extended_corner = -1;
        if registered_in_intersection && wheel_turn_angle.abs() > f32::from_bits(0x3ccc_cccd) {
            extended_corner = if wheel_turn_angle > 0. { 3 } else { 2 };
        }
        let side2 = if extended_corner == 2 {
            extended_side
        } else {
            standard_side
        };
        let side3 = if extended_corner == 3 {
            extended_side
        } else {
            standard_side
        };
        let mut area = if motion.backing_up {
            [
                add(center, standard_side),
                sub(center, standard_side),
                add(rear, side2),
                sub(rear, side3),
            ]
        } else {
            [
                sub(center, standard_side),
                add(center, standard_side),
                sub(front, side2),
                add(front, side3),
            ]
        };
        let mut skitch = [
            add(front, standard_side),
            sub(front, standard_side),
            add(rear, extended_side),
            sub(rear, extended_side),
        ];
        // Normalize in 3D before extension; flatten Y only after ALL four
        // extensions, as raw scalar stores82d1e434..4c0 require. Preserve W.
        extend(
            &mut area,
            2,
            0,
            motion.speed + motion.minimum_entity_distance,
            numeric,
        )
        .map_err(VisionError::Numeric)?;
        extend(
            &mut area,
            3,
            1,
            motion.speed + motion.minimum_entity_distance,
            numeric,
        )
        .map_err(VisionError::Numeric)?;
        extend(&mut skitch, 2, 0, skitch_radius, numeric).map_err(VisionError::Numeric)?;
        extend(&mut skitch, 3, 1, skitch_radius, numeric).map_err(VisionError::Numeric)?;
        for i in 0..4 {
            area[i][1] = 0.;
            skitch[i][1] = 0.;
        }
        self.area = Some(area);
        self.skitch_helper = Some(skitch);
        Ok(())
    }
}
fn add(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn extend<N: ReckoningNumeric>(
    points: &mut [[f32; 4]; 4],
    end: usize,
    start: usize,
    distance: f32,
    numeric: &mut N,
) -> Result<(), N::Error> {
    let direction = try_normalize_unsafe(sub(points[end], points[start]), numeric)?;
    points[end] = std::array::from_fn(|i| direction[i].mul_add(distance, points[end][i]));
    Ok(())
}
