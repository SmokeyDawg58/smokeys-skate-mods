//! World.CollectNearbyVehicles827ac638 / CollectNearbyDMOs827aed18.
//! Uses existing manager registries, native KD order and authored model bounds.
use crate::physics_state_ids::PHYSICS_CATEGORY_GRIND;
use super::LivingWorldGrindServices;
#[path = "../pedestrians/pedestrian_sphere_query.rs"]
pub(crate) mod pedestrian_sphere;

use crate::{
    physical_player_phys_out::conditioners::scoring_external::{
        NearbyDmo, NearbyVehicle, NearbyPedestrian, ScoringExternalInput,
    },
    reckoning_numeric::{HostReckoningNumeric, host_dot3, try_length},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[derive(Debug)]
pub(crate) enum ScoringEntitiesError {
    Missing(&'static str),
    Query(String),
    Numeric(crate::reckoning_numeric::ReckoningHostNumericError),
    PedestrianPoseAbsent(u32),
}
impl std::fmt::Display for ScoringEntitiesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
fn query(error: impl std::fmt::Debug) -> ScoringEntitiesError {
    ScoringEntitiesError::Query(format!("{error:?}"))
}
impl LivingWorldGrindServices {
    /// LWPedestrianMan.ResetScoringCollisionFlags82e83648, separate from the
    /// collision flags reset that preserves this scoring mask.
    pub(crate) fn reset_pedestrian_scoring_collision_flags(&mut self) -> Result<(), ScoringEntitiesError> {
        let pedestrians = self.pedestrian_entities.as_mut()
            .ok_or(ScoringEntitiesError::Missing("pedestrian manager"))?;
        for id in pedestrians.ids() {
            pedestrians.get_mut(id).expect("pedestrian map order").state.as_mut()
                .ok_or(ScoringEntitiesError::Missing("pedestrian state"))?.skater_collision_flags = 0;
        }
        Ok(())
    }

    /// DynamicObject.OnEntityCollision82d3ce88 for every contact in the body's
    /// list: unlike Vehicle.OnEntityCollision, its callback returns false. An
    /// actor sets its scoring flag (82d3d080..d0a0); any other contact goes to
    /// the StateOut contact list. This does not replace DMO motion.
    pub(crate) fn resolve_dmo_scoring_contacts(
        &mut self,
        world: &mut crate::physical_player_assembly::PhysicalAssemblyWorld,
    ) -> Result<(), ScoringEntitiesError> {
        use crate::simulation_assembly_backend::constraint_session::physical_island::{
            SimulationIslandBodyHandle, contact_owners::SimulationContactEntityOwner,
        };
        let dmos = self
            .dmo_objects
            .as_mut()
            .ok_or(ScoringEntitiesError::Missing("DMO manager"))?;
        for id in dmos.ids() {
            let dmo = dmos
                .object_mut(id)
                .ok_or(ScoringEntitiesError::Missing("DMO"))?;
            let contacts: Vec<_> = world
                .try_get_island_collisions(&SimulationIslandBodyHandle::Dmo(dmo.body.clone()))
                .map_err(query)?
                .iter()
                .map(|c| {
                    let [vx, vy, vz] = *c.relative_velocity();
                    let [nx, ny, nz] = *c.normal();
                    // 82d3cf88..cfc8: vmsum3fp, then the sign cleared.
                    let force = host_dot3([vx, vy, vz, 0.], [nx, ny, nz, 0.]).abs();
                    (c.other_body().cloned(), c.part(), c.surface(), force)
                })
                .collect();
            for (other, part, surface, force) in contacts {
                let Some(other) = other else {
                    dmo.add_surface_contact(part, surface, force);
                    continue;
                };
                match world.island_body_owner_for_contacts(&other).map_err(query)? {
                    SimulationContactEntityOwner::Physical(owner) => {
                        let actor = owner.actor_query.lock().map_err(query)?;
                        let player = actor
                            .as_ref()
                            .ok_or(ScoringEntitiesError::Missing("DMO contact actor"))?
                            .try_player_index()
                            .map_err(query)?;
                        dmo.skater_collision_flags |= 1u32.checked_shl(player & 63).unwrap_or(0);
                    }
                    SimulationContactEntityOwner::LivingWorld(_) => dmo.add_entity_contact(part, force),
                }
            }
        }
        Ok(())
    }
    pub(crate) fn collect_scoring_entities(
        &self,
        root: [f32; 3],
        player: u32,
        category: i32,
        grind_pathed_guid: u64,
        out: &mut ScoringExternalInput,
    ) -> Result<(), ScoringEntitiesError> {
        use ScoringEntitiesError as E;
        let point = [root[0], root[2]];
        let bit = 1u32.checked_shl(player & 63).unwrap_or(0);
        out.vehicles.clear();
        let mut count = 0;
        let mut ids = [0; 8];
        let mut distances = [0.; 8];
        self.try_get_vehicles_within_range(point, 8., 8, &mut count, &mut ids, &mut distances)
            .map_err(query)?;
        let vehicles = self
            .grind_vehicle_entities
            .as_ref()
            .ok_or(E::Missing("vehicle manager"))?;
        for id in &ids[..count as usize] {
            let Some(vehicle) = vehicles.get(*id) else {
                continue;
            };
            // The +0x9c virtual is IsInitialized82d1d248 (core pointer exists),
            // NOT an active/visible or parked filter. Published host cores exist.
            let size = vehicle.dimensions();
            out.vehicles.push(NearbyVehicle {
                transform: SkeletonRootMatrix::from_columns(
                    vehicle.core.transform.map(SkeletonRootVector::known),
                ),
                length: size[2],
                width: size[0],
                unique_id: vehicle.id,
                kind: vehicle.core.entity_key,
                colliding: vehicle.skater_collision_flags & bit != 0,
            });
        }
        out.pedestrians.clear();
        self.try_get_pedestrians_within_range(point, 3., 8, &mut count, &mut ids, &mut distances)
            .map_err(query)?;
        let pedestrians = self.pedestrian_entities.as_ref().ok_or(E::Missing("pedestrian manager"))?;
        for id in &ids[..count as usize] {
            let Some(pedestrian) = pedestrians.get(*id) else { continue; };
            // Raw827ac92c..948 invokes LWCommonCoreDataProxy.IsValid.
            let Some(core) = pedestrian.core.as_ref() else { continue; };
            let pose = pedestrian.animation.as_ref()
                .and_then(|animation| animation.sim_lod_bone_global.as_ref())
                .ok_or(E::PedestrianPoseAbsent(*id))?;
            let state = pedestrian.state.as_ref().ok_or(E::Missing("pedestrian state"))?;
            let (torso, legs) = pose.scoring_endpoints();
            out.pedestrians.push(NearbyPedestrian {
                torso: SkeletonRootVector::known(torso),
                legs: SkeletonRootVector::known(legs),
                unique_id: pedestrian.id,
                kind: core.entity_key,
                colliding: state.skater_collision_flags & bit != 0,
            });
        }
        out.dmos.clear();
        let objects = self.dmo_objects.as_ref().ok_or(E::Missing("DMO manager"))?;
        let candidates = objects
            .try_objects_within_range(point, 8., 32)
            .map_err(query)?;
        let mut seeking_grind = category == PHYSICS_CATEGORY_GRIND;
        for (id, _) in candidates {
            let Some(object) = objects.object(id) else {
                continue;
            };
            // A network game asks Dmo::SimManager, whose CollectNearbyDMOsForScoring 0x825e8dc0
            // adds none.
            if object.online_static {
                continue;
            }
            if !dmo_close_enough(
                root,
                &object.transform,
                object.model.bbox_min,
                object.model.bbox_max,
            )? {
                continue;
            }
            if out.dmos.len() >= 8 {
                continue;
            }
            let mut is_grinding = false;
            if seeking_grind {
                // DoesContainGrindSpline82e57e38 compares the full GUID in each
                // pointed-to tSplineHeader, not the manager's registration ID.
                for spline in object.grind_splines.values() {
                    if let Some(header) = spline
                        .source
                        .data()
                        .spline_at_object_offset(spline.header_offset)
                    {
                        if header.guid == grind_pathed_guid {
                            is_grinding = true;
                            break;
                        }
                    }
                }
                if is_grinding {
                    seeking_grind = false;
                }
            }
            out.dmos.push(NearbyDmo {
                transform: SkeletonRootMatrix::from_columns(std::array::from_fn(|i| {
                    SkeletonRootVector::known(
                        object.transform[i * 4..i * 4 + 4].try_into().unwrap(),
                    )
                })),
                minimum: SkeletonRootVector::known(object.model.bbox_min),
                maximum: SkeletonRootVector::known(object.model.bbox_max),
                kind: object.model.characteristic_collection,
                unique_id: id,
                colliding: object.skater_collision_flags & bit != 0,
                skater_is_grinding: is_grinding,
            });
        }
        Ok(())
    }
    /// World.Update3 resets these AFTER scoring, before later LW contact
    /// callbacks populate the next collection. Distinct from Update1 reset.
    pub(crate) fn reset_scoring_collision_flags(&mut self) -> Result<(), ScoringEntitiesError> {
        let vehicles = self
            .grind_vehicle_entities
            .as_mut()
            .ok_or(ScoringEntitiesError::Missing("vehicle manager"))?;
        for id in vehicles.ids() {
            vehicles
                .get_mut(id)
                .ok_or(ScoringEntitiesError::Missing("vehicle"))?
                .skater_collision_flags = 0;
        }
        let dmos = self
            .dmo_objects
            .as_mut()
            .ok_or(ScoringEntitiesError::Missing("DMO manager"))?;
        for id in dmos.ids() {
            dmos.object_mut(id)
                .ok_or(ScoringEntitiesError::Missing("DMO"))?
                .skater_collision_flags = 0;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "living_world_scoring_tests.rs"]
mod tests;

fn dmo_close_enough(
    root: [f32; 3],
    transform: &[f32; 16],
    minimum: [f32; 4],
    maximum: [f32; 4],
) -> Result<bool, ScoringEntitiesError> {
    let extent: [f32; 4] = std::array::from_fn(|i| maximum[i] - minimum[i]);
    let centre: [f32; 4] = std::array::from_fn(|i| extent[i].mul_add(0.5, minimum[i]));
    let world: [f32; 3] = std::array::from_fn(|i| {
        let x = transform[i].mul_add(centre[0], transform[12 + i]);
        let y = transform[4 + i].mul_add(centre[1], x);
        transform[8 + i].mul_add(centre[2], y)
    });
    let distance = try_length(
        [world[0] - root[0], 0., world[2] - root[2], 0.],
        &mut HostReckoningNumeric,
    )
    .map_err(ScoringEntitiesError::Numeric)?;
    let radius =
        try_length(extent, &mut HostReckoningNumeric).map_err(ScoringEntitiesError::Numeric)? * 0.5;
    Ok(distance - (radius + 0.5) < f32::from_bits(0x3d4c_cccd))
}
