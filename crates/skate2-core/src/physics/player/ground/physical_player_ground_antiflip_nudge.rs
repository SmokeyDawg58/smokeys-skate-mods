//! Unregistered actual Ground.UpdateAntiFlipNudge82d8d508..d71c.
use crate::{
    physical_player_phys_out::construct::board_fill::board_speed_from_squared_and_estimate,
    physical_player_physics_state::PhysicsGroundFields, reckoning_numeric::ReckoningNumeric,
    skateboard_foot_forces::SkateboardFootForce, skateboard_force_queue::SkateboardForceQueue,
    skeleton_root_transforms::SkeletonRootVector,
};
pub(crate) trait NudgeSource {
    type Error;
    type Target;
    fn minimum_frames(&mut self) -> Result<i32, Self::Error>;
    fn deck_speed(&mut self) -> Result<f32, Self::Error>;
    fn maximum_speed(&mut self) -> Result<f32, Self::Error>;
    fn deck_z(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn capture_strength(&mut self) -> Result<Self::Target, Self::Error>;
    fn load_strength(&mut self, target: &Self::Target) -> Result<f32, Self::Error>;
    fn debug(&mut self) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum NudgeError<S, N> {
    MissingTorque,
    Source(S),
    Numeric(N),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for NudgeError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTorque => f.write_str("nudge missing retained Ground+90"),
            Self::Source(e) => write!(f, "nudge source: {e:?}"),
            Self::Numeric(e) => write!(f, "nudge numeric: {e:?}"),
        }
    }
}
#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "native NOTLT and NOTGT include unordered values"
)]
pub(crate) fn try_update_nudge<S: NudgeSource, N: ReckoningNumeric>(
    ground: &mut PhysicsGroundFields,
    source: &mut S,
    queue: &mut SkateboardForceQueue,
    numeric: &mut N,
) -> Result<(), NudgeError<S::Error, N::Error>> {
    use NudgeError as E;
    let torque = ground.anti_flip_torque.ok_or(E::MissingTorque)?;
    let squared = numeric.dot_vector(torque, torque).map_err(E::Numeric)?;
    ground.anti_flip_frames = if squared > 0.0 {
        ground.anti_flip_frames.wrapping_add(1)
    } else {
        0
    };
    let minimum = source.minimum_frames().map_err(E::Source)?;
    if ground.anti_flip_frames <= minimum {
        return Ok(());
    }
    let speed = source.deck_speed().map_err(E::Source)?;
    let maximum = source.maximum_speed().map_err(E::Source)?;
    if !(speed < maximum) {
        return Ok(());
    } // raw4098 NOTLT includes unordered
    let z = source.deck_z().map_err(E::Source)?;
    let mut direction = if z.xyz[1] > 0.0 {
        z.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000))
    } else {
        z
    };
    direction.xyz[1] = 0.0;
    let squared = numeric
        .dot_vector(direction, direction)
        .map_err(E::Numeric)?;
    if !(squared > f32::from_bits(0x3a83_126f)) {
        return Ok(());
    }
    let target = source.capture_strength().map_err(E::Source)?; //d670 before estimate
    let estimate = numeric
        .reciprocal_sqrt_estimate(squared)
        .map_err(E::Numeric)?;
    let strength = source.load_strength(&target).map_err(E::Source)?; //d690 before refinements
    // Same two VNMSUB refinements, product and zero-select as BoardFill's helper.
    let length = board_speed_from_squared_and_estimate(squared, estimate);
    let scale = strength / length;
    let force = SkateboardFootForce {
        local_space_position: SkeletonRootVector::known([0.0; 4]),
        world_space_force: direction.map(|v| v * scale),
    };
    queue.add(0x11, force); //capacity failure is native silent ignore
    ground.anti_flip_nudge = true; //even if the queue was full
    if source.debug().map_err(E::Source)? {
        source.require_overlay().map_err(E::Source)?;
    }
    Ok(())
}
