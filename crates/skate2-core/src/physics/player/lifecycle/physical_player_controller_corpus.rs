//! Actual retained hand-drive admission and explicit source-gated cleanup.
use crate::physics_state_ids::PHYSICS_STATE_PHYSICS_GROUND;
use super::PhysicalAssemblyWorld;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
    },
    simulation_assembly_backend::constraint_session::physical_skateboard_controller::{
        PhysicalPlayerControllerSessionError, SkateboardControllerCvars,
    },
    skateboard_controller::SkateboardControllerPhase,
};
use std::{convert::Infallible, sync::Arc};
struct Cvars {
    value: bool,
    captures: usize,
    reads: usize,
}
impl SkateboardControllerCvars for Cvars {
    type Error = Infallible;
    type Target = usize;
    fn capture_use_hand_drives(&mut self) -> Result<usize, Self::Error> {
        self.captures += 1;
        Ok(self.captures)
    }
    fn use_hand_drives(&mut self, target: &usize) -> Result<bool, Self::Error> {
        assert_eq!(*target, self.captures);
        self.reads += 1;
        Ok(self.value)
    }
}
pub(super) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) {
    let owner = Arc::clone(world.physical_body_member(board).unwrap().owner());
    let before = world.backend().drive_snapshot();
    assert_eq!(before.counts.free, 2);
    let mut cvars = Cvars {
        value: true,
        captures: 0,
        reads: 0,
    };
    world
        .try_construct_pair_skateboard_controller(board, skeleton, &mut cvars)
        .unwrap();
    assert_eq!((cvars.captures, cvars.reads), (2, 2));
    assert_eq!(
        world.backend().drive_snapshot().active_order.len(),
        before.active_order.len() + 2
    );
    {
        let mut controller = owner.skateboard_controller().lock().unwrap();
        assert_eq!(controller.phase(), SkateboardControllerPhase::Complete);
        assert_eq!(
            (
                controller.state(),
                controller.hand(),
                controller.system_on()
            ),
            (Some(0), Some(2), Some(false))
        );
        let deck = world
            .board(board)
            .unwrap()
            .part_contact_spy_handle(world.backend(), 6)
            .unwrap();
        for (slot, part) in [3, 7].into_iter().enumerate() {
            let hand = world
                .skeleton(skeleton)
                .unwrap()
                .session()
                .part_contact_spy_handle(world.backend(), part)
                .unwrap();
            assert!(
                world
                    .backend()
                    .try_validate_owned_drive_payloads(
                        controller.drive(slot).copied(),
                        deck,
                        hand,
                        controller.frames(slot).unwrap(),
                        controller.dynamics(slot).unwrap()
                    )
                    .unwrap()
            );
        }
        controller
            .try_enter_physics_state::<Infallible>(PHYSICS_STATE_PHYSICS_GROUND)
            .unwrap();
        assert_eq!(controller.system_on(), Some(false));
    }
    assert!(matches!(
        world.try_remove_skeleton(skeleton),
        Err(SimulationSkaterPoolError::ControllerDrivesRemain {
            slots: [true, true]
        })
    ));
    let admitted = world.backend().drive_snapshot();
    cvars.value = false;
    assert!(matches!(
        world.try_remove_pair_skateboard_controller_drives(board, skeleton, &mut cvars),
        Err(PhysicalPlayerControllerSessionError::DrivesRemain {
            slots: [true, true]
        })
    ));
    assert_eq!(world.backend().drive_snapshot(), admitted);
    cvars.value = true;
    world
        .try_remove_pair_skateboard_controller_drives(board, skeleton, &mut cvars)
        .unwrap();
    assert_eq!(
        world.backend().drive_snapshot().active_order.len(),
        before.active_order.len()
    );
    owner.require_controller_drives_removed().unwrap();
    assert_eq!((cvars.captures, cvars.reads), (4, 4));
    println!(
        "actual controller same-owner hand3/deck6 and hand7/deck6 drives admitted with retained payloads; inactive Stop succeeds, false removal preserves handles/refuses body cleanup, true removal releases both. Selected late constructor, diagnostic source-default cvar=true; not full startup or active retrieval"
    );
}
