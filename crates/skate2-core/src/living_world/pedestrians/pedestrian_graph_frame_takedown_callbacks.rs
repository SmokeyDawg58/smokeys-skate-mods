//! Takedown behavior callbacks on the existing pedestrian and target owners.
use super::FrameServices;
use crate::living_world_pedestrian_entity::PedestrianConstruction;
impl FrameServices<'_>{
    /// IgnoreTakedownTargetNavRigVolume Update8270b468 / End826ee5d0.
    pub(crate) fn ignore_takedown_nav_volume(&mut self,entity:&mut PedestrianConstruction,
        end:bool)->Result<(),String>{
        if end {entity.nav_rig_ignore_entity=None;return Ok(());}
        let target=entity.state.as_ref().ok_or("takedown state absent")?.take_down_sink;
        // GetTakeDownSink82e7aa28 resolves ITargetable through the same roster.
        // A missing target leaves the previous ignore volume unchanged.
        if self.target_exists(entity,target)?{entity.nav_rig_ignore_entity=Some(target);}
        Ok(())
    }
}

impl FrameServices<'_>{
    /// TakedownTauntVictim.Begin8270b100 / End826ee3c8.
    pub(crate) fn takedown_taunt(&mut self,entity:&mut PedestrianConstruction,end:bool)->Result<(),String>{
        use crate::{living_world_guard_graph::GuardGraphServices,
            pedestrian_adjust_owners::HeadTrackingMode,pedestrian_intent_packet::IntentStage,
            play_animation::FastString30,reckoning_numeric::{HostReckoningNumeric,try_normalize_unsafe}};
        let hash=skate2_formats::hash::sk8_hash_value_32(b"SGIntent");
        if end{
            entity.head_tracking.as_mut().ok_or("taunt head tracking absent")?.pop();
            entity.navigation_intent.explicit_direction=None;
            let state=entity.state.as_mut().ok_or("taunt state absent")?;
            entity.intent_queue.as_mut().ok_or("taunt intent queue absent")?.release_monitored(state,hash)?;
            state.set_want_active(3,false).map_err(|e|e.to_string())?;
            return Ok(());
        }
        let want=entity.state.as_ref().ok_or("taunt state absent")?.wants[3];
        if !want.active||!self.target_exists(entity,want.target)?{return Ok(());}
        // Entity vtable+52 is IsLocalPlayer, not merely IsActor/IsSkater.
        entity.state_out.state=if self.is_local_player(want.target,entity)?{71}else{19};
        entity.head_tracking.as_mut().ok_or("taunt head tracking absent")?
            .push_targetable(HeadTrackingMode::TargetableVisibilityPoint,want.target,||Ok(true))?;
        let packet=entity.intent_queue.as_mut().ok_or("taunt intent queue absent")?
            .create_monitored(entity.state.as_mut().ok_or("taunt state absent")?,hash)?;
        let mut stage=IntentStage::default();
        stage.add_intent(FastString30::encode("Taunt").expect("native intent"))?;
        let target=self.target_position(entity,want.target)?.ok_or("taunt target disappeared")?;
        let core=entity.core.as_ref().ok_or("taunt core absent")?;
        let direction=[target[0]-core.transform[3][0],0.,target[2]-core.transform[3][2],0.];
        // Native component-wise comparison against EPSILON_23 (34000000),
        // not Math.NormalizeSafe's different squared-length threshold.
        let direction=if direction.iter().any(|v|v.abs()>f32::from_bits(0x3400_0000)){
            try_normalize_unsafe(direction,&mut HostReckoningNumeric).map_err(|e|format!("taunt direction: {e:?}"))?
        }else{core.transform[2]};
        entity.navigation_intent.explicit_direction=Some(direction);
        let queue=entity.intent_queue.as_mut().ok_or("taunt intent queue absent")?;
        queue.packet_mut(packet)?.add_stage(stage)?;
        queue.suggest(packet,&mut entity.animation.as_mut().ok_or("taunt animation absent")?.ai_input)?;
        Ok(())
    }
    /// AttemptTakeDownTargetable.End8270a2e8 releases its monitor; it does not
    /// cancel the queued animation. The queue retains ownership when active.
    pub(crate) fn end_takedown_attempt(&mut self,entity:&mut PedestrianConstruction)->Result<(),String>{
        let hash=skate2_formats::hash::sk8_hash_value_32(b"ActiveTakedown");
        entity.intent_queue.as_mut().ok_or("takedown queue absent")?
            .release_monitored(entity.state.as_mut().ok_or("takedown state absent")?,hash)?;
        entity.head_tracking.as_mut().ok_or("takedown head tracking absent")?.pop();
        Ok(())
    }
}

impl FrameServices<'_>{
    /// Actual ITargetable+16 implementations: Actor/Pedestrian/Guard/DMO/
    /// Vehicle fold to li r3,1 at826c6d80; BigBlack folds to li r3,0.
    fn target_can_be_taken_down(&self,entity:&PedestrianConstruction,
        target:crate::living_world_chase::TargetableId)->Result<bool,String>{
        if !self.target_exists(entity,target)?{return Ok(false);}
        if target.kind==1{
            let ped=if target.id as u32==entity.id{entity}else{self.pedestrian(target).ok_or("takedown target absent")?};
            return Ok(!ped.allocations.iter().any(|(kind,_)|*kind==crate::living_world_pedestrian_memory::PedestrianStore::BigBlack));
        }
        Ok(true)
    }
    /// AttemptTakeDownTargetable.Update8270a038; contact result comes from
    /// cLivingWorldCollision.RequiredToBeginColliding+8 and the retained AI collider ID.
    pub(crate) fn update_takedown_attempt(&mut self,entity:&mut PedestrianConstruction)->Result<(),String>{
        use crate::pedestrian_intent_packet::PacketState;
        let hash=skate2_formats::hash::sk8_hash_value_32(b"ActiveTakedown");
        let state=entity.state.as_ref().ok_or("takedown state absent")?;
        let Some(&id)=state.intent_monitors.get(&hash)else{return Ok(());};
        let target=state.take_down_sink;
        if !self.target_exists(entity,target)?{return Ok(());}
        let packet=entity.intent_queue.as_ref().ok_or("takedown queue absent")?.packet(id)?;
        if packet.state!=PacketState::Active{return Ok(());}
        let contact=entity.collision.as_ref().ok_or("takedown collision absent")?.required_to_begin_colliding;
        if state.takedown_status.ok_or("takedown status producer absent")?!=0{return Ok(());}
        let result=if !contact{
            if packet.active_stage_number()!=1{return Ok(());}
            3
        }else{
            if state.collider!=target{return Ok(());}
            if self.target_can_be_taken_down(entity,target)?{2}else{3}
        };
        entity.state.as_mut().ok_or("takedown state absent")?.takedown_status=Some(result);
        Ok(())
    }
    /// TakeDownTargetableFailure826ed560 / Begin8270a398 / End8270a528.
    pub(crate) fn takedown_failure(&mut self,entity:&mut PedestrianConstruction,
        occurrence:u16,end:bool)->Result<(),String>{
        if end{entity.state.as_mut().ok_or("takedown state absent")?.flags[1]&=!0x80;return Ok(());}
        let target=entity.state.as_ref().ok_or("takedown state absent")?.take_down_sink;
        if self.target_exists(entity,target)?{
            let rejected=!self.target_can_be_taken_down(entity,target)?;
            let fall=entity.state.as_ref().ok_or("takedown state absent")?.flags[0]&1!=0;
            let trip=if !rejected&&fall{
                let chance=crate::living_world_guard_graph::binding::attribute(occurrence,"tripChance")?
                    .map(str::parse::<f32>).transpose().map_err(|e|e.to_string())?.unwrap_or(0.35);
                use crate::native_skater_random::rw_math::{bernoulli_choice,global_next};
                bernoulli_choice(chance,global_next().map_err(|e|format!("takedown random: {e:?}"))?)
            }else{false};
            if rejected||trip{
                entity.collision.as_mut().ok_or("takedown collision absent")?.required_to_begin_colliding=true;
                use crate::pedestrian_graph::colliding::{CollisionForce,CollisionDirection};
                entity.collision_info.force=CollisionForce::Knockdown;
                entity.collision_info.direction=CollisionDirection::Back;
            }
        }
        entity.state.as_mut().ok_or("takedown state absent")?.flags[1]|=0x80;
        Ok(())
    }
}

impl FrameServices<'_>{
    /// ITargetable.ForceWipeOut: only Actor825edad0 has gameplay work;
    /// pedestrian/guard/vehicle/DMO vtables bind folded empty825187c0.
    fn force_target_wipe_out(&self,entity:&PedestrianConstruction,
        target:crate::living_world_chase::TargetableId)->Result<(),String>{
        if let Some(actor)=self.actor(target)?{
            actor.try_living_world_wipe_out(entity.core.as_ref().ok_or("takedown source core absent")?.transform[3])?;
        }
        Ok(())
    }
    /// TakeDownTargetableSuccess.Begin82717918.
    pub(crate) fn takedown_success(&mut self,entity:&mut PedestrianConstruction)->Result<(),String>{
        use crate::{pedestrian_intent_packet::IntentStage,play_animation::FastString30,
            living_world_guard_graph::GuardGraphServices};
        let hash=skate2_formats::hash::sk8_hash_value_32(b"ActiveTakedown");
        let state=entity.state.as_ref().ok_or("takedown state absent")?;
        let id=*state.intent_monitors.get(&hash).ok_or("native success requires monitored takedown")?;
        let target=state.take_down_sink;
        let mut stage=IntentStage::default();
        stage.add_intent(FastString30::encode("DoTakeDown").expect("native intent"))?;
        stage.add_intent(FastString30::encode("TakedownSuccessful").expect("native intent"))?;
        stage.rotate_to_target=false;
        // Begin always creates two stages; native success overwrites index1.
        *entity.intent_queue.as_mut().ok_or("takedown queue absent")?.packet_mut(id)?.stage_mut(1)?=stage;
        entity.collision.as_mut().ok_or("takedown collision absent")?.required_to_begin_colliding=false;
        entity.state.as_mut().ok_or("takedown state absent")?.collision_flags|=0x20;
        if self.target_exists(entity,target)?{
            self.force_target_wipe_out(entity,target)?;
            entity.ignore_collision_target=target;
            if self.is_local_player(target,entity)?{
                self.show_chase_end_message(true,false,entity)?;
                entity.state_out.state=71;
            }else{entity.state_out.state=19;}
        }
        Ok(())
    }
    /// TazeWantTarget.Begin8270bb48; unlike taunt this reads the named want's
    /// target without checking its active bit. Chase-end is conditional.
    pub(crate) fn taze_want_target(&mut self,entity:&mut PedestrianConstruction,occurrence:u16)->Result<(),String>{
        use crate::living_world_guard_graph::GuardGraphServices;
        let want=crate::living_world_guard_graph::binding::attribute(occurrence,"want")?
            .and_then(crate::pedestrian_graph::conditions::want_type).ok_or("TazeWantTarget requires valid want")?;
        let target=entity.state.as_ref().ok_or("taze state absent")?.wants[want].target;
        if !self.target_exists(entity,target)?{return Ok(());}
        self.force_target_wipe_out(entity,target)?;
        // IChasee vtable32/24 checked against its original interface layout.
        if self.with_chasee(entity,target,|state|state.is_some_and(|s|!s.chasers.is_empty()))?{
            self.with_chasee_mut(entity,target,|state|{
                state.ok_or("taze chasee disappeared")?.group_end_reason=1;Ok::<_,String>(())
            })??;
        }
        if self.is_local_player(target,entity)?{self.show_chase_end_message(true,true,entity)?;}
        Ok(())
    }
}

impl FrameServices<'_>{
    /// AttemptTakeDownTargetable.Begin82717660, using the authored selected
    /// tAnimTakedown and the same monitored packet consumed by Update/End.
    pub(crate) fn begin_takedown_attempt(&mut self,entity:&mut PedestrianConstruction)->Result<(),String>{
        use crate::{pedestrian_intent_packet::IntentStage,play_animation::FastString30,
            pedestrian_adjust_owners::HeadTrackingMode,living_world_chase::TargetableId};
        let hash=skate2_formats::hash::sk8_hash_value_32(b"ActiveTakedown");
        let packet=entity.intent_queue.as_mut().ok_or("takedown queue absent")?
            .create_monitored(entity.state.as_mut().ok_or("takedown state absent")?,hash)?;
        let target=entity.state.as_ref().ok_or("takedown state absent")?.take_down_sink;
        let exists=self.target_exists(entity,target)?;
        let mut attack=IntentStage::default();
        attack.add_intent(FastString30::encode("DoTakeDown").expect("native intent"))?;
        if exists{
            let selected=entity.selected_take_down()?.ok_or("native takedown selected descriptor assertion")?;
            let state=entity.state.as_mut().ok_or("takedown state absent")?;
            let name=if state.flags[0]&0x80!=0{&selected.mirrored_attack_name}else{&selected.attack_name};
            attack.add_intent(FastString30::encode(name).map_err(|e|format!("takedown attack name: {e:?}"))?)?;
            state.flags[0]=(state.flags[0]&!1)|u8::from(selected.fall_on_fail);
            attack.rotate_to_target=selected.rotate_to_target;
        }
        let mut failed=IntentStage::default();
        failed.add_intent(FastString30::encode("DoTakeDown").expect("native intent"))?;
        failed.add_intent(FastString30::encode("TakedownFailed").expect("native intent"))?;
        failed.rotate_to_target=false;
        let queue=entity.intent_queue.as_mut().ok_or("takedown queue absent")?;
        queue.packet_mut(packet)?.add_stage(attack)?;
        queue.packet_mut(packet)?.add_stage(failed)?;
        queue.suggest(packet,&mut entity.animation.as_mut().ok_or("takedown animation absent")?.ai_input)?;
        entity.state.as_mut().ok_or("takedown state absent")?.takedown_status=Some(0);
        entity.collision.as_mut().ok_or("native takedown collision assertion")?.required_to_begin_colliding=false;
        entity.head_tracking.as_mut().ok_or("takedown head tracking absent")?
            .push_targetable(HeadTrackingMode::TargetableVisibilityPoint,
                if exists{target}else{TargetableId::INVALID},||Ok(exists))?;
        Ok(())
    }
    /// TakeDownTargetablePredictions.Update82709f70 invokes the existing
    /// ILWTakeDownSource owner only while its current sink resolves.
    pub(crate) fn update_takedown_prediction(&mut self,entity:&mut PedestrianConstruction)->Result<(),String>{
        let target=entity.state.as_ref().ok_or("takedown state absent")?.take_down_sink;
        if self.target_exists(entity,target)?{self.determine_take_down_type(entity,target)?;}
        Ok(())
    }
}
