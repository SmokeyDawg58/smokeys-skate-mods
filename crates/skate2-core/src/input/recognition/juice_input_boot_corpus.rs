//! Explicit original-config/disabled-override fixture. No network attempt occurs.
use super::*;
use crate::{
    input_pad::device::PadRemap,
    juice_trick_input::boot::{
        JuiceBootPlatform, JuiceConnection, JuiceConsoleName, JuiceEndpoint, JuiceHostOption,
        JuiceInputBoot, JuiceRemapHandle,
    },
};
#[derive(Clone, Copy, Debug)]
pub(crate) enum QueuedRemapPolicy {
    ExistingFixture,
    NativeDisabledJuice,
    ScriptedKickflip,
    ScriptedY,
}
#[derive(Clone)]
pub(super) enum CorpusRemapHandle {
    Fixture(u32),
    Juice(JuiceRemapHandle),
}
pub(super) enum CorpusRemap {
    Fixture(Remap),
    Juice(JuiceInputBoot<()>),
}
impl PadRemap<CorpusRemapHandle> for CorpusRemap {
    type Error = String;
    fn update(
        &mut self,
        handle: &CorpusRemapHandle,
        player: u32,
        values: &mut [f32; 24],
    ) -> Result<(), Self::Error> {
        match (self, handle) {
            (Self::Fixture(remap), CorpusRemapHandle::Fixture(handle)) => {
                remap.update(handle, player, values).map_err(str::to_owned)
            }
            (Self::Juice(boot), CorpusRemapHandle::Juice(handle)) => boot
                .update(handle, player, values)
                .map_err(|e| e.to_string()),
            _ => Err("different corpus remap mode".into()),
        }
    }
}
struct DisabledFixturePlatform {
    initializations: u32,
}
impl JuiceBootPlatform for DisabledFixturePlatform {
    type Error = &'static str;
    type Connection = ();
    fn initialize_network(&mut self) -> Result<bool, Self::Error> {
        self.initializations += 1;
        Ok(false)
    }
    fn debug_address(&mut self) -> Result<[u8; 4], Self::Error> {
        Err("disabled fixture must not query endpoint")
    }
    fn console_name(&mut self) -> Result<JuiceConsoleName, Self::Error> {
        Err("disabled fixture must not query console name")
    }
    fn connect(&mut self, _: &JuiceEndpoint) -> Result<JuiceConnection<()>, Self::Error> {
        Err("disabled fixture must not connect")
    }
    fn sleep_milliseconds(&mut self, _: u32) -> Result<(), Self::Error> {
        Err("disabled fixture must not sleep")
    }
}
pub(super) fn supplied_remap(
    players: &SelectedPlayerStartupOwners,
    policy: QueuedRemapPolicy,
) -> (CorpusRemapHandle, CorpusRemap) {
    match policy {
        QueuedRemapPolicy::ExistingFixture | QueuedRemapPolicy::ScriptedKickflip => {
            (CorpusRemapHandle::Fixture(42), CorpusRemap::Fixture(Remap))
        }
        QueuedRemapPolicy::NativeDisabledJuice | QueuedRemapPolicy::ScriptedY => {
            let root = std::path::PathBuf::from(
                std::env::var_os("SKATE2_GAME_ROOT").expect("original data root"),
            );
            let bytes = std::fs::read(root.join("data").join("juice").join("Juice_m.ini")).unwrap();
            let boot = JuiceInputBoot::new_static_initialized(players.juice.clone());
            // Explicit test platform result, never a production initialization claim.
            let mut platform = DisabledFixturePlatform { initializations: 0 };
            boot.try_construct(
                Some(&bytes),
                JuiceHostOption::DisableBeforeStart,
                &mut platform,
            )
            .unwrap();
            assert!(
                boot.source_enabled().unwrap(),
                "original corpus enables Juice; override must remain explicit"
            );
            assert_eq!(platform.initializations, 1);
            boot.try_start(&players.cvars, &mut platform).unwrap();
            (
                CorpusRemapHandle::Juice(boot.try_remap_handle().unwrap()),
                CorpusRemap::Juice(boot),
            )
        }
    }
}
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT; explicit Juice enable0 override and controlled hardware"]
fn original_native_disabled_juice_remap_reaches_same_queued_player_tick() {
    with_original_inputs(ground_startup::check_queued_juice_ticks);
}
