//! Original unranked/unranked_custom_match (screen 110, asset 105, fingerprint f1cd6f30; class
//! `unranked_custom_match`, InitAction character 34 `0x7a40`) and
//! unranked/unranked_custom_freeskate (screen 109, asset 106, fingerprint b8cacbd1; class
//! `unranked_custom_freeskate`, InitAction character 62 `0x8360`), driven by
//! `skate2_core::ui::online_match`: `highlight_selector` rows for the type, event and location,
//! the location's image, the event's description, the player's `online_gamer_tag`
//! (controls/online_gamer_tag, asset 107, fingerprint c9726e6b) and `mMatch`, the matchmaking
//! card.
use super::{assets::Assets, clips::Clip, edit_skater_movie, map_movie};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::{edit_skater::EditSkaterRow, online_match::Snapshot};

pub(super) const CUSTOM: usize = 105;
pub(super) const FREESKATE: usize = 106;
pub(super) const GAMER_TAG: usize = 107;
/// `InitializeScreen`'s widest title before it is scaled down.
const TITLE_WIDTH: f32 = 616.0;
const LOCATIONS: &str = "fe\\source\\images\\locations\\";

pub(super) struct OnlineMatchMovie {
    pub root: Clip,
    asset: usize,
    open: bool,
    shown: Option<Snapshot>,
    /// `ScreenManager.IntroComplete` and `OutroComplete` since the last calls.
    intro_complete: bool,
    /// The location image the screen's `LoadRX2Texture` names, for the owner to load.
    pub image_request: Option<String>,
    /// The texture key the preview shows.
    pub image: Option<String>,
}

impl OnlineMatchMovie {
    pub fn new(assets: &Assets, asset: usize) -> RenderResult<Self> {
        let mut root = Clip::new(assets, asset, 0)?;
        root.playback.stop();
        root.visible = false;
        Ok(Self { root, asset, open: false, shown: None, intro_complete: false, image_request: None, image: None })
    }

    pub fn take_intro_complete(&mut self) -> bool {
        std::mem::take(&mut self.intro_complete)
    }

    /// Opens the screen with its first state, plays the outro once it closes, and refreshes it
    /// as the state changes.
    pub fn update(&mut self, assets: &Assets, snapshot: Option<&Snapshot>, nickname: &str) -> RenderResult<()> {
        let snapshot = snapshot.filter(|snapshot| snapshot.freeskate == (self.asset == FREESKATE));
        match snapshot {
            Some(snapshot) if !self.open => {
                self.open = true;
                self.root = Clip::new(assets, self.asset, 0)?;
                self.root.visible = true;
                self.shown = None;
                self.calls(assets, snapshot, nickname)?;
            }
            None if self.open => {
                self.open = false;
                self.shown = None;
                self.root.goto(assets, "outro")?;
                return Ok(());
            }
            _ => {}
        }
        let Some(snapshot) = snapshot else { return Ok(()) };
        self.apply(assets, snapshot)
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if !self.root.visible {
            return Ok(());
        }
        self.root.tick(assets)?;
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "IntroComplete" => self.intro_complete = true,
                "OutroComplete" => self.root.visible = false,
                "WheelLoop" => self.root.named(&["mOptions", "mMatch", "mLoadingGraphic", "mcWheel"])?.goto_number(assets, 1)?,
                _ => return Err(format!("untranslated online match callback {call}").into()),
            }
        }
        Ok(())
    }

    /// The constructor (`InitializeScreen`), then the screen manager's `SetState(mNormalState)`.
    fn calls(&mut self, assets: &Assets, snapshot: &Snapshot, nickname: &str) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "OnlineMatchScreen" => self.initialize(assets, snapshot, nickname)?,
                _ => return Err(format!("untranslated online match callback {call}").into()),
            }
        }
        Ok(())
    }

    /// `InitializeScreen`: the card hidden, the title auto-sized left and scaled down to 616
    /// wide around its middle, the gamer tag's `Update`, the B/A help, the selector titles, and
    /// the selectors hidden until `SetNormalState`.
    fn initialize(&mut self, assets: &Assets, snapshot: &Snapshot, nickname: &str) -> RenderResult<()> {
        self.root.named(&["mOptions", "mMatch"])?.visible = false;
        let title = self.root.named(&["mTitle"])?;
        let text = assets.localize(snapshot.title)?;
        let mut width = 0.;
        for field in ["txtTitle", "txtGlow"] {
            let label = title.named(&[field])?;
            label.text = Some(text.encode_utf16().collect());
            width = edit_skater_movie::autosize_text(label, assets, &text)?;
        }
        if width > TITLE_WIDTH {
            let scale = TITLE_WIDTH / width;
            for field in ["txtTitle", "txtGlow"] {
                let child = super::pause_movie::child(title, field)?;
                child.entry.matrix.a *= scale;
                child.entry.matrix.d *= scale;
                child.clip.script_transform = true;
            }
        }
        let gamer = self.root.named(&["mGamerTag"])?;
        gamer.goto(assets, "none")?;
        gamer.named(&["txtGamertag"])?.text = Some(nickname.encode_utf16().collect());
        map_movie::styled_button_items(
            self.root.named(&["mContent", "mButtonHelp"])?,
            assets,
            &[("B", "ID_COMMON_BACK"), ("A", "ID_COMMON_ACCEPT")],
            0xffff_feec,
            false,
            "Marker_streak",
            20.0,
        )?;
        self.root.named(&["mOptions"])?.visible = true;
        self.root.named(&["mContent"])?.visible = true;
        Ok(())
    }

    /// `SetNormalState`'s refreshes and `UpdateSelector` as the state changes; the first
    /// matchmaking state runs `SetupMatchMaking`.
    fn apply(&mut self, assets: &Assets, snapshot: &Snapshot) -> RenderResult<()> {
        if self.shown.as_ref() == Some(snapshot) {
            return Ok(());
        }
        let previous = self.shown.replace(snapshot.clone());
        for (index, selector) in snapshot.selectors.iter().enumerate() {
            let row = EditSkaterRow {
                kind: "selector".into(),
                title: assets.localize(&selector.title)?,
                subtitle: assets.localize(&selector.subtitle)?,
                ..Default::default()
            };
            let control = self.root.named(&["mOptions", &format!("mSelector{index}")])?;
            edit_skater_movie::highlight(control, assets, &row, selector.selected)?;
        }
        if previous.as_ref().is_none_or(|previous| previous.image != snapshot.image) {
            let image = snapshot.image.strip_prefix(LOCATIONS).unwrap_or(&snapshot.image);
            self.image_request = Some(image.to_owned());
        }
        if let Some(description) = snapshot.description {
            let help = self.root.named(&["mContent", "txtHelp"])?;
            help.text = Some(assets.localize(description)?.encode_utf16().collect());
        }
        let was_matching = previous.as_ref().is_some_and(|previous| previous.matchmaking.is_some());
        match snapshot.matchmaking {
            Some(text) if !was_matching => {
                let card = self.root.named(&["mOptions", "mMatch"])?;
                card.visible = true;
                card.named(&["txtTitle"])?.text = Some(assets.localize(text)?.encode_utf16().collect());
                self.root.named(&["mContent", "mButtonHelp"])?.visible = false;
                self.root.named(&["mOptions"])?.goto(assets, "match")?;
            }
            None if was_matching => {
                self.root.named(&["mOptions", "mMatch"])?.visible = false;
                self.root.named(&["mContent", "mButtonHelp"])?.visible = true;
                self.root.named(&["mOptions"])?.goto(assets, "options")?;
            }
            _ => {}
        }
        Ok(())
    }

    /// The preview's texture once the owner has it.
    pub fn set_image(&mut self, key: &str) -> RenderResult<()> {
        self.image = Some(key.to_owned());
        self.root.named(&["mContent", "mPreviewImage"])?.properties.insert("cac_texture".into(), key.to_owned());
        Ok(())
    }
}

/// The screens' InitActions: their classes, and the gamer tag control's class and its
/// `registerClass`.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (CUSTOM, 34, 0x7a40) | (FREESKATE, 62, 0x8360) | (GAMER_TAG, 6, 0x3f0) | (GAMER_TAG, 5, 0x534))
}

/// Timeline actions besides `Stop`: each root constructs its class (`InitializeScreen`), turns
/// the dimmer on and off (a `ScreenManager` method screenmanager.apt never defines) and stops
/// its intro and outro with `IntroComplete` and `OutroComplete`; the loading wheel loops.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match (clip.asset, offset) {
        (CUSTOM, 0x7a00) | (FREESKATE, 0x8320) => clip.calls.push("OnlineMatchScreen"),
        (CUSTOM, 0x8730 | 0x8770) | (FREESKATE, 0x8e30 | 0x8e78) => {}
        (CUSTOM, 0x874c) | (FREESKATE, 0x8e54) => {
            clip.playback.stop();
            clip.calls.push("IntroComplete");
        }
        (CUSTOM, 0x878c) | (FREESKATE, 0x8e9c) => {
            clip.playback.stop();
            clip.calls.push("OutroComplete");
        }
        (CUSTOM, 0x87d4) | (FREESKATE, 0x8ff4) => clip.calls.push("WheelLoop"),
        _ => return Err(format!("untranslated online match APT action {}:{offset:#x}", clip.asset).into()),
    }
    Ok(())
}
