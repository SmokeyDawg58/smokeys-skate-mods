//! Executable usetrashbin graph over retained Prop, hand-prop and channel owners.
use super::{Family,PluginController,direct::{self,DirectLeaf}};
use crate::{pedestrian_graph::{PedestrianGraph,PedestrianGraphBackend},
    generated_action_load::runtime::ActionUpdateError as Error,generated_graph_selection::Mask,
    living_world_pedestrian_entity::PedestrianConstruction,
    living_world_grind_services::waypoints::PropMan,reckoning_numeric::ReckoningNumeric,
    input_cvar_runtime::InputCvarReferences,native_skater_random::NativeSkaterRandom};
use std::sync::Arc;
pub(crate) struct TrashBindings {disallow:DirectLeaf}
impl TrashBindings {
    pub(crate) fn construct()->Result<Self,String>{Ok(Self{
        disallow:DirectLeaf::bind(Family::Usetrashbin,0)?.ok_or("trash disallow binding absent")?,
    })}
}
pub(crate) struct TrashController {
    pub(crate) controller:PluginController,pub(crate) bindings:Arc<TrashBindings>,
}
impl TrashController {
    pub(crate) fn construct(bindings:Arc<TrashBindings>)->Self{
        Self{controller:PluginController::construct(Family::Usetrashbin),bindings}
    }
    pub(crate) fn update<N:ReckoningNumeric>(&mut self,dt:f32,entity:&mut PedestrianConstruction,
        source:Option<u32>,props:&mut PropMan,cvars:&InputCvarReferences,random:&mut NativeSkaterRandom,numeric:&mut N
    )->Result<(),String> where N::Error:std::fmt::Debug {
        let mut backend=TrashBackend{entity,source,props,cvars,random,numeric,bindings:&self.bindings,failure:None};
        self.controller.update(dt,&mut backend).map_err(|e|backend.failure.unwrap_or_else(||format!("trash graph: {e:?}")))
    }
    pub(crate) fn end<N:ReckoningNumeric>(&mut self,entity:&mut PedestrianConstruction,
        source:Option<u32>,props:&mut PropMan,cvars:&InputCvarReferences,random:&mut NativeSkaterRandom,numeric:&mut N
    )->Result<(),String> where N::Error:std::fmt::Debug {
        let mut backend=TrashBackend{entity,source,props,cvars,random,numeric,bindings:&self.bindings,failure:None};
        self.controller.end_all(&mut backend).map_err(|e|backend.failure.unwrap_or_else(||format!("trash graph end: {e:?}")))
    }
}
struct TrashBackend<'a,N>{entity:&'a mut PedestrianConstruction,source:Option<u32>,props:&'a mut PropMan,
    cvars:&'a InputCvarReferences,random:&'a mut NativeSkaterRandom,numeric:&'a mut N,
    bindings:&'a TrashBindings,failure:Option<String>}
impl<N:ReckoningNumeric> TrashBackend<'_,N> where N::Error:std::fmt::Debug {
    ///ThrowHandPropAtTrashBin.Begin8270b520 queries source IWaypointGroup and
    ///GetWaypoint(0), regardless of that point's reservation state.
    fn throw_at_bin(&mut self)->Result<(),String>{
        let Some(source)=self.source else{return Ok(());};
        let prop=self.props.entities.get(source).ok_or("trash plugin source Prop absent")?;
        let Some(group)=prop.waypoint.as_ref() else{return Ok(());};
        let Some(point)=group.points.first() else{return Ok(());};
        self.entity.throw_hand_prop_at_position(point.location,false,self.cvars,self.random,self.numeric)
    }
}
impl<N:ReckoningNumeric> PedestrianGraphBackend for TrashBackend<'_,N> where N::Error:std::fmt::Debug {
    fn condition(&mut self,_:&PedestrianGraph,_:Mask,index:u16,_:u16)->Result<bool,Error>{
        if index!=0{return Err(Error::PendingExpression(index));}
        self.entity.has_hand_prop().map_err(|e|{self.failure=Some(e);Error::PendingExpression(index)})
    }
    fn enabled(&self,index:u16)->Result<bool,Error>{if index<3{Ok(true)}else{Err(Error::MissingInstance(index))}}
    fn create_instance(&mut self,_:&PedestrianGraph,index:u16)->Result<(),Error>{
        // All three actual vtable slot44 callbacks fold to null82bb3da8.
        if index<3{Ok(())}else{Err(Error::MissingInstance(index))}
    }
    fn begin(&mut self,_:&PedestrianGraph,index:u16)->Result<(),Error>{
        let result=match index {
            0=>self.bindings.disallow.begin(self.entity,self.source),
            1=>direct::exit_plugin(self.entity,self.source,self.props),
            2=>self.throw_at_bin(),_=>return Err(Error::MissingInstance(index)),
        };
        result.map_err(|e|{self.failure=Some(e);Error::MissingInstance(index)})
    }
    fn end_and_remove(&mut self,_:&PedestrianGraph,index:u16)->Result<(),Error>{
        match index {
            0=>self.bindings.disallow.end(self.entity).map_err(|e|{self.failure=Some(e);Error::MissingInstance(index)}),
            1|2=>Ok(()), //native empty End825187c0
            _=>Err(Error::MissingInstance(index)),
        }
    }
    fn transition_hook(&mut self,_:&PedestrianGraph,index:u16,_:u16)->Result<(),Error>{Err(Error::PendingTransitionHook(index))}
    fn generated_update(&mut self,_:&PedestrianGraph,source:u32,active:&[u16])->Result<(),Error>{
        // Folded native update only loads each null instance then calls empty
        //825187c0. Root has one occurrence; either child has root plus its own.
        match source {
            0x828f81d0 if active==[0]=>Ok(()),
            0x828f81e8 if active==[0,1]||active==[0,2]=>Ok(()),
            _=>Err(Error::PendingStateUpdate(source)),
        }
    }
}
