//! Concrete native Patrol behaviour and waypoint pointer lifecycle.
use super::{PedestrianConstruction, targeting::TargetingWorld};
use crate::{
    living_world_grind_services::waypoints::{PropMan, WaypointRef},
    pedestrian_navigation::{NavigationTargetMode, mesh::NavigationEntityFacts, wander::WanderAvoidVolumes},
    reckoning_numeric::ReckoningNumeric,
};

/// Per-behaviour instance payload, not shared between unrelated graph leaves.
pub(crate) struct PatrolExecution { pub(crate) saved_arrival_radius:Option<f32> }
// Folded CreateInstance82712148 writes the float payload at+8 to zero.
impl PatrolExecution {pub(crate) fn new()->Self{Self{saved_arrival_radius:Some(0.)}}}

impl PedestrianConstruction {
    /// ReturnToPatrolZone.Update8270ead0..ec28. Return arrival before looking
    /// up the optional entity of interest, preserving the native lazy read.
    pub(crate) fn update_return_to_patrol_zone<N: ReckoningNumeric, A: WanderAvoidVolumes>(
        &mut self, facts: &NavigationEntityFacts, world: &mut TargetingWorld<'_, A>, numeric: &mut N,
    ) -> Result<bool, String> where N::Error: std::fmt::Debug {
        let start = self.patrol_zone_start_point;
        self.target_explicit(start, NavigationTargetMode::Explicit, facts, world, numeric)?;
        let navigation = self.navigation_mut()?;
        navigation.seek_speed = 1.8;
        navigation.has_arrived_xz(facts.position, start, navigation.arrival_radius, numeric)
            .map_err(|e| format!("return to patrol arrival: {e:?}"))
    }

    /// ReturnToPatrolZone.Update8270ec2c..ecc8 arrival suffix. Caller resolves
    /// GetEntityOfInterest only AFTER the prefix returns true, then passes its
    /// actual GetPosition result. A null interest still stops ramped/seek speed.
    pub(crate) fn finish_return_to_patrol_zone<N: ReckoningNumeric, A: WanderAvoidVolumes>(
        &mut self, interest_position: Option<[f32; 4]>, facts: &NavigationEntityFacts,
        world: &mut TargetingWorld<'_, A>, numeric: &mut N,
    ) -> Result<(), String> where N::Error: std::fmt::Debug {
        if let Some(position) = interest_position {
            self.target_explicit(position, NavigationTargetMode::ExplicitNoSeekNoAstar, facts, world, numeric)?;
        }
        let navigation = self.navigation_mut()?;
        navigation.ramped_seek_speed = 0.;
        navigation.seek_speed = 0.;
        Ok(())
    }

    /// LockFirstWaypoint.Begin82708020 selects vector entry zero, NOT the first
    /// unlocked entry. The source Prop comes from the actual behaviour source.
    pub(crate) fn lock_first_waypoint(&mut self,source_prop:Option<u32>,props:&mut PropMan)->Result<(),String>{
        self.navigation_mut()?.arrival_radius=0.2;
        if self.current_waypoint.is_some(){return Ok(());}
        let Some(prop)=source_prop else{return Ok(());};
        let group=props.entities.get(prop).ok_or("behaviour source Prop absent")?;
        let Some(group)=group.waypoint.as_ref() else{return Ok(());};
        if group.points.first().is_none_or(|point|point.locking_entity.is_some()){return Ok(());}
        self.set_current_waypoint(Some(WaypointRef{prop,index:0}));
        props.point_mut(WaypointRef{prop,index:0})?.lock(self.id)?;
        self.navigation_rig_enabled=true;
        Ok(())
    }
    /// LockClosestWaypoint.Begin8270ad88 uses full XYZ squared distance,
    /// including the group's native last-entry tie rule.
    pub(crate) fn lock_closest_waypoint(&mut self,source_prop:Option<u32>,props:&mut PropMan)->Result<(),String>{
        self.navigation_mut()?.arrival_radius=0.2;
        if self.current_waypoint.is_some(){return Ok(());}
        let Some(prop)=source_prop else{return Ok(());};
        let group=props.entities.get(prop).ok_or("behaviour source Prop absent")?;
        let Some(group)=group.waypoint.as_ref() else{return Ok(());};
        let position=self.core.as_ref().ok_or("pedestrian position absent")?.transform[3];
        let Some(index)=group.closest_unlocked(position) else{return Ok(());};
        self.set_current_waypoint(Some(WaypointRef{prop,index}));
        props.point_mut(WaypointRef{prop,index})?.lock(self.id)?;
        self.navigation_rig_enabled=true;
        Ok(())
    }
    /// SetCurrentWaypoint82e6e600 only assigns the pointer. The iterator's
    /// current index is an independent field; callers must not infer/reset it.
    pub(crate) fn set_current_waypoint(&mut self,reference:Option<WaypointRef>){self.current_waypoint=reference;}
    /// ClearCurrentWaypoint82e79b48 ignores locking identity, then nulls pointer.
    pub(crate) fn clear_current_waypoint(&mut self,props:&mut PropMan)->Result<(),String>{
        if let Some(reference)=self.current_waypoint {
            props.point_mut(reference)?.unlock();self.current_waypoint=None;
        }
        Ok(())
    }
    pub(crate) fn notify_waypoint_deregister(&mut self,guid:u64,props:&mut PropMan)->Result<(),String>{
        if let Some(reference)=self.current_waypoint {
            if props.group(reference.prop)?.guid()?==guid {self.clear_current_waypoint(props)?;}
        }
        Ok(())
    }
    /// Begin826fe358: switch navigation, switch+seek waypoint, enable rig,
    /// save arrival radius, apply authored patrol radius, unlock current point.
    pub(crate) fn begin_patrol<N:ReckoningNumeric,A:WanderAvoidVolumes>(
        &mut self,execution:&mut PatrolExecution,facts:&NavigationEntityFacts,
        world:&mut TargetingWorld<'_,A>,numeric:&mut N,
    )->Result<(),String> where N::Error:std::fmt::Debug {
        self.switch_navigation_mode(0);
        self.switch_targeting_mode(1,facts,world,numeric)?;
        self.navigation_rig_enabled=true;
        execution.saved_arrival_radius=Some(self.navigation_mut()?.arrival_radius);
        let attributes=&self.resources.as_ref().ok_or("pedestrian locomotion resources absent")?.locomotion;
        if attributes.is_null(){return Err("native patrol locomotion collection assertion".into());}
        let value=attributes.try_get_data(0xb90a_af68_4974_0864,0).map_err(|e|e.to_string())?;
        let radius=match value {None=>0.,Some(value)=>f32::from_be_bytes(value.bytes().get(..4)
            .ok_or("patrol arrival radius truncated")?.try_into().expect("four bytes"))};
        self.navigation_mut()?.arrival_radius=radius;
        if let Some(reference)=self.current_waypoint {world.props.point_mut(reference)?.unlock();}
        Ok(())
    }
    /// Update826ec230 sets Auto before seek speed8. It does not directly move
    /// the entity; navigation/motion retain responsibility for actual movement.
    pub(crate) fn update_patrol(&mut self)->Result<(),String>{
        self.navigation_mut()?.target_mode=NavigationTargetMode::Auto;
        self.navigation_mut()?.seek_speed=8.;Ok(())
    }
    /// End826fe478 restores only the saved arrival radius.
    pub(crate) fn end_patrol(&mut self,execution:&PatrolExecution)->Result<(),String>{
        self.navigation_mut()?.arrival_radius=execution.saved_arrival_radius.ok_or("Patrol.End without native Begin payload")?;
        Ok(())
    }
}
