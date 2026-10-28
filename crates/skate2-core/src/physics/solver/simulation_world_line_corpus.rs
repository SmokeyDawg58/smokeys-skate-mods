//! Original terrain through the scoped actual World line route. This verifies
//! constructor-empty dynamic owners and batch reuse; it is not a native tick,
//! spawn, `DynamicPool` publication, or completed World scheduling trace.

use super::*;
use crate::{
    collision_line_numeric::HostCollisionLineNumeric,
    simulation_assembly_backend::constraint_session::SimulationAssemblyConstraintSession,
};

/// Called after the enclosing original-data session initialized its actual
/// query owners. Input and retained ground are the caller's original probe.
pub(crate) fn assert_original_world_line<C>(
    world: &mut SimulationAssemblyConstraintSession<AssemblyPartMaterial, C>,
    ground: &StaticGroundPool,
    input: CollisionLineInput,
) {
    assert_eq!(world.dmo_pool().unwrap().len(), 0);
    let child = ground
        .try_run_line_test(input, &mut HostCollisionLineNumeric)
        .unwrap();
    assert!(child.hit_count > 0);

    // Existing independent, simplified original geometry oracle. The corpus
    // caller's selected Rob's Park surface belongs to the first retained mesh.
    let mesh = ground.assembly_ground_meshes().next().unwrap().mesh;
    let reference = mesh
        .clustered_mesh
        .thin_segment_intersections(&mesh.object_bytes, input.start.xyz, input.end.xyz)
        .unwrap();
    let expected = reference
        .iter()
        .min_by(|a, b| {
            a.intersection
                .line_parameter
                .total_cmp(&b.intersection.line_parameter)
        })
        .unwrap();
    assert_eq!(child.surface_id, expected.triangle.surface_id.unwrap());
    assert!((child.line_parameter - expected.intersection.line_parameter).abs() < 0.000_002);
    for axis in 0..3 {
        assert!((child.position.xyz[axis] - expected.intersection.position[axis]).abs() < 0.000_02);
    }

    // The retained batches must release and be acquired again. Full equality
    // includes both W lanes, nearest provenance/count, surface and null owner.
    for _ in 0..2 {
        let result = world
            .try_world_line_test(
                ground,
                input,
                "OriginalWorldLineConstructorEmpty",
                &mut HostCollisionLineNumeric,
            )
            .unwrap()
            .expect("actual original ground hit must survive World aggregation");
        assert_eq!(result.position, child.position);
        assert_eq!(result.normal, child.normal);
        assert_eq!(result.line_parameter, child.line_parameter);
        assert_eq!(result.surface_id, child.surface_id);
        assert_eq!(result.provenance, child.provenance);
        assert_eq!(result.hit_count, child.hit_count);
        assert!(result.owner.is_none() && child.owner.is_none());
    }
    assert_eq!(world.dmo_pool().unwrap().len(), 0);
    println!(
        "original World line constructor-empty scope: repeated retained batches, surface={}, parameter={:08x}, ground provenance={:?}; no DMO spawn/publication or native scheduling claim",
        child.surface_id,
        child.line_parameter.to_bits(),
        child.provenance
    );
}
