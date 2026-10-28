//! Guard leaf services borrowing the same live entity and navigation frame.
use super::FrameServices;
use crate::{living_world_pedestrian_entity::{PedestrianConstruction as Entity,targeting::TargetingWorld},
    living_world_chase::TargetableId,living_world_guard_graph::GuardGraphServices,
    reckoning_numeric::HostReckoningNumeric};
fn error(e:impl std::fmt::Debug)->String{format!("guard service: {e:?}")}
fn xyz(v:[f32;4])->[f32;3]{[v[0],v[1],v[2]]}
impl FrameServices<'_>{
    pub(crate) fn dispatch_guard_behaviour(&mut self,entity:&mut Entity,occurrence:u16,
        phase:crate::pedestrian_graph::behaviour_binding::Phase,dt:f32)->Result<bool,String>{
        use crate::living_world_guard_graph::{GuardBehaviour,GuardBehaviourContext,binding::GuardTuning};
        use crate::pedestrian_graph::behaviour_binding::Phase;
        let declaration=crate::pedestrian_graph_data::BEHAVIOURS.get(occurrence as usize).ok_or("guard occurrence absent")?;
        let mut tuning=GuardTuning{alert_time:None,patrol_interest_time:None,warn_interest_time:None};
        let scalar=|data:&crate::attribute_runtime::AttributeRuntimeInstance,key|->Result<f32,String>{
            if data.is_null(){return Err("native guard parameter collection assertion".into());}
            Ok(f32::from_bits(data.try_scalar_word(key,0).map_err(|e|e.to_string())?.unwrap_or(0)))
        };
        match declaration.name{
            "SetAlertTimer"=>tuning.alert_time=Some(scalar(&entity.resources.as_ref().ok_or("guard chase resources absent")?.chase,0xfa76_2b2e_a065_a6d7)?),
            "SetPatrolZoneMaintainInterestTimerOnBegin"=>tuning.patrol_interest_time=Some(scalar(entity.patrol_zone_parameters.as_ref().ok_or("guard patrol parameters absent")?,0x1f4e_7aea_9a02_da2a)?),
            "ChannelWarnWantTarget"=>tuning.warn_interest_time=Some(self.cvars.try_pedestrian_warn_interest_timer().map_err(error)?),
            _=>{}
        }
        let Some(behaviour)=GuardBehaviour::bind(occurrence,&tuning)? else{return Ok(false);};
        let mut context=GuardBehaviourContext{self_id:TargetableId{kind:1,id:entity.id as i32},
            is_security_guard:entity.security_guard.is_some(),entity,services:self};
        match phase{Phase::Begin=>context.begin(&behaviour)?,Phase::Update=>context.update(&behaviour,dt)?,Phase::End=>context.end(&behaviour)?}
        Ok(true)
    }
}
impl GuardGraphServices for FrameServices<'_>{
    fn targetable_exists(&mut self,target:TargetableId,entity:&mut Entity)->Result<bool,String>{self.target_exists(entity,target)}
    fn chasee_exists(&mut self,target:TargetableId,entity:&mut Entity)->Result<bool,String>{self.with_chasee(entity,target,|v|v.is_some())}
    fn is_local_player(&mut self,target:TargetableId,_entity:&mut Entity)->Result<bool,String>{
        let Some(actor)=self.actor(target)? else{return Ok(false);};
        self.world.local_skater_identity.as_ref().ok_or("guard local SkaterManager absent")?
            .try_is_local_player(||actor.prefix().map_err(error)?.fields().map(|f|f.player_index as i32)
                .ok_or_else(|| "guard Actor fields absent".to_owned())).map_err(error)
    }
    fn is_local_actor(&mut self,target:TargetableId,entity:&mut Entity)->Result<bool,String>{self.is_local_player(target,entity)}
    fn number_of_reactions(&mut self,target:TargetableId,entity:&mut Entity)->Result<i32,String>{
        Ok(entity.state.as_ref().ok_or("guard state absent")?.knowledge.iter().find(|k|k.target==target)
            .map_or(0,|k|k.reaction_count as i32))
    }
    fn entity_knowledge_suppressed(&self,_entity:&mut Entity)->Result<bool,String>{self.cvars.try_pedestrian_entity_knowledge_suppressed().map_err(error)}
    fn channel(&mut self,name:&str,entity:&mut Entity)->Result<(),String>{self.request_channel(entity,name)}
    fn speech_event(&mut self,event:u32,entity:&mut Entity)->Result<(),String>{
        //SendSpeechEvent.Begin826ed200 -> Pedestrian.SetAudioState82e94ca8.
        //The native call writes the existing StateOut field; audio playback
        //is consumed downstream, not a prerequisite for this setter.
        entity.state_out.state=event;
        Ok(())
    }
    fn chasee_primary_speech_event(&mut self,chasee:TargetableId,event:u32,entity:&mut Entity)->Result<(),String>{
        let primary=self.with_chasee(entity,chasee,|state|state.and_then(|s|s.chasers.first().map(|c|c.id)))?;
        //NewChasee.Begin82708f98..9048: GetPrimaryChaser(0), resolve its
        //actual entity, then SetAudioState15. Null pointers do nothing.
        if let Some(primary)=primary {
            if primary.kind==1 && primary.id as u32==entity.id {entity.state_out.state=event;}
            else if let Some(target)=self.pedestrian_mut(primary){target.state_out.state=event;}
        }
        Ok(())
    }
    fn chasee_remove_chaser(&mut self,chasee:TargetableId,chaser:TargetableId,entity:&mut Entity)->Result<(),String>{
        //82de4990..49f8: erase before unregister; clear end reason afterward.
        let removed=self.with_chasee_mut(entity,chasee,|state|
            state.is_some_and(|state|state.erase_chaser(chaser)))?;
        if removed && self.is_local_player(chasee,entity)? {
            self.register_local_chaser(entity,chaser,false)?;
        }
        self.with_chasee_mut(entity,chasee,|state|{if let Some(state)=state{state.finish_chaser_removal();}})
    }
    fn chasee_add_chaser(&mut self,chasee:TargetableId,chaser:TargetableId,entity:&mut Entity)->Result<bool,String>{
        // Original requires a real IChaser before examining chasee capacity.
        if !chaser.is_valid() || chaser.kind!=1 ||
            (chaser.id as u32!=entity.id && self.pedestrian(chaser).is_none()) {
            return Err("Chasee.AddChaser IChaser assertion82de4cc4".into());
        }
        let Some(chasee_position)=self.target_position(entity,chasee)? else{return Ok(false);};
        let Some(chaser_position)=self.target_position(entity,chaser)? else{return Ok(false);};
        let attributes=self.vault.instance(crate::living_world_chase::attributes::CHASE_CLASS,
            skate2_formats::attrib::string_hash64("global"));
        if attributes.is_null(){return Err("Chasee global parameters absent".into());}
        let max=crate::living_world_chase::attributes::max_chasers(&attributes)?;
        let added=self.with_chasee_mut(entity,chasee,|state|state.map_or(Ok(false),|state|
            state.add_chaser(chaser,max,xyz(chasee_position),xyz(chaser_position)).map_err(error)))??;
        if added && self.is_local_player(chasee,entity)? {
            self.register_local_chaser(entity,chaser,true)?;
        }
        Ok(added)
    }
    fn group_end_chase(&mut self,chasee:TargetableId,reason:i32,entity:&mut Entity)->Result<(),String>{
        self.with_chasee_mut(entity,chasee,|state|{
            state.ok_or("native group end requires chasee")?.group_end_reason=reason;Ok::<_,String>(())
        })?
    }
    fn show_chase_end_message(&mut self,captured:bool,tazed:bool,_entity:&mut Entity)->Result<(),String>{
        self.world.local_skater_identity.as_ref().ok_or("chase HUD SkaterManager absent")?.send_chase_end_message(captured,tazed)
    }
    fn set_nav_desired_speed(&mut self,speed:f32,entity:&mut Entity)->Result<(),String>{entity.navigation_mut()?.seek_speed=speed;Ok(())}
    fn clear_chasing_flag(&mut self,entity:&mut Entity)->Result<(),String>{entity.navigation_rig_enabled=false;Ok(())}
    fn set_chasing_flag(&mut self,chasing:bool,entity:&mut Entity)->Result<(),String>{entity.navigation_rig_enabled=chasing;Ok(())}
    fn begin_chase_navigation(&mut self,entity:&mut Entity)->Result<(),String>{
        let facts=entity.navigation_entity_facts(self.cvars)?;
        let mut world=TargetingWorld{dmos:self.dmos,mesh:self.mesh,rain:self.rain,random:self.random,avoid:self.avoid,
            cvars:self.cvars,props:self.props,flee_source:None,chasee_position:None};
        entity.begin_chase_navigation(&facts,&mut world,&mut HostReckoningNumeric)
    }
    fn chaser_position(&mut self,entity:&mut Entity)->Result<[f32;3],String>{Ok(xyz(entity.core.as_ref().ok_or("chaser core absent")?.transform[3]))}
    fn chaser_block_position(&mut self,entity:&mut Entity)->Result<[f32;3],String>{Ok(xyz(entity.chaser_block_position))}
    fn max_chase_speed(&mut self,entity:&mut Entity)->Result<f32,String>{entity.max_chase_speed()}
    fn chasee_position(&mut self,chasee:TargetableId,entity:&mut Entity)->Result<[f32;3],String>{
        self.target_position(entity,chasee)?.map(xyz).ok_or("native GetChaseePosition target absent".into())
    }
    fn recommended_chaser_position(&mut self,chasee:TargetableId,chaser:TargetableId,entity:&mut Entity)->Result<[f32;3],String>{
        if !chaser.is_valid(){return Err("native recommended chaser id assertion".into());}
        let offset=self.with_chasee(entity,chasee,|state|state.and_then(|s|s.chasers.iter().find(|c|c.id==chaser).map(|c|c.recommended_position)))?;
        //GetChaserRecommendedPosition82de23e0 returns zero when no entry.
        let Some(offset)=offset else{return Ok([0.;3]);};
        let position=self.chasee_position(chasee,entity)?;
        Ok(std::array::from_fn(|i|position[i]+offset[i]))
    }
    fn set_strafe_target(&mut self,target:[f32;3],entity:&mut Entity)->Result<(),String>{entity.navigation_mut()?.set_strafe_target([target[0],target[1],target[2],0.]);Ok(())}
    fn clear_strafe_target(&mut self,entity:&mut Entity)->Result<(),String>{entity.navigation_mut()?.use_strafe_target=false;Ok(())}
    fn target_explicit(&mut self,target:[f32;3],native_mode:u32,entity:&mut Entity)->Result<(),String>{
        use crate::pedestrian_navigation::NavigationTargetMode as Mode;
        let mode=match native_mode{1=>Mode::Explicit,2=>Mode::ExplicitNoSeekNoAstar,
            _=>return Err(format!("unexpected explicit navigation mode {native_mode}"))};
        let facts=entity.navigation_entity_facts(self.cvars)?;
        let mut world=TargetingWorld{dmos:self.dmos,mesh:self.mesh,rain:self.rain,random:self.random,avoid:self.avoid,
            cvars:self.cvars,props:self.props,flee_source:None,chasee_position:None};
        entity.target_explicit([target[0],target[1],target[2],0.],mode,&facts,&mut world,&mut HostReckoningNumeric)
    }
    fn set_returning_to_patrol_zone(&mut self,returning:bool,entity:&mut Entity)->Result<(),String>{entity.patrol_returning=returning;Ok(())}
    fn update_return_to_patrol_zone(&mut self,entity:&mut Entity)->Result<(),String>{
        let facts=entity.navigation_entity_facts(self.cvars)?;
        let arrived={
            let mut world=TargetingWorld{dmos:self.dmos,mesh:self.mesh,rain:self.rain,random:self.random,avoid:self.avoid,
                cvars:self.cvars,props:self.props,flee_source:None,chasee_position:None};
            entity.update_return_to_patrol_zone(&facts,&mut world,&mut HostReckoningNumeric)?
        };
        if arrived{
            let interest=entity.patrol_entity_of_interest.map(|id|self.target_position(entity,id)).transpose()?.flatten();
            let mut world=TargetingWorld{dmos:self.dmos,mesh:self.mesh,rain:self.rain,random:self.random,avoid:self.avoid,
                cvars:self.cvars,props:self.props,flee_source:None,chasee_position:None};
            entity.finish_return_to_patrol_zone(interest,&facts,&mut world,&mut HostReckoningNumeric)?;
        }
        Ok(())
    }
    fn set_alt_target_to_chasee_position(&mut self,chasee:TargetableId,entity:&mut Entity)->Result<(),String>{
        let position=self.target_position(entity,chasee)?.ok_or("alternate chase target absent")?;
        entity.set_chase_alternate_target(position,self.mesh,self.cvars)
    }
    fn register_warner(&mut self,id:TargetableId,_entity:&mut Entity)->Result<(),String>{self.chase.register_warner(id).map_err(error)}
    fn register_tazer(&mut self,id:TargetableId,_entity:&mut Entity)->Result<(),String>{self.chase.register_tazer(id).map_err(error)}
    fn unregister_tazer(&mut self,id:TargetableId,_entity:&mut Entity)->Result<(),String>{self.chase.unregister_tazer(id);Ok(())}
}
