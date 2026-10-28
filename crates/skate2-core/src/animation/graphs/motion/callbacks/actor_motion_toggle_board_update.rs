//! ToggleBoard::Update82cc2c08 on the retained Actor interfaces.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError};
use crate::{
    actor_animation_construction::channel_runtime::{ChannelParams, transition::ChannelStartFlags},
    actor_construction::animation_mount::MountedActorAnimation,
    actor_graph_construction::ActorGraphIntents,
    generated_motion_load::{
        MotionLoadError, construct::ToggleBoardNames, controller::ToggleBoardInstance,
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    play_animation::{FastString30, FastString36},
};
use std::sync::{Arc, Mutex, MutexGuard};
#[path = "motion_toggle_board.rs"]
pub(crate) mod kernel;
use kernel::{ToggleBoardCvar, ToggleBoardSource};

/// The seven Update interfaces resolve to these retained owners; the drop and
/// retrieval states re-resolve the same interfaces.
struct Source<'a> {
    actor: &'a ActorConstruction,
    motion: Arc<()>,
    animation: Arc<Mutex<MountedActorAnimation>>,
    physical: Arc<PhysicalPlayerAssemblyOwner>,
    cvars: Option<&'a InputCvarReferences>,
    channel: FastString36,
}
impl Source<'_> {
    fn animation(&self) -> Result<MutexGuard<'_, MountedActorAnimation>, MotionUpdateError> {
        self.animation.lock().map_err(|_| MotionUpdateError::Lock)
    }
    fn cvars(&self) -> Result<&InputCvarReferences, MotionUpdateError> {
        self.cvars
            .ok_or(MotionUpdateError::MissingOwner("ToggleBoard cvars"))
    }
}
impl ToggleBoardSource for Source<'_> {
    type Error = MotionUpdateError;
    fn holding(&mut self) -> Result<bool, Self::Error> {
        use MotionUpdateError as E;
        let output = self.physical.phys_out();
        let output = output.lock().map_err(|_| E::Lock)?;
        Ok(output
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .is_holding_skateboard)
    }
    fn yaw_to_free_board(&mut self) -> Result<f32, Self::Error> {
        use MotionUpdateError as E;
        let output = self.physical.phys_out();
        let output = output.lock().map_err(|_| E::Lock)?;
        Ok(output
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .yaw_to_free_board)
    }
    fn pitch_to_free_board(&mut self) -> Result<f32, Self::Error> {
        use MotionUpdateError as E;
        let output = self.physical.phys_out();
        let output = output.lock().map_err(|_| E::Lock)?;
        Ok(output
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .pitch_to_free_board)
    }
    fn mirrored(&mut self) -> Result<bool, Self::Error> {
        let animation = self.animation()?;
        let player = animation.player().ok_or(MotionUpdateError::MissingOwner(
            "ToggleBoard SkaterAnim player",
        ))?;
        if player.flags_known_mask & 0x4000_0000 == 0 {
            return Err(MotionUpdateError::MissingOwner("ToggleBoard mirrored flag"));
        }
        Ok(player.flags_bits & 0x4000_0000 != 0)
    }
    fn narrow(&mut self, name: FastString36) -> Result<FastString30, Self::Error> {
        let text = name
            .decode()
            .map_err(|_| MotionUpdateError::Load(MotionLoadError::PackedString("ToggleBoard")))?;
        FastString30::encode(&text)
            .map_err(|_| MotionUpdateError::Load(MotionLoadError::PackedString("ToggleBoard")))
    }
    fn motion_attribute(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error> {
        use MotionUpdateError as E;
        let mut actor = self.actor.prefix().map_err(E::Actor)?;
        let motion = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .ok_or(E::MissingOwner("ToggleBoard captured Motion"))?;
        if !Arc::ptr_eq(&self.motion, motion.occurrence()) {
            return Err(E::ForeignOwner("ToggleBoard captured Motion"));
        }
        motion
            .state
            .as_mut()
            .ok_or(E::MissingOwner("ToggleBoard Motion state"))?
            .try_append_motion_attribute(name, value)
    }
    fn settable(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error> {
        self.animation()?
            .try_add_gesture_attribute(name, value, false, -1)
            .map_err(MotionUpdateError::AnimationWrite)
    }
    fn has_channel(&mut self) -> Result<bool, Self::Error> {
        self.animation()?
            .has_channel(&self.channel)
            .map_err(MotionUpdateError::AnimationChannel)
    }
    fn time_left(&mut self) -> Result<f32, Self::Error> {
        self.animation()?
            .time_left_in_channel(&self.channel)
            .map_err(MotionUpdateError::AnimationChannel)
    }
    fn time_elapsed(&mut self) -> Result<f32, Self::Error> {
        self.animation()?
            .time_elapsed_in_channel(&self.channel)
            .map_err(MotionUpdateError::AnimationChannel)
    }
    fn transition_to(
        &mut self,
        animation: FastString36,
        length: f32,
        params: ChannelParams,
        flags: ChannelStartFlags,
    ) -> Result<(), Self::Error> {
        let channel = self.channel;
        self.animation()?
            .try_transition_channel_to(channel, animation, length, params, flags)
            .map(|_| ())
            .map_err(MotionUpdateError::AnimationChannel)
    }
    fn sequence_to(
        &mut self,
        animation: FastString36,
        params: ChannelParams,
        flags: ChannelStartFlags,
    ) -> Result<(), Self::Error> {
        let channel = self.channel;
        self.animation()?
            .try_sequence_channel_to(channel, animation, params, flags)
            .map(|_| ())
            .map_err(MotionUpdateError::AnimationChannel)
    }
    fn end_channel(&mut self, length: f32) -> Result<(), Self::Error> {
        let channel = self.channel;
        self.animation()?
            .try_end_channel_with_blend(&channel, length, false)
            .map_err(MotionUpdateError::AnimationWrite)
    }
    fn cvar(&mut self, cvar: ToggleBoardCvar) -> Result<f32, Self::Error> {
        let cvars = self.cvars()?;
        let value = match cvar {
            ToggleBoardCvar::YawChange => cvars.try_max_retrieve_board_yaw_change(),
            ToggleBoardCvar::BlendFrames => cvars.try_max_retrieve_board_blend_frames(),
            ToggleBoardCvar::BlendOutFrames => cvars.try_max_retrieve_board_blend_out_frames(),
            ToggleBoardCvar::RetrievePlayOutFrames => {
                cvars.try_max_retrieve_board_play_out_frames()
            }
            ToggleBoardCvar::DropPlayOutFrames => cvars.try_max_drop_board_play_out_frames(),
        };
        value.map_err(MotionUpdateError::Cvar)
    }
    fn unwritten(&mut self, field: &'static str) -> Self::Error {
        MotionUpdateError::MissingOwner(field)
    }
    fn assertion(&mut self, site: &'static str) -> Self::Error {
        MotionUpdateError::MissingOwner(site)
    }
}

impl ActorConstruction {
    /// IsRetrievingSkateboard82c94ca8 / IsDroppingSkateboard82c94d18.
    /// Both return false when Entity has no registered IPhysical.
    pub(crate) fn motion_runtime_board_operation_flag(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        retrieving: bool,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        match context.entity() {
            None => return Ok(false),
            Some(entity) if entity != &self.authority.entity_handle() => {
                return Err(E::ForeignOwner("board operation IPhysical entity"));
            }
            Some(_) => {}
        }
        let physical = self
            .prefix()
            .map_err(E::Actor)?
            .physical_interface
            .as_ref()
            .map(Arc::clone);
        let Some(physical) = physical else {
            return Ok(false);
        };
        let output = physical.owner().phys_out();
        let output = output.lock().map_err(|_| E::Lock)?;
        let offboard = output.try_offboard_output().map_err(E::PhysOut)?;
        Ok(if retrieving {
            offboard.is_retrieving_board
        } else {
            offboard.is_dropping_board
        })
    }

    /// Source checks all seven Actor interfaces before reading the Motion map
    /// or PhysOut, then gates the three board commands and dispatches.
    pub(crate) fn motion_runtime_toggle_board_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        data: &mut ToggleBoardInstance,
        names: &ToggleBoardNames,
        cvars: Option<&InputCvarReferences>,
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;

        // IPhysical: cached Actor+0x200 and registered Entity interface.
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let (motion_occurrence, animation) = {
            let actor = self.prefix().map_err(E::Actor)?;
            let retained = actor
                .animation
                .as_ref()
                .filter(|_| actor.animation_published)
                .ok_or(E::MissingOwner("ToggleBoard IAnimatable"))?;
            let animation = retained.lock().map_err(|_| E::Lock)?;
            if !animation
                .entity()
                .is_same_issuance(&self.authority.entity_handle())
            {
                return Err(E::ForeignOwner("ToggleBoard animation entity"));
            }
            // IAnimatable, IChannelAnimatable, ISkaterAnim in native order.
            for (index, name) in ["IAnimatable", "IChannelAnimatable", "ISkaterAnim"]
                .into_iter()
                .enumerate()
            {
                let registered = actor.animation_interfaces[index]
                    .as_ref()
                    .ok_or(E::MissingOwner(name))?;
                if !Arc::ptr_eq(registered, animation.occurrence()) {
                    return Err(E::ForeignOwner(name));
                }
            }
            drop(animation);
            // ISkaterMotionGraph, IMotionGraph, then IActor.
            let motion = actor
                .graphs
                .as_ref()
                .and_then(|graphs| graphs.motion())
                .filter(|motion| actor.motion_graph_published && motion.is_complete())
                .ok_or(E::MissingOwner("ToggleBoard ISkaterMotionGraph"))?;
            for (index, name) in [(2, "ISkaterMotionGraph"), (1, "IMotionGraph")] {
                let registered = actor.graph_interfaces[index]
                    .as_ref()
                    .ok_or(E::MissingOwner(name))?;
                if !Arc::ptr_eq(registered, motion.occurrence()) {
                    return Err(E::ForeignOwner(name));
                }
            }
            if context.entity() != Some(&self.authority.entity_handle()) {
                return Err(E::ForeignOwner("ToggleBoard IActor"));
            }
            let controller = motion
                .controller
                .as_ref()
                .ok_or(E::MissingOwner("ToggleBoard Motion controller"))?;
            if context.controller() != Some(controller.controller_identity()) {
                return Err(E::ForeignOwner("ToggleBoard Motion controller"));
            }
            // IMotionGraph::GetIntents precedes IPhysical::GetOutput.
            if motion.intents.is_none() {
                return Err(E::MissingOwner("ToggleBoard Motion intents"));
            }
            (Arc::clone(motion.occurrence()), Arc::clone(retained))
        };

        let grabbing = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .is_grabbing_object;
        // Native captures the Motion intent owner before GetOutput. Resolve
        // each lazy query through that retained occurrence without holding
        // the Actor lock across a later physical output read.
        let command = kernel::select_command::<E>(
            grabbing,
            |name| {
                let actor = self.prefix().map_err(E::Actor)?;
                let motion = actor
                    .graphs
                    .as_ref()
                    .and_then(|graphs| graphs.motion())
                    .ok_or(E::MissingOwner("ToggleBoard captured Motion graph"))?;
                if !Arc::ptr_eq(&motion_occurrence, motion.occurrence()) {
                    return Err(E::ForeignOwner("ToggleBoard captured Motion graph"));
                }
                let intents = motion
                    .intents
                    .as_ref()
                    .ok_or(E::MissingOwner("ToggleBoard captured Motion intents"))?;
                Ok(intents.get(&ActorGraphIntents::native_name(name)).is_some())
            },
            || {
                Ok(data
                    .state
                    .ok_or(E::MissingOwner("ToggleBoard instance state"))?
                    == 0)
            },
            || {
                // Each present idle command invokes IPhysical::GetOutput anew.
                let output = physical.owner().phys_out();
                let output = output.lock().map_err(|_| E::Lock)?;
                let offboard = output.try_offboard_output().map_err(E::PhysOut)?;
                Ok(offboard.is_holding_skateboard)
            },
        )?;
        if let Some(command) = command {
            kernel::apply_command(data, command);
        }
        kernel::dispatch(
            &mut Source {
                actor: self,
                motion: motion_occurrence,
                animation,
                physical: Arc::clone(physical.owner()),
                cvars,
                channel: FastString36::encode(kernel::CHANNEL).expect("native literal"),
            },
            data,
            names,
        )
    }
}
