//! Same-player authored presentation startup over supplied mounted/job owners.
use super::{
    PresentationAnimation, PresentationAnimationError,
    restore::{PresentationRestoreSource, RestoredPresentationBase},
};
use crate::{
    actor_animation_construction::initialization::ActorAnimationInitializeSource,
    actor_construction::{
        player_animation::{ActorAnimationPlayerServices, PlayerAnimationError},
        selected_startup::SelectedPlayerStartupOwners,
    },
    actor_face_playback::FaceAnimationManagers,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    simulation_animation_startup::AnimationStartupSource,
};
use skate2_formats::recipe::CharacterRecipe;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PresentationRestoreServiceError {
    Cvar(InputCvarError),
    CustomizationsOwnerRequired,
}
impl std::fmt::Display for PresentationRestoreServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(e) => write!(f, "presentation restore cvar: {e:?}"),
            Self::CustomizationsOwnerRequired => {
                f.write_str("presentation recipe customizations owner required")
            }
        }
    }
}
pub(crate) struct PresentationRestoreServices<'a>(pub(crate) &'a InputCvarReferences);
impl PresentationRestoreSource for PresentationRestoreServices<'_> {
    type Error = PresentationRestoreServiceError;
    fn individual_full_bind_pose(&mut self) -> Result<bool, Self::Error> {
        self.0
            .try_individual_full_bind_pose()
            .map_err(PresentationRestoreServiceError::Cvar)
    }
    fn is_male(&mut self, recipe: &CharacterRecipe) -> Result<bool, Self::Error> {
        // CharacterEntity::IsMale82cfe138 returns true for the ACTUAL null
        // customizations pointer. The mounted recipe parser proves that null.
        // Never replace a present customizations owner with backend gender.
        if let Some(customizations) = &recipe.customizations {
            Ok(customizations.is_male)
        } else if recipe.has_customizations {
            Err(PresentationRestoreServiceError::CustomizationsOwnerRequired)
        } else {
            Ok(true)
        }
    }
}
pub(crate) struct SelectedPresentationStartup {
    pub(crate) animation: PresentationAnimation,
    pub(crate) restored: Option<RestoredPresentationBase>,
    pub(crate) material_source:
        Option<std::sync::Arc<crate::player::mounted_recipe::finalize::FinalizedRecipeScene>>,
    pub(crate) twist_attributes: Option<super::expressions::PresentationTwistAttributes>,
}
#[derive(Debug)]
pub(crate) enum PresentationStartupError<S, J> {
    AlreadyAttempted,
    Recipe(crate::player::mounted_recipe::MountedRecipeError),
    Binding(PlayerAnimationError),
    Initialize(PresentationAnimationError<S, J>),
    Pedestrian(crate::selected_living_world_pedestrian_visual::PedestrianPlaybackError<S>),
    Restore(PresentationAnimationError<PresentationRestoreServiceError, J>),
}
impl<S: std::fmt::Debug, J: std::fmt::Debug> std::fmt::Display for PresentationStartupError<S, J> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAttempted => {
                f.write_str("selected presentation startup already attempted")
            }
            Self::Recipe(e) => write!(f, "selected presentation recipe: {e}"),
            Self::Binding(e) => write!(f, "selected presentation jobs: {e}"),
            Self::Initialize(e) => write!(f, "selected presentation initialization: {e}"),
            Self::Pedestrian(e) => write!(f, "selected pedestrian animation initialization: {e}"),
            Self::Restore(e) => write!(f, "selected presentation restore base: {e}"),
        }
    }
}
impl SelectedPresentationStartup {
    pub(crate) fn new(managers: FaceAnimationManagers) -> Self {
        Self {
            animation: PresentationAnimation::new(managers),
            restored: None,
            material_source: None,
            twist_attributes: None,
        }
    }
    pub(crate) fn try_initialize_and_restore<
        S: AnimationStartupSource,
        J: ActorAnimationInitializeSource,
    >(
        &mut self,
        recipe: Arc<CharacterRecipe>,
        source: &mut S,
        cvars: &InputCvarReferences,
        jobs: &mut J,
    ) -> Result<(), PresentationStartupError<S::Error, J::Error>> {
        self.animation
            .try_initialize(source, jobs)
            .map_err(PresentationStartupError::Initialize)?;
        self.restored = Some(
            self.animation
                .try_restore_first_base(recipe, &mut PresentationRestoreServices(cvars), jobs)
                .map_err(PresentationStartupError::Restore)?,
        );
        Ok(())
    }
}
impl SelectedPlayerStartupOwners {
    /// A replaced CAC recipe follows the same RestoreBase face/bind path as
    /// first creation; the shared animation managers and NPC registrations live on.
    pub(crate) fn try_restore_replaced_recipe(&mut self)->Result<(),String> {
        let load=self.recipe_load.as_ref().ok_or("CAC recipe owner missing")?;
        let recipe=Arc::clone(load.recipe().map_err(|e|e.to_string())?);
        let finalized=Arc::clone(load.finalized().map_err(|e|e.to_string())?);
        let presentation=self.presentation.as_mut().ok_or("CAC presentation owner missing")?;
        let mut jobs=ActorAnimationPlayerServices::try_bind(&self.services,&self.animation,&self.cvars)
            .map_err(|e|format!("CAC animation services: {e:?}"))?;
        let restored=presentation.animation.try_restore_first_base(recipe,
            &mut PresentationRestoreServices(&self.cvars),&mut jobs)
            .map_err(|e|format!("CAC RestoreBase: {e:?}"))?;
        presentation.restored=Some(restored);
        presentation.material_source=Some(finalized);
        Ok(())
    }
    /// A later SkaterPresEntity borrows the global SkaterPresentation animation
    /// initialized by Presentation::Init827f10c0, then restores its own recipe.
    /// Neither spawning nor removing an entity recreates/destroys the global ACS.
    pub(crate) fn try_new_skater_presentation<S: AnimationStartupSource>(
        &self,
        recipe: Arc<CharacterRecipe>,
        finalized: Arc<crate::player::mounted_recipe::finalize::FinalizedRecipeScene>,
        _source: &mut S,
    ) -> Result<SelectedPresentationStartup, PresentationStartupError<S::Error, PlayerAnimationError>> {
        let shared = self.presentation.as_ref().ok_or(PresentationStartupError::Initialize(
            PresentationAnimationError::Missing("global SkaterPresentation")))?;
        let animation = shared.animation.try_for_entity().map_err(PresentationStartupError::Initialize)?;
        let mut presentation = SelectedPresentationStartup {
            animation, restored: None, material_source: None, twist_attributes: None,
        };
        presentation.material_source = Some(finalized);
        presentation.twist_attributes = Some(
            super::expressions::PresentationTwistAttributes::bind_global(&self.services.vault),
        );
        let mut jobs =
            ActorAnimationPlayerServices::try_bind(&self.services, &self.animation, &self.cvars)
                .map_err(PresentationStartupError::Binding)?;
        presentation.restored = Some(presentation.animation.try_restore_first_base(
            recipe, &mut PresentationRestoreServices(&self.cvars), &mut jobs,
        ).map_err(PresentationStartupError::Restore)?);
        Ok(presentation)
    }
    /// Caller supplies the actual decoded recipe associated with this selected
    /// player. This does not complete loader registration or publish a live pose.
    pub(crate) fn try_initialize_selected_presentation<S: AnimationStartupSource>(
        &mut self,
        recipe: Arc<CharacterRecipe>,
        source: &mut S,
    ) -> Result<(), PresentationStartupError<S::Error, PlayerAnimationError>> {
        if self.presentation.is_some() {
            return Err(PresentationStartupError::AlreadyAttempted);
        }
        let managers =
            FaceAnimationManagers::new_with_database(Arc::clone(&self.animation.managers.database));
        let presentation = self
            .presentation
            .insert(SelectedPresentationStartup::new(managers));
        presentation.material_source = self
            .recipe_load
            .as_ref()
            .map(|load| load.finalized().cloned())
            .transpose()
            .map_err(PresentationStartupError::Recipe)?;
        // Selected retained GlobalAttributeCollection+d8 capability, shared Vault.
        presentation.twist_attributes = Some(
            super::expressions::PresentationTwistAttributes::bind_global(&self.services.vault),
        );
        let mut jobs =
            ActorAnimationPlayerServices::try_bind(&self.services, &self.animation, &self.cvars)
                .map_err(PresentationStartupError::Binding)?;
        presentation.try_initialize_and_restore(recipe, source, &self.cvars, &mut jobs)?;
        drop(jobs);
        self.pedestrian_animation
            .initialize(&mut self.animation, &mut presentation.animation, source)
            .map_err(PresentationStartupError::Pedestrian)
    }
}

#[cfg(test)]
#[path = "skater_presentation_startup_corpus.rs"]
pub(crate) mod corpus;
#[cfg(test)]
#[path = "skater_presentation_startup_tests.rs"]
mod tests;
