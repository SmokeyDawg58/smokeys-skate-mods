//! Same-pair bridge for the two native Skeleton local foot target setters.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    dynamic_assembly_session::skeleton_normal::{
        OffboardFootTargetNormalError, SkeletonNormalAssembly,
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    skeleton_current_animation::SkeletonIkUnconstructed,
    skeleton_offboard_reorient::{ReorientError, try_reorient_skater_upright},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum OffboardLocalFootTargetsError {
    MissingPool,
    MissingFeetIk,
    MissingLocomotionInfluence { foot: usize },
    Poisoned,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    Setter(SkeletonIkUnconstructed),
    Cvar(crate::input_cvar_runtime::InputCvarError),
    Normal(OffboardFootTargetNormalError),
    Reorient(ReorientError<ReckoningHostNumericError, std::convert::Infallible>),
}

impl PhysicalAssemblyWorld {
    /// UpdateOffBoard passes current Skeleton AnimToWorld Z to the same
    /// retained Reckoning after NextTrajectory reset.
    pub(crate) fn try_pair_offboard_reckoning_heading(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
    ) -> Result<SkeletonRootVector, OffboardLocalFootTargetsError> {
        self.try_with_offboard_ik_pair(board, skeleton, expected, |normal| {
            Ok(normal.root_transforms().animation_to_world().columns()[2])
        })
    }
    /// OffBoard caller resets the next trajectory only after GeneralUpdate.
    pub(crate) fn try_finish_pair_offboard_next_trajectory(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
    ) -> Result<(), OffboardLocalFootTargetsError> {
        self.try_with_offboard_ik_pair(board, skeleton, expected, |normal| {
            normal.finish_offboard_next_trajectory();
            Ok(())
        })
    }
    /// UpdateRootTransformsOffBoard 0x82b4499c..a4 conditionally calls the
    /// native upright leaf on the same retained Skeleton root-transform owner.
    pub(crate) fn try_reorient_pair_offboard_skeleton(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
    ) -> Result<bool, OffboardLocalFootTargetsError> {
        self.try_with_offboard_ik_pair(board, skeleton, expected, |normal| {
            let roots = normal.root_transforms_mut();
            let original = *roots.animation_to_world();
            let result = try_reorient_skater_upright(
                &original,
                &mut HostReckoningNumeric,
                &mut HostReckoningSinCosNumeric,
            )
            .map_err(OffboardLocalFootTargetsError::Reorient)?;
            if let Some(matrix) = result {
                roots.set_offboard_animation_to_world(matrix);
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }

    /// UpdateRootTransformsOffBoard 0x82b44db8..c4 replaces all four
    /// animation-to-world columns on the same checked Skeleton pair.
    pub(crate) fn try_set_pair_offboard_animation_to_world(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
        transform: SkeletonRootMatrix,
    ) -> Result<(), OffboardLocalFootTargetsError> {
        self.try_with_offboard_ik_pair(board, skeleton, expected, |normal| {
            normal
                .root_transforms_mut()
                .set_offboard_animation_to_world(transform);
            Ok(())
        })
    }

    /// 0x82b4541c updates only the retained animation-to-world P column.
    pub(crate) fn try_set_pair_offboard_animation_translation(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
        translation: SkeletonRootVector,
    ) -> Result<(), OffboardLocalFootTargetsError> {
        self.try_with_offboard_ik_pair(board, skeleton, expected, |normal| {
            normal
                .root_transforms_mut()
                .set_offboard_animation_translation(translation);
            Ok(())
        })
    }

    /// UpdateRootTransformsOffBoard 0x82b45740..758 publishes the inverse
    /// into the same Skeleton's world-to-animation owner.
    pub(crate) fn try_set_pair_offboard_world_to_animation_inverse(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
        inverse: SkeletonRootMatrix,
    ) -> Result<(), OffboardLocalFootTargetsError> {
        self.try_with_offboard_ik_pair(board, skeleton, expected, |normal| {
            normal
                .root_transforms_mut()
                .set_offboard_world_to_animation_inverse(inverse);
            Ok(())
        })
    }

    /// `SetOffBoardSkateRootTransform` 0x82b4d314..320 writes all four
    /// columns on the same retained Skeleton, after the caller's attribute
    /// and Processed gate reads.
    pub(crate) fn try_set_pair_offboard_skate_root_transform(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
        transform: SkeletonRootMatrix,
    ) -> Result<(), OffboardLocalFootTargetsError> {
        self.try_with_offboard_ik_pair(board, skeleton, expected, |normal| {
            normal.set_offboard_skate_root_transform(transform);
            Ok(())
        })
    }

    /// `Skeleton::UpdateOffBoard` 0x82b5c428: first byte store on the same
    /// retained Skeleton root-transform owner, before its SkateRoot child.
    pub(crate) fn try_begin_pair_offboard_skeleton_update(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
    ) -> Result<(), OffboardLocalFootTargetsError> {
        self.try_with_offboard_ik_pair(board, skeleton, expected, |normal| {
            normal.root_transforms_mut().begin_offboard_update();
            Ok(())
        })
    }

    fn try_with_offboard_ik_pair<T>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
        operation: impl FnOnce(&mut SkeletonNormalAssembly) -> Result<T, OffboardLocalFootTargetsError>,
    ) -> Result<T, OffboardLocalFootTargetsError> {
        self.try_with_offboard_pair(board, skeleton, expected, operation)?
    }

    /// Shared retained-pair access for off-board IK and Pushing501. Preserve
    /// the existing pool, attachment and expected-owner checks in one place.
    pub(crate) fn try_with_offboard_pair<T>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
        operation: impl FnOnce(&mut SkeletonNormalAssembly) -> T,
    ) -> Result<T, OffboardLocalFootTargetsError> {
        use OffboardLocalFootTargetsError as E;
        self.skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, owner| {
                if !Arc::ptr_eq(owner, expected) {
                    return Err(E::ForeignOwner);
                }
                Ok(operation(normal))
            })
            .map_err(E::Pair)?
    }

    /// ApplyIK locomotion branch 0x82dcd90c..95c: only influenced feet get
    /// world-space targets at blend 1. No local targets or surface normals.
    pub(crate) fn try_set_pair_offboard_locomotion_foot_targets(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
    ) -> Result<(), OffboardLocalFootTargetsError> {
        use OffboardLocalFootTargetsError as E;
        let processed_owner = Arc::clone(
            expected
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        self.try_with_offboard_ik_pair(board, skeleton, expected, |normal| {
            for foot in 0..2 {
                let target = {
                    let remaining = expected.remaining().lock().map_err(|_| E::Poisoned)?;
                    let feet = remaining.feet_ik.as_ref().ok_or(E::MissingFeetIk)?;
                    if !feet.binding.matches(expected, &processed_owner, skeleton) {
                        return Err(E::ForeignOwner);
                    }
                    let processed = processed_owner.lock().map_err(|_| E::Poisoned)?;
                    let influence = processed
                        .prior_output()
                        .offboard_foot_influence(foot)
                        .ok_or(E::MissingLocomotionInfluence { foot })?;
                    // BO4/BI25 at 82dcd928/948 skips !GT, including NaN.
                    (influence > 0.0).then_some(if foot == 0 {
                        feet.left_foot_position
                    } else {
                        feet.right_foot_position
                    })
                };
                if let Some(target) = target {
                    normal
                        .try_set_world_foot_ik_target(foot == 0, target, 1.0)
                        .map_err(E::Setter)?;
                }
            }
            Ok(())
        })
    }

    /// ApplyIK82dcd98c..9f0: planted feet use the world setter at blend1;
    /// free feet use the existing local setter. Reload the planted-side timer
    /// after its setter, before proceeding to the other foot.
    pub(crate) fn try_set_pair_offboard_ground_foot_targets(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
        blend: f32,
        cvars: &InputCvarReferences,
    ) -> Result<(), OffboardLocalFootTargetsError> {
        use OffboardLocalFootTargetsError as E;
        self.try_with_offboard_ik_pair(board, skeleton, expected, |normal| {
            let mut remaining = expected.remaining().lock().map_err(|_| E::Poisoned)?;
            let feet = remaining.feet_ik.as_mut().ok_or(E::MissingFeetIk)?;
            if feet.state == 0 {
                normal
                    .try_set_world_foot_ik_target(true, feet.left_foot_position, 1.0)
                    .map_err(E::Setter)?;
                feet.left_fixed_blend_out_timer = cvars
                    .try_ob_ik_time_to_blend_out_fixed_foot_pos()
                    .map_err(E::Cvar)?;
            } else {
                normal
                    .try_set_local_foot_ik_target(true, feet.left_foot_position, blend)
                    .map_err(E::Setter)?;
            }
            if feet.state == 1 {
                normal
                    .try_set_world_foot_ik_target(false, feet.right_foot_position, 1.0)
                    .map_err(E::Setter)?;
                feet.right_fixed_blend_out_timer = cvars
                    .try_ob_ik_time_to_blend_out_fixed_foot_pos()
                    .map_err(E::Cvar)?;
            } else {
                normal
                    .try_set_local_foot_ik_target(false, feet.right_foot_position, blend)
                    .map_err(E::Setter)?;
            }
            Ok(())
        })
    }

    /// Source ApplyIK tests left surface and normal before right. Missing
    /// options skip that setter and all its linked reads.
    pub(crate) fn try_set_pair_offboard_foot_normals(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<PhysicalPlayerAssemblyOwner>,
        cvars: &InputCvarReferences,
    ) -> Result<(), OffboardLocalFootTargetsError> {
        use OffboardLocalFootTargetsError as E;
        self.try_with_offboard_ik_pair(board, skeleton, expected, |normal| {
            let left = {
                let remaining = expected.remaining().lock().map_err(|_| E::Poisoned)?;
                let feet = remaining.feet_ik.as_ref().ok_or(E::MissingFeetIk)?;
                (feet.left_foot_down
                    && feet.result_left.valid
                    && feet.result_left.normal.xyz[1] > 0.7)
                    .then_some(feet.ground_normal_left)
            };
            if let Some(left) = left {
                normal
                    .try_set_offboard_foot_ik_target_normal(true, left, cvars)
                    .map_err(E::Normal)?;
            }
            let right = {
                let remaining = expected.remaining().lock().map_err(|_| E::Poisoned)?;
                let feet = remaining.feet_ik.as_ref().ok_or(E::MissingFeetIk)?;
                (feet.right_foot_down
                    && feet.result_right.valid
                    && feet.result_right.normal.xyz[1] > 0.7)
                    .then_some(feet.ground_normal_right)
            };
            if let Some(right) = right {
                normal
                    .try_set_offboard_foot_ik_target_normal(false, right, cvars)
                    .map_err(E::Normal)?;
            }
            Ok(())
        })
    }
}
