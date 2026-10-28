//! Original chase leaves over the sole entity, Chasee and GameSim owners.
use super::FrameServices;
use crate::{living_world_pedestrian_entity::PedestrianConstruction as Entity,
    living_world_chase::TargetableId,hermes_game_sim_queue::{LivingWorldMessage,MoodEvent},
    pedestrian_graph::behaviour_binding::Phase,living_world_guard_graph::GuardGraphServices};
impl FrameServices<'_>{
    pub(crate) fn dispatch_chase_callback(&mut self,entity:&mut Entity,occurrence:u16,
        phase:Phase,dt:f32)->Result<bool,String>{
        let declaration=crate::pedestrian_graph_data::BEHAVIOURS.get(occurrence as usize).ok_or("chase occurrence absent")?;
        let state=entity.state.as_ref().ok_or("chase AI state absent")?;
        let chasee=state.chaser.chasee;
        let id=TargetableId{kind:1,id:entity.id as i32};
        match (declaration.name,phase){
            ("InterceptChasee",Phase::Begin)=>{
                //82709290 exactly shares the Block/Pursue navigation prefix.
                self.with_navigation(entity,Some(crate::living_world_pedestrian_entity::targeting::TargetIteratorKind::Chase),
                    |entity,facts,world,numeric|entity.begin_chase_navigation(facts,world,numeric))?;
            }
            ("InterceptChasee",Phase::Update)=>self.intercept_chasee(entity,chasee,dt)?,
            ("LostChasee",Phase::Update)=>self.lost_chasee(entity,chasee)?,
            ("LostChasee",Phase::End)=>entity.navigation_intent.explicit_direction=None,
            ("UpdateBlockPrediction",Phase::Update)=>self.update_block_prediction(entity,chasee)?,
            ("ChaserEvaluateWhoToTakeDown",Phase::Update)=>{
                //82709e58 -> ILWTakeDownSource.SetTakeDownSinkId82e7aa50.
                let target=if self.with_chasee(entity,chasee,|s|s.is_some())?{chasee}else{TargetableId::INVALID};
                entity.state.as_mut().unwrap().take_down_sink=target;
            }
            ("GiveUpBeingPrimaryChaser",Phase::Begin)=>{
                //826ff270 passes the default invalid optional replacement;
                //InternalChangePrimaryChaser82de4e80 chooses index1.
                if self.with_chasee(entity,chasee,|s|s.is_some_and(|s|s.chasers.len()>=2&&s.chasers[0].id==id))?{
                    let time=crate::living_world_guard_graph::binding::attribute(occurrence,"timeout")?.ok_or("native primary timeout attribute absent")?.parse::<f32>().map_err(|e|e.to_string())?;
                    let state=entity.state.as_mut().unwrap();let previous=state.timer(29);
                    state.set_timer(29,if time-previous>=0.{time}else{previous}).map_err(|e|e.to_string())?;
                    self.with_chasee_mut(entity,chasee,|s|s.ok_or("primary Chasee disappeared")?
                        .finish_primary_switch(id).map_err(|e|e.to_string()))??;
                }
            }
            ("TellChaseeToFlee",Phase::Begin)=>{
                //8270b628 queries ILWMoodable on the actual chasee. Actor,
                //Vehicle and DMO have no pedestrian Moodable interface.
                if chasee.kind==1&&self.with_chasee(entity,chasee,|s|s.is_some())?{
                    let event=self.chase_mood_event(entity,17,id,chasee)?;
                    let ground=self.ground.ok_or("direct chase mood requires StaticGroundPool")?;
                    self.receive_direct_mood(entity,chasee,&event,ground)?;
                }
            }
            ("EmotionalResponseToGivingWarning",Phase::Begin)=>{
                //827122c8 is ambient event12 with self as both participants.
                let event=self.chase_mood_event(entity,12,id,id)?;
                self.game_sim_events.send(LivingWorldMessage::Mood(event))?;
            }
            ("SendChaseStateMessage",Phase::Begin)=>{
                let name=crate::living_world_guard_graph::binding::attribute(occurrence,"stateName")?.ok_or("native chase stateName assertion")?;
                let value=if name.eq_ignore_ascii_case("warn"){0}else if name.eq_ignore_ascii_case("chase"){1}
                    else if name.eq_ignore_ascii_case("tired"){2}else if name.eq_ignore_ascii_case("giveup"){3}else{4};
                let exists=self.with_chasee(entity,chasee,|s|s.is_some())?;
                let send=if exists{self.is_local_player(chasee,entity)?}else if value==0{
                    //PDB wants base412 + entry2*12 => target436/flags444.
                    let want=entity.state.as_ref().unwrap().wants[2];
                    want.active&&self.is_local_player(want.target,entity)?
                }else{false};
                if send{self.game_sim_events.send(LivingWorldMessage::ChaserState{entity:entity.id,state:value})?;}
            }
            _=>return Ok(false),
        }
        Ok(true)
    }
    fn chase_mood_event(&self,entity:&Entity,event_type:i32,perpetrator:TargetableId,victim:TargetableId)->Result<MoodEvent,String>{
        let position=self.target_position(entity,TargetableId{kind:1,id:entity.id as i32})?.ok_or("chase mood source absent")?;
        Ok(MoodEvent{event_type,position:position.map(f32::to_bits),perpetrator,victim,
            loudness_distance_factor:1_f32.to_bits(),visibility_distance_factor:1_f32.to_bits()})
    }
    /// InterceptChasee.Update82709388; XZ intercept reuses the native solver,
    /// while the eventual navigation target retains the greater source Y.
    fn intercept_chasee(&mut self,entity:&mut Entity,chasee:TargetableId,dt:f32)->Result<(),String>{
        if !self.with_chasee(entity,chasee,|s|s.is_some())?{return Ok(());}
        let own=self.target_position(entity,TargetableId{kind:1,id:entity.id as i32})?.ok_or("intercept source absent")?;
        let target=self.target_position(entity,chasee)?.ok_or("intercept Chasee position absent")?;
        let speed=entity.max_chase_speed()?;
        let mut destination=target;
        let tolerance=self.cvars.try_chase_intercept_y_tolerance().map_err(|e|format!("intercept tolerance: {e:?}"))?;
        if tolerance>=(target[1]-own[1]).abs(){
            let height=if target[1]>own[1]{target[1]}else{own[1]};
            let parameters=&entity.resources.as_ref().ok_or("intercept chase resources absent")?.chase;
            if parameters.is_null(){return Err("native GetMaxInterceptTime collection assertion".into());}
            let max_time=f32::from_bits(parameters.try_scalar_word(0x47c9_6c54_4337_ff32,0).map_err(|e|e.to_string())?.unwrap_or(0));
            let velocity=self.target_velocity(entity,chasee)?.ok_or("intercept Chasee velocity absent")?;
            let result=crate::living_world_chase::prediction::intercept_target(
                [target[0],0.,target[2]],[velocity[0],0.,velocity[2]],speed,
                [own[0],0.,own[2]],speed,max_time,self.chase.min_behind_intercept_angle()?,
                &mut crate::reckoning_numeric::HostReckoningNumeric).map_err(|e|format!("intercept solver: {e:?}"))?;
            if result.possible{destination=[result.point[0],height,result.point[2],0.];}
        }
        entity.navigation_mut()?.seek_speed=speed;
        self.with_navigation(entity,Some(crate::living_world_pedestrian_entity::targeting::TargetIteratorKind::Chase),
            |entity,facts,world,numeric|entity.target_explicit(destination,crate::pedestrian_navigation::NavigationTargetMode::Explicit,facts,world,numeric))?;
        //ExertChaser82e6fcf0 adds dt without capping or a second timer owner.
        entity.state.as_mut().ok_or("intercept AI state absent")?.exhaustion+=dt;
        Ok(())
    }
    /// LostChasee.Update82709af8 uses known position solely for its distance
    /// test; navigation still targets the Chasee's actual current position.
    fn lost_chasee(&mut self,entity:&mut Entity,chasee:TargetableId)->Result<(),String>{
        if !self.with_chasee(entity,chasee,|s|s.is_some())?||!self.target_exists(entity,chasee)?{return Ok(());}
        let id=TargetableId{kind:1,id:entity.id as i32};
        let position=self.target_position(entity,id)?.ok_or("lost chase source absent")?;
        let velocity=self.target_velocity(entity,id)?.ok_or("lost chase velocity absent")?;
        let mut remembered=[position[0],0.,position[2],0.];
        let mut direction=[velocity[0],0.,velocity[2],0.];
        if let Some(knowledge)=entity.state.as_ref().ok_or("lost chase state absent")?.knowledge.iter().find(|k|k.target==chasee){
            remembered=[knowledge.position[0],knowledge.position[1],knowledge.position[2],0.];
            direction=[knowledge.velocity[0],knowledge.velocity[1],knowledge.velocity[2],0.];
        }
        let target=self.target_position(entity,chasee)?.ok_or("lost chase target absent")?;
        self.with_navigation(entity,Some(crate::living_world_pedestrian_entity::targeting::TargetIteratorKind::Chase),
            |entity,facts,world,numeric|entity.target_explicit(target,crate::pedestrian_navigation::NavigationTargetMode::Explicit,facts,world,numeric))?;
        let delta=std::array::from_fn(|i|remembered[i]-position[i]);
        use crate::reckoning_numeric::ReckoningNumeric;
        let distance=crate::reckoning_numeric::HostReckoningNumeric.dot3(delta,delta).map_err(|e|format!("lost chase distance: {e:?}"))?;
        if distance>1.{let speed=entity.max_chase_speed()?;entity.navigation_mut()?.seek_speed=speed;}
        else{entity.navigation_mut()?.seek_speed=0.;entity.navigation_intent.explicit_direction=Some(direction);}
        Ok(())
    }
    /// UpdateBlockPrediction827096a0 -> Chaser82de3220. The original keeps
    /// its unnormalized negative delta separately from NormalizeXZSafe's
    /// result; retain that distinction in the recommended-offset projection.
    fn update_block_prediction(&mut self,entity:&mut Entity,chasee:TargetableId)->Result<(),String>{
        use crate::reckoning_numeric::{HostReckoningNumeric,ReckoningNumeric,try_normalize_safe,try_unsigned_angle};
        entity.chaser_should_block=false;
        if !self.with_chasee(entity,chasee,|s|s.is_some())?{return Ok(());}
        let own=self.target_position(entity,TargetableId{kind:1,id:entity.id as i32})?.ok_or("block chaser position absent")?;
        let target=self.target_position(entity,chasee)?.ok_or("block chasee position absent")?;
        let delta=[target[0]-own[0],0.,target[2]-own[2],0.];
        let squared=HostReckoningNumeric.dot3(delta,delta).map_err(|e|format!("block distance: {e:?}"))?;
        let maximum=chase_parameter(entity,0x8bae_6f43_9f2b_83e8)?;
        if !(maximum*maximum>squared){return Ok(());}
        let radius=self.with_chasee(entity,chasee,|s|s.map(|s|s.block_radius))?.ok_or("block chasee disappeared")?;
        let arrival=chase_parameter(entity,0xf3e7_21eb_a7f9_f040)?;
        let radius=radius+arrival;
        let id=TargetableId{kind:1,id:entity.id as i32};
        //Native BGE is branch-if-not-LT, including unordered values.
        if squared-radius*radius<0.{
                entity.chaser_should_block=true;
                let p=self.recommended_chaser_position(chasee,id,entity)?;
                entity.chaser_block_position=[p[0],p[1],p[2],0.];
                return Ok(());
        }
        let normalized=try_normalize_safe(delta,[0.;4],&mut HostReckoningNumeric).map_err(|e|format!("block normalize: {e:?}"))?;
        let inverse=delta.map(|v|v*(-1.));
        let velocity=self.target_velocity(entity,chasee)?.ok_or("block chasee velocity absent")?;
        let velocity=[velocity[0],0.,velocity[2],0.];
        let approaching=HostReckoningNumeric.dot3(normalized,velocity).map_err(|e|format!("block speed: {e:?}"))?*(-1.);
        if !(approaching>chase_parameter(entity,0x670a_0b1f_a8a3_fe21)?){return Ok(());}
        let angle=try_unsigned_angle(velocity,inverse,&mut HostReckoningNumeric).map_err(|e|format!("block angle: {e:?}"))?;
        if !(angle<chase_parameter(entity,0xe48b_ccf9_ef47_3450)?*f32::from_bits(0x3c8e_fa35)){return Ok(());}
        //IChasee slot68 is GetChaserInterceptDirection82de2628, loading
        //ChaserInfo+32. A missing entry returns zero; a present entry must
        //have had its original formation producer initialize that field.
        let offset=self.with_chasee(entity,chasee,|s|s.and_then(|s|s.chasers.iter().find(|c|c.id==id).map(|c|c.intercept_direction)))?
            .map(|direction|direction.ok_or("chaser intercept direction has no native producer"))
            .transpose()?.unwrap_or([0.;3]);
        let projection=HostReckoningNumeric.dot3(inverse,[offset[0],offset[1],offset[2],0.]).map_err(|e|format!("block projection: {e:?}"))?;
        if !(projection>0.){
            let p=self.recommended_chaser_position(chasee,id,entity)?;
            entity.chaser_block_position=[p[0],p[1],p[2],0.];
        }else{
            //Separate native VMUL/VADD, not an FMA.
            let offset=[offset[0],offset[1],offset[2],0.];
            entity.chaser_block_position=std::array::from_fn(|i|target[i]+offset[i]*projection);
        }
        entity.chaser_should_block=true;
        Ok(())
    }
}
fn chase_parameter(entity:&Entity,key:u64)->Result<f32,String>{
    let instance=&entity.resources.as_ref().ok_or("chaser parameters absent")?.chase;
    if instance.is_null(){return Err("native Pedestrian chase-parameter assertion".into());}
    Ok(f32::from_bits(instance.try_scalar_word(key,0).map_err(|e|e.to_string())?.unwrap_or(0)))
}
