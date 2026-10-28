//! Original physical board and Vault, with controlled contact flags and explicit
//! steering/postphysics stages. This is not an Actor-driven gameplay tick.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    simulation_assembly_backend::constraint_session::{
        contact_feedback::ControlledNoFreezing, skateboard_ground::SkateboardGroundFrameInput,
    },
    skateboard_steering::{
        SkateboardSteeringAttributes, SkateboardSteeringInput, SkateboardSteeringVaultAttributes,
    },
    skateboard_truck_drives::HostSkateboardTruckDriveNumeric,
    static_ground_pool::StaticGroundPool,
};
use std::{convert::Infallible, sync::Arc};

struct ControlledContacts {
    events: Vec<&'static str>,
}
impl SkateboardSteeringInput for ControlledContacts {
    type Processed = ();
    type Error = Infallible;
    fn capture_processed(&mut self) -> Result<(), Infallible> {
        self.events.push("capture");
        Ok(())
    }
    fn flags_8c4(&mut self, (): &()) -> Result<u32, Infallible> {
        self.events.push("flipped flags");
        Ok(0)
    }
    fn flags_8c8(&mut self, (): &()) -> Result<u32, Infallible> {
        self.events.push("wheel flags");
        Ok(0x1800_0000)
    }
}

pub(crate) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    ground: &StaticGroundPool,
) {
    // Independently inspected original VLT with prebuilt inspect_vault:
    // physics_steering class3b76fd332f0d4d06/default parent0, explicit
    // c3249a9df26600c9 nodeflags40/type1/size4 inline word3e4ccccd.
    // This word is an oracle, never the runtime coefficient provider.
    let mut original = SkateboardSteeringVaultAttributes::bind(&owner.inputs().vault);
    assert_eq!(original.blend_coefficient().unwrap().to_bits(), 0x3e4c_cccd);

    let (occurrence, frames, dynamics, initial_frames) = world
        .try_with_constructed_board_truck_drives(board, |drives, _| {
            assert_eq!(drives.count(), 2);
            assert_eq!(drives.add_results(), [Some(true); 2]);
            for payload in drives.dynamics() {
                assert_eq!(payload.read().unwrap().angular.raw_type, 1);
            }
            (
                Arc::clone(drives.occurrence()),
                drives.frames().clone(),
                drives.dynamics().clone(),
                drives
                    .frames()
                    .each_ref()
                    .map(|frame| *frame.read().unwrap()),
            )
        })
        .unwrap();
    let pool_before = world.backend().drive_snapshot();
    let mut input = ControlledContacts { events: Vec::new() };
    world
        .try_update_constructed_board_steering(board, 0.0, &mut input)
        .unwrap();
    assert_eq!(input.events, ["capture", "flipped flags", "wheel flags"]);
    // The setter changes retained angles/timers/force-turn; it does not publish
    // dynamics early. Native publication belongs at the END of PostPhysics.
    assert!(
        dynamics
            .iter()
            .all(|payload| payload.read().unwrap().angular.raw_type == 1)
    );
    world
        .try_with_constructed_board_truck_drives(board, |drives, attributes| {
            drives.try_update_dynamics(attributes).unwrap();
            drives
                .try_update_frames(&mut HostSkateboardTruckDriveNumeric)
                .unwrap();
        })
        .unwrap();
    assert!(
        dynamics
            .iter()
            .all(|payload| payload.read().unwrap().angular.raw_type == 2)
    );

    // Second controlled steering stage advances the same retained contact
    // timers. It is not a fabricated full tick or a reset of the old owner.
    world
        .try_update_constructed_board_steering(board, 0.5, &mut input)
        .unwrap();
    world
        .try_with_constructed_board_truck_drives(board, |drives, attributes| {
            assert!(Arc::ptr_eq(drives.occurrence(), &occurrence));
            drives.try_update_dynamics(attributes).unwrap();
            drives
                .try_update_frames(&mut HostSkateboardTruckDriveNumeric)
                .unwrap();
            for slot in 0..2 {
                assert!(Arc::ptr_eq(&drives.frames()[slot], &frames[slot]));
                assert!(Arc::ptr_eq(&drives.dynamics()[slot], &dynamics[slot]));
                let current = *frames[slot].read().unwrap();
                assert_ne!(
                    current.quaternion_b.map(f32::to_bits),
                    initial_frames[slot].quaternion_b.map(f32::to_bits)
                );
            }
        })
        .unwrap();
    assert_eq!(world.backend().drive_snapshot(), pool_before);

    // Real next solve consumes the already-bound same payloads. Earlier native
    // PostPhysics sibling calculations remain outside this controlled schedule.
    let (counts, report) = world
        .try_step_authored_ground(
            SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                ground_pool: ground,
                ground_group: 0,
                pipeline_culling: None,
                allow_multiple_writes: true,
                spy_flags: 4,
            },
            f32::from_bits(0x3d08_8889),
            50,
            &mut ControlledNoFreezing,
        )
        .unwrap();
    assert!(counts.drives >= 2);
    assert_eq!(report.processed_assemblies, 2);
    let built = world.drive_words_for_test();
    // Original construction inserts the two board truck drives first, followed
    // by hook/root/skeleton drives. Build/Horus preserves angular dynamics W.
    for record in &built[..2] {
        assert_eq!(
            record[3][3],
            1.0_f32.to_bits(),
            "actual forced HARD angular Build coefficient"
        );
    }
    assert_eq!(world.backend().drive_snapshot(), pool_before);
    println!(
        "actual board steering: original blend word3e4ccccd, zero target enables forced truck dynamics, retained nonzero frames, same Drive bindings consumed by next original ground solve; controlled contacts/schedule, no Actor claim"
    );
}
