//! Retained selected model palettes from actual presentation matrix publication.
use super::{CharacterHierarchy, CharacterScene, CharacterSkinningError, CharacterSkinningState};
use crate::animation::AnimationMatrix;
use std::{ops::Range, sync::Arc};

/// `CACPartDB::mPartSlotNames` 0x830073f0 for `SKATE_DECK` (slot 11).
const SKATE_DECK_COMPONENT: &str = "SkateBoard";
/// `SkaterPresentation::InitAnimation` 0x827e1d54: `mSkateboardBone` is this hierarchy bone.
const SKATEBOARD_ROOT: &str = "skateboard_root";

#[derive(Debug)]
pub enum CharacterPaletteError {
    Skinning(CharacterSkinningError),
    MissingPart(usize),
    MissingMesh { part: usize, mesh: usize },
    MissingMatrices(usize),
    NotPublished,
    GenerationOverflow,
}
impl std::fmt::Display for CharacterPaletteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Skinning(e) => e.fmt(f),
            Self::MissingPart(p) => write!(f, "selected character palette part {p} absent"),
            Self::MissingMesh { part, mesh } => {
                write!(f, "selected character palette mesh {part}/{mesh} absent")
            }
            Self::MissingMatrices(p) => write!(
                f,
                "selected character palette part {p} has no current matrices"
            ),
            Self::NotPublished => {
                f.write_str("selected character palette has no completed publication")
            }
            Self::GenerationOverflow => {
                f.write_str("selected character palette generation overflow")
            }
        }
    }
}
impl std::error::Error for CharacterPaletteError {}
impl From<CharacterSkinningError> for CharacterPaletteError {
    fn from(e: CharacterSkinningError) -> Self {
        Self::Skinning(e)
    }
}
#[derive(Clone)]
struct PalettePart {
    matrices: Option<Vec<AnimationMatrix>>,
    /// The matrices the published rows were packed from.
    packed: Option<Vec<AnimationMatrix>>,
    rows: Option<Vec<[f32; 4]>>,
    meshes: Vec<Range<usize>>,
}
/// Holds the supplied real model and its one hierarchy inverse-bind allocation.
/// This is selected host admission, not native `SkaterPresEntity` construction.
/// A clone shares the identity: it is the same palette posed elsewhere.
#[derive(Clone)]
pub struct CharacterPalette {
    identity: CharacterPaletteIdentity,
    scene: Arc<CharacterScene>,
    skinning: CharacterSkinningState,
    /// The component `RemoveComponentInstances` took out of every LOD.
    removed_component: Option<usize>,
    parts: Vec<PalettePart>,
    generation: u64,
    published: bool,
    transforms_complete: bool,
    /// `mSkateboardBone`: the hierarchy index of `skateboard_root` (`HierarchyData::GetBone`).
    deck_bone: Option<usize>,
    /// That bone's matrix (global pose times inverse bind) from the last
    /// `update_bone_transforms`.
    deck_matrix: Option<AnimationMatrix>,
    /// The deck's world matrix the last publication placed it at.
    deck_world: Option<AnimationMatrix>,
    /// The local-to-world the last publication used.
    local_to_world: Option<AnimationMatrix>,
}
/// Host publication ownership; distinct palettes may share the same asset scene.
#[derive(Clone, Debug)]
pub struct CharacterPaletteIdentity(Arc<()>);
impl PartialEq for CharacterPaletteIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for CharacterPaletteIdentity {}
/// Borrowed original mesh-palette rows suitable for the existing GPU consumer.
pub struct PublishedCharacterRows<'a> {
    pub rows: &'a [[f32; 4]],
    pub generation: u64,
}
impl CharacterPalette {
    /// Uses the actual model's inverse-bind matrices/remaps for an explicit LOD.
    ///
    /// # Errors
    /// Returns an error for invalid model ownership or hierarchy remapping.
    pub fn new(
        scene: Arc<CharacterScene>,
        hierarchy: &CharacterHierarchy,
        lod: u8,
    ) -> Result<Self, CharacterPaletteError> {
        Self::with_default_bone(scene, hierarchy, lod, "Hips")
    }
    /// Same RemapBones/FillInvBPBuffer path; vehicles supply `Chassis`.
    pub fn with_default_bone(
        scene: Arc<CharacterScene>,
        hierarchy: &CharacterHierarchy,
        lod: u8,
        default_bone: &str,
    ) -> Result<Self, CharacterPaletteError> {
        let skinning = scene.build_skinning_state(lod, hierarchy, default_bone)?;
        Self::from_skinning(scene, skinning, None, skateboard_bone(hierarchy))
    }
    /// `cPresPedestrian::cPresPedestrian` 0x828122b0: with the model's
    /// `livingworld_models` `Hide_Skateboard` set, `RemoveComponentInstances`
    /// takes the `SkateBoard` component out before `InitSkinning`
    /// (0x828124e0..0x82812594).
    pub(crate) fn pedestrian(
        scene: Arc<CharacterScene>,
        hierarchy: &CharacterHierarchy,
        lod: u8,
        hide_skateboard: bool,
    ) -> Result<Self, CharacterPaletteError> {
        let removed = if hide_skateboard { skate_deck_component(&scene) } else { None };
        Self::pedestrian_without(scene, hierarchy, lod, removed)
    }
    fn pedestrian_without(
        scene: Arc<CharacterScene>,
        hierarchy: &CharacterHierarchy,
        lod: u8,
        removed: Option<usize>,
    ) -> Result<Self, CharacterPaletteError> {
        let skinning = scene.build_skinning_state_without(lod, hierarchy, "Hips", removed)?;
        Self::from_skinning(scene, skinning, removed, skateboard_bone(hierarchy))
    }
    /// Rebind a saved presentation's recorded remaps to the same original assets.
    /// A `SkateBoard` the recording left out of a LOD that has its models stays
    /// out at the next LOD switch. The deck bone comes from the recorded rig.
    pub(crate) fn from_replay_skinning(scene:Arc<CharacterScene>,skinning:CharacterSkinningState)->Result<Self,CharacterPaletteError> {
        let removed = skate_deck_component(&scene).filter(|&index| {
            scene.components[index].models.iter().flat_map(|model| &model.lods)
                .any(|lod| lod.level == skinning.lod_level)
                && !skinning.parts.iter().any(|part| part.component_index == index)
        });
        Self::from_skinning(scene, skinning, removed, None)
    }
    fn from_skinning(
        scene: Arc<CharacterScene>,
        skinning: CharacterSkinningState,
        removed_component: Option<usize>,
        deck_bone: Option<usize>,
    ) -> Result<Self, CharacterPaletteError> {
        let mut parts = Vec::with_capacity(skinning.parts.len());
        for part in &skinning.parts {
            let asset = &scene.components[part.component_index].models[part.model_index].lods
                [part.lod_index];
            let model = super::lod_render_model(asset)?;
            let mut cursor = 0;
            let mut meshes = Vec::with_capacity(model.meshes.len());
            for mesh in &model.meshes {
                let count = asset
                    .arena
                    .meshes
                    .get(mesh.mesh_index)
                    .ok_or(CharacterSkinningError::MissingMesh(mesh.mesh_index))?
                    .skin_palette
                    .len()
                    * 3;
                meshes.push(cursor..cursor + count);
                cursor += count;
            }
            parts.push(PalettePart {
                matrices: None,
                packed: None,
                rows: None,
                meshes,
            });
        }
        Ok(Self {
            identity: CharacterPaletteIdentity(Arc::new(())),
            scene,
            skinning,
            removed_component,
            parts,
            generation: 0,
            published: false,
            transforms_complete: false,
            deck_bone,
            deck_matrix: None,
            deck_world: None,
            local_to_world: None,
        })
    }
    #[must_use]
    pub fn identity(&self) -> CharacterPaletteIdentity {
        self.identity.clone()
    }
    #[must_use]
    pub fn scene(&self) -> &Arc<CharacterScene> {
        &self.scene
    }
    #[must_use]
    pub fn skinning(&self) -> &CharacterSkinningState {
        &self.skinning
    }
    /// The publication count the GPU upload compares.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Start this copy's publication count elsewhere, so its publications never
    /// repeat another copy's.
    pub(crate) fn set_generation(&mut self, generation: u64) {
        self.generation = generation;
    }

    /// Local matrices for CPU skinning, including a retained published pose.
    /// StartJobs (0x827dbb90) gathers these before BeginSkinJob; EndJobs then
    /// packs them for the GPU (0x828221e0). Our upload moves the original local
    /// matrices to `packed`, separately from the world-space rows. Replay exit
    /// can skin the retained game pose there without another simulation tick.
    pub(crate) fn cloth_matrices(
        &self,
        part: usize,
    ) -> Result<&[AnimationMatrix], CharacterPaletteError> {
        let selected = self
            .parts
            .get(part)
            .ok_or(CharacterPaletteError::MissingPart(part))?;
        let matrices = if self.published {
            &selected.packed
        } else {
            // An unfinished/failed update must never use the previous pose.
            &selected.matrices
        };
        matrices
            .as_deref()
            .ok_or(CharacterPaletteError::MissingMatrices(part))
    }

    /// Pedestrian SwitchLOD82806528 calls RemapBones again, while the existing
    /// shared inverse-bind allocation remains unchanged. The caller must then
    /// repack from the synchronized hierarchy pose, not evaluate animation.
    /// Removed component instances stay out of the new LOD.
    pub fn select_pedestrian_lod(&mut self,hierarchy:&CharacterHierarchy,lod:u8)
        ->Result<bool,CharacterPaletteError>{
        if self.skinning.lod_level==lod{return Ok(false);}
        let mut next=Self::pedestrian_without(Arc::clone(&self.scene),hierarchy,lod,self.removed_component)?;
        next.skinning.hierarchy_inverse_bind_pose=Arc::clone(&self.skinning.hierarchy_inverse_bind_pose);
        next.identity=self.identity.clone();
        next.generation=self.generation;
        *self=next;
        Ok(true)
    }

    /// `UpdateBoneTransforms827efb78`: fill every selected part from the actual
    /// synchronized hierarchy output, including the saved hips at slot zero.
    ///
    /// # Errors
    /// Returns the first remap/pose error; earlier part copies remain retained.
    pub fn update_bone_transforms(
        &mut self,
        matrices: &[AnimationMatrix],
    ) -> Result<(), CharacterPaletteError> {
        if crate::diagnostics::lean_trace::enabled() {
            for (bone, matrix) in matrices.iter().enumerate() {
                crate::diagnostics::lean_trace::record(
                    "palette_pose",
                    std::ptr::from_ref(self) as usize,
                    bone as u64,
                    matrix,
                );
            }
        }
        self.published = false;
        self.transforms_complete = false;
        self.deck_matrix = self.deck_bone.and_then(|bone| matrices.get(bone).copied());
        for (index, part) in self.parts.iter_mut().enumerate() {
            part.matrices = Some(
                self.scene
                    .pack_skin_matrices(&self.skinning, index, matrices)?,
            );
        }
        self.transforms_complete = true;
        Ok(())
    }

    /// `PackAndMultiplyMatricesForUpload828221e0`: X multiply then Y/Z/W fused
    /// additions, transpose, and retain the first three rows for each bone.
    /// The caller supplies the actual selected frame's local-to-world matrix.
    ///
    /// # Errors
    /// Returns an error if this frame has not supplied each part's matrices.
    pub fn pack_for_upload(
        &mut self,
        local_to_world: AnimationMatrix,
    ) -> Result<(), CharacterPaletteError> {
        self.published = false;
        if !self.transforms_complete {
            return Err(CharacterPaletteError::NotPublished);
        }
        self.transforms_complete = false;
        for (index, part) in self.parts.iter_mut().enumerate() {
            let matrices = part
                .matrices
                .take()
                .ok_or(CharacterPaletteError::MissingMatrices(index))?;
            pack_rows_into(&matrices, local_to_world, part.rows.get_or_insert_with(Vec::new));
            part.packed = Some(matrices);
        }
        self.deck_world = self.deck_frame(local_to_world);
        self.local_to_world = Some(local_to_world);
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(CharacterPaletteError::GenerationOverflow)?;
        self.published = true;
        Ok(())
    }

    /// Publish the same pose again under another local-to-world: an entity the
    /// presentation's restore moved between the ticks that posed it.
    ///
    /// # Errors
    /// Rejects a palette that has not published a pose.
    pub fn repack_for_upload(
        &mut self,
        local_to_world: AnimationMatrix,
    ) -> Result<(), CharacterPaletteError> {
        if !self.published {
            return Err(CharacterPaletteError::NotPublished);
        }
        for (index, part) in self.parts.iter_mut().enumerate() {
            let matrices = part
                .packed
                .as_ref()
                .ok_or(CharacterPaletteError::MissingMatrices(index))?;
            pack_rows_into(matrices, local_to_world, part.rows.get_or_insert_with(Vec::new));
        }
        self.deck_world = self.deck_frame(local_to_world);
        self.local_to_world = Some(local_to_world);
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(CharacterPaletteError::GenerationOverflow)?;
        Ok(())
    }

    /// The deck bone's frame under a local-to-world: its matrix times its bind pose (the
    /// inverse of its inverse bind), placed by the local-to-world.
    fn deck_frame(&self, local_to_world: AnimationMatrix) -> Option<AnimationMatrix> {
        let bone = self.deck_bone?;
        let bind = inverse(self.skinning.hierarchy_inverse_bind_pose.get(bone)?)?;
        let pose = crate::actor_face_playback::presentation::move_root::multiply(self.deck_matrix?, bind);
        Some(crate::actor_face_playback::presentation::move_root::multiply(local_to_world, pose))
    }

    /// `SkaterPresentation::GetDeckMatrix` 0x8282e940 of the published pose: the deck's world
    /// matrix (the `skateboard_root` pose, the hips transform and the root), its columns being
    /// the native rows; none before a publication or without a deck bone.
    #[must_use]
    pub fn deck_world(&self) -> Option<AnimationMatrix> {
        self.deck_world.filter(|_| self.published)
    }

    /// PC Object Dropper: whether part `part` belongs to the skateboard (the recipe's
    /// `SkateBoard`, `SkateTruck` and `SkateWheel` components), not the skater.
    #[must_use]
    pub fn is_board_part(&self, part: usize) -> bool {
        self.skinning
            .parts
            .get(part)
            .and_then(|part| self.scene.components.get(part.component_index))
            .is_some_and(|component| {
                let name = component.name.to_ascii_lowercase();
                name.starts_with("skateboard") || name.starts_with("skatetruck") || name.starts_with("skatewheel")
            })
    }

    /// The local-to-world of the last publication (the PC Object Dropper moves a copy of this
    /// skater's posed board onto a dropped board with it).
    #[must_use]
    pub fn published_local_to_world(&self) -> Option<AnimationMatrix> {
        self.local_to_world.filter(|_| self.published)
    }

    pub(crate) fn has_deck_bone(&self) -> bool {
        self.deck_bone.is_some()
    }

    /// A recorded palette's `mSkateboardBone`, from the rig recorded with it.
    pub(crate) fn set_deck_bone(&mut self, bone: usize) {
        self.deck_bone = Some(bone);
    }

    /// # Errors
    /// Rejects unpublished/stale partial work and an absent selected mesh.
    pub fn mesh_rows(
        &self,
        part: usize,
        mesh: usize,
    ) -> Result<PublishedCharacterRows<'_>, CharacterPaletteError> {
        if !self.published {
            return Err(CharacterPaletteError::NotPublished);
        }
        let selected = self
            .parts
            .get(part)
            .ok_or(CharacterPaletteError::MissingPart(part))?;
        let range = selected
            .meshes
            .get(mesh)
            .ok_or(CharacterPaletteError::MissingMesh { part, mesh })?;
        let rows = selected
            .rows
            .as_ref()
            .and_then(|v| v.get(range.clone()))
            .ok_or(CharacterPaletteError::MissingMesh { part, mesh })?;
        Ok(PublishedCharacterRows {
            rows,
            generation: self.generation,
        })
    }
}

/// A general 4x4 inverse (cofactors, in double precision); none for a singular matrix.
fn inverse(m: &AnimationMatrix) -> Option<AnimationMatrix> {
    let a: [f64; 16] = std::array::from_fn(|index| f64::from(m[index]));
    let mut out = [0.0_f64; 16];
    out[0] = a[5] * a[10] * a[15] - a[5] * a[11] * a[14] - a[9] * a[6] * a[15] + a[9] * a[7] * a[14] + a[13] * a[6] * a[11] - a[13] * a[7] * a[10];
    out[4] = -a[4] * a[10] * a[15] + a[4] * a[11] * a[14] + a[8] * a[6] * a[15] - a[8] * a[7] * a[14] - a[12] * a[6] * a[11] + a[12] * a[7] * a[10];
    out[8] = a[4] * a[9] * a[15] - a[4] * a[11] * a[13] - a[8] * a[5] * a[15] + a[8] * a[7] * a[13] + a[12] * a[5] * a[11] - a[12] * a[7] * a[9];
    out[12] = -a[4] * a[9] * a[14] + a[4] * a[10] * a[13] + a[8] * a[5] * a[14] - a[8] * a[6] * a[13] - a[12] * a[5] * a[10] + a[12] * a[6] * a[9];
    out[1] = -a[1] * a[10] * a[15] + a[1] * a[11] * a[14] + a[9] * a[2] * a[15] - a[9] * a[3] * a[14] - a[13] * a[2] * a[11] + a[13] * a[3] * a[10];
    out[5] = a[0] * a[10] * a[15] - a[0] * a[11] * a[14] - a[8] * a[2] * a[15] + a[8] * a[3] * a[14] + a[12] * a[2] * a[11] - a[12] * a[3] * a[10];
    out[9] = -a[0] * a[9] * a[15] + a[0] * a[11] * a[13] + a[8] * a[1] * a[15] - a[8] * a[3] * a[13] - a[12] * a[1] * a[11] + a[12] * a[3] * a[9];
    out[13] = a[0] * a[9] * a[14] - a[0] * a[10] * a[13] - a[8] * a[1] * a[14] + a[8] * a[2] * a[13] + a[12] * a[1] * a[10] - a[12] * a[2] * a[9];
    out[2] = a[1] * a[6] * a[15] - a[1] * a[7] * a[14] - a[5] * a[2] * a[15] + a[5] * a[3] * a[14] + a[13] * a[2] * a[7] - a[13] * a[3] * a[6];
    out[6] = -a[0] * a[6] * a[15] + a[0] * a[7] * a[14] + a[4] * a[2] * a[15] - a[4] * a[3] * a[14] - a[12] * a[2] * a[7] + a[12] * a[3] * a[6];
    out[10] = a[0] * a[5] * a[15] - a[0] * a[7] * a[13] - a[4] * a[1] * a[15] + a[4] * a[3] * a[13] + a[12] * a[1] * a[7] - a[12] * a[3] * a[5];
    out[14] = -a[0] * a[5] * a[14] + a[0] * a[6] * a[13] + a[4] * a[1] * a[14] - a[4] * a[2] * a[13] - a[12] * a[1] * a[6] + a[12] * a[2] * a[5];
    out[3] = -a[1] * a[6] * a[11] + a[1] * a[7] * a[10] + a[5] * a[2] * a[11] - a[5] * a[3] * a[10] - a[9] * a[2] * a[7] + a[9] * a[3] * a[6];
    out[7] = a[0] * a[6] * a[11] - a[0] * a[7] * a[10] - a[4] * a[2] * a[11] + a[4] * a[3] * a[10] + a[8] * a[2] * a[7] - a[8] * a[3] * a[6];
    out[11] = -a[0] * a[5] * a[11] + a[0] * a[7] * a[9] + a[4] * a[1] * a[11] - a[4] * a[3] * a[9] - a[8] * a[1] * a[7] + a[8] * a[3] * a[5];
    out[15] = a[0] * a[5] * a[10] - a[0] * a[6] * a[9] - a[4] * a[1] * a[10] + a[4] * a[2] * a[9] + a[8] * a[1] * a[6] - a[8] * a[2] * a[5];
    let determinant = a[0] * out[0] + a[1] * out[4] + a[2] * out[8] + a[3] * out[12];
    if determinant == 0.0 || !determinant.is_finite() {
        return None;
    }
    #[expect(clippy::cast_possible_truncation, reason = "the inverse's single-precision lanes")]
    let inverse = std::array::from_fn(|index| (out[index] / determinant) as f32);
    Some(inverse)
}

/// `RemoveComponentInstances` finds the component by its pooled name.
fn skate_deck_component(scene: &CharacterScene) -> Option<usize> {
    scene.components.iter().position(|component| component.name == SKATE_DECK_COMPONENT)
}

fn skateboard_bone(hierarchy: &CharacterHierarchy) -> Option<usize> {
    hierarchy.bone_names.iter().position(|name| name.eq_ignore_ascii_case(SKATEBOARD_ROOT))
}

/// Reuse each part's output allocation, as the retained matrix buffer in
/// PackAndMultiplyMatricesForUpload 0x828221e0 does. Every matrix is recomputed.
fn pack_rows_into(matrices: &[AnimationMatrix], world: AnimationMatrix, rows: &mut Vec<[f32; 4]>) {
    rows.clear();
    rows.reserve(matrices.len() * 3);
    for matrix in matrices {
        let matrix = crate::actor_face_playback::presentation::move_root::multiply(world, *matrix);
        rows.extend((0..3).map(|row| std::array::from_fn(|column| matrix[column * 4 + row])));
    }
}

#[cfg(test)]
fn pack_rows(matrices: &[AnimationMatrix], world: AnimationMatrix) -> Vec<[f32; 4]> {
    let mut rows = Vec::new();
    pack_rows_into(matrices, world, &mut rows);
    rows
}

#[cfg(test)]
#[path = "character_palette_upload_tests.rs"]
mod tests;
