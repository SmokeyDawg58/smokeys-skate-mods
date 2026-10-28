//! `WaypointMan::Update` 0x82ce8e00 on the live living world: the queued spectator waypoint groups'
//! forced spectators (`UpdateSpectatorActivationInfo` 0x82ce71b8).
use crate::{
    input_cvar_runtime::InputCvarReferences, living_world_grind_services::census::targets,
    native_skater_random::SimulationCommandRandom, physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::HostReckoningNumeric, static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

impl super::SelectedRecipeFrame<'_> {
    /// `WaypointMan::Update`: with spectators queued, the census reserves a forced slot for each
    /// activation (`ReserveForceSpawnSlots`), and each activation spawns one spectator an update
    /// until all have spawned, when it leaves the queue and gives back its forced count.
    pub(crate) fn update_spectators(
        &self,
        world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences,
        ground: &StaticGroundPool,
        mut simulation_random: Option<&mut SimulationCommandRandom>,
    ) -> Result<(), String> {
        let owner = Arc::clone(world.grind_services.living_world());
        {
            let mut living = owner.lock().map_err(|_| "spectator LivingWorld lock")?;
            let count = living.waypoint_man.pending.len();
            if count == 0 {
                return Ok(());
            }
            living.census.reserve_force_spawn_slots(i32::try_from(count).map_err(|_| "spectator reservation")?);
        }
        let mut index = 0;
        loop {
            let entry = {
                let living = owner.lock().map_err(|_| "spectator LivingWorld lock")?;
                let Some(entry) = living.waypoint_man.pending.get(index) else { break };
                entry.clone()
            };
            let remaining = self.update_spectator_activation(world, cvars, ground, simulation_random.as_deref_mut(), &entry)?;
            let mut living = owner.lock().map_err(|_| "spectator LivingWorld lock")?;
            if remaining == 0 {
                let man = &mut living.waypoint_man;
                man.num_force_spectators = man.num_force_spectators.wrapping_sub(entry.num_force_spawn);
                man.pending.remove(index);
            } else {
                living.waypoint_man.pending[index].force_spawn_remaining = remaining;
                index += 1;
            }
        }
        Ok(())
    }

    /// `UpdateSpectatorActivationInfo`: the GUID's group activates (`Prop::Activate`, every update:
    /// nothing sets `WaypointActivated`); at its first unlocked waypoint a pedestrian of the
    /// category group (`ResolveCategoryGroupToEntity`) is forced to spawn facing along the
    /// waypoint (`SpawnPedestrian`, initial false, allowCull false, force true,
    /// allowWorldCollisions true, ignoreNavMesh false, allowHandProp false) and, when the group's
    /// behaviour transfers to it (`HasTransferableBehaviour`, `CheckTransferConditionsWithSink`,
    /// `TransferBehaviourToSink`), it locks the waypoint and takes it as its current one. Each
    /// spawn counts down the remaining spectators, which this returns.
    fn update_spectator_activation(
        &self,
        world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences,
        ground: &StaticGroundPool,
        simulation_random: Option<&mut SimulationCommandRandom>,
        entry: &crate::living_world_grind_services::waypoints::spectators::SpectatorActivation,
    ) -> Result<u32, String> {
        let owner = Arc::clone(world.grind_services.living_world());
        let mut remaining = entry.force_spawn_remaining;
        let (prop, waypoint, entity) = {
            let mut living = owner.lock().map_err(|_| "spectator LivingWorld lock")?;
            let props = living.props.as_deref_mut().ok_or("spectator PropMan absent")?;
            let Some(prop) = props.waypoint_group_by_guid(entry.waypoint_guid) else { return Ok(remaining) };
            if !entry.waypoint_activated {
                props.activate_prop(prop)?;
            }
            let Some(waypoint) = props.first_unlocked_waypoint(prop)? else { return Ok(remaining) };
            let mut random = living.random.take().ok_or("Manager RNG absent")?;
            let entity = targets::resolve_category_group_to_entity(entry.category_group, self.vault, &mut random);
            living.random = Some(random);
            (prop, waypoint, entity?)
        };
        let (reference, location, orientation) = waypoint;
        let look = std::array::from_fn(|lane| location[lane] + orientation[lane]);
        let transform = crate::reckoning_numeric::try_look_at_up_eye(look, [0., 1., 0., 0.], location, &mut HostReckoningNumeric)
            .map_err(|e| format!("spectator transform: {e:?}"))?;
        let Some(id) = self.spawn_census_pedestrian(world, cvars, ground, entity, transform, false, false, true, true, false, false)? else {
            return Ok(remaining);
        };
        let transfers = owner
            .lock()
            .map_err(|_| "spectator LivingWorld lock")?
            .props
            .as_deref()
            .and_then(|props| props.entities.get(prop))
            .is_some_and(|source| source.transferable.is_some());
        if transfers {
            let actors = world.entity_roster.as_ref().ok_or("spectator Actor roster absent")?.snapshot().map_err(|e| format!("spectator Actor roster: {e:?}"))?;
            let triggers = self.triggers.lock().map_err(|_| "spectator triggers lock")?;
            let avoid = self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
            drop(triggers);
            let painter = self.painter.lock().map_err(|_| "spectator painter lock")?;
            let mut living = owner.lock().map_err(|_| "spectator LivingWorld lock")?;
            let mut random = living.random.take().ok_or("Manager RNG absent")?;
            let result = crate::pedestrian_graph::backend::frame::with_entity_and_random(
                &mut living,
                world,
                id,
                cvars,
                &actors,
                &painter,
                self.vault,
                &avoid,
                &mut random,
                |pedestrian, frame| {
                    if !super::transfer::check_transfer_conditions(frame, pedestrian, prop, ground, simulation_random)? {
                        return Ok(());
                    }
                    frame.props.transfer_behaviour_to_pedestrian(prop, pedestrian)?;
                    frame.props.point_mut(reference)?.lock(id)?;
                    pedestrian.set_current_waypoint(Some(reference));
                    Ok(())
                },
            );
            living.random = Some(random);
            result?;
        }
        remaining = remaining.wrapping_sub(1);
        Ok(remaining)
    }
}
