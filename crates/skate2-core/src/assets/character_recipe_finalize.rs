//! Selected `CharacterEntity` finalization over the actual mounted allocations.
//! Clone policies 0 and 3 retain each registry instance (`82d04994`); material rebind returns
//! without stores (`82d08fd8..82d08fe4`). No material/geometry copies are made.
use crate::{character::CharacterScene, presentation::PresentationInstance};
use skate2_formats::hash::rw_hash32_string;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RecipeInstance {
    component: usize,
    model: usize,
    lod: usize,
    instance: usize,
}
pub(crate) struct FinalizedRecipeScene {
    scene: Arc<CharacterScene>,
    resolved: Vec<RecipeInstance>,
    lods: Vec<Vec<RecipeInstance>>,
    ready: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RecipeFinalizeError {
    CloneType(u32),
    MissingInstance(u64),
    DuplicateInstance(u64),
    EmptyLods,
    EmptyLodZero,
    DuplicateComponentHash(u32),
    NotReady,
}
impl std::fmt::Display for RecipeFinalizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CloneType(v) => {
                write!(f, "recipe clone policy {v} requires another instance path")
            }
            Self::MissingInstance(v) => write!(f, "recipe instance {v:016x} absent from its arena"),
            Self::DuplicateInstance(v) => write!(
                f,
                "recipe instance {v:016x} has ambiguous registry identity"
            ),
            Self::EmptyLods => f.write_str("recipe has no visible character LODs"),
            Self::EmptyLodZero => f.write_str("recipe LOD0 has no parts"),
            Self::DuplicateComponentHash(v) => write!(
                f,
                "recipe component hash collision {v:08x} needs native map replacement"
            ),
            Self::NotReady => f.write_str("recipe scene finalization incomplete"),
        }
    }
}
impl FinalizedRecipeScene {
    pub(crate) fn new(scene: Arc<CharacterScene>) -> Self {
        Self {
            scene,
            resolved: Vec::new(),
            lods: Vec::new(),
            ready: false,
        }
    }
    /// Selected initial load only. Late admission failure retains resolved handles
    /// and any already-published LOD prefix; it never reports an assets-ready token.
    pub(crate) fn try_finalize(&mut self, clone_type: u32) -> Result<(), RecipeFinalizeError> {
        use RecipeFinalizeError as E;
        self.ready = false;
        if !matches!(clone_type, 0..=3) {
            return Err(E::CloneType(clone_type));
        }
        let mut components = (0..self.scene.components.len()).collect::<Vec<_>>();
        components.sort_by_key(|&index| rw_hash32_string(&self.scene.components[index].name));
        for pair in components.windows(2) {
            let hash = rw_hash32_string(&self.scene.components[pair[0]].name);
            if hash == rw_hash32_string(&self.scene.components[pair[1]].name) {
                return Err(E::DuplicateComponentHash(hash));
            }
        }
        self.resolved.clear();
        for &component in &components {
            let data = &self.scene.components[component];
            let mut models = (0..data.models.len()).collect::<Vec<_>>();
            models.sort_by_key(|&index| data.models[index].asset_id);
            for model in models {
                let mut lods = (0..data.models[model].lods.len()).collect::<Vec<_>>();
                lods.sort_by_key(|&index| data.models[model].lods[index].level);
                for lod in lods {
                    let data = &data.models[model].lods[lod];
                    let mut matches = data
                        .arena
                        .instances
                        .iter()
                        .enumerate()
                        .filter(|(_, instance)| instance.guid == data.asset_id);
                    let (instance, _) = matches.next().ok_or(E::MissingInstance(data.asset_id))?;
                    if matches.next().is_some() {
                        return Err(E::DuplicateInstance(data.asset_id));
                    }
                    self.resolved.push(RecipeInstance {
                        component,
                        model,
                        lod,
                        instance,
                    });
                }
            }
        }
        // Native clone0/3 RebindToMaterials walks components and performs no stores.
        // BuildTInstanceArray destroys the previous lists before sizing the new ones.
        self.lods.clear();
        let has_rostral = self
            .scene
            .components
            .iter()
            .any(|c| rw_hash32_string(&c.name) == rw_hash32_string("Rostral"));
        let visible = |handle: &RecipeInstance| {
            !has_rostral
                || rw_hash32_string(&self.scene.components[handle.component].name)
                    != rw_hash32_string("Beard")
        };
        let count = self
            .resolved
            .iter()
            .filter(|h| visible(h))
            .map(|h| {
                usize::from(self.scene.components[h.component].models[h.model].lods[h.lod].level)
                    + 1
            })
            .max()
            .ok_or(E::EmptyLods)?;
        self.lods.resize_with(count, Vec::new);
        for &handle in &self.resolved {
            if visible(&handle) {
                let level = usize::from(
                    self.scene.components[handle.component].models[handle.model].lods[handle.lod]
                        .level,
                );
                self.lods[level].push(handle);
            }
        }
        if self.lods[0].is_empty() {
            return Err(E::EmptyLodZero);
        }
        self.ready = true;
        Ok(())
    }
    pub(crate) fn scene(&self) -> Result<&Arc<CharacterScene>, RecipeFinalizeError> {
        if !self.ready {
            return Err(RecipeFinalizeError::NotReady);
        }
        Ok(&self.scene)
    }
    /// Original component/model/LOD map order, including non-rendered parts.
    /// GetTInstanceWithCollisionVolume82d10f10 traverses this same ordering.
    pub(crate) fn ordered_instances(
        &self,
    ) -> Result<
        impl Iterator<Item = (&crate::character::CharacterLodAsset, &PresentationInstance)>,
        RecipeFinalizeError,
    > {
        if !self.ready {
            return Err(RecipeFinalizeError::NotReady);
        }
        Ok(self.resolved.iter().map(|h| {
            let lod = &self.scene.components[h.component].models[h.model].lods[h.lod];
            (lod, &lod.arena.instances[h.instance])
        }))
    }
    pub(crate) fn instances(
        &self,
        level: u8,
    ) -> Result<impl Iterator<Item = &PresentationInstance>, RecipeFinalizeError> {
        if !self.ready {
            return Err(RecipeFinalizeError::NotReady);
        }
        Ok(self
            .lods
            .get(usize::from(level))
            .into_iter()
            .flatten()
            .map(|h| {
                &self.scene.components[h.component].models[h.model].lods[h.lod]
                    .arena
                    .instances[h.instance]
            }))
    }
}
#[cfg(test)]
#[path = "character_recipe_finalize_tests.rs"]
mod tests;
