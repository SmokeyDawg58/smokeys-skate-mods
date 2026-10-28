//! Ordered native board output 0x82b30050..0x82b300e0.
use super::super::{PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut};
use super::ConcretePhysOutError;
use std::convert::Infallible;

pub(crate) trait BoardFillWobbleSource {
    type Error;
    type Wobble;
    fn capture_wobble(&mut self) -> Result<Self::Wobble, Self::Error>;
    fn effect(&mut self, wobble: &Self::Wobble) -> Result<f32, Self::Error>;
    fn tilt(&mut self, wobble: &Self::Wobble) -> Result<f32, Self::Error>;
    fn deck_tilt(&mut self) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum BoardFillWobbleOutputError<S> {
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
    Source(S),
}
impl<S: std::fmt::Debug> std::fmt::Display for BoardFillWobbleOutputError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(e) => write!(f, "board wobble guard: {e:?}"),
            Self::Component(e) => write!(f, "board wobble component: {e:?}"),
            Self::Source(e) => write!(f, "board wobble source: {e:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_board_wobble<S: BoardFillWobbleSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), BoardFillWobbleOutputError<S::Error>> {
        use BoardFillWobbleOutputError as E;
        let wobble = source.capture_wobble().map_err(E::Source)?; //30050
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let effect = source.effect(&wobble).map_err(E::Source)?; //3005c
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Motion)
            .map_err(E::Component)?;
        self.concrete
            .motion
            .as_mut()
            .expect("guarded Motion")
            .speed_wobble_effect = effect;
        let wobble = source.capture_wobble().map_err(E::Source)?; //30080 fresh pointer
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let tilt = source.tilt(&wobble).map_err(E::Source)?; //3008c
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Motion)
            .map_err(E::Component)?;
        self.concrete
            .motion
            .as_mut()
            .expect("guarded Motion")
            .speed_wobble_tilt = tilt;
        // This third read follows the assertion and Motion pointer lookup.
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Motion)
            .map_err(E::Component)?;
        let tilt = source.deck_tilt().map_err(E::Source)?; //300d4
        self.concrete
            .motion
            .as_mut()
            .expect("guarded Motion")
            .deck_tilt = tilt;
        Ok(())
    }
}

#[cfg(test)]
#[path = "physical_player_phys_out_board_fill_wobble_tests.rs"]
mod tests;
