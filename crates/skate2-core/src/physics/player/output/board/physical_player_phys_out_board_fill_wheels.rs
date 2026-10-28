//! Actual wheel body positions 0x82b301a4..0x82b30284.
use super::super::{PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut};
use super::ConcretePhysOutError;
use crate::skeleton_root_transforms::SkeletonRootVector;
use std::convert::Infallible;
pub(crate) trait BoardFillWheelPositionSource {
    type Error;
    type Parts;
    type Body;
    fn capture_parts(&mut self) -> Result<Self::Parts, Self::Error>;
    fn body(&mut self, parts: &Self::Parts, wheel: usize) -> Result<Self::Body, Self::Error>;
    fn position(&mut self, body: &Self::Body) -> Result<SkeletonRootVector, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum BoardWheelPositionOutputError<S> {
    Source(S),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
}
impl<S: std::fmt::Debug> std::fmt::Display for BoardWheelPositionOutputError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "wheel position source: {e:?}"),
            Self::Access(e) => write!(f, "wheel position guard: {e:?}"),
            Self::Component(e) => write!(f, "wheel position output: {e:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_board_wheel_positions<S: BoardFillWheelPositionSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), BoardWheelPositionOutputError<S::Error>> {
        use BoardWheelPositionOutputError as E;
        for wheel in 0..4 {
            let parts = source.capture_parts().map_err(E::Source)?;
            // The first body pointer precedes the latch; subsequent pointers
            // follow it. Values always load after the latch, before assertion.
            let first_body = if wheel == 0 {
                Some(source.body(&parts, wheel).map_err(E::Source)?)
            } else {
                None
            };
            let guard = self.sample_write_guard().map_err(E::Access)?;
            let body = match first_body {
                Some(body) => body,
                None => source.body(&parts, wheel).map_err(E::Source)?,
            };
            let position = source.position(&body).map_err(E::Source)?;
            self.check_write_guard(&guard).map_err(E::Access)?;
            self.require_complete_component(PhysOutComponent::SkateboardReckoning)
                .map_err(E::Component)?;
            self.concrete
                .skateboard_reckoning
                .as_mut()
                .expect("guarded Reckoning")
                .wheel_pos[wheel] = position;
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "physical_player_phys_out_board_fill_wheels_tests.rs"]
mod tests;
