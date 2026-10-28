//! Same Actor/backend/manager getters in native `WriteData827dbf98` order.
use super::{
    ActorConstruction, ActorConstructionError, body_session::BodySessionError,
    physical_mount::ActorPhysicalMountError, presentation_pose_source::PresentationPoseSourceError,
};
use crate::{
    physical_player_phys_out::construct::ConcretePhysOutError,
    player::attributes_runtime::PlayerAttributeServices,
    players_backend_settings::PlayersSettingsError,
    skater_manager_local_identity::LocalIdentityError,
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::convert::Infallible;
#[derive(Debug)]
pub(crate) enum PresentationFrameSourceError {
    Actor(ActorConstructionError),
    Physical(ActorPhysicalMountError),
    Animation(BodySessionError),
    Pose(PresentationPoseSourceError),
    Output(ConcretePhysOutError<Infallible>),
    Settings(PlayersSettingsError),
    Identity(LocalIdentityError),
    Missing(&'static str),
    Bone(i32),
    Lock,
}
impl std::fmt::Display for PresentationFrameSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(e) => write!(f, "frame Actor: {e:?}"),
            Self::Physical(e) => write!(f, "frame physical: {e:?}"),
            Self::Animation(e) => write!(f, "frame animation: {e:?}"),
            Self::Pose(e) => e.fmt(f),
            Self::Output(e) => write!(f, "frame PhysOut: {e:?}"),
            Self::Settings(e) => e.fmt(f),
            Self::Identity(e) => e.fmt(f),
            Self::Missing(s) => write!(f, "frame source missing {s}"),
            Self::Bone(i) => write!(f, "frame local bone {i} missing"),
            Self::Lock => f.write_str("frame owner poisoned"),
        }
    }
}
/// Caller supplies the SAME services retained by selected player construction.
/// This adapter does not create a backend, animation owner, or second pose.
pub(crate) struct ActorPresentationFrameSource<'a> {
    pub(crate) actor: &'a ActorConstruction,
    pub(crate) services: &'a PlayerAttributeServices,
}
impl ActorPresentationFrameSource<'_> {
    pub(crate) fn player_id(&self) -> Result<u32, PresentationFrameSourceError> {
        let prefix = self
            .actor
            .prefix()
            .map_err(PresentationFrameSourceError::Actor)?;
        Ok(prefix
            .fields()
            .ok_or(PresentationFrameSourceError::Missing("Actor fields"))?
            .player_index)
    }
    /// `SubmittableSkater::ShouldSubmit` 0x825f0008: slot8 or a nonempty backend
    /// row. `DoSimSubmit` 0x827dd3c0..3e8 subsequently excludes slot8 from the
    /// ordinary actor list (its playback skater is submitted separately).
    pub(crate) fn submit_in_actor_list(&self) -> Result<bool, PresentationFrameSourceError> {
        let player = self.player_id()?;
        if player == 8 { return Ok(false); }
        Ok(self.services.settings.try_player_type(player)
            .map_err(PresentationFrameSourceError::Settings)? != 0)
    }
    pub(crate) fn peer_id(&self) -> Result<i64, PresentationFrameSourceError> {
        let index = self.player_id()?;
        self.services
            .settings
            .try_peer_id(index)
            .map_err(PresentationFrameSourceError::Settings)
    }
    pub(crate) fn is_local(&self) -> Result<bool, PresentationFrameSourceError> {
        use PresentationFrameSourceError as E;
        let player = i32::from_ne_bytes(self.player_id()?.to_ne_bytes());
        if player
            == self
                .services
                .skaters
                .try_local_index()
                .map_err(E::Identity)?
        {
            return Ok(true);
        }
        Ok(player
            == self
                .services
                .skaters
                .try_local_sim_index()
                .map_err(E::Identity)?)
    }
    pub(crate) fn is_collidable(&self) -> Result<bool, PresentationFrameSourceError> {
        use PresentationFrameSourceError as E;
        if self
            .services
            .skaters
            .try_ghost_is_null()
            .map_err(E::Identity)?
        {
            Ok(true)
        } else {
            Err(E::Missing("actual Ghost::IsActorCollideable"))
        }
    }
    pub(crate) fn opacity(&self) -> Result<f32, PresentationFrameSourceError> {
        use PresentationFrameSourceError as E;
        let animation = self.actor.checked_body_animation().map_err(E::Animation)?;
        let animation = animation.lock().map_err(|_| E::Lock)?;
        Ok(animation
            .player()
            .ok_or(E::Missing("SkaterAnim player"))?
            .opacity)
    }
    pub(crate) fn opacity_main_camera_only(&self) -> Result<bool, PresentationFrameSourceError> {
        use PresentationFrameSourceError as E;
        let animation = self.actor.checked_body_animation().map_err(E::Animation)?;
        let animation = animation.lock().map_err(|_| E::Lock)?;
        let fields = animation.player().ok_or(E::Missing("SkaterAnim player"))?;
        if fields.flags_known_mask & 0x0400_0000 == 0 {
            return Err(E::Missing("opacity camera flag"));
        }
        Ok(fields.flags_bits & 0x0400_0000 != 0)
    }
    pub(crate) fn was_teleported(&self) -> Result<bool, PresentationFrameSourceError> {
        use PresentationFrameSourceError as E;
        let physical = self
            .actor
            .cached_physical_allocation()
            .map_err(E::Physical)?;
        let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
        Ok(output
            .try_state_output()
            .map_err(E::Output)?
            .has_just_teleported)
    }
    pub(crate) fn board_speed(&self) -> Result<f32, PresentationFrameSourceError> {
        use PresentationFrameSourceError as E;
        let physical = self
            .actor
            .cached_physical_allocation()
            .map_err(E::Physical)?;
        let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
        output.try_presentation_board_speed().map_err(E::Output)
    }
    pub(crate) fn root_matrix(&self) -> Result<SkeletonRootMatrix, PresentationFrameSourceError> {
        use PresentationFrameSourceError as E;
        let physical = self
            .actor
            .cached_physical_allocation()
            .map_err(E::Physical)?;
        let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
        Ok(output
            .try_skeleton_output()
            .map_err(E::Output)?
            .skeleton_root_matrix)
    }
    /// This is the separate header `GetLocalPose827dc3c0`, before `WriteSkater`
    /// calls its own local/face/right/left getters. No capture is reused early.
    pub(crate) fn header_local_pose_owner(
        &self,
    ) -> Result<
        std::sync::Arc<std::sync::Mutex<super::anim_out_phys_in::AnimOutPhysInOwner>>,
        PresentationFrameSourceError,
    > {
        use PresentationFrameSourceError as E;
        let physical = self
            .actor
            .cached_physical_allocation()
            .map_err(E::Physical)?;
        let input = physical
            .owner()
            .animation_physics_input
            .lock()
            .map_err(|_| E::Lock)?
            .as_ref()
            .cloned()
            .ok_or(E::Missing("AnimOutPhysIn"))?;
        Ok(input)
    }
    pub(crate) fn character_damage(&self) -> Result<[u8; 14], PresentationFrameSourceError> {
        use PresentationFrameSourceError as E;
        let physical = self
            .actor
            .cached_physical_allocation()
            .map_err(E::Physical)?;
        let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
        output
            .try_presentation_character_damage()
            .map_err(E::Output)
    }
    pub(crate) fn is_wiping_out(&self) -> Result<bool, PresentationFrameSourceError> {
        use PresentationFrameSourceError as E;
        let physical = self
            .actor
            .cached_physical_allocation()
            .map_err(E::Physical)?;
        let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
        Ok(output.try_state_output().map_err(E::Output)?.is_wiping_out)
    }
    pub(crate) fn time_in_state(&self) -> Result<i32, PresentationFrameSourceError> {
        use PresentationFrameSourceError as E;
        let physical = self
            .actor
            .cached_physical_allocation()
            .map_err(E::Physical)?;
        let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
        Ok(output
            .try_state_output()
            .map_err(E::Output)?
            .state_update_count)
    }
}
