//! Actual four board bodies and SAME retained ground basis after original solve.
use super::PhysicalAssemblyWorld;
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    skeleton_root_transforms::SkeletonRootVector,
};
pub(super) fn check(
    world: &PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    phase: &PhysicalPlayerReckoningPhase,
) {
    let owner = world.physical_body_member(board).unwrap().owner();
    let basis = *owner
        .reckoning()
        .lock()
        .unwrap()
        .try_completed(phase)
        .unwrap()
        .ground_transform();
    let actual_board = world.board(board).unwrap();
    let points: [[f64; 3]; 4] = std::array::from_fn(|part| {
        let body = actual_board
            .try_part_contact_spy_observation(world.backend(), part)
            .unwrap();
        world
            .backend()
            .collision_body_pool()
            .try_contact_spy_center_of_mass(body)
            .unwrap()
            .map(f64::from)
    });
    let local: [[f64; 3]; 4] = points.map(|point| {
        std::array::from_fn(|axis| {
            (0..3)
                .map(|lane| point[lane] * f64::from(basis.columns()[axis].xyz[lane]))
                .sum()
        })
    });
    let mean: [f64; 3] =
        std::array::from_fn(|axis| local.iter().map(|point| point[axis]).sum::<f64>() / 4.0);
    let selected = [
        mean[0],
        local
            .iter()
            .map(|point| point[1])
            .fold(f64::INFINITY, f64::min),
        mean[2],
    ];
    let expected: [f64; 3] = std::array::from_fn(|lane| {
        (0..3)
            .map(|axis| selected[axis] * f64::from(basis.columns()[axis].xyz[lane]))
            .sum()
    });
    let mut result = SkeletonRootVector {
        xyz: [99.0; 3],
        w: None,
    };
    world
        .try_calculate_board_ground_position(board, phase, &mut result)
        .unwrap();
    for (actual, expected) in result.xyz.into_iter().zip(expected) {
        assert!(
            (f64::from(actual) - expected).abs() < 0.0005,
            "ground position {actual} vs independent f64 {expected}"
        );
    }
    assert_eq!(result.w, Some(0.0));
    println!(
        "actual four wheel body positions + retained ground basis -> ground position {result:?}; independent f64 agrees, no HiLOD category or contact-point substitution"
    );
}
