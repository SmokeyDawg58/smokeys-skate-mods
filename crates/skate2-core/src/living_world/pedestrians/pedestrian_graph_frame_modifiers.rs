//! EvaluateNavModifiers82e80080..82e80364, live skater branch.
use super::FrameServices;
use crate::{living_world_pedestrian_entity::PedestrianConstruction as Entity,
    living_world_chase::TargetableId,pedestrian_graph::colliding::{CollidingServices,CollisionMoodEvent},
    pedestrian_navigation_modifiers::skater_avoidance::{SkaterAvoidanceTarget,SkaterAvoidanceTuning},
    simulation_assembly_backend::constraint_session::physical_island::contact_owners::SimulationContactEntityOwner,
    reckoning_numeric::HostReckoningNumeric,actor_construction::ActorConstruction};
impl FrameServices<'_>{
    /// Complete original evaluation order, including the active protector.
    pub(crate) fn evaluate_all_navigation_modifiers(&mut self,entity:&mut Entity)->Result<(),String>{
        self.evaluate_pedestrian_modifiers(entity)?;
        self.evaluate_skater_modifier(entity)?;
        self.evaluate_vehicle_modifier(entity)?;
        self.evaluate_big_black_modifier(entity)?;
        self.evaluate_dmo_modifier(entity)
    }
    fn evaluate_vehicle_modifier(&mut self,entity:&mut Entity)->Result<(),String>{
        if !entity.navigation_modifiers.as_ref().ok_or("modifiers absent")?.vehicle.enabled{return Ok(());}
        let radius=self.cvars.try_pedestrian_road_prediction().map_err(|e|format!("vehicle KD tuning: {e:?}"))?.1;
        let core=entity.core.as_ref().ok_or("vehicle avoidance core absent")?;
        let nearby=self.vehicles.find_near([core.transform[3][0],core.transform[3][2]],radius,10,&mut HostReckoningNumeric)
            .map_err(|e|format!("vehicle avoidance KD: {e:?}"))?;
        for(id,_)in nearby{
            let Some(vehicle)=self.vehicles.get(id)else{continue;};
            if entity.ignore_avoid_target==(TargetableId{kind:3,id:id as i32}){continue;}
            let tuning=self.cvars.try_pedestrian_vehicle_avoidance_tuning().map_err(|e|format!("vehicle avoidance tuning: {e:?}"))?;
            let on_road=crate::pedestrian_graph::conditions::entity_condition("IsOnRoad",entity)?.ok_or("road condition binding absent")?;
            let target=crate::pedestrian_navigation_modifiers::vehicle_avoidance::VehicleTarget{
                id,position:vehicle.core.transform[3],velocity:vehicle.core.velocity,
                breaking_rules:if on_road{vehicle.motion.breaking_rules.ok_or("vehicle breaking-rules state absent")?}else{false},
                rig:vehicle.nav_rig.as_ref()};
            let core=entity.core.as_ref().unwrap();
            //Both Navigation and MeshNavigation slot184 fold to82e6bb58,
            //loading original base+112, our path_target.
            let interim=entity.mesh_navigation.as_ref().ok_or("vehicle bound MESH navigation absent")?.base.path_target;
            entity.navigation_modifiers.as_mut().unwrap().vehicle.recommend(core.transform[3],core.velocity,interim,on_road,target,&tuning,
                entity.intent_queue.as_mut().ok_or("vehicle avoid intent queue absent")?,entity.state.as_mut().ok_or("vehicle avoid AI state absent")?,&mut HostReckoningNumeric)?;
            if entity.navigation_modifiers.as_ref().unwrap().vehicle.flags.active{break;}
        }
        Ok(())
    }
    /// BigBlackAvoidanceNavModifier.RecommendTarget82e9dfa8 and
    /// HandleBurdened_BigBlackAvoid82e9c1c8, over the original MESH owner.
    fn evaluate_big_black_modifier(&mut self,entity:&mut Entity)->Result<(),String>{
        use crate::ai::vmx::{self,sub,scale,dot3,length3};
        let modifier=&mut entity.navigation_modifiers.as_mut().ok_or("modifiers absent")?.big_black;
        if !modifier.enabled{return Ok(());}
        if self.big_black_entity.is_some_and(|id|entity.ignore_avoid_target==(TargetableId{kind:1,id:id as i32})){return Ok(());}
        modifier.target_entity=self.big_black_entity.map(SimulationContactEntityOwner::LivingWorld);
        modifier.flags=crate::pedestrian_navigation_modifiers::RecommendationFlags::default();
        modifier.max_turn_angle_degrees=self.cvars.try_pedestrian_big_black_turn_angle().map_err(|e|format!("BigBlack turn tuning: {e:?}"))?;
        modifier.big_black=None;modifier.influence_radius=0.;
        let Some(id)=*self.big_black_entity else{return Ok(());};
        let target=if id==entity.id{&*entity}else{self.pedestrian(TargetableId{kind:1,id:id as i32}).ok_or("BigBlack Manager entity absent")?};
        let data=target.big_black.as_ref().ok_or("BigBlack protection interface absent")?;
        let influence=data.scalar(crate::living_world_pedestrian_entity::big_black::PROTECTION_RADIUS)?;
        let location=target.core.as_ref().ok_or("BigBlack core absent")?.transform[3];
        let navigation=&entity.mesh_navigation.as_ref().ok_or("BigBlack avoidance mesh absent")?.base;
        let scalar=|key|->Result<f32,String>{Ok(f32::from_bits(navigation.parameters.try_scalar_word(key,0).map_err(|e|e.to_string())?.unwrap_or(0)))};
        let medium=scalar(0xeec8_3d9c_b769_fed8)?;let high=scalar(0x8986_e69c_dfdd_8053)?;
        let timeout=scalar(0x7c8e_344c_dec7_510b)?;
        let mut seek=navigation.seek_direction;seek[1]=0.;seek[3]=0.;
        let old_speed=navigation.speed();
        let parameters=&entity.resources.as_ref().ok_or("pedestrian locomotion absent")?.locomotion;
        if parameters.is_null(){return Err("native manoeuvrable run speed collection assertion".into());}
        let run_speed=f32::from_bits(parameters.try_scalar_word(0xb45d_57ff_1bf0_17d7,0).map_err(|e|e.to_string())?.unwrap_or(0));
        let position=entity.core.as_ref().ok_or("avoidance pedestrian core absent")?.transform[3];
        let modifier=&mut entity.navigation_modifiers.as_mut().unwrap().big_black;
        modifier.big_black=Some(SimulationContactEntityOwner::LivingWorld(id));
        modifier.influence_radius=influence;modifier.medium_avoid_radius=medium;modifier.high_avoid_radius=high;
        modifier.avoidance_location=location;
        // The native precheck reads pEntity (the BigBlack target), not the owner.
        if length3(sub(location,location))>influence || !self.cvars.try_big_black_avoidance_enabled().map_err(|e|format!("BigBlack avoidance tuning: {e:?}"))?{return Ok(());}
        let seek=scale(seek,vmx::rsqrt(dot3(seek,seek)));
        let mut delta=sub(location,position);delta[1]=0.;delta[3]=0.;
        let distance=length3(delta);let direction=scale(delta,vmx::reciprocal(distance));
        let(mut away,mut tangent,mut forward,mut old,mut run)=(0.,0.,1.,1.,0.);
        if distance<high{away=1.;forward=0.;old=0.;run=1.;}
        else if distance<medium{
            let t=(1./(medium-high))*(distance-high);tangent=t;old=t;away=1.-t;run=1.-t;
        }else if distance<influence{
            forward=(1./(influence-medium))*(distance-medium);tangent=1.-forward;
        }
        if dot3(seek,direction)>=0.{
            let perpendicular=vmx::cross([0.,1.,0.,0.],delta);
            let mut perpendicular=scale(perpendicular,vmx::rsqrt(dot3(perpendicular,perpendicular)));
            let state=entity.state.as_mut().ok_or("avoidance AI state absent")?;
            let timer=crate::living_world_pedestrian_state::timer_type("BigBlackAvoidanceDeviationTimer").ok_or("BigBlack deviation timer absent")?;
            if state.timer(timer)<=0.{
                let left=dot3(seek,perpendicular)<0.;
                if left!=modifier.last_deviation_was_left{
                    modifier.last_deviation_was_left=left;
                    state.set_timer(timer,timeout.max(state.timer(timer))).map_err(|e|format!("BigBlack deviation timeout: {e:?}"))?;
                }
            }
            if modifier.last_deviation_was_left{perpendicular=scale(perpendicular,-1.);}
            let direction=sub(vmx::madd(seek,[forward;4],scale(perpendicular,tangent)),scale(delta,away));
            let direction=scale(direction,vmx::rsqrt(dot3(direction,direction)));
            modifier.target=vmx::madd(direction,[10.;4],position);modifier.flags.target=true;
        }
        modifier.speed=old.mul_add(old_speed,run*run_speed);
        modifier.flags.active=true;modifier.flags.speed=true;
        Ok(())
    }
    pub(crate) fn evaluate_dmo_modifier(&mut self,entity:&mut Entity)->Result<(),String>{
        if !entity.navigation_modifiers.as_ref().ok_or("modifiers absent")?.dynamic_object.enabled{return Ok(());}
        let attributes=self.vault.instance(skate2_formats::attrib::string_hash64("physics_skeleton"),skate2_formats::attrib::string_hash64("default"));
        let minimum_mass=attributes.try_scalar_word(0x9ab5_7b38_d5bb_9449,0).map_err(|e|e.to_string())?.map_or(0.,f32::from_bits);
        let tuning=self.cvars.try_pedestrian_dmo_avoidance_tuning().map_err(|e|format!("DMO avoidance tuning: {e:?}"))?;
        let p=entity.core.as_ref().ok_or("DMO avoidance core absent")?.transform[3];
        let nearby=self.dmos.try_objects_within_range([p[0],p[2]],tuning.influence_radius,10).map_err(|e|format!("DMO avoidance KD: {e:?}"))?;
        for(id,_)in nearby{
            let Some(object)=self.dmos.object(id)else{continue;};
            //Raw82e807a4 BLE skips equal mass; unordered continues.
            if object.is_set_as_hand_prop()||object.model.mass<=minimum_mass||entity.ignore_avoid_target==(TargetableId{kind:2,id:id as i32}){continue;}
            let bbox=object.model.bbox_transform(&object.transform);
            let target=crate::pedestrian_navigation_modifiers::dmo_avoidance::DmoTarget{
                id,position:std::array::from_fn(|i|object.transform[12+i]),center:std::array::from_fn(|i|bbox[12+i]),
                velocity:object.velocity,corners:object.farthest_corners};
            let avoid=self.cvars.try_pedestrian_skater_avoidance_tuning().map_err(|e|format!("avoidance mesh tuning: {e:?}"))?;
            let core=entity.core.as_ref().unwrap();
            entity.navigation_modifiers.as_mut().unwrap().dynamic_object.recommend(core.transform[3],core.transform[2],target,&tuning,&avoid,
                entity.nav_rig.as_ref().ok_or("DMO avoidance NavRig absent")?,&self.mesh.store,
                entity.intent_queue.as_mut().ok_or("DMO avoidance queue absent")?,entity.state.as_mut().ok_or("DMO avoidance state absent")?,&mut HostReckoningNumeric)?;
            if entity.navigation_modifiers.as_ref().unwrap().dynamic_object.active{break;}
        }
        Ok(())
    }
    /// EvaluateNavModifiers82e7fcc8..8007c, before the skater branch.
    pub(crate) fn evaluate_pedestrian_modifiers(&mut self,entity:&mut Entity)->Result<(),String>{
        entity.navigation_modifiers.as_mut().ok_or("navigation modifiers absent")?.reset_evaluation();
        let (prediction,radius)=self.cvars.try_pedestrian_road_prediction().map_err(|e|format!("road prediction: {e:?}"))?;
        entity.navigation_modifiers.as_mut().unwrap().general.predict_time=prediction;
        let chasee=entity.state.as_ref().ok_or("modifier AI state absent")?.chaser.chasee;
        if self.with_chasee(entity,chasee,|target|target.is_some())?{
            if entity.navigation_modifiers.as_ref().unwrap().chase_glue.enabled{
                let position=self.target_position(entity,chasee)?.ok_or("Chasee GetSelf entity absent")?;
                let owner=if chasee.kind==0{
                    SimulationContactEntityOwner::Physical(self.actor(chasee)?.ok_or("Chasee Actor absent")?
                        .cached_physical_allocation().map_err(|e|format!("glue Actor: {e:?}"))?.owner().clone())
                }else{SimulationContactEntityOwner::LivingWorld(chasee.id as u32)};
                entity.navigation_modifiers.as_mut().unwrap().chase_glue.recommend(
                    &entity.mesh_navigation.as_ref().ok_or("glue bound MESH navigation absent")?.base,
                    owner,position,&mut HostReckoningNumeric).map_err(|e|format!("glue numeric: {e:?}"))?;
            }
        }else{
            let master=entity.state.as_ref().unwrap().master_id;
            if master!=-1&&(master as u32==entity.id||self.pedestrian(TargetableId{kind:1,id:master}).is_some()){
                self.recommend_general(entity,master as u32)?;
                if !entity.navigation_modifiers.as_ref().unwrap().general.flags.active{
                    let master=entity.state.as_ref().unwrap().master_id;
                    if master!=-1&&(master as u32==entity.id||self.pedestrian(TargetableId{kind:1,id:master}).is_some()){
                        self.clear_pathing_chain(entity,master as u32)?;
                        entity.state.as_mut().unwrap().master_id=-1;
                    }
                }
            }
        }
        let general=&entity.navigation_modifiers.as_ref().unwrap().general;
        if general.enabled&&!general.flags.active{
            let p=entity.core.as_ref().ok_or("modifier core absent")?.transform[3];
            let nearby=self.pedestrian_spatial.find_near_numeric([p[0],p[2]],radius,10,&mut HostReckoningNumeric)
                .map_err(|e|format!("pedestrian modifier KD: {e:?}"))?;
            for (id,_) in nearby{
                let id=id as u32;
                if id==entity.id||entity.ignore_avoid_target== (TargetableId{kind:1,id:id as i32}){continue;}
                if self.pedestrian(TargetableId{kind:1,id:id as i32}).is_none(){continue;}
                self.recommend_general(entity,id)?;
                if entity.navigation_modifiers.as_ref().unwrap().general.flags.active{break;}
            }
        }
        Ok(())
    }
    fn recommend_general(&mut self,entity:&mut Entity,id:u32)->Result<(),String>{
        let target=if id==entity.id{&*entity}else{self.pedestrian(TargetableId{kind:1,id:id as i32}).ok_or("general candidate absent")?};
        let target=target.core.as_ref().ok_or("general candidate core absent")?;
        let target=crate::pedestrian_navigation_modifiers::general_avoidance::GeneralTarget{
            owner:SimulationContactEntityOwner::LivingWorld(id),position:target.transform[3],velocity:target.velocity};
        let tuning=self.cvars.try_pedestrian_general_avoidance_tuning().map_err(|e|format!("general tuning: {e:?}"))?;
        let core=entity.core.as_ref().ok_or("general core absent")?;
        entity.navigation_modifiers.as_mut().ok_or("general modifier absent")?.general.recommend(
            core.transform[3],core.velocity,core.transform[2],core.transform[0],target,&tuning,
            entity.intent_queue.as_mut().ok_or("general queue absent")?,&mut HostReckoningNumeric)
    }
    /// ClearPathingMasterSlaveChain82e6ecb8 follows the actual peers in-place.
    fn clear_pathing_chain(&mut self,entity:&mut Entity,mut id:u32)->Result<(),String>{
        loop{
            let target=if id==entity.id{&mut *entity}else{
                let Some(target)=self.pedestrian_mut(TargetableId{kind:1,id:id as i32}) else{break;};target
            };
            let state=target.state.as_mut().ok_or("pathing chain state absent")?;
            let slave=state.slave_id;state.slave_id=-1;
            let next=if slave as u32==entity.id{Some(&mut *entity)}else{self.pedestrian_mut(TargetableId{kind:1,id:slave})};
            let Some(next)=next else{break;};
            next.state.as_mut().ok_or("pathing slave state absent")?.master_id=-1;
            id=slave as u32;
        }
        Ok(())
    }
    /// Called only at the skater branch, after General/ChaseGlue evaluation.
    /// Refresh precedes GetLocal's null gate. NPC traversal follows manager order.
    pub(crate) fn evaluate_skater_modifier(&mut self,entity:&mut Entity)->Result<(),String>{
        if !entity.navigation_modifiers.as_ref().ok_or("navigation modifiers absent")?.skater.enabled{return Ok(());}
        let tuning=self.cvars.try_pedestrian_skater_avoidance_tuning().map_err(|e|format!("skater avoidance tuning: {e:?}"))?;
        entity.navigation_modifiers.as_mut().unwrap().skater.refresh_tuning(&tuning);
        let local_index=self.world.local_skater_identity.as_ref().ok_or("avoidance SkaterManager absent")?
            .try_local_index().map_err(|e|e.to_string())?;
        let local=TargetableId{kind:0,id:local_index};
        let Some(actor)=self.actor(local)?.cloned() else{return Ok(());};
        if entity.ignore_avoid_target!=local{self.recommend_skater(entity,&actor,local,&tuning)?;}
        if !tuning.avoid_npc_skaters||entity.navigation_modifiers.as_ref().unwrap().skater.active{return Ok(());}
        for index in 0..self.actors.len(){
            let actor=self.actors[index].clone();
            let id=actor.prefix().map_err(|e|e.to_string())?.fields().ok_or("avoidance Actor fields absent")?.player_index as i32;
            let target=TargetableId{kind:0,id};
            if id==local_index||target==entity.ignore_avoid_target{continue;}
            self.recommend_skater(entity,&actor,target,&tuning)?;
            if entity.navigation_modifiers.as_ref().unwrap().skater.active{break;}
        }
        Ok(())
    }
    fn recommend_skater(&mut self,entity:&mut Entity,actor:&ActorConstruction,
        targetable:TargetableId,tuning:&SkaterAvoidanceTuning)->Result<(),String>{
        let physical=actor.cached_physical_allocation().map_err(|e|format!("avoidance Actor allocation: {e:?}"))?;
        let owner=physical.owner().clone();
        let target={
            let output=owner.phys_out().lock().map_err(|_|"avoidance PhysOut lock")?;
            let position=output.try_skeleton_output().map_err(|e|format!("avoidance position: {e:?}"))?.skeleton_root_matrix.columns()[3];
            let velocity=output.try_system_centre_and_damped_velocity().map_err(|e|format!("avoidance velocity: {e:?}"))?.1;
            let state=output.try_filtered_output().map_err(|e|format!("avoidance physics state: {e:?}"))?.filtered_physics_state as u32;
            // Existing reader with 'any bail' gate projects the same Scoring2
            // HoM flag, with completed-component validation and no copied state.
            let bail=!output.try_vehicle_avoidance([true,false,false]).map_err(|e|format!("avoidance HoM: {e:?}"))?;
            let lanes=|v:crate::skeleton_root_transforms::SkeletonRootVector|[v.xyz[0],v.xyz[1],v.xyz[2],v.w.unwrap_or(0.)];
            SkaterAvoidanceTarget{owner:SimulationContactEntityOwner::Physical(owner.clone()),targetable,
                position:lanes(position),velocity:lanes(velocity),filtered_physics_state:state,hom_bail_in_progress:bail}
        };
        let core=entity.core.as_ref().ok_or("avoidance pedestrian core absent")?;
        let id=TargetableId{kind:1,id:entity.id as i32};
        let mood=entity.navigation_modifiers.as_mut().ok_or("navigation modifiers absent")?.skater.recommend(
            id,core.transform[3],core.transform[2],target,tuning,
            entity.nav_rig.as_ref().ok_or("avoidance NavRig absent")?,&self.mesh.store,
            entity.intent_queue.as_mut().ok_or("avoidance intent queue absent")?,&mut HostReckoningNumeric)?;
        if let Some(mood)=mood{self.broadcast_mood(CollisionMoodEvent{event_type:3,position:mood.position,
            perpetrator:mood.perpetrator,victim:mood.victim,magnitude:1.,exposure:1.})?;}
        Ok(())
    }
}
