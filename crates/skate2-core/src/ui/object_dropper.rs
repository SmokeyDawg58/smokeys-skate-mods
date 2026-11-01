//! PC-only Object Dropper (laid out after Skate 3's park editor, with Skate 2 props): the props
//! it offers, where a drop lands, and its on-screen panel, which the session sets while the
//! editor is open and the renderer draws in Skate 2's menu colours.
use std::sync::Mutex;
use std::sync::atomic::{AtomicU16, Ordering};

/// The editor's panel: a title bar, an optional tab strip, a list (selected row marked), a few
/// lines of information and the button hints along the bottom.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Menu {
    pub title: String,
    /// Tab names and the open one (the triggers change it).
    pub tabs: Option<(Vec<String>, usize)>,
    /// List rows: the text and a count drawn after it in Skate 2's cash green.
    pub rows: Vec<(String, Option<usize>)>,
    pub selected: Option<usize>,
    pub info: Vec<String>,
    /// Button (A, B, X, Y, LB, RB, LT, RT, LS, RS, L3, D-pad) and what it does.
    pub hints: Vec<(&'static str, String)>,
    /// The crosshair in the middle of the screen.
    pub crosshair: bool,
}

static MENU: Mutex<Option<Menu>> = Mutex::new(None);

/// While the editor is open: its panel, drawn until cleared.
pub(crate) fn set_menu(menu: Option<Menu>) {
    if let Ok(mut shown) = MENU.lock() {
        *shown = menu;
    }
}

/// The editor's panel while it is open.
#[must_use]
pub fn menu() -> Option<Menu> {
    MENU.lock().ok()?.clone()
}

static SKATE3_BOARD_WEIGHT: AtomicU16 = AtomicU16::new(EditorSettings::DEFAULTS[EditorSettings::BOARD_WEIGHT]);

/// Skate 3 physics only: a dropped board's weight as a share of the player's board (the
/// park editor's Board weight setting).
#[must_use]
pub(crate) fn skate3_board_weight() -> f32 {
    f32::from(SKATE3_BOARD_WEIGHT.load(Ordering::Relaxed)) / 100.0
}

static SKATE3_PLAYER_WEIGHT: AtomicU16 = AtomicU16::new(100);
static SKATE3_PLAYER_BOARD_WEIGHT: AtomicU16 = AtomicU16::new(100);

/// Skate 3 physics only: how heavy the player (its body) and the player's own board are against
/// the movable props they hit, as a share of their own weight (the park editor's settings).
#[must_use]
pub(crate) fn skate3_player_weights() -> (f32, f32) {
    let share = |weight: &AtomicU16| f32::from(weight.load(Ordering::Relaxed)) / 100.0;
    #[cfg(test)]
    if let Ok(value) = std::env::var("SKATE3_TEST_WEIGHTS") {
        // "player,board" percent, for the corpus.
        let mut parts = value.split(',').filter_map(|part| part.trim().parse::<f32>().ok());
        if let (Some(player), Some(board)) = (parts.next(), parts.next()) {
            return (player / 100.0, board / 100.0);
        }
    }
    (share(&SKATE3_PLAYER_WEIGHT), share(&SKATE3_PLAYER_BOARD_WEIGHT))
}

/// The park editor's speeds, each a percentage of its default, kept in `park-editor.json`
/// beside `gameplay.json`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EditorSettings {
    pub(crate) percent: [u16; EditorSettings::ROWS.len()],
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self { percent: Self::DEFAULTS }
    }
}

impl EditorSettings {
    /// File key and label of each setting, in the settings page's order.
    pub(crate) const ROWS: [(&'static str, &'static str); 9] = [
        ("camera_move", "Camera move speed"),
        ("camera_look", "Camera look speed"),
        ("prop_move", "Prop move speed"),
        ("spin", "90 degree spin speed"),
        ("fine_turn", "Fine turn speed"),
        ("board_weight", "Dropped board weight (Skate 3)"),
        ("player_weight", "Player weight (Skate 3)"),
        ("player_board_weight", "Your board weight (Skate 3)"),
        ("theme", "Skate 3 piece theme"),
    ];
    /// Each setting's default: the speeds at 100%, a dropped board in Skate 3 physics at a
    /// quarter of the player's board (play-tested tuning).
    /// The theme row holds a `THEMES` index, not a percentage.
    pub(crate) const DEFAULTS: [u16; Self::ROWS.len()] = [100, 100, 100, 100, 100, 25, 100, 100, 0];
    pub(crate) const CAMERA_MOVE: usize = 0;
    pub(crate) const CAMERA_LOOK: usize = 1;
    pub(crate) const PROP_MOVE: usize = 2;
    pub(crate) const SPIN: usize = 3;
    pub(crate) const FINE_TURN: usize = 4;
    pub(crate) const BOARD_WEIGHT: usize = 5;
    pub(crate) const PLAYER_WEIGHT: usize = 6;
    pub(crate) const PLAYER_BOARD_WEIGHT: usize = 7;
    pub(crate) const THEME: usize = 8;

    /// A setting's range and how much each D-pad press changes it (percent).
    pub(crate) const fn limits(row: usize) -> (u16, u16, u16) {
        match row {
            Self::BOARD_WEIGHT => (5, 200, 5),
            Self::PLAYER_WEIGHT | Self::PLAYER_BOARD_WEIGHT => (25, 1000, 25),
            Self::THEME => (0, THEMES.len() as u16 - 1, 1),
            _ => (10, 300, 10),
        }
    }

    /// Hands the board weight to Skate 3 physics, which reads it as it builds a dropped
    /// board's solid every tick.
    pub(crate) fn publish(&self) {
        SKATE3_BOARD_WEIGHT.store(self.percent[Self::BOARD_WEIGHT], Ordering::Relaxed);
        SKATE3_PLAYER_WEIGHT.store(self.percent[Self::PLAYER_WEIGHT], Ordering::Relaxed);
        SKATE3_PLAYER_BOARD_WEIGHT.store(self.percent[Self::PLAYER_BOARD_WEIGHT], Ordering::Relaxed);
    }

    /// A setting as a multiplier of its default.
    pub(crate) fn scale(&self, row: usize) -> f32 {
        f32::from(self.percent[row]) / 100.0
    }

    /// The theme the object list takes Skate 3 pieces in (`THEMES` index).
    pub(crate) fn theme(&self) -> usize {
        usize::from(self.percent[Self::THEME]).min(THEMES.len() - 1)
    }

    /// A setting as the settings page shows it.
    pub(crate) fn value_text(&self, row: usize) -> String {
        if row == Self::THEME { THEMES[self.theme()].to_owned() } else { format!("{}%", self.percent[row]) }
    }

    fn path() -> std::io::Result<std::path::PathBuf> {
        let root = std::env::var_os("SKATE2_SAVE_ROOT")
            .map_or_else(std::env::current_dir, |root| Ok(std::path::PathBuf::from(root)))?;
        Ok(root.join("park-editor.json"))
    }

    /// The saved settings; the defaults for a missing file or any value it cannot read.
    pub(crate) fn load() -> Self {
        let mut settings = Self::default();
        let Ok(text) = Self::path().and_then(std::fs::read_to_string) else {
            return settings;
        };
        for line in text.lines() {
            let Some((key, value)) = line.split_once(':') else { continue };
            let key = key.trim().trim_matches('"');
            let value = value.trim().trim_end_matches(',').trim();
            if let (Some(row), Ok(value)) = (Self::ROWS.iter().position(|(name, _)| *name == key), value.parse::<u16>()) {
                let (min, max, _) = Self::limits(row);
                settings.percent[row] = value.clamp(min, max);
            }
        }
        settings
    }

    pub(crate) fn save(&self) -> std::io::Result<()> {
        let lines: Vec<String> = Self::ROWS
            .iter()
            .zip(self.percent)
            .map(|((key, _), value)| format!("  \"{key}\": {value}"))
            .collect();
        std::fs::write(Self::path()?, format!("{{\n{}\n}}\n", lines.join(",\n")))
    }
}

/// One prop type's settings (Start while holding it): spawned static (immovable) rather than
/// dynamic, and added to the session marker as it is placed. Kept by catalog name in
/// `park-editor-props.json` beside `park-editor.json`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PropSettings {
    pub(crate) fixed: bool,
    pub(crate) marker: bool,
    /// A character's animation: the file (`ANIMATION_FILES` index; 0, the character viewer's)
    /// and the clip in it (0 is the viewer's idle).
    pub(crate) anim_file: u16,
    pub(crate) clip: u16,
}

/// Every animation database a placed character can play from: the character viewer's, the
/// create-a-skater gestures and viewers, each marquee character's, then every NIS scene's (the
/// trick guide last). Only databases whose skeleton names all 148 marquee bones: the crowds'
/// pedestrian skeleton (43) and the vehicles' would leave bones on the hips.
pub(crate) const ANIMATION_FILES: &[&str] = &[
    "data\\anim\\marquee_viewer.abin",
    "data\\anim\\gesture\\cac_female_airguitar.abin",
    "data\\anim\\gesture\\cac_female_airplane.abin",
    "data\\anim\\gesture\\cac_female_axe.abin",
    "data\\anim\\gesture\\cac_female_boxing.abin",
    "data\\anim\\gesture\\cac_female_bruce.abin",
    "data\\anim\\gesture\\cac_female_clothesline.abin",
    "data\\anim\\gesture\\cac_female_devil.abin",
    "data\\anim\\gesture\\cac_female_doublegun.abin",
    "data\\anim\\gesture\\cac_female_dunno.abin",
    "data\\anim\\gesture\\cac_female_fingerpoint.abin",
    "data\\anim\\gesture\\cac_female_fingerwag.abin",
    "data\\anim\\gesture\\cac_female_fists.abin",
    "data\\anim\\gesture\\cac_female_flex.abin",
    "data\\anim\\gesture\\cac_female_fliptable.abin",
    "data\\anim\\gesture\\cac_female_fonz.abin",
    "data\\anim\\gesture\\cac_female_freedom.abin",
    "data\\anim\\gesture\\cac_female_fu.abin",
    "data\\anim\\gesture\\cac_female_getoutahere.abin",
    "data\\anim\\gesture\\cac_female_paper.abin",
    "data\\anim\\gesture\\cac_female_peace.abin",
    "data\\anim\\gesture\\cac_female_pumphigh.abin",
    "data\\anim\\gesture\\cac_female_pumplow.abin",
    "data\\anim\\gesture\\cac_female_push.abin",
    "data\\anim\\gesture\\cac_female_raisetheroof.abin",
    "data\\anim\\gesture\\cac_female_rock.abin",
    "data\\anim\\gesture\\cac_female_scissors.abin",
    "data\\anim\\gesture\\cac_female_shaka.abin",
    "data\\anim\\gesture\\cac_female_shrug.abin",
    "data\\anim\\gesture\\cac_female_sky.abin",
    "data\\anim\\gesture\\cac_female_snap.abin",
    "data\\anim\\gesture\\cac_female_spock.abin",
    "data\\anim\\gesture\\cac_female_throwarms.abin",
    "data\\anim\\gesture\\cac_female_thumbsdown.abin",
    "data\\anim\\gesture\\cac_female_wave.abin",
    "data\\anim\\gesture\\cac_female_what.abin",
    "data\\anim\\gesture\\cac_female_wings.abin",
    "data\\anim\\gesture\\cac_male_airguitar.abin",
    "data\\anim\\gesture\\cac_male_airplane.abin",
    "data\\anim\\gesture\\cac_male_axe.abin",
    "data\\anim\\gesture\\cac_male_boxing.abin",
    "data\\anim\\gesture\\cac_male_bruce.abin",
    "data\\anim\\gesture\\cac_male_clothesline.abin",
    "data\\anim\\gesture\\cac_male_devil.abin",
    "data\\anim\\gesture\\cac_male_doublegun.abin",
    "data\\anim\\gesture\\cac_male_dunno.abin",
    "data\\anim\\gesture\\cac_male_fingerpoint.abin",
    "data\\anim\\gesture\\cac_male_fingerwag.abin",
    "data\\anim\\gesture\\cac_male_fists.abin",
    "data\\anim\\gesture\\cac_male_flex.abin",
    "data\\anim\\gesture\\cac_male_fliptable.abin",
    "data\\anim\\gesture\\cac_male_fonz.abin",
    "data\\anim\\gesture\\cac_male_freedom.abin",
    "data\\anim\\gesture\\cac_male_fu.abin",
    "data\\anim\\gesture\\cac_male_getoutahere.abin",
    "data\\anim\\gesture\\cac_male_paper.abin",
    "data\\anim\\gesture\\cac_male_peace.abin",
    "data\\anim\\gesture\\cac_male_pumphigh.abin",
    "data\\anim\\gesture\\cac_male_pumplow.abin",
    "data\\anim\\gesture\\cac_male_push.abin",
    "data\\anim\\gesture\\cac_male_raisetheroof.abin",
    "data\\anim\\gesture\\cac_male_rock.abin",
    "data\\anim\\gesture\\cac_male_scissors.abin",
    "data\\anim\\gesture\\cac_male_shaka.abin",
    "data\\anim\\gesture\\cac_male_shrug.abin",
    "data\\anim\\gesture\\cac_male_sky.abin",
    "data\\anim\\gesture\\cac_male_snap.abin",
    "data\\anim\\gesture\\cac_male_spock.abin",
    "data\\anim\\gesture\\cac_male_throwarms.abin",
    "data\\anim\\gesture\\cac_male_thumbsdown.abin",
    "data\\anim\\gesture\\cac_male_wave.abin",
    "data\\anim\\gesture\\cac_male_what.abin",
    "data\\anim\\gesture\\cac_male_wings.abin",
    "data\\anim\\cac_edit_female.abin",
    "data\\anim\\cac_edit_male.abin",
    "data\\anim\\cac_viewer_female.abin",
    "data\\anim\\cac_viewer_male.abin",
    "data\\scene\\anim\\01_ProChall_Appleyard1.abin",
    "data\\scene\\anim\\01_ProChall_Female1.abin",
    "data\\scene\\anim\\01_ProChall_Player1.abin",
    "data\\scene\\anim\\01_ProChall_Reda1.abin",
    "data\\scene\\anim\\02_ProChall_Barbee1.abin",
    "data\\scene\\anim\\02_ProChall_Female1.abin",
    "data\\scene\\anim\\02_ProChall_Player1.abin",
    "data\\scene\\anim\\02_ProChall_Reda1.abin",
    "data\\scene\\anim\\03_ProChall_Busenitz1.abin",
    "data\\scene\\anim\\03_ProChall_Female1.abin",
    "data\\scene\\anim\\03_ProChall_Player1.abin",
    "data\\scene\\anim\\03_ProChall_Reda1.abin",
    "data\\scene\\anim\\04_ProChall_Cardiel1.abin",
    "data\\scene\\anim\\04_ProChall_Female1.abin",
    "data\\scene\\anim\\04_ProChall_Player1.abin",
    "data\\scene\\anim\\04_ProChall_Reda1.abin",
    "data\\scene\\anim\\04_ProChall_shingo1.abin",
    "data\\scene\\anim\\05_ProChall_Carroll1.abin",
    "data\\scene\\anim\\05_ProChall_Female1.abin",
    "data\\scene\\anim\\05_ProChall_Koston1.abin",
    "data\\scene\\anim\\05_ProChall_Player1.abin",
    "data\\scene\\anim\\06_ProChall_ChrisCole1.abin",
    "data\\scene\\anim\\06_ProChall_Female1.abin",
    "data\\scene\\anim\\06_ProChall_Player1.abin",
    "data\\scene\\anim\\06_ProChall_Reda1.abin",
    "data\\scene\\anim\\07_ProChall_ChrisHaslam1.abin",
    "data\\scene\\anim\\07_ProChall_Female1.abin",
    "data\\scene\\anim\\07_ProChall_Player1.abin",
    "data\\scene\\anim\\07_ProChall_Reda1.abin",
    "data\\scene\\anim\\08a_ProChall_Female1.abin",
    "data\\scene\\anim\\08a_ProChall_JasonDill1.abin",
    "data\\scene\\anim\\08a_ProChall_Player1.abin",
    "data\\scene\\anim\\08a_ProChall_RobDyrdek1.abin",
    "data\\scene\\anim\\09_ProChall_Female1.abin",
    "data\\scene\\anim\\09_ProChall_JakeBrown1.abin",
    "data\\scene\\anim\\09_ProChall_Player1.abin",
    "data\\scene\\anim\\09_ProChall_Reda1.abin",
    "data\\scene\\anim\\10_ProChall_Female1.abin",
    "data\\scene\\anim\\10_ProChall_MarkJohnson1.abin",
    "data\\scene\\anim\\10_ProChall_Player1.abin",
    "data\\scene\\anim\\11a_ProChall_Female1.abin",
    "data\\scene\\anim\\11a_ProChall_Navarette1.abin",
    "data\\scene\\anim\\11a_ProChall_Player1.abin",
    "data\\scene\\anim\\12_ProChall_Chalmers1.abin",
    "data\\scene\\anim\\12_ProChall_Female1.abin",
    "data\\scene\\anim\\12_ProChall_Player1.abin",
    "data\\scene\\anim\\12_ProChall_puig1.abin",
    "data\\scene\\anim\\13_ProChall_Female1.abin",
    "data\\scene\\anim\\13_ProChall_Player1.abin",
    "data\\scene\\anim\\13_ProChall_Rattray1.abin",
    "data\\scene\\anim\\13_ProChall_Reda1.abin",
    "data\\scene\\anim\\14a_ProChall_Braydon1.abin",
    "data\\scene\\anim\\14a_ProChall_Female1.abin",
    "data\\scene\\anim\\14a_ProChall_Player1.abin",
    "data\\scene\\anim\\14a_ProChall_Reda1.abin",
    "data\\scene\\anim\\14b_ProChall_Braydon1.abin",
    "data\\scene\\anim\\14b_ProChall_Female1.abin",
    "data\\scene\\anim\\14b_ProChall_Player1.abin",
    "data\\scene\\anim\\14b_ProChall_TK1.abin",
    "data\\scene\\anim\\ArrivalofMikey_Cap1.abin",
    "data\\scene\\anim\\ArrivalofMikey_Female1.abin",
    "data\\scene\\anim\\ArrivalofMikey_Mikey1.abin",
    "data\\scene\\anim\\ArrivalofMikey_Player1.abin",
    "data\\scene\\anim\\Audio_Sync_Test1.abin",
    "data\\scene\\anim\\BlackArrival_Atiba1.abin",
    "data\\scene\\anim\\BlackArrival_BigBlack1.abin",
    "data\\scene\\anim\\BlackArrival_Female1.abin",
    "data\\scene\\anim\\BlackArrival_Player1.abin",
    "data\\scene\\anim\\BlackArrival_SG11.abin",
    "data\\scene\\anim\\BlackArrival_SG21.abin",
    "data\\scene\\anim\\BlackArrival_SUV1.abin",
    "data\\scene\\anim\\corex_spotfilm011.abin",
    "data\\scene\\anim\\corex_spotfilm021.abin",
    "data\\scene\\anim\\Cuz_Bowl_Series_bg041.abin",
    "data\\scene\\anim\\Cuz_Bowl_Series_bg051.abin",
    "data\\scene\\anim\\Cuz_Bowl_Series_Cuz1.abin",
    "data\\scene\\anim\\Cuz_Bowl_Series_Female1.abin",
    "data\\scene\\anim\\Cuz_Bowl_Series_Player1.abin",
    "data\\scene\\anim\\Cuz_Bowl_Series_Shingo1.abin",
    "data\\scene\\anim\\Cuz_Bowl_TableChair1.abin",
    "data\\scene\\anim\\Fundraiser_Dyrdek1.abin",
    "data\\scene\\anim\\Fundraiser_Female1.abin",
    "data\\scene\\anim\\Fundraiser_Player1.abin",
    "data\\scene\\anim\\Fundraiser_Tallyboard1.abin",
    "data\\scene\\anim\\goto_school21.abin",
    "data\\scene\\anim\\goto_school31.abin",
    "data\\scene\\anim\\goto_school_011.abin",
    "data\\scene\\anim\\goto_school_041.abin",
    "data\\scene\\anim\\goto_school_051.abin",
    "data\\scene\\anim\\goto_school_50501.abin",
    "data\\scene\\anim\\goto_Slappy_011.abin",
    "data\\scene\\anim\\goto_Slappy_021.abin",
    "data\\scene\\anim\\goto_Slappy_031.abin",
    "data\\scene\\anim\\goto_Slappy_041.abin",
    "data\\scene\\anim\\GVR_Goofy_011.abin",
    "data\\scene\\anim\\GVR_Goofy_031.abin",
    "data\\scene\\anim\\GVR_Goofy_041.abin",
    "data\\scene\\anim\\GVR_Reg_011.abin",
    "data\\scene\\anim\\GVR_Reg_021.abin",
    "data\\scene\\anim\\GVR_Reg_031.abin",
    "data\\scene\\anim\\Mag_Cover_Atiba1.abin",
    "data\\scene\\anim\\Mag_Cover_BigBlack1.abin",
    "data\\scene\\anim\\Mag_Cover_Female1.abin",
    "data\\scene\\anim\\Mag_Cover_guard1.abin",
    "data\\scene\\anim\\Mag_Cover_Player1.abin",
    "data\\scene\\anim\\Mag_Cover_Reda1.abin",
    "data\\scene\\anim\\Meet_Freeway_Female1.abin",
    "data\\scene\\anim\\Meet_Freeway_Freeway1.abin",
    "data\\scene\\anim\\Meet_Freeway_Player1.abin",
    "data\\scene\\anim\\Meet_Freeway_Reda1.abin",
    "data\\scene\\anim\\Meet_Freeway_Sammy1.abin",
    "data\\scene\\anim\\Meet_Freeway_Shingo1.abin",
    "data\\scene\\anim\\NIS_flythroughtest_S01_01.abin",
    "data\\scene\\anim\\Online_Race2_Skater11.abin",
    "data\\scene\\anim\\Online_Race2_Skater21.abin",
    "data\\scene\\anim\\Online_Race2_Skater31.abin",
    "data\\scene\\anim\\Online_Race2_Skater41.abin",
    "data\\scene\\anim\\Online_Race2_Skater51.abin",
    "data\\scene\\anim\\Online_Race2_Skater61.abin",
    "data\\scene\\anim\\Online_Race2_Skater71.abin",
    "data\\scene\\anim\\Online_Race2_Skater81.abin",
    "data\\scene\\anim\\Online_Race_Skater11.abin",
    "data\\scene\\anim\\Online_Race_Skater21.abin",
    "data\\scene\\anim\\Online_Race_Skater31.abin",
    "data\\scene\\anim\\Online_Race_Skater41.abin",
    "data\\scene\\anim\\Online_Race_Skater51.abin",
    "data\\scene\\anim\\Online_Race_Skater61.abin",
    "data\\scene\\anim\\Online_Race_Skater71.abin",
    "data\\scene\\anim\\Online_Race_Skater81.abin",
    "data\\scene\\anim\\OTS_f_1a_02_artstructure_in_A01_01.abin",
    "data\\scene\\anim\\OTS_f_1a_03_wall_intro_A01_01.abin",
    "data\\scene\\anim\\Perm_Pool_1_Buz1.abin",
    "data\\scene\\anim\\Perm_Pool_1_cardiel1.abin",
    "data\\scene\\anim\\Perm_Pool_1_JakeBrown1.abin",
    "data\\scene\\anim\\Perm_Pool_1_Nav1.abin",
    "data\\scene\\anim\\Perm_Pool_2_Brown1.abin",
    "data\\scene\\anim\\Perm_Pool_2_Burnett1.abin",
    "data\\scene\\anim\\Perm_Pool_2_Busenitz1.abin",
    "data\\scene\\anim\\Perm_Pool_2_Cardiel1.abin",
    "data\\scene\\anim\\Perm_Pool_2_Female1.abin",
    "data\\scene\\anim\\Perm_Pool_2_Nav1.abin",
    "data\\scene\\anim\\Perm_Pool_2_Player1.abin",
    "data\\scene\\anim\\Perm_Pool_3_Burnett1.abin",
    "data\\scene\\anim\\Perm_Pool_3_Female1.abin",
    "data\\scene\\anim\\Perm_Pool_3_Player1.abin",
    "data\\scene\\anim\\Perm_Pool_3_Reda1.abin",
    "data\\scene\\anim\\Perm_Pool_3_Sammy1.abin",
    "data\\scene\\anim\\photochallenge_test1_A02_01.abin",
    "data\\scene\\anim\\photochallenge_test1_A03_01.abin",
    "data\\scene\\anim\\photochallenge_test1_A04_01.abin",
    "data\\scene\\anim\\photochallenge_test1_A05_01.abin",
    "data\\scene\\anim\\photochallenge_test1_A06_01.abin",
    "data\\scene\\anim\\photochallenge_test1_A07_01.abin",
    "data\\scene\\anim\\photochallenge_test1_A08_01.abin",
    "data\\scene\\anim\\photochallenge_test1_A09_01.abin",
    "data\\scene\\anim\\photochallenge_testshot1_A01_01.abin",
    "data\\scene\\anim\\PlanB_Complete_DannyWay1.abin",
    "data\\scene\\anim\\PlanB_Complete_Female1.abin",
    "data\\scene\\anim\\PlanB_Complete_Player1.abin",
    "data\\scene\\anim\\PlanB_Complete_Reda1.abin",
    "data\\scene\\anim\\PlanB_Intro_DannyWay1.abin",
    "data\\scene\\anim\\PlanB_Intro_Female1.abin",
    "data\\scene\\anim\\PlanB_Intro_PatDuffy1.abin",
    "data\\scene\\anim\\PlanB_Intro_PJLadd1.abin",
    "data\\scene\\anim\\PlanB_Intro_Player1.abin",
    "data\\scene\\anim\\ProsArrival_Atiba1.abin",
    "data\\scene\\anim\\ProsArrival_Carroll1.abin",
    "data\\scene\\anim\\ProsArrival_Female1.abin",
    "data\\scene\\anim\\ProsArrival_Koston1.abin",
    "data\\scene\\anim\\ProsArrival_Player1.abin",
    "data\\scene\\anim\\ProsArrival_Reda1.abin",
    "data\\scene\\anim\\ProsArrival_Slappy1.abin",
    "data\\scene\\anim\\ProsLeave_Atiba1.abin",
    "data\\scene\\anim\\ProsLeave_Carroll1.abin",
    "data\\scene\\anim\\ProsLeave_Female1.abin",
    "data\\scene\\anim\\ProsLeave_Koston1.abin",
    "data\\scene\\anim\\ProsLeave_Player1.abin",
    "data\\scene\\anim\\ProsLeave_Reda1.abin",
    "data\\scene\\anim\\ProsLeave_Slappy1.abin",
    "data\\scene\\anim\\RaceAnim1_S01_01.abin",
    "data\\scene\\anim\\RaceAnim2_S01_01.abin",
    "data\\scene\\anim\\RaceAnim3_S01_01.abin",
    "data\\scene\\anim\\RaceAnim4_S01_01.abin",
    "data\\scene\\anim\\RaceEnd_TiredSkaters_Skater11.abin",
    "data\\scene\\anim\\RaceEnd_TiredSkaters_Skater21.abin",
    "data\\scene\\anim\\RaceEnd_TiredSkaters_Skater31.abin",
    "data\\scene\\anim\\RaceEnd_TiredSkaters_Skater41.abin",
    "data\\scene\\anim\\RaceStarts_1_A01_01.abin",
    "data\\scene\\anim\\RaceStarts_1_S01_01.abin",
    "data\\scene\\anim\\RaceStarts_1_S02_01.abin",
    "data\\scene\\anim\\RaceStarts_1_S03_01.abin",
    "data\\scene\\anim\\RaceStarts_1_S04_01.abin",
    "data\\scene\\anim\\RaceStarts_2_Slap02_Cuz1.abin",
    "data\\scene\\anim\\RaceStarts_2_Slap02_Skater11.abin",
    "data\\scene\\anim\\RaceStarts_2_Slap02_Skater21.abin",
    "data\\scene\\anim\\RaceStarts_2_Slap02_Skater31.abin",
    "data\\scene\\anim\\RaceStarts_2_Slap02_Skater41.abin",
    "data\\scene\\anim\\RaceStarts_2_Slap_Cuz1.abin",
    "data\\scene\\anim\\RaceStarts_2_Slap_Skater11.abin",
    "data\\scene\\anim\\RaceStarts_2_Slap_Skater21.abin",
    "data\\scene\\anim\\RaceStarts_2_Slap_Skater31.abin",
    "data\\scene\\anim\\RaceStarts_2_Slap_Skater41.abin",
    "data\\scene\\anim\\RaceStarts_3_TalkingSmack_Cuz1.abin",
    "data\\scene\\anim\\RaceStarts_3_TalkingSmack_Skater11.abin",
    "data\\scene\\anim\\RaceStarts_3_TalkingSmack_Skater21.abin",
    "data\\scene\\anim\\RaceStarts_3_TalkingSmack_Skater31.abin",
    "data\\scene\\anim\\RaceStarts_3_TalkingSmack_Skater41.abin",
    "data\\scene\\anim\\RCR_Haslam_071.abin",
    "data\\scene\\anim\\RCR_Player_051.abin",
    "data\\scene\\anim\\RCR_Scene_03_Skater011.abin",
    "data\\scene\\anim\\RCR_Scene_03_Skater021.abin",
    "data\\scene\\anim\\RCR_Scene_03_Skater031.abin",
    "data\\scene\\anim\\RCR_Security2_051.abin",
    "data\\scene\\anim\\RCR_Security_051.abin",
    "data\\scene\\anim\\RCR_Skater01_061.abin",
    "data\\scene\\anim\\RCR_Skater02_061.abin",
    "data\\scene\\anim\\SanVanPark_BG_Skater_011.abin",
    "data\\scene\\anim\\SanVanPark_BG_Skater_021.abin",
    "data\\scene\\anim\\SanVanPark_BG_Skater_031.abin",
    "data\\scene\\anim\\SanVanPark_DannyWay1.abin",
    "data\\scene\\anim\\SanVanPark_Dyrdek1.abin",
    "data\\scene\\anim\\SanVanPark_Female1.abin",
    "data\\scene\\anim\\SanVanPark_Player1.abin",
    "data\\scene\\anim\\Sebs_Street_Female1.abin",
    "data\\scene\\anim\\Sebs_Street_Mongoboy1.abin",
    "data\\scene\\anim\\Sebs_Street_Player1.abin",
    "data\\scene\\anim\\Sebs_Street_Reda1.abin",
    "data\\scene\\anim\\Sebs_Street_Seb1.abin",
    "data\\scene\\anim\\Sebs_Street_Shingo1.abin",
    "data\\scene\\anim\\ShutDown_Atiba1.abin",
    "data\\scene\\anim\\ShutDown_Braydon1.abin",
    "data\\scene\\anim\\ShutDown_Female1.abin",
    "data\\scene\\anim\\ShutDown_Guard1.abin",
    "data\\scene\\anim\\ShutDown_Player1.abin",
    "data\\scene\\anim\\ShutDown_TK1.abin",
    "data\\scene\\anim\\SlappyEnder_Female1.abin",
    "data\\scene\\anim\\SlappyEnder_Player1.abin",
    "data\\scene\\anim\\SlappyEnder_Reda1.abin",
    "data\\scene\\anim\\SlappyEnder_Slappy1.abin",
    "data\\scene\\anim\\SlappyEnder_Slappy21.abin",
    "data\\scene\\anim\\TestAnim_2010_freeway1.abin",
    "data\\scene\\anim\\TestAnim_2010_mike1.abin",
    "data\\scene\\anim\\TestAnim_2010_player1.abin",
    "data\\scene\\anim\\TestAnim_BumpsHiBacknock0N1.abin",
    "data\\scene\\anim\\TestAnim_rob_dyrdek_face1.abin",
    "data\\scene\\anim\\TheRealGame_Dyrdek1.abin",
    "data\\scene\\anim\\TheRealGame_Female1.abin",
    "data\\scene\\anim\\TheRealGame_Haslam1.abin",
    "data\\scene\\anim\\TheRealGame_Player1.abin",
    "data\\scene\\anim\\TheRealGame_Reda1.abin",
    "data\\scene\\anim\\TheRealGame_Wipeout1.abin",
    "data\\scene\\anim\\ThrasherCover_Burnett1.abin",
    "data\\scene\\anim\\ThrasherCover_Female1.abin",
    "data\\scene\\anim\\ThrasherCover_Player1.abin",
    "data\\scene\\anim\\ThrasherCover_Reda1.abin",
    "data\\scene\\anim\\ThrasherCover_Sammy1.abin",
    "data\\scene\\anim\\tutorial_m_1a_05_photo_rail_A01_01.abin",
    "data\\scene\\anim\\tutorial_m_1a_05_photo_rail_A02_01.abin",
    "data\\scene\\anim\\tutorial_m_1a_goto_slappys_A01_01.abin",
    "data\\scene\\anim\\VertChallenge_BG_Buz1.abin",
    "data\\scene\\anim\\VertChallenge_BG_carrol1.abin",
    "data\\scene\\anim\\VertChallenge_BG_Dyrdek1.abin",
    "data\\scene\\anim\\VertChallenge_DannyWay1.abin",
    "data\\scene\\anim\\VertChallenge_Female1.abin",
    "data\\scene\\anim\\VertChallenge_Player1.abin",
    "data\\scene\\anim\\VertChallenge_Reda1.abin",
    "data\\scene\\anim\\WelcomeBack_Female1.abin",
    "data\\scene\\anim\\WelcomeBack_Player1.abin",
    "data\\scene\\anim\\WelcomeBack_Reda1.abin",
    "data\\scene\\anim\\WelcomeBack_Slappy1.abin",
    "data\\scene\\anim\\Win_Big_1_Autographs_Pen1.abin",
    "data\\scene\\anim\\Win_Big_1_Autographs_Skater11.abin",
    "data\\scene\\anim\\Win_Big_1_Autographs_Skater21.abin",
    "data\\scene\\anim\\Win_Big_1_Autographs_Skater31.abin",
    "data\\scene\\anim\\Win_Big_1_Autographs_Skater41.abin",
    "data\\scene\\anim\\Win_Big_1_Autographs_Skater51.abin",
    "data\\scene\\anim\\Win_Big_1_Autographs_Skater61.abin",
    "data\\scene\\anim\\Win_Big_2_IMNumber1_Skater11.abin",
    "data\\scene\\anim\\Win_Big_2_IMNumber1_Skater21.abin",
    "data\\scene\\anim\\Win_Big_2_IMNumber1_Skater31.abin",
    "data\\scene\\anim\\Win_Big_2_IMNumber1_Skater41.abin",
    "data\\scene\\anim\\Win_Big_3_Podium_Podium1.abin",
    "data\\scene\\anim\\Win_Big_3_Podium_Skater11.abin",
    "data\\scene\\anim\\Win_Big_3_Podium_Skater21.abin",
    "data\\scene\\anim\\Win_Big_3_Podium_Skater31.abin",
    "data\\scene\\anim\\Win_Med_1_BoardToss_Skater11.abin",
    "data\\scene\\anim\\Win_Med_2_SurprisedWin_Skater11.abin",
    "data\\scene\\anim\\Win_Med_2_SurprisedWin_Skater21.abin",
    "data\\scene\\anim\\Win_Med_2_SurprisedWin_Skater31.abin",
    "data\\scene\\anim\\Win_Med_2_SurprisedWin_Skater41.abin",
    "data\\scene\\anim\\Win_Med_3_EnergeticWin_Skater11.abin",
    "data\\scene\\anim\\Win_Med_3_EnergeticWin_Skater21.abin",
    "data\\scene\\anim\\Win_Med_3_EnergeticWin_Skater31.abin",
    "data\\scene\\anim\\Win_Med_3_EnergeticWin_Skater41.abin",
    "data\\scene\\anim\\Win_Med_4_RaceWin_Skater11.abin",
    "data\\scene\\anim\\Win_Med_4_RaceWin_Skater21.abin",
    "data\\scene\\anim\\Win_Med_4_RaceWin_Skater31.abin",
    "data\\scene\\anim\\Win_Med_4_RaceWin_Skater41.abin",
    "data\\scene\\trickguide\\trickguide_360_hardflip_011.abin",
    "data\\scene\\trickguide\\trickguide_360_inward_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_360_pop_shuvit_011.abin",
    "data\\scene\\trickguide\\trickguide_360flip_011.abin",
    "data\\scene\\trickguide\\trickguide_airwalk_011.abin",
    "data\\scene\\trickguide\\trickguide_backflip_011.abin",
    "data\\scene\\trickguide\\trickguide_benihana_011.abin",
    "data\\scene\\trickguide\\trickguide_bindpose1.abin",
    "data\\scene\\trickguide\\trickguide_brake_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_50_50_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_5_0_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_beanplant_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_blunt_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_blunt_nosegrab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_board_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_boneless_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_crooked_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_crooked_tail_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_fingerflip_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_grab_50_50_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_grab_bs_crooked_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_grab_bs_footplant_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_grab_fs_crooked_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_grab_fs_footplant_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_grab_nose_grind_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_grab_to_mute_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_ground_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_handplant_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_handplant_back_foot_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_handplant_crossbone_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_handplant_front_foot_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_handplant_method_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_handplant_no_foot_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_handplant_tuck_knee_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_handplant_tweak_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_lip_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_nose_grind_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_nose_slide_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_nose_slide_tail_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_noseblunt_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_noseblunt_21.abin",
    "data\\scene\\trickguide\\trickguide_bs_noseblunt_tail_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_nosegrind_tailgrab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_one_foot_behind_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_over_crooks_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_over_crooks_bs_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_over_crooks_tail_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_over_willy_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_salad_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_salad_fs_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_salad_nose_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_shifty_crail_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_shifty_nose_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_shifty_seatbelt_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_shifty_tail_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_smith_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_tail_nose_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_tail_slide_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_tweak_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_varial_011.abin",
    "data\\scene\\trickguide\\trickguide_bs_willy_011.abin",
    "data\\scene\\trickguide\\trickguide_cannonball_011.abin",
    "data\\scene\\trickguide\\trickguide_christ_air_011.abin",
    "data\\scene\\trickguide\\trickguide_coffin_011.abin",
    "data\\scene\\trickguide\\trickguide_crail_011.abin",
    "data\\scene\\trickguide\\trickguide_crail_to_tail_011.abin",
    "data\\scene\\trickguide\\trickguide_crossbone_011.abin",
    "data\\scene\\trickguide\\trickguide_dbl_ground_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_dog_piss_011.abin",
    "data\\scene\\trickguide\\trickguide_double_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_double_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_double_kickflip_011.abin",
    "data\\scene\\trickguide\\trickguide_frontflip_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_360_pop_shuvit_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_50_50_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_5_0_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_back_foot_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_back_foot_right_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_blunt_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_blunt_nosegrab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_board_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_boneless_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_boneless_fastplant_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_crooked_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_crooked_tail_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_fastplant_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_feeble_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_fingerflip_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_grab_50_50_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_grab_5_0_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_grab_bs_footplant_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_grab_fs_footplant_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_grab_to_stale_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_ground_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_handplant_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_handplant_back_foot_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_handplant_front_foot_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_handplant_no_foot_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_handplant_nosebone_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_handplant_stiffy_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_handplant_tailbone_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_handplant_tuck_knee_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_lip_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_nose_grind_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_nose_slide_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_nose_slide_tail_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_noseblunt_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_noseblunt_tail_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_nosegrind_tailgrab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_over_crooks_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_over_crooks_bs_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_over_crooks_tail_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_over_willy_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_pop_shuvit_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_salad_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_salad_fs_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_salad_nose_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_shifty_crail_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_shifty_nose_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_shifty_seatbelt_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_shifty_tail_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_smith_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_tail_nose_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_tail_slide_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_tailbone_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_varial_011.abin",
    "data\\scene\\trickguide\\trickguide_fs_willy_011.abin",
    "data\\scene\\trickguide\\trickguide_hardflip_011.abin",
    "data\\scene\\trickguide\\trickguide_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_hippy_jump_011.abin",
    "data\\scene\\trickguide\\trickguide_inward_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_judo_011.abin",
    "data\\scene\\trickguide\\trickguide_judo_footplant_011.abin",
    "data\\scene\\trickguide\\trickguide_jump_011.abin",
    "data\\scene\\trickguide\\trickguide_karate_kick_011.abin",
    "data\\scene\\trickguide\\trickguide_kickflip_011.abin",
    "data\\scene\\trickguide\\trickguide_laser_flip_011.abin",
    "data\\scene\\trickguide\\trickguide_late_backfoot_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_late_backfoot_kickflip_011.abin",
    "data\\scene\\trickguide\\trickguide_late_fs_shuvit_011.abin",
    "data\\scene\\trickguide\\trickguide_late_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_late_kickflip_011.abin",
    "data\\scene\\trickguide\\trickguide_late_shuvit_011.abin",
    "data\\scene\\trickguide\\trickguide_melon_011.abin",
    "data\\scene\\trickguide\\trickguide_method_011.abin",
    "data\\scene\\trickguide\\trickguide_mount_dismount_011.abin",
    "data\\scene\\trickguide\\trickguide_mute_011.abin",
    "data\\scene\\trickguide\\trickguide_mute_flip_011.abin",
    "data\\scene\\trickguide\\trickguide_mute_nosebone_011.abin",
    "data\\scene\\trickguide\\trickguide_mute_tailbone_011.abin",
    "data\\scene\\trickguide\\trickguide_mute_to_bs_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_mute_tuck_knee_011.abin",
    "data\\scene\\trickguide\\trickguide_no_comply_011.abin",
    "data\\scene\\trickguide\\trickguide_no_foot_air_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_360_hardflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_360_inward_heelflip1.abin",
    "data\\scene\\trickguide\\trickguide_nollie_360_pop_shuvit_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_360flip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_double_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_double_kickflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_fs_360_pop_shuvit_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_fs_pop_shuvit_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_hardflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_inward_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_kickflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_laser_flip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_pop_shuvit_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_quad_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_quad_kickflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_triple_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_triple_kickflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_varial_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nollie_varial_kickflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nose_grab_5_0_011.abin",
    "data\\scene\\trickguide\\trickguide_nose_grab_judo_011.abin",
    "data\\scene\\trickguide\\trickguide_nose_manual_011.abin",
    "data\\scene\\trickguide\\trickguide_nosebone_011.abin",
    "data\\scene\\trickguide\\trickguide_noseflip_011.abin",
    "data\\scene\\trickguide\\trickguide_nosegrab_011.abin",
    "data\\scene\\trickguide\\trickguide_ollie_011.abin",
    "data\\scene\\trickguide\\trickguide_ollie_one_foot_back_011.abin",
    "data\\scene\\trickguide\\trickguide_ollie_one_foot_front_011.abin",
    "data\\scene\\trickguide\\trickguide_pop_shuvit_011.abin",
    "data\\scene\\trickguide\\trickguide_powerslide_011.abin",
    "data\\scene\\trickguide\\trickguide_quad_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_quad_kickflip_011.abin",
    "data\\scene\\trickguide\\trickguide_retrieve_drop_011.abin",
    "data\\scene\\trickguide\\trickguide_revert_011.abin",
    "data\\scene\\trickguide\\trickguide_rocket_air_011.abin",
    "data\\scene\\trickguide\\trickguide_seatbelt_011.abin",
    "data\\scene\\trickguide\\trickguide_seatbelt_to_nose_011.abin",
    "data\\scene\\trickguide\\trickguide_spread_eagle_011.abin",
    "data\\scene\\trickguide\\trickguide_sprint_011.abin",
    "data\\scene\\trickguide\\trickguide_stale_011.abin",
    "data\\scene\\trickguide\\trickguide_stale_flip_011.abin",
    "data\\scene\\trickguide\\trickguide_stale_nosebone_011.abin",
    "data\\scene\\trickguide\\trickguide_stale_tailbone_011.abin",
    "data\\scene\\trickguide\\trickguide_stale_to_fs_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_stale_tuck_knee_011.abin",
    "data\\scene\\trickguide\\trickguide_stiffy_011.abin",
    "data\\scene\\trickguide\\trickguide_superman_011.abin",
    "data\\scene\\trickguide\\trickguide_tail_grab_011.abin",
    "data\\scene\\trickguide\\trickguide_tail_grab_airwalk_011.abin",
    "data\\scene\\trickguide\\trickguide_tail_grab_one_foot_front_011.abin",
    "data\\scene\\trickguide\\trickguide_tail_manual_011.abin",
    "data\\scene\\trickguide\\trickguide_tailbone_011.abin",
    "data\\scene\\trickguide\\trickguide_tailflip_011.abin",
    "data\\scene\\trickguide\\trickguide_tailwalk_011.abin",
    "data\\scene\\trickguide\\trickguide_torpedo_011.abin",
    "data\\scene\\trickguide\\trickguide_triple_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_triple_kickflip_011.abin",
    "data\\scene\\trickguide\\trickguide_tuck_knee_011.abin",
    "data\\scene\\trickguide\\trickguide_varial_heelflip_011.abin",
    "data\\scene\\trickguide\\trickguide_varial_kickflip_011.abin",
    "data\\scene\\trickguide\\trickguide_walk_011.abin",
    "data\\scene\\trickguide\\trickguide_wipeout_011.abin",
];

/// An animation file's short name.
#[must_use]
pub(crate) fn animation_file_name(index: usize) -> &'static str {
    ANIMATION_FILES.get(index).map_or("?", |path| {
        let name = path.rsplit('\\').next().unwrap_or(path);
        name.strip_suffix(".abin").unwrap_or(name)
    })
}

fn prop_settings_path() -> std::io::Result<std::path::PathBuf> {
    Ok(EditorSettings::path()?.with_file_name("park-editor-props.json"))
}

/// Every prop type's settings, by `PROPS` index (a type not in the file has the defaults).
pub(crate) fn load_prop_settings() -> std::collections::BTreeMap<usize, PropSettings> {
    let mut settings = std::collections::BTreeMap::new();
    let Ok(text) = prop_settings_path().and_then(std::fs::read_to_string) else {
        return settings;
    };
    for line in text.lines() {
        let Some((key, value)) = line.rsplit_once(':') else { continue };
        let key = key.trim().trim_matches('"');
        let value = value.trim().trim_end_matches(',').trim().trim_matches('"');
        if let Some(index) = PROPS.iter().position(|entry| entry.1 == key) {
            let words: Vec<&str> = value.split(',').map(str::trim).collect();
            // "anim=<file name>#<clip>"
            let (anim_file, clip) = words
                .iter()
                .find_map(|word| {
                    let (file, clip) = word.strip_prefix("anim=")?.split_once('#')?;
                    let file = (0..ANIMATION_FILES.len()).find(|&i| animation_file_name(i).eq_ignore_ascii_case(file))?;
                    Some((u16::try_from(file).ok()?, clip.parse().ok()?))
                })
                .unwrap_or((0, 0));
            settings.insert(index, PropSettings { fixed: words.contains(&"static"), marker: words.contains(&"marker"), anim_file, clip });
        }
    }
    settings
}

pub(crate) fn save_prop_settings(settings: &std::collections::BTreeMap<usize, PropSettings>) -> std::io::Result<()> {
    let lines: Vec<String> = settings
        .iter()
        .filter(|(_, value)| **value != PropSettings::default())
        .map(|(&index, value)| {
            let anim = format!("anim={}#{}", animation_file_name(usize::from(value.anim_file)), value.clip);
            let words: Vec<&str> = [(value.fixed, "static"), (value.marker, "marker"), (value.anim_file != 0 || value.clip != 0, anim.as_str())]
                .into_iter()
                .filter(|(on, _)| *on)
                .map(|(_, word)| word)
                .collect();
            format!("  \"{}\": \"{}\"", PROPS[index].1, words.join(","))
        })
        .collect();
    std::fs::write(prop_settings_path()?, format!("{{\n{}\n}}\n", lines.join(",\n")))
}

/// The skater root's axes and translation (XYZ only; Skate 2 physics leaves W unwritten).
#[cfg(test)]
pub(crate) fn root_lanes(root: &crate::skeleton_root_transforms::SkeletonRootMatrix) -> [f32; 16] {
    let mut lanes = [0.0; 16];
    for (column, vector) in root.columns().iter().enumerate() {
        lanes[column * 4..column * 4 + 3].copy_from_slice(&vector.xyz);
    }
    lanes[15] = 1.0;
    lanes
}

/// How far ahead of the skater a prop lands (metres).
#[cfg(test)]
const DROP_DISTANCE: f32 = 3.5;

/// A mission-only prop: the part of its hull's fused mesh inside the box (world space, as the
/// hull's Spatial record gives it) its catalog entry cuts out.
#[derive(Clone, Copy, Debug)]
pub struct MissionProp {
    pub name: &'static str,
    pub hull: &'static str,
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// The mission props offered, from the hull records whose box holds that prop alone.
pub const MISSION_PROPS: &[MissionProp] = &[
    // Each type's instance whose box no other Spatial record of its hull touches (the cleanest
    // cut), from a census of every mission hull.
    MissionProp { name: "Speaker", hull: "GVR", min: [540.94, -55.43, 1129.60], max: [542.01, -53.88, 1130.68] },
    MissionProp { name: "Grandstand", hull: "GVR", min: [476.20, -57.45, 1112.91], max: [484.42, -55.14, 1120.02] },
    MissionProp { name: "Entrance banner (GVR)", hull: "GVR", min: [525.95, -57.43, 1122.72], max: [527.21, -52.00, 1129.15] },
    MissionProp { name: "Tent (black)", hull: "GVR", min: [527.92, -57.43, 1119.75], max: [530.94, -53.79, 1122.91] },
    MissionProp { name: "Tent (small)", hull: "c_1a_01_bestrick_street_props", min: [1128.44, 59.11, -163.17], max: [1131.64, 62.77, -159.93] },
    MissionProp { name: "Media tower (GVR)", hull: "GVR", min: [522.20, -57.43, 1160.95], max: [525.23, -53.41, 1163.97] },
    MissionProp { name: "Media tower (Black Box)", hull: "ol_bt_01_DT_round2_contest_pro", min: [608.86, -49.11, 540.99], max: [613.00, -45.09, 545.13] },
    MissionProp { name: "Media tower (Zero, Creature...)", hull: "c_1a_02_bestrick_street_props", min: [769.29, 31.91, -189.99], max: [772.99, 35.94, -186.29] },
    MissionProp { name: "Media tower (Anti Hero, Blind...)", hull: "c_2a_02_jam_branded_props", min: [985.68, -61.56, 965.73], max: [989.94, -57.54, 969.99] },
    MissionProp { name: "Media tower (Lakai, Girl...)", hull: "c_2a_02_street_all_props", min: [255.75, -44.57, 641.01], max: [260.03, -40.55, 645.29] },
    MissionProp { name: "Media tower (Slappy's, Thrasher...)", hull: "m_7a_01_demo_props", min: [469.60, -39.46, 443.66], max: [473.87, -35.44, 447.94] },
    MissionProp { name: "Porta-potty", hull: "c_2a_01_jam_branded_props", min: [987.23, -54.50, 717.41], max: [989.30, -51.50, 719.15] },
    MissionProp { name: "Couch", hull: "ol_bt_02_OT_round_2_contest_pr", min: [1408.83, 31.20, 275.96], max: [1411.12, 32.08, 278.41] },
    MissionProp { name: "Flag (AWS)", hull: "m_6p_01_Alien", min: [941.15, -59.71, 969.01], max: [942.11, -54.59, 969.43] },
    MissionProp { name: "Flag (Adidas)", hull: "c_2a_02_jam_branded_props", min: [981.58, -59.24, 913.92], max: [981.67, -54.11, 914.92] },
    MissionProp { name: "Flag (Almost)", hull: "c_2a_01_street_rd3_props", min: [242.01, 43.43, -249.41], max: [242.45, 48.56, -248.46] },
    MissionProp { name: "Flag (Anti Hero)", hull: "c_2a_02_jam_branded_props", min: [983.85, -61.00, 943.56], max: [984.25, -55.87, 944.52] },
    MissionProp { name: "Flag (Baker)", hull: "m_6p_01_Baker", min: [941.15, -59.71, 969.01], max: [942.11, -54.59, 969.43] },
    MissionProp { name: "Flag (Black Box)", hull: "ol_bt_01_DT_round1_contest_pro", min: [630.86, -48.62, 397.19], max: [631.79, -43.49, 397.70] },
    MissionProp { name: "Flag (Blind)", hull: "m_6p_01_Blind", min: [941.15, -59.71, 969.01], max: [942.11, -54.59, 969.43] },
    MissionProp { name: "Flag (Chocolate)", hull: "c_2a_02_street_all_props", min: [255.16, -44.46, 667.79], max: [255.93, -39.33, 668.53] },
    MissionProp { name: "Flag (Cliche)", hull: "m_6p_01_Cliche", min: [941.15, -59.71, 969.01], max: [942.11, -54.59, 969.43] },
    MissionProp { name: "Flag (Creature)", hull: "c_1a_02_bestrick_street_props", min: [740.22, 26.86, -244.99], max: [740.85, 31.98, -244.13] },
    MissionProp { name: "Flag (Element)", hull: "m_6p_01_Element", min: [941.15, -59.71, 969.01], max: [942.11, -54.59, 969.43] },
    MissionProp { name: "Flag (Enjoi)", hull: "c_2a_02_jam_branded_props", min: [981.62, -59.24, 893.16], max: [981.76, -54.11, 894.16] },
    MissionProp { name: "Flag (Etnies)", hull: "GVR", min: [476.22, -57.43, 1158.68], max: [476.38, -52.31, 1159.68] },
    MissionProp { name: "Flag (Fallen)", hull: "c_1a_02_bestrick_street_props", min: [841.49, 36.16, -178.33], max: [842.11, 41.28, -177.46] },
    MissionProp { name: "Flag (Flip)", hull: "m_6p_01_Flip", min: [941.15, -59.71, 969.01], max: [942.11, -54.59, 969.43] },
    MissionProp { name: "Flag (Girl)", hull: "c_2a_02_street_all_props", min: [259.44, -44.46, 663.58], max: [260.21, -39.33, 664.33] },
    MissionProp { name: "Flag (Mystery)", hull: "c_1a_02_bestrick_street_props", min: [825.99, 34.67, -168.92], max: [826.55, 39.79, -168.02] },
    MissionProp { name: "Flag (Plan B)", hull: "c_2a_01_street_rd1_props", min: [1096.08, 292.39, -1711.20], max: [1096.19, 297.51, -1710.19] },
    MissionProp { name: "Flag (Real)", hull: "c_2a_02_jam_branded_props", min: [1027.83, -61.30, 962.97], max: [1028.48, -56.17, 963.82] },
    MissionProp { name: "Flag (Skateboard Mag)", hull: "GVR", min: [451.52, -53.48, 1127.56], max: [451.59, -48.35, 1128.56] },
    MissionProp { name: "Flag (Slap)", hull: "GVR", min: [541.91, -55.43, 1128.46], max: [541.98, -50.31, 1129.46] },
    MissionProp { name: "Flag (Slappy's)", hull: "c_1a_01_bestrick_street_props", min: [1149.70, 74.14, -285.18], max: [1150.66, 79.26, -284.74] },
    MissionProp { name: "Flag (Thrasher)", hull: "c_1a_01_bowl_series_props", min: [204.20, 39.17, -214.19], max: [205.10, 44.29, -213.62] },
    MissionProp { name: "Flag (Vox)", hull: "c_1a_02_bestrick_street_props", min: [714.72, 25.51, -198.89], max: [715.31, 30.63, -198.01] },
    MissionProp { name: "Flag (Vans)", hull: "c_2a_02_jam_branded_props", min: [979.90, -60.83, 947.88], max: [980.09, -55.71, 948.88] },
    MissionProp { name: "Flag (Zero)", hull: "c_1a_02_bestrick_street_props", min: [818.99, 35.55, -155.18], max: [819.68, 40.67, -154.36] },
    MissionProp { name: "Perimeter fence", hull: "c_2a_02_jam_branded", min: [994.94, -61.58, 959.81], max: [1000.99, -60.39, 965.55] },
    MissionProp { name: "Wooden fence (long)", hull: "PJ03_HotelAlley", min: [1597.84, 107.18, -654.89], max: [1602.44, 111.98, -650.50] },
    MissionProp { name: "Cargo container", hull: "m_4h_01_pro_appleyard", min: [1237.32, 82.57, -349.07], max: [1245.72, 85.57, -342.38] },
    MissionProp { name: "Bike rack", hull: "m_4i_01_spotfilm_barbee", min: [620.02, 26.19, -239.24], max: [621.22, 26.88, -238.16] },
];

/// The cut's box reaches this far past the record's (metres); its floor stays this far above
/// the record's bottom, so the ground or stage the prop stands on is left behind.
const CUT_MARGIN: f32 = 0.05;
const CUT_FLOOR: f32 = 0.01;

/// The prop's origin in its hull: its box's footprint centre at the box's bottom.
#[must_use]
pub fn mission_prop_origin(prop: &MissionProp) -> [f32; 3] {
    [(prop.min[0] + prop.max[0]) * 0.5, prop.min[1], (prop.min[2] + prop.max[2]) * 0.5]
}

/// Whether a triangle with these hull (world-space) corners belongs to the prop: all its corners
/// lie in the cut box (a neighbour's large triangle reaching in stays out) and its centroid
/// above the box's floor (the ground the prop stands on stays out). The collision and render
/// cuts both use it.
#[must_use]
pub fn in_mission_cut(prop: &MissionProp, vertices: &[[f32; 3]; 3]) -> bool {
    let centroid_y = (vertices[0][1] + vertices[1][1] + vertices[2][1]) / 3.0;
    centroid_y >= prop.min[1] + CUT_FLOOR
        && vertices.iter().all(|vertex| (0..3).all(|axis| vertex[axis] >= prop.min[axis] - CUT_MARGIN && vertex[axis] <= prop.max[axis] + CUT_MARGIN))
}

/// Catalog IDs for mission props: this base (no DMO bank ID uses the range) with a hash of the
/// prop's name in the low half (`mission_prop_id`), so an ID stays the same however the list
/// changes (online, a placed prop is its catalog ID).
pub(crate) const MISSION_PROP_BASE: u64 = 0x4d49_5353_0000_0000;

/// A stable 32-bit key for a name: FNV-1a.
#[must_use]
pub(crate) const fn name_key(name: &str) -> u32 {
    let bytes = name.as_bytes();
    let mut hash: u32 = 0x811c_9dc5;
    let mut at = 0;
    while at < bytes.len() {
        hash = (hash ^ bytes[at] as u32).wrapping_mul(0x0100_0193);
        at += 1;
    }
    hash
}

/// A mission prop's catalog ID, from its `MISSION_PROPS` name (which must therefore never be
/// renamed).
#[must_use]
pub(crate) const fn mission_prop_id(name: &str) -> u64 {
    MISSION_PROP_BASE | name_key(name) as u64
}

/// One mission prop's collision: model-space triangles (the prop's footprint centre at its base
/// is the origin) and their native contact features.
#[derive(Clone, Debug, Default)]
pub(crate) struct MissionPropShape {
    pub(crate) triangles: Vec<skate2_formats::collision::MeshTriangle>,
    pub(crate) features: Vec<crate::static_ground_pool::ResidentGroundFeatures>,
    /// Its grind splines (model space; a Skate 3 park piece's rails, ledges and coping), as a
    /// dropped prop's are, and their segments' bounds (None: it has none).
    pub(crate) grind_splines: Vec<crate::living_world_grind_services::dmo_models::DmoGrindSpline>,
    pub(crate) grind_bounds: Option<([f32; 3], [f32; 3])>,
}

/// A placed mission prop as Skate 3 physics takes it: an immovable body (`MISSION_BODY_BASE`
/// range ID) carrying the prop's collision at `transform`.
#[derive(Clone)]
pub(crate) struct MissionPlacement {
    pub(crate) id: u64,
    pub(crate) shape: std::sync::Arc<MissionPropShape>,
    pub(crate) transform: [f32; 16],
}

/// Skate 3 body IDs of placed mission props: this base plus a count.
pub(crate) const MISSION_BODY_BASE: u64 = 0x4d50_4c43_0000_0000;

/// Whether a Skate 3 body ID is a placed mission prop's.
#[must_use]
pub(crate) const fn is_mission_body(id: u64) -> bool {
    id & 0xffff_ffff_0000_0000 == MISSION_BODY_BASE
}

static MISSION_PLACEMENTS: Mutex<Option<std::sync::Arc<Vec<MissionPlacement>>>> = Mutex::new(None);
/// Counts publications, so readers rebuild what they derive from the placements only when they
/// change.
static MISSION_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The park editor's placed mission props, for Skate 3 physics to add each tick.
pub(crate) fn publish_mission_placements(placements: Vec<MissionPlacement>) {
    if let Ok(mut slot) = MISSION_PLACEMENTS.lock() {
        *slot = Some(std::sync::Arc::new(placements));
        MISSION_GENERATION.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    }
}

/// Skate 3's park editor props (the Props tab's pieces) as living-world DMO assets, so they
/// spawn, move and are knocked about as the main bank's props are: each `.rx2` holds one
/// `Instance_DMO` (its local GUID the piece's ID), its RModel, CModel and a one-Part Assembly,
/// as a bank arena does. Its asset ID is the piece's, so the DMO renderer (which loads every
/// piece under its ID) draws it. A piece that does not build is left static and reported.
pub(crate) fn load_park_prop_assets(mounts: &skate2_formats::mount::MountSet) -> (Vec<std::sync::Arc<crate::dmo_bank_processed_asset::DmoBankProcessedArenaAsset>>, Vec<String>) {
    let mut assets = Vec::new();
    let mut skipped = Vec::new();
    let Some(archive) = park_assets_archive(mounts) else {
        return (assets, skipped);
    };
    for piece in PARK_PIECES.iter().filter(|piece| piece.tab == 2) {
        let built = (|| -> Result<_, String> {
            let (mut arena, mut resources) = read_park_arena(&archive, piece.path)?;
            add_procedural_assembly(&mut arena, &mut resources)?;
            let asset_resource_descriptors = std::array::from_fn(|index| skate2_formats::stream::BaseResourceDescriptor {
                size: u32::try_from(resources[index].len()).unwrap_or(u32::MAX),
                alignment: arena.resource_descriptor[index].alignment,
            });
            let decoded = crate::decoded_arena_asset::DecodedArenaAsset {
                asset_id: piece.id,
                asset_type: crate::decoded_arena_asset::WORLD_STREAM_ARENA_ASSET_TYPE,
                asset_resource_descriptors,
                resources: skate2_formats::stream::ArenaResources { resources },
                arena,
            };
            let asset = crate::dmo_bank_processed_asset::DmoBankProcessedArenaAsset::try_from_decoded(decoded).map_err(|e| format!("{e:?}"))?;
            // Exactly its own DMO instance (a bank GUID would alias the bank's model).
            let guids: Vec<u64> = asset
                .plan()
                .operations()
                .iter()
                .filter_map(|operation| match operation {
                    crate::simulation_normal_arena_plan::SimulationNormalOperation::DmoInstance { local_guid, .. } => Some(*local_guid),
                    _ => None,
                })
                .collect();
            if guids != [piece.id] {
                return Err(format!("DMO instances {guids:x?}"));
            }
            Ok(std::sync::Arc::new(asset))
        })();
        match built {
            Ok(asset) => assets.push(asset),
            Err(e) => skipped.push(format!("{}: {e}", piece.recipe)),
        }
    }
    (assets, skipped)
}

/// `LWDynamicObjectModel::CreateFromMesh` (Skate 2 0x82d337b0, Skate 3 0x82c4da40) for a
/// CModel with a mesh table (`m_iNumMeshes` != -1) rather than a pipe Definition: a new
/// one-Part `AssemblyDefinition` (`PartDefinition::Initialize` defaults: inverse mass 1, unit
/// inverse tensor and spherical, unlimited velocities, no drag, friction 0.5/0.3, restitution
/// 0.4; no `ApplyAttribulatorOffsets`) whose volume, with the default multi-tier cvar, is an
/// `AggregateVolume` over a two-volume `SimpleMappedArray`: the mesh volume with its enable bit
/// cleared, then a box of the RModel bounds. Four Skate 3 props (two bins, the balloon, the
/// guard rail) ship this way. Their arena gains those objects and an `Assembly` naming the
/// Definition, which the model build takes as the arena's only one. A pipe CModel is left alone.
fn add_procedural_assembly(
    arena: &mut skate2_formats::rw4::Arena,
    resources: &mut [Vec<u8>; 5],
) -> Result<(), String> {
    use skate2_formats::collision::{ASSEMBLY_DEFINITION_TYPE_ID, ASSEMBLY_TYPE_ID, ObjectType, VolumeType};
    const R_MODEL_TYPE_ID: u32 = 0x00eb_0001;
    let word = |bytes: &[u8], at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let float = |bytes: &[u8], at: usize| f32::from_bits(word(bytes, at));
    let typed = |arena: &skate2_formats::rw4::Arena, type_id: u32| {
        let found: Vec<usize> = arena.dictionary.iter().enumerate().filter(|(_, entry)| entry.type_id == type_id).map(|(index, _)| index).collect();
        match found[..] {
            [index] => Ok(arena.dictionary[index].object_range.clone()),
            _ => Err(format!("{} objects of type {type_id:#x}", found.len())),
        }
    };
    let c_model = &resources[0][typed(arena, skate2_formats::pegasus::C_MODEL_DATA_TYPE_ID)?];
    let meshes = word(c_model, 4);
    if meshes == u32::MAX {
        return Ok(());
    }
    // The table (an object-relative offset) holds { m_pVolume, m_BoneName } per mesh.
    let table = word(c_model, 8) as usize;
    if meshes != 1 || c_model.len() < table + 8 {
        return Err(format!("{meshes} procedural meshes"));
    }
    let mesh_volume = word(c_model, table) as usize;
    let entry = |index: usize| arena.dictionary.get(index).map(|entry| (entry.type_id, entry.object_range.clone()));
    let Some((type_id, range)) = entry(mesh_volume) else {
        return Err(format!("mesh volume object {mesh_volume:#x}"));
    };
    let mesh = resources[0][range].to_vec();
    let target = word(&mesh, 0x44) as usize;
    let clustered = match entry(target) {
        Some((target_type, range))
            if type_id == ObjectType::Volume as u32
                && word(&mesh, 0x40) == VolumeType::Aggregate as u32
                && target_type == ObjectType::ClusteredMesh as u32 =>
        {
            range
        }
        _ => return Err("mesh volume is not a clustered mesh".to_owned()),
    };
    let r_model = &resources[0][typed(arena, R_MODEL_TYPE_ID)?];
    let min: [f32; 3] = std::array::from_fn(|lane| float(r_model, lane * 4));
    let max: [f32; 3] = std::array::from_fn(|lane| float(r_model, 0x10 + lane * 4));
    // The clustered mesh's AABB opens its object; Update bounds the array by both children.
    let clustered = &resources[0][clustered];
    let bounds_min: [f32; 3] = std::array::from_fn(|lane| min[lane].min(float(clustered, lane * 4)));
    let bounds_max: [f32; 3] = std::array::from_fn(|lane| max[lane].max(float(clustered, 0x10 + lane * 4)));

    let push = |bytes: &mut Vec<u8>, value: u32| bytes.extend_from_slice(&value.to_be_bytes());
    let pushf = |bytes: &mut Vec<u8>, value: f32| bytes.extend_from_slice(&value.to_be_bytes());
    let identity = |bytes: &mut Vec<u8>, translation: [f32; 3]| {
        for row in 0..3 {
            for column in 0..4 {
                pushf(bytes, if row == column { 1.0 } else { 0.0 });
            }
        }
        for lane in translation {
            pushf(bytes, lane);
        }
        pushf(bytes, 0.0);
    };
    let first = arena.dictionary.len() as u32;
    let (array_id, volume_id, definition_id) = (first, first + 1, first + 2);

    // SimpleMappedArray::Initialize(2 volumes) and its Update's bounds.
    let mut array = Vec::new();
    for bound in [bounds_min, bounds_max] {
        for lane in bound {
            pushf(&mut array, lane);
        }
        pushf(&mut array, 0.0);
    }
    for value in [0, 2, 2, 0, 0x40, 0, 0, 0] {
        push(&mut array, value);
    }
    let mut disabled = mesh.clone();
    disabled[0x5f] &= !1;
    array.extend_from_slice(&disabled);
    // BoxVolume::Initialize of the half extents, at the bounds' middle.
    identity(&mut array, std::array::from_fn(|lane| min[lane] + (max[lane] - min[lane]) * 0.5));
    push(&mut array, VolumeType::Box as u32);
    for lane in 0..3 {
        pushf(&mut array, (max[lane] - min[lane]) * 0.5);
    }
    for value in [0, 0, 0, 1] {
        push(&mut array, value);
    }

    // PhysicsUtility::CreateAggregateVolume over the array.
    let mut volume = mesh;
    volume[0x44..0x48].copy_from_slice(&array_id.to_be_bytes());

    // PhysicsUtility::CreateAssemblyDefinition(1 Part, 0 joints), no culling table.
    let mut definition = Vec::new();
    for value in [0x30, 1, 0, 0xd0, 0xd0, 0xd0, 0, 0xd0, 0xd0, 0xd0, 0, 0xd0, volume_id, 0, 0, 0] {
        push(&mut definition, value);
    }
    for value in [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, f32::MAX, f32::MAX, 0.0, 0.0] {
        pushf(&mut definition, value);
    }
    definition.resize(0x30 + 0x80, 0);
    for value in [0.5, 0.3, 0.4] {
        pushf(&mut definition, value);
    }
    definition.resize(0x30 + 0xa0, 0);

    // The Assembly naming it: header, then one Part record.
    let mut assembly = Vec::new();
    for value in [0, definition_id, 0x30, 0x90, 0x90, 0xffff, 0, 1, 0, 0, 0, 0] {
        push(&mut assembly, value);
    }
    identity(&mut assembly, [0.0; 3]);
    assembly.resize(0x90, 0);

    for (bytes, type_id) in [
        (array, ObjectType::SimpleMappedArray as u32),
        (volume, ObjectType::Volume as u32),
        (definition, ASSEMBLY_DEFINITION_TYPE_ID),
        (assembly, ASSEMBLY_TYPE_ID),
    ] {
        let resource = &mut resources[0];
        resource.resize(resource.len().next_multiple_of(16), 0);
        let start = resource.len();
        resource.extend_from_slice(&bytes);
        arena.dictionary.push(skate2_formats::rw4::DictionaryEntry {
            resource_index: 0,
            object_offset: start as u32,
            relocation_offset: 0,
            size: bytes.len() as u32,
            alignment: 16,
            type_index: 0,
            type_id,
            object_range: start..start + bytes.len(),
        });
    }
    let count = arena.dictionary.len() as u32;
    arena.entry_count = count;
    arena.used_entry_count = count;
    let size = resources[0].len() as u32;
    arena.resource_descriptor[0].size = size;
    arena.resources_used[0].size = size;
    Ok(())
}

/// Skate 3's props registered as living-world DMO models this session (by catalog ID): they
/// drop as the bank's props do (dynamic, or static by their settings), not as static pieces.
static DYNAMIC_PARK_PROPS: Mutex<Vec<u64>> = Mutex::new(Vec::new());

pub(crate) fn publish_dynamic_park_props(ids: Vec<u64>) {
    if let Ok(mut slot) = DYNAMIC_PARK_PROPS.lock() {
        *slot = ids;
    }
}

/// Whether `id` is a Skate 3 prop registered as a living-world DMO model.
#[must_use]
pub(crate) fn is_dynamic_park_prop(id: u64) -> bool {
    DYNAMIC_PARK_PROPS.lock().is_ok_and(|ids| ids.contains(&id))
}

/// Which publication the placements are (read it before `mission_placements`).
#[must_use]
pub(crate) fn mission_generation() -> u64 {
    MISSION_GENERATION.load(std::sync::atomic::Ordering::Acquire)
}

/// The placements as last published, shared: physics reads them many times a tick.
#[must_use]
pub(crate) fn mission_placements() -> std::sync::Arc<Vec<MissionPlacement>> {
    MISSION_PLACEMENTS.lock().ok().and_then(|slot| slot.clone()).unwrap_or_else(empty_shared)
}

/// One shared empty list, so reading an empty one allocates nothing.
fn empty_shared<T>() -> std::sync::Arc<Vec<T>>
where
    T: Send + Sync + 'static,
{
    use std::any::{Any, TypeId};
    static EMPTY: Mutex<Vec<(TypeId, std::sync::Arc<dyn Any + Send + Sync>)>> = Mutex::new(Vec::new());
    let mut empty = EMPTY.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((_, list)) = empty.iter().find(|(id, _)| *id == TypeId::of::<T>())
        && let Ok(list) = std::sync::Arc::clone(list).downcast::<Vec<T>>()
    {
        return list;
    }
    let list = std::sync::Arc::new(Vec::<T>::new());
    empty.push((TypeId::of::<T>(), list.clone()));
    list
}

/// The dropped grind boards: each one's grind ID and its deck (world, as of the last frame),
/// for both physics' grind queries.
static GRIND_BOARDS: Mutex<Option<std::sync::Arc<Vec<(u32, [f32; 16])>>>> = Mutex::new(None);

/// Publishes the dropped grind boards' decks, once a frame.
pub(crate) fn publish_grind_boards(decks: Vec<(u32, [f32; 16])>) {
    if let Ok(mut slot) = GRIND_BOARDS.lock() {
        // Unchanged (no grind board, or none moved): the list readers hold stays.
        if slot.as_deref().map_or(decks.is_empty(), |old| *old == decks) {
            return;
        }
        *slot = Some(std::sync::Arc::new(decks));
    }
}

#[must_use]
pub(crate) fn grind_boards() -> std::sync::Arc<Vec<(u32, [f32; 16])>> {
    GRIND_BOARDS.lock().ok().and_then(|slot| slot.clone()).unwrap_or_else(empty_shared)
}

/// The grind board's deck edges in deck space (x across, y up, z along), on the deck's
/// collision, tail to nose: up the tail's edge, along the flat (the edge capsules' centre lines,
/// 0.1125 out, at the deck's 7.5 mm top), up the nose's edge. The kicktails' edges are their
/// triangles' at 0.385 along, 2 cm up (plus the same 7.5 mm), before they round in to the tip.
const GRIND_BOARD_EDGE: [[f32; 3]; 4] = [
    [0.091, 0.0275, -0.385],
    [0.1125, 0.0075, -0.295],
    [0.1125, 0.0075, 0.295],
    [0.091, 0.0275, 0.385],
];
/// The grind board's splines' GUIDs (one per edge, this plus the edge's index).
const GRIND_BOARD_SPLINE_GUID: u64 = 0x4752_4e44_424f_0000;

/// The grind board's splines (deck space): one along each deck edge (`GRIND_BOARD_EDGE`, and
/// mirrored), a straight segment per stretch laid out as Skate 3's straight rails are (an ease
/// along the span: a = -2 span, b = 3 span, c = 0, d = the start).
pub(crate) fn grind_board_splines() -> &'static [crate::living_world_grind_services::dmo_models::DmoGrindSpline] {
    use skate2_formats::pegasus::{ObjectId, SplineData, SplineHeader, SplineSegment};
    static SPLINES: std::sync::OnceLock<Vec<crate::living_world_grind_services::dmo_models::DmoGrindSpline>> = std::sync::OnceLock::new();
    SPLINES.get_or_init(|| {
        // `tSplineData`'s own 16 bytes, then its headers (32 bytes each), then its segments (0x90).
        const HEADERS_AT: u32 = 16;
        const SEGMENTS_AT: u32 = HEADERS_AT + 2 * 32;
        const PER_EDGE: u32 = GRIND_BOARD_EDGE.len() as u32 - 1;
        let mut splines = Vec::new();
        let mut segments = Vec::new();
        for (index, side) in [-1.0f32, 1.0].into_iter().enumerate() {
            let header = HEADERS_AT + index as u32 * 32;
            let first = SEGMENTS_AT + index as u32 * PER_EDGE * 0x90;
            splines.push(SplineHeader {
                guid: GRIND_BOARD_SPLINE_GUID + index as u64,
                local_guid: YOUR_GRIND_BOARD,
                instance: ObjectId(0),
                head_offset: Some(first),
                tail_offset: Some(first + (PER_EDGE - 1) * 0x90),
            });
            let point = |p: [f32; 3]| [p[0] * side, p[1], p[2]];
            for pair in GRIND_BOARD_EDGE.windows(2) {
                let (start, end) = (point(pair[0]), point(pair[1]));
                let span: [f32; 3] = std::array::from_fn(|axis| end[axis] - start[axis]);
                segments.push(SplineSegment {
                    basis_matrix: [
                        -2.0 * span[0], -2.0 * span[1], -2.0 * span[2], 0.0,
                        3.0 * span[0], 3.0 * span[1], 3.0 * span[2], 0.0,
                        0.0, 0.0, 0.0, 0.0,
                        start[0], start[1], start[2], 1.0,
                    ],
                    inverse: [0.0; 4],
                    bounding_box_min: [start[0].min(end[0]), start[1].min(end[1]), start[2].min(end[2]), 0.0],
                    bounding_box_max: [start[0].max(end[0]), start[1].max(end[1]), start[2].max(end[2]), 0.0],
                    length: 0.0,
                    distance: 0.0,
                    spline_offset: Some(header),
                    previous_offset: None,
                    next_offset: None,
                });
            }
        }
        let data = SplineData { splines_offset: HEADERS_AT, segments_offset: SEGMENTS_AT, splines, segments };
        let source = crate::grind_data_runtime::GrindSplineSource::from_built_spline_data(YOUR_GRIND_BOARD, data);
        (0..2)
            .map(|index| crate::living_world_grind_services::dmo_models::DmoGrindSpline { source: source.clone(), header_offset: HEADERS_AT + index * 32 })
            .collect()
    })
}

/// One of Skate 3's park editor pieces (`parkassets.big`): its asset ID (the `.rx2` arena ID,
/// which also names it in the catalog, so a placed piece is the same ID on every machine), the
/// editor tab it is listed under (0 Terrain, 1 Other, 2 Props), its recipe name, its model's
/// path in the archive and RModel dictionary entry, and its model-space bounds (its origin is
/// where Skate 3 places it).
#[derive(Clone, Copy, Debug)]
pub struct ParkPiece {
    pub id: u64,
    pub tab: usize,
    pub recipe: &'static str,
    pub path: &'static str,
    pub model: u32,
    pub min: [f32; 3],
    pub max: [f32; 3],
}

include!("object_dropper_park_pieces.rs");

/// Skate 3's park editor archive (its own `EB` format), copied beside Skate 2's own world
/// archives.
pub const PARK_ASSETS_ARCHIVE: &str = "data\\content\\parkassets.big";
include!(concat!(env!("OUT_DIR"), "/park_assets.rs"));

/// Skate 3's park editor archive beside the mounted Skate 2 archives (each mounted at
/// `<game root>\data\<folder>\<name>.big`), if it has been copied there.
#[must_use]
pub fn park_assets_archive(mounts: &skate2_formats::mount::MountSet) -> Option<skate2_formats::big_eb::EbArchive> {
    let root = mounts.archives().first()?.data_path().parent()?.parent()?.parent()?;
    let path = root.join(PARK_ASSETS_ARCHIVE.replace('\\', "/"));
    if path.is_file() {
        skate2_formats::big_eb::EbArchive::open(&path).ok()
    } else if !BUNDLED_PARK_ASSETS.is_empty() {
        skate2_formats::big_eb::EbArchive::open_embedded(BUNDLED_PARK_ASSETS).ok()
    } else {
        None
    }
}

/// A park piece's arena from the archive: its parsed dictionary and resources.
///
/// # Errors
///
/// The model cannot be read from the archive or is not an RW4 arena.
pub fn read_park_arena(
    archive: &skate2_formats::big_eb::EbArchive,
    path: &str,
) -> Result<(skate2_formats::rw4::Arena, [Vec<u8>; 5]), String> {
    let bytes = archive.read_path(path)?;
    let resources = skate2_formats::rw4::Arena::split_file_resources(&bytes).map_err(|e| format!("{path}: {e:?}"))?;
    let arena = skate2_formats::rw4::Arena::parse(&resources).map_err(|e| format!("{path}: {e:?}"))?;
    Ok((arena, resources))
}

/// The `PARK_PIECES` index a catalog ID names, if it names one.
#[must_use]
pub fn park_piece(id: u64) -> Option<usize> {
    static INDEX: std::sync::OnceLock<std::collections::HashMap<u64, usize>> = std::sync::OnceLock::new();
    INDEX.get_or_init(|| PARK_PIECES.iter().enumerate().map(|(index, piece)| (piece.id, index)).collect()).get(&id).copied()
}

/// The object list tab catalog entry `index` is listed under.
#[must_use]
pub(crate) fn prop_tab(index: usize) -> usize {
    park_piece(PROPS[index].2).map_or(2, |piece| PARK_PIECES[piece].tab)
}

/// Skate 3's park pieces come in its three themes, each its own asset (and catalog entry, so a
/// placed piece keeps its own ID); the object list shows a piece once and takes it in the theme
/// the editor settings name.
pub(crate) const THEMES: [&str; 3] = ["Downtown", "Industrial", "University"];

/// A themed name's theme-less name and theme (`THEMES` index): "Ramp (large, Downtown)" is
/// "Ramp (large)" in Downtown.
fn split_theme(name: &str) -> Option<(String, usize)> {
    let (base, inner) = name.strip_suffix(')')?.rsplit_once(" (")?;
    let (size, theme) = inner.rsplit_once(", ").map_or((None, inner), |(size, theme)| (Some(size), theme));
    let theme = THEMES.iter().position(|name| *name == theme)?;
    Some((size.map_or_else(|| base.to_owned(), |size| format!("{base} ({size})")), theme))
}

/// What a Skate 3 piece's theme variants share: its theme-less name, spaces dropped (some
/// recipes spell one variant's name run together, "Ledgedown b").
fn theme_key(index: usize) -> Option<String> {
    park_piece(PROPS[index].2)?;
    split_theme(PROPS[index].1).map(|(base, _)| base.replace(' ', "").to_lowercase())
}

/// The catalog entries listed as one object list row: a Skate 3 piece's theme variants (first
/// entry and count), anything else on its own.
#[must_use]
pub(crate) fn theme_row(index: usize) -> (usize, usize) {
    // Worked out once for the whole catalog: the list asks for every row every frame.
    static ROWS: std::sync::OnceLock<Vec<(usize, usize)>> = std::sync::OnceLock::new();
    ROWS.get_or_init(|| (0..PROPS.len()).map(find_theme_row).collect())[index]
}

fn find_theme_row(index: usize) -> (usize, usize) {
    let Some(key) = theme_key(index) else {
        return (index, 1);
    };
    let same = |other: usize| PROPS[other].0 == PROPS[index].0 && theme_key(other).as_ref() == Some(&key);
    let start = (0..index).rev().take_while(|&other| same(other)).last().unwrap_or(index);
    (start, (start..PROPS.len()).take_while(|&other| same(other)).count())
}

/// Catalog entry `index`'s row in `theme`; itself when the row has no such variant.
#[must_use]
pub(crate) fn in_theme(index: usize, theme: usize) -> usize {
    let (start, len) = theme_row(index);
    (start..start + len).find(|&entry| split_theme(PROPS[entry].1).is_some_and(|(_, other)| other == theme)).unwrap_or(index)
}

/// An object list row's name: a piece's theme-less name when it lists several themes.
#[must_use]
pub(crate) fn row_name(index: usize) -> String {
    let (start, len) = theme_row(index);
    if len > 1 && let Some((base, _)) = split_theme(PROPS[start].1) {
        return base;
    }
    PROPS[index].1.to_owned()
}

/// The marquee recipes the park editor places (`data\content\recipe\marquee`), and
/// whether each plays the character viewer's idle loop: the people do, the `z_` models stand
/// frozen on its first frame (their meshes follow the clip's Hips).
pub(crate) const MARQUEE: &[(&str, bool)] = &[
    ("ai_skater_01", true),
    ("ai_skater_02", true),
    ("ai_skater_03", true),
    ("ai_skater_04", true),
    ("ai_skater_05", true),
    ("ai_skater_06", true),
    ("ai_skater_07", true),
    ("ai_skater_08", true),
    ("ai_skater_09", true),
    ("alex_chalmers", true),
    ("attiba_jefferson", true),
    ("attiba_jefferson_prop", true),
    ("big_black", true),
    ("brayden_szafranski", true),
    ("chris_cole", true),
    ("chris_haslam", true),
    ("colin_mckay", true),
    ("cuz", true),
    ("cuz_can", true),
    ("danny_way", true),
    ("darren_navarette", true),
    ("dem_bones", true),
    ("dennis_busenitz", true),
    ("eric_koston", true),
    ("freeway", true),
    ("giovanni_reda", true),
    ("giovanni_reda_prop", true),
    ("jake_brown", true),
    ("jason_dill", true),
    ("jerry_hsu", true),
    ("john_cardiel", true),
    ("john_rattray", true),
    ("lucas_puig", true),
    ("marc_johnson", true),
    ("mark_appleyard", true),
    ("michael_burnette", true),
    ("michael_burnette_prop", true),
    ("mike", true),
    ("mike_carroll", true),
    ("mike_prop", true),
    ("pat_duffy", true),
    ("pj_ladd", true),
    ("ray_barbee", true),
    ("rob_dyrdek", true),
    ("ryan_gallant", true),
    ("ryan_smith", true),
    ("sammy", true),
    ("sammy_prop", true),
    ("seb", true),
    ("seb_prop", true),
    ("security_03", true),
    ("shingo", true),
    ("slappy", true),
    ("slappy_prop", true),
    ("terry_kennedy", true),
    ("tutorial_male", true),
    ("z_podium", false),
    ("z_bigblacks_suv", false),
    ("z_chair_table", false),
    ("z_pen", false),
    ("z_railing_cap", false),
    ("z_tally_board", false),
];

/// Catalog IDs for marquee characters and models: this base with a hash of the recipe name in
/// the low half (`marquee_id`), stable however the list changes.
pub(crate) const MARQUEE_BASE: u64 = 0x4d51_4545_0000_0000;

/// A marquee entry's catalog ID, from its recipe name.
#[must_use]
pub(crate) const fn marquee_id(recipe: &str) -> u64 {
    MARQUEE_BASE | name_key(recipe) as u64
}

/// The `MARQUEE` index a catalog ID names, if it names one.
#[must_use]
pub(crate) fn marquee_entry(id: u64) -> Option<usize> {
    if id & 0xffff_ffff_0000_0000 != MARQUEE_BASE {
        return None;
    }
    MARQUEE.iter().position(|(recipe, _)| marquee_id(recipe) == id)
}

/// The `MISSION_PROPS` index a catalog ID names, if it names one.
#[must_use]
pub(crate) fn mission_prop(id: u64) -> Option<usize> {
    if id & 0xffff_ffff_0000_0000 != MISSION_PROP_BASE {
        return None;
    }
    MISSION_PROPS.iter().position(|prop| mission_prop_id(prop.name) == id)
}

/// The catalog entry for a copy of the player's own board: a real riderless board (not a bank
/// prop), drawn with the player's board.
pub(crate) const YOUR_BOARD: u64 = 0;

/// The catalog entry for a copy of the player's board with grind splines along its deck's two
/// edges (`grind_board_splines`), so it is ground like a little ledge.
pub(crate) const YOUR_GRIND_BOARD: u64 = 1;

/// Whether a catalog ID is a copy of the player's board (either kind).
#[must_use]
pub(crate) fn is_board(id: u64) -> bool {
    id == YOUR_BOARD || id == YOUR_GRIND_BOARD
}

/// The props offered, grouped by Skate 3's park editor Props subcategories, in its order:
/// subcategory, display name, main DMO bank model (instance GUID). Every main-bank model the
/// dropper can spawn (93; tools/list_dmo_props.rs over the ROM lists them) and the player's
/// board, ordered so each subcategory is one run.
pub(crate) const PROPS: &[(&str, &str, u64)] = &[
    ("Benches & barriers", "Bench", 0x000192d303e38707),
    ("Benches & barriers", "Bench (no back)", 0x00015a5a03e38707),
    ("Benches & barriers", "Park bench", 0x000177f703e38707),
    ("Benches & barriers", "Kinked bench", 0x000177f903e38707),
    ("Benches & barriers", "Barcelona bench", 0x0001992403e38707),
    ("Benches & barriers", "Old Town bench (no back)", 0x000163ff03e38707),
    ("Benches & barriers", "SoHo bench", 0x00018abe03e38707),
    ("Benches & barriers", "University bench", 0x00018abf03e38707),
    ("Benches & barriers", "Waterfront bench", 0x00018bed03e38707),
    ("Benches & barriers", "Broken bench", 0x000196c003e38707),
    ("Benches & barriers", "Construction barrier", 0x00017fb203e38707),
    ("Benches & barriers", "Safety barrier", 0x00017fb103e38707),
    ("Benches & barriers", "Traffic barrier", 0x0001907903e38707),
    ("Benches & barriers", "Construction sign", 0x0001907a03e38707),
    ("Benches & barriers", "Construction sign (board)", 0x0001b4fd03e38707),
    ("Benches & barriers", "No skating sign", 0x000199c203e38707),
    ("Benches & barriers", "Pylon", 0x0000aa6403e38707),
    ("Benches & barriers", "Pylon drum", 0x0001907b03e38707),
    ("Benches & barriers", "Pylon pole", 0x00018ad903e38707),
    ("Benches & barriers", "Hay bale", 0x00018ada03e38707),
    ("Garbage bins", "Dumpster", 0x000187a003e38707),
    ("Garbage bins", "Garbage bin (Art Deco)", 0x000192d103e38707),
    ("Garbage bins", "Garbage bin (projects)", 0x00018d7503e38707),
    ("Garbage bins", "Garbage can (University)", 0x00018ad003e38707),
    ("Garbage bins", "Garbage can (Waterfront)", 0x000171b003e38707),
    ("Garbage bins", "Garbage can (SoHo)", 0x0001a97703e38707),
    ("Garbage bins", "Industrial garbage can", 0x000196ae03e38707),
    ("Garbage bins", "Tin trash can", 0x0001907803e38707),
    ("Garbage bins", "Tin trash lid", 0x0001a96003e38707),
    ("Garbage bins", "Recycling bin", 0x00018acd03e38707),
    ("Garbage bins", "Garbage bag", 0x00018bf003e38707),
    ("Other", "Your board", YOUR_BOARD),
    ("Other", "Your board (grind)", YOUR_GRIND_BOARD),
    ("Other", "Broken board", 0x00019cf403e38707),
    ("Other", "Planter", 0x000196c103e38707),
    ("Other", "Flower pot (Old Town)", 0x0001993803e38707),
    ("Other", "Flower pot (SoHo)", 0x000199c703e38707),
    ("Other", "Potted plant", 0x000196b803e38707),
    ("Other", "Mailbox", 0x0001a78f03e38707),
    ("Other", "Newspaper box", 0x0001907703e38707),
    ("Other", "Newspaper stack", 0x000196ac03e38707),
    ("Other", "Basketball", 0x000196a903e38707),
    ("Other", "Beach ball", 0x000196aa03e38707),
    ("Other", "Apple", 0x0001990803e38707),
    ("Other", "Coffee cup", 0x0001990a03e38707),
    ("Other", "Crumpled paper", 0x0001990d03e38707),
    ("Other", "Grocery bag", 0x0001990f03e38707),
    ("Other", "Magazine", 0x0001991303e38707),
    ("Other", "Letter", 0x0001991103e38707),
    ("Other", "Newspaper", 0x0001991403e38707),
    ("Other", "Panhandling can", 0x0001991603e38707),
    ("Other", "Wrapped bottle", 0x0001991703e38707),
    ("Other", "Pop can", 0x0001991803e38707),
    ("Other", "Purse", 0x0001991903e38707),
    ("Other", "Water bottle", 0x0001991b03e38707),
    ("Other", "Map", 0x0001991203e38707),
    ("Other", "Briefcase", 0x0001990903e38707),
    ("Other", "Cell phone", 0x0001990c03e38707),
    ("Other", "Duffle bag", 0x0001990e03e38707),
    ("Other", "Taser", 0x0001991a03e38707),
    ("Boxes", "Cardboard box", 0x000196b503e38707),
    ("Boxes", "Wood crate", 0x0001907c03e38707),
    ("Boxes", "Open wood crate", 0x0001992f03e38707),
    ("Boxes", "Oil drum", 0x00018adb03e38707),
    ("Boxes", "Metal keg", 0x000199b703e38707),
    ("Boxes", "Cinder block", 0x000196ab03e38707),
    ("Rails", "Rail (long, low)", 0x000170c503e38707),
    ("Rails", "Rail (long, medium)", 0x000170c403e38707),
    ("Rails", "Rail (long, high)", 0x000170c603e38707),
    ("Rails", "Rail (high-low kink)", 0x000170c903e38707),
    ("Rails", "Rail (corner bend)", 0x000192e503e38707),
    ("Rails", "Rail (flat)", 0x0001bb7403e38707),
    ("Rails", "Dragon rail", 0x0001ac8003e38707),
    ("Rails", "Rainbow rail (up)", 0x0001acd503e38707),
    ("Rails", "Rainbow rail (down)", 0x0001acd403e38707),
    ("Ramps", "Kicker (large)", 0x000192e403e38707),
    ("Ramps", "Kicker (medium)", 0x00011fda03e38707),
    ("Ramps", "Wedge ramp (large)", 0x00016ebb03e38707),
    ("Ramps", "Wedge ramp (medium)", 0x00017fdf03e38707),
    ("Ramps", "Quarter pipe (large)", 0x00017fac03e38707),
    ("Ramps", "Quarter pipe (large, deck)", 0x00017fad03e38707),
    ("Ramps", "Quarter pipe (medium)", 0x00017fab03e38707),
    ("Ramps", "Quarter pipe (medium, deck)", 0x00017fae03e38707),
    ("Ramps", "Spine ramp", 0x00017fb003e38707),
    ("Ramps", "Loading ramp", 0x000196bd03e38707),
    ("Ramps", "Manual pad", 0x000170c303e38707),
    ("Ramps", "Wood panel", 0x000196b203e38707),
    ("Ramps", "Wooden pallet", 0x00009e7a03e38707),
    ("Tables", "Picnic table", 0x00017fb303e38707),
    ("Tables", "Picnic table (wood)", 0x00018c9203e38707),
    ("Tables", "Picnic table (broken)", 0x00018ad603e38707),
    ("Tables", "Patio table", 0x0001c9bb03e38707),
    ("Tables", "Patio table (umbrella)", 0x0000047503e38707),
    ("Tables", "Patio chair", 0x0001a7a503e38707),
    ("Tables", "Sun chair", 0x0001732c03e38707),
    // Skate 3's park editor pieces (`PARK_PIECES`, by Skate 3 asset ID): Terrain and Other tab
    // subcategories, then its props Skate 2 does not have.
    ("Premade", "Quarterpipe preset (Downtown)", 0x2c70170700030297),
    ("Premade", "Quarterpipe preset (Industrial)", 0x2c70170700030296),
    ("Premade", "Quarterpipe preset (University)", 0x2c70170700030295),
    ("Premade", "Funbox a (Downtown)", 0x2c7017070003029d),
    ("Premade", "Funbox a (Industrial)", 0x2c7017070003029c),
    ("Premade", "Funbox a (University)", 0x2c7017070003029b),
    ("Premade", "Funbox b (Downtown)", 0x2c7017070003029f),
    ("Premade", "Funbox b (Industrial)", 0x2c701707000302a0),
    ("Premade", "Funbox b (University)", 0x2c7017070003029e),
    ("Premade", "Funbox c (Downtown)", 0x2c701707000302a2),
    ("Premade", "Funbox c (Industrial)", 0x2c701707000302a3),
    ("Premade", "Funbox c (University)", 0x2c701707000302a1),
    ("Premade", "Funbox d (Downtown)", 0x2c701707000302a5),
    ("Premade", "Funbox d (Industrial)", 0x2c701707000302a6),
    ("Premade", "Funbox d (University)", 0x2c701707000302a4),
    ("Premade", "Funbox e (Downtown)", 0x2c701707000302a8),
    ("Premade", "Funbox e (Industrial)", 0x2c701707000302a9),
    ("Premade", "Funbox e (University)", 0x2c701707000302a7),
    ("Premade", "Funbox f (Downtown)", 0x2c701707000302ab),
    ("Premade", "Funbox f (Industrial)", 0x2c701707000302ac),
    ("Premade", "Funbox f (University)", 0x2c701707000302aa),
    ("Premade", "Funbox g (Downtown)", 0x2c701707000302ae),
    ("Premade", "Funbox g (Industrial)", 0x2c701707000302af),
    ("Premade", "Funbox g (University)", 0x2c701707000302ad),
    ("Premade", "Funboxh (Downtown)", 0x2c701707000302b1),
    ("Premade", "Funboxh (Industrial)", 0x2c701707000302b2),
    ("Premade", "Funboxh (University)", 0x2c701707000302b0),
    ("Premade", "Funbox i (Downtown)", 0x2c701707000302b4),
    ("Premade", "Funbox i (Industrial)", 0x2c701707000302b5),
    ("Premade", "Funbox i (University)", 0x2c701707000302b3),
    ("Premade", "Funbox j (Downtown)", 0x2c701707000302b7),
    ("Premade", "Funbox j (Industrial)", 0x2c701707000302b8),
    ("Premade", "Funbox j (University)", 0x2c701707000302b6),
    ("Premade", "Bowl stairs preset (Downtown)", 0x2c70170700030299),
    ("Premade", "Bowl stairs preset (Industrial)", 0x2c7017070003029a),
    ("Premade", "Bowl stairs preset (University)", 0x2c70170700030298),
    ("Street", "Pad street (large, Downtown)", 0x2c701707000301a5),
    ("Street", "Pad street (large, Industrial)", 0x2c70170700030194),
    ("Street", "Pad street (large, University)", 0x00021cc903e38707),
    ("Street", "Pad street (medium, Downtown)", 0x2c701707000301b6),
    ("Street", "Pad street (medium, Industrial)", 0x2c701707000301c6),
    ("Street", "Pad street (medium, University)", 0x00021c6903e38707),
    ("Street", "Pad street (small, Downtown)", 0x2c701707000301e3),
    ("Street", "Pad street (small, Industrial)", 0x2c701707000301d4),
    ("Street", "Pad street (small, University)", 0x00021c6b03e38707),
    ("Street", "Stairs (large, Downtown)", 0x2c701707000301aa),
    ("Street", "Stairs (large, Industrial)", 0x2c70170700030199),
    ("Street", "Stairs (large, University)", 0x00021ccb03e38707),
    ("Street", "Stairs (medium, Downtown)", 0x2c701707000301b1),
    ("Street", "Stairs (medium, Industrial)", 0x2c701707000301c1),
    ("Street", "Stairs (medium, University)", 0x00021c7003e38707),
    ("Street", "Stairs (small, Downtown)", 0x2c701707000301de),
    ("Street", "Stairs (small, Industrial)", 0x2c701707000301d9),
    ("Street", "Stairs (small, University)", 0x2c701707000300e8),
    ("Street", "Ramp (large, Downtown)", 0x2c701707000301a9),
    ("Street", "Ramp (large, Industrial)", 0x2c70170700030198),
    ("Street", "Ramp (large, University)", 0x00021cca03e38707),
    ("Street", "Ramp (medium, Downtown)", 0x2c701707000301b2),
    ("Street", "Ramp (medium, Industrial)", 0x2c701707000301c2),
    ("Street", "Ramp (medium, University)", 0x00021c6a03e38707),
    ("Street", "Ramp (small, Downtown)", 0x2c701707000301df),
    ("Street", "Ramp (small, Industrial)", 0x2c701707000301d8),
    ("Street", "Ramp (small, University)", 0x00021c6c03e38707),
    ("Street", "Bankedcorner (large, Downtown)", 0x2c70170700030218),
    ("Street", "Bankedcorner (large, Industrial)", 0x2c701707000301a2),
    ("Street", "Bankedcorner (large, University)", 0x2c70170700030018),
    ("Street", "Bankedcorner (medium, Downtown)", 0x2c701707000301b9),
    ("Street", "Bankedcorner (medium, Industrial)", 0x2c701707000301c9),
    ("Street", "Bankedcorner (medium, University)", 0x2c70170700030017),
    ("Street", "Bankedcorner (small, Downtown)", 0x2c701707000301e6),
    ("Street", "Bankedcorner (small, Industrial)", 0x2c701707000301d1),
    ("Street", "Bankedcorner (small, University)", 0x2c70170700030016),
    ("Street", "Bankedhip (large, Downtown)", 0x2c701707000301a3),
    ("Street", "Bankedhip (large, Industrial)", 0x2c70170700030192),
    ("Street", "Bankedhip (large, University)", 0x2c70170700030001),
    ("Street", "Bankedhip (medium, Downtown)", 0x2c701707000301b8),
    ("Street", "Bankedhip (medium, Industrial)", 0x2c701707000301c8),
    ("Street", "Bankedhip (medium, University)", 0x2c70170700030003),
    ("Street", "Bankedhip (small, Downtown)", 0x2c701707000301e5),
    ("Street", "Bankedhip (small, Industrial)", 0x2c701707000301d2),
    ("Street", "Bankedhip (small, University)", 0x2c70170700030000),
    ("Street", "Foldedpad (large, Downtown)", 0x2c701707000301a4),
    ("Street", "Foldedpad (large, Industrial)", 0x2c70170700030193),
    ("Street", "Foldedpad (large, University)", 0x2c7017070003001c),
    ("Street", "Foldedpad (medium, Downtown)", 0x2c701707000301b7),
    ("Street", "Foldedpad (medium, Industrial)", 0x2c701707000301c7),
    ("Street", "Foldedpad (medium, University)", 0x2c7017070003001b),
    ("Street", "Foldedpad (small, Downtown)", 0x2c701707000301e4),
    ("Street", "Foldedpad (small, Industrial)", 0x2c701707000301d3),
    ("Street", "Foldedpad (small, University)", 0x2c7017070003001a),
    ("Street", "Steepramp (large, Downtown)", 0x2c701707000301ab),
    ("Street", "Steepramp (large, Industrial)", 0x2c7017070003019f),
    ("Street", "Steepramp (large, University)", 0x2c7017070003001f),
    ("Street", "Steepramp (medium, Downtown)", 0x2c701707000301b0),
    ("Street", "Steepramp (medium, Industrial)", 0x2c701707000301c0),
    ("Street", "Steepramp (medium, University)", 0x2c7017070003001e),
    ("Street", "Steepramp (small, Downtown)", 0x2c701707000301dd),
    ("Street", "Steepramp (small, Industrial)", 0x2c701707000301da),
    ("Street", "Steepramp (small, University)", 0x2c7017070003001d),
    ("Street", "Steeprampcorner (large, Downtown)", 0x2c701707000301ac),
    ("Street", "Steeprampcorner (large, Industrial)", 0x2c7017070003019a),
    ("Street", "Steeprampcorner (large, University)", 0x2c70170700030020),
    ("Street", "Steeprampcorner (medium, Downtown)", 0x2c701707000301af),
    ("Street", "Steeprampcorner (medium, Industrial)", 0x2c701707000301bf),
    ("Street", "Steeprampcorner (medium, University)", 0x2c70170700030021),
    ("Street", "Steeprampcorner (small, Downtown)", 0x2c701707000301dc),
    ("Street", "Steeprampcorner (small, Industrial)", 0x2c701707000301db),
    ("Street", "Steeprampcorner (small, University)", 0x2c70170700030022),
    ("Street", "Quarterpipe (large, Downtown)", 0x2c701707000301a6),
    ("Street", "Quarterpipe (large, Industrial)", 0x2c70170700030196),
    ("Street", "Quarterpipe (large, University)", 0x00021cce03e38707),
    ("Street", "Quarterpipe (medium, Downtown)", 0x2c701707000301b5),
    ("Street", "Quarterpipe (medium, Industrial)", 0x2c701707000301c5),
    ("Street", "Quarterpipe (medium, University)", 0x00021cd203e38707),
    ("Street", "Quarterpipe (small, Downtown)", 0x2c701707000301e2),
    ("Street", "Quarterpipe (small, Industrial)", 0x2c701707000301d5),
    ("Street", "Quarterpipe (small, University)", 0x00021cd303e38707),
    ("Street", "Quarterpipecorner (large, Downtown)", 0x2c701707000301a7),
    ("Street", "Quarterpipecorner (large, Industrial)", 0x2c70170700030195),
    ("Street", "Quarterpipecorner (large, University)", 0x2c7017070003000f),
    ("Street", "Quarterpipecorner (medium, Downtown)", 0x2c701707000301b4),
    ("Street", "Quarterpipecorner (medium, Industrial)", 0x2c701707000301c4),
    ("Street", "Quarterpipecorner (medium, University)", 0x2c70170700030011),
    ("Street", "Quarterpipecorner (small, Downtown)", 0x2c701707000301e1),
    ("Street", "Quarterpipecorner (small, Industrial)", 0x2c701707000301d6),
    ("Street", "Quarterpipecorner (small, University)", 0x2c70170700030010),
    ("Street", "Quarterpipecornerinvert (large, Downtown)", 0x2c701707000301a8),
    ("Street", "Quarterpipecornerinvert (large, Industrial)", 0x2c70170700030197),
    ("Street", "Quarterpipecornerinvert (large, University)", 0x2c7017070003002b),
    ("Street", "Quarterpipecornerinvert (medium, Downtown)", 0x2c701707000301b3),
    ("Street", "Quarterpipecornerinvert (medium, Industrial)", 0x2c701707000301c3),
    ("Street", "Quarterpipecornerinvert (medium, University)", 0x2c7017070003002c),
    ("Street", "Quarterpipecornerinvert (small, Downtown)", 0x2c701707000301e0),
    ("Street", "Quarterpipecornerinvert (small, Industrial)", 0x2c701707000301d7),
    ("Street", "Quarterpipecornerinvert (small, University)", 0x2c7017070003002d),
    ("Vert", "Pad vert (large, Downtown)", 0x2c70170700030247),
    ("Vert", "Pad vert (large, Industrial)", 0x2c70170700030248),
    ("Vert", "Pad vert (large, University)", 0x2c7017070003005e),
    ("Vert", "Pad vert (medium, Downtown)", 0x2c70170700030257),
    ("Vert", "Pad vert (medium, Industrial)", 0x2c70170700030264),
    ("Vert", "Pad vert (medium, University)", 0x2c7017070003005d),
    ("Vert", "Pad vert (small, Downtown)", 0x2c7017070003028e),
    ("Vert", "Pad vert (small, Industrial)", 0x2c7017070003028f),
    ("Vert", "Pad vert (small, University)", 0x2c701707000300b4),
    ("Vert", "Quarterpipe (large, Downtown)", 0x2c7017070003023d),
    ("Vert", "Quarterpipe (large, Industrial)", 0x2c70170700030252),
    ("Vert", "Quarterpipe (large, University)", 0x2c70170700030053),
    ("Vert", "Quarterpipe (medium, Downtown)", 0x2c70170700030261),
    ("Vert", "Quarterpipe (medium, Industrial)", 0x2c7017070003026f),
    ("Vert", "Quarterpipe (medium, University)", 0x2c70170700030052),
    ("Vert", "Quarterpipe (small, Downtown)", 0x2c70170700030278),
    ("Vert", "Quarterpipe (small, Industrial)", 0x2c70170700030284),
    ("Vert", "Quarterpipe (small, University)", 0x2c701707000300a8),
    ("Vert", "Quaterpipeescalator a (large, Downtown)", 0x2c7017070003023f),
    ("Vert", "Quaterpipeescalator a (large, Industrial)", 0x2c70170700030250),
    ("Vert", "Quaterpipeescalator a (large, University)", 0x2c70170700030060),
    ("Vert", "Quaterpipeescalator a (medium, Downtown)", 0x2c7017070003025e),
    ("Vert", "Quaterpipeescalator a (medium, Industrial)", 0x2c7017070003026c),
    ("Vert", "Quaterpipeescalator a (medium, University)", 0x2c7017070003005f),
    ("Vert", "Quaterpipeescalator a (small, Downtown)", 0x2c70170700030281),
    ("Vert", "Quaterpipeescalator a (small, Industrial)", 0x2c7017070003028d),
    ("Vert", "Quaterpipeescalator a (small, University)", 0x2c701707000300b8),
    ("Vert", "Quaterpipeescalator b (large, Downtown)", 0x2c701707000305f9),
    ("Vert", "Quaterpipeescalator b (large, Industrial)", 0x2c701707000305fa),
    ("Vert", "Quaterpipeescalator b (large, University)", 0x2c701707000305fb),
    ("Vert", "Quaterpipeescalator b (medium, Downtown)", 0x2c701707000305f6),
    ("Vert", "Quaterpipeescalator b (medium, Industrial)", 0x2c701707000305f7),
    ("Vert", "Quaterpipeescalator b (medium, University)", 0x2c701707000305f8),
    ("Vert", "Quaterpipeescalator b (small, Downtown)", 0x2c7017070003025f),
    ("Vert", "Quaterpipeescalator b (small, Industrial)", 0x2c7017070003026d),
    ("Vert", "Quaterpipeescalator b (small, University)", 0x2c701707000300b7),
    ("Vert", "Quarterpipe 90corner (large, Downtown)", 0x2c70170700030242),
    ("Vert", "Quarterpipe 90corner (large, Industrial)", 0x2c7017070003024d),
    ("Vert", "Quarterpipe 90corner (large, University)", 0x2c70170700030062),
    ("Vert", "Quarterpipe 90corner (medium, Downtown)", 0x2c7017070003025b),
    ("Vert", "Quarterpipe 90corner (medium, Industrial)", 0x2c70170700030269),
    ("Vert", "Quarterpipe 90corner (medium, University)", 0x2c70170700030061),
    ("Vert", "Quarterpipe 90corner (small, Downtown)", 0x2c7017070003027d),
    ("Vert", "Quarterpipe 90corner (small, Industrial)", 0x2c70170700030289),
    ("Vert", "Quarterpipe 90corner (small, University)", 0x2c701707000300ac),
    ("Vert", "Quarterpipe 90hip (large, Downtown)", 0x2c70170700030240),
    ("Vert", "Quarterpipe 90hip (large, Industrial)", 0x2c7017070003024f),
    ("Vert", "Quarterpipe 90hip (large, University)", 0x2c70170700030068),
    ("Vert", "Quarterpipe 90hip (medium, Downtown)", 0x2c7017070003025d),
    ("Vert", "Quarterpipe 90hip (medium, Industrial)", 0x2c7017070003026b),
    ("Vert", "Quarterpipe 90hip (medium, University)", 0x2c70170700030069),
    ("Vert", "Quarterpipe 90hip (small, Downtown)", 0x2c7017070003027e),
    ("Vert", "Quarterpipe 90hip (small, Industrial)", 0x2c7017070003028a),
    ("Vert", "Quarterpipe 90hip (small, University)", 0x2c701707000300ad),
    ("Vert", "Quarterpiperollin (large, Downtown)", 0x2c70170700030239),
    ("Vert", "Quarterpiperollin (large, Industrial)", 0x2c70170700030256),
    ("Vert", "Quarterpiperollin (large, University)", 0x2c70170700030063),
    ("Vert", "Quarterpiperollin (medium, Downtown)", 0x2c70170700030263),
    ("Vert", "Quarterpiperollin (medium, Industrial)", 0x2c70170700030271),
    ("Vert", "Quarterpiperollin (medium, University)", 0x2c70170700030064),
    ("Vert", "Quarterpiperollin (small, Downtown)", 0x2c701707000305d5),
    ("Vert", "Quarterpiperollin (small, Industrial)", 0x2c701707000305d7),
    ("Vert", "Quarterpiperollin (small, University)", 0x2c701707000305d6),
    ("Vert", "Pipeflatbank (large, Downtown)", 0x2c70170700030246),
    ("Vert", "Pipeflatbank (large, Industrial)", 0x2c70170700030249),
    ("Vert", "Pipeflatbank (large, University)", 0x2c7017070003007b),
    ("Vert", "Pipeflatbank (medium, Downtown)", 0x2c70170700030258),
    ("Vert", "Pipeflatbank (medium, Industrial)", 0x2c70170700030265),
    ("Vert", "Pipeflatbank (medium, University)", 0x2c7017070003007c),
    ("Vert", "Pipeflatbank (small, Downtown)", 0x2c70170700030276),
    ("Vert", "Pipeflatbank (small, Industrial)", 0x2c70170700030282),
    ("Vert", "Pipeflatbank (small, University)", 0x2c701707000300b5),
    ("Vert", "Minirampkicker (Downtown)", 0x2c70170700030223),
    ("Vert", "Minirampkicker (Industrial)", 0x2c7017070003021b),
    ("Vert", "Minirampkicker (University)", 0x2c70170700030078),
    ("Vert", "Minirampkickersteep (Downtown)", 0x2c70170700030226),
    ("Vert", "Minirampkickersteep (Industrial)", 0x2c7017070003021c),
    ("Vert", "Minirampkickersteep (University)", 0x2c70170700030079),
    ("Vert", "Quaterpipeextensiona (Downtown)", 0x2c70170700030227),
    ("Vert", "Quaterpipeextensiona (Industrial)", 0x2c7017070003021d),
    ("Vert", "Quaterpipeextensiona (University)", 0x2c701707000300a5),
    ("Vert", "Quaterpipeextensionb (Downtown)", 0x2c70170700030229),
    ("Vert", "Quaterpipeextensionb (Industrial)", 0x2c7017070003021f),
    ("Vert", "Quaterpipeextensionb (University)", 0x2c701707000300a4),
    ("Vert", "Quaterpipeextensionamirror (Downtown)", 0x2c70170700030228),
    ("Vert", "Quaterpipeextensionamirror (Industrial)", 0x2c7017070003021e),
    ("Vert", "Quaterpipeextensionamirror (University)", 0x2c701707000300a6),
    ("Vert", "Wavedeck (Downtown)", 0x2c70170700030225),
    ("Vert", "Wavedeck (Industrial)", 0x2c70170700030220),
    ("Vert", "Wavedeck (University)", 0x2c70170700030096),
    ("Vert", "Bridgeclip (Downtown)", 0x2c70170700030222),
    ("Vert", "Bridgeclip (Industrial)", 0x2c7017070003021a),
    ("Vert", "Bridgeclip (University)", 0x2c70170700030097),
    ("Hubbas", "Ledge (large, Downtown)", 0x2c701707000303ba),
    ("Hubbas", "Ledge (large, Industrial)", 0x2c701707000303b9),
    ("Hubbas", "Ledge (large, University)", 0x2c701707000303b8),
    ("Hubbas", "Ledge (medium, Downtown)", 0x2c701707000303b5),
    ("Hubbas", "Ledge (medium, Industrial)", 0x2c701707000303b6),
    ("Hubbas", "Ledge (medium, University)", 0x2c701707000303b7),
    ("Hubbas", "Ledge (small, Downtown)", 0x2c701707000301e8),
    ("Hubbas", "Ledge (small, Industrial)", 0x2c701707000301cf),
    ("Hubbas", "Ledge (small, University)", 0x2c70170700030006),
    ("Hubbas", "Downledge (large, Downtown)", 0x2c70170700030219),
    ("Hubbas", "Downledge (large, Industrial)", 0x2c70170700030191),
    ("Hubbas", "Downledge (large, University)", 0x2c7017070003000a),
    ("Hubbas", "Downledge (medium, Downtown)", 0x2c701707000301ba),
    ("Hubbas", "Downledge (medium, Industrial)", 0x2c701707000301ca),
    ("Hubbas", "Downledge (medium, University)", 0x2c70170700030008),
    ("Hubbas", "Downledge (small, Downtown)", 0x2c701707000301e9),
    ("Hubbas", "Downledge (small, Industrial)", 0x2c701707000301ce),
    ("Hubbas", "Downledge (small, University)", 0x2c70170700030007),
    ("Hubbas", "Ledge b (large, Downtown)", 0x2c701707000305a1),
    ("Hubbas", "Ledge b (large, Industrial)", 0x2c701707000305a2),
    ("Hubbas", "Ledge b (large, University)", 0x2c70170700030334),
    ("Hubbas", "Ledge b (medium, Downtown)", 0x2c701707000305b0),
    ("Hubbas", "Ledge b (medium, Industrial)", 0x2c701707000305b1),
    ("Hubbas", "Ledge b (medium, University)", 0x2c70170700030332),
    ("Hubbas", "Ledge b (small, Downtown)", 0x2c701707000305b5),
    ("Hubbas", "Ledge b (small, Industrial)", 0x2c701707000305b6),
    ("Hubbas", "Ledge b (small, University)", 0x2c7017070003032f),
    ("Hubbas", "Ledge down b (large, Downtown)", 0x2c701707000305a3),
    ("Hubbas", "Ledgedown b (large, Industrial)", 0x2c701707000305a4),
    ("Hubbas", "Ledge down b (large, University)", 0x2c70170700030335),
    ("Hubbas", "Ledge down b (medium, Downtown)", 0x2c701707000305b3),
    ("Hubbas", "Ledgedown b (medium, Industrial)", 0x2c701707000305b4),
    ("Hubbas", "Ledge down b (medium, University)", 0x2c701707000305b2),
    ("Hubbas", "Ledge down b (small, Downtown)", 0x2c701707000305b7),
    ("Hubbas", "Ledgedown b (small, Industrial)", 0x2c701707000305b8),
    ("Hubbas", "Ledge down b (small, University)", 0x2c70170700030331),
    ("Hubbas", "Hubbaledge (large, Downtown)", 0x2c70170700030550),
    ("Hubbas", "Hubbaledge (large, Industrial)", 0x2c7017070003054f),
    ("Hubbas", "Hubbaledge (large, University)", 0x2c701707000300db),
    ("Hubbas", "Hubbaledge (medium, Downtown)", 0x2c70170700030606),
    ("Hubbas", "Hubbaledge (medium, Industrial)", 0x2c70170700030607),
    ("Hubbas", "Hubbaledge (medium, University)", 0x2c70170700030608),
    ("Hubbas", "Hubbaledge (small, Downtown)", 0x2c70170700030609),
    ("Hubbas", "Hubbaledge (small, Industrial)", 0x2c7017070003060a),
    ("Hubbas", "Hubbaledge (small, University)", 0x2c7017070003060b),
    ("Hubbas", "Hubbadownledge (large, Downtown)", 0x2c701707000304f8),
    ("Hubbas", "Hubbadownledge (large, Industrial)", 0x2c70170700030502),
    ("Hubbas", "Hubbadownledge (large, University)", 0x2c701707000300dd),
    ("Hubbas", "Hubbadownledge (medium, Downtown)", 0x2c70170700030517),
    ("Hubbas", "Hubbadownledge (medium, Industrial)", 0x2c70170700030521),
    ("Hubbas", "Hubbadownledge (medium, University)", 0x2c701707000300dc),
    ("Hubbas", "Hubbadownledge (small, Downtown)", 0x2c70170700030536),
    ("Hubbas", "Hubbadownledge (small, Industrial)", 0x2c70170700030542),
    ("Hubbas", "Hubbadownledge (small, University)", 0x2c701707000300da),
    ("Rails & ledges", "Fatroundrail (Downtown)", 0x2c70170700030547),
    ("Rails & ledges", "Fatroundrail (Industrial)", 0x2c70170700030548),
    ("Rails & ledges", "Fatroundrail (University)", 0x2c701707000300f2),
    ("Rails & ledges", "Fatroundraildown (large, Downtown)", 0x2c701707000304f5),
    ("Rails & ledges", "Fatroundraildown (large, Industrial)", 0x2c701707000304ff),
    ("Rails & ledges", "Fatroundraildown (large, University)", 0x2c701707000300f5),
    ("Rails & ledges", "Fatroundraildown (medium, Downtown)", 0x2c7017070003051a),
    ("Rails & ledges", "Fatroundraildown (medium, Industrial)", 0x2c70170700030525),
    ("Rails & ledges", "Fatroundraildown (medium, University)", 0x2c701707000300f4),
    ("Rails & ledges", "Fatroundraildown (small, Downtown)", 0x2c70170700030539),
    ("Rails & ledges", "Fatroundraildown (small, Industrial)", 0x2c7017070003053f),
    ("Rails & ledges", "Fatroundraildown (small, University)", 0x2c701707000300f3),
    ("Rails & ledges", "Roundrail (Downtown)", 0x2c7017070003020b),
    ("Rails & ledges", "Roundrail (Industrial)", 0x2c70170700030202),
    ("Rails & ledges", "Roundrail (University)", 0x2c7017070003000b),
    ("Rails & ledges", "Round rail down (large, Downtown)", 0x2c701707000301a0),
    ("Rails & ledges", "Round rail down (large, Industrial)", 0x2c70170700030190),
    ("Rails & ledges", "Round rail down (large, University)", 0x2c7017070003000e),
    ("Rails & ledges", "Round rail down (medium, Downtown)", 0x2c701707000301bc),
    ("Rails & ledges", "Round rail down (medium, Industrial)", 0x2c701707000301cb),
    ("Rails & ledges", "Round rail down (medium, University)", 0x2c7017070003000d),
    ("Rails & ledges", "Round rail down (small, Downtown)", 0x2c701707000301ea),
    ("Rails & ledges", "Round rail down (small, Industrial)", 0x2c701707000301cd),
    ("Rails & ledges", "Round rail down (small, University)", 0x2c7017070003000c),
    ("Rails & ledges", "Squarerail (Downtown)", 0x2c70170700030207),
    ("Rails & ledges", "Squarerail (Industrial)", 0x2c70170700030206),
    ("Rails & ledges", "Squarerail (University)", 0x2c70170700030023),
    ("Rails & ledges", "Square rail down (large, Downtown)", 0x2c701707000301a1),
    ("Rails & ledges", "Square rail down (large, Industrial)", 0x2c7017070003019b),
    ("Rails & ledges", "Square rail down (large, University)", 0x2c70170700030026),
    ("Rails & ledges", "Square rail down (medium, Downtown)", 0x2c701707000301bb),
    ("Rails & ledges", "Square rail down (medium, Industrial)", 0x2c701707000301be),
    ("Rails & ledges", "Square rail down (medium, University)", 0x2c70170700030025),
    ("Rails & ledges", "Square rail down (small, Downtown)", 0x2c701707000301e7),
    ("Rails & ledges", "Square rail down (small, Industrial)", 0x2c701707000301d0),
    ("Rails & ledges", "Square rail down (small, University)", 0x2c70170700030024),
    ("Rails & ledges", "Squarerailvariation a (Downtown)", 0x2c70170700030551),
    ("Rails & ledges", "Squarerailvariation a (Industrial)", 0x2c70170700030552),
    ("Rails & ledges", "Squarerailvariation a (University)", 0x2c701707000300f7),
    ("Rails & ledges", "Squareraildownvariation a (large, Downtown)", 0x2c701707000304fb),
    ("Rails & ledges", "Squareraildownvariation a (large, Industrial)", 0x2c70170700030505),
    ("Rails & ledges", "Squareraildownvariation a (large, University)", 0x2c701707000300f8),
    ("Rails & ledges", "Squareraildownvariation a (medium, Downtown)", 0x2c7017070003051b),
    ("Rails & ledges", "Squareraildownvariation a (medium, Industrial)", 0x2c70170700030526),
    ("Rails & ledges", "Squareraildownvariation a (medium, University)", 0x2c701707000300f6),
    ("Rails & ledges", "Squareraildownvariation a (small, Downtown)", 0x2c7017070003053a),
    ("Rails & ledges", "Squareraildownvariation a (small, Industrial)", 0x2c70170700030545),
    ("Rails & ledges", "Squareraildownvariation a (small, University)", 0x2c701707000300f9),
    ("Rails & ledges", "Squarerailvariation b (Downtown)", 0x2c70170700030553),
    ("Rails & ledges", "Squarerailvariation b (Industrial)", 0x2c70170700030554),
    ("Rails & ledges", "Squarerailvariation b (University)", 0x2c701707000300fc),
    ("Rails & ledges", "Squarerail downvariation b (large, Downtown)", 0x2c701707000304fc),
    ("Rails & ledges", "Squarerail downvariation b (large, Industrial)", 0x2c70170700030506),
    ("Rails & ledges", "Squarerail downvariation b (large, University)", 0x2c701707000300fd),
    ("Rails & ledges", "Squarerail downvariation b (medium, Downtown)", 0x2c7017070003051c),
    ("Rails & ledges", "Squarerail downvariation b (medium, Industrial)", 0x2c70170700030527),
    ("Rails & ledges", "Squarerail downvariation b (medium, University)", 0x2c701707000300fa),
    ("Rails & ledges", "Squarerail downvariation b (small, Downtown)", 0x2c7017070003053b),
    ("Rails & ledges", "Squarerail downvariation b (small, Industrial)", 0x2c70170700030546),
    ("Rails & ledges", "Squarerail downvariation b (small, University)", 0x2c701707000300fb),
    ("Rails & ledges", "Squarefence (Downtown)", 0x2c70170700030188),
    ("Rails & ledges", "Squarefence (Industrial)", 0x2c70170700030184),
    ("Rails & ledges", "Squarefence (University)", 0x2c70170700030027),
    ("Rails & ledges", "Squarefencedown (large, Downtown)", 0x2c70170700030187),
    ("Rails & ledges", "Squarefencedown (large, Industrial)", 0x2c70170700030183),
    ("Rails & ledges", "Squarefencedown (large, University)", 0x2c7017070003002a),
    ("Rails & ledges", "Squarefencedown (medium, Downtown)", 0x2c70170700030186),
    ("Rails & ledges", "Squarefencedown (medium, Industrial)", 0x2c70170700030182),
    ("Rails & ledges", "Squarefencedown (medium, University)", 0x2c70170700030029),
    ("Rails & ledges", "Squarefencedown (small, Downtown)", 0x2c70170700030185),
    ("Rails & ledges", "Squarefencedown (small, Industrial)", 0x2c70170700030181),
    ("Rails & ledges", "Squarefencedown (small, University)", 0x2c70170700030028),
    ("Rails & ledges", "Squarefence b (Downtown)", 0x2c701707000305c2),
    ("Rails & ledges", "Squarefence b (Industrial)", 0x2c701707000305c3),
    ("Rails & ledges", "Squarefence b (University)", 0x2c701707000305c4),
    ("Rails & ledges", "Squarefencedown b (large, Downtown)", 0x2c701707000305cb),
    ("Rails & ledges", "Squarefencedown b (large, Industrial)", 0x2c701707000305cc),
    ("Rails & ledges", "Squarefencedown b (large, University)", 0x2c701707000305cd),
    ("Rails & ledges", "Squarefencedown b (medium, Downtown)", 0x2c701707000305c8),
    ("Rails & ledges", "Squarefencedown b (medium, Industrial)", 0x2c701707000305c9),
    ("Rails & ledges", "Squarefencedown b (medium, University)", 0x2c701707000305ca),
    ("Rails & ledges", "Squarefencedown b (small, Downtown)", 0x2c701707000305c5),
    ("Rails & ledges", "Squarefencedown b (small, Industrial)", 0x2c701707000305c6),
    ("Rails & ledges", "Squarefencedown b (small, University)", 0x2c701707000305c7),
    ("Rails & ledges", "Roundrail rainbow down (Downtown)", 0x2c70170700030209),
    ("Rails & ledges", "Roundrail rainbow down (Industrial)", 0x2c70170700030204),
    ("Rails & ledges", "Roundrail rainbow down (University)", 0x2c70170700030038),
    ("Rails & ledges", "Roundrail rainbow up (Downtown)", 0x2c70170700030208),
    ("Rails & ledges", "Roundrail rainbow up (Industrial)", 0x2c70170700030205),
    ("Rails & ledges", "Roundrail rainbow up (University)", 0x2c70170700030037),
    ("Rails & ledges", "Roundrail longcornerbend (Downtown)", 0x2c7017070003020a),
    ("Rails & ledges", "Roundrail longcornerbend (Industrial)", 0x2c70170700030203),
    ("Rails & ledges", "Roundrail longcornerbend (University)", 0x2c70170700030039),
    ("Rails & ledges", "Roundrai hlilokink (Downtown)", 0x2c7017070003020c),
    ("Rails & ledges", "Roundrai hlilokink (Industrial)", 0x2c70170700030201),
    ("Rails & ledges", "Roundrai hlilokink (University)", 0x2c70170700030036),
    ("Rails & ledges", "Big mega rail a", 0x2c701707000303cd),
    ("Plaza", "Plaza pad (large, Downtown)", 0x2c701707000302ce),
    ("Plaza", "Plaza pad (large, Industrial)", 0x2c701707000302cf),
    ("Plaza", "Plaza pad (large, University)", 0x2c701707000302d0),
    ("Plaza", "Plaza pad (medium, Downtown)", 0x2c701707000302cb),
    ("Plaza", "Plaza pad (medium, Industrial)", 0x2c701707000302cc),
    ("Plaza", "Plaza pad (medium, University)", 0x2c701707000302cd),
    ("Plaza", "Plaza pad (small, Downtown)", 0x2c701707000302c8),
    ("Plaza", "Plaza pad (small, Industrial)", 0x2c701707000302c9),
    ("Plaza", "Plaza pad (small, University)", 0x2c701707000302ca),
    ("Plaza", "Bigstairs (large, Downtown)", 0x2c701707000304f6),
    ("Plaza", "Bigstairs (large, Industrial)", 0x2c70170700030500),
    ("Plaza", "Bigstairs (large, University)", 0x2c701707000300e7),
    ("Plaza", "Bigstairs (medium, Downtown)", 0x2c70170700030515),
    ("Plaza", "Bigstairs (medium, Industrial)", 0x2c70170700030520),
    ("Plaza", "Bigstairs (medium, University)", 0x2c701707000300e6),
    ("Plaza", "Bigstairs (small, Downtown)", 0x2c70170700030534),
    ("Plaza", "Bigstairs (small, Industrial)", 0x2c70170700030540),
    ("Plaza", "Bigstairs (small, University)", 0x00021c6f03e38707),
    ("Plaza", "Roundbigstairs 90 (large, Downtown)", 0x2c701707000304f9),
    ("Plaza", "Roundbigstairs 90 (large, Industrial)", 0x2c70170700030503),
    ("Plaza", "Roundbigstairs 90 (large, University)", 0x2c701707000300e0),
    ("Plaza", "Roundbigstairs 90 (medium, Downtown)", 0x2c70170700030518),
    ("Plaza", "Roundbigstairs 90 (medium, Industrial)", 0x2c70170700030523),
    ("Plaza", "Roundbigstairs 90 (medium, University)", 0x2c701707000300df),
    ("Plaza", "Roundbigstairs 90 (small, Downtown)", 0x2c70170700030537),
    ("Plaza", "Roundbigstairs 90 (small, Industrial)", 0x2c70170700030543),
    ("Plaza", "Roundbigstairs 90 (small, University)", 0x2c701707000300de),
    ("Plaza", "Roundbigstairs 90invert (large, Downtown)", 0x2c701707000304fa),
    ("Plaza", "Roundbigstairs 90invert (large, Industrial)", 0x2c70170700030504),
    ("Plaza", "Roundbigstairs 90invert (large, University)", 0x2c701707000300ec),
    ("Plaza", "Roundbigstairs 90invert (medium, Downtown)", 0x2c70170700030519),
    ("Plaza", "Roundbigstairs 90invert (medium, Industrial)", 0x2c70170700030524),
    ("Plaza", "Roundbigstairs 90invert (medium, University)", 0x2c701707000300eb),
    ("Plaza", "Roundbigstairs 90invert (small, Downtown)", 0x2c70170700030538),
    ("Plaza", "Roundbigstairs 90invert (small, Industrial)", 0x2c70170700030544),
    ("Plaza", "Roundbigstairs 90invert (small, University)", 0x2c70170700030573),
    ("Plaza", "Big stairs bowed in (large, Downtown)", 0x2c7017070003059e),
    ("Plaza", "Bigstairsbowedin (large, Industrial)", 0x2c7017070003059d),
    ("Plaza", "Big stairs bowed in (large, University)", 0x2c701707000302eb),
    ("Plaza", "Big stairs bowed in (medium, Downtown)", 0x2c701707000305a6),
    ("Plaza", "Bigstairsbowedin (medium, Industrial)", 0x2c701707000305a5),
    ("Plaza", "Big stairs bowed in (medium, University)", 0x2c701707000302ec),
    ("Plaza", "Big stairs bowed in (small, Downtown)", 0x2c701707000305ad),
    ("Plaza", "Bigstairsbowedin (small, Industrial)", 0x2c701707000305ac),
    ("Plaza", "Big stairs bowed in (small, University)", 0x2c701707000302ed),
    ("Plaza", "Big stairs bowed out (large, Downtown)", 0x2c701707000305a0),
    ("Plaza", "Bigstairsbowedout (large, Industrial)", 0x2c7017070003059f),
    ("Plaza", "Big stairs bowed out (large, University)", 0x2c7017070003031e),
    ("Plaza", "Big stairs bowed out (medium, Downtown)", 0x2c701707000305a7),
    ("Plaza", "Bigstairsbowedout (medium, Industrial)", 0x2c701707000305a8),
    ("Plaza", "Big stairs bowed out (medium, University)", 0x2c7017070003031f),
    ("Plaza", "Big stairs bowed out (small, Downtown)", 0x2c701707000305af),
    ("Plaza", "Bigstairsbowedout (small, Industrial)", 0x2c701707000305ae),
    ("Plaza", "Big stairs bowed out (small, University)", 0x2c70170700030320),
    ("Plaza", "Ledge planter", 0x2c70170700030321),
    ("Plaza", "Ledge planter 30 long curve", 0x2c70170700030324),
    ("Plaza", "Ledge planter 30 short curve", 0x2c70170700030325),
    ("Plaza", "Ledge planter 90 corner", 0x2c70170700030323),
    ("Plaza", "Ledge planter cap", 0x2c70170700030322),
    ("Plaza", "Water box (large)", 0x2c70170700030314),
    ("Plaza", "Water box (medium)", 0x2c70170700030310),
    ("Plaza", "Water box (small)", 0x2c70170700030309),
    ("Plaza", "Grass box (large)", 0x2c70170700030313),
    ("Plaza", "Grass box (medium)", 0x2c7017070003030b),
    ("Plaza", "Grass box (small)", 0x2c70170700030307),
    ("Plaza", "Bankedgrass (large)", 0x2c70170700030112),
    ("Plaza", "Bankedgrass (medium)", 0x2c7017070003010c),
    ("Plaza", "Bankedgrass (small)", 0x2c7017070003010e),
    ("Plaza", "Bankedgrasscorner (large)", 0x2c70170700030111),
    ("Plaza", "Bankedgrasscorner (medium)", 0x2c7017070003010b),
    ("Plaza", "Bankedgrasscorner (small)", 0x2c7017070003010f),
    ("Plaza", "Bankedgrasship (large)", 0x2c70170700030113),
    ("Plaza", "Bankedgrasship (medium)", 0x2c7017070003010d),
    ("Plaza", "Bankedgrasship (small)", 0x2c70170700030110),
    ("Mega", "Mega flat ramp", 0x2c701707000303c2),
    ("Mega", "Mega park roll in", 0x2c701707000303c5),
    ("Mega", "Mega down ramp", 0x2c701707000303c6),
    ("Mega", "Mega lower ramp", 0x2c701707000303c7),
    ("Mega", "Mega launch ramp a", 0x2c701707000303c4),
    ("Mega", "Mega launch ramp b", 0x2c701707000303c3),
    ("Mega", "Mega landing ramp", 0x2c701707000303cf),
    ("Mega", "Mega quater pipe", 0x2c701707000303ce),
    ("Mega", "Mega quater pipe 90 corner", 0x2c701707000303e6),
    ("Mega", "Mega quater pipe 90 corner invert", 0x2c7017070003060d),
    ("Mega", "Mega park filler pad", 0x2c701707000303c1),
    ("Mega", "Mega park filler pad x lrg", 0x2c701707000304e5),
    ("Mega", "Full pipe straight", 0x2c7017070003057c),
    ("Mega", "Full pipe short 45", 0x2c7017070003057e),
    ("Mega", "Full pipe 90", 0x2c701707000305f5),
    ("Catwalks & misc", "Catwalkstairs (Downtown)", 0x2c7017070003018a),
    ("Catwalks & misc", "Catwalkstairs (Industrial)", 0x2c7017070003017f),
    ("Catwalks & misc", "Catwalkstairs (University)", 0x2c70170700030030),
    ("Catwalks & misc", "Catwalk (Downtown)", 0x2c70170700030189),
    ("Catwalks & misc", "Catwalk (Industrial)", 0x2c7017070003017e),
    ("Catwalks & misc", "Catwalk (University)", 0x2c7017070003002e),
    ("Catwalks & misc", "Catwalksupport (Downtown)", 0x2c7017070003018b),
    ("Catwalks & misc", "Catwalksupport (Industrial)", 0x2c70170700030180),
    ("Catwalks & misc", "Catwalksupport (University)", 0x2c7017070003003b),
    ("Catwalks & misc", "Measuring stick", 0x2c701707000303cc),
    ("Catwalks & misc", "Jumbo tron", 0x2c70170700030575),
    ("Catwalks & misc", "Jumbo tron b", 0x2c70170700030579),
    ("Catwalks & misc", "Grand stand", 0x2c7017070003057a),
    ("Catwalks & misc", "Dr pepper can balloon", 0x2c701707000305e7),
    ("Catwalks & misc", "Chain link fence", 0x2c701707000305d1),
    ("Catwalks & misc", "Tri pod camera", 0x2c701707000303cb),
    ("Branded", "Branded pole flag", 0x2c701707000304eb),
    ("Branded", "Branded mediatower", 0x2c701707000304ee),
    ("Branded", "Branded tent svm", 0x2c701707000304f1),
    ("Branded", "Branded billboard wide", 0x2c701707000304f0),
    ("Branded", "Branded bill board wall mount", 0x2c701707000305f3),
    ("Branded", "Bill board stand", 0x2c701707000305f4),
    ("Downtown", "Bench wrainbow rail", 0x2c701707000300cd),
    ("Downtown", "Benchclean organic", 0x2c701707000300ce),
    ("Downtown", "Floatingbench", 0x2c701707000300c2),
    ("Downtown", "Waterfrontholebench", 0x2c701707000300c4),
    ("Downtown", "Lampost blue", 0x2c70170700030319),
    ("Downtown", "Lampost diamond metal", 0x2c7017070003031a),
    ("Downtown", "Lampost metal", 0x2c70170700030317),
    ("Downtown", "Lampost reclaimed", 0x2c70170700030318),
    ("Downtown", "Ad board shelter", 0x2c7017070003031b),
    ("Downtown", "Ad board", 0x2c7017070003031c),
    ("Downtown", "Awning", 0x2c701707000303bc),
    ("Downtown", "Gazebo", 0x2c701707000303bb),
    ("Foliage", "Hedge", 0x2c701707000300e3),
    ("Foliage", "Shrub", 0x2c701707000300e4),
    ("Foliage", "Bush", 0x2c701707000300e2),
    ("Foliage", "Fall tree", 0x2c701707000300e5),
    ("Foliage", "Palm tree", 0x2c701707000303bd),
    ("Foliage", "Short palm", 0x2c701707000305f0),
    ("Foliage", "Tree yucca", 0x2c701707000305f2),
    ("Big objects", "Brewery watertower", 0x2c70170700030593),
    ("Big objects", "Cubes", 0x2c70170700030594),
    ("Big objects", "Mall store sign", 0x2c701707000305bc),
    ("Big objects", "Brewery island", 0x2c701707000305bd),
    ("Big objects", "Hotel planter", 0x2c701707000305be),
    ("Big objects", "Spillway street planter", 0x2c701707000305c1),
    ("Big objects", "Spillway legdes", 0x2c701707000305c0),
    ("Big objects", "University sign", 0x2c70170700030590),
    ("Big objects", "University shark", 0x2c70170700030591),
    ("Big objects", "Stadium trans wall", 0x2c70170700030592),
    ("Big objects", "The kink", 0x2c70170700030604),
    ("Big objects", "Round planter", 0x2c70170700030605),
    ("Big objects", "Industrial guard rail", 0x2c7017070003058a),
    ("Big objects", "Propeller", 0x2c7017070003058e),
    ("Big objects", "Industrial stacks", 0x2c7017070003058f),
    ("Big objects", "Skylight", 0x2c701707000305fc),
    ("Big objects", "Electric box", 0x2c701707000305fd),
    ("Big objects", "In bank end", 0x2c701707000305fe),
    ("Big objects", "In bank mid", 0x2c701707000305ff),
    ("Big objects", "Ind sign", 0x2c70170700030603),
    ("Big objects", "Logging vehicle", 0x2c7017070003058b),
    ("Big objects", "Tractor yellow a", 0x2c701707000304f4),
    ("Big objects", "Flat bed trailer", 0x2c7017070003058c),
    ("Big objects", "Tanker trailer", 0x2c7017070003058d),
    ("S3 barriers", "Bench flat", 0x2c701707000f00df),
    ("S3 barriers", "Bench", 0x2c701707001102d1),
    ("S3 barriers", "Barrier plastic", 0x2c701707000601de),
    ("S3 barriers", "Bent traffic barrier", 0x2c701707001102d9),
    ("S3 barriers", "Bench aggregate", 0x2c701707000f00e3),
    ("S3 barriers", "Barrier traffic", 0x2c701707000601fb),
    ("S3 barriers", "Bench contemp simple", 0x2c701707000600d3),
    ("S3 barriers", "Bench reclaimed", 0x2c701707000f0096),
    ("S3 barriers", "Canadian barrier", 0x2c701707001102ce),
    ("S3 barriers", "Bench z shaped metal", 0x2c701707000f00f6),
    ("S3 barriers", "Safety barrier long", 0x2c70170700110588),
    ("S3 barriers", "Communtiy skate park bench", 0x00020dd103e38707),
    ("S3 barriers", "Bench barcelona", 0x2c701707000601e1),
    ("S3 bins", "Garbage bin", 0x2c701707001102d2),
    ("S3 bins", "Garbage bin blue", 0x2c701707000f00e1),
    ("S3 bins", "Garbage can reclaimed", 0x2c701707000f00fd),
    ("S3 bins", "Garbage bin angular", 0x2c701707000f00f9),
    ("S3 bins", "Garbage can contemporary", 0x2c701707000600d2),
    ("S3 bins", "Garbage can square", 0x2c701707000600e2),
    ("S3 bins", "Recycling bin", 0x2c701707000600d4),
    ("S3 bins", "Dumpster red", 0x2c701707000f00aa),
    ("S3 bins", "Dumpster closed", 0x2c701707000600cf),
    ("S3 bins", "Dumpster open", 0x2c701707000600ce),
    ("S3 bins", "Dumpster large blue", 0x2c701707000f0101),
    ("S3 bins", "Dumpster small yellow", 0x2c701707000f00fc),
    ("S3 bins", "Dumpster medium", 0x2c701707000f00fb),
    ("S3 bins", "Lnd dumpster", 0x2c701707001102d6),
    ("S3 boxes", "Mail box large", 0x2c701707000f00fa),
    ("S3 boxes", "Newspaper box reclaimed black", 0x2c701707000f00a9),
    ("S3 boxes", "Newspaper box reclaimed red", 0x2c701707000f00a8),
    ("S3 boxes", "Newspaper box triple", 0x2c701707000f00f8),
    ("S3 boxes", "Newspaper box yellow", 0x2c701707000f00ed),
    ("S3 boxes", "Old newspaperbin 2", 0x2c701707000503d8),
    ("S3 misc", "Construction sign stand", 0x2c701707000601e5),
    ("S3 misc", "Portable potty", 0x2c701707001102df),
    ("S3 misc", "Wood stack", 0x0001f52403e38707),
    ("S3 misc", "Soccer net", 0x2c701707000600d7),
    ("S3 misc", "Vending machine", 0x2c701707000600d5),
    ("S3 misc", "Container 1", 0x2c7017070003040f),
    ("S3 misc", "Container 2", 0x2c7017070003040e),
    ("S3 misc", "Ball stainless steel", 0x2c701707000f00f7),
    ("S3 misc", "Plywood", 0x2c70170700110585),
    ("S3 misc", "Shoppingcart", 0x2c701707002f06a0),
    ("S3 misc", "Domino", 0x2c70170700030610),
    ("S3 misc", "Basketball stantion", 0x2c701707000600db),
    ("S3 misc", "Miracle whip balloon", 0x2c70170700030578),
    ("S3 misc", "T mobile manual pad", 0x2c70170700030574),
    ("S3 misc", "Zumiez couch", 0x2c701707000304f3),
    ("S3 misc", "Dr pepper vending machine", 0x2c70170700030577),
    ("S3 misc", "Monster vending machine", 0x0001dcb303e38707),
    ("S3 rails", "Rail fancy med flat med", 0x2c701707000601e2),
    ("S3 rails", "Rail long bend low", 0x2c701707000601d6),
    ("S3 rails", "Rail long corner bend low", 0x2c701707000601df),
    ("S3 rails", "Rail long corner bend med", 0x2c701707000601e0),
    ("S3 rails", "Rail long flat high", 0x2c701707000601d5),
    ("S3 rails", "Rail long flat low", 0x2c701707000601d4),
    ("S3 rails", "Rail long flat med", 0x2c701707000601d8),
    ("S3 rails", "Rail long hi low", 0x2c701707000601d7),
    ("S3 rails", "Bike rack", 0x2c701707000600cd),
    ("S3 rails", "Ibeam", 0x2c701707001102d4),
    ("S3 rails", "Kinked tbar", 0x2c701707001102d8),
    ("S3 rails", "Long pipe", 0x2c70170700110586),
    ("S3 rails", "Guard rail grind", 0x2c701707000600d9),
    ("S3 rails", "Hurdle short", 0x2c701707000600d1),
    ("S3 rails", "Hurdle tall", 0x2c701707000600d0),
    ("S3 rails", "Flat bar", 0x2c701707001102d0),
    ("S3 rails", "Billboard on stand", 0x2c701707000601e7),
    ("S3 rails", "Lampole", 0x2c70170700110584),
    ("S3 rails", "Short pipe", 0x2c70170700110587),
    ("S3 ramps", "Large quarter pipe deck", 0x2c70170700110582),
    ("S3 ramps", "Large quarter pipe", 0x2c70170700110581),
    ("S3 ramps", "Medium quarter pipe deck", 0x2c7017070011058e),
    ("S3 ramps", "Medium quarter pipe", 0x2c7017070011058f),
    ("S3 ramps", "Manual padrail 1", 0x2c7017070011058c),
    ("S3 ramps", "Cracked ground jump", 0x2c701707000600da),
    ("S3 ramps", "Ramp", 0x2c701707001102e0),
    ("S3 ramps", "Slide with platform stair", 0x2c701707000601e9),
    ("S3 ramps", "Staircase moveable", 0x2c701707000601e8),
    ("S3 ramps", "Tile broken", 0x2c701707000f00e2),
    ("S3 tables", "Picnic table", 0x00020dec03e38707),
    ("S3 tables", "Picnic table ramp", 0x2c701707000600d8),
    ("S3 tables", "Table", 0x2c701707001102d3),
    ("S3 tables", "Patio table", 0x2c7017070011058d),
    ("Characters", "AI skater 1", marquee_id("ai_skater_01")),
    ("Characters", "AI skater 2", marquee_id("ai_skater_02")),
    ("Characters", "AI skater 3", marquee_id("ai_skater_03")),
    ("Characters", "AI skater 4", marquee_id("ai_skater_04")),
    ("Characters", "AI skater 5", marquee_id("ai_skater_05")),
    ("Characters", "AI skater 6", marquee_id("ai_skater_06")),
    ("Characters", "AI skater 7", marquee_id("ai_skater_07")),
    ("Characters", "AI skater 8", marquee_id("ai_skater_08")),
    ("Characters", "AI skater 9", marquee_id("ai_skater_09")),
    ("Characters", "Alex Chalmers", marquee_id("alex_chalmers")),
    ("Characters", "Attiba Jefferson", marquee_id("attiba_jefferson")),
    ("Characters", "Attiba Jefferson (prop)", marquee_id("attiba_jefferson_prop")),
    ("Characters", "Big Black", marquee_id("big_black")),
    ("Characters", "Brayden Szafranski", marquee_id("brayden_szafranski")),
    ("Characters", "Chris Cole", marquee_id("chris_cole")),
    ("Characters", "Chris Haslam", marquee_id("chris_haslam")),
    ("Characters", "Colin McKay", marquee_id("colin_mckay")),
    ("Characters", "Cuz", marquee_id("cuz")),
    ("Characters", "Cuz (can)", marquee_id("cuz_can")),
    ("Characters", "Danny Way", marquee_id("danny_way")),
    ("Characters", "Darren Navarette", marquee_id("darren_navarette")),
    ("Characters", "Dem Bones", marquee_id("dem_bones")),
    ("Characters", "Dennis Busenitz", marquee_id("dennis_busenitz")),
    ("Characters", "Eric Koston", marquee_id("eric_koston")),
    ("Characters", "Freeway", marquee_id("freeway")),
    ("Characters", "Giovanni Reda", marquee_id("giovanni_reda")),
    ("Characters", "Giovanni Reda (prop)", marquee_id("giovanni_reda_prop")),
    ("Characters", "Jake Brown", marquee_id("jake_brown")),
    ("Characters", "Jason Dill", marquee_id("jason_dill")),
    ("Characters", "Jerry Hsu", marquee_id("jerry_hsu")),
    ("Characters", "John Cardiel", marquee_id("john_cardiel")),
    ("Characters", "John Rattray", marquee_id("john_rattray")),
    ("Characters", "Lucas Puig", marquee_id("lucas_puig")),
    ("Characters", "Marc Johnson", marquee_id("marc_johnson")),
    ("Characters", "Mark Appleyard", marquee_id("mark_appleyard")),
    ("Characters", "Michael Burnette", marquee_id("michael_burnette")),
    ("Characters", "Michael Burnette (prop)", marquee_id("michael_burnette_prop")),
    ("Characters", "Mike", marquee_id("mike")),
    ("Characters", "Mike Carroll", marquee_id("mike_carroll")),
    ("Characters", "Mike (prop)", marquee_id("mike_prop")),
    ("Characters", "Pat Duffy", marquee_id("pat_duffy")),
    ("Characters", "PJ Ladd", marquee_id("pj_ladd")),
    ("Characters", "Ray Barbee", marquee_id("ray_barbee")),
    ("Characters", "Rob Dyrdek", marquee_id("rob_dyrdek")),
    ("Characters", "Ryan Gallant", marquee_id("ryan_gallant")),
    ("Characters", "Ryan Smith", marquee_id("ryan_smith")),
    ("Characters", "Sammy", marquee_id("sammy")),
    ("Characters", "Sammy (prop)", marquee_id("sammy_prop")),
    ("Characters", "Seb", marquee_id("seb")),
    ("Characters", "Seb (prop)", marquee_id("seb_prop")),
    ("Characters", "Security guard", marquee_id("security_03")),
    ("Characters", "Shingo", marquee_id("shingo")),
    ("Characters", "Slappy", marquee_id("slappy")),
    ("Characters", "Slappy (prop)", marquee_id("slappy_prop")),
    ("Characters", "Terry Kennedy", marquee_id("terry_kennedy")),
    ("Characters", "Tutorial skater", marquee_id("tutorial_male")),
    ("Marquee models", "Podium", marquee_id("z_podium")),
    ("Marquee models", "Big Black's SUV", marquee_id("z_bigblacks_suv")),
    ("Marquee models", "Chair and table", marquee_id("z_chair_table")),
    ("Marquee models", "Pen", marquee_id("z_pen")),
    ("Marquee models", "Railing cap", marquee_id("z_railing_cap")),
    ("Marquee models", "Tally board", marquee_id("z_tally_board")),
    ("Mission stage", "Speaker", mission_prop_id("Speaker")),
    ("Mission stage", "Grandstand", mission_prop_id("Grandstand")),
    ("Mission stage", "Entrance banner (GVR)", mission_prop_id("Entrance banner (GVR)")),
    ("Mission stage", "Tent (black)", mission_prop_id("Tent (black)")),
    ("Mission stage", "Tent (small)", mission_prop_id("Tent (small)")),
    ("Mission stage", "Media tower (GVR)", mission_prop_id("Media tower (GVR)")),
    ("Mission stage", "Media tower (Black Box)", mission_prop_id("Media tower (Black Box)")),
    ("Mission stage", "Media tower (Zero, Creature...)", mission_prop_id("Media tower (Zero, Creature...)")),
    ("Mission stage", "Media tower (Anti Hero, Blind...)", mission_prop_id("Media tower (Anti Hero, Blind...)")),
    ("Mission stage", "Media tower (Lakai, Girl...)", mission_prop_id("Media tower (Lakai, Girl...)")),
    ("Mission stage", "Media tower (Slappy's, Thrasher...)", mission_prop_id("Media tower (Slappy's, Thrasher...)")),
    ("Mission stage", "Porta-potty", mission_prop_id("Porta-potty")),
    ("Mission stage", "Couch", mission_prop_id("Couch")),
    ("Mission flags", "Flag (AWS)", mission_prop_id("Flag (AWS)")),
    ("Mission flags", "Flag (Adidas)", mission_prop_id("Flag (Adidas)")),
    ("Mission flags", "Flag (Almost)", mission_prop_id("Flag (Almost)")),
    ("Mission flags", "Flag (Anti Hero)", mission_prop_id("Flag (Anti Hero)")),
    ("Mission flags", "Flag (Baker)", mission_prop_id("Flag (Baker)")),
    ("Mission flags", "Flag (Black Box)", mission_prop_id("Flag (Black Box)")),
    ("Mission flags", "Flag (Blind)", mission_prop_id("Flag (Blind)")),
    ("Mission flags", "Flag (Chocolate)", mission_prop_id("Flag (Chocolate)")),
    ("Mission flags", "Flag (Cliche)", mission_prop_id("Flag (Cliche)")),
    ("Mission flags", "Flag (Creature)", mission_prop_id("Flag (Creature)")),
    ("Mission flags", "Flag (Element)", mission_prop_id("Flag (Element)")),
    ("Mission flags", "Flag (Enjoi)", mission_prop_id("Flag (Enjoi)")),
    ("Mission flags", "Flag (Etnies)", mission_prop_id("Flag (Etnies)")),
    ("Mission flags", "Flag (Fallen)", mission_prop_id("Flag (Fallen)")),
    ("Mission flags", "Flag (Flip)", mission_prop_id("Flag (Flip)")),
    ("Mission flags", "Flag (Girl)", mission_prop_id("Flag (Girl)")),
    ("Mission flags", "Flag (Mystery)", mission_prop_id("Flag (Mystery)")),
    ("Mission flags", "Flag (Plan B)", mission_prop_id("Flag (Plan B)")),
    ("Mission flags", "Flag (Real)", mission_prop_id("Flag (Real)")),
    ("Mission flags", "Flag (Skateboard Mag)", mission_prop_id("Flag (Skateboard Mag)")),
    ("Mission flags", "Flag (Slap)", mission_prop_id("Flag (Slap)")),
    ("Mission flags", "Flag (Slappy's)", mission_prop_id("Flag (Slappy's)")),
    ("Mission flags", "Flag (Thrasher)", mission_prop_id("Flag (Thrasher)")),
    ("Mission flags", "Flag (Vox)", mission_prop_id("Flag (Vox)")),
    ("Mission flags", "Flag (Vans)", mission_prop_id("Flag (Vans)")),
    ("Mission flags", "Flag (Zero)", mission_prop_id("Flag (Zero)")),
    ("Mission other", "Perimeter fence", mission_prop_id("Perimeter fence")),
    ("Mission other", "Wooden fence (long)", mission_prop_id("Wooden fence (long)")),
    ("Mission other", "Cargo container", mission_prop_id("Cargo container")),
    ("Mission other", "Bike rack", mission_prop_id("Bike rack")),
];

/// A prop standing upright at `at`, turned to `heading` (radians about the vertical, 0 facing
/// +Z), as a row-major DMO transform (axis rows, translation row 3).
pub(crate) fn prop_transform(at: [f32; 3], heading: f32) -> [f32; 16] {
    let (sin, cos) = heading.sin_cos();
    [
        cos, 0.0, -sin, 0.0,
        0.0, 1.0, 0.0, 0.0,
        sin, 0.0, cos, 0.0,
        at[0], at[1], at[2], 1.0,
    ]
}

/// The skater's yaw only, `DROP_DISTANCE` ahead of the root, as a row-major DMO transform
/// (axis rows, translation row 3). Facing the skater's way, so a kicker or ramp is ridden into.
#[cfg(test)]
pub(crate) fn drop_transform(root: [f32; 16]) -> [f32; 16] {
    let mut forward = [root[8], 0.0, root[10]];
    let length = forward[0].hypot(forward[2]);
    if length > 1e-4 {
        forward = [forward[0] / length, 0.0, forward[2] / length];
    } else {
        forward = [0.0, 0.0, 1.0];
    }
    let right = [forward[2], 0.0, -forward[0]];
    let at = [
        root[12] + forward[0] * DROP_DISTANCE,
        root[13],
        root[14] + forward[2] * DROP_DISTANCE,
    ];
    [
        right[0], right[1], right[2], 0.0,
        0.0, 1.0, 0.0, 0.0,
        forward[0], forward[1], forward[2], 0.0,
        at[0], at[1], at[2], 1.0,
    ]
}

#[cfg(test)]
mod catalog_id_tests {
    use super::*;

    /// Every catalog entry has its own ID, mission props and marquee entries find themselves
    /// by it, and their IDs are fixed by name (pinned here: online, a placed prop is its ID).
    #[test]
    fn catalog_ids_are_unique_and_stable() {
        let mut seen = std::collections::HashMap::new();
        for (index, entry) in PROPS.iter().enumerate() {
            if let Some(other) = seen.insert(entry.2, index) {
                assert!(entry.2 == YOUR_BOARD && PROPS[other].2 == YOUR_BOARD, "{} and {} share {:#x}", entry.1, PROPS[other].1, entry.2);
            }
        }
        for (index, prop) in MISSION_PROPS.iter().enumerate() {
            assert_eq!(mission_prop(mission_prop_id(prop.name)), Some(index), "{}", prop.name);
        }
        for (index, (recipe, _)) in MARQUEE.iter().enumerate() {
            assert_eq!(marquee_entry(marquee_id(recipe)), Some(index), "{recipe}");
        }
        assert_eq!(mission_prop_id("Speaker"), 0x4d49_5353_0000_0000 | u64::from(name_key("Speaker")));
        assert_eq!(name_key(""), 0x811c_9dc5);
        assert_eq!(name_key("a"), 0xe40c_292c);
        assert_eq!(mission_prop(MISSION_PROP_BASE), None);
        assert_eq!(marquee_entry(MARQUEE_BASE + 1), None);
    }

    /// Every themed Skate 3 piece lists as one row holding all three themes, each findable.
    #[test]
    fn themed_pieces_list_once() {
        let mut rows = 0;
        let mut index = 0;
        while index < PROPS.len() {
            let (start, len) = theme_row(index);
            assert_eq!(start, index, "{}", PROPS[index].1);
            if len > 1 {
                rows += 1;
                assert_eq!(len, THEMES.len(), "{}", PROPS[index].1);
                for theme in 0..THEMES.len() {
                    assert_eq!(theme_row(in_theme(index, theme)), (start, len), "{}", PROPS[index].1);
                    assert_eq!(split_theme(PROPS[in_theme(index, theme)].1).map(|(_, t)| t), Some(theme));
                }
            }
            index += len;
        }
        assert_eq!(row_name(in_theme(PROPS.iter().position(|entry| entry.1 == "Ledgedown b (large, Industrial)").unwrap(), 1)), "Ledge down b (large)");
        assert!(rows > 100, "{rows} themed rows");
    }
}
