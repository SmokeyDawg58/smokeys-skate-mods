//! Actual `GrindCollision.FindNearbyPrimitives`82cfa500..a5f8.
use super::{
    fields::GrindCollision,
    update_prefix::{GrindPrefixError, need},
};
use crate::{
    debug_overlay::DebugOverlaySlot, grind_data_octree::OctreeBounds,
    grind_data_runtime::GrindDataQueryResult, input_cvar_runtime::InputCvarReferences,
};
impl GrindCollision {
    pub(crate) fn try_find_nearby<Q>(
        &self,
        refs: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
        query: &mut impl FnMut(OctreeBounds, &mut GrindDataQueryResult) -> Result<bool, Q>,
    ) -> Result<(), GrindPrefixError<Q>> {
        use GrindPrefixError as E;
        let processed = self.bindings.processed.lock().map_err(|_| E::Poisoned)?;
        let flags = need(processed.physics_state_fields().flags_8cc, "flags8cc")?; //a528
        let translation = need(processed.deck_transform(), "deck transform")?.columns()[3]; //a530
        drop(processed);
        let radius = refs.try_grind_detection_radius().map_err(E::Cvar)?; //a540 one read
        let mut result = self.result.lock().map_err(|_| E::Poisoned)?;
        if flags & 0x0400_0000 != 0 {
            result.clear_count(); //a558, old slots/flags retained
        } else {
            // BoxTest overlap reads X/Y/Z only (grind_data_octree); the W lane
            // of the box never reaches a comparison.
            let position = [
                translation.xyz[0],
                translation.xyz[1],
                translation.xyz[2],
                translation.w.unwrap_or(0.0),
            ];
            let extent = [radius, radius, radius, 0.0];
            let bounds = OctreeBounds {
                minimum: std::array::from_fn(|i| position[i] - extent[i]),
                maximum: std::array::from_fn(|i| position[i] + extent[i]),
            };
            // Native argument sampled even though BoxTest97e0 does not consume it.
            let _debug = refs.try_debug_nearby_grinds().map_err(E::Cvar)?; //a598
            query(bounds, &mut result).map_err(E::Query)?; //a5b0, ignorevertical=false
        }
        if result.count() == 40 {
            overlay.require().map_err(E::Overlay)?;
        }
        Ok(())
    }
}
