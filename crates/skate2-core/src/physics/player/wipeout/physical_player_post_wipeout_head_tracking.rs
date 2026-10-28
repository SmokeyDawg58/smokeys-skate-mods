//! Child of `skeleton_head_tracking`: native Update82dca850.
//! Shared PID and valid-target orientation; enabled debug arithmetic remains pending.
use crate::{
    reckoning_numeric::{ReckoningNumeric, try_length},
    reckoning_pid::{PhysicsControllerData, VectorPidFilter},
    skeleton_ik_line_frame::normalize_safe,
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};

#[cfg(test)]
#[path = "physical_player_post_wipeout_head_tracking_tests.rs"]
mod tests;

pub(crate) trait HeadTrackingSource {
    type Error;
    type ForceTarget;
    type AngleTarget;
    fn controller_layout(&mut self) -> Result<[f32; 4], Self::Error>;
    fn capture_force_look_at(&mut self) -> Result<Self::ForceTarget, Self::Error>;
    fn capture_maximum_target_angle(&mut self) -> Result<Self::AngleTarget, Self::Error>;
    fn force_look_at(&mut self, target: &Self::ForceTarget) -> Result<bool, Self::Error>;
    fn maximum_target_angle(&mut self, target: &Self::AngleTarget) -> Result<f32, Self::Error>;
    fn forced_target(&mut self) -> Result<[f32; 4], Self::Error>;
    fn acceleration_cap(&mut self) -> Result<f32, Self::Error>;
    fn draw_head_adjust(&mut self) -> Result<bool, Self::Error>;
    fn clamp_ideal(&mut self) -> Result<bool, Self::Error>;
    fn largest_up_angle(&mut self) -> Result<f32, Self::Error>;
    fn rotation_limits(&mut self) -> Result<[[f32; 4]; 2], Self::Error>;
}

/// References into the actual constructor owner; this is not another filter.
pub(crate) struct HeadTrackingState<'a> {
    pub(crate) filter: &'a mut Option<VectorPidFilter>,
    pub(crate) target_position: &'a mut Option<Vector>,
    pub(crate) rotation: &'a mut Option<[f32; 4]>,
    pub(crate) rotation_velocity: &'a mut Option<[f32; 4]>,
    pub(crate) valid_target: &'a mut Option<bool>,
}

#[derive(Debug)]
pub(crate) enum HeadTrackingError<S, N> {
    Source(S),
    Numeric(N),
    Missing(&'static str),
    DebugArithmeticPending,
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for HeadTrackingError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "HeadTracking source: {e:?}"),
            Self::Numeric(e) => write!(f, "HeadTracking numeric: {e:?}"),
            Self::Missing(field) => write!(f, "HeadTracking missing {field}"),
            Self::DebugArithmeticPending => {
                f.write_str("HeadTracking debug arithmetic82dcb128..b288 pending")
            }
        }
    }
}

/// Both input matrices must be the actual captured caller values even when the
/// invalid-target branch does not consume their coordinates. `output` is the
/// caller's actual axisAngleOut local: native publishes it before debug/clear.
#[expect(
    clippy::needless_pass_by_value,
    reason = "consume the temporary bundle of exclusive owner borrows"
)]
pub(crate) fn try_update<S: HeadTrackingSource, N: ReckoningNumeric>(
    state: HeadTrackingState<'_>,
    world_to_animation: &Matrix,
    head_transform: &Matrix,
    output: &mut Vector,
    source: &mut S,
    numeric: &mut N,
) -> Result<(), HeadTrackingError<S::Error, N::Error>> {
    use HeadTrackingError as E;
    // a8d8/a8e8/a8f0 capture the head basis before control stores; shared input
    // borrows exclude replacement between these native loads and the branch.
    let _captured_basis = &head_transform.columns()[..3];
    let control = source.controller_layout().map_err(E::Source)?;
    let filter = state.filter.as_mut().ok_or(E::Missing("PID constructor"))?;
    filter.replace_control(PhysicsControllerData {
        proportional: control[0],
        integral: control[1],
        derivative: control[2],
        integral_blend: control[3],
    });
    //82dca92c/930 capture both allocations before value loads934/938.
    let force_target = source.capture_force_look_at().map_err(E::Source)?;
    let angle_target = source.capture_maximum_target_angle().map_err(E::Source)?;
    let force = source.force_look_at(&force_target).map_err(E::Source)?;
    // This read/multiply is unconditional, including valid_target=false.
    let maximum_radians = source
        .maximum_target_angle(&angle_target)
        .map_err(E::Source)?
        * f32::from_bits(0x3c8e_fa35);
    if force {
        *state.target_position = Some(Vector::known(source.forced_target().map_err(E::Source)?));
        *state.valid_target = Some(true);
    }
    let mut target_rotation = Vector::known([0.0; 4]);
    if state.valid_target.ok_or(E::Missing("valid target"))? {
        use crate::reckoning_numeric::{cross_typed, try_acos, wrap_turns_to_radians};
        let target = state.target_position.ok_or(E::Missing("target position"))?;
        let c = world_to_animation.columns();
        let mut position = c[0].zip(c[3], |a, b| a.mul_add(target.xyz[0], b));
        position = c[1].zip(position, |a, b| a.mul_add(target.xyz[1], b));
        position = c[2].zip(position, |a, b| a.mul_add(target.xyz[2], b));
        let h = head_transform.columns();
        let mut y =
            normalize_safe(position.zip(h[3], |a, b| a - b), numeric).map_err(E::Numeric)?;
        let mut z = normalize_safe(cross_typed(h[0], y), numeric).map_err(E::Numeric)?;
        let mut x = normalize_safe(cross_typed(y, z), numeric).map_err(E::Numeric)?;
        // Native performs these two normalized horizontal scratch projections,
        // although the subsequent angle dot consumes the original full vectors.
        let mut flat_head = h[1];
        flat_head.xyz[1] = 0.0;
        let mut flat_target = y;
        flat_target.xyz[1] = 0.0;
        let _ = normalize_safe(flat_head, numeric).map_err(E::Numeric)?;
        let _ = normalize_safe(flat_target, numeric).map_err(E::Numeric)?;
        let dot = numeric.dot_vector(h[1], y).map_err(E::Numeric)?;
        let angle = try_acos(dot, numeric).map_err(E::Numeric)?;
        let normalized = wrap_turns_to_radians(angle * f32::from_bits(0x3e22f983));
        if maximum_radians > normalized.abs() {
            if source.clamp_ideal().map_err(E::Source)? && h[0].xyz[1] > x.xyz[1] {
                let up = Vector::known([0.0, 1.0, 0.0, 0.0]);
                let old_dot = numeric.dot_vector(h[0], up).map_err(E::Numeric)?;
                let old_angle = wrap_turns_to_radians(
                    try_acos(old_dot, numeric).map_err(E::Numeric)? * f32::from_bits(0x3e22f983),
                );
                let limit =
                    source.largest_up_angle().map_err(E::Source)? * f32::from_bits(0x3c8efa35);
                let limit = if old_angle > limit { old_angle } else { limit };
                let ideal_dot = numeric.dot_vector(x, up).map_err(E::Numeric)?;
                let ideal_angle = wrap_turns_to_radians(
                    try_acos(ideal_dot, numeric).map_err(E::Numeric)? * f32::from_bits(0x3e22f983),
                );
                if ideal_angle > limit {
                    x = crate::reckoning::air::try_clamp_air_angle(
                        x,
                        up,
                        limit,
                        numeric,
                        &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
                    )
                    .map_err(|error| match error {
                        crate::reckoning::air::AirVectorError::Numeric(e) => E::Numeric(e),
                        crate::reckoning::air::AirVectorError::SinCos(e) => match e {},
                    })?;
                    y = normalize_safe(cross_typed(z, x), numeric).map_err(E::Numeric)?;
                    z = normalize_safe(cross_typed(x, y), numeric).map_err(E::Numeric)?;
                }
            }
            let yy = numeric.dot_vector(y, y).map_err(E::Numeric)?;
            let xx = numeric.dot_vector(x, x).map_err(E::Numeric)?;
            let zz = numeric.dot_vector(z, z).map_err(E::Numeric)?;
            if (xx * yy) * zz > 0.001 {
                let mut relative = [0.0; 16];
                for (column, ideal) in [x, y, z].iter().enumerate() {
                    for row in 0..3 {
                        // Head inverse rotation composed with ideal rotation.
                        relative[column * 4 + row] = h[row].xyz[2].mul_add(
                            ideal.xyz[2],
                            h[row].xyz[1].mul_add(ideal.xyz[1], h[row].xyz[0] * ideal.xyz[0]),
                        );
                    }
                }
                let q = crate::world::try_simulation_quaternion_candidate(relative, |values| {
                    let mut seeds = [0.0; 4];
                    for i in 0..4 {
                        seeds[i] = numeric.reciprocal_sqrt_estimate(values[i])?;
                    }
                    Ok(seeds)
                })
                .map_err(E::Numeric)?;
                let squared = 1.0 - q[3] * q[3];
                let estimate = numeric
                    .reciprocal_sqrt_estimate(squared)
                    .map_err(E::Numeric)?;
                let inverse = crate::simulation_drive_jacobian::refine_drive_reciprocal_sqrt(
                    squared, estimate,
                );
                let angle = try_acos(q[3], numeric).map_err(E::Numeric)? * 2.0;
                let axis = if squared >= f32::from_bits(0x37800000) {
                    Vector::known(q).map(|v| v * inverse)
                } else {
                    Vector::known([1.0, 0.0, 0.0, 0.0])
                };
                target_rotation = axis.map(|v| v * angle);
                let limits = source.rotation_limits().map_err(E::Source)?;
                target_rotation =
                    target_rotation.zip(
                        Vector::known(limits[1]),
                        |v, max| if max < v { max } else { v },
                    );
                target_rotation =
                    target_rotation.zip(
                        Vector::known(limits[0]),
                        |v, min| if min > v { min } else { v },
                    );
            }
        }
    }
    // afc4..afe0 executes even when the constructor target is invalid.
    let filtered = filter.update(target_rotation);
    let cap = source.acceleration_cap().map_err(E::Source)?;
    let old_rotation = Vector::known(state.rotation.ok_or(E::Missing("rotation"))?);
    let old_velocity = Vector::known(
        state
            .rotation_velocity
            .ok_or(E::Missing("rotation velocity"))?,
    );
    let delta = filtered.zip(old_rotation, |a, b| a - b);
    let acceleration = delta.zip(old_velocity, |a, b| a - b);
    // b070..b0b0 computes length with two rsqrt refinements even at zero,
    // then b0b4 independently NormalizeSafe (a second dot, conditional seed).
    let length = try_length(acceleration, numeric).map_err(E::Numeric)?;
    let direction = normalize_safe(acceleration, numeric).map_err(E::Numeric)?;
    // Scalar FSEL, not min: unordered difference selects length; negative caps
    // are neither rejected nor silently clamped to zero by the original.
    let selected = if length - cap >= 0.0 { cap } else { length };
    let base = old_rotation.zip(old_velocity, |a, b| a + b);
    let next = direction.zip(base, |a, b| a.mul_add(selected, b));
    let velocity = next.zip(old_rotation, |a, b| a - b);
    *state.rotation_velocity = Some(velocity.known_lanes().ok_or(E::Missing("velocity W"))?);
    *state.rotation = Some(next.known_lanes().ok_or(E::Missing("rotation W"))?);
    *output = next;
    if source.draw_head_adjust().map_err(E::Source)? {
        return Err(E::DebugArithmeticPending);
    }
    *state.valid_target = Some(false);
    Ok(())
}
