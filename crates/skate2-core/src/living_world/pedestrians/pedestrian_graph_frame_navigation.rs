//! Active navigation tasks borrowing the same targetable and targeting owners.
use super::FrameServices;
use crate::{
    living_world_chase::TargetableId,
    living_world_pedestrian_entity::{PedestrianConstruction, targeting::{FleeSource, TargetIteratorKind, TargetingWorld}},
    pedestrian_navigation::{mesh::NavigationEntityFacts, wander::ResolvedAvoidVolumes},
    reckoning_numeric::{HostReckoningNumeric, ReckoningVector},
};

impl FrameServices<'_> {
    /// ITargetable facing wrappers -> Entity.GetFacing8258f128: column2 of
    /// GetTransform. Actor uses PhysOut_Skeleton+432, not the raw root pose.
    pub(crate) fn target_facing(&self,entity:&PedestrianConstruction,target:TargetableId)
        ->Result<Option<[f32;4]>,String>{
        if !self.target_exists(entity,target)? {return Ok(None);}
        if target.kind==2 {return Ok(self.dmos.object(target.id as u32).map(|o|
            [o.transform[8],o.transform[9],o.transform[10],o.transform[11]]));}
        if target.kind==3 {return Ok(self.vehicles.get(target.id as u32).map(|v|v.core.transform[2]));}
        if let Some(actor)=self.actor(target)? {
            let physical=actor.cached_physical_allocation().map_err(|e|format!("target facing allocation: {e:?}"))?;
            let output=physical.owner().phys_out().lock().map_err(|_|"target facing PhysOut lock")?;
            return Ok(Some(output.try_skeleton_output().map_err(|e|format!("target facing output: {e:?}"))?
                .effective_skeleton_root_matrix.columns()[2].operand()));
        }
        let target=if target.id as u32==entity.id{entity}else{
            self.pedestrian(target).ok_or("target facing pedestrian disappeared")?};
        Ok(Some(target.core.as_ref().ok_or("target facing core absent")?.transform[2]))
    }

    /// Actor825ed338 reads damped SystemReckoning velocity without the census
    /// near-zero clamp. Pedestrian82e94658 and Vehicle/DMO82d36158 read core+144.
    pub(crate) fn target_velocity(&self, entity: &PedestrianConstruction, target: TargetableId)
        -> Result<Option<[f32; 4]>, String> {
        if !target.is_valid() { return Ok(None); }
        if target.kind == 2 { return Ok(self.dmos.object(target.id as u32).map(|o| o.velocity)); }
        if target.kind == 3 { return Ok(self.vehicles.get(target.id as u32).map(|v| v.core.velocity)); }
        if let Some(actor) = self.actor(target)? {
            let allocation = actor.cached_physical_allocation().map_err(|e| format!("target velocity allocation: {e:?}"))?;
            let output = allocation.owner().phys_out().lock().map_err(|_| "target velocity PhysOut lock")?;
            return Ok(Some(output.try_camera_system_output().map_err(|e| format!("target velocity output: {e:?}"))?
                .damped_centre_of_mass_velocity.operand()));
        }
        let target = if target.kind == 1 && target.id as u32 == entity.id { Some(entity) } else { self.pedestrian(target) };
        target.map(|target| target.core.as_ref().map(|core| core.velocity)
            .ok_or_else(|| "target pedestrian velocity core absent".to_string())).transpose()
    }

    /// Resolve only the external targets consumed by the selected iterator.
    /// Absent targetables retain native null semantics, not a fabricated point.
    pub(crate) fn with_navigation<R>(&mut self, entity: &mut PedestrianConstruction, kind: Option<TargetIteratorKind>,
        run: impl FnOnce(&mut PedestrianConstruction, &NavigationEntityFacts,
            &mut TargetingWorld<'_, ResolvedAvoidVolumes>, &mut HostReckoningNumeric) -> Result<R, String>,
    ) -> Result<R, String> {
        let facts = entity.navigation_entity_facts(self.cvars)?;
        //Flee.Begin initializes the iterator then immediately increments it.
        //Resolve the real want10 source also for that first increment.
        let flee_source = if kind == Some(TargetIteratorKind::Flee) {
            let target = entity.state.as_ref().ok_or("flee state absent")?.wants[10].target;
            match self.target_velocity(entity, target)? {
                Some(velocity) => Some(FleeSource { velocity,
                    position: self.target_position(entity, target)?.ok_or("flee target position absent")? }),
                None => None,
            }
        } else { None };
        let chasee_position = if kind == Some(TargetIteratorKind::Chase) {
            let target = entity.state.as_ref().ok_or("chase state absent")?.chaser.chasee;
            if self.with_chasee(entity, target, |state| state.is_some())? {
                self.target_position(entity, target)?
            } else { None }
        } else { None };
        let mut world = TargetingWorld {dmos:self.dmos, mesh: self.mesh, rain: self.rain, random: self.random,
            avoid: self.avoid, cvars: self.cvars, props: self.props, flee_source, chasee_position };
        run(entity, &facts, &mut world, &mut HostReckoningNumeric)
    }

    pub(crate) fn seek_navigation(&mut self, entity: &mut PedestrianConstruction) -> Result<(), String> {
        if entity.navigation_if_present_mut().is_none() { return Ok(()); }
        let kind = entity.current_target_iterator();
        self.with_navigation(entity, kind, |entity, facts, world, numeric|
            entity.seek_current_target(false, facts, world, numeric))
    }
    pub(crate) fn resolve_navigation(&mut self, entity: &mut PedestrianConstruction) -> Result<(), String> {
        if !entity.uses_mesh_navigation() { return Ok(()); }
        if entity.navigation_if_present_mut().is_none() { return Ok(()); }
        let kind = entity.current_target_iterator();
        self.with_navigation(entity, kind, |entity, facts, world, numeric|
            entity.resolve_current_navigation(facts, world, numeric))
    }
    pub(crate) fn update_navigation(&mut self, entity: &mut PedestrianConstruction, dt: f32) -> Result<(), String> {
        if !entity.uses_mesh_navigation() { return Ok(()); }
        if entity.navigation_if_present_mut().is_none() { return Ok(()); }
        let kind = entity.current_target_iterator();
        self.with_navigation(entity, kind, |entity, facts, world, numeric|
            entity.update_current_navigation(dt, facts, world, numeric))
    }
    pub(crate) fn update_navigation_position(&mut self, entity: &mut PedestrianConstruction,
        player_position: [f32; 4],
    ) -> Result<(), String> {
        let tolerance = self.cvars.try_pedestrian_default_mesh_y_tolerance()
            .map_err(|e| format!("pedestrian position mesh tolerance: {e:?}"))?;
        let kind = entity.target_iterators.mesh_current;
        self.with_navigation(entity, kind, |entity, facts, world, numeric|
            entity.update_positional_data(player_position, tolerance, facts, world, numeric))
    }
}
