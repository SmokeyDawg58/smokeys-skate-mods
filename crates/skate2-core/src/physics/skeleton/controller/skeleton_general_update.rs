//! Grounded root publication and `Skeleton::GeneralUpdate`'s velocity/hooks.
//! References: 0x82b5e490..0x82b5e9d8, 0x82b5ba88..0x82b5bb18 and
//! `UpdateHookPositions` 0x82b4d380..0x82b4d600. The caller owns the subsequent
//! conditional full reset, IK and drives; their completion is not implied here.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained getters and flags await the remaining native lifecycle consumers"
    )
)]

use crate::physics_state_ids::{PHYSICS_STATE_INVALID, PHYSICS_STATE_TELEPORTING};
use crate::{
    animation::AnimationMatrix,
    reckoning_numeric::ReckoningNumeric,
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::{
        SkeletonRootBoard, SkeletonRootMatrix, SkeletonRootTransforms, SkeletonRootVector,
    },
};

use crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::{
    GroundedInputError, GroundedOutputWrite,
};
struct ControlledGroundedOutput<'a> {
    orientation: std::cell::RefCell<&'a mut SkeletonRootMatrix>,
    flags: std::cell::RefCell<&'a mut u32>,
}
impl GroundedOutputWrite for ControlledGroundedOutput<'_> {
    fn write_orientation(&self, value: SkeletonRootMatrix) -> Result<(), GroundedInputError> {
        **self.orientation.borrow_mut() = value;
        Ok(())
    }
    fn clear_orientation_flag(&self) -> Result<(), GroundedInputError> {
        **self.flags.borrow_mut() &= !0x0008_0000;
        Ok(())
    }
}

const IDENTITY: AnimationMatrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

/// The actual drives-owned hook's two Parts. Setters preserve the requested
/// cache's W knowledge after updating the existing attached body, and may not
/// substitute a detached hook or a serialized transform publication.
pub(crate) trait SkeletonGeneralUpdateHooks {
    type Error;
    fn transform(&mut self, part: usize) -> Result<SkeletonRootMatrix, Self::Error>;
    fn set_transform(
        &mut self,
        part: usize,
        transform: &SkeletonRootMatrix,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkeletonGroundedSuffixAssertion {
    PredictedBoardPosition,
    AnimationToWorld,
    SkateboardOrientation,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonGroundedSuffixError<B> {
    Assertion(SkeletonGroundedSuffixAssertion),
    Board(B),
    Processed(GroundedInputError),
}
impl<B: std::fmt::Debug> std::fmt::Display for SkeletonGroundedSuffixError<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Skeleton grounded suffix: {self:?}")
    }
}
impl<B: std::fmt::Debug> std::error::Error for SkeletonGroundedSuffixError<B> {}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonHookUpdateError<H, N> {
    Hook(H),
    Numeric(N),
}
impl<H: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for SkeletonHookUpdateError<H, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Skeleton hook positions: {self:?}")
    }
}
impl<H: std::fmt::Debug, N: std::fmt::Debug> std::error::Error for SkeletonHookUpdateError<H, N> {}

/// One retained owner for Skeleton+2ec0/+2fc0/+3c80/+3da0/+3db0 and the
/// next trajectory+2e80 and three flags+3dd4/+3e42/+3e44. Constructor0x82b56d70
/// initializes these matrices I/J/K/P0, both vectors0 and all flags false. The
/// `AddSkateboardError` zero store is0x82b573cc. This state is not Clone.
pub(crate) struct SkeletonGeneralUpdate {
    physics_skateboard: SkeletonRootMatrix,
    skate_root: SkeletonRootMatrix,
    animation_board_to_physics_board: SkeletonRootMatrix,
    next_trajectory_transform: SkeletonRootMatrix,
    animation_world_velocity: SkeletonRootVector,
    last_animation_world_position: SkeletonRootVector,
    add_skateboard_error: bool,
    teleporting: bool,
    want_respawn: bool,
}

impl SkeletonGeneralUpdate {
    pub(crate) fn new() -> Self {
        let identity = SkeletonRootMatrix::known(IDENTITY);
        Self {
            physics_skateboard: identity,
            skate_root: identity,
            animation_board_to_physics_board: identity,
            next_trajectory_transform: identity,
            animation_world_velocity: SkeletonRootVector::known([0.0; 4]),
            last_animation_world_position: SkeletonRootVector::known([0.0; 4]),
            add_skateboard_error: false,
            teleporting: false,
            want_respawn: false,
        }
    }

    /// Animated82b5c344..350 precedes `ApplyBoardAnimation`.
    pub(crate) fn set_animated_skate_root(&mut self, value: SkeletonRootMatrix) {
        self.skate_root = value;
    }
    /// Animated82b5c388..3a0, after the actual Part0 transform has been set.
    pub(crate) fn set_animated_physics_skateboard(&mut self, value: SkeletonRootMatrix) {
        self.physics_skateboard = value;
    }
    pub(crate) fn physics_skateboard(&self) -> &SkeletonRootMatrix {
        &self.physics_skateboard
    }
    pub(crate) fn skate_root(&self) -> &SkeletonRootMatrix {
        &self.skate_root
    }
    pub(crate) fn animation_board_to_physics_board(&self) -> &SkeletonRootMatrix {
        &self.animation_board_to_physics_board
    }
    /// Retained Skeleton+2e80; constructor stores 0x82b57048..0x82b57070.
    /// Distinct from current trajectory+2e00, and preserved by skeleton resets.
    pub(crate) fn next_trajectory_transform(&self) -> &SkeletonRootMatrix {
        &self.next_trajectory_transform
    }
    /// ProcessData82b5ee48..ee60 writes the same retained `NextTrajectory`.
    pub(crate) fn next_trajectory_transform_mut(&mut self) -> &mut SkeletonRootMatrix {
        &mut self.next_trajectory_transform
    }
    /// Grounded tail 0x82b5ea04/08/0c/10, only after full `GeneralUpdate`.
    /// Exclusive ownership excludes intermediate per-column observers.
    pub(crate) fn finish_grounded_trajectory(&mut self) {
        let identity = SkeletonRootMatrix::known(IDENTITY);
        let mut columns = *self.next_trajectory_transform.columns();
        columns[0] = identity.columns()[0];
        columns[3] = identity.columns()[3];
        columns[1] = identity.columns()[1];
        columns[2] = identity.columns()[2];
        self.next_trajectory_transform = SkeletonRootMatrix::from_columns(columns);
    }
    pub(crate) fn animation_world_velocity(&self) -> SkeletonRootVector {
        self.animation_world_velocity
    }
    pub(crate) fn last_animation_world_position(&self) -> SkeletonRootVector {
        self.last_animation_world_position
    }
    /// Ground.Update82da1040 writes the existing Skeleton+3dd4 flag.
    pub(crate) fn mark_add_skateboard_error(&mut self) {
        self.add_skateboard_error = true;
    }

    pub(crate) fn add_skateboard_error(&self) -> bool {
        self.add_skateboard_error
    }
    pub(crate) fn teleporting(&self) -> bool {
        self.teleporting
    }
    /// Skeleton::TeleportNow 0x82b5eb50/eb60 brackets UpdateGroundSkateboard.
    pub(crate) fn set_teleporting_now(&mut self, value: bool) {
        self.teleporting = value;
    }
    /// `UpdatePostPhysics82b57650` after actual bad-data reset and IK reset.
    pub(crate) fn mark_want_respawn_after_bad_data(&mut self) {
        self.want_respawn = true;
    }
    pub(crate) fn want_respawn(&self) -> bool {
        self.want_respawn
    }
    /// Source59848, only after successful teleport-request output publication.
    pub(crate) fn clear_want_respawn_after_fill(&mut self) {
        self.want_respawn = false;
    }

    /// First store of `ResetSkeleton`, 0x82b45c14; the caller must then execute
    /// the actual body and state reset. No other retained history is cleared.
    pub(crate) fn clear_add_skateboard_error(&mut self) {
        self.add_skateboard_error = false;
    }

    /// 0x82b5bb30..0x82b5bb4c, only AFTER a failed hook result's full reset.
    /// The input is `ProcessedPhysIn::mCurrentPhysicsState`+8e8, not+8e4.
    pub(crate) fn mark_want_respawn_after_reset(&mut self, current_state: i32) {
        if current_state != PHYSICS_STATE_INVALID && current_state != PHYSICS_STATE_TELEPORTING {
            self.want_respawn = true;
        }
    }

    /// Grounded suffix0x82b5e494..0x82b5e9d4. The mutable output is the PASSED
    /// processed orientation+0/flags+8c4, distinct from the retained processed
    /// input used later by `GeneralUpdate`. Board transform is sampled again
    /// only after output publication and clearing bit0x80000.
    pub(crate) fn try_grounded_suffix<B: SkeletonRootBoard>(
        &mut self,
        roots: &SkeletonRootTransforms,
        mapped_zero: &AnimationMatrix,
        passed_orientation: &mut SkeletonRootMatrix,
        passed_flags: &mut u32,
        board: &mut B,
    ) -> Result<(), SkeletonGroundedSuffixError<B::Error>> {
        let output = ControlledGroundedOutput {
            orientation: std::cell::RefCell::new(passed_orientation),
            flags: std::cell::RefCell::new(passed_flags),
        };
        self.try_grounded_suffix_write(roots, mapped_zero, &output, board)
    }

    pub(crate) fn try_grounded_suffix_write<B: SkeletonRootBoard>(
        &mut self,
        roots: &SkeletonRootTransforms,
        mapped_zero: &AnimationMatrix,
        output: &impl GroundedOutputWrite,
        board: &mut B,
    ) -> Result<(), SkeletonGroundedSuffixError<B::Error>> {
        use SkeletonGroundedSuffixAssertion as Assertion;
        use SkeletonGroundedSuffixError as Error;
        let predicted = roots.predicted_board_position();
        if predicted.xyz.iter().any(|value| value.is_nan()) {
            return Err(Error::Assertion(Assertion::PredictedBoardPosition));
        }
        if roots.animation_to_world().has_nan_xyz() {
            return Err(Error::Assertion(Assertion::AnimationToWorld));
        }
        // Native deliberately recomposes raw+2d40; the retained normalized
        // world matrix is only asserted above. Even identity's zero products
        // execute, preserving the source exceptional-value and W behavior.
        let board_space = roots
            .animation_to_board()
            .compose(&SkeletonRootMatrix::known(*mapped_zero));
        let orientation = SkeletonRootMatrix::translation(predicted).compose(&board_space);
        if orientation.has_nan_xyz() {
            return Err(Error::Assertion(Assertion::SkateboardOrientation));
        }
        output
            .write_orientation(orientation)
            .map_err(Error::Processed)?;
        output.clear_orientation_flag().map_err(Error::Processed)?;
        let physical = board.transform_typed().map_err(Error::Board)?;
        self.physics_skateboard = physical;
        self.skate_root = self.physics_skateboard;
        Ok(())
    }

    /// `GeneralUpdate`0x82b5baa0..0x82b5bb14. One estimate and two fused
    /// refinements precede the velocity and last-position commits. The timestep
    /// comes from retained processed+93c, not the earlier root-input timestep.
    pub(crate) fn try_update_velocity<N: ReckoningNumeric>(
        &mut self,
        roots: &SkeletonRootTransforms,
        retained_time_delta: f32,
        numeric: &mut N,
    ) -> Result<(), N::Error> {
        let position = roots.animation_to_world().columns()[3];
        let delta = SkeletonRootVector {
            xyz: std::array::from_fn(|lane| {
                position.xyz[lane] - self.last_animation_world_position.xyz[lane]
            }),
            w: position
                .w
                .zip(self.last_animation_world_position.w)
                .map(|(current, last)| current - last),
        };
        let estimate = numeric.reciprocal_estimate(retained_time_delta)?;
        let reciprocal = refine_drive_reciprocal(retained_time_delta, estimate);
        self.animation_world_velocity = SkeletonRootVector {
            xyz: delta.xyz.map(|value| reciprocal * value),
            w: delta.w.map(|value| reciprocal * value),
        };
        self.last_animation_world_position = position;
        Ok(())
    }

    /// `UpdateHookPositions`0x82b4d380. True means the source accepts the new
    /// placement; false requests the caller's full reset. Each actual hook
    /// service is lazy and retains earlier cache/body/proxy commits on refusal.
    pub(crate) fn try_update_hooks<H: SkeletonGeneralUpdateHooks, N: ReckoningNumeric>(
        &mut self,
        roots: &SkeletonRootTransforms,
        adjusted_hips: &AnimationMatrix,
        adjusted_skate: &AnimationMatrix,
        proxy_target: &mut Option<SkeletonRootVector>,
        hooks: &mut H,
        numeric: &mut N,
    ) -> Result<bool, SkeletonHookUpdateError<H::Error, N::Error>> {
        use SkeletonHookUpdateError as Error;
        let hips = roots
            .animation_to_world()
            .compose(&SkeletonRootMatrix::known(*adjusted_hips));
        let world_skate = roots
            .animation_to_world()
            .compose(&SkeletonRootMatrix::known(*adjusted_skate));
        self.animation_board_to_physics_board = roots.board_inverse().compose(&world_skate);
        let old_hips = hooks.transform(0).map_err(Error::Hook)?;
        let delta: [f32; 4] = std::array::from_fn(|lane| {
            if lane == 3 {
                // The sole consumer below is dot3. This local unused lane
                // never becomes a retained transform or a claimed native W.
                0.0
            } else {
                old_hips.columns()[3].xyz[lane] - hips.columns()[3].xyz[lane]
            }
        });
        hooks.set_transform(0, &hips).map_err(Error::Hook)?;
        hooks
            .set_transform(1, &self.skate_root)
            .map_err(Error::Hook)?;
        // Raw82b4d5b0/d5b4 copies all native lanes. An attached Part's
        // provenance may leave W unknown; preserve it in the same State target.
        *proxy_target = Some(self.skate_root.columns()[3]);
        let teleporting = self.teleporting;
        // This service executes after all stores even when teleporting. Raw
        // 0x41990008 at0x82b4d5e8 tests ordered GT, so NaN alone does not reset.
        let squared = numeric.dot3(delta, delta).map_err(Error::Numeric)?;
        Ok(!teleporting && squared.partial_cmp(&1.0) != Some(std::cmp::Ordering::Greater))
    }
}

#[cfg(test)]
#[path = "skeleton_general_update_tests.rs"]
mod tests;
