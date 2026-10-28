//! PostWipeoutCheck82b57c7c..581f0. Child of `SkeletonNormalAssembly`.
use super::SkeletonNormalAssembly;
use crate::{
    actor_construction::anim_out_phys_in::{
        AnimOutPhysInOwner,
        pose_lanes::{PoseAccessError, PoseBuffer},
    },
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate},
    reckoning_sin_cos::{ReckoningSinCosNumeric, try_sin_cos},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_assembly_construction::constructor::init_state::SkeletonInitState,
    skeleton_ik_state::external::normal_rotation,
    skeleton_physics_history::{SkeletonPhysicsHistory, SkeletonPhysicsHistoryError},
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};
use std::convert::Infallible;

#[cfg(test)]
#[path = "physical_player_post_wipeout_head_orientation_tests.rs"]
mod tests;

/// Native57d4c captures one target; scalar57d58 and vector57d68 both read it.
/// Implement with a retained cvar allocation, not two fresh reference lookups.
pub(crate) trait HeadOrientationCvars {
    type Error;
    type HeadTarget;
    fn capture_head_fraction(&mut self) -> Result<Self::HeadTarget, Self::Error>;
    fn head_fraction(&mut self, target: &Self::HeadTarget) -> Result<f32, Self::Error>;
    fn spine_fraction(&mut self) -> Result<f32, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum HeadOrientationOutputError {
    Missing(&'static str),
    Pose(PoseAccessError),
    Record(SkeletonPhysicsHistoryError<Infallible, Infallible>),
}
impl std::fmt::Display for HeadOrientationOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "head orientation missing {field}"),
            Self::Pose(error) => error.fmt(f),
            Self::Record(error) => write!(f, "head orientation record: {error:?}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum HeadOrientationError<C, N, R, O = HeadOrientationOutputError> {
    Cvar(C),
    Numeric(N),
    Trigonometry(R),
    Output(O),
}
impl<C: std::fmt::Debug, N: std::fmt::Debug, R: std::fmt::Debug, O: std::fmt::Debug>
    std::fmt::Display for HeadOrientationError<C, N, R, O>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "post head orientation: {self:?}")
    }
}

/// Reaching this token authorizes the caller's fresh kUpdatePoseBuffer read,
/// not frame completion or unconditional pose publication.
pub(crate) struct HeadOrientationBeforePose;

trait OrientationOutput {
    type Error;
    fn adjust_parent_record(&mut self, rotation: &Matrix) -> Result<(), Self::Error>;
    fn adjust_grandparent_record(&mut self, rotation: &Matrix) -> Result<(), Self::Error>;
    fn publish_head_global(&mut self, head: &Matrix) -> Result<(), Self::Error>;
    fn publish_head_local(&mut self) -> Result<(), Self::Error>;
}

impl SkeletonNormalAssembly {
    /// Actual captured head and same `constructor.init/animation_input` owners
    /// must come from the identity-checked pool caller after HeadTracking.Update.
    /// Reference: 0x82b57c7c. No Part/body transform is written by this suffix.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_post_wipeout_head_orientation<
        C: HeadOrientationCvars,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        pose: &mut AnimOutPhysInOwner,
        indices: &SkeletonInitState,
        captured_head: Matrix,
        axis_angle: Vector,
        cvars: &mut C,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<HeadOrientationBeforePose, HeadOrientationError<C::Error, N::Error, R::Error>> {
        let mut output = LiveOutput {
            pose,
            indices,
            history: &mut self.physics_history,
            world_to_animation: self.root_transforms.world_to_animation(),
            animation_to_world: self.root_transforms.animation_to_world(),
        };
        orient(
            captured_head,
            axis_angle,
            cvars,
            numeric,
            rounding,
            &mut output,
        )
    }
}

#[expect(clippy::type_complexity)]
#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "native ordered comparison skips unordered values"
)]
fn orient<
    C: HeadOrientationCvars,
    N: ReckoningNumeric,
    R: ReckoningSinCosNumeric,
    O: OrientationOutput,
>(
    captured_head: Matrix,
    axis_angle: Vector,
    cvars: &mut C,
    numeric: &mut N,
    rounding: &mut R,
    output: &mut O,
) -> Result<HeadOrientationBeforePose, HeadOrientationError<C::Error, N::Error, R::Error, O::Error>>
{
    use HeadOrientationError as E;
    let squared = numeric
        .dot_vector(axis_angle, axis_angle)
        .map_err(E::Numeric)?;
    // SMALL_FLOAT_82@821f0c20, splatted by VECFLOAT_SMALL initializer82f76340.
    // Raw57d0c skips zero/equal/unordered, before either fractional cvar.
    if !(squared > f32::from_bits(0x3780_0000)) {
        return Ok(HeadOrientationBeforePose);
    }
    let seed = numeric
        .reciprocal_sqrt_estimate(squared)
        .map_err(E::Numeric)?;
    let target = cvars.capture_head_fraction().map_err(E::Cvar)?;
    let remainder = 1.0 - cvars.head_fraction(&target).map_err(E::Cvar)?;
    let fraction = cvars.head_fraction(&target).map_err(E::Cvar)?;
    let length = length_from_squared_and_estimate(squared, seed);
    let reciprocal_seed = numeric.reciprocal_estimate(length).map_err(E::Numeric)?;
    let head_angle = fraction * length;
    let remaining_angle = remainder * length;
    let reciprocal = refine_drive_reciprocal(length, reciprocal_seed);
    let unit = axis_angle.xyz.map(|v| reciprocal * v);
    let (sine, cosine) = try_sin_cos(head_angle, rounding).map_err(E::Trigonometry)?;
    let head_rotation = normal_rotation(unit, sine, cosine);
    let (sine, cosine) = try_sin_cos(remaining_angle, rounding).map_err(E::Trigonometry)?;
    //57ecc/57ee8: sampled only after the SECOND SinCos returns.
    let spine_fraction = cvars.spine_fraction().map_err(E::Cvar)?;
    let parent_rotation = normal_rotation(unit, sine, cosine);
    let (sine, cosine) =
        try_sin_cos(spine_fraction * remaining_angle, rounding).map_err(E::Trigonometry)?;
    let grandparent_rotation = normal_rotation(unit, sine, cosine);
    let adjusted_head = rotate_basis(&head_rotation, captured_head);
    output
        .adjust_parent_record(&parent_rotation)
        .map_err(E::Output)?;
    output
        .adjust_grandparent_record(&grandparent_rotation)
        .map_err(E::Output)?;
    output
        .publish_head_global(&adjusted_head)
        .map_err(E::Output)?;
    output.publish_head_local().map_err(E::Output)?;
    Ok(HeadOrientationBeforePose)
}

struct LiveOutput<'a> {
    pose: &'a mut AnimOutPhysInOwner,
    indices: &'a SkeletonInitState,
    history: &'a mut SkeletonPhysicsHistory,
    world_to_animation: &'a Matrix,
    animation_to_world: &'a Matrix,
}
impl LiveOutput<'_> {
    fn head(&self) -> Result<usize, HeadOrientationOutputError> {
        self.indices.named_bones[6]
            .and_then(|v| usize::try_from(v).ok())
            .ok_or(HeadOrientationOutputError::Missing("Head bone index"))
    }
    fn adjust(&mut self, rotation: &Matrix, part: usize) -> Result<(), HeadOrientationOutputError> {
        let original = self.history.post_wipeout_record_transform(part);
        let adjusted = adjust_record_orientation(
            original,
            rotation,
            self.world_to_animation,
            self.animation_to_world,
        );
        self.history
            .try_set_post_wipeout_record_transform(part, &adjusted)
            .map_err(HeadOrientationOutputError::Record)
    }
}
impl OrientationOutput for LiveOutput<'_> {
    type Error = HeadOrientationOutputError;
    fn adjust_parent_record(&mut self, rotation: &Matrix) -> Result<(), Self::Error> {
        let part = self
            .indices
            .head_parent_part
            .ok_or(Self::Error::Missing("head parent Part"))?;
        self.adjust(rotation, part.index())
    }
    fn adjust_grandparent_record(&mut self, rotation: &Matrix) -> Result<(), Self::Error> {
        let part = self
            .indices
            .head_grandparent_part
            .flatten()
            .ok_or(Self::Error::Missing("head grandparent Part"))?;
        self.adjust(rotation, part.index())
    }
    fn publish_head_global(&mut self, head: &Matrix) -> Result<(), Self::Error> {
        let bone = self.head()?;
        self.pose
            .set_pose_matrix_typed(PoseBuffer::Global, bone, head)
            .map_err(Self::Error::Pose)
    }
    fn publish_head_local(&mut self) -> Result<(), Self::Error> {
        //58104 re-loads the same actual pose owner; fresh parent-global read
        //precedes freshly reading the global head already published above.
        let head = self.head()?;
        let parent = self
            .indices
            .head_parent
            .and_then(|v| usize::try_from(v).ok())
            .ok_or(Self::Error::Missing("head parent animation bone"))?;
        let parent = self
            .pose
            .pose_matrix_typed(PoseBuffer::Global, parent)
            .map_err(Self::Error::Pose)?;
        let inverse = parent.inverse();
        let child = self
            .pose
            .pose_matrix_typed(PoseBuffer::Global, head)
            .map_err(Self::Error::Pose)?;
        let local = inverse.compose(&child);
        self.pose
            .set_pose_matrix_typed(PoseBuffer::Local, head, &local)
            .map_err(Self::Error::Pose)
    }
}

/// `AdjustPhysicsBoneRecordOrientation82b3fbe8`: direction-only triples in source
/// order; original translation copied verbatim, then caller checked record store.
/// Never precompose these matrices or route the original translation through them.
fn adjust_record_orientation(
    original: Matrix,
    rotation: &Matrix,
    world_to_animation: &Matrix,
    animation_to_world: &Matrix,
) -> Matrix {
    let in_animation = rotate_basis(world_to_animation, original);
    let adjusted = rotate_basis(rotation, in_animation);
    rotate_basis(animation_to_world, adjusted)
}
fn rotate_basis(rotation: &Matrix, original: Matrix) -> Matrix {
    let mut columns = *original.columns();
    for column in &mut columns[..3] {
        *column = direction(rotation, *column);
    }
    Matrix::from_columns(columns)
}
fn direction(matrix: &Matrix, value: Vector) -> Vector {
    let columns = matrix.columns();
    let x = columns[0].map(|v| value.xyz[0] * v);
    let xy = columns[1].zip(x, |v, x| value.xyz[1].mul_add(v, x));
    columns[2].zip(xy, |v, xy| value.xyz[2].mul_add(v, xy))
}
