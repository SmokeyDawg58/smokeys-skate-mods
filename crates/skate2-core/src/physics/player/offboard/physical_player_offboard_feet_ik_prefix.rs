//! First two calls of `OffBoardFeetIK::UpdateGround` 0x82dd1e00.
//! The surfaces are already in ProcessedPhysIn; this stage starts no line test.
use crate::physics_state_ids::PHYSICS_STATE_OFFBOARD_AIR;
use super::{OffboardFeetIk, SurfaceResult};
use crate::{
    dynamic_assembly_session::skeleton_normal::{
        SkeletonNormalAssembly, part_transform_query::SkeletonPartTransformQueryError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::hilod_tail::SkeletonLineResult,
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric, host_dot3, try_length,
    },
    simulation_drive_jacobian::{refine_drive_reciprocal, refine_drive_reciprocal_sqrt},
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OffboardFeetIkPrefixError {
    ProcessedPoisoned,
    MissingSurface { left: bool },
    MissingAnimatedVolume { part: usize },
    MissingEffectiveAnimation,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ClampFootNormalError<C, N> {
    Cvar(C),
    Numeric(N),
}

/// `ClampFootNormal` 0x82dcdae8. The source first stores the input normal,
/// then samples the linked maximum. Only a normal below that Y ceiling takes
/// the horizontal-normalize path; zero horizontal length returns the original.
/// The two final cvar reads are distinct and may observe changed values.
fn try_clamp_foot_normal<C, N: ReckoningNumeric>(
    normal: SkeletonRootVector,
    mut maximum: impl FnMut() -> Result<f32, C>,
    numeric: &mut N,
) -> Result<SkeletonRootVector, ClampFootNormalError<C, N::Error>> {
    let first = maximum().map_err(ClampFootNormalError::Cvar)?;
    if !(first > normal.xyz[1]) {
        return Ok(normal);
    }
    let mut horizontal = normal;
    horizontal.xyz[1] = 0.0;
    let horizontal_length =
        try_length(horizontal, numeric).map_err(ClampFootNormalError::Numeric)?;
    if !(horizontal_length > 0.0) {
        return Ok(normal);
    }

    // Source vrefp at 0x82dcdbf0 precedes the second scalar cvar value read.
    let inverse_seed = numeric
        .reciprocal_estimate(horizontal_length)
        .map_err(ClampFootNormalError::Numeric)?;
    let second = maximum().map_err(ClampFootNormalError::Cvar)?;
    let one_minus_maximum_squared = (-second).mul_add(second, 1.0); // 82dcdc10
    let inverse_sqrt_seed = numeric
        .reciprocal_sqrt_estimate(one_minus_maximum_squared)
        .map_err(ClampFootNormalError::Numeric)?;
    let inverse_length = refine_drive_reciprocal(horizontal_length, inverse_seed);
    let horizontal_scale = one_minus_maximum_squared
        * refine_drive_reciprocal_sqrt(one_minus_maximum_squared, inverse_sqrt_seed);
    let mut clamped = horizontal.map(|lane| lane * (inverse_length * horizontal_scale));
    clamped.xyz[1] = maximum().map_err(ClampFootNormalError::Cvar)?; // 82dcdc7c
    Ok(clamped)
}

#[derive(Debug, PartialEq)]
pub(crate) enum FeetDownPrefixError {
    Cvar(InputCvarError),
    Numeric(ReckoningHostNumericError),
    MissingAnimatedVolume { part: usize },
}

trait FeetDownSource {
    type Error;
    fn clamp_normal(
        &mut self,
        left: bool,
        normal: SkeletonRootVector,
    ) -> Result<SkeletonRootVector, Self::Error>;
    fn raw_local_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error>;
    fn foot_down_height(&mut self) -> Result<f32, Self::Error>;
}

/// The four local animation translations are stack temporaries in native code.
/// Return them so a later continuation can use the same reads without making
/// a second query. The first `GetPartTransform` follows this prefix.
fn calculate_feet_down_prefix<S: FeetDownSource>(
    normal_left: &mut SkeletonRootVector,
    normal_right: &mut SkeletonRootVector,
    anim_left_down: &mut bool,
    anim_right_down: &mut bool,
    result_left: &SurfaceResult,
    result_right: &SurfaceResult,
    source: &mut S,
) -> Result<[SkeletonRootVector; 4], S::Error> {
    *normal_left = source.clamp_normal(true, result_left.normal)?; // 82dcdcc4..e0
    *normal_right = source.clamp_normal(false, result_right.normal)?; // 82dcdce4..fc
    let left_toe = source.raw_local_position(15)?; // 82dcdd04..1c
    let right_toe = source.raw_local_position(19)?; // 82dcdd20..34
    let left_foot = source.raw_local_position(16)?; // 82dcdd38..4c
    let right_foot = source.raw_local_position(20)?; // 82dcdd50..60
    let left_height = source.foot_down_height()?; // 82dcdd74
    *anim_left_down = left_height > left_toe.xyz[1] && left_height > left_foot.xyz[1]; // 82dcdd90..e8
    let right_height = source.foot_down_height()?; // 82dcddfc
    *anim_right_down = right_height > right_toe.xyz[1] && right_height > right_foot.xyz[1]; // 82dcde18..6c
    Ok([left_toe, right_toe, left_foot, right_foot])
}

struct RetainedFeetDownSource<'a> {
    skeleton: &'a SkeletonNormalAssembly,
    cvars: &'a InputCvarReferences,
    numeric: HostReckoningNumeric,
}

/// Native retains these eight translation vectors in registers across its
/// following surface-normal projection; the four local rows are not replaced
/// by the later attribute-selected part transforms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FeetDownPositionScratch {
    pub(crate) raw_local: [SkeletonRootVector; 4],
    pub(crate) part_world: [SkeletonRootVector; 4],
}

trait PartTransformSource {
    type Error;
    fn part_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error>;
}

fn query_feet_down_part_positions<S: PartTransformSource>(
    raw_local: [SkeletonRootVector; 4],
    source: &mut S,
) -> Result<FeetDownPositionScratch, S::Error> {
    let left_toe = source.part_position(15)?; // 82dcde7c..90
    let right_toe = source.part_position(19)?; // 82dcde94..a8
    let left_foot = source.part_position(16)?; // 82dcdeac..c0
    let right_foot = source.part_position(20)?; // 82dcdec4..cc
    Ok(FeetDownPositionScratch {
        raw_local,
        part_world: [left_toe, right_toe, left_foot, right_foot],
    })
}

struct RetainedPartTransformSource<'a>(&'a SkeletonNormalAssembly);

impl PartTransformSource for RetainedPartTransformSource<'_> {
    type Error = (usize, SkeletonPartTransformQueryError);

    fn part_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        self.0
            .try_query_part_transform(part)
            .map(|matrix| matrix.columns()[3])
            .map_err(|error| (part, error))
    }
}

/// `CalculateFeetDownState` 0x82dcde7c..cc invokes the existing Skeleton
/// query afresh for each part, including its per-call animation attribute.
pub(crate) fn try_query_feet_down_part_positions(
    skeleton: &SkeletonNormalAssembly,
    raw_local: [SkeletonRootVector; 4],
) -> Result<FeetDownPositionScratch, (usize, SkeletonPartTransformQueryError)> {
    query_feet_down_part_positions(raw_local, &mut RetainedPartTransformSource(skeleton))
}

/// `CalculateFeetDownState` 0x82dcded4..df84. Each valid surface reads the
/// linked height after subtracting the queried foot position and before the
/// toe subtraction. An invalid surface copies its animation foot-down flag
/// without reading that height.
fn resolve_surface_foot_down<E>(
    result: SurfaceResult,
    normal: SkeletonRootVector,
    toe: SkeletonRootVector,
    foot: SkeletonRootVector,
    animation_down: bool,
    mut height: impl FnMut() -> Result<f32, E>,
) -> Result<bool, E> {
    if !result.valid {
        return Ok(animation_down);
    }
    let foot_delta = foot.zip(result.position, |position, surface| position - surface);
    let maximum_height = height()?;
    let toe_delta = toe.zip(result.position, |position, surface| position - surface);
    let normal_xyz = [normal.xyz[0], normal.xyz[1], normal.xyz[2], 0.0];
    let foot_distance = host_dot3(
        [foot_delta.xyz[0], foot_delta.xyz[1], foot_delta.xyz[2], 0.0],
        normal_xyz,
    );
    let toe_distance = host_dot3(
        [toe_delta.xyz[0], toe_delta.xyz[1], toe_delta.xyz[2], 0.0],
        normal_xyz,
    );
    let highest = if toe_distance > foot_distance {
        toe_distance
    } else {
        foot_distance
    };
    Ok(highest < maximum_height)
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ProcessedFootFlagsError {
    ProcessedPoisoned,
    MissingWord { offset: usize },
}

trait ProcessedFootFlagsSource {
    type Error;
    fn flags_8d0(&mut self) -> Result<u32, Self::Error>;
    fn flags_8cc(&mut self) -> Result<u32, Self::Error>;
}

/// `CalculateFeetDownState` 0x82dcdf88..e004. The +8d0 word is loaded once;
/// +8cc bit3 is reloaded on each reached branch. If both requested fix bits
/// are set, the native first-taken branch determines which foot is forced.
fn override_processed_foot_flags<S: ProcessedFootFlagsSource>(
    left_down: &mut bool,
    right_down: &mut bool,
    source: &mut S,
) -> Result<(), S::Error> {
    let word = source.flags_8d0()?;
    let bit9 = word & 0x200 != 0; // extrwi pos22 at 82dcdf8c
    if bit9 && source.flags_8cc()? & 8 == 0 {
        *left_down = true;
        *right_down = false;
        return Ok(());
    }
    let bit8 = word & 0x100 != 0; // extrwi pos23 at 82dcdfa8
    if bit8 && source.flags_8cc()? & 8 != 0 {
        *left_down = true;
        *right_down = false;
        return Ok(());
    }
    if bit8 && source.flags_8cc()? & 8 == 0 {
        *left_down = false;
        *right_down = true;
        return Ok(());
    }
    if bit9 && source.flags_8cc()? & 8 != 0 {
        *left_down = false;
        *right_down = true;
    }
    Ok(())
}

struct RetainedProcessedFootFlagsSource<'a>(
    &'a std::sync::Arc<std::sync::Mutex<SkeletonProcessedContact>>,
);

impl ProcessedFootFlagsSource for RetainedProcessedFootFlagsSource<'_> {
    type Error = ProcessedFootFlagsError;

    fn flags_8d0(&mut self) -> Result<u32, Self::Error> {
        self.0
            .lock()
            .map_err(|_| Self::Error::ProcessedPoisoned)?
            .physics_state_fields()
            .flags_8d0
            .ok_or(Self::Error::MissingWord { offset: 0x8d0 })
    }

    fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
        self.0
            .lock()
            .map_err(|_| Self::Error::ProcessedPoisoned)?
            .physics_state_fields()
            .flags_8cc
            .ok_or(Self::Error::MissingWord { offset: 0x8cc })
    }
}

#[derive(Debug)]
pub(crate) enum FeetIkUpdateStateError {
    ProcessedPoisoned,
    MissingProcessedWord {
        offset: usize,
    },
    PartTransform {
        part: usize,
        error: SkeletonPartTransformQueryError,
    },
}

trait FeetIkUpdateStateSource {
    type Error;
    fn flags_8cc(&mut self) -> Result<u32, Self::Error>;
    fn current_physics_state(&mut self) -> Result<i32, Self::Error>;
    fn part_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error>;
}

/// `OffBoardFeetIK::UpdateState` 0x82dc89a0. The fixed source timestep is
/// read before Processed flags; a changed state is stored before its fresh
/// part query, while an unchanged state keeps its prior foot position.
fn update_feet_ik_state<S: FeetIkUpdateStateSource>(
    state: &mut i32,
    time_in_state: &mut f32,
    foot_position: &mut SkeletonRootVector,
    foot_locking_active: bool,
    left_down: bool,
    right_down: bool,
    source: &mut S,
) -> Result<(), S::Error> {
    *time_in_state += f32::from_bits(0x3c88_8889); // kUpdateTime_68, 82dc89c8
    let flags = source.flags_8cc()?;
    let next = if flags & 0x40 != 0 {
        3 // locomotion
    } else if source.current_physics_state()? == PHYSICS_STATE_OFFBOARD_AIR {
        4 // offboard air
    } else if foot_locking_active && left_down && !right_down {
        0 // left fixed
    } else if foot_locking_active && right_down && !left_down {
        1 // right fixed
    } else {
        2 // no fixed foot
    };
    if next != *state {
        *state = next;
        *time_in_state = 0.0;
        *foot_position = match next {
            0 => source.part_position(15)?,
            1 => source.part_position(19)?,
            _ => SkeletonRootVector::known([0.0; 4]),
        };
    }
    Ok(())
}

trait NewFootYSource {
    type Error;
    fn minimum_height(&mut self) -> Result<f32, Self::Error>;
    fn maximum_lift(&mut self) -> Result<f32, Self::Error>;
    fn maximum_up_distance(&mut self) -> Result<f32, Self::Error>;
}

trait FootTargetSource: NewFootYSource {
    fn blend_out_time(&mut self) -> Result<f32, Self::Error>;
}

#[derive(Debug, PartialEq)]
pub(crate) enum CalculateIkTargetsError {
    ProcessedPoisoned,
    MissingLocomotionField { foot: usize, target: bool },
    Overlay(crate::debug_overlay::DebugOverlayError),
    Cvar(InputCvarError),
    PendingState(i32),
    Numeric(ReckoningHostNumericError),
    Clamp(crate::skeleton_ik_line_frame::SkeletonIkClampError<ReckoningHostNumericError>),
}

/// CalculateIKTargets82dd0a34..ae4 / 82dd0bf4..c90. Both gates use XZ
/// distance, but only the LEFT clamp flattens its inputs. The right clamp
/// uses full XYZ before restoring the planted Y. Preserve this native asymmetry.
fn calculate_fixed_foot_target(
    left: bool,
    animation: SkeletonRootVector,
    fixed: &mut SkeletonRootVector,
    target: &mut SkeletonRootVector,
    cvars: &InputCvarReferences,
) -> Result<(), CalculateIkTargetsError> {
    use CalculateIkTargetsError as E;
    let mut delta = fixed.zip(animation, |a, b| a - b);
    delta.xyz[1] = 0.0;
    let range = cvars.try_ob_ik_max_dist().map_err(E::Cvar)?;
    let range_squared = range * range;
    let mut numeric = HostReckoningNumeric;
    let squared = numeric.dot_vector(delta, delta).map_err(E::Numeric)?;
    // Native bge tests !LT, including unordered, at 82dd0a74/0c34.
    if squared.partial_cmp(&range_squared) == Some(std::cmp::Ordering::Less) {
        *target = *fixed;
        return Ok(());
    }
    let saved_y = fixed.xyz[1];
    let mut a = *fixed;
    let mut b = animation;
    if left {
        a.xyz[1] = 0.0;
        b.xyz[1] = 0.0;
    }
    let range = cvars.try_ob_ik_max_dist().map_err(E::Cvar)?;
    let mut result = *fixed;
    crate::skeleton_ik_line_frame::try_clamp_vector_within_range(
        a, b, range, &mut result, &mut numeric,
    )
    .map_err(E::Clamp)?;
    result.xyz[1] = saved_y;
    *target = result;
    *fixed = result;
    Ok(())
}

/// `OffBoardFeetIK::CalculateNewYPos` 0x82dc8b38. This reproduces the
/// scalar instruction order and three linked target reads. The three `fsel`
/// operations use their ordinary-finite selection graph; Xenon exceptional
/// FPSCR/NaN behavior remains an explicit numeric boundary.
fn calculate_new_foot_y<S: NewFootYSource>(
    ground_y: f32,
    animation_y: f32,
    animation_space_y: f32,
    source: &mut S,
) -> Result<f32, S::Error> {
    let animation_relative = animation_y - animation_space_y; // 82dc8b44
    let animation_world = animation_relative + ground_y; // 82dc8b60
    let minimum = source.minimum_height()?; // 82dc8b68
    let floor = minimum + ground_y; // 82dc8b6c
    let maximum_lift = source.maximum_lift()?; // 82dc8b70
    let maximum_up_distance = source.maximum_up_distance()?; // 82dc8b74
    let ceiling = maximum_lift + floor; // 82dc8b78
    let capped = if ceiling - animation_world >= 0.0 {
        animation_world
    } else {
        ceiling
    }; // 82dc8b80
    let bounded = if capped - floor >= 0.0 { capped } else { floor }; // 82dc8b88
    let result = if animation_y - bounded >= 0.0 {
        animation_y
    } else {
        bounded
    }; // 82dc8b90
    Ok(if result - animation_y > maximum_up_distance {
        animation_y
    } else {
        result
    }) // 82dc8b9c..a0
}

struct RetainedNewFootYSource<'a>(&'a InputCvarReferences);

impl NewFootYSource for RetainedNewFootYSource<'_> {
    type Error = InputCvarError;

    fn minimum_height(&mut self) -> Result<f32, Self::Error> {
        self.0.try_y_min_height_to_be_off_ground()
    }

    fn maximum_lift(&mut self) -> Result<f32, Self::Error> {
        self.0.try_ob_ik_max_y_lift_up()
    }

    fn maximum_up_distance(&mut self) -> Result<f32, Self::Error> {
        self.0.try_ob_ik_max_y_dist_up()
    }
}

impl FootTargetSource for RetainedNewFootYSource<'_> {
    fn blend_out_time(&mut self) -> Result<f32, Self::Error> {
        self.0.try_ob_ik_time_to_blend_out_fixed_foot_pos()
    }
}

/// State 2's left and right halves of `CalculateIKTargets` use the same
/// source sequence: optional surface Y correction, optional timer decay and
/// linked blend read, then one complete VMX target store. The caller runs
/// this once for left and then once for right.
fn calculate_state2_foot_target<S: FootTargetSource>(
    current_animation: SkeletonRootVector,
    result: SurfaceResult,
    slope_adjust_y: f32,
    animation_space_y: f32,
    target: &mut SkeletonRootVector,
    blend_out_timer: &mut f32,
    source: &mut S,
) -> Result<(), S::Error> {
    let mut animation = current_animation;
    if result.valid {
        // Native fsel(-slope, 0, slope), ordinary-finite branch.
        let positive_slope = if -slope_adjust_y >= 0.0 {
            0.0
        } else {
            slope_adjust_y
        };
        animation.xyz[1] = calculate_new_foot_y(
            result.position.xyz[1],
            animation.xyz[1] - positive_slope,
            animation_space_y,
            source,
        )?;
    }
    let mut weight = 1.0;
    if *blend_out_timer > 0.0 {
        let elapsed = *blend_out_timer - f32::from_bits(0x3c88_8889);
        *blend_out_timer = elapsed; // native first store, before cvar read
        *blend_out_timer = if -elapsed >= 0.0 { 0.0 } else { elapsed };
        weight = 1.0 - (*blend_out_timer / source.blend_out_time()?);
    }
    let previous_weight = 1.0 - weight;
    *target = target.zip(animation, |previous, animated| {
        animated.mul_add(weight, previous * previous_weight)
    });
    Ok(())
}

struct RetainedFeetIkUpdateStateSource<'a> {
    processed: &'a std::sync::Arc<std::sync::Mutex<SkeletonProcessedContact>>,
    skeleton: &'a SkeletonNormalAssembly,
}

impl FeetIkUpdateStateSource for RetainedFeetIkUpdateStateSource<'_> {
    type Error = FeetIkUpdateStateError;

    fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::ProcessedPoisoned)?
            .physics_state_fields()
            .flags_8cc
            .ok_or(Self::Error::MissingProcessedWord { offset: 0x8cc })
    }

    fn current_physics_state(&mut self) -> Result<i32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::ProcessedPoisoned)?
            .physics_state_fields()
            .current
            .ok_or(Self::Error::MissingProcessedWord { offset: 0x8e8 })
    }

    fn part_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        self.skeleton
            .try_query_part_transform(part)
            .map(|transform| transform.columns()[3])
            .map_err(|error| Self::Error::PartTransform { part, error })
    }
}

impl FeetDownSource for RetainedFeetDownSource<'_> {
    type Error = FeetDownPrefixError;

    fn clamp_normal(
        &mut self,
        _left: bool,
        normal: SkeletonRootVector,
    ) -> Result<SkeletonRootVector, Self::Error> {
        try_clamp_foot_normal(
            normal,
            || self.cvars.try_ob_ik_max_y_foot_normal(),
            &mut self.numeric,
        )
        .map_err(|error| match error {
            ClampFootNormalError::Cvar(error) => Self::Error::Cvar(error),
            ClampFootNormalError::Numeric(error) => Self::Error::Numeric(error),
        })
    }

    fn raw_local_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        let volume = self
            .skeleton
            .current_animation()
            .record()
            .poses()
            .get(part)
            .ok_or(Self::Error::MissingAnimatedVolume { part })?;
        Ok(SkeletonRootVector::known([
            volume[12], volume[13], volume[14], volume[15],
        ]))
    }

    fn foot_down_height(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_y_height_to_be_off_ground()
            .map_err(Self::Error::Cvar)
    }
}

/// `Update` 0x82dd1b84..1ba8. Processed+0xf0 is the fourth column of
/// `mEffectiveAnimTransform` at +0xc0, not `mGroundPos` at +0x1e0.
fn copy_last_good_animation_position(
    bad_ground_correction_counter: i32,
    effective_animation: Option<SkeletonRootMatrix>,
    last_good_position: &mut SkeletonRootVector,
    last_good_position_valid: &mut bool,
) -> Result<(), OffboardFeetIkPrefixError> {
    if bad_ground_correction_counter == 0 {
        *last_good_position = effective_animation
            .ok_or(OffboardFeetIkPrefixError::MissingEffectiveAnimation)?
            .columns()[3];
        *last_good_position_valid = true;
    }
    Ok(())
}

trait PrefixSource {
    type Error;
    fn surface(&mut self, left: bool) -> Result<SurfaceResult, Self::Error>;
    fn raw_toe_world_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error>;
}

/// Preserve the four native stores and two Skeleton calls in source order.
fn load_prefix<S: PrefixSource>(
    result_left: &mut SurfaceResult,
    result_right: &mut SurfaceResult,
    current_left: &mut SkeletonRootVector,
    current_right: &mut SkeletonRootVector,
    source: &mut S,
) -> Result<(), S::Error> {
    *result_left = source.surface(true)?; // 82dcd294..2b0
    *result_right = source.surface(false)?; // 82dcd2bc..2d8
    *current_left = source.raw_toe_world_position(15)?; // 82dcd2e0..2f8
    *current_right = source.raw_toe_world_position(19)?; // 82dcd300..30c
    Ok(())
}

struct RetainedSource<'a> {
    processed: &'a SkeletonProcessedContact,
    skeleton: &'a SkeletonNormalAssembly,
}

impl RetainedSource<'_> {
    fn copy_surface(source: SkeletonLineResult) -> SurfaceResult {
        SurfaceResult {
            position: source.position,
            normal: source.normal,
            surface_id: source.surface_id,
            valid: source.valid,
        }
    }
}

impl PrefixSource for RetainedSource<'_> {
    type Error = OffboardFeetIkPrefixError;

    fn surface(&mut self, left: bool) -> Result<SurfaceResult, Self::Error> {
        let surfaces = self.processed.line_surfaces();
        let source = if left { surfaces.left } else { surfaces.right }
            .ok_or(Self::Error::MissingSurface { left })?;
        Ok(Self::copy_surface(source))
    }

    fn raw_toe_world_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        // GetAnimPartTransformWorldRaw82b45f58 composes the last animation
        // volume with raw animation-to-world. GetPartTransform can select
        // physics history and would change this source read.
        let volume = self
            .skeleton
            .current_animation()
            .record()
            .poses()
            .get(part)
            .ok_or(Self::Error::MissingAnimatedVolume { part })?;
        Ok(self
            .skeleton
            .root_transforms()
            .animation_to_world()
            .compose(&SkeletonRootMatrix::known(*volume))
            .columns()[3])
    }
}

/// `ConditionLineTestResultsForGround` 0x82dcd188. Original validity is
/// captured before either result can be invalidated. A tie passes both tests.
fn condition_ground_results<E>(
    left: &mut SurfaceResult,
    right: &mut SurfaceResult,
    current_left_y: f32,
    current_right_y: f32,
    mut read_maximum: impl FnMut() -> Result<f32, E>,
) -> Result<(), E> {
    let original_left_valid = left.valid;
    let original_right_valid = right.valid;
    let left_delta = if original_left_valid {
        left.position.xyz[1] - current_left_y
    } else {
        0.0
    };
    let right_delta = if original_right_valid {
        right.position.xyz[1] - current_right_y
    } else {
        0.0
    };
    let maximum = read_maximum()?; // target->value at 82dcd214
    if (left_delta - right_delta).abs() > maximum {
        if left_delta.abs() > maximum && original_right_valid {
            left.valid = false;
        }
        let maximum = read_maximum()?; // target->value reloaded at 82dcd244
        if right_delta.abs() > maximum && original_left_valid {
            right.valid = false;
        }
    }
    Ok(())
}

impl OffboardFeetIk {
    /// Both UpdateGround and UpdateLocomotion call the same Update ->
    /// CalculateIKTargets82dd0928. Select from the retained IK state, which
    /// may still be locomotion on the first ground tick after stopping.
    pub(crate) fn try_calculate_targets(
        &mut self,
        cvars: &InputCvarReferences,
        overlay: &crate::debug_overlay::DebugOverlaySlot,
    ) -> Result<(), CalculateIkTargetsError> {
        if self.state == 3 {
            self.try_calculate_locomotion_targets(overlay)
        } else {
            self.try_calculate_non_locomotion_targets(cvars)
        }
    }

    /// CalculateIKTargets82dd0928: left first, then right. In states0/1 only
    /// the planted side clamps; the other side keeps the existing free-foot flow.
    pub(crate) fn try_calculate_non_locomotion_targets(
        &mut self,
        cvars: &InputCvarReferences,
    ) -> Result<(), CalculateIkTargetsError> {
        if !matches!(self.state, 0 | 1 | 2 | 4) {
            return Err(CalculateIkTargetsError::PendingState(self.state));
        }
        let mut source = RetainedNewFootYSource(cvars);
        if self.state == 0 {
            calculate_fixed_foot_target(
                true,
                self.current_animation_position_left,
                &mut self.foot_position,
                &mut self.left_foot_position,
                cvars,
            )?;
        } else {
            calculate_state2_foot_target(
                self.current_animation_position_left,
                self.result_left,
                self.left_slope_adjust_y,
                self.animation_space_y,
                &mut self.left_foot_position,
                &mut self.left_fixed_blend_out_timer,
                &mut source,
            )
            .map_err(CalculateIkTargetsError::Cvar)?;
        }
        if self.state == 1 {
            calculate_fixed_foot_target(
                false,
                self.current_animation_position_right,
                &mut self.foot_position,
                &mut self.right_foot_position,
                cvars,
            )
        } else {
            calculate_state2_foot_target(
                self.current_animation_position_right,
                self.result_right,
                self.right_slope_adjust_y,
                self.animation_space_y,
                &mut self.right_foot_position,
                &mut self.right_fixed_blend_out_timer,
                &mut source,
            )
            .map_err(CalculateIkTargetsError::Cvar)
        }
    }

    /// CalculateIKTargets82dd0968..9f8: locomotion keeps the old target when
    /// influence is not ordered positive; the source adds explicit +0 lanes.
    pub(crate) fn try_calculate_locomotion_targets(
        &mut self,
        overlay: &crate::debug_overlay::DebugOverlaySlot,
    ) -> Result<(), CalculateIkTargetsError> {
        use CalculateIkTargetsError as E;
        if self.state != 3 {
            return Err(E::PendingState(self.state));
        }
        overlay.require().map_err(E::Overlay)?;
        for foot in 0..2 {
            let p = self
                .binding
                .processed
                .lock()
                .map_err(|_| E::ProcessedPoisoned)?;
            let influence = p.prior_output().offboard_foot_influence(foot).ok_or(
                E::MissingLocomotionField {
                    foot,
                    target: false,
                },
            )?;
            if influence > 0.0 {
                let value = p
                    .prior_output()
                    .offboard_foot_target(foot)
                    .ok_or(E::MissingLocomotionField { foot, target: true })?;
                let value = value.map(|lane| lane + 0.0);
                if foot == 0 {
                    self.left_foot_position = value;
                } else {
                    self.right_foot_position = value;
                }
            }
        }
        Ok(())
    }

    /// Isolated scalar leaf called by `CalculateIKTargets` for a valid ground
    /// result. The enclosing left/right target and blend decisions stay typed.
    pub(crate) fn try_calculate_new_foot_y(
        &self,
        ground_y: f32,
        animation_y: f32,
        cvars: &InputCvarReferences,
    ) -> Result<f32, InputCvarError> {
        calculate_new_foot_y(
            ground_y,
            animation_y,
            self.animation_space_y,
            &mut RetainedNewFootYSource(cvars),
        )
    }

    /// First reached operation of `Update` 0x82dd1b70. Its next call is
    /// `CalculateFeetDownState` at 0x82dd1bb0.
    pub(crate) fn try_begin_ground_update(&mut self) -> Result<(), OffboardFeetIkPrefixError> {
        if self.bad_ground_correction_counter != 0 {
            return Ok(());
        }
        let processed = self
            .binding
            .processed
            .lock()
            .map_err(|_| OffboardFeetIkPrefixError::ProcessedPoisoned)?;
        copy_last_good_animation_position(
            self.bad_ground_correction_counter,
            processed.ground_position_fields().effective_animation,
            &mut self.last_good_position,
            &mut self.last_good_position_valid,
        )
    }

    /// `CalculateFeetDownState` 0x82dcdc90 through the animation foot-down
    /// flags at 0x82dcde6c. The returned local positions remain stack scratch
    /// for the subsequent physical/animated part-transform queries.
    pub(crate) fn try_begin_calculate_feet_down_state(
        &mut self,
        skeleton: &SkeletonNormalAssembly,
        cvars: &InputCvarReferences,
    ) -> Result<[SkeletonRootVector; 4], FeetDownPrefixError> {
        let mut source = RetainedFeetDownSource {
            skeleton,
            cvars,
            numeric: HostReckoningNumeric,
        };
        calculate_feet_down_prefix(
            &mut self.ground_normal_left,
            &mut self.ground_normal_right,
            &mut self.animation_left_foot_down,
            &mut self.animation_right_foot_down,
            &self.result_left,
            &self.result_right,
            &mut source,
        )
    }

    /// Surface-validity branches of `CalculateFeetDownState` through
    /// `mRightFootDown` at 0x82dcdf84. Processed foot-fix bits follow next.
    pub(crate) fn try_resolve_surface_feet_down(
        &mut self,
        positions: FeetDownPositionScratch,
        cvars: &InputCvarReferences,
    ) -> Result<(), InputCvarError> {
        self.left_foot_down = resolve_surface_foot_down(
            self.result_left,
            self.ground_normal_left,
            positions.part_world[0],
            positions.part_world[2],
            self.animation_left_foot_down,
            || cvars.try_y_height_to_be_off_ground(),
        )?;
        self.right_foot_down = resolve_surface_foot_down(
            self.result_right,
            self.ground_normal_right,
            positions.part_world[1],
            positions.part_world[3],
            self.animation_right_foot_down,
            || cvars.try_y_height_to_be_off_ground(),
        )?;
        Ok(())
    }

    /// Final `CalculateFeetDownState` Processed flag override. This leaves
    /// `UpdateState` at 0x82dd1bb8 as the next call in FeetIK `Update`.
    pub(crate) fn try_override_processed_foot_flags(
        &mut self,
    ) -> Result<(), ProcessedFootFlagsError> {
        let processed = std::sync::Arc::clone(&self.binding.processed);
        override_processed_foot_flags(
            &mut self.left_foot_down,
            &mut self.right_foot_down,
            &mut RetainedProcessedFootFlagsSource(&processed),
        )
    }

    /// `OffBoardFeetIK::UpdateState` 0x82dc89a0. `CalculateIKTargets` at
    /// 0x82dd1bc0 is the next call in the enclosing `Update`.
    pub(crate) fn try_update_ground_state(
        &mut self,
        skeleton: &SkeletonNormalAssembly,
    ) -> Result<(), FeetIkUpdateStateError> {
        let processed = std::sync::Arc::clone(&self.binding.processed);
        update_feet_ik_state(
            &mut self.state,
            &mut self.time_in_state,
            &mut self.foot_position,
            self.foot_locking_active,
            self.left_foot_down,
            self.right_foot_down,
            &mut RetainedFeetIkUpdateStateSource {
                processed: &processed,
                skeleton,
            },
        )
    }

    /// `InitUpdate` 0x82dcd260. The retained Processed surface is the result
    /// copied from the previous Skeleton line-test batch earlier in Update1.
    pub(crate) fn try_init_ground_update(
        &mut self,
        skeleton: &SkeletonNormalAssembly,
    ) -> Result<(), OffboardFeetIkPrefixError> {
        let processed = std::sync::Arc::clone(&self.binding.processed);
        let processed = processed
            .lock()
            .map_err(|_| OffboardFeetIkPrefixError::ProcessedPoisoned)?;
        let mut source = RetainedSource {
            processed: &processed,
            skeleton,
        };
        load_prefix(
            &mut self.result_left,
            &mut self.result_right,
            &mut self.current_animation_position_left,
            &mut self.current_animation_position_right,
            &mut source,
        )
    }

    /// Source samples the linked cvar after Y deltas, with a second value
    /// read before testing the right foot if the outer threshold was exceeded.
    pub(crate) fn try_condition_ground_line_results(
        &mut self,
        cvars: &InputCvarReferences,
    ) -> Result<(), InputCvarError> {
        condition_ground_results(
            &mut self.result_left,
            &mut self.result_right,
            self.current_animation_position_left.xyz[1],
            self.current_animation_position_right.xyz[1],
            || cvars.try_ob_ik_max_feet_y_dist(),
        )
    }
}

#[derive(Debug)]
pub(crate) enum OffboardAirHeightError {
    ProcessedPoisoned,
    MissingMirrorFlag,
    Cvar(InputCvarError),
    Numeric(ReckoningHostNumericError),
}

impl OffboardFeetIk {
    /// Identical first five stores of UpdateAir82dd1e50 and UpdateLocomotion82dd1eb0, before InitUpdate.
    pub(crate) fn begin_air_update(&mut self) {
        self.left_fixed_blend_out_timer = 0.0;
        self.bad_ground_correction_counter = 0;
        self.right_fixed_blend_out_timer = 0.0;
        self.wipeout_counter = 0;
        self.foot_not_down_wipeout_counter = 0;
    }
    /// CalculateHeightAboveGroundAir82dd05d8, called AFTER this frame's ApplyIK.
    pub(crate) fn try_finish_air_height(
        &mut self,
        skeleton: &SkeletonNormalAssembly,
        cvars: &InputCvarReferences,
    ) -> Result<(), OffboardAirHeightError> {
        use OffboardAirHeightError as E;
        let left_y = self.current_animation_position_left.xyz[1];
        let right_y = self.current_animation_position_right.xyz[1];
        self.animation_space_y =
            skeleton.root_transforms().animation_to_world().columns()[3].xyz[1];
        let processed = self
            .binding
            .processed
            .lock()
            .map_err(|_| E::ProcessedPoisoned)?;
        let z = skeleton
            .try_effective_animation_to_world(&processed)
            .map_err(|_| E::MissingMirrorFlag)?
            .columns()[2];
        let x = skeleton
            .try_effective_animation_to_world(&processed)
            .map_err(|_| E::MissingMirrorFlag)?
            .columns()[0];
        drop(processed);
        let mut scalar = || cvars.try_slope_effect_scalar();
        let left = air_surface_height(
            self.result_left,
            left_y,
            x,
            z,
            10.0,
            &mut self.left_slope_adjust_y,
            &mut scalar,
        )?;
        let height = air_surface_height(
            self.result_right,
            right_y,
            x,
            z,
            left,
            &mut self.right_slope_adjust_y,
            &mut scalar,
        )?;
        self.height_above_ground = height;
        Ok(())
    }
}

#[expect(clippy::too_many_arguments)]
fn air_surface_height(
    surface: SurfaceResult,
    animation_y: f32,
    x: SkeletonRootVector,
    z: SkeletonRootVector,
    previous: f32,
    slope: &mut f32,
    scalar: &mut impl FnMut() -> Result<f32, InputCvarError>,
) -> Result<f32, OffboardAirHeightError> {
    use OffboardAirHeightError as E;
    if !surface.valid {
        return Ok(previous);
    }
    let ground_y = surface.position.xyz[1];
    let mut numeric = HostReckoningNumeric;
    let dot = numeric.dot_vector(x, surface.normal).map_err(E::Numeric)?;
    let projected = surface.normal.zip(x.map(|v| v * dot), |n, p| n - p);
    let normal = crate::reckoning_numeric::try_normalize_safe(
        projected,
        SkeletonRootVector::known([0.0; 4]),
        &mut numeric,
    )
    .map_err(E::Numeric)?;
    let dot = numeric.dot_vector(normal, z).map_err(E::Numeric)?;
    let adjustment = scalar().map_err(E::Cvar)? * dot;
    let corrected_y = adjustment + ground_y;
    *slope = adjustment;
    let candidate = animation_y - corrected_y;
    Ok(if previous - candidate >= 0.0 {
        candidate
    } else {
        previous
    })
}

#[cfg(test)]
#[path = "physical_player_offboard_feet_ik_prefix_tests.rs"]
mod tests;
