//! Executable lookat generated controller with the actual pedestrian owners.
use super::{Family,PluginController,direct::{self,DirectLeaf}};
use crate::{pedestrian_graph::{PedestrianGraph,PedestrianGraphBackend},
    generated_action_load::runtime::ActionUpdateError as Error,
    generated_graph_selection::Mask,
    living_world_pedestrian_entity::{PedestrianConstruction,targeting::TargetingWorld},
    pedestrian_navigation::{mesh::NavigationEntityFacts,wander::WanderAvoidVolumes},
    reckoning_numeric::ReckoningNumeric};
use std::sync::Arc;

/// Delegate Init and TemplateHC constructor create these class payloads once;
/// per-controller null CreateInstance results are retained by the active set.
pub(crate) struct LookAtBindings {look_at:DirectLeaf}
impl LookAtBindings {
    pub(crate) fn construct()->Result<Self,String>{
        // Original LookAt/Wander/ExitPlugin declarations have no attributes,
        // and Base82cf6f88 receives null parents: enabled defaults true.
        let look_at=DirectLeaf::bind(Family::Lookat,0)?.ok_or("LookAt binding missing")?;
        Ok(Self{look_at})
    }
}
pub(crate) struct LookAtController {
    pub(crate) controller:PluginController,
    pub(crate) bindings:Arc<LookAtBindings>,
}
impl LookAtController {
    pub(crate) fn construct(bindings:Arc<LookAtBindings>)->Self{
        Self{controller:PluginController::construct(Family::Lookat),bindings}
    }
    pub(crate) fn update<N:ReckoningNumeric,A:WanderAvoidVolumes>(&mut self,dt:f32,
        entity:&mut PedestrianConstruction,source:Option<u32>,facts:&NavigationEntityFacts,
        world:&mut TargetingWorld<'_,A>,numeric:&mut N)->Result<(),String> where N::Error:std::fmt::Debug {
        let mut backend=LookAtBackend{entity,source,facts,world,numeric,bindings:&self.bindings,failure:None};
        let result=self.controller.update(dt,&mut backend);
        result.map_err(|error|backend.failure.unwrap_or_else(||format!("lookat graph: {error:?}")))
    }
    pub(crate) fn end<N:ReckoningNumeric,A:WanderAvoidVolumes>(&mut self,
        entity:&mut PedestrianConstruction,source:Option<u32>,facts:&NavigationEntityFacts,
        world:&mut TargetingWorld<'_,A>,numeric:&mut N)->Result<(),String> where N::Error:std::fmt::Debug {
        let mut backend=LookAtBackend{entity,source,facts,world,numeric,bindings:&self.bindings,failure:None};
        let result=self.controller.end_all(&mut backend);
        result.map_err(|error|backend.failure.unwrap_or_else(||format!("lookat graph end: {error:?}")))
    }
}
struct LookAtBackend<'a,'w,N,A:WanderAvoidVolumes>{
    entity:&'a mut PedestrianConstruction,source:Option<u32>,facts:&'a NavigationEntityFacts,
    world:&'a mut TargetingWorld<'w,A>,numeric:&'a mut N,bindings:&'a LookAtBindings,failure:Option<String>,
}
impl<N:ReckoningNumeric,A:WanderAvoidVolumes> PedestrianGraphBackend for LookAtBackend<'_,'_,N,A>
where N::Error:std::fmt::Debug {
    fn condition(&mut self,_:&PedestrianGraph,_:Mask,index:u16,_:u16)->Result<bool,Error>{
        if index!=0{return Err(Error::PendingExpression(index));}
        direct::look_at_timed_out(self.entity).map_err(|e|{self.failure=Some(e);Error::PendingExpression(index)})
    }
    fn enabled(&self,index:u16)->Result<bool,Error>{if index<3 {Ok(true)}else{Err(Error::MissingInstance(index))}}
    fn create_instance(&mut self,_:&PedestrianGraph,index:u16)->Result<(),Error>{
        // All three actual vtables slot44 point at null-instance82bb3da8.
        if index<3 {Ok(())}else{Err(Error::MissingInstance(index))}
    }
    fn begin(&mut self,_:&PedestrianGraph,index:u16)->Result<(),Error>{
        let result=match index {
            0=>self.bindings.look_at.begin(self.entity,self.source),
            1=>crate::pedestrian_graph::locomotion::begin_wander(self.entity,self.facts,self.world,self.numeric),
            2=>direct::exit_plugin(self.entity,self.source,self.world.props),
            _=>return Err(Error::MissingInstance(index)),
        };
        result.map_err(|e|{self.failure=Some(e);Error::MissingInstance(index)})
    }
    fn end_and_remove(&mut self,_:&PedestrianGraph,index:u16)->Result<(),Error>{
        match index {
            0=>self.bindings.look_at.end(self.entity).map_err(|e|{self.failure=Some(e);Error::MissingInstance(index)}),
            1|2=>Ok(()), // actual End825187c0
            _=>Err(Error::MissingInstance(index)),
        }
    }
    fn transition_hook(&mut self,_:&PedestrianGraph,index:u16,_:u16)->Result<(),Error>{Err(Error::PendingTransitionHook(index))}
    fn generated_update(&mut self,_:&PedestrianGraph,source:u32,active:&[u16])->Result<(),Error>{
        match source {
            0x825187c0=>Ok(()),
            // StateUpdate_LookAt_1 calls empty LookAt then Wander slot1.
            0x828f8358 if active==[0,1]=>{crate::pedestrian_graph::locomotion::update_wander(self.entity);Ok(())},
            // ExitPlugin update reads its one actual null instance, then blr.
            0x828f81d0 if active==[2]=>Ok(()),
            _=>Err(Error::PendingStateUpdate(source)),
        }
    }
}
