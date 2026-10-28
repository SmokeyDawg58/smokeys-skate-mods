//! SAME retained Update5 sources579c..5b5c, before existing Ground.Fill5b74.
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::post_category::{PostCategoryError, PostCategorySource},
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, try_unsigned_angle},
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum PostCategoryRuntimeError {
    Poisoned,
    Operation(String),
    Missing(&'static str),
}
impl std::fmt::Display for PostCategoryRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operation(e) => f.write_str(e),
            Self::Poisoned => f.write_str("post-category retained owner poisoned"),
            Self::Missing(field) => write!(f, "post-category requires {field}"),
        }
    }
}
struct Source<'a> {
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    world: &'a PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    skeleton: &'a SimulationSkaterAssemblyHandle,
    cvars: &'a InputCvarReferences,
}
impl Source<'_> {
    fn processed<T>(
        &self,
        read: impl FnOnce(
            &crate::skeleton_processed_contact::SkeletonProcessedContact,
        ) -> Result<T, PostCategoryRuntimeError>,
    ) -> Result<T, PostCategoryRuntimeError> {
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| PostCategoryRuntimeError::Poisoned)?
                .processed(),
        );
        let value = processed
            .lock()
            .map_err(|_| PostCategoryRuntimeError::Poisoned)?;
        read(&value)
    }
}
impl PostCategorySource for Source<'_> {
    type Error = PostCategoryRuntimeError;
    fn left_foot_surface_id(&mut self) -> Result<u32, Self::Error> {
        self.processed(|p| {
            p.line_surfaces()
                .left_id
                .map(|id| u32::from_ne_bytes(id.to_ne_bytes()))
                .ok_or(Self::Error::Missing("retained leftFootSurface934"))
        })
    }
    fn flags_8d0(&mut self) -> Result<u32, Self::Error> {
        self.processed(|p| {
            p.physics_state_fields()
                .flags_8d0
                .ok_or(Self::Error::Missing("Processed8d0"))
        })
    }
    fn board_mount_invalid(&mut self) -> Result<bool, Self::Error> {
        let error = |e: String| Self::Error::Operation(e);
        let skeleton = self
            .world
            .skeleton(self.skeleton)
            .map_err(|e| error(format!("mount skeleton: {e:?}")))?;
        let animated =
            skeleton
                .root_transforms()
                .animation_to_world()
                .compose(&SkeletonRootMatrix::known(
                    skeleton.current_animation().record().poses()[0],
                ));
        let board = self
            .world
            .board(self.board)
            .map_err(|e| error(format!("mount board: {e:?}")))?
            .try_live_part_transform_typed(self.world.backend(), 6)
            .map_err(|e| error(format!("mount transform: {e:?}")))?;
        // Update5 82e05900 compares its GetType read (r27) with 0x1f6, which
        // OffBoard_Jump82d98590 also returns.
        let air = matches!(
            self.owner
                .physics_state()
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .current,
            CurrentPhysicalState::OffBoardAir | CurrentPhysicalState::OffBoardJump
        );
        let (linear, angular) = self
            .cvars
            .try_board_mount_thresholds(air)
            .map_err(|e| error(format!("mount cvars: {e:?}")))?;
        // Update5 82e05944..5a0c: XYZ distance, then Z angle, then X angle.
        // W is never consumed by these operations.
        let operand = |m: &SkeletonRootMatrix, column: usize| {
            let [x, y, z] = m.columns()[column].xyz;
            [x, y, z, 0.0]
        };
        let a = operand(&animated, 3);
        let b = operand(&board, 3);
        let delta = [a[0] - b[0], a[1] - b[1], a[2] - b[2], 0.0];
        let mut numeric = HostReckoningNumeric;
        if numeric
            .dot3(delta, delta)
            .map_err(|e| error(format!("mount distance: {e:?}")))?
            - linear * linear
            > 0.0
        {
            return Ok(true);
        }
        let radians = angular * f32::from_bits(0x3c8e_fa35);
        for column in [2, 0] {
            let angle = try_unsigned_angle(
                operand(&animated, column),
                operand(&board, column),
                &mut numeric,
            )
            .map_err(|e| error(format!("mount angle: {e:?}")))?;
            if angle.abs() > radians {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn interactable_object_id(&mut self) -> Result<u32, Self::Error> {
        // Root-required retained field/reset addition, raw Reset82b39420.
        self.processed(|p| {
            p.physics_state_fields()
                .interactable_object_id
                .ok_or(Self::Error::Missing("Processed interactable8c0"))
        })
    }
    fn teleport_exit_state(&mut self) -> Result<i32, Self::Error> {
        self.processed(|p| {
            p.physics_state_fields()
                .teleport_exit_state
                .ok_or(Self::Error::Missing("Processed teleportExit8d8"))
        })
    }
    fn dismount_count(&mut self) -> Result<i32, Self::Error> {
        let prefix = self
            .owner
            .prefix()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?;
        Ok(prefix
            .as_ref()
            .ok_or(Self::Error::Missing("HiLOD prefix"))?
            .physics_requests_dismount_count)
    }
    fn set_dismount_count(&mut self, value: i32) -> Result<(), Self::Error> {
        let mut prefix = self
            .owner
            .prefix()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?;
        prefix
            .as_mut()
            .ok_or(Self::Error::Missing("HiLOD prefix"))?
            .physics_requests_dismount_count = value;
        Ok(())
    }
    fn physically_pushing(&mut self) -> Result<bool, Self::Error> {
        self.processed(|p| {
            let (bits, known) = p.known_flags_8c4();
            if known & 0x0200_0000 == 0 {
                return Err(Self::Error::Missing("Processed physical push8c4"));
            }
            Ok(bits & 0x0200_0000 != 0)
        })
    }
    fn animation_push_speed(&mut self) -> Result<f32, Self::Error> {
        self.processed(|p| {
            p.animation_attribute_fields()
                .push_speed
                .ok_or(Self::Error::Missing("Processed push speed940"))
        })
    }
    fn animation_time_delta(&mut self) -> Result<f32, Self::Error> {
        let input = Arc::clone(
            self.owner
                .animation_physics_input
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .as_ref()
                .ok_or(Self::Error::Missing("actual AnimOutPhysIn2b4"))?,
        );
        let value = input.lock().map_err(|_| Self::Error::Poisoned)?;
        Ok(value.time_delta)
    }
}
/// Called after actual category/dt5798. Returns before the existing current
/// state virtual Fill5b74; this function does not bypass earlier Update5 spans.
pub(crate) fn try_fill_post_category_output(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    world: &PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    cvars: &InputCvarReferences,
) -> Result<(), PostCategoryError<PostCategoryRuntimeError>> {
    let mut output = owner
        .phys_out()
        .lock()
        .map_err(|_| PostCategoryError::Source(PostCategoryRuntimeError::Poisoned))?;
    output.try_fill_post_category(&mut Source {
        owner,
        world,
        board,
        skeleton,
        cvars,
    })
}
