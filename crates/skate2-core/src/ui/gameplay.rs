//! PC mod preferences, independent of Skate 2's game modes and career state.
use std::{fs, io, path::{Path, PathBuf}};
use super::online_config::{quote, strict_string_fields};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulationFamily { Skate2, Skate3 }

/// Individual replacements remain meaningful when the complete replacement is off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Skate3Features {
    pub all: bool,
    pub darkcatch: bool,
    pub footplants: bool,
    pub offboard: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsFeature { All, Darkcatch, Footplants, Offboard }
impl Skate3Features {
    pub const ALL: Self = Self { all: true, darkcatch: true, footplants: true, offboard: true };
    #[must_use]
    pub const fn enabled(self, feature: PhysicsFeature) -> bool {
        match feature { PhysicsFeature::All => self.all, PhysicsFeature::Darkcatch => self.darkcatch,
            PhysicsFeature::Footplants => self.footplants, PhysicsFeature::Offboard => self.offboard }
    }
    pub fn toggle(&mut self, feature: PhysicsFeature) {
        if feature == PhysicsFeature::All {
            *self = if self.all { Self::default() } else { Self::ALL };
        } else {
            let value = !self.enabled(feature);
            match feature {
                PhysicsFeature::Darkcatch => self.darkcatch = value,
                PhysicsFeature::Footplants => self.footplants = value,
                PhysicsFeature::Offboard => self.offboard = value,
                PhysicsFeature::All => unreachable!(),
            }
            if !value { self.all = false; }
        }
    }
    #[must_use]
    pub const fn simulation(self, offboard: bool, board_transition: bool) -> SimulationFamily {
        if self.all || (self.offboard && (offboard || board_transition)) {
            SimulationFamily::Skate3
        } else { SimulationFamily::Skate2 }
    }
    #[must_use]
    pub const fn is_empty(self) -> bool { !self.all && !self.darkcatch && !self.footplants && !self.offboard }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Difficulty {
    #[default]
    Skate2Default, Easy, Normal, Hardcore, Motorized, EasyMotorized,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuningSource { Skate2, Skate3(&'static str) }
impl Difficulty {
    pub const ALL: [Self; 6] = [Self::Skate2Default, Self::Easy, Self::Normal, Self::Hardcore, Self::Motorized, Self::EasyMotorized];
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self { Self::Skate2Default => "skate2", Self::Easy => "easy", Self::Normal => "normal",
            Self::Hardcore => "hardcore", Self::Motorized => "motorized", Self::EasyMotorized => "easy_motorized" }
    }
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self { Self::Skate2Default => "Skate 2", Self::Easy => "Easy", Self::Normal => "Normal",
            Self::Hardcore => "Hardcore", Self::Motorized => "Motorized", Self::EasyMotorized => "Easy Motorized" }
    }
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        // The first playtest used this label. Keep existing preferences readable.
        if key == "skate2_default" { return Some(Self::Skate2Default); }
        Self::ALL.into_iter().find(|value| value.key() == key)
    }
    #[must_use]
    pub fn step(self, forward: bool) -> Self {
        let index = Self::ALL.iter().position(|&value| value == self).unwrap_or(0);
        Self::ALL[(index + if forward { 1 } else { Self::ALL.len() - 1 }) % Self::ALL.len()]
    }
    #[must_use]
    pub const fn tuning(self) -> TuningSource {
        match self { Self::Skate2Default => TuningSource::Skate2,
            Self::Easy | Self::EasyMotorized => TuningSource::Skate3("easy"),
            Self::Normal => TuningSource::Skate3("normal"), Self::Hardcore => TuningSource::Skate3("hardcore"),
            Self::Motorized => TuningSource::Skate3("motorized") }
    }
    #[must_use]
    pub const fn motor_enabled(self) -> bool { matches!(self, Self::Motorized | Self::EasyMotorized) }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameplaySettings { pub skate3: Skate3Features, pub difficulty: Difficulty, pub airtime_slow_motion: bool }
impl GameplaySettings {
    pub fn step_difficulty(&mut self, forward: bool) { self.difficulty = self.difficulty.step(forward); }
    #[must_use]
    pub const fn simulation(self, offboard: bool, board_transition: bool) -> SimulationFamily {
        self.skate3.simulation(offboard, board_transition)
    }
    #[must_use]
    pub const fn stock_skate2(self) -> bool { self.skate3.is_empty() && matches!(self.difficulty, Difficulty::Skate2Default) }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GameplayConfig { pub settings: GameplaySettings }
impl GameplayConfig {
    pub fn path() -> io::Result<PathBuf> {
        let root = std::env::var_os("SKATE2_SAVE_ROOT")
            .map_or_else(std::env::current_dir, |root| Ok(PathBuf::from(root)))?;
        Ok(root.join("gameplay.json"))
    }

    /// Only a missing file selects the stock first-run defaults. Invalid
    /// preferences must be reported rather than silently selecting stock physics.
    pub fn load() -> io::Result<Self> {
        Self::load_from(&Self::path()?)
    }

    pub fn load_from(path: &Path) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to(&Self::path()?)
    }

    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.text())
    }

    #[must_use]
    pub fn text(&self) -> String {
        let value = |on| quote(if on { "on" } else { "off" });
        let features = self.settings.skate3;
        format!("{{\n  \"version\": \"3\",\n  \"all_skate3_physics\": {},\n  \"darkcatch\": {},\n  \"footplants\": {},\n  \"offboard\": {},\n  \"difficulty\": {},\n  \"airtime_slow_motion\": {}\n}}\n",
            value(features.all), value(features.darkcatch), value(features.footplants), value(features.offboard),
            quote(self.settings.difficulty.key()), value(self.settings.airtime_slow_motion))
    }
    pub fn parse(text: &str) -> io::Result<Self> {
        let invalid = |message: String| io::Error::new(io::ErrorKind::InvalidData, message);
        let fields = strict_string_fields(text).ok_or_else(|| invalid("Invalid gameplay.json object".into()))?;
        let mut config = Self::default();
        let mut seen = std::collections::BTreeSet::new();
        let mut legacy = false;
        let mut current = false;
        for (key, value) in fields {
            if !seen.insert(key.clone()) { return Err(invalid(format!("Duplicate gameplay setting {key}"))); }
            let toggle = || match value.as_str() {
                "on" => Ok(true), "off" => Ok(false), _ => Err(invalid(format!("Invalid {key} toggle {value:?}")))
            };
            match key.as_str() {
                "version" if value == "2" || value == "3" => current = true,
                "airtime_slow_motion" => { config.settings.airtime_slow_motion = toggle()?; current = true; }
                "all_skate3_physics" => { config.settings.skate3.all = toggle()?; current = true; }
                "darkcatch" => { config.settings.skate3.darkcatch = toggle()?; current = true; }
                "footplants" => { config.settings.skate3.footplants = toggle()?; current = true; }
                "offboard" => { config.settings.skate3.offboard = toggle()?; current = true; }
                "difficulty" => config.settings.difficulty = Difficulty::parse(&value)
                    .ok_or_else(|| invalid(format!("Unknown difficulty {value:?}")))?,
                "physics_mode" => {
                    legacy = true;
                    config.settings.skate3 = match value.as_str() {
                        "skate2" => Skate3Features::default(), "skate3" => Skate3Features::ALL,
                        "skate2_skate3_offboard" => Skate3Features { offboard: true, ..Skate3Features::default() },
                        _ => return Err(invalid(format!("Unknown legacy physics mode {value:?}"))),
                    };
                }
                _ => return Err(invalid(format!("Unknown gameplay setting {key}"))),
            }
        }
        if legacy && current { return Err(invalid("Mixed legacy/current mod preferences".into())); }
        let features = config.settings.skate3;
        if features.all && !(features.darkcatch && features.footplants && features.offboard) {
            return Err(invalid("All Skate 3 Physics requires each individual feature".into()));
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn feature_combinations_round_trip_and_difficulty_never_changes_solver() {
        for bits in 0..8 {
            let features = Skate3Features { all: false, darkcatch: bits & 1 != 0,
                footplants: bits & 2 != 0, offboard: bits & 4 != 0 };
            for features in [features, Skate3Features::ALL] {
                for difficulty in Difficulty::ALL {
                    let settings = GameplaySettings { skate3: features, difficulty, ..Default::default() };
                    let config = GameplayConfig { settings };
                    assert_eq!(GameplayConfig::parse(&config.text()).unwrap(), config);
                    assert_eq!(settings.simulation(false, false) == SimulationFamily::Skate3, features.all);
                    assert_eq!(settings.simulation(true, false) == SimulationFamily::Skate3, features.all || features.offboard);
                }
            }
        }
    }
    #[test]
    fn airtime_slow_motion_is_optional_persistent_and_disabled_for_old_preferences() {
        for text in [r#"{}"#, r#"{"version":"2","difficulty":"normal"}"#,
            r#"{"physics_mode":"skate3","difficulty":"easy"}"#] {
            assert!(!GameplayConfig::parse(text).unwrap().settings.airtime_slow_motion);
        }
        for airtime_slow_motion in [false, true] {
            let config=GameplayConfig { settings: GameplaySettings {
                skate3: Skate3Features::ALL, difficulty: Difficulty::Hardcore, airtime_slow_motion,
            } };
            assert_eq!(GameplayConfig::parse(&config.text()).unwrap(),config);
            assert_eq!(config.settings.simulation(false,false),SimulationFamily::Skate3);
        }
        assert!(GameplayConfig::parse(r#"{"airtime_slow_motion":"sometimes"}"#).is_err());
        assert!(GameplayConfig::parse(r#"{"airtime_slow_motion":"on","airtime_slow_motion":"off"}"#).is_err());
    }
    #[test]
    fn master_and_individual_toggles_keep_explicit_intent() {
        let mut features = Skate3Features::default();
        features.toggle(PhysicsFeature::All);
        assert_eq!(features, Skate3Features::ALL);
        features.toggle(PhysicsFeature::Darkcatch);
        assert!(!features.all && !features.darkcatch && features.footplants && features.offboard);
        features.toggle(PhysicsFeature::Darkcatch);
        assert!(!features.all); // The master includes additional source systems.
        features.toggle(PhysicsFeature::All);
        features.toggle(PhysicsFeature::All);
        assert!(features.is_empty());
    }
    #[test]
    fn legacy_preferences_migrate_without_changing_selected_physics() {
        let legacy = GameplayConfig::parse(r#"{"physics_mode":"skate3","difficulty":"normal"}"#).unwrap();
        assert_eq!(legacy.settings.skate3, Skate3Features::ALL);
        assert_eq!(legacy.settings.difficulty, Difficulty::Normal);
        assert!(GameplayConfig::parse(r#"{"physics_mode":"skate2","difficulty":"skate2_default"}"#).unwrap().settings.stock_skate2());
        assert_eq!(GameplayConfig::parse(r#"{"physics_mode":"skate2_skate3_offboard"}"#).unwrap().settings.skate3,
            Skate3Features { offboard: true, ..Skate3Features::default() });
    }
    #[test]
    fn malformed_or_ambiguous_preferences_fail_explicitly() {
        for text in ["broken", "{\x0b}", r#"{} trailing"#, r#"{"difficulty":3}"#,
            r#"{"offboard":"sometimes"}"#, r#"{"offboard":"on","offboard":"off"}"#,
            r#"{"physics_mode":"skate3","offboard":"off"}"#, r#"{"all_skate3_physics":"on"}"#,
            r#"{"difficulty":"easy",}"#, r#"{"unexpected":"value"}"#,
            r#"{"skate3_asset_root":"C:\Skate 3"}"#] {
            assert!(GameplayConfig::parse(text).is_err(), "{text}");
        }
    }
    #[test]
    fn six_difficulties_wrap_without_mutating_features() {
        let mut settings = GameplaySettings { skate3: Skate3Features::ALL, ..GameplaySettings::default() };
        for _ in Difficulty::ALL { settings.step_difficulty(true); assert_eq!(settings.skate3, Skate3Features::ALL); }
        assert_eq!(settings.difficulty, Difficulty::Skate2Default);
        assert_eq!(Difficulty::Skate2Default.step(false), Difficulty::EasyMotorized);
        assert_eq!(Difficulty::EasyMotorized.tuning(), Difficulty::Easy.tuning());
        assert_eq!(Difficulty::ALL.into_iter().filter(|value| value.motor_enabled()).count(), 2);
    }
}

/// Authored Skate 3 labels used by the additional tricks in the original HUD.
#[cfg(feature = "skate3-runtime")]
pub fn skate3_trick_strings() -> Result<Vec<(u32, Vec<u16>)>, String> {
    crate::physical_player_phys_out::conditioners::skate3_trick_strings()
}
