//! DynamicObject.UpdateFarthestCorners82d2c7d0 / AddRAINTestPoints82d363d8.
//! Shared retained farthest-corner samples and native held-hand-prop gate.
use super::{DmoObjectMan, DynamicObject};
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    living_world_grind_services::vehicle_entity::frame::{VehicleFrameError, error},
    living_world_mesh_store::MeshStore,
    living_world_rain::{RainMan, RainPoint},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

fn points(object: &DynamicObject) -> [[f32; 4]; 5] {
    let bbox = object.model.bbox_transform(&object.transform);
    let center = std::array::from_fn(|i| bbox[12 + i]);
    let mut out = [center; 5];
    out[1..].copy_from_slice(&object.farthest_corners);
    out
}
impl DmoObjectMan {
    pub(crate) fn update_road_flags_for_entity(
        &mut self,
        id:u32,
        rain: &RainMan,
        mesh: &MeshStore,
    ) -> Result<(), VehicleFrameError> {
            let object = self
                .object_mut(id)
                .ok_or_else(|| error("DMO RAIN member"))?;
            // UpdatePositionalRAINData82d2a7f0 uses IDynamicObject+60:
            // IsSetAsHandProp, not IsStatic. Held props cannot block traffic.
            if object.is_set_as_hand_prop(){object.flags&=!8;return Ok(());}
            let mut on_road = false;
            for point in points(object) {
                if rain
                    .try_point_on_road_network(mesh, SkeletonRootVector::known(point))
                    .map_err(error)?
                {
                    on_road = true;
                    break;
                }
            }
            object.flags = (object.flags & !8) | if on_road { 8 } else { 0 };
        Ok(())
    }
    pub(crate) fn add_road_points(
        &self,
        rain: &mut RainMan,
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<(), VehicleFrameError> {
        let attributes = vault.instance(
            skate2_formats::attrib::string_hash64("physics_skeleton"),
            skate2_formats::attrib::string_hash64("default"),
        );
        for id in self.ids() {
            let object = self.object(id).ok_or_else(|| error("DMO RAIN member"))?;
            if object.flags & 8 == 0 {
                continue;
            }
            let threshold = attributes
                .try_scalar_word(0x9ab5_7b38_d5bb_9449, 0)
                .map_err(error)?
                .map_or(0., f32::from_bits);
            if !(object.model.mass >= threshold) {
                continue;
            }
            let d = object.model.dimensions;
            let minimum = if d[0] >= d[1] { d[1] } else { d[0] };
            let minimum = if minimum >= d[2] { d[2] } else { minimum };
            for point in points(object) {
                rain.entity_points.push(RainPoint {
                    position: point,
                    radius: [minimum * 0.5; 4],
                    compulsory_stop: true,
                    pedestrian_id: -1,
                });
            }
        }
        Ok(())
    }
}
