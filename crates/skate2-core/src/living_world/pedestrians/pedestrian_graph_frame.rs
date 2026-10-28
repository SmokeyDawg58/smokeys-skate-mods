//! Borrowed world owners shared by pedestrian AI and motion callbacks.
//! The selected entity remains the sole EntityMan payload and is passed to
//! callbacks separately; this frame never stores a copied entity state.
#[path = "pedestrian_graph_frame_big_black.rs"]
mod big_black;
#[path = "pedestrian_graph_frame_takedown_callbacks.rs"]
mod takedown_callbacks;
#[path = "pedestrian_graph_frame_guard.rs"]
mod guard;
#[path = "pedestrian_graph_frame_conditions.rs"]
mod conditions;
#[path = "pedestrian_graph_frame_callbacks.rs"]
mod callbacks;
#[path = "pedestrian_graph_frame_want_callbacks.rs"]
mod want_callbacks;
#[path = "pedestrian_graph_frame_takedown_selection.rs"]
mod takedown_selection;
#[path = "pedestrian_graph_frame_guard_behaviours.rs"]
mod guard_behaviours;
#[path = "pedestrian_graph_frame_chase.rs"]
mod chase_registration;
#[path = "pedestrian_graph_frame_chase_callbacks.rs"]
mod chase_callbacks;
#[path = "pedestrian_graph_frame_navigation.rs"]
mod navigation;
#[path = "pedestrian_graph_frame_knowledge.rs"]
mod knowledge;
#[path = "pedestrian_graph_frame_prestate.rs"]
mod prestate;
#[path = "pedestrian_graph_frame_colliding.rs"]
mod colliding;
#[path = "pedestrian_graph_frame_modifiers.rs"]
mod modifiers;
#[path = "pedestrian_graph_frame_mood.rs"]
mod mood;
#[path = "pedestrian_graph_frame_mood_results.rs"]
mod mood_results;
#[path = "pedestrian_graph_frame_threats.rs"]
mod threats;
#[path = "pedestrian_graph_frame_chase_update.rs"]
mod chase_update;
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    living_world_grind_services::{waypoints::PropMan,dmo_objects::DmoObjectMan},
    living_world_chase::{ChaseMan,TargetableId},living_world_mesh_store::MeshMan,
    living_world_rain::RainMan,physical_player_assembly::PhysicalAssemblyWorld,
    native_skater_random::NativeSkaterRandom,input_cvar_runtime::InputCvarReferences,
    world_painter_layer::WorldPainterLayerMan,attribute_runtime::AttributeRuntimeVault,
    actor_construction::ActorConstruction,pedestrian_navigation::wander::ResolvedAvoidVolumes};
use std::sync::Arc;

/// Borrows all callbacks from the sole live roster and auxiliary owners.
/// Callers already holding LivingWorld/world locks must use this directly.
pub(crate) fn with_entity<R>(
    living:&mut crate::living_world_grind_services::LivingWorldGrindServices,
    world:&mut PhysicalAssemblyWorld,id:u32,cvars:&InputCvarReferences,
    actors:&[Arc<ActorConstruction>],painter:&WorldPainterLayerMan,
    vault:&Arc<AttributeRuntimeVault>,avoid:&ResolvedAvoidVolumes,
    callback:impl FnOnce(&mut PedestrianConstruction,&mut FrameServices<'_>)->Result<R,String>,
)->Result<R,String>{
    with_entity_sim(living,world,id,cvars,actors,painter,vault,avoid,None,None,callback)
}

pub(crate) fn with_entity_sim<R>(
    living:&mut crate::living_world_grind_services::LivingWorldGrindServices,
    world:&mut PhysicalAssemblyWorld,id:u32,cvars:&InputCvarReferences,
    actors:&[Arc<ActorConstruction>],painter:&WorldPainterLayerMan,
    vault:&Arc<AttributeRuntimeVault>,avoid:&ResolvedAvoidVolumes,
    simulation_random:Option<&mut crate::native_skater_random::SimulationCommandRandom>,
    ground:Option<&crate::static_ground_pool::StaticGroundPool>,
    callback:impl FnOnce(&mut PedestrianConstruction,&mut FrameServices<'_>)->Result<R,String>,
)->Result<R,String>{
    let mut random=living.random.take().ok_or("AI Manager RNG absent")?;
    let result=with_entity_and_random_sim(living,world,id,cvars,actors,painter,vault,avoid,&mut random,simulation_random,ground,callback);
    living.random=Some(random);
    result
}

/// Census already borrows the sole Manager random stream. This entry shares
/// that borrow without manufacturing another random owner for End callbacks.
pub(crate) fn with_entity_and_random<R>(
    living:&mut crate::living_world_grind_services::LivingWorldGrindServices,
    world:&mut PhysicalAssemblyWorld,id:u32,cvars:&InputCvarReferences,
    actors:&[Arc<ActorConstruction>],painter:&WorldPainterLayerMan,
    vault:&Arc<AttributeRuntimeVault>,avoid:&ResolvedAvoidVolumes,random:&mut NativeSkaterRandom,
    callback:impl FnOnce(&mut PedestrianConstruction,&mut FrameServices<'_>)->Result<R,String>,
)->Result<R,String>{
    with_entity_and_random_sim(living,world,id,cvars,actors,painter,vault,avoid,random,None,None,callback)
}

pub(crate) fn with_entity_and_random_sim<R>(
    living:&mut crate::living_world_grind_services::LivingWorldGrindServices,
    world:&mut PhysicalAssemblyWorld,id:u32,cvars:&InputCvarReferences,
    actors:&[Arc<ActorConstruction>],painter:&WorldPainterLayerMan,
    vault:&Arc<AttributeRuntimeVault>,avoid:&ResolvedAvoidVolumes,random:&mut NativeSkaterRandom,
    simulation_random:Option<&mut crate::native_skater_random::SimulationCommandRandom>,
    ground:Option<&crate::static_ground_pool::StaticGroundPool>,
    callback:impl FnOnce(&mut PedestrianConstruction,&mut FrameServices<'_>)->Result<R,String>,
)->Result<R,String>{
    let (mut owners,dmos,locators)=FrameOwners::split(living)?;
    owners.with_entity(world,dmos,locators,id,cvars,actors,painter,vault,avoid,random,simulation_random,ground,callback)
}

/// Split borrows of existing LivingWorld owners. DMO lifecycle callers already
/// own DMOMan/locators separately; pass those at the synchronous callback site.
/// This stores no alternate roster, world, state, or cleanup queue.
pub(crate) struct FrameOwners<'a>{
    pub(crate) pedestrians:&'a mut crate::living_world_grind_services::entities::EntityMan<40,PedestrianConstruction>,
    pub(crate) mood:&'a crate::pedestrian_mood::MoodMan,
    pub(crate) game_sim_events:&'a crate::hermes_game_sim_queue::GameSimEventRoute,
    pub(crate) big_black_entity:&'a Option<u32>,
    pub(crate) props:&'a mut PropMan,
    pub(crate) mesh:&'a MeshMan,
    pub(crate) rain:&'a mut RainMan,
    pub(crate) chase:&'a mut ChaseMan,
    pub(crate) vehicles:&'a crate::living_world_grind_services::entities::EntityMan<24,crate::living_world_grind_services::vehicle_entity::Vehicle>,
}
impl<'a> FrameOwners<'a>{
    pub(crate) fn split(living:&'a mut crate::living_world_grind_services::LivingWorldGrindServices)
        ->Result<(Self,&'a mut DmoObjectMan,&'a mut crate::living_world_grind_services::dmo_locators::DynamicObjectLocatorMan),String>{
        Ok((Self{
            pedestrians:living.pedestrian_entities.as_deref_mut().ok_or("AI pedestrian roster absent")?,
            mood:living.mood.as_ref().ok_or("AI MoodMan absent")?,
            game_sim_events:living.game_sim_events.as_deref().ok_or("AI GameSim broadcaster absent")?,
            big_black_entity:&living.big_black_entity,
            props:living.props.as_deref_mut().ok_or("AI PropMan absent")?,
            mesh:living.mesh.as_ref().ok_or("AI MeshMan absent")?,
            rain:living.rain.as_mut().ok_or("AI RainMan absent")?,
            chase:living.chase.as_mut().ok_or("AI ChaseMan absent")?,
            vehicles:living.grind_vehicle_entities.as_deref().ok_or("AI VehicleMan absent")?,
        },living.dmo_objects.as_deref_mut().ok_or("AI DMOMan absent")?,
            living.dmo_locators.as_deref_mut().ok_or("AI DMO locators absent")?))
    }
    pub(crate) fn with_entity<R>(&mut self,
        world:&mut PhysicalAssemblyWorld,dmos:&mut DmoObjectMan,
        dmo_locators:&mut crate::living_world_grind_services::dmo_locators::DynamicObjectLocatorMan,
        id:u32,cvars:&InputCvarReferences,actors:&[Arc<ActorConstruction>],
        painter:&WorldPainterLayerMan,vault:&Arc<AttributeRuntimeVault>,avoid:&ResolvedAvoidVolumes,
        random:&mut NativeSkaterRandom,
        simulation_random:Option<&mut crate::native_skater_random::SimulationCommandRandom>,
        ground:Option<&crate::static_ground_pool::StaticGroundPool>,
        callback:impl FnOnce(&mut PedestrianConstruction,&mut FrameServices<'_>)->Result<R,String>,
    )->Result<R,String>{
    let (entity,mut others,pedestrian_spatial)=self.pedestrians.split_entity_and_peers_and_spatial_mut(id).ok_or("AI pedestrian roster or spatial initialization absent")?;
    let mut frame=FrameServices{
        mood:self.mood,
        world,cvars,actors,painter,vault,avoid,others:&mut others,entity_id:id,pedestrian_spatial,
        game_sim_events:self.game_sim_events,
        big_black_entity:self.big_black_entity,
        random,simulation_random,ground,
        dmos,dmo_locators,props:self.props,mesh:self.mesh,rain:self.rain,chase:self.chase,vehicles:self.vehicles,
    };
    callback(entity,&mut frame)
    }
}

pub(crate) struct FrameServices<'a> {
    pub(crate) mood: &'a crate::pedestrian_mood::MoodMan,
    pub(crate) big_black_entity:&'a Option<u32>,
    pub(crate) pedestrian_spatial:&'a crate::living_world_grind_services::dmo_locators::LocatorKdTree,
    pub(crate) entity_id:u32,
    pub(crate) game_sim_events:&'a crate::hermes_game_sim_queue::GameSimEventRoute,
    pub(crate) vehicles: &'a crate::living_world_grind_services::entities::EntityMan<24,
        crate::living_world_grind_services::vehicle_entity::Vehicle>,
    pub(crate) world:&'a mut PhysicalAssemblyWorld,
    pub(crate) cvars:&'a InputCvarReferences,
    pub(crate) random:&'a mut NativeSkaterRandom,
    pub(crate) simulation_random:Option<&'a mut crate::native_skater_random::SimulationCommandRandom>,
    pub(crate) ground:Option<&'a crate::static_ground_pool::StaticGroundPool>,
    pub(crate) dmos:&'a mut DmoObjectMan,
    pub(crate) dmo_locators:&'a mut crate::living_world_grind_services::dmo_locators::DynamicObjectLocatorMan,
    pub(crate) props:&'a mut PropMan,
    pub(crate) mesh:&'a MeshMan,
    pub(crate) rain:&'a mut RainMan,
    pub(crate) chase:&'a mut ChaseMan,
    pub(crate) painter:&'a WorldPainterLayerMan,
    pub(crate) vault:&'a Arc<AttributeRuntimeVault>,
    pub(crate) others:&'a mut [(u32,&'a mut PedestrianConstruction)],
    pub(crate) actors:&'a [Arc<ActorConstruction>],
    pub(crate) avoid:&'a ResolvedAvoidVolumes,
}
impl FrameServices<'_> {
    pub(crate) fn actor(&self,target:TargetableId)->Result<Option<&Arc<ActorConstruction>>,String>{
        if target.kind!=0{return Ok(None);}
        for actor in self.actors {
            let prefix=actor.prefix().map_err(|e|e.to_string())?;
            if prefix.fields().ok_or("target Actor fields absent")?.player_index==target.id as u32 {
                return Ok(Some(actor));
            }
        }
        Ok(None)
    }
    pub(crate) fn pedestrian(&self,target:TargetableId)->Option<&PedestrianConstruction>{
        if target.kind!=1{return None;}
        self.others.iter().find(|(id,_)|*id==target.id as u32).map(|(_,entity)|&**entity)
    }
    pub(crate) fn pedestrian_mut(&mut self,target:TargetableId)->Option<&mut PedestrianConstruction>{
        if target.kind!=1{return None;}
        self.others.iter_mut().find(|(id,_)|*id==target.id as u32).map(|(_,entity)|&mut **entity)
    }
    pub(crate) fn target_position(&self,entity:&PedestrianConstruction,target:TargetableId)
        ->Result<Option<[f32;4]>,String>{
        if !target.is_valid(){return Ok(None);}
        //Folded Vehicle/DynamicObject.GetPosition82d36140 reads the common
        //core transform translation (+64), never the desired target transform.
        if target.kind==2{return Ok(self.dmos.object(target.id as u32).map(|object|
            [object.transform[12],object.transform[13],object.transform[14],object.transform[15]]));}
        if target.kind==3{return Ok(self.vehicles.get(target.id as u32).map(|vehicle|vehicle.core.transform[3]));}
        if let Some(actor)=self.actor(target)? {
            let allocation=actor.cached_physical_allocation().map_err(|e|format!("target Actor physical: {e:?}"))?;
            let output=allocation.owner().phys_out().lock().map_err(|_|"target Actor PhysOut lock")?;
            let position=output.try_skeleton_output().map_err(|e|format!("target Actor position: {e:?}"))?
                .skeleton_root_matrix.columns()[3];
            return Ok(Some([position.xyz[0],position.xyz[1],position.xyz[2],position.w.unwrap_or(0.)]));
        }
        let target=if target.kind==1 && target.id as u32==entity.id {Some(entity)} else {self.pedestrian(target)};
        target.map(|target|target.core.as_ref().map(|c|c.transform[3]).ok_or("target pedestrian core absent".to_string())).transpose()
    }
    /// Borrow the sole Chasee state without cloning its mutable chaser roster.
    /// Actor lock lifetime ends before callers can mutate the target elsewhere.
    pub(crate) fn with_chasee<R>(&self,entity:&PedestrianConstruction,target:TargetableId,
        read:impl FnOnce(Option<&crate::living_world_chase::ChaseeState>)->R)->Result<R,String>{
        if !target.is_valid(){return Ok(read(None));}
        //PDB Vehicle/DynamicObject share ITargetable but no IChasee base;
        //their constructors register no Chasee interface.
        if matches!(target.kind,2|3){return Ok(read(None));}
        if let Some(actor)=self.actor(target)? {
            let prefix=actor.prefix().map_err(|e|e.to_string())?;
            return Ok(read(Some(prefix.chasee.as_ref().ok_or("target Actor Chasee absent")?)));
        }
        let target=if target.kind==1 && target.id as u32==entity.id {Some(entity)} else {self.pedestrian(target)};
        Ok(read(target.map(|target|target.chasee.as_ref().ok_or("target pedestrian Chasee absent")).transpose()?))
    }
    pub(crate) fn with_chasee_mut<R>(&mut self,entity:&mut PedestrianConstruction,target:TargetableId,
        write:impl FnOnce(Option<&mut crate::living_world_chase::ChaseeState>)->R)->Result<R,String>{
        if !target.is_valid(){return Ok(write(None));}
        if matches!(target.kind,2|3){return Ok(write(None));}
        if let Some(actor)=self.actor(target)?{
            let mut prefix=actor.prefix().map_err(|e|e.to_string())?;
            return Ok(write(Some(prefix.chasee.as_mut().ok_or("target Actor Chasee absent")?)));
        }
        let target=if target.kind==1&&target.id as u32==entity.id{Some(entity)}else{self.pedestrian_mut(target)};
        Ok(write(target.map(|target|target.chasee.as_mut().ok_or("target pedestrian Chasee absent")).transpose()?))
    }
}
