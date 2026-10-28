//! Retained world-transform blend stage, 0x82b520d0..0x82b52e8c.
//! Previous record deck P and physical-board basis are distinct producers.
use super::SkeletonIkState;
use crate::{
    animation::AnimationMatrix,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_affine_interpolation::{
        SkeletonAffineInterpolationError, try_interpolate_affine_typed, try_normalize_rotation,
        try_slerp_typed,
    },
    skeleton_ik::SkeletonIkDataIn,
    skeleton_ik_targets::SkeletonIkTargets,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkTransformBlendError<N, R> {
    PreviousDeckUnavailable,
    Affine(SkeletonAffineInterpolationError<N, R>),
}

impl SkeletonIkState {
    /// Base primary commits before optional-parent work. External interpolation
    /// always updates the parent matrix too, even for hands without a parent ID.
    #[expect(
        clippy::single_match,
        reason = "explicit active and inactive raw mode cases"
    )]
    pub(crate) fn try_blend_transforms<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
        &mut self,
        data: &SkeletonIkDataIn,
        targets: &SkeletonIkTargets,
        previous: &[AnimationMatrix],
        numeric: &mut N,
        trigonometry: &mut R,
    ) -> Result<(), SkeletonIkTransformBlendError<N::Error, R::Error>> {
        use SkeletonIkTransformBlendError as Error;
        let previous_deck = previous.first().ok_or(Error::PreviousDeckUnavailable)?;
        let deck_position =
            SkeletonRootVector::known(std::array::from_fn(|lane| previous_deck[12 + lane]));
        let world_position = data
            .animation_to_world
            .compose(&SkeletonRootMatrix::translation(deck_position))
            .columns()[3];
        let mut board_columns = *data.physics_skateboard.columns();
        board_columns[3] = world_position;
        let board = SkeletonRootMatrix::from_columns(board_columns);
        for entry in 0..4 {
            match self.modes[entry] {
                1..=3 => {
                    let blend = self.on_deck_blends[entry];
                    if blend > 0.0 {
                        let candidate = board.compose(&self.part_to_deck[entry]);
                        let result = try_base_blend(
                            &self.part_to_world[entry],
                            &candidate,
                            blend,
                            numeric,
                            trigonometry,
                        )
                        .map_err(Error::Affine)?;
                        // Mode1 source stores P/X/Y/Z; external modes Z/P/X/Y.
                        // No callback/observable borrow occurs between columns.
                        self.part_to_world[entry] = result;
                        if targets.parents()[entry].is_some() {
                            let blend = self.on_deck_blends[entry];
                            let candidate = board.compose(&self.parent_to_deck[entry]);
                            self.parent_to_world[entry] = try_base_blend(
                                &self.parent_to_world[entry],
                                &candidate,
                                blend,
                                numeric,
                                trigonometry,
                            )
                            .map_err(Error::Affine)?;
                        }
                    }
                    if self.modes[entry] != 1 {
                        let blend = self.external_blends[entry];
                        if blend < 1.0 {
                            self.part_to_world[entry] = try_interpolate_affine_typed(
                                &self.part_to_world[entry],
                                &self.part_to_world_external[entry],
                                blend,
                                numeric,
                                trigonometry,
                            )
                            .map_err(Error::Affine)?;
                            let blend = self.external_blends[entry];
                            self.parent_to_world[entry] = try_interpolate_affine_typed(
                                &self.parent_to_world[entry],
                                &self.parent_to_world_external[entry],
                                blend,
                                numeric,
                                trigonometry,
                            )
                            .map_err(Error::Affine)?;
                        } else {
                            // Raw NOT_LT includes NaN; this branch performs no math.
                            self.part_to_world[entry] = self.part_to_world_external[entry];
                            self.parent_to_world[entry] = self.parent_to_world_external[entry];
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn try_base_blend<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    old: &SkeletonRootMatrix,
    candidate: &SkeletonRootMatrix,
    blend: f32,
    numeric: &mut N,
    trigonometry: &mut R,
) -> Result<SkeletonRootMatrix, SkeletonAffineInterpolationError<N::Error, R::Error>> {
    let complement = 1.0 - blend;
    let old_position = old.columns()[3];
    let candidate_position = candidate.columns()[3];
    let position = SkeletonRootVector {
        xyz: std::array::from_fn(|lane| {
            let retained = old_position.xyz[lane] * complement;
            candidate_position.xyz[lane].mul_add(blend, retained)
        }),
        w: old_position
            .w
            .zip(candidate_position.w)
            .map(|(old, candidate)| {
                let retained = old * complement;
                candidate.mul_add(blend, retained)
            }),
    };
    let zero_position = |matrix: &SkeletonRootMatrix| {
        let mut columns = *matrix.columns();
        columns[3] = SkeletonRootVector::known([0.0; 4]);
        SkeletonRootMatrix::from_columns(columns)
    };
    let old_rotation = try_normalize_rotation(&zero_position(old), numeric)
        .map_err(SkeletonAffineInterpolationError::Numeric)?;
    let candidate_rotation = try_normalize_rotation(&zero_position(candidate), numeric)
        .map_err(SkeletonAffineInterpolationError::Numeric)?;
    let (rotation, _) = try_slerp_typed(
        &old_rotation,
        &candidate_rotation,
        blend,
        numeric,
        trigonometry,
    )?;
    let mut columns = *rotation.columns();
    columns[3] = position;
    Ok(SkeletonRootMatrix::from_columns(columns))
}

#[cfg(test)]
#[path = "skeleton_ik_blend_transforms_tests.rs"]
mod tests;
