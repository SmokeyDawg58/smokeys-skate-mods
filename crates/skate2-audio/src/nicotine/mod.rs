//! Nicotine, the dynamic mixer (`Nicotine::IDynamicMixer`, `NFSMixMaster`, `NFSMixMap`,
//! `NFSMixShape`; 0x829c9f10-0x829d2a28). `AudioSystem::InitializeMixer` 0x824ed830 creates it with
//! 13 mixer states and loads `data\audio\MixMapSK8.mxb`; `AudioSystem::Update` 0x82507b88 runs
//! `ProcessMixMap` in every second update group with the accumulated frame time.
//!
//! `SnapshotMixer` is never initialised in this game (nothing loads a snapshot header), so its
//! `Update` and `AttachToMixMap` return at once and it is not ported.

pub mod map;
pub mod shape;
mod tables;

pub use map::{DMixIoId, MixMap, MixerHost, PresetType};

/// `Nicotine::IDynamicMixer`.
pub struct DynamicMixer {
    /// `miCamState`: -1 from the constructor 0x829c9f10, and nothing in the game changes it.
    pub cam_state: i32,
    map: Option<MixMap>,
}

impl Default for DynamicMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicMixer {
    pub fn new() -> Self {
        Self { cam_state: -1, map: None }
    }

    /// `CreateMixer` 0x829ca168 with `num_states` (`MAP_CREATE_PARAMS::NumMixStates`), then
    /// `InitMap` 0x829c9f48.
    pub fn init_map(&mut self, data: Vec<u8>, num_states: usize, host: &mut dyn MixerHost) {
        self.map = Some(MixMap::new(data, num_states, host));
    }

    /// `ProcessMixMap` 0x829c9fb0: `NFSMixMaster::ProcessMixMap` only runs once the map is ready.
    pub fn process_mix_map(&mut self, dt: f32) {
        if let Some(map) = &mut self.map {
            map.process(dt, self.cam_state);
        }
    }

    pub fn map(&self) -> Option<&MixMap> {
        self.map.as_ref()
    }

    pub fn map_mut(&mut self) -> Option<&mut MixMap> {
        self.map.as_mut()
    }
}
