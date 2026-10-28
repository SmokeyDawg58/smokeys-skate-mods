//! Original DMO Definitions through the same manager's source backend and
//! generic serialized Insert/Create/base Remove. Placement and address bases
//! are explicit corpus stimuli; catalog selection, native allocator leases,
//! outer Island Body lists and `DynamicPool` mesh publication are not claimed.

use super::{
    SimulationDmoAssemblyPartOwner, SimulationDmoResolvedLocatorPartOwner,
    dynamic_pool::SimulationDmoAssemblyCandidate,
    resident_model_catalog::tests::placed_locator_plan,
};
use crate::{
    dmo_bank_processed_asset::DmoBankProcessedArenaAsset,
    serialised_assembly_bbox::HostSerialisedAssemblyBBoxNumeric,
    serialised_assembly_template::SerialisedAssemblyTemplate,
    simulation_assembly_backend::constraint_session::SimulationAssemblyConstraintSession,
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    simulation_xenon_address_image::{
        SimulationArenaXenonAddressBinding, SimulationXenonAddressImage,
    },
    world::CollisionAggregateRef,
};
use skate2_formats::{
    collision::{AssemblyPartMaterial, VolumeType},
    pegasus::ObjectId,
};
use std::sync::Arc;

const SOURCE_ASSET: u64 = 0xd309_6f13_cf1c_0aab;
const SOURCE_ASSEMBLY: ObjectId = ObjectId(0x219);
const SIMULATION_BASE: u32 = 0x1000_0080;

/// Direct mesh-root inventory is diagnostic source provenance for the next
/// `DynamicPool` publisher. It does not publish collection slots or traverse an
/// invented runtime mesh list.
fn inventory_mesh_roots(plans: &[Arc<SimulationNormalArenaPlan>]) {
    let mut assemblies = 0;
    let mut body_only = 0;
    let mut direct_mesh_roots = 0;
    for plan in plans {
        let asset = plan.static_plan().asset();
        for template in SerialisedAssemblyTemplate::all_in_plan(plan).unwrap() {
            assemblies += 1;
            if template.assembly().joint_count != 0
                || template.assembly().drive_count != 0
                || template.definition().joint_count != 0
                || template.definition().drive_count != 0
            {
                continue;
            }
            body_only += 1;
            for (part, source) in template.parts().iter().enumerate() {
                let root = &asset.collision.aggregate_roots[source.root_index];
                if let Some(CollisionAggregateRef::Mesh(mesh_index)) = root.aggregate {
                    let mesh = &asset.collision.meshes[mesh_index];
                    direct_mesh_roots += 1;
                    println!(
                        "original DMO mesh candidate asset={:016x} assembly={:08x} definition={:08x} part={part} volume={:08x} root={} mesh={} bytes={}",
                        asset.decoded.asset_id,
                        template.assembly_object_id().0,
                        template.definition_object_id().0,
                        source.volume_object_id.0,
                        root.dictionary_index,
                        mesh.dictionary_index,
                        mesh.object_bytes.len()
                    );
                }
            }
        }
    }
    assert_eq!(assemblies, 93);
    println!(
        "original DMO inventory assemblies={assemblies} body_only={body_only} direct_mesh_roots={direct_mesh_roots}; nested mesh occurrences remain the existing decoder's next publication input"
    );
}

/// Independent scalar source graph, including all outer packed W lanes.
fn full_compose(outer: [f32; 16], inner: [f32; 16]) -> [f32; 16] {
    std::array::from_fn(|index| {
        let lane = index % 4;
        let column = index / 4 * 4;
        let first = if column == 12 {
            outer[lane].mul_add(inner[column], outer[12 + lane])
        } else {
            outer[lane] * inner[column]
        };
        let second = outer[4 + lane].mul_add(inner[column + 1], first);
        outer[8 + lane].mul_add(inner[column + 2], second)
    })
}

/// Called from the existing mounted DMO contact corpus; no extra loader or
/// ignored test. Geometry/material/inertia/culling come from the original
/// retained plan. The parsed locator fixture explicitly bypasses model catalog
/// selection, matching the caller's already controlled contact placement.
#[expect(
    clippy::too_many_lines,
    reason = "one actual two-occurrence admission/publication/removal lifetime"
)]
pub(crate) fn assert_original_dmo_pool<C>(
    world: &mut SimulationAssemblyConstraintSession<AssemblyPartMaterial, C>,
    plans: &[Arc<SimulationNormalArenaPlan>],
    placement: [f32; 16],
) {
    inventory_mesh_roots(plans);
    let plan = plans
        .iter()
        .find(|plan| plan.static_plan().asset().decoded.asset_id == SOURCE_ASSET)
        .unwrap();
    let templates = SerialisedAssemblyTemplate::all_in_plan(plan).unwrap();
    let template = templates
        .iter()
        .find(|template| template.assembly_object_id() == SOURCE_ASSEMBLY)
        .unwrap();
    assert_eq!(template.definition_object_id(), ObjectId(0x217));
    assert_eq!(template.parts().len(), 1);
    assert_eq!(template.parts()[0].is_static_body, 0);
    assert_eq!(template.assembly().joint_count, 0);
    assert_eq!(template.assembly().drive_count, 0);
    assert!(
        template
            .volume_sources()
            .iter()
            .any(|source| source.volume_type == VolumeType::Sphere)
    );

    let before = world.backend().rigid_body_snapshot();
    assert!(before.counts.free >= 2);
    let bases = std::array::from_fn(|index| {
        if plan.static_plan().asset().decoded.resources.resources[index].is_empty() {
            0
        } else {
            0x2000_0000 + u32::try_from(index).unwrap() * 0x0100_0000
        }
    });
    let binding =
        Arc::new(SimulationArenaXenonAddressBinding::try_new(Arc::clone(plan), bases).unwrap());
    let image = SimulationXenonAddressImage::try_new(
        SIMULATION_BASE,
        u32::try_from(before.slots.len()).unwrap(),
        0,
        0,
    )
    .unwrap();
    world
        .try_initialize_dmo_pool(image, vec![(Arc::clone(plan), binding)], false)
        .unwrap();
    assert_eq!(world.backend().rigid_body_snapshot(), before);
    let expected_inertia = bases[template.parts()[0].inertia_source.resource_index()]
        .checked_add(template.parts()[0].inertia_source.resource_offset())
        .unwrap();
    let expected_material = template.parts()[0].material;
    let inverse = (template.parts()[0].has_inverse_body_transform != 0)
        .then_some(template.parts()[0].inverse_body_transform);
    let asset = Arc::new(DmoBankProcessedArenaAsset::from_parts_for_test(
        Arc::clone(plan),
        templates,
    ));
    let parts = SimulationDmoAssemblyPartOwner::try_new_from_validated_route(
        asset,
        SOURCE_ASSEMBLY,
        world.backend(),
    )
    .unwrap();
    // Fake keys are confined to this parsed-locator placement fixture; they
    // are never represented as original model/locator association evidence.
    let locator_plan = placed_locator_plan(&[(0x1234, &[(1, 2, 3)])], placement);
    let owner = Arc::new(SimulationDmoResolvedLocatorPartOwner {
        locator: locator_plan.try_locator_occurrence(0, 0).unwrap(),
        parts,
    });
    let mut handles = Vec::new();
    for _ in 0..2 {
        let candidate = SimulationDmoAssemblyCandidate::try_from_locator(&owner).unwrap();
        assert_eq!(candidate.world_index(), 0xffff);
        handles.push(
            world
                .try_insert_dmo(candidate, 4, &mut HostSerialisedAssemblyBBoxNumeric)
                .unwrap(),
        );
    }
    let after = world.backend().rigid_body_snapshot();
    assert_eq!(after.authority, before.authority);
    assert_eq!(after.counts.free, before.counts.free - 2);
    let pool = world.dmo_pool().unwrap();
    assert_eq!(pool.len(), 2);
    let ids = handles
        .iter()
        .map(|handle| pool.candidate(handle).unwrap().world_index())
        .collect::<Vec<_>>();
    assert_eq!(ids[1], ids[0].wrapping_add(1));
    let mut expected_parts = Vec::new();
    let mut observations = Vec::new();
    for handle in &handles {
        let publication = pool.publication(handle).unwrap();
        assert!(Arc::ptr_eq(publication.owner(), plan));
        assert_eq!(publication.definition_object_id(), ObjectId(0x217));
        assert_eq!(publication.header().group, 0);
        assert_eq!(publication.rigid_bodies().len(), 1);
        let body = publication.rigid_bodies()[0];
        observations.push(body);
        let context = world.backend().try_source_context(plan).unwrap();
        let snapshot = context
            .body_pool
            .try_serialised_assembly_snapshot(body)
            .unwrap();
        // Native Initialize image layout: aligned base+0x300, 0x40 per RF row.
        // This oracle does not call the packed provider or Part getter.
        let solver_word = SIMULATION_BASE + 0x300 + body.identity().id().index() * 0x40;
        let raw = [
            snapshot.right[0],
            snapshot.right[1],
            snapshot.right[2],
            f32::from_bits(SIMULATION_BASE),
            snapshot.up[0],
            snapshot.up[1],
            snapshot.up[2],
            f32::from_bits(expected_inertia),
            snapshot.at[0],
            snapshot.at[1],
            snapshot.at[2],
            f32::from_bits(snapshot.tag),
            snapshot.center_of_mass[0],
            snapshot.center_of_mass[1],
            snapshot.center_of_mass[2],
            f32::from_bits(solver_word),
        ];
        let expected = inverse.map_or(raw, |inverse| full_compose(raw, inverse));
        expected_parts.push(expected);
        for volume in publication.volume_data().records() {
            assert_eq!(
                volume.transform.map(f32::to_bits),
                expected.map(f32::to_bits)
            );
            assert_eq!(volume.material, expected_material);
            assert_eq!(volume.group, 0);
        }
        println!(
            "original DMO shared Create asset={SOURCE_ASSET:016x} assembly=00000219 world_index={} body={body:?} part={:08x?}",
            pool.candidate(handle).unwrap().world_index(),
            expected.map(f32::to_bits)
        );
    }
    assert_ne!(observations[0], observations[1]);
    let survivor = pool
        .publication(&handles[1])
        .unwrap()
        .volume_data()
        .records()
        .to_vec();
    for index in 0..2 {
        let removed = world.try_remove_dmo_serialized(&handles[index]).unwrap();
        assert!(!removed.is_attached());
        assert_eq!(removed.world_index(), ids[index]);
        assert_eq!(
            removed.part_transforms().next().unwrap().map(f32::to_bits),
            expected_parts[index].map(f32::to_bits)
        );
        assert!(
            world
                .backend()
                .try_source_context(plan)
                .unwrap()
                .body_pool
                .try_validate_observation(observations[index])
                .is_err()
        );
        if index == 0 {
            let pool = world.dmo_pool().unwrap();
            assert_eq!(pool.len(), 1);
            assert!(pool.handles().next().unwrap().same_as(&handles[1]));
            assert_eq!(
                pool.publication(&handles[1])
                    .unwrap()
                    .volume_data()
                    .records(),
                survivor
            );
        }
    }
    assert_eq!(world.dmo_pool().unwrap().len(), 0);
    let final_snapshot = world.backend().rigid_body_snapshot();
    assert_eq!(final_snapshot.authority, before.authority);
    assert_eq!(final_snapshot.counts, before.counts);
    for old in before
        .slots
        .iter()
        .filter(|slot| before.static_order.contains(&slot.id))
    {
        assert_eq!(
            final_snapshot
                .slots
                .iter()
                .find(|slot| slot.id == old.id)
                .unwrap(),
            old
        );
    }
}
