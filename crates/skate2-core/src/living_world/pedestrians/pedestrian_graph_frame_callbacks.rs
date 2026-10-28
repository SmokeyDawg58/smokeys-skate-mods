//! Original callback dispatch over the shared AI/motion frame.
use super::FrameServices;
use crate::{living_world_pedestrian_entity::{PedestrianConstruction,targeting::TargetingWorld},
    pedestrian_graph::{backend::AiWorldServices,behaviour_binding::{LocalBehaviourBindings,LocalDispatch,Phase}},
    reckoning_numeric::HostReckoningNumeric};

impl AiWorldServices for FrameServices<'_>{
    fn create_nonlocal_instance(&mut self,_entity:&mut PedestrianConstruction,occurrence:u16,source:u32)->Result<(),String>{
        Err(format!("pedestrian occurrence {occurrence} requires native instance factory {source:#x}"))
    }
    fn release_nonlocal_instance(&mut self,_entity:&mut PedestrianConstruction,_occurrence:u16)->Result<(),String>{
        // All currently admitted occurrence payloads are retained by the sole
        // LocalBehaviourBindings. Nonlocal factories fail before admission.
        Ok(())
    }
    fn dispatch_nonlocal(&mut self,entity:&mut PedestrianConstruction,bindings:&mut LocalBehaviourBindings,
        occurrence:u16,phase:Phase,source:u32,dt:f32)->Result<(),String>{
        if self.big_black_callback(entity,bindings,occurrence,source)?{return Ok(());}
        let shared_occurrence=bindings.forwarded_occurrence(occurrence);
        match source{
            0x826ed228|0x826ed288=>self.with_navigation(entity,
                Some(crate::living_world_pedestrian_entity::targeting::TargetIteratorKind::Flee),
                |entity,facts,world,numeric|{
                    match bindings.dispatch_with_targeting(occurrence,phase,entity,facts,world,numeric)?{
                        LocalDispatch::Handled=>Ok(()),
                        LocalDispatch::Nonlocal{source}=>Err(format!("Flee callback {source:#x} was not bound")),
                    }
                }),
            0x826ed398=>{entity.ignore_avoid_target=crate::living_world_chase::TargetableId::INVALID;Ok(())},
            0x82717660=>self.begin_takedown_attempt(entity),
            0x82709f70=>self.update_takedown_prediction(entity),
            0x82717918=>self.takedown_success(entity),
            0x8270bb48=>self.taze_want_target(entity,shared_occurrence),
            0x8270a038=>self.update_takedown_attempt(entity),
            0x8270a398|0x8270a528=>self.takedown_failure(entity,shared_occurrence,source==0x8270a528),
            0x8270b100|0x826ee3c8=>self.takedown_taunt(entity,source==0x826ee3c8),
            0x8270a2e8=>self.end_takedown_attempt(entity),
            0x8270b468|0x826ee5d0=>self.ignore_takedown_nav_volume(entity,source==0x826ee5d0),
            0x826ff360|0x826ef2a8|0x826eb7d0|0x826ef3c0|0x826fde30|
            0x826eb920|0x826ef4f0|0x826eead0|0x826eeb50|0x8270e840|0x82707d48|0x82707ef0|0x826fe580|0x826fe5e0=>
                self.dispatch_want_callback(entity,shared_occurrence,source),
            0x826ff678|0x8270b768|0x826ff738|0x826ff7f0|0x8270ba10|0x826ff8b0=>
                bindings.dispatch_watch_want(occurrence,phase,entity,self),
            0x826fdcc8=>{
                if let Some(plugin)=entity.plugin.as_ref(){
                    let hash=plugin.template.root_state_hash()?;
                    let time=self.cvars.try_min_plugin_timer().map_err(|e|format!("plugin retry timer: {e:?}"))?;
                    entity.state.as_mut().ok_or("plugin timer AI state absent")?.reset_plugin_timer(hash,time);
                    self.update_plugin(entity,dt)?;
                }Ok(())
            }
            0x826eb098=>{
                if entity.plugin.is_some(){self.cleanly_end_plugin(entity)?;entity.remove_plugin(self.props)?;}Ok(())
            }
            0x82716648|0x8271ae50|0x827074c0=>bindings.dispatch_collision_frame(occurrence,phase,entity,dt,self),
            0x82717548|0x82717018|0x827078f8=>bindings.dispatch_marquee_frame(occurrence,phase,entity,dt,self),
            0x826ef720=>{
                //KnowAboutChasers82e964c8 copies each actual Chasee id in order.
                let count=entity.chasee.as_ref().ok_or("KnowAboutChasers Chasee absent")?.chasers.len();
                for index in 0..count{
                    let target=entity.chasee.as_ref().unwrap().chasers[index].id;
                    self.know_about_entity(entity,target,30.)?;
                }
                Ok(())
            }
            0x826ff968|0x826ffa38=>bindings.dispatch_draw_tazer(occurrence,phase,entity,self),
            0x827172c0=>crate::pedestrian_graph::rest::begin(entity),
            0x826eb218=>crate::pedestrian_graph::rest::end(entity),
            // Only these callbacks consume this targeting frame. Other leaves
            // must not fail merely because unrelated navigation data is absent.
            0x826fe358|0x826eb108|0x826eccb0|0x826ef5c8|0x826eed40|
            0x826ef650|0x826ecbe8|0x82722f50=>{
                let facts=entity.navigation_entity_facts(self.cvars)?;
                let mut world=TargetingWorld{dmos:self.dmos,mesh:self.mesh,rain:self.rain,random:self.random,
                    avoid:self.avoid,cvars:self.cvars,props:self.props,flee_source:None,chasee_position:None};
                match bindings.dispatch_with_targeting(occurrence,phase,entity,&facts,&mut world,&mut HostReckoningNumeric)?{
                    LocalDispatch::Handled=>Ok(()),
                    LocalDispatch::Nonlocal{source}=>Err(format!("targeting callback {source:#x} was not bound")),
                }
            }
            _=>if self.dispatch_chase_callback(entity,shared_occurrence,phase,dt)? || self.dispatch_guard_behaviour(entity,shared_occurrence,phase,dt)?{Ok(())}
                else{Err(format!("pedestrian occurrence {occurrence} requires native {phase:?} callback {source:#x}"))},
        }
    }
}

impl FrameServices<'_>{
    /// KnowAboutEntity82e929e0 checks suppression before self/invalid/lookup.
    pub(crate) fn know_about_entity(&mut self,entity:&mut PedestrianConstruction,
        target:crate::living_world_chase::TargetableId,time:f32)->Result<(),String>{
        let suppressed=self.cvars.try_pedestrian_entity_knowledge_suppressed().map_err(|e|format!("knowledge tuning: {e:?}"))?;
        let id=crate::living_world_chase::TargetableId{kind:1,id:entity.id as i32};
        let exists=if suppressed||target==id||!target.is_valid(){false}else{self.target_exists(entity,target)?};
        entity.state.as_mut().ok_or("knowledge AI state absent")?
            .know_about_entity(id,target,time,suppressed,exists).map_err(|e|e.to_string())?;
        Ok(())
    }
    /// Short Pedestrian.PlayChannelAnim82e6e830 ->82e91e08, shared RemapAnim
    /// then TransitionChannelAnim, with all three blend lengths .2.
    pub(crate) fn request_channel(&mut self,entity:&mut PedestrianConstruction,name:&str)->Result<(),String>{
        entity.request_channel(name,self.random)
    }
}
