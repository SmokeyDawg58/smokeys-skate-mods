//! Original main/partyplay APT (`FrontEndState_PartyPlay`, screen 51), fingerprint e1735e94: four
//! player cards, the skater name, the three games and the location with its image, on the
//! `partyplay` wall. It imports `image_renderer` (`mImage`) and `highlight_category`
//! (`mCharname`, `mLocationTitle`), drives the front end's letterbox, and every nested timeline
//! script is a `Stop`.
use super::{assets::Assets, clips::Clip, edit_skater_movie, letterbox::LetterBox, localizer, map_movie};
use crate::gpu_renderer::RenderResult;
use skate2_core::ui::{
    edit_skater::EditSkaterRow,
    party_play::{screen_state, Game, Snapshot},
};

pub(super) const ASSET: usize = 51;
const PLAYERS: usize = 4;

pub(super) struct PartyPlayMovie {
    pub root: Clip,
    /// The screen is open (between the constructor and the outro).
    open: bool,
    /// `mCurrentNumPlayers`.
    players: usize,
    intro_complete: bool,
}

impl PartyPlayMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, ASSET, 0)?;
        root.playback.stop();
        root.visible = false;
        Ok(Self { root, open: false, players: 2, intro_complete: false })
    }

    /// The intro's `ScreenManager.IntroComplete` since the last call.
    pub fn take_intro_complete(&mut self) -> bool {
        std::mem::take(&mut self.intro_complete)
    }

    /// Opens with the screen's first snapshot, plays the outro when the screen closes, and runs
    /// each `SetState` the state sent; `image` is the loaded texture `LoadRX2Texture` names.
    pub fn update(
        &mut self,
        assets: &Assets,
        snapshot: Option<&Snapshot>,
        states: &[u32],
        letterbox: &mut LetterBox,
        image: Option<&str>,
    ) -> RenderResult<()> {
        match snapshot {
            Some(_) if !self.open => self.construct(assets, letterbox)?,
            None if self.open => {
                self.open = false;
                self.root.goto(assets, "outro")?;
            }
            _ => {}
        }
        let Some(snapshot) = snapshot.filter(|_| self.open) else { return Ok(()) };
        for &state in states {
            self.set_state(assets, snapshot, state, letterbox, image)?;
        }
        Ok(())
    }

    fn screen(&mut self) -> RenderResult<&mut Clip> {
        self.root.named(&["mPartyplay"])
    }

    fn player(&mut self, index: usize) -> RenderResult<&mut Clip> {
        self.root.named(&["mPartyplay", &format!("mPlayer{index}")])
    }

    /// The constructor 006593: the letterbox title and B/A help, each card's `ID_INPUT_P<n>` label,
    /// tab and `ID_PARTYPLAY_PLAYER_JOIN` prompt, both titles selected, then the Xbox ports.
    /// `mPlayername.autoSize` is set on the clip, not its field, and changes nothing.
    fn construct(&mut self, assets: &Assets, letterbox: &mut LetterBox) -> RenderResult<()> {
        self.root = Clip::new(assets, ASSET, 0)?;
        self.root.visible = true;
        self.root.goto(assets, "intro")?;
        self.open = true;
        self.players = 2;
        letterbox.set_title(&assets.localize("ID_PARTYPLAY_TITLE")?)?;
        Self::standard_options(assets, letterbox, "ID_COMMON_SELECT")?;
        for index in 0..PLAYERS {
            let label: Vec<u16> = assets.localize(&format!("ID_INPUT_P{}", index + 1))?.encode_utf16().collect();
            let number = i32::try_from(index + 1).map_err(|_| "party play player number")?;
            let join = localizer::localized_format_string(assets, "ID_PARTYPLAY_PLAYER_JOIN", &[localizer::AptArgument::Integer(number)])?;
            let player = self.player(index)?;
            for field in ["playerText", "playerTextGlow"] {
                player.named(&["mPlayer", field])?.text = Some(label.clone());
            }
            player.named(&["mPlayertab"])?.goto(assets, &index.to_string())?;
            for field in ["labelText", "labelTextGlow"] {
                player.named(&["mJoin", "mJointext", field])?.text = Some(join.clone());
            }
        }
        let selected = EditSkaterRow { kind: "category".into(), ..Default::default() };
        for path in [["mCharselect", "mBacker", "mCharname"].as_slice(), ["mLocationselect", "mLocationTitle"].as_slice()] {
            edit_skater_movie::highlight(self.screen()?.named(path)?, assets, &selected, true)?;
        }
        // UpdatePlayerPorts 006871: PlatformCurrent is the Xbox 360.
        for index in 0..PLAYERS {
            let port = self.player(index)?.named(&["mPort"])?;
            port.named(&["mXboxports"])?.visible = true;
            port.named(&["mPS3ports"])?.visible = false;
        }
        Ok(())
    }

    /// `SetupStandardOptions` 006e17 (A Select) / `SetupStartOptions` 006e5f (A Start): B Back
    /// and A on the letterbox's bottom menu.
    fn standard_options(assets: &Assets, letterbox: &mut LetterBox, a: &str) -> RenderResult<()> {
        map_movie::configure_button_items(
            letterbox.clip.named(&["mcLetterbox", "mInfo", "mcBottomMenu", "mButtonMenu"])?,
            assets,
            &[("B", "ID_COMMON_BACK"), ("A", a)],
            0xffff_ffec,
        )
    }

    /// `SetActivePlayerPort` 006945: the Xbox ports show the controller, or `none`.
    fn port(&mut self, assets: &Assets, player: usize, controller: i32) -> RenderResult<()> {
        let label = if controller < 0 { "none".to_owned() } else { controller.to_string() };
        self.player(player)?.named(&["mPort", "mXboxports"])?.goto(assets, &label)
    }

    /// `highlight_category.SetTitle`: the label, then the right arrow after it.
    fn category_title(control: &mut Clip, assets: &Assets, title: &str) -> RenderResult<()> {
        let row = EditSkaterRow { kind: "category".into(), subtitle: title.to_owned(), ..Default::default() };
        edit_skater_movie::category_title(control, assets, &row)
    }

    /// `SetState` 006a02.
    fn set_state(&mut self, assets: &Assets, snapshot: &Snapshot, state: u32, letterbox: &mut LetterBox, image: Option<&str>) -> RenderResult<()> {
        let active = snapshot.active.min(PLAYERS - 1);
        match state {
            screen_state::SELECT_PLAYERS => self.screen()?.goto(assets, "numplayers")?,
            screen_state::REFRESH_SELECT_PLAYERS => {
                self.players = snapshot.players;
                let frame = match self.players {
                    2 => Some("0"),
                    3 => Some("1"),
                    4 => Some("2"),
                    _ => None,
                };
                if let Some(frame) = frame {
                    self.screen()?.named(&["mNumplayers"])?.goto(assets, frame)?;
                }
            }
            screen_state::CONTROLLER_SELECT => {
                self.screen()?.goto(assets, "setup")?;
                for index in 0..PLAYERS {
                    self.player(index)?.visible = index < self.players;
                }
                for index in active..self.players.min(PLAYERS) {
                    self.player(index)?.goto(assets, "open")?;
                    self.port(assets, index, -1)?;
                }
            }
            screen_state::PLAYER_JOIN => {
                self.player(active)?.goto(assets, "join")?;
                self.screen()?.named(&["mCharselect"])?.goto(assets, "off")?;
            }
            screen_state::PLAYER_SELECT_CHARACTER => {
                self.player(active)?.goto(assets, "charselect")?;
                let charselect = self.screen()?.named(&["mCharselect"])?;
                charselect.named(&["mBacker"])?.goto(assets, &active.to_string())?;
                charselect.goto(assets, "on")?;
                self.port(assets, active, snapshot.controllers[active])?;
                self.set_state(assets, snapshot, screen_state::REFRESH_SELECT_CHARACTER, letterbox, image)?;
            }
            screen_state::REFRESH_SELECT_CHARACTER => {
                let name = assets.localize(&snapshot.character)?;
                Self::category_title(self.screen()?.named(&["mCharselect", "mBacker", "mCharname"])?, assets, &name)?;
            }
            screen_state::ACCEPT_SELECT_CHARACTER => {
                let name = assets.localize(&snapshot.character)?;
                let player = self.player(active)?;
                player.goto(assets, "set")?;
                player.named(&["mPlayername", "labelText"])?.text = Some(name.encode_utf16().collect());
            }
            screen_state::SETUP_GAME => {
                Self::standard_options(assets, letterbox, "ID_COMMON_SELECT")?;
                self.screen()?.named(&["mCharselect"])?.goto(assets, "off")?;
                self.screen()?.goto(assets, "eventselect")?;
                self.set_state(assets, snapshot, screen_state::REFRESH_GAME, letterbox, image)?;
            }
            screen_state::REFRESH_GAME => self.update_game(assets, snapshot.game)?,
            screen_state::SETUP_SELECT_LOCATION => {
                Self::standard_options(assets, letterbox, "ID_PARTYPLAY_START")?;
                self.screen()?.goto_and_stop(assets, "locationselect")?;
                self.set_state(assets, snapshot, screen_state::REFRESH_SELECT_LOCATION, letterbox, image)?;
            }
            screen_state::REFRESH_SELECT_LOCATION => {
                let title = assets.localize(&snapshot.location)?;
                Self::category_title(self.screen()?.named(&["mLocationselect", "mLocationTitle"])?, assets, &title)?;
            }
            screen_state::REFRESH_IMAGE => {
                let control = self.screen()?.named(&["mImage"])?;
                match image {
                    Some(texture) => {
                        control.properties.insert("cac_texture".into(), texture.to_owned());
                    }
                    None => {
                        control.properties.remove("cac_texture");
                    }
                }
                self.screen()?.goto(assets, "locationselect")?;
            }
            _ => {}
        }
        Ok(())
    }

    /// `UpdatePartyPlayChallengeInfo` 006ea7: every game `off`, the chosen one `on` with its title
    /// and description. Its `mLocationselect.mIcon` and `mEventtitle` lines name clips the movie
    /// never places, so they do nothing.
    fn update_game(&mut self, assets: &Assets, game: Game) -> RenderResult<()> {
        let (item, title, description) = match game {
            Game::SpotBattle => ("item0", "ID_PARTYPLAY_SPOT_BATTLE_TITLE", "ID_PARTYPLAY_SPOT_BATTLE_DESC"),
            Game::Skate => ("item1", "ID_PARTYPLAY_SKATE_TITLE", "ID_PARTYPLAY_SKATE_DESC"),
            Game::Hom => ("item2", "ID_PARTYPLAY_HOM_TITLE", "ID_PARTYPLAY_HOM_DESC"),
        };
        let title: Vec<u16> = assets.localize(title)?.encode_utf16().collect();
        let description: Vec<u16> = assets.localize(description)?.encode_utf16().collect();
        let events = self.screen()?.named(&["mEventselect"])?;
        for index in ["item0", "item1", "item2"] {
            events.named(&[index])?.goto(assets, "off")?;
        }
        events.named(&["mEventinfo", "titleText"])?.text = Some(title);
        events.named(&["mEventinfo", "infoText"])?.text = Some(description);
        events.named(&[item])?.goto(assets, "on")?;
        Ok(())
    }

    pub fn tick(&mut self, assets: &Assets, letterbox: &mut LetterBox) -> RenderResult<()> {
        if !self.root.visible {
            return Ok(());
        }
        self.root.tick(assets)?;
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                // Frame 30: ScreenManager.IntroComplete, EnableLetterBoxInfo(true).
                "PartyPlayIntroComplete" => {
                    self.intro_complete = true;
                    letterbox.enable_info(assets, true)?;
                }
                // Frame 31: EnableLetterBoxInfo(false), CycleLetterBox.
                "PartyPlayOutroBegin" => {
                    letterbox.enable_info(assets, false)?;
                    letterbox.cycle(assets)?;
                }
                // Frame 60: ScreenManager.OutroComplete.
                "PartyPlayOutroComplete" => self.root.visible = false,
                _ => return Err(format!("untranslated party play callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// The InitAction (character 140) defining `_global.partyplay`, translated by `PartyPlayMovie`.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 140, 0x6578))
}

pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        // `var screen = new partyplay(this)`; PartyPlayMovie owns the constructor.
        0x6558 => {}
        0x7148 => {
            clip.playback.stop();
            clip.calls.push("PartyPlayIntroComplete");
        }
        0x7168 => clip.calls.push("PartyPlayOutroBegin"),
        0x718c => {
            clip.playback.stop();
            clip.calls.push("PartyPlayOutroComplete");
        }
        _ => return Err(format!("untranslated party play APT action {offset:#x}").into()),
    }
    Ok(())
}
