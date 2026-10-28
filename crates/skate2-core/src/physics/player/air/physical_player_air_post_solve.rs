//! Air.Post82d8b238 and Air.Fill82d84340 on the retained owner.
//!
//! Child of `physical_player_phys_out::construct` retaining existing
//! `PhysOut` guard/component authority. No current-state dispatch is installed here.
//!
//! Fresh raw evidence: Post b24c/b254 skips velocity after reachedApex; b274 reads
//! skateboard-body velocity; b288 splats Y; b2ac bytes1b7cf165 decode physical
//! vcmpgtfp128. v59,v60,v62 (zero > Y), despite IDA printing VA92. b2bc skips
//! the b2c4 apex store unless all splatted lanes compare true. b2c8 passes false
//! to `CheckForAirWipeout` at b2d0, unconditionally after the optional store.
//!
//! Fill 4360 guard ->4380 reachedApex ->4388 byte store;438c maxY;
//!4390 samples guard;4394 startY;4398 fsubs;439c checks sampled guard;
//!43bc height store;43c0 third guard;43e8/ec full landing-normal vector copy.
//!
//! The runtime adapter supplies the actual retained Air wipeout checker.
#![expect(
    dead_code,
    reason = "reviewed Air state methods await complete Air update and wipeout dispatch"
)]
use super::prior_input::PriorInputReadError;
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_air_state::PhysicsAirFields,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::{PhysOutComponent, PhysicalPlayerPhysOut},
    physical_player_physics_state::{CurrentPhysicalState, PhysicalPlayerPhysicsStateOwner},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

#[cfg(test)]
#[path = "physical_player_air_post_solve_tests.rs"]
mod tests;

#[derive(Debug)]
pub(crate) enum AirStateAccessError {
    Poisoned,
    WrongState(CurrentPhysicalState),
    MissingAir,
    ForeignOwner,
}
impl std::fmt::Display for AirStateAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("Air state owner lock poisoned"),
            Self::WrongState(state) => write!(f, "Air operation requires current Air: {state:?}"),
            Self::MissingAir => f.write_str("actual Air instance missing"),
            Self::ForeignOwner => f.write_str("Air instance belongs to another physical pair"),
        }
    }
}

pub(crate) fn validate_air<'a>(
    state: &'a PhysicalPlayerPhysicsStateOwner,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) -> Result<&'a PhysicsAirFields, AirStateAccessError> {
    if state.current != CurrentPhysicalState::Air {
        return Err(AirStateAccessError::WrongState(state.current));
    }
    let air = state.air.as_ref().ok_or(AirStateAccessError::MissingAir)?;
    if !air.bindings.board.same_as(board)
        || !air.bindings.skeleton.same_as(skeleton)
        || !air
            .bindings
            .owner
            .upgrade()
            .is_some_and(|actual| Arc::ptr_eq(&actual, owner))
    {
        return Err(AirStateAccessError::ForeignOwner);
    }
    Ok(&air.fields)
}

/// Native collaborators must operate on the provided retained pair. In
/// particular, `check_air_wipeout` is the Air detector, not `BeginPost` or Ground.
pub(crate) trait AirPostCollaborators {
    type Error;
    fn board_velocity_y(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<f32, Self::Error>;
    fn check_air_wipeout(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        use_com: bool,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum AirPostError<E> {
    State(AirStateAccessError),
    Collaborator(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for AirPostError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::State(error) => error.fmt(f),
            Self::Collaborator(error) => write!(f, "Air.Post collaborator: {error:?}"),
        }
    }
}

/// Exact strict comparison: unordered and either signed zero do not latch.
/// A velocity failure preserves the old latch. Already reached means no read.
fn try_latch_apex<E>(
    reached: &mut bool,
    velocity_y: impl FnOnce() -> Result<f32, E>,
) -> Result<(), E> {
    if !*reached && 0.0 > velocity_y()? {
        *reached = true;
    }
    Ok(())
}

/// Does not mint a state-update or frame-completion token. The apex write remains
/// visible if the actual Air wipeout collaborator fails.
pub(crate) fn try_update_current_air_post<C: AirPostCollaborators>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    source: &mut C,
) -> Result<(), AirPostError<C::Error>> {
    {
        let mut state = owner
            .physics_state()
            .lock()
            .map_err(|_| AirPostError::State(AirStateAccessError::Poisoned))?;
        validate_air(&state, owner, board, skeleton).map_err(AirPostError::State)?;
        let fields = &mut state.air.as_mut().unwrap().fields;
        try_latch_apex(&mut fields.reached_apex, || source.board_velocity_y(board))
            .map_err(AirPostError::Collaborator)?;
    }
    // Release the state lock before the separate Wipeout object is entered.
    source
        .check_air_wipeout(owner, board, skeleton, false)
        .map_err(AirPostError::Collaborator)
}

pub(crate) trait AirFillSource {
    type Error;
    fn reached_apex(&mut self) -> Result<bool, Self::Error>;
    fn max_y(&mut self) -> Result<f32, Self::Error>;
    fn start_y(&mut self) -> Result<f32, Self::Error>;
    fn landing_normal(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum AirFillError<E> {
    Output(PriorInputReadError),
    Source(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for AirFillError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Output(error) => write!(f, "Air.Fill output: {error:?}"),
            Self::Source(error) => write!(f, "Air.Fill source: {error:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    pub(super) fn air_fill_guard(&self) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(PhysOutComponent::Air)
            .map_err(PriorInputReadError::Component)
    }
    pub(crate) fn try_fill_air_state<S: AirFillSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), AirFillError<S::Error>> {
        use AirFillError as E;
        self.air_fill_guard().map_err(E::Output)?;
        self.concrete.air.as_mut().unwrap().is_at_apex =
            source.reached_apex().map_err(E::Source)?;
        let max_y = source.max_y().map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(PriorInputReadError::Access)
            .map_err(E::Output)?;
        let start_y = source.start_y().map_err(E::Source)?;
        let height = max_y - start_y;
        self.check_write_guard(&guard)
            .map_err(PriorInputReadError::Access)
            .map_err(E::Output)?;
        self.require_complete_component(PhysOutComponent::Air)
            .map_err(PriorInputReadError::Component)
            .map_err(E::Output)?;
        self.concrete.air.as_mut().unwrap().jump_height = height;
        self.air_fill_guard().map_err(E::Output)?;
        self.concrete.air.as_mut().unwrap().landing_normal =
            source.landing_normal().map_err(E::Source)?;
        Ok(())
    }
}
struct RetainedFill<'a>(&'a PhysicsAirFields);
impl AirFillSource for RetainedFill<'_> {
    type Error = AirStateAccessError;
    fn reached_apex(&mut self) -> Result<bool, Self::Error> {
        Ok(self.0.reached_apex)
    }
    fn max_y(&mut self) -> Result<f32, Self::Error> {
        Ok(self.0.max_y)
    }
    fn start_y(&mut self) -> Result<f32, Self::Error> {
        Ok(self.0.start_y)
    }
    fn landing_normal(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(self.0.landing_normal)
    }
}

/// Same retained Air instance at Update5's virtual State.Fill slot. Only its
/// three native fields are published; unrelated Air output remains untouched.
pub(crate) fn try_fill_current_air_state(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) -> Result<(), AirFillError<AirStateAccessError>> {
    let state = owner
        .physics_state()
        .lock()
        .map_err(|_| AirFillError::Source(AirStateAccessError::Poisoned))?;
    let fields = validate_air(&state, owner, board, skeleton).map_err(AirFillError::Source)?;
    let mut output = owner
        .phys_out()
        .lock()
        .map_err(|_| AirFillError::Source(AirStateAccessError::Poisoned))?;
    output.try_fill_air_state(&mut RetainedFill(fields))
}

#[path = "physical_player_air_post_runtime.rs"]
pub(crate) mod runtime;
