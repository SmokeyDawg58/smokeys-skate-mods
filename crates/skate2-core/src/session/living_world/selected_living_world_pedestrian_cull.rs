//! Pedestrian RequestCull8271a5e8: callbacks, detach, retained destruction queue.
use super::SelectedRecipeFrame;
use crate::{living_world_grind_services::LivingWorldGrindServices,
    physical_player_assembly::PhysicalAssemblyWorld,input_cvar_runtime::InputCvarReferences,
    world_painter_layer::WorldPainterLayerMan,native_skater_random::NativeSkaterRandom,
    living_world_chase::TargetableId,living_world_guard_graph::GuardGraphServices};

impl SelectedRecipeFrame<'_>{
    pub(crate) fn request_pedestrian_cull(&self,living:&mut LivingWorldGrindServices,
        world:&mut PhysicalAssemblyWorld,id:u32,cvars:&InputCvarReferences,
        painter:&WorldPainterLayerMan,random:&mut NativeSkaterRandom)->Result<(),String>{
        if living.pedestrian_culls.iter().any(|entity|entity.id==id){return Ok(());}
        let Some(entity)=living.pedestrian_entities.as_deref().and_then(|p|p.get(id)) else{return Ok(());};
        let children=entity.contained_props.clone();
        let is_big_black=entity.big_black.is_some();
        let actors=world.entity_roster.as_ref().ok_or("cull Actor roster absent")?
            .snapshot().map_err(|e|format!("cull Actor roster: {e:?}"))?;
        let triggers=self.triggers.lock().map_err(|_|"cull trigger lock")?;
        let avoid=self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
        drop(triggers);
        // BigBlack.OnCullRequested82e863c0 releases IProtectee before base cull.
        if is_big_black{
            crate::pedestrian_graph::backend::frame::with_entity_and_random(living,world,id,cvars,
                &actors,painter,self.vault,&avoid,random,|entity,frame|frame.unprotect_big_black(entity))?;
        }
        // DeRegisterAllSinks82e50318 visits every directly contained Prop
        // before DeInitContainedProps begins requesting their culls.
        for child in children.iter().copied(){
            self.prop_recipe_frame().force_prop_sinks(living,world,child,cvars,painter,random,&actors,&avoid)?;
        }
        for child in children {
            self.prop_recipe_frame().request_contained_prop_cull(living,world,child,cvars,painter,random,&actors,&avoid)?;
            living.pedestrian_entities.as_deref_mut().and_then(|p|p.get_mut(id))
                .ok_or("culling pedestrian absent")?.contained_props.retain(|entry|*entry!=child);
        }
        crate::pedestrian_graph::backend::frame::with_entity_and_random(living,world,id,cvars,
            &actors,painter,self.vault,&avoid,random,|entity,frame|{
                frame.cleanly_end_plugin(entity)?;
                entity.remove_plugin(frame.props)?;
                entity.end_ai_graph(frame)?;
                if let Some(prop)=entity.held_hand_prop {
                    entity.remove_hand_prop(frame.world,frame.dmos,frame.cvars)?;
                    frame.dmos.try_request_explicit_cull_with_monitors(prop,frame.dmo_locators,
                        &mut crate::living_world_grind_services::dmo_objects::FrameLocatorServices,
                        |objects,monitor,id|entity.notify_monitored_object_for_peers(frame.others,monitor,id,frame.world,objects,frame.cvars))
                        .map_err(|e|format!("culled hand prop: {e:?}"))?;
                }
                let self_id=TargetableId{kind:1,id:entity.id as i32};
                let chasee=entity.state.as_ref().ok_or("cull AI state absent")?.chaser.chasee;
                if frame.with_chasee(entity,chasee,|state|state.is_some())? {
                    frame.chasee_remove_chaser(chasee,self_id,entity)?;
                }
                while let Some(chaser)=entity.chasee.as_ref().ok_or("cull Chasee absent")?.chasers.last().map(|entry|entry.id) {
                    frame.chasee_remove_chaser(self_id,chaser,entity)?;
                }
                frame.register_local_chaser(entity,self_id,false)?;
                frame.chase.unregister_warner(self_id);
                frame.chase.unregister_tazer(self_id);
                Ok(())
            })?;
        living.pedestrian_cull_messages(id)?;
        let losim=living.pedestrian_entities.as_deref().and_then(|p|p.get(id))
            .and_then(|p|p.core.as_ref()).ok_or("cull pedestrian core absent")?.current_losim;
        living.pedestrian_execution.remove(losim,id).map_err(|e|format!("pedestrian detach: {e:?}"))?;
        let entity=living.pedestrian_entities.as_deref_mut().ok_or("cull roster absent")?
            .remove(id).ok_or("detached pedestrian absent")?;
        living.pedestrian_culls.push(entity);
        Ok(())
    }

}

impl super::props::PropRecipeFrame<'_>{
    pub(crate) fn force_prop_sinks(&self,living:&mut LivingWorldGrindServices,world:&mut PhysicalAssemblyWorld,
        prop:u32,cvars:&InputCvarReferences,painter:&WorldPainterLayerMan,random:&mut NativeSkaterRandom,
        actors:&[std::sync::Arc<crate::actor_construction::ActorConstruction>],
        avoid:&crate::pedestrian_navigation::wander::ResolvedAvoidVolumes)->Result<(),String>{
        let (mut owners,dmos,locators)=crate::pedestrian_graph::backend::frame::FrameOwners::split(living)?;
        owners.force_prop_sinks(world,dmos,locators,prop,cvars,painter,self.vault,random,actors,avoid)
    }
    pub(crate) fn request_contained_prop_cull(&self,living:&mut LivingWorldGrindServices,world:&mut PhysicalAssemblyWorld,
        prop:u32,cvars:&InputCvarReferences,painter:&WorldPainterLayerMan,random:&mut NativeSkaterRandom,
        actors:&[std::sync::Arc<crate::actor_construction::ActorConstruction>],
        avoid:&crate::pedestrian_navigation::wander::ResolvedAvoidVolumes)->Result<(),String>{
        let (mut owners,dmos,locators)=crate::pedestrian_graph::backend::frame::FrameOwners::split(living)?;
        owners.request_contained_prop_cull(world,dmos,locators,prop,cvars,painter,self.vault,random,actors,avoid)
    }
}

impl crate::pedestrian_graph::backend::frame::FrameOwners<'_>{
    pub(crate) fn force_prop_sinks(&mut self,world:&mut PhysicalAssemblyWorld,
        dmos:&mut crate::living_world_grind_services::dmo_objects::DmoObjectMan,
        locators:&mut crate::living_world_grind_services::dmo_locators::DynamicObjectLocatorMan,
        prop:u32,cvars:&InputCvarReferences,painter:&WorldPainterLayerMan,
        vault:&std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,random:&mut NativeSkaterRandom,
        actors:&[std::sync::Arc<crate::actor_construction::ActorConstruction>],
        avoid:&crate::pedestrian_navigation::wander::ResolvedAvoidVolumes)->Result<(),String>{
        loop {
            let sink=self.props.entities.get(prop)
                .ok_or("contained cull Prop absent")?.behaviour_sinks.first().copied();
            let Some(sink)=sink else{break;};
            self.with_entity(world,dmos,locators,sink,cvars,
                actors,painter,vault,avoid,random,None,None,|entity,frame|{
                    frame.cleanly_end_plugin(entity)?;
                    entity.remove_plugin(frame.props)
                })?;
            self.props.entities.get_mut(prop)
                .ok_or("contained cull Prop absent")?.behaviour_sinks.retain(|id|*id!=sink);
        }
        Ok(())
    }
    pub(crate) fn request_contained_prop_cull(&mut self,world:&mut PhysicalAssemblyWorld,
        dmos:&mut crate::living_world_grind_services::dmo_objects::DmoObjectMan,
        locators:&mut crate::living_world_grind_services::dmo_locators::DynamicObjectLocatorMan,
        prop:u32,cvars:&InputCvarReferences,painter:&WorldPainterLayerMan,
        vault:&std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,random:&mut NativeSkaterRandom,
        actors:&[std::sync::Arc<crate::actor_construction::ActorConstruction>],
        avoid:&crate::pedestrian_navigation::wander::ResolvedAvoidVolumes)->Result<(),String>{
        let entity=self.props.entities.get_mut(prop).ok_or("contained cull Prop absent")?;
        // RequestExplicitCull82e675b0 sets this before OnCullRequested.
        // BehaviourMan removal belongs to destructor82e668bc, after callbacks
        // and census detach; process_culls is the existing owner of that step.
        entity.core.as_mut().ok_or("contained cull Prop core absent")?.allow_culling=true;
        entity.containing_entity=None;
        let children=entity.contained_props.clone();
        for child in children.iter().copied(){self.force_prop_sinks(world,dmos,locators,child,cvars,painter,vault,random,actors,avoid)?;}
        for child in children {
            self.request_contained_prop_cull(world,dmos,locators,child,cvars,painter,vault,random,actors,avoid)?;
            self.props.entities.get_mut(prop).ok_or("contained cull Prop absent")?
                .contained_props.retain(|entry|*entry!=child);
        }
        // Prop.OnCullRequested82e65bf0 first nulls every transferable copy's
        // Context source, clears the copy list, then ForceUnRegisterOfAllSinks.
        // End/remove must observe NULL, not the soon-to-be-destroyed Prop.
        let copies=self.props.entities.get(prop)
            .ok_or("contained cull Prop absent")?.behaviour_copies.clone();
        let roster=&self.pedestrians;
        for id in roster.ids(){
            if let Some(plugin)=roster.get(id).and_then(|entity|entity.plugin.as_ref()){
                if copies.iter().any(|copy|std::sync::Arc::ptr_eq(copy,&plugin.identity)){
                    plugin.clear_source();
                }
            }
        }
        self.props.entities.get_mut(prop)
            .ok_or("contained cull Prop absent")?.behaviour_copies.clear();
        self.force_prop_sinks(world,dmos,locators,prop,cvars,painter,vault,random,actors,avoid)?;
        self.props.request_explicit_cull(prop,self.game_sim_events)
    }
}

impl LivingWorldGrindServices {
    /// Census.Update destroys only already detached pedestrians. Keep a failed
    /// destructor in this retained queue, including its completed release prefix.
    pub(crate) fn process_pedestrian_culls(&mut self,world:&mut PhysicalAssemblyWorld)->Result<(),String>{
        use crate::living_world_pedestrian_memory::PedestrianStore as Store;
        let mesh=self.try_mesh_store().map_err(|e|format!("cull mesh: {e:?}"))?;
        let roster=self.pedestrian_entities.as_deref_mut().ok_or("cull roster absent")?;
        let memory=self.pedestrian_memory.as_mut().ok_or("cull memory absent")?;
        while !self.pedestrian_culls.is_empty(){
            let entity=&mut self.pedestrian_culls[0];
            let release=|entity:&mut crate::living_world_pedestrian_entity::PedestrianConstruction,
                memory:&mut crate::living_world_pedestrian_memory::PedestrianMemory,store:Store|->Result<(),String>{
                if let Some(index)=entity.allocations.iter().position(|(kind,_)|*kind==store){
                    let (_,slot)=entity.allocations[index];
                    if !memory.release(store,slot){return Err(format!("pedestrian destructor slot {store:?} mismatch"));}
                    entity.allocations.remove(index);
                }
                Ok(())
            };
            entity.big_black=None;
            release(entity,memory,Store::BigBlack)?;
            entity.security_guard=None;
            release(entity,memory,Store::SecurityGuard)?;
            for id in roster.ids(){roster.get_mut(id).expect("native cull notification order").notify_entity_culled(entity.id)?;}
            if entity.current_waypoint.is_some(){entity.clear_current_waypoint(self.props.as_deref_mut().ok_or("cull PropMan absent")?)?;}
            if let Some(intent)=entity.intent_queue.as_mut(){intent.clear(false,None)?;}
            entity.intent_queue=None;
            release(entity,memory,Store::Intent)?;
            if let Some(animation)=entity.animation.as_mut(){animation.prepare_destruction()?;}
            entity.animation=None;
            release(entity,memory,Store::Animation)?;
            if let Some(state)=entity.state.as_mut(){
                let mut slave=state.slave_id;state.slave_id=-1;
                while let Some(next)=roster.get_mut(slave as u32){
                    let state=next.state.as_mut().ok_or("cull pathing slave state absent")?;
                    state.master_id=-1;slave=state.slave_id;state.slave_id=-1;
                }
            }
            entity.state=None;release(entity,memory,Store::AiState)?;
            entity.post_adjust=None;release(entity,memory,Store::PostAdjust)?;
            entity.head_tracking=None;release(entity,memory,Store::HeadTracking)?;
            entity.spatial_adjust=None;release(entity,memory,Store::SpatialAdjust)?;
            // The rig's raw-pointer query hooks end with its allocation.
            // Do not discard requests at RequestCull while the rig is alive.
            self.pedestrian_queries.as_mut().ok_or("cull query owner absent")?
                .retire_rig(entity.id);
            entity.nav_rig=None;release(entity,memory,Store::NavRig)?;
            if let Some(collision)=entity.collision.as_mut(){
                for body in [&collision.world_body,&collision.skater_body]{
                    let handle=match body{
                        Some(crate::pedestrian_collision::PedestrianCollisionBody::Constructed(candidate))=>Some(candidate.handle()),
                        Some(crate::pedestrian_collision::PedestrianCollisionBody::InWorld(handle))=>Some(handle),None=>None};
                    if let Some(handle)=handle{self.vehicle_collideables.remove(handle);}
                }
                collision.destroy_bodies(world)?;
            }
            entity.collision=None;release(entity,memory,Store::Collision)?;
            // Road iterator destructor follows core destruction natively. The
            // current target release reads only its retained RAIN reservation.
            entity.core=None;release(entity,memory,Store::Core)?;
            entity.target_iterators.road.deinit_current(&mut crate::pedestrian_navigation::road::RoadNavigationContext{
                rain:self.rain.as_mut().ok_or("cull RAIN absent")?,mesh:&mesh,
                random:self.random.as_mut().ok_or("cull Manager random absent")?,
                position:[0.;4],facing:[0.;4],entity_id:entity.id})?;
            release(entity,memory,Store::Entity)?;
            if !entity.allocations.is_empty(){return Err("pedestrian destructor unhandled allocation".into());}
            self.pedestrian_culls.remove(0);
        }
        Ok(())
    }
}

impl crate::pedestrian_graph::backend::frame::FrameOwners<'_>{
    /// LWBasicEntity.OnCullRequested82e502a8, called before DMO monitors.
    pub(crate) fn before_dmo_cull(&mut self,world:&mut PhysicalAssemblyWorld,
        dmos:&mut crate::living_world_grind_services::dmo_objects::DmoObjectMan,
        locators:&mut crate::living_world_grind_services::dmo_locators::DynamicObjectLocatorMan,
        object:u32,cvars:&InputCvarReferences,painter:&WorldPainterLayerMan,
        vault:&std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,random:&mut NativeSkaterRandom,
        actors:&[std::sync::Arc<crate::actor_construction::ActorConstruction>],
        avoid:&crate::pedestrian_navigation::wander::ResolvedAvoidVolumes)->Result<(),String>{
        let children=dmos.object(object).ok_or("DMO cull entity absent")?.contained_props.clone();
        // DeRegisterAllSinks for every child precedes DeInitContainedProps.
        for child in children.iter().copied(){
            self.force_prop_sinks(world,dmos,locators,child,cvars,painter,vault,random,actors,avoid)?;
        }
        // DynamicObject.DeInitContainedProps82d2e098 clears each original
        // IWaypointGroup source before base child culls. SetData drains the
        // point allocations; the later destructor sees an empty point list.
        for child in children.iter().copied(){
            let prop=self.props.entities.get_mut(child).ok_or("DMO child waypoint absent")?;
            if let Some(group)=prop.waypoint.as_mut(){group.set_data(None,&mut self.props.memory)?;}
        }
        for child in children {
            self.request_contained_prop_cull(world,dmos,locators,child,cvars,painter,vault,random,actors,avoid)?;
            dmos.object_mut(object).ok_or("DMO disappeared during child cull")?
                .contained_props.retain(|id|*id!=child);
        }
        Ok(())
    }
}
