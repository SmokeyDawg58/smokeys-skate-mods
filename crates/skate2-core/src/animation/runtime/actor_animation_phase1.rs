//! Actual Actor Phase1 opacity and holding prefix, before locomotion/graphs.
use super::{
    ActorConstruction, ActorConstructionError, body_session::BodySessionError,
    physical_mount::ActorPhysicalMountError,
};
use crate::{
    actor_animation_construction::frame_phase::AnimationPhaseError,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_phys_out::construct::ConcretePhysOutError,
};

#[derive(Debug)]
pub(crate) enum ActorAnimationPhase1Error {
    Actor(ActorConstructionError),
    Body(BodySessionError),
    Physical(ActorPhysicalMountError),
    Output(ConcretePhysOutError<std::convert::Infallible>),
    Animation(AnimationPhaseError),
    Cvar(InputCvarError),
    Missing(&'static str),
    Lock,
    TimeNan,
    /// The `Simulation::GetCameraManager` owner read by the final opacity branch.
    Camera(String),
    Locomotion(crate::actor_animation_construction::locomotion_update::LocomotionUpdateError),
    WalkingWorld(
        crate::actor_construction::motion_runtime_session::neutral_lifecycle::WalkingWorldError,
    ),
    Dispatch(
        crate::actor_animation_construction::initialization::body_dispatch::BodyDispatchError<
            BodySessionError,
        >,
    ),
}
impl std::fmt::Display for ActorAnimationPhase1Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => write!(f, "animation phase1 Actor: {error}"),
            Self::Body(error) => write!(f, "animation phase1 owner: {error}"),
            Self::Physical(error) => write!(f, "animation phase1 physical owner: {error}"),
            Self::Output(error) => write!(f, "animation phase1 output: {error:?}"),
            Self::Animation(error) => write!(f, "animation phase1: {error}"),
            Self::Cvar(error) => write!(f, "animation phase1 cvar: {error:?}"),
            Self::Missing(owner) => write!(f, "animation phase1 requires {owner}"),
            other => write!(f, "animation phase1: {other:?}"),
        }
    }
}

/// `Simulation::GetCameraManager` as read by UpdateSkaterOpacity825f1368..13e0.
pub(crate) trait SkaterOpacityCamera {
    /// `GetCameraSkater82581bf8` (GameCameraManager+0xd0) compared with this
    /// Actor, then `GetCameraSkaterOpacity82e34960`; `None` is another skater.
    fn camera_skater_opacity(&self, actor: &ActorConstruction) -> Result<Option<f32>, String>;
}

impl ActorConstruction {
    /// `Actor::UpdatePhase1`825f41a8..42c0. Success reaches the nonlocomotion
    /// branch's actual Action/Motion calls; the outer scheduler executes them.
    pub(crate) fn try_update_animation_phase1_prefix(
        &self,
        dt: f32,
        cvars: &InputCvarReferences,
        body: &mut super::body_session::ActorBodyAnimationServices,
        world: &mut dyn super::motion_runtime_session::neutral_lifecycle::ForcePhysicsWriter,
        camera: &dyn SkaterOpacityCamera,
    ) -> Result<(), ActorAnimationPhase1Error> {
        use ActorAnimationPhase1Error as E;
        // VMX equality-with-self825f41e0..1f8 rejects NaN, accepts infinity.
        if dt.is_nan() {
            return Err(E::TimeNan);
        }
        self.try_update_skater_opacity(dt, cvars, camera)?;
        // Native calls GetPhysOut twice. Do not combine into an early snapshot.
        let offboard = self
            .cached_physical_allocation()
            .map_err(E::Physical)?
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_state_output()
            .map_err(E::Output)?
            .is_offboard;
        let holding = self
            .cached_physical_allocation()
            .map_err(E::Physical)?
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_offboard_output()
            .map_err(E::Output)?
            .is_holding_skateboard;
        let animation = self.checked_body_animation().map_err(E::Body)?;
        let locomotion = animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_update_phase1_holding(offboard, holding)
            .map_err(E::Animation)?;
        if locomotion {
            let overlay = world.walking_debug_overlay().map_err(E::WalkingWorld)?;
            let mut animation = animation.lock().map_err(|_| E::Lock)?;
            animation
                .try_update_locomotion_trajectory(
                    dt,
                    cvars,
                    &overlay,
                    &mut crate::reckoning_numeric::HostReckoningNumeric,
                    &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
                )
                .map_err(E::Locomotion)?;
            animation
                .try_add_animation_time(dt)
                .map_err(|error| E::Animation(AnimationPhaseError::Time(error)))?;
            animation
                .try_process_tree_attributes()
                .map_err(E::Animation)?;
            animation
                .try_dispatch_body_animation(body)
                .map_err(E::Dispatch)?;
        }
        Ok(())
    }

    /// `Actor::UpdateSkaterOpacity825f1240`. The fade branch requires no renderer:
    /// `SetOpacity82cb75d0` writes the retained scalar and main-camera flag only.
    fn try_update_skater_opacity(
        &self,
        dt: f32,
        cvars: &InputCvarReferences,
        camera: &dyn SkaterOpacityCamera,
    ) -> Result<(), ActorAnimationPhase1Error> {
        use ActorAnimationPhase1Error as E;
        let elapsed = self
            .prefix()
            .map_err(E::Actor)?
            .fields
            .as_ref()
            .ok_or(E::Missing("Actor fade fields"))?
            .time_since_fade_in;
        let fade_in = cvars.try_skater_fade_in_time().map_err(E::Cvar)?;
        let (opacity, main_camera_only) = if elapsed < fade_in {
            let collide_world_only = self
                .cached_physical_allocation()
                .map_err(E::Physical)?
                .owner()
                .phys_out()
                .lock()
                .map_err(|_| E::Lock)?
                .try_state_output()
                .map_err(E::Output)?
                .collide_with_world_only;
            if collide_world_only {
                let mut state = self.prefix().map_err(E::Actor)?;
                let fields = state
                    .fields
                    .as_mut()
                    .ok_or(E::Missing("Actor fade fields"))?;
                if fields.time_since_fade_in > fade_in * 0.5 {
                    fields.time_since_fade_in -= dt;
                }
            }
            // Native re-loads this cvar target after the optional timer store.
            let elapsed = self
                .prefix()
                .map_err(E::Actor)?
                .fields
                .as_ref()
                .ok_or(E::Missing("Actor fade fields"))?
                .time_since_fade_in;
            let fade_in = cvars.try_skater_fade_in_time().map_err(E::Cvar)?;
            (clamp_opacity(elapsed / fade_in), false)
        } else {
            let elapsed = self
                .prefix()
                .map_err(E::Actor)?
                .fields
                .as_ref()
                .ok_or(E::Missing("Actor fade fields"))?
                .time_since_fade_out;
            let fade_out = cvars.try_skater_fade_out_time().map_err(E::Cvar)?;
            if elapsed < fade_out {
                (1.0 - clamp_opacity(elapsed / fade_out), false)
            } else {
                // Source1368..13e0: Simulation::GetCameraManager, then the camera
                // skater takes GetCameraSkaterOpacity with the main-camera-only
                // flag set; any other camera skater gets SetOpacity(1.0, false).
                match camera.camera_skater_opacity(self).map_err(E::Camera)? {
                    Some(opacity) => (opacity, true),
                    None => (1.0, false),
                }
            }
        };
        let animation = self.checked_body_animation().map_err(E::Body)?;
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_set_animation_opacity(opacity, main_camera_only)
            .map_err(E::Animation)?;
        // Only successful SetOpacity reaches both original timer increments.
        let mut state = self.prefix().map_err(E::Actor)?;
        let fields = state
            .fields
            .as_mut()
            .ok_or(E::Missing("Actor fade fields"))?;
        let next_in = fields.time_since_fade_in + dt;
        let next_out = fields.time_since_fade_out + dt;
        fields.time_since_fade_in = next_in;
        fields.time_since_fade_out = next_out;
        Ok(())
    }
}

/// Two source FSELs825f12f8/1300; NaN and signed zero differ from Rust clamp.
fn clamp_opacity(value: f32) -> f32 {
    let lower = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::clamp_opacity;

    #[test]
    fn opacity_clamp_uses_source_fsel_for_nan_signed_zero_and_endpoints() {
        for (value, expected) in [
            (f32::NAN, 1.0_f32),
            (-0.0, 0.0),
            (-1.0, 0.0),
            (0.25, 0.25),
            (2.0, 1.0),
        ] {
            assert_eq!(clamp_opacity(value).to_bits(), expected.to_bits());
        }
    }
}
