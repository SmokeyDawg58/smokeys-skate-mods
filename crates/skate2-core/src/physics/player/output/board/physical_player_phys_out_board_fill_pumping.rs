//! Board output82b2f7fc..f94c. This reads Pumping; it does not run its Update.
use super::super::{PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut};
use super::ConcretePhysOutError;
use crate::skeleton_root_transforms::SkeletonRootVector;
use std::convert::Infallible;

pub(crate) trait BoardFillPumpingSource {
    type Error;
    type Pumping;
    fn capture_pumping(&mut self) -> Result<Self::Pumping, Self::Error>;
    fn pump_amount(&mut self, pumping: &Self::Pumping) -> Result<f32, Self::Error>;
    fn intentional_pump(&mut self) -> Result<bool, Self::Error>;
    fn abort_push(&mut self) -> Result<bool, Self::Error>;
    fn captured_body_position(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn board_flipped(&mut self) -> Result<bool, Self::Error>;
    fn has_jump_delta(&mut self) -> Result<bool, Self::Error>;
    fn jump_delta(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum BoardFillPumpingOutputError<S> {
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
    Source(S),
}
impl<S: std::fmt::Debug> std::fmt::Display for BoardFillPumpingOutputError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(e) => write!(f, "board Pumping output guard: {e:?}"),
            Self::Component(e) => write!(f, "board Pumping output component: {e:?}"),
            Self::Source(e) => write!(f, "board Pumping output source: {e:?}"),
        }
    }
}

impl PhysicalPlayerPhysOut {
    /// Source latch samples precede reads, but assertions follow those reads.
    pub(crate) fn try_fill_board_pumping<S: BoardFillPumpingSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), BoardFillPumpingOutputError<S::Error>> {
        use BoardFillPumpingOutputError as E;
        let pumping = source.capture_pumping().map_err(E::Source)?; //f7fc
        let guard = self.sample_write_guard().map_err(E::Access)?; //f800
        let amount = source.pump_amount(&pumping).map_err(E::Source)?; //f808
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Motion)
            .map_err(E::Component)?;
        self.concrete
            .motion
            .as_mut()
            .expect("guarded Motion")
            .pump_potential = amount;

        let guard = self.sample_write_guard().map_err(E::Access)?; //f82c
        let intentional = source.intentional_pump().map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Motion)
            .map_err(E::Component)?;
        self.concrete
            .motion
            .as_mut()
            .expect("guarded Motion")
            .is_intentional_pump = intentional;

        let guard = self.sample_write_guard().map_err(E::Access)?; //f860
        let abort = source.abort_push().map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Skeleton)
            .map_err(E::Component)?;
        self.concrete
            .skeleton
            .as_mut()
            .expect("guarded Skeleton")
            .abort_push = abort;

        let guard = self.sample_write_guard().map_err(E::Access)?; //f890
        let position = source.captured_body_position().map_err(E::Source)?; //f894
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::SkateboardReckoning)
            .map_err(E::Component)?;
        self.concrete
            .skateboard_reckoning
            .as_mut()
            .expect("guarded Reckoning")
            .deck_position = position;

        let guard = self.sample_write_guard().map_err(E::Access)?; //f8c4
        let flipped = source.board_flipped().map_err(E::Source)?; //f8cc
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::SkateboardReckoning)
            .map_err(E::Component)?;
        self.concrete
            .skateboard_reckoning
            .as_mut()
            .expect("guarded Reckoning")
            .is_transform_flipped = flipped;

        // Only select the source address before the guard. True JumpDV's payload
        // is loaded at f944, after the assertion and Air pointer lookup.
        let has_delta = source.has_jump_delta().map_err(E::Source)?;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Air)
            .map_err(E::Component)?;
        let delta = if has_delta {
            source.jump_delta().map_err(E::Source)?
        } else {
            SkeletonRootVector::known([0.0; 4])
        };
        self.concrete.air.as_mut().expect("guarded Air").jump_dv = delta;
        Ok(())
    }
}

#[cfg(test)]
#[path = "physical_player_phys_out_board_fill_pumping_tests.rs"]
mod tests;
