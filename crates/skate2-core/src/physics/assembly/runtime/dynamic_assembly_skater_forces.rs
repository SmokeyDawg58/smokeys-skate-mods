//! Actual retained board force queue and Part 6 consumer. These entries are
//! explicit stages, not a replacement for wheel queries or postphysics work.
use super::{
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SimulationSkaterPoolError,
};
use crate::{
    attribute_runtime::AttributeRuntimeError,
    dynamic_assembly_session::part_contact_spy::DynamicAssemblyPartContactSpyError,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_assembly::SkateboardSimulationBackend, skateboard_foot_forces::SkateboardFootForce,
    skateboard_force_queue::SkateboardForceApplyError,
    skateboard_force_runtime::SkateboardForceRuntime,
};

#[path = "../../player/board/physical_player_board_force_displacement.rs"]
pub(crate) mod force_displacement;

#[derive(Debug)]
pub(crate) enum SkateboardQueuedBodyError {
    Part(DynamicAssemblyPartContactSpyError),
    Body(SimulationRigidBodyPoolError),
}
impl std::fmt::Display for SkateboardQueuedBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Part(error) => write!(f, "queued deck Part: {error}"),
            Self::Body(error) => write!(f, "queued deck body: {error}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum SimulationSkateboardForceError {
    Pool(SimulationSkaterPoolError),
    MissingRuntime,
    LinearDrag(crate::skateboard_linear_drag::SkateboardLinearDragError),
    SetVelocity(crate::skateboard_set_velocity::SkateboardSetVelocityError),
    SetStandard(crate::skateboard_body_construction::standard::StandardError),
    CollisionVolumes(crate::skateboard_body_construction::wipeout::WipeoutBoardError),
    Apply(SkateboardForceApplyError<AttributeRuntimeError, SkateboardQueuedBodyError>),
}
impl std::fmt::Display for SimulationSkateboardForceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(error) => write!(f, "board force owner: {error}"),
            Self::MissingRuntime => f.write_str("board has no retained Skateboard force owner"),
            Self::Apply(error) => write!(f, "{error}"),
            Self::LinearDrag(error) => write!(f, "{error}"),
            Self::SetVelocity(error) => write!(f, "{error}"),
            Self::SetStandard(error) => write!(f, "{error}"),
            Self::CollisionVolumes(error) => write!(f, "{error}"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum SimulationGroundForceError<I, D, N> {
    Owner(SimulationSkateboardForceError),
    Calculate(crate::skateboard_ground_force_tail::SkateboardGroundForceError<I, D, N>),
}
impl<I: std::fmt::Debug, D: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display
    for SimulationGroundForceError<I, D, N>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(error) => write!(f, "ground forces owner: {error}"),
            Self::Calculate(error) => write!(f, "ground forces calculation: {error}"),
        }
    }
}

impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_enable_board_collision_volumes(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkateboardForceError> {
        use SimulationSkateboardForceError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let board = self.members[index].content.board_mut().map_err(E::Pool)?;
        board
            .construction
            .as_ref()
            .ok_or(E::MissingRuntime)?
            .try_enable_collision_volumes(&board.session)
            .map_err(E::CollisionVolumes)
    }
    pub(crate) fn try_disable_board_collision_volumes(
        &mut self,
        backend: &SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkateboardForceError> {
        use SimulationSkateboardForceError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let board = self.members[index].content.board_mut().map_err(E::Pool)?;
        board
            .construction
            .as_ref()
            .ok_or(E::MissingRuntime)?
            .try_disable_collision_volumes(&board.session)
            .map_err(E::CollisionVolumes)
    }
    pub(crate) fn try_set_board_standard(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkateboardForceError> {
        use SimulationSkateboardForceError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let board = self.members[index].content.board_mut().map_err(E::Pool)?;
        board
            .construction
            .as_mut()
            .ok_or(E::MissingRuntime)?
            .try_set_standard(&mut board.session, backend)
            .map_err(E::SetStandard)
    }
    /// `SkateboardBody::SetAngularDamping` 0x82b24a78 (`Some`) or
    /// `SetStandardAngularDamping` 0x82b2b818 (`None`) on this board.
    pub(crate) fn try_set_board_angular_damping(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        damping: Option<f32>,
    ) -> Result<(), SimulationSkateboardForceError> {
        use SimulationSkateboardForceError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let board = self.members[index].content.board_mut().map_err(E::Pool)?;
        let construction = board.construction.as_mut().ok_or(E::MissingRuntime)?;
        match damping {
            Some(damping) => {
                construction.try_set_angular_damping(&mut board.session, backend, damping)
            }
            None => construction.try_set_standard_angular_damping(&mut board.session, backend),
        }
        .map_err(E::SetStandard)
    }
    #[cfg(test)]
    pub(crate) fn check_late_board_velocity_failure(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) {
        self.check_backend(backend).unwrap();
        let index = self.member_index(handle).unwrap();
        let board = self.members[index].content.board_mut().unwrap();
        crate::dynamic_assembly_session::set_velocity_tests::check_late_part_velocity_failure(
            &mut board.session,
            backend,
        );
    }
    pub(crate) fn try_set_board_velocity(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        velocity: &crate::skeleton_root_transforms::SkeletonRootVector,
    ) -> Result<(), SimulationSkateboardForceError> {
        use SimulationSkateboardForceError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let board = self.members[index].content.board().map_err(E::Pool)?.0;
        crate::skateboard_set_velocity::try_set_skateboard_velocity(
            &board.session,
            backend,
            velocity,
        )
        .map_err(E::SetVelocity)
    }
    /// Actual six-producer tail. Nothing is queued before every calculation
    /// completes; collision success omits the ordinary drag write entirely.
    #[expect(
        clippy::type_complexity,
        reason = "keeps source calculation failures distinct from retained owner failures"
    )]
    pub(crate) fn try_update_board_grounded_force_tail<P, D, N>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        input: &mut P,
        debug: &mut D,
        numeric: &mut N,
    ) -> Result<
        (),
        SimulationGroundForceError<
            <P as crate::skateboard_foot_forces::SkateboardFootInputRead>::Error,
            D::Error,
            N::Error,
        >,
    >
    where
        P: crate::skateboard_ground_force_tail::SkateboardGroundForceInput,
        D: crate::skateboard_foot_forces::SkateboardFootDebugRead,
        N: crate::reckoning_numeric::ReckoningNumeric,
    {
        use crate::skateboard_ground_force_tail::SkateboardGroundForceSelection as Choice;
        use SimulationGroundForceError as E;
        self.check_backend(backend)
            .map_err(|e| E::Owner(SimulationSkateboardForceError::Pool(e)))?;
        let index = self
            .member_index(handle)
            .map_err(|e| E::Owner(SimulationSkateboardForceError::Pool(e)))?;
        let candidate = self.members[index]
            .content
            .board_mut()
            .map_err(|e| E::Owner(SimulationSkateboardForceError::Pool(e)))?;
        let runtime = candidate
            .force_runtime
            .as_mut()
            .ok_or(E::Owner(SimulationSkateboardForceError::MissingRuntime))?;
        let selection = runtime
            .try_calculate_grounded_force_tail(input, debug, numeric)
            .map_err(E::Calculate)?;
        if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
            let wheels = candidate.construction.as_ref().and_then(|body| {
                let lines = body.wheel_lines().try_lock().ok()?;
                Some((lines.distances, lines.height))
            });
            eprintln!("known air: ground-animation wheels distances/height={wheels:?}");
            match &selection {
                Choice::Collision(force) => eprintln!(
                    "known air: ground-animation forces collision={:?}@{:?}",
                    force.world_space_force.xyz, force.local_space_position.xyz
                ),
                Choice::Ordinary {
                    front,
                    rear,
                    slide,
                    brake,
                    drag,
                } => eprintln!(
                    "known air: ground-animation forces front={:?}@{:?} rear={:?}@{:?} slide={:?}@{:?} brake={:?}@{:?} drag={drag}",
                    front.world_space_force.xyz,
                    front.local_space_position.xyz,
                    rear.world_space_force.xyz,
                    rear.local_space_position.xyz,
                    slide.world_space_force.xyz,
                    slide.local_space_position.xyz,
                    brake.world_space_force.xyz,
                    brake.local_space_position.xyz
                ),
            }
        }
        match selection {
            Choice::Collision(force) => runtime.queue.add(15, force),
            Choice::Ordinary {
                front,
                rear,
                slide,
                brake,
                drag,
            } => {
                runtime.queue.add(4, front);
                runtime.queue.add(5, rear);
                runtime.queue.add(1, slide);
                runtime.queue.add(2, brake);
                crate::skateboard_linear_drag::try_set_skateboard_linear_drag(
                    &candidate.session,
                    backend,
                    drag,
                )
                .map_err(|e| E::Owner(SimulationSkateboardForceError::LinearDrag(e)))?;
            }
        }
        Ok(())
    }
    pub(crate) fn try_set_board_linear_drag(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
        drag: f32,
    ) -> Result<(), SimulationSkateboardForceError> {
        use SimulationSkateboardForceError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let board = self.members[index].content.board().map_err(E::Pool)?.0;
        crate::skateboard_linear_drag::try_set_skateboard_linear_drag(&board.session, backend, drag)
            .map_err(E::LinearDrag)
    }
    pub(crate) fn try_with_board_force_runtime<T>(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
        operation: impl FnOnce(&mut SkateboardForceRuntime) -> T,
    ) -> Result<T, SimulationSkateboardForceError> {
        let index = self
            .member_index(handle)
            .map_err(SimulationSkateboardForceError::Pool)?;
        let candidate = self.members[index]
            .content
            .board_mut()
            .map_err(SimulationSkateboardForceError::Pool)?;
        let runtime = candidate
            .force_runtime
            .as_mut()
            .ok_or(SimulationSkateboardForceError::MissingRuntime)?;
        Ok(operation(runtime))
    }

    pub(crate) fn try_add_board_force(
        &mut self,
        handle: &SimulationSkaterAssemblyHandle,
        kind: u32,
        force: SkateboardFootForce,
    ) -> Result<(), SimulationSkateboardForceError> {
        self.try_with_board_force_runtime(handle, |runtime| runtime.queue.add(kind, force))
    }

    pub(crate) fn try_apply_board_forces(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        handle: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkateboardForceError> {
        use SimulationSkateboardForceError as E;
        self.check_backend(backend).map_err(E::Pool)?;
        let index = self.member_index(handle).map_err(E::Pool)?;
        let candidate = self.members[index].content.board_mut().map_err(E::Pool)?;
        let runtime = candidate.force_runtime.as_mut().ok_or(E::MissingRuntime)?;
        let board = &candidate.session;
        runtime
            .queue
            .try_apply(&mut runtime.own_attributes, |force, offset| {
                // Source reloads the same actual deck body per entry. No cached
                // serialized Part, replacement rigid body or foreign owner enters.
                let observation = board
                    .try_part_contact_spy_observation(backend, 6)
                    .map_err(SkateboardQueuedBodyError::Part)?;
                backend
                    .try_apply_skateboard_queued_force(
                        observation,
                        force.world_space_force,
                        force.local_space_position,
                        offset,
                    )
                    .map_err(SkateboardQueuedBodyError::Body)
            })
            .map_err(E::Apply)
    }
}
