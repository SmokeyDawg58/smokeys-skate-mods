//! Eight actual `ComputeExpressions_Individual827d99a0` corrections.
use super::corrections::CorrectionChannels;
use super::{
    move_root::matrix_inverse::PresentationMatrixNumeric,
    twist::{TwistError, try_twist_channels},
};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    reckoning_sin_cos::ReckoningSinCosNumeric,
};
#[cfg(test)]
use skate2_formats::andale::AndaleSqt;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TwistRatio {
    Arm,
    Forearm,
    ForearmSecond,
    UpperLeg,
}
impl TwistRatio {
    fn key(self) -> u64 {
        match self {
            Self::Arm => 0x3831_7705_e5a4_571a,
            Self::Forearm => 0x246f_c198_b6f1_624d,
            Self::ForearmSecond => 0x5cdb_40ca_8c5a_5044,
            Self::UpperLeg => 0x3eb7_0888_6e6e_4607,
        }
    }
}
pub(crate) trait TwistAttributeSource {
    type Error;
    fn read_ratio(&mut self, ratio: TwistRatio) -> Result<f32, Self::Error>;
}

/// Actual `GlobalAttributeCollection+d8` instance from `ctor828b27cc..2824`.
/// The enclosing global owner must retain this allocation across frame reads.
pub(crate) struct PresentationTwistAttributes(AttributeRuntimeInstance);
impl PresentationTwistAttributes {
    pub(crate) fn bind_global(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self(vault.instance(0xbabb_3ec1_1084_9e08, 0x2db7_7096_45a1_9ce8))
    }
}
#[derive(Debug)]
pub(crate) enum TwistAttributeError {
    MissingGlobal,
    Attribute(AttributeRuntimeError),
    Size(usize),
}
impl TwistAttributeSource for Option<&PresentationTwistAttributes> {
    type Error = TwistAttributeError;
    fn read_ratio(&mut self, ratio: TwistRatio) -> Result<f32, Self::Error> {
        let instance = &self.ok_or(TwistAttributeError::MissingGlobal)?.0;
        let data = instance
            .try_get_data(ratio.key(), 0)
            .map_err(TwistAttributeError::Attribute)?;
        let Some(data) = data else { return Ok(0.0) }; //actual DefaultDataArea(4)
        let bytes = data
            .bytes()
            .try_into()
            .map_err(|_| TwistAttributeError::Size(data.bytes().len()))?;
        Ok(f32::from_be_bytes(bytes))
    }
}
#[derive(Debug)]
pub(crate) enum ExpressionsError<A, N, R> {
    Attribute(A),
    Twist(TwistError<N, R>),
}
impl std::fmt::Display for TwistAttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingGlobal => f.write_str("animation_twistbones global instance missing"),
            Self::Attribute(error) => write!(f, "twist attribute: {error:?}"),
            Self::Size(size) => write!(f, "twist ratio payload has {size} bytes"),
        }
    }
}
impl<A: std::fmt::Debug, N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display
    for ExpressionsError<A, N, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attribute(error) => write!(f, "expressions attribute: {error:?}"),
            Self::Twist(error) => error.fmt(f),
        }
    }
}

/// `indices` is the fourteen named indices retained by actual `InitAnimation`.
/// All four live ratios are sampled before the first correction; each side
/// reuses those samples, not a second query or a hardcoded tuning value.
#[expect(
    clippy::type_complexity,
    reason = "distinct source attribute, numeric and channel mutation errors"
)]
pub(crate) fn try_compute_expression_channels<
    A: TwistAttributeSource,
    N: PresentationMatrixNumeric,
    R: ReckoningSinCosNumeric,
>(
    channels: &mut impl CorrectionChannels,
    indices: &[usize; 14],
    attributes: &mut A,
    numeric: &mut N,
    rounding: &mut R,
) -> Result<(), ExpressionsError<A::Error, N::Error, R::Error>> {
    let arm = attributes
        .read_ratio(TwistRatio::Arm)
        .map_err(ExpressionsError::Attribute)?;
    let forearm = attributes
        .read_ratio(TwistRatio::Forearm)
        .map_err(ExpressionsError::Attribute)?;
    let forearm_second = attributes
        .read_ratio(TwistRatio::ForearmSecond)
        .map_err(ExpressionsError::Attribute)?;
    let upper_leg = attributes
        .read_ratio(TwistRatio::UpperLeg)
        .map_err(ExpressionsError::Attribute)?;
    for (source, destination, ratio) in [
        (0, 1, arm),
        (2, 3, forearm),
        (2, 4, forearm_second),
        (5, 6, upper_leg),
        (7, 8, arm),
        (9, 10, forearm),
        (9, 11, forearm_second),
        (12, 13, upper_leg),
    ] {
        try_twist_channels(
            channels,
            indices[source],
            indices[destination],
            ratio,
            numeric,
            rounding,
        )
        .map_err(ExpressionsError::Twist)?;
    }
    Ok(())
}

#[cfg(test)]
#[expect(
    clippy::type_complexity,
    reason = "distinct source attribute and numeric errors"
)]
fn try_compute_expressions<
    A: TwistAttributeSource,
    N: PresentationMatrixNumeric,
    R: ReckoningSinCosNumeric,
>(
    bind: &[AndaleSqt],
    replay: &mut [AndaleSqt],
    indices: &[usize; 14],
    attributes: &mut A,
    numeric: &mut N,
    rounding: &mut R,
) -> Result<(), ExpressionsError<A::Error, N::Error, R::Error>> {
    try_compute_expression_channels(
        &mut super::corrections::SliceChannels { bind, replay },
        indices,
        attributes,
        numeric,
        rounding,
    )
}
#[cfg(test)]
#[path = "skater_presentation_expressions_tests.rs"]
mod tests;
