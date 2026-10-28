//! Ordered deck line, deck-X validation and acceleration82b2fc24..fdd4.
use super::super::{PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut};
use super::ConcretePhysOutError;
use crate::skeleton_root_transforms::SkeletonRootVector;
use std::convert::Infallible;

pub(crate) trait BoardFillDeckSource {
    type Error;
    type Line;
    fn capture_line(&mut self) -> Result<Self::Line, Self::Error>;
    fn valid(&mut self, line: &Self::Line) -> Result<bool, Self::Error>;
    fn start(&mut self, line: &Self::Line) -> Result<SkeletonRootVector, Self::Error>;
    fn position(&mut self, line: &Self::Line) -> Result<SkeletonRootVector, Self::Error>;
    fn normal(&mut self, line: &Self::Line) -> Result<SkeletonRootVector, Self::Error>;
    fn surface(&mut self, line: &Self::Line) -> Result<u32, Self::Error>;
    fn acceleration(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum BoardFillDeckError<S> {
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
    Source(S),
}
impl<S: std::fmt::Debug> std::fmt::Display for BoardFillDeckError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(e) => write!(f, "board deck output guard: {e:?}"),
            Self::Component(e) => write!(f, "board deck output component: {e:?}"),
            Self::Source(e) => write!(f, "board deck output source: {e:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_board_deck<S: BoardFillDeckSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), BoardFillDeckError<S::Error>> {
        use BoardFillDeckError as E;
        let line = source.capture_line().map_err(E::Source)?; //fc30
        self.guard_board_deck(PhysOutComponent::Ground)?;
        let valid = source.valid(&line).map_err(E::Source)?;
        self.concrete
            .ground
            .as_mut()
            .expect("guarded")
            .below_deck_ground_data_is_valid = valid;
        self.guard_board_deck(PhysOutComponent::Ground)?;
        let start = source.start(&line).map_err(E::Source)?;
        self.concrete
            .ground
            .as_mut()
            .expect("guarded")
            .below_deck_ground_data_start_pos = start;
        // Native fc94 rereads source validity, not the previously published bool.
        if source.valid(&line).map_err(E::Source)? {
            self.guard_board_deck(PhysOutComponent::Ground)?;
            let position = source.position(&line).map_err(E::Source)?;
            self.concrete
                .ground
                .as_mut()
                .expect("guarded")
                .below_deck_ground_data_position = position;
            self.guard_board_deck(PhysOutComponent::Ground)?;
            let normal = source.normal(&line).map_err(E::Source)?;
            self.concrete
                .ground
                .as_mut()
                .expect("guarded")
                .below_deck_ground_data_normal = normal;
            self.guard_board_deck(PhysOutComponent::Ground)?;
            let surface = source.surface(&line).map_err(E::Source)?;
            self.concrete
                .ground
                .as_mut()
                .expect("guarded")
                .below_deck_ground_data_surface_id = surface;
        }
        self.guard_board_deck(PhysOutComponent::SkateboardReckoning)?;
        let x = self
            .concrete
            .skateboard_reckoning
            .as_ref()
            .expect("guarded")
            .deck_position
            .xyz[0];
        // Raw fd6c compares splatted X with itself. Only NaN fails; infinities,
        // signed zero and unknown W do not select this replacement.
        if x.is_nan() {
            self.guard_board_deck(PhysOutComponent::SkateboardReckoning)?;
            self.concrete
                .skateboard_reckoning
                .as_mut()
                .expect("guarded")
                .deck_position = SkeletonRootVector::known([0.0; 4]);
        }
        let guard = self.sample_write_guard().map_err(E::Access)?; //fdac
        let acceleration = source.acceleration().map_err(E::Source)?; //fdb4
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Motion)
            .map_err(E::Component)?;
        self.concrete
            .motion
            .as_mut()
            .expect("guarded")
            .deck_acceleration = acceleration;
        Ok(())
    }
    fn guard_board_deck<S>(
        &self,
        component: PhysOutComponent,
    ) -> Result<(), BoardFillDeckError<S>> {
        self.check_current_guard()
            .map_err(BoardFillDeckError::Access)?;
        self.require_complete_component(component)
            .map_err(BoardFillDeckError::Component)
    }
}

#[cfg(test)]
#[path = "physical_player_phys_out_board_fill_deck_tests.rs"]
mod tests;
