//! Native global spline ID sequences and per-manager grab ownership. Reuses
//! the original DMO registration logic for vehicles; no category-local IDs.
use super::dmo_models::{DmoGrabSpline, DmoGrindSpline};
use std::collections::BTreeMap;

pub(crate) struct LivingWorldSplineIds {
    pub(crate) next_grab: u32,
    pub(crate) next_grind: u32,
}
impl LivingWorldSplineIds {
    /// .data83025058/5c both1, GetNext82e4d368/388 wrapping post-increment.
    pub(crate) const fn new() -> Self {
        Self {
            next_grab: 1,
            next_grind: 1,
        }
    }
    pub(crate) fn register_grinds(
        &mut self,
        sources: &[DmoGrindSpline],
    ) -> BTreeMap<u32, DmoGrindSpline> {
        let mut result = BTreeMap::new();
        for source in sources {
            let id = self.next_grind;
            self.next_grind = id.wrapping_add(1);
            // AddGrindSpline82e62ea0 checks size before operator[], including
            // wrapped/repeated keys. Full maps still consume the global ID.
            if result.len() < 64 {
                result.insert(id, source.clone());
            }
        }
        result
    }
}
pub(crate) struct GrabbableObjectMan {
    owners: BTreeMap<u32, u32>,
}
impl GrabbableObjectMan {
    pub(crate) const fn new() -> Self {
        Self {
            owners: BTreeMap::new(),
        }
    }
    pub(crate) fn owner(&self, id: u32) -> u32 {
        self.owners.get(&id).copied().unwrap_or(0)
    }
    pub(crate) fn remove(&mut self, id: u32) {
        self.owners.remove(&id);
    }
    /// AddGrabSpline82e638b0: independent local64/global512 gates, in order.
    pub(crate) fn register(
        &mut self,
        entity: u32,
        sources: &[DmoGrabSpline],
        ids: &mut LivingWorldSplineIds,
    ) -> BTreeMap<u32, DmoGrabSpline> {
        let mut result = BTreeMap::new();
        for source in sources {
            let id = ids.next_grab;
            ids.next_grab = id.wrapping_add(1);
            if result.len() < 64 {
                result.insert(id, source.clone());
                if self.owners.len() < 512 {
                    self.owners.insert(id, entity);
                }
            }
        }
        result
    }
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.owners.len()
    }
}
