//! DynamicObject.PreStateGraphUpdate82d2ea08, on actual execution targets.
use super::SelectedRecipeFrame;
use crate::{living_world_grind_services::LivingWorldGrindServices,
    physical_player_assembly::PhysicalAssemblyWorld,input_cvar_runtime::InputCvarReferences,
    reckoning_numeric::{ReckoningNumeric,HostReckoningNumeric,matrix33::{try_orthonormalize,try_rotation_vector}},
    reckoning_sin_cos::HostReckoningSinCosNumeric};
impl SelectedRecipeFrame<'_>{
    pub(crate) fn update_dmo_preframe(&self,living:&mut LivingWorldGrindServices,
        world:&mut PhysicalAssemblyWorld,center:[f32;4],cvars:&InputCvarReferences,
        painter:&crate::world_painter_layer::WorldPainterLayerMan,actor:&crate::actor_construction::ActorConstruction)->Result<(),String>{
        let targets=living.dmo_objects.as_deref_mut().ok_or("DMO preframe manager absent")?
            .collect_preframe(center)?;
        self.update_dmo_positional_data(living,&targets,painter,center)?;
        for target in targets {
            // Native Task82d2b890 retains pointers until Census.Update. An
            // earlier recursive reset can already have culled this target.
            // Cull clears its locator, disabling BOTH prestate branches;
            // graph getter is null and poststate is debug-only. PendingCull
            // retains destruction ownership, so that remainder has no effect.
            if living.dmo_objects.as_deref().ok_or("DMO prestate manager absent")?
                .is_pending_cull(target.entity){continue;}

            // DMO LWBasicEntity plugin getter is the folded null routine, so
            // base prestate has no removal callback for this native class.
            let decision={
                let object=living.dmo_objects.as_deref().and_then(|o|o.object(target.entity))
                    .ok_or("DMO prestate target absent")?;
                if object.is_static() || object.contained_props.is_empty()
                    || object.locator_id==crate::living_world_grind_services::dmo_models::NULL_ASSET_ID
                    || object.model.characteristics.is_null(){None}else{
                    let locator=living.dmo_locators.as_deref().ok_or("DMO prestate locators absent")?
                        .locator(object.locator_id);
                    locator.map(|locator|contained_transform_within_tolerance(
                        object.transform,locator.initial_stream_transform,&object.model.characteristics)).transpose()?
                }
            };
            if let Some(activate)=decision {
                if activate {self.prop_recipe_frame().activate_dmo_contained_props(living,target.entity,cvars)?;}
                else {self.prop_recipe_frame().release_dmo_contained_props(living,target.entity)?;}
            }
            let object=living.dmo_objects.as_deref().and_then(|o|o.object(target.entity))
                .ok_or("DMO prestate target absent")?;
            // Entity.GetSafetyIndex+84 gates the recursive reset suffix.
            if object.safety_index==0 && object.locator_id!=crate::living_world_grind_services::dmo_models::NULL_ASSET_ID {
                self.try_reset_unsafe_dmo(living,world,target.entity,cvars,painter,actor)?;
            }
            // DMO has no AI graph; base UpdateState and debug-only PostState
            // do not add gameplay mutation. target.dt remains group-owned.
        }
        Ok(())
    }
}
fn contained_transform_within_tolerance(current:[f32;16],locator:[f32;16],
    attributes:&crate::attribute_runtime::AttributeRuntimeInstance)->Result<bool,String>{
    let position=f32::from_bits(attributes.try_scalar_word(0x2114_f932_9e98_f00c,0)
        .map_err(|e|e.to_string())?.unwrap_or(0));
    let mut numeric=HostReckoningNumeric;
    let delta=std::array::from_fn(|i|locator[12+i]-current[12+i]);
    if numeric.dot3(delta,delta).map_err(|e|format!("DMO position tolerance: {e:?}"))?-position*position>0. {
        return Ok(false);
    }
    // Raw82d2ebe0..ec20: all three row compares report all-false. The
    // neighboring original EPSILON is37800000 (not the degrees constant).
    if (0..3).all(|axis|(0..3).all(|lane|
        !((locator[axis*4+lane]-current[axis*4+lane]).abs()>f32::from_bits(0x37800000)))) {
        return Ok(true);
    }
    let data=attributes.try_get_data(0xe434_4503_6aae_e7c5,0).map_err(|e|e.to_string())?;
    let tolerance=if let Some(data)=data {
        if data.bytes().len()<16{return Err("DMO RotationAngleTolerance payload shorter than Vector3".into());}
        std::array::from_fn(|i|f32::from_be_bytes(data.bytes()[i*4..i*4+4].try_into().expect("checked vector"))
            *f32::from_bits(0x3c8efa35))
    }else{[0.;4]};
    // Native relative frame is transpose(current)*locator, with the VMX
    // merge's replicated Y padding lane, as in the shared collision drive.
    let transpose:[[f32;4];3]=std::array::from_fn(|i|[current[i],current[4+i],current[8+i],current[4+i]]);
    let relative=std::array::from_fn(|axis|std::array::from_fn(|lane|{
        let first=transpose[0][lane]*locator[axis*4];
        let second=transpose[1][lane].mul_add(locator[axis*4+1],first);
        transpose[2][lane].mul_add(locator[axis*4+2],second)
    }));
    let normalized=try_orthonormalize(relative,&mut numeric).map_err(|e|format!("DMO rotation basis: {e:?}"))?;
    let rotation=try_rotation_vector(normalized,&mut numeric,&mut HostReckoningSinCosNumeric)
        .map_err(|e|format!("DMO rotation vector: {e:?}"))?;
    Ok((0..3).all(|i|!(rotation[i].abs()>tolerance[i])))
}
