//! DMO specialization of the shared execution groups, using its existing map.
use super::{DmoObjectMan, DynamicObject};
use crate::living_world_grind_services::{
    LivingWorldGrindServices, entity_core::CommonEntityCore,
    execution::{ExecutionEntity, ExecutionTarget},
};
impl ExecutionEntity for DynamicObject {
    fn execution_core(&self)->Option<&CommonEntityCore>{None}
    fn execution_core_mut(&mut self)->Option<&mut CommonEntityCore>{None}
    fn execution_position(&self)->Option<[f32;4]>{
        Some(std::array::from_fn(|i|self.transform[12+i]))
    }
    fn take_explicit_update(&mut self)->Option<bool>{
        Some(std::mem::take(&mut self.explicit_update_requested))
    }
}
impl LivingWorldGrindServices {
    /// ComponentMan.UpdateLOSStates82d2f760. Collideable OnLOSimSwitch82e50580
    /// calls its collision getter (no mutation) and returns true.
    pub(crate) fn dynamic_object_update_los_states(&mut self,center:[f32;4],
        vault:&std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>)->Result<(),String>{
        let objects=self.dmo_objects.as_deref_mut().ok_or("DMO LOSim manager absent")?;
        let mut numeric=crate::reckoning_numeric::HostReckoningNumeric;
        for id in objects.order.ids(){
            let object=objects.objects.get_mut(&id).ok_or("DMO LOSim entity absent")?;
            if object.attrib_cache.is_none(){
                object.attrib_cache=Some(self.census_targets.get(object.presentation_model_key,vault)
                    .map_err(|e|format!("DMO LOSim attributes: {e:?}"))?);
            }
            let position=std::array::from_fn(|i|object.transform[12+i]);
            let next=CommonEntityCore::test_losim_at(position,center,
                object.attrib_cache.as_deref().expect("loaded above"),&mut numeric)
                .map_err(|e|format!("DMO LOSim distance: {e:?}"))?;
            if next!=object.current_losim {
                objects.execution.remove(object.current_losim,id).map_err(|e|format!("DMO LOSim remove: {e:?}"))?;
                object.current_losim=next;
                objects.execution.add(next,id).map_err(|e|format!("DMO LOSim add: {e:?}"))?;
            }
        }
        Ok(())
    }
}
impl DmoObjectMan {
    /// UpdatePreFrame82d38e50: collect/sort. Caller completes each selected
    /// entity's distance/RAIN/WorldPainter pass before any stategraph update.
    pub(crate) fn collect_preframe(&mut self,center:[f32;4])->Result<Vec<ExecutionTarget>,String>{
        let mut targets=Vec::new();
        let mut numeric=crate::reckoning_numeric::HostReckoningNumeric;
        self.execution.collect_preframe(&mut self.objects,center,&mut targets,&mut numeric)
            .map_err(|e|format!("DMO preframe targets: {e:?}"))?;
        Ok(targets)
    }
}
