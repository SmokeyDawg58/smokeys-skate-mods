//! Manager.EvaluateHandProps82e9d150 -> Pedestrian.SpawnAndSetHandProp82e9c878.
use super::*;
impl SelectedLivingWorldDmo {
    pub(super) fn try_evaluate_hand_props(world:&mut PhysicalAssemblyWorld,
        vault:&Arc<AttributeRuntimeVault>,actor:&ActorConstruction,cvars:&InputCvarReferences,
        census:&DmoCensusCvars,recipes:Option<&crate::selected_living_world_recipes::SelectedRecipeFrame<'_>>)->Result<(),SelectedLivingWorldDmoError>{
        let owner=Arc::clone(world.grind_services.living_world());
        let ids=owner.lock().map_err(|_|SelectedLivingWorldDmoError::LivingWorldLock)?
            .pedestrian_entities.as_deref().ok_or(SelectedLivingWorldDmoError::MissingManagers)?.ids();
        for id in ids {
            let prepared=(||->Result<_,String>{
                let mut guard=owner.lock().map_err(|_|"hand prop Manager lock")?;
                let living:&mut LivingWorldGrindServices=&mut guard;
                let entity=living.pedestrian_entities.as_deref_mut().and_then(|p|p.get_mut(id))
                    .ok_or("hand prop pedestrian absent")?;
                if !entity.renderable || entity.state.as_ref().ok_or("hand prop state absent")?.flags[3]&2==0 {
                    return Ok(None);
                }
                let key=entity.hand_prop_attributes.as_ref().and_then(|a|a.collection_key())
                    .ok_or("pending hand prop collection assertion")?;
                let attributes=vault.instance(0xec8b_7565_46e2_26f6,key);
                entity.hand_prop_attributes=Some(attributes.clone());
                // Generated dmo_models constructor takes Model.collection at layout40.
                let model_key=attributes.try_cached_layout_field(40,8).map_err(|e|e.to_string())?
                    .map_or(0,|d|u64::from_be_bytes(d.bytes().try_into().expect("eight bytes")));
                let model_attributes=vault.instance(skate2_formats::attrib::string_hash64("dmo_models"),model_key);
                let asset=model_attributes.try_cached_layout_field(0,8).map_err(|e|e.to_string())?
                    .map_or(0,|d|u64::from_be_bytes(d.bytes().try_into().expect("eight bytes")));
                let Some(model)=living.dmo_models.model(asset).cloned() else {
                    entity.hand_prop_attributes=None;
                    entity.state.as_mut().expect("checked state").flags[3]&=!2;
                    return Ok(None);
                };
                let transform=entity.hand_prop_spawn_transform(model.hand_hotpoint_transform
                    .ok_or("hand prop model grip transform absent")?)?;
                Ok(Some((key,model,transform)))
            })().map_err(SelectedLivingWorldDmoError::Pedestrians)?;
            let Some((key,model,transform))=prepared else {continue;};
            // Same native factory flags as ordinary admitted DMO: initial false,
            // allowCull true, force false, world collisions true; null locator.
            let object=Self::try_with_frame(world,vault,actor,cvars,recipes,|frame|
                frame.try_spawn_dynamic_object_id(crate::living_world_grind_services::dmo_models::NULL_ASSET_ID,
                    &model,&transform,census).map_err(SelectedLivingWorldDmoError::Census))?;
            (||->Result<(),String>{
                let mut guard=owner.lock().map_err(|_|"hand prop Manager lock")?;
                let living:&mut LivingWorldGrindServices=&mut guard;
                let (entity,mut peers,_)=living.pedestrian_entities.as_deref_mut()
                    .ok_or("hand prop roster absent")?.split_entity_and_peers_and_spatial_mut(id)
                    .ok_or("hand prop entity/spatial absent")?;
                if let Some(object)=object {
                    entity.set_hand_prop(object,world,living.dmo_objects.as_deref_mut().ok_or("hand prop DMOMan absent")?,
                        living.dmo_locators.as_deref_mut().ok_or("hand prop locators absent")?,cvars,&mut peers)?;
                    entity.initialize_hand_prop_usage(key,vault,living.random.as_mut().ok_or("hand prop Manager RNG absent")?)?;
                } else {entity.hand_prop_attributes=None;}
                entity.state.as_mut().ok_or("hand prop state absent")?.flags[3]&=!2;
                Ok(())
            })().map_err(SelectedLivingWorldDmoError::Pedestrians)?;
        }
        Ok(())
    }
}

