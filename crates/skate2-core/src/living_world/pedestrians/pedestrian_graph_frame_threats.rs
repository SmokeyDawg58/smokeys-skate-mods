//! UpdateThreats82e9aef0 / UpdateZoneInterests82e6fa78 over retained frame owners.
use super::FrameServices;
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    living_world_chase::TargetableId, hermes_game_sim_queue::MoodEvent,
    static_ground_pool::StaticGroundPool,
    reckoning_numeric::{HostReckoningNumeric,ReckoningNumeric}};

impl FrameServices<'_> {
    pub(crate) fn patrol_zone_valid(&self,entity:&PedestrianConstruction)->bool{
        // BigBlack owns LWCircularPatrolZone, whose validity virtual returns true.
        entity.big_black.is_some() || entity.security_guard.as_ref().is_some_and(|guard|guard.zone.is_valid())
    }
    /// LWPatrolZoneOwner82e6a330/82e6a410 dispatch through the concrete zone.
    pub(crate) fn patrol_zone_contains(&self,entity:&PedestrianConstruction,position:[f32;4],
        include_buffer:bool)->Result<bool,String>{
        if let Some(data)=entity.big_black.as_ref(){
            // LWCircularPatrolZone82e4d040/82e4d130: full XYZ, inclusive radius.
            // The second radius is the outer boundary, not extra buffer width.
            let radius=if include_buffer{data.maintain_radius}else{data.zone_radius};
            let delta=std::array::from_fn(|i|position[i]-data.zone_centre[i]);
            let squared=HostReckoningNumeric.dot3(delta,delta).map_err(|e|format!("protection zone: {e:?}"))?;
            return Ok(radius*radius>=squared);
        }
        let Some(guard)=entity.security_guard.as_ref().filter(|guard|guard.zone.is_valid()) else{return Ok(false);};
        (if include_buffer{guard.zone.contains_boundary_or_buffer(self.painter,xyz(position))}
            else{guard.zone.contains(self.painter,xyz(position))}).map_err(|e|e.to_string())
    }
    /// Native snapshots the two due flags and own-zone state before any mood
    /// callbacks. Deliveries are immediate, so later callbacks see prior moods.
    pub(crate) fn update_threats(&mut self,entity:&mut PedestrianConstruction,
        local_player_index:i32,ground:&StaticGroundPool)->Result<(),String>{
        let self_id=TargetableId{kind:1,id:entity.id as i32};
        let state=entity.state.as_ref().ok_or("threat AI state absent")?;
        let presence_due=state.timer(25)<=0.;
        let buffer_due=state.timer(26)<=0.;
        let valid_zone=self.patrol_zone_valid(entity);
        let position=entity.core.as_ref().ok_or("threat core absent")?.transform[3];
        let own_zone=self.patrol_zone_contains(entity,position,false)?;
        if presence_due {
            // Raw82e9b064 passes20, not the roster's40-member capacity.
            let nearby=self.pedestrian_spatial.find_near_numeric([position[0],position[2]],35.,20,
                &mut HostReckoningNumeric).map_err(|e|format!("threat pedestrian KD: {e:?}"))?;
            for (id,_) in nearby {
                let id=id as u32;
                if id==entity.id {continue;}
                let target=TargetableId{kind:1,id:id as i32};
                // Source checks CommonCoreDataProxy.IsInitialized twice around
                // the native assertion; it does not require renderability.
                self.pedestrian(target).ok_or("threat native missing pedestrian assertion")?
                    .core.as_ref().ok_or("threat native uninitialized pedestrian assertion")?;
                self.deliver_threat(entity,8,target,self_id,local_player_index,ground)?;
            }
        }
        if presence_due || buffer_due {
            for index in 0..self.actors.len() {
                let target={
                    let prefix=self.actors[index].prefix().map_err(|e|e.to_string())?;
                    TargetableId{kind:0,id:prefix.fields().ok_or("threat Actor fields absent")?.player_index as i32}
                };
                let actor_position=self.target_position(entity,target)?.ok_or("threat Actor position absent")?;
                // Native uses full XYZ and a strict radius test for actors.
                let position=entity.core.as_ref().ok_or("threat core absent")?.transform[3];
                let delta=std::array::from_fn(|i|position[i]-actor_position[i]);
                let squared=HostReckoningNumeric.dot3(delta,delta).map_err(|e|format!("threat Actor distance: {e:?}"))?;
                if !(1225.>squared) {continue;}
                if presence_due {self.deliver_threat(entity,8,target,self_id,local_player_index,ground)?;}
                if buffer_due && valid_zone && !own_zone
                    && entity.state.as_ref().ok_or("threat AI state absent")?.timer(18)>0. {
                    // Re-read after presence processing: ProcessNewMoodEntry
                    // is allowed to mutate retained state before this branch.
                    let position=self.target_position(entity,target)?.ok_or("threat buffer Actor position absent")?;
                    if self.patrol_zone_contains(entity,position,true)? {
                        self.deliver_threat(entity,9,target,self_id,local_player_index,ground)?;
                    }
                }
            }
        }
        // Both authored categories are read even when neither timer is due.
        // Preserve reads-before-writes and source prefix on checked failure.
        let presence_rate=self.mood.update_rate(8)?;
        let buffer_rate=self.mood.update_rate(9)?;
        let state=entity.state.as_mut().ok_or("threat AI state absent")?;
        if presence_due {state.set_timer(25,presence_rate).map_err(|e|format!("threat presence timer: {e:?}"))?;}
        if buffer_due {state.set_timer(26,buffer_rate).map_err(|e|format!("threat patrol timer: {e:?}"))?;}
        Ok(())
    }
    fn deliver_threat(&mut self,entity:&mut PedestrianConstruction,event_type:i32,
        perpetrator:TargetableId,victim:TargetableId,local_player_index:i32,ground:&StaticGroundPool)
        ->Result<(),String>{
        let position=self.target_centre_of_mass(entity,perpetrator)?
            .ok_or("threat target visibility point absent")?;
        let event=MoodEvent{event_type,position:position.map(f32::to_bits),perpetrator,victim,
            loudness_distance_factor:1_f32.to_bits(),visibility_distance_factor:1_f32.to_bits()};
        self.receive_ambient_mood(entity,&event,local_player_index,ground)?;
        Ok(())
    }

    pub(crate) fn update_zone_interests(&mut self,entity:&mut PedestrianConstruction)->Result<(),String>{
        if entity.big_black.is_some(){return self.big_black_zone_interests(entity);}
        if !self.patrol_zone_valid(entity) {return Ok(());}
        let state=entity.state.as_ref().ok_or("zone interest AI state absent")?;
        let chasee=state.chaser.chasee;
        let interest=if self.with_chasee(entity,chasee,|target|target.is_some())? {
            Some(chasee)
        }else if state.wants[7].active {
            let target=state.wants[7].target;
            if !self.target_exists(entity,target)? {return Err("zone interest native null targetable dereference".into());}
            Some(target)
        }else{None};
        // Native writes the entity pointer before fetching its position. Point
        // remains unchanged when no interest exists or it leaves the buffer.
        entity.patrol_entity_of_interest=interest;
        if let Some(target)=interest {
            let position=self.target_position(entity,target)?.ok_or("zone interest entity position absent")?;
            if self.patrol_zone_contains(entity,position,true)? {
                entity.patrol_point_of_interest=position;
            }
        }
        Ok(())
    }
}
fn xyz(value:[f32;4])->[f32;3]{[value[0],value[1],value[2]]}
