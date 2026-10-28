//! **Front-end (UI) sounds** (engine-facing surface; doc `docs/hails-additions/15-world-audio.md`
//! "Session marker sounds").
//!
//! Retail plays its UI sounds from the `fe` records (237 of them: the session marker's cellphone
//! menu, menus, challenges, scores): each names a `sk8_menu` Splice sound and a level, read from
//! the install (`audio_export.frontend_sounds`). An engine system never sees banks or ids:
//!
//! - [`FrontendSound`]: play one record by name (`FrontendSound::named("challenge_count_go")`);
//!   the audio plays it like retail's front-end object (10 slots, one Splice one-shot each).
//! - [`SessionMarkerEvent`]: what the session marker just did (menu opened, marker placed, place
//!   refused, returned to the marker). `session_marker` sends it; [`SessionMarkerSounds`] maps
//!   each action to the record retail asks for there (data, so a mod or a later feature can
//!   remap or silence one), and mods see the same event as `on_event {name = "session_marker"}`.
//!
//! A mod plays a record with `sdk.audio.frontend(name)` (the same [`FrontendSound`]).
//!
//! - [`TeleportEffect`]: retail's teleport effect amount (`cMsgTeleportEffectAmount`, 0 → 1 over a
//!   Go To Marker hold). The screen static draws it and the skater's `Class_Treatment` plays its
//!   teleport layer from it (the crackling static, `Treatments` slots 0–12; doc 15 "Follow-up 3").
//!   The session marker sets it; a mod can drive it with `sdk.audio.teleport_effect(amount)`.
use bevy::prelude::*;
use skate_audio::frontend::marker;

/// Play one `fe` record (by key = the vault name hash of its name).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrontendSound {
    pub(crate) key: u64,
}

impl FrontendSound {
    pub(crate) fn named(name: &str) -> Self {
        Self {
            key: skate_audio::frontend::key(name),
        }
    }
}

/// What the session marker did.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SessionMarkerAction {
    /// The marker menu opened (LB pressed).
    Opened,
    /// Place Marker put the marker down.
    Placed,
    /// Place Marker was refused here (retail's error sound).
    Refused,
    /// Go To Marker completed: the skater is being moved to the marker.
    Returned,
}

impl SessionMarkerAction {
    /// The name mods receive (`on_event {name = "session_marker", action = ...}`).
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Opened => "opened",
            Self::Placed => "placed",
            Self::Refused => "refused",
            Self::Returned => "returned",
        }
    }
}

#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SessionMarkerEvent {
    pub(crate) action: SessionMarkerAction,
}

/// The `fe` record each session marker action plays (None = silent). Defaults: the records retail's
/// session marker asks for (PlayerUI `sub_82898FC8`, the cellphone UI `sub_826682B0`).
#[derive(Resource, Clone, Debug, PartialEq)]
pub(crate) struct SessionMarkerSounds {
    pub(crate) opened: Option<String>,
    pub(crate) placed: Option<String>,
    pub(crate) refused: Option<String>,
    pub(crate) returned: Option<String>,
}

impl Default for SessionMarkerSounds {
    fn default() -> Self {
        Self {
            opened: Some(marker::ACTIVATE.into()),
            placed: Some(marker::PLACE.into()),
            refused: Some(marker::ERROR.into()),
            returned: Some(marker::GOTO.into()),
        }
    }
}

impl SessionMarkerSounds {
    pub(crate) fn record(&self, action: SessionMarkerAction) -> Option<&str> {
        match action {
            SessionMarkerAction::Opened => self.opened.as_deref(),
            SessionMarkerAction::Placed => self.placed.as_deref(),
            SessionMarkerAction::Refused => self.refused.as_deref(),
            SessionMarkerAction::Returned => self.returned.as_deref(),
        }
    }
}

/// Retail's teleport effect amount as the VisualDirector holds it for a frame
/// (`cMsgTeleportEffectAmount`, message `0xFAF37902`): `None` on frames without the message. PlayerUI
/// (`sub_82898FC8`) sends it on every UI tick of a Go To Marker hold (the hold's progress) and for
/// the relocation tick and two more at 1.0; the VisualDirector keeps the last amount until its next
/// presentation build, which writes it into the presentation block's teleport field (`sub_827AB790`:
/// `B+16`+148 present, +152 the amount, i.e. `B+164` / `B+168`) and resets it. Two readers: the
/// screen static (`session_marker::effect`) and `Class_Treatment`'s update (`B+164` → w12 = 1,
/// `B+168` × 10000 → w13), whose program then plays the teleport crackle.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TeleportEffect {
    /// The engine's amount this frame (the session marker's hold), 0..=1.
    pub(crate) engine: Option<f32>,
    /// A mod's amount (`sdk.audio.teleport_effect`) and the real time (s) it lapses at.
    pub(crate) from_mod: Option<(f32, f64)>,
}

impl TeleportEffect {
    /// How long a mod's amount holds without being sent again: four UI ticks (retail sends one per
    /// tick; a mod sends one per frame, so frames slower than 15 fps let it lapse between sends).
    pub(crate) const MOD_HOLD: f64 = 4.0 / 60.0;

    /// The amount this frame: the larger of the engine's and a mod's.
    pub(crate) fn amount(&self) -> Option<f32> {
        match (self.engine, self.from_mod.map(|m| m.0)) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        }
    }

    /// `sdk.audio.teleport_effect(amount)` at real time `now` (amount clamped to 0..=1; 0 or a
    /// non-finite amount clears the mod's).
    pub(crate) fn set_from_mod(&mut self, amount: f32, now: f64) {
        self.from_mod =
            (amount.is_finite() && amount > 0.0).then(|| (amount.min(1.0), now + Self::MOD_HOLD));
    }
}

fn lapse_teleport_effect(mut effect: ResMut<TeleportEffect>, time: Res<Time<Real>>) {
    if effect
        .from_mod
        .is_some_and(|(_, until)| time.elapsed_secs_f64() >= until)
    {
        effect.from_mod = None;
    }
}

/// Runs before the audio pass (`game_audio`), after the simulation that sends the events.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UiAudioSet;

pub(crate) struct UiAudioPlugin;
impl Plugin for UiAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FrontendSound>()
            .add_message::<SessionMarkerEvent>()
            .init_resource::<SessionMarkerSounds>()
            .init_resource::<TeleportEffect>()
            .add_systems(
                Update,
                (marker_sounds, lapse_teleport_effect)
                    .in_set(UiAudioSet)
                    .after(crate::app::FrameSet::Animation),
            );
    }
}

fn marker_sounds(
    mut events: MessageReader<SessionMarkerEvent>,
    sounds: Res<SessionMarkerSounds>,
    mut out: MessageWriter<FrontendSound>,
) {
    for e in events.read() {
        if let Some(name) = sounds.record(e.action) {
            out.write(FrontendSound::named(name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_marker_action_asks_for_retails_record() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .configure_sets(Update, crate::app::FrameSet::Animation);
        app.add_plugins(UiAudioPlugin);
        for action in [
            SessionMarkerAction::Opened,
            SessionMarkerAction::Placed,
            SessionMarkerAction::Refused,
            SessionMarkerAction::Returned,
        ] {
            app.world_mut().write_message(SessionMarkerEvent { action });
        }
        app.update();
        let sounds = app.world().resource::<Messages<FrontendSound>>();
        let keys: Vec<u64> = sounds
            .iter_current_update_messages()
            .map(|s| s.key)
            .collect();
        assert_eq!(
            keys,
            [
                0x47FE_75BF_61F1_9941,
                0x0D6C_88A3_B91C_828F,
                0x66B3_AFE3_B602_918C,
                0x7F13_5F9F_D28F_7F21
            ]
        );
    }

    #[test]
    fn the_teleport_amount_takes_the_larger_and_a_mods_lapses() {
        let mut e = TeleportEffect::default();
        assert_eq!(e.amount(), None);
        e.engine = Some(0.25);
        assert_eq!(e.amount(), Some(0.25));
        e.set_from_mod(0.5, 10.0);
        assert_eq!(e.amount(), Some(0.5));
        e.set_from_mod(7.0, 10.0);
        assert_eq!(e.amount(), Some(1.0), "clamped");
        e.engine = None;
        assert_eq!(e.amount(), Some(1.0));
        for clear in [0.0, -1.0, f32::NAN] {
            e.set_from_mod(clear, 10.0);
            assert_eq!(e.from_mod, None, "{clear} clears");
        }
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .configure_sets(Update, crate::app::FrameSet::Animation);
        app.add_plugins(UiAudioPlugin);
        app.update();
        let now = app.world().resource::<Time<Real>>().elapsed_secs_f64();
        app.world_mut()
            .resource_mut::<TeleportEffect>()
            .set_from_mod(0.5, now);
        app.update();
        assert!(
            app.world().resource::<TeleportEffect>().from_mod.is_some(),
            "holds within four UI ticks"
        );
        app.world_mut().resource_mut::<TeleportEffect>().from_mod = Some((0.5, now - 1.0));
        app.update();
        assert_eq!(
            app.world().resource::<TeleportEffect>().from_mod,
            None,
            "lapses when not sent again"
        );
    }

    #[test]
    fn a_remapped_or_silenced_action_follows_the_resource() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .configure_sets(Update, crate::app::FrameSet::Animation);
        app.add_plugins(UiAudioPlugin);
        app.insert_resource(SessionMarkerSounds {
            placed: Some("core_a_button".into()),
            refused: None,
            ..Default::default()
        });
        app.world_mut().write_message(SessionMarkerEvent {
            action: SessionMarkerAction::Placed,
        });
        app.world_mut().write_message(SessionMarkerEvent {
            action: SessionMarkerAction::Refused,
        });
        app.update();
        let keys: Vec<u64> = app
            .world()
            .resource::<Messages<FrontendSound>>()
            .iter_current_update_messages()
            .map(|s| s.key)
            .collect();
        assert_eq!(keys, [skate_audio::frontend::key("core_a_button")]);
    }
}
