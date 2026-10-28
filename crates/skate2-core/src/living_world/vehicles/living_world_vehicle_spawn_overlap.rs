//! Spawn82d281bc..283d8 DMO clearance, AFTER road target placement and BEFORE
//! UpdateAttributes/Island admission. Uses existing DMO KD tree/model center.
use super::Vehicle;
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    living_world_grind_services::{
        dmo_locators::LocatorError,
        dmo_objects::{DmoFrameServiceError, DmoObjectMan},
    },
    reckoning_numeric::host_dot3,
};

#[derive(Debug)]
pub(crate) enum SpawnOverlapError {
    Cvar(InputCvarError),
    Query(LocatorError<DmoFrameServiceError>),
}

/// Raw fsel(a-b,a,b), including unordered and signed-zero selection. Native
/// vehicle radius uses max(X,Z), while DMO radius uses max(max(X,Z),Y).
fn extent(a: f32, b: f32) -> f32 {
    if a - b >= 0. { a } else { b }
}

/// Math.CompareDistanceXZ82707440: separately rounded radius square, XYZ dot
/// with Y=0, then subtraction. Shared host VMX-reduction policy remains in use.
fn distance_minus_radius_squared(a: [f32; 4], b: [f32; 4], radius: f32) -> f32 {
    let delta = [a[0] - b[0], 0., a[2] - b[2], 0.];
    let squared = radius * radius;
    host_dot3(delta, delta) - squared
}

impl Vehicle {
    /// false means Spawn must DestroyEntity (including RAIN departure) and
    /// return null. No active-roster insertion or collision body is fabricated.
    pub(crate) fn clear_of_spawn_dmos(
        &self,
        force_spawn: bool,
        dmos: &DmoObjectMan,
        cvars: &InputCvarReferences,
    ) -> Result<bool, SpawnOverlapError> {
        // PDB LWCommonSpawnInfo.mbForceSpawn+62; unrelated to the Juice
        // model-name override that selects lane0 in the earlier road check.
        if force_spawn {
            return Ok(true);
        }
        let position = self.core.target_transform[3];
        let half = self.dimensions().map(|value| value * 0.5);
        let vehicle_radius = extent(half[0], half[2]);
        let radius = cvars
            .try_vehicle_spawn_dmo_check_radius()
            .map_err(SpawnOverlapError::Cvar)?;
        // Source asks for64; SAME GetDynamicObjectsWithinRange clamps to50.
        let objects = dmos
            .try_objects_within_range([position[0], position[2]], radius, 64)
            .map_err(SpawnOverlapError::Query)?;
        for (id, _) in objects {
            let Some(object) = dmos.object(id) else {
                continue;
            };
            let half = object.model.dimensions.map(|value| value * 0.5);
            let dmo_radius = extent(extent(half[0], half[2]), half[1]);
            let combined_radius = dmo_radius + vehicle_radius;
            // IDynamicObject+44 -> GetCenterPosition82d361c8 -> SAME
            // model.GetBBoxTransform82e4cd60 of the current core transform.
            let centered = object.model.bbox_transform(&object.transform);
            let center = std::array::from_fn(|i| centered[12 + i]);
            // Raw ble82d28388 is !GT, also rejecting unordered distances.
            if !(distance_minus_radius_squared(position, center, combined_radius) > 0.) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_clearance_includes_boundary_ignores_height_and_keeps_unordered_branch() {
        let position = [0., 900., 0., 0.];
        assert_eq!(
            distance_minus_radius_squared(position, [3., -900., 4., 0.], 5.),
            0.
        );
        assert!(distance_minus_radius_squared(position, [3., 0., 4., 0.], 4.) > 0.);
        assert!(!(distance_minus_radius_squared(position, [f32::NAN, 0., 4., 0.], 5.) > 0.));
        assert_eq!(extent(-0., 0.).to_bits(), (-0.0_f32).to_bits());
        assert_eq!(extent(f32::NAN, 2.), 2.);
    }
}
