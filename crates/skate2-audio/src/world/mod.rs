//! The world state managers (`eMAINMAPSTATES` 2 and 4-12). `AudioSystem::Update` 0x82507b88
//! runs every manager's `UpdateParams` in state order, `ProcessMixMap`, then every
//! `ProcessUpdate` in state order.

pub mod environment;
pub mod traffic;
pub mod traffic_engine;
pub mod traffic_horn;
pub mod traffic_skids;
pub mod pedestrian;
pub mod emitter;
pub mod emitter_assets;
pub mod emitter_system;
pub mod crowd;
pub mod dynamic_object;
pub mod speaker;
pub mod whoosh;
pub mod object_instance;
pub mod nis_character;
