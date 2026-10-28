//! Ground1028..109c: conditional push exit, late stores and grab branch.
use crate::physical_player_physics_state::PhysicsGroundFields;
pub(crate) trait GroundFinishSource {
    type Error;
    fn request_push_exit(&mut self) -> Result<(), Self::Error>;
    fn mark_add_skateboard_error(&mut self) -> Result<(), Self::Error>;
    fn time_delta(&mut self) -> Result<f32, Self::Error>;
    fn flags_8cc(&mut self) -> Result<u32, Self::Error>;
    fn invalidate_grab_query(&mut self) -> Result<(), Self::Error>;
    fn check_for_grabbable_object(
        &mut self,
        ground: &mut PhysicsGroundFields,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum GroundFinishError<S> {
    Source(S),
}
impl<S: std::fmt::Debug> std::fmt::Display for GroundFinishError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "Ground finish source: {e:?}"),
        }
    }
}
pub(crate) fn try_finish_ground<S: GroundFinishSource>(
    ground: &mut PhysicsGroundFields,
    source: &mut S,
) -> Result<(), GroundFinishError<S::Error>> {
    use GroundFinishError as E;
    if ground.request_push_stop {
        source.request_push_exit().map_err(E::Source)?;
    }
    source.mark_add_skateboard_error().map_err(E::Source)?;
    let old = ground.time_in_state; //1044 before DT
    let dt = source.time_delta().map_err(E::Source)?;
    ground.time_in_state = old + dt; //1054 before branch read
    if source.flags_8cc().map_err(E::Source)? & 0x0100_0000 != 0 {
        return source.check_for_grabbable_object(ground).map_err(E::Source);
    }
    source.invalidate_grab_query().map_err(E::Source)
}
