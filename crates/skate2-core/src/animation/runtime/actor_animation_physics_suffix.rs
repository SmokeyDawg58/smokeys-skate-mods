//! Remaining `SetUpPhysics825ff7f4..978` copies into the same retained input.
use super::{ActorConstruction, AnimationPhysicsPublicationError};
use crate::{
    actor_construction::anim_out_phys_in::AnimationAiInput,
    players_backend_settings::{PhysicsSetting, PlayersBackendSettings},
};

impl AnimationAiInput {
    /// `AIPhysicsInput.operator=825f0f60`: ten vectors then seven bit stores.
    pub(crate) fn assign_source(&mut self, source: &Self) {
        for column in 0..4 {
            self.desired_deck_transform[column * 4..column * 4 + 4]
                .copy_from_slice(&source.desired_deck_transform[column * 4..column * 4 + 4]);
        }
        self.desired_deck_velocity = source.desired_deck_velocity;
        self.trajectory_position = source.trajectory_position;
        self.trajectory_velocity = source.trajectory_velocity;
        self.trajectory_gravity = source.trajectory_gravity;
        self.trajectory_time = source.trajectory_time;
        self.trajectory_offset = source.trajectory_offset;
        for mask in [
            0x8000_0000,
            0x4000_0000,
            0x2000_0000,
            0x1000_0000,
            0x0800_0000,
            0x0400_0000,
            0x0200_0000,
        ] {
            self.flags = (self.flags & !mask) | (source.flags & mask);
        }
    }
}

impl ActorConstruction {
    /// Source suffix after animation/Motion attributes; settings are sampled
    /// from the retained player backend in enum order, never cached per Actor.
    pub(crate) fn try_finish_animation_physics_outputs(
        &self,
        settings: &PlayersBackendSettings,
    ) -> Result<(), AnimationPhysicsPublicationError> {
        use AnimationPhysicsPublicationError as E;
        let input = self.published_animation_physics_input()?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let mut actor = self.prefix().map_err(E::Actor)?;
        let mut output = input.lock().map_err(|_| E::Poisoned)?;
        let flags = actor
            .input_generation_flags
            .ok_or(E::Missing("Actor flags+258"))?;
        output.flags.living_world_impulse_active = flags & 0x4000_0000 != 0;
        let auxiliary = actor
            .physics_auxiliary
            .as_ref()
            .ok_or(E::Missing("Actor physics auxiliaries"))?;
        output.living_world_impulse = auxiliary.living_world_impulse;
        output.ai.assign_source(&auxiliary.ai);
        output.flags.ai_input_active = flags & 0x0800_0000 != 0;
        output.flags.under_ai_control = auxiliary.ai_controller.is_some();
        let external = actor
            .external_player_reset
            .as_ref()
            .ok_or(E::Missing("Actor external reset"))?;
        for column in 0..4 {
            output
                .external_reset_transform
                .set_column(column, external.transform.columns()[column]);
        }
        output.external_reset_on_board = external.on_board;
        output.external_reset_active = flags & 0x2000_0000 != 0;
        output.flags.prevent_manual_respawn = flags & 0x1000_0000 != 0;
        output.flags.force_braking = flags & 0x0400_0000 != 0;
        output.locomotion_flags =
            (output.locomotion_flags & !0x0100_0000) | ((flags >> 1) & 0x0100_0000);
        actor.input_generation_flags = Some(flags & !0x0200_0000);
        output.flags.ignore_respawn_reset_button = physical
            .owner()
            .world_services()
            .ok_or(E::Missing("physical world services"))?
            .try_ignore_wipeout_reset_button()
            .map_err(E::World)?;
        let under_ai_control = actor
            .physics_auxiliary
            .as_ref()
            .expect("retained auxiliaries")
            .ai_controller
            .is_some();
        actor.input_generation_flags = Some(flags & !0x1200_0000);
        if under_ai_control {
            output.truck_tightness = f32::from_bits(0x3e99_999a);
            output.wheel_hardness = f32::from_bits(0x3f33_3333);
        } else {
            let player = actor
                .fields
                .as_ref()
                .ok_or(E::Missing("Actor player index"))?
                .player_index;
            output.truck_tightness = settings
                .try_get(player, PhysicsSetting::TruckTightness)
                .map_err(E::Settings)?;
            output.wheel_hardness = settings
                .try_get(player, PhysicsSetting::WheelHardness)
                .map_err(E::Settings)?;
        }
        drop(output);
        drop(actor);
        self.try_bind_animation_physics_input()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ai_assignment_preserves_destination_residual_bits_and_all_source_lanes() {
        let source = AnimationAiInput {
            flags: 0xa400_0123,
            trajectory_time: [-1.0; 4],
            trajectory_offset: [f32::from_bits(0x8000_0000); 4],
            ..Default::default()
        };
        let mut destination = AnimationAiInput {
            flags: 0x5a00_4567,
            ..Default::default()
        };
        destination.assign_source(&source);
        assert_eq!(destination.flags, 0xa400_4567);
        assert_eq!(
            destination.trajectory_time.map(f32::to_bits),
            [0xbf80_0000; 4]
        );
        assert_eq!(
            destination.trajectory_offset.map(f32::to_bits),
            [0x8000_0000; 4]
        );
    }
}
