//! Update5 outer stores82e04f88..53ec, before current-type publication.
use super::super::{PhysOutComponent, PhysicalPlayerPhysOut, SkeletonRootVector};
use super::prior_input::PriorInputReadError;
#[cfg(test)]
#[path = "physical_player_phys_out_outer_locality_tests.rs"]
mod tests;

pub(crate) trait Update5OuterLocalitySource {
    type Error;
    type Processed;
    /// A fresh Processed owner read for each call; require only the named bit.
    fn processed_flag(&mut self, offset: u32, bit: u32) -> Result<bool, Self::Error>;
    fn debug_render_local_sim_player(&mut self) -> Result<bool, Self::Error>;
    /// Actual Actor query: manager index BEFORE Actor player ID.
    fn is_local_player(&mut self) -> Result<bool, Self::Error>;
    /// Actual Actor query: Actor player ID BEFORE manager sim index.
    fn is_local_sim_player(&mut self) -> Result<bool, Self::Error>;
    fn state_count(&mut self) -> Result<i32, Self::Error>;
    fn collide_with_world_only(&mut self) -> Result<bool, Self::Error>;
    fn turn(&mut self) -> Result<f32, Self::Error>;
    fn capture_processed(&mut self) -> Result<Self::Processed, Self::Error>;
    fn ground_position(
        &mut self,
        owner: &Self::Processed,
    ) -> Result<SkeletonRootVector, Self::Error>;
    fn captured_flag(
        &mut self,
        owner: &Self::Processed,
        offset: u32,
        bit: u32,
    ) -> Result<bool, Self::Error>;
    fn jump_strength(&mut self) -> Result<f32, Self::Error>;
    fn balance(&mut self) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum Update5OuterLocalityError<E> {
    Output(PriorInputReadError),
    Source(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for Update5OuterLocalityError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Output(error) => write!(f, "Update5 outer output: {error:?}"),
            Self::Source(error) => write!(f, "Update5 outer source: {error:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    fn outer_locality_guard(&self, component: PhysOutComponent) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
    }
    #[expect(
        clippy::too_many_lines,
        reason = "source guard and partial-store order across outer state publication"
    )]
    pub(crate) fn try_publish_update5_outer_locality<S: Update5OuterLocalitySource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), Update5OuterLocalityError<S::Error>> {
        use Update5OuterLocalityError as E;
        let value = source.processed_flag(0x8c4, 29).map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete.state.as_mut().unwrap().is_manual_braking = value;
        let value = source.processed_flag(0x8c4, 30).map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete.state.as_mut().unwrap().is_foot_braking = value;
        let value = source.processed_flag(0x8c4, 25).map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete.state.as_mut().unwrap().is_physically_pushing = value;
        let value = source.processed_flag(0x8c4, 27).map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete.state.as_mut().unwrap().is_generally_pushing = value;
        let value = source.processed_flag(0x8c4, 26).map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete
            .state
            .as_mut()
            .unwrap()
            .is_generally_pushing_right_foot = value;
        let value = source.processed_flag(0x8c4, 28).map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete.state.as_mut().unwrap().is_generally_braking = value;
        let debug_sim = source.debug_render_local_sim_player().map_err(E::Source)?;
        let local = if debug_sim {
            source.is_local_sim_player()
        } else {
            source.is_local_player()
        }
        .map_err(E::Source)?;
        self.outer_locality_guard(PhysOutComponent::State)
            .map_err(E::Output)?;
        self.concrete.state.as_mut().unwrap().is_debug_render_player = local;
        self.outer_locality_guard(PhysOutComponent::State)
            .map_err(E::Output)?;
        self.concrete.state.as_mut().unwrap().state_update_count =
            source.state_count().map_err(E::Source)?;
        let collide = source.collide_with_world_only().map_err(E::Source)?;
        self.outer_locality_guard(PhysOutComponent::State)
            .map_err(E::Output)?;
        self.concrete
            .state
            .as_mut()
            .unwrap()
            .collide_with_world_only = collide;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        let value = source.processed_flag(0x8cc, 26).map_err(E::Source)?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete
            .state
            .as_mut()
            .unwrap()
            .apply_pinning_optimizations = value;
        let value = source.processed_flag(0x8c8, 12).map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete.state.as_mut().unwrap().has_just_teleported = value;
        let value = source.processed_flag(0x8c4, 21).map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::Air)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete.air.as_mut().unwrap().is_preparing_to_jump = value;
        let value = source.processed_flag(0x8c4, 22).map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::Air)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete.air.as_mut().unwrap().is_starting_to_jump = value;
        self.outer_locality_guard(PhysOutComponent::Ground)
            .map_err(E::Output)?;
        self.concrete.ground.as_mut().unwrap().turn = source.turn().map_err(E::Source)?;
        self.outer_locality_guard(PhysOutComponent::Ground)
            .map_err(E::Output)?;
        let processed = source.capture_processed().map_err(E::Source)?;
        self.concrete.ground.as_mut().unwrap().last_ground_pos =
            source.ground_position(&processed).map_err(E::Source)?;
        let processed = source.capture_processed().map_err(E::Source)?; //52f8, fresh owner
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        let hippy = source
            .captured_flag(&processed, 0x8d0, 15)
            .map_err(E::Source)?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::Ground)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete.ground.as_mut().unwrap().is_hippy_jumping = hippy;
        self.outer_locality_guard(PhysOutComponent::Ground)
            .map_err(E::Output)?;
        self.concrete.ground.as_mut().unwrap().jump_strength =
            source.jump_strength().map_err(E::Source)?;
        let guard = self
            .sample_write_guard()
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        let value = source.processed_flag(0x8c4, 18).map_err(E::Source)?;
        self.check_write_guard(&guard)
            .map_err(|e| E::Output(PriorInputReadError::Access(e)))?;
        self.require_complete_component(PhysOutComponent::State)
            .map_err(|e| E::Output(PriorInputReadError::Component(e)))?;
        self.concrete.state.as_mut().unwrap().is_wiping_out = value;
        let manual = source.balance().map_err(E::Source)? != 0.0;
        self.outer_locality_guard(PhysOutComponent::State)
            .map_err(E::Output)?;
        self.concrete.state.as_mut().unwrap().is_manual = manual;
        Ok(())
    }
}
