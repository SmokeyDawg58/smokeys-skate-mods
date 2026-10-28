//! Original owners/Vault with controlled current processed values. Exercises
//! the actual six-calculation tail and both publication branches, not an Actor.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    reckoning::Reckoning,
    reckoning_attributes::ReckoningVaultAttributes,
    reckoning_numeric::HostReckoningNumeric,
    skateboard_brake_drag::SkateboardBrakeDragInputRead,
    skateboard_foot_forces::{SkateboardFootDebugRead, SkateboardFootInputRead},
    skateboard_ground_force_tail::SkateboardGroundForceInput,
    skateboard_processed_surface::{SkateboardProcessedSurface, SkateboardSurfaceProjection},
    skateboard_slide_collision::SkateboardSlideCollisionInputRead,
    skateboard_slide_collision_attributes::{SkateboardSelectedSurface, SlideSurfaceField},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{convert::Infallible, sync::Arc};

struct Input<'a> {
    reckoning: &'a Reckoning<ReckoningVaultAttributes>,
    surface: SkateboardSelectedSurface,
    mass: f32,
    collision: bool,
    events: Vec<&'static str>,
}
fn vector(xyz: [f32; 3]) -> SkeletonRootVector {
    SkeletonRootVector { xyz, w: None }
}
impl SkateboardFootInputRead for Input<'_> {
    type Error = String;
    fn balance(&mut self) -> Result<f32, String> {
        self.events.push("foot balance");
        Ok(0.0)
    }
    fn surface_speed(&mut self) -> Result<f32, String> {
        Ok(8.0)
    }
    fn body_y_direction(&mut self) -> Result<SkeletonRootVector, String> {
        Ok(vector([0.0, 1.0, 0.0]))
    }
    fn velocity(&mut self) -> Result<SkeletonRootVector, String> {
        Ok(vector([1.0, 0.0, 2.0]))
    }
    fn reckoning_up(&mut self) -> Result<SkeletonRootVector, String> {
        Ok(self.reckoning.up())
    }
}
impl SkateboardBrakeDragInputRead for Input<'_> {
    type Error = String;
    type Surface = SkateboardSelectedSurface;
    fn flags_8c4(&mut self) -> Result<u32, String> {
        Ok(0x2000_0000)
    }
    fn foot_brake(&mut self) -> Result<f32, String> {
        panic!("manual only")
    }
    fn capture_surface(&mut self) -> Result<Self::Surface, String> {
        self.events.push("brake");
        Ok(self.surface.clone())
    }
    fn z_speed(&mut self) -> Result<f32, String> {
        Ok(1.0)
    }
    fn surface_braking_scalar(&mut self, surface: &Self::Surface) -> Result<f32, String> {
        surface
            .try_float(SlideSurfaceField::BrakingScalar)
            .map_err(|e| e.to_string())
    }
    fn abs_body_speed(&mut self) -> Result<f32, String> {
        Ok(1.0)
    }
    fn balance(&mut self) -> Result<f32, String> {
        self.events.push("drag balance");
        Ok(0.0)
    }
    fn push(&mut self) -> Result<f32, String> {
        Ok(0.0)
    }
    fn body_z_direction(&mut self) -> Result<SkeletonRootVector, String> {
        Ok(vector([0.0, 0.0, 1.0]))
    }
    fn reckoning_ground_normal(&mut self) -> Result<SkeletonRootVector, String> {
        Ok(self.reckoning.ground_normal())
    }
}
impl SkateboardSlideCollisionInputRead for Input<'_> {
    type Error = String;
    type Surface = SkateboardSelectedSurface;
    fn ground_normal(&mut self) -> Result<SkeletonRootVector, String> {
        Ok(SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]))
    }
    fn velocity(&mut self) -> Result<SkeletonRootVector, String> {
        Ok(vector([1.0, 0.0, 2.0]))
    }
    fn deck_right(&mut self) -> Result<SkeletonRootVector, String> {
        Ok(vector([1.0, 0.0, 0.0]))
    }
    fn surface_speed(&mut self) -> Result<f32, String> {
        Ok(8.0)
    }
    fn surface_collection(&mut self) -> Result<Self::Surface, String> {
        self.events.push("slide surface");
        Ok(self.surface.clone())
    }
    fn wheel_hardness(&mut self) -> Result<f32, String> {
        Ok(0.5)
    }
    fn is_skeleton_in_collision(&mut self) -> Result<bool, String> {
        self.events.push("collision gate");
        Ok(self.collision)
    }
    fn reckoning_ground_normal(&mut self) -> Result<SkeletonRootVector, String> {
        Ok(self.reckoning.ground_normal())
    }
    fn collision_hook_to_hips(&mut self) -> Result<SkeletonRootVector, String> {
        Ok(SkeletonRootVector::known([-1.0, 0.0, 0.0, 0.0]))
    }
    fn surface_velocity(&mut self) -> Result<SkeletonRootVector, String> {
        Ok(SkeletonRootVector::known([2.0, 0.0, 0.0, 0.0]))
    }
    fn time_delta(&mut self) -> Result<f32, String> {
        Ok(f32::from_bits(0x3d08_8889))
    }
    fn skateboard_mass(&mut self) -> Result<f32, String> {
        Ok(self.mass)
    }
    fn z_axis_velocity_dir(&mut self) -> Result<SkeletonRootVector, String> {
        Ok(SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]))
    }
    fn up_vector(&mut self) -> Result<SkeletonRootVector, String> {
        Ok(SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]))
    }
}
impl SkateboardGroundForceInput for Input<'_> {
    fn state_timer(&mut self) -> Result<f32, String> {
        self.events.push("slide timer");
        Ok(1.0)
    }
}
struct DebugDefaults;
impl SkateboardFootDebugRead for DebugDefaults {
    type Error = Infallible;
    type Target = ();
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

pub(crate) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    reckoning: &Reckoning<ReckoningVaultAttributes>,
) {
    crate::skateboard_processed_surface::tests::check_original(&owner.inputs().vault);
    let (_retained_surface_owner, projection) = world
        .try_with_board_force_runtime(board, |runtime| {
            let mut surface = SkateboardProcessedSurface::bind(&runtime.slide_collision_attributes);
            surface
                .try_initialize_materials(&mut runtime.slide_collision_attributes)
                .unwrap();
            let mut projection = SkateboardSurfaceProjection::default();
            // Explicit prior published collision fixture; no current line hit is
            // relabeled as PhysOut and no default collection replaces smooth.
            surface
                .dispatch_from_published_type(1, &mut projection)
                .unwrap();
            (surface, projection)
        })
        .unwrap();
    let mut input = Input {
        reckoning,
        surface: projection.selected.unwrap(),
        mass: crate::skateboard_body_mass::try_skateboard_body_mass(
            world.board(board).unwrap(),
            world.backend(),
        )
        .unwrap(),
        collision: false,
        events: Vec::new(),
    };
    for collision in [false, true] {
        input.collision = collision;
        input.events.clear();
        world
            .try_with_board_force_runtime(board, |runtime| runtime.queue.clear_post_physics())
            .unwrap();
        world.try_set_board_linear_drag(board, 0.5).unwrap();
        let before = world.backend().rigid_body_snapshot();
        world
            .try_update_board_grounded_force_tail(
                board,
                &mut input,
                &mut DebugDefaults,
                &mut HostReckoningNumeric,
            )
            .unwrap();
        let records = world
            .try_with_board_force_runtime(board, |runtime| runtime.queue.records_for_test())
            .unwrap();
        assert_eq!(
            input.events,
            [
                "foot balance",
                "foot balance",
                "foot balance",
                "foot balance",
                "brake",
                "slide timer",
                "slide surface",
                "slide surface",
                "drag balance",
                "collision gate"
            ]
        );
        let kinds: Vec<_> = records.iter().map(|record| record.kind).collect();
        let after = world.backend().rigid_body_snapshot();
        if collision {
            assert_eq!(kinds, [15]);
            assert_eq!(
                after, before,
                "collision choice must suppress linear-drag/body writes"
            );
        } else {
            assert_eq!(kinds, [4, 5, 1, 2]);
            for payload in world.board(board).unwrap().template().part_payloads() {
                assert_eq!(payload.inertia.try_read().unwrap().linear_drag.to_bits(), 0);
            }
        }
        let deck = world
            .board(board)
            .unwrap()
            .try_part_contact_spy_observation(world.backend(), 6)
            .unwrap()
            .identity()
            .id();
        world.try_apply_board_forces(board).unwrap();
        let applied = world.backend().rigid_body_snapshot();
        let old = after.slots.iter().find(|slot| slot.id == deck).unwrap();
        let new = applied.slots.iter().find(|slot| slot.id == deck).unwrap();
        assert_ne!(new.force.map(f32::to_bits), old.force.map(f32::to_bits));
        assert_eq!(new.cooldown, 0);
        assert_eq!(
            world
                .try_with_board_force_runtime(board, |runtime| runtime.queue.len())
                .unwrap(),
            records.len()
        );
    }
    world
        .try_with_board_force_runtime(board, |runtime| runtime.queue.clear_post_physics())
        .unwrap();
}
