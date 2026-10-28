//! Concrete per-sink TransferableBehaviour.Copy82cf3b38 and plugin lifecycle.
use super::{load::{ProcessedTransferable,ExecutableTemplate},lookat::LookAtController,trash::TrashController};
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    living_world_grind_services::waypoints::PropMan};
use std::sync::{Arc,Mutex,atomic::{AtomicU64,Ordering}};
pub(crate) struct AttachedPlugin {
    pub(crate) template:Arc<ProcessedTransferable>,
    source:AtomicU64,
    pub(crate) identity:Arc<()>,
    pub(crate) controller:Mutex<ExecutableTemplate>,
}
impl AttachedPlugin {
    // Native Context.SetBehaviourSource82ceed30 is mutable independently of
    // controller ownership. Zero encodes NULL; every u32 entity ID remains valid.
    pub(crate) fn source(&self)->Option<u32>{
        self.source.load(Ordering::Relaxed).checked_sub(1).map(|id|id as u32)
    }
    pub(crate) fn clear_source(&self){self.source.store(0,Ordering::Relaxed);}
}
impl ProcessedTransferable {
    ///Copy constructs a NEW controller using the same delegate payloads and
    ///shared expression declarations; it does not clone current runtime state.
    pub(crate) fn copy_for_source(self:&Arc<Self>,source:u32)->Result<AttachedPlugin,String>{
        let template=self.controller.lock().map_err(|_|"plugin template controller lock")?;
        let controller=match &*template {
            ExecutableTemplate::LookAt(value)=>ExecutableTemplate::LookAt(LookAtController::construct(Arc::clone(&value.bindings))),
            ExecutableTemplate::Trash(value)=>ExecutableTemplate::Trash(TrashController::construct(Arc::clone(&value.bindings))),
            ExecutableTemplate::Waypoint(value)=>ExecutableTemplate::Waypoint(super::waypoint::WaypointController::construct(Arc::clone(&value.bindings))),
            ExecutableTemplate::Spectate(value)=>ExecutableTemplate::Spectate(super::spectate::SpectateController::construct(Arc::clone(&value.bindings))),
        };
        Ok(AttachedPlugin{template:Arc::clone(self),source:AtomicU64::new(u64::from(source)+1),identity:Arc::new(()),controller:Mutex::new(controller)})
    }
}
impl PropMan {
    ///Prop.TransferBehaviourToSink82e68230, after the caller's real transfer
    ///eligibility checks. Manager82710aac requires an empty plugin before entry.
    pub(crate) fn transfer_behaviour_to_pedestrian(&mut self,source:u32,entity:&mut PedestrianConstruction)->Result<(),String>{
        if entity.plugin.is_some(){return Err("plugin transfer requires source-checked empty sink".into());}
        let prop=self.entities.get_mut(source).ok_or("plugin transfer source absent")?;
        let template=prop.transferable.as_ref().ok_or("plugin transfer processed template absent")?;
        let copy=template.copy_for_source(source)?;
        prop.behaviour_copies.push(Arc::clone(&copy.identity));
        entity.plugin=Some(Arc::new(copy));
        entity.plugin_removal_requested=false; //SetBehaviourPlugin82e4c4e4
        prop.behaviour_sinks.push(entity.id); //AddBehaviourSink82e66ca8
        Ok(())
    }
}
impl crate::pedestrian_graph::backend::frame::FrameServices<'_>{
    pub(crate) fn update_plugin(&mut self,entity:&mut PedestrianConstruction,dt:f32)->Result<(),String>{self.run_plugin(entity,Some(dt))}
    pub(crate) fn cleanly_end_plugin(&mut self,entity:&mut PedestrianConstruction)->Result<(),String>{self.run_plugin(entity,None)}
    fn run_plugin(&mut self,entity:&mut PedestrianConstruction,dt:Option<f32>)->Result<(),String>{
        let Some(plugin)=entity.plugin.as_ref().map(Arc::clone) else{return Ok(());};
        let mut retained=plugin.controller.lock().map_err(|_|"attached plugin controller lock")?;
        if let ExecutableTemplate::Spectate(controller)=&mut *retained{return controller.run(entity,plugin.source(),self,dt);}
        self.with_navigation(entity,None,|entity,facts,world,numeric|match (&mut *retained,dt){
            (ExecutableTemplate::LookAt(c),Some(dt))=>c.update(dt,entity,plugin.source(),facts,world,numeric),
            (ExecutableTemplate::LookAt(c),None)=>c.end(entity,plugin.source(),facts,world,numeric),
            (ExecutableTemplate::Trash(c),Some(dt))=>c.update(dt,entity,plugin.source(),world.props,world.cvars,world.random,numeric),
            (ExecutableTemplate::Trash(c),None)=>c.end(entity,plugin.source(),world.props,world.cvars,world.random,numeric),
            (ExecutableTemplate::Waypoint(c),Some(dt))=>c.update(dt,entity,plugin.source(),facts,world,numeric),
            (ExecutableTemplate::Waypoint(c),None)=>c.end(entity,plugin.source(),facts,world,numeric),
            (ExecutableTemplate::Spectate(_),_)=>unreachable!("handled with full frame above"),
        })
    }
}
impl PedestrianConstruction {
    /// Pedestrian.RemoveBehaviourPlugin82e79ca8 then base82e4c3e8.
    pub(crate) fn remove_plugin(&mut self,props:&mut PropMan)->Result<(),String>{
        if let Some(plugin)=self.plugin.as_ref(){
            self.head_tracking.as_mut().ok_or("plugin removal head owner absent")?.on_remove_behaviour_source(plugin.source());
        }
        self.clear_current_waypoint(props)?;
        if let Some(source)=self.plugin.as_ref().and_then(|p|p.source()){
            let prop=props.entities.get_mut(source).ok_or("plugin removal source absent")?;
            if let Some(index)=prop.behaviour_sinks.iter().position(|id|*id==self.id){prop.behaviour_sinks.remove(index);}
        }
        self.clear_plugin(props)
    }
    ///The null SetBehaviourPlugin deletion suffix: native destructor82cf1e48
    ///signals source copy deletion; it does not remove the source's sink list.
    ///Callers perform CleanlyEndAllPluginBehaviours in their original order.
    pub(crate) fn clear_plugin(&mut self,props:&mut PropMan)->Result<(),String>{
        if let Some(plugin)=self.plugin.as_ref() {
            if let Some(source)=plugin.source() {
                let prop=props.entities.get_mut(source).ok_or("plugin destructor source absent")?;
                if let Some(index)=prop.behaviour_copies.iter().position(|id|Arc::ptr_eq(id,&plugin.identity)) {
                    prop.behaviour_copies.remove(index); //82e66d58 first matching pointer
                }
            }
        }
        self.plugin=None;
        self.plugin_removal_requested=false;
        Ok(())
    }
}
