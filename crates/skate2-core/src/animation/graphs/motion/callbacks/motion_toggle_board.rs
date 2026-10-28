//! ToggleBoard::Update82cc2c08 command gates and dispatch, UpdateDropState82cbbc38,
//! UpdateRetrievalState82cbb240 and UpdateYawPitch82caae98 on the retained instance.
#![allow(clippy::neg_cmp_op_on_partial_ord)]
use crate::{
    actor_animation_construction::channel_runtime::{ChannelParams, transition::ChannelStartFlags},
    generated_motion_load::{construct::ToggleBoardNames, controller::ToggleBoardInstance},
    play_animation::{FastString30, FastString36},
};

/// Dynamic initializers 82f81f70/88/a0: the intent FastStrings omit "Intent".
pub(crate) const DROP: &[u8] = b"OB_DropBoard";
pub(crate) const THROW: &[u8] = b"OB_ThrowBoard";
pub(crate) const RETRIEVE: &[u8] = b"OB_RetrieveBoard";
/// `RetrieveBoardSkaterAnim`830a5f7c, initializer82f84558: "RetrieveBoard".
pub(crate) const CHANNEL: &str = "RetrieveBoard";
/// `kUpdateTime`822f2204 (1/60). Frame cvars scale by it, never by the frame dt.
const UPDATE_TIME: f32 = f32::from_bits(0x3c88_8889);
const NEGATIVE_DEGREES: f32 = f32::from_bits(0xc265_2ee0);
const DEGREES: f32 = f32::from_bits(0x4265_2ee0);
const MINUS_QUARTER: f32 = f32::from_bits(0xc2b4_0000);
const MINUS_THREE_EIGHTHS: f32 = f32::from_bits(0xc307_0000);
const HALF_TURN: f32 = f32::from_bits(0x4334_0000);
const QUARTER: f32 = f32::from_bits(0x42b4_0000);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ToggleBoardCvar {
    YawChange,
    BlendFrames,
    BlendOutFrames,
    RetrievePlayOutFrames,
    DropPlayOutFrames,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BoardCommand {
    Drop,
    Throw,
    Retrieve,
}

/// Actor services in native call order. Every physical read is a fresh
/// `IPhysical::GetOutput`; attribute names arrive already narrowed.
pub(crate) trait ToggleBoardSource {
    type Error;
    fn holding(&mut self) -> Result<bool, Self::Error>;
    fn yaw_to_free_board(&mut self) -> Result<f32, Self::Error>;
    fn pitch_to_free_board(&mut self) -> Result<f32, Self::Error>;
    /// `ISkaterAnim::GetIsMirrored82cb7598`.
    fn mirrored(&mut self) -> Result<bool, Self::Error>;
    /// `FastString<30>::FastString<30>(FastString<36> const&)82594ad0`.
    fn narrow(&mut self, name: FastString36) -> Result<FastString30, Self::Error>;
    /// `ISkaterMotionGraph::AddMgAttribute825fe7a0`.
    fn motion_attribute(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error>;
    /// `IAnimatable::AddSettableAtt82d7f5f0` with normalized false, sequence -1.
    fn settable(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error>;
    fn has_channel(&mut self) -> Result<bool, Self::Error>;
    fn time_left(&mut self) -> Result<f32, Self::Error>;
    fn time_elapsed(&mut self) -> Result<f32, Self::Error>;
    fn transition_to(
        &mut self,
        animation: FastString36,
        length: f32,
        params: ChannelParams,
        flags: ChannelStartFlags,
    ) -> Result<(), Self::Error>;
    fn sequence_to(
        &mut self,
        animation: FastString36,
        params: ChannelParams,
        flags: ChannelStartFlags,
    ) -> Result<(), Self::Error>;
    /// `EndChannelPrematurely82d7d648` with r6 false.
    fn end_channel(&mut self, length: f32) -> Result<(), Self::Error>;
    fn cvar(&mut self, cvar: ToggleBoardCvar) -> Result<f32, Self::Error>;
    fn unwritten(&mut self, field: &'static str) -> Self::Error;
    fn assertion(&mut self, site: &'static str) -> Self::Error;
}

/// The command checks are lazy. A present command only acts in idle with the
/// matching holding flag; a failed gate falls through to the next intent.
pub(crate) fn select_command<E>(
    grabbing: bool,
    mut has_intent: impl FnMut(&[u8]) -> Result<bool, E>,
    mut is_idle: impl FnMut() -> Result<bool, E>,
    mut is_holding: impl FnMut() -> Result<bool, E>,
) -> Result<Option<BoardCommand>, E> {
    if grabbing {
        return Ok(None);
    }
    for (name, required_holding, command) in [
        (DROP, true, BoardCommand::Drop),
        (THROW, true, BoardCommand::Throw),
        (RETRIEVE, false, BoardCommand::Retrieve),
    ] {
        if has_intent(name)? && is_idle()? && is_holding()? == required_holding {
            return Ok(Some(command));
        }
    }
    Ok(None)
}

/// 82cc3054..3128: drop and throw both select state5 and replace only the
/// +0x60 bit6; retrieval selects state1.
pub(crate) fn apply_command(data: &mut ToggleBoardInstance, command: BoardCommand) {
    match command {
        BoardCommand::Drop => {
            data.state = Some(5);
            data.throw_selected = Some(false);
        }
        BoardCommand::Throw => {
            data.state = Some(5);
            data.throw_selected = Some(true);
        }
        BoardCommand::Retrieve => data.state = Some(1),
    }
}

/// 82cc3130..318c: the source reaches this switch even when grabbing
/// suppresses every command. Other states do nothing.
pub(crate) fn dispatch<S: ToggleBoardSource>(
    source: &mut S,
    data: &mut ToggleBoardInstance,
    names: &ToggleBoardNames,
) -> Result<(), S::Error> {
    match data.state {
        Some(0) => {
            data.reset_yaw_pitch(names);
            Ok(())
        }
        Some(1..=4) => update_retrieval_state(source, data, names),
        Some(5..=6) => update_drop_state(source, data, names),
        Some(_) => Ok(()),
        None => Err(source.unwritten("ToggleBoard state")),
    }
}

fn params(blend: f32, endpoints: bool) -> ChannelParams {
    ChannelParams {
        priority: 0,
        cycle: false,
        mirror: false,
        playback_speed: 1.0,
        blend_in_length: blend,
        blend_in_to_first_frame: endpoints,
        blend_out_length: blend,
        blend_out_from_last_frame: endpoints,
    }
}

fn start_flags(create_if_missing: bool) -> ChannelStartFlags {
    ChannelStartFlags {
        resurrect_if_blending_out: true,
        create_if_missing,
        use_attributes: true,
    }
}

/// Both graphs receive the attribute through separate FastString<30> temporaries.
fn publish<S: ToggleBoardSource>(source: &mut S, name: FastString36) -> Result<(), S::Error> {
    let narrow = source.narrow(name)?;
    source.motion_attribute(narrow, 1.0)?;
    let narrow = source.narrow(name)?;
    source.settable(narrow, 1.0)
}

/// `UpdateDropState82cbbc38`.
pub(crate) fn update_drop_state<S: ToggleBoardSource>(
    source: &mut S,
    data: &mut ToggleBoardInstance,
    names: &ToggleBoardNames,
) -> Result<(), S::Error> {
    publish(source, names.dropping)?;
    match data.state {
        Some(5) => {
            // c168..c20c: state6 precedes TransitionTo, whose status is unused.
            let blend = source.cvar(ToggleBoardCvar::BlendFrames)? * UPDATE_TIME;
            data.state = Some(6);
            let throw = data
                .throw_selected
                .ok_or_else(|| source.unwritten("ToggleBoard throw bit"))?;
            source.transition_to(
                if throw { names.throw } else { names.drop },
                blend,
                params(blend, false),
                start_flags(true),
            )
        }
        Some(6) => {
            // c0e4/c11c BLE: unordered time keeps the channel.
            let finished = !source.has_channel()? || !(source.time_left()? > 0.0) || {
                let play_out = source.cvar(ToggleBoardCvar::DropPlayOutFrames)?;
                source.time_elapsed()? > play_out * UPDATE_TIME
            };
            if finished {
                let out = source.cvar(ToggleBoardCvar::BlendOutFrames)? * UPDATE_TIME;
                source.end_channel(out)?;
                data.state = Some(0);
            }
            Ok(())
        }
        _ => Err(source.assertion("ToggleBoard drop state")),
    }
}

/// 82cbb870..b8c8 / ba2c..baf8: SequenceTo the cycle, then the retrieve attribute.
fn sequence_cycle<S: ToggleBoardSource>(
    source: &mut S,
    data: &ToggleBoardInstance,
    names: &ToggleBoardNames,
) -> Result<(), S::Error> {
    let blend = source.cvar(ToggleBoardCvar::BlendFrames)? * UPDATE_TIME;
    source.sequence_to(data.retrieve[1], params(blend, true), start_flags(false))?;
    publish(source, names.retrieve)
}

/// `UpdateRetrievalState82cbb240`.
pub(crate) fn update_retrieval_state<S: ToggleBoardSource>(
    source: &mut S,
    data: &mut ToggleBoardInstance,
    names: &ToggleBoardNames,
) -> Result<(), S::Error> {
    publish(source, names.retrieving)?;
    match data.state {
        Some(1) => {
            update_yaw_pitch(source, data, names)?;
            let blend = source.cvar(ToggleBoardCvar::BlendFrames)? * UPDATE_TIME;
            data.state = Some(2);
            source.transition_to(
                data.retrieve[0],
                blend,
                params(blend, false),
                start_flags(true),
            )?;
        }
        Some(state @ (2 | 3)) => {
            update_yaw_pitch(source, data, names)?;
            // b7b4/b934 BLE: unordered time left also resets to idle.
            if !source.has_channel()? || !(source.time_left()? > 0.0) {
                data.state = Some(0);
            } else if state == 3 && source.holding()? {
                let blend = source.cvar(ToggleBoardCvar::BlendFrames)? * UPDATE_TIME;
                data.state = Some(4);
                source.transition_to(
                    data.retrieve[2],
                    blend,
                    params(blend, true),
                    start_flags(false),
                )?;
            } else {
                // b7f0/ba20 BGT: another pass keeps the current clip.
                let blend = source.cvar(ToggleBoardCvar::BlendFrames)? * UPDATE_TIME;
                if !(source.time_left()? > blend) {
                    data.state = Some(3);
                    sequence_cycle(source, data, names)?;
                }
            }
        }
        Some(4) => {
            // bb54/bb8c BLE: unordered values keep the out clip.
            let finished = !source.has_channel()? || !(source.time_left()? > 0.0) || {
                let play_out = source.cvar(ToggleBoardCvar::RetrievePlayOutFrames)?;
                source.time_elapsed()? > play_out * UPDATE_TIME
            };
            if finished {
                data.state = Some(0);
                let out = source.cvar(ToggleBoardCvar::BlendOutFrames)? * UPDATE_TIME;
                source.end_channel(out)?;
            }
        }
        _ => return Err(source.assertion("ToggleBoard retrieval state")),
    }
    // bbe0..bc24: mirror-corrected yaw, then pitch.
    let yaw = data
        .signed_yaw
        .ok_or_else(|| source.unwritten("ToggleBoard mirror-corrected yaw"))?;
    source.settable(names.yaw, yaw)?;
    let pitch = data
        .pitch
        .ok_or_else(|| source.unwritten("ToggleBoard pitch"))?;
    source.settable(names.pitch, pitch)
}

/// PPC FSEL: nonnegative operand for either zero, negative operand for NaN.
fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}

/// `UpdateYawPitch82caae98`. Both interfaces are resolved by the caller.
pub(crate) fn update_yaw_pitch<S: ToggleBoardSource>(
    source: &mut S,
    data: &mut ToggleBoardInstance,
    names: &ToggleBoardNames,
) -> Result<(), S::Error> {
    let mut yaw = source.yaw_to_free_board()? * NEGATIVE_DEGREES;
    if source.mirrored()? {
        yaw = -yaw;
    }
    // b048 BGE: only an ordered yaw below -90 is folded.
    if yaw < MINUS_QUARTER {
        let smoothing = data
            .yaw_cached
            .ok_or_else(|| source.unwritten("ToggleBoard smoothing bit"))?;
        yaw = if smoothing {
            let previous = data
                .signed_yaw
                .ok_or_else(|| source.unwritten("ToggleBoard mirror-corrected yaw"))?;
            select(previous, HALF_TURN, MINUS_QUARTER)
        } else if yaw < MINUS_THREE_EIGHTHS {
            HALF_TURN
        } else {
            MINUS_QUARTER
        };
    }
    if source.mirrored()? {
        yaw = -yaw;
    }
    if data
        .yaw_cached
        .ok_or_else(|| source.unwritten("ToggleBoard smoothing bit"))?
    {
        // b0c4..b0f0: two FSEL clamps of the change around the previous yaw.
        let previous = data
            .yaw
            .ok_or_else(|| source.unwritten("ToggleBoard yaw"))?;
        let delta = yaw - previous;
        let maximum = source.cvar(ToggleBoardCvar::YawChange)?;
        let lower = select(-maximum - delta, -maximum, delta);
        yaw = select(maximum - lower, lower, maximum) + previous;
    }
    let corrected = if source.mirrored()? { -yaw } else { yaw };
    data.yaw = Some(yaw);
    data.signed_yaw = Some(corrected);
    let pitch = source.pitch_to_free_board()?;
    data.yaw_cached = Some(true);
    data.pitch = Some(pitch * DEGREES);
    // b18c BGT: unordered selects the front set.
    data.retrieve = if corrected.abs() > QUARTER {
        names.retrieve_back
    } else {
        names.retrieve_front
    };
    Ok(())
}

#[cfg(test)]
#[path = "motion_toggle_board_tests.rs"]
mod tests;
