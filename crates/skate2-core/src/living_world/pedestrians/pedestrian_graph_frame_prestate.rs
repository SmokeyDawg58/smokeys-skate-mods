//! Pedestrian.PreStateGraphUpdate82e9f328 phases over retained owners.
use super::FrameServices;
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    living_world_chase::TargetableId,static_ground_pool::StaticGroundPool};

impl FrameServices<'_> {
    /// Gameplay portion of PostStateGraphUpdate82e6e090. LWBasicEntity this
    /// is complete-object+32: +A80 is perception+2720, NOT PedStateOut.
    /// Virtual slots12/28 clear only the audible/spectate active flags.
    /// Base82e62858 maintains optional debug text history, not simulation state.
    pub(crate) fn pedestrian_poststate(&mut self,entity:&mut PedestrianConstruction) {
        entity.perception.clear_audible();
        entity.perception.clear_spectate_event();
    }

    /// Native prefix through UpdateEntityKnowledge. UpdateThreats, needs/wants,
    /// Chasee.UpdateByChasee and zone interests MUST follow before the suffix.
    pub(crate) fn pedestrian_prestate_prefix(&mut self,entity:&mut PedestrianConstruction,
        dt:f32,ground:&StaticGroundPool)->Result<(),String>{
        // LWBasicEntity.PreStateGraphUpdate82e4c248, before perception/timers.
        if entity.plugin_removal_requested {
            self.cleanly_end_plugin(entity)?;
            entity.remove_plugin(self.props)?;
        }
        // Virtual UpdatePerceptions82e6fa70 -> sole SimLOD publication.
        let pose=entity.animation.as_ref().ok_or("prestate animation absent")?
            .sim_lod_bone_global.as_ref().ok_or("prestate simulation pose absent")?;
        entity.perception.update(pose);
        let state=entity.state.as_mut().ok_or("prestate AI state absent")?;
        state.update_timers(dt);
        state.update_mood(dt,|event|self.mood.expire_time(event))?;
        self.update_entity_knowledge(entity,dt,ground)
    }

    /// Native suffix AFTER UpdateZoneInterests. Keep the ignore target only
    /// while it still resolves to an actually wiped-out ITargetable.
    pub(crate) fn pedestrian_prestate_suffix(&mut self,entity:&mut PedestrianConstruction)
        ->Result<(),String>{
        entity.update_hand_prop(self.world,self.dmos,self.dmo_locators,self.vault,self.cvars,self.random,self.others)?;
        let target=entity.ignore_collision_target;
        if target.is_valid() && self.target_wiped_out(entity,target)?!=Some(true) {
            entity.ignore_collision_target=TargetableId::INVALID;
        }
        Ok(())
    }
}
