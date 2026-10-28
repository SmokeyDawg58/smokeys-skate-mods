//! Bounded selected original-asset boot inputs.
//!
//! Reuses one caller-owned mount set, the existing immutable selected Vault
//! loader, the live cvar manager, and the authored location parser. This is
//! not native asset registration, source `SpawnManager` selection, or session boot.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "awaits selected session caller")
)]
use crate::{
    attribute_runtime::{AttributeRuntimeVault, mounted::MountedVaultError},
    input_cvar_runtime::{InputCvarError, InputCvarManager, InputCvarReferences},
    player::selected_spawn_location,
    selected_original_asset_classes::selected_classes,
};
use skate2_formats::{
    location::{LocationError, LocationTable},
    mount::{MountError, MountSet},
};
use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

const VAULT_PATH: &str = "data\\db\\skater.vlt";
const LOCATION_PATH: &str = "data\\content\\world\\stream\\BAM\\BAM_Sim.loc";

#[path = "selected_map_catalog.rs"]
pub(crate) mod map_catalog;

#[derive(Debug)]
pub(crate) enum SelectedOriginalAssetsError {
    Vault(MountedVaultError),
    StartupRead { path: PathBuf, cause: io::Error },
    Cvars(InputCvarError),
    LocationRead(MountError),
    LocationParse(LocationError),
    MissingSpawn(&'static str),
    MapCatalog(String),
}
impl std::fmt::Display for SelectedOriginalAssetsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MapCatalog(e) => write!(f, "selected Map catalog: {e}"),
            Self::Vault(e) => write!(f, "selected Vault: {e}"),
            Self::StartupRead { path, cause } => {
                write!(f, "selected startup {}: {cause}", path.display())
            }
            Self::Cvars(e) => write!(f, "selected startup cvars: {e:?}"),
            Self::LocationRead(e) => write!(f, "selected location read: {e}"),
            Self::LocationParse(e) => write!(f, "selected location parse: {e}"),
            Self::MissingSpawn(name) => write!(f, "selected location {name} absent"),
        }
    }
}
impl std::error::Error for SelectedOriginalAssetsError {}

/// A single retained source graph for a host-selected first-test location.
/// Caller supplies one matching `MountSet` and external root. `MountSet` does not
/// expose a canonical root, so this constructor cannot prove they correspond.
pub(crate) struct SelectedOriginalAssets {
    pub(crate) mounts: Arc<MountSet>,
    pub(crate) data_root: PathBuf,
    pub(crate) vault: Arc<AttributeRuntimeVault>,
    // Keep target ownership alive beside clones handed to selected services.
    #[cfg_attr(
        test,
        expect(
            dead_code,
            reason = "retains linked cvar target storage for session lifetime"
        )
    )]
    pub(crate) cvar_manager: InputCvarManager,
    pub(crate) cvars: InputCvarReferences,
    pub(crate) spawn: [f32; 16],
    pub(crate) map_catalog: map_catalog::MapCatalog,
}
impl SelectedOriginalAssets {
    /// Each stage fails without pretending later startup stages ran. The
    /// authored location is a host policy for the first test, not evidence of
    /// the original `SpawnManager`'s placement choice.
    pub(crate) fn try_new(
        mounts: Arc<MountSet>,
        data_root: &Path,
    ) -> Result<Self, SelectedOriginalAssetsError> {
        use SelectedOriginalAssetsError as E;
        let vault =
            AttributeRuntimeVault::try_load_mounted(&mounts, VAULT_PATH, &selected_classes())
                .map_err(E::Vault)?;
        let cvars = InputCvarReferences::new_static_initialized();
        let mut cvar_manager = InputCvarManager::new();
        cvar_manager.try_link(&cvars).map_err(E::Cvars)?;
        // A retail game folder has no startup.ini: the cvars keep their defaults, as the
        // FileLoader's failed load leaves them.
        let startup_path = data_root.join("data").join("startup.ini");
        let startup = skate2_formats::host_path::read_if_present(&startup_path)
            .map_err(|cause| E::StartupRead { path: startup_path, cause })?;
        cvar_manager
            .try_deserialize_startup(startup.as_deref())
            .map_err(E::Cvars)?;
        let locations =
            LocationTable::parse(&mounts.read_path(LOCATION_PATH).map_err(E::LocationRead)?)
                .map_err(E::LocationParse)?;
        let spawn = locations
            .find(selected_spawn_location())
            .ok_or(E::MissingSpawn(selected_spawn_location()))?
            .transform;
        let map_catalog = map_catalog::MapCatalog::load(&vault, &locations)
            .map_err(E::MapCatalog)?;
        Ok(Self {
            mounts,
            data_root: data_root.to_owned(),
            vault,
            cvar_manager,
            cvars,
            spawn,
            map_catalog,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn mounted_selected_bootstrap_retains_one_mount_and_source_startup() {
        let root = std::path::PathBuf::from(
            std::env::var_os("SKATE2_GAME_ROOT").expect("original game root"),
        );
        let mounts = Arc::new(
            crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_corpus(),
        );
        let assets = SelectedOriginalAssets::try_new(Arc::clone(&mounts), &root).unwrap();
        assert!(Arc::ptr_eq(&assets.mounts, &mounts));
        assert_eq!(assets.data_root, root);
        assert_eq!(
            assets
                .cvars
                .try_camera_near_clip_distance()
                .unwrap()
                .to_bits(),
            0.1_f32.to_bits()
        );
        assert!(assets.spawn.iter().all(|value| value.is_finite()));
        // A filtered-out render_locations class previously published nine
        // zero vectors to every vehicle despite authored paint variation.
        let probe =
            crate::selected_living_world_vehicle_visual::vehicle_default_irradiance(&assets.vault)
                .unwrap();
        assert!(probe.iter().flatten().any(|value| *value != 0.0));
        assert!(probe.iter().all(|coefficient| coefficient[3] == 0.0));
        let offboard = assets
            .vault
            .instance(0x4920_26e0_f096_f283, 0xd7ed_bd36_2d7d_2152);
        let turn = offboard
            .try_get_data(0x9bf4_8a85_5197_5cef, 0)
            .unwrap()
            .expect("authored stationary offboard turn rate (82da042c high-half insert)");
        let turn_rate = f32::from_be_bytes(turn.bytes()[..4].try_into().unwrap());
        assert!(
            turn_rate > 0.0 && turn_rate.is_finite(),
            "turn rate {turn_rate}"
        );
        assert!(
            offboard
                .try_get_data(0x5197_5cef_9bf4_8a85, 0)
                .unwrap()
                .is_none(),
            "the formerly reversed key must not stand in for authored turn data"
        );
        eprintln!("authored offboard turn rate: {turn_rate}");
        let physics = assets.vault.physics(0xd7ed_bd36_2d7d_2152);
        let layout = physics.try_physics_layout().unwrap();
        assert_eq!(layout.raw_bytes.len(), 32);
        // A missing physics_trajectory class previously silently bound null:
        // every live ollie queried zero seconds and stayed in Air200.
        let trajectory =
            crate::skateboard_foot_attributes::wallride::WallRideTrajectoryAttributes::bind(
                &assets.vault,
            );
        let data = trajectory
            .capture()
            .unwrap()
            .expect("authored trajectory layout");
        let scalar = |offset: usize| {
            f32::from_be_bytes(data.bytes()[offset..offset + 4].try_into().unwrap())
        };
        assert!(scalar(0x1ac) > 0.0, "positive trajectory lookahead cap");
        assert!(scalar(0x1b0) > 0.0, "positive starting sweep error");
        assert!(scalar(0x1b4) > 0.0, "positive ending sweep error");
        // SlideGround101 also requires its actual class in this narrowed Vault.
        // A null binding would silently replace orientation/force tuning with0.
        let slide = assets.vault.instance(
            crate::skateboard_attributes::SkateboardAttributeClass::Slide.key(),
            crate::skateboard_attributes::SkateboardAttributes::DEFAULT_COLLECTION,
        );
        for key in [0x4515_ae74_0928_9c50, 0x27db_0582_0432_b632] {
            let value = slide
                .try_get_data(key, 0)
                .unwrap()
                .expect("authored SlideGround attribute");
            assert_eq!(value.bytes().len(), 4);
            assert!(f32::from_be_bytes(value.bytes().try_into().unwrap()).is_finite());
        }
    }
}
