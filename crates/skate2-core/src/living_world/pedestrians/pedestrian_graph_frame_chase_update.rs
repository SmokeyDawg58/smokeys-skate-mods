//! Pedestrian adapter for the shared Chasee.UpdateByChasee82de5200 owner.
use super::FrameServices;
use crate::{living_world_pedestrian_entity::PedestrianConstruction as Entity,
    living_world_chase::{TargetableId,update::{self,ChaseeUpdateServices}},
    hermes_game_sim_queue::{LivingWorldMessage,MoodEvent}};
struct Services<'a,'b>{frame:&'a mut FrameServices<'b>,entity:&'a mut Entity}
impl FrameServices<'_>{
    pub(crate) fn update_chasee(&mut self,entity:&mut Entity,dt:f32)->Result<(),String>{
        let mut chasee=entity.chasee.take().ok_or("pedestrian Chasee absent")?;
        let id=TargetableId{kind:1,id:entity.id as i32};
        let result=update::update(&mut chasee,id,dt,&mut Services{frame:self,entity});
        entity.chasee=Some(chasee);
        result
    }
}
impl ChaseeUpdateServices for Services<'_,'_>{
    fn chaser(&self,id:TargetableId)->Result<&Entity,String>{
        if !id.is_valid()||id.kind!=1{return Err("native Chasee missing IChaser assertion".into());}
        if id.id as u32==self.entity.id{Ok(self.entity)}else{self.frame.pedestrian(id).ok_or("chaser absent".into())}
    }
    fn chaser_mut(&mut self,id:TargetableId)->Result<&mut Entity,String>{
        if !id.is_valid()||id.kind!=1{return Err("native Chasee missing IChaser assertion".into());}
        if id.id as u32==self.entity.id{Ok(self.entity)}else{self.frame.pedestrian_mut(id).ok_or("chaser absent".into())}
    }
    fn position(&self,id:TargetableId)->Result<[f32;4],String>{self.frame.target_position(self.entity,id)?.ok_or("Chasee position absent".into())}
    fn velocity(&self,id:TargetableId)->Result<[f32;4],String>{self.frame.target_velocity(self.entity,id)?.ok_or("Chasee velocity absent".into())}
    fn facing(&self,id:TargetableId)->Result<[f32;4],String>{self.frame.target_facing(self.entity,id)?.ok_or("Chasee facing absent".into())}
    fn switch_thresholds(&self)->Result<(f32,f32),String>{self.frame.chase.primary_switch_thresholds()}
    fn switch_time(&self)->Result<f32,String>{self.frame.chase.primary_auto_switch_time()}
    // PDB Pedestrian/Guard has no IProtectee interface.
    fn protector_distance(&self)->Result<Option<f32>,String>{Ok(None)}
    fn group_end_reason(&self,stored:i32)->Result<i32,String>{Ok(stored)}
    fn ongoing_rate(&self)->Result<f32,String>{self.frame.mood.update_rate(14)}
    fn send_ongoing(&self,chaser:TargetableId,chasee:TargetableId,position:[f32;4])->Result<(),String>{
        self.frame.game_sim_events.send(LivingWorldMessage::Mood(MoodEvent{event_type:14,
            position:position.map(f32::to_bits),perpetrator:chaser,victim:chasee,
            loudness_distance_factor:1_f32.to_bits(),visibility_distance_factor:1_f32.to_bits()}))
    }
}
