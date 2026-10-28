//! Actual MoodMan/Pedestrian ambient delivery and ProcessNewMoodEntry82e9f010.
use super::FrameServices;
use crate::{hermes_game_sim_queue::MoodEvent, living_world_pedestrian_entity::PedestrianConstruction,
    living_world_pedestrian_state::MoodEntry, living_world_chase::TargetableId,
    reckoning_numeric::{HostReckoningNumeric,ReckoningNumeric},static_ground_pool::StaticGroundPool};

impl FrameServices<'_> {
    /// Direct target virtual ProcessNewMoodEntry (for example Greet.Begin
    /// 82707d48). Reborrow the target as the active entity while retaining the
    /// caller among peers, so category/prerequisite lookups see both owners.
    pub(crate) fn receive_direct_mood(&mut self,entity:&mut PedestrianConstruction,
        target:TargetableId,event:&MoodEvent,ground:&StaticGroundPool)->Result<bool,String>{
        if target.kind!=1{return Err("direct pedestrian mood target is not a pedestrian".into());}
        let local=self.world.local_skater_identity.as_ref().ok_or("direct mood SkaterManager absent")?
            .try_local_index().map_err(|e|format!("direct mood local index: {e:?}"))?;
        if target.id as u32==entity.id{return self.receive_ambient_mood(entity,event,local,ground);}
        let index=self.others.iter().position(|(id,_)|*id==target.id as u32)
            .ok_or("direct mood target disappeared")?;
        let caller_id=self.entity_id;
        std::mem::swap(entity,&mut *self.others[index].1);
        self.others[index].0=caller_id;
        self.entity_id=target.id as u32;
        let result=self.receive_ambient_mood(entity,event,local,ground);
        std::mem::swap(entity,&mut *self.others[index].1);
        self.others[index].0=target.id as u32;
        self.entity_id=caller_id;
        result
    }
    /// PedestrianPerception.CanDetectMoodEvent82e740f8. Debug drawing has no
    /// influence on detection; the existing perception owner performs LOS.
    pub(crate) fn can_detect_mood_event(&mut self,entity:&PedestrianConstruction,event_type:i32,
        position:[f32;4],audibility_factor:f32,visibility_factor:f32,ground:&StaticGroundPool)
        ->Result<bool,String>{
        let audible=self.mood.audibility_distance(event_type)?*audibility_factor;
        let visible=self.mood.visibility_distance(event_type)?*visibility_factor;
        Ok(entity.perception.can_detect(entity.perception_centre()?,position,audible,visible,
            self.world,ground,&mut crate::collision_line_numeric::HostCollisionLineNumeric)?.detected())
    }
    /// Native targetable category methods: Actor825f9a30 (IsLocalPlayer),
    /// Pedestrian82e6ad90, Vehicle82d1d068 and DMO82d36290.
    pub(crate) fn target_category_key(&self,entity:&PedestrianConstruction,target:TargetableId,
        local_player_index:i32)->Result<u64,String>{
        if !self.target_exists(entity,target)? {return Ok(0);}
        Ok(match target.kind {
            0=>skate2_formats::attrib::string_hash64(if target.id==local_player_index {"skater"}else{"ai_skater"}),
            1=>{let target=if target.id as u32==entity.id {entity}else{self.pedestrian(target).ok_or("mood target vanished")?};
                target.core.as_ref().ok_or("mood target core absent")?.entity_key},
            2=>skate2_formats::attrib::string_hash64("dynamicobject"),
            3=>skate2_formats::attrib::string_hash64("vehicle"),
            _=>unreachable!("target_exists validates type"),
        })
    }
    /// EvaluateEventPrerequisite82ce7bb0. Numeric predicates use independent
    /// scalar/boolean accumulators, including ordered float comparisons.
    pub(crate) fn mood_prerequisite(&self,entity:&PedestrianConstruction,position:[f32;4],victim_self:bool,
        kind:i32,operator:i32,mut value:f32)->Result<bool,String>{
        let state=entity.state.as_ref().ok_or("mood prerequisite state absent")?;
        let mut flag=false;let mut scalar=0.;
        match kind {
            1=>flag=entity.plugin.is_some(),
            2=>flag=state.flags[1]&1!=0,
            3=>{value*=value;
                let centre=entity.core.as_ref().ok_or("mood prerequisite core absent")?.transform[3];
                let delta=std::array::from_fn(|i|centre[i]-position[i]);
                scalar=HostReckoningNumeric.dot3(delta,delta).map_err(|e|format!("mood distance: {e:?}"))?;},
            4=>flag=self.with_chasee(entity,state.chaser.chasee,|target|target.is_some())?,
            5=>flag=self.patrol_zone_contains(entity,position,false)?,
            6=>flag=victim_self,
            7=>if self.with_chasee(entity,state.chaser.chasee,|target|target.is_some())? {
                if let Some(actor)=self.actor(state.chaser.chasee)? {
                    let prefix=actor.prefix().map_err(|e|e.to_string())?;
                    flag=self.protector_exists(entity,prefix.fields().ok_or("mood protectee fields absent")?.protector);
                }
            },
            8=>flag=state.wants[10].active,
            _=>return Err(format!("native mood prerequisite assertion82ce7e70: {kind}")),
        }
        match operator {0=>Ok(flag),1=>Ok(!flag),2=>Ok(scalar>value),3=>Ok(scalar<value),
            _=>Err(format!("native mood prerequisite operator assertion82ce7f30: {operator}"))}
    }
    /// Pedestrian.ReceiveAmbientMoodEventMsg82e9f468 constructs the same input
    /// as direct ProcessNewMoodEntry82e9f510, then delegates to state processing.
    pub(crate) fn receive_ambient_mood(&mut self,entity:&mut PedestrianConstruction,event:&MoodEvent,
        local_player_index:i32,ground:&StaticGroundPool)->Result<bool,String>{
        if self.cvars.try_pedestrian_moods_suppressed().map_err(|e|format!("mood cvar: {e:?}"))? {return Ok(false);}
        if !self.mood.reactions_allowed(){return Ok(false);}
        let position=event.position.map(f32::from_bits);
        let mut incoming=MoodEntry{probability_timers:Vec::new(),position:[position[0],position[1],position[2]],
            perpetrator:event.perpetrator,victim:event.victim,event_type:event.event_type,
            exposure:0.,occurrences:0,cooldown:0.,since_refresh:0.};
        let state=entity.state.as_ref().ok_or("mood AI state absent")?;
        if state.flags[2]&8!=0 || state.is_mood_suppressed(&incoming){return Ok(false);}
        let perpetrator=self.target_category_key(entity,event.perpetrator,local_player_index)?;
        let self_id=TargetableId{kind:1,id:entity.id as i32};
        let victim_self=self_id==event.victim;
        let witness=entity.core.as_ref().ok_or("mood core absent")?.entity_key;
        let cares=|kind|self.mood.entity_cares_about_event(self.vault,witness,perpetrator,kind,
            |prereq,op,value|self.mood_prerequisite(entity,position,victim_self,prereq,op,value));
        if !cares(incoming.event_type)? {
            if incoming.event_type!=10 || victim_self{return Ok(false);}
            incoming.event_type=11;
            if !cares(incoming.event_type)?{return Ok(false);}
        }
        if !self.mood.is_local(incoming.event_type)? && event.perpetrator==self_id{return Ok(false);}
        if !self.can_detect_mood_event(entity,incoming.event_type,position,
            f32::from_bits(event.loudness_distance_factor),f32::from_bits(event.visibility_distance_factor),ground)?{return Ok(false);}
        incoming.exposure=self.mood.update_rate(incoming.event_type)?;
        incoming.occurrences=1;
        let state=entity.state.as_mut().ok_or("mood AI state absent")?;
        if state.merge_mood(&incoming){return Ok(true);}
        state.add_new_mood(&incoming,|kind|self.mood.priority(kind))
    }
}
