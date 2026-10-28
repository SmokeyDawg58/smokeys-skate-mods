//! Read-only snapshots at the public session boundary, including frontend ticks.
use super::{SelectedLiveError, SelectedLiveSession, SelectedQueuedFrameOutcome};
use crate::diagnostics::test_log;
use std::time::Duration;

impl<T, C> SelectedLiveSession<T, C> {
    pub(super) fn log_test_frame(
        &self,
        result: &Result<SelectedQueuedFrameOutcome, SelectedLiveError>,
        elapsed: Duration,
    ) {
        test_log::event(
            "frame",
            format_args!(
                "frame={} elapsed_ms={:.3} outcome={result:?} completed={} settings={:?} pause={:?} frontend_audio={:?} editor={} replay={} map_visible={} travel_pending={} travel_gate={} fade={} reveal_pending={} travel_dispatched={} expected_generation={:?} ready_seen={} actor_generation={:?} states_action_motion_physics={:?} board_position_zspeed_deckspeed={:?} manual_kickturn={:?} offboard_jump_trajectory_hold={:?}",
                self.completed_frames,
                elapsed.as_secs_f64() * 1000.,
                self.last_step_completed,
                self.options_menu.snapshot(),
                self.pause_menu.snapshot(),
                self.frontend_audio_state(),
                self.editor.is_some(),
                self.replay_editor.is_some(),
                self.map_menu_snapshot().visible,
                self.map_travel.is_some(),
                self.map_travel_gate,
                self.frontend_fade.alpha(),
                self.frontend_reveal_pending,
                self.map_travel_dispatched,
                self.map_travel_generation,
                self.map_travel_ready_seen,
                self.inner.player.as_ref().and_then(|player| player.selected.actor.as_ref())
                    .and_then(|actor| actor.try_teleport_publication().ok()).map(|(generation, _)| generation),
                self.diagnostic_state_ids(),
                self.diagnostic_board_kinematics(),
                self.diagnostic_manual_flags(),
                self.diagnostic_offboard_flags(),
            ),
        );
        if let Some(editor) = &self.editor {
            test_log::event(
                "edit_skater",
                format_args!(
                    "frame={} selected_set={:?} skate2_up_down_left_right={:?} skate3_up_down_left_right={:?}",
                    self.completed_frames,
                    editor.frontend.gesture_settings.selected,
                    editor.frontend.record.data.gestures,
                    editor.frontend.gesture_settings.skate3
                ),
            );
        }
        if let Some(actor) = self
            .inner
            .player
            .as_ref()
            .and_then(|player| player.selected.actor.as_ref())
            && let Ok(input) = actor.published_animation_physics_input()
            && let Ok(input) = input.try_lock()
        {
            test_log::event(
                "animation_physics_input",
                format_args!(
                    "frame={} gesture={} gesturing={} wipeout_gesture={:?}",
                    self.completed_frames, input.gesture, input.gesturing, input.wipeout_gesture
                ),
            );
        }
    }
}
