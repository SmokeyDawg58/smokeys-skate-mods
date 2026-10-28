//! Numerical graphs used by grounded Reckoning and Skeleton root transforms.
//!
//! These preserve ordinary f32 products, fused operations and lazy numeric
//! calls. Dot reductions flush denormals like VMX non-Java mode; their
//! rounding, reciprocal estimates, NaN payloads, VMX min/max exceptional
//! selection and VNMSUB exact-zero signs are explicit host boundaries, not
//! Xenon bit parity. All vector arithmetic retains W; only dot3 ignores W.
//! Inputs are exclusive copied values, not aliased native memory that can
//! change between source loads.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "numeric services await the live Reckoning caller")
)]

use crate::simulation_drive_jacobian::{refine_drive_reciprocal, refine_drive_reciprocal_sqrt};
use std::{cmp::Ordering, fmt};

#[path = "reckoning_matrix33.rs"]
pub(crate) mod matrix33;

#[cfg(test)]
#[path = "reckoning_numeric_tests.rs"]
mod tests;

const PI: f32 = f32::from_bits(0x4049_0fdb);
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
const INVERSE_TWO_PI: f32 = f32::from_bits(0x3e22_f983);
const ANGLE_MINIMUM_SQUARED: f32 = f32::from_bits(0x38d1_b717);
const NORMALIZE_EPSILON: f32 = f32::from_bits(0x3586_37bd);
const CLAMP_SMALL_LENGTH: f32 = f32::from_bits(0x3780_0000);

/// Primitive numerical boundaries, not whole substitute geometry operations.
/// Lane carrier for the same scalar graph. Scratch W is never published.
pub(crate) trait ReckoningVector: Copy {
    fn operand(self) -> [f32; 4];
    fn map_lanes(self, f: impl Fn(f32) -> f32) -> Self;
}
impl ReckoningVector for [f32; 4] {
    fn operand(self) -> [f32; 4] {
        self
    }
    fn map_lanes(self, f: impl Fn(f32) -> f32) -> Self {
        self.map(f)
    }
}
impl ReckoningVector for crate::skeleton_root_transforms::SkeletonRootVector {
    fn operand(self) -> [f32; 4] {
        [self.xyz[0], self.xyz[1], self.xyz[2], self.w.unwrap_or(0.0)]
    }
    fn map_lanes(self, f: impl Fn(f32) -> f32) -> Self {
        self.map(f)
    }
}

pub(crate) trait ReckoningNumeric {
    type Error;
    /// XYZ reduction only, even when W is NaN or infinite.
    fn dot_vector(
        &mut self,
        a: impl ReckoningVector,
        b: impl ReckoningVector,
    ) -> Result<f32, Self::Error> {
        self.dot3(a.operand(), b.operand())
    }
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error>;
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error>;
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error>;
    /// Native fctiwz: signed 32-bit truncation, not Rust saturating casts.
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReckoningHostNumericError {
    /// The native exceptional conversion result/FP status is not modeled.
    IntegerConversion { bits: u32 },
}

impl fmt::Display for ReckoningHostNumericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IntegerConversion { bits } => {
                write!(
                    f,
                    "unsupported native angle integer conversion ({bits:08x})"
                )
            }
        }
    }
}
impl std::error::Error for ReckoningHostNumericError {}

/// Separately rounded products and left-associated XYZ sums; ordinary host
/// 1/sqrt and 1/x seeds. Inverse trig below still executes the source polynomial.
pub(crate) struct HostReckoningNumeric;

/// The title never writes VSCR, so VMX runs in the console's non-Java mode:
/// a denormal operand or result is a zero of the same sign. Without this a
/// squared length such as 1e-40 survives into the two-step rsqrt refinement,
/// whose squared seed overflows, and the length becomes NaN where
/// `ClampVectorWithinMaxLength`82e49430 selects zero.
pub(crate) fn vmx_flush(value: f32) -> f32 {
    if value.is_subnormal() {
        f32::from_bits(value.to_bits() & 0x8000_0000)
    } else {
        value
    }
}

/// Shared host reduction policy for callers with several numerical traits.
/// This does not establish Xenon reduction rounding.
/// Same selected host dot4 reduction policy used by ACS and camera quaternion math.
/// This is not a claim of exact VMX horizontal reduction rounding.
pub(crate) fn host_dot4(first: [f32; 4], second: [f32; 4]) -> f32 {
    let [first, second] = [first, second].map(|lanes| lanes.map(vmx_flush));
    vmx_flush(first[0].mul_add(
        second[0],
        first[1].mul_add(second[1], first[2].mul_add(second[2], first[3] * second[3])),
    ))
}

pub(crate) fn host_dot3(left: [f32; 4], right: [f32; 4]) -> f32 {
    let x = vmx_flush(left[0]) * vmx_flush(right[0]);
    let y = vmx_flush(left[1]) * vmx_flush(right[1]);
    let z = vmx_flush(left[2]) * vmx_flush(right[2]);
    vmx_flush((x + y) + z)
}

impl ReckoningNumeric for HostReckoningNumeric {
    type Error = ReckoningHostNumericError;
    fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(host_dot3(left, right))
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        Ok(1.0 / value.sqrt())
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        Ok(1.0 / value)
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "source signed truncation after explicit finite representable-range gate"
    )]
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
        // +2^31 is representable, i32::MAX as f32 rounds to that same excluded
        // value. -2^31 is valid. NaN/infinity must not become saturated integers.
        if !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
            return Err(ReckoningHostNumericError::IntegerConversion {
                bits: value.to_bits(),
            });
        }
        Ok(value as i32)
    }
}

/// `Math::Angle::Normalize`, 0x825ece78..0x825ecef0. Fast interval [-PI,PI),
/// otherwise rounded multiply, signed integer truncation/conversion, negative
/// fused turn subtraction, then at most one upper/lower correction. Not modulo.
#[expect(
    clippy::cast_precision_loss,
    reason = "source explicitly rounds converted signed integer turns to f32"
)]
pub(crate) fn try_normalize_angle<N: ReckoningNumeric>(
    angle: f32,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    if (-PI..PI).contains(&angle) {
        return Ok(angle);
    }
    let turns = angle * INVERSE_TWO_PI;
    let integer_turns = numeric.truncate_i32(turns)?;
    let rounded_turns = integer_turns as f32;
    let remainder = (-rounded_turns).mul_add(TWO_PI, angle);
    // Raw LT-not-set upper arm includes unordered. A successful host conversion
    // ordinarily excludes that case, but the branch itself is not sanitized.
    if remainder.partial_cmp(&PI) != Some(Ordering::Less) {
        Ok(remainder - TWO_PI)
    } else if !matches!(
        remainder.partial_cmp(&-PI),
        Some(Ordering::Greater | Ordering::Equal)
    ) {
        Ok(remainder + TWO_PI)
    } else {
        Ok(remainder)
    }
}

// Literal vector tables 0x823cfe70/80/90, shared by XMVectorASin and ACos.
const ASIN_COEFFICIENTS: [[f32; 4]; 3] = [
    [
        f32::from_bits(0xbd6d_d42d),
        f32::from_bits(0xbed6_5553),
        f32::from_bits(0x3e66_3246),
        f32::from_bits(0x400b_1889),
    ],
    [
        f32::from_bits(0x3f1d_d7b6),
        f32::from_bits(0x4089_80bd),
        f32::from_bits(0xbf98_3f2f),
        f32::from_bits(0xc0d1_360e),
    ],
    [
        f32::from_bits(0xbfaf_4418),
        f32::from_bits(0xc08f_6ad9),
        f32::from_bits(0x3fb5_8485),
        f32::from_bits(0x40af_6ad8),
    ],
];

/// One lane of `XMVectorASin` (0x825e8ce0) / `ACos` (0x824b0278). Each lane is
/// independent; the source callers use replicated scalar inputs/results. This
/// API makes one estimate request for that scalar, not four artificial calls.
/// No domain clamp, libc asin/acos, endpoint repair or zero special case.
fn try_inverse_trig<N: ReckoningNumeric>(
    value: f32,
    cosine: bool,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let square = value * value;
    let absolute = f32::from_bits(value.to_bits() & 0x7fff_ffff);
    let denominator = f32::from_bits(0x3f80_0001) - absolute;
    let cube = square * absolute;
    let weighted_input = (-absolute).mul_add(value, value);
    let mut linear = [0.0; 4];
    for index in (0..4).rev() {
        linear[index] = ASIN_COEFFICIENTS[0][index].mul_add(absolute, ASIN_COEFFICIENTS[1][index]);
    }
    let seed = numeric.reciprocal_sqrt_estimate(denominator)?;
    let half_denominator = denominator * 0.5;
    let half_pi = PI * 0.5;
    let mut quadratic = [0.0; 4];
    for index in (0..4).rev() {
        quadratic[index] = linear[index].mul_add(absolute, ASIN_COEFFICIENTS[2][index]);
    }
    let seed_square = seed * seed;
    let high = quadratic[2].mul_add(cube, quadratic[3]);
    let low = quadratic[0].mul_add(cube, quadratic[1]);
    let correction = (-half_denominator).mul_add(seed_square, 0.5);
    let base = value * high;
    let weighted_low = weighted_input * low;
    let refined = seed.mul_add(correction, seed);
    let result = weighted_low.mul_add(refined, base);
    Ok(if cosine { half_pi - result } else { result })
}

pub(crate) fn try_asin<N: ReckoningNumeric>(value: f32, numeric: &mut N) -> Result<f32, N::Error> {
    try_inverse_trig(value, false, numeric)
}

pub(crate) fn try_acos<N: ReckoningNumeric>(value: f32, numeric: &mut N) -> Result<f32, N::Error> {
    try_inverse_trig(value, true, numeric)
}

/// `Math::AngleBetweenVectors` overloads 0x829b0528 and 0x829b0628. Lazy first
/// dot/gate, second dot/gate, first seed, second seed, one refinement each.
struct AngleInputs {
    a: [f32; 4],
    b: [f32; 4],
    angle: f32,
}

fn try_angle_inputs<N: ReckoningNumeric>(
    a: [f32; 4],
    b: [f32; 4],
    numeric: &mut N,
) -> Result<Option<AngleInputs>, N::Error> {
    let norm_a = numeric.dot3(a, a)?;
    if norm_a.partial_cmp(&ANGLE_MINIMUM_SQUARED) != Some(Ordering::Greater) {
        return Ok(None);
    }
    let norm_b = numeric.dot3(b, b)?;
    if norm_b.partial_cmp(&ANGLE_MINIMUM_SQUARED) != Some(Ordering::Greater) {
        return Ok(None);
    }
    let seed_a = numeric.reciprocal_sqrt_estimate(norm_a)?;
    let seed_b = numeric.reciprocal_sqrt_estimate(norm_b)?;
    let square_a = seed_a * seed_a;
    let square_b = seed_b * seed_b;
    let half_a = seed_a * 0.5;
    let half_b = seed_b * 0.5;
    let residual_a = (-norm_a).mul_add(square_a, 1.0);
    let residual_b = (-norm_b).mul_add(square_b, 1.0);
    let refined_a = half_a.mul_add(residual_a, seed_a);
    let refined_b = half_b.mul_add(residual_b, seed_b);
    let normalized_a = a.map(|lane| lane * refined_a);
    let normalized_b = b.map(|lane| lane * refined_b);
    let dot = numeric.dot3(normalized_a, normalized_b)?;
    // Ordinary max(dot,-1), then min(result,1). Host NaN propagation here is
    // explicit rather than claiming a proven native VMX exceptional selection.
    let lower = if dot < -1.0 { -1.0 } else { dot };
    let bounded = if lower > 1.0 { 1.0 } else { lower };
    let angle = try_acos(bounded, numeric)?;
    Ok(Some(AngleInputs {
        a: normalized_a,
        b: normalized_b,
        angle,
    }))
}

pub(crate) fn try_unsigned_angle<N: ReckoningNumeric>(
    a: impl ReckoningVector,
    b: impl ReckoningVector,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    Ok(try_angle_inputs(a.operand(), b.operand(), numeric)?.map_or(0.0, |input| input.angle))
}

/// Vector2 overload829b03b8: separately rounded two-lane sums and TWO rsqrt
/// refinements per vector (the existing Vector3 overload uses one).
pub(crate) fn try_unsigned_angle_2d<N: ReckoningNumeric>(
    a: [f32; 2], b: [f32; 2], numeric: &mut N,
) -> Result<f32, N::Error> {
    let norm_a = a[0] * a[0] + a[1] * a[1];
    if norm_a.partial_cmp(&ANGLE_MINIMUM_SQUARED) != Some(Ordering::Greater) { return Ok(0.0); }
    let norm_b = b[0] * b[0] + b[1] * b[1];
    if norm_b.partial_cmp(&ANGLE_MINIMUM_SQUARED) != Some(Ordering::Greater) { return Ok(0.0); }
    let seed_a = numeric.reciprocal_sqrt_estimate(norm_a)?;
    let seed_b = numeric.reciprocal_sqrt_estimate(norm_b)?;
    let a = normalize_from_squared_and_estimate([a[0], a[1], 0.0, 0.0], norm_a, seed_a);
    let b = normalize_from_squared_and_estimate([b[0], b[1], 0.0, 0.0], norm_b, seed_b);
    let dot = a[0] * b[0] + a[1] * b[1];
    let lower = if dot < -1.0 { -1.0 } else { dot };
    try_acos(if lower > 1.0 { 1.0 } else { lower }, numeric)
}

pub(crate) fn try_signed_angle<N: ReckoningNumeric>(
    a: impl ReckoningVector,
    b: impl ReckoningVector,
    wind: impl ReckoningVector,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    try_signed_angle_with_wind(a.operand(), b.operand(), || Ok(wind.operand()), numeric)
}

/// Same 0x829b0628 graph, with the source wind load deferred until after `ACos`.
/// A rejected input or earlier numeric failure never reads that retained owner.
pub(crate) fn try_signed_angle_with_wind<N: ReckoningNumeric>(
    a: [f32; 4],
    b: [f32; 4],
    read_wind: impl FnOnce() -> Result<[f32; 4], N::Error>,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let Some(input) = try_angle_inputs(a, b, numeric)? else {
        return Ok(0.0);
    };
    // Source wind read/dot follows ACos, including its estimate callback.
    let wind = read_wind()?;
    let orientation = numeric.dot3(cross_full4(input.a, input.b), wind)?;
    Ok(if orientation < 0.0 {
        TWO_PI - input.angle
    } else {
        input.angle
    })
}

/// YZXW cross graph. W is the rounded a.w*b.w minus the fused identical product,
/// not a forced zero: it can retain a rounding residual or exceptional result.
pub(crate) fn cross_full4(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| {
        let (j, k) = if lane == 3 {
            (3, 3)
        } else {
            ((lane + 1) % 3, (lane + 2) % 3)
        };
        let positive = a[j] * b[k];
        -(a[k].mul_add(b[j], -positive))
    })
}

/// Unsafe two-refinement normalization finish. Allows caller's source stores
/// between dot and estimate; no degenerate/finite guard and all four lanes scale.
pub(crate) fn normalize_from_squared_and_estimate<V: ReckoningVector>(
    vector: V,
    squared: f32,
    estimate: f32,
) -> V {
    let inverse = refine_drive_reciprocal_sqrt(squared, estimate);
    vector.map_lanes(|component| component * inverse)
}

pub(crate) fn try_normalize_unsafe<N: ReckoningNumeric, V: ReckoningVector>(
    vector: V,
    numeric: &mut N,
) -> Result<V, N::Error> {
    let squared = numeric.dot_vector(vector, vector)?;
    let estimate = numeric.reciprocal_sqrt_estimate(squared)?;
    Ok(normalize_from_squared_and_estimate(
        vector, squared, estimate,
    ))
}

/// Math::Matrix44AffineFromLookAtUpEyePos829b0860. Unlike Matrix33FromLookAt,
/// normalize the input direction and first cross only; the second cross is raw.
pub(crate) fn try_look_at_up_eye<N: ReckoningNumeric>(
    look_at: [f32; 4],
    up: [f32; 4],
    eye: [f32; 4],
    numeric: &mut N,
) -> Result<[[f32; 4]; 4], N::Error> {
    let z = try_normalize_unsafe(
        std::array::from_fn::<_, 4, _>(|i| look_at[i] - eye[i]),
        numeric,
    )?;
    let x = try_normalize_unsafe(cross_full4(up, z), numeric)?;
    Ok([x, cross_full4(z, x), z, eye])
}

/// `Math::NormalizeSafe`, 0x824b18e0: squared minus epsilon-squared must be
/// ordered positive; otherwise return the actual fallback's complete raw lanes.
pub(crate) fn try_normalize_safe<N: ReckoningNumeric, V: ReckoningVector>(
    vector: V,
    fallback: impl Into<V>,
    numeric: &mut N,
) -> Result<V, N::Error> {
    try_normalize_safe_with_refinement(vector, fallback, numeric, refine_drive_reciprocal_sqrt)
}

/// Shared source gate with caller-selected reciprocal-square-root refinement.
pub(crate) fn try_normalize_safe_with_refinement<N: ReckoningNumeric, V: ReckoningVector>(
    vector: V,
    fallback: impl Into<V>,
    numeric: &mut N,
    refine: impl FnOnce(f32, f32) -> f32,
) -> Result<V, N::Error> {
    let squared = numeric.dot_vector(vector, vector)?;
    let difference = squared - NORMALIZE_EPSILON * NORMALIZE_EPSILON;
    if difference > 0.0 {
        let estimate = numeric.reciprocal_sqrt_estimate(squared)?;
        let inverse = refine(squared, estimate);
        Ok(vector.map_lanes(|component| component * inverse))
    } else {
        Ok(fallback.into())
    }
}

pub(crate) fn length_from_squared_and_estimate(squared: f32, estimate: f32) -> f32 {
    let inverse = refine_drive_reciprocal_sqrt(squared, estimate);
    let product = squared * inverse;
    if squared == 0.0 { 0.0 } else { product }
}

pub(crate) fn try_length<N: ReckoningNumeric, V: ReckoningVector>(
    vector: V,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let squared = numeric.dot_vector(vector, vector)?;
    let estimate = numeric.reciprocal_sqrt_estimate(squared)?;
    Ok(length_from_squared_and_estimate(squared, estimate))
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ReckoningClampError<E> {
    NegativeMaximum { bits: u32 },
    Numeric(E),
}
impl<E: fmt::Display> fmt::Display for ReckoningClampError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeMaximum { bits } => write!(f, "negative clamp maximum ({bits:08x})"),
            Self::Numeric(error) => write!(f, "reckoning clamp numeric service: {error}"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for ReckoningClampError<E> {}

/// `ClampVectorWithinMaxLength`, 0x82e49430. Negative maximum is a checked host
/// stop at the native assertion. Both post-selection multiplications remain.
pub(crate) fn try_clamp<N: ReckoningNumeric, V: ReckoningVector>(
    vector: V,
    maximum: f32,
    numeric: &mut N,
) -> Result<V, ReckoningClampError<N::Error>> {
    if maximum < 0.0 {
        return Err(ReckoningClampError::NegativeMaximum {
            bits: maximum.to_bits(),
        });
    }
    let length = try_length(vector, numeric).map_err(ReckoningClampError::Numeric)?;
    if length < CLAMP_SMALL_LENGTH {
        return Ok(vector);
    }
    let selected = if maximum - length >= 0.0 {
        length
    } else {
        maximum
    };
    let estimate = numeric
        .reciprocal_estimate(length)
        .map_err(ReckoningClampError::Numeric)?;
    let scaled = vector.map_lanes(|component| component * selected);
    let inverse = refine_drive_reciprocal(length, estimate);
    Ok(scaled.map_lanes(|component| inverse * component))
}

/// Source floor wrap suffix shared by animation tilt and `FillPhysOut` ff54..ff70.
/// Caller performs its distinct angle-to-turns input arithmetic first. Uses the
/// existing host floor policy, not a claim of Xenon exceptional payload parity.
pub(crate) fn wrap_turns_to_radians(turns: f32) -> f32 {
    let fraction = turns - turns.floor();
    let wrapped = if fraction > 0.5 {
        fraction - 1.0
    } else {
        fraction
    };
    wrapped * TWO_PI
}

/// Same cross graph, retaining unknown W rather than publishing scratch data.
pub(crate) fn cross_typed(
    a: impl Into<crate::skeleton_root_transforms::SkeletonRootVector>,
    b: impl Into<crate::skeleton_root_transforms::SkeletonRootVector>,
) -> crate::skeleton_root_transforms::SkeletonRootVector {
    use crate::skeleton_root_transforms::SkeletonRootVector;
    let a = a.into();
    let b = b.into();
    let mut result = SkeletonRootVector::known(cross_full4(a.operand(), b.operand()));
    if a.w.is_none() || b.w.is_none() {
        result.w = None;
    }
    result
}
