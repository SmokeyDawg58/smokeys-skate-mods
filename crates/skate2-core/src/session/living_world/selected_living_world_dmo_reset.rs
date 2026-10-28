//! Native CanReset82d3a908 / Reset82d39e10 and recursive reset lists.
use super::SelectedRecipeFrame;
use crate::{living_world_grind_services::{LivingWorldGrindServices,dmo_objects::{
    SkaterTestCapsule,FrameLocatorServices,DmoCullEvent}},
    physical_player_assembly::PhysicalAssemblyWorld,input_cvar_runtime::InputCvarReferences,
    world_painter_layer::WorldPainterLayerMan,
    actor_construction::ActorConstruction};

// BuildRecursiveObjectResetList82d37c70: depth-first, pool order, capacity50.
fn reset_list(living:&LivingWorldGrindServices,world:&PhysicalAssemblyWorld,
    entity:u32,list:&mut Vec<u32>,ignore_marker:bool)->Result<(),String>{
    let objects=living.dmo_objects.as_deref().ok_or("reset DMO manager absent")?;
    let Some(object)=objects.object(entity) else{return Ok(());};
    // A SpawnStatic body has no LWDynamicObjectMan owner to reset.
    if object.online_static{return Ok(());}
    let locator=living.dmo_locators.as_deref().ok_or("reset locators absent")?.locator(object.locator_id);
    if !ignore_marker || locator.is_none_or(|l|l.flags&0x10==0) {
        if list.len()==50{return Err("native recursive DMO reset list capacity50 exceeded".into());}
        list.push(entity);
    }
    let Some(locator)=locator else{return Ok(());};
    let transform=locator.initial_stream_transform;
    let overlaps=world.try_model_volume_test_dynamic_objects(&object.model,transform)
        .map_err(|e|format!("recursive DMO reset overlap: {e:?}"))?;
    for body in overlaps{
        let other=objects.owner_of(&body);
        if other!=entity && !list.contains(&other){reset_list(living,world,other,list,ignore_marker)?;}
    }
    Ok(())
}
// BuildObjectResetListVolumeTestInfo82d39c60 and VolumeVolumeListOverlapTest.
fn overlaps_reset_list(living:&LivingWorldGrindServices,list:&[u32],
    capsule:SkaterTestCapsule)->Result<bool,String>{
    let objects=living.dmo_objects.as_deref().ok_or("reset DMO manager absent")?;
    let locators=living.dmo_locators.as_deref().ok_or("reset locators absent")?;
    for &id in list{
        let Some(object)=objects.object(id) else{continue;};
        let Some(locator)=locators.locator(object.locator_id) else{continue;};
        let Some(model)=living.dmo_models.model(locator.model_id) else{continue;};
        let transform=locator.initial_stream_transform;
        if crate::simulation_assembly_backend::constraint_session::dmo_living_world::reset_model_overlaps_capsule(
            model,transform,capsule).map_err(|e|format!("reset model overlap: {e:?}"))?{return Ok(true);}
    }
    Ok(false)
}
pub(crate) fn can_reset_dmo(living:&LivingWorldGrindServices,world:&PhysicalAssemblyWorld,entity:u32)->Result<bool,String> {
    let capsules=living.dmo_objects.as_deref().ok_or("reset manager absent")?.reset_capsules;
    let capsules=capsules.ok_or("DMO CanReset requires retained UpdateTransforms capsules")?;
    let mut list=Vec::new();
    reset_list(living,world,entity,&mut list,false)?;
    Ok(!overlaps_reset_list(living,&list,capsules[0])?)
}
impl SelectedRecipeFrame<'_>{
    /// SessionMarker.ResetDynamicObjectLocatorsInSessionMarker828ade78.
    pub(crate) fn restore_marker_props(&self,living:&mut LivingWorldGrindServices,
        world:&mut PhysicalAssemblyWorld,saved:&[(u64,[f32;16])],cvars:&InputCvarReferences,
        painter:&WorldPainterLayerMan)->Result<(),String> {
        for &(id,transform) in saved {
            let mut list=Vec::new();
            if let Some(locator)=living.dmo_locators.as_deref().ok_or("marker locators absent")?.locator(id) {
                if let Some(model)=living.dmo_models.model(locator.model_id) {
                    let user=locator.user_object_id;
                    for body in world.try_model_volume_test_dynamic_objects(model,transform).map_err(|e|format!("marker reset overlap: {e:?}"))? {
                        let other=living.dmo_objects.as_deref().ok_or("marker DMO manager absent")?.owner_of(&body);
                        if other!=user && !list.contains(&other) {reset_list(living,world,other,&mut list,true)?;}
                    }
                    list.retain(|&other|other!=user);
                    if user!=0 && locator.flags&0x10==0 {list.push(user);}
                }
            }
            self.with_dmo_culls(living,world,cvars,painter,|objects,locators,cull| {
                for entity in list {
                    let Some(object)=objects.object(entity) else{continue;};
                    let initial=locators.locator(object.locator_id).map(|l|(l.locator_id,l.initial_stream_transform));
                    cull(objects,locators,entity)?;
                    if let Some((key,initial))=initial {locators.reset_locator_transform(key,&initial,&mut FrameLocatorServices).map_err(|e|format!("marker obstruction reset: {e:?}"))?;}
                }
                if let Some(locator)=locators.locator(id) {
                    let user=locator.user_object_id;
                    if user!=0 {cull(objects,locators,user)?;}
                    locators.reset_locator_transform(id,&transform,&mut FrameLocatorServices).map_err(|e|format!("marker saved reset: {e:?}"))?;
                }
                Ok(())
            })?;
        }
        // RequestAllUnusedHandPropsCull82d37620, after all saved locators.
        self.with_dmo_culls(living,world,cvars,painter,|objects,locators,cull| {
            for id in objects.ids() {
                if objects.object(id).is_some_and(|o|o.locator_id==crate::living_world_grind_services::dmo_models::NULL_ASSET_ID && !o.is_set_as_hand_prop() && !o.online_static) {cull(objects,locators,id)?;}
            }
            Ok(())
        })
    }

    pub(crate) fn try_reset_unsafe_dmo(&self,living:&mut LivingWorldGrindServices,
        world:&mut PhysicalAssemblyWorld,entity:u32,cvars:&InputCvarReferences,
        painter:&WorldPainterLayerMan,actor:&ActorConstruction)->Result<(),String>{
        let capsules=living.dmo_objects.as_deref().ok_or("reset DMO manager absent")?
            .reset_capsules.ok_or("DMO reset requires retained UpdateTransforms capsules")?;
        let mut list=Vec::new();
        reset_list(living,world,entity,&mut list,false)?;
        if overlaps_reset_list(living,&list,capsules[0])?{return Ok(());}
        // Reset rebuilds the list after CanReset; callbacks may change it.
        list.clear();
        reset_list(living,world,entity,&mut list,false)?;
        let offboard={
            let physical=actor.cached_physical_allocation().map_err(|e|format!("DMO reset Actor physical: {e:?}"))?;
            let output=physical.owner().phys_out().lock().map_err(|_|"DMO reset PhysOut lock")?;
            output.try_state_output().map_err(|e|format!("DMO reset Actor state: {e:?}"))?.is_offboard
        };
        if overlaps_reset_list(living,&list,capsules[1])? && offboard{
            crate::simulation_assembly_backend::constraint_session::dmo_living_world::request_actor_hide_skateboard(world,actor,cvars)
                .map_err(|e|format!("DMO reset board hide: {e:?}"))?;
        }
        let actors=world.entity_roster.as_ref().ok_or("reset Actor roster absent")?
            .snapshot().map_err(|e|format!("reset Actor roster: {e:?}"))?;
        let avoid={let triggers=self.triggers.lock().map_err(|_|"reset triggers lock")?;
            self.runtime.challenge_avoid_volumes.resolve(&triggers)?};
        let mut random=living.random.take().ok_or("reset Manager RNG absent")?;
        let result=(||->Result<(),String>{
            let (mut owners,objects,locators)=crate::pedestrian_graph::backend::frame::FrameOwners::split(living)?;
            for id in list{
                let Some(object)=objects.object(id) else{continue;};
                let locator=locators.locator(object.locator_id)
                    .map(|locator|(locator.locator_id,locator.initial_stream_transform));
                objects.try_request_explicit_cull_with_lifecycle(id,locators,&mut FrameLocatorServices,
                    |objects,locators,event|match event{
                        DmoCullEvent::BeforeCull(id)=>owners.before_dmo_cull(world,objects,locators,id,cvars,
                            painter,self.vault,&mut random,&actors,&avoid),
                        DmoCullEvent::Monitor{entity,monitor}=>{
                            if let Some(pedestrian)=owners.pedestrians.get_mut(monitor){
                                pedestrian.monitored_object_being_culled(entity,world,objects,cvars)?;
                            }
                            Ok(())
                        }
                    }).map_err(|e|format!("DMO reset cull: {e:?}"))?;
                if let Some((id,transform))=locator{
                    locators.reset_locator_transform(id,&transform,&mut FrameLocatorServices)
                        .map_err(|e|format!("DMO reset locator: {e:?}"))?;
                }
            }
            Ok(())
        })();
        living.random=Some(random);
        result
    }
}
