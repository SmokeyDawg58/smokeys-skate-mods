//! BehaviourMan cache Process for implemented executable plugin families.
use super::{Family,lookat::{LookAtBindings,LookAtController}};
use crate::{behaviour_graph_runtime::{TransferableBehaviour,TransferableBehaviourGraph},
    generated_onboard_state_graph::{select_onboard_controller,OnboardControllerSelection},
    actor_construction::graph_startup::ActorGraphStartupOwners};
use std::sync::{Arc,Mutex};

pub(crate) enum ExecutableTemplate {LookAt(LookAtController),Trash(super::trash::TrashController),Waypoint(super::waypoint::WaypointController),Spectate(super::spectate::SpectateController)}
pub(crate) struct ProcessedTransferable {
    pub(crate) declaration:TransferableBehaviour,
    pub(crate) controller:Mutex<ExecutableTemplate>,
}
impl ProcessedTransferable {
    /// HC.Load82cef1e0 retains HashValue32 of the actual root state name.
    /// CanUseBehaviourSource82e86140 uses this identity for the retry timer.
    pub(crate) fn root_state_hash(&self)->Result<u32,String>{
        let controller=self.controller.lock().map_err(|_|"plugin template controller lock")?;
        let family=match &*controller {
            ExecutableTemplate::LookAt(value)=>value.controller.family,
            ExecutableTemplate::Trash(value)=>value.controller.family,
            ExecutableTemplate::Waypoint(value)=>value.controller.family,
            ExecutableTemplate::Spectate(value)=>value.controller.family,
        };
        let declarations=family.declarations();
        let root=declarations.states.first().ok_or("native plugin root-state assertion")?;
        Ok(skate2_formats::hash::sk8_hash_value_32(root.name.as_bytes()))
    }
}
pub(crate) fn load(owners:&ActorGraphStartupOwners,path:&str)->Result<Arc<ProcessedTransferable>,String>{
    let mut cache=owners.plugin_cache.lock().map_err(|_|"BehaviourGraph cache lock")?;
    cache.load_with(path,false,|normalized|{
        let compiled=crate::graph_asset_cache::compiled_graph_path(normalized).map_err(|e|e.to_string())?;
        let mounts=owners.plugin_mounts.as_ref().ok_or("plugin mounted assets absent")?;
        let bytes=mounts.read_path(&compiled).map_err(|e|e.to_string())?;
        let wire=skate2_formats::behaviour::BehaviourBinaryElement::parse(&bytes).map_err(|e|e.to_string())?;
        let declaration=TransferableBehaviourGraph::build(&wire).map_err(|e|e.to_string())?.root;
        let enabled=owners.living_world_enablement().map_err(|e|e.to_string())?;
        let selection=select_onboard_controller(declaration.file.as_bytes(),true,false,enabled).map_err(|e|e.to_string())?;
        let base=declaration.file.rsplit(['\\','/']).next().ok_or("plugin graph path missing")?;
        let family=Family::from_basename(base).ok_or_else(||format!("unsupported plugin graph {base}"))?;
        // Native Process82cf0a40 zeroes Context then HC.Load(file,true).
        // Generated selection precedes any dynamic StateGraph cache lookup.
        let controller=match (family,selection){
            (Family::Lookat,OnboardControllerSelection::GeneratedOther{delegate_index:8})=>{
                let bindings=owners.plugin_lookat_bindings.get_or_init(||LookAtBindings::construct().map(Arc::new)).clone()?;
                ExecutableTemplate::LookAt(LookAtController::construct(bindings))
            }
            (Family::Usetrashbin,OnboardControllerSelection::GeneratedOther{delegate_index:13})=>{
                let bindings=owners.plugin_trash_bindings.get_or_init(||super::trash::TrashBindings::construct().map(Arc::new)).clone()?;
                ExecutableTemplate::Trash(super::trash::TrashController::construct(bindings))
            }
            (Family::Sit,OnboardControllerSelection::GeneratedOther{delegate_index:9})|
            (Family::Useatm,OnboardControllerSelection::GeneratedOther{delegate_index:11})|
            (Family::Usenewspaperbox,OnboardControllerSelection::GeneratedOther{delegate_index:12})|
            (Family::Usevendingmachine,OnboardControllerSelection::GeneratedOther{delegate_index:14})|
            (Family::Usewaterfountain,OnboardControllerSelection::GeneratedOther{delegate_index:15})=>{
                let index=match family{Family::Useatm=>0,Family::Usenewspaperbox=>1,Family::Usevendingmachine=>2,Family::Usewaterfountain=>3,_=>4};
                let bindings=owners.plugin_waypoint_bindings[index].get_or_init(||super::waypoint::WaypointBindings::construct(family).map(Arc::new)).clone()?;
                ExecutableTemplate::Waypoint(super::waypoint::WaypointController::construct(bindings))
            }
            (Family::Spectate,OnboardControllerSelection::GeneratedOther{delegate_index:10})|
            (Family::ActortrackerSpectate,OnboardControllerSelection::GeneratedOther{delegate_index:6})=>{
                let index=usize::from(family==Family::ActortrackerSpectate);
                let bindings=owners.plugin_spectate_bindings[index].get_or_init(||super::spectate::SpectateBindings::construct(family).map(Arc::new)).clone()?;
                ExecutableTemplate::Spectate(super::spectate::SpectateController::construct(bindings))
            }
            (_,OnboardControllerSelection::DynamicFallback)=>return Err("plugin dynamic controller factory binding absent".into()),
            _=>return Err(format!("plugin generated factory binding absent: {base}")),
        };
        Ok::<_,String>(Some(ProcessedTransferable{declaration,controller:Mutex::new(controller)}))
    }).map_err(|e|e.to_string())?.ok_or("native Prop SetTransferableBehaviour null assertion".into())
}
