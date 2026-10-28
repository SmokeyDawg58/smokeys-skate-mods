//! Selected restore-base work; caller supplies the recipe from actual `UpdatePresEntity`.
use super::{
    ActorFaceError, ActorGraphIntents, Arc, BindPoseError, Mutex, PresentationAnimation,
    PresentationAnimationError, hierarchy, offsets::part_range, runtime,
};
use crate::{
    actor_animation_construction::{
        AnimationPointerState,
        hands_face::{HandFaceStreamCommands, commands::StreamCommands},
        initialization::ActorAnimationInitializeSource,
    },
    actor_animation_tree::eval::AnimationEvalCommands,
};
use skate2_formats::recipe::CharacterRecipe;

pub(crate) trait PresentationRestoreSource {
    type Error;
    fn individual_full_bind_pose(&mut self) -> Result<bool, Self::Error>;
    /// Actual `CharacterEntity::IsMale` is queried only after a missing CAC pose.
    fn is_male(&mut self, recipe: &CharacterRecipe) -> Result<bool, Self::Error>;
}

pub(crate) struct RestoredPresentationBase {
    pub(crate) recipe: Arc<CharacterRecipe>,
    pub(crate) previous: super::OwnedAnimationSqtBuffer,
    pub(crate) next: super::OwnedAnimationSqtBuffer,
    pub(crate) face_pose_found: bool,
}

impl PresentationAnimation {
    /// Executes the first selected Actor's authored base and face initialization.
    /// This does not replace the caller's entity association, compressed reads,
    /// frame mapping or subsequent `UpdateBodies` work.
    pub(crate) fn try_restore_first_base<
        S: PresentationRestoreSource,
        J: ActorAnimationInitializeSource,
    >(
        &mut self,
        recipe: Arc<CharacterRecipe>,
        source: &mut S,
        jobs: &mut J,
    ) -> Result<RestoredPresentationBase, PresentationAnimationError<S::Error, J::Error>> {
        if !self.initialized {
            return Err(PresentationAnimationError::Missing("InitAnimation"));
        }
        let full = self
            .full
            .as_ref()
            .ok_or(PresentationAnimationError::Missing("SkaterFull"))?;
        let full_hierarchy = hierarchy(&full.lock().map_err(|_| ActorFaceError::Lock)?.database)?;
        let individual_full = source
            .individual_full_bind_pose()
            .map_err(PresentationAnimationError::Source)?;
        let (playback_name, offset) = if individual_full {
            (b"SkaterFull".as_slice(), 0)
        } else {
            let offset = part_range(&full_hierarchy, "Face_Offset0")?.start;
            (b"SkaterFace".as_slice(), offset)
        };
        // Raw68b8 resolves the current manager entry after cvar/part selection.
        // Do not silently substitute the constructor's cached playback owner.
        let playback = self
            .managers
            .playback
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .entries
            .get(&(ActorGraphIntents::native_name(playback_name), 0))
            .cloned()
            .ok_or(PresentationAnimationError::Missing(
                "selected presentation playback",
            ))?;
        let bind = self
            .bind
            .as_ref()
            .ok_or(PresentationAnimationError::Missing("authored bind SQT"))?;
        let next = self
            .replay
            .get(1)
            .ok_or(PresentationAnimationError::Missing("odd replay allocation"))?
            .clone();
        // OverrideWith827e6968 copies matching hierarchy parts in source order.
        // Cvar6834 and playback lookup68b8 precede this copy; BeginSet follows.
        next.try_override_from(bind)?;
        let commands = Arc::clone(&playback.lock().map_err(|_| ActorFaceError::Lock)?.commands);
        runtime::begin_command_set(&commands)?;
        let playback = playback.lock().map_err(|_| ActorFaceError::Lock)?;
        let mut pose = playback.get_pose(ActorGraphIntents::native_name(
            recipe.schema_name.as_bytes(),
        ))?;
        if pose.is_none() {
            // Raw827e6b70 compares recipe+44 directly with3 before IsMale.
            // The category is never inferred from the recipe name.
            let fallback = if recipe.category == CREATE_A_CHARACTER_CATEGORY {
                if source
                    .is_male(&recipe)
                    .map_err(PresentationAnimationError::Source)?
                {
                    b"CAC_male".as_slice()
                } else {
                    b"CAC_female".as_slice()
                }
            } else {
                b"rig_Tpose".as_slice()
            };
            pose = playback.get_pose(ActorGraphIntents::native_name(fallback))?;
        }
        let face_pose_found = pose.is_some();
        if let Some(pose) = pose {
            let count = hierarchy(&playback.database)?.bone_names.len();
            let end = offset
                .checked_add(count)
                .ok_or(PresentationAnimationError::Allocation)?;
            let destination = next.try_subrange(offset..end)?;
            let mut stream = StreamCommands::new(
                Arc::clone(&commands),
                destination,
                AnimationPointerState::Null,
            )?;
            stream.begin_stream()?;
            stream.fetch_pose(&pose)?;
            stream.extract_sqt()?;
            stream.end_stream()?;
        } else {
            // Source still BeginCommands/EndCommands when both pose lookups miss.
            begin_and_end_empty(&commands)?;
        }
        drop(playback);
        runtime::end_command_set(&commands, jobs).map_err(PresentationAnimationError::Dispatch)?;
        runtime::sync_command_set(&commands, jobs).map_err(PresentationAnimationError::Dispatch)?;
        // Actual odd->even copy follows FacePres Sync827e6fc8, before UpdateBodies7014.
        let previous = self
            .replay
            .first()
            .ok_or(PresentationAnimationError::Missing(
                "even replay allocation",
            ))?
            .clone();
        previous.try_override_from(&next)?;
        Ok(RestoredPresentationBase {
            recipe,
            previous,
            next,
            face_pose_found,
        })
    }
}

fn begin_and_end_empty(commands: &Arc<Mutex<super::FaceCommands>>) -> Result<(), BindPoseError> {
    let commands = commands.lock().map_err(|_| ActorFaceError::Lock)?;
    let mut buffered = commands
        .buffered
        .as_ref()
        .ok_or(BindPoseError::BufferedOwnerRequired)?
        .lock()
        .map_err(|_| ActorFaceError::Lock)?;
    if !buffered.constructed || buffered.in_command_set == 0 || buffered.in_commands != 0 {
        return Err(BindPoseError::AlreadyInCommands);
    }
    buffered.in_commands += 1;
    buffered.in_commands -= 1;
    Ok(())
}

// CharacterEntity category3, raw cmpwi827e6b70.
const CREATE_A_CHARACTER_CATEGORY: u32 = 3;
