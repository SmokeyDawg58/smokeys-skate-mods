//! Ordered Skeleton output 0x82b58230..0x82b58614; the next Part query is excluded.
use super::{
    super::{
        PhysOutAccessError, PhysOutComponent, PhysOutPublicationError, PhysicalPlayerPhysOut,
        com::PhysOutComSource,
    },
    ConcretePhysOutError,
};
use crate::{
    reckoning_numeric::ReckoningNumeric, skeleton_query_rotation::try_atan,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::convert::Infallible;

pub(crate) trait SkeletonFillPrefixSource: PhysOutComSource {
    type Input;
    fn capture_input(&mut self) -> Result<Self::Input, Self::Error>;
    fn animation_deck_z(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn board_flipped(&mut self, input: &Self::Input) -> Result<bool, Self::Error>;
    fn root_column(&mut self, column: usize) -> Result<SkeletonRootVector, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum SkeletonFillPrefixOutputError<S, N> {
    Source(S),
    Numeric(N),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
    InvalidRootColumn(usize),
    Com(PhysOutPublicationError<S>),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display
    for SkeletonFillPrefixOutputError<S, N>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(error) => write!(f, "Skeleton Fill source: {error:?}"),
            Self::Numeric(error) => write!(f, "Skeleton Fill numeric: {error:?}"),
            Self::Access(error) => write!(f, "Skeleton Fill access: {error:?}"),
            Self::Component(error) => write!(f, "Skeleton Fill component: {error:?}"),
            Self::InvalidRootColumn(column) => {
                write!(f, "Skeleton Fill invalid root column {column}")
            }
            Self::Com(error) => write!(f, "Skeleton Fill COM: {error:?}"),
        }
    }
}

/// Native VNMSUB then VMADDFP at 582f0/582f4 (or 58370/58374).
fn inverse(denominator: f32, seed: f32) -> f32 {
    let residual = -seed.mul_add(denominator, -1.0);
    seed.mul_add(residual, seed)
}
fn signed_constant(value: f32, numerator: f32) -> f32 {
    f32::from_bits(value.to_bits() | (numerator.to_bits() & 0x8000_0000))
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_skeleton_prefix<S: SkeletonFillPrefixSource, N: ReckoningNumeric>(
        &mut self,
        source: &mut S,
        numeric: &mut N,
    ) -> Result<(), SkeletonFillPrefixOutputError<S::Error, N::Error>> {
        self.try_fill_skeleton_prefix_capture(source, numeric)
            .map(|_| ())
    }
    pub(crate) fn try_fill_skeleton_prefix_capture<
        S: SkeletonFillPrefixSource,
        N: ReckoningNumeric,
    >(
        &mut self,
        source: &mut S,
        numeric: &mut N,
    ) -> Result<SkeletonRootVector, SkeletonFillPrefixOutputError<S::Error, N::Error>> {
        use SkeletonFillPrefixOutputError as E;
        let input = source.capture_input().map_err(E::Source)?; //58270
        let z = source.animation_deck_z().map_err(E::Source)?; //58274
        let flip = source.board_flipped(&input).map_err(E::Source)?; //58284
        let sign = if flip { 0x8000_0000 } else { 0 };
        let numerator = f32::from_bits(z.xyz[0].to_bits() ^ sign);
        let denominator = f32::from_bits(z.xyz[2].to_bits() ^ sign);
        let seed = numeric
            .reciprocal_estimate(denominator)
            .map_err(E::Numeric)?;
        let ratio = numerator.mul_add(inverse(denominator, seed), 0.0);
        let atan = try_atan(ratio, numeric).map_err(E::Numeric)?;
        let guard = self.sample_write_guard().map_err(E::Access)?; //583b8
        let mut yaw = if 0.0 > denominator {
            signed_constant(f32::from_bits(0x4049_0fdb), numerator) + atan
        } else {
            atan
        };
        if denominator == 0.0 {
            yaw = signed_constant(f32::from_bits(0x3fc9_0fdb), numerator);
        }
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Skeleton)
            .map_err(E::Component)?;
        self.concrete
            .skeleton
            .as_mut()
            .expect("guarded Skeleton")
            .skeleton_deck_yaw = yaw;
        let input = source.capture_input().map_err(E::Source)?; //583f8 fresh
        let flip = source.board_flipped(&input).map_err(E::Source)?;
        let pitch_z = if flip {
            z.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000))
        } else {
            z
        };
        let guard = self.sample_write_guard().map_err(E::Access)?;
        // Only XYZ participates; scratch W is never a retained source value.
        let pitch = numeric
            .dot3(
                [pitch_z.xyz[0], pitch_z.xyz[1], pitch_z.xyz[2], 0.0],
                [0.0, 1.0, 0.0, 0.0],
            )
            .map_err(E::Numeric)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Skeleton)
            .map_err(E::Component)?;
        self.concrete
            .skeleton
            .as_mut()
            .expect("guarded Skeleton")
            .skeleton_deck_pitch = pitch;
        for column in 0..4 {
            let value = source.root_column(column).map_err(E::Source)?;
            // vrlimi mask1 replaces W with X before the self-equality test.
            if value.xyz.into_iter().any(f32::is_nan) {
                return Err(E::InvalidRootColumn(column));
            }
        }
        self.check_current_guard().map_err(E::Access)?; //58554
        self.require_complete_component(PhysOutComponent::Skeleton)
            .map_err(E::Component)?;
        for column in 0..4 {
            let value = source.root_column(column).map_err(E::Source)?; //58580..5a4 fresh
            self.concrete
                .skeleton
                .as_mut()
                .expect("guarded Skeleton")
                .skeleton_root_matrix
                .set_column(column, value);
        }
        self.try_publish_skeleton_com_capture(source)
            .map_err(E::Com)
    }
}

#[cfg(test)]
#[path = "physical_player_phys_out_skeleton_fill_prefix_tests.rs"]
mod tests;
