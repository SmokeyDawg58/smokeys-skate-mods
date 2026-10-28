//! UNREGISTERED complete AntiFlipTorque82b299a8..29dcc, hidden result r3/board r4.
use crate::{
    reckoning_numeric::{
        ReckoningNumeric, cross_typed, try_acos, try_normalize_safe_with_refinement,
    },
    skeleton_root_transforms::SkeletonRootVector as Vector,
};

pub(crate) trait AntiFlipSource {
    type Error;
    fn manual_flag(&mut self) -> Result<bool, Self::Error>;
    fn balance(&mut self) -> Result<f32, Self::Error>;
    fn deck_z(&mut self) -> Result<Vector, Self::Error>;
    fn up(&mut self) -> Result<Vector, Self::Error>;
    fn deck_x(&mut self) -> Result<Vector, Self::Error>;
    /// Fresh actual global feet layout capture at each call; X=b0 then60.
    fn feet_graph(&mut self, x_offset: usize, input: f32) -> Result<f32, Self::Error>;
    fn draw(&mut self) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum AntiFlipError<S, N> {
    Source(S),
    Numeric(N),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for AntiFlipError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "anti-flip input: {e:?}"),
            Self::Numeric(e) => write!(f, "anti-flip numeric: {e:?}"),
        }
    }
}
fn refine(squared: f32, seed: f32) -> f32 {
    let residual = -(squared.mul_add(seed * seed, -1.0));
    let first = (seed * 0.5).mul_add(residual, seed);
    let residual = -(squared.mul_add(first * first, -1.0));
    (first * 0.5).mul_add(residual, first)
}
fn scale(v: Vector, s: f32) -> Vector {
    v.map(|x| x * s)
}
fn subtract(a: Vector, b: Vector) -> Vector {
    Vector {
        xyz: std::array::from_fn(|i| a.xyz[i] - b.xyz[i]),
        w: a.w.zip(b.w).map(|(a, b)| a - b),
    }
}
fn add_product(a: Vector, s: f32, b: Vector) -> Vector {
    Vector {
        xyz: std::array::from_fn(|i| a.xyz[i].mul_add(s, b.xyz[i])),
        w: a.w.zip(b.w).map(|(a, b)| a.mul_add(s, b)),
    }
}
/// Ordinary host VMX max(0,dot), then min(1,result) operand-selection policy.
/// Equality keeps right operand, including -0. NaN remains NaN; no domain repair.
fn positive_cosine(dot: f32) -> f32 {
    let lower = if 0.0 > dot { 0.0 } else { dot };
    if 1.0 < lower { 1.0 } else { lower }
}
pub(crate) fn try_antiflip<S: AntiFlipSource, N: ReckoningNumeric>(
    source: &mut S,
    numeric: &mut N,
    output: &mut Vector,
) -> Result<(), AntiFlipError<S::Error, N::Error>> {
    use AntiFlipError as E;
    let manual = source.manual_flag().map_err(E::Source)?; //99e0 precedes output
    let zero = Vector::known([0.0; 4]);
    *output = zero; //9a00
    if manual {
        return Ok(());
    }
    let pitch_enabled = if source.balance().map_err(E::Source)? == 0.0 {
        1.0
    } else {
        0.0
    };
    let z = source.deck_z().map_err(E::Source)?; //9a44
    let up = source.up().map_err(E::Source)?; //9a4c, Processed220
    let z_dot = numeric.dot_vector(up, z).map_err(E::Numeric)?;
    let x = source.deck_x().map_err(E::Source)?; //9a60, after first dot
    let planar_z =
        try_normalize_safe_with_refinement(subtract(z, scale(up, z_dot)), zero, numeric, refine)
            .map_err(E::Numeric)?;
    let x_dot = numeric.dot_vector(up, x).map_err(E::Numeric)?;
    let planar_x =
        try_normalize_safe_with_refinement(subtract(x, scale(up, x_dot)), zero, numeric, refine)
            .map_err(E::Numeric)?;
    let z_cos = numeric.dot_vector(z, planar_z).map_err(E::Numeric)?;
    let x_cos = numeric.dot_vector(x, planar_x).map_err(E::Numeric)?;
    let z_cos = positive_cosine(z_cos);
    let x_cos = positive_cosine(x_cos);
    let pitch_sign = numeric
        .dot_vector(x, cross_typed(z, planar_z))
        .map_err(E::Numeric)?;
    let roll_sign = numeric
        .dot_vector(z, cross_typed(x, planar_x))
        .map_err(E::Numeric)?;
    // Source first acos then scale, second acos then scale. Both inputs are
    // captured before either call; no later re-read of projection vectors.
    let pitch = f32::from_bits(0x3f22_f983) * try_acos(z_cos, numeric).map_err(E::Numeric)?;
    let roll = f32::from_bits(0x3f22_f983) * try_acos(x_cos, numeric).map_err(E::Numeric)?;
    let pitch_sign = if pitch_sign >= 0.0 { 1.0 } else { -1.0 };
    let roll_sign = if roll_sign >= 0.0 { 1.0 } else { -1.0 };
    let pitch_value = source.feet_graph(0xb0, pitch).map_err(E::Source)?;
    let roll_value = source.feet_graph(0x60, roll).map_err(E::Source)?;
    let pitch_scaled = pitch_value * pitch_sign;
    let roll_scaled = roll_value * roll_sign;
    let pitch_scaled = pitch_scaled * pitch_enabled;
    let draw = source.draw().map_err(E::Source)?; //9cc4/c8 BEFORE final vector store
    *output = add_product(x, pitch_scaled, scale(z, roll_scaled)); //9cdc/e4/e8
    if draw {
        source.require_overlay().map_err(E::Source)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "skateboard_antiflip_torque_tests.rs"]
mod tests;
