//! Four retained deck scalars from `SkeletonIK::SkeletonIK`,
//! 0x82b55178..0x82b55388. The aggregate's Instance/owner/DataIn/Reset prefix
//! precedes this stage. Reads borrow the actual retained global board provider.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "IK aggregate owns constructor integration")
)]

use crate::{
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skateboard_attributes::{SkateboardAttributeClass, SkateboardAttributeRead},
    skeleton_ik_tangent::{SkeletonIkTangentError, try_tangent},
};

#[cfg(test)]
#[path = "skeleton_ik_geometry_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkGeometryProgress {
    Fresh,
    BoxHalfWidth,
    BoxHalfLength,
    TotalHalfLength,
    EndSlope,
    Complete,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkGeometryError<D, N, R> {
    AlreadyAttempted(SkeletonIkGeometryProgress),
    Deck(D),
    Tangent(SkeletonIkTangentError<N, R>),
}

/// Unknown scalars remain absent until their source store. Construction is
/// attempted once; native Reset preserves all four completed geometry fields.
pub(crate) struct SkeletonIkGeometry {
    box_half_width: Option<f32>,
    box_half_length: Option<f32>,
    total_half_length: Option<f32>,
    end_slope: Option<f32>,
    progress: SkeletonIkGeometryProgress,
}

impl SkeletonIkGeometry {
    pub(crate) const fn new() -> Self {
        Self {
            box_half_width: None,
            box_half_length: None,
            total_half_length: None,
            end_slope: None,
            progress: SkeletonIkGeometryProgress::Fresh,
        }
    }

    pub(crate) const fn box_half_width(&self) -> Option<f32> {
        self.box_half_width
    }
    pub(crate) const fn box_half_length(&self) -> Option<f32> {
        self.box_half_length
    }
    pub(crate) const fn total_half_length(&self) -> Option<f32> {
        self.total_half_length
    }
    pub(crate) const fn end_slope(&self) -> Option<f32> {
        self.end_slope
    }
    pub(crate) const fn progress(&self) -> SkeletonIkGeometryProgress {
        self.progress
    }

    #[expect(
        clippy::type_complexity,
        reason = "distinct attribute and numerical service errors"
    )]
    pub(crate) fn try_construct<
        D: SkateboardAttributeRead,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        deck: &mut D,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), SkeletonIkGeometryError<D::Error, N::Error, R::Error>> {
        use SkeletonIkGeometryError as Error;
        use SkeletonIkGeometryProgress as Progress;
        if self.progress != Progress::Fresh {
            return Err(Error::AlreadyAttempted(self.progress));
        }
        let half = f32::from_bits(0x3f00_0000);
        self.progress = Progress::BoxHalfWidth;
        let width = deck
            .read_float(SkateboardAttributeClass::Deck, 0xd76d_9db5_ee1f_d008)
            .map_err(Error::Deck)?;
        self.box_half_width = Some(width * half);

        self.progress = Progress::BoxHalfLength;
        let length = deck
            .read_float(SkateboardAttributeClass::Deck, 0xc94d_6509_1dcb_828b)
            .map_err(Error::Deck)?;
        self.box_half_length = Some(length * half);

        self.progress = Progress::TotalHalfLength;
        let extra_length = deck
            .read_float(SkateboardAttributeClass::Deck, 0x1d53_bc53_9321_90e7)
            .map_err(Error::Deck)?;
        // 0x82b5529c..552a8 loads new value, then the actual retained half-length.
        self.total_half_length =
            Some(extra_length + self.box_half_length.expect("preceding source store"));

        self.progress = Progress::EndSlope;
        let degrees = deck
            .read_float(SkateboardAttributeClass::Deck, 0x8020_3de9_f9b7_32ca)
            .map_err(Error::Deck)?;
        let radians = degrees * f32::from_bits(0x3c8e_fa35);
        let lower = f32::from_bits(0x3c23_d70a);
        let upper = f32::from_bits(0x3fc7_c82d);
        // Ordered scalar fsel graph: unordered chooses the final operand.
        let lower_test = lower - radians;
        let limited_lower = if lower_test >= 0.0 { lower } else { radians };
        let upper_test = upper - limited_lower;
        let angle = if upper_test >= 0.0 {
            limited_lower
        } else {
            upper
        };
        self.end_slope = Some(try_tangent(angle, numeric, rounding).map_err(Error::Tangent)?);
        self.progress = Progress::Complete;
        Ok(())
    }
}
