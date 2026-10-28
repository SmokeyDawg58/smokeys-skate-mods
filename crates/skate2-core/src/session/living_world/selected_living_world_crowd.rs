//! `Sk8::CrowdPresentation` (0x827ea608): a contest crowd drawn as imposters. Each change of the
//! challenge's `cCrowdPositionBuffer` (`Presentation::ReceiveCrowdBufferChanged` 0x827eefa8)
//! rebuilds it (`Update` 0x827e8320): at most six imposter targets, each a `cNoSimPresPedestrian`
//! of a registered pedestrian recipe playing a spectator clip, drawn into a cell of an atlas, and
//! at most 200 imposters, quads facing the camera at the buffer's positions that show a target's
//! cell in their own colours.
use super::SelectedRecipeFrame;
use crate::{
    actor_animation_construction::{
        hands_face::sqt_jobs::{QueuedSqtProgram, matrix::{MatrixProgram, ProgramOutput}},
        initialization::bind_pose::OwnedAnimationPoseBuffer,
    },
    actor_animation_tree::{
        AnimationClipRef, AnimationNodeHandle, AnimationTreePool,
        eval::AnimationEvalParams, time::AnimationProperties,
    },
    actor_face_playback::{FaceCommands, presentation::move_root::matrix_inverse::try_inverse},
    animation::AnimationMatrix,
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    challenge::crowd::CrowdPositionBuffer,
    character::{CharacterHierarchy, palette::CharacterPalette},
    input_cvar_runtime::InputCvarReferences,
    living_world_recipe_manager::attributes::RecipeAttributes,
    reckoning_numeric::HostReckoningNumeric,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

pub(crate) const MAX_TARGETS: usize = 6;
pub(crate) const MAX_IMPOSTERS: usize = 200;
/// `cLivingWorldPresEntityManager::NoSimPedestrians`, a `fixed_map` of 20.
const MAX_NO_SIM_PEDESTRIANS: usize = 20;
/// `GetRegisteredLWRecipeIDs(ids, 50)`.
const MAX_RECIPES: usize = 50;
/// `GetValidCrowdImposterModelKeys`' `fixed_set` of 256.
const MAX_MODEL_KEYS: usize = 256;
/// `ResolveCategoryGroupToModel` weighs at most sixteen members.
const MAX_GROUP_WEIGHTS: u32 = 16;
/// `kImposterClipNames`.
const CLIP_NAMES: [&[u8]; 6] = [
    b"NPC_SPECT_CRWD01_N_0_CYC",
    b"NPC_SPECT_CRWD02_N_0_CYC",
    b"NPC_SPECT_CRWD03_N_0_CYC",
    b"NPC_SPECT_CRWD04_N_0_CYC",
    b"NPC_SPECT_CRWD05_N_0_CYC",
    b"NPC_SPECT_CRWD06_N_0_CYC",
];
const GROUPS_CLASS: u64 = 0xd4dd_1d82_b998_1bc5;
const MEMBERS: u64 = 0xd5e1_267e_2d71_5124;
const ENTITIES_CLASS: u64 = 0x60d7_03c7_7cab_1631;
const MODELS_CLASS: u64 = 0x70f2_41a7_4e74_0a66;
/// The category group whose leaves hold the crowd's models, and the one a group without
/// members falls back to.
const CROWD_GROUPS: u64 = 0x96a0_9047_6c57_f4b9;
/// `Attrib::StringToKey("default")`: the model of a recipe without a crowd model.
const DEFAULT_MODEL: u64 = 0xd7ed_bd36_2d7d_2152;
/// `livingworld_models`' colorize lists (`i_colorize_red`, `i_colorize_blue`).
const RED_COLOURS: u64 = 0xdf76_d7d7_7385_7edb;
const BLUE_COLOURS: u64 = 0x1202_6e2e_ed18_cc8d;
/// `livingworld_models` `Hide_Skateboard`.
const HIDE_SKATEBOARD: u64 = 0x0626_fa10_792b_ba58;
/// `cLivingWorldPresEntityManager::mRegisteredBindPoseId`: the PRES ACS's first fixed SQT.
const BIND_POSE_SQT: u32 = 0;
/// `EvalNoSimPedestrians`' `tAnimationEvalParams::BlendFrames` (`ALL_FETCHES`).
const ALL_FETCHES: u32 = 2;
/// `PauseMan` flag 4, which `UpdateNoSimPedestrians` stops the clips for.
const PAUSE_FLAG: u32 = 4;
/// `CrowdImposterRenderer`'s cell, in pixels.
pub const IMPOSTER_CELL: [u32; 2] = [364, 310];
/// `PostFX::m_main2Rt`, the atlas the targets draw into, at the original's 720p.
pub const IMPOSTER_ATLAS: [u32; 2] = [1280, 720];

fn word(data: &[u8], at: usize) -> u64 {
    u64::from_be_bytes(data[at..at + 8].try_into().expect("eight bytes"))
}

fn float(data: &[u8], at: usize) -> f32 {
    f32::from_be_bytes(data[at..at + 4].try_into().expect("four bytes"))
}

fn attribute(error: impl std::fmt::Debug) -> String {
    format!("crowd attribute: {error:?}")
}

fn random() -> Result<u32, String> {
    crate::native_skater_random::rw_math::global_next().map_err(|e| format!("crowd rw::math::Random: {e:?}"))
}

/// `AttribSysUtils::GetLeafCollections(class, key)`, or the key itself when it has none.
fn leaves_or_self(vault: &Arc<AttributeRuntimeVault>, class: u64, key: u64) -> Result<Vec<u64>, String> {
    let leaves = vault.leaf_descendants_sorted(class, key).map_err(attribute)?;
    Ok(if leaves.is_empty() { vec![key] } else { leaves })
}

/// A category's member entity's `livingworld_models` parent (layout +8), when both exist.
fn entity_model(vault: &Arc<AttributeRuntimeVault>, entity: u64) -> Result<Option<u64>, String> {
    let entity = vault.instance(ENTITIES_CLASS, entity);
    if entity.is_null() {
        return Ok(None);
    }
    let model = entity.try_cached_layout_field(8, 8).map_err(attribute)?.map_or(0, |value| word(value.bytes(), 0));
    Ok((!vault.instance(MODELS_CLASS, model).is_null()).then_some(model))
}

/// `GetValidCrowdImposterModelKeys` 0x827e43c8: every leaf model of every member entity of every
/// category of the crowd's leaf category groups. A full set ends the walk.
fn valid_model_keys(vault: &Arc<AttributeRuntimeVault>) -> Result<BTreeSet<u64>, String> {
    let mut keys = BTreeSet::new();
    for group in vault.leaf_descendants_sorted(GROUPS_CLASS, CROWD_GROUPS).map_err(attribute)? {
        let group = vault.instance(GROUPS_CLASS, group);
        if group.is_null() {
            continue;
        }
        let categories = group.try_reference_members(MEMBERS, true).map_err(attribute)?;
        for index in 0..group.try_length(MEMBERS).map_err(attribute)? {
            let category = categories.as_ref().and_then(|members| members.get(index)).map_or(0, |data| word(data, 8));
            let category = vault.instance(crate::living_world_grind_services::census::targets::CATEGORIES_CLASS, category);
            if category.is_null() {
                continue;
            }
            let entities = category.try_reference_members(MEMBERS, false).map_err(attribute)?;
            for entity in 0..category.try_length(MEMBERS).map_err(attribute)? {
                let entity = entities.as_ref().and_then(|members| members.get(entity)).map_or(0, |data| word(data, 8));
                let Some(model) = entity_model(vault, entity)? else { continue };
                for key in leaves_or_self(vault, MODELS_CLASS, model)? {
                    if keys.len() >= MAX_MODEL_KEYS {
                        return Ok(keys);
                    }
                    keys.insert(key);
                }
            }
        }
    }
    Ok(keys)
}

/// `ResolveCategoryGroupToModel` 0x827e4740: a member category of the group (the crowd's groups
/// for one without members) by the first sixteen weights (`DiscreteRandom`, the first without a
/// pick), its entity at `random`, then that entity's leaf model at `random`; 0 when a step has
/// nothing.
fn resolve_category_group_to_model(vault: &Arc<AttributeRuntimeVault>, random_word: u32, group: u64) -> Result<u64, String> {
    use crate::living_world_grind_services::census::targets::{CATEGORIES_CLASS, vector_discrete_choice};
    let mut group = vault.instance(GROUPS_CLASS, group);
    if group.is_null() || group.try_length(MEMBERS).map_err(attribute)? == 0 {
        group = vault.instance(GROUPS_CLASS, CROWD_GROUPS);
    }
    let members = group.try_reference_members(MEMBERS, true).map_err(attribute)?;
    let member = |index: u32| members.as_ref().and_then(|members| members.get(index));
    let count = group.try_length(MEMBERS).map_err(attribute)?.min(MAX_GROUP_WEIGHTS);
    let weights: Vec<f32> = (0..count).map(|index| member(index).map_or(0.0, |data| float(data, 24))).collect();
    let index = vector_discrete_choice(&weights, random()?).unwrap_or(0);
    let category = vault.instance(CATEGORIES_CLASS, member(u32::try_from(index).map_err(|_| "crowd category index")?).map_or(0, |data| word(data, 8)));
    let length = category.try_length(MEMBERS).map_err(attribute)?;
    if length == 0 {
        return Ok(0);
    }
    let entities = category.try_reference_members(MEMBERS, false).map_err(attribute)?;
    let entity = entities.as_ref().and_then(|members| members.get(random_word % length)).map_or(0, |data| word(data, 8));
    let Some(model) = entity_model(vault, entity)? else { return Ok(0) };
    let leaves = leaves_or_self(vault, MODELS_CLASS, model)?;
    #[expect(clippy::cast_possible_truncation, reason = "at most the class's collections")]
    let pick = random_word as usize % leaves.len();
    Ok(leaves[pick])
}

/// A `livingworld_models` colour (`Vector4`), zero where the list has no such entry.
fn colour(model: &AttributeRuntimeInstance, key: u64, index: u32) -> Result<[f32; 4], String> {
    let Some(data) = model.try_vector_array_data(key, index, 4).map_err(attribute)? else { return Ok([0.0; 4]) };
    let bytes = data.bytes();
    if bytes.len() < 16 {
        return Err("crowd colour float4 truncated".into());
    }
    Ok(std::array::from_fn(|lane| float(bytes, lane * 4)))
}

/// `GetRandomCrowdImposterColor` 0x827d19f8: one draw indexes both of the target model's lists.
fn imposter_colours(vault: &Arc<AttributeRuntimeVault>, model_key: u64) -> Result<[[f32; 4]; 2], String> {
    let model = vault.instance(MODELS_CLASS, model_key);
    let draw = random()?;
    let mut colours = [[0.0; 4]; 2];
    for (colour_slot, key) in colours.iter_mut().zip([RED_COLOURS, BLUE_COLOURS]) {
        let length = model.try_length(key).map_err(attribute)?;
        if length == 0 {
            return Err("crowd imposter colour list empty (native divide trap)".into());
        }
        *colour_slot = colour(&model, key, draw % length)?;
    }
    Ok(colours)
}

/// `CrowdRecipeConfig`: the registered recipes that have crowd models, which of them the
/// fallback may still pick, their models, the buffer's category group and the targets made.
struct RecipeConfig {
    recipe_ids: Vec<u64>,
    selections: usize,
    asset_models: BTreeMap<u64, Vec<u64>>,
    category_group: u64,
    existing: BTreeMap<(u64, u64), usize>,
}

impl RecipeConfig {
    /// `GetCrowdRecipeConfig` 0x827e7f00: each valid model's asset (`GenerateModelAssetID`, a
    /// null asset left out), then the first fifty registered recipes (`GetRegisteredLWRecipeIDs`)
    /// that are one of them, in asset order.
    fn new(
        vault: &Arc<AttributeRuntimeVault>,
        mut asset_of: impl FnMut(u64) -> Result<u64, String>,
        registered: impl Iterator<Item = u64>,
        category_group: u64,
    ) -> Result<Self, String> {
        let mut asset_models: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
        for model in valid_model_keys(vault)? {
            let asset = asset_of(model)?;
            if asset != crate::living_world_grind_services::dmo_models::NULL_ASSET_ID {
                asset_models.entry(asset).or_default().push(model);
            }
        }
        let registered: Vec<u64> = registered.take(MAX_RECIPES).collect();
        let recipe_ids: Vec<u64> =
            asset_models.keys().filter(|asset| registered.contains(asset)).copied().take(MAX_RECIPES).collect();
        Ok(Self { selections: recipe_ids.len(), recipe_ids, asset_models, category_group, existing: BTreeMap::new() })
    }

    /// `GetRandomCrowdModel`'s fallback: a remaining recipe at the draw (swapped to the end of
    /// the remaining ones, which then exclude it) and one of its models at the same draw.
    fn fallback_model(&mut self, draw: u32) -> Option<(u64, u64)> {
        if self.selections == 0 {
            return None;
        }
        let last = self.selections - 1;
        let pick = draw as usize % self.selections;
        self.recipe_ids.swap(pick, last);
        self.selections = last;
        let asset = self.recipe_ids[last];
        let model = match self.asset_models.get(&asset) {
            Some(models) if !models.is_empty() => models[draw as usize % models.len()],
            _ => DEFAULT_MODEL,
        };
        Some((asset, model))
    }
}

/// The clip `GetRandomCrowdModel` gives a draw: the pointer table's `(draw % 24) & ~3`.
fn clip_index(draw: u32) -> usize {
    (draw % 24 / 4) as usize
}

/// `cNoSimPresPedestrian` (0x82812830): a presentation-only pedestrian whose pose is its clip
/// over the bind pose (`AddSQT(Clip, SetSQTById(mRegisteredBindPoseId))`), at the origin.
pub(crate) struct NoSimPedestrian {
    asset: u64,
    model_key: u64,
    colours: [[f32; 4]; 2],
    pool: AnimationTreePool,
    root: AnimationNodeHandle,
    properties: AnimationProperties,
    pose: OwnedAnimationPoseBuffer,
    palette: Arc<Mutex<CharacterPalette>>,
    hierarchy: Arc<CharacterHierarchy>,
    inverse_bind: Arc<[AnimationMatrix]>,
    /// `GetPresEntityWorldAABB` 0x827d1d00 of the last synchronized pose.
    bounds: Option<[[f32; 4]; 2]>,
}

/// `CrowdImposterTarget` (0x90 bytes): the no-sim pedestrian it draws and its model.
struct CrowdTarget {
    entity: u32,
    model_key: u64,
}

/// `CrowdImposter` (0x70 bytes).
struct CrowdImposter {
    target: usize,
    position: [f32; 4],
    colours: [[f32; 4]; 2],
}

#[derive(Default)]
pub(crate) struct CrowdPresentation {
    targets: Vec<CrowdTarget>,
    imposters: Vec<CrowdImposter>,
    /// `cLivingWorldPresEntityManager::NoSimPedestrians`, by entity ID.
    pedestrians: BTreeMap<u32, NoSimPedestrian>,
    /// `LivingWorldEntity::NoSimPresEntityId`, never reset.
    next_id: u32,
    /// The shared `PauseMan`.
    pause: Option<Arc<crate::selected_pause_manager::SelectedPauseManager>>,
}

impl CrowdPresentation {
    pub(crate) fn attach_pause(&mut self, pause: Arc<crate::selected_pause_manager::SelectedPauseManager>) {
        self.pause = Some(pause);
    }

    /// `Flush` 0x827d1888: every target's no-sim pedestrian removed (`RemoveNoSimPresPedestrian`),
    /// then the targets and imposters.
    pub(crate) fn flush(&mut self) {
        for target in self.targets.drain(..) {
            self.pedestrians.remove(&target.entity);
        }
        self.imposters.clear();
    }
}

/// `CrowdImposterRenderer::GetImposterTargetViewport` 0x827cfc20: the target's cell, the
/// atlas's spare width and height spread between its columns and rows.
#[expect(clippy::cast_precision_loss, reason = "the native's unsigned to float conversions")]
pub fn imposter_viewport(index: usize, atlas: [u32; 2]) -> Result<[f32; 6], String> {
    let [width, height] = atlas;
    let columns = width / IMPOSTER_CELL[0];
    let rows = height / IMPOSTER_CELL[1];
    if columns < 2 || rows < 2 {
        return Err("crowd imposter atlas below two cells a side (native divide trap)".into());
    }
    let gap_x = width % IMPOSTER_CELL[0] / (columns - 1);
    let gap_y = height % IMPOSTER_CELL[1] / (rows - 1);
    let index = u32::try_from(index).map_err(|_| "crowd target index")?;
    let (row, column) = (index / columns, index % columns);
    let cell = [IMPOSTER_CELL[0] as f32, IMPOSTER_CELL[1] as f32];
    let x = (gap_x as f32 + cell[0]) * column as f32;
    let y = (gap_y as f32 + cell[1]) * row as f32;
    if x + cell[0] > width as f32 || cell[1] + y > height as f32 {
        return Err("crowd imposter cell outside the atlas".into());
    }
    Ok([x, y, cell[0], cell[1], width as f32, height as f32])
}

type Matrix = [[f32; 4]; 4];

fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            let mut sum = a[row][0] * b[0][column];
            for k in 1..4 {
                sum = a[row][k].mul_add(b[k][column], sum);
            }
            sum
        })
    })
}

fn sub3(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2], 0.0]
}

fn dot3(a: [f32; 4], b: [f32; 4]) -> f32 {
    a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2]))
}

fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0], 0.0]
}

/// `vrsqrtefp` and two Newton steps, as the native's `Normalize`.
fn normalize(v: [f32; 4]) -> [f32; 4] {
    let length = dot3(v, v);
    let mut estimate = 1.0 / length.sqrt();
    for _ in 0..2 {
        estimate = (estimate * 0.5).mul_add(1.0 - length * (estimate * estimate), estimate);
    }
    [v[0] * estimate, v[1] * estimate, v[2] * estimate, 0.0]
}

/// `CrowdImposterTarget::MatrixLookAtRH` 0x827d6ac0 (row vectors).
fn look_at_rh(eye: [f32; 4], at: [f32; 4], up: [f32; 4]) -> Matrix {
    let z = normalize(sub3(eye, at));
    let x = normalize(cross(up, z));
    let y = cross(z, x);
    [
        [x[0], y[0], z[0], 0.0],
        [x[1], y[1], z[1], 0.0],
        [x[2], y[2], z[2], 0.0],
        [-dot3(x, eye), -dot3(y, eye), -dot3(z, eye), 1.0],
    ]
}

/// `CrowdImposterTarget::MatrixOrthoOffCenterRH` 0x827cfb50 (row vectors).
fn ortho_off_center_rh(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Matrix {
    [
        [2.0 / (right - left), 0.0, 0.0, 0.0],
        [0.0, 2.0 / (top - bottom), 0.0, 0.0],
        [0.0, 0.0, 1.0 / (near - far), 0.0],
        [(left + right) / (left - right), (bottom + top) / (bottom - top), near / (near - far), 1.0],
    ]
}

/// A target's camera and placement in the atlas (`SetViewProjectionMatrix` 0x827d6c90), as row
/// vectors: its view, its projection mapped into its cell, the quad's size (`mMatModel`) and
/// the quad's texture coordinates into the cell (`mMatTexture`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrowdTargetCamera {
    pub view: Matrix,
    pub projection: Matrix,
    pub model: Matrix,
    pub texture: Matrix,
}

/// The pedestrian's bounding box framed from one unit in front (+Z) at its height's centre,
/// orthographically: its larger horizontal extent plus 0.2 wide and its height plus 0.1 tall,
/// grown on one axis to the cell's aspect; its feet on the cell's bottom edge.
pub fn target_camera(bounds: [[f32; 4]; 2], viewport: [f32; 6], near: f32, far: f32) -> CrowdTargetCamera {
    let [min, max] = bounds;
    let [x, y, w, h, tw, th] = viewport;
    let aspect = w / h;
    let height = max[1] - min[1];
    let mut width = (max[0] - min[0]).max(max[2] - min[2]) + f32::from_bits(0x3e4c_cccd);
    let mut tall = height + f32::from_bits(0x3dcc_cccd);
    let grown = tall * aspect;
    if width > grown {
        tall = width / aspect;
    } else {
        width = grown;
    }
    let centre = (min[1] + max[1]) * 0.5;
    let at = [0.0, centre, 0.0, 0.0];
    let eye = [0.0, centre, 1.0, 0.0];
    let view = look_at_rh(eye, at, [0.0, 1.0, 0.0, 0.0]);
    let left = width * -0.5;
    let bottom = height * -0.5;
    let ortho = ortho_off_center_rh(left, left + width, bottom, bottom + tall, near, far);
    let cell = [
        [w * (1.0 / tw), 0.0, 0.0, 0.0],
        [0.0, h * (1.0 / th), 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [
            (f64::from(x.mul_add(2.0, w) * (1.0 / tw)) - 1.0) as f32,
            -(y.mul_add(2.0, h) * (1.0 / th)) + 1.0,
            0.0,
            1.0,
        ],
    ];
    let projection = multiply(&ortho, &cell);
    let model = [[width, 0.0, 0.0, 0.0], [0.0, tall, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
    let texture = [
        [(1.0 / tw) * w, 0.0, 0.0, 0.0],
        [0.0, h * (1.0 / th), 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [(1.0 / tw) * x, (1.0 / th) * y, 0.0, 1.0],
    ];
    CrowdTargetCamera { view, projection, model, texture }
}

/// `GetPresEntityWorldAABB` without a pedestrian.
pub const DEFAULT_TARGET_BOUNDS: [[f32; 4]; 2] = [[-0.5, 0.0, -0.5, 0.0], [0.5, 2.0, -0.5, 0.0]];

/// `CrowdImposter::UpdateTransform` 0x827d1f20 (row vectors): the quad faces along the camera's
/// Z axis flattened onto the ground, upright, at the position.
pub fn imposter_transform(camera_z: [f32; 3], position: [f32; 3]) -> Matrix {
    let forward = normalize([camera_z[0], 0.0, camera_z[2], 0.0]);
    let right = normalize(cross([0.0, 1.0, 0.0, 0.0], forward));
    let up = cross(forward, right);
    [right, up, forward, [position[0], position[1], position[2], 1.0]]
}

/// One imposter target for the renderer: its pedestrian (none once its recipe went) and camera.
#[derive(Clone)]
pub struct SelectedCrowdTarget {
    pub pedestrian: Option<crate::SelectedPedestrianVisual>,
    pub camera: CrowdTargetCamera,
    /// `CrowdImposterTarget::Render`'s scissor: the cell less its first row and column.
    pub scissor: [u32; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct SelectedCrowdImposter {
    pub target: usize,
    pub position: [f32; 3],
    /// `i_colorize_red`, `i_colorize_blue`.
    pub colours: [[f32; 4]; 2],
}

#[derive(Clone, Default)]
pub struct SelectedCrowdVisual {
    pub targets: Vec<SelectedCrowdTarget>,
    pub imposters: Vec<SelectedCrowdImposter>,
}

impl SelectedRecipeFrame<'_> {
    /// `CrowdPresentation::Update`: the presentation flushed, then, while `kCrowdPresEnable`
    /// holds, an imposter for each of the buffer's first 200 positions (`AddImposter`).
    pub(crate) fn update_crowd(&mut self, buffer: &CrowdPositionBuffer, cvars: &InputCvarReferences) -> Result<(), String> {
        self.runtime.crowd.flush();
        if !cvars.try_crowd_presentation_enabled().map_err(|e| format!("kCrowdPresEnable: {e:?}"))? {
            return Ok(());
        }
        let attributes = RecipeAttributes::new(self.vault, cvars);
        let recipes = &self.runtime.recipes;
        let mut config = RecipeConfig::new(
            self.vault,
            |model| recipes.model_asset_id(model, &attributes).map_err(|e| format!("crowd GenerateModelAssetID: {e:?}")),
            self.runtime.presentation.ids(),
            buffer.population_breakdown(),
        )?;
        for &position in buffer.positions().iter().take(MAX_IMPOSTERS) {
            self.add_imposter(position, &mut config, cvars)?;
        }
        Ok(())
    }

    /// `AddImposter` 0x827e7b28.
    fn add_imposter(&mut self, position: [f32; 4], config: &mut RecipeConfig, cvars: &InputCvarReferences) -> Result<(), String> {
        if self.runtime.crowd.imposters.len() >= MAX_IMPOSTERS {
            return Ok(());
        }
        let Some(target) = self.crowd_target(config, cvars)? else { return Ok(()) };
        let colours = imposter_colours(self.vault, self.runtime.crowd.targets[target].model_key)?;
        self.runtime.crowd.imposters.push(CrowdImposter { target, position, colours });
        Ok(())
    }

    /// `CreateCrowdImposterEntity` 0x827e7540: once six targets exist, or no crowd model comes,
    /// an existing target at random; a key made already gives its target; otherwise a new target
    /// of a new no-sim pedestrian, its clip started at a random time.
    fn crowd_target(&mut self, config: &mut RecipeConfig, cvars: &InputCvarReferences) -> Result<Option<usize>, String> {
        let existing = |crowd: &CrowdPresentation| -> Result<Option<usize>, String> {
            Ok(if crowd.targets.is_empty() { None } else { Some(random()? as usize % crowd.targets.len()) })
        };
        if self.runtime.crowd.targets.len() >= MAX_TARGETS {
            return existing(&self.runtime.crowd);
        }
        let Some((asset, model_key, clip)) = self.random_crowd_model(config, cvars)? else {
            return existing(&self.runtime.crowd);
        };
        if let Some(&target) = config.existing.get(&(asset, model_key)) {
            return Ok(Some(target));
        }
        let Some(entity) = self.add_no_sim_pedestrian(clip, model_key, asset)? else { return Ok(None) };
        let crowd = &mut self.runtime.crowd;
        let target = crowd.targets.len();
        crowd.targets.push(CrowdTarget { entity, model_key });
        config.existing.insert((asset, model_key), target);
        let pedestrian = crowd.pedestrians.get_mut(&entity).ok_or("crowd no-sim pedestrian absent")?;
        let length = pedestrian.pool.length(&pedestrian.root).map_err(|e| format!("crowd clip length: {e}"))?;
        if length > 0.0 {
            #[expect(clippy::cast_precision_loss, reason = "the native's u64 to double to float")]
            let fraction = f64::from(random()?) as f32 * f32::from_bits(0x2f80_0000);
            let scaled = fraction * length;
            let time = if scaled - length >= 0.0 { length } else { scaled };
            let time = if -time >= 0.0 { 0.0 } else { time };
            pedestrian.pool.set_time(&pedestrian.root, time).map_err(|e| format!("crowd clip SetTime: {e}"))?;
        }
        Ok(Some(target))
    }

    /// `GetRandomCrowdModel` 0x827e5488: with registered crowd recipes, a draw names the clip;
    /// the buffer's category group gives a model whose asset is one of them, else the fallback.
    fn random_crowd_model(&mut self, config: &mut RecipeConfig, cvars: &InputCvarReferences) -> Result<Option<(u64, u64, &'static [u8])>, String> {
        if config.recipe_ids.is_empty() {
            return Ok(None);
        }
        let draw = random()?;
        let clip = CLIP_NAMES[clip_index(draw)];
        if config.category_group != 0 {
            let model = resolve_category_group_to_model(self.vault, draw, config.category_group)?;
            if !self.vault.instance(MODELS_CLASS, model).is_null() {
                let attributes = RecipeAttributes::new(self.vault, cvars);
                let asset = self.runtime.recipes.model_asset_id(model, &attributes).map_err(|e| format!("crowd GenerateModelAssetID: {e:?}"))?;
                if config.recipe_ids.contains(&asset) {
                    return Ok(Some((asset, model, clip)));
                }
            }
        }
        Ok(config.fallback_model(draw).map(|(asset, model)| (asset, model, clip)))
    }

    /// `AddNoSimPresPedestrian` 0x82812c10 / `CreateNoSimPresPedestrian` 0x82812988: under the
    /// cap, a registered recipe's pedestrian with its model's first red and second blue colours,
    /// the regular inverse bind pose registered on the PRES ACS before its first use.
    fn add_no_sim_pedestrian(&mut self, clip: &[u8], model_key: u64, asset: u64) -> Result<Option<u32>, String> {
        if self.runtime.crowd.pedestrians.len() >= MAX_NO_SIM_PEDESTRIANS {
            return Ok(None);
        }
        let model = self.vault.instance(MODELS_CLASS, model_key);
        let colours = [colour(&model, RED_COLOURS, 0)?, colour(&model, BLUE_COLOURS, 1)?];
        let Some(assets) = self.runtime.presentation.get(asset) else { return Ok(None) };
        let scene = Arc::clone(assets.finalized.scene().map_err(|e| e.to_string())?);
        let startup = self.pedestrian_animation.startup;
        let playback = startup.presentation.as_ref().ok_or("crowd PRES playback absent")?;
        let commands = Arc::clone(&playback.lock().map_err(|_| "crowd PRES playback lock")?.commands);
        let database = Arc::clone(&commands.lock().map_err(|_| "crowd PRES commands lock")?.database);
        let (hierarchy, bones, record) = {
            let retained = database.lock().map_err(|_| "crowd PRES database lock")?;
            let parsed = retained.parsed.as_ref().ok_or("crowd PRES database absent")?;
            let record = retained
                .clip_record(crate::actor_graph_construction::ActorGraphIntents::native_name(clip))
                .ok_or_else(|| format!("crowd clip {} absent", String::from_utf8_lossy(clip)))?;
            (CharacterHierarchy { bone_names: parsed.hierarchy.bone_names.clone() }, parsed.hierarchy.parents.len(), record)
        };
        let hide_skateboard = model
            .try_get_data(HIDE_SKATEBOARD, 0)
            .map_err(attribute)?
            .is_some_and(|data| data.bytes().first().is_some_and(|byte| *byte != 0));
        let palette = CharacterPalette::pedestrian(scene, &hierarchy, 0, hide_skateboard).map_err(|e| e.to_string())?;
        let inverse_bind = {
            let mut registered = startup.regular_inverse_bind.lock().map_err(|_| "regular inverse bind lock")?;
            if registered.is_none() {
                let matrices = Arc::clone(&palette.skinning().hierarchy_inverse_bind_pose);
                commands
                    .lock()
                    .map_err(|_| "regular ACS lock")?
                    .register_fixed_matrix(&matrices)
                    .map_err(|e| format!("regular inverse-bind registration: {e:?}"))?;
                *registered = Some(matrices);
            }
            Arc::clone(registered.as_ref().expect("registered"))
        };
        let mut pool = AnimationTreePool::new(0x200, 0x200);
        let clip_node = pool.push_clip().map_err(|e| format!("crowd Push<Clip>: {e}"))?;
        pool.initialize_clip(&clip_node, AnimationClipRef { database: Arc::clone(&database), record_index: record }, 1.0, 0.0)
            .map_err(|e| format!("crowd Clip::Init: {e}"))?;
        let bind = pool.push_fixed_sqt(BIND_POSE_SQT).map_err(|e| format!("crowd Push<SetSQTById>: {e}"))?;
        let root = pool.push_add_sqt(clip_node.clone(), bind, clip_node).map_err(|e| format!("crowd Push<AddSQT>: {e}"))?;
        let pose = OwnedAnimationPoseBuffer::allocate_unknown(Arc::clone(&database), bones)
            .map_err(|e| format!("crowd pose allocation: {e:?}"))?;
        let crowd = &mut self.runtime.crowd;
        crowd.next_id = crowd.next_id.wrapping_add(1);
        let id = crowd.next_id & 0x00ff_ffff | 0x0100_0000;
        if crowd.pedestrians.contains_key(&id) {
            return Err("native AddNoSimPresPedestrian duplicate entity assertion".into());
        }
        crowd.pedestrians.insert(id, NoSimPedestrian {
            asset,
            model_key,
            colours,
            pool,
            root,
            properties: AnimationProperties::default(),
            pose,
            palette: Arc::new(Mutex::new(palette)),
            hierarchy: Arc::new(hierarchy),
            inverse_bind,
            bounds: None,
        });
        Ok(Some(id))
    }

    /// `cLivingWorldPresEntityManager::UpdateNoSimPedestrians` 0x828098a8 (each clip advanced
    /// unless `PauseMan` flag 4 is set), then `SyncAnimationPoses`' `EvalNoSimPedestrians`
    /// 0x82809988 in its own PRES command set: each pose evaluated, synchronized and published.
    /// A pedestrian whose recipe was deregistered goes first (`NotifyOfAssetRemoval`).
    pub(crate) fn present_crowd(&mut self, dt: f32, cvars: &InputCvarReferences) -> Result<(), String> {
        let presentation = &self.runtime.presentation;
        let crowd = &mut self.runtime.crowd;
        crowd.pedestrians.retain(|_, pedestrian| presentation.get(pedestrian.asset).is_some());
        if crowd.pedestrians.is_empty() {
            return Ok(());
        }
        let paused = match &crowd.pause {
            Some(pause) => pause.try_is_paused(PAUSE_FLAG).map_err(|e| e.to_string())?,
            None => false,
        };
        if !paused {
            for pedestrian in crowd.pedestrians.values_mut() {
                pedestrian
                    .pool
                    .add_time(&pedestrian.root.clone(), dt, &mut pedestrian.properties)
                    .map_err(|e| format!("crowd AddTime: {e}"))?;
            }
        }
        let playback = self.pedestrian_animation.startup.presentation.as_ref().ok_or("crowd PRES playback absent")?;
        let commands = Arc::clone(&playback.lock().map_err(|_| "crowd PRES playback lock")?.commands);
        let mut source =
            crate::actor_construction::body_session::ActorBodyAnimationServices::try_from_startup(self.pedestrian_animation.shared, cvars)
                .map_err(|e| e.to_string())?;
        use crate::actor_animation_construction::hands_face::runtime;
        runtime::begin_command_set(&commands).map_err(|e| format!("crowd PRES Begin: {e:?}"))?;
        for pedestrian in crowd.pedestrians.values_mut() {
            queue_no_sim(pedestrian, &commands)?;
        }
        runtime::end_command_set(&commands, &mut source).map_err(|e| format!("crowd PRES End: {e:?}"))?;
        runtime::sync_command_set(&commands, &mut source).map_err(|e| format!("crowd PRES Sync: {e:?}"))?;
        let identity: AnimationMatrix = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];
        for pedestrian in crowd.pedestrians.values_mut() {
            let pose = pedestrian.pose.matrices().map_err(|e| format!("crowd pose: {e:?}"))?.ok_or("crowd pose extraction not synchronized")?;
            pedestrian.bounds = Some(pose_bounds(&pose, &pedestrian.inverse_bind, &pedestrian.hierarchy_parents(&commands)?)?);
            let mut palette = pedestrian.palette.lock().map_err(|_| "crowd palette lock")?;
            palette.update_bone_transforms(&pose).map_err(|e| e.to_string())?;
            palette.pack_for_upload(identity).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

impl NoSimPedestrian {
    fn hierarchy_parents(&self, commands: &Arc<Mutex<FaceCommands>>) -> Result<Vec<i32>, String> {
        let database = Arc::clone(&commands.lock().map_err(|_| "crowd PRES commands lock")?.database);
        let retained = database.lock().map_err(|_| "crowd PRES database lock")?;
        Ok(retained.parsed.as_ref().ok_or("crowd PRES database absent")?.hierarchy.parents.clone())
    }
}

/// `cNoSimPresPedestrian::EvalAnimation` 0x82806598: the root's Eval on the PRES ACS, then
/// `PoseWithMatMult` by the registered inverse bind pose and `ExtractMatrix` into its pose.
fn queue_no_sim(pedestrian: &mut NoSimPedestrian, commands: &Arc<Mutex<FaceCommands>>) -> Result<(), String> {
    let (owner, hierarchy) = {
        let retained = commands.lock().map_err(|_| "crowd PRES commands lock")?;
        let owner = retained.buffered.clone().ok_or("crowd PRES buffered ACS absent")?;
        let hierarchy = retained.database.lock().map_err(|_| "crowd PRES hierarchy lock")?
            .hierarchy_snapshot().ok_or("crowd PRES hierarchy absent")?;
        (owner, hierarchy)
    };
    {
        let mut buffered = owner.lock().map_err(|_| "crowd PRES buffered lock")?;
        if buffered.in_command_set != 1 || buffered.in_commands != 0 {
            return Err("crowd PRES command state".into());
        }
        buffered.in_commands += 1;
        buffered.nblend.begin_commands();
    }
    let params = AnimationEvalParams {
        commands: Arc::clone(commands),
        update_old_time: true,
        blend_frames: ALL_FETCHES,
        blend_cull_threshold: 0.0,
        blend_space_cvars: None,
    };
    let mut tree = crate::actor_animation_construction::initialization::body_dispatch::tree_commands::TreeCommands::new_stream(Arc::clone(&owner))
        .map_err(|e| format!("crowd stream allocation: {e:?}"))?;
    tree.fetch_code = crate::animation_job_owners::AnimationJobCode::FetchVbr;
    tree.enable_reuse();
    let root = pedestrian.root.clone();
    pedestrian.pool.eval(&root, &params, true, &mut tree).map_err(|e| format!("crowd Eval: {e}"))?;
    tree.output().map_err(|e| format!("crowd pose stack: {e:?}"))?;
    let output = MatrixProgram::new_fixed(Arc::clone(&pedestrian.inverse_bind), &pedestrian.pose)
        .map_err(|e| format!("crowd matrix destination: {e:?}"))?;
    owner
        .lock()
        .map_err(|_| "crowd PRES buffered lock")?
        .retain_sqt_program(QueuedSqtProgram { commands: tree, output: ProgramOutput::Matrix(output), hierarchy })
        .map_err(|e| format!("crowd EndCommands: {e:?}"))
}

/// `GetPresEntityWorldAABB`: the bounds of the bones below the root's children, each bind pose
/// origin (the inverse of its inverse bind pose) carried by its skin matrix.
fn pose_bounds(pose: &[AnimationMatrix], inverse_bind: &[AnimationMatrix], parents: &[i32]) -> Result<[[f32; 4]; 2], String> {
    let mut bounds: Option<[[f32; 4]; 2]> = None;
    for ((skin, inverse), &parent) in pose.iter().zip(inverse_bind).zip(parents) {
        if parent <= 0 {
            continue;
        }
        let bind = try_inverse(*inverse, &mut HostReckoningNumeric).map_err(|e| format!("crowd bind inverse: {e:?}"))?;
        let origin = [bind[12], bind[13], bind[14]];
        let point: [f32; 4] = std::array::from_fn(|lane| {
            if lane == 3 {
                return 0.0;
            }
            let x = skin[lane].mul_add(origin[0], skin[12 + lane]);
            let y = skin[4 + lane].mul_add(origin[1], x);
            skin[8 + lane].mul_add(origin[2], y)
        });
        bounds = Some(match bounds {
            None => [point, point],
            Some([min, max]) => [
                std::array::from_fn(|lane| min[lane].min(point[lane])),
                std::array::from_fn(|lane| max[lane].max(point[lane])),
            ],
        });
    }
    bounds.ok_or_else(|| "native GetPresEntityWorldAABB empty-bounds assertion".into())
}

impl super::SelectedLivingWorldRecipes {
    /// The crowd for the renderer: each target's pedestrian, camera and scissor, and the
    /// imposters at their positions.
    pub(crate) fn crowd_visual(
        &self,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
        actor: &crate::actor_construction::ActorConstruction,
    ) -> Result<SelectedCrowdVisual, String> {
        let crowd = &self.crowd;
        if crowd.targets.is_empty() {
            return Ok(SelectedCrowdVisual::default());
        }
        let near = cvars.try_crowd_imposter_near().map_err(|e| format!("kCrowdImposterPN: {e:?}"))?;
        let far = cvars.try_crowd_imposter_far().map_err(|e| format!("kCrowdImposterPF: {e:?}"))?;
        let mut targets = Vec::with_capacity(crowd.targets.len());
        for (index, target) in crowd.targets.iter().enumerate() {
            let viewport = imposter_viewport(index, IMPOSTER_ATLAS)?;
            let pedestrian = crowd.pedestrians.get(&target.entity);
            let bounds = pedestrian.and_then(|pedestrian| pedestrian.bounds).unwrap_or(DEFAULT_TARGET_BOUNDS);
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "the native's fctidz of cell edges")]
            let scissor = {
                let left = (viewport[0] + 1.0) as i64;
                let top = (viewport[1] + 1.0) as i64;
                let right = (viewport[2] - 1.0) as i64 + left;
                let bottom = (viewport[3] - 1.0) as i64 + top;
                [left as u32, top as u32, right as u32, bottom as u32]
            };
            let visual = match pedestrian {
                Some(pedestrian) => Some(crate::selected_living_world_recipes::pedestrian_animation::presentation::no_sim_visual(
                    target.entity,
                    pedestrian.asset,
                    pedestrian.model_key,
                    &pedestrian.palette,
                    pedestrian.colours,
                    &pedestrian.hierarchy,
                    &pedestrian.pose,
                    vault,
                    cvars,
                    actor,
                )?),
                None => None,
            };
            targets.push(SelectedCrowdTarget { pedestrian: visual, camera: target_camera(bounds, viewport, near, far), scissor });
        }
        let imposters = crowd
            .imposters
            .iter()
            .map(|imposter| SelectedCrowdImposter {
                target: imposter.target,
                position: [imposter.position[0], imposter.position[1], imposter.position[2]],
                colours: imposter.colours,
            })
            .collect();
        Ok(SelectedCrowdVisual { targets, imposters })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clip_follows_the_pointer_table_offset() {
        assert_eq!(clip_index(0), 0);
        assert_eq!(clip_index(3), 0);
        assert_eq!(clip_index(4), 1);
        assert_eq!(clip_index(23), 5);
        assert_eq!(clip_index(24), 0);
        assert_eq!(clip_index(u32::MAX), (u32::MAX % 24 / 4) as usize);
    }

    #[test]
    fn the_fallback_removes_each_pick_from_the_remaining_recipes() {
        let mut config = RecipeConfig {
            recipe_ids: vec![10, 20, 30],
            selections: 3,
            asset_models: BTreeMap::from([(10, vec![1, 2]), (20, vec![]), (30, vec![3])]),
            category_group: 0,
            existing: BTreeMap::new(),
        };
        assert_eq!(config.fallback_model(4), Some((20, DEFAULT_MODEL)));
        assert_eq!(config.recipe_ids, [10, 30, 20]);
        assert_eq!(config.fallback_model(3), Some((30, 3)));
        assert_eq!(config.fallback_model(7), Some((10, 2)));
        assert_eq!(config.fallback_model(0), None);
    }

    #[test]
    fn the_720p_atlas_holds_six_cells() {
        let cells: Vec<[f32; 2]> = (0..MAX_TARGETS).map(|index| {
            let viewport = imposter_viewport(index, IMPOSTER_ATLAS).unwrap();
            [viewport[0], viewport[1]]
        }).collect();
        assert_eq!(cells, [[0.0, 0.0], [458.0, 0.0], [916.0, 0.0], [0.0, 410.0], [458.0, 410.0], [916.0, 410.0]]);
        assert!(imposter_viewport(6, IMPOSTER_ATLAS).is_err());
    }

    fn transform(point: [f32; 4], matrix: &Matrix) -> [f32; 4] {
        std::array::from_fn(|column| (0..4).map(|row| point[row] * matrix[row][column]).sum())
    }

    #[test]
    fn the_target_camera_maps_the_feet_to_its_cell_bottom() {
        let viewport = imposter_viewport(4, IMPOSTER_ATLAS).unwrap();
        let bounds = [[-0.3, 0.0, -0.2, 0.0], [0.3, 1.8, 0.2, 0.0]];
        let camera = target_camera(bounds, viewport, -10.0, 10.0);
        let clip = transform(transform([0.0, 0.0, 0.0, 1.0], &camera.view), &camera.projection);
        let pixel_x = (clip[0] / clip[3] + 1.0) * 0.5 * 1280.0;
        let pixel_y = (1.0 - clip[1] / clip[3]) * 0.5 * 720.0;
        assert!((pixel_x - (458.0 + 182.0)).abs() < 0.01, "{pixel_x}");
        assert!((pixel_y - (410.0 + 310.0)).abs() < 0.01, "{pixel_y}");
        let texture = transform([0.5, 1.0, 0.0, 1.0], &camera.texture);
        assert!((texture[0] * 1280.0 - 640.0).abs() < 0.01 && (texture[1] * 720.0 - 720.0).abs() < 0.01);
        let width = camera.model[0][0];
        let tall = camera.model[1][1];
        assert!((width / tall - 364.0 / 310.0).abs() < 1e-5);
        assert!((tall - 1.9).abs() < 1e-5, "the height pads by 0.1 and decides here");
    }

    #[test]
    fn the_imposter_faces_the_flattened_camera_axis() {
        let matrix = imposter_transform([0.6, 0.8, 0.0], [5.0, 1.0, -2.0]);
        assert_eq!(matrix[2], [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(matrix[1], [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(matrix[0], [0.0, 0.0, -1.0, 0.0]);
        assert_eq!(matrix[3], [5.0, 1.0, -2.0, 1.0]);
    }
}
