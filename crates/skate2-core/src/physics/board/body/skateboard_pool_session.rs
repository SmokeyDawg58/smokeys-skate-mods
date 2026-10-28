//! Manager-owned mixed `SkaterPool` and narrow operations on retained sessions.
//! Source `AssemblyPool` Insert/Remove: 0x82e0dbb0 / 0x82e0c1c8.
//! This does not construct Island Body lists or adjacent pairs.

use super::SimulationAssemblyConstraintSession;
use crate::{
    dynamic_assembly_session::skeleton_normal::drive_owner::SkeletonDriveConstructionError,
    dynamic_assembly_session::skeleton_normal::history_owner::SkeletonHistoryManagerError,
    skateboard_wheels::SkateboardWheelMassNumeric,
    skeleton_drives::{SkeletonDriveAttributeRead, SkeletonDriveCvars, SkeletonDriveNumeric},
    skeleton_physics_history::{SkeletonHistoryInput, SkeletonPhysicsHistoryNumeric},
};
use crate::{
    dynamic_assembly_session::{
        hook_pool::SimulationHookAssemblyHandle,
        skateboard_publication::SkateboardCollisionPublication,
        skater_pool::{
            SimulationSkaterAssemblyCandidate, SimulationSkaterAssemblyHandle,
            SimulationSkaterAssemblyPool, SimulationSkaterInsertFailure, SimulationSkaterPoolError,
            SimulationSkeletonAssemblyCandidate, SimulationSkeletonInsertFailure,
            SimulationSkeletonMutationError,
        },
        skeleton_collision_publication::SkeletonCollisionPublication,
        skeleton_normal::SkeletonNormalAssembly,
    },
    simulation_drive_pool::{SimulationDriveDynamics, SimulationDriveFrames},
    skateboard_assembly::SkateboardAssemblySession,
    skateboard_attributes::SkateboardAttributeRead,
    skateboard_hook::SkateboardHookDefinition,
    skateboard_spawn::FallibleSkateboardSpawnServices,
    skateboard_truck_drives::{
        SkateboardTruckDriveError, SkateboardTruckDriveNumeric, SkateboardTruckDrives,
    },
    skeleton_body_attributes::SkeletonBodyAttributeRead,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::sync::{Arc, RwLock};

/// PC Object Dropper: how many times firmer a dropped board's straight-held
/// trucks are than the bail's forced truck drive (0 = not held, as before).
/// Kick test truck give from rest: 2.7-3.0 deg unheld -> 0.03-0.6 deg at 4x.
const RIDERLESS_TRUCK_STIFFNESS: f32 = 4.0;

fn riderless_truck_stiffness() -> f32 {
    #[cfg(test)]
    if let Some(value) = std::env::var("RIDERLESS_TRUCKS").ok().and_then(|v| v.parse().ok()) {
        return value;
    }
    RIDERLESS_TRUCK_STIFFNESS
}

#[path = "../../player/board/physical_player_board_entry_session.rs"]
mod board_entry;

#[path = "skateboard_riderless_session.rs"]
pub(crate) mod riderless;

#[cfg(test)]
#[path = "skateboard_pool_session_tests.rs"]
pub(crate) mod tests;

impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    /// Same retained pool bridge for OffBoard's current-trajectory and
    /// animation root-pose stores after both COM-step branches join.
    pub(crate) fn try_reset_offboard_trajectory_and_root_poses(
        &mut self,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
    ) -> Result<
        (),
        crate::dynamic_assembly_session::skater_pool::SimulationSkeletonOffboardTrajectoryResetError,
    >{
        use crate::dynamic_assembly_session::skater_pool::SimulationSkeletonOffboardTrajectoryResetError as E;
        self.skaters
            .as_mut()
            .ok_or(E::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_reset_offboard_trajectory_and_root_poses(&self.backend, skeleton, expected_owner)
    }

    pub(crate) fn try_construct_paired_skeleton_members(
        &mut self,
        skeleton: &mut crate::skeleton_assembly_construction::SkeletonAssemblyConstruction,
        board: &SimulationSkaterAssemblyHandle,
        current_entity_flags: &mut impl FnMut() -> u32,
    ) -> Result<(), crate::dynamic_assembly_session::skater_pool::ik_construction::SimulationSkeletonIkConstructionError>{
        use crate::dynamic_assembly_session::skater_pool::ik_construction::SimulationSkeletonIkConstructionError as Error;
        self.skaters
            .as_mut()
            .ok_or(Error::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_construct_paired_skeleton_members(
                &self.backend,
                skeleton,
                board,
                current_entity_flags,
                &mut crate::reckoning_numeric::HostReckoningNumeric,
                &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
            )
    }

    pub(crate) fn try_detach_unadmitted_physical_board(
        &mut self,
        candidate: &mut SimulationSkaterAssemblyCandidate,
    ) -> Result<(), SimulationSkaterPoolError> {
        if candidate
            .construction()
            .and_then(|c| c.physical_owner())
            .is_none_or(|owner| owner.manager() != self.physical_manager_authority())
        {
            return Err(SimulationSkaterPoolError::ForeignManager);
        }
        candidate.try_detach_failed_admission(&mut self.backend)
    }

    pub(crate) fn try_detach_unadmitted_physical_skeleton(
        &mut self,
        candidate: &mut SimulationSkeletonAssemblyCandidate,
    ) -> Result<(), SimulationSkaterPoolError> {
        if candidate
            .construction()
            .is_none_or(|c| c.owner().manager() != self.physical_manager_authority())
        {
            return Err(SimulationSkaterPoolError::ForeignManager);
        }
        candidate.try_detach_failed_admission(&mut self.backend)
    }

    pub(crate) fn try_detach_unadmitted_physical_hook(
        &mut self,
        candidate: &mut crate::dynamic_assembly_session::hook_pool::SimulationHookAssemblyCandidate<
            AssemblyPartMaterial,
            SkateboardHookDefinition,
        >,
    ) -> Result<(), crate::dynamic_assembly_session::hook_pool::SimulationHookPoolError> {
        candidate.try_detach_failed_admission(&mut self.backend)
    }

    pub(crate) fn try_construct_physical_skeleton(
        &mut self,
        construction: &mut crate::skeleton_assembly_construction::SkeletonAssemblyConstruction,
        hat_enabled: &mut impl FnMut() -> bool,
    ) -> Result<(), crate::physical_player_assembly::PhysicalPlayerAssemblyError> {
        use crate::physical_player_assembly::PhysicalPlayerAssemblyError as Error;
        if construction.owner().manager() != self.physical_manager_authority() {
            return Err(Error::Pool(SimulationSkaterPoolError::ForeignManager));
        }
        construction
            .try_construct(&mut self.skeleton_culling, hat_enabled)
            .map_err(Error::SkeletonDefinition)
    }

    pub(crate) fn skeleton_construction(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<
        Option<&crate::skeleton_assembly_construction::SkeletonAssemblyConstruction>,
        SimulationSkaterPoolError,
    > {
        self.skater_pool()?.skeleton_construction(handle)
    }

    pub(crate) fn try_set_constructed_skeleton_normal(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<
        (),
        SimulationSkeletonMutationError<
            crate::attribute_runtime::AttributeRuntimeError,
            std::convert::Infallible,
        >,
    > {
        self.skaters
            .as_mut()
            .ok_or(SimulationSkeletonMutationError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_set_constructed_skeleton_normal(&mut self.backend, handle)
    }

    pub(crate) fn try_teleport_reset_skeleton(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.skaters
            .as_mut()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?
            .try_teleport_reset_skeleton(&self.backend, handle)
    }

    pub(crate) fn physical_manager_authority(
        &self,
    ) -> crate::simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority
    {
        self.contacts.assembly_pool_authority()
    }
    pub(crate) fn board_construction(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<
        Option<&crate::skateboard_body_construction::SkateboardBodyConstruction>,
        SimulationSkaterPoolError,
    > {
        self.skater_pool()?.board_construction(handle)
    }

    /// First nonempty Skateboard::TeleportReset child, reusing the admitted
    /// board's construction attributes and attached Part session.
    pub(crate) fn try_teleport_initialize_board_transforms(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<
        (),
        crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError,
    >{
        use crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError as E;
        self.skaters
            .as_mut()
            .ok_or(E::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_teleport_initialize_board_transforms(&mut self.backend, handle)
    }

    pub(crate) fn try_teleport_reset_board_motion_prefix(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError>{
        use crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError as E;
        self.skaters
            .as_mut()
            .ok_or(E::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_teleport_reset_board_motion_prefix(&mut self.backend, handle)
    }

    pub(crate) fn try_teleport_reset_collision_prefix(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError>{
        use crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError as E;
        self.skaters
            .as_mut()
            .ok_or(E::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_teleport_reset_collision_prefix(&self.backend, handle)
    }

    pub(crate) fn try_teleport_reset_collision_part_loop(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError>{
        use crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError as E;
        self.skaters
            .as_mut()
            .ok_or(E::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_teleport_reset_collision_part_loop(&self.backend, handle)
    }

    pub(crate) fn try_teleport_reset_collision_wheel_loop(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError>{
        use crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError as E;
        self.skaters
            .as_mut()
            .ok_or(E::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_teleport_reset_collision_wheel_loop(&self.backend, handle)
    }

    pub(crate) fn try_teleport_reset_board_grip(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError>{
        use crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError as E;
        self.skaters
            .as_mut()
            .ok_or(E::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_teleport_reset_board_grip(&self.backend, handle)
    }

    pub(crate) fn try_teleport_reset_outer_board(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError>{
        use crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError as E;
        self.skaters
            .as_mut()
            .ok_or(E::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_teleport_reset_outer_board(&self.backend, handle)
    }

    pub(crate) fn try_calc_constructed_board_truck_transforms(&mut self, handle: &SimulationSkaterAssemblyHandle)
    -> Result<crate::skateboard_joints::SkateboardTruckTransforms, crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError>{
        use crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError as Error;
        self.skaters
            .as_mut()
            .ok_or(Error::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_calc_constructed_truck_transforms(handle)
    }

    pub(crate) fn try_initialize_constructed_board_truck_drives(&mut self, handle: &SimulationSkaterAssemblyHandle, drives: &mut SkateboardTruckDrives)
    -> Result<(), crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError>{
        use crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError as Error;
        self.skaters
            .as_mut()
            .ok_or(Error::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_initialize_constructed_truck_drives(&mut self.backend, handle, drives)
    }

    pub(crate) fn try_retain_constructed_board_truck_drives(&mut self, handle: &SimulationSkaterAssemblyHandle, source: &mut Option<SkateboardTruckDrives>)
    -> Result<(), crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError>{
        use crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError as Error;
        self.skaters
            .as_mut()
            .ok_or(Error::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_retain_constructed_truck_drives(&self.backend, handle, source)
    }

    pub(crate) fn try_with_constructed_board_truck_drives<T>(&mut self, handle: &SimulationSkaterAssemblyHandle,
        operation: impl FnOnce(&mut SkateboardTruckDrives, &mut crate::skateboard_attributes::SkateboardAttributes) -> T)
    -> Result<T, crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError>{
        use crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError as Error;
        self.skaters
            .as_mut()
            .ok_or(Error::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_with_constructed_truck_drives(&self.backend, handle, operation)
    }

    pub(crate) fn try_update_constructed_board_steering<I:crate::skateboard_steering::SkateboardSteeringInput>(&mut self,handle:&SimulationSkaterAssemblyHandle,target:f32,input:&mut I)
    ->Result<(),crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedSteeringError<I::Error>>{
        use crate::dynamic_assembly_session::skater_pool::board_construction::{
            SimulationConstructedBoardError, SimulationConstructedSteeringError as Error,
        };
        self.skaters
            .as_mut()
            .ok_or(Error::Owner(SimulationConstructedBoardError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            )))?
            .try_update_constructed_steering(&self.backend, handle, target, input)
    }

    pub(crate) fn try_constructed_board_optional_wheel_drive_byte(&mut self, handle: &SimulationSkaterAssemblyHandle)
    -> Result<u8, crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError>{
        use crate::dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError as Error;
        self.skaters
            .as_mut()
            .ok_or(Error::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_constructed_optional_wheel_drive_byte(handle)
    }

    pub(crate) fn try_initialize_physics_history<I: SkeletonHistoryInput>(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
        input: &mut I,
        numeric: &mut impl SkeletonPhysicsHistoryNumeric,
    ) -> Result<(), SkeletonHistoryManagerError<I::Error>> {
        self.skaters
            .as_mut()
            .ok_or(SkeletonHistoryManagerError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_initialize_physics_history(&self.backend, handle, input, numeric)
    }

    /// Samples the pool's initialization setting supplied by the caller; true
    /// requires the unimplemented parallel path and is never coerced to false.
    pub(crate) fn try_initialize_skater_pool(
        &mut self,
        island_flags: u32,
        current_update_in_parallel: bool,
    ) -> Result<(), SimulationSkaterPoolError> {
        if self.skaters.is_some() {
            return Err(SimulationSkaterPoolError::AlreadyInitialized);
        }
        if island_flags != self.island_flags {
            return Err(SimulationSkaterPoolError::WorldFlagsMismatch {
                expected: self.island_flags,
                actual: island_flags,
            });
        }
        self.skaters = Some(SimulationSkaterAssemblyPool::try_new(
            &self.contacts,
            &self.backend,
            island_flags,
            current_update_in_parallel,
        )?);
        Ok(())
    }

    pub(crate) fn try_insert_board(
        &mut self,
        candidate: SimulationSkaterAssemblyCandidate,
        state: u32,
        bend_threshold: f32,
    ) -> Result<SimulationSkaterAssemblyHandle, Box<SimulationSkaterInsertFailure>> {
        let Some(pool) = self.skaters.as_mut() else {
            return Err(Box::new(SimulationSkaterInsertFailure {
                error: SimulationSkaterPoolError::Uninitialized,
                candidate,
            }));
        };
        pool.try_insert(
            &mut self.backend,
            &mut self.contacts,
            candidate,
            state,
            bend_threshold,
        )
    }

    pub(crate) fn board(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<&SkateboardAssemblySession, SimulationSkaterPoolError> {
        self.skater_pool()?.session(handle)
    }

    pub(crate) fn board_publication(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<&SkateboardCollisionPublication, SimulationSkaterPoolError> {
        self.skater_pool()?.publication(handle)
    }

    pub(crate) fn board_world_index(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<u32, SimulationSkaterPoolError> {
        self.skater_world_index(handle)
    }

    /// One manager-issued index namespace for both dense member kinds.
    pub(crate) fn skater_world_index(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<u32, SimulationSkaterPoolError> {
        self.skater_pool()?.world_index(handle)
    }

    pub(crate) fn board_count(&self) -> usize {
        self.skaters
            .as_ref()
            .map_or(0, SimulationSkaterAssemblyPool::board_count)
    }

    pub(crate) fn try_initialize_board_truck_drives<
        A: SkateboardAttributeRead,
        N: SkateboardTruckDriveNumeric,
    >(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
        drives: &mut SkateboardTruckDrives,
        attributes: &mut A,
        numeric: &mut N,
    ) -> Result<(), SkateboardTruckDriveError<A::Error, N::Error, SimulationSkaterPoolError>> {
        self.skaters
            .as_mut()
            .ok_or(SkateboardTruckDriveError::Admission(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_initialize_truck_drives(&mut self.backend, handle, drives, attributes, numeric)
    }

    pub(crate) fn try_connect_registered_hook(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        hook: &SimulationHookAssemblyHandle,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<bool, SimulationSkaterPoolError> {
        self.skaters
            .as_mut()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?
            .try_connect_hook(
                &mut self.backend,
                board,
                &mut self.hooks,
                hook,
                frames,
                dynamics,
            )
    }

    pub(crate) fn try_set_registered_board_transform(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        hook: &SimulationHookAssemblyHandle,
        requested: [f32; 16],
        services: &mut impl FallibleSkateboardSpawnServices,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.skaters
            .as_mut()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?
            .try_set_board_transform(
                &mut self.backend,
                board,
                &mut self.hooks,
                hook,
                requested,
                services,
            )
    }

    pub(crate) fn try_remove_board_drives(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.skaters
            .as_mut()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?
            .try_remove_post_drives(&mut self.backend, board)
    }

    pub(crate) fn try_remove_board(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<SimulationSkaterAssemblyCandidate, SimulationSkaterPoolError> {
        self.try_prepare_physical_body_cleanup(board,
            crate::dynamic_assembly_session::skater_pool::physical::SimulationPhysicalBodyKind::Board)?;
        let candidate = self
            .skaters
            .as_mut()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?
            .try_remove(&mut self.backend, board)?;
        self.finish_physical_body_cleanup(board);
        Ok(candidate)
    }

    pub(crate) fn try_refresh_boards(&mut self) -> Result<(), SimulationSkaterPoolError> {
        self.try_refresh_skaters()
    }

    /// All dense members, including humanoids, follow the same staged refresh.
    pub(crate) fn try_refresh_skaters(&mut self) -> Result<(), SimulationSkaterPoolError> {
        let pool = self
            .skaters
            .as_mut()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?;
        // PC Object Dropper (a port addition; no-op without riderless boards).
        if pool.riderless_count() > 0 {
            pool.try_wake_partly_frozen_riderless(&mut self.backend)?;
            if riderless_truck_stiffness() > 0.0 {
                pool.try_hold_riderless_trucks(&self.backend, riderless_truck_stiffness())?;
            }
        }
        pool.try_refresh(&self.backend)
    }

    pub(crate) fn try_insert_skeleton(
        &mut self,
        candidate: SimulationSkeletonAssemblyCandidate,
        bend_threshold: f32,
    ) -> Result<SimulationSkaterAssemblyHandle, Box<SimulationSkeletonInsertFailure>> {
        let Some(pool) = self.skaters.as_mut() else {
            return Err(Box::new(SimulationSkeletonInsertFailure {
                error: SimulationSkaterPoolError::Uninitialized,
                candidate,
            }));
        };
        pool.try_insert_skeleton(
            &mut self.backend,
            &mut self.contacts,
            candidate,
            bend_threshold,
        )
    }

    pub(crate) fn skeleton(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<&SkeletonNormalAssembly, SimulationSkaterPoolError> {
        self.skater_pool()?.skeleton(handle)
    }

    pub(crate) fn skeleton_publication(
        &self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<&SkeletonCollisionPublication, SimulationSkaterPoolError> {
        self.skater_pool()?.skeleton_publication(handle)
    }

    pub(crate) fn try_set_skeleton_pre_normal_properties(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<
        (),
        SimulationSkeletonMutationError<std::convert::Infallible, std::convert::Infallible>,
    > {
        self.skaters
            .as_mut()
            .ok_or(SimulationSkeletonMutationError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_set_skeleton_pre_normal_properties(&mut self.backend, handle)
    }

    pub(crate) fn try_set_skeleton_normal<R: SkeletonBodyAttributeRead, C>(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
        reader: &mut R,
        cosine: &mut impl FnMut(f32) -> Result<f32, C>,
    ) -> Result<(), SimulationSkeletonMutationError<R::Error, C>> {
        self.skaters
            .as_mut()
            .ok_or(SimulationSkeletonMutationError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_set_skeleton_normal(&mut self.backend, handle, reader, cosine)
    }

    #[expect(
        clippy::type_complexity,
        reason = "preserve independent source-service error types through the manager"
    )]
    pub(crate) fn try_initialize_skeleton_drives<
        A: SkeletonDriveAttributeRead,
        N: SkeletonDriveNumeric,
        V: SkeletonDriveCvars,
    >(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
        attributes: &mut A,
        numeric: &mut N,
        mass: &mut impl SkateboardWheelMassNumeric,
        cvars: &mut V,
    ) -> Result<(), SkeletonDriveConstructionError<A::Error, N::Error, V::Error>> {
        self.skaters
            .as_mut()
            .ok_or(SkeletonDriveConstructionError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_initialize_skeleton_drives(
                &mut self.backend,
                &mut self.contacts,
                &mut self.hooks,
                handle,
                attributes,
                numeric,
                mass,
                cvars,
            )
    }

    pub(crate) fn try_initialize_skeleton_animation_buffers(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.skaters
            .as_mut()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?
            .try_initialize_skeleton_animation_buffers(&self.backend, handle)
    }

    pub(crate) fn try_finish_skeleton_init(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.skaters
            .as_mut()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?
            .try_finish_skeleton_init(&self.backend, handle)
    }

    pub(crate) fn try_set_skeleton_pose_with_hooks(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.skaters
            .as_mut()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?
            .try_set_skeleton_pose_with_hooks(&mut self.backend, &mut self.hooks, handle)
    }

    pub(crate) fn try_retain_skeleton_drive_attributes(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
        attributes: &mut Option<crate::skeleton_drive_attributes::SkeletonDriveAttributes>,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.skaters
            .as_mut()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?
            .try_retain_skeleton_drive_attributes(&self.backend, handle, attributes)
    }

    pub(crate) fn try_release_skeleton_drives(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.skaters
            .as_mut()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?
            .try_release_skeleton_drives(&mut self.backend, &mut self.hooks, handle)
    }

    pub(crate) fn try_remove_skeleton(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<SimulationSkeletonAssemblyCandidate, SimulationSkaterPoolError> {
        self.try_prepare_physical_body_cleanup(handle,
            crate::dynamic_assembly_session::skater_pool::physical::SimulationPhysicalBodyKind::Skeleton)?;
        let candidate = self
            .skaters
            .as_mut()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?
            .try_remove_skeleton(&mut self.backend, handle)?;
        self.finish_physical_body_cleanup(handle);
        Ok(candidate)
    }

    pub(super) fn skater_pool(
        &self,
    ) -> Result<&SimulationSkaterAssemblyPool, SimulationSkaterPoolError> {
        self.skaters
            .as_ref()
            .ok_or(SimulationSkaterPoolError::Uninitialized)
    }
}
