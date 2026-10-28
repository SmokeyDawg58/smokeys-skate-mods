//! `Skeleton::UpdateLandingOnSkateboardAdjust`, 0x82b417c8..0x82b41c80.
//! Consumes the actual mapped deck, retained COM velocity and `WorldToAnim` from
//! one Skeleton owner. It produces the existing full/Y offset state; application
//! to mapped deck and retained IK remains in the following original stage.

use crate::{
    animation::AnimationMatrix,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skateboard_spawn_math::{skateboard_spawn_compose, skateboard_spawn_rigid_inverse},
    skeleton_affine_interpolation::{SkeletonAffineInterpolationError, try_interpolate_affine},
    skeleton_skateboard_offset::{SkeletonSkateboardOffset, SkeletonSkateboardOffsetCvars},
};

#[cfg(test)]
#[path = "skeleton_landing_on_board_tests.rs"]
mod tests;

/// Explicit immutable processed-input projection, distinct from retained COM.
pub(crate) struct SkeletonLandingOnBoardInput {
    pub(crate) flags_8d0: u32,
    pub(crate) deck_transform: AnimationMatrix,
    pub(crate) landing_on_deck_adjust: [f32; 4],
    pub(crate) time_to_land: f32,
}

impl crate::skeleton_current_animation::continuation::AnimationProcessedRead
    for SkeletonLandingOnBoardInput
{
    fn flags(
        &self,
        offset: u32,
        mask: u32,
    ) -> Result<u32, crate::skeleton_current_animation::continuation::AnimationInputError> {
        if offset != 0x8d0 {
            return Err(crate::skeleton_current_animation::continuation::AnimationInputError::UnknownFlags { offset, mask });
        }
        Ok(self.flags_8d0 & mask)
    }
    fn deck_matrix(
        &self,
    ) -> Result<AnimationMatrix, crate::skeleton_current_animation::continuation::AnimationInputError>
    {
        Ok(self.deck_transform)
    }
    fn landing_adjust_xyz(
        &self,
    ) -> Result<[f32; 3], crate::skeleton_current_animation::continuation::AnimationInputError>
    {
        Ok(self.landing_on_deck_adjust[..3].try_into().expect("XYZ"))
    }
    fn time_to_land(
        &self,
    ) -> Result<f32, crate::skeleton_current_animation::continuation::AnimationInputError> {
        Ok(self.time_to_land)
    }
}

pub(crate) trait SkeletonLandingOnBoardAttributes {
    type Error;
    /// Original `physics_animation.default` key0x6aea4c4efbba747b, index0.
    /// `PointNegGraphData8` metadata is ignored; Evaluate8 reads X+0x10/Y+0x30.
    fn blend(&mut self, time_to_land: f32) -> Result<f32, Self::Error>;
}

pub(crate) trait SkeletonLandingOnBoardCvars {
    type Error;
    fn show_landing_adjust(&mut self) -> Result<u8, Self::Error>;
    /// Debug branch's only effective collaborator is `GetDebugOverlay82581e40`.
    /// Both surrounding drawing calls resolve to the BLR stub825187c0.
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error>;
    fn negate_y(&mut self) -> Result<u8, Self::Error>;
    fn frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error>;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonLandingOnBoardError<A, C, N, R> {
    Input(crate::skeleton_current_animation::continuation::AnimationInputError),
    Attribute(A),
    Cvar(C),
    DebugOverlay(C),
    Interpolation(SkeletonAffineInterpolationError<N, R>),
}
impl<A: std::fmt::Debug, C: std::fmt::Debug, N: std::fmt::Debug, R: std::fmt::Debug>
    std::fmt::Display for SkeletonLandingOnBoardError<A, C, N, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(error) => error.fmt(f),
            Self::Attribute(error) => write!(f, "landing-on-board attribute: {error:?}"),
            Self::Cvar(error) => write!(f, "landing-on-board cvar: {error:?}"),
            Self::DebugOverlay(error) => write!(f, "landing-on-board debug singleton: {error:?}"),
            Self::Interpolation(error) => write!(f, "landing-on-board {error}"),
        }
    }
}
impl<A: std::fmt::Debug, C: std::fmt::Debug, N: std::fmt::Debug, R: std::fmt::Debug>
    std::error::Error for SkeletonLandingOnBoardError<A, C, N, R>
{
}

struct OffsetCvars<'a, C>(&'a mut C);
impl<C: SkeletonLandingOnBoardCvars> SkeletonSkateboardOffsetCvars for OffsetCvars<'_, C> {
    type Error = C::Error;
    fn frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.0.frames_to_blend_away_offset()
    }
}

/// Caller selects flags8cc&0x200 && !(flags8cc&0x40000). This routine's own
/// ordered downward-velocity gate precedes the flip-board flag and every read.
/// The passed full `WorldToAnim` comes from the actual owner's rigid inverse,
/// whose source graph establishes W; no unknown lane is filled by this helper.
#[expect(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "distinct retained owners, processed snapshot and lazy services"
)]
#[cfg(test)]
pub(crate) fn try_update_landing_on_board<A, C, N, R>(
    offset: &mut SkeletonSkateboardOffset,
    mapped: &AnimationMatrix,
    world_to_animation: &AnimationMatrix,
    retained_com_y: f32,
    input: &SkeletonLandingOnBoardInput,
    attributes: &mut A,
    cvars: &mut C,
    numeric: &mut N,
    trigonometry: &mut R,
) -> Result<(), SkeletonLandingOnBoardError<A::Error, C::Error, N::Error, R::Error>>
where
    A: SkeletonLandingOnBoardAttributes,
    C: SkeletonLandingOnBoardCvars,
    N: ReckoningNumeric,
    R: ReckoningSinCosNumeric,
{
    try_update_landing_on_board_read(
        offset,
        mapped,
        world_to_animation,
        retained_com_y,
        input,
        attributes,
        cvars,
        numeric,
        trigonometry,
    )
}

#[expect(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "distinct retained owners, late inputs and numerical services"
)]
pub(crate) fn try_update_landing_on_board_read<
    A,
    C,
    N,
    R,
    I: crate::skeleton_current_animation::continuation::AnimationProcessedRead,
>(
    offset: &mut SkeletonSkateboardOffset,
    mapped: &AnimationMatrix,
    world_to_animation: &AnimationMatrix,
    retained_com_y: f32,
    input: &I,
    attributes: &mut A,
    cvars: &mut C,
    numeric: &mut N,
    trigonometry: &mut R,
) -> Result<(), SkeletonLandingOnBoardError<A::Error, C::Error, N::Error, R::Error>>
where
    A: SkeletonLandingOnBoardAttributes,
    C: SkeletonLandingOnBoardCvars,
    N: ReckoningNumeric,
    R: ReckoningSinCosNumeric,
{
    use SkeletonLandingOnBoardError as Error;
    if retained_com_y.partial_cmp(&0.0) != Some(std::cmp::Ordering::Less) {
        return Ok(());
    }
    if input.flag(0x8d0, 0x10).map_err(Error::Input)? {
        return Ok(());
    }
    let mut desired = skateboard_spawn_compose(
        world_to_animation,
        &input.deck_matrix().map_err(Error::Input)?,
    );
    let landing_adjust = input.landing_adjust_xyz().map_err(Error::Input)?;
    let mut adjustment: [f32; 4] = std::array::from_fn(|lane| {
        let x = world_to_animation[lane] * landing_adjust[0];
        let xy = world_to_animation[4 + lane].mul_add(landing_adjust[1], x);
        world_to_animation[8 + lane].mul_add(landing_adjust[2], xy)
    });
    adjustment[1] = 0.0;
    if desired[13] > mapped[13] {
        adjustment[1] = desired[13] - mapped[13];
    }
    for lane in 0..4 {
        desired[12 + lane] = mapped[12 + lane] + adjustment[lane];
    }
    let blend = attributes
        .blend(input.time_to_land().map_err(Error::Input)?)
        .map_err(Error::Attribute)?;
    let interpolated = try_interpolate_affine(mapped, &desired, blend, numeric, trigonometry)
        .map_err(Error::Interpolation)?;
    let inverse = skateboard_spawn_rigid_inverse(mapped);
    let mut adjustment_transform = skateboard_spawn_compose(&interpolated, &inverse);

    if cvars.show_landing_adjust().map_err(Error::Cvar)? != 0 {
        cvars.require_debug_overlay().map_err(Error::DebugOverlay)?;
    }
    let sign = if cvars.negate_y().map_err(Error::Cvar)? != 0 {
        -1.0
    } else {
        1.0
    };
    let vertical = sign * adjustment_transform[13];
    adjustment_transform[13] = 0.0;
    // First failure leaves the full newly copied matrix with Y+0 and JustSet1,
    // old full count, and untouched Y-count/flag. No retained writes precede it.
    offset
        .try_set_full_transform(&adjustment_transform, &mut OffsetCvars(cvars))
        .map_err(Error::Cvar)?;
    // Source second value load at0x82b41c5c precedes Y matrix/flag/count stores.
    let vertical_count = cvars.frames_to_blend_away_offset().map_err(Error::Cvar)?;
    offset.transform[13] = vertical;
    offset.vertical_just_set = true;
    offset.vertical_count = vertical_count;
    Ok(())
}
