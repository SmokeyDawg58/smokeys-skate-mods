//! LWEntityDistanceStack.Set82e4ca18 and BasicEntity.UpdatePositionalData82e50040.
//! Shared core storage, not a second perception cache. VMX operands checked.
use crate::{
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate, try_length},
    simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
};

pub(crate) struct EntityDistanceStack {
    pub(crate) vector: [f32; 4],
    pub(crate) normal: [f32; 4],
    pub(crate) magnitude: f32,
    pub(crate) magnitude_delta: f32,
}
impl EntityDistanceStack {
    /// Not an inferred constructor default: MemStore.Init AND FreeBlock clear
    /// all bytes. RequestBlock does not change them before core construction.
    pub(crate) const fn from_cleared_memstore() -> Self {
        Self {
            vector: [0.; 4],
            normal: [0.; 4],
            magnitude: 0.,
            magnitude_delta: 0.,
        }
    }

    /// The source has exactly one stack slot (idx must be zero). Its delta is
    /// the length of (previous vector - new vector)*60, NOT signed radial
    /// speed and NOT divided by the vehicle execution group's timestep.
    fn set<N: ReckoningNumeric>(
        &mut self,
        magnitude: f32,
        distance: [f32; 4],
        normal: [f32; 4],
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        let change: [f32; 4] = std::array::from_fn(|i| (self.vector[i] - distance[i]) * 60.);
        self.magnitude = magnitude; // source store precedes delta length graph
        self.magnitude_delta = try_length(change, numeric)?;
        self.vector = distance;
        self.normal = normal;
        Ok(())
    }

    pub(crate) fn update<N: ReckoningNumeric>(
        &mut self,
        player_position: [f32; 4],
        entity_position: [f32; 4],
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        let distance = std::array::from_fn(|i| player_position[i] - entity_position[i]);
        let squared = numeric.dot3(distance, distance)?;
        // Two distinct estimate calls, both before their Newton refinements.
        let normal_seed = numeric.reciprocal_sqrt_estimate(squared)?;
        let length_seed = numeric.reciprocal_sqrt_estimate(squared)?;
        let inverse = refine_drive_reciprocal_sqrt(squared, normal_seed);
        let normal = distance.map(|lane| lane * inverse);
        let magnitude = length_from_squared_and_estimate(squared, length_seed);
        self.set(magnitude, distance, normal, numeric)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;

    // UNRUN. Tangential motion separates this from radial-distance deltas.
    #[test]
    fn delta_tracks_vector_change_at_source_sixty_hz() {
        let mut data = EntityDistanceStack::from_cleared_memstore();
        let mut numeric = HostReckoningNumeric;
        data.update([3., 0., 4., 8.], [0.; 4], &mut numeric)
            .unwrap();
        assert_eq!(data.magnitude, 5.);
        assert_eq!(data.magnitude_delta, 300.);
        assert_eq!(data.vector[3], 8.);
        assert!((data.normal[3] - 1.6).abs() < 0.000001);
        data.update([4., 0., 3., 7.], [0.; 4], &mut numeric)
            .unwrap();
        assert_eq!(data.magnitude, 5.);
        assert!((data.magnitude_delta - 7200_f32.sqrt()).abs() < 0.0001);
        data.update([4., 0., 3., 100.], [0.; 4], &mut numeric)
            .unwrap();
        assert_eq!(data.magnitude_delta.to_bits(), 0);
    }

    #[test]
    fn zero_distance_keeps_native_unsafe_normal_and_safe_length() {
        let mut data = EntityDistanceStack::from_cleared_memstore();
        data.update([0.; 4], [0.; 4], &mut HostReckoningNumeric)
            .unwrap();
        assert_eq!(data.magnitude.to_bits(), 0);
        assert_eq!(data.magnitude_delta.to_bits(), 0);
        assert!(data.normal.iter().all(|lane| lane.is_nan()));
    }
}
