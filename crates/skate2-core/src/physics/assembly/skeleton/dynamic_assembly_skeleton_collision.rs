//! Actual normal Skeleton owner, current history, and borrowed live body reads
//! for `SkeletonCollision::Update` (0x82b4b8f0). No shadow response state.
use super::SkeletonNormalAssembly;
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    dynamic_assembly_session::{
        part_transform::DynamicAssemblyPartTransformError, skater_pool::SimulationSkaterPoolError,
    },
    simulation_assembly_backend::constraint_session::{
        physical_contact_list::PhysicalContactInfo, physical_island::SimulationIslandBodyHandle,
    },
    simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame,
    simulation_rigid_body_pool::{SimulationRigidBodyObservation, SimulationRigidBodyPoolError},
    skateboard_assembly::SkateboardSimulationBackend,
    skeleton_assembly::SkeletonPreNormalAssemblySession,
    skeleton_collision_data::update::{
        SkeletonCollisionCvars, SkeletonCollisionLiveRead, SkeletonCollisionUpdateError,
        SkeletonCollisionUpdateInput,
    },
    skeleton_collision_numeric::HostSkeletonCollisionNumeric,
    skeleton_collision_update_attributes::SkeletonCollisionUpdateAttributes,
};
use std::{convert::Infallible, fmt, sync::Arc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonCollisionFrameProgress {
    NotStarted,
    Updating,
    Complete,
}
pub(crate) struct SkeletonCollisionFrameState {
    frame: Option<Arc<SimulationContactSpyFrame>>,
    progress: SkeletonCollisionFrameProgress,
}
impl SkeletonCollisionFrameState {
    pub(crate) fn new() -> Self {
        Self {
            frame: None,
            progress: SkeletonCollisionFrameProgress::NotStarted,
        }
    }
    pub(crate) fn frame(&self) -> Option<&Arc<SimulationContactSpyFrame>> {
        self.frame.as_ref()
    }
    pub(crate) fn progress(&self) -> SkeletonCollisionFrameProgress {
        self.progress
    }
}

#[derive(Debug)]
pub(crate) enum SkeletonCollisionLiveError {
    Group(SimulationSkaterPoolError),
    Body(SimulationRigidBodyPoolError),
    Part(DynamicAssemblyPartTransformError),
}
impl fmt::Display for SkeletonCollisionLiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Group(error) => write!(f, "collision other Body group: {error}"),
            Self::Body(error) => write!(f, "collision rigid body: {error}"),
            Self::Part(error) => write!(f, "collision live Part: {error}"),
        }
    }
}
impl std::error::Error for SkeletonCollisionLiveError {}
pub(crate) type SkeletonNormalCollisionError<C> =
    SkeletonCollisionUpdateError<AttributeRuntimeError, C, Infallible, SkeletonCollisionLiveError>;

struct Live<'a, F> {
    backend: &'a SkateboardSimulationBackend,
    session: &'a SkeletonPreNormalAssemblySession,
    groups: F,
}
impl<F: FnMut(&SimulationIslandBodyHandle) -> Result<u32, SimulationSkaterPoolError>>
    SkeletonCollisionLiveRead for Live<'_, F>
{
    type Error = SkeletonCollisionLiveError;
    fn other_body_group(&mut self, body: &SimulationIslandBodyHandle) -> Result<u32, Self::Error> {
        (self.groups)(body).map_err(SkeletonCollisionLiveError::Group)
    }
    fn raw_state(&mut self, body: SimulationRigidBodyObservation) -> Result<u32, Self::Error> {
        self.backend
            .collision_body_pool()
            .try_raw_state(body)
            .map_err(SkeletonCollisionLiveError::Body)
    }
    fn cached_inverse_mass(
        &mut self,
        body: SimulationRigidBodyObservation,
    ) -> Result<f32, Self::Error> {
        self.backend
            .collision_body_pool()
            .try_contact_response_cached_inverse_mass(body)
            .map_err(SkeletonCollisionLiveError::Body)
    }
    fn velocity(&mut self, body: SimulationRigidBodyObservation) -> Result<[f32; 3], Self::Error> {
        self.backend
            .collision_body_pool()
            .try_contact_spy_linear_velocity(body)
            .map_err(SkeletonCollisionLiveError::Body)
    }
    fn part_transform(&mut self, part: usize) -> Result<[f32; 16], Self::Error> {
        self.session
            .try_live_part_transform_host(self.backend, part)
            .map_err(SkeletonCollisionLiveError::Part)
    }
}

impl SkeletonNormalAssembly {
    /// Historical attempt bookkeeping only, not a freshness certificate.
    pub(crate) fn collision_frame_state(&self) -> &SkeletonCollisionFrameState {
        &self.collision_frame
    }
    pub(crate) fn try_begin_collision_frame(
        &mut self,
        frame: Arc<SimulationContactSpyFrame>,
    ) -> bool {
        if self
            .collision_frame
            .frame
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, &frame))
        {
            return false;
        }
        self.collision_frame.frame = Some(frame);
        self.collision_frame.progress = SkeletonCollisionFrameProgress::Updating;
        true
    }
    pub(crate) fn try_update_normal_collision<C: SkeletonCollisionCvars>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        contacts: &[PhysicalContactInfo],
        info: &SkeletonCollisionUpdateInput,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &mut C,
        groups: impl FnMut(&SimulationIslandBodyHandle) -> Result<u32, SimulationSkaterPoolError>,
    ) -> Result<(), SkeletonNormalCollisionError<C::Error>> {
        // Binding retains graph identity only; every source value is fetched
        // lazily by the Update body. No replacement tuning snapshot is made.
        let mut attributes = SkeletonCollisionUpdateAttributes::bind(vault);
        // UpdatePostPhysics577e0 samples OLD retained Skeleton velocity. The
        // later UpdateRecords derivative feeds the next collision call.
        let retained = self.post_physics_records().centre_of_mass_velocity();
        let info = SkeletonCollisionUpdateInput {
            ignore_rigid_body: info.ignore_rigid_body,
            ground_position: info.ground_position,
            ground_normal: info.ground_normal,
            system_velocity: info.system_velocity,
            centre_of_mass_velocity: [retained[0], retained[1], retained[2]],
            timestep: info.timestep,
            danger_zone: info.danger_zone,
            ai_skater: info.ai_skater,
            offboard: info.offboard,
            offboard_air: info.offboard_air,
            ground_trick: info.ground_trick,
            request_partial_ragdoll: info.request_partial_ragdoll,
        };
        self.state.collision.try_update_normal(
            self.state.is_ragdoll,
            &info,
            contacts,
            &self.physics_history,
            &mut attributes,
            cvars,
            &mut HostSkeletonCollisionNumeric,
            &mut Live {
                backend,
                session: &self.session,
                groups,
            },
        )?;
        self.collision_frame.progress = SkeletonCollisionFrameProgress::Complete;
        Ok(())
    }
}
