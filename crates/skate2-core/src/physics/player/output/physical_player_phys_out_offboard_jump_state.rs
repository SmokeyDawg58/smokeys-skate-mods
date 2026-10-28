//! State503 FillPhysOut82d89a88 after the LandingOnDeck manager child: time to
//! land, landing COM height and the landed flag, then OffBoardFeetIK::FillPhysOut
//! 82dc8ba8 writes the two foot-down flags.
use super::super::PhysOutAccessError;
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut};

pub(crate) trait JumpOutputSource {
    type Error;
    fn time_to_land(&mut self) -> Result<f32, Self::Error>;
    fn com_height_on_landing(&mut self) -> Result<f32, Self::Error>;
    fn has_landed(&mut self) -> Result<bool, Self::Error>;
    fn foot_down(&mut self, left: bool) -> Result<bool, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum JumpOutputError<S> {
    Source(S),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<std::convert::Infallible>),
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_offboard_jump_state<S: JumpOutputSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), JumpOutputError<S::Error>> {
        use JumpOutputError as E;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().time_to_land =
            source.time_to_land().map_err(E::Source)?;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete
            .offboard
            .as_mut()
            .unwrap()
            .com_height_on_landing = source.com_height_on_landing().map_err(E::Source)?;
        let landed = source.has_landed().map_err(E::Source)?;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::OffBoard)
            .map_err(E::Component)?;
        self.concrete.offboard.as_mut().unwrap().has_landed = landed;
        for left in [true, false] {
            self.check_current_guard().map_err(E::Access)?;
            self.require_complete_component(PhysOutComponent::OffBoard)
                .map_err(E::Component)?;
            let down = source.foot_down(left).map_err(E::Source)?;
            let out = self.concrete.offboard.as_mut().unwrap();
            if left {
                out.left_foot_down = down;
            } else {
                out.right_foot_down = down;
            }
        }
        Ok(())
    }
}
