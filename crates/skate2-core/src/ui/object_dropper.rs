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
    pub(crate) const ROWS: [(&'static str, &'static str); 8] = [
        ("camera_move", "Camera move speed"),
        ("camera_look", "Camera look speed"),
        ("prop_move", "Prop move speed"),
        ("spin", "90 degree spin speed"),
        ("fine_turn", "Fine turn speed"),
        ("board_weight", "Dropped board weight (Skate 3)"),
        ("player_weight", "Player weight (Skate 3)"),
        ("player_board_weight", "Your board weight (Skate 3)"),
    ];
    /// Each setting's default: the speeds at 100%, a dropped board in Skate 3 physics at a
    /// quarter of the player's board (play-tested tuning).
    pub(crate) const DEFAULTS: [u16; Self::ROWS.len()] = [100, 100, 100, 100, 100, 25, 100, 100];
    pub(crate) const CAMERA_MOVE: usize = 0;
    pub(crate) const CAMERA_LOOK: usize = 1;
    pub(crate) const PROP_MOVE: usize = 2;
    pub(crate) const SPIN: usize = 3;
    pub(crate) const FINE_TURN: usize = 4;
    pub(crate) const BOARD_WEIGHT: usize = 5;
    pub(crate) const PLAYER_WEIGHT: usize = 6;
    pub(crate) const PLAYER_BOARD_WEIGHT: usize = 7;

    /// A setting's range and how much each D-pad press changes it (percent).
    pub(crate) const fn limits(row: usize) -> (u16, u16, u16) {
        match row {
            Self::BOARD_WEIGHT => (5, 200, 5),
            Self::PLAYER_WEIGHT | Self::PLAYER_BOARD_WEIGHT => (25, 1000, 25),
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

/// The catalog entry for a copy of the player's own board: a real riderless board (not a bank
/// prop), drawn with the player's board.
pub(crate) const YOUR_BOARD: u64 = 0;

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
