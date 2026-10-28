//! `ObstacleAvoider::AddPedestrianObstacles` 0x824c4ab8, `AddVehicleObstacles` 0x824c4f00 and
//! `AddDynamicObstacles` 0x824c52a0: the Living World entities near the local skater as the
//! avoider's proxies, each kind in its manager's range-query order. A manager the session has
//! not published yet (the front end runs none) has no entities.
use super::{
    obstacle_avoider::{EntityObstacle, ObstacleType, ProxyObstacle},
    vmx::{self, V4},
};
use crate::living_world_grind_services::{
    LivingWorldGrindServices, PedestrianNearQueryError, VehicleNearQueryError,
    dmo_locators::{DynamicObjectLocator, LOCATOR_STATIC},
};

/// `AI_Skater_Avoidance` cvars at their defaults (`startup.ini` sets none).
mod cvars {
    /// `kPedestrianHeightInMeters`.
    pub(super) const PEDESTRIAN_HEIGHT_IN_METERS: f32 = 1.6;
    /// `kAmbientSkatersIgnoreDMOsAtDefaultLocation`.
    pub(super) const AMBIENT_SKATERS_IGNORE_DMOS_AT_DEFAULT_LOCATION: bool = false;
    /// `kChallengeSkatersIgnoreDMOsAtDefaultLocation`.
    pub(super) const CHALLENGE_SKATERS_IGNORE_DMOS_AT_DEFAULT_LOCATION: bool = true;
    /// `kbMakeAllDMOsRunOverable` and `kbMakeAllDMOsJumpOverable`.
    pub(super) const MAKE_ALL_DMOS_RUN_OVERABLE: bool = false;
    pub(super) const MAKE_ALL_DMOS_JUMP_OVERABLE: bool = false;
    /// `kMaxPosDiffForDefaultDMO_XZ`, `kMaxPosDiffForDefaultDMO_Y` and `kMaxDirDiffForDefaultDMO`.
    pub(super) const MAX_POS_DIFF_FOR_DEFAULT_DMO_XZ: f32 = 0.15;
    pub(super) const MAX_POS_DIFF_FOR_DEFAULT_DMO_Y: f32 = 0.15;
    pub(super) const MAX_DIR_DIFF_FOR_DEFAULT_DMO: f32 = 0.05;
}

/// Each adder's request to its manager's range query.
const MAX_ENTITIES: u32 = 20;
/// `livingworld_dynamicobject_characteristics` `CanAISkatersRunOver` and `CanAISkaterJumpOver`.
const CAN_AI_SKATERS_RUN_OVER: usize = 0x7d;
const CAN_AI_SKATER_JUMP_OVER: usize = 0x7e;

/// What the adders read of the local skater.
pub(crate) struct ObstacleQuery {
    /// `Actor::GetPosition`.
    pub(crate) position: V4,
    /// `ai_skater` 0x7c1d006c, the gathering radius.
    pub(crate) radius: f32,
    /// `AddDynamicObstacles`' client tests: an ambient skater (client type 0), and a challenge
    /// one (type 1, or a later type during an AI challenge).
    pub(crate) ambient: bool,
    pub(crate) challenge: bool,
    /// `physics_skeleton` `SmallObjectMassThreshold`.
    pub(crate) small_object_mass_threshold: f32,
}

/// The three adders in `ObstacleAvoider::Update`'s order.
pub(crate) fn try_gather(living: &LivingWorldGrindServices, query: &ObstacleQuery) -> Result<Vec<EntityObstacle>, String> {
    let mut obstacles = Vec::new();
    add_pedestrian_obstacles(living, query, &mut obstacles)?;
    add_vehicle_obstacles(living, query, &mut obstacles)?;
    add_dynamic_obstacles(living, query, &mut obstacles)?;
    Ok(obstacles)
}

/// Half the box diagonal, the proxy's radius.
fn half_diagonal(min: V4, max: V4) -> f32 {
    vmx::length3(vmx::sub(max, min)) * 0.5
}

/// `AddPedestrianObstacles`: the pedestrians with an initialized core within the gathering
/// radius, each a box on its model's width and length, `kPedestrianHeightInMeters` tall.
fn add_pedestrian_obstacles(
    living: &LivingWorldGrindServices,
    query: &ObstacleQuery,
    obstacles: &mut Vec<EntityObstacle>,
) -> Result<(), String> {
    let (mut count, mut ids, mut distances) = (0, [0; MAX_ENTITIES as usize], [0.0; MAX_ENTITIES as usize]);
    let point = [query.position[0], query.position[2]];
    match living.try_get_pedestrians_within_range(point, query.radius, MAX_ENTITIES, &mut count, &mut ids, &mut distances) {
        Ok(()) => {}
        Err(PedestrianNearQueryError::UnpublishedManager | PedestrianNearQueryError::MissingEntityMan) => return Ok(()),
        Err(error) => return Err(format!("AI pedestrian obstacles: {error:?}")),
    }
    let Some(roster) = living.pedestrian_entities.as_deref() else { return Ok(()) };
    for (&id, &distance) in ids.iter().zip(&distances).take(count as usize) {
        let Some(pedestrian) = roster.get(id) else { continue };
        let Some(core) = pedestrian.core.as_ref() else { continue };
        let dimensions = pedestrian.model.as_ref().map_or([0.0; 4], |model| model.dimensions);
        let (half_width, half_length) = (dimensions[0] * 0.5, dimensions[2] * 0.5);
        let aabb_min = [-half_width, 0.0, -half_length, 0.0];
        let aabb_max = [half_width, cvars::PEDESTRIAN_HEIGHT_IN_METERS, half_length, 0.0];
        obstacles.push(EntityObstacle {
            proxy: ProxyObstacle {
                position: core.transform[3],
                velocity: core.velocity,
                heading: core.target_transform[2],
                aabb_min,
                aabb_max,
                transform: core.transform,
                width: dimensions[0],
                length: dimensions[2],
                radius: half_diagonal(aabb_min, aabb_max),
                obstacle_type: ObstacleType::Pedestrian,
                can_run_over: false,
                can_jump_over: false,
            },
            distance,
        });
    }
    Ok(())
}

/// `AddVehicleObstacles`: the vehicles within two and a half gathering radii, each its
/// collision model's box. A vehicle whose collision is not made yet is skipped.
fn add_vehicle_obstacles(
    living: &LivingWorldGrindServices,
    query: &ObstacleQuery,
    obstacles: &mut Vec<EntityObstacle>,
) -> Result<(), String> {
    let (mut count, mut ids, mut distances) = (0, [0; MAX_ENTITIES as usize], [0.0; MAX_ENTITIES as usize]);
    let point = [query.position[0], query.position[2]];
    match living.try_get_vehicles_within_range(point, query.radius * 2.5, MAX_ENTITIES, &mut count, &mut ids, &mut distances) {
        Ok(()) => {}
        Err(VehicleNearQueryError::MissingEntityMan) => return Ok(()),
        Err(error) => return Err(format!("AI vehicle obstacles: {error:?}")),
    }
    let Some(vehicles) = living.grind_vehicle_entities.as_deref() else { return Ok(()) };
    for (&id, &distance) in ids.iter().zip(&distances).take(count as usize) {
        let Some(vehicle) = vehicles.get(id) else { continue };
        let Some((aabb_min, aabb_max)) = vehicle.collision.as_ref().and_then(|collision| collision.model_bounds()) else {
            continue;
        };
        let dimensions = vehicle.dimensions();
        obstacles.push(EntityObstacle {
            proxy: ProxyObstacle {
                position: vehicle.core.transform[3],
                velocity: vehicle.core.velocity,
                heading: vehicle.core.target_transform[2],
                aabb_min,
                aabb_max,
                transform: vehicle.core.transform,
                width: dimensions[0],
                length: dimensions[2],
                radius: half_diagonal(aabb_min, aabb_max),
                obstacle_type: ObstacleType::Vehicle,
                can_run_over: false,
                can_jump_over: false,
            },
            distance,
        });
    }
    Ok(())
}

/// `AddDynamicObstacles`: the dynamic objects within two gathering radii that weigh at least
/// `SmallObjectMassThreshold`, each its model's box placed at its centre and measured from
/// there. Ambient or challenge skaters (per the cvars) pass objects still at their locators.
fn add_dynamic_obstacles(
    living: &LivingWorldGrindServices,
    query: &ObstacleQuery,
    obstacles: &mut Vec<EntityObstacle>,
) -> Result<(), String> {
    let ignore_default = (cvars::AMBIENT_SKATERS_IGNORE_DMOS_AT_DEFAULT_LOCATION && query.ambient)
        || (cvars::CHALLENGE_SKATERS_IGNORE_DMOS_AT_DEFAULT_LOCATION && query.challenge);
    let Some(objects) = living.dmo_objects.as_deref() else { return Ok(()) };
    let point = [query.position[0], query.position[2]];
    let found = objects
        .try_objects_within_range(point, query.radius * 2.0, MAX_ENTITIES)
        .map_err(|error| format!("AI dynamic obstacles: {error:?}"))?;
    for (id, _) in found {
        let Some(object) = objects.object(id) else { continue };
        let model = &object.model;
        if model.mass < query.small_object_mass_threshold {
            continue;
        }
        if ignore_default
            && let Some(locator) = living.dmo_locators.as_deref().and_then(|locators| locators.locator(object.locator_id))
            && is_object_in_default_location(locator)
        {
            continue;
        }
        let (aabb_min, aabb_max) = (model.bbox_min, model.bbox_max);
        let local: V4 = std::array::from_fn(|lane| (aabb_min[lane] + aabb_max[lane]) * 0.5);
        let rows: [V4; 4] = std::array::from_fn(|row| std::array::from_fn(|lane| object.transform[row * 4 + lane]));
        let centre: V4 = std::array::from_fn(|lane| {
            rows[2][lane].mul_add(local[2], rows[1][lane].mul_add(local[1], rows[0][lane].mul_add(local[0], rows[3][lane])))
        });
        let (can_run_over, can_jump_over) = if cvars::MAKE_ALL_DMOS_RUN_OVERABLE || cvars::MAKE_ALL_DMOS_JUMP_OVERABLE {
            (cvars::MAKE_ALL_DMOS_RUN_OVERABLE, cvars::MAKE_ALL_DMOS_JUMP_OVERABLE)
        } else {
            let flag = |offset| -> Result<bool, String> {
                Ok(model
                    .characteristics
                    .try_cached_layout_field(offset, 1)
                    .map_err(|error| format!("AI dynamic obstacle characteristics: {error:?}"))?
                    .is_some_and(|data| data.bytes()[0] != 0))
            };
            (flag(CAN_AI_SKATERS_RUN_OVER)?, flag(CAN_AI_SKATER_JUMP_OVER)?)
        };
        obstacles.push(EntityObstacle {
            proxy: ProxyObstacle {
                position: centre,
                velocity: object.velocity,
                heading: std::array::from_fn(|lane| object.target_transform[8 + lane]),
                aabb_min,
                aabb_max,
                transform: rows,
                width: model.dimensions[0],
                length: model.dimensions[2],
                radius: half_diagonal(aabb_min, aabb_max),
                obstacle_type: ObstacleType::Dmo,
                can_run_over,
                can_jump_over,
            },
            distance: vmx::length3(vmx::sub(centre, query.position)),
        });
    }
    Ok(())
}

/// `IsObjectInDefaultLocation` 0x824bc8a8: a static locator always; otherwise its transform
/// within `kMaxPosDiffForDefaultDMO_Y` up and `kMaxPosDiffForDefaultDMO_XZ` across of its initial
/// stream transform, each axis within `kMaxDirDiffForDefaultDMO`.
fn is_object_in_default_location(locator: &DynamicObjectLocator) -> bool {
    if locator.flags & LOCATOR_STATIC != 0 {
        return true;
    }
    let delta = |row: usize| -> V4 {
        std::array::from_fn(|lane| locator.transform[row * 4 + lane] - locator.initial_stream_transform[row * 4 + lane])
    };
    let moved = delta(3);
    if !(moved[1].abs() <= cvars::MAX_POS_DIFF_FOR_DEFAULT_DMO_Y) {
        return false;
    }
    let across = moved[2].mul_add(moved[2], moved[0] * moved[0]);
    if across > cvars::MAX_POS_DIFF_FOR_DEFAULT_DMO_XZ * cvars::MAX_POS_DIFF_FOR_DEFAULT_DMO_XZ {
        return false;
    }
    let turned = cvars::MAX_DIR_DIFF_FOR_DEFAULT_DMO * cvars::MAX_DIR_DIFF_FOR_DEFAULT_DMO;
    (0..3).all(|row| {
        let axis = delta(row);
        !(vmx::dot3(axis, axis) > turned)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locator(transform: [f32; 16], initial: [f32; 16], flags: u8) -> DynamicObjectLocator {
        DynamicObjectLocator {
            transform,
            initial_stream_transform: initial,
            locator_id: 1,
            model_id: 2,
            attachment_time: 0,
            user_object_id: 0,
            flags,
        }
    }

    const IDENTITY: [f32; 16] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 5.0, 2.0, -3.0, 1.0];

    #[test]
    fn a_static_locator_is_always_default() {
        let mut moved = IDENTITY;
        moved[12] += 10.0;
        assert!(is_object_in_default_location(&locator(moved, IDENTITY, LOCATOR_STATIC)));
    }

    #[test]
    fn small_moves_stay_default_and_larger_ones_do_not() {
        let mut nudged = IDENTITY;
        nudged[12] += 0.1;
        nudged[13] += 0.1;
        assert!(is_object_in_default_location(&locator(nudged, IDENTITY, 0)));
        let mut lifted = IDENTITY;
        lifted[13] += 0.2;
        assert!(!is_object_in_default_location(&locator(lifted, IDENTITY, 0)));
        let mut slid = IDENTITY;
        slid[12] += 0.12;
        slid[14] += 0.12;
        assert!(!is_object_in_default_location(&locator(slid, IDENTITY, 0)));
        let mut turned = IDENTITY;
        turned[0] = f32::from_bits(0x3f7f_0000);
        turned[2] = 0.1;
        assert!(!is_object_in_default_location(&locator(turned, IDENTITY, 0)));
    }

    #[test]
    fn the_radius_is_half_the_box_diagonal() {
        assert_eq!(half_diagonal([-1.0, 0.0, -2.0, 0.0], [1.0, 1.6, 2.0, 0.0]), vmx::length3([2.0, 1.6, 4.0, 0.0]) * 0.5);
    }
}
