//! Native DMO positional callbacks and the actual safety-index producer.
use super::SelectedRecipeFrame;
use crate::{living_world_grind_services::{LivingWorldGrindServices,execution::ExecutionTarget},
    world_painter_layer::WorldPainterLayerMan};

impl SelectedRecipeFrame<'_>{
    /// Task_UpdatePositionalData82d2b950 completes every selected entity
    /// before Task_UpdateStateGraphs82d2b890 starts.
    pub(crate) fn update_dmo_positional_data(&self,living:&mut LivingWorldGrindServices,
        targets:&[ExecutionTarget],painter:&WorldPainterLayerMan,center:[f32;4])->Result<(),String>{
        let mesh=living.try_mesh_store().map_err(|e|format!("DMO positional mesh: {e:?}"))?;
        let rain=living.rain.as_ref().ok_or("DMO positional RainMan absent")?;
        let objects=living.dmo_objects.as_deref_mut().ok_or("DMO positional roster absent")?;
        for target in targets {
            let object=objects.object_mut(target.entity).ok_or("DMO positional entity absent")?;
            object.player_distance.update(center,std::array::from_fn(|i|object.transform[12+i]),
                &mut crate::reckoning_numeric::HostReckoningNumeric)
                .map_err(|e|format!("DMO positional distance: {e:?}"))?;
            // LWBasicEntity.UpdatePositionalData82e50040 calls RAIN then WP.
            objects.update_road_flags_for_entity(target.entity,rain,&mesh)
                .map_err(|e|format!("DMO positional RAIN: {e:?}"))?;
            let object=objects.object_mut(target.entity).ok_or("DMO positional entity absent")?;
            let bbox=object.model.bbox_transform(&object.transform);
            let center=std::array::from_fn::<_,4,_>(|i|bbox[12+i]);
            // UpdatePositionalWorldPainterData82d393e8 first gathers all keys,
            // preserving duplicates, then reads them until the first zero.
            let mut collections=vec![0xd7ed_bd36_2d7d_2152];
            for point in std::iter::once(center).chain(object.farthest_corners){
                let key=painter.try_attrib_key(14,[point[0],point[2]])
                    .map_err(|e|format!("DMO safety WorldPainter: {e:?}"))?;
                if key!=0 {collections.push(key);}
            }
            let read=|key|->Result<u32,String>{
                let attributes=self.vault.instance(0xd558_c5d6_3afe_bcad,key);
                Ok(attributes.try_cached_layout_field(0,4).map_err(|e|e.to_string())?
                    .map_or(0,|data|u32::from_be_bytes(data.bytes().try_into().expect("checked safety word"))))
            };
            let mut safety=1;
            for key in collections {
                let value=read(key)?;
                if value==0 {safety=0;break;}
                if value>safety {safety=value;}
            }
            object.wp_safety_index=safety;
            // UpdatePositionalData82d2c708 tests mpRAINData(+70), NOT flags8.
            // Preserve the distinct retained source pointer despite the DMO
            // RAIN override updating only on-road flags in normal execution.
            let rain_safety=if object.rain_data.is_some(){read(0x991d_f788_b8f6_1295)?}else{1};
            object.safety_index=if rain_safety==0||safety==0 {0}else{rain_safety.max(safety)};
        }
        Ok(())
    }
}
