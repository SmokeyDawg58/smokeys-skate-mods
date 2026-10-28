//! Selected retained hang state and complete `ConsiderPinning`, through a998.
use crate::{
    physical_player_physics_state::PhysicsGroundFields, reckoning_numeric::ReckoningNumeric,
    skeleton_root_transforms::SkeletonRootVector as Vector,
};

pub(crate) trait GroundHangPinSource {
    type Error;
    fn hang_flags(&mut self) -> Result<u32, Self::Error>;
    fn hang_frames(&mut self) -> Result<i32, Self::Error>;
    fn deck_speed(&mut self) -> Result<f32, Self::Error>;
    fn wipeout_frames(&mut self) -> Result<i32, Self::Error>;
    fn teleport_frames(&mut self) -> Result<i32, Self::Error>;
    fn maximum_pin_frames(&mut self) -> Result<i32, Self::Error>;
    fn maximum_pin_start_frames(&mut self) -> Result<i32, Self::Error>;
    fn flags_8c8(&mut self) -> Result<u32, Self::Error>;
    fn time_delta(&mut self) -> Result<f32, Self::Error>;
    fn deck_y(&mut self) -> Result<f32, Self::Error>;
    fn deck_position(&mut self) -> Result<Vector, Self::Error>;
    fn set_velocity(&mut self, velocity: Vector) -> Result<(), Self::Error>;
}
#[derive(Debug, PartialEq)]
pub(crate) enum HangPinPending {
    /// Before d758: nonempty hang predicates require additional live cvars.
    ActiveHangPredicates,
    /// At d7c0: pulse target/value and primitive helpers are not admitted.
    PulseProducer,
    /// At d91c: real retained hang force application and overlay service.
    HangForce,
    /// At d994: newly published active boardslide/geometry route.
    HangGeometry,
    /// At da4c: actual Wipeout owner request12.
    Wipeout,
    /// At da98: active halfpipe predicates and primitive location.
    Halfpipe,
}
#[derive(Debug)]
pub(crate) enum HangPinError<S, N> {
    Missing(&'static str),
    Source(S),
    Numeric(N),
    Pending(HangPinPending),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for HangPinError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "Ground hang/pin missing {s}"),
            Self::Source(e) => write!(f, "Ground hang/pin source: {e:?}"),
            Self::Numeric(e) => write!(f, "Ground pin numeric: {e:?}"),
            Self::Pending(p) => write!(f, "Ground hang service pending: {p:?}"),
        }
    }
}
pub(crate) fn try_hang_and_halfpipe<S: GroundHangPinSource, N>(
    ground: &mut PhysicsGroundFields,
    source: &mut S,
) -> Result<(), HangPinError<S::Error, N>> {
    use HangPinError as E;
    const EXISTS: u32 = 0x0800_0000;
    if source.hang_flags().map_err(E::Source)? & EXISTS != 0 {
        return Err(E::Pending(HangPinPending::ActiveHangPredicates));
    }
    ground.hang_timer = Some(0); //d7a4, even if subsequent cvar fails
    if source.hang_frames().map_err(E::Source)? == 0 {
        return Err(E::Pending(HangPinPending::PulseProducer));
    }
    let countdown = ground
        .hang_pulse_countdown
        .ok_or(E::Missing("Ground countdown104"))?;
    if countdown > 0 {
        if source.hang_flags().map_err(E::Source)? & EXISTS != 0 {
            return Err(E::Pending(HangPinPending::HangForce));
        }
        let next = countdown.wrapping_sub(1);
        ground.hang_pulse_countdown = Some(next); //d95c, also when exists cleared
        if next == 0 {
            ground.hang_timer = Some(0);
        }
    }
    let speed = source.deck_speed().map_err(E::Source)?; //always d974
    if speed < f32::from_bits(0x3f8c_cccd) && source.hang_flags().map_err(E::Source)? & EXISTS != 0
    {
        return Err(E::Pending(HangPinPending::HangGeometry));
    }
    ground.hung_up_counter = 0; //da34 before cvar
    if ground.hung_up_counter > source.wipeout_frames().map_err(E::Source)? {
        return Err(E::Pending(HangPinPending::Wipeout));
    }
    // Separate call da78: even the default route rereads actual flags.
    if source.hang_flags().map_err(E::Source)? & EXISTS != 0 {
        return Err(E::Pending(HangPinPending::Halfpipe));
    }
    Ok(())
}
pub(crate) fn try_consider_pinning<S: GroundHangPinSource, N: ReckoningNumeric>(
    ground: &mut PhysicsGroundFields,
    source: &mut S,
    numeric: &mut N,
) -> Result<(), HangPinError<S::Error, N::Error>> {
    use HangPinError as E;
    let human = ground.is_human;
    let frames = source.teleport_frames().map_err(E::Source)?; //unconditional dc78
    if human != 0 && frames < source.maximum_pin_frames().map_err(E::Source)? {
        let allowed =
            ground.was_pinning || frames < source.maximum_pin_start_frames().map_err(E::Source)?;
        let controls = frames > 0 && source.flags_8c8().map_err(E::Source)? & 0x0100_0000 == 0;
        ground.player_provided_controls = ground.player_provided_controls || controls; //dd10
        if allowed && ground.spawn_coordinates_set && !ground.player_provided_controls {
            let dt = source.time_delta().map_err(E::Source)?;
            let y = source.deck_y().map_err(E::Source)?;
            let target = Vector::known([ground.valid_spawn_x, y, ground.valid_spawn_z, 0.0]);
            let position = source.deck_position().map_err(E::Source)?;
            // Native loads the position before the estimate and subtracts afterward.
            let estimate = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?;
            let difference = target.zip(position, |a, b| a - b);
            let residual = -estimate.mul_add(dt, -1.0);
            let first = estimate.mul_add(residual, estimate);
            let residual = -first.mul_add(dt, -1.0);
            let inverse = first.mul_add(residual, first);
            source
                .set_velocity(difference.map(|v| inverse * v))
                .map_err(E::Source)?;
            ground.is_pinning = true; //only after actual SetVelocity
        }
    }
    ground.was_pinning = ground.is_pinning;
    Ok(())
}
