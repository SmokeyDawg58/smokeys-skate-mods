//! Selected surface-motion `FillPhysOut` fdd8..3004c. Same retained owners are
//! supplied by the pair bridge; no preceding Fill or whole-frame completion.
use super::construct::board_fill::board_speed_from_squared_and_estimate;
use super::{
    PhysOutAccessError, PhysOutMotionScalar, PhysOutMotionVector, PhysOutOrigin,
    PhysicalPlayerPhysOut,
};
use crate::{
    reckoning_numeric::{
        ReckoningNumeric, try_normalize_safe, try_signed_angle, wrap_turns_to_radians,
    },
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
pub(crate) trait PhysOutSurfaceMotionSource {
    type Error;
    type Reckoning;
    type Processed;
    fn capture_reckoning(&mut self) -> Result<Self::Reckoning, Self::Error>;
    fn capture_processed(&mut self) -> Result<Self::Processed, Self::Error>;
    fn ground_normal(
        &mut self,
        reckoning: &Self::Reckoning,
    ) -> Result<SkeletonRootVector, Self::Error>;
    fn deck_z(&mut self, processed: &Self::Processed) -> Result<SkeletonRootVector, Self::Error>;
    fn ground_transform(
        &mut self,
        reckoning: &Self::Reckoning,
    ) -> Result<SkeletonRootMatrix, Self::Error>;
}
#[derive(Debug, PartialEq)]
pub(crate) enum PhysOutSurfaceMotionError<S, N> {
    Access(PhysOutAccessError),
    Source(S),
    Numeric(N),
    UnpublishedDeckVelocity,
}
impl PhysicalPlayerPhysOut {
    /// The original discarded `NormalizeSafe` result does not replace projected Z.
    pub(crate) fn try_publish_surface_motion<S: PhysOutSurfaceMotionSource, N: ReckoningNumeric>(
        &mut self,
        source: &mut S,
        numeric: &mut N,
    ) -> Result<(), PhysOutSurfaceMotionError<S::Error, N::Error>> {
        use PhysOutSurfaceMotionError as E;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        let reckoning = source.capture_reckoning().map_err(E::Source)?;
        let processed = source.capture_processed().map_err(E::Source)?;
        let ground = source.ground_normal(&reckoning).map_err(E::Source)?;
        let velocity = self
            .motion_vector(PhysOutMotionVector::DeckVelocity)
            .map_err(E::Access)?;
        if velocity.origin != PhysOutOrigin::Publication {
            return Err(E::UnpublishedDeckVelocity);
        }
        let dot = numeric
            .dot3(scratch(velocity.value), scratch(ground))
            .map_err(E::Numeric)?;
        let surface = project(velocity.value, ground, dot);
        let deck = source.deck_z(&processed).map_err(E::Source)?;
        let dot = numeric
            .dot3(scratch(deck), scratch(ground))
            .map_err(E::Numeric)?;
        let projected = project(deck, ground, dot);
        // Unknown scratch W is never retained: output of this source call is unused;
        // normalization decisions/estimates and signed-angle XYZ do not consume W.
        let _discarded =
            try_normalize_safe(scratch(projected), [0.0; 4], numeric).map_err(E::Numeric)?;
        let direction_dot = numeric
            .dot3(scratch(projected), scratch(surface))
            .map_err(E::Numeric)?;
        let direction = if direction_dot > 0.0 {
            projected
        } else {
            negate(projected)
        };
        let reckoning = source.capture_reckoning().map_err(E::Source)?;
        let transform = source.ground_transform(&reckoning).map_err(E::Source)?;
        let angle = try_signed_angle(
            scratch(direction),
            scratch(surface),
            scratch(transform.columns()[1]),
            numeric,
        )
        .map_err(E::Numeric)?;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let wrapped = wrap_turns_to_radians(angle * f32::from_bits(0x3e22_f983));
        self.publish_motion_vector(&guard, PhysOutMotionVector::SurfaceVelocity, surface)
            .map_err(E::Access)?;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        self.publish_motion_scalar(&guard, PhysOutMotionScalar::SurfaceAngle, wrapped)
            .map_err(E::Access)?;
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let squared = numeric
            .dot3(scratch(surface), scratch(surface))
            .map_err(E::Numeric)?;
        let estimate = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        let speed = board_speed_from_squared_and_estimate(squared, estimate);
        self.publish_motion_scalar(&guard, PhysOutMotionScalar::SurfaceSpeed, speed)
            .map_err(E::Access)
    }
}
fn scratch(v: SkeletonRootVector) -> [f32; 4] {
    [v.xyz[0], v.xyz[1], v.xyz[2], v.w.unwrap_or(0.0)]
}
fn project(v: SkeletonRootVector, g: SkeletonRootVector, dot: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|i| {
            let product = g.xyz[i] * dot;
            v.xyz[i] - product
        }),
        w: v.w.zip(g.w).map(|(v, g)| {
            let product = g * dot;
            v - product
        }),
    }
}
fn negate(v: SkeletonRootVector) -> SkeletonRootVector {
    fn sign(v: f32) -> f32 {
        f32::from_bits(v.to_bits() ^ 0x8000_0000)
    }
    SkeletonRootVector {
        xyz: v.xyz.map(sign),
        w: v.w.map(sign),
    }
}
#[cfg(test)]
#[path = "physical_player_phys_out_surface_motion_tests.rs"]
mod tests;

impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for PhysOutSurfaceMotionError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(e) => write!(f, "surface motion output: {e:?}"),
            Self::Source(e) => write!(f, "surface motion source: {e:?}"),
            Self::Numeric(e) => write!(f, "surface motion numeric: {e:?}"),
            Self::UnpublishedDeckVelocity => {
                f.write_str("surface motion requires published deck velocity")
            }
        }
    }
}
