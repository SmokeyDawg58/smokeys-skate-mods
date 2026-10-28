//! Active `Toolkit_CalcGroundJump82b315a8`; caller owns its inactive prefix.
use crate::{
    reckoning_numeric::{self as math, ReckoningClampError, ReckoningNumeric},
    skeleton_processed_contact::jump_events::ground_jump_attributes::{
        GroundJumpAttribute as A, GroundJumpGraph as G,
    },
    skeleton_root_transforms::SkeletonRootVector as V,
};
#[derive(Clone, Copy, Debug)]
pub(crate) enum Vector {
    Normal,
    Prepared,
    Velocity,
    Ground,
    Com,
    ReckoningUp,
    BodyZ,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Scalar {
    SurfaceSpeed,
    Strength,
    Gravity,
    AdjustX,
    AdjustZ,
}
pub(crate) trait Source {
    type Error;
    fn vector(&mut self, field: Vector) -> Result<V, Self::Error>;
    fn scalar(&mut self, field: Scalar) -> Result<f32, Self::Error>;
    fn attribute(&mut self, field: A) -> Result<f32, Self::Error>;
    fn graph(&mut self, field: G, input: f32) -> Result<f32, Self::Error>;
    fn vertical_response_enabled(&mut self) -> Result<bool, Self::Error>;
    fn publish_delta(&mut self, value: V) -> Result<(), Self::Error>;
    fn debug(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum Error<S, N> {
    Source(S),
    Numeric(N),
    Clamp(ReckoningClampError<N>),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for Error<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "ground jump source: {e:?}"),
            Self::Numeric(e) => write!(f, "ground jump numeric: {e:?}"),
            Self::Clamp(e) => write!(f, "ground jump clamp: {e:?}"),
        }
    }
}
const ZERO: V = V::known([0.0; 4]);
const UP: V = V::known([0.0, 1.0, 0.0, 0.0]);
fn fsel(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
fn unit(value: f32) -> f32 {
    let lower = fsel(-value, 0.0, value);
    fsel(1.0 - lower, lower, 1.0)
}
fn subtract(a: V, b: V) -> V {
    a.zip(b, |a, b| a - b)
}
/// Existing explicit host VMX min/max exceptional-value policy, not Xenon
/// NaN payload or denormal parity. Scalar FSEL branches below remain distinct.
fn bounded_dot(value: f32, lower: f32) -> f32 {
    value.max(lower).min(1.0)
}

pub(crate) fn calculate<S: Source, N: ReckoningNumeric>(
    maximum: f32,
    s: &mut S,
    n: &mut N,
) -> Result<V, Error<S::Error, N::Error>> {
    use Error as E;
    let normal = s.vector(Vector::Normal).map_err(E::Source)?;
    let world_y = s
        .graph(G::WorldYResponse, normal.xyz[1])
        .map_err(E::Source)?;
    let prepared = s.vector(Vector::Prepared).map_err(E::Source)?;
    let prepared_dot = n.dot_vector(prepared, normal).map_err(E::Numeric)?;
    let current = s.vector(Vector::Velocity).map_err(E::Source)?;
    let current_dot = n.dot_vector(current, normal).map_err(E::Numeric)?;
    let current_tangent = subtract(current, normal.map(|v| v * current_dot));
    let mut tangent = subtract(prepared, normal.map(|v| v * prepared_dot));
    let difference = subtract(current_tangent, tangent);
    let squared = n.dot_vector(difference, difference).map_err(E::Numeric)?;
    if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
        eprintln!(
            "known air: ground jump error_squared={squared} maximum={maximum} corrected={} current={:?} prepared={:?}",
            squared > maximum,
            current_tangent.xyz,
            tangent.xyz
        );
    }
    // VCMPGT against the scalar itself, not its square; unordered skips.
    if squared > maximum {
        let current_direction =
            math::try_normalize_safe(current_tangent, ZERO, n).map_err(E::Numeric)?;
        let prepared_direction = math::try_normalize_safe(tangent, ZERO, n).map_err(E::Numeric)?;
        let dot = n
            .dot_vector(current_direction, prepared_direction)
            .map_err(E::Numeric)?;
        let pi = f32::from_bits(0x4049_0fdb);
        let seed = n.reciprocal_estimate(pi).map_err(E::Numeric)?;
        let once = seed.mul_add(-seed.mul_add(pi, -1.0), seed);
        let angle = math::try_acos(bounded_dot(dot, 0.0), n).map_err(E::Numeric)?;
        let inverse = once.mul_add(-once.mul_add(pi, -1.0), once);
        let response = s
            .graph(G::TangentLimit, inverse * angle)
            .map_err(E::Source)?;
        let length = math::try_length(tangent, n).map_err(E::Numeric)?;
        tangent = math::try_clamp(current_tangent, length * response, n).map_err(E::Clamp)?;
        s.debug().map_err(E::Source)?;
    }
    let reference = s.attribute(A::ReferenceSpeed).map_err(E::Source)?;
    let speed = s.scalar(Scalar::SurfaceSpeed).map_err(E::Source)?;
    let ratio = unit(speed / reference);
    let minimum_curve = s.graph(G::MinimumHeight, ratio).map_err(E::Source)?;
    let maximum_curve = s.graph(G::MaximumHeight, ratio).map_err(E::Source)?;
    let base = s.attribute(A::BaseHeight).map_err(E::Source)?;
    let minimum = s.attribute(A::MinimumHeight).map_err(E::Source)?;
    let low = (minimum - base).mul_add(minimum_curve, base);
    let maximum = s.attribute(A::MaximumHeight).map_err(E::Source)?;
    let ground = s.vector(Vector::Ground).map_err(E::Source)?;
    let com = s.vector(Vector::Com).map_err(E::Source)?;
    let up = s.vector(Vector::ReckoningUp).map_err(E::Source)?;
    let strength = s.scalar(Scalar::Strength).map_err(E::Source)?;
    let gravity = s.scalar(Scalar::Gravity).map_err(E::Source)?;
    let high = (maximum - base).mul_add(maximum_curve, base);
    let height = n
        .dot_vector(subtract(com, ground), up)
        .map_err(E::Numeric)?;
    let remaining = high.mul_add(strength, (1.0 - strength) * low) - height;
    let radicand = (fsel(remaining, remaining, 0.0) * gravity) * -2.0;
    let estimate = n.reciprocal_sqrt_estimate(radicand).map_err(E::Numeric)?;
    let ballistic = math::length_from_squared_and_estimate(radicand, estimate);
    let response = if s.vertical_response_enabled().map_err(E::Source)? {
        let angle = math::try_acos(bounded_dot(normal.xyz[1], -1.0), n).map_err(E::Numeric)?;
        s.graph(
            G::VerticalResponse,
            unit(angle * f32::from_bits(0x3f22_f983)),
        )
        .map_err(E::Source)?
    } else {
        1.0
    };
    let lower = s.attribute(A::MinimumVerticalResponse).map_err(E::Source)?;
    let launch = ballistic * fsel(response - lower, response, lower);
    let mut velocity = normal.zip(tangent, |u, t| u.mul_add(launch, t));
    let limit = s.attribute(A::MaximumYCompensation).map_err(E::Source)?;
    let product = velocity.xyz[1] * (world_y - 1.0);
    let low = fsel(-product, 0.0, product);
    velocity.xyz[1] += fsel(limit - low, low, limit);
    let along = n.dot_vector(velocity, normal).map_err(E::Numeric)?;
    let body_z = s.vector(Vector::BodyZ).map_err(E::Source)?;
    let side =
        math::try_normalize_safe(math::cross_typed(UP, body_z), ZERO, n).map_err(E::Numeric)?;
    let forward = math::cross_typed(side, UP);
    let side_scale = s.attribute(A::SidewaysScale).map_err(E::Source)?;
    let adjust_x = s.scalar(Scalar::AdjustX).map_err(E::Source)?;
    let side = side.map(|v| v * (-((adjust_x * side_scale) * along)));
    let forward_scale = s.attribute(A::ForwardScale).map_err(E::Source)?;
    let adjust_z = s.scalar(Scalar::AdjustZ).map_err(E::Source)?;
    let forward_factor = (adjust_z * along) * forward_scale;
    let adjustment = forward.zip(side, |forward, side| forward.mul_add(forward_factor, side));
    velocity = velocity.zip(adjustment, |v, a| v + a);
    let current = s.vector(Vector::Velocity).map_err(E::Source)?;
    s.publish_delta(subtract(velocity, current))
        .map_err(E::Source)?;
    s.debug().map_err(E::Source)?;
    Ok(velocity)
}

#[cfg(test)]
#[path = "skateboard_ground_jump_tests.rs"]
mod tests;
