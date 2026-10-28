//! Native CAS master component/model/material selection.
//! CharacterEntity::CreateCopyFromComponents82d13ad0 and RebindComponentToMaterial82d08fd8.
use skate2_formats::{cac_parts::{CacPartDb,SLOT_NAMES},recipe::CharacterRecipe,skater_profile::CacPartId};
use crate::{character::{CharacterHierarchy,palette::CharacterPalette,material_admission::CharacterMaterialAdmission},
    player::mounted_recipe::finalize::FinalizedRecipeScene};
use std::sync::Arc;
pub type EditSkaterCharacterResult<T> = Result<T,Box<dyn std::error::Error>>;

pub fn compose_recipe(db:&CacPartDb,parts:&[CacPartId;26],
    customizations:skate2_formats::recipe::CharacterCustomizations)->EditSkaterCharacterResult<CharacterRecipe> {
    let mut recipe=db.master.clone();
    recipe.components.clear();recipe.clone_type=2;recipe.packed_size=0;
    recipe.has_customizations=true;recipe.customizations=Some(customizations);
    let invalid=CacPartId::invalid();
    for (slot,id) in parts.iter().enumerate() {
        let Some(component)=db.master.components.iter().find(|p|p.name==SLOT_NAMES[slot]) else {continue};
        if *id==invalid {continue;}
        let mut selected=component.clone();
        let model=component.models.iter().find(|m|m.asset_id==id.model)
            .ok_or_else(||format!("CAS {} model {:016x} absent from master",component.name,id.model))?;
        let mut model=model.clone();
        for lod in &mut model.lods {
            for instance in &mut lod.material_instances {
                // Native uses the selected material if present, otherwise the
                // first original ordered material (parts can share geometry).
                let selected_material=instance.materials.iter().any(|m|m.asset_id==id.material);
                let mut material=instance.materials.iter().find(|m|m.asset_id==id.material)
                    .or_else(||instance.materials.iter().min_by_key(|m|m.asset_id)).cloned();
                if selected_material {
                    if let (Some(material),Some(custom))=(&mut material,&recipe.customizations){
                        apply_texture_customizations(&db.master,material,id,custom)?;
                    }
                }
                instance.materials=material.into_iter().collect();
            }
        }
        selected.models=vec![model];recipe.components.push(selected);
    }
    recipe.components.sort_by_key(|p|skate2_formats::hash::rw_hash32_string(&p.name));
    if recipe.components.is_empty(){return Err("CAS assembled recipe contains no components".into());}
    Ok(recipe)
}

/// CreateCopyFromComponents82d13f34..406c matches both model/material, then
/// resolves the FIRST record with the matching key. Raw PPC confirms both
/// comparisons are full64-bit, despite Hex-Rays splitting the registers.
fn apply_texture_customizations(master:&CharacterRecipe,material:&mut skate2_formats::recipe::CharacterMaterial,
    part:&CacPartId,custom:&skate2_formats::recipe::CharacterCustomizations)->EditSkaterCharacterResult<()> {
    let Some(part_stamp)=custom.stamps.iter().find(|s|s.model_id==part.model&&s.material_id==part.material) else{return Ok(());};
    let Some(stamp)=custom.stamps.iter().find(|s|s.key==part_stamp.key) else{return Ok(());};
    if stamp.user_generated {return Err("CAS user-generated stamp texture owner is not loaded".into());}
    // GetStampableTextureFromMaterialVariant82d093d0 searches Misc, models in
    // asset order, FIRST LOD only, then original material-instance order.
    let Some(misc)=master.components.iter().find(|c|c.name=="Misc") else{return Ok(());};
    let mut models=misc.models.iter().collect::<Vec<_>>();models.sort_by_key(|m|m.asset_id);
    for model in models {
        let Some(lod)=model.lods.iter().min_by_key(|l|l.level) else{continue;};
        for instance in &lod.material_instances {
            if let Some(source)=instance.materials.iter().find(|m|m.asset_id==stamp.stamp_material_id){
                if let Some(texture)=source.shader_parameters.iter().find(|p|p.channel_name=="decal"){
                    if let Some(channel)=material.shader_parameters.iter_mut().find(|p|p.channel_name=="decal"){
                        channel.asset_id=texture.asset_id;
                    }
                }
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Owned scene/texture copies are the host implementation of clone2. Mutating
/// their material bindings cannot modify a retained master or gameplay recipe.
pub struct EditSkaterCharacter {
    pub recipe:Arc<CharacterRecipe>,
    pub palette:CharacterPalette,
    pub materials:CharacterMaterialAdmission,
    pub(crate) finalized:Arc<FinalizedRecipeScene>,
}
impl EditSkaterCharacter {
    pub(crate) fn load(mounts:&skate2_formats::mount::MountSet,recipe:CharacterRecipe,
        hierarchy:&CharacterHierarchy,vault:Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        cvars:crate::input_cvar_runtime::InputCvarReferences)->EditSkaterCharacterResult<Self> {
        let scene=crate::mounted_character_scene::own_character_scene_category(mounts,&recipe,"createacharacter")?;
        let scene=Arc::new(scene);
        let mut finalized=FinalizedRecipeScene::new(Arc::clone(&scene));
        finalized.try_finalize(2).map_err(|e|format!("CAS finalize: {e:?}"))?;
        let palette=CharacterPalette::new(scene,hierarchy,0)?;
        let mut materials=CharacterMaterialAdmission::try_new(&finalized,&palette,vault,cvars)?;
        materials.try_construct_cas()?;
        Ok(Self{recipe:Arc::new(recipe),palette,materials,finalized:Arc::new(finalized)})
    }
}
