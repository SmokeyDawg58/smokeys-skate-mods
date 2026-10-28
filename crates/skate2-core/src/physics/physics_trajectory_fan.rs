//! Shared `InitTrajectoryInfo82db9018` numeric graph; storage remains caller-owned.
use crate::{
    reckoning_numeric::{
        ReckoningNumeric, cross_typed, length_from_squared_and_estimate, try_length,
        try_normalize_safe,
    },
    reckoning_sin_cos::{ReckoningSinCosNumeric, sine::try_sine, try_cosine},
    skeleton_root_transforms::SkeletonRootVector as Vector,
};

const ZERO: Vector = Vector::known([0.0; 4]);
const UP: Vector = Vector::known([0.0, 1.0, 0.0, 0.0]);

/// Ephemeral arguments, never a parallel trajectory owner.
#[derive(Clone, Copy)]
pub(crate) struct FanLaunch {
    pub(crate) start: Vector,
    pub(crate) before_jump: Vector,
    pub(crate) cone_x: f32,
    pub(crate) cone_z: f32,
    pub(crate) jumped: bool,
    pub(crate) count: u16,
}
pub(crate) trait FanAttributes {
    type Error;
    fn cone_z_graph(&mut self, input: f32) -> Result<f32, Self::Error>;
    fn speed_bounds(&mut self) -> Result<(f32, f32), Self::Error>;
}
pub(crate) enum FanStore {
    Count(i16),
    HighestWallride(f32),
    Velocity(usize, Vector),
}
#[derive(Debug)]
pub(crate) enum FanError<A, N, R> {
    Attribute(A),
    Numeric(N),
    Rounding(R),
}

#[expect(
    clippy::type_complexity,
    reason = "separate attribute, vector and rounding error authorities"
)]
pub(crate) fn initialize_fan<A: FanAttributes, N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    launch: FanLaunch,
    source: &mut A,
    numeric: &mut N,
    rounding: &mut R,
    mut store: impl FnMut(FanStore),
) -> Result<(), FanError<A::Error, N::Error, R::Error>> {
    use FanError::{Attribute, Numeric, Rounding};
    let count = u8::try_from(launch.count.min(7)).expect("source min seven");
    store(FanStore::Count(i16::from(count)));
    store(FanStore::HighestWallride(-1_000_000.0));
    store(FanStore::Velocity(0, launch.start));

    // These operations occur even for zero/one requested candidate.
    let right =
        try_normalize_safe(cross_typed(UP, launch.start), ZERO, numeric).map_err(Numeric)?;
    let forward = try_normalize_safe(cross_typed(right, UP), ZERO, numeric).map_err(Numeric)?;
    // Raw9110/9134/913c/9158: both dots precede both estimates, even !jumped.
    let start_squared = numeric
        .dot_vector(launch.start, launch.start)
        .map_err(Numeric)?;
    let before_squared = numeric
        .dot_vector(launch.before_jump, launch.before_jump)
        .map_err(Numeric)?;
    let start_estimate = numeric
        .reciprocal_sqrt_estimate(start_squared)
        .map_err(Numeric)?;
    let before_estimate = numeric
        .reciprocal_sqrt_estimate(before_squared)
        .map_err(Numeric)?;
    let start_speed = length_from_squared_and_estimate(start_squared, start_estimate);
    let before_speed = length_from_squared_and_estimate(before_squared, before_estimate);
    let cone_speed = if launch.jumped {
        before_speed
    } else {
        start_speed
    };
    let radians = f32::from_bits(0x3c8e_fa35);
    let angle_x = launch.cone_x * radians;
    let graph = source
        .cone_z_graph(cone_speed * f32::from_bits(0x3d4_ccccd))
        .map_err(Attribute)?;
    let angle_z = (launch.cone_z * graph) * radians;
    let (lower, upper) = source.speed_bounds().map_err(Attribute)?;
    // Scalar FSEL preserves its unordered alternatives; Rust clamp/min differ.
    let lower_bounded = if lower - cone_speed >= 0.0 {
        lower
    } else {
        cone_speed
    };
    let speed_factor = if upper - lower_bounded >= 0.0 {
        lower_bounded
    } else {
        upper
    };
    let sine_x = try_sine(angle_x, rounding).map_err(Rounding)?;
    let x_factor = sine_x * speed_factor;
    let x_spread = right.map(|lane| lane * x_factor);
    let sine_z = try_sine(angle_z, rounding).map_err(Rounding)?;
    let z_factor = sine_z * speed_factor;
    let z_spread = forward.map(|lane| lane * z_factor);
    if count <= 1 {
        return Ok(());
    }
    let step = f32::from_bits(0x40c9_0fdb) / f32::from(count - 1); // native fdivs
    let mut angle = 0.0;
    for index in 1..usize::from(count) {
        let cosine = try_cosine(angle, rounding).map_err(Rounding)?;
        let sine = try_sine(angle, rounding).map_err(Rounding)?;
        let first = x_spread.zip(launch.start, |spread, start| spread.mul_add(sine, start));
        let velocity = z_spread.zip(first, |spread, first| spread.mul_add(cosine, first));
        let speed = try_length(velocity, numeric).map_err(Numeric)?;
        let direction = try_normalize_safe(velocity, ZERO, numeric).map_err(Numeric)?;
        let capped = if speed - start_speed >= 0.0 {
            start_speed
        } else {
            speed
        };
        angle += step;
        store(FanStore::Velocity(
            index,
            direction.map(|lane| lane * capped),
        ));
    }
    Ok(())
}
