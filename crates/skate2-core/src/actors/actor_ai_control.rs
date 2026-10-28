//! The Actor side of `AI::AIController`: the frame view an AI reads through `Actor::GetPhysOut`,
//! the deck and skeleton getters and `ISkaterAnim`, and the Actor calls the AI makes.
use super::{ActorConstruction, ActorConstructionError, anim_out_phys_in::AnimationAiInput};
use crate::{
    actor_graph_construction::ActorGraphIntents,
    ai::actor::{AiActorCommand, AiActorView, AiClientType, AiPhysicsInput, flags},
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::PhysicalAssemblyWorld,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::Arc;

/// `SkaterAnim+0xe4c`'s `ISkaterAnim::GetIsRidingFakie` 0x82cb7530 and `GetIsMirrored`
/// 0x82cb7598 bits.
const RIDING_FAKIE: u32 = 0x2000_0000;
const MIRRORED: u32 = 0x4000_0000;

#[derive(Debug)]
pub(crate) enum ActorAiError {
    Actor(ActorConstructionError),
    Missing(&'static str),
    Poisoned,
    Physical(String),
    PhysOut(String),
    Animation(&'static str),
    Teleport(super::teleport::ActorTeleportError),
    World(String),
}

impl std::fmt::Display for ActorAiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Actor AI: {self:?}")
    }
}

fn phys_out_error(error: impl std::fmt::Debug) -> ActorAiError {
    ActorAiError::PhysOut(format!("{error:?}"))
}

fn world_error(error: impl std::fmt::Debug) -> ActorAiError {
    ActorAiError::World(format!("{error:?}"))
}

fn lanes(vector: SkeletonRootVector) -> [f32; 4] {
    [vector.xyz[0], vector.xyz[1], vector.xyz[2], vector.w.unwrap_or(0.0)]
}

fn rows(matrix: &SkeletonRootMatrix) -> [[f32; 4]; 4] {
    std::array::from_fn(|row| lanes(matrix.columns()[row]))
}

/// `AIPhysicsInput::operator=` 0x825f0f60: every vector, and the flag word's top seven bits.
fn assign_ai_physics_input(target: &mut AnimationAiInput, input: &AiPhysicsInput) {
    target.desired_deck_transform = std::array::from_fn(|index| input.desired_deck_transform[index / 4][index % 4]);
    target.desired_deck_velocity = input.desired_deck_velocity;
    target.trajectory_position = input.trajectory.p0;
    target.trajectory_velocity = input.trajectory.v0;
    target.trajectory_gravity = input.trajectory.g;
    target.trajectory_time = input.trajectory.t;
    target.trajectory_offset = input.trajectory_offset;
    target.flags = (target.flags & 0x01ff_ffff) | (input.flags & 0xfe00_0000);
}

impl ActorConstruction {
    /// `SkaterManager::CreateAiController` 0x825f40b8: the Actor's `mAIController`, then
    /// `SkaterAnim::EnableAccelDecel(false)`.
    pub(crate) fn try_attach_ai_controller(&self, client_type: AiClientType) -> Result<(), ActorAiError> {
        self.prefix()
            .map_err(ActorAiError::Actor)?
            .physics_auxiliary
            .as_mut()
            .ok_or(ActorAiError::Missing("physics auxiliaries"))?
            .ai_controller = Some(Arc::new(client_type));
        self.published_camera_animation()
            .map_err(ActorAiError::Actor)?
            .ok_or(ActorAiError::Missing("SkaterAnim"))?
            .lock()
            .map_err(|_| ActorAiError::Poisoned)?
            .try_enable_accel_decel(false)
            .map_err(ActorAiError::Animation)
    }

    /// The map `AIController::GenerateActionGraphIntents` filled for this frame's
    /// `Actor::GenerateActionGraphIntents`.
    pub(crate) fn try_publish_ai_intents(&self, intents: ActorGraphIntents) -> Result<(), ActorAiError> {
        self.prefix()
            .map_err(ActorAiError::Actor)?
            .physics_auxiliary
            .as_mut()
            .ok_or(ActorAiError::Missing("physics auxiliaries"))?
            .ai_intents = Some(intents);
        Ok(())
    }

    /// `IActor::GetPlayerID`: Actor+0x1d8.
    pub(crate) fn try_ai_player_id(&self) -> Result<i32, ActorAiError> {
        let state = self.prefix().map_err(ActorAiError::Actor)?;
        let fields = state.fields().ok_or(ActorAiError::Missing("Actor fields"))?;
        Ok(i32::from_ne_bytes(fields.player_index.to_ne_bytes()))
    }

    /// `Actor::GetPosition` 0x825f9df0: the published skeleton root's translation.
    pub(crate) fn try_ai_position(&self) -> Result<[f32; 4], ActorAiError> {
        let physical = self.cached_physical_allocation().map_err(|error| ActorAiError::Physical(format!("{error:?}")))?;
        let output = physical.owner().phys_out().lock().map_err(|_| ActorAiError::Poisoned)?;
        Ok(lanes(output.try_skeleton_output().map_err(phys_out_error)?.skeleton_root_matrix.columns()[3]))
    }

    /// The teleport matrix's position (`Actor+0x21c`) while the Actor's external-reset flag
    /// (`+600` bit 29) waits for it.
    pub(crate) fn try_ai_teleport_destination(&self) -> Result<Option<[f32; 4]>, ActorAiError> {
        let state = self.prefix().map_err(ActorAiError::Actor)?;
        let flags = state.input_generation_flags.ok_or(ActorAiError::Missing("Actor flags+258"))?;
        if flags & flags::EXTERNAL_RESET == 0 {
            return Ok(None);
        }
        Ok(state.external_player_reset.as_ref().map(|reset| lanes(reset.transform.columns()[3])))
    }

    /// `Actor::GetOpacity`: `SkaterAnim::GetOpacity`.
    pub(crate) fn try_ai_opacity(&self) -> Result<f32, ActorAiError> {
        Ok(self
            .published_camera_animation()
            .map_err(ActorAiError::Actor)?
            .ok_or(ActorAiError::Missing("SkaterAnim"))?
            .lock()
            .map_err(|_| ActorAiError::Poisoned)?
            .player()
            .ok_or(ActorAiError::Missing("SkaterAnim player fields"))?
            .opacity)
    }

    /// What an `AIController` reads of its Actor this frame: the last physics step's `PhysOut`,
    /// `Actor+0x258`, and the `ISkaterAnim` stance comparison `GeneratePushIntents` 0x824d1aa0
    /// makes (mirrored against riding fakie, false without the interface).
    pub(crate) fn try_ai_actor_view(&self, fade_out_time: f32) -> Result<AiActorView, ActorAiError> {
        use ActorAiError as E;
        let (player_id, actor_flags) = {
            let state = self.prefix().map_err(E::Actor)?;
            let fields = state.fields().ok_or(E::Missing("Actor fields"))?;
            let actor_flags = state.input_generation_flags.ok_or(E::Missing("Actor flags+258"))?;
            (i32::from_ne_bytes(fields.player_index.to_ne_bytes()), actor_flags)
        };
        let teleport_destination = self.try_ai_teleport_destination()?;
        let push_right = match self.published_camera_animation().map_err(E::Actor)? {
            Some(animation) => animation.lock().map_err(|_| E::Poisoned)?.player().is_some_and(|player| {
                (player.flags_bits & MIRRORED != 0) != (player.flags_bits & RIDING_FAKIE != 0)
            }),
            None => false,
        };
        let physical = self.cached_physical_allocation().map_err(|error| E::Physical(format!("{error:?}")))?;
        let output = physical.owner().phys_out().lock().map_err(|_| E::Poisoned)?;
        let state = output.try_state_output().map_err(phys_out_error)?;
        let animation = output.try_animation_output().map_err(phys_out_error)?;
        let air = output.try_air_output().map_err(phys_out_error)?;
        let offboard = output.try_offboard_output().map_err(phys_out_error)?;
        let ground = output.try_ground_output().map_err(phys_out_error)?;
        let skeleton = output.try_skeleton_output().map_err(phys_out_error)?;
        let board = output.try_camera_board_output().map_err(phys_out_error)?;
        let (_, velocity) = output.try_system_centre_and_damped_velocity().map_err(phys_out_error)?;
        let (hippy_jumping, in_state_air, air_spin_180_count, active_trick_scorables) =
            output.try_ai_scoring_view().map_err(phys_out_error)?;
        let (just_wiped_out, just_respawned, banked_point_change_this_frame) =
            output.try_observed_events().map_err(phys_out_error)?;
        Ok(AiActorView {
            player_id,
            teleport_destination,
            flags: actor_flags,
            physics_state_category: state.physics_state_category,
            physics_state: state.physics_state,
            is_wiping_out: state.is_wiping_out,
            is_teleport_finished: state.is_teleport_finished,
            collide_with_world_only: state.collide_with_world_only,
            just_wiped_out,
            just_respawned,
            banked_point_change_this_frame,
            riding_fakie: animation.is_riding_fakie,
            transitioning_on_off_board: animation.transitioning_on_off_board,
            skater_off_board: animation.skater_off_board,
            running_out: animation.running_out,
            skateboard_airborne: air.is_skateboard_airborne,
            mounting: offboard.is_mounting,
            unmounting: offboard.is_unmounting,
            below_hips_ground_valid: ground.below_hips_ground_data_is_valid,
            effective_anim_z: lanes(skeleton.effective_anim_z),
            skeleton_root: rows(&skeleton.skeleton_root_matrix),
            hips_position: lanes(skeleton.skeleton_hips_pos),
            hips_velocity: lanes(skeleton.skeleton_hips_vel),
            hips_look: lanes(skeleton.skeleton_hips_look),
            time_until_wipeout_teleport: skeleton.time_until_wipeout_teleport,
            wipeout_teleport_pending: skeleton.is_wipeout_teleport_pending,
            deck_position: lanes(board.deck_position),
            hippy_jumping,
            in_state_air,
            air_spin_180_count,
            active_trick_scorables,
            effective_deck_transform: rows(&board.effective_transform),
            deck_transform: rows(&board.transform),
            transform: rows(&skeleton.effective_skeleton_root_matrix),
            velocity: lanes(velocity),
            push_right,
            fade_out_time,
        })
    }

    /// One Actor call an AI made, in its call order.
    pub(crate) fn try_apply_ai_command(
        &self,
        command: &AiActorCommand,
        world: &mut PhysicalAssemblyWorld,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        fade_out_time: f32,
    ) -> Result<(), ActorAiError> {
        use ActorAiError as E;
        match command {
            AiActorCommand::Teleport(matrix) => {
                let matrix = SkeletonRootMatrix::known(std::array::from_fn(|index| matrix[index / 4][index % 4]));
                self.try_publish_teleport(&matrix, true).map_err(E::Teleport)
            }
            // `Actor::SetVelocityUnsafe` 0x825ed388: `Skateboard::SetVelocity`.
            AiActorCommand::SetVelocity(velocity) => world
                .try_set_board_velocity(board, &SkeletonRootVector { xyz: [velocity[0], velocity[1], velocity[2]], w: None })
                .map_err(world_error),
            // `Actor::ApplyAIPhysicsInput` 0x825f4168.
            AiActorCommand::ApplyPhysicsInput(input) => {
                let mut state = self.prefix().map_err(E::Actor)?;
                assign_ai_physics_input(
                    &mut state.physics_auxiliary.as_mut().ok_or(E::Missing("physics auxiliaries"))?.ai,
                    input,
                );
                *state.input_generation_flags.as_mut().ok_or(E::Missing("Actor flags+258"))? |= flags::AI_INPUT;
                Ok(())
            }
            // `Actor::SetFadingOut` 0x825ed848.
            AiActorCommand::SetFadingOut(fade) => {
                let mut state = self.prefix().map_err(E::Actor)?;
                let fields = state.fields_mut().ok_or(E::Missing("Actor fields"))?;
                if !fade {
                    fields.time_since_fade_out = f32::MAX;
                } else if fields.time_since_fade_out > fade_out_time {
                    fields.time_since_fade_out = 0.0;
                }
                Ok(())
            }
            // `Actor::DisableNonWorldCollisionsForFadeout` 0x825ed3d0.
            AiActorCommand::DisableNonWorldCollisionsForFadeout => {
                let physical =
                    self.cached_physical_allocation().map_err(|error| E::Physical(format!("{error:?}")))?;
                world
                    .try_disable_non_world_collisions_for_fadeout(physical.owner(), board, skeleton)
                    .map_err(E::World)
            }
            AiActorCommand::ClearPin => {
                *self
                    .prefix()
                    .map_err(E::Actor)?
                    .input_generation_flags
                    .as_mut()
                    .ok_or(E::Missing("Actor flags+258"))? &= !flags::PINNED;
                Ok(())
            }
        }
    }
}
