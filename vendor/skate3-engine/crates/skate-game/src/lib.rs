//! Embedded access to the captured local Skate 3 simulation.
//!
//! Loading this library does not initialize an application, window, or device.
#![allow(dead_code)]
include!("main.rs");

pub use physics::embedded::{
    BodyOutput, BodyReaction, EmbeddedRuntime, FrameOutput, HostGrindAsset, HostGrindSegment,
    RawController, Scene, SceneRail, SceneTriangle, SceneTriangleFeatures, SkeletonPartOutput,
};
pub use skate_core::physics::board_world::{ExternalLineHit, ExternalQueries};
pub use skate_core::player::offboard::grab_scene::{
    AssemblyData as GrabAssembly, Geometry as GrabGeometry, Object as GrabObject,
    Provider as GrabProvider, Spline as GrabSpline,
};
pub use skate_dynamics::{SolidBody, SolidCollider, rapier3d};

/// Diagnostic counter for verifying that an embedded host never reads external
/// Skate 3 asset files. Host Skate 2 ROM reads are independent of this reader.
pub fn embedded_asset_disk_reads() -> usize {
    skate_data::asset_io::disk_read_count()
}
