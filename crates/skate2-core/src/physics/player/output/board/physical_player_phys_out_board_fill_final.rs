//! Final contiguous board output 0x82b3065c..0x82b3074c.
use super::super::{PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut};
use super::ConcretePhysOutError;
use std::convert::Infallible;
pub(crate) trait BoardFillFinalSource {
    type Error;
    type Body;
    fn capture_body(&mut self) -> Result<Self::Body, Self::Error>;
    fn deck_slide_speed(&mut self, body: &Self::Body) -> Result<f32, Self::Error>;
    fn deck_to_wheel_base(&mut self, body: &Self::Body) -> Result<f32, Self::Error>;
    fn wheel_hardness(&mut self) -> Result<f32, Self::Error>;
    fn wants_respawn(&mut self) -> Result<bool, Self::Error>;
    fn clear_wants_respawn(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum BoardFinalOutputError<S> {
    Source(S),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
}
impl<S: std::fmt::Debug> std::fmt::Display for BoardFinalOutputError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "final board source: {e:?}"),
            Self::Access(e) => write!(f, "final board guard: {e:?}"),
            Self::Component(e) => write!(f, "final board component: {e:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_board_final<S: BoardFillFinalSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), BoardFinalOutputError<S::Error>> {
        use BoardFinalOutputError as E;
        let body = source.capture_body().map_err(E::Source)?;
        let slide = source.deck_slide_speed(&body).map_err(E::Source)?;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Collision)
            .map_err(E::Component)?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded Collision")
            .deck_slide_speed = slide;
        let body = source.capture_body().map_err(E::Source)?;
        let base = source.deck_to_wheel_base(&body).map_err(E::Source)?;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::SkateboardReckoning)
            .map_err(E::Component)?;
        self.concrete
            .skateboard_reckoning
            .as_mut()
            .expect("guarded Reckoning")
            .deck_to_wheel_base = base;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Motion)
            .map_err(E::Component)?;
        let hardness = source.wheel_hardness().map_err(E::Source)?;
        self.concrete
            .motion
            .as_mut()
            .expect("guarded Motion")
            .wheel_hardness = hardness;
        if source.wants_respawn().map_err(E::Source)? {
            self.check_current_guard().map_err(E::Access)?;
            self.require_complete_component(PhysOutComponent::State)
                .map_err(E::Component)?;
            self.concrete
                .state
                .as_mut()
                .expect("guarded State")
                .physics_requested_teleport = true;
            source.clear_wants_respawn().map_err(E::Source)?;
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "physical_player_phys_out_board_fill_final_tests.rs"]
mod tests;
