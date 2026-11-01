//! NIS scene characters: `Sk8::Scene::SceneCharacter` (constructor
//! `0x82875100`) and its `SceneCharacterAnim` (`Init` `0x82873088`,
//! `EvalCurrAnim` `0x82866cd0`). A scene character's fetch system has no
//! trajectory node (`NewACS` with `hasTrajectoryNode` false, `0x828732f4`), so
//! `BatchFetch` (`0x82d737d0`) extracts the trajectory part at the clip's time
//! like any other part.

use crate::{
    actor_animation_construction::initialization::body_dispatch::tree_commands::{numeric::add_sqt, sample},
    actor_face_playback::{
        ActorFaceError,
        presentation::{
            corrections::CorrectionChannels,
            move_root::{multiply, rigid_matrix},
            twist::try_scene_twist_channels,
        },
        tree_database::clip_fetch::timing::{BufferedFetchTiming, frame_pair},
    },
    animation::{AnimationMatrix, AnimationSkeleton, LocalPose, apply_inverse_bind, multiply_matrices},
    attribute_runtime::AttributeRuntimeVault,
    character::palette::CharacterPalette,
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    ui::edit_skater_presentation::named_pose,
};
use skate2_formats::{
    andale::{AndaleDatabase, AndaleSqt},
    ice_camera::compute_transform,
    mount::MountSet,
};
use super::directory::SceneCharacterInfo;
use std::sync::Arc;

const IDENTITY_SQT: AndaleSqt = AndaleSqt { scale: [1.0; 4], rotation: [0.0, 0.0, 0.0, 1.0], translation: [0.0, 0.0, 0.0, 1.0] };

/// `GlobalAttributeCollection::animation_twistbones`, which
/// `ComputeExpressions` (`0x82866760`) reads with the skater's and the
/// create-a-skater twists' keys: Arm to ArmTwist, ForeArm to ForeArmTwist,
/// ForeArm to ForeArmTwist1, UpLeg to UpLegTwist.
pub type TwistRatios = [f32; 4];

const TWIST_CLASS: u64 = 0xbabb_3ec1_1084_9e08;
const TWIST_COLLECTION: u64 = 0x2db7_7096_45a1_9ce8;
const TWIST_KEYS: [u64; 4] = [0x3831_7705_e5a4_571a, 0x246f_c198_b6f1_624d, 0x5cdb_40ca_8c5a_5044, 0x3eb7_0888_6e6e_4607];
/// The bones `Init` looks up for the twists, in `ComputeExpressions`' order
/// (each side: source, destination and ratio index).
const TWIST_BONES: [(&str, &str, usize); 8] = [
    ("LeftArm", "LeftArmTwist", 0),
    ("LeftForeArm", "LeftForeArmTwist", 1),
    ("LeftForeArm", "LeftForeArmTwist1", 2),
    ("LeftUpLeg", "LeftUpLegTwist", 3),
    ("RightArm", "RightArmTwist", 0),
    ("RightForeArm", "RightForeArmTwist", 1),
    ("RightForeArm", "RightForeArmTwist1", 2),
    ("RightUpLeg", "RightUpLegTwist", 3),
];

/// The files the scene loaders read and what `InstanceCharacter` asks of
/// the local player.
pub trait SceneCharacterSource: Send + Sync {
    /// A file under `data\`, `None` when it cannot be read.
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    fn twist_ratios(&self) -> TwistRatios;
    /// The local player's render recipe: its schema name and, for a
    /// create-a-skater recipe, whether it is male.
    fn local_player(&self) -> Option<LocalPlayerRecipe>;
    /// `Simulation::GetLocalPlayerID`: the player whose entity a scene can borrow.
    fn local_player_id(&self) -> Option<i32> {
        None
    }
    /// `SkaterManager::GetLocalIndex`: the player the camera follows, whose entity
    /// "current_player" borrows (`GetSkaterPresEntityFromIndex(GetLocalIndex())` 0x82876ee0).
    fn local_index(&self) -> Option<i32> {
        self.local_player_id()
    }
    /// GetSkaterPresEntityFromPlayerId82876aac: any existing skater can be lent.
    fn player(&self, id: i32) -> Option<LocalPlayerRecipe> {
        (self.local_player_id() == Some(id)).then(|| self.local_player()).flatten()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalPlayerRecipe {
    pub schema: String,
    /// `Some(male)` for `Category_CreateACharacter`.
    pub create_a_character: Option<bool>,
}

/// The twist ratios from `skater.vlt`; a missing value reads 0
/// (`Attrib::DefaultDataArea`).
///
/// # Errors
/// A vault the attribute runtime cannot load.
pub fn twist_ratios(mounts: &MountSet) -> Result<TwistRatios, String> {
    let vault = AttributeRuntimeVault::try_load_mounted(mounts, "data/db/skater.vlt", &[TWIST_CLASS]).map_err(|e| e.to_string())?;
    let instance = vault.instance(TWIST_CLASS, TWIST_COLLECTION);
    let mut ratios = [0.0; 4];
    for (ratio, key) in ratios.iter_mut().zip(TWIST_KEYS) {
        *ratio = f32::from_bits(instance.try_scalar_word(key, 0).map_err(|e| e.to_string())?.unwrap_or(0));
    }
    Ok(ratios)
}

/// A `SceneCharacterAnimLoader`'s buffer (`Load` `0x8286f5e8`: `data\` and the
/// record's anim DB name), shared by every character naming it
/// (`SceneAssetLoaderManager::GetAnimLoaderData` `0x8286f990`).
pub struct SceneAnimData {
    pub database: AndaleDatabase,
    bytes: Vec<u8>,
}

impl SceneAnimData {
    /// # Errors
    /// An `.abin` the Andale parser rejects.
    pub fn parse(bytes: Vec<u8>) -> Result<Self, String> {
        let database = AndaleDatabase::parse(&bytes).map_err(|e| e.to_string())?;
        Ok(Self { database, bytes })
    }
}

/// `SceneCharacterAnim`: the clip `Animatable::PlayAnim` builds for
/// `mAnimRealPlayName` (speed 1) and the pose `EvalCurrAnim` leaves.
pub struct SceneCharacterAnim {
    data: Arc<SceneAnimData>,
    clip: usize,
    /// `mBPSQTBuffer`.
    bind: Vec<AndaleSqt>,
    /// `mAnimOffset`.
    offset: f32,
    /// `Andale::Clip` `mTime`, `mOldTime` and `mNumLoops`.
    time: f32,
    old_time: f32,
    loops: u32,
    /// `mTrajBoneIndex`, `mHipsBoneIndex`, `mSkateboardBoneIndex`.
    trajectory: usize,
    hips: usize,
    skateboard: Option<usize>,
    /// The twist bones with their ratios, on a rig with a skateboard root.
    twists: Vec<(usize, usize, f32)>,
    /// `FetchSys::mHasTrajectoryBone`, from `NewACS`'s `hasTrajectoryNode`.
    trajectory_node: bool,
    /// `mTwistSQTBuffer` as `EvalCurrAnim` leaves it before the skinning.
    pose: Vec<AndaleSqt>,
    /// The hips' local transform `EvalCurrAnim` takes out of the pose
    /// (`0x82866df0`), and the translation it was built from.
    hips_matrix: AnimationMatrix,
    hips_translation: [f32; 4],
}

impl SceneCharacterAnim {
    /// The constructor (`0x82874828`) and `Init` for a clip of `data`: the
    /// bind pose is the pose named like the hierarchy record (`0x82873844`);
    /// `Trajectory` (else `Vehicle_Root`), `Hips` (else `Chassis`) and
    /// `Skateboard_Root` are looked up by name.
    ///
    /// # Errors
    /// A name that is not a clip of the database, or a database without the
    /// bones `Init` asserts on.
    pub fn new(data: Arc<SceneAnimData>, anim_name: &str, offset: f32, ratios: TwistRatios) -> Result<Self, String> {
        Self::build(data, anim_name, offset, Some(ratios), false)
    }

    /// `Character::ViewerCharacterAnimator::Init` (`0x8259adc8`) for
    /// `ViewerAnimationLibrary`'s database: the same bind pose and bones, and
    /// `PlayAnim`'s tree the same clip over `mBPSQTBuffer`
    /// (`CASCharacterAnimator::GetAnimTree` `0x8258e618`). Its `NewACS` has a
    /// trajectory node (`0x8259b024`), and its `EvalCurrAnim` (`0x8258b848`)
    /// runs no `ComputeExpressions`.
    ///
    /// # Errors
    /// As `new`.
    pub fn viewer(data: Arc<SceneAnimData>, anim_name: &str) -> Result<Self, String> {
        Self::build(data, anim_name, 0.0, None, true)
    }

    fn build(
        data: Arc<SceneAnimData>,
        anim_name: &str,
        offset: f32,
        ratios: Option<TwistRatios>,
        trajectory_node: bool,
    ) -> Result<Self, String> {
        let database = &data.database;
        let clip = database
            .clips
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(anim_name))
            .ok_or_else(|| format!("scene animation {anim_name} is not a clip"))?;
        let hierarchy_name = database
            .records
            .iter()
            .find(|r| r.header.data_type == 4)
            .ok_or("scene animation hierarchy header missing")?
            .header
            .name
            .decode()
            .map_err(|e| e.to_string())?;
        let bind = named_pose(database, &hierarchy_name)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("scene animation bind pose {hierarchy_name} missing"))?;
        let bone = |name: &str| database.hierarchy.bone_names.iter().position(|b| b.eq_ignore_ascii_case(name));
        let trajectory = bone("Trajectory").or_else(|| bone("Vehicle_Root")).ok_or("scene animation has no trajectory bone")?;
        let hips = bone("Hips").or_else(|| bone("Chassis")).ok_or("scene animation has no hips bone")?;
        let skateboard = bone("Skateboard_Root");
        let mut twists = Vec::new();
        if let Some(ratios) = ratios.filter(|_| skateboard.is_some()) {
            for (source, destination, ratio) in TWIST_BONES {
                let index = |name| bone(name).ok_or_else(|| format!("scene animation has no {name}"));
                twists.push((index(source)?, index(destination)?, ratios[ratio]));
            }
        }
        let bones = bind.len();
        Ok(Self {
            data,
            clip,
            bind,
            offset,
            time: 0.0,
            old_time: 0.0,
            loops: 0,
            trajectory,
            hips,
            skateboard,
            twists,
            trajectory_node,
            pose: vec![IDENTITY_SQT; bones],
            hips_matrix: rigid_matrix([0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, 1.0]),
            hips_translation: [0.0, 0.0, 0.0, 1.0],
        })
    }

    /// `InitFace` (`0x82873918`): the `SkaterFace` pose named after the
    /// schema; else for a create-a-skater recipe `CAC_male` or `CAC_female`
    /// (a female's clip name takes "_Female" for its first "_Player", else
    /// "_player"); else `rig_Tpose`. It overrides the bind pose part by
    /// part (`PoseBufferExt::OverrideWith` `0x825920b0`).
    ///
    /// # Errors
    /// No face pose at all, or a renamed clip the database lacks.
    pub fn init_face(&mut self, face: &AndaleDatabase, schema: &str, create_a_character: Option<bool>) -> Result<(), String> {
        let pose = |name: &str| named_pose(face, name).map_err(|e| e.to_string());
        let mut found = pose(schema)?;
        if found.is_none()
            && let Some(male) = create_a_character
        {
            if !male {
                let database = &self.data.database;
                let mut name = database.clips[self.clip].name.clone();
                if let Some(at) = name.find("_Player").or_else(|| name.find("_player")) {
                    name.replace_range(at..at + 7, "_Female");
                    self.clip = database
                        .clips
                        .iter()
                        .position(|c| c.name.eq_ignore_ascii_case(&name))
                        .ok_or_else(|| format!("scene animation {name} is not a clip"))?;
                }
            }
            found = pose(if male { "CAC_male" } else { "CAC_female" })?;
        }
        if found.is_none() {
            found = pose("rig_Tpose")?;
        }
        let found = found.ok_or("face pose rig_Tpose missing")?;
        let mut bind = LocalPose::from_andale(&self.bind);
        bind.overlay_matching_parts(
            &AnimationSkeleton::from_andale(&self.data.database.hierarchy).map_err(|e| e.to_string())?,
            &AnimationSkeleton::from_andale(&face.hierarchy).map_err(|e| e.to_string())?,
            &LocalPose::from_andale(&found),
        )
        .map_err(|e| e.to_string())?;
        self.bind = bind.bones.into_iter().map(|v| AndaleSqt { scale: v.scale, rotation: v.rotation, translation: v.translation }).collect();
        Ok(())
    }

    /// `Clip::GetLength`: `FrameToTime(mNbFrames - 1)` (`0x82805ed0`,
    /// `frame / (mBaseSpeed * mFps * mSpeed)`).
    pub fn length(&self) -> f32 {
        let clip = &self.data.database.clips[self.clip];
        (clip.frame_count - 1.0) / (clip.base_speed * clip.frames_per_second)
    }

    /// `SceneCharacter::SetTime` (`0x82862080`) through `SetAnimToTime`
    /// (`0x82873c80`): `PlayAnim` builds a new clip at time zero
    /// (`Clip::Init` `0x82807260`) and sets `mAnimOffset + t` only when that
    /// is positive; then `EvalCurrAnim`.
    ///
    /// # Errors
    /// A fetch the clip data cannot answer.
    pub fn set_time(&mut self, t: f32) -> Result<(), String> {
        let time = self.offset + t;
        (self.time, self.old_time, self.loops) = (0.0, 0.0, 0);
        if time > 0.0 {
            (self.time, self.old_time) = (time, time);
        }
        self.eval()
    }

    /// `SceneCharacter::AddTime` (`0x828620d0`): `Clip::AddTime`
    /// (`0x82d6d238`: a looping clip wraps by its length, counting loops; any
    /// other keeps its time), then `EvalCurrAnim`.
    ///
    /// # Errors
    /// A fetch the clip data cannot answer.
    pub fn add_time(&mut self, dt: f32) -> Result<(), String> {
        self.time += dt;
        let length = self.length();
        if self.time > length && self.data.database.clips[self.clip].looping() {
            loop {
                self.time -= length;
                self.loops += 1;
                if self.time <= length {
                    break;
                }
            }
        }
        self.eval()
    }

    /// `EvalCurrAnim` before the skinning: the clip over the bind pose
    /// (`AddSQT`, clip first: `ACS::AddSQT` `0x82a2d5a0`), the hips'
    /// transform taken out of the pose (`0x82866df0`), then on a rig with a
    /// skateboard root the twists against the bind pose
    /// (`ComputeExpressions`).
    fn eval(&mut self) -> Result<(), String> {
        let delta = self.fetch()?;
        let mut pose: Vec<_> = delta.iter().zip(&self.bind).map(|(a, b)| add_sqt(*a, *b, true)).collect();
        let hips = &mut pose[self.hips];
        self.hips_matrix = rigid_matrix(hips.rotation, hips.translation);
        self.hips_translation = hips.translation;
        *hips = IDENTITY_SQT;
        for &(source, destination, ratio) in &self.twists {
            try_scene_twist_channels(
                &mut SceneChannels { bind: &self.bind, pose: &mut pose },
                source,
                destination,
                ratio,
                &mut HostReckoningNumeric,
                &mut HostReckoningSinCosNumeric,
            )
            .map_err(|e| format!("scene character twist: {e}"))?;
        }
        self.pose = pose;
        Ok(())
    }

    /// `Clip::Eval` (`0x82d6ef78`): the old and current times scaled by the
    /// base speed and clamped to `(mNbFrames - 1) / mFps`, the old one pulled
    /// to the current one when no loop came between; the ACS buffers the pair
    /// (`Fetch` `0x82a2f740`, fetch mode 2 resolving to the ACS's blend mode 1)
    /// and `BatchFetch` samples the current time with truncation offset 0.5.
    /// `UpdateOldTime` then moves the old time and clears the loops. Parts the
    /// clip lacks keep the extract buffer's identity, as does the part at bone
    /// 0 when the fetch system has a trajectory bone (`0x82d73ad8..0x82d73b0c`),
    /// whose motion goes to the trajectory node instead.
    fn fetch(&mut self) -> Result<Vec<AndaleSqt>, String> {
        let database = &self.data.database;
        let clip = &database.clips[self.clip];
        let scale = clip.base_speed;
        let last = (clip.frame_count - 1.0) / clip.frames_per_second;
        let clamp = |t: f32| {
            let t = if -t >= 0.0 { 0.0 } else { t };
            if last - t >= 0.0 { t } else { last }
        };
        // Clip::GetTime (0x828070c0): min(mTime, GetLength()).
        let length = self.length();
        let now = if length - self.time >= 0.0 { self.time } else { length };
        let end = clamp(now * scale);
        let mut start = clamp(self.old_time * scale);
        if self.loops == 0 && start > end {
            start = end;
        }
        let timing = BufferedFetchTiming::encode(start, end, self.loops, 2, 1).map_err(|e| e.to_string())?;
        let (_, current) = timing.decode();
        let pair = frame_pair(current, clip.frames_per_second, clip.frame_count, timing.blend_frames, 0.5).map_err(|e| e.to_string())?;
        let channel = clip.flags & 0x2000_0000 != 0;
        let mut delta = vec![
            AndaleSqt { translation: [0.0, 0.0, 0.0, if channel { 0.0 } else { 1.0 }], ..IDENTITY_SQT };
            database.hierarchy.bone_names.len()
        ];
        let record = database.records.iter().find(|r| r.offset == clip.record_offset).ok_or("scene clip header missing")?;
        for (local_part, destination) in database.hierarchy.parts.iter().enumerate() {
            if self.trajectory_node && destination.bone_offset == 0 {
                continue;
            }
            // Clip and hierarchy share this database: the part header's bits
            // 1..5 hold the hierarchy part.
            let Some(part) = clip.parts.iter().find(|p| ((p.header >> 1) & 31) as usize == local_part) else {
                continue;
            };
            let mut values = sample(part, pair).map_err(|e| format!("scene clip sample: {e:?}"))?;
            let start = destination.bone_offset as usize;
            let count = destination.bone_count as usize;
            if values.len() != count {
                return Err("scene clip part size mismatch".into());
            }
            for (i, value) in values.iter_mut().enumerate() {
                value.translation[3] = if channel {
                    let offset = record.data_range.start + (part.encoded_offset & 0x00ff_ffff) as usize + 16 + i * 4;
                    let bytes = self.data.bytes.get(offset..offset + 4).ok_or("scene clip channel weight outside clip")?;
                    f32::from_be_bytes(bytes.try_into().map_err(|_| "scene clip channel weight")?)
                } else {
                    1.0
                };
            }
            delta.get_mut(start..start + count).ok_or("scene clip part outside hierarchy")?.copy_from_slice(&values);
        }
        self.old_time = self.time;
        self.loops = 0;
        Ok(delta)
    }

    #[must_use]
    pub fn pose(&self) -> &[AndaleSqt] {
        &self.pose
    }

    #[must_use]
    pub fn hips_matrix(&self) -> AnimationMatrix {
        self.hips_matrix
    }

    #[must_use]
    pub fn trajectory_bone(&self) -> usize {
        self.trajectory
    }

    #[must_use]
    pub fn data(&self) -> &Arc<SceneAnimData> {
        &self.data
    }
}

/// `kNISCharacterLimit` (registered with 32, its maximum).
const CHARACTER_LIMIT: usize = 32;
/// The presentation's `SkaterFace` playback data, which `InitFace` reads.
pub(crate) const SKATER_FACE: &str = "data\\anim\\SkaterFacePres.abin";

/// `Character::GetCategory` (`0x82cfd2b0`, `gCategoryMapping`): a record
/// category's index and name; other names are "undefined".
#[must_use]
pub fn recipe_category(name: &str) -> (u32, &'static str) {
    match name {
        "marquee" => (0, "marquee"),
        "livingworld" => (1, "livingworld"),
        "vehicle" => (2, "vehicle"),
        "createacharacter" => (3, "createacharacter"),
        "test" => (5, "test"),
        _ => (4, "undefined"),
    }
}

/// `Character::GetRecipeFileName` (`0x82cfdfc0`) for the recipe the scene's
/// loader asks for (`SceneCharacterLoader::Load` `0x82863840`).
#[must_use]
pub fn recipe_path(category: &str, schema: &str) -> String {
    format!("data\\content/recipe/{}/{schema}.recipe", recipe_category(category).1)
}

/// `SceneBinDirectory::AddEntities` (`0x82877838`) with the ICE editor off:
/// records in order, each at its entry of `locations` (the snapped
/// `mCharacterLocationList`) when there is one. A "dynamic" record within the
/// playback overrides' `player_indexes` borrows the skater of the player at its
/// index (`InstanceCharacter` `0x828769f0`: `GetSkaterPresEntityFromPlayerId`,
/// its recipe's schema and face). A missing player is left out. Any other "dynamic" record is left
/// out, every other record counts against `kNISCharacterLimit`, and past it
/// every later one is left out.
/// `SceneCharacterManager::InstanceCharacter` (`0x82876bd8`) leaves out a
/// record whose `.abin` did not load (`GetAnimLoaderData` `0x8286f990`), and
/// one whose render entity was not created for want of its recipe
/// (`0x82877070..0x828770b8`); "current_player" needs the local player and
/// takes its schema and face (`InitFace`). Each character takes the next
/// free slot.
#[must_use]
pub fn instance_characters(
    records: &[SceneCharacterInfo],
    locations: &[[f32; 3]],
    player_indexes: &[i32],
    source: &dyn SceneCharacterSource,
) -> Vec<SceneCharacter> {
    let local = source.local_player();
    let local_index = source.local_index();
    let ratios = source.twist_ratios();
    let mut loaded: Vec<(String, Option<Arc<SceneAnimData>>)> = Vec::new();
    let mut face: Option<Option<AndaleDatabase>> = None;
    let mut characters = Vec::new();
    let mut non_dynamic = 0;
    for (index, record) in records.iter().enumerate() {
        let dynamic = record.category == "dynamic";
        let lent = if dynamic { player_indexes.get(index).copied() } else { None };
        if lent.is_none() {
            if !dynamic {
                non_dynamic += 1;
            }
            if non_dynamic > CHARACTER_LIMIT || dynamic {
                continue;
            }
        }
        let data = match loaded.iter().find(|(name, _)| *name == record.start_anim_db) {
            Some((_, data)) => data.clone(),
            None => {
                let data = load_anim(source, &record.start_anim_db, local.as_ref());
                loaded.push((record.start_anim_db.clone(), data.clone()));
                data
            }
        };
        let Some(data) = data else { continue };
        let player_recipe = match lent.or(local_index) {
            Some(player) => source.player(player),
            None => local.clone(),
        };
        let current_player = lent.is_some() || record.name == "current_player";
        let schema = if current_player {
            // GetSkaterPresEntityFromIndex(GetLocalIndex()) (0x82876ed0).
            let Some(local) = player_recipe.as_ref() else { continue };
            local.schema.clone()
        } else {
            if source.read(&recipe_path(&record.category, &record.name)).is_none() {
                continue;
            }
            record.name.clone()
        };
        let anim = SceneCharacterAnim::new(data, &record.start_anim_name, record.anim_offset, ratios).and_then(|mut anim| {
            if current_player {
                let face = face.get_or_insert_with(|| source.read(SKATER_FACE).and_then(|bytes| AndaleDatabase::parse(&bytes).ok()));
                let face = face.as_ref().ok_or("SkaterFace playback data missing")?;
                anim.init_face(face, &schema, player_recipe.as_ref().and_then(|l| l.create_a_character))?;
            }
            Ok(anim)
        });
        match anim {
            Ok(anim) => {
                let slot = characters.len();
                let location = locations.get(index).copied().unwrap_or(record.location);
                let mut character = SceneCharacter::new(slot, schema, record.category.clone(), current_player, location, record.orientation, anim);
                character.player = if current_player { lent.or(local_index) } else { None };
                characters.push(character);
            }
            Err(error) => eprintln!("NIS: character {} left out: {error}", record.name),
        }
    }
    characters
}

/// `SceneCharacterAnimLoader::Load` (`0x8286f5e8`): `data\` and the anim DB,
/// its first "_Player" read as "_Female" for a female create-a-skater
/// local player; a load that fails completes without data.
fn load_anim(source: &dyn SceneCharacterSource, anim_db: &str, local: Option<&LocalPlayerRecipe>) -> Option<Arc<SceneAnimData>> {
    let mut path = format!("data\\{anim_db}");
    if let Some(at) = path.find("_Player")
        && local.is_some_and(|l| l.create_a_character == Some(false))
    {
        path.replace_range(at..at + 7, "_Female");
    }
    let bytes = source.read(&path)?;
    match SceneAnimData::parse(bytes) {
        Ok(data) => Some(Arc::new(data)),
        Err(error) => {
            eprintln!("NIS: {path}: {error}");
            None
        }
    }
}

/// `SceneCharacter`'s `mUserTransform` (constructor `0x82875100`): the
/// orientation flattened (`0x82875208` stores 0 over its y), (0, 0, -1)
/// when x and z are both 0, then `Scene::ComputeTransform`'s basis with up
/// (0, 1, 0) stored as columns, the location in row 3.
#[must_use]
pub fn user_transform(location: [f32; 3], orientation: [f32; 3]) -> AnimationMatrix {
    let flat = if orientation[0] == 0.0 && orientation[2] == 0.0 { [0.0, 0.0, -1.0] } else { [orientation[0], 0.0, orientation[2]] };
    let m = compute_transform(location, flat, [0.0, 1.0, 0.0]);
    [
        m[0][0], m[1][0], m[2][0], 0.0, m[0][1], m[1][1], m[2][1], 0.0, m[0][2], m[1][2], m[2][2], 0.0, location[0], location[1], location[2],
        1.0,
    ]
}

/// `Sk8::Scene::SceneCharacter`: an instanced character record.
pub struct SceneCharacter {
    /// `mManagerIndex`, the `SceneCharacterManager` slot.
    pub slot: usize,
    /// `mSchemaName`: the recipe its render entity shows, the local
    /// player's for "current_player".
    pub schema: String,
    /// The record's category (`SceneBinCharacterInfo_t::mCategory`).
    pub category: String,
    /// "current_player": the local player's gameplay entity is posed
    /// (`ControlCharacterRenderable` `0x82870fb0`).
    pub borrowed: bool,
    /// The player whose presentation entity ControlCharacterRenderable borrowed.
    pub player: Option<i32>,
    user_transform: AnimationMatrix,
    /// `mTransform`.
    transform: AnimationMatrix,
    pub anim: SceneCharacterAnim,
    /// The render entity's inverse bind pose at the trajectory bone
    /// (`GetInvBindPoseBuffer` `0x82866068`), once it has an entity.
    trajectory_inverse_bind: Option<AnimationMatrix>,
    /// The render entity's `m_MatLtoW`, which `EvalCurrAnim` sets through
    /// `SetObjectTransform` (`0x82865fd0`).
    object: Option<AnimationMatrix>,
    /// PC park editor: the clip plays in place, its travel dropped and its hips kept over the
    /// placement (a NIS scene's clip carries its scene's position).
    pub in_place: bool,
}

impl SceneCharacter {
    #[must_use]
    pub fn new(slot: usize, schema: String, category: String, borrowed: bool, location: [f32; 3], orientation: [f32; 3], anim: SceneCharacterAnim) -> Self {
        let user_transform = user_transform(location, orientation);
        Self {
            slot,
            schema,
            category,
            borrowed,
            player: None,
            user_transform,
            transform: user_transform,
            anim,
            trajectory_inverse_bind: None,
            object: None, in_place: false,
        }
    }

    /// `SetSceneTransform` (`0x82862158`): `mUserTransform` times the
    /// scene's `ComputeTransform`.
    pub fn set_scene_transform(&mut self, scene: AnimationMatrix) {
        self.transform = multiply(scene, self.user_transform);
    }

    /// `GetSceneTransform` (`0x82862120`).
    #[must_use]
    pub fn scene_transform(&self) -> AnimationMatrix {
        self.transform
    }

    /// `Character::ViewerCharacterEntity::UpdateTransform` (`0x8258b278`):
    /// `mDefRot` about up (`XMVectorSinCos`), rows (cos, 0, -sin), (0, 1, 0),
    /// (sin, 0, cos), at `mDefPos`. The viewer's `EvalCurrAnim` reads it
    /// through `GetTransform` (`0x82586e70`) where a scene character reads its
    /// scene transform.
    pub fn set_viewer_transform(&mut self, position: [f32; 3], rotation: f32) {
        let (sin, cos) = try_sin_cos(rotation, &mut HostReckoningSinCosNumeric).unwrap_or_else(|never| match never {});
        self.user_transform =
            [cos, 0.0, -sin, 0.0, 0.0, 1.0, 0.0, 0.0, sin, 0.0, cos, 0.0, position[0], position[1], position[2], 1.0];
        self.transform = self.user_transform;
    }

    /// PC park editor: the character placed by a full matrix (row major, translation last), its
    /// tilts kept as well as its heading.
    ///
    /// # Errors
    /// The clip's evaluation (the object matrix follows at once, frozen clip or not).
    pub fn set_viewer_matrix(&mut self, matrix: [f32; 16]) -> Result<(), String> {
        self.user_transform = matrix;
        self.transform = matrix;
        self.update_object()
    }

    /// `SceneCharacter::SetTime` (`0x82862080`): the clip at `time`, then
    /// `EvalCurrAnim`'s object matrix.
    ///
    /// # Errors
    /// The clip's evaluation.
    pub fn set_time(&mut self, time: f32) -> Result<(), String> {
        self.anim.set_time(time)?;
        self.update_object()
    }

    /// `SceneCharacter::AddTime` (`0x828620d0`).
    ///
    /// # Errors
    /// The clip's evaluation.
    pub fn add_time(&mut self, dt: f32) -> Result<(), String> {
        self.anim.add_time(dt)?;
        self.update_object()
    }

    /// Gives the character its render entity: that entity's inverse bind
    /// pose at the trajectory bone, which the object matrix takes.
    ///
    /// # Errors
    /// The clip's hierarchy.
    pub fn attach_render_entity(&mut self, trajectory_inverse_bind: AnimationMatrix) -> Result<(), String> {
        self.trajectory_inverse_bind = Some(trajectory_inverse_bind);
        self.update_object()
    }

    #[must_use]
    pub fn has_render_entity(&self) -> bool {
        self.trajectory_inverse_bind.is_some()
    }

    /// `GetObjectTransform` (`0x82865f08`): the render entity's
    /// local-to-world matrix as the last evaluation left it.
    #[must_use]
    pub fn object_transform(&self) -> Option<AnimationMatrix> {
        self.object
    }

    fn update_object(&mut self) -> Result<(), String> {
        if let Some(inverse_bind) = self.trajectory_inverse_bind {
            let mut trajectory = multiply_matrices(self.global_pose()?[self.anim.trajectory], inverse_bind);
            if self.in_place {
                trajectory[12] = 0.0;
                trajectory[13] = 0.0;
                trajectory[14] = 0.0;
            }
            let mut root = compose_object(self.transform, trajectory, self.anim.hips_matrix, self.anim.hips_translation).root;
            if self.in_place {
                // Over the placement across the ground; the clip's own height stays.
                root[12] = self.transform[12];
                root[14] = self.transform[14];
            }
            self.object = Some(root);
        }
        Ok(())
    }

    /// The clip's pose through the hierarchy: `NewACS` gives the skeleton
    /// trajectory node 0 (`0x824d6340`), so that bone's children are roots.
    fn global_pose(&self) -> Result<Vec<AnimationMatrix>, String> {
        let mut skeleton = AnimationSkeleton::from_andale(&self.anim.data.database.hierarchy).map_err(|e| e.to_string())?;
        skeleton.trajectory_bone = Some(0);
        skeleton.global_matrices(&LocalPose::from_andale(&self.anim.pose)).map_err(|e| e.to_string())
    }

    /// `EvalCurrAnim`'s skinning and object matrix (`0x82866edc..0x8286717c`):
    /// the pose through the hierarchy and the render entity's inverse bind
    /// pose (`PoseWithMatMult`); the object matrix applies the hips'
    /// transform, the trajectory's matrix, then `mTransform`, and the
    /// trajectory's palette slot takes the hips' transform.
    ///
    /// # Errors
    /// A palette built over another hierarchy.
    pub fn publish(&self, palette: &mut CharacterPalette) -> Result<ScenePose, String> {
        let skin = self.pose_palette(palette)?;
        palette
            .pack_for_upload(skin.pose.root)
            .map_err(|e| e.to_string())?;
        Ok(skin.pose)
    }

    /// [`Self::publish`] before the upload, which a borrowed render entity's
    /// jobs come between (`SkaterPresEntity::StartJobs` `0x827dbb90`,
    /// `EndJobs` `0x827dbd50`).
    ///
    /// # Errors
    /// A palette built over another hierarchy.
    pub fn pose_palette(&self, palette: &mut CharacterPalette) -> Result<SceneSkin, String> {
        let anim = &self.anim;
        let global = self.global_pose()?;
        let mut matrices = apply_inverse_bind(&global, &palette.skinning().hierarchy_inverse_bind_pose).map_err(|e| e.to_string())?;
        let pose = compose_object(self.transform, matrices[anim.trajectory], anim.hips_matrix, anim.hips_translation);
        matrices[anim.trajectory] = anim.hips_matrix;
        palette.update_bone_transforms(&matrices).map_err(|e| e.to_string())?;
        let mut first_bone = matrices[0];
        // 0x8286717c: the trajectory's slot keeps the hips' translation whole.
        if anim.trajectory == 0 {
            first_bone[15] = anim.hips_translation[3];
        }
        Ok(SceneSkin { pose, first_bone })
    }
}

/// What `EvalCurrAnim` leaves for a borrowed render entity's jobs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneSkin {
    pub pose: ScenePose,
    /// The pose buffer's first matrix (`GetSkaterBoneBuffer` `0x82865d60`),
    /// from which `SkaterPresEntity::StartJobs` (`0x827dbb90`) takes
    /// `mHipsTransform`.
    pub first_bone: AnimationMatrix,
}

/// What `EvalCurrAnim` publishes for a character.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScenePose {
    /// The render entity's local-to-world matrix.
    pub root: AnimationMatrix,
    /// `mHipsPos` (`0x8286713c`), which the render entity's `GetHipsPos`
    /// callback answers.
    pub hips: [f32; 3],
}

/// `EvalCurrAnim` `0x82866f5c..0x82867150`: `R` is the trajectory's matrix
/// times `mTransform`, the object matrix the hips' transform times `R`, and
/// `mHipsPos` the hips' translation (its own w) times `R`.
fn compose_object(transform: AnimationMatrix, trajectory: AnimationMatrix, hips_matrix: AnimationMatrix, hips: [f32; 4]) -> ScenePose {
    let r = multiply(transform, trajectory);
    let root = multiply(r, hips_matrix);
    let hips = std::array::from_fn(|c| hips[3].mul_add(r[12 + c], hips[2].mul_add(r[8 + c], hips[1].mul_add(r[4 + c], hips[0] * r[c]))));
    ScenePose { root, hips }
}

/// `mBPSQTBuffer` and `mTwistSQTBuffer` for the twists.
struct SceneChannels<'a> {
    bind: &'a [AndaleSqt],
    pose: &'a mut [AndaleSqt],
}

impl CorrectionChannels for SceneChannels<'_> {
    fn bind(&self, i: usize) -> Option<AndaleSqt> {
        self.bind.get(i).copied()
    }
    fn replay(&self, i: usize) -> Option<AndaleSqt> {
        self.pose.get(i).copied()
    }
    fn rotation(&mut self, i: usize, v: [f32; 4]) -> Result<(), ActorFaceError> {
        self.pose.get_mut(i).ok_or(ActorFaceError::UnsupportedPose)?.rotation = v;
        Ok(())
    }
    fn translation(&mut self, i: usize, v: [f32; 4]) -> Result<(), ActorFaceError> {
        self.pose.get_mut(i).ok_or(ActorFaceError::UnsupportedPose)?.translation = v;
        Ok(())
    }
    fn scale(&mut self, i: usize, v: [f32; 4]) -> Result<(), ActorFaceError> {
        self.pose.get_mut(i).ok_or(ActorFaceError::UnsupportedPose)?.scale = v;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};

    /// The boot archives and `scene.big`.
    fn mounts() -> MountSet {
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let mut mounts = MountSet::new();
        for archive in SKATE2_ARCHIVES.iter().take(5) {
            mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).unwrap());
        }
        mounts
    }

    #[test]
    fn the_object_matrix_and_hips_match_the_native_composition() {
        // An emulation of EvalCurrAnim 0x82866df0..0x82867180 (hips SQT,
        // trajectory matrix and scene transform in, rows and mHipsPos out).
        let f = |bits: &[u32]| bits.iter().map(|&b| f32::from_bits(b)).collect::<Vec<_>>();
        let q: [f32; 4] = f(&[0x3e3a_f4ba, 0x3eba_f4ba, 0xbf0c_378c, 0x3f3a_f4ba]).try_into().unwrap();
        let t: [f32; 4] = f(&[0x3e80_0000, 0x3f60_0000, 0xbe00_0000, 0x3f80_0000]).try_into().unwrap();
        let trajectory: AnimationMatrix = f(&[
            0x3f53_4932, 0, 0xbf10_8c69, 0, 0, 0x3f80_0000, 0, 0, 0x3f10_8c69, 0, 0x3f53_4932, 0, 0x44d5_634b, 0x42db_4104, 0xc410_ad13, 0x3f80_0000,
        ])
        .try_into()
        .unwrap();
        let scene: AnimationMatrix = f(&[
            0x3ee8_3dc0, 0, 0x3f64_262a, 0, 0, 0x3f80_0000, 0, 0, 0xbf64_262a, 0, 0x3ee8_3dc0, 0, 0xc2f1_0000, 0x4050_0000, 0x4223_0000, 0x3f80_0000,
        ])
        .try_into()
        .unwrap();
        let pose = compose_object(scene, trajectory, rigid_matrix(q, t), t);
        let root = f(&[
            0x3eef_eafc, 0xbf2a_aaab, 0xbf14_6319, 0, 0x3f62_0c52, 0x3eaa_aaaa, 0x3ea9_311b, 0, 0xbcdd_eada, 0xbf2a_aaaa, 0x3f3e_af61, 0, 0x4492_3b86, 0x42e3_8104, 0x44a2_74a2, 0x3f80_0000,
        ]);
        assert_eq!(pose.root.map(f32::to_bits), <[f32; 16]>::try_from(root).unwrap().map(f32::to_bits));
        assert_eq!(pose.hips.map(f32::to_bits), [0x4492_3b86, 0x42e3_8104, 0x44a2_74a2]);
    }

    #[test]
    fn the_user_transform_is_the_flattened_basis_in_columns() {
        // A tilted forward loses its y (0x82875208); the basis is stored as
        // columns with the location in row 3.
        let m = user_transform([1.0, 2.0, 3.0], [0.6, 0.3, 0.8]);
        let flat = compute_transform([1.0, 2.0, 3.0], [0.6, 0.0, 0.8], [0.0, 1.0, 0.0]);
        for row in 0..3 {
            for column in 0..3 {
                assert_eq!(m[row * 4 + column].to_bits(), flat[column][row].to_bits());
            }
        }
        assert_eq!(&m[12..], &[1.0, 2.0, 3.0, 1.0]);
        assert_eq!(user_transform([4.0, 5.0, 6.0], [0.0, 0.7, 0.0]), user_transform([4.0, 5.0, 6.0], [0.0, 0.0, -1.0]));
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn current_player_clips_pose_the_gameplay_presentation_rig() {
        // The borrowed entity skins the scene's pose buffer with its own
        // inverse bind pose, bone for bone (EvalCurrAnim 0x82866efc..0x82866f38).
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let mut mounts = MountSet::new();
        for archive in SKATE2_ARCHIVES {
            let path = archive.path_under(std::path::Path::new(&root));
            if path.is_file() {
                mounts.push(MountedBig::open(path, archive.use_external_header).unwrap());
            }
        }
        let presentation = AndaleDatabase::parse(&mounts.read_path("data/anim/SkaterPresentationDB.abin").unwrap()).unwrap();
        for clip in ["WelcomeBack_Player1", "01_ProChall_Player1", "WelcomeBack_Female1"] {
            let bytes = mounts.read_path(&format!("data/scene/anim/{clip}.abin")).unwrap();
            let scene = AndaleDatabase::parse(&bytes).unwrap();
            assert_eq!(scene.hierarchy.bone_names, presentation.hierarchy.bone_names, "{clip}");
        }
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn the_borrowed_rigs_first_bone_is_its_trajectory() {
        // StartJobs (0x827dbb90) takes mHipsTransform from the bone buffer's
        // first matrix, the trajectory's slot EvalCurrAnim fills with the hips.
        let mounts = mounts();
        let bytes = mounts
            .read_path("data\\scene/anim/01_ProChall_Player1.abin")
            .unwrap();
        let data = Arc::new(SceneAnimData::parse(bytes).unwrap());
        let clip = data.database.clips[0].name.clone();
        let anim =
            SceneCharacterAnim::new(data, &clip, 0.0, twist_ratios(&mounts).unwrap()).unwrap();
        assert_eq!(anim.trajectory_bone(), 0);
    }

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn the_first_tutorial_scene_clip_poses_its_character() {
        let mounts = mounts();
        let ratios = twist_ratios(&mounts).unwrap();
        eprintln!("twist ratios {ratios:?}");
        let bytes = mounts.read_path("data\\scene/anim/tutorial_m_1a_goto_slappys_A01_01.abin").unwrap();
        let data = Arc::new(SceneAnimData::parse(bytes).unwrap());
        let hierarchy = &data.database.hierarchy;
        let clip = data.database.clips.iter().find(|c| c.name.eq_ignore_ascii_case("tutorial_m_1a_goto_slappys_A01_0")).unwrap();
        eprintln!(
            "bones {} has_trajectory {} codec {:#x} flags {:#x} looping {} frames {} fps {} base {} parts {:?}",
            hierarchy.bone_names.len(),
            hierarchy.has_trajectory,
            clip.codec,
            clip.flags,
            clip.looping(),
            clip.frame_count,
            clip.frames_per_second,
            clip.base_speed,
            hierarchy.parts.iter().map(|p| (p.name.clone(), p.bone_offset, p.bone_count)).collect::<Vec<_>>()
        );
        eprintln!("first bones {:?}", &hierarchy.bone_names[..hierarchy.bone_names.len().min(8)]);
        let anim = SceneCharacterAnim::new(Arc::clone(&data), "tutorial_m_1a_goto_slappys_A01_0", 0.0, ratios);
        match anim {
            Ok(mut anim) => {
                let result = anim.set_time(0.0);
                eprintln!("set_time: {result:?}");
                for step in 0..4 {
                    let traj = anim.pose()[anim.trajectory_bone()];
                    eprintln!("t {:.3} trajectory {:?} hips {:?}", anim.time, traj, &anim.hips_matrix()[12..15]);
                    let _ = step;
                    if anim.add_time(2.0).is_err() {
                        break;
                    }
                }
            }
            Err(error) => eprintln!("anim: {error}"),
        }
    }
}
