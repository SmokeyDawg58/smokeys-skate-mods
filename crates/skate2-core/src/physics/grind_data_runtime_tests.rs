//! Same-owner source lookup, full-lane primitive and query lifecycle tests.
use super::*;
use crate::{
    simulation_collision_asset::SimulationCollisionAsset,
    world::{CollisionArena, CollisionInstance, CollisionModel},
};
use skate2_formats::{
    pegasus::{C_MODEL_DATA_TYPE_ID, ObjectId},
    rw4::{Arena, DictionaryEntry, FileHeader, ResourceDescriptor},
    stream::ArenaResources,
};
use std::ops::Range;

fn put(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}
pub(crate) fn source(id: u64, count: usize) -> GrindSplineSource {
    let mut fixture = ArenaFixture::new();
    let mut spline = vec![0; 16 + count * 144];
    put(&mut spline, 4, u32::try_from(count).unwrap());
    put(&mut spline, 8, 16);
    put(&mut spline, 12, 16);
    for index in 0..count {
        let offset = 16 + index * 144;
        // A+B cancels, then C+D yields2. Diagnostic(C+B)+(A+D) yields0.
        for (lane, value) in [
            (0, 1.0e20_f32),
            (4, -1.0e20),
            (8, 1.0),
            (12, 1.0),
            (15, 17.0),
        ] {
            put(&mut spline, offset + lane * 4, value.to_bits());
        }
        for lane in 0..4 {
            put(&mut spline, offset + 80 + lane * 4, (-4.0_f32).to_bits());
            put(&mut spline, offset + 96 + lane * 4, 4.0_f32.to_bits());
        }
    }
    let object = fixture.push(SPLINE_DATA_TYPE_ID, &spline);
    let mut toc = vec![0; 45];
    for (offset, value) in [
        (0, 1),
        (4, 20),
        (8, 44),
        (16, 44),
        (20, 44),
        (36, SPLINE_DATA_TYPE_ID),
        (40, object.0),
    ] {
        put(&mut toc, offset, value);
    }
    toc[28..36].copy_from_slice(&id.to_be_bytes());
    fixture.push(TABLE_OF_CONTENTS_TYPE_ID, &toc);
    let owner = Arc::new(
        SimulationNormalArenaPlan::from_collision_asset(fixture.finish(Vec::new())).unwrap(),
    );
    GrindSplineSource::try_from_operation(owner, 0).unwrap()
}
struct Dynamic {
    calls: usize,
    observed: usize,
}
impl DynamicGrindSplineProvider for Dynamic {
    type Error = &'static str;
    fn query(
        &mut self,
        _: OctreeBounds,
        result: &mut GrindDataQueryResult,
    ) -> Result<(), Self::Error> {
        self.calls += 1;
        self.observed = result.count();
        Ok(())
    }
}
fn bounds() -> OctreeBounds {
    OctreeBounds {
        minimum: [-8.0; 4],
        maximum: [8.0; 4],
    }
}

#[test]
fn source_retains_arena_identity_and_native_endpoint_w_graph() {
    let source = source(0x1234_5678_0000_0001, 1);
    assert_eq!(source.asset_id(), 0x1234_5678_0000_0001);
    assert!(source.is_same_source(&source.clone()));
    assert_eq!(source.operation_index(), Some(0));
    assert_eq!(source.dictionary_index(), 0);
    let record = GrindAssetRecord::try_new(source, &mut HostGrindDataNumeric).unwrap();
    assert_eq!(record.primitives[0].segment_index, 0);
    assert_eq!(record.primitives[0].spline_header_offset, None);
    assert_eq!(record.primitives[0].start[3].to_bits(), 17.0_f32.to_bits());
    assert_eq!(
        record.primitives[0].end.map(f32::to_bits),
        [2.0_f32.to_bits(), 0, 0, 2.0_f32.to_bits()]
    );
    assert_eq!(
        record.bounds.minimum.map(f32::to_bits),
        [-5.0_f32, -5.0, -5.0, -4.0].map(f32::to_bits)
    );
    assert_eq!(
        record.bounds.maximum.map(f32::to_bits),
        [5.0_f32, 5.0, 5.0, 4.0].map(f32::to_bits)
    );
}

#[test]
fn repeated_bootstrap_refusal_retains_existing_record_and_arena_owner() {
    let source = source(9, 1);
    let GrindSplineOwner::Normal(owner) = &source.owner else {
        panic!("normal-arena fixture returned a character owner");
    };
    let backing = Arc::downgrade(owner);
    let mut runtime = GrindDataRuntime::new();
    assert!(!runtime.is_initialized());
    runtime.try_initialize().unwrap();
    assert!(runtime.is_initialized());
    assert!(runtime.records.as_ref().unwrap().is_empty());
    runtime.try_add(source, &mut HostGrindDataNumeric).unwrap();
    let count_before = backing.strong_count();
    assert!(matches!(
        runtime.try_initialize(),
        Err(GrindDataError::AlreadyInitialized)
    ));
    assert_eq!(backing.strong_count(), count_before);
    assert_eq!(runtime.records.as_ref().unwrap().len(), 1);
    assert_eq!(runtime.records.as_ref().unwrap()[0].source.asset_id(), 9);
}

#[test]
fn removal_uses_full_guid_and_rechecks_swapped_duplicate() {
    let a = source(0x1111_0000_0000_0001, 1);
    let b = source(0x2222_0000_0000_0001, 1);
    let mut runtime = GrindDataRuntime::new();
    runtime.try_initialize().unwrap();
    for item in [a.clone(), b.clone(), a.clone()] {
        runtime.try_add(item, &mut HostGrindDataNumeric).unwrap();
    }
    runtime.try_remove(a.asset_id()).unwrap();
    let records = runtime.records.as_ref().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].source.asset_id(), b.asset_id());
}

#[test]
fn static_cap_still_calls_dynamic_and_nan_extent_clears_only_count() {
    let mut runtime = GrindDataRuntime::new();
    runtime.try_initialize().unwrap();
    runtime
        .try_add(source(7, 41), &mut HostGrindDataNumeric)
        .unwrap();
    let mut result = GrindDataQueryResult::new();
    result.flags[0] = (0x1234_5678, u32::MAX);
    let mut dynamic = Dynamic {
        calls: 0,
        observed: 0,
    };
    assert!(
        runtime
            .try_box_test(
                bounds(),
                &mut result,
                false,
                &mut HostGrindDataNumeric,
                Some(&mut dynamic)
            )
            .unwrap()
    );
    assert_eq!(
        (result.count(), dynamic.calls, dynamic.observed),
        (40, 1, 40)
    );
    assert_eq!(result.flags[0], (0x1234_5678, u32::MAX));
    let mut invalid = bounds();
    invalid.minimum[0] = f32::NAN;
    assert!(
        !runtime
            .try_box_test(
                invalid,
                &mut result,
                false,
                &mut HostGrindDataNumeric,
                Some(&mut dynamic)
            )
            .unwrap()
    );
    assert_eq!((result.count(), dynamic.calls), (0, 1));
    assert!(result.slots[0].is_some());
}

#[test]
fn box_overlap_rejects_only_ordered_xyz_separation() {
    let mut other = bounds();
    other.minimum[0] = 9.0;
    assert!(!overlap(bounds(), other));
    other.minimum[0] = f32::NAN;
    assert!(overlap(bounds(), other));
    other.minimum[3] = f32::INFINITY;
    assert!(overlap(bounds(), other));
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

    fn finish(self, instances: Vec<CollisionInstance>) -> SimulationCollisionAsset {
        let descriptor = ResourceDescriptor {
            size: 0,
            alignment: 1,
        };
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
            entry_count: u32::try_from(self.dictionary.len()).unwrap(),
            used_entry_count: u32::try_from(self.dictionary.len()).unwrap(),
            alignment: 1,
            virtual_size: u32::try_from(self.bytes.len()).unwrap(),
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
        let collision_models = arena
            .dictionary
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.type_id == C_MODEL_DATA_TYPE_ID)
            .map(|(dictionary_index, _)| CollisionModel {
                dictionary_index: u32::try_from(dictionary_index).unwrap(),
                bounding_root: None,
                collision_roots: Vec::new(),
                assembly_object_id: None,
            })
            .collect();
        SimulationCollisionAsset {
            decoded: crate::decoded_arena_asset::DecodedArenaAsset::from_parts_for_test(
                0xabcd_ef01_2345_6789,
                crate::decoded_arena_asset::WORLD_STREAM_ARENA_ASSET_TYPE,
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
                collision_models,
                instances,
            },
        }
    }
}
