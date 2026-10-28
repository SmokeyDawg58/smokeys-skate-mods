//! Runtime-owned world collision assembled from decoded stream assets.

use crate::grind::{
    GrindAsset, GrindBounds, GrindGeometryCollision, GrindGeometryProbeBatch,
    GrindPairFilterTuning, GrindPrimitive, LivingWorldGrindScene, grind_box_test,
    grind_box_test_with_dynamic, grind_box_test_with_living_world, trajectory_grind_box_test,
    trajectory_grind_box_test_with_dynamic, trajectory_grind_box_test_with_living_world,
};
use crate::narrow_phase::{GpPrimitive, generic_overlap_proven_subset};
use crate::physics::{TrajectoryCollisionWorld, TrajectoryGroundTriangle, TrajectorySweepHit};
use crate::streaming::StreamFocusState;
use skate2_formats::collision::{
    ClusteredMesh, CollisionError, KdTreeMappedArray, MeshTriangle, PrimitiveHit, Volume,
    triangle_aabb_overlap,
};

#[derive(Clone, Debug, PartialEq)]
pub struct CollisionScene {
    pub location_name: String,
    pub focus: [f32; 3],
    pub stream_focus: StreamFocusState,
    pub collections: Vec<CollisionCollection>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollisionCollection {
    pub id: u64,
    pub name: String,
    pub position: [f32; 3],
    pub distance_xz: f32,
    pub arenas: Vec<CollisionArena>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollisionArena {
    pub id: u32,
    pub grind_assets: Vec<GrindAsset>,
    pub aggregate_roots: Vec<CollisionRoot>,
    pub primitive_sets: Vec<CollisionPrimitiveSet>,
    pub simple_primitive_sets: Vec<CollisionSimplePrimitiveSet>,
    pub meshes: Vec<CollisionMesh>,
    pub collision_models: Vec<CollisionModel>,
    pub instances: Vec<CollisionInstance>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollisionAggregateRef {
    PrimitiveSet(usize),
    SimplePrimitiveSet(usize),
    Mesh(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollisionRoot {
    pub dictionary_index: u32,
    pub volume: Volume,
    pub aggregate: Option<CollisionAggregateRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollisionModel {
    pub dictionary_index: u32,
    pub bounding_root: Option<usize>,
    pub collision_roots: Vec<usize>,
    pub assembly_object_id: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollisionInstance {
    pub guid: u64,
    pub local_guid: u64,
    pub name: String,
    pub transform: [f32; 16],
    pub bounding_box_min: [f32; 3],
    pub bounding_box_max: [f32; 3],
    pub collision_model: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollisionPrimitiveSet {
    pub dictionary_index: u32,
    pub bounding_box_min: [f32; 3],
    pub bounding_box_max: [f32; 3],
    pub volumes: Vec<Volume>,
    pub mapped_array: KdTreeMappedArray,
    pub object_bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollisionSimplePrimitiveSet {
    pub dictionary_index: u32,
    pub bounding_box_min: [f32; 3],
    pub bounding_box_max: [f32; 3],
    pub volumes: Vec<Volume>,
}

/// Collision primitives for one assembly part in a streamed dynamic-object
/// model. The part transform is the serialized aggregate root's local matrix.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicCollisionPart {
    pub transform: [f32; 16],
    pub volumes: Vec<Volume>,
    pub triangles: Vec<MeshTriangle>,
}

/// Runtime collision instance corresponding to a DMO or vehicle physics body.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicCollisionBody {
    pub model_id: u64,
    pub transform: [f32; 16],
    pub bounding_box_min: [f32; 3],
    pub bounding_box_max: [f32; 3],
    pub parts: Vec<DynamicCollisionPart>,
}

/// One live physics assembly and the application owner ID recovered by
/// `LWCollideableObjectMan::GetAssemblyOwnerID`.
#[derive(Clone, Debug, PartialEq)]
pub struct OwnedDynamicCollisionBody {
    pub object_id: u32,
    pub body: DynamicCollisionBody,
}

/// One live DMO collision body paired with the priority record installed by
/// `DynamicObject::OnSpawnComplete`. The record's object ID is also the
/// assembly-owner ID returned by the overlap query.
#[derive(Clone, Debug, PartialEq)]
pub struct PrioritizedDynamicCollisionBody {
    pub priority: DynamicObjectPriorityIndex,
    pub body: DynamicCollisionBody,
}

/// The dynamic-pool scratch vectors stop accepting assemblies at 64 entries.
pub const DYNAMIC_POOL_OVERLAP_ASSEMBLY_CAPACITY: usize = 64;
/// `LWDynamicObjectMan` owns a fixed 50-entry overlap-ID list.
pub const DYNAMIC_OBJECT_OVERLAP_ID_CAPACITY: usize = 50;
/// `LWVehicleMan` owns a fixed 24-entry overlap-ID list.
pub const VEHICLE_OVERLAP_ID_CAPACITY: usize = 24;
/// DMO candidates shrink both core shapes by this amount during overlap tests.
pub const DYNAMIC_OBJECT_OVERLAP_PADDING: f32 = -0.05;
/// Vehicle candidates use their unmodified primitive fatness.
pub const VEHICLE_OVERLAP_PADDING: f32 = 0.0;
/// Fixed number of full DMO objects reserved by
/// `LWDynamicObjectMemoryMan::Init` at `0x82d29d60`.
pub const DYNAMIC_OBJECT_ENTITY_CAPACITY: usize = 50;
pub const DYNAMIC_OBJECT_ENTITY_BLOCK_SIZE: usize = 0x1120;
pub const DYNAMIC_OBJECT_CORE_DATA_BLOCK_SIZE: usize = 0x120;
pub const DYNAMIC_OBJECT_COLLISION_BLOCK_SIZE: usize = 0x34;
pub const DYNAMIC_OBJECT_LOCATOR_CAPACITY: usize = 0x400;
pub const DYNAMIC_OBJECT_LOCATOR_BLOCK_SIZE: usize = 0xa0;
/// Fixed EASTL bucket count used by the 50-entry DMO pending-cull map.
pub const DYNAMIC_OBJECT_CULL_BUCKET_COUNT: usize = 51;

/// High-byte discriminator installed by `LWBasicEntity` in every living-world
/// entity ID. Only the dynamic-object value has currently been proven.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LivingWorldEntityType(u8);

impl LivingWorldEntityType {
    /// `eBaseEntityType_DYNAMICOBJECTTYPE`, passed by `DynamicObject` at
    /// `0x82d3dc58`.
    pub const DYNAMIC_OBJECT: Self = Self(5);

    /// Retains an entity type recovered by a caller without inventing an enum
    /// name for values not yet identified in the executable.
    #[must_use]
    pub const fn from_raw(value: u8) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
}

/// Shared ID source used by all living-world entity types.
///
/// `LWBasicEntity::LWBasicEntity` at `0x82e61400` emits the supplied type in
/// the high byte and the low 24 bits of `LivingWorldEntity::NextEntityID`, then
/// increments that global counter. Static storage at `0x83025054` initializes
/// the counter to one. Consequently this allocator must eventually be owned by
/// the complete living-world runtime rather than by the DMO pool alone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LivingWorldEntityIdAllocator {
    next_counter: u32,
}

impl LivingWorldEntityIdAllocator {
    /// Restores the global counter from a known runtime state or trace.
    #[must_use]
    pub const fn from_next_counter(next_counter: u32) -> Self {
        Self { next_counter }
    }

    #[must_use]
    pub const fn next_counter(self) -> u32 {
        self.next_counter
    }

    /// Allocates one ID and preserves the executable's 32-bit counter wrap.
    pub fn allocate(&mut self, entity_type: LivingWorldEntityType) -> u32 {
        let id = (u32::from(entity_type.raw()) << 24) | (self.next_counter & 0x00ff_ffff);
        self.next_counter = self.next_counter.wrapping_add(1);
        id
    }
}

impl Default for LivingWorldEntityIdAllocator {
    fn default() -> Self {
        Self::from_next_counter(1)
    }
}

/// Valid-block allocation order used by `Sk8::MemStore`.
///
/// Initialization fills the free list with descending indices and pops from
/// its end, so a fresh store allocates slots `0, 1, ...`. Freeing pushes the
/// slot back onto the same end, giving exact LIFO reuse. Raw invalid/interior
/// pointers are intentionally outside this typed native contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingWorldMemStoreSlots<const N: usize> {
    free_list: [u32; N],
    available: usize,
}

impl<const N: usize> LivingWorldMemStoreSlots<N> {
    /// # Panics
    ///
    /// Panics when `N` cannot be represented by the reference store's
    /// unsigned 32-bit slot indices.
    #[must_use]
    pub fn new() -> Self {
        assert!(u32::try_from(N).is_ok(), "MemStore slot count exceeds uint");
        Self {
            free_list: std::array::from_fn(|index| {
                u32::try_from(N - index - 1).expect("slot count was checked above")
            }),
            available: N,
        }
    }

    #[must_use]
    pub const fn available(&self) -> usize {
        self.available
    }

    pub fn allocate(&mut self) -> Option<u32> {
        self.available = self.available.checked_sub(1)?;
        let slot = self.free_list[self.available];
        self.free_list[self.available] = 0xdead_beef;
        Some(slot)
    }

    /// Returns a valid slot to the free-list top. The runtime only calls this
    /// with a block previously returned by the same store.
    pub fn free(&mut self, slot: u32) -> bool {
        let Ok(slot_index) = usize::try_from(slot) else {
            return false;
        };
        if slot_index >= N || self.available >= N {
            return false;
        }
        self.free_list[self.available] = slot;
        self.available += 1;
        true
    }
}

impl<const N: usize> Default for LivingWorldMemStoreSlots<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Identity allocated for one full DMO object block. Core-data and collision
/// stores have matching 50-slot capacities but are requested separately by
/// the DMO constructor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DynamicObjectEntityAllocation {
    pub object_id: u32,
    pub entity_slot: u32,
    pub core_data_slot: u32,
    pub collision_slot: u32,
}

/// Couples the proven DMO object store with the shared cross-entity ID source.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DynamicObjectEntityAllocator {
    entities: LivingWorldMemStoreSlots<DYNAMIC_OBJECT_ENTITY_CAPACITY>,
    core_data: LivingWorldMemStoreSlots<DYNAMIC_OBJECT_ENTITY_CAPACITY>,
    collisions: LivingWorldMemStoreSlots<DYNAMIC_OBJECT_ENTITY_CAPACITY>,
}

impl DynamicObjectEntityAllocator {
    /// `CreateEntity` obtains storage before invoking the constructor, so an
    /// exhausted object store must not consume the global entity-ID counter.
    pub fn allocate(
        &mut self,
        entity_ids: &mut LivingWorldEntityIdAllocator,
    ) -> Option<DynamicObjectEntityAllocation> {
        if self.available() == 0 {
            return None;
        }
        let entity_slot = self.entities.allocate()?;
        let object_id = entity_ids.allocate(LivingWorldEntityType::DYNAMIC_OBJECT);
        let core_data_slot = self.core_data.allocate()?;
        let collision_slot = self.collisions.allocate()?;
        Some(DynamicObjectEntityAllocation {
            object_id,
            entity_slot,
            core_data_slot,
            collision_slot,
        })
    }

    pub fn free(&mut self, allocation: DynamicObjectEntityAllocation) -> bool {
        // `DynamicObject::~DynamicObject` deinitializes collision before
        // freeing core data; factory destruction returns the full object last.
        let collision_freed = self.collisions.free(allocation.collision_slot);
        let core_data_freed = self.core_data.free(allocation.core_data_slot);
        let entity_freed = self.entities.free(allocation.entity_slot);
        collision_freed && core_data_freed && entity_freed
    }

    #[must_use]
    pub fn available(&self) -> usize {
        self.entities
            .available()
            .min(self.core_data.available())
            .min(self.collisions.available())
    }
}

/// Replays the reference pending-cull hash iteration when the original DMO
/// object-store base address is known from a trace.
///
/// EASTL hashes each entity pointer as its unchanged 32-bit value, reduces it
/// modulo 51, inserts new nodes at the bucket head, and visits buckets from 0
/// upward during census update. DMO pointers are `base + slot * 0x1120`.
/// Duplicate slots model repeated cull requests and are retained only once.
///
/// Returns `None` for a forged slot or an address outside the 32-bit reference
/// address space.
#[must_use]
pub fn dynamic_object_reference_cull_order(
    pending_requests: &[DynamicObjectEntityAllocation],
    entity_store_base: u32,
) -> Option<Vec<u32>> {
    let mut seen_slots = [false; DYNAMIC_OBJECT_ENTITY_CAPACITY];
    let mut buckets: [Vec<u32>; DYNAMIC_OBJECT_CULL_BUCKET_COUNT] =
        std::array::from_fn(|_| Vec::new());
    let stride = u32::try_from(DYNAMIC_OBJECT_ENTITY_BLOCK_SIZE).ok()?;
    let bucket_count = u32::try_from(DYNAMIC_OBJECT_CULL_BUCKET_COUNT).ok()?;

    for allocation in pending_requests {
        let slot = usize::try_from(allocation.entity_slot).ok()?;
        if slot >= DYNAMIC_OBJECT_ENTITY_CAPACITY {
            return None;
        }
        if seen_slots[slot] {
            continue;
        }
        seen_slots[slot] = true;

        let slot_offset = allocation.entity_slot.checked_mul(stride)?;
        let pointer = entity_store_base.checked_add(slot_offset)?;
        let bucket = usize::try_from(pointer % bucket_count).ok()?;
        buckets[bucket].insert(0, allocation.object_id);
    }

    Some(buckets.into_iter().flatten().collect())
}

/// Per-entity lifetime around the census manager's deferred-cull set.
///
/// The first `RequestCull` at `0x8271a5e8` invokes `OnCullRequested`, detaches
/// the entity from its managers, and inserts it into the pending set. Repeated
/// requests are ignored. Census `Update` at `0x82e67b10` later invokes the
/// typed `Cull`/factory destruction path for every pending entity and clears
/// the set. This state deliberately makes no claim about relative destruction
/// order between multiple entities because the reference iterates a
/// pointer-keyed EASTL hash table.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LivingWorldEntityLifetime {
    #[default]
    Active,
    CullRequested,
    Destroyed,
}

impl LivingWorldEntityLifetime {
    /// Starts the immediate request phase. `true` means the caller must run
    /// `OnCullRequested` and detach the entity from its managers exactly once.
    pub const fn request_cull(&mut self) -> bool {
        if matches!(self, Self::Active) {
            *self = Self::CullRequested;
            true
        } else {
            false
        }
    }

    /// Completes the census-update phase. `true` means the caller must invoke
    /// the typed `Cull`/factory destruction path exactly once.
    pub const fn finish_cull_update(&mut self) -> bool {
        if matches!(self, Self::CullRequested) {
            *self = Self::Destroyed;
            true
        } else {
            false
        }
    }
}

/// Global weights used by the living-world dynamic-object priority queue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicObjectPriorityWeights {
    pub camera_plane: f32,
    pub in_use: f32,
    pub priority: f32,
    pub safety: f32,
    pub distance: f32,
}

/// Camera and simulation positions sampled while evaluating a DMO locator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicObjectPriorityContext {
    pub simulation_center: [f32; 3],
    pub camera_position: [f32; 3],
    pub camera_heading: [f32; 3],
}

/// The five-field `Sk8::LW::tObjectPriorityIndex` recovered from the PDB.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicObjectPriorityIndex {
    pub object_id: u32,
    pub priority: f32,
    pub safety: f32,
    pub other_factors: f32,
    pub multiplier: f32,
}

impl DynamicObjectPriorityIndex {
    /// Queue comparison value, preserving the reference addition and
    /// multiplication order at `0x82d2da38`.
    #[must_use]
    pub fn score(self) -> f32 {
        ((self.other_factors + self.safety) + self.priority) * self.multiplier
    }
}

/// Priority fields sampled from one live DMO returned by the model-overlap
/// query. The reference fills a complete `tObjectPriorityIndex`, but overlap
/// admission deliberately compares only these two fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicObjectOverlapPriority {
    pub object_id: u32,
    pub priority: f32,
    pub safety: f32,
}

/// Ordered cull requests made immediately before an admitted DMO is spawned.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicObjectAdmissionPlan {
    pub overlapping_dynamic_objects: Vec<u32>,
    pub overlapping_vehicles: Vec<u32>,
    pub lowest_priority_object: Option<u32>,
}

/// Result of the dynamic-object census admission transaction at `0x8271c690`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicObjectAdmissionRejection {
    PoolPriority,
    OverlapPriority { blocking_object_id: u32 },
}

/// Applies the priority gates which follow locator selection in the reference
/// census. Supplying `lowest_pool_object` means the pool is full. An admitted
/// plan preserves the observed cull order: overlapping DMOs, overlapping
/// vehicles, then the pool's lowest-priority DMO.
///
/// Physical overlap discovery is intentionally outside this policy. The game
/// builds those ID lists by testing `RenderWare` collision primitives with DMO
/// padding `-0.05` and vehicle padding `0.0`.
///
/// # Errors
///
/// Returns the first failed gate: a non-greater full-pool score or the first
/// overlapping live DMO within the reference priority tolerance.
pub fn plan_dynamic_object_admission(
    candidate: DynamicObjectPriorityIndex,
    lowest_pool_object: Option<DynamicObjectPriorityIndex>,
    overlapping_dynamic_objects: &[DynamicObjectOverlapPriority],
    overlapping_vehicles: &[u32],
) -> Result<DynamicObjectAdmissionPlan, DynamicObjectAdmissionRejection> {
    if lowest_pool_object.is_some_and(|lowest| candidate.score() <= lowest.score()) {
        return Err(DynamicObjectAdmissionRejection::PoolPriority);
    }

    let candidate_overlap_priority = candidate.safety + candidate.priority;
    for overlapping in overlapping_dynamic_objects {
        let blocking_threshold = (overlapping.safety + overlapping.priority) + 0.001;
        if candidate_overlap_priority <= blocking_threshold {
            return Err(DynamicObjectAdmissionRejection::OverlapPriority {
                blocking_object_id: overlapping.object_id,
            });
        }
    }

    Ok(DynamicObjectAdmissionPlan {
        overlapping_dynamic_objects: overlapping_dynamic_objects
            .iter()
            .map(|overlapping| overlapping.object_id)
            .collect(),
        overlapping_vehicles: overlapping_vehicles.to_vec(),
        lowest_priority_object: lowest_pool_object.map(|lowest| lowest.object_id),
    })
}

/// Reconstructs `LWDynamicObjectMan::GeneratePriorityIndexForLocator` at
/// `0x82d34b68`. The caller supplies the model's vault-backed priority index,
/// persistence safety value, runtime static flag, and locator position.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn dynamic_object_priority_index(
    priority_index: i32,
    safety_index: i32,
    is_static: bool,
    position: [f32; 3],
    context: DynamicObjectPriorityContext,
    weights: DynamicObjectPriorityWeights,
) -> DynamicObjectPriorityIndex {
    let simulation_delta = subtract3(position, context.simulation_center);
    let camera_delta = subtract3(position, context.camera_position);
    let camera_direction = normalize3_or(camera_delta, [1.0, 0.0, 0.0]);
    let camera_plane = if dot3(context.camera_heading, camera_direction) > 0.0 {
        weights.camera_plane
    } else {
        1.0
    };
    let in_use = if is_static { weights.in_use } else { 1.0 };
    let priority = if is_static {
        f32::from_bits(0x4f00_0000)
    } else {
        ((priority_index as f32) * weights.priority) * in_use
    };
    let safety = if safety_index == -1 {
        f32::from_bits(0xcf00_0000)
    } else {
        ((-safety_index) as f32 * in_use) * weights.safety
    };
    let negative_distance = -dot3(simulation_delta, simulation_delta).sqrt();
    let other_factors =
        (weights.distance * negative_distance) / ((in_use * camera_plane) * camera_plane);
    DynamicObjectPriorityIndex {
        // Candidate entries are initialized to zero. Live queue entries gain
        // their object ID in the object-registration path.
        object_id: 0,
        priority,
        safety,
        other_factors,
        multiplier: camera_plane,
    }
}

fn subtract3(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| left[axis] - right[axis])
}

fn dot3(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn normalize3_or(value: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let length_squared = dot3(value, value);
    if length_squared == 0.0 {
        fallback
    } else {
        let inverse_length = length_squared.sqrt().recip();
        value.map(|component| component * inverse_length)
    }
}

impl DynamicCollisionBody {
    /// Transforms the model bounds with the reference centre/extent algorithm
    /// used before living-world volume-overlap queries.
    #[must_use]
    pub fn world_bounding_box(&self) -> ([f32; 3], [f32; 3]) {
        transform_collision_aabb(self.bounding_box_min, self.bounding_box_max, self.transform)
    }

    /// Expands the body's assembly parts into world-space GP leaves for the
    /// living-world volume/volume overlap path.
    #[must_use]
    pub fn world_gp_primitives(&self) -> Vec<GpPrimitive> {
        let mut primitives = Vec::new();
        for part in &self.parts {
            let part_to_world = compose_collision_affine(self.transform, part.transform);
            primitives.extend(
                part.volumes
                    .iter()
                    .filter_map(|volume| GpPrimitive::from_volume(volume, Some(part_to_world))),
            );
            primitives.extend(part.triangles.iter().map(|triangle| {
                GpPrimitive::from_triangle_points(
                    triangle
                        .vertices
                        .map(|point| transform_affine_point(point, part_to_world)),
                )
            }));
        }
        primitives
    }

    /// Reconstructs the broad-phase and primitive loop used by
    /// `VolumeVolumeListOverlapTest` for one candidate/live assembly pair.
    /// Candidate primitives are reused while live primitives remain the outer
    /// loop, matching `VolumeTestFunctor::operator()` at `0x82e49858`.
    #[must_use]
    pub fn overlaps_dynamic_body(&self, live_body: &Self, padding: f32) -> bool {
        let (test_minimum, test_maximum) = self.world_bounding_box();
        let (live_minimum, live_maximum) = live_body.world_bounding_box();
        if !aabb_overlaps(test_minimum, test_maximum, live_minimum, live_maximum) {
            return false;
        }

        let test_primitives = self.world_gp_primitives();
        let live_primitives = live_body.world_gp_primitives();
        live_primitives.iter().any(|live_primitive| {
            test_primitives.iter().any(|test_primitive| {
                generic_overlap_proven_subset(test_primitive, live_primitive, padding) == Some(true)
            })
        })
    }

    /// Runs the five thin lines against the body's model-space primitives and
    /// retains the nearest primitive per line. Vector order preserves assembly
    /// part and simple-array volume order.
    #[must_use]
    pub fn grind_geometry_batch(
        &self,
        batch: &GrindGeometryProbeBatch,
    ) -> [GrindGeometryCollision; 5] {
        let mut collisions = [GrindGeometryCollision::default(); 5];
        for part in &self.parts {
            let part_to_world = compose_collision_affine(self.transform, part.transform);
            for volume in &part.volumes {
                let mut world_volume = volume.clone();
                world_volume.transform = compose_collision_affine(part_to_world, volume.transform);
                for (line, collision) in batch.lines.iter().zip(&mut collisions) {
                    let delta =
                        std::array::from_fn(|axis| line.low_point[axis] - line.high_point[axis]);
                    let Some(hit) = world_volume.thin_segment_intersection(line.high_point, delta)
                    else {
                        continue;
                    };
                    if collision.hit && hit.line_parameter >= collision.line_parameter {
                        continue;
                    }
                    *collision = GrindGeometryCollision {
                        hit: true,
                        line_parameter: hit.line_parameter,
                        position: hit.position,
                        normal: hit.normal,
                        surface_id: if volume.surface_id == u32::MAX {
                            0
                        } else {
                            volume.surface_id
                        },
                    };
                }
            }
            for triangle in &part.triangles {
                let vertices = triangle
                    .vertices
                    .map(|vertex| transform_affine_point(vertex, part_to_world));
                for (line, collision) in batch.lines.iter().zip(&mut collisions) {
                    let delta =
                        std::array::from_fn(|axis| line.low_point[axis] - line.high_point[axis]);
                    let Some(hit) = skate2_formats::collision::thin_segment_triangle_intersection(
                        line.high_point,
                        delta,
                        vertices,
                    ) else {
                        continue;
                    };
                    if collision.hit && hit.line_parameter >= collision.line_parameter {
                        continue;
                    }
                    *collision = GrindGeometryCollision {
                        hit: true,
                        line_parameter: hit.line_parameter,
                        position: hit.position,
                        normal: hit.normal,
                        surface_id: triangle.surface_id.map_or(0, u32::from),
                    };
                }
            }
        }
        collisions
    }
}

/// Builds the DMO overlap-owner list in live assembly order. The pool admits
/// at most 64 assemblies to its scratch arrays; owner zero is discarded and
/// repeated owners retain only their first occurrence.
#[must_use]
pub fn dynamic_object_overlap_ids(
    candidate: &DynamicCollisionBody,
    live_bodies: &[OwnedDynamicCollisionBody],
) -> Vec<u32> {
    dynamic_body_overlap_ids(
        candidate,
        live_bodies.iter().map(|live| (live.object_id, &live.body)),
        DYNAMIC_OBJECT_OVERLAP_PADDING,
        DYNAMIC_OBJECT_OVERLAP_ID_CAPACITY,
    )
}

/// Vehicle counterpart of [`dynamic_object_overlap_ids`], preserving its
/// zero padding and 24-owner output capacity.
#[must_use]
pub fn vehicle_overlap_ids(
    candidate: &DynamicCollisionBody,
    live_bodies: &[OwnedDynamicCollisionBody],
) -> Vec<u32> {
    dynamic_body_overlap_ids(
        candidate,
        live_bodies.iter().map(|live| (live.object_id, &live.body)),
        VEHICLE_OVERLAP_PADDING,
        VEHICLE_OVERLAP_ID_CAPACITY,
    )
}

/// Connects the recovered physical overlap queries to the census admission
/// policy for one locator. Runtime object creation and cull-request lifetime
/// remain caller-owned; this function is a pure snapshot of the decisions
/// made before those requests are issued.
///
/// # Errors
///
/// Returns the same full-pool or overlapping-DMO priority rejection as
/// [`plan_dynamic_object_admission`].
pub fn plan_dynamic_object_admission_from_world(
    candidate_priority: DynamicObjectPriorityIndex,
    candidate_body: &DynamicCollisionBody,
    lowest_pool_object: Option<DynamicObjectPriorityIndex>,
    live_dynamic_objects: &[PrioritizedDynamicCollisionBody],
    live_vehicles: &[OwnedDynamicCollisionBody],
) -> Result<DynamicObjectAdmissionPlan, DynamicObjectAdmissionRejection> {
    let overlapping_dynamic_objects = dynamic_body_overlap_ids(
        candidate_body,
        live_dynamic_objects
            .iter()
            .map(|object| (object.priority.object_id, &object.body)),
        DYNAMIC_OBJECT_OVERLAP_PADDING,
        DYNAMIC_OBJECT_OVERLAP_ID_CAPACITY,
    )
    .into_iter()
    .filter_map(|object_id| {
        live_dynamic_objects
            .iter()
            .find(|object| object.priority.object_id == object_id)
            .map(|object| DynamicObjectOverlapPriority {
                object_id,
                priority: object.priority.priority,
                safety: object.priority.safety,
            })
    })
    .collect::<Vec<_>>();
    let overlapping_vehicles = vehicle_overlap_ids(candidate_body, live_vehicles);

    plan_dynamic_object_admission(
        candidate_priority,
        lowest_pool_object,
        &overlapping_dynamic_objects,
        &overlapping_vehicles,
    )
}

fn dynamic_body_overlap_ids<'a>(
    candidate: &DynamicCollisionBody,
    live_bodies: impl IntoIterator<Item = (u32, &'a DynamicCollisionBody)>,
    padding: f32,
    maximum_ids: usize,
) -> Vec<u32> {
    let mut ids = Vec::with_capacity(maximum_ids);
    for (object_id, live_body) in live_bodies
        .into_iter()
        .take(DYNAMIC_POOL_OVERLAP_ASSEMBLY_CAPACITY)
    {
        if !candidate.overlaps_dynamic_body(live_body, padding)
            || object_id == 0
            || ids.contains(&object_id)
        {
            continue;
        }
        if ids.len() == maximum_ids {
            break;
        }
        ids.push(object_id);
    }
    ids
}

/// Executes one dynamic collision pool in stable body order. Strictly-nearer
/// replacement keeps the first body/part/volume on an equal-parameter tie.
#[must_use]
pub fn dynamic_grind_geometry_batch(
    bodies: &[DynamicCollisionBody],
    batch: &GrindGeometryProbeBatch,
) -> [GrindGeometryCollision; 5] {
    let body_batches = bodies
        .iter()
        .map(|body| body.grind_geometry_batch(batch))
        .collect::<Vec<_>>();
    crate::grind::merge_grind_geometry_collision_batches(&body_batches)
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollisionMesh {
    pub dictionary_index: u32,
    pub bounding_box_min: [f32; 3],
    pub bounding_box_max: [f32; 3],
    pub triangles: Vec<MeshTriangle>,
    pub clustered_mesh: ClusteredMesh,
    pub object_bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollisionHitSource {
    Primitive {
        collection_index: usize,
        arena_index: usize,
        set_index: usize,
        volume_index: usize,
    },
    Mesh {
        collection_index: usize,
        arena_index: usize,
        mesh_index: usize,
        cluster_index: usize,
        unit_stream_offset: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionHit {
    pub line_parameter: f32,
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub surface_id: Option<u32>,
    pub source: CollisionHitSource,
}

impl CollisionScene {
    #[must_use]
    pub fn current_collection(&self) -> Option<&CollisionCollection> {
        let current_id = self.stream_focus.current_id()?;
        self.collections
            .iter()
            .find(|collection| collection.id == current_id)
    }

    /// Queries the static authored grind splines owned by the resident stream
    /// collections. Results preserve collection/arena/asset load order and the
    /// reference 40-entry cap.
    #[must_use]
    pub fn grind_box_test(
        &self,
        minimum: [f32; 3],
        maximum: [f32; 3],
        ignore_vertical: bool,
    ) -> Vec<GrindPrimitive> {
        let assets = self
            .collections
            .iter()
            .flat_map(|collection| &collection.arenas)
            .flat_map(|arena| &arena.grind_assets);
        grind_box_test(assets, minimum, maximum, ignore_vertical)
    }

    /// Adds world-transformed dynamic-object and vehicle rails after resident
    /// authored rails, using the reference provider/cap ordering.
    #[must_use]
    pub fn grind_box_test_with_dynamic<I>(
        &self,
        minimum: [f32; 3],
        maximum: [f32; 3],
        ignore_vertical: bool,
        query_dynamic: impl FnOnce(GrindBounds) -> I,
    ) -> Vec<GrindPrimitive>
    where
        I: IntoIterator<Item = GrindPrimitive>,
    {
        let assets = self
            .collections
            .iter()
            .flat_map(|collection| &collection.arenas)
            .flat_map(|arena| &arena.grind_assets);
        grind_box_test_with_dynamic(assets, minimum, maximum, ignore_vertical, query_dynamic)
    }

    /// Executes resident authored rails followed by the concrete
    /// living-world DMO and vehicle managers.
    #[must_use]
    pub fn grind_box_test_with_living_world(
        &self,
        minimum: [f32; 3],
        maximum: [f32; 3],
        ignore_vertical: bool,
        living_world: &LivingWorldGrindScene,
    ) -> Vec<GrindPrimitive> {
        let assets = self
            .collections
            .iter()
            .flat_map(|collection| &collection.arenas)
            .flat_map(|arena| &arena.grind_assets);
        grind_box_test_with_living_world(assets, minimum, maximum, ignore_vertical, living_world)
    }

    /// Executes the static authored portion of the reference trajectory grind
    /// query, including its duplicate/stacked-rail pair filter.
    #[must_use]
    pub fn trajectory_grind_box_test(
        &self,
        minimum: [f32; 3],
        maximum: [f32; 3],
        takeoff_position: [f32; 3],
        pair_tuning: GrindPairFilterTuning,
    ) -> Vec<GrindPrimitive> {
        let assets = self
            .collections
            .iter()
            .flat_map(|collection| &collection.arenas)
            .flat_map(|arena| &arena.grind_assets);
        trajectory_grind_box_test(assets, minimum, maximum, takeoff_position, pair_tuning)
    }

    /// Dynamic-provider form of [`Self::trajectory_grind_box_test`].
    #[must_use]
    pub fn trajectory_grind_box_test_with_dynamic<I>(
        &self,
        minimum: [f32; 3],
        maximum: [f32; 3],
        takeoff_position: [f32; 3],
        pair_tuning: GrindPairFilterTuning,
        query_dynamic: impl FnOnce(GrindBounds) -> I,
    ) -> Vec<GrindPrimitive>
    where
        I: IntoIterator<Item = GrindPrimitive>,
    {
        let assets = self
            .collections
            .iter()
            .flat_map(|collection| &collection.arenas)
            .flat_map(|arena| &arena.grind_assets);
        trajectory_grind_box_test_with_dynamic(
            assets,
            minimum,
            maximum,
            takeoff_position,
            pair_tuning,
            query_dynamic,
        )
    }

    /// Concrete living-world form of [`Self::trajectory_grind_box_test`].
    #[must_use]
    pub fn trajectory_grind_box_test_with_living_world(
        &self,
        minimum: [f32; 3],
        maximum: [f32; 3],
        takeoff_position: [f32; 3],
        pair_tuning: GrindPairFilterTuning,
        living_world: &LivingWorldGrindScene,
    ) -> Vec<GrindPrimitive> {
        let assets = self
            .collections
            .iter()
            .flat_map(|collection| &collection.arenas)
            .flat_map(|arena| &arena.grind_assets);
        trajectory_grind_box_test_with_living_world(
            assets,
            minimum,
            maximum,
            takeoff_position,
            pair_tuning,
            living_world,
        )
    }

    /// Executes a native thin-segment query against runtime-owned geometry.
    /// Results are ordered nearest first, matching the collision consumers'
    /// expected ordering boundary.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] if an owned KD or mesh allocation is
    /// inconsistent. Scene construction validates these same allocations.
    pub fn thin_segment_hits(
        &self,
        start: [f32; 3],
        end: [f32; 3],
    ) -> Result<Vec<CollisionHit>, CollisionError> {
        let delta = std::array::from_fn(|axis| end[axis] - start[axis]);
        let mut hits = Vec::new();
        for (collection_index, collection) in self.collections.iter().enumerate() {
            for (arena_index, arena) in collection.arenas.iter().enumerate() {
                append_primitive_hits(
                    &mut hits,
                    collection_index,
                    arena_index,
                    arena,
                    start,
                    end,
                    delta,
                )?;
                append_mesh_hits(&mut hits, collection_index, arena_index, arena, start, end)?;
            }
        }
        hits.sort_by(|left, right| left.line_parameter.total_cmp(&right.line_parameter));
        Ok(hits)
    }

    /// Executes the static-world portion of a grind-geometry line batch. Each
    /// line retains only its nearest hit, matching the batch result contract.
    /// Dynamic island bodies are a separate future provider.
    ///
    /// # Errors
    ///
    /// Returns [`CollisionError`] if an owned KD or mesh allocation is
    /// inconsistent.
    pub fn static_grind_geometry_batch(
        &self,
        batch: &GrindGeometryProbeBatch,
    ) -> Result<[GrindGeometryCollision; 5], CollisionError> {
        let mut collisions = [GrindGeometryCollision::default(); 5];
        for (line, collision) in batch.lines.iter().zip(&mut collisions) {
            let Some(hit) = self
                .thin_segment_hits(line.high_point, line.low_point)?
                .into_iter()
                .next()
            else {
                continue;
            };
            *collision = GrindGeometryCollision {
                hit: true,
                line_parameter: hit.line_parameter,
                position: hit.position,
                normal: hit.normal,
                surface_id: hit.surface_id.unwrap_or(0),
            };
        }
        Ok(collisions)
    }

    #[must_use]
    pub fn counts(&self) -> CollisionSceneCounts {
        let mut counts = CollisionSceneCounts::default();
        for collection in &self.collections {
            counts.arenas += collection.arenas.len();
            for arena in &collection.arenas {
                counts.grind_assets += arena.grind_assets.len();
                counts.grind_primitives += arena
                    .grind_assets
                    .iter()
                    .map(|asset| asset.primitives.len())
                    .sum::<usize>();
                counts.vertical_grind_primitives += arena
                    .grind_assets
                    .iter()
                    .flat_map(|asset| &asset.primitives)
                    .filter(|primitive| primitive.is_vertical)
                    .count();
                counts.aggregate_roots += arena.aggregate_roots.len();
                counts.primitive_sets += arena.primitive_sets.len();
                counts.primitive_sets += arena.simple_primitive_sets.len();
                counts.primitives += arena
                    .primitive_sets
                    .iter()
                    .map(|set| set.volumes.len())
                    .sum::<usize>();
                counts.primitives += arena
                    .simple_primitive_sets
                    .iter()
                    .map(|set| set.volumes.len())
                    .sum::<usize>();
                counts.meshes += arena.meshes.len();
                counts.collision_models += arena.collision_models.len();
                counts.instances += arena.instances.len();
                counts.collision_instances += arena
                    .instances
                    .iter()
                    .filter(|instance| instance.collision_model.is_some())
                    .count();
                counts.triangles += arena
                    .meshes
                    .iter()
                    .map(|mesh| mesh.triangles.len())
                    .sum::<usize>();
            }
        }
        counts
    }
}

impl TrajectoryCollisionWorld for CollisionScene {
    type Error = CollisionError;

    fn sweep_sphere_segment(
        &mut self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
    ) -> Result<Option<TrajectorySweepHit>, Self::Error> {
        let mut nearest: Option<(f32, TrajectorySweepHit)> = None;
        for collection in &self.collections {
            for arena in &collection.arenas {
                let delta = std::array::from_fn(|axis| end[axis] - start[axis]);
                let minimum = std::array::from_fn(|axis| start[axis].min(end[axis]) - radius);
                let maximum = std::array::from_fn(|axis| start[axis].max(end[axis]) + radius);
                for set in &arena.primitive_sets {
                    let candidates = set.mapped_array.candidate_volume_indices_for_aabb(
                        &set.object_bytes,
                        minimum,
                        maximum,
                    )?;
                    for volume_index in candidates {
                        let Some(volume) = set.volumes.get(volume_index) else {
                            return Err(CollisionError::KdEntryOutsideMappedArray {
                                start: volume_index,
                                end: volume_index.saturating_add(1),
                                volume_count: set.volumes.len(),
                            });
                        };
                        consider_swept_volume(&mut nearest, volume, start, delta, radius);
                    }
                }
                for set in &arena.simple_primitive_sets {
                    if !aabb_overlaps(set.bounding_box_min, set.bounding_box_max, minimum, maximum)
                    {
                        continue;
                    }
                    for volume in &set.volumes {
                        consider_swept_volume(&mut nearest, volume, start, delta, radius);
                    }
                }
                for mesh in &arena.meshes {
                    for hit in mesh.clustered_mesh.sphere_segment_intersections(
                        &mesh.object_bytes,
                        start,
                        end,
                        radius,
                    )? {
                        let parameter = hit.intersection.line_parameter;
                        if nearest
                            .as_ref()
                            .is_some_and(|(current, _)| parameter >= *current)
                        {
                            continue;
                        }
                        nearest = Some((
                            parameter,
                            TrajectorySweepHit {
                                position: hit.intersection.position,
                                normal: hit.intersection.normal,
                                surface_id: hit.triangle.surface_id.map_or(0, u32::from),
                            },
                        ));
                    }
                }
            }
        }
        Ok(nearest.map(|(_, hit)| hit))
    }

    fn triangles_overlapping_aabb(
        &mut self,
        minimum: [f32; 3],
        maximum: [f32; 3],
        capacity: usize,
    ) -> Result<Vec<TrajectoryGroundTriangle>, Self::Error> {
        if capacity == 0 {
            return Ok(Vec::new());
        }
        let mut result = Vec::with_capacity(capacity);
        for collection in &self.collections {
            for arena in &collection.arenas {
                for mesh in &arena.meshes {
                    for triangle in &mesh.triangles {
                        if triangle_aabb_overlap(minimum, maximum, triangle.vertices) {
                            result.push(TrajectoryGroundTriangle {
                                vertices: triangle.vertices,
                                surface_id: triangle.surface_id.map_or(0, u32::from),
                            });
                            if result.len() == capacity {
                                return Ok(result);
                            }
                        }
                    }
                }
            }
        }
        Ok(result)
    }
}

fn consider_swept_volume(
    nearest: &mut Option<(f32, TrajectorySweepHit)>,
    volume: &Volume,
    start: [f32; 3],
    delta: [f32; 3],
    radius: f32,
) {
    let Some(hit) = volume.segment_intersection_with_fatness(start, delta, radius) else {
        return;
    };
    if nearest
        .as_ref()
        .is_some_and(|(current, _)| hit.line_parameter >= *current)
    {
        return;
    }
    *nearest = Some((
        hit.line_parameter,
        TrajectorySweepHit {
            position: hit.position,
            normal: hit.normal,
            surface_id: if volume.surface_id == u32::MAX {
                0
            } else {
                volume.surface_id
            },
        },
    ));
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CollisionSceneCounts {
    pub arenas: usize,
    pub aggregate_roots: usize,
    pub primitive_sets: usize,
    pub primitives: usize,
    pub meshes: usize,
    pub triangles: usize,
    pub collision_models: usize,
    pub instances: usize,
    pub collision_instances: usize,
    pub grind_assets: usize,
    pub grind_primitives: usize,
    pub vertical_grind_primitives: usize,
}

fn append_primitive_hits(
    hits: &mut Vec<CollisionHit>,
    collection_index: usize,
    arena_index: usize,
    arena: &CollisionArena,
    start: [f32; 3],
    end: [f32; 3],
    delta: [f32; 3],
) -> Result<(), CollisionError> {
    let minimum = std::array::from_fn(|axis| start[axis].min(end[axis]));
    let maximum = std::array::from_fn(|axis| start[axis].max(end[axis]));
    for (set_index, set) in arena.primitive_sets.iter().enumerate() {
        let candidates = set.mapped_array.candidate_volume_indices_for_line(
            &set.object_bytes,
            start,
            end,
            0.0,
        )?;
        for volume_index in candidates {
            let Some(volume) = set.volumes.get(volume_index) else {
                return Err(CollisionError::KdEntryOutsideMappedArray {
                    start: volume_index,
                    end: volume_index.saturating_add(1),
                    volume_count: set.volumes.len(),
                });
            };
            let Some(PrimitiveHit {
                line_parameter,
                position,
                normal,
            }) = volume.thin_segment_intersection(start, delta)
            else {
                continue;
            };
            hits.push(CollisionHit {
                line_parameter,
                position,
                normal,
                surface_id: (volume.surface_id != u32::MAX).then_some(volume.surface_id),
                source: CollisionHitSource::Primitive {
                    collection_index,
                    arena_index,
                    set_index,
                    volume_index,
                },
            });
        }
    }
    let set_index_base = arena.primitive_sets.len();
    for (simple_index, set) in arena.simple_primitive_sets.iter().enumerate() {
        if !aabb_overlaps(set.bounding_box_min, set.bounding_box_max, minimum, maximum) {
            continue;
        }
        for (volume_index, volume) in set.volumes.iter().enumerate() {
            let Some(PrimitiveHit {
                line_parameter,
                position,
                normal,
            }) = volume.thin_segment_intersection(start, delta)
            else {
                continue;
            };
            hits.push(CollisionHit {
                line_parameter,
                position,
                normal,
                surface_id: (volume.surface_id != u32::MAX).then_some(volume.surface_id),
                source: CollisionHitSource::Primitive {
                    collection_index,
                    arena_index,
                    set_index: set_index_base + simple_index,
                    volume_index,
                },
            });
        }
    }
    Ok(())
}

fn aabb_overlaps(
    left_minimum: [f32; 3],
    left_maximum: [f32; 3],
    right_minimum: [f32; 3],
    right_maximum: [f32; 3],
) -> bool {
    (0..3).all(|axis| {
        left_minimum[axis] <= right_maximum[axis] && right_minimum[axis] <= left_maximum[axis]
    })
}

/// Composes `RenderWare` `Matrix44Affine` values without consuming their unused
/// W lanes. Collision matrices commonly serialize every W lane, including
/// element 15, as zero; the reference VMX path multiplies the 3x3 axes and
/// adds translation explicitly. For ordinary finite XYZ inputs, the operation
/// order matches `Part::GetTransform` at `0x826c34dc..0x826c352c`: one
/// multiply followed by two fused multiply-adds for each basis column, and
/// three fused multiply-adds beginning with the outer translation for the
/// translation column. Xenon denormal, NaN, signed-zero, and unused W-lane
/// behavior remains outside this host reconstruction's proven boundary.
#[must_use]
pub fn compose_collision_affine(outer: [f32; 16], inner: [f32; 16]) -> [f32; 16] {
    let mut result = [0.0; 16];
    for column in 0..3 {
        for row in 0..3 {
            let x = outer[row] * inner[column * 4];
            let xy = outer[4 + row].mul_add(inner[column * 4 + 1], x);
            result[column * 4 + row] = outer[8 + row].mul_add(inner[column * 4 + 2], xy);
        }
    }
    for row in 0..3 {
        let x = outer[row].mul_add(inner[12], outer[12 + row]);
        let xy = outer[4 + row].mul_add(inner[13], x);
        result[12 + row] = outer[8 + row].mul_add(inner[14], xy);
    }
    result[15] = 1.0;
    result
}

/// Reconstructs the pointer-selected result of `rw::physics::Part::GetTransform`
/// at `0x826c3480..0x826c35a4`.
///
/// A detached Part returns its stored Part transform bit-for-bit. An attached
/// Part without `m_inverseBodyLTM` returns the rigid-body pose bit-for-bit. If
/// both pointers are present, the reference evaluates
/// `body_transform * inverse_body_ltm` using the multiply/FMA graph documented
/// by [`compose_collision_affine`]. The body branch is tested first.
///
/// Ordinary finite affine XYZ behavior is preserved. The composed path shares
/// [`compose_collision_affine`]'s explicit Xenon exceptional-value and unused
/// W-lane boundary.
#[must_use]
pub fn part_get_transform(
    part_transform: [f32; 16],
    body_transform: Option<&[f32; 16]>,
    inverse_body_ltm: Option<&[f32; 16]>,
) -> [f32; 16] {
    let Some(body_transform) = body_transform else {
        return part_transform;
    };
    let Some(inverse_body_ltm) = inverse_body_ltm else {
        return *body_transform;
    };
    compose_collision_affine(*body_transform, *inverse_body_ltm)
}

/// Reconstructs `rw::math::vpu::OrthoNormalize3x3` at
/// `0x82ae33b8..0x82ae3750` for the ordinary finite-input host boundary.
///
/// The reference independently normalizes all three four-lane axes, computes
/// a second independent length estimate for each axis, and only then replaces
/// an exact-zero length with positive zero for its strict `> 0` selection
/// tests. The duplicate X/Y estimates receive two Newton refinements, while
/// the duplicate Z estimate receives only one. When all three axes are
/// admitted, strict comparisons retain the cyclic pair with the smallest
/// absolute correlation; ties prefer X/Y, then Z/X, then Y/Z. It repairs the
/// missing axis as normalized `A x B`, repairs B as normalized `M x A`, and
/// copies the translation column verbatim. Equivalently, with
/// `qX = |Y.Z|`, `qY = |Z.X|`, and `qZ = |X.Y|`, `qX == qY < qZ` repairs Y,
/// while every minimum tie involving qZ repairs Z.
///
/// This host model seeds each reciprocal square root with
/// `length_squared.sqrt().recip()`, preserves both fused Newton refinements,
/// and uses the project's nested XYZ host model for `vmsum3fp`. Exact Xenon
/// `vrsqrtefp` seeds and `vmsum3fp` rounding, denormal handling, NaN/infinity
/// behavior, signed-zero details, and W-lane bits dependent on those host
/// substitutions remain explicit boundaries. There is deliberately no
/// epsilon or finite guard.
#[must_use]
pub fn orthonormalize_collision_affine(matrix: [f32; 16]) -> [f32; 16] {
    let source_axes: [[f32; 4]; 3] =
        std::array::from_fn(|column| std::array::from_fn(|row| matrix[column * 4 + row]));
    let mut axes = source_axes.map(collision_ortho_normalize_axis_host);
    let lengths = collision_ortho_axis_lengths_host(source_axes);

    let (axis_a, axis_b, missing_axis) = if !collision_ortho_length_is_admitted(lengths[0]) {
        (1, 2, 0)
    } else if !collision_ortho_length_is_admitted(lengths[1]) {
        (2, 0, 1)
    } else if collision_ortho_length_is_admitted(lengths[2]) {
        let yz = collision_ortho_dot3_host(axes[1], axes[2]).to_bits() & 0x7fff_ffff;
        let zx = collision_ortho_dot3_host(axes[2], axes[0]).to_bits() & 0x7fff_ffff;
        let xy = collision_ortho_dot3_host(axes[0], axes[1]).to_bits() & 0x7fff_ffff;
        let yz = f32::from_bits(yz);
        let zx = f32::from_bits(zx);
        let xy = f32::from_bits(xy);

        if zx > yz {
            if xy > yz { (1, 2, 0) } else { (0, 1, 2) }
        } else if xy > zx {
            (2, 0, 1)
        } else {
            (0, 1, 2)
        }
    } else {
        (0, 1, 2)
    };

    axes[missing_axis] = collision_ortho_normalized_cross_host(axes[axis_a], axes[axis_b]);
    axes[axis_b] = collision_ortho_normalized_cross_host(axes[missing_axis], axes[axis_a]);

    let mut result = matrix;
    for (column, axis) in axes.into_iter().enumerate() {
        result[column * 4..column * 4 + 4].copy_from_slice(&axis);
    }
    result
}

fn collision_ortho_axis_lengths_host(source_axes: [[f32; 4]; 3]) -> [f32; 3] {
    [
        collision_ortho_refined_length_host(source_axes[0]),
        collision_ortho_refined_length_host(source_axes[1]),
        collision_ortho_one_refinement_length_host(source_axes[2]),
    ]
}

fn collision_ortho_normalize_axis_host(axis: [f32; 4]) -> [f32; 4] {
    let squared_length = collision_ortho_dot3_host(axis, axis);
    let reciprocal_length = collision_ortho_refined_rsqrt_host(squared_length);
    axis.map(|value| value * reciprocal_length)
}

fn collision_ortho_refined_length_host(axis: [f32; 4]) -> f32 {
    let squared_length = collision_ortho_dot3_host(axis, axis);
    let reciprocal_length = collision_ortho_refined_rsqrt_host(squared_length);
    collision_ortho_select_length_host(squared_length, reciprocal_length)
}

fn collision_ortho_one_refinement_length_host(axis: [f32; 4]) -> f32 {
    let squared_length = collision_ortho_dot3_host(axis, axis);
    let reciprocal_length = collision_ortho_rsqrt_first_refinement_host(squared_length);
    collision_ortho_select_length_host(squared_length, reciprocal_length)
}

fn collision_ortho_select_length_host(squared_length: f32, reciprocal_length: f32) -> f32 {
    let length = squared_length * reciprocal_length;
    if squared_length == 0.0 { 0.0 } else { length }
}

fn collision_ortho_length_is_admitted(length: f32) -> bool {
    length > 0.0
}

fn collision_ortho_refined_rsqrt_host(value: f32) -> f32 {
    let estimate1 = collision_ortho_rsqrt_first_refinement_host(value);
    let square1 = estimate1 * estimate1;
    let half1 = estimate1 * 0.5;
    let residual1 = (-value).mul_add(square1, 1.0);
    half1.mul_add(residual1, estimate1)
}

fn collision_ortho_rsqrt_first_refinement_host(value: f32) -> f32 {
    let estimate0 = value.sqrt().recip();
    let square0 = estimate0 * estimate0;
    let half0 = estimate0 * 0.5;
    let residual0 = (-value).mul_add(square0, 1.0);
    half0.mul_add(residual0, estimate0)
}

fn collision_ortho_dot3_host(left: [f32; 4], right: [f32; 4]) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn collision_ortho_normalized_cross_host(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    collision_ortho_normalize_axis_host(collision_vmx_cross_host(left, right))
}

// `ClusteredMeshData::Reset` and `OrthoNormalize3x3` use this identical
// YZXW/multiply/fused-negative-subtract/YZXW graph, including its W residual.
fn collision_vmx_cross_host(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    let left_yzxw = [left[1], left[2], left[0], left[3]];
    let right_yzxw = [right[1], right[2], right[0], right[3]];
    let products: [f32; 4] = std::array::from_fn(|lane| left[lane] * right_yzxw[lane]);
    let differences: [f32; 4] =
        std::array::from_fn(|lane| (-left_yzxw[lane]).mul_add(right[lane], products[lane]));
    [
        differences[1],
        differences[2],
        differences[0],
        differences[3],
    ]
}

/// Builds the quaternion stored by `rw::physics::Simulation::AddRigidBody`
/// after its call to `OrthoNormalize3x3`.
///
/// The active-body copy is at `0x82ae6fb4..0x82ae716c`; the frozen and static
/// copies reproduce the same graph. `matrix` must be the already-orthonormalized
/// output: this helper deliberately does not call [`orthonormalize_collision_affine`].
/// It preserves the reference candidate construction, sign-bit XORs, strict
/// trace/diagonal selection, factor multiplication, and both two-refinement
/// reciprocal-square-root stages. The input translation and all three axis W
/// lanes are not operands; the returned lanes are quaternion XYZW.
///
/// This host model seeds `vrsqrtefp128` with `f32::sqrt().recip()` and models
/// `vmsum4fp128` with a nested four-lane fused reduction. Exact Xenon estimate
/// seeds, horizontal-sum rounding, denormal behavior, NaN payload propagation,
/// infinities, and exceptional signed-zero behavior remain unproved boundaries.
#[must_use]
pub fn simulation_quaternion_from_orthonormalized_matrix_host(matrix: [f32; 16]) -> [f32; 4] {
    let candidate = simulation_quaternion_candidate_host(matrix);
    let norm_squared = simulation_quaternion_dot4_host(candidate);
    let reciprocal_norm = simulation_rsqrt_two_refinements_host(norm_squared);
    candidate.map(|lane| lane * reciprocal_norm)
}

fn simulation_quaternion_candidate_host(matrix: [f32; 16]) -> [f32; 4] {
    try_simulation_quaternion_candidate(matrix, |magnitudes| {
        Ok::<_, core::convert::Infallible>(magnitudes.map(|value| value.sqrt().recip()))
    })
    .expect("host candidate estimate service is infallible")
}

/// Unnormalized matrix-to-quaternion candidate shared by the body conversion
/// and `JointFrames` setters (0x82b22f58, 0x82b23108, 0x82b232b8).
///
/// One vector estimate callback observes all four candidate magnitudes, even
/// when some are zero/negative or later unselected. The two source refinement
/// rounds and strict candidate selection are retained here. This does not
/// apply the final normalization used by the separate body conversion caller.
pub(crate) fn try_simulation_quaternion_candidate<E>(
    matrix: [f32; 16],
    estimate: impl FnOnce([f32; 4]) -> Result<[f32; 4], E>,
) -> Result<[f32; 4], E> {
    let x = [matrix[0], matrix[1], matrix[2], matrix[3]];
    let y = [matrix[4], matrix[5], matrix[6], matrix[7]];
    let z = [matrix[8], matrix[9], matrix[10], matrix[11]];
    let xx = x[0];
    let yy = y[1];
    let zz = z[2];

    let signed_x = [
        xx,
        xx,
        simulation_flip_float_sign(xx),
        simulation_flip_float_sign(xx),
    ];
    let signed_y = [
        yy,
        simulation_flip_float_sign(yy),
        yy,
        simulation_flip_float_sign(yy),
    ];
    let signed_z = [
        zz,
        simulation_flip_float_sign(zz),
        simulation_flip_float_sign(zz),
        zz,
    ];
    let candidate_magnitudes: [f32; 4] = std::array::from_fn(|lane| {
        let xy = signed_x[lane] + signed_y[lane];
        let z_one = signed_z[lane] + 1.0;
        xy + z_one
    });
    let mut reciprocal_roots = estimate(candidate_magnitudes)?;
    for _ in 0..2 {
        let squares = reciprocal_roots.map(|value| value * value);
        let halves = reciprocal_roots.map(|value| value * 0.5);
        let residuals: [f32; 4] =
            std::array::from_fn(|lane| -candidate_magnitudes[lane].mul_add(squares[lane], -1.0));
        reciprocal_roots = std::array::from_fn(|lane| {
            halves[lane].mul_add(residuals[lane], reciprocal_roots[lane])
        });
    }
    let roots: [f32; 4] =
        std::array::from_fn(|lane| candidate_magnitudes[lane] * reciprocal_roots[lane]);
    let factors = reciprocal_roots.map(|estimate| 0.5 * estimate);

    let differences = [y[2] - z[1], z[0] - x[2], x[1] - y[0], 0.5];
    let sums = [y[2] + z[1], z[0] + x[2], x[1] + y[0], 0.5];
    let candidate_w = [
        differences[0] * factors[0],
        differences[1] * factors[0],
        differences[2] * factors[0],
        differences[3] * roots[0],
    ];
    let candidate_x = [
        differences[3] * roots[1],
        sums[2] * factors[1],
        sums[1] * factors[1],
        differences[0] * factors[1],
    ];
    let candidate_y = [
        sums[2] * factors[2],
        differences[3] * roots[2],
        sums[0] * factors[2],
        differences[1] * factors[2],
    ];
    let candidate_z = [
        sums[1] * factors[3],
        sums[0] * factors[3],
        differences[3] * roots[3],
        differences[2] * factors[3],
    ];

    let trace = (xx + yy) + zz;
    Ok(if trace > 0.0 {
        candidate_w
    } else if xx > yy && xx > zz {
        candidate_x
    } else if yy > zz {
        candidate_y
    } else {
        candidate_z
    })
}

fn simulation_flip_float_sign(value: f32) -> f32 {
    f32::from_bits(value.to_bits() ^ 0x8000_0000)
}

fn simulation_rsqrt_two_refinements_host(value: f32) -> f32 {
    let estimate0 = value.sqrt().recip();
    let square0 = estimate0 * estimate0;
    let half0 = estimate0 * 0.5;
    let residual0 = (-value).mul_add(square0, 1.0);
    let estimate1 = half0.mul_add(residual0, estimate0);
    let square1 = estimate1 * estimate1;
    let half1 = estimate1 * 0.5;
    let residual1 = (-value).mul_add(square1, 1.0);
    half1.mul_add(residual1, estimate1)
}

fn simulation_quaternion_dot4_host(value: [f32; 4]) -> f32 {
    value[0].mul_add(
        value[0],
        value[1].mul_add(value[1], value[2].mul_add(value[2], value[3] * value[3])),
    )
}

/// Six-word world-space inverse-inertia tensor produced by
/// `rw::physics::Simulation::AddRigidBody`, plus its copied inverse mass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationRotatedInverseInertia {
    /// `[Ixx, Ixy, Ixz]`, stored natively in `RigidBody::mIfull`.
    pub full: [f32; 3],
    /// `[Izz, Iyy, Iyz]`, stored natively in `RigidBody::mIsplt`.
    pub split: [f32; 3],
    /// Direct, arithmetic-free copy of `Inertia::mInvMass`.
    pub inverse_mass: f32,
}

/// Rotates the inverse-inertia diagonal using the non-null-inertia block in
/// `rw::physics::Simulation::AddRigidBody`.
///
/// The active copy is `0x82ae7170..0x82ae7210`; frozen and statically emitted
/// copies use the same graph. `orthonormalized_matrix` is the already-stored
/// post-Ortho basis. The reference first scales each whole basis vector by its
/// corresponding inverse-tensor component, then uses the recovered ordinary
/// multiply/FMA and `vpermwi128 0x97/0x9b` lane graph. Axis W lanes and affine
/// translation are irrelevant to the six returned tensor words. `inverse_mass`
/// is returned bit-for-bit without arithmetic. There is deliberately no zero
/// shortcut; callers model the native null-inertia branch outside this helper.
///
/// Ordinary finite operation order is preserved with `f32::mul_add`. Exact
/// Xenon denormal flushing, NaN payloads, infinities, overflow/underflow, and
/// fused exceptional signed-zero behavior remain unproved host boundaries.
#[must_use]
pub fn simulation_rotated_inverse_inertia_host(
    orthonormalized_matrix: [f32; 16],
    inverse_tensor: [f32; 3],
    inverse_mass: f32,
) -> SimulationRotatedInverseInertia {
    let right = [
        orthonormalized_matrix[0],
        orthonormalized_matrix[1],
        orthonormalized_matrix[2],
        orthonormalized_matrix[3],
    ];
    let up = [
        orthonormalized_matrix[4],
        orthonormalized_matrix[5],
        orthonormalized_matrix[6],
        orthonormalized_matrix[7],
    ];
    let at = [
        orthonormalized_matrix[8],
        orthonormalized_matrix[9],
        orthonormalized_matrix[10],
        orthonormalized_matrix[11],
    ];
    let right_scaled = right.map(|lane| lane * inverse_tensor[0]);
    let up_scaled = up.map(|lane| lane * inverse_tensor[1]);
    let at_scaled = at.map(|lane| lane * inverse_tensor[2]);

    let full = std::array::from_fn(|lane| {
        let up_term = up_scaled[lane] * up[0];
        let with_right = right_scaled[lane].mul_add(right[0], up_term);
        at_scaled[lane].mul_add(at[0], with_right)
    });
    let split = std::array::from_fn(|lane| {
        let left_lane = [2_usize, 1, 1][lane];
        let right_lane = [2_usize, 1, 2][lane];
        let up_term = up[left_lane] * up_scaled[right_lane];
        let with_right = right[left_lane].mul_add(right_scaled[right_lane], up_term);
        at[left_lane].mul_add(at_scaled[right_lane], with_right)
    });

    SimulationRotatedInverseInertia {
        full,
        split,
        inverse_mass,
    }
}

/// Reconstructs the transform selected by `rw::physics::Part::AddToSimulation`
/// at `0x82ae0ad4..0x82ae0bcc`.
///
/// A missing inverse-body transform returns `part_transform` bit-for-bit. When
/// present, the reference builds a rigid inverse by transposing only the XYZ
/// basis, computes `+0 - translation` arithmetically, and reduces inverse
/// translation in Z, Y, X order. It then evaluates the full four-lane
/// `part_transform * rigid_inverse` graph with the recovered multiply/FMA
/// operand order. This is intentionally distinct from the general Reset
/// inverse and from [`compose_collision_affine`].
///
/// The ordinary finite operation graph is preserved. Exact Xenon denormal,
/// NaN-payload, infinity, and signed-zero instruction behavior remains outside
/// the host parity boundary.
#[must_use]
pub fn part_attachment_transform(
    part_transform: [f32; 16],
    inverse_body_transform: Option<&[f32; 16]>,
) -> [f32; 16] {
    let Some(inverse_body_transform) = inverse_body_transform else {
        return part_transform;
    };
    let rigid_inverse = part_rigid_inverse_by_transpose(*inverse_body_transform);
    part_attachment_compose_host(part_transform, rigid_inverse)
}

/// Applies the `Part::AddToSimulation` body-state override at
/// `0x82ae0ab8..0x82ae0ad0`.
///
/// Raw requested state `4` becomes state `1` only when internal flag bit zero
/// is set. All other requested states pass through unchanged, and all other
/// internal-flag bits are ignored.
#[must_use]
pub fn effective_part_body_state(requested: u32, internal_flags: u16) -> u32 {
    if requested == 4 && internal_flags & 1 != 0 {
        1
    } else {
        requested
    }
}

fn part_rigid_inverse_by_transpose(matrix: [f32; 16]) -> [f32; 16] {
    let columns: [[f32; 4]; 4] =
        std::array::from_fn(|column| std::array::from_fn(|row| matrix[column * 4 + row]));
    let positive_zero = f32::from_bits(0);
    let mut inverse = [
        [columns[0][0], columns[1][0], columns[2][0], positive_zero],
        [columns[0][1], columns[1][1], columns[2][1], positive_zero],
        [columns[0][2], columns[1][2], columns[2][2], positive_zero],
        [positive_zero; 4],
    ];
    let negative_translation = columns[3].map(|value| positive_zero - value);
    inverse[3] = std::array::from_fn(|lane| {
        let z = negative_translation[2] * inverse[2][lane];
        let zy = negative_translation[1].mul_add(inverse[1][lane], z);
        negative_translation[0].mul_add(inverse[0][lane], zy)
    });
    std::array::from_fn(|index| inverse[index / 4][index % 4])
}

fn part_attachment_compose_host(part: [f32; 16], inverse: [f32; 16]) -> [f32; 16] {
    let part_columns: [[f32; 4]; 4] =
        std::array::from_fn(|column| std::array::from_fn(|row| part[column * 4 + row]));
    let inverse_columns: [[f32; 4]; 4] =
        std::array::from_fn(|column| std::array::from_fn(|row| inverse[column * 4 + row]));
    let mut result = [[0.0; 4]; 4];
    for column in 0..3 {
        result[column] = std::array::from_fn(|lane| {
            let x = inverse_columns[column][0] * part_columns[0][lane];
            let xy = inverse_columns[column][1].mul_add(part_columns[1][lane], x);
            inverse_columns[column][2].mul_add(part_columns[2][lane], xy)
        });
    }
    result[3] = std::array::from_fn(|lane| {
        let x = inverse_columns[3][0].mul_add(part_columns[0][lane], part_columns[3][lane]);
        let xy = inverse_columns[3][1].mul_add(part_columns[1][lane], x);
        inverse_columns[3][2].mul_add(part_columns[2][lane], xy)
    });
    std::array::from_fn(|index| result[index / 4][index % 4])
}

/// Reconstructs the affine inverse written by
/// `Sk8::Physics::ClusteredMeshData::Reset` at `0x827c83e4..0x827c84ac`.
///
/// The reference forms cyclic cofactors with an ordinary multiply followed by
/// fused negative multiply-subtract, transposes them with the original W-lane
/// merges, and evaluates the determinant with `vmsum3fp`. This host model uses
/// the project's established nested XYZ multiply-add reduction for that last
/// instruction. It seeds the reciprocal with [`f32::recip`] and then preserves
/// both recovered fused Newton refinements. Finally it bitwise-negates all four
/// translation lanes and evaluates `inverse3x3 * -translation` as one multiply
/// followed by two fused multiply-adds.
///
/// There is deliberately no singularity branch. Exact Xenon `vrefp` seed and
/// `vmsum3fp` rounding, denormal handling, NaN payload propagation, signed-zero
/// behavior, and other exceptional-value bit parity remain unproven host
/// boundaries.
#[must_use]
pub fn inverse_collision_affine_reset(matrix: [f32; 16]) -> [f32; 16] {
    let columns: [[f32; 4]; 4] =
        std::array::from_fn(|column| std::array::from_fn(|row| matrix[column * 4 + row]));
    let cofactor_x = collision_vmx_cross_host(columns[1], columns[2]);
    let cofactor_z = collision_vmx_cross_host(columns[0], columns[1]);
    let cofactor_y = collision_vmx_cross_host(columns[2], columns[0]);
    let determinant = collision_reset_dot3_host(columns[0], cofactor_x);
    let reciprocal = refined_collision_reset_reciprocal(determinant);
    let mut inverse = [[0.0; 4]; 4];
    inverse[0] = [cofactor_x[0], cofactor_y[0], cofactor_z[0], cofactor_y[0]]
        .map(|value| value * reciprocal);
    inverse[1] = [cofactor_x[1], cofactor_y[1], cofactor_z[1], cofactor_y[1]]
        .map(|value| value * reciprocal);
    inverse[2] = [cofactor_x[2], cofactor_y[2], cofactor_z[2], cofactor_y[2]]
        .map(|value| value * reciprocal);
    let negative_translation =
        columns[3].map(|value| f32::from_bits(value.to_bits() ^ 0x8000_0000));
    inverse[3] = std::array::from_fn(|lane| {
        inverse[2][lane].mul_add(
            negative_translation[2],
            inverse[1][lane].mul_add(
                negative_translation[1],
                inverse[0][lane] * negative_translation[0],
            ),
        )
    });
    std::array::from_fn(|index| inverse[index / 4][index % 4])
}

fn collision_reset_dot3_host(left: [f32; 4], right: [f32; 4]) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn refined_collision_reset_reciprocal(value: f32) -> f32 {
    let estimate0 = value.recip();
    let residual0 = (-estimate0).mul_add(value, 1.0);
    let estimate1 = estimate0.mul_add(residual0, estimate0);
    let residual1 = (-estimate1).mul_add(value, 1.0);
    estimate1.mul_add(residual1, estimate1)
}

/// Reconstructs `AABBoxTemplate::Transform` at `0x825e9480`. The reference
/// transforms the local centre and projects half-extents through the absolute
/// 3x3 affine axes, then subtracts/adds those world extents. For ordinary
/// finite inputs the extent graph at `0x825e9520..0x825e953c` starts with a Y
/// multiply, then accumulates X and Z with fused multiply-adds. Xenon
/// exceptional-value behavior remains outside the proven host boundary.
#[must_use]
#[allow(clippy::manual_midpoint)]
pub fn transform_collision_aabb(
    minimum: [f32; 3],
    maximum: [f32; 3],
    transform: [f32; 16],
) -> ([f32; 3], [f32; 3]) {
    let center = std::array::from_fn(|axis| (minimum[axis] + maximum[axis]) * 0.5);
    let half_extents: [f32; 3] = std::array::from_fn(|axis| (maximum[axis] - minimum[axis]) * 0.5);
    let world_center = transform_affine_point(center, transform);
    let world_extents: [f32; 3] = std::array::from_fn(|component| {
        let y = transform[4 + component].abs() * half_extents[1];
        let xy = transform[component].abs().mul_add(half_extents[0], y);
        transform[8 + component].abs().mul_add(half_extents[2], xy)
    });
    (
        std::array::from_fn(|axis| world_center[axis] - world_extents[axis]),
        std::array::from_fn(|axis| world_center[axis] + world_extents[axis]),
    )
}

fn transform_affine_point(point: [f32; 3], transform: [f32; 16]) -> [f32; 3] {
    std::array::from_fn(|component| {
        transform[8 + component].mul_add(
            point[2],
            transform[4 + component].mul_add(
                point[1],
                transform[component].mul_add(point[0], transform[12 + component]),
            ),
        )
    })
}

fn append_mesh_hits(
    hits: &mut Vec<CollisionHit>,
    collection_index: usize,
    arena_index: usize,
    arena: &CollisionArena,
    start: [f32; 3],
    end: [f32; 3],
) -> Result<(), CollisionError> {
    for (mesh_index, mesh) in arena.meshes.iter().enumerate() {
        for hit in mesh
            .clustered_mesh
            .thin_segment_intersections(&mesh.object_bytes, start, end)?
        {
            hits.push(CollisionHit {
                line_parameter: hit.intersection.line_parameter,
                position: hit.intersection.position,
                normal: hit.intersection.normal,
                surface_id: hit.triangle.surface_id.map(u32::from),
                source: CollisionHitSource::Mesh {
                    collection_index,
                    arena_index,
                    mesh_index,
                    cluster_index: hit.triangle.cluster_index,
                    unit_stream_offset: hit.triangle.unit_stream_offset,
                },
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::collision::VolumeType;

    fn overlap_test_identity() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }

    fn orthonormalize_test_identity() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }

    fn mapped_test_sphere(center: [f32; 3], radius: f32, surface_id: u32) -> Volume {
        let mut transform = overlap_test_identity();
        transform[12..15].copy_from_slice(&center);
        Volume {
            transform,
            vtable_offset: VolumeType::Sphere as u32,
            shape_words: [0; 3],
            radius,
            group_id: 0,
            surface_id,
            flags: 0,
        }
    }

    fn mapped_test_kd_bytes(
        axis: u32,
        low: (u32, u32),
        high: (u32, u32),
        extents: [f32; 2],
    ) -> Vec<u8> {
        const BRANCH_OFFSET: usize = 48;
        let mut bytes = vec![0_u8; BRANCH_OFFSET + 32];
        let entry_count = low.0.checked_add(high.0).unwrap();
        bytes[0..4].copy_from_slice(&u32::try_from(BRANCH_OFFSET).unwrap().to_be_bytes());
        bytes[4..8].copy_from_slice(&1_u32.to_be_bytes());
        bytes[8..12].copy_from_slice(&entry_count.to_be_bytes());
        for (index, value) in [-4.0_f32; 4].into_iter().enumerate() {
            let offset = 16 + index * 4;
            bytes[offset..offset + 4].copy_from_slice(&value.to_bits().to_be_bytes());
        }
        for (index, value) in [4.0_f32; 4].into_iter().enumerate() {
            let offset = 32 + index * 4;
            bytes[offset..offset + 4].copy_from_slice(&value.to_bits().to_be_bytes());
        }
        bytes[BRANCH_OFFSET + 4..BRANCH_OFFSET + 8].copy_from_slice(&axis.to_be_bytes());
        bytes[BRANCH_OFFSET + 8..BRANCH_OFFSET + 12].copy_from_slice(&low.0.to_be_bytes());
        bytes[BRANCH_OFFSET + 12..BRANCH_OFFSET + 16].copy_from_slice(&low.1.to_be_bytes());
        bytes[BRANCH_OFFSET + 16..BRANCH_OFFSET + 20].copy_from_slice(&high.0.to_be_bytes());
        bytes[BRANCH_OFFSET + 20..BRANCH_OFFSET + 24].copy_from_slice(&high.1.to_be_bytes());
        bytes[BRANCH_OFFSET + 24..BRANCH_OFFSET + 28]
            .copy_from_slice(&extents[0].to_bits().to_be_bytes());
        bytes[BRANCH_OFFSET + 28..BRANCH_OFFSET + 32]
            .copy_from_slice(&extents[1].to_bits().to_be_bytes());
        bytes
    }

    fn mapped_test_branchless_kd_bytes(entry_count: u32) -> Vec<u8> {
        let mut bytes = vec![0_u8; 48];
        bytes[8..12].copy_from_slice(&entry_count.to_be_bytes());
        for (index, value) in [-4.0_f32; 4].into_iter().enumerate() {
            let offset = 16 + index * 4;
            bytes[offset..offset + 4].copy_from_slice(&value.to_bits().to_be_bytes());
        }
        for (index, value) in [4.0_f32; 4].into_iter().enumerate() {
            let offset = 32 + index * 4;
            bytes[offset..offset + 4].copy_from_slice(&value.to_bits().to_be_bytes());
        }
        bytes
    }

    fn mapped_test_scene(volumes: Vec<Volume>, object_bytes: Vec<u8>) -> CollisionScene {
        let volume_count = u32::try_from(volumes.len()).unwrap();
        CollisionScene {
            location_name: "test".to_owned(),
            focus: [0.0; 3],
            stream_focus: StreamFocusState::new([0.0; 3], false),
            collections: vec![CollisionCollection {
                id: 7,
                name: "test".to_owned(),
                position: [0.0; 3],
                distance_xz: 0.0,
                arenas: vec![CollisionArena {
                    id: 1,
                    grind_assets: vec![],
                    aggregate_roots: vec![],
                    primitive_sets: vec![CollisionPrimitiveSet {
                        dictionary_index: 0,
                        bounding_box_min: [-4.0; 3],
                        bounding_box_max: [4.0; 3],
                        volumes,
                        mapped_array: KdTreeMappedArray {
                            bounding_box_min: [-4.0, -4.0, -4.0, 0.0],
                            bounding_box_max: [4.0, 4.0, 4.0, 0.0],
                            aggregate_vtable_offset: 0,
                            tag_bit_count: 0,
                            volume_count,
                            aggregate_padding: 0,
                            volumes_offset: 0,
                            kd_tree_offset: 0,
                        },
                        object_bytes,
                    }],
                    simple_primitive_sets: vec![],
                    meshes: vec![],
                    collision_models: vec![],
                    instances: vec![],
                }],
            }],
        }
    }

    fn overlap_test_sphere_body(position: [f32; 3]) -> DynamicCollisionBody {
        let mut body_transform = overlap_test_identity();
        body_transform[12..15].copy_from_slice(&position);
        DynamicCollisionBody {
            model_id: 1,
            transform: body_transform,
            bounding_box_min: [-1.0; 3],
            bounding_box_max: [1.0; 3],
            parts: vec![DynamicCollisionPart {
                transform: overlap_test_identity(),
                volumes: vec![Volume {
                    transform: overlap_test_identity(),
                    vtable_offset: VolumeType::Sphere as u32,
                    shape_words: [0; 3],
                    radius: 1.0,
                    group_id: 0,
                    surface_id: 0,
                    flags: 0,
                }],
                triangles: vec![],
            }],
        }
    }

    #[test]
    fn collision_affine_composition_preserves_native_noncommuting_operand_order() {
        let body = [
            0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 20.0, 30.0, 1.0,
        ];
        let part = [
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 2.0, 3.0, 1.0,
        ];

        assert_eq!(
            compose_collision_affine(body, part).map(f32::to_bits),
            [
                0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 8.0, 21.0, 33.0, 1.0,
            ]
            .map(f32::to_bits)
        );
        assert_eq!(
            compose_collision_affine(part, body).map(f32::to_bits),
            [
                0.0, 0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 11.0, -28.0, 23.0,
                1.0,
            ]
            .map(f32::to_bits)
        );
    }

    #[test]
    fn part_get_transform_preserves_pointer_branch_order_and_copy_bits() {
        let part = std::array::from_fn(|index| {
            f32::from_bits(0x8000_0000_u32.wrapping_add(u32::try_from(index).unwrap()))
        });
        let body = std::array::from_fn(|index| {
            f32::from_bits(0x7fc0_0100_u32.wrapping_add(u32::try_from(index).unwrap()))
        });
        let inverse = [
            0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 2.0, 3.0, 4.0, 1.0,
        ];

        assert_eq!(
            part_get_transform(part, None, Some(&inverse)).map(f32::to_bits),
            part.map(f32::to_bits)
        );
        assert_eq!(
            part_get_transform(part, Some(&body), None).map(f32::to_bits),
            body.map(f32::to_bits)
        );

        let affine_body = [
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 10.0, 20.0, 30.0, 1.0,
        ];
        assert_eq!(
            part_get_transform(part, Some(&affine_body), Some(&inverse)).map(f32::to_bits),
            compose_collision_affine(affine_body, inverse).map(f32::to_bits)
        );
    }

    #[test]
    fn collision_affine_composition_uses_native_fused_rounding_graph() {
        let outer = [
            0.798_635_5,
            0.601_815,
            0.125,
            0.0,
            -0.601_815,
            0.798_635_5,
            -0.375,
            0.0,
            -0.21875,
            0.40625,
            0.8875,
            0.0,
            12345.67,
            -9876.543,
            0.125,
            1.0,
        ];
        let inner = [
            1.0,
            0.03125,
            -0.125,
            0.0,
            0.0625,
            0.920_504_87,
            0.390_731_13,
            0.0,
            -0.333_333_34,
            -0.390_731_13,
            0.920_504_87,
            0.0,
            0.333_333_34,
            -2048.125,
            8192.75,
            1.0,
        ];

        let result = compose_collision_affine(outer, inner);
        assert_eq!(result[6].to_bits(), 0x3c19_f607);
        assert_eq!(result[8].to_bits(), 0xbe6e_00ac);
        assert_eq!(result[12].to_bits(), 0x4638_2976);
        assert_eq!(result[14].to_bits(), 0x45fb_3a3c);
    }

    #[test]
    fn collision_orthonormalize_selects_each_smallest_cyclic_pair() {
        let diagonal = std::f32::consts::FRAC_1_SQRT_2;
        let fixtures = [
            // X/Y is uniquely least correlated; Z is repaired.
            [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, diagonal, diagonal, 0.0, 0.0, 0.0, 0.0,
                0.0, 1.0,
            ],
            // Z/X is uniquely least correlated; Y is repaired.
            [
                1.0, 0.0, 0.0, 0.0, diagonal, 0.0, diagonal, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
                0.0, 1.0,
            ],
            // Y/Z is uniquely least correlated; X is repaired.
            [
                0.0, diagonal, diagonal, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
                0.0, 1.0,
            ],
        ];
        let identity = orthonormalize_test_identity();

        for fixture in fixtures {
            assert_eq!(
                orthonormalize_collision_affine(fixture).map(f32::to_bits),
                identity.map(f32::to_bits)
            );
        }
    }

    #[test]
    fn collision_orthonormalize_matches_finite_host_w_lane_discriminator() {
        let matrix = [
            2.0,
            0.25,
            -0.125,
            0.375,
            0.5,
            3.0,
            0.25,
            -0.625,
            -0.25,
            0.75,
            4.0,
            0.875,
            f32::from_bits(0x40a0_0000),
            f32::from_bits(0xc0c0_0000),
            f32::from_bits(0x8000_0000),
            f32::from_bits(0x40e0_0000),
        ];

        assert_eq!(
            [
                collision_ortho_refined_length_host(matrix[0..4].try_into().unwrap()),
                collision_ortho_refined_length_host(matrix[4..8].try_into().unwrap()),
                collision_ortho_one_refinement_length_host(matrix[8..12].try_into().unwrap(),),
            ]
            .map(f32::to_bits),
            [0x4001_3e74, 0x4043_4e0e, 0x4082_79df]
        );
        assert_eq!(
            orthonormalize_collision_affine(matrix).map(f32::to_bits),
            [
                0x3f7d_3917,
                0x3e12_1133,
                0x3d0f_ac31,
                0xa319_1610,
                0xbe08_b03b,
                0x3f78_f7d9,
                0xbe43_44e8,
                0xb0fa_3157,
                0xbd7b_2452,
                0x3e3c_5b3e,
                0x3f7b_2452,
                0x3e5b_bfc8,
                0x40a0_0000,
                0xc0c0_0000,
                0x8000_0000,
                0x40e0_0000,
            ]
        );
    }

    #[test]
    fn collision_orthonormalize_strict_partial_ties_prefer_xy_then_zx() {
        let fixtures = [
            // |Y.Z| == |Z.X| < |X.Y| chooses Z/X rather than Y/Z.
            [
                1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            // |Y.Z| == |X.Y| < |Z.X| chooses X/Y rather than Y/Z.
            [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            // |Z.X| == |X.Y| < |Y.Z| chooses X/Y rather than Z/X.
            [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            // The all-equal orthogonal tie also retains X/Y.
            orthonormalize_test_identity(),
        ];
        let identity = orthonormalize_test_identity();

        for fixture in fixtures {
            assert_eq!(
                orthonormalize_collision_affine(fixture).map(f32::to_bits),
                identity.map(f32::to_bits)
            );
        }
    }

    #[test]
    fn collision_orthonormalize_repairs_each_exact_zero_axis_after_length_refinement() {
        let fixtures = [
            [
                -0.0, -0.0, -0.0, -0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 4.0, 5.0, 6.0, 1.0,
            ],
            [
                1.0, 0.0, 0.0, 0.0, -0.0, -0.0, -0.0, -0.0, 0.0, 0.0, 1.0, 0.0, 4.0, 5.0, 6.0, 1.0,
            ],
            [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -0.0, -0.0, -0.0, -0.0, 4.0, 5.0, 6.0, 1.0,
            ],
        ];
        let mut expected = orthonormalize_test_identity();
        expected[12..16].copy_from_slice(&fixtures[0][12..16]);

        for (missing_axis, matrix) in fixtures.into_iter().enumerate() {
            assert_eq!(
                collision_ortho_axis_lengths_host([
                    matrix[0..4].try_into().unwrap(),
                    matrix[4..8].try_into().unwrap(),
                    matrix[8..12].try_into().unwrap(),
                ])[missing_axis]
                    .to_bits(),
                0x0000_0000
            );
            assert_eq!(
                orthonormalize_collision_affine(matrix).map(f32::to_bits),
                expected.map(f32::to_bits)
            );
        }
    }

    #[test]
    fn collision_orthonormalize_copies_translation_bits_verbatim() {
        let mut matrix = orthonormalize_test_identity();
        matrix[12..16].copy_from_slice(
            [0x8000_0000, 0x7fc0_1234, 0xff80_0000, 0x3f00_0001]
                .map(f32::from_bits)
                .as_slice(),
        );

        assert_eq!(
            orthonormalize_collision_affine(matrix)[12..16]
                .iter()
                .copied()
                .map(f32::to_bits)
                .collect::<Vec<_>>(),
            [0x8000_0000, 0x7fc0_1234, 0xff80_0000, 0x3f00_0001]
        );
    }

    #[test]
    fn collision_orthonormalize_exposes_host_exception_policy_without_guards() {
        let mut nan_axis = orthonormalize_test_identity();
        nan_axis[0] = f32::NAN;
        assert_eq!(
            orthonormalize_collision_affine(nan_axis).map(f32::to_bits),
            orthonormalize_test_identity().map(f32::to_bits)
        );

        let mut all_zero = [0.0; 16];
        all_zero[12..16].copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        let result = orthonormalize_collision_affine(all_zero);
        assert!(result[..12].iter().all(|value| value.is_nan()));
        assert_eq!(
            result[12..16]
                .iter()
                .copied()
                .map(f32::to_bits)
                .collect::<Vec<_>>(),
            [1.0, 2.0, 3.0, 4.0].map(f32::to_bits)
        );
    }

    #[test]
    fn collision_orthonormalize_rsqrt_keeps_fused_refinement_graph() {
        let value = f32::from_bits(0x06a8_c2d0);
        let estimate0 = value.sqrt().recip();
        let squared0 = estimate0 * estimate0;
        let half0 = estimate0 * 0.5;
        let residual0 = (-value).mul_add(squared0, 1.0);
        let separated_residual0 = 1.0 - value * squared0;

        assert_eq!(estimate0.to_bits(), 0x5bde_f364);
        assert_eq!(squared0.to_bits(), 0x7842_2b09);
        assert_eq!(half0.to_bits(), 0x5b5e_f364);
        assert_eq!(residual0.to_bits(), 0xb30f_92a0);
        assert_eq!(separated_residual0.to_bits(), 0x0000_0000);
        assert_eq!(
            collision_ortho_rsqrt_first_refinement_host(value).to_bits(),
            0x5bde_f364
        );
        assert_eq!(
            collision_ortho_refined_rsqrt_host(value).to_bits(),
            0x5bde_f364
        );
    }

    #[test]
    fn collision_orthonormalize_duplicate_z_length_stops_after_one_refinement() {
        let axis = [f32::from_bits(0x29ad_7174), 0.0, 0.0, 0.0];

        assert_eq!(collision_ortho_dot3_host(axis, axis).to_bits(), 0x13eb_0512);
        assert_eq!(
            collision_ortho_axis_lengths_host([axis, axis, axis]).map(f32::to_bits),
            [0x29ad_7173, 0x29ad_7173, 0x29ad_7174]
        );
    }

    #[test]
    fn simulation_quaternion_matches_source_graph_and_textbook_discriminator() {
        let matrix = [
            f32::from_bits(0x3f2a_9fbe),
            f32::from_bits(0xbea1_55d4),
            f32::from_bits(0x3e68_2e2d),
            f32::from_bits(0x4f12_3456),
            f32::from_bits(0x3e91_6b7a),
            f32::from_bits(0x3f50_4f33),
            f32::from_bits(0xbf01_19e8),
            f32::from_bits(0xcf23_4567),
            f32::from_bits(0xbef2_8130),
            f32::from_bits(0x3ebc_27c1),
            f32::from_bits(0x3f0d_7519),
            f32::from_bits(0x7f00_1234),
            101.0,
            -202.0,
            303.0,
            f32::from_bits(0x7fc0_1234),
        ];
        let candidate = simulation_quaternion_candidate_host(matrix);
        assert_eq!(
            candidate.map(f32::to_bits),
            [0xbe80_2779, 0xbe4d_e9bc, 0xbe30_2540, 0x3f5e_e90a]
        );
        assert_eq!(
            simulation_quaternion_from_orthonormalized_matrix_host(matrix).map(f32::to_bits),
            [0xbe87_c6db, 0xbe5a_2923, 0xbe3a_9f64, 0x3f6c_2b42]
        );

        let x = [matrix[0], matrix[1], matrix[2], matrix[3]];
        let y = [matrix[4], matrix[5], matrix[6], matrix[7]];
        let z = [matrix[8], matrix[9], matrix[10], matrix[11]];
        let trace = (x[0] + y[1]) + z[2];
        let scale = (trace + 1.0).sqrt() * 2.0;
        let textbook = [
            (y[2] - z[1]) / scale,
            (z[0] - x[2]) / scale,
            (x[1] - y[0]) / scale,
            0.25 * scale,
        ];
        assert_eq!(
            textbook.map(f32::to_bits),
            [0xbe80_2779, 0xbe4d_e9bc, 0xbe30_2541, 0x3f5e_e90a]
        );
        assert_ne!(candidate.map(f32::to_bits), textbook.map(f32::to_bits));
    }

    #[test]
    fn joint_candidate_has_one_vector_seed_call_and_no_final_normalization() {
        let mut calls = 0;
        let actual =
            try_simulation_quaternion_candidate(orthonormalize_test_identity(), |magnitudes| {
                calls += 1;
                assert_eq!(magnitudes.map(f32::to_bits), [0x4080_0000, 0, 0, 0]);
                // Deliberately inaccurate seed. The exact rational refinement is
                // 1/4 -> 11/32 -> 7117/16384; candidate W becomes 7117/8192.
                Ok::<_, ()>([0.25, 1.0, 1.0, 1.0])
            })
            .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(actual.map(f32::to_bits), [0, 0, 0, 0x3f5e_6800]);
        assert_ne!(
            actual.map(f32::to_bits),
            simulation_quaternion_from_orthonormalized_matrix_host(orthonormalize_test_identity())
                .map(f32::to_bits)
        );
    }

    #[test]
    fn joint_candidate_estimate_failure_has_no_fallback_or_selected_lane_retry() {
        let mut calls = 0;
        let result =
            try_simulation_quaternion_candidate(orthonormalize_test_identity(), |magnitudes| {
                calls += 1;
                assert_eq!(magnitudes.map(f32::to_bits), [0x4080_0000, 0, 0, 0]);
                Err("vector estimate unavailable")
            });
        assert_eq!(calls, 1);
        assert_eq!(result, Err("vector estimate unavailable"));
    }

    #[test]
    fn simulation_quaternion_non_diagonal_x_y_z_winners_match_oracle() {
        let cases = [
            (
                "x",
                [
                    0x3f4c_cccd,
                    0x3e2e_147b,
                    0xbe6b_851f,
                    0x4f12_3456,
                    0xbe9e_b852,
                    0xbf00_0000,
                    0x3ed1_eb85,
                    0xcf23_4567,
                    0x3e94_7ae1,
                    0xbebd_70a4,
                    0xbf19_999a,
                    0x7f00_1234,
                    0x42ca_0000,
                    0xc34a_0000,
                    0x4397_8000,
                    0x7fc0_1234,
                ],
                [0x3f59_f9e4, 0xbd28_5e26, 0x3c90_50b1, 0x3e6a_8323],
                [0x3f76_e590, 0xbd3e_b4d9, 0x3ca3_766e, 0x3e84_d03c],
            ),
            (
                "y",
                [
                    0xbf00_0000,
                    0x3e2e_147b,
                    0xbe6b_851f,
                    0x4f12_3456,
                    0xbe9e_b852,
                    0x3f4c_cccd,
                    0x3ed1_eb85,
                    0xcf23_4567,
                    0x3e94_7ae1,
                    0xbebd_70a4,
                    0xbf19_999a,
                    0x7f00_1234,
                    0x42ca_0000,
                    0xc34a_0000,
                    0x4397_8000,
                    0x7fc0_1234,
                ],
                [0xbd28_5e26, 0x3f59_f9e4, 0x3c40_6b97, 0x3e1c_576c],
                [0xbd42_6580, 0x3f7b_ac8d, 0x3c5e_2ad9, 0x3e34_82d2],
            ),
            (
                "z",
                [
                    0xbf00_0000,
                    0x3e2e_147b,
                    0xbe6b_851f,
                    0x4f12_3456,
                    0xbe9e_b852,
                    0xbf19_999a,
                    0x3ed1_eb85,
                    0xcf23_4567,
                    0x3e94_7ae1,
                    0xbebd_70a4,
                    0x3f4c_cccd,
                    0x7f00_1234,
                    0x42ca_0000,
                    0xc34a_0000,
                    0x4397_8000,
                    0x7fc0_1234,
                ],
                [0x3c90_50b1, 0x3c40_6b97, 0x3f59_f9e4, 0x3e10_50b4],
                [0x3ca7_2a0f, 0x3c5e_e2bf, 0x3f7c_7ce0, 0x3e27_2a13],
            ),
        ];

        for (winner, matrix_bits, expected_candidate, expected_normalized) in cases {
            let matrix = matrix_bits.map(f32::from_bits);
            assert_eq!(
                simulation_quaternion_candidate_host(matrix).map(f32::to_bits),
                expected_candidate,
                "{winner}-winner candidate"
            );
            assert_eq!(
                simulation_quaternion_from_orthonormalized_matrix_host(matrix).map(f32::to_bits),
                expected_normalized,
                "{winner}-winner normalized result"
            );
        }
    }

    #[test]
    fn simulation_quaternion_strict_ties_prefer_y_then_z() {
        let diagonal = |values: [f32; 3]| {
            [
                values[0], 0.0, 0.0, 11.0, 0.0, values[1], 0.0, 22.0, 0.0, 0.0, values[2], 33.0,
                44.0, 55.0, 66.0, 77.0,
            ]
        };
        let dominant_lane = |matrix| {
            simulation_quaternion_candidate_host(matrix)
                .into_iter()
                .position(|lane| lane != 0.0)
                .expect("a diagonal candidate has one nonzero lane")
        };

        assert_eq!(dominant_lane(diagonal([0.0; 3])), 2);
        assert_eq!(dominant_lane(diagonal([0.25, 0.25, -0.75])), 1);
        assert_eq!(dominant_lane(diagonal([-0.75, 0.25, 0.25])), 2);
        assert_eq!(dominant_lane(diagonal([0.25, -0.75, 0.25])), 2);
    }

    #[test]
    fn simulation_quaternion_ignores_axis_w_and_translation_and_has_no_zero_guard() {
        let identity = orthonormalize_test_identity();
        assert_eq!(
            simulation_quaternion_from_orthonormalized_matrix_host(identity).map(f32::to_bits),
            [0, 0, 0, 1.0_f32.to_bits()]
        );

        let mut noisy = identity;
        noisy[3] = f32::from_bits(0x7fc0_1234);
        noisy[7] = f32::from_bits(0xff80_0000);
        noisy[11] = f32::from_bits(0x4f12_3456);
        noisy[12..16].copy_from_slice(&[
            f32::from_bits(0x7fc0_5678),
            -22.0,
            33.0,
            f32::from_bits(0x8000_0000),
        ]);
        assert_eq!(
            simulation_quaternion_from_orthonormalized_matrix_host(noisy).map(f32::to_bits),
            simulation_quaternion_from_orthonormalized_matrix_host(identity).map(f32::to_bits)
        );

        assert_eq!(
            simulation_quaternion_from_orthonormalized_matrix_host([0.0; 16]).map(f32::to_bits),
            [0, 0, 1.0_f32.to_bits(), 0]
        );
    }

    #[test]
    fn simulation_quaternion_rsqrt_preserves_both_fused_refinements() {
        let value = f32::from_bits(0x68d3_92ce);
        let estimate0 = value.sqrt().recip();
        let square0 = estimate0 * estimate0;
        let half0 = estimate0 * 0.5;
        let split_residual0 = 1.0 - value * square0;
        let split_estimate1 = estimate0 + half0 * split_residual0;
        let split_square1 = split_estimate1 * split_estimate1;
        let split_half1 = split_estimate1 * 0.5;
        let split_residual1 = 1.0 - value * split_square1;
        let defused = split_estimate1 + split_half1 * split_residual1;

        assert_eq!(
            simulation_rsqrt_two_refinements_host(value).to_bits(),
            0x2ac7_1ea9
        );
        assert_eq!(defused.to_bits(), 0x2ac7_1ea8);
    }

    #[test]
    fn simulation_rotated_inertia_matches_scale_first_fused_discriminators() {
        let matrix = [
            0.36,
            0.48,
            -0.8,
            f32::from_bits(0x4f12_3456),
            -0.8,
            0.60,
            0.0,
            f32::from_bits(0xcf23_4567),
            0.48,
            0.64,
            0.60,
            f32::from_bits(0x7f00_1234),
            100.0,
            200.0,
            300.0,
            400.0,
        ];
        let inverse_tensor = [
            f32::from_bits(0x3f33_3333),
            f32::from_bits(0x3fa6_6666),
            f32::from_bits(0x4006_6666),
        ];
        let result = simulation_rotated_inverse_inertia_host(
            matrix,
            inverse_tensor,
            f32::from_bits(0x3eaa_aaab),
        );
        assert_eq!(
            result.full.map(f32::to_bits),
            [0x3fb4_0a28, 0x3e11_7d65, 0x3ece_703a]
        );
        assert_eq!(
            result.split.map(f32::to_bits),
            [0x3f9a_1cac, 0x3fbe_a5f8, 0x3f09_a027]
        );
        assert_eq!(result.inverse_mass.to_bits(), 0x3eaa_aaab);
        assert_ne!(
            result.full.map(f32::to_bits),
            [0x3fb4_0a28, 0x3e11_7d64, 0x3ece_703a]
        );
        assert_ne!(
            result.split.map(f32::to_bits),
            [0x3f9a_1cac, 0x3fbe_a5f8, 0x3f09_a028]
        );

        let right = [matrix[0], matrix[1], matrix[2]];
        let up = [matrix[4], matrix[5], matrix[6]];
        let at = [matrix[8], matrix[9], matrix[10]];
        let pair_first = |row: usize, column: usize| {
            inverse_tensor[2].mul_add(
                at[row] * at[column],
                inverse_tensor[0].mul_add(
                    right[row] * right[column],
                    inverse_tensor[1] * (up[row] * up[column]),
                ),
            )
        };
        let textbook_full = [pair_first(0, 0), pair_first(1, 0), pair_first(2, 0)];
        assert_eq!(
            textbook_full.map(f32::to_bits),
            [0x3fb4_0a28, 0x3e11_7d66, 0x3ece_703b]
        );
        assert_ne!(
            result.full.map(f32::to_bits),
            textbook_full.map(f32::to_bits)
        );

        let mut irrelevant_lanes_changed = matrix;
        irrelevant_lanes_changed[3] = f32::from_bits(0x7fc0_1234);
        irrelevant_lanes_changed[7] = f32::from_bits(0xff80_0000);
        irrelevant_lanes_changed[11] = f32::from_bits(0x7f00_5678);
        irrelevant_lanes_changed[12..16].copy_from_slice(&[-1.0, -2.0, -3.0, -4.0]);
        assert_eq!(
            simulation_rotated_inverse_inertia_host(
                irrelevant_lanes_changed,
                inverse_tensor,
                f32::from_bits(0x3eaa_aaab),
            ),
            result
        );
    }

    #[test]
    fn simulation_rotated_inertia_runs_zero_graph_and_preserves_inverse_mass_bits() {
        let mut matrix = [
            0.36, 0.48, -0.8, 11.0, -0.8, 0.60, 0.0, 22.0, 0.48, 0.64, 0.60, 33.0, 44.0, 55.0,
            66.0, 77.0,
        ];
        let result = simulation_rotated_inverse_inertia_host(
            matrix,
            [0.0, -0.0, 0.0],
            f32::from_bits(0x8000_0000),
        );
        assert_eq!(result.full.map(f32::to_bits), [0; 3]);
        assert_eq!(result.split.map(f32::to_bits), [0; 3]);
        assert_eq!(result.inverse_mass.to_bits(), 0x8000_0000);

        matrix[3] = f32::from_bits(0x7fc0_1234);
        matrix[7] = f32::from_bits(0xff80_0000);
        matrix[11] = f32::from_bits(0x7f00_1234);
        matrix[12..16].copy_from_slice(&[-1.0, -2.0, -3.0, -4.0]);
        let noisy = simulation_rotated_inverse_inertia_host(
            matrix,
            [0.0, -0.0, 0.0],
            f32::from_bits(0x8000_0000),
        );
        assert_eq!(noisy.full.map(f32::to_bits), result.full.map(f32::to_bits));
        assert_eq!(
            noisy.split.map(f32::to_bits),
            result.split.map(f32::to_bits)
        );
        assert_eq!(noisy.inverse_mass.to_bits(), result.inverse_mass.to_bits());
    }

    #[test]
    fn part_attachment_uses_rigid_transpose_inverse_not_general_inverse() {
        let part = [
            1.25, -0.5, 0.75, 3.0, 0.25, 2.0, -1.5, -2.0, -0.75, 0.5, 1.125, 0.5, 10.0, -20.0,
            30.0, 4.0,
        ];
        let inverse_body = [
            2.0, 0.25, -0.5, 9.0, 0.75, 1.5, 0.125, -8.0, -0.25, 0.5, 0.8, 7.0, 3.0, -4.0, 5.0,
            -0.0,
        ];
        let rigid_inverse = part_rigid_inverse_by_transpose(inverse_body);

        assert_eq!(
            rigid_inverse.map(f32::to_bits),
            [
                0x4000_0000,
                0x3f40_0000,
                0xbe80_0000,
                0x0000_0000,
                0x3e80_0000,
                0x3fc0_0000,
                0x3f00_0000,
                0x0000_0000,
                0xbf00_0000,
                0x3e00_0000,
                0x3f4c_cccd,
                0x0000_0000,
                0xc020_0000,
                0x4048_0000,
                0xbfa0_0000,
                0x0000_0000,
            ]
        );
        assert_eq!(
            part_attachment_transform(part, Some(&inverse_body)).map(f32::to_bits),
            [
                0x4038_0000,
                0x3ec0_0000,
                0x3dc0_0000,
                0x408c_0000,
                0x3ea0_0000,
                0x4048_0000,
                0xbfc0_0000,
                0xc000_0000,
                0xbf98_cccd,
                0x3f66_6666,
                0x3eac_cccd,
                0xbfac_cccd,
                0x4109_8000,
                0xc152_0000,
                0x41b0_4000,
                0xc126_0000,
            ]
        );

        let general =
            part_attachment_compose_host(part, inverse_collision_affine_reset(inverse_body));
        assert_eq!(
            general.map(f32::to_bits),
            [
                0x3ed4_9b34,
                0xbf1e_6be8,
                0x3fb0_d8bd,
                0x402c_c573,
                0x3ce3_c268,
                0x3fd8_f7f6,
                0xbff8_ff4f,
                0xc03d_6ac2,
                0xbf53_3a8c,
                0xbf20_b7ad,
                0x4043_71a6,
                0x4054_606d,
                0x414f_db44,
                0xc103_9662,
                0x4033_840e,
                0xc202_1dd9,
            ]
        );
        assert_ne!(
            general.map(f32::to_bits),
            part_attachment_transform(part, Some(&inverse_body)).map(f32::to_bits)
        );
    }

    #[test]
    fn part_rigid_inverse_uses_arithmetic_zero_minus_for_signed_zero() {
        let inverse_body = [
            0x0000_0000,
            0x8000_0000,
            0x0000_0000,
            0x0000_0000,
            0x0000_0000,
            0x0000_0000,
            0x0000_0000,
            0x8000_0000,
            0x0000_0000,
            0x0000_0000,
            0x0000_0000,
            0x0000_0000,
            0x0000_0000,
            0x8000_0000,
            0x0000_0000,
            0x8000_0000,
        ]
        .map(f32::from_bits);
        let arithmetic = part_rigid_inverse_by_transpose(inverse_body);
        assert_eq!(
            arithmetic.map(f32::to_bits),
            [
                0x0000_0000,
                0x0000_0000,
                0x0000_0000,
                0x0000_0000,
                0x8000_0000,
                0x0000_0000,
                0x0000_0000,
                0x0000_0000,
                0x0000_0000,
                0x0000_0000,
                0x0000_0000,
                0x0000_0000,
                0x0000_0000,
                0x0000_0000,
                0x0000_0000,
                0x0000_0000,
            ]
        );

        assert_eq!(
            inverse_body[12..16]
                .iter()
                .copied()
                .map(|value| (0.0 - value).to_bits())
                .collect::<Vec<_>>(),
            [0x0000_0000; 4]
        );
        assert_eq!(
            inverse_body[12..16]
                .iter()
                .copied()
                .map(|value| value.to_bits() ^ 0x8000_0000)
                .collect::<Vec<_>>(),
            [0x8000_0000, 0x0000_0000, 0x8000_0000, 0x0000_0000]
        );

        let xor_negative_translation: [f32; 4] = std::array::from_fn(|lane| {
            f32::from_bits(inverse_body[12 + lane].to_bits() ^ 0x8000_0000)
        });
        let wrong_xor_translation: [f32; 4] = std::array::from_fn(|lane| {
            let z = xor_negative_translation[2] * arithmetic[8 + lane];
            let zy = xor_negative_translation[1].mul_add(arithmetic[4 + lane], z);
            xor_negative_translation[0].mul_add(arithmetic[lane], zy)
        });
        assert_eq!(
            wrong_xor_translation.map(f32::to_bits),
            [0x8000_0000, 0x0000_0000, 0x0000_0000, 0x0000_0000]
        );
    }

    #[test]
    fn part_rigid_inverse_translation_reduces_z_then_fused_y_then_x() {
        let inverse_body = [
            0x3f99_5913,
            0x3f91_0b56,
            0x3fb6_cabd,
            0x3ff3_8cf8,
            0x3faf_ebf7,
            0x3f81_58ea,
            0x3fc7_b341,
            0x3fd7_f3ac,
            0xbfc8_db1b,
            0xbf91_d5be,
            0xbf8e_8205,
            0x3ff6_8ba0,
            0x3fb5_ca7f,
            0x3fbf_d5d2,
            0x3fe0_7b09,
            0xbfbb_c8d4,
        ]
        .map(f32::from_bits);
        let native = part_rigid_inverse_by_transpose(inverse_body);
        assert_eq!(
            <[f32; 4]>::try_from(&native[12..16])
                .unwrap()
                .map(f32::to_bits),
            [0xc0bc_ef90, 0xc0c6_7b98, 0x40bc_702a, 0x8000_0000]
        );

        let columns: [[f32; 4]; 4] =
            std::array::from_fn(|column| std::array::from_fn(|row| inverse_body[column * 4 + row]));
        let basis: [[f32; 4]; 3] = std::array::from_fn(|column| {
            [
                columns[0][column],
                columns[1][column],
                columns[2][column],
                0.0,
            ]
        });
        let negative_translation = columns[3].map(|value| 0.0 - value);
        let wrong_xyz: [f32; 4] = std::array::from_fn(|lane| {
            let x = negative_translation[0] * basis[0][lane];
            let xy = negative_translation[1].mul_add(basis[1][lane], x);
            negative_translation[2].mul_add(basis[2][lane], xy)
        });
        assert_eq!(
            wrong_xyz.map(f32::to_bits),
            [0xc0bc_ef91, 0xc0c6_7b98, 0x40bc_702a, 0x8000_0000]
        );
    }

    #[test]
    fn part_attachment_translation_preserves_fused_full_lane_graph() {
        let part = [
            0x3fc3_2777,
            0xbfb0_5e6a,
            0x3fdb_fac1,
            0x3fdc_952c,
            0xbfde_0e9b,
            0xbfaa_733e,
            0xbf8f_8185,
            0xbfd0_8520,
            0xbfb0_75ff,
            0xbfcf_8b52,
            0x3ffd_b289,
            0x3fd4_9a54,
            0x3fec_41a3,
            0x3f80_3aa6,
            0xbf86_11cd,
            0x3fda_88c8,
        ]
        .map(f32::from_bits);
        let inverse_body = [
            0x3f80_0000,
            0,
            0,
            0,
            0,
            0x3f80_0000,
            0,
            0,
            0,
            0,
            0x3f80_0000,
            0,
            0x3f8f_9587,
            0xbffc_553a,
            0xbfde_6351,
            0x3f80_0000,
        ]
        .map(f32::from_bits);
        let result = part_attachment_transform(part, Some(&inverse_body));
        assert_eq!(
            result.map(f32::to_bits),
            [
                0x3fc3_2777,
                0xbfb0_5e6a,
                0x3fdb_fac1,
                0x3fdc_952c,
                0xbfde_0e9b,
                0xbfaa_733e,
                0xbf8f_8185,
                0xbfd0_8520,
                0xbfb0_75ff,
                0xbfcf_8b52,
                0x3ffd_b289,
                0x3fd4_9a54,
                0xc0b5_bfa7,
                0xc039_4472,
                0xbfde_f4f6,
                0xbf0d_2fb9,
            ]
        );

        let inverse = part_rigid_inverse_by_transpose(inverse_body);
        let part_columns: [[f32; 4]; 4] =
            std::array::from_fn(|column| std::array::from_fn(|row| part[column * 4 + row]));
        let separated: [f32; 4] = std::array::from_fn(|lane| {
            let x = inverse[12] * part_columns[0][lane] + part_columns[3][lane];
            let xy = inverse[13] * part_columns[1][lane] + x;
            inverse[14] * part_columns[2][lane] + xy
        });
        assert_eq!(
            separated.map(f32::to_bits),
            [0xc0b5_bfa7, 0xc039_4472, 0xbfde_f4fa, 0xbf0d_2fb8]
        );
    }

    #[test]
    fn part_attachment_without_inverse_returns_every_input_bit() {
        let part = [
            0x8000_0000,
            0x7fc0_1234,
            0xffc0_5678,
            0x0000_0001,
            0x7f80_0000,
            0xff80_0000,
            0x3f80_0000,
            0xbf80_0000,
            0x7f7f_ffff,
            0xff7f_ffff,
            0x0080_0000,
            0x8080_0000,
            0x0000_0000,
            0x8000_0000,
            0x3f00_0001,
            0xbf00_0001,
        ]
        .map(f32::from_bits);

        assert_eq!(
            part_attachment_transform(part, None).map(f32::to_bits),
            part.map(f32::to_bits)
        );
    }

    #[test]
    fn effective_part_body_state_changes_only_active_flagged_parts() {
        let cases = [(4, 0), (4, 1), (4, 2), (4, 3), (1, 1), (2, 1), (7, 1)];
        assert_eq!(
            cases.map(|(requested, flags)| effective_part_body_state(requested, flags)),
            [4, 1, 4, 1, 1, 2, 7]
        );
    }

    #[test]
    fn collision_reset_inverse_preserves_native_cofactor_w_and_translation_order() {
        let matrix = [
            2.0, 1.0, 0.0, 0.0, 0.0, 2.0, 1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 5.0, 6.0, 7.0, 1.0,
        ];

        assert_eq!(
            inverse_collision_affine_reset(matrix).map(f32::to_bits),
            [
                0.5, -0.25, 0.125, -0.25, 0.0, 0.5, -0.25, 0.5, 0.0, 0.0, 0.5, 0.0, -2.5, -1.75,
                -2.625, -1.75,
            ]
            .map(f32::to_bits)
        );
    }

    #[test]
    fn collision_reset_inverse_matches_finite_bit_discriminator() {
        let matrix = [
            0x3c98_c4b6,
            0x3c57_749d,
            0xbeab_6d58,
            0xc24c_90d7,
            0xc106_f84a,
            0xbff1_0b21,
            0x3e54_aa0c,
            0x3da8_5dfb,
            0x3e80_0000,
            0xbf00_0000,
            0x4000_0000,
            0x3eaa_55cc,
            0x40a4_0000,
            0xc0c8_0000,
            0x40f0_0000,
            0xc110_0000,
        ]
        .map(f32::from_bits);

        assert_eq!(
            inverse_collision_affine_reset(matrix).map(f32::to_bits),
            [
                0x4025_9578,
                0xbdcc_263b,
                0x3ee3_0fdc,
                0xbdcc_263b,
                0xc13f_4746,
                0xbdaf_0f3e,
                0xbfff_09c4,
                0xbdaf_0f3e,
                0xc053_f9f5,
                0xbc11_f840,
                0xbd5b_5dea,
                0xbc11_f840,
                0xc27c_8c13,
                0x3d31_fad1,
                0xc165_2feb,
                0x3d31_fad1,
            ]
        );
    }

    #[test]
    fn collision_vmx_cross_keeps_fused_negative_multiply_subtract_and_w_residual() {
        let left = [0x3c98_c4b6, 0x3c57_749d, 0xbeab_6d58, 0xc24c_90d7].map(f32::from_bits);
        let right = [0xc106_f84a, 0xbff1_0b21, 0x3e54_aa0c, 0x3da8_5dfb].map(f32::from_bits);

        let fused = collision_vmx_cross_host(left, right).map(f32::to_bits);
        let left_yzxw = [left[1], left[2], left[0], left[3]];
        let right_yzxw = [right[1], right[2], right[0], right[3]];
        let products: [f32; 4] = std::array::from_fn(|lane| left[lane] * right_yzxw[lane]);
        let separated_differences: [f32; 4] =
            std::array::from_fn(|lane| products[lane] - left_yzxw[lane] * right[lane]);
        let separated = [
            separated_differences[1],
            separated_differences[2],
            separated_differences[0],
            separated_differences[3],
        ]
        .map(f32::to_bits);

        assert_eq!(fused, [0xbf20_b668, 0x4034_838b, 0x3d9b_4419, 0x3410_3b9a]);
        assert_eq!(
            separated,
            [0xbf20_b668, 0x4034_838b, 0x3d9b_441a, 0x0000_0000]
        );
    }

    #[test]
    fn collision_reset_reciprocal_keeps_fused_newton_graph_against_separate_rounding() {
        let value = f32::from_bits(0x4c7f_ffff);
        let estimate0 = value.recip();
        let residual0 = (-estimate0).mul_add(value, 1.0);
        let estimate1 = estimate0.mul_add(residual0, estimate0);
        let residual1 = (-estimate1).mul_add(value, 1.0);
        let estimate2 = estimate1.mul_add(residual1, estimate1);
        let separately_rounded_product0 = estimate0 * residual0;
        let separately_rounded_estimate1 = separately_rounded_product0 + estimate0;

        assert_eq!(estimate0.to_bits(), 0x3280_0001);
        assert_eq!(residual0.to_bits(), 0xb37f_fffe);
        assert_eq!(estimate1.to_bits(), 0x3280_0001);
        assert_eq!(residual1.to_bits(), 0xb37f_fffe);
        assert_eq!(estimate2.to_bits(), 0x3280_0001);
        assert_eq!(
            refined_collision_reset_reciprocal(value).to_bits(),
            0x3280_0001
        );
        assert_eq!(separately_rounded_estimate1.to_bits(), 0x3280_0000);
        // The correctly rounded host seed makes both fused refinements
        // observationally stable for this fixture, while explicitly defusing
        // the first step changes one bit. Both source steps remain required by
        // the recovered graph; a native `vrefp` seed can make them significant.
    }

    #[test]
    fn collision_reset_inverse_composes_back_to_identity_for_ordinary_affine_input() {
        let matrix = [
            2.0, 1.0, 0.0, 0.0, 0.0, 2.0, 1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 5.0, 6.0, 7.0, 1.0,
        ];
        let result = compose_collision_affine(matrix, inverse_collision_affine_reset(matrix));
        let identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];

        assert_eq!(result.map(f32::to_bits), identity.map(f32::to_bits));
    }

    #[test]
    fn collision_reset_inverse_exposes_singular_and_nan_host_policy_without_branching() {
        let identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let mut singular = identity;
        singular[10] = 0.0;
        assert!(
            inverse_collision_affine_reset(singular)
                .into_iter()
                .all(f32::is_nan)
        );

        let mut with_nan = identity;
        with_nan[0] = f32::NAN;
        assert!(
            inverse_collision_affine_reset(with_nan)
                .into_iter()
                .all(f32::is_nan)
        );
    }

    #[test]
    fn collision_aabb_projection_uses_native_y_then_fused_xz_order() {
        let outer = [
            0.798_635_5,
            0.601_815,
            0.125,
            0.0,
            -0.601_815,
            0.798_635_5,
            -0.375,
            0.0,
            -0.21875,
            0.40625,
            0.8875,
            0.0,
            12345.67,
            -9876.543,
            0.125,
            1.0,
        ];
        let inner = [
            1.0,
            0.03125,
            -0.125,
            0.0,
            0.0625,
            0.920_504_87,
            0.390_731_13,
            0.0,
            -0.333_333_34,
            -0.390_731_13,
            0.920_504_87,
            0.0,
            0.333_333_34,
            -2048.125,
            8192.75,
            1.0,
        ];
        let mut transform = compose_collision_affine(outer, inner);
        transform[12..15].fill(0.0);
        let half = [
            f32::from_bits(0x4300_9fd2),
            f32::from_bits(0x3c6e_1d02),
            f32::from_bits(0x3c77_21ab),
        ];

        let (minimum, maximum) =
            transform_collision_aabb(half.map(|value| -value), half, transform);
        assert_eq!(minimum[1].to_bits(), 0xc294_3444);
        assert_eq!(minimum[2].to_bits(), 0xbea1_89c4);
        assert_eq!(maximum[1].to_bits(), 0x4294_3444);
        assert_eq!(maximum[2].to_bits(), 0x3ea1_89c4);
    }

    #[test]
    fn living_world_ids_start_at_the_recovered_global_counter() {
        let mut allocator = LivingWorldEntityIdAllocator::default();

        assert_eq!(
            allocator.allocate(LivingWorldEntityType::DYNAMIC_OBJECT),
            0x0500_0001
        );
        assert_eq!(
            allocator.allocate(LivingWorldEntityType::DYNAMIC_OBJECT),
            0x0500_0002
        );
        assert_eq!(allocator.next_counter(), 3);
    }

    #[test]
    fn living_world_counter_is_shared_across_entity_types() {
        let mut allocator = LivingWorldEntityIdAllocator::default();

        assert_eq!(
            allocator.allocate(LivingWorldEntityType::from_raw(2)),
            0x0200_0001
        );
        assert_eq!(
            allocator.allocate(LivingWorldEntityType::DYNAMIC_OBJECT),
            0x0500_0002
        );
    }

    #[test]
    fn living_world_id_masks_low_bits_and_wraps_the_full_counter() {
        let mut allocator = LivingWorldEntityIdAllocator::from_next_counter(u32::MAX);

        assert_eq!(
            allocator.allocate(LivingWorldEntityType::DYNAMIC_OBJECT),
            0x05ff_ffff
        );
        assert_eq!(allocator.next_counter(), 0);
        assert_eq!(
            allocator.allocate(LivingWorldEntityType::DYNAMIC_OBJECT),
            0x0500_0000
        );
        assert_eq!(allocator.next_counter(), 1);
    }

    #[test]
    fn living_world_memstore_allocates_ascending_slots_then_exhausts() {
        let mut slots = LivingWorldMemStoreSlots::<3>::new();

        assert_eq!(slots.allocate(), Some(0));
        assert_eq!(slots.allocate(), Some(1));
        assert_eq!(slots.allocate(), Some(2));
        assert_eq!(slots.allocate(), None);
        assert_eq!(slots.available(), 0);
    }

    #[test]
    fn living_world_memstore_reuses_freed_slots_lifo() {
        let mut slots = LivingWorldMemStoreSlots::<4>::new();
        assert_eq!(slots.allocate(), Some(0));
        assert_eq!(slots.allocate(), Some(1));
        assert_eq!(slots.allocate(), Some(2));

        assert!(slots.free(1));
        assert!(slots.free(0));
        assert_eq!(slots.allocate(), Some(0));
        assert_eq!(slots.allocate(), Some(1));
        assert_eq!(slots.allocate(), Some(3));
    }

    #[test]
    fn dmo_store_exhaustion_does_not_consume_a_global_entity_id() {
        let mut entities = DynamicObjectEntityAllocator::default();
        let mut ids = LivingWorldEntityIdAllocator::default();
        let allocations = (0..DYNAMIC_OBJECT_ENTITY_CAPACITY)
            .map(|_| entities.allocate(&mut ids).expect("fixed DMO slot"))
            .collect::<Vec<_>>();

        assert_eq!(allocations[0].entity_slot, 0);
        assert_eq!(allocations[0].core_data_slot, 0);
        assert_eq!(allocations[0].collision_slot, 0);
        assert_eq!(allocations[0].object_id, 0x0500_0001);
        assert_eq!(allocations[49].entity_slot, 49);
        assert_eq!(allocations[49].object_id, 0x0500_0032);
        assert_eq!(entities.allocate(&mut ids), None);
        assert_eq!(ids.next_counter(), 51);

        assert!(entities.free(allocations[17]));
        assert_eq!(
            entities.allocate(&mut ids),
            Some(DynamicObjectEntityAllocation {
                object_id: 0x0500_0033,
                entity_slot: 17,
                core_data_slot: 17,
                collision_slot: 17,
            })
        );
    }

    #[test]
    fn dmo_reference_cull_order_uses_pointer_buckets_not_request_order() {
        let allocation = |entity_slot, object_id| DynamicObjectEntityAllocation {
            object_id,
            entity_slot,
            core_data_slot: entity_slot,
            collision_slot: entity_slot,
        };
        let pending = [allocation(1, 11), allocation(0, 10), allocation(2, 12)];

        assert_eq!(
            dynamic_object_reference_cull_order(&pending, 0),
            Some(vec![10, 12, 11])
        );
        assert_eq!(
            dynamic_object_reference_cull_order(&pending, 2),
            Some(vec![11, 10, 12])
        );
    }

    #[test]
    fn dmo_reference_cull_map_suppresses_duplicate_entity_pointers() {
        let allocation = DynamicObjectEntityAllocation {
            object_id: 0x0500_0010,
            entity_slot: 7,
            core_data_slot: 7,
            collision_slot: 7,
        };

        assert_eq!(
            dynamic_object_reference_cull_order(&[allocation, allocation], 0x1000_0000),
            Some(vec![allocation.object_id])
        );
    }

    #[test]
    fn living_world_cull_request_detaches_only_once_before_update() {
        let mut lifetime = LivingWorldEntityLifetime::default();

        assert!(lifetime.request_cull());
        assert_eq!(lifetime, LivingWorldEntityLifetime::CullRequested);
        assert!(!lifetime.request_cull());
    }

    #[test]
    fn living_world_cull_update_destroys_only_pending_entities() {
        let mut active = LivingWorldEntityLifetime::Active;
        let mut pending = LivingWorldEntityLifetime::CullRequested;

        assert!(!active.finish_cull_update());
        assert!(pending.finish_cull_update());
        assert_eq!(pending, LivingWorldEntityLifetime::Destroyed);
        assert!(!pending.finish_cull_update());
        assert!(!pending.request_cull());
    }

    #[test]
    fn counts_and_caps_runtime_owned_mesh_overlap_results() {
        let triangle = MeshTriangle {
            cluster_index: 0,
            unit_stream_offset: 0,
            vertices: [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
            group_id: None,
            surface_id: Some(0x1234),
        };
        let mut scene = CollisionScene {
            location_name: "test".to_owned(),
            focus: [0.0; 3],
            stream_focus: StreamFocusState::new([0.0; 3], false),
            collections: vec![CollisionCollection {
                id: 7,
                name: "test".to_owned(),
                position: [0.0; 3],
                distance_xz: 0.0,
                arenas: vec![CollisionArena {
                    id: 1,
                    grind_assets: vec![],
                    aggregate_roots: vec![],
                    primitive_sets: vec![],
                    simple_primitive_sets: vec![],
                    meshes: vec![CollisionMesh {
                        dictionary_index: 0,
                        bounding_box_min: [0.0; 3],
                        bounding_box_max: [1.0; 3],
                        triangles: vec![triangle; 2],
                        clustered_mesh: ClusteredMesh {
                            bounding_box_min: [0.0; 4],
                            bounding_box_max: [1.0; 4],
                            aggregate_vtable_offset: 0,
                            tag_bit_count: 0,
                            volume_count: 0,
                            aggregate_padding: 0,
                            kd_tree_offset: 96,
                            cluster_data_offset: 144,
                            vertex_compression_granularity: 1.0,
                            cluster_flags: 0,
                            cluster_count: 0,
                            maximum_cluster_count: 0,
                            unit_count: 0,
                            maximum_unit_count: 0,
                            allocation_size: 0,
                            group_id_size: 0,
                            surface_id_size: 0,
                            default_group_id: 0,
                            default_surface_id: 0,
                            default_edge_angle: 0,
                        },
                        object_bytes: vec![],
                    }],
                    collision_models: vec![],
                    instances: vec![],
                }],
            }],
        };

        assert_eq!(
            scene.counts(),
            CollisionSceneCounts {
                arenas: 1,
                meshes: 1,
                triangles: 2,
                ..CollisionSceneCounts::default()
            }
        );

        let nearby = scene
            .triangles_overlapping_aabb([0.1, -0.1, 0.1], [0.3, 0.1, 0.3], 1)
            .unwrap();
        assert_eq!(nearby.len(), 1);
        assert_eq!(nearby[0].surface_id, 0x1234);

        assert_eq!(
            scene
                .triangles_overlapping_aabb([0.1, -0.1, 0.1], [0.3, 0.1, 0.3], 0)
                .unwrap(),
            []
        );
    }

    #[test]
    fn mapped_thin_hits_follow_line_direction_on_equal_time_ties() {
        let bytes = mapped_test_kd_bytes(0, (1, 0), (1, 1), [0.0; 2]);
        let sphere = mapped_test_sphere([0.0; 3], 1.0, 42);
        let scene = mapped_test_scene(vec![sphere.clone(), sphere], bytes);

        let forward = scene
            .thin_segment_hits([-3.0, 0.0, 0.0], [3.0, 0.0, 0.0])
            .unwrap();
        let reverse = scene
            .thin_segment_hits([3.0, 0.0, 0.0], [-3.0, 0.0, 0.0])
            .unwrap();

        assert_eq!(forward.len(), 2);
        assert_eq!(reverse.len(), 2);
        assert_eq!(
            forward[0].line_parameter.to_bits(),
            forward[1].line_parameter.to_bits()
        );
        assert_eq!(
            reverse[0].line_parameter.to_bits(),
            reverse[1].line_parameter.to_bits()
        );
        assert_eq!(
            forward.iter().map(|hit| hit.source).collect::<Vec<_>>(),
            [
                CollisionHitSource::Primitive {
                    collection_index: 0,
                    arena_index: 0,
                    set_index: 0,
                    volume_index: 0,
                },
                CollisionHitSource::Primitive {
                    collection_index: 0,
                    arena_index: 0,
                    set_index: 0,
                    volume_index: 1,
                },
            ]
        );
        assert_eq!(
            reverse.iter().map(|hit| hit.source).collect::<Vec<_>>(),
            [
                CollisionHitSource::Primitive {
                    collection_index: 0,
                    arena_index: 0,
                    set_index: 0,
                    volume_index: 1,
                },
                CollisionHitSource::Primitive {
                    collection_index: 0,
                    arena_index: 0,
                    set_index: 0,
                    volume_index: 0,
                },
            ]
        );
    }

    #[test]
    fn trajectory_mapped_sweep_keeps_the_existing_aabb_candidate_route() {
        // Coincident primitives make candidate order observable through the
        // strict nearest-hit tie. Both child bounds cover both unit spheres.
        // This distinguishes routes without depending on erroneous KD padding.
        let bytes = mapped_test_kd_bytes(0, (1, 0), (1, 1), [1.0, -1.0]);
        let volumes = vec![
            mapped_test_sphere([0.0; 3], 1.0, 42),
            mapped_test_sphere([0.0; 3], 1.0, 84),
        ];
        let mut scene = mapped_test_scene(volumes, bytes);
        let start = [3.0_f32, 0.0, 0.0];
        let end = [-3.0_f32, 0.0, 0.0];
        let radius = 0.5_f32;
        let set = &scene.collections[0].arenas[0].primitive_sets[0];
        let minimum = std::array::from_fn(|axis| start[axis].min(end[axis]) - radius);
        let maximum = std::array::from_fn(|axis| start[axis].max(end[axis]) + radius);

        assert_eq!(
            set.mapped_array
                .candidate_volume_indices_for_line(&set.object_bytes, start, end, radius)
                .unwrap(),
            [1, 0]
        );
        assert_eq!(
            set.mapped_array
                .candidate_volume_indices_for_aabb(&set.object_bytes, minimum, maximum)
                .unwrap(),
            [0, 1]
        );

        let hit = scene
            .sweep_sphere_segment(start, end, radius)
            .unwrap()
            .expect("the unchanged mapped AABB route must retain this swept hit");
        assert_eq!(hit.surface_id, 42);
    }

    #[test]
    fn trajectory_sweep_includes_mapped_sphere_primitives() {
        let kd_bytes = mapped_test_branchless_kd_bytes(1);
        let volume = Volume {
            transform: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            vtable_offset: VolumeType::Sphere as u32,
            shape_words: [0; 3],
            radius: 1.0,
            group_id: 0,
            surface_id: 42,
            flags: 0,
        };
        let mut scene = CollisionScene {
            location_name: "test".to_owned(),
            focus: [0.0; 3],
            stream_focus: StreamFocusState::new([0.0; 3], false),
            collections: vec![CollisionCollection {
                id: 7,
                name: "test".to_owned(),
                position: [0.0; 3],
                distance_xz: 0.0,
                arenas: vec![CollisionArena {
                    id: 1,
                    grind_assets: vec![],
                    aggregate_roots: vec![],
                    primitive_sets: vec![CollisionPrimitiveSet {
                        dictionary_index: 0,
                        bounding_box_min: [-1.0; 3],
                        bounding_box_max: [1.0; 3],
                        volumes: vec![volume],
                        mapped_array: KdTreeMappedArray {
                            bounding_box_min: [-1.0, -1.0, -1.0, 0.0],
                            bounding_box_max: [1.0, 1.0, 1.0, 0.0],
                            aggregate_vtable_offset: 0,
                            tag_bit_count: 0,
                            volume_count: 1,
                            aggregate_padding: 0,
                            volumes_offset: 0,
                            kd_tree_offset: 0,
                        },
                        object_bytes: kd_bytes,
                    }],
                    simple_primitive_sets: vec![],
                    meshes: vec![],
                    collision_models: vec![],
                    instances: vec![],
                }],
            }],
        };

        let hit = scene
            .sweep_sphere_segment([-3.0, 0.0, 0.0], [3.0, 0.0, 0.0], 0.5)
            .unwrap()
            .unwrap();
        assert!((hit.position[0] + 1.5).abs() < f32::EPSILON);
        assert!((hit.normal[0] + 1.0).abs() < f32::EPSILON);
        assert_eq!(hit.surface_id, 42);

        let mut batch = GrindGeometryProbeBatch::default();
        batch.lines[0].high_point = [-3.0, 0.0, 0.0];
        batch.lines[0].low_point = [3.0, 0.0, 0.0];
        let collisions = scene.static_grind_geometry_batch(&batch).unwrap();
        assert!(collisions[0].hit);
        assert_eq!(collisions[0].surface_id, 42);
    }

    #[test]
    fn dynamic_geometry_batch_composes_body_part_and_volume_transforms() {
        let identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let mut body_transform = identity;
        body_transform[12] = 5.0;
        let mut collision_identity = identity;
        collision_identity[15] = 0.0;
        let body = DynamicCollisionBody {
            model_id: 7,
            transform: body_transform,
            bounding_box_min: [-1.0; 3],
            bounding_box_max: [1.0; 3],
            parts: vec![DynamicCollisionPart {
                transform: collision_identity,
                volumes: vec![Volume {
                    transform: collision_identity,
                    vtable_offset: VolumeType::Sphere as u32,
                    shape_words: [0; 3],
                    radius: 1.0,
                    group_id: 0,
                    surface_id: 23,
                    flags: 0,
                }],
                triangles: vec![],
            }],
        };
        let mut batch = GrindGeometryProbeBatch::default();
        batch.lines[0].high_point = [2.0, 0.0, 0.0];
        batch.lines[0].low_point = [8.0, 0.0, 0.0];

        let collisions = dynamic_grind_geometry_batch(&[body], &batch);
        assert!(collisions[0].hit);
        assert!((collisions[0].line_parameter - 1.0 / 3.0).abs() < 1.0e-6);
        for (actual, expected) in collisions[0].position.into_iter().zip([4.0, 0.0, 0.0]) {
            assert!((actual - expected).abs() < 1.0e-6);
        }
        for (actual, expected) in collisions[0].normal.into_iter().zip([-1.0, 0.0, 0.0]) {
            assert!((actual - expected).abs() < 1.0e-6);
        }
        assert_eq!(collisions[0].surface_id, 23);
    }

    #[test]
    fn dynamic_body_overlap_preserves_dmo_and_vehicle_padding_boundaries() {
        let candidate = overlap_test_sphere_body([0.0; 3]);
        let touching_dmo = overlap_test_sphere_body([1.95, 0.0, 0.0]);
        let beyond_dmo =
            overlap_test_sphere_body([f32::from_bits(1.95_f32.to_bits() + 1), 0.0, 0.0]);
        let touching_vehicle = overlap_test_sphere_body([2.0, 0.0, 0.0]);

        assert!(candidate.overlaps_dynamic_body(&touching_dmo, DYNAMIC_OBJECT_OVERLAP_PADDING));
        assert!(!candidate.overlaps_dynamic_body(&beyond_dmo, DYNAMIC_OBJECT_OVERLAP_PADDING));
        assert!(
            !candidate.overlaps_dynamic_body(&touching_vehicle, DYNAMIC_OBJECT_OVERLAP_PADDING)
        );
        assert!(candidate.overlaps_dynamic_body(&touching_vehicle, VEHICLE_OVERLAP_PADDING));
    }

    #[test]
    fn overlap_owner_lists_drop_zero_deduplicate_and_keep_pool_order() {
        let candidate = overlap_test_sphere_body([0.0; 3]);
        let live_bodies = [0, 7, 7, 8]
            .into_iter()
            .map(|object_id| OwnedDynamicCollisionBody {
                object_id,
                body: overlap_test_sphere_body([0.0; 3]),
            })
            .collect::<Vec<_>>();
        assert_eq!(dynamic_object_overlap_ids(&candidate, &live_bodies), [7, 8]);

        let many_live_bodies = (1_u32..=70)
            .map(|object_id| OwnedDynamicCollisionBody {
                object_id,
                body: overlap_test_sphere_body([0.0; 3]),
            })
            .collect::<Vec<_>>();
        let dynamic_ids = dynamic_object_overlap_ids(&candidate, &many_live_bodies);
        assert_eq!(dynamic_ids.len(), DYNAMIC_OBJECT_OVERLAP_ID_CAPACITY);
        assert_eq!(dynamic_ids.first(), Some(&1));
        assert_eq!(dynamic_ids.last(), Some(&50));
        let vehicle_ids = vehicle_overlap_ids(&candidate, &many_live_bodies);
        assert_eq!(vehicle_ids.len(), VEHICLE_OVERLAP_ID_CAPACITY);
        assert_eq!(vehicle_ids.first(), Some(&1));
        assert_eq!(vehicle_ids.last(), Some(&24));
    }

    #[test]
    fn world_overlap_snapshot_feeds_admission_priority_and_cull_order() {
        let priority = |object_id, value| DynamicObjectPriorityIndex {
            object_id,
            priority: value,
            safety: 0.0,
            other_factors: 0.0,
            multiplier: 1.0,
        };
        let candidate_body = overlap_test_sphere_body([0.0; 3]);
        let live_dynamic_objects = vec![
            PrioritizedDynamicCollisionBody {
                priority: priority(4, 100.0),
                body: overlap_test_sphere_body([20.0, 0.0, 0.0]),
            },
            PrioritizedDynamicCollisionBody {
                priority: priority(7, 10.0),
                body: overlap_test_sphere_body([0.0; 3]),
            },
        ];
        let live_vehicles = vec![OwnedDynamicCollisionBody {
            object_id: 20,
            body: overlap_test_sphere_body([0.0; 3]),
        }];

        assert_eq!(
            plan_dynamic_object_admission_from_world(
                priority(0, 10.001),
                &candidate_body,
                None,
                &live_dynamic_objects,
                &live_vehicles,
            ),
            Err(DynamicObjectAdmissionRejection::OverlapPriority {
                blocking_object_id: 7,
            })
        );

        let plan = plan_dynamic_object_admission_from_world(
            priority(0, 10.002),
            &candidate_body,
            Some(priority(30, -1.0)),
            &live_dynamic_objects,
            &live_vehicles,
        )
        .expect("higher-priority candidate should be admitted");
        assert_eq!(plan.overlapping_dynamic_objects, [7]);
        assert_eq!(plan.overlapping_vehicles, [20]);
        assert_eq!(plan.lowest_priority_object, Some(30));
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn dynamic_body_overlap_expands_transformed_mesh_triangles_to_gp_instances() {
        let mut triangle_body_transform = overlap_test_identity();
        triangle_body_transform[14] = 5.0;
        let triangle_body = DynamicCollisionBody {
            model_id: 2,
            transform: triangle_body_transform,
            bounding_box_min: [0.0, 0.0, 0.0],
            bounding_box_max: [1.0, 1.0, 0.0],
            parts: vec![DynamicCollisionPart {
                transform: overlap_test_identity(),
                volumes: vec![],
                triangles: vec![MeshTriangle {
                    cluster_index: 0,
                    unit_stream_offset: 0,
                    vertices: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                    group_id: None,
                    surface_id: None,
                }],
            }],
        };
        let primitives = triangle_body.world_gp_primitives();
        assert_eq!(primitives.len(), 1);
        assert_eq!(primitives[0].points[0], [0.0, 0.0, 5.0]);
        assert_eq!(primitives[0].points[1], [1.0, 0.0, 5.0]);
        assert_eq!(primitives[0].points[2], [0.0, 1.0, 5.0]);
        assert_eq!(primitives[0].face_normals[0], [0.0, 0.0, 1.0]);

        let candidate = overlap_test_sphere_body([0.25, 0.25, 5.0]);
        assert!(candidate.overlaps_dynamic_body(&triangle_body, 0.0));
    }

    #[test]
    fn dynamic_body_bounds_use_reference_absolute_axis_projection() {
        let body = DynamicCollisionBody {
            model_id: 9,
            transform: [
                0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 10.0, 20.0, 30.0, 1.0,
            ],
            bounding_box_min: [-1.0, -2.0, -3.0],
            bounding_box_max: [1.0, 2.0, 3.0],
            parts: vec![],
        };

        assert_eq!(
            body.world_bounding_box(),
            ([7.0, 18.0, 29.0], [13.0, 22.0, 31.0])
        );
    }

    #[test]
    fn dynamic_object_priority_matches_recovered_candidate_arithmetic() {
        let index = dynamic_object_priority_index(
            200,
            1,
            false,
            [10.0, 0.0, 0.0],
            DynamicObjectPriorityContext {
                simulation_center: [0.0; 3],
                camera_position: [0.0; 3],
                camera_heading: [1.0, 0.0, 0.0],
            },
            DynamicObjectPriorityWeights {
                camera_plane: 2.5,
                in_use: 2.0,
                priority: 250.0,
                safety: 300.0,
                distance: 450.0,
            },
        );

        assert_eq!(index.object_id, 0);
        assert_eq!(index.priority.to_bits(), 50_000.0_f32.to_bits());
        assert_eq!(index.safety.to_bits(), (-300.0_f32).to_bits());
        assert_eq!(index.other_factors.to_bits(), (-720.0_f32).to_bits());
        assert_eq!(index.multiplier.to_bits(), 2.5_f32.to_bits());
        assert_eq!(index.score().to_bits(), 122_450.0_f32.to_bits());
    }

    #[test]
    fn static_dynamic_object_priority_uses_reference_sentinels() {
        let index = dynamic_object_priority_index(
            1,
            1,
            true,
            [0.0; 3],
            DynamicObjectPriorityContext {
                simulation_center: [0.0; 3],
                camera_position: [0.0; 3],
                camera_heading: [0.0, 1.0, 0.0],
            },
            DynamicObjectPriorityWeights {
                camera_plane: 2.5,
                in_use: 2.0,
                priority: 250.0,
                safety: 300.0,
                distance: 450.0,
            },
        );

        assert_eq!(index.priority.to_bits(), 0x4f00_0000);
        assert_eq!(index.safety.to_bits(), (-600.0_f32).to_bits());
        assert_eq!(index.multiplier.to_bits(), 1.0_f32.to_bits());
    }

    #[test]
    fn dynamic_object_admission_requires_strict_pool_priority() {
        let candidate = DynamicObjectPriorityIndex {
            object_id: 0,
            priority: 100.0,
            safety: 0.0,
            other_factors: 0.0,
            multiplier: 1.0,
        };
        let mut lowest = candidate;
        lowest.object_id = 41;

        assert_eq!(
            plan_dynamic_object_admission(candidate, Some(lowest), &[], &[]),
            Err(DynamicObjectAdmissionRejection::PoolPriority)
        );

        lowest.priority = 99.0;
        assert_eq!(
            plan_dynamic_object_admission(candidate, Some(lowest), &[], &[]),
            Ok(DynamicObjectAdmissionPlan {
                overlapping_dynamic_objects: vec![],
                overlapping_vehicles: vec![],
                lowest_priority_object: Some(41),
            })
        );
    }

    #[test]
    fn dynamic_object_overlap_gate_uses_exact_epsilon_and_defers_all_culls() {
        let threshold = 100.0_f32 + 0.001;
        let mut candidate = DynamicObjectPriorityIndex {
            object_id: 0,
            priority: threshold,
            safety: 0.0,
            other_factors: 0.0,
            multiplier: 1.0,
        };
        let overlaps = [
            DynamicObjectOverlapPriority {
                object_id: 7,
                priority: 90.0,
                safety: 0.0,
            },
            DynamicObjectOverlapPriority {
                object_id: 8,
                priority: 100.0,
                safety: 0.0,
            },
        ];

        assert_eq!(
            plan_dynamic_object_admission(candidate, None, &overlaps, &[21, 22]),
            Err(DynamicObjectAdmissionRejection::OverlapPriority {
                blocking_object_id: 8,
            })
        );

        candidate.priority = f32::from_bits(threshold.to_bits() + 1);
        assert_eq!(
            plan_dynamic_object_admission(candidate, None, &overlaps, &[21, 22]),
            Ok(DynamicObjectAdmissionPlan {
                overlapping_dynamic_objects: vec![7, 8],
                overlapping_vehicles: vec![21, 22],
                lowest_priority_object: None,
            })
        );
    }
}
