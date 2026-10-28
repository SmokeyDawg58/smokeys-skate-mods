//! IVehicle::UpdateVehicleNavRigValidator82d20f18. These are the vehicle's
//! four line/sphere validation members used by navigation, not physics bodies.
use crate::{
    living_world_rain::geometry::Transform,
    reckoning_numeric::{ReckoningNumeric, try_length, try_normalize_unsafe},
};
pub(crate) struct VehicleNavRigValidator {
    pub(crate) line_starts: [[f32; 4]; 4],
    pub(crate) line_headings: [[f32; 4]; 4],
    pub(crate) line_magnitudes: [f32; 4],
    pub(crate) sphere_radii: [f32; 4],
    pub(crate) sphere_positions: [[f32; 4]; 4],
}
impl VehicleNavRigValidator {
    pub(crate) fn from_vehicle<N: ReckoningNumeric>(
        transform: Transform,
        collision_model_dimensions: [f32; 4],
        entity_velocity: [f32; 4],
        numeric: &mut N,
    ) -> Result<Self, N::Error> {
        let half = collision_model_dimensions.map(|v| v * 0.5);
        let forward: [f32; 4] = transform[2].map(|v| v * half[2]);
        let side: [f32; 4] = transform[0].map(|v| v * half[0]);
        let forward = forward.map(|v| v * 1.5);
        let side = side.map(|v| v * 1.5);
        let front = add(transform[3], forward);
        let rear = sub(transform[3], forward);
        let velocity_extension = entity_velocity.map(|v| v * 3.);
        let starts = [
            add(add(front, side), velocity_extension),
            add(sub(front, side), velocity_extension),
            sub(rear, side),
            add(rear, side),
        ];
        let edges: [[f32; 4]; 4] = std::array::from_fn(|i| sub(starts[(i + 1) % 4], starts[i]));
        let mut headings = [[0.; 4]; 4];
        let mut magnitudes = [0.; 4];
        for i in 0..4 {
            // Scalar reconstructed heading vectors have Y=0 AND W=0;
            // magnitudes use the original full 3D edge (raw VMX82d212fc).
            headings[i] = try_normalize_unsafe([edges[i][0], 0., edges[i][2], 0.], numeric)?;
            magnitudes[i] = try_length(edges[i], numeric)?;
        }
        let near = f32::from_bits(0x3e4c_cccd); // .2
        let far = f32::from_bits(0x3f33_3333); // .7
        let z: [f32; 4] = transform[2].map(|v| v * half[2]);
        // Front uses vmadd (fused); rear uses separate multiply/subtract.
        let positions = [
            std::array::from_fn(|i| z[i].mul_add(far, transform[3][i])),
            std::array::from_fn(|i| z[i].mul_add(near, transform[3][i])),
            std::array::from_fn(|i| transform[3][i] - z[i] * near),
            std::array::from_fn(|i| transform[3][i] - z[i] * far),
        ];
        let small = half[0] + f32::from_bits(0x3da3_d70a); // .08
        let large = half[0] + f32::from_bits(0x3ee6_6666); // .45
        Ok(Self {
            line_starts: starts,
            line_headings: headings,
            line_magnitudes: magnitudes,
            sphere_radii: [small, large, large, small],
            sphere_positions: positions,
        })
    }
}
fn add(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| a[i] - b[i])
}
