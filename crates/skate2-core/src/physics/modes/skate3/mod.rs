//! Skate 3 controllers and calculations ported into Skate 2's physical player.
//! No standalone game, Bevy App, window, renderer, updater or device polling.
#![allow(dead_code)]
pub(crate) mod math_host {
    pub(crate) use bevy_math::*;
    pub(crate) use bevy_transform::components::Transform;
    pub(crate) use bevy_log::{info, warn, error, info_span};
    pub(crate) fn default<T: Default>() -> T { T::default() }
}
pub(crate) mod physics;
pub(crate) mod animation_pose;
pub(crate) mod skater_animation;
pub(crate) mod graph_host;
pub(crate) mod graph_runtime;
pub(crate) mod camera;
pub(crate) mod input;
pub(crate) mod grind_world;
pub(crate) mod skate_world;
pub(crate) mod scoring_runtime;
pub(crate) mod difficulty;
pub(crate) mod custom_difficulty;
pub(crate) mod crash_context;
pub(crate) mod modding;
pub(crate) mod animation;

pub(crate) mod world_types;
pub(crate) use world_types::{BodyOutput, BodyReaction, FrameOutput, SkeletonPartOutput, HostGrindAsset, HostGrindSegment, RawController, Scene, SceneRail, SceneTriangle, SceneTriangleFeatures};
pub(crate) use skate3_core::physics::board_world::{ExternalLineHit, ExternalQueries};
pub(crate) use skate3_core::player::offboard::grab_scene::{AssemblyData as GrabAssembly, Geometry as GrabGeometry, Object as GrabObject, Provider as GrabProvider, Spline as GrabSpline};
pub(crate) use skate_dynamics::{SolidBody, SolidCollider, rapier3d};
pub(crate) fn embedded_asset_disk_reads() -> usize { skate_data::asset_io::disk_read_count() }
