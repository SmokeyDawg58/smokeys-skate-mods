//! Existing original terrain pair and produced adjusted pose -> lagged lines.
//! The enclosing corpus controls placement and scheduling; no new pose/history.
use super::*;
use crate::{
    collision_query_batches::{CollisionQueryBatchCapacity, CollisionQueryBatchConfig},
    reckoning_numeric::HostReckoningNumeric,
    simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueryConfig,
};

/// One shared query owner for this whole original collision corpus. Three
/// concurrent skeleton trajectories precede IK and the five-test wheel batch.
pub(super) fn initialize_query_world(world: &mut PhysicalAssemblyWorld) {
    let capacity = CollisionQueryBatchCapacity {
        batches: 1,
        tests: 5,
    };
    let batches = CollisionQueryBatchConfig {
        line: capacity,
        boxes: capacity,
        trajectory: CollisionQueryBatchCapacity {
            batches: 3,
            tests: 1,
        },
        step_selector: capacity,
    };
    world
        .try_initialize_world_line_queries(
            SimulationWorldLineQueryConfig {
                vehicle_update_in_parallel: false,
                ground: batches,
                dmo: batches,
                vehicle: batches,
            },
            false,
        )
        .unwrap();
}

pub(super) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    ground: &StaticGroundPool,
) {
    let processed = Arc::clone(owner.reckoning().lock().unwrap().processed());
    let old = {
        let tail = owner.tail().lock().unwrap();
        std::array::from_fn::<_, 3, _>(|i| tail.line_records.as_ref().unwrap()[i].result)
    };
    world
        .try_start_skeleton_line_tests(board, skeleton, ground, &mut HostReckoningNumeric)
        .unwrap();
    {
        let tail = owner.tail().lock().unwrap();
        let records = tail.line_records.as_ref().unwrap();
        assert!(
            records
                .iter()
                .all(|record| record.trajectory_batch.is_some())
        );
        assert_eq!(std::array::from_fn::<_, 3, _>(|i| records[i].result), old);
    }
    world
        .try_fill_skeleton_line_surfaces(board, skeleton)
        .unwrap();
    assert_eq!(processed.lock().unwrap().line_surfaces().left, Some(old[1]));
    world.try_end_skeleton_line_tests(board, skeleton).unwrap();
    let new = {
        let tail = owner.tail().lock().unwrap();
        let records = tail.line_records.as_ref().unwrap();
        assert!(
            records
                .iter()
                .all(|record| record.trajectory_batch.is_none())
        );
        std::array::from_fn::<_, 3, _>(|i| records[i].result)
    };
    assert_eq!(processed.lock().unwrap().line_surfaces().left, Some(old[1]));
    world
        .try_fill_skeleton_line_surfaces(board, skeleton)
        .unwrap();
    let surfaces = processed.lock().unwrap();
    assert_eq!(surfaces.line_surfaces().left, Some(new[1]));
    assert_eq!(surfaces.line_surfaces().right, Some(new[2]));
    assert_eq!(surfaces.line_surfaces().hips, Some(new[0]));
    println!("original same terrain pair skeleton trajectory results: {new:?}");
    drop(surfaces);
    crate::simulation_assembly_backend::constraint_session::physical_skeleton_lines::tests::check_terrain_hit(world, ground, TERRAIN);
}
