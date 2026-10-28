//! Concrete selected-condition reads over the shared frame, no eager snapshot.
use super::FrameServices;
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    living_world_pedestrian_state::PedestrianStateStorage,
    living_world_chase::{TargetableId,conditions::{self,ChaseConditionServices,ChaseTargetView,TargetQuery}},
    pedestrian_graph::condition_binding::ConditionServices,
    pedestrian_navigation::road::RoadNavigationContext};

impl ConditionServices for FrameServices<'_>{
    fn protectee_position(&mut self,target:TargetableId)->Result<Option<[f32;4]>,String>{
        self.protected_position(target)
    }
    fn guard_condition(&mut self,name:&str,reason:Option<i32>,entity:&PedestrianConstruction)->Result<bool,String>{
        self.evaluate_guard_condition(name,reason,entity)
    }
    fn chase_condition(&mut self,name:&str,entity:&PedestrianConstruction)->Result<bool,String>{
        let state=entity.state.as_ref().ok_or("chase condition state absent")?;
        conditions::evaluate(name,&mut ChaseRead{frame:self,entity,state})?
            .ok_or_else(||format!("unknown chase condition {name}"))
    }
    fn chasee_exists(&mut self,entity:&PedestrianConstruction)->Result<bool,String>{
        let target=entity.state.as_ref().ok_or("chase condition state absent")?.chaser.chasee;
        self.with_chasee(entity,target,|state|state.is_some())
    }
    fn target_position(&mut self,entity:&PedestrianConstruction,target:TargetableId)->Result<Option<[f32;4]>,String>{
        FrameServices::target_position(self,entity,target)
    }
    fn entering_crosswalk(&mut self,entity:&PedestrianConstruction)->Result<bool,String>{
        let core=entity.core.as_ref().ok_or("crosswalk core absent")?;
        let mut road=RoadNavigationContext{rain:self.rain,mesh:&self.mesh.store,random:self.random,
            position:core.transform[3],facing:core.transform[2],entity_id:entity.id};
        entity.target_iterators.road.is_crossing(&mut road)
    }
    fn walk_sign_says_go(&mut self,entity:&PedestrianConstruction)->Result<bool,String>{
        let core=entity.core.as_ref().ok_or("walk sign core absent")?;
        let tuning=self.cvars.try_pedestrian_road_tuning().map_err(|e|format!("road tuning: {e:?}"))?;
        let mut road=RoadNavigationContext{rain:self.rain,mesh:&self.mesh.store,random:self.random,
            position:core.transform[3],facing:core.transform[2],entity_id:entity.id};
        entity.target_iterators.road.can_proceed(&mut road,tuning)
    }
    fn interest_in_zone_boundary_or_buffer(&mut self,entity:&PedestrianConstruction)->Result<bool,String>{
        if !self.patrol_zone_valid(entity){return Ok(true);}
        let Some(target)=entity.patrol_entity_of_interest else{return Ok(true);};
        let Some(position)=self.target_position(entity,target)? else{return Ok(true);};
        self.patrol_zone_contains(entity,position,true)
    }
    fn interest_in_maintain_radius(&mut self,entity:&PedestrianConstruction)->Result<bool,String>{
        if !self.patrol_zone_valid(entity){return Ok(false);}
        let Some(target)=entity.patrol_entity_of_interest else{return Ok(false);};
        let Some(target)=self.target_position(entity,target)? else{return Ok(false);};
        let attributes=entity.patrol_zone_parameters.as_ref().ok_or("patrol parameters absent")?;
        if attributes.is_null(){return Err("native patrol parameters collection assertion".into());}
        let radius=f32::from_bits(attributes.try_scalar_word(0xcea7_d8e5_94da_a39e,0).map_err(|e|e.to_string())?.unwrap_or(0));
        let position=entity.core.as_ref().ok_or("patrol condition core absent")?.transform[3];
        let delta=std::array::from_fn(|i|target[i]-position[i]);
        use crate::reckoning_numeric::ReckoningNumeric;
        let squared=crate::reckoning_numeric::HostReckoningNumeric.dot3(delta,delta).map_err(|e|format!("patrol radius: {e:?}"))?;
        Ok(radius*radius>squared)
    }
    fn local_skater_centre_of_mass(&mut self)->Result<[f32;4],String>{
        crate::pedestrian_graph::marquee::local_skater_centre(self)
    }
}
fn xyz(v:[f32;4])->[f32;3]{[v[0],v[1],v[2]]}
struct ChaseRead<'a,'b>{frame:&'a mut FrameServices<'b>,entity:&'a PedestrianConstruction,state:&'a PedestrianStateStorage}
impl ChaseConditionServices for ChaseRead<'_,'_>{
    fn new_chases_allowed(&self)->bool{self.frame.chase.are_new_chases_allowed()}
    fn self_id(&self)->TargetableId{TargetableId{kind:1,id:self.entity.id as i32}}
    fn end_reason(&self)->i32{self.state.chaser.end_reason}
    fn with_target<R>(&mut self,query:TargetQuery,read:impl FnOnce(Option<ChaseTargetView<'_>>)->R)->Result<R,String>{
        let target=self.state.chaser.chasee;
        let exists=self.frame.with_chasee(self.entity,target,|state|state.is_some())?;
        if !exists{return Ok(read(None));}
        let group_end_reason=if matches!(query,TargetQuery::State){
            self.frame.chase_group_end_reason(self.entity,target)?.ok_or("chasee disappeared")?
        }else{-1};
        let self_entity_exists=self.frame.target_exists(self.entity,target)?;
        let position=if matches!(query,TargetQuery::Position){self.frame.target_position(self.entity,target)?.map(xyz)}else{None};
        let wiped_out=if matches!(query,TargetQuery::WipedOut){self.frame.target_wiped_out(self.entity,target)?}else{None};
        let has_protector=if matches!(query,TargetQuery::Protector){
            if let Some(actor)=self.frame.actor(target)? {
                let state=actor.prefix().map_err(|e|e.to_string())?;
                let protector=state.fields().ok_or("Actor protectee fields absent")?.protector;
                Some(self.frame.protector_exists(self.entity,protector))
            }else{
                //PDB Pedestrian bases/ctor expose ITargetable/Chasee, but no
                //IProtectee. Native8270cad0's interface query therefore fails.
                Some(false)
            }
        }else{None};
        let knowledge_flags=if matches!(query,TargetQuery::Knowledge){self.state.knowledge.iter().find(|k|k.target==target).map(|k|k.perception_flags())}else{None};
        self.frame.with_chasee(self.entity,target,|state|read(state.map(|state|ChaseTargetView{
            state,group_end_reason,self_entity_exists,position,wiped_out,has_protector,knowledge_flags})))
    }
    fn can_want_target_add_chaser(&mut self)->Result<bool,String>{
        let target=self.state.wants[1].target;
        let Some(count)=self.frame.with_chasee(self.entity,target,|state|state.map(|s|s.chasers.len()))? else{return Ok(false);};
        let attributes=self.frame.vault.instance(crate::living_world_chase::attributes::CHASE_CLASS,
            skate2_formats::attrib::string_hash64("global"));
        if attributes.is_null(){return Err("Chasee global chase parameters absent".into());}
        let max=crate::living_world_chase::attributes::max_chasers(&attributes)?;
        Ok(count<max&&self.frame.chase_group_end_reason(self.entity,target)?==Some(-1))
    }
    fn position(&self)->Result<[f32;3],String>{Ok(xyz(self.entity.core.as_ref().ok_or("chaser core absent")?.transform[3]))}
    fn escape_distance(&self)->Result<f32,String>{
        let attributes=&self.entity.resources.as_ref().ok_or("chase resources absent")?.chase;
        if attributes.is_null(){return Ok(0.);}
        Ok(f32::from_bits(attributes.try_scalar_word(0x240e_b11a_0cb4_69c3,0).map_err(|e|e.to_string())?.unwrap_or(0)))
    }
    fn cannot_reach(&self)->bool{self.state.flags[2]&0x20!=0}
    fn should_block(&self)->Result<bool,String>{Ok(self.entity.chaser_should_block)}
    fn give_up_after_takedowns(&self)->Result<bool,String>{
        let attributes=&self.entity.resources.as_ref().ok_or("chase resources absent")?.chase;
        if attributes.is_null(){return Err("native chase collection assertion".into());}
        Ok(attributes.try_cached_layout_field(8,1).map_err(|e|e.to_string())?.ok_or("chase cached layout absent")?.bytes()[0]!=0)
    }
    fn takedown_count(&self)->Result<u32,String>{self.state.takedown_count.ok_or("takedown counter constructor absent".into())}
    fn takedowns_to_give_up(&self)->Result<u32,String>{
        let attributes=&self.entity.resources.as_ref().ok_or("chase resources absent")?.chase;
        if attributes.is_null(){return Err("native chase collection assertion".into());}
        Ok(attributes.try_scalar_word(0x78e2_e1b1_9721_fd4e,0).map_err(|e|e.to_string())?.unwrap_or(0))
    }
    fn patrol_interest_timer(&self)->f32{self.state.timer(15)}
    fn in_own_zone(&self,include_buffer:bool)->Result<bool,String>{
        if !self.frame.patrol_zone_valid(self.entity){return Ok(true);}
        let position=self.position()?;
        self.frame.patrol_zone_contains(self.entity,[position[0],position[1],position[2],0.],include_buffer)
    }
}
