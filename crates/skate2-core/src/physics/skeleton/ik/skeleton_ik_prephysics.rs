//! `SkeletonIK::DoTwoBoneIk_PrePhysics`, 0x82b49950..0x82b4a140.
//! Borrows the actual Part-order pose and mapped rows; only the two ancestor
//! rows are written here. The drive-frame caller owns target-row writes.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "active IK drive-frame integration")
)]

use crate::{
    ragdoll::SkeletonPartIndex,
    reckoning_numeric::{ReckoningNumeric, cross_full4, length_from_squared_and_estimate},
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_affine_interpolation::try_normalize_rotation,
    skeleton_data::SkeletonData,
    skeleton_ik_line_frame::{
        SkeletonIkClampError, normalize_safe, try_calculate_matrix_from_line,
        try_clamp_vector_within_range,
    },
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
    skeleton_two_bone::{
        TwoBoneChain, TwoBoneIkError, TwoBoneIkOutcome, TwoBoneLimits, try_solve_two_bone,
    },
};

#[cfg(test)]
#[path = "skeleton_ik_prephysics_tests.rs"]
mod tests;

const ZERO: SkeletonRootVector = SkeletonRootVector::known([0.0; 4]);
const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

/// Reads refer to the existing animation buffer and CURRENT mapped row. A
/// setter must retain typed W knowledge or refuse its particular store.
pub(crate) trait SkeletonIkPrePhysicsRows {
    type Error;
    fn animation_position(
        &mut self,
        part: SkeletonPartIndex,
    ) -> Result<SkeletonRootVector, Self::Error>;
    fn mapped_transform(
        &mut self,
        part: SkeletonPartIndex,
    ) -> Result<SkeletonRootMatrix, Self::Error>;
    fn set_mapped_transform(
        &mut self,
        part: SkeletonPartIndex,
        value: &SkeletonRootMatrix,
    ) -> Result<(), Self::Error>;
}

/// Two separate lazy reads from GLOBAL `physics_animation.default` (+0xcc),
/// never the IK owner's `physics_skeletonik` instance. Minimum key
/// b76b5101768e856c is fetched/loaded at 0x82b49c2c/44; maximum key
/// a175875dec9d6db7 at 0x82b49c84/9c. Each read checks its singleton anew.
pub(crate) trait SkeletonIkPrePhysicsAttributes {
    type Error;
    fn minimum_angle_degrees(&mut self) -> Result<f32, Self::Error>;
    fn maximum_angle_degrees(&mut self) -> Result<f32, Self::Error>;
}

pub(crate) trait SkeletonIkPrePhysicsServices {
    type Error;
    fn draw_pre_feet_ik(&mut self) -> Result<bool, Self::Error>;
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy)]
pub(crate) struct SkeletonIkPrePhysicsInput<'a> {
    pub(crate) data: &'a SkeletonData,
    pub(crate) selected_part: Option<SkeletonPartIndex>,
    pub(crate) delta: SkeletonRootVector,
    pub(crate) max_length: f32,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkPrePhysicsError<P, A, S, N, R> {
    MissingSelectedPart,
    MissingParent { selected: SkeletonPartIndex },
    MissingGrandparent { parent: SkeletonPartIndex },
    Rows(P),
    Clamp(SkeletonIkClampError<N>),
    Attribute(A),
    Service(S),
    Numeric(N),
    TwoBone(TwoBoneIkError<N, R>),
    UnexpectedOutcome(TwoBoneIkOutcome),
}

type PrePhysicsResult<T, P, A, S, N, R> = Result<T, SkeletonIkPrePhysicsError<P, A, S, N, R>>;

/// There is deliberately no all-row preflight or atomic pair replacement.
/// Adjustment commits before the outcome assertion; grandparent commits before
/// parent is even read. Debug drawing itself is a proven native BLR stub.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
#[expect(
    clippy::too_many_lines,
    reason = "ordered solver and row commit boundaries"
)]
pub(crate) fn try_do_two_bone_prephysics<
    P: SkeletonIkPrePhysicsRows,
    A: SkeletonIkPrePhysicsAttributes,
    S: SkeletonIkPrePhysicsServices,
    N: ReckoningNumeric,
    R: ReckoningSinCosNumeric,
>(
    input: SkeletonIkPrePhysicsInput<'_>,
    rows: &mut P,
    attributes: &mut A,
    services: &mut S,
    adjust: &mut SkeletonRootVector,
    numeric: &mut N,
    rounding: &mut R,
) -> PrePhysicsResult<bool, P::Error, A::Error, S::Error, N::Error, R::Error> {
    use SkeletonIkPrePhysicsError as Error;
    let selected = input.selected_part.ok_or(Error::MissingSelectedPart)?;
    let parent = input.data.parents()[selected.index()].ok_or(Error::MissingParent { selected })?;
    let end = rows.animation_position(selected).map_err(Error::Rows)?;
    let grandparent = input.data.parents()[parent.index()];
    let middle = rows.animation_position(parent).map_err(Error::Rows)?;
    let grandparent = grandparent.ok_or(Error::MissingGrandparent { parent })?;
    let root = rows.animation_position(grandparent).map_err(Error::Rows)?;
    let requested = add(end, input.delta);
    let mut solved_middle = ZERO;
    let mut solved_end = ZERO;
    try_clamp_vector_within_range(requested, root, input.max_length, &mut solved_end, numeric)
        .map_err(Error::Clamp)?;

    if services.draw_pre_feet_ik().map_err(Error::Service)? {
        let original = subtract(requested, root);
        let clamped = subtract(solved_end, root);
        // BOTH dots precede BOTH seeds, even when clamping did nothing.
        let original_squared = numeric
            .dot3(scratch(original), scratch(original))
            .map_err(Error::Numeric)?;
        let clamped_squared = numeric
            .dot3(scratch(clamped), scratch(clamped))
            .map_err(Error::Numeric)?;
        let original_seed = numeric
            .reciprocal_sqrt_estimate(original_squared)
            .map_err(Error::Numeric)?;
        let clamped_seed = numeric
            .reciprocal_sqrt_estimate(clamped_squared)
            .map_err(Error::Numeric)?;
        let original_length = length_from_squared_and_estimate(original_squared, original_seed);
        let clamped_length = length_from_squared_and_estimate(clamped_squared, clamped_seed);
        if original_length > clamped_length {
            services.require_debug_overlay().map_err(Error::Service)?;
        }
    }
    let minimum = attributes
        .minimum_angle_degrees()
        .map_err(Error::Attribute)?;
    let maximum = attributes
        .maximum_angle_degrees()
        .map_err(Error::Attribute)?;
    let outcome = try_solve_two_bone(
        &TwoBoneChain { root, middle, end },
        &mut solved_middle,
        &mut solved_end,
        TwoBoneLimits {
            minimum_degrees: minimum,
            maximum_degrees: maximum,
            override_maximum_with_original: true,
        },
        2,
        numeric,
        rounding,
    )
    .map_err(Error::TwoBone)?;
    if outcome == TwoBoneIkOutcome::Invalid {
        *adjust = ZERO;
        return Ok(false);
    }
    *adjust = subtract(solved_end, requested);
    if outcome != TwoBoneIkOutcome::Reachable {
        return Err(Error::UnexpectedOutcome(outcome));
    }

    let old_normal = cross(subtract(root, middle), subtract(end, middle));
    let new_normal = cross(
        subtract(root, solved_middle),
        subtract(solved_end, solved_middle),
    );
    let normal = normalize_safe(add(old_normal, new_normal), numeric).map_err(Error::Numeric)?;
    let mut lower = SkeletonRootMatrix::known(IDENTITY);
    let mut upper = SkeletonRootMatrix::known(IDENTITY);
    let mapped = try_calculate_matrix_from_line(
        [middle, end],
        normal,
        [solved_middle, solved_end],
        normal,
        &mut lower,
        numeric,
    )
    .map_err(Error::Numeric)?
        && try_calculate_matrix_from_line(
            [root, middle],
            normal,
            [root, solved_middle],
            normal,
            &mut upper,
            numeric,
        )
        .map_err(Error::Numeric)?;
    if mapped {
        let current = rows.mapped_transform(grandparent).map_err(Error::Rows)?;
        let updated =
            try_normalize_rotation(&upper.compose(&current), numeric).map_err(Error::Numeric)?;
        rows.set_mapped_transform(grandparent, &updated)
            .map_err(Error::Rows)?;
        let current = rows.mapped_transform(parent).map_err(Error::Rows)?;
        let updated =
            try_normalize_rotation(&lower.compose(&current), numeric).map_err(Error::Numeric)?;
        rows.set_mapped_transform(parent, &updated)
            .map_err(Error::Rows)?;
    }

    if services.draw_pre_feet_ik().map_err(Error::Service)? {
        // Final singleton precedes numeric work, unlike the conditional first one.
        services.require_debug_overlay().map_err(Error::Service)?;
        let delta = scratch(input.delta);
        let squared = numeric.dot3(delta, delta).map_err(Error::Numeric)?;
        let seed = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(Error::Numeric)?;
        let _length = length_from_squared_and_estimate(squared, seed);
    }
    Ok(mapped)
}

fn scratch(value: SkeletonRootVector) -> [f32; 4] {
    [
        value.xyz[0],
        value.xyz[1],
        value.xyz[2],
        value.w.unwrap_or(0.0),
    ]
}
fn add(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| left.xyz[lane] + right.xyz[lane]),
        w: left.w.zip(right.w).map(|(left, right)| left + right),
    }
}
fn subtract(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| left.xyz[lane] - right.xyz[lane]),
        w: left.w.zip(right.w).map(|(left, right)| left - right),
    }
}
fn cross(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    let result = cross_full4(scratch(left), scratch(right));
    SkeletonRootVector {
        xyz: [result[0], result[1], result[2]],
        w: left.w.zip(right.w).map(|_| result[3]),
    }
}
