//! CensusTargetCache.GetCensusTarget82e62b70 / LWAttribCache.Refresh82e4fd00.
//! Shared by native ambient categories, keyed by model (not entity/recipe).
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    living_world_recipe_manager::LIVING_WORLD_MODELS_CLASS_ID,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug)]
pub(crate) struct LwAttribCache {
    pub(crate) hi_lod_radius: [f32; 4],
    pub(crate) lo_lod_radius: [f32; 4],
    pub(crate) hi_losim_radius: [f32; 4],
    pub(crate) lo_losim_radius: [f32; 4],
    pub(crate) model_key: u64,
}
impl LwAttribCache {
    fn read(
        vault: &Arc<AttributeRuntimeVault>,
        model_key: u64,
    ) -> Result<Self, AttributeRuntimeError> {
        let model = vault.instance(LIVING_WORLD_MODELS_CLASS_ID, model_key);
        let scalar = |key| -> Result<[f32; 4], AttributeRuntimeError> {
            // Source DefaultDataArea(4)=0; raw VMX splats ALL four lanes.
            Ok([f32::from_bits(model.try_scalar_word(key, 0)?.unwrap_or(0)); 4])
        };
        // Full64 ori/insrdi fd3c/fd90/fdec/fe48, not decompiler's low words.
        let hi_lod_radius = scalar(0x99dd_8f0f_8226_9171)?;
        let lo_lod_radius = scalar(0xf918_c455_5e23_b93a)?;
        let hi_losim_radius = scalar(0x1023_caad_fc90_b0a1)?;
        let lo_losim_radius = scalar(0x1b9d_7cde_c5ba_d784)?;
        Ok(Self {
            hi_lod_radius,
            lo_lod_radius,
            hi_losim_radius,
            lo_losim_radius,
            model_key,
        })
    }
}

pub(crate) struct CensusTargetCache {
    entries: BTreeMap<u64, Option<Arc<LwAttribCache>>>,
}
#[derive(Debug)]
pub(crate) enum AttribCacheError {
    Capacity,
    Attribute(AttributeRuntimeError),
}
impl CensusTargetCache {
    pub(crate) const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
    pub(crate) fn get(
        &mut self,
        model_key: u64,
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Arc<LwAttribCache>, AttribCacheError> {
        // operator[] creates its null slot BEFORE allocation/Refresh. Keep
        // that slot on checked attribute failure and retry it on next lookup.
        if !self.entries.contains_key(&model_key) {
            if self.entries.len() >= 256 {
                return Err(AttribCacheError::Capacity);
            }
            self.entries.insert(model_key, None);
        }
        let slot = self
            .entries
            .get_mut(&model_key)
            .expect("inserted cache slot");
        if let Some(cache) = slot {
            return Ok(Arc::clone(cache));
        }
        let cache =
            Arc::new(LwAttribCache::read(vault, model_key).map_err(AttribCacheError::Attribute)?);
        *slot = Some(Arc::clone(&cache));
        Ok(cache)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute_runtime::tests::{collection, definition, graph, node};

    // UNRUN: model-key sharing, exact splats, capacity and existing-key lookup.
    #[test]
    fn model_cache_keeps_one_occurrence_and_existing_keys_work_at_capacity() {
        let keys = [
            0x99dd_8f0f_8226_9171,
            0xf918_c455_5e23_b93a,
            0x1023_caad_fc90_b0a1,
            0x1b9d_7cde_c5ba_d784,
        ];
        let words = [
            12.5_f32.to_bits(),
            87.0_f32.to_bits(),
            0x8000_0000,
            0x7fc0_1234,
        ];
        let mut vault = graph(
            LIVING_WORLD_MODELS_CLASS_ID,
            keys.into_iter().map(|key| definition(key, 0, 4)).collect(),
        );
        collection(
            &mut vault,
            LIVING_WORLD_MODELS_CLASS_ID,
            900,
            0,
            None,
            keys.into_iter()
                .zip(words)
                .map(|(key, word)| node(key, 0x40, word))
                .collect(),
        );
        let vault = Arc::new(vault);
        let mut caches = CensusTargetCache::new();
        let first = caches.get(900, &vault).unwrap();
        for (vector, word) in [
            first.hi_lod_radius,
            first.lo_lod_radius,
            first.hi_losim_radius,
            first.lo_losim_radius,
        ]
        .into_iter()
        .zip(words)
        {
            assert_eq!(vector.map(f32::to_bits), [word; 4]);
        }
        for key in 0..255 {
            caches.get(key, &vault).unwrap();
        }
        assert!(matches!(
            caches.get(901, &vault),
            Err(AttribCacheError::Capacity)
        ));
        assert!(Arc::ptr_eq(&first, &caches.get(900, &vault).unwrap()));
        assert_eq!(first.model_key, 900);
        assert_eq!(caches.entries.len(), 256);
    }
}
