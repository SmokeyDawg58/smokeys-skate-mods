//! FindCurrentMoodResults82e98be0 and UpdateNeedsAndWants82e9e8f0.
use super::FrameServices;
use crate::{living_world_pedestrian_entity::PedestrianConstruction,living_world_chase::TargetableId,
    pedestrian_mood::{self,layout},attribute_runtime::AttributeRuntimeInstance,
    native_skater_random::rw_math};

struct Selection { mood:usize,target_kind:u32,cooldown:f32,probability:f32,index:u32,retry:f32,reset:bool,results:Vec<u32> }
fn word(a:&AttributeRuntimeInstance,offset:usize)->Result<u32,String>{Ok(u32::from_be_bytes(layout(a,offset,4)?.try_into().expect("mood result word")))}
fn flag(a:&AttributeRuntimeInstance,offset:usize)->Result<bool,String>{Ok(layout(a,offset,1)?[0]!=0)}
fn results(a:&AttributeRuntimeInstance)->Result<Vec<u32>,String>{
    // Generated MoodResults82e70e10: embedded Private header+72, count+2,
    // contiguous enum entries+80. Use the shared checked layout projection.
    let count=u16::from_be_bytes(layout(a,74,2)?.try_into().expect("result count"));
    (0..count).map(|i|word(a,80+usize::from(i)*4)).collect()
}
impl FrameServices<'_> {
    /// Pedestrian.IsBusy82e6ebb8: wiped out, reacting/waiting, then plugin.
    pub(crate) fn pedestrian_busy(&self,entity:&PedestrianConstruction)->Result<bool,String>{
        let state=entity.state.as_ref().ok_or("busy pedestrian state absent")?;
        Ok(state.flags[1]&4!=0 || state.flags[2]&0x30!=0 || entity.plugin.is_some())
    }
    fn mood_target_busy(&self,entity:&PedestrianConstruction,target:TargetableId)->Result<bool,String>{
        match target.kind {
            // Raw825f9aa0 calls ITargetable+24, then negates IsWipedOut.
            0=>Ok(!self.target_wiped_out(entity,target)?.ok_or("mood Actor disappeared")?),
            1=>self.pedestrian_busy(if target.id as u32==entity.id {entity}else{self.pedestrian(target).ok_or("mood pedestrian disappeared")?}),
            2|3=>Ok(false), // Actual ITargetable vtables folded82784530.
            _=>Err("invalid mood target".into()),
        }
    }
    fn mood_target_onboard(&self,target:TargetableId)->Result<bool,String>{
        let Some(actor)=self.actor(target)? else{return Ok(false);};
        let allocation=actor.cached_physical_allocation().map_err(|e|format!("mood Actor allocation: {e:?}"))?;
        let output=allocation.owner().phys_out().lock().map_err(|_|"mood Actor output lock")?;
        let state=output.try_state_output().map_err(|e|format!("mood Actor state: {e:?}"))?;
        Ok(!state.is_offboard && !state.is_wiping_out)
    }
    fn find_current_mood_results(&self,entity:&PedestrianConstruction,local_player_index:i32)->Result<Option<Selection>,String>{
        let state=entity.state.as_ref().ok_or("mood results state absent")?;
        let witness=entity.core.as_ref().ok_or("mood results core absent")?.entity_key;
        let attrs=self.vault.instance(0x60d7_03c7_7cab_1631,witness);
        if attrs.is_null(){return Err("native mood entity collection assertion82e98c68".into());}
        let Some(mood)=attrs.try_ref_spec_instance(0x712e_5451_3736_63be).map_err(|e|e.to_string())? else{return Ok(None);};
        let self_id=TargetableId{kind:1,id:entity.id as i32};
        let mut least_priority=u32::MAX;let mut chosen=None;let mut last_index=0;
        for (mood_index,entry) in state.moods.iter().enumerate(){
            if !(entry.cooldown<=0.) {continue;}
            let category=self.mood.category_key(entry.event_type)?;
            let perp=self.target_category_key(entity,entry.perpetrator,local_player_index)?;
            let victim=self.target_category_key(entity,entry.victim,local_player_index)?;
            let reactions=state.knowledge.iter().find(|k|k.target==entry.perpetrator).map_or(0,|k|k.reaction_count);
            for index in 0..mood.try_length(pedestrian_mood::RESULT_LIST).map_err(|e|e.to_string())? {
                let priority=mood.try_array_field_data(pedestrian_mood::RESULT_LIST,index,pedestrian_mood::RESULT_ENTRY_TYPE,32,24..28)
                    .map_err(|e|e.to_string())?.map(|d|u32::from_be_bytes(d.bytes().try_into().expect("priority"))).unwrap_or(0);
                if priority>=least_priority {continue;}
                let key=mood.try_array_field_data(pedestrian_mood::RESULT_LIST,index,pedestrian_mood::RESULT_ENTRY_TYPE,32,8..16)
                    .map_err(|e|e.to_string())?.map(|d|u64::from_be_bytes(d.bytes().try_into().expect("result RefSpec"))).unwrap_or(0);
                let result=self.vault.instance(pedestrian_mood::RESULT_CLASS,key);
                let target_kind=word(&result,104)?;
                let target=match target_kind {0=>entry.perpetrator,1=>entry.victim,_=>self_id};
                if !self.target_exists(entity,target)? || word(&result,60)?!=category as u32 {continue;}
                if flag(&result,97)? && word(&result,36)?!=perp as u32 {continue;}
                if flag(&result,100)? && entry.exposure<f32::from_bits(word(&result,136)?) {continue;}
                if flag(&result,98)? && entry.occurrences<word(&result,120)? {continue;}
                if flag(&result,92)? && word(&result,12)?!=victim as u32 {continue;}
                if flag(&result,99)? && (state.flags[2]&2!=0)!=flag(&result,132)? {continue;}
                if flag(&result,93)? && self.mood_target_onboard(target)?!=flag(&result,101)? {continue;}
                if flag(&result,94)? && self.mood_target_busy(entity,target)?!=flag(&result,102)? {continue;}
                if flag(&result,95)? && (reactions<word(&result,124)? || reactions>word(&result,128)?) {continue;}
                if flag(&result,96)? && entry.probability_timers.iter().find(|(i,_)|*i==index).is_some_and(|(_,t)| !(*t<=0.)) {continue;}
                let position=self.target_position(entity,target)?.ok_or("mood result target disappeared")?;
                if !self.mood.entity_cares_about_event(self.vault,witness,perp,entry.event_type,
                    |kind,op,value|self.mood_prerequisite(entity,position,entry.victim==self_id,kind,op,value))? {continue;}
                // Raw82e99110/140 update priority and selected result index
                // BEFORE testing already-active wants, even for rejected wants.
                last_index=index;least_priority=priority;
                let list=results(&result)?;
                let mut inactive=true;
                for &want in &list {if state.wants.get(want as usize).ok_or("mood result want out of range")?.active {inactive=false;break;}}
                if !inactive {continue;}
                chosen=Some(Selection{mood:mood_index,target_kind,cooldown:f32::from_bits(word(&result,140)?),
                    probability:if flag(&result,96)?{f32::from_bits(word(&result,116)?)}else{0.},
                    index,retry:f32::from_bits(word(&result,112)?),reset:flag(&result,108)?,results:Vec::new()});
            }
        }
        if let Some(selected)=&mut chosen {
            let key=mood.try_array_field_data(pedestrian_mood::RESULT_LIST,last_index,pedestrian_mood::RESULT_ENTRY_TYPE,32,8..16)
                .map_err(|e|e.to_string())?.map(|d|u64::from_be_bytes(d.bytes().try_into().expect("result RefSpec"))).unwrap_or(0);
            selected.results=results(&self.vault.instance(pedestrian_mood::RESULT_CLASS,key))?;
            if selected.results.len()>=8{return Err("native mood result capacity assertion82e99314".into());}
        }
        Ok(chosen)
    }
    pub(crate) fn update_needs_and_wants(&mut self,entity:&mut PedestrianConstruction,local_player_index:i32)->Result<(),String>{
        if self.cvars.try_all_mood_behaviour_suppressed().map_err(|e|format!("mood behaviour cvar: {e:?}"))? {return Ok(());}
        if !self.mood.reactions_allowed(){return Ok(());}
        if entity.state.as_ref().ok_or("mood wants state absent")?.flags[2]&8!=0 || self.pedestrian_busy(entity)? {return Ok(());}
        if entity.state.as_ref().unwrap().moods.is_empty(){return Ok(());}
        let Some(selected)=self.find_current_mood_results(entity,local_player_index)? else{return Ok(());};
        let mut succeeds=true;
        // Raw82e9ea84..a9c: zero/unordered/negligible probability succeeds
        // without consuming RNG or adding a probability timer.
        if selected.probability.abs()>f32::from_bits(0x3400_0000) {
            succeeds=rw_math::bernoulli_choice(selected.probability,rw_math::global_next().map_err(|e|format!("mood random: {e:?}"))?);
            let entry=&mut entity.state.as_mut().unwrap().moods[selected.mood];
            if entry.probability_timers.len()>=3{return Err("mood probability timer fixed capacity3 exceeded".into());}
            entry.probability_timers.push((selected.index,selected.retry));
            if self.cvars.try_show_mood_random_rolls().map_err(|e|format!("mood debug cvar: {e:?}"))? {
                return Err("mood random debug overlay requires native line/text owner".into());
            }
        }
        if succeeds {
            let state=entity.state.as_mut().unwrap();state.flags[2]|=0x20;
            let entry=&mut state.moods[selected.mood];entry.cooldown=selected.cooldown;
            let target=match selected.target_kind {1=>entry.victim,2=>TargetableId{kind:1,id:entity.id as i32},_=>entry.perpetrator};
            for want in selected.results {
                if want==0 {continue;}
                let state=entity.state.as_mut().unwrap();
                state.set_want_active(want as usize,true).map_err(|e|e.to_string())?;
                state.wants[want as usize].explicit=selected.reset;
                state.wants[want as usize].target=target;
                self.know_about_entity(entity,target,30.)?;
            }
        }
        Ok(())
    }
}
