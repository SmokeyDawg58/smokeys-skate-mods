//! Original retained board/Vault/Reckoning with controlled processed inputs.
//! Tests force production and actual queued effects, not a full gameplay tick.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    reckoning::Reckoning,
    reckoning_attributes::ReckoningVaultAttributes,
    reckoning_numeric::HostReckoningNumeric,
    simulation_rigid_body_pool::SimulationRigidBodySlotSnapshot,
    skateboard_foot_forces::{
        SkateboardFootDebugRead, SkateboardFootForce, SkateboardFootForceServices,
        SkateboardFootInputRead,
    },
    skateboard_force_attributes::{
        SkateboardQueuedForceAttributes, SkateboardQueuedForceVaultAttributes,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{convert::Infallible, sync::Arc};

struct Inputs<'a> {
    reckoning: &'a Reckoning<ReckoningVaultAttributes>,
}
impl SkateboardFootInputRead for Inputs<'_> {
    type Error = Infallible;
    fn balance(&mut self) -> Result<f32, Self::Error> {
        Ok(-1.0)
    }
    fn surface_speed(&mut self) -> Result<f32, Self::Error> {
        Ok(8.0)
    }
    fn body_y_direction(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(SkeletonRootVector {
            xyz: [0.0, 1.0, 0.0],
            w: None,
        })
    }
    fn velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(SkeletonRootVector {
            xyz: [0.0; 3],
            w: None,
        })
    }
    fn reckoning_up(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(self.reckoning.up())
    }
}
struct DebugDefaults;
impl crate::skateboard_brake_drag::SkateboardBrakeDragInputRead for Inputs<'_> {
    type Error = Infallible;
    type Surface = ();
    fn flags_8c4(&mut self) -> Result<u32, Infallible> {
        Ok(0x2000_0000)
    }
    fn foot_brake(&mut self) -> Result<f32, Infallible> {
        panic!("drag does not sample footBrake")
    }
    fn capture_surface(&mut self) -> Result<(), Infallible> {
        panic!("drag does not sample surface")
    }
    fn z_speed(&mut self) -> Result<f32, Infallible> {
        panic!("drag samples absolute speed")
    }
    fn surface_braking_scalar(&mut self, (): &()) -> Result<f32, Infallible> {
        panic!("drag does not sample surface")
    }
    fn abs_body_speed(&mut self) -> Result<f32, Infallible> {
        Ok(0.0)
    }
    fn balance(&mut self) -> Result<f32, Infallible> {
        Ok(0.0)
    }
    fn push(&mut self) -> Result<f32, Infallible> {
        Ok(0.0)
    }
    fn body_z_direction(&mut self) -> Result<SkeletonRootVector, Infallible> {
        panic!("drag does not sample bodyZ")
    }
    fn reckoning_ground_normal(&mut self) -> Result<SkeletonRootVector, Infallible> {
        Ok(self.reckoning.ground_normal())
    }
}
impl SkateboardFootDebugRead for DebugDefaults {
    type Target = ();
    type Error = Infallible;
    fn capture_draw_absorption_target(&mut self) -> Result<(), Infallible> {
        Ok(())
    }
    fn draw_absorption(&mut self, (): &()) -> Result<bool, Infallible> {
        Ok(false)
    }
    fn require_overlay(&mut self) -> Result<(), Infallible> {
        panic!("original default false")
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one retained original board force/queue/inertia/next-solve proof"
)]
pub(crate) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    reckoning: &Reckoning<ReckoningVaultAttributes>,
    ground: &crate::static_ground_pool::StaticGroundPool,
) {
    crate::skateboard_foot_attributes::tests::check_original(&owner.inputs().vault);
    crate::skateboard_brake_attributes::tests::check_original(&owner.inputs().vault);
    let observation = world
        .board(board)
        .unwrap()
        .try_part_contact_spy_observation(world.backend(), 6)
        .unwrap();
    let before = world.backend().rigid_body_snapshot();
    let deck_index = before
        .slots
        .iter()
        .position(|slot| slot.id == observation.identity().id())
        .unwrap();
    // Initial empty queue must not modify any body.
    world.try_apply_board_forces(board).unwrap();
    assert_eq!(world.backend().rigid_body_snapshot(), before);
    let zero = SkateboardFootForce {
        world_space_force: SkeletonRootVector::known([0.0; 4]),
        local_space_position: SkeletonRootVector::known([0.0; 4]),
    };
    let mut front = zero;
    let mut rear = zero;
    world
        .try_with_board_force_runtime(board, |runtime| {
            assert_eq!(runtime.queue.len(), 0);
            let mut input = Inputs { reckoning };
            let mut numeric = HostReckoningNumeric;
            let mut debug = DebugDefaults;
            let mut services = SkateboardFootForceServices {
                input: &mut input,
                attributes: &mut runtime.foot_attributes,
                debug: &mut debug,
                numeric: &mut numeric,
            };
            services.try_front(1.0, 0.0, &mut front).unwrap();
            services.try_rear(1.0, 0.0, &mut rear).unwrap();
        })
        .unwrap();
    assert_eq!(
        front.local_space_position,
        SkeletonRootVector::known([0.0, 0.0, 0.22, 0.0])
    );
    assert_eq!(
        rear.local_space_position,
        SkeletonRootVector::known([0.0, 0.0, -0.22, 0.0])
    );
    assert_eq!(front.world_space_force.w, None);
    let expected_front = reckoning
        .up()
        .xyz
        .iter()
        .map(|lane| lane * -40.0)
        .collect::<Vec<_>>();
    for (actual, expected) in front.world_space_force.xyz.into_iter().zip(expected_front) {
        assert!((actual - expected).abs() < 1.0e-5);
    }
    // Only the two recovered producers are exercised. This does not substitute
    // for the six-producer GroundAnimation selection/queueing stage.
    world.try_add_board_force(board, 4, front).unwrap();
    world.try_add_board_force(board, 5, rear).unwrap();
    let offset = SkateboardQueuedForceVaultAttributes::bind(&owner.inputs().vault)
        .local_y_offset()
        .unwrap();
    assert_eq!(
        offset.to_bits(),
        0,
        "own collection has no offset; global default 0x3d4ccccd must not substitute"
    );
    let mut expected = before.clone();
    for _ in 0..2 {
        oracle(&mut expected.slots[deck_index], front, offset);
        oracle(&mut expected.slots[deck_index], rear, offset);
        world.try_apply_board_forces(board).unwrap();
        assert_eq!(world.backend().rigid_body_snapshot(), expected);
        world
            .try_with_board_force_runtime(board, |runtime| assert_eq!(runtime.queue.len(), 2))
            .unwrap();
    }
    assert_ne!(
        expected.slots[deck_index].force.map(f32::to_bits),
        before.slots[deck_index].force.map(f32::to_bits)
    );
    assert_ne!(
        expected.slots[deck_index].torque.map(f32::to_bits),
        before.slots[deck_index].torque.map(f32::to_bits)
    );
    assert_eq!(expected.slots[deck_index].cooldown, 0);
    // Explicit diagnostic arrival at reset boundary, not a claimed full postphysics update.
    world
        .try_with_board_force_runtime(board, |runtime| runtime.queue.clear_post_physics())
        .unwrap();
    world.try_apply_board_forces(board).unwrap();
    assert_eq!(world.backend().rigid_body_snapshot(), expected);
    // Controlled stopped/manual-brake inputs exercise the original scalar
    // producer through all seven actual inertia owners and the next real solve.
    let drag = world
        .try_with_board_force_runtime(board, |runtime| {
            crate::skateboard_brake_drag::SkateboardBrakeDragServices {
                input: &mut Inputs { reckoning },
                attributes: &mut runtime.brake_attributes,
            }
            .try_linear_drag()
            .unwrap()
        })
        .unwrap();
    assert_eq!(drag.to_bits(), 1.0_f32.to_bits());
    let part_ids: Vec<_> = (0..7)
        .map(|part| {
            world
                .board(board)
                .unwrap()
                .try_part_contact_spy_observation(world.backend(), part)
                .unwrap()
                .identity()
                .id()
        })
        .collect();
    let mut expected_drag = world.backend().rigid_body_snapshot();
    world.try_set_board_linear_drag(board, drag).unwrap();
    for slot in &mut expected_drag.slots {
        if part_ids.contains(&slot.id) {
            slot.inertia.as_mut().unwrap().linear_drag = f32::from_bits(0x426f_ffff);
        }
    }
    assert_eq!(world.backend().rigid_body_snapshot(), expected_drag);
    for payload in world.board(board).unwrap().template().part_payloads() {
        assert_eq!(
            payload.inertia.try_read().unwrap().linear_drag.to_bits(),
            0x426f_ffff
        );
    }
    world.try_step_authored_ground(
        crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
            ground_pool: ground, ground_group: 0, pipeline_culling: None, allow_multiple_writes: false, spy_flags: 4,
        },
        f32::from_bits(0x3d08_8889), 50,
        &mut crate::simulation_assembly_backend::constraint_session::contact_feedback::ControlledNoFreezing,
    ).unwrap();
    for slot in world.backend().rigid_body_snapshot().slots {
        if part_ids.contains(&slot.id) {
            assert!(slot.velocity.iter().all(|value| value.abs().to_bits() == 0));
            assert_eq!(slot.inertia.unwrap().linear_drag.to_bits(), 0x426f_ffff);
        }
    }
}

fn oracle(body: &mut SimulationRigidBodySlotSnapshot, force: SkateboardFootForce, offset: f32) {
    let f = force.world_space_force.xyz;
    let p = force.local_space_position.xyz;
    let local = [p[0] + 0.0, p[1] + offset, p[2] + 0.0];
    let arm: [f32; 3] = std::array::from_fn(|i| {
        body.at[i].mul_add(
            local[2],
            body.up[i].mul_add(local[1], body.right[i] * local[0]),
        )
    });
    let acceleration = f.map(|value| value * body.inverse_mass);
    body.force = [
        acceleration[0] + body.force[0],
        body.force[1] + acceleration[1],
        body.force[2] + acceleration[2],
    ];
    let t = [
        (-arm[2]).mul_add(f[1], arm[1] * f[2]),
        (-arm[0]).mul_add(f[2], arm[2] * f[0]),
        (-arm[1]).mul_add(f[0], arm[0] * f[1]),
    ];
    let [xx, xy, xz] = body.inverse_inertia_full;
    let [zz, yy, yz] = body.inverse_inertia_split;
    let tensor = [[xx, xy, xz], [xy, yy, yz], [xz, yz, zz]];
    for (axis, row) in tensor.into_iter().enumerate() {
        body.torque[axis] += row[2].mul_add(t[2], row[1].mul_add(t[1], row[0] * t[0]));
    }
    body.cooldown = 0;
}
