//! Retained input delivery and recognition stage for the selected Actor tick.
//! Reuses Pack82a8c070, `DoCommand825e5610` and Update826e2578; this is not the
//! native `GameModule` pause/network/RNG scheduler. Fill and Action run later.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained input session is exercised by original corpus; production scheduler mounting follows"
    )
)]
use crate::{
    debug_overlay::DebugOverlaySlot,
    game_input_command::{
        GameInputCommandNumeric, GameInputPackError, GameInputWrite, try_pack_player_input,
    },
    game_input_manager::{
        EncodedGameInputCommand, GameInputCommandManager, GameInputDispatch, GameInputManagerError,
        GameInputSyncLevel,
    },
    game_input_player::{PlayerInputData, recognizer::GameInputRecognizerServices},
    gameplay_input_update_manager::{GameplayInputUpdateError, GameplayInputUpdateManager},
};

/// Move the already constructed managers here; never Reset them per frame.
/// The registered listener, player occurrences and recognizer histories survive.
pub(crate) struct GameInputFrameSession<K> {
    pub(crate) commands: GameInputCommandManager,
    pub(crate) gameplay: GameplayInputUpdateManager<K>,
}

/// One converted device cache line, serialized before gameplay map evaluation.
/// This is not the SIM group's mapped pad/history. Caller supplies the actual
/// receiver and selected update mask; no local-player or sync-level substitute.
#[derive(Clone, Copy)]
pub(crate) struct GameInputFrameRequest<'a> {
    pub(crate) input: &'a PlayerInputData,
    pub(crate) receiver_id: u32,
    pub(crate) update_mask: u32,
    pub(crate) add_to_crc: bool,
}

#[derive(Debug)]
pub(crate) enum GameInputFrameError<N, S, R> {
    Pack(GameInputPackError<N>),
    Command(GameInputManagerError<S>),
    Update(GameplayInputUpdateError<R>),
}

impl<N: std::fmt::Debug, S: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display
    for GameInputFrameError<N, S, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "input frame delivery: {self:?}")
    }
}

#[derive(Debug)]
pub(crate) struct GameInputFrameDelivery {
    pub(crate) dispatch: GameInputDispatch,
    pub(crate) flushed_events: usize,
}

type FrameResult<N, S, R> = Result<GameInputFrameDelivery, GameInputFrameError<N, S, R>>;

impl<K> GameInputFrameSession<K> {
    /// Selects the original loaded pattern used by this session's live matcher.
    pub(crate) fn original_right_pattern(
        &self,
        name: &str,
    ) -> std::sync::Arc<crate::input_pattern::InputPattern> {
        self.gameplay
            .loaded_patterns(crate::input_pattern_matcher::InputPatternStick::Right)
            .iter()
            .flat_map(|set| set.patterns().iter())
            .find(|pattern| pattern.name() == name)
            .cloned()
            .expect("requested original right-stick pattern")
    }

    /// Native admitted command slice over this session's SAME dispatcher.
    /// Recognition remains a later explicit stage, after all queued commands.
    pub(crate) fn try_deliver_queued_commands<S: GameInputSyncLevel>(
        &mut self,
        queue: &mut crate::selected_sim_command_queue::SelectedSimCommandQueue,
        random: &mut crate::native_skater_random::SimulationCommandRandom,
        sync: &mut S,
    ) -> Result<
        crate::selected_sim_command_queue::CommandStageDelivery,
        crate::selected_sim_command_queue::CommandConsumeError<S::Error>,
    > {
        queue.try_consume_admitted(&mut self.commands, random, sync)
    }
    /// Already-delivered input entry: does not repack or redispatch a command.
    /// Caller reaches the real gameplay update stage with its actual mask.
    pub(crate) fn try_recognize_delivered<R: GameInputRecognizerServices>(
        &mut self,
        update_mask: u32,
        recognizer: &mut R,
        overlay: &DebugOverlaySlot,
    ) -> Result<usize, GameplayInputUpdateError<R::Error>> {
        try_recognize_delivered(&mut self.gameplay, update_mask, recognizer, overlay)
    }

    pub(crate) fn try_step<N, S, R>(
        &mut self,
        request: GameInputFrameRequest<'_>,
        numeric: &mut N,
        sync: &mut S,
        recognizer: &mut R,
        overlay: &DebugOverlaySlot,
    ) -> FrameResult<N::Error, S::Error, R::Error>
    where
        N: GameInputCommandNumeric,
        S: GameInputSyncLevel,
        R: GameInputRecognizerServices,
    {
        try_deliver_frame(
            &mut self.commands,
            &mut self.gameplay,
            request,
            numeric,
            sync,
            recognizer,
            overlay,
        )
    }
}

/// Shared borrowed form lets constructor/replay callers retain their managers
/// in place. Successful command delivery precedes recognizer Update and Flush.
/// A native missing-receiver result is reported, not changed into a new error.
/// Failures retain any completed CRC/player/history/listener writes: do not
/// retry this stage as though the command or frame had never happened.
pub(crate) fn try_deliver_frame<K, N, S, R>(
    commands: &mut GameInputCommandManager,
    gameplay: &mut GameplayInputUpdateManager<K>,
    request: GameInputFrameRequest<'_>,
    numeric: &mut N,
    sync: &mut S,
    recognizer: &mut R,
    overlay: &DebugOverlaySlot,
) -> FrameResult<N::Error, S::Error, R::Error>
where
    N: GameInputCommandNumeric,
    S: GameInputSyncLevel,
    R: GameInputRecognizerServices,
{
    // One mask plus at most eight sparse fields; unfocused input writes zero
    // bytes. The original codec performs all quantization/focus behavior.
    let mut bytes = [0; 9];
    let count = {
        let mut stream = GameInputWrite::new(&mut bytes);
        try_pack_player_input(request.input, &mut stream, numeric)
            .map_err(GameInputFrameError::Pack)?;
        stream.position()
    };
    let dispatch = commands
        .try_do_command(
            &EncodedGameInputCommand {
                receiver_id: request.receiver_id,
                kind: 1,
                payload: &bytes[..count],
            },
            request.add_to_crc,
            sync,
        )
        .map_err(GameInputFrameError::Command)?;
    let flushed_events =
        try_recognize_delivered(gameplay, request.update_mask, recognizer, overlay)
            .map_err(GameInputFrameError::Update)?;
    Ok(GameInputFrameDelivery {
        dispatch,
        flushed_events,
    })
}

/// Shared borrowed recognition stage for immediate and queued delivery callers.
pub(crate) fn try_recognize_delivered<K, R: GameInputRecognizerServices>(
    gameplay: &mut GameplayInputUpdateManager<K>,
    update_mask: u32,
    recognizer: &mut R,
    overlay: &DebugOverlaySlot,
) -> Result<usize, GameplayInputUpdateError<R::Error>> {
    gameplay.try_update(update_mask, overlay, recognizer)
}
