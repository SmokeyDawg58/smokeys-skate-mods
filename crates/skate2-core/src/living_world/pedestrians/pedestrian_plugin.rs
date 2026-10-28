//! Generated plugin controllers over the same TemplateHC runtime as Pedestrian.
use crate::pedestrian_graph::{PedestrianGraph,PedestrianGraphBehaviours,PedestrianGraphBackend,GeneratedDeclarations};
use crate::generated_action_load::runtime::ActionUpdateError as Error;
#[path="pedestrian_plugin_data.rs"] pub(crate) mod data;
#[path="pedestrian_plugin_expressions.rs"] mod expressions;
#[path="pedestrian_plugin_direct.rs"] pub(crate) mod direct;
#[path="pedestrian_plugin_lookat.rs"] pub(crate) mod lookat;
#[path="pedestrian_plugin_trash.rs"] pub(crate) mod trash;
#[path="pedestrian_plugin_waypoint.rs"] pub(crate) mod waypoint;
#[path="pedestrian_plugin_spectate.rs"] pub(crate) mod spectate;
#[path="pedestrian_plugin_load.rs"] pub(crate) mod load;
#[path="pedestrian_plugin_attached.rs"] pub(crate) mod attached;
#[path="pedestrian_plugin_transfer.rs"] pub(crate) mod transfer;
#[derive(Clone,Copy,Debug,PartialEq,Eq)] pub(crate) enum Family { ActortrackerSpectate,Conversation,Lookat,Sit,Spectate,Useatm,Usenewspaperbox,Usetrashbin,Usevendingmachine,Usewaterfountain }
impl Family {
pub(crate) fn from_basename(name:&str)->Option<Self>{Some(match name {
"actortracker_spectate.xml"=>Self::ActortrackerSpectate,
"conversation.xml"=>Self::Conversation,
"lookat.xml"=>Self::Lookat,
"sit.xml"=>Self::Sit,
"spectate.xml"=>Self::Spectate,
"useatm.xml"=>Self::Useatm,
"usenewspaperbox.xml"=>Self::Usenewspaperbox,
"usetrashbin.xml"=>Self::Usetrashbin,
"usevendingmachine.xml"=>Self::Usevendingmachine,
"usewaterfountain.xml"=>Self::Usewaterfountain,
_=>return None,})}
pub(crate) fn declarations(self)->GeneratedDeclarations {match self {
Self::ActortrackerSpectate=>GeneratedDeclarations{states:&data::actortracker_spectate::STATES,transitions:&data::actortracker_spectate::TRANSITIONS,indices:&data::actortracker_spectate::INDICES,behaviours:&data::actortracker_spectate::BEHAVIOURS,active_capacity:5},
Self::Conversation=>GeneratedDeclarations{states:&data::conversation::STATES,transitions:&data::conversation::TRANSITIONS,indices:&data::conversation::INDICES,behaviours:&data::conversation::BEHAVIOURS,active_capacity:8},
Self::Lookat=>GeneratedDeclarations{states:&data::lookat::STATES,transitions:&data::lookat::TRANSITIONS,indices:&data::lookat::INDICES,behaviours:&data::lookat::BEHAVIOURS,active_capacity:3},
Self::Sit=>GeneratedDeclarations{states:&data::sit::STATES,transitions:&data::sit::TRANSITIONS,indices:&data::sit::INDICES,behaviours:&data::sit::BEHAVIOURS,active_capacity:8},
Self::Spectate=>GeneratedDeclarations{states:&data::spectate::STATES,transitions:&data::spectate::TRANSITIONS,indices:&data::spectate::INDICES,behaviours:&data::spectate::BEHAVIOURS,active_capacity:9},
Self::Useatm=>GeneratedDeclarations{states:&data::useatm::STATES,transitions:&data::useatm::TRANSITIONS,indices:&data::useatm::INDICES,behaviours:&data::useatm::BEHAVIOURS,active_capacity:6},
Self::Usenewspaperbox=>GeneratedDeclarations{states:&data::usenewspaperbox::STATES,transitions:&data::usenewspaperbox::TRANSITIONS,indices:&data::usenewspaperbox::INDICES,behaviours:&data::usenewspaperbox::BEHAVIOURS,active_capacity:6},
Self::Usetrashbin=>GeneratedDeclarations{states:&data::usetrashbin::STATES,transitions:&data::usetrashbin::TRANSITIONS,indices:&data::usetrashbin::INDICES,behaviours:&data::usetrashbin::BEHAVIOURS,active_capacity:3},
Self::Usevendingmachine=>GeneratedDeclarations{states:&data::usevendingmachine::STATES,transitions:&data::usevendingmachine::TRANSITIONS,indices:&data::usevendingmachine::INDICES,behaviours:&data::usevendingmachine::BEHAVIOURS,active_capacity:6},
Self::Usewaterfountain=>GeneratedDeclarations{states:&data::usewaterfountain::STATES,transitions:&data::usewaterfountain::TRANSITIONS,indices:&data::usewaterfountain::INDICES,behaviours:&data::usewaterfountain::BEHAVIOURS,active_capacity:6},
}}
pub(crate) fn attributes(self)->&'static [crate::generated_action_load::Attribute]{match self {
Self::ActortrackerSpectate=>&data::actortracker_spectate::ATTRIBUTES,
Self::Conversation=>&data::conversation::ATTRIBUTES,
Self::Lookat=>&data::lookat::ATTRIBUTES,
Self::Sit=>&data::sit::ATTRIBUTES,
Self::Spectate=>&data::spectate::ATTRIBUTES,
Self::Useatm=>&data::useatm::ATTRIBUTES,
Self::Usenewspaperbox=>&data::usenewspaperbox::ATTRIBUTES,
Self::Usetrashbin=>&data::usetrashbin::ATTRIBUTES,
Self::Usevendingmachine=>&data::usevendingmachine::ATTRIBUTES,
Self::Usewaterfountain=>&data::usewaterfountain::ATTRIBUTES,
}}
pub(crate) fn conditions(self)->&'static [crate::generated_action_load::ConditionDeclaration]{match self {
Self::ActortrackerSpectate=>&data::actortracker_spectate::CONDITIONS,
Self::Conversation=>&data::conversation::CONDITIONS,
Self::Lookat=>&data::lookat::CONDITIONS,
Self::Sit=>&data::sit::CONDITIONS,
Self::Spectate=>&data::spectate::CONDITIONS,
Self::Useatm=>&data::useatm::CONDITIONS,
Self::Usenewspaperbox=>&data::usenewspaperbox::CONDITIONS,
Self::Usetrashbin=>&data::usetrashbin::CONDITIONS,
Self::Usevendingmachine=>&data::usevendingmachine::CONDITIONS,
Self::Usewaterfountain=>&data::usewaterfountain::CONDITIONS,
}}
}
pub(crate) struct PluginController {pub(crate) family:Family,pub(crate) graph:PedestrianGraph,instances:PedestrianGraphBehaviours}
impl PluginController {
pub(crate) fn end_all(&mut self,backend:&mut impl PedestrianGraphBackend)->Result<(),Error>{self.graph.end_all(&mut self.instances,backend)}
pub(crate) fn construct(family:Family)->Self {Self {family,graph:PedestrianGraph::default(),instances:PedestrianGraphBehaviours::default()}}
pub(crate) fn update(&mut self,dt:f32,backend:&mut impl PedestrianGraphBackend)->Result<(),Error>{
let family=self.family; let declarations=family.declarations();
match family {
Family::ActortrackerSpectate=>self.graph.update_declarations::<3>(dt,&mut self.instances,backend,&declarations,|mask,e,c|expressions::evaluate(family,mask,e,c)),
Family::Conversation=>self.graph.update_declarations::<7>(dt,&mut self.instances,backend,&declarations,|mask,e,c|expressions::evaluate(family,mask,e,c)),
Family::Lookat=>self.graph.update_declarations::<2>(dt,&mut self.instances,backend,&declarations,|mask,e,c|expressions::evaluate(family,mask,e,c)),
Family::Sit=>self.graph.update_declarations::<6>(dt,&mut self.instances,backend,&declarations,|mask,e,c|expressions::evaluate(family,mask,e,c)),
Family::Spectate=>self.graph.update_declarations::<3>(dt,&mut self.instances,backend,&declarations,|mask,e,c|expressions::evaluate(family,mask,e,c)),
Family::Useatm=>self.graph.update_declarations::<5>(dt,&mut self.instances,backend,&declarations,|mask,e,c|expressions::evaluate(family,mask,e,c)),
Family::Usenewspaperbox=>self.graph.update_declarations::<6>(dt,&mut self.instances,backend,&declarations,|mask,e,c|expressions::evaluate(family,mask,e,c)),
Family::Usetrashbin=>self.graph.update_declarations::<2>(dt,&mut self.instances,backend,&declarations,|mask,e,c|expressions::evaluate(family,mask,e,c)),
Family::Usevendingmachine=>self.graph.update_declarations::<6>(dt,&mut self.instances,backend,&declarations,|mask,e,c|expressions::evaluate(family,mask,e,c)),
Family::Usewaterfountain=>self.graph.update_declarations::<6>(dt,&mut self.instances,backend,&declarations,|mask,e,c|expressions::evaluate(family,mask,e,c)),
}}
}
