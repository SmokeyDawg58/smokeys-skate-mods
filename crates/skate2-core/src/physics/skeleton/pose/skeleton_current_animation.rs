//! Bounded `ProcessData` mapped-pose through adjusted root copies,
//! 0x82b5efe0..0x82b5f25c. The same-owner `ProcessData` caller surrounds this with
//! trajectory/attribute production and the processed-output/teleport suffix.
//! Offboard, landing on board, landing spring and retained offsets execute; remaining adjustments stop at
//! their source stage. Ground never implies a reset.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "current animation awaits Actor scheduling")
)]

use crate::physics_state_ids::{PHYSICS_CATEGORY_OFFBOARD, PHYSICS_STATE_LANDINGONDECK};
use crate::{
    animation::AnimationMatrix,
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_animation_record::{
        SkeletonAnimationRecord, SkeletonAnimationRecordError, SkeletonAnimationRecordInput,
    },
    skeleton_animation_targets::{SkeletonAnimationTargetError, SkeletonAnimationTargets},
    skeleton_data::SkeletonData,
    skeleton_ik_targets::{SkeletonIkTargetError, SkeletonIkTargets},
    skeleton_landing_adjust::{
        SkeletonLandingAdjust, SkeletonLandingAdjustAttributes, SkeletonLandingAdjustCvars,
        SkeletonLandingAdjustError, SkeletonLandingAdjustScalar,
    },
    skeleton_landing_attributes::SkeletonLandingVaultAttributes,
    skeleton_landing_on_board::{
        SkeletonLandingOnBoardAttributes, SkeletonLandingOnBoardCvars, SkeletonLandingOnBoardError,
    },
    skeleton_offboard_adjust::{SkeletonOffboardAdjustError, try_update_offboard_adjust},
    skeleton_physics_history::{SkeletonPhysicsHistory, SkeletonPhysicsHistoryNumeric},
    skeleton_root_transforms::{SkeletonRootInput, SkeletonRootMatrix},
    skeleton_skateboard_offset::{SkeletonSkateboardOffset, SkeletonSkateboardOffsetCvars},
};
use std::{fmt, sync::Arc};

#[path = "../../player/wipeout/physical_player_post_wipeout_foot_ik_current.rs"]
mod post_wipeout_foot_current;

#[path = "../movement/skeleton_process_data_continuation.rs"]
pub(crate) mod continuation;

#[cfg(test)]
use crate::skeleton_landing_on_board::SkeletonLandingOnBoardInput;

pub(crate) const PHYSICS_ANIMATION_CLASS: u64 = 0x74bc_9c93_affe_6aac;
pub(crate) const LANDING_ON_BOARD_BLEND: u64 = 0x6aea_4c4e_fbba_747b;

/// Current fields from Skeleton's retained processed collaborator, separately
/// borrowed from the passed output. Immutable snapshots exclude alias mutation
/// during callbacks; they do not implement the original processed-input owner.
#[cfg(test)]
pub(crate) struct SkeletonAdjustmentInput {
    pub(crate) category: i32,
    pub(crate) state: i32,
    pub(crate) filtered_state: i32,
    pub(crate) flags_8c4: u32,
    pub(crate) flags_8c8: u32,
    pub(crate) flags_8cc: u32,
    pub(crate) balance: f32,
    pub(crate) centre_of_mass_velocity: [f32; 4],
    pub(crate) up: [f32; 4],
    pub(crate) landing_on_board: SkeletonLandingOnBoardInput,
}

pub(crate) trait SkeletonCurrentAnimationCvars: SkeletonAnimationRecordInput {
    fn debug_landing_adjust(&mut self) -> Result<u8, Self::Error>;
    fn landing_require_debug_overlay(&mut self) -> Result<(), Self::Error>;
    fn landing_use_physics_com(&mut self) -> Result<u8, Self::Error>;
    fn landing_first_frame_hack(&mut self) -> Result<f32, Self::Error>;
    fn landing_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error>;
    /// Offboard samples the current blend count twice at distinct source stages.
    fn offboard_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error>;
    fn landing_on_board_show_adjust(&mut self) -> Result<u8, Self::Error>;
    fn landing_on_board_require_debug_overlay(&mut self) -> Result<(), Self::Error>;
    fn landing_on_board_negate_y(&mut self) -> Result<u8, Self::Error>;
    fn landing_on_board_frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error>;
}

struct OffboardCvars<'a, C>(&'a mut C);
impl<C: SkeletonCurrentAnimationCvars> SkeletonSkateboardOffsetCvars for OffboardCvars<'_, C> {
    type Error = C::Error;
    fn frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.0.offboard_frames_to_blend_away_offset()
    }
}

pub(crate) trait SkeletonCurrentAnimationAttributeRead:
    SkeletonLandingAdjustAttributes
{
    fn landing_max_velocity(&mut self) -> Result<f32, Self::Error> {
        self.scalar(SkeletonLandingAdjustScalar::MaximumVelocity)
    }
    fn landing_on_board_blend(&mut self, time_to_land: f32) -> Result<f32, Self::Error>;
}

struct SpringCvars<'a, C>(&'a mut C);
impl<C: SkeletonCurrentAnimationCvars> SkeletonLandingAdjustCvars for SpringCvars<'_, C> {
    type Error = C::Error;
    fn debug_landing_adjust(&mut self) -> Result<u8, Self::Error> {
        self.0.debug_landing_adjust()
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.0.landing_require_debug_overlay()
    }
    fn use_physics_com(&mut self) -> Result<u8, Self::Error> {
        self.0.landing_use_physics_com()
    }
    fn first_frame_hack(&mut self) -> Result<f32, Self::Error> {
        self.0.landing_first_frame_hack()
    }
    fn frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.0.landing_frames_to_blend_away_offset()
    }
}

struct LandingAttributes<'a, A>(&'a mut A);
impl<A: SkeletonCurrentAnimationAttributeRead> SkeletonLandingOnBoardAttributes
    for LandingAttributes<'_, A>
{
    type Error = A::Error;
    fn blend(&mut self, time_to_land: f32) -> Result<f32, Self::Error> {
        self.0.landing_on_board_blend(time_to_land)
    }
}
struct LandingCvars<'a, C>(&'a mut C);
impl<C: SkeletonCurrentAnimationCvars> SkeletonLandingOnBoardCvars for LandingCvars<'_, C> {
    type Error = C::Error;
    fn show_landing_adjust(&mut self) -> Result<u8, Self::Error> {
        self.0.landing_on_board_show_adjust()
    }
    fn require_debug_overlay(&mut self) -> Result<(), Self::Error> {
        self.0.landing_on_board_require_debug_overlay()
    }
    fn negate_y(&mut self) -> Result<u8, Self::Error> {
        self.0.landing_on_board_negate_y()
    }
    fn frames_to_blend_away_offset(&mut self) -> Result<f32, Self::Error> {
        self.0.landing_on_board_frames_to_blend_away_offset()
    }
}

/// Separate global landing-history and Skeleton-owned landing-on-board
/// instances. Both bind the original default collection in this immutable
/// Vault projection; editor replacement and native construction timing remain open.
pub(crate) struct SkeletonCurrentAnimationAttributes {
    global: SkeletonLandingVaultAttributes,
    skeleton: AttributeRuntimeInstance,
}
impl SkeletonCurrentAnimationAttributes {
    /// `IsSkateboardAnimating82b45b60`, raw key5b88/90/9c. Null data uses
    /// `DefaultDataArea4`; each query reads the SAME retained Skeleton instance.
    pub(crate) fn skateboard_animation_threshold(&self) -> Result<f32, AttributeRuntimeError> {
        const KEY: u64 = 0xb997_72c1_07b9_a8a6;
        let data = self.skeleton.try_get_data(KEY, 0)?;
        let bytes = data.as_ref().map_or(&[0; 4][..], |data| data.bytes());
        if bytes.len() != 4 {
            return Err(AttributeRuntimeError::PayloadSize {
                key: KEY,
                expected: 4,
                actual: bytes.len(),
            });
        }
        Ok(f32::from_bits(u32::from_be_bytes(
            bytes.try_into().expect("checked scalar"),
        )))
    }
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self::from_constructor(
            vault,
            vault.instance(PHYSICS_ANIMATION_CLASS, 0xd7ed_bd36_2d7d_2152),
        )
    }
    /// Retain the SAME Skeleton+14 Instance constructed at82b56e00. The
    /// global landing attributes are the separate source global instance.
    pub(crate) fn from_constructor(
        vault: &Arc<AttributeRuntimeVault>,
        skeleton: AttributeRuntimeInstance,
    ) -> Self {
        Self {
            global: SkeletonLandingVaultAttributes::bind(vault),
            skeleton,
        }
    }
}
impl SkeletonLandingAdjustAttributes for SkeletonCurrentAnimationAttributes {
    type Error = AttributeRuntimeError;
    fn scalar(&mut self, attribute: SkeletonLandingAdjustScalar) -> Result<f32, Self::Error> {
        self.global.scalar(attribute)
    }
    fn manual_blend(&mut self, time: f32) -> Result<f32, Self::Error> {
        self.global.manual_blend(time)
    }
    fn grind_blend(&mut self, time: f32) -> Result<f32, Self::Error> {
        self.global.grind_blend(time)
    }
    fn coffin_bounce(&mut self, time: f32) -> Result<f32, Self::Error> {
        self.global.coffin_bounce(time)
    }
}
impl SkeletonCurrentAnimationAttributeRead for SkeletonCurrentAnimationAttributes {
    fn landing_on_board_blend(&mut self, time_to_land: f32) -> Result<f32, Self::Error> {
        // Source82b419c8 fetches on each active call; only a genuinely missing
        // pointer uses DefaultDataArea50 (original first80 bytes are zero).
        let data = self.skeleton.try_get_data(LANDING_ON_BOARD_BLEND, 0)?;
        let bytes = data.as_ref().map_or(&[0; 0x50][..], |data| data.bytes());
        if bytes.len() != 0x50 {
            return Err(AttributeRuntimeError::PayloadSize {
                key: LANDING_ON_BOARD_BLEND,
                expected: 0x50,
                actual: bytes.len(),
            });
        }
        // PointNegGraphData8 metadata0..10 is not consumed by this caller.
        let word = |offset| {
            f32::from_bits(u32::from_be_bytes(
                bytes[offset..offset + 4]
                    .try_into()
                    .expect("checked graph payload"),
            ))
        };
        let graph = crate::physics::PointGraph::<8> {
            x: std::array::from_fn(|index| word(0x10 + 4 * index)),
            y: std::array::from_fn(|index| word(0x30 + 4 * index)),
        };
        Ok(graph.evaluate(time_to_land))
    }
}

impl crate::skeleton_ik_prephysics::SkeletonIkPrePhysicsAttributes
    for SkeletonCurrentAnimationAttributes
{
    type Error = AttributeRuntimeError;
    fn minimum_angle_degrees(&mut self) -> Result<f32, Self::Error> {
        crate::skeleton_ik_prephysics::SkeletonIkPrePhysicsAttributes::minimum_angle_degrees(
            &mut self.global,
        )
    }
    fn maximum_angle_degrees(&mut self) -> Result<f32, Self::Error> {
        crate::skeleton_ik_prephysics::SkeletonIkPrePhysicsAttributes::maximum_angle_degrees(
            &mut self.global,
        )
    }
}

#[derive(Debug)]
pub(crate) enum SkeletonCurrentAnimationError<A, C> {
    Input(continuation::AnimationInputError),
    Targets(SkeletonAnimationTargetError),
    Ik(SkeletonIkTargetError),
    Offboard(SkeletonOffboardAdjustError<C>),
    LandingOnBoard(
        SkeletonLandingOnBoardError<
            A,
            C,
            crate::reckoning_numeric::ReckoningHostNumericError,
            std::convert::Infallible,
        >,
    ),
    Attribute(A),
    Cvar(C),
    LandingAdjust(SkeletonLandingAdjustError<A, C>),
    Record(SkeletonAnimationRecordError<C>),
}
impl<A: fmt::Debug, C: fmt::Debug> fmt::Display for SkeletonCurrentAnimationError<A, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(e) => e.fmt(f),
            Self::Targets(e) => write!(
                f,
                "animation Part {} needs bone {}, pose count {}",
                e.part, e.bone, e.pose_count
            ),
            Self::Ik(e) => write!(f, "animation IK targets: {e}"),
            Self::Offboard(e) => write!(f, "animation offboard adjustment: {e}"),
            Self::LandingOnBoard(e) => write!(f, "animation landing on board: {e:?}"),
            Self::Attribute(e) => write!(f, "animation attribute: {e:?}"),
            Self::Cvar(e) => write!(f, "animation cvar: {e:?}"),
            Self::LandingAdjust(e) => write!(f, "animation landing spring: {e:?}"),
            Self::Record(e) => write!(f, "animation record: {e:?}"),
        }
    }
}
impl<A: fmt::Debug, C: fmt::Debug> std::error::Error for SkeletonCurrentAnimationError<A, C> {}

#[derive(Debug)]
pub(crate) struct SkeletonIkUnconstructed;

/// Identity AnimToWorld for the controlled test entry points, whose fixtures
/// also use an identity WorldToAnim.
#[cfg(test)]
const TEST_ANIMATION_TO_WORLD: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0; 4],
];

/// One non-Clone projection on the actual Skeleton owner. The already-owned
/// mapped buffers and physics masses are borrowed, never copied into another
/// authority. Unimplemented active-state producers have no public setters.
pub(crate) struct SkeletonCurrentAnimation {
    ik: SkeletonIkTargets,
    ik_runtime: Option<crate::skeleton_ik::SkeletonIk>,
    record: SkeletonAnimationRecord,
    landing: SkeletonLandingAdjust,
    offset: SkeletonSkateboardOffset,
    // Projection of the separately retained native GrindAirAdjust collaborator.
    // Its full HiLOD construction/lifetime and active producers remain open.
    #[cfg(test)]
    grind_activated: bool,
    #[cfg(test)]
    grind_adjusting: bool,
    raw_skateboard: AnimationMatrix,
    hips: AnimationMatrix,
    skateboard: AnimationMatrix,
    last_deck_zy: f32,
    // Host readiness only; stale retained copies are visible but cannot be
    // exported as the completed result of a later failed attempt.
    completed: bool,
}
impl SkeletonCurrentAnimation {
    pub(crate) fn new(data: Arc<SkeletonData>) -> Self {
        Self::with_targets(SkeletonIkTargets::new(data))
    }

    pub(crate) fn before_skeleton_data_init_with_record(record: SkeletonAnimationRecord) -> Self {
        Self::with_targets_and_record(SkeletonIkTargets::before_skeleton_data_init(), record)
    }

    fn with_targets(ik: SkeletonIkTargets) -> Self {
        Self::with_targets_and_record(ik, SkeletonAnimationRecord::new())
    }

    fn with_targets_and_record(ik: SkeletonIkTargets, record: SkeletonAnimationRecord) -> Self {
        Self {
            ik,
            ik_runtime: None,
            record,
            landing: SkeletonLandingAdjust::new(),
            offset: SkeletonSkateboardOffset::new(),
            #[cfg(test)]
            grind_activated: false,
            #[cfg(test)]
            grind_adjusting: false,
            raw_skateboard: FRESH_VOLUME_TRANSFORM,
            hips: FRESH_VOLUME_TRANSFORM,
            skateboard: FRESH_VOLUME_TRANSFORM,
            last_deck_zy: 0.0,
            completed: false,
        }
    }

    pub(crate) fn bind_initialized_skeleton_data(&mut self, data: Arc<SkeletonData>) {
        self.ik.bind_initialized_data(data);
    }

    /// Four original name lookups in Init82b5eb80. This explicit projection
    /// does not claim that the remaining native Skeleton/IK Init has run.
    pub(crate) fn initialize_ik_targets(&mut self) {
        self.ik.initialize();
    }

    /// Host readiness is invalidated when another controlled entry point
    /// replaces mapped rows without completing this adjustment pipeline.
    pub(crate) fn invalidate_mapped_pose(&mut self) {
        self.completed = false;
    }
    pub(crate) fn ik(&self) -> &SkeletonIkTargets {
        &self.ik
    }
    pub(crate) fn ik_runtime(&self) -> Option<&crate::skeleton_ik::SkeletonIk> {
        self.ik_runtime.as_ref()
    }
    pub(crate) fn ik_runtime_mut(&mut self) -> Option<&mut crate::skeleton_ik::SkeletonIk> {
        self.ik_runtime.as_mut()
    }
    pub(crate) fn try_reset_push_abort(&mut self) -> Result<(), SkeletonIkUnconstructed> {
        self.ik_runtime
            .as_mut()
            .ok_or(SkeletonIkUnconstructed)?
            .reset_push_abort();
        Ok(())
    }

    pub(crate) fn try_finish_skeleton_init(&mut self) -> Result<(), SkeletonIkUnconstructed> {
        self.ik_runtime
            .as_mut()
            .ok_or(SkeletonIkUnconstructed)?
            .finish_skeleton_init();
        Ok(())
    }

    pub(crate) fn try_enable_ik(&mut self) -> Result<(), SkeletonIkUnconstructed> {
        self.ik_runtime
            .as_mut()
            .ok_or(SkeletonIkUnconstructed)?
            .enable();
        Ok(())
    }

    pub(crate) fn try_disable_ik(&mut self) -> Result<(), SkeletonIkUnconstructed> {
        self.ik_runtime
            .as_mut()
            .ok_or(SkeletonIkUnconstructed)?
            .disable();
        Ok(())
    }

    /// Same retained runtime; the normal owner supplies completed local input.
    pub(crate) fn try_update_ik_prefix(
        &mut self,
        data: crate::skeleton_ik::SkeletonIkDataIn,
        status_flags: u32,
        blend_flags: u32,
    ) -> Result<(), crate::skeleton_ik::SkeletonIkUpdateError> {
        self.ik_runtime
            .as_mut()
            .ok_or(crate::skeleton_ik::SkeletonIkUpdateError::Unconstructed)?
            .try_update_prefix(data, status_flags, blend_flags)
    }
    pub(crate) fn try_update_ik_prefix_read(
        &mut self,
        data: crate::skeleton_ik::SkeletonIkDataIn,
        read_status: impl FnOnce()->Result<u32,crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError>,
        read_blend: impl FnMut()->Result<bool,crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError>,
    ) -> Result<(), crate::skeleton_ik::SkeletonIkUpdateError> {
        self.ik_runtime
            .as_mut()
            .ok_or(crate::skeleton_ik::SkeletonIkUpdateError::Unconstructed)?
            .try_update_prefix_read(data, read_status, read_blend)
    }

    #[expect(
        clippy::type_complexity,
        reason = "distinct numerical and cvar failures"
    )]
    pub(crate) fn try_update_ik_transforms<N, R, C>(
        &mut self,
        cvars: &mut C,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<(), crate::skeleton_ik::SkeletonIkStagesError<N::Error, R::Error, C::Error>>
    where
        N: crate::reckoning_numeric::ReckoningNumeric,
        R: crate::reckoning_sin_cos::ReckoningSinCosNumeric,
        C: crate::skeleton_ik_state::initial::SkeletonIkInitialCvars,
    {
        self.ik_runtime
            .as_mut()
            .ok_or(crate::skeleton_ik::SkeletonIkStagesError::Prefix(
                crate::skeleton_ik::SkeletonIkUpdateError::Unconstructed,
            ))?
            .try_update_transforms(&self.ik, self.record.poses(), cvars, numeric, rounding)
    }
    /// Remaining Update calls 0x82b59c98..0x82b59cb0 use the same retained
    /// IK/target authority and the actual mapped rows consumed next by Drives.
    #[expect(clippy::type_complexity)]
    pub(crate) fn try_update_ik_suffix<P, S, N, R>(
        &mut self,
        processed: &P,
        rows: &mut SkeletonAnimationTargets,
        attributes: &mut SkeletonCurrentAnimationAttributes,
        services: &mut S,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<
        (),
        crate::skeleton_ik::SkeletonIkSuffixError<
            crate::skeleton_animation_targets::SkeletonIkPoseError,
            AttributeRuntimeError,
            <S as crate::skeleton_ik_state::contact::SkeletonIkContactServices>::Error,
            N::Error,
            R::Error,
        >,
    >
    where
        P: crate::skeleton_ik_state::contact::SkeletonIkContactRead,
        S: crate::skeleton_ik::SkeletonIkSuffixServices,
        N: crate::reckoning_numeric::ReckoningNumeric,
        R: crate::reckoning_sin_cos::ReckoningSinCosNumeric,
    {
        self.ik_runtime
            .as_mut()
            .ok_or(crate::skeleton_ik::SkeletonIkSuffixError::Unconstructed)?
            .try_update_suffix(
                &self.ik, processed, rows, attributes, services, numeric, rounding,
            )
    }

    /// Actual Skeleton constructor call82b5743c precedes Island routing/Init.
    /// A failed geometry read leaves the same reset owner and completed fields.
    #[expect(
        clippy::type_complexity,
        reason = "distinct attribute and numerical service errors"
    )]
    pub(crate) fn try_construct_ik<D, N, R>(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        deck: &mut D,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<
        (),
        crate::skeleton_ik_geometry::SkeletonIkGeometryError<D::Error, N::Error, R::Error>,
    >
    where
        D: crate::skateboard_attributes::SkateboardAttributeRead,
        N: crate::reckoning_numeric::ReckoningNumeric,
        R: crate::reckoning_sin_cos::ReckoningSinCosNumeric,
    {
        if let Some(runtime) = &self.ik_runtime {
            return Err(
                crate::skeleton_ik_geometry::SkeletonIkGeometryError::AlreadyAttempted(
                    runtime.geometry().progress(),
                ),
            );
        }
        self.ik_runtime = Some(crate::skeleton_ik::SkeletonIk::new(vault, &mut self.ik));
        self.ik_runtime
            .as_mut()
            .expect("retained constructor prefix")
            .geometry_mut()
            .try_construct(deck, numeric, rounding)
    }
    pub(crate) fn record(&self) -> &SkeletonAnimationRecord {
        &self.record
    }
    pub(crate) fn reset_record_after_physics_reset(
        &mut self,
        proxy_target: &mut Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    ) {
        self.record.reset_after_physics_reset(proxy_target);
    }
    pub(crate) fn offset(&self) -> &SkeletonSkateboardOffset {
        &self.offset
    }
    /// Original setter 0x82b3b178. Caller-supplied transform and lazy cvar are
    /// explicit until the preceding gameplay adjustment producers are connected.
    pub(crate) fn try_set_skateboard_offset<C: SkeletonSkateboardOffsetCvars>(
        &mut self,
        transform: &AnimationMatrix,
        cvars: &mut C,
    ) -> Result<(), C::Error> {
        self.offset.try_set_full_transform(transform, cvars)
    }
    pub(crate) fn raw_skateboard(&self) -> &AnimationMatrix {
        &self.raw_skateboard
    }
    pub(crate) fn hips(&self) -> &AnimationMatrix {
        &self.hips
    }
    pub(crate) fn last_deck_zy(&self) -> f32 {
        self.last_deck_zy
    }
    pub(crate) fn last_filtered_state(&self) -> i32 {
        self.landing.last_filtered_state()
    }
    pub(crate) fn second_last_com_velocity(&self) -> f32 {
        self.landing.second_last_com_velocity()
    }
    pub(crate) fn landing_adjust(&self) -> &SkeletonLandingAdjust {
        &self.landing
    }
    pub(crate) fn root_input(&self, time_delta: f32) -> Option<SkeletonRootInput> {
        self.completed.then_some(SkeletonRootInput {
            time_delta,
            skate_animation: self.skateboard,
        })
    }
    /// Represented Skeleton `TeleportReset` stores: deck ZY at 0x82b54254,
    /// followed by target reset in its `IK::Reset` call at 0x82b5427c. Separate
    /// from `Body::TeleportReset` at 0x82b3bf88; this is not the full reset.
    pub(crate) fn reset_teleport_animation_projection(&mut self) {
        self.last_deck_zy = 0.0;
        if let Some(runtime) = &mut self.ik_runtime {
            runtime.reset(&mut self.ik);
        } else {
            // Standalone target-only component owners have no full constructor.
            self.ik.reset_target_transforms();
        }
    }

    /// Skeleton::TeleportReset 0x82b54254 precedes Body and IK resets.
    pub(crate) fn clear_last_deck_zy_for_teleport(&mut self) {
        self.last_deck_zy = 0.0;
    }

    /// Skeleton::TeleportReset 0x82b5427c calls the retained full IK runtime.
    pub(crate) fn try_reset_ik_for_teleport(&mut self) -> Result<(), SkeletonIkUnconstructed> {
        self.ik_runtime
            .as_mut()
            .ok_or(SkeletonIkUnconstructed)?
            .reset(&mut self.ik);
        Ok(())
    }

    /// Source prefix is committed on every error. Supported adjustment gates
    /// and active offsets reach record/COM and the final adjusted root copies.
    #[expect(
        clippy::too_many_arguments,
        reason = "distinct original owners and lazy services"
    )]
    #[cfg(test)]
    pub(crate) fn try_process<A, C, N>(
        &mut self,
        targets: &mut SkeletonAnimationTargets,
        pose: &[AnimationMatrix],
        input: &SkeletonAdjustmentInput,
        passed_deck_zy_delta: &mut f32,
        masses: &SkeletonPhysicsHistory,
        animation_to_board: &SkeletonRootMatrix,
        world_to_animation: &AnimationMatrix,
        retained_com_y: f32,
        attributes: &mut A,
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<(), SkeletonCurrentAnimationError<A::Error, C::Error>>
    where
        A: SkeletonCurrentAnimationAttributeRead,
        C: SkeletonCurrentAnimationCvars,
        N: SkeletonPhysicsHistoryNumeric,
    {
        let mut grind = continuation::ControlledGrind {
            activated: self.grind_activated,
            adjusting: self.grind_adjusting,
        };
        let result = self.try_process_read(
            targets,
            pose,
            input,
            passed_deck_zy_delta,
            masses,
            animation_to_board,
            &TEST_ANIMATION_TO_WORLD,
            world_to_animation,
            retained_com_y,
            attributes,
            cvars,
            numeric,
            &mut grind,
        );
        self.grind_adjusting = grind.adjusting;
        result
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "distinct retained owners and late source services"
    )]
    pub(crate) fn try_process_read<
        A,
        C,
        N,
        I: continuation::AnimationProcessedRead,
        G: continuation::GrindAdjustRead,
    >(
        &mut self,
        targets: &mut SkeletonAnimationTargets,
        pose: &[AnimationMatrix],
        input: &I,
        passed_deck_zy_delta: &mut impl continuation::AnimationDeckDeltaWrite,
        masses: &SkeletonPhysicsHistory,
        animation_to_board: &SkeletonRootMatrix,
        animation_to_world: &[[f32; 4]; 4],
        world_to_animation: &AnimationMatrix,
        retained_com_y: f32,
        attributes: &mut A,
        cvars: &mut C,
        numeric: &mut N,
        grind: &mut G,
    ) -> Result<(), SkeletonCurrentAnimationError<A::Error, C::Error>>
    where
        A: SkeletonCurrentAnimationAttributeRead,
        C: SkeletonCurrentAnimationCvars,
        N: SkeletonPhysicsHistoryNumeric,
    {
        use SkeletonCurrentAnimationError as E;
        self.completed = false;
        targets
            .try_map_current_pose(self.ik.data(), pose)
            .map_err(E::Targets)?;
        let volumes = targets
            .completed_volumes()
            .expect("completed original mapping");
        self.raw_skateboard = volumes[0];
        self.ik.try_calculate(pose).map_err(E::Ik)?;
        // Source UpdateOffboardAdjust82b4c808 runs before the other adjustment
        // stages. ProcessData82b5ece8 retains this same passed pose owner.
        if input.category().map_err(E::Input)? == PHYSICS_CATEGORY_OFFBOARD && input.state().map_err(E::Input)? != PHYSICS_STATE_LANDINGONDECK {
            try_update_offboard_adjust(
                &self.ik,
                pose,
                &mut self.offset,
                input.flag(0x8cc, 8).map_err(E::Input)?,
                &mut OffboardCvars(cvars),
            )
            .map_err(E::Offboard)?;
        }
        let mapped_skateboard = self.raw_skateboard;
        self.try_adjust_read(
            input,
            &mapped_skateboard,
            animation_to_world,
            world_to_animation,
            retained_com_y,
            masses,
            attributes,
            cvars,
            numeric,
            grind,
        )?;
        self.offset.update(
            targets
                .completed_skateboard_mut()
                .expect("completed original mapping"),
            &mut self.ik,
            &mut crate::skateboard_spawn_math::HostSkateboardSpawnMathNumeric,
        );
        let volumes = targets
            .completed_volumes()
            .expect("adjustments retain completed mapping");
        self.record
            .try_update(&volumes, masses, animation_to_board, cvars)
            .map_err(E::Record)?;
        self.hips = volumes[23];
        self.skateboard = volumes[0];
        passed_deck_zy_delta.write_delta(self.skateboard[9] - self.last_deck_zy);
        self.last_deck_zy = self.skateboard[9];
        self.completed = true;
        Ok(())
    }

    #[cfg(test)]
    #[expect(
        clippy::too_many_arguments,
        reason = "controlled numerical test inputs"
    )]
    fn try_adjust<A, C, N>(
        &mut self,
        input: &SkeletonAdjustmentInput,
        mapped_skateboard: &AnimationMatrix,
        world_to_animation: &AnimationMatrix,
        retained_com_y: f32,
        physics: &SkeletonPhysicsHistory,
        attributes: &mut A,
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<(), SkeletonCurrentAnimationError<A::Error, C::Error>>
    where
        A: SkeletonCurrentAnimationAttributeRead,
        C: SkeletonCurrentAnimationCvars,
        N: SkeletonPhysicsHistoryNumeric,
    {
        let mut grind = continuation::ControlledGrind {
            activated: self.grind_activated,
            adjusting: self.grind_adjusting,
        };
        let result = self.try_adjust_read(
            input,
            mapped_skateboard,
            &TEST_ANIMATION_TO_WORLD,
            world_to_animation,
            retained_com_y,
            physics,
            attributes,
            cvars,
            numeric,
            &mut grind,
        );
        self.grind_adjusting = grind.adjusting;
        result
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "distinct retained owners and lazy source collaborators"
    )]
    fn try_adjust_read<
        A,
        C,
        N,
        I: continuation::AnimationProcessedRead,
        G: continuation::GrindAdjustRead,
    >(
        &mut self,
        input: &I,
        mapped_skateboard: &AnimationMatrix,
        animation_to_world: &[[f32; 4]; 4],
        world_to_animation: &AnimationMatrix,
        retained_com_y: f32,
        physics: &SkeletonPhysicsHistory,
        attributes: &mut A,
        cvars: &mut C,
        numeric: &mut N,
        grind: &mut G,
    ) -> Result<(), SkeletonCurrentAnimationError<A::Error, C::Error>>
    where
        A: SkeletonCurrentAnimationAttributeRead,
        C: SkeletonCurrentAnimationCvars,
        N: SkeletonPhysicsHistoryNumeric,
    {
        use SkeletonCurrentAnimationError as E;
        if input.flag(0x8cc, 0x200).map_err(E::Input)? {
            if !input.flag(0x8cc, 0x4_0000).map_err(E::Input)? {
                crate::skeleton_landing_on_board::try_update_landing_on_board_read(
                    &mut self.offset,
                    mapped_skateboard,
                    world_to_animation,
                    retained_com_y,
                    input,
                    &mut LandingAttributes(attributes),
                    &mut LandingCvars(cvars),
                    &mut crate::reckoning_numeric::HostReckoningNumeric,
                    &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
                )
                .map_err(E::LandingOnBoard)?;
            }
        } else {
            // `Skeleton::UpdateGrindAirAdjust` 0x82b4cd80.
            let out = grind.update(input).map_err(E::Input)?;
            if out.adjustment_required {
                let transform =
                    crate::physical_player_grind_air_adjust::update::grind_air_offset_transform(
                        &out,
                        animation_to_world,
                        world_to_animation,
                        mapped_skateboard,
                        &mut crate::physical_player_grind_manager::math::GrindNum {
                            n: &mut crate::reckoning_numeric::HostReckoningNumeric,
                            s: &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
                        },
                    )
                    .map_err(|e| {
                        E::Input(continuation::AnimationInputError::GrindAirAdjust(
                            e.to_string(),
                        ))
                    })?;
                // `kLandingAdjustFramesToBlendAwayOffset` 0x8308e9d4 is sampled
                // after the full store and again after the unchanged W re-store.
                self.offset.transform = transform;
                self.offset.full_just_set = true;
                self.offset.full_count = cvars
                    .landing_frames_to_blend_away_offset()
                    .map_err(E::Cvar)?;
                self.offset.vertical_just_set = true;
                self.offset.vertical_count = cvars
                    .landing_frames_to_blend_away_offset()
                    .map_err(E::Cvar)?;
            }
        }
        self.landing
            .try_update_read(
                input,
                physics.record(),
                &self.record,
                mapped_skateboard,
                &mut self.offset,
                attributes,
                &mut SpringCvars(cvars),
                numeric,
            )
            .map_err(|error| match error {
                SkeletonLandingAdjustError::Attribute(e) => E::Attribute(e),
                SkeletonLandingAdjustError::Cvar(e) => E::Cvar(e),
                other => E::LandingAdjust(other),
            })
    }
}

#[cfg(test)]
#[path = "skeleton_current_animation_tests.rs"]
mod tests;

impl SkeletonCurrentAnimation {
    pub(crate) fn skateboard(&self) -> &crate::animation::AnimationMatrix {
        &self.skateboard
    }
    pub(crate) fn try_update_ik_data_only(
        &mut self,
        data: crate::skeleton_ik::SkeletonIkDataIn,
    ) -> Result<(), crate::skeleton_ik::SkeletonIkUpdateError> {
        self.ik_runtime
            .as_mut()
            .ok_or(crate::skeleton_ik::SkeletonIkUpdateError::Unconstructed)?
            .try_store_data_only(data)
    }
    pub(crate) fn try_begin_wipeout_ik_adjust(
        &mut self,
        read_status:impl FnOnce()->Result<u32,crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError>,
        read_blend:impl FnMut()->Result<bool,crate::dynamic_assembly_session::skeleton_normal::general_owner::live_input::GroundedInputError>,
    ) -> Result<(), crate::skeleton_ik::SkeletonIkUpdateError> {
        self.ik_runtime
            .as_mut()
            .ok_or(crate::skeleton_ik::SkeletonIkUpdateError::Unconstructed)?
            .try_begin_wipeout_adjust(&mut self.ik, read_status, read_blend)
    }
    pub(crate) fn finish_wipeout_ik_adjust(
        &mut self,
    ) -> Result<(), crate::skeleton_ik::SkeletonIkUpdateError> {
        self.ik_runtime
            .as_mut()
            .ok_or(crate::skeleton_ik::SkeletonIkUpdateError::Unconstructed)?
            .finish_wipeout_adjust();
        Ok(())
    }
}
