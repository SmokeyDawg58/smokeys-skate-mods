//! Concrete TemplateHC backend over the retained entity and declaration owners.
//! Context occurrence changes synchronously before each native callback.
#[path = "pedestrian_graph_frame.rs"]
pub(crate) mod frame;
use super::{PedestrianGraph,PedestrianGraphBackend,PedestrianAiBindings,
    behaviour_binding::{LocalBehaviourBindings,LocalDispatch,Phase},condition_binding::{ConditionFrame,ConditionServices}};
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    generated_action_load::runtime::ActionUpdateError as Error,generated_graph_selection::Mask,
    reckoning_numeric::HostReckoningNumeric};

/// Concrete world callbacks receive the actual occurrence and source address.
/// No fallback can report success merely because a binding is absent.
pub(crate) trait AiWorldServices: ConditionServices {
    fn create_nonlocal_instance(&mut self,entity:&mut PedestrianConstruction,occurrence:u16,source:u32)->Result<(),String>;
    fn dispatch_nonlocal(&mut self,entity:&mut PedestrianConstruction,bindings:&mut LocalBehaviourBindings,
        occurrence:u16,phase:Phase,source:u32,dt:f32)->Result<(),String>;
    fn release_nonlocal_instance(&mut self,entity:&mut PedestrianConstruction,occurrence:u16)->Result<(),String>;
}
struct Backend<'a,S>{entity:&'a mut PedestrianConstruction,bindings:&'a mut PedestrianAiBindings,
    services:&'a mut S,failure:Option<String>,context_instance:Option<u16>}
impl<S:AiWorldServices> Backend<'_,S>{
    fn capture<T>(&mut self,result:Result<T,String>,error:Error)->Result<T,Error>{
        result.map_err(|message|{self.failure=Some(message);error})
    }
    fn dispatch(&mut self,occurrence:u16,phase:Phase,dt:f32)->Result<(),String>{
        self.context_instance=Some(occurrence);
        match self.bindings.behaviours.dispatch(occurrence,phase,self.entity)? {
            LocalDispatch::Handled=>Ok(()),
            LocalDispatch::Nonlocal{source}=>self.services.dispatch_nonlocal(self.entity,
                &mut self.bindings.behaviours,occurrence,phase,source,dt),
        }
    }
}
impl<S:AiWorldServices> PedestrianGraphBackend for Backend<'_,S>{
    fn condition(&mut self,_graph:&PedestrianGraph,_mask:Mask,condition:u16,_state:u16)->Result<bool,Error>{
        let result=self.bindings.conditions.evaluate(condition,&mut ConditionFrame{entity:self.entity,
            services:self.services,numeric:&mut HostReckoningNumeric});
        self.capture(result,Error::PendingExpression(condition))
    }
    fn enabled(&self,occurrence:u16)->Result<bool,Error>{
        self.bindings.behaviours.enabled(occurrence).map_err(|_|Error::InvalidActiveSet)
    }
    fn create_instance(&mut self,_graph:&PedestrianGraph,occurrence:u16)->Result<(),Error>{
        let result=(||match self.bindings.behaviours.create_instance(occurrence)?{
            LocalDispatch::Handled=>Ok(()),LocalDispatch::Nonlocal{source}=>
                self.services.create_nonlocal_instance(self.entity,occurrence,source)})();
        self.capture(result,Error::MissingInstance(occurrence))
    }
    fn begin(&mut self,graph:&PedestrianGraph,occurrence:u16)->Result<(),Error>{
        let result=self.dispatch(occurrence,Phase::Begin,graph.delta_time);
        self.capture(result,Error::InvalidActiveSet)
    }
    fn end_and_remove(&mut self,graph:&PedestrianGraph,occurrence:u16)->Result<(),Error>{
        let result=(||{self.dispatch(occurrence,Phase::End,graph.delta_time)?;
            self.services.release_nonlocal_instance(self.entity,occurrence)})();
        self.capture(result,Error::InvalidActiveSet)
    }
    fn transition_hook(&mut self,_graph:&PedestrianGraph,hook:u16,_previous:u16)->Result<(),Error>{
        //Current native transition table has no hook section; never silently
        //accept a new nonempty hook when that declaration changes.
        Err(Error::PendingTransitionHook(hook))
    }
    fn generated_update(&mut self,graph:&PedestrianGraph,source:u32,active:&[u16])->Result<(),Error>{
        let result=super::updates::dispatch(source,active,|occurrence,callback|{
            self.context_instance=Some(occurrence);
            if callback==super::updates::UpdateCallback::Noop {return Ok(());}
            self.dispatch(occurrence,Phase::Update,graph.delta_time)
        }).map_err(|e|format!("pedestrian generated update {source:#x}: {e:?}"));
        self.capture(result,Error::PendingStateUpdate(source))
    }
}
impl PedestrianConstruction{
    /// Entity.GetHeading825f4880, MeshNavigation.Seek82e75b68 and
    /// ResolvePath82e76120 read these facts from their original retained owners.
    pub(crate) fn navigation_entity_facts(&self,
        cvars:&crate::input_cvar_runtime::InputCvarReferences,
    )->Result<crate::pedestrian_navigation::mesh::NavigationEntityFacts,String>{
        let core=self.core.as_ref().ok_or("navigation facts require entity core")?;
        let rig=self.nav_rig.as_ref().ok_or("navigation facts require NavRig")?;
        let (_,heading)=crate::camera_compass::runtime::math::euler_angles(
            core.transform[2].into(),&mut HostReckoningNumeric)
            .map_err(|e|format!("entity heading: {e:?}"))?;
        let depths=cvars.try_pedestrian_astar_depths().map_err(|e|format!("AStar depths: {e:?}"))?;
        let max_astar_depth=if self.security_guard.is_some(){depths[3]}
            else if self.navigation_rig_enabled && core.current_losim<2 {depths[core.current_losim as usize]}
            else{depths[2]};
        Ok(crate::pedestrian_navigation::mesh::NavigationEntityFacts{
            position:core.transform[3],heading,cell:rig.mesh_info.cell.clone(),max_astar_depth,
            can_astar:self.navigation_rig_enabled,
            //Update82e7de5c tests LOSim != NoFi, including LoFi validation.
            high_fidelity:core.current_losim!=2,
            last_valid_cell_position:rig.mesh_info.last_valid_cell_position,
        })
    }
    /// EndAllBehaviours82934830 uses the same occurrence callbacks as Update;
    /// successfully ended suffix entries remain removed on checked failure.
    pub(crate) fn end_ai_graph(&mut self,services:&mut impl AiWorldServices)->Result<(),String>{
        let Some(mut bindings)=self.ai_bindings.take() else {return Ok(());};
        let graph=std::mem::take(&mut self.graph);
        let mut instances=std::mem::take(&mut self.behaviours);
        let mut backend=Backend{entity:self,bindings:&mut bindings,services,failure:None,context_instance:None};
        let result=(||{
            while let Some(&occurrence)=instances.active.last(){
                backend.end_and_remove(&graph,occurrence)?;
                instances.active.pop();
            }
            Ok::<_,Error>(())
        })();
        let failure=backend.failure.take();
        drop(backend);
        self.graph=graph;self.behaviours=instances;self.ai_bindings=Some(bindings);
        result.map_err(|error|failure.unwrap_or_else(||error.to_string()))
    }
    /// No graph/instance clone: move sole runners aside solely for disjoint
    /// mutable access to callback fields, and restore their partial state on
    /// every checked failure. No callback observes a replacement graph owner.
    pub(crate) fn update_ai_graph(&mut self,dt:f32,services:&mut impl AiWorldServices)->Result<(),String>{
        let mut bindings=self.ai_bindings.take().ok_or("pedestrian AI graph not loaded")?;
        let delegate=bindings.delegate;
        let mut graph=std::mem::take(&mut self.graph);
        let mut instances=std::mem::take(&mut self.behaviours);
        let mut backend=Backend{entity:self,bindings:&mut bindings,services,failure:None,context_instance:None};
        let result=graph.update_delegate(delegate,dt,&mut instances,&mut backend);
        let failure=backend.failure.take();
        drop(backend);
        self.graph=graph;self.behaviours=instances;self.ai_bindings=Some(bindings);
        result.map_err(|error|failure.unwrap_or_else(||error.to_string()))
    }
}
