//! ChaseMan registration effects over the actual shared session consumers.
use super::FrameServices;
use crate::{living_world_chase::{ChaseEvent,ChaseRegistrationServices,TargetableId},
    living_world_pedestrian_entity::PedestrianConstruction as Entity,
    skater_manager_local_identity::SkaterManagerLocalIdentity,
    input_cvar_runtime::InputCvarReferences,
    hermes_game_sim_queue::{GameSimEventRoute,LivingWorldMessage}};
struct Registration<'a,'b> {
    entity:&'a Entity,
    others:&'a [(u32,&'b mut Entity)],
    local:&'a SkaterManagerLocalIdentity,
    cvars:&'a InputCvarReferences,
    events:&'a GameSimEventRoute,
    local_chase_forced:bool,
}
impl ChaseRegistrationServices for Registration<'_,'_> {
    fn dispatch(&mut self,event:ChaseEvent)->Result<(),String>{
        match event {
            ChaseEvent::LocalChaseStart|ChaseEvent::LocalChaseEnd=>{
                self.local.try_challenge_broadcaster().map_err(|e|format!("chase broadcaster: {e:?}"))?
                    .send_local_chase(matches!(event,ChaseEvent::LocalChaseStart))
                    .map_err(|e|format!("chase challenge enqueue: {e:?}"))
            }
            ChaseEvent::ChaserMoodStarted(id)=>self.events.send(LivingWorldMessage::ChaseStart(id.id as u32)),
            ChaseEvent::ChaserMoodEnded(id)=>self.events.send(LivingWorldMessage::ChaseEnd(id.id as u32)),
            ChaseEvent::PersonalBestChaserCount(count)=>{
                self.local.backend().update_personal_best(1,count as f32,self.cvars)?;
                self.local.backend().record_personal_best(1,self.cvars)
            }
            ChaseEvent::ChaseFinished{captured,duration,offboard_duration}=>{
                let stats=self.local.backend();
                stats.update_additive_statistic(6,1.,false,self.cvars)?;
                stats.update_personal_best(0,duration,self.cvars)?;
                stats.record_personal_best(0,self.cvars)?;
                if captured {stats.update_additive_statistic(7,1.,false,self.cvars)} else {
                    self.local.send_chase_end_message(false,false)?;
                    // UnRegisterChaser82718a00..7c: career, STILL offboard
                    // when the last chaser leaves, and strictly more than
                    // the live threshold's accumulated offboard chase time.
                    if stats.try_game_mode().map_err(|e|e.to_string())?==1 {
                        let actor=self.local.try_local_actor().map_err(|e|e.to_string())?
                            .ok_or("RunningMan local Actor absent")?;
                        let offboard={
                            let physical=actor.cached_physical_allocation().map_err(|e|e.to_string())?;
                            let output=physical.owner().phys_out().lock().map_err(|_|"RunningMan PhysOut lock")?;
                            output.try_filtered_output().map_err(|e|format!("RunningMan PhysOut: {e:?}"))?.filtered_physics_state as i32==6
                        };
                        if offboard && offboard_duration>self.cvars.try_running_man_achievement_time().map_err(|e|e.to_string())? {
                            // Raw li r3,0x22 at82718a78: AT_RunningMan.
                            stats.grant_achievement(34,0)?;
                            if std::env::var_os("SKATE2_TRACE_SECURITY_CAPTURE").is_some(){
                                eprintln!("security chase: RunningMan duration={duration} offboard={offboard_duration}");
                            }
                        }
                    }
                    Ok(())
                }
            }
            _=>Err(format!("unexpected event at chase registration boundary: {event:?}")),
        }
    }
    fn suppress_chaser_mood(&mut self,id:TargetableId)->Result<bool,String>{
        if id.kind!=1||!id.is_valid(){return Err("native ChaseMan pedestrian assertion".into());}
        let entity=if id.id as u32==self.entity.id {Some(self.entity)} else {
            self.others.iter().find(|(key,_)|*key==id.id as u32).map(|(_,e)|&**e)
        }.ok_or("native ChaseMan pedestrian absent")?;
        crate::hermes_game_sim_queue::security_message_enabled(entity)
    }
    fn local_capture(&mut self)->Result<Option<bool>,String>{
        let Some(actor)=self.local.try_local_actor().map_err(|e|e.to_string())? else{return Ok(None);};
        let prefix=actor.prefix().map_err(|e|e.to_string())?;
        Ok(Some(!self.local_chase_forced && prefix.chasee.as_ref().ok_or("local Actor Chasee absent")?.group_end_reason==1))
    }
}
impl SkaterManagerLocalIdentity {
    /// ChaseMan.ShowHUDChaseEndMessage82713cc8. The original global Math
    /// stream is consumed even for the unconditional ID_CHASE_OVER case.
    pub(crate) fn send_chase_end_message(&self,captured:bool,tazed:bool)->Result<(),String>{
        let word=crate::native_skater_random::rw_math::global_next().map_err(|e|format!("chase message random: {e:?}"))?;
        let chance=(word%101) as f32*0.01_f32;
        let key=if !captured{"ID_CHASE_OVER"}else if tazed{
            if chance<0.5{"ID_CHASE_TAZER_1"}else{"ID_CHASE_TAZER_2"}
        }else if chance<0.25{"ID_CHASE_CAUGHT_1"}else if chance<0.5{"ID_CHASE_CAUGHT_2"}
        else if chance<0.75{"ID_CHASE_CAUGHT_3"}else{"ID_CHASE_CAUGHT_4"};
        self.backend().send_tricks_command(crate::ui::trick_backend::BackendCommand::ShowCentralMessage(key))
    }
}
impl FrameServices<'_>{
    pub(crate) fn register_local_chaser(&mut self,entity:&Entity,id:TargetableId,add:bool)->Result<(),String>{
        let local=self.world.local_skater_identity.as_ref().ok_or("chase SkaterManager absent")?;
        let local_chase_forced=self.chase.are_local_chases_forced_to_terminate();
        let mut services=Registration{entity,others:self.others,local,cvars:self.cvars,events:self.game_sim_events,local_chase_forced};
        if add {self.chase.register_chaser(id,&mut services)} else {self.chase.unregister_chaser(id,&mut services)}
    }
}
