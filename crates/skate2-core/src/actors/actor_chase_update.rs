//! Actual Actor Chasee callback at825f1140, sharing the pedestrian chase owner.
use super::{ActorConstruction,update_input_flags::SelectedActorUpdateError};
use crate::{living_world_grind_services::LivingWorldGrindServices,
    reckoning_numeric::ReckoningVector,
    living_world_chase::{TargetableId,update::{self,ChaseeUpdateServices}},
    living_world_pedestrian_entity::PedestrianConstruction as Entity,
    physical_player_phys_out::PhysicalPlayerPhysOut,
    hermes_game_sim_queue::{LivingWorldMessage,MoodEvent}};
struct Services<'a>{living:&'a mut LivingWorldGrindServices,output:&'a PhysicalPlayerPhysOut,
    id:TargetableId,protector:TargetableId,is_local:bool}
impl ActorConstruction {
    pub(crate) fn try_update_after_physics_setup_living_world(&self,dt:f32,living:&mut LivingWorldGrindServices,
        local:&crate::skater_manager_local_identity::SkaterManagerLocalIdentity)
        ->Result<(),SelectedActorUpdateError>{
        use SelectedActorUpdateError as E;
        let physical=self.cached_physical_allocation().map_err(|e|E::Chasee(format!("Actor chase physical: {e:?}")))?;
        self.try_update_after_physics_setup_with_chasee(dt,|chasee,id,protector|{
            // Native empty branch reads no physics or Manager services.
            if chasee.chasers.is_empty(){return Ok(());}
            let is_local=local.try_is_local_player(||Ok::<_,String>(id.id)).map_err(|e|E::Chasee(e.to_string()))?;
            let output=physical.owner().phys_out().lock().map_err(|_|E::Chasee("Actor chase PhysOut lock".into()))?;
            update::update(chasee,id,dt,&mut Services{living,output:&output,id,protector,is_local}).map_err(E::Chasee)
        })
    }
}
impl ChaseeUpdateServices for Services<'_>{
    fn chaser(&self,id:TargetableId)->Result<&Entity,String>{
        if !id.is_valid()||id.kind!=1{return Err("native Actor Chasee missing IChaser assertion".into());}
        self.living.pedestrian_entities.as_ref().and_then(|r|r.get(id.id as u32)).ok_or("Actor chaser absent".into())
    }
    fn chaser_mut(&mut self,id:TargetableId)->Result<&mut Entity,String>{
        if !id.is_valid()||id.kind!=1{return Err("native Actor Chasee missing IChaser assertion".into());}
        self.living.pedestrian_entities.as_mut().and_then(|r|r.get_mut(id.id as u32)).ok_or("Actor chaser absent".into())
    }
    fn position(&self,id:TargetableId)->Result<[f32;4],String>{
        if id==self.id{return Ok(self.output.try_skeleton_output().map_err(|e|format!("Actor chase position: {e:?}"))?
            .skeleton_root_matrix.columns()[3].operand());}
        Ok(self.chaser(id)?.core.as_ref().ok_or("chaser position core absent")?.transform[3])
    }
    fn velocity(&self,id:TargetableId)->Result<[f32;4],String>{
        if id==self.id{return Ok(self.output.try_camera_system_output().map_err(|e|format!("Actor chase velocity: {e:?}"))?
            .damped_centre_of_mass_velocity.operand());}
        Ok(self.chaser(id)?.core.as_ref().ok_or("chaser velocity core absent")?.velocity)
    }
    fn facing(&self,id:TargetableId)->Result<[f32;4],String>{
        if id==self.id{return Ok(self.output.try_skeleton_output().map_err(|e|format!("Actor chase facing: {e:?}"))?
            .effective_skeleton_root_matrix.columns()[2].operand());}
        Ok(self.chaser(id)?.core.as_ref().ok_or("chaser facing core absent")?.transform[2])
    }
    fn switch_thresholds(&self)->Result<(f32,f32),String>{self.living.chase.as_ref().ok_or("Actor ChaseMan absent")?.primary_switch_thresholds()}
    fn switch_time(&self)->Result<f32,String>{self.living.chase.as_ref().ok_or("Actor ChaseMan absent")?.primary_auto_switch_time()}
    fn group_end_reason(&self,stored:i32)->Result<i32,String>{
        Ok(self.living.chase.as_ref().ok_or("Actor ChaseMan absent")?.group_end_reason(stored,self.is_local))
    }
    fn protector_distance(&self)->Result<Option<f32>,String>{
        // Actual Actor constructor initializes its protector ID invalid. No
        // fake default for a later registered protector with no typed owner.
        if !self.protector.is_valid() || self.protector.kind!=1{return Ok(None);}
        Ok(self.living.pedestrian_entities.as_deref().and_then(|r|r.get(self.protector.id as u32))
            .and_then(|p|p.big_black.as_ref()).map(|b|b.scalar(
                crate::living_world_pedestrian_entity::big_black::IMMINENT_RADIUS)).transpose()?)
    }
    fn ongoing_rate(&self)->Result<f32,String>{self.living.mood.as_ref().ok_or("Actor MoodMan absent")?.update_rate(14)}
    fn send_ongoing(&self,chaser:TargetableId,chasee:TargetableId,position:[f32;4])->Result<(),String>{
        self.living.game_sim_events.as_ref().ok_or("Actor GameSim broadcaster absent")?
            .send(LivingWorldMessage::Mood(MoodEvent{event_type:14,position:position.map(f32::to_bits),
                perpetrator:chaser,victim:chasee,loudness_distance_factor:1_f32.to_bits(),visibility_distance_factor:1_f32.to_bits()}))
    }
}
