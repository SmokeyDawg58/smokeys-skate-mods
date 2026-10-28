//! Same-Actor `SubmittableSkater` pose getters, in `WriteSkater827d8ab8` order.
//! Draft child of `actor_construction`; compression/publication is a separate caller.
use super::{
    ActorConstruction,
    anim_out_phys_in::{AnimOutPhysInOwner, publication::AnimationPhysicsPublicationError},
    body_session::BodySessionError,
    physical_mount::ActorPhysicalMountError,
};
use crate::actor_animation_construction::initialization::bind_pose::sqt_output::OwnedAnimationSqtBuffer;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum PresentationPoseSourceError {
    Physical(ActorPhysicalMountError),
    Input(AnimationPhysicsPublicationError),
    Animation(BodySessionError),
    Missing(&'static str),
    ForeignInput,
    Lock,
}
impl std::fmt::Display for PresentationPoseSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Physical(e) => write!(f, "presentation physical source: {e:?}"),
            Self::Input(e) => write!(f, "presentation local pose: {e}"),
            Self::Animation(e) => write!(f, "presentation animation source: {e}"),
            Self::Missing(field) => write!(f, "presentation source missing {field}"),
            Self::ForeignInput => f.write_str("presentation local pose belongs to another input"),
            Self::Lock => f.write_str("presentation source lock poisoned"),
        }
    }
}
/// Retained source allocations, not copied pose values. The writer samples data
/// at its native compression stages and samples `IsInitFace` separately afterward.
pub(crate) struct CapturedPresentationPose<'a> {
    pub(crate) actor: &'a ActorConstruction,
    pub(crate) local: Arc<Mutex<AnimOutPhysInOwner>>,
    pub(crate) face: OwnedAnimationSqtBuffer,
    pub(crate) right: OwnedAnimationSqtBuffer,
    pub(crate) left: OwnedAnimationSqtBuffer,
}
impl ActorConstruction {
    /// `GetLocalPose825f04e0` -> `GetFace825f05f8` -> `GetRight825f06e8` -> `GetLeft825f0670`.
    /// The admitted initialized-player route requires actual allocated buffers;
    /// an unknown/unconstructed hand is not interpreted as a native null pointer.
    pub(crate) fn try_capture_presentation_pose(
        &self,
    ) -> Result<CapturedPresentationPose<'_>, PresentationPoseSourceError> {
        use PresentationPoseSourceError as E;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let local = physical
            .owner()
            .animation_physics_input
            .lock()
            .map_err(|_| E::Lock)?
            .as_ref()
            .map(Arc::clone)
            .ok_or(E::Missing("physical local pose owner"))?;
        if !Arc::ptr_eq(
            &local,
            &self.published_animation_physics_input().map_err(E::Input)?,
        ) {
            return Err(E::ForeignInput);
        }
        let face = {
            let animation = self.checked_body_animation().map_err(E::Animation)?;
            let animation = animation.lock().map_err(|_| E::Lock)?;
            animation
                .face()
                .ok_or(E::Missing("face pose allocation"))?
                .local_pose
                .clone()
        };
        let right = self.try_capture_presentation_hand(1)?;
        let left = self.try_capture_presentation_hand(0)?;
        Ok(CapturedPresentationPose {
            actor: self,
            local,
            face,
            right,
            left,
        })
    }
    fn try_capture_presentation_hand(
        &self,
        side: usize,
    ) -> Result<OwnedAnimationSqtBuffer, PresentationPoseSourceError> {
        use PresentationPoseSourceError as E;
        let animation = self.checked_body_animation().map_err(E::Animation)?;
        let animation = animation.lock().map_err(|_| E::Lock)?;
        let state = animation
            .initialization()
            .ok_or(E::Missing("animation Initialize"))?;
        Ok(state.hands[side]
            .as_ref()
            .ok_or(E::Missing("initialized hand allocation"))?
            .local_pose
            .clone())
    }
    /// `IsInitFace825f0230` reads the actual `SkaterAnim` flag23 when `WriteSkater`
    /// reaches its face branch, after both hand compression paths.
    pub(crate) fn try_presentation_face_initialized(
        &self,
    ) -> Result<bool, PresentationPoseSourceError> {
        use PresentationPoseSourceError as E;
        let animation = self.checked_body_animation().map_err(E::Animation)?;
        let animation = animation.lock().map_err(|_| E::Lock)?;
        let player = animation.player().ok_or(E::Missing("SkaterAnim player"))?;
        if player.flags_known_mask & 0x0080_0000 == 0 {
            return Err(E::Missing("IsInitFace flag"));
        }
        Ok(player.flags_bits & 0x0080_0000 != 0)
    }
}

#[cfg(test)]
#[path = "../rendering/skater/skater_presentation_write_tests.rs"]
mod tests;
