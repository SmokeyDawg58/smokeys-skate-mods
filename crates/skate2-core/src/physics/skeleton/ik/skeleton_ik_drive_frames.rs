//! `SkeletonIK::UpdateDriveFrames`, 0x82b52e90..0x82b5321c.
//! Retained targets feed the existing two-bone solver and actual mapped rows.
//! Caller target-row writes follow the helper's ancestor writes, without a
//! second pose mapping or replacement `SkeletonData` owner.

use super::SkeletonIkState;
use crate::{
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_ik::SkeletonIkDataIn,
    skeleton_ik_prephysics::{
        SkeletonIkPrePhysicsAttributes, SkeletonIkPrePhysicsError, SkeletonIkPrePhysicsInput,
        SkeletonIkPrePhysicsRows, SkeletonIkPrePhysicsServices, try_do_two_bone_prephysics,
    },
    skeleton_ik_targets::SkeletonIkTargets,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[cfg(test)]
#[path = "skeleton_ik_drive_frames_tests.rs"]
mod tests;

pub(crate) trait SkeletonIkDriveFramesServices: SkeletonIkPrePhysicsServices {
    fn enable_prephysics(&mut self) -> Result<bool, Self::Error>;
    fn maximum_leg_length(&mut self) -> Result<f32, Self::Error>;
    fn maximum_arm_length(&mut self) -> Result<f32, Self::Error>;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkDriveFramesError<P, A, S, N, R> {
    Service(S),
    MissingSelectedPart { entry: usize },
    MissingPrimaryPart { entry: usize },
    Rows(P),
    PrePhysics(SkeletonIkPrePhysicsError<P, A, S, N, R>),
}

impl SkeletonIkState {
    /// Enable then both length reads precede every mode test. A false solver
    /// result preserves its own prefix but skips all caller target commits.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_update_drive_frames<
        P: SkeletonIkPrePhysicsRows,
        A: SkeletonIkPrePhysicsAttributes,
        S: SkeletonIkDriveFramesServices,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        data: &SkeletonIkDataIn,
        targets: &SkeletonIkTargets,
        rows: &mut P,
        attributes: &mut A,
        services: &mut S,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), SkeletonIkDriveFramesError<P::Error, A::Error, S::Error, N::Error, R::Error>>
    {
        use SkeletonIkDriveFramesError as Error;
        if !services.enable_prephysics().map_err(Error::Service)? {
            return Ok(());
        }
        let leg = services.maximum_leg_length().map_err(Error::Service)?;
        let arm = services.maximum_arm_length().map_err(Error::Service)?;
        for (entry, maximum) in [leg, leg, arm, arm].into_iter().enumerate() {
            if !matches!(self.modes[entry], 1..=3) {
                continue;
            }
            let primary = data.world_to_animation.compose(&self.part_to_world[entry]);
            let parent = targets.parents()[entry].map(|_| {
                data.world_to_animation
                    .compose(&self.parent_to_world[entry])
            });
            let selected_part = targets.parents()[entry]
                .or(targets.parts()[entry])
                .ok_or(Error::MissingSelectedPart { entry })?;
            let selected = parent.as_ref().unwrap_or(&primary);
            // GetBoneMtxFromVolMtx82b3ae60: selected volume is OUTER; actual
            // per-Part volume-to-bone is INNER. No inverse/normalization here.
            let bone = selected.compose(&SkeletonRootMatrix::known(
                targets.data().volume_to_bone()[selected_part.index()],
            ));
            let animated_position = rows
                .animation_position(selected_part)
                .map_err(Error::Rows)?;
            let delta = subtract(bone.columns()[3], animated_position);
            let mut adjust = SkeletonRootVector::known([0.0; 4]);
            let solved = try_do_two_bone_prephysics(
                SkeletonIkPrePhysicsInput {
                    data: targets.data(),
                    selected_part: Some(selected_part),
                    delta,
                    max_length: maximum,
                },
                rows,
                attributes,
                services,
                &mut adjust,
                numeric,
                rounding,
            )
            .map_err(Error::PrePhysics)?;
            if !solved {
                continue;
            }
            let part = targets.parts()[entry].ok_or(Error::MissingPrimaryPart { entry })?;
            rows.set_mapped_transform(part, &with_adjustment(primary, adjust))
                .map_err(Error::Rows)?;
            self.part_to_world[entry] = with_adjustment(self.part_to_world[entry], adjust);
            if let (Some(part), Some(parent)) = (targets.parents()[entry], parent) {
                rows.set_mapped_transform(part, &with_adjustment(parent, adjust))
                    .map_err(Error::Rows)?;
                // Native adds the SAME animation-space adjustment directly to
                // world P. Do not rotate it or derive P back from the row.
                self.parent_to_world[entry] = with_adjustment(self.parent_to_world[entry], adjust);
            }
        }
        Ok(())
    }
}

fn subtract(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| a.xyz[lane] - b.xyz[lane]),
        w: a.w.zip(b.w).map(|(a, b)| a - b),
    }
}
fn with_adjustment(matrix: SkeletonRootMatrix, adjust: SkeletonRootVector) -> SkeletonRootMatrix {
    let mut columns = *matrix.columns();
    let position = columns[3];
    columns[3] = SkeletonRootVector {
        xyz: std::array::from_fn(|lane| position.xyz[lane] + adjust.xyz[lane]),
        w: position.w.zip(adjust.w).map(|(p, a)| p + a),
    };
    SkeletonRootMatrix::from_columns(columns)
}
