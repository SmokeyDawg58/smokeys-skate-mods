//! Owner-bound runtime routing for ID-dispatched Simulation kinds.
//!
//! This adapter is deliberately a subsequence, not a complete
//! `Simulation::RegArena` (`0x828a01e0`) implementation. It preserves the
//! relative order of kinds 22, 23, 25, 27, 28, and 32 while skipping static
//! and pointer-backed operations. A unified adapter must eventually preserve
//! their global interleaving.

pub use crate::living_world_mesh_store::MeshStoreError;
pub use crate::living_world_rain::network::RainNetworkError;
pub use crate::world_painter_layer::WorldPainterLayerError;

use crate::simulation_normal_arena_plan::{
    SimulationNormalArenaPlan, SimulationNormalOperation, SimulationRegistryAssetKind,
};
use std::{error::Error, fmt, sync::Arc, sync::Weak};

/// Exact void-returning manager calls reached by ID-dispatched normal assets.
///
/// `Simulation::RegArena` and `DRegArena` forward the complete 64-bit asset ID.
/// Waypoint registration additionally forwards literal `true`, while
/// deregistration forwards literal `false`.
pub trait SimulationRegistryArenaDispatch {
    fn register_streamed_tile(&mut self, asset_id: u64);
    fn unregister_streamed_tile(&mut self, asset_id: u64);
    /// # Errors
    /// Preserves a retained `LayerMan`'s explicit source/host boundary.
    fn try_register_streamed_tile(&mut self, asset_id: u64) -> Result<(), WorldPainterLayerError> {
        self.register_streamed_tile(asset_id);
        Ok(())
    }
    /// # Errors
    /// Preserves a retained `LayerMan`'s explicit source/host boundary.
    fn try_unregister_streamed_tile(
        &mut self,
        asset_id: u64,
    ) -> Result<(), WorldPainterLayerError> {
        self.unregister_streamed_tile(asset_id);
        Ok(())
    }
    fn add_nav_mesh_data(&mut self, asset_id: u64);
    fn remove_nav_mesh_data(&mut self, asset_id: u64);
    /// Typed continuation boundary for an actual retained mesh owner.
    /// Existing fully supplied void dispatchers retain their behavior.
    /// # Errors
    /// Returns the mesh owner's unreconstructed native operation boundary.
    fn try_add_nav_mesh_data(&mut self, asset_id: u64) -> Result<(), MeshStoreError> {
        self.add_nav_mesh_data(asset_id);
        Ok(())
    }
    /// # Errors
    /// Returns the mesh owner's unreconstructed native removal boundary.
    fn try_remove_nav_mesh_data(&mut self, asset_id: u64) -> Result<(), MeshStoreError> {
        self.remove_nav_mesh_data(asset_id);
        Ok(())
    }
    fn add_rain_data(&mut self, asset_id: u64);
    fn remove_rain_data(&mut self, asset_id: u64);
    /// Whether the selected host profile supplies its real RAIN owner.
    fn selected_rain_enabled(&self) -> bool {
        false
    }
    fn try_add_rain_data(&mut self, asset_id: u64) -> Result<(), RainNetworkError> {
        self.add_rain_data(asset_id);
        Ok(())
    }
    fn try_remove_rain_data(&mut self, asset_id: u64) -> Result<(), RainNetworkError> {
        self.remove_rain_data(asset_id);
        Ok(())
    }
    fn add_ai_path_asset(&mut self, asset_id: u64);
    fn remove_ai_path_asset(&mut self, asset_id: u64);
    fn register_waypoint_data(&mut self, asset_id: u64, streamed: bool);
    fn deregister_waypoint_data(&mut self, asset_id: u64, streamed: bool);
    fn add_spatial_map_asset(&mut self, asset_id: u64);
    fn remove_spatial_map_asset(&mut self, asset_id: u64);
}

/// One prevalidated ID-only operation in authored normal-plan order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreparedSimulationRegistryOperation {
    kind: SimulationRegistryAssetKind,
    asset_id: u64,
}

impl PreparedSimulationRegistryOperation {
    #[must_use]
    pub const fn kind(self) -> SimulationRegistryAssetKind {
        self.kind
    }

    #[must_use]
    pub const fn asset_id(self) -> u64 {
        self.asset_id
    }
}

#[derive(Debug)]
struct PreparedSimulationRegistryArenaInner {
    plan: Arc<SimulationNormalArenaPlan>,
    operations: Vec<PreparedSimulationRegistryOperation>,
}

/// Prevalidated ID-dispatched subsequence for one owned normal arena plan.
pub struct PreparedSimulationRegistryArena {
    inner: Arc<PreparedSimulationRegistryArenaInner>,
}

impl fmt::Debug for PreparedSimulationRegistryArena {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedSimulationRegistryArena")
            .field(
                "asset_id",
                &self.inner.plan.static_plan().asset().decoded.asset_id,
            )
            .field("operation_count", &self.inner.operations.len())
            .finish()
    }
}

impl PreparedSimulationRegistryArena {
    /// Retains the plan and extracts only kinds 22, 23, 25, 27, 28, and 32.
    ///
    /// # Errors
    ///
    /// Returns a typed host allocation error before runtime effects exist.
    pub fn try_new(
        plan: Arc<SimulationNormalArenaPlan>,
    ) -> Result<Self, SimulationRegistryArenaPrepareError> {
        let requested = plan
            .operations()
            .iter()
            .filter(|operation| {
                matches!(operation, SimulationNormalOperation::RegistryAsset { .. })
            })
            .count();
        let mut operations = Vec::new();
        operations.try_reserve_exact(requested).map_err(|_| {
            SimulationRegistryArenaPrepareError::OperationStorageAllocation { requested }
        })?;
        for operation in plan.operations() {
            if let SimulationNormalOperation::RegistryAsset { kind, asset_id, .. } = *operation {
                operations.push(PreparedSimulationRegistryOperation { kind, asset_id });
            }
        }
        Ok(Self {
            inner: Arc::new(PreparedSimulationRegistryArenaInner { plan, operations }),
        })
    }

    #[must_use]
    pub fn plan(&self) -> &SimulationNormalArenaPlan {
        &self.inner.plan
    }

    #[must_use]
    pub fn operations(&self) -> &[PreparedSimulationRegistryOperation] {
        &self.inner.operations
    }

    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.inner.operations.len()
    }
}

#[derive(Debug)]
struct LiveRegistryRegistration {
    id: u64,
    prepared: Arc<PreparedSimulationRegistryArenaInner>,
}

/// Routes ID-only operations and retains each processed arena occurrence.
///
/// `dispatch` is declared before the source-holding ledger so Rust destroys
/// manager state before any still-referenced arena bytes. No implicit manager
/// call is made from `Drop`.
pub struct SimulationRegistryArenaRegistrationAdapter<D> {
    dispatch: D,
    live: Vec<LiveRegistryRegistration>,
    domain: Arc<()>,
    next_registration_id: u64,
}

impl<D: fmt::Debug> fmt::Debug for SimulationRegistryArenaRegistrationAdapter<D> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SimulationRegistryArenaRegistrationAdapter")
            .field("dispatch", &self.dispatch)
            .field("live_registration_count", &self.live.len())
            .field("next_registration_id", &self.next_registration_id)
            .finish_non_exhaustive()
    }
}

impl<D: SimulationRegistryArenaDispatch> SimulationRegistryArenaRegistrationAdapter<D> {
    #[must_use]
    pub fn new(dispatch: D) -> Self {
        Self {
            dispatch,
            live: Vec::new(),
            domain: Arc::new(()),
            next_registration_id: 0,
        }
    }

    /// Walks the ID-dispatched subsequence forward.
    ///
    /// The live ledger and one-shot token identity are installed before the
    /// first manager call. Native manager calls return `void`, so this adapter
    /// does not invent a recoverable effect error or rollback path.
    ///
    /// # Errors
    ///
    /// Host ledger allocation and registration-ID exhaustion are rejected
    /// before effects.
    pub fn register(
        &mut self,
        prepared: &PreparedSimulationRegistryArena,
    ) -> Result<SimulationRegistryArenaRegistration, SimulationRegistryArenaRegistrationError> {
        self.live.try_reserve(1).map_err(|_| {
            SimulationRegistryArenaRegistrationError::LiveStorageAllocation {
                requested: self.live.len().saturating_add(1),
            }
        })?;
        let registration_id = self.next_registration_id;
        self.next_registration_id = registration_id
            .checked_add(1)
            .ok_or(SimulationRegistryArenaRegistrationError::RegistrationIdExhausted)?;
        let registration = SimulationRegistryArenaRegistration {
            domain: Arc::downgrade(&self.domain),
            id: registration_id,
        };
        self.live.push(LiveRegistryRegistration {
            id: registration_id,
            prepared: Arc::clone(&prepared.inner),
        });
        dispatch_registration(&mut self.dispatch, &prepared.inner);
        Ok(registration)
    }

    /// Consumes one same-domain occurrence and walks its ID-only subsequence
    /// forward in the native deregistration direction.
    ///
    /// # Errors
    ///
    /// Foreign and unknown tokens are returned without effects.
    pub fn deregister(
        &mut self,
        registration: SimulationRegistryArenaRegistration,
    ) -> Result<(), SimulationRegistryArenaDeregistrationError> {
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            return Err(SimulationRegistryArenaDeregistrationError {
                kind: SimulationRegistryArenaDeregistrationErrorKind::ForeignDomain,
                registration,
            });
        }
        let Some(live_index) = self.live.iter().position(|live| live.id == registration.id) else {
            return Err(SimulationRegistryArenaDeregistrationError {
                kind: SimulationRegistryArenaDeregistrationErrorKind::UnknownRegistration,
                registration,
            });
        };
        let prepared = Arc::clone(&self.live[live_index].prepared);
        dispatch_deregistration(&mut self.dispatch, &prepared);
        self.live.swap_remove(live_index);
        Ok(())
    }

    #[must_use]
    pub fn live_registration_count(&self) -> usize {
        self.live.len()
    }

    #[must_use]
    pub const fn dispatch(&self) -> &D {
        &self.dispatch
    }
}

fn dispatch_registration<D: SimulationRegistryArenaDispatch>(
    dispatch: &mut D,
    prepared: &PreparedSimulationRegistryArenaInner,
) {
    for operation in &prepared.operations {
        match operation.kind {
            SimulationRegistryAssetKind::WorldPainter => {
                dispatch.register_streamed_tile(operation.asset_id);
            }
            SimulationRegistryAssetKind::NavMesh => {
                dispatch.add_nav_mesh_data(operation.asset_id);
            }
            SimulationRegistryAssetKind::Rain => dispatch.add_rain_data(operation.asset_id),
            SimulationRegistryAssetKind::AiPath => dispatch.add_ai_path_asset(operation.asset_id),
            SimulationRegistryAssetKind::Waypoint => {
                dispatch.register_waypoint_data(operation.asset_id, true);
            }
            SimulationRegistryAssetKind::SpatialMap => {
                dispatch.add_spatial_map_asset(operation.asset_id);
            }
        }
    }
}

fn dispatch_deregistration<D: SimulationRegistryArenaDispatch>(
    dispatch: &mut D,
    prepared: &PreparedSimulationRegistryArenaInner,
) {
    for operation in &prepared.operations {
        match operation.kind {
            SimulationRegistryAssetKind::WorldPainter => {
                dispatch.unregister_streamed_tile(operation.asset_id);
            }
            SimulationRegistryAssetKind::NavMesh => {
                dispatch.remove_nav_mesh_data(operation.asset_id);
            }
            SimulationRegistryAssetKind::Rain => dispatch.remove_rain_data(operation.asset_id),
            SimulationRegistryAssetKind::AiPath => {
                dispatch.remove_ai_path_asset(operation.asset_id);
            }
            SimulationRegistryAssetKind::Waypoint => {
                dispatch.deregister_waypoint_data(operation.asset_id, false);
            }
            SimulationRegistryAssetKind::SpatialMap => {
                dispatch.remove_spatial_map_asset(operation.asset_id);
            }
        }
    }
}

/// Non-cloneable authority for one live ID-only arena occurrence.
#[derive(Debug)]
pub struct SimulationRegistryArenaRegistration {
    domain: Weak<()>,
    id: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationRegistryArenaPrepareError {
    OperationStorageAllocation { requested: usize },
}

impl fmt::Display for SimulationRegistryArenaPrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationStorageAllocation { requested } => write!(
                formatter,
                "could not allocate {requested} ID-dispatched Simulation operations"
            ),
        }
    }
}

impl Error for SimulationRegistryArenaPrepareError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationRegistryArenaRegistrationError {
    LiveStorageAllocation { requested: usize },
    RegistrationIdExhausted,
}

impl fmt::Display for SimulationRegistryArenaRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LiveStorageAllocation { requested } => write!(
                formatter,
                "could not allocate ID-dispatched Simulation registration {requested}"
            ),
            Self::RegistrationIdExhausted => {
                formatter.write_str("ID-dispatched Simulation registration IDs are exhausted")
            }
        }
    }
}

impl Error for SimulationRegistryArenaRegistrationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationRegistryArenaDeregistrationErrorKind {
    ForeignDomain,
    UnknownRegistration,
}

#[derive(Debug)]
pub struct SimulationRegistryArenaDeregistrationError {
    kind: SimulationRegistryArenaDeregistrationErrorKind,
    registration: SimulationRegistryArenaRegistration,
}

impl SimulationRegistryArenaDeregistrationError {
    #[must_use]
    pub const fn kind(&self) -> SimulationRegistryArenaDeregistrationErrorKind {
        self.kind
    }

    #[must_use]
    pub fn into_registration(self) -> SimulationRegistryArenaRegistration {
        self.registration
    }
}

impl fmt::Display for SimulationRegistryArenaDeregistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            SimulationRegistryArenaDeregistrationErrorKind::ForeignDomain => {
                "registration belongs to a different ID-dispatched Simulation adapter domain"
            }
            SimulationRegistryArenaDeregistrationErrorKind::UnknownRegistration => {
                "registration is not live in this ID-dispatched Simulation adapter"
            }
        })
    }
}

impl Error for SimulationRegistryArenaDeregistrationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_normal_arena_plan::{
            AI_PATH_DATA_TYPE_ID, NAV_MESH_2_DATA_TYPE_ID, RAIN_DATA_TYPE_ID,
            SPATIAL_MAP_DATA_TYPE_ID, WAYPOINT_DATA_TYPE_ID, WORLD_PAINTER_DATA_TYPE_ID_0F,
            WORLD_PAINTER_DATA_TYPE_ID_10, WORLD_PAINTER_DATA_TYPE_ID_11,
        },
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::CollisionArena,
    };
    use skate2_formats::{
        pegasus::{ObjectId, TABLE_OF_CONTENTS_TYPE_ID},
        rw4::{Arena, DictionaryEntry, FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };
    use std::{
        ops::Range,
        sync::atomic::{AtomicBool, Ordering},
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Event {
        WorldPainter(u64, bool),
        NavMesh(u64, bool),
        Rain(u64, bool),
        AiPath(u64, bool),
        Waypoint(u64, bool, bool),
        SpatialMap(u64, bool),
    }

    #[derive(Debug, Default)]
    struct Recorder {
        events: Vec<Event>,
    }

    impl SimulationRegistryArenaDispatch for Recorder {
        fn register_streamed_tile(&mut self, asset_id: u64) {
            self.events.push(Event::WorldPainter(asset_id, true));
        }

        fn unregister_streamed_tile(&mut self, asset_id: u64) {
            self.events.push(Event::WorldPainter(asset_id, false));
        }

        fn add_nav_mesh_data(&mut self, asset_id: u64) {
            self.events.push(Event::NavMesh(asset_id, true));
        }

        fn remove_nav_mesh_data(&mut self, asset_id: u64) {
            self.events.push(Event::NavMesh(asset_id, false));
        }

        fn add_rain_data(&mut self, asset_id: u64) {
            self.events.push(Event::Rain(asset_id, true));
        }

        fn remove_rain_data(&mut self, asset_id: u64) {
            self.events.push(Event::Rain(asset_id, false));
        }

        fn add_ai_path_asset(&mut self, asset_id: u64) {
            self.events.push(Event::AiPath(asset_id, true));
        }

        fn remove_ai_path_asset(&mut self, asset_id: u64) {
            self.events.push(Event::AiPath(asset_id, false));
        }

        fn register_waypoint_data(&mut self, asset_id: u64, streamed: bool) {
            self.events.push(Event::Waypoint(asset_id, true, streamed));
        }

        fn deregister_waypoint_data(&mut self, asset_id: u64, streamed: bool) {
            self.events.push(Event::Waypoint(asset_id, false, streamed));
        }

        fn add_spatial_map_asset(&mut self, asset_id: u64) {
            self.events.push(Event::SpatialMap(asset_id, true));
        }

        fn remove_spatial_map_asset(&mut self, asset_id: u64) {
            self.events.push(Event::SpatialMap(asset_id, false));
        }
    }

    struct ArenaFixture {
        bytes: Vec<u8>,
        dictionary: Vec<DictionaryEntry>,
    }

    impl ArenaFixture {
        fn new() -> Self {
            Self {
                bytes: Vec::new(),
                dictionary: Vec::new(),
            }
        }

        fn push(&mut self, type_id: u32, object: &[u8]) -> ObjectId {
            let index = self.dictionary.len();
            let start = self.bytes.len();
            self.bytes.extend_from_slice(object);
            let end = self.bytes.len();
            self.dictionary.push(DictionaryEntry {
                resource_index: 0,
                object_offset: u32::try_from(start).unwrap(),
                relocation_offset: 0,
                size: u32::try_from(object.len()).unwrap(),
                alignment: 1,
                type_index: 0,
                type_id,
                object_range: Range { start, end },
            });
            ObjectId(u32::try_from(index).unwrap())
        }

        fn finish(self) -> Arc<SimulationNormalArenaPlan> {
            let descriptor = ResourceDescriptor {
                size: 0,
                alignment: 1,
            };
            let entry_count = u32::try_from(self.dictionary.len()).unwrap();
            let virtual_size = u32::try_from(self.bytes.len()).unwrap();
            let arena = Arena {
                file_header: FileHeader {
                    magic: *b"\x89RW4xb2\0\r\n\x1a\n",
                    is_big_endian: true,
                    pointer_size_bits: 32,
                    pointer_alignment: 4,
                    unused: 0,
                    major_version: *b"454\0",
                    minor_version: *b"000\0",
                    build_number: 0,
                },
                id: 0x1234_5678,
                entry_count,
                used_entry_count: entry_count,
                alignment: 1,
                virtual_size,
                dictionary_offset: 0,
                sections_offset: 0,
                base_offset: 0,
                unfix_context_offset: 0,
                fix_context_offset: 0,
                resource_descriptor: [descriptor; 5],
                resources_used: [descriptor; 5],
                resource_offsets: [0; 5],
                arena_group_offset: 0,
                dictionary: self.dictionary,
            };
            let asset = SimulationCollisionAsset {
                decoded: DecodedArenaAsset::from_parts_for_test(
                    0xabcd_ef01_2345_6789,
                    WORLD_STREAM_ARENA_ASSET_TYPE,
                    ArenaResources {
                        resources: [self.bytes, Vec::new(), Vec::new(), Vec::new(), Vec::new()],
                    },
                    arena,
                ),
                collision: CollisionArena {
                    id: 0x1234_5678,
                    grind_assets: Vec::new(),
                    aggregate_roots: Vec::new(),
                    primitive_sets: Vec::new(),
                    simple_primitive_sets: Vec::new(),
                    meshes: Vec::new(),
                    collision_models: Vec::new(),
                    instances: Vec::new(),
                },
            };
            let static_plan = Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
                asset,
                Vec::new(),
            ));
            Arc::new(SimulationNormalArenaPlan::from_static_plan(static_plan).unwrap())
        }
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn toc(entries: &[(u64, u32, ObjectId)]) -> Vec<u8> {
        let names = 20 + entries.len() * 24;
        let mut bytes = vec![0; names + 1];
        put_u32(&mut bytes, 0, u32::try_from(entries.len()).unwrap());
        put_u32(&mut bytes, 4, 20);
        put_u32(&mut bytes, 8, u32::try_from(names).unwrap());
        put_u32(&mut bytes, 16, u32::try_from(names).unwrap());
        for (index, (guid, object_type, object)) in entries.iter().enumerate() {
            let offset = 20 + index * 24;
            put_u32(&mut bytes, offset, u32::try_from(names).unwrap());
            put_u64(&mut bytes, offset + 8, *guid);
            put_u32(&mut bytes, offset + 16, *object_type);
            put_u32(&mut bytes, offset + 20, object.0);
        }
        bytes
    }

    fn registry_plan() -> Arc<SimulationNormalArenaPlan> {
        let mut fixture = ArenaFixture::new();
        let types = [
            WORLD_PAINTER_DATA_TYPE_ID_0F,
            NAV_MESH_2_DATA_TYPE_ID,
            RAIN_DATA_TYPE_ID,
            AI_PATH_DATA_TYPE_ID,
            WAYPOINT_DATA_TYPE_ID,
            SPATIAL_MAP_DATA_TYPE_ID,
            WORLD_PAINTER_DATA_TYPE_ID_10,
            WORLD_PAINTER_DATA_TYPE_ID_11,
        ];
        let objects = types.map(|kind| fixture.push(kind, &[0]));
        let ids = [
            0x0000_0001_0000_0001,
            0x0000_0002_0000_0002,
            0x0000_0003_0000_0003,
            0x0000_0004_0000_0004,
            0x0000_0005_0000_0005,
            0x0000_0006_0000_0006,
            0xffff_ffff_0000_0001,
            0x0000_0001_0000_0001,
        ];
        let entries =
            std::array::from_fn::<_, 8, _>(|index| (ids[index], types[index], objects[index]));
        fixture.push(TABLE_OF_CONTENTS_TYPE_ID, &toc(&entries));
        fixture.finish()
    }

    fn empty_plan() -> Arc<SimulationNormalArenaPlan> {
        ArenaFixture::new().finish()
    }

    #[test]
    fn id_only_subsequence_is_forward_full_width_and_preserves_duplicates() {
        let prepared = PreparedSimulationRegistryArena::try_new(registry_plan()).unwrap();
        assert_eq!(prepared.operation_count(), 8);
        assert_eq!(
            prepared.operations()[6],
            PreparedSimulationRegistryOperation {
                kind: SimulationRegistryAssetKind::WorldPainter,
                asset_id: 0xffff_ffff_0000_0001,
            }
        );
        let mut adapter = SimulationRegistryArenaRegistrationAdapter::new(Recorder::default());

        let registration = adapter.register(&prepared).unwrap();
        adapter.deregister(registration).unwrap();

        assert_eq!(
            adapter.dispatch().events,
            [
                Event::WorldPainter(0x0000_0001_0000_0001, true),
                Event::NavMesh(0x0000_0002_0000_0002, true),
                Event::Rain(0x0000_0003_0000_0003, true),
                Event::AiPath(0x0000_0004_0000_0004, true),
                Event::Waypoint(0x0000_0005_0000_0005, true, true),
                Event::SpatialMap(0x0000_0006_0000_0006, true),
                Event::WorldPainter(0xffff_ffff_0000_0001, true),
                Event::WorldPainter(0x0000_0001_0000_0001, true),
                Event::WorldPainter(0x0000_0001_0000_0001, false),
                Event::NavMesh(0x0000_0002_0000_0002, false),
                Event::Rain(0x0000_0003_0000_0003, false),
                Event::AiPath(0x0000_0004_0000_0004, false),
                Event::Waypoint(0x0000_0005_0000_0005, false, false),
                Event::SpatialMap(0x0000_0006_0000_0006, false),
                Event::WorldPainter(0xffff_ffff_0000_0001, false),
                Event::WorldPainter(0x0000_0001_0000_0001, false),
            ]
        );
        assert_eq!(adapter.live_registration_count(), 0);
    }

    #[test]
    fn repeated_registrations_are_distinct_and_can_deregister_out_of_order() {
        let prepared = PreparedSimulationRegistryArena::try_new(registry_plan()).unwrap();
        let mut adapter = SimulationRegistryArenaRegistrationAdapter::new(Recorder::default());
        let first = adapter.register(&prepared).unwrap();
        let second = adapter.register(&prepared).unwrap();
        assert_eq!(adapter.live_registration_count(), 2);

        adapter.deregister(second).unwrap();
        adapter.deregister(first).unwrap();

        assert_eq!(adapter.live_registration_count(), 0);
        assert_eq!(adapter.dispatch().events.len(), 32);
    }

    #[test]
    fn foreign_token_is_returned_without_effects() {
        let prepared = PreparedSimulationRegistryArena::try_new(registry_plan()).unwrap();
        let mut first = SimulationRegistryArenaRegistrationAdapter::new(Recorder::default());
        let mut second = SimulationRegistryArenaRegistrationAdapter::new(Recorder::default());
        let registration = first.register(&prepared).unwrap();
        let before = second.dispatch().events.len();

        let error = second.deregister(registration).unwrap_err();
        assert_eq!(
            error.kind(),
            SimulationRegistryArenaDeregistrationErrorKind::ForeignDomain
        );
        assert_eq!(second.dispatch().events.len(), before);
        first.deregister(error.into_registration()).unwrap();
    }

    #[test]
    fn empty_plan_still_has_one_explicit_occurrence_token() {
        let plan = empty_plan();
        let prepared = PreparedSimulationRegistryArena::try_new(Arc::clone(&plan)).unwrap();
        let weak = Arc::downgrade(&plan);
        drop(plan);
        let mut adapter = SimulationRegistryArenaRegistrationAdapter::new(Recorder::default());

        let registration = adapter.register(&prepared).unwrap();
        drop(prepared);
        assert!(weak.upgrade().is_some());
        assert!(adapter.dispatch().events.is_empty());
        adapter.deregister(registration).unwrap();
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn dropped_token_is_inert_and_dispatch_drops_before_retained_source() {
        struct DropObserver {
            plan: Weak<SimulationNormalArenaPlan>,
            source_was_alive: Arc<AtomicBool>,
        }

        impl Drop for DropObserver {
            fn drop(&mut self) {
                self.source_was_alive
                    .store(self.plan.upgrade().is_some(), Ordering::Release);
            }
        }

        impl SimulationRegistryArenaDispatch for DropObserver {
            fn register_streamed_tile(&mut self, _asset_id: u64) {}
            fn unregister_streamed_tile(&mut self, _asset_id: u64) {}
            fn add_nav_mesh_data(&mut self, _asset_id: u64) {}
            fn remove_nav_mesh_data(&mut self, _asset_id: u64) {}
            fn add_rain_data(&mut self, _asset_id: u64) {}
            fn remove_rain_data(&mut self, _asset_id: u64) {}
            fn add_ai_path_asset(&mut self, _asset_id: u64) {}
            fn remove_ai_path_asset(&mut self, _asset_id: u64) {}
            fn register_waypoint_data(&mut self, _asset_id: u64, _streamed: bool) {}
            fn deregister_waypoint_data(&mut self, _asset_id: u64, _streamed: bool) {}
            fn add_spatial_map_asset(&mut self, _asset_id: u64) {}
            fn remove_spatial_map_asset(&mut self, _asset_id: u64) {}
        }

        let plan = registry_plan();
        let weak = Arc::downgrade(&plan);
        let prepared = PreparedSimulationRegistryArena::try_new(Arc::clone(&plan)).unwrap();
        let source_was_alive = Arc::new(AtomicBool::new(false));
        let dispatch = DropObserver {
            plan: weak.clone(),
            source_was_alive: Arc::clone(&source_was_alive),
        };
        let mut adapter = SimulationRegistryArenaRegistrationAdapter::new(dispatch);
        let registration = adapter.register(&prepared).unwrap();
        drop(registration);
        drop(prepared);
        drop(plan);
        assert!(weak.upgrade().is_some());

        drop(adapter);

        assert!(source_was_alive.load(Ordering::Acquire));
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn registration_id_exhaustion_precedes_effects() {
        let prepared = PreparedSimulationRegistryArena::try_new(registry_plan()).unwrap();
        let mut adapter = SimulationRegistryArenaRegistrationAdapter::new(Recorder::default());
        adapter.next_registration_id = u64::MAX;

        assert_eq!(
            adapter.register(&prepared).unwrap_err(),
            SimulationRegistryArenaRegistrationError::RegistrationIdExhausted
        );
        assert!(adapter.dispatch().events.is_empty());
        assert_eq!(adapter.live_registration_count(), 0);
    }
}
