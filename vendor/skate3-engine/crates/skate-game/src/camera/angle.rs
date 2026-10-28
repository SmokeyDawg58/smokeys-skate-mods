//! Retail "Camera Angle" game setting (Game Settings > Control Settings: High / Low).
//!
//! Retail keeps the choice in the camera preferences object (TU3 `*(0x83085450)` +20, zeroed by
//! the constructor `82DF61F0`), and the stock camera graph's `IsCameraTypeActive` condition
//! (`82DF4F70`) compares its `type` attribute with it. `Default_cameragraph` routes type 0 (or
//! observer mode) to `cameragraph_low.xml` and type 1 to `cameragraph_high.xml`, which choose
//! different stock shots (`bl_chase` vs `bl_high_chase`, ...). Every value of both cameras is
//! setup data; this module only owns the selection, its persistence and the mod surface.
//!
//! Engine-facing: [`CameraAngleSettings`] is the one source of truth. [`sync_runtime`] copies
//! the active angle and the shot tunings into [`super::CameraRuntime`] every frame, so a map
//! reload (which rebuilds the runtime) keeps them.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use skate_mods::presentation::CameraShotTuning;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CameraAngle {
    /// Graph type 0.
    Low,
    /// Graph type 1. The engine's default: retail asks at first boot instead (boot flow
    /// "Choose your camera:"), so there is no shipped default to copy.
    #[default]
    High,
}

impl CameraAngle {
    pub const ALL: [Self; 2] = [Self::High, Self::Low];
    /// The stock camera graph's `IsCameraTypeActive` value.
    pub fn graph_type(self) -> u32 {
        match self {
            Self::Low => 0,
            Self::High => 1,
        }
    }
    /// Retail labels (`ID_GAMESETTINGS_CAMERA_LOW` / `_HIGH`).
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::High => "High",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::High => "high",
        }
    }
}

impl From<skate_mods::presentation::CameraAngle> for CameraAngle {
    fn from(value: skate_mods::presentation::CameraAngle) -> Self {
        match value {
            skate_mods::presentation::CameraAngle::Low => Self::Low,
            skate_mods::presentation::CameraAngle::High => Self::High,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Saved {
    angle: CameraAngle,
}

/// The player's Camera Angle plus what mods have layered on top of it.
#[derive(Resource, Debug)]
pub(crate) struct CameraAngleSettings {
    /// The player's choice (`settings/camera.json`).
    pub selected: CameraAngle,
    forced: Option<(String, CameraAngle)>,
    tunings: BTreeMap<String, (String, CameraShotTuning)>,
    /// Bumped whenever `tunings` changes; the runtime re-reads them when it differs.
    generation: u64,
    path: PathBuf,
}

impl CameraAngleSettings {
    pub fn path(asset_root: &Path) -> PathBuf {
        asset_root
            .parent()
            .unwrap_or(asset_root)
            .join("settings/camera.json")
    }

    pub fn new(selected: CameraAngle, path: PathBuf) -> Self {
        Self {
            selected,
            forced: None,
            tunings: BTreeMap::new(),
            generation: 1,
            path,
        }
    }

    /// A missing file is the default; an unreadable one is reported and ignored.
    pub fn load(asset_root: &Path) -> Self {
        let path = Self::path(asset_root);
        let selected = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<Saved>(&bytes)
                .map(|s| s.angle)
                .unwrap_or_else(|e| {
                    warn!("Camera settings {}: {e}", path.display());
                    CameraAngle::default()
                }),
            Err(_) => CameraAngle::default(),
        };
        Self::new(selected, path)
    }

    pub fn save(&self) -> Result<(), String> {
        std::fs::create_dir_all(self.path.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(
            &self.path,
            serde_json::to_vec_pretty(&Saved {
                angle: self.selected,
            })
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }

    /// What the camera graph uses: a mod's forced angle, else the player's setting.
    pub fn active(&self) -> CameraAngle {
        self.forced
            .as_ref()
            .map_or(self.selected, |(_, angle)| *angle)
    }

    pub fn forced_by(&self) -> Option<&str> {
        self.forced.as_ref().map(|(owner, _)| owner.as_str())
    }

    pub fn tunings(&self) -> impl Iterator<Item = (&str, &CameraShotTuning)> {
        self.tunings
            .iter()
            .map(|(shot, (_, tuning))| (shot.as_str(), tuning))
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// `angle == None` releases the owner's hold.
    pub fn force(&mut self, owner: &str, angle: Option<CameraAngle>) -> Result<(), String> {
        if let Some((holder, _)) = &self.forced
            && holder != owner
        {
            return Err(format!("camera angle is forced by mod {holder}"));
        }
        self.forced = angle.map(|angle| (owner.to_owned(), angle));
        Ok(())
    }

    /// `tuning == None` restores the stock shot. The caller checks that the shot exists.
    pub fn tune(
        &mut self,
        owner: &str,
        shot: &str,
        tuning: Option<CameraShotTuning>,
    ) -> Result<(), String> {
        if let Some((holder, _)) = self.tunings.get(shot)
            && holder != owner
        {
            return Err(format!("camera shot {shot} is tuned by mod {holder}"));
        }
        match tuning {
            Some(tuning) => {
                self.tunings
                    .insert(shot.to_owned(), (owner.to_owned(), tuning));
            }
            None => {
                if self.tunings.remove(shot).is_none() {
                    return Ok(());
                }
            }
        }
        self.generation += 1;
        Ok(())
    }

    /// Mod disable / failure (`Some(id)`) or a full mod runtime reset (`None`).
    pub fn clear_owner(&mut self, owner: Option<&str>) {
        if owner.is_none_or(|o| self.forced_by() == Some(o)) {
            self.forced = None;
        }
        let before = self.tunings.len();
        self.tunings
            .retain(|_, (holder, _)| owner.is_some_and(|o| o != holder));
        if self.tunings.len() != before {
            self.generation += 1;
        }
    }

    /// `sdk.snapshot.camera_angle`.
    pub fn snapshot(&self, shot: &str) -> serde_json::Value {
        serde_json::json!({
            "selected": self.selected.key(),
            "active": self.active().key(),
            "owner": self.forced_by(),
            "shot": shot,
            "tuned": self.tunings.iter().map(|(shot, (owner, _))| (shot.clone(), serde_json::Value::from(owner.clone())))
                .collect::<serde_json::Map<_, _>>(),
        })
    }
}

/// Copies the active angle and shot tunings into the camera runtime before simulation.
pub(crate) fn sync_runtime(
    settings: Option<Res<CameraAngleSettings>>,
    runtime: Option<ResMut<super::CameraRuntime>>,
) {
    let (Some(settings), Some(mut runtime)) = (settings, runtime) else {
        return;
    };
    let camera_type = settings.active().graph_type();
    if runtime.camera_type() != camera_type {
        info!("Camera angle: {}", settings.active().label());
        runtime.set_camera_type(camera_type);
    }
    if runtime.tuning_generation() != settings.generation() {
        runtime.set_shot_tunings(settings.generation(), settings.tunings());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> CameraAngleSettings {
        CameraAngleSettings::new(CameraAngle::High, PathBuf::from("unused/camera.json"))
    }

    #[test]
    fn graph_types_match_the_stock_camera_graph() {
        // Default_cameragraph: CameraLow = IsCameraTypeActive type 0, CameraHigh = type 1.
        assert_eq!(CameraAngle::Low.graph_type(), 0);
        assert_eq!(CameraAngle::High.graph_type(), 1);
        assert_eq!(CameraAngle::default(), CameraAngle::High);
    }

    #[test]
    fn saved_setting_round_trips_and_rejects_unknown_values() {
        let dir = std::env::temp_dir().join(format!("skate-camera-angle-{}", std::process::id()));
        let root = dir.join("assets");
        let mut s = CameraAngleSettings::load(&root);
        assert_eq!(
            s.selected,
            CameraAngle::High,
            "a missing file is the default"
        );
        s.selected = CameraAngle::Low;
        s.save().unwrap();
        assert_eq!(CameraAngleSettings::load(&root).selected, CameraAngle::Low);
        std::fs::write(CameraAngleSettings::path(&root), br#"{"angle":"sideways"}"#).unwrap();
        assert_eq!(CameraAngleSettings::load(&root).selected, CameraAngle::High);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_mod_forces_the_angle_until_it_releases_or_is_disabled() {
        let mut s = settings();
        s.force("a", Some(CameraAngle::Low)).unwrap();
        assert_eq!(s.active(), CameraAngle::Low);
        assert_eq!(
            s.selected,
            CameraAngle::High,
            "the player's setting is untouched"
        );
        assert!(s.force("b", Some(CameraAngle::High)).is_err());
        s.clear_owner(Some("b"));
        assert_eq!(s.active(), CameraAngle::Low);
        s.clear_owner(Some("a"));
        assert_eq!(s.active(), CameraAngle::High);
        s.force("b", Some(CameraAngle::Low)).unwrap();
        s.force("b", None).unwrap();
        assert_eq!(s.forced_by(), None);
    }

    #[test]
    fn shot_tunings_belong_to_one_mod_and_bump_the_generation() {
        let mut s = settings();
        let g = s.generation();
        let tuning = CameraShotTuning {
            position_distance: Some(1.2),
            ..Default::default()
        };
        s.tune("a", "chase_flat_slow", Some(tuning.clone()))
            .unwrap();
        assert!(s.generation() > g);
        assert!(
            s.tune("b", "chase_flat_slow", Some(tuning.clone()))
                .is_err()
        );
        s.tune("b", "high_chase", Some(tuning)).unwrap();
        let g = s.generation();
        s.tune("a", "unknown_but_untuned", None).unwrap();
        assert_eq!(s.generation(), g, "releasing nothing changes nothing");
        s.clear_owner(Some("a"));
        assert_eq!(
            s.tunings().map(|(shot, _)| shot).collect::<Vec<_>>(),
            ["high_chase"]
        );
        s.clear_owner(None);
        assert_eq!(s.tunings().count(), 0);
        let snap = s.snapshot("bl_chase");
        assert_eq!(snap["selected"], "high");
        assert_eq!(snap["shot"], "bl_chase");
    }
}
