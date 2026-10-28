use crate::physics_state_ids::{PHYSICS_STATE_OFFBOARD, PHYSICS_STATE_PHYSICS_GROUND};
use super::*;
use crate::{
    physical_player_phys_out::construct::PhysOutAllocation,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[test]
fn ready_fill_selects_exit_state_and_copies_every_transform_lane() {
    let columns = [
        [1.0, 2.0, 3.0, 4.0],
        [5.0, 6.0, 7.0, 8.0],
        [9.0, 10.0, 11.0, 12.0],
        [13.0, 14.0, 15.0, 16.0],
    ];
    let transform = SkeletonRootMatrix::from_columns(columns.map(SkeletonRootVector::known));
    for (on_board, exit_state) in [(true, PHYSICS_STATE_PHYSICS_GROUND), (false, PHYSICS_STATE_OFFBOARD)] {
        let mut output = PhysicalPlayerPhysOut::new();
        output
            .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        output
            .try_publish_teleporting_ready(on_board, transform)
            .unwrap();
        let state = output.concrete.state.as_ref().unwrap();
        let motion = output.concrete.motion.as_ref().unwrap();
        assert!(state.is_teleport_finished);
        assert_eq!(state.teleport_exit_state, exit_state);
        assert!(motion.did_teleport);
        assert_eq!(motion.teleport_transform.columns(), transform.columns());
    }
}
