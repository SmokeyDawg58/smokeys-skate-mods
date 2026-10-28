use super::*;
use crate::{input_cvar_runtime::InputCvarManager, world_stream_adapter::LoadedStreamPublisher};
#[derive(Default)]
struct Effects {
    high: usize,
    finalized: usize,
}
impl WorldStreamEffects for Effects {
    fn add_high_collection(&mut self, _: u64, _: u32, _: u32) {
        self.high += 1;
    }
    fn finalize_world_bounds(&mut self) {
        self.finalized += 1;
    }
}
#[test]
fn live_setting_is_sampled_after_runtime_creation_and_relinked_without_cache() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut runtime = WorldStreamRuntime::new(
        WorldStreamCatalog::new(),
        LiveTextureGate(refs.clone()),
        Effects::default(),
    );
    assert_eq!(
        runtime.texture_stream_enabled(),
        Err(WorldStreamRuntimeError::TextureGate(
            TextureStreamGateError::Unlinked("kEnableTextureStream")
        ))
    );
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    assert!(runtime.texture_stream_enabled().unwrap());
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkEnableTextureStream=0\r\n"))
        .unwrap();
    assert!(!runtime.texture_stream_enabled().unwrap());
    InputCvarManager::new().try_link(&refs).unwrap();
    assert!(runtime.texture_stream_enabled().unwrap());
    assert!(matches!(
        runtime.world_bounds().read(),
        Err(WorldBoundsError::Pending)
    ));
}
#[test]
fn exported_gate_error_preserves_actual_error_identity() {
    let refs = InputCvarReferences::new_static_initialized();
    let direct = refs.try_enable_texture_stream().unwrap_err();
    let mut gate = LiveTextureGate(refs);
    let via_public_trait: TextureStreamGateError = gate.texture_stream_enabled().unwrap_err();
    assert_eq!(direct, via_public_trait);
}
