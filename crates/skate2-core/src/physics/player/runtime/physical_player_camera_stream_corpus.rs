//! Original camera stream driver; observations and polling policy stay test-only.
use crate::{
    game::{WorldStreamPoll, WorldStreamSpec},
    input_cvar_runtime::InputCvarReferences,
    selected_world_stream::SelectedWorldStream,
    world_stream_catalog::WorldStreamCatalog,
    world_stream_runtime::WorldStreamEffects,
};
use skate2_formats::mount::MountSet;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
#[derive(Default)]
pub(super) struct StreamObservations {
    high_collections: Vec<(u64, u32, u32)>,
    finalized: usize,
}
impl WorldStreamEffects for StreamObservations {
    fn add_high_collection(&mut self, id: u64, x: u32, z: u32) {
        self.high_collections.push((id, x, z));
    }
    fn finalize_world_bounds(&mut self) {
        self.finalized += 1;
    }
}
// Explicit observation seam, not a claim of recovered proxy-world effects.
pub(super) type RetainedCameraStream = SelectedWorldStream<StreamObservations>;
pub(super) fn load(mounts: Arc<MountSet>, cvars: &InputCvarReferences) -> RetainedCameraStream {
    let mut stream = SelectedWorldStream::try_new(
        mounts,
        WorldStreamCatalog::new(),
        cvars.clone(),
        WorldStreamSpec::derive("BAM", None).unwrap(),
        StreamObservations::default(),
    )
    .unwrap();
    assert!(matches!(
        stream.bounds.read(),
        Err(crate::world_stream_runtime::WorldBoundsError::Pending)
    ));
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        assert!(
            Instant::now() < deadline,
            "actual BAM stream camera prerequisite timed out"
        );
        let state = stream.state().unwrap();
        match stream.try_poll().unwrap() {
            WorldStreamPoll::Pending => std::thread::yield_now(),
            WorldStreamPoll::CompletedThisPoll => {
                assert_eq!(state, 7, "bounds must publish at the actual final state");
                break;
            }
        }
    }
    let result = stream.bounds.read().unwrap();
    assert!(
        result.min[..3]
            .iter()
            .chain(&result.max[..3])
            .all(|bits| f32::from_bits(*bits).is_finite())
    );
    eprintln!("camera retained actual finalized BAM bounds: {result:?}");
    stream
}
