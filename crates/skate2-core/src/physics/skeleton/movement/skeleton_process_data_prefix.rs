//! ProcessData82b5ed48..efd0 trajectory continuation, after actual attributes.
//! The caller borrows the constructor's current/inverse and the sole General
//! Update `NextTrajectory`; this module owns no duplicate frame state.
use crate::{
    actor_construction::anim_out_phys_in::{
        AnimOutPhysInOwner,
        pose_lanes::{PoseAccessError, PoseBuffer},
    },
    animation::AnimationMatrix,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::ReckoningNumeric,
    skateboard_spawn_math::{skateboard_spawn_compose, skateboard_spawn_rigid_inverse},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

pub(crate) struct TrajectoryFields<'a> {
    pub(crate) current: &'a mut AnimationMatrix,
    pub(crate) inverse: &'a mut AnimationMatrix,
    pub(crate) next: &'a mut SkeletonRootMatrix,
    pub(crate) velocity: &'a mut SkeletonRootVector,
}
pub(crate) struct TrajectoryInput<'a> {
    pub(crate) pose: TrajectoryPose<'a>,
    pub(crate) trajectory_node: i32,
    pub(crate) processed_dt: Option<f32>,
    pub(crate) animation_to_world: &'a SkeletonRootMatrix,
}
pub(crate) enum TrajectoryPose<'a> {
    Retained(&'a AnimOutPhysInOwner),
    #[cfg(test)]
    Known(Option<&'a [AnimationMatrix]>),
}
impl TrajectoryPose<'_> {
    fn read(&self) -> Result<&[AnimationMatrix], PoseAccessError> {
        match self {
            Self::Retained(owner) => owner.pose_known_lanes(PoseBuffer::Global),
            #[cfg(test)]
            Self::Known(pose) => pose.ok_or(PoseAccessError::Missing(PoseBuffer::Global)),
        }
    }
}
#[derive(Debug)]
pub(crate) enum TrajectoryError<N> {
    Missing(&'static str),
    UnknownNextTrajectory,
    Pose(PoseAccessError),
    InvalidTrajectoryNode(i32),
    Numeric(N),
    Cvar(InputCvarError),
}
impl<N: std::fmt::Debug> std::fmt::Display for TrajectoryError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "trajectory source field {field} unavailable"),
            Self::UnknownNextTrajectory => f.write_str("NextTrajectory has unknown lanes"),
            Self::Pose(error) => error.fmt(f),
            Self::InvalidTrajectoryNode(node) => {
                write!(f, "trajectory pose node {node} unavailable")
            }
            Self::Numeric(error) => write!(f, "trajectory reciprocal: {error:?}"),
            Self::Cvar(error) => write!(f, "trajectory cvar: {error:?}"),
        }
    }
}

impl super::SkeletonNormalAssembly {
    /// Borrow the same constructor remainder and live general/root projections.
    pub(crate) fn try_process_data_trajectory<N: ReckoningNumeric>(
        &mut self,
        constructor: &mut crate::skeleton_assembly_construction::constructor::SkeletonConstructorRuntime,
        input: &crate::actor_construction::anim_out_phys_in::AnimOutPhysInOwner,
        processed_dt: Option<f32>,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), TrajectoryError<N::Error>> {
        let outer = constructor
            .outer
            .as_mut()
            .ok_or(TrajectoryError::Missing("constructor remainder"))?;
        let pose = TrajectoryPose::Retained(input);
        try_process_trajectory(
            TrajectoryFields {
                current: &mut outer.trajectory_transform,
                inverse: &mut outer.trajectory_inverse,
                next: self.general_update.next_trajectory_transform_mut(),
                velocity: &mut outer.trajectory_velocity,
            },
            &TrajectoryInput {
                pose,
                trajectory_node: constructor.init.trajectory_node,
                processed_dt,
                animation_to_world: self.root_transforms.animation_to_world(),
            },
            cvars,
            numeric,
        )
    }
}

/// Raw82b5ee9c/eea4 VNMSUB negates the rounded FMA; exact-zero signs matter.
fn reciprocal_refinement(dt: f32, mut seed: f32) -> f32 {
    for _ in 0..2 {
        let residual = -(seed.mul_add(dt, -1.0));
        seed = seed.mul_add(residual, seed);
    }
    seed
}

/// Does not guard zero dt or normalize the accumulated trajectory. Each late
/// failure preserves all native writes preceding its read/call boundary.
pub(crate) fn try_process_trajectory<N: ReckoningNumeric>(
    fields: TrajectoryFields<'_>,
    input: &TrajectoryInput<'_>,
    cvars: &InputCvarReferences,
    numeric: &mut N,
) -> Result<(), TrajectoryError<N::Error>> {
    use TrajectoryError as E;
    let TrajectoryFields {
        current,
        inverse,
        next,
        velocity,
    } = fields;
    *current = next.known_lanes().ok_or(E::UnknownNextTrajectory)?;
    let poses = input.pose.read().map_err(E::Pose)?;
    let pose = usize::try_from(input.trajectory_node)
        .ok()
        .and_then(|node| poses.get(node))
        .ok_or(E::InvalidTrajectoryNode(input.trajectory_node))?;
    *current = skateboard_spawn_compose(pose, current);
    *next = SkeletonRootMatrix::known(*current);
    let dt = input.processed_dt.ok_or(E::Missing("processed dt"))?;
    let [x_axis, y_axis, z_axis, _] = *input.animation_to_world.columns();
    let seed = numeric.reciprocal_estimate(dt).map_err(E::Numeric)?;
    let reciprocal = reciprocal_refinement(dt, seed);
    let translation: [f32; 3] = std::array::from_fn(|lane| current[12 + lane] * reciprocal);
    let rotate_lane = |x, y: f32, z: f32| {
        let x = x * translation[0];
        let xy = y.mul_add(translation[1], x);
        z.mul_add(translation[2], xy)
    };
    *velocity = SkeletonRootVector {
        xyz: std::array::from_fn(|lane| {
            rotate_lane(x_axis.xyz[lane], y_axis.xyz[lane], z_axis.xyz[lane])
        }),
        w: x_axis
            .w
            .zip(y_axis.w)
            .zip(z_axis.w)
            .map(|((x, y), z)| rotate_lane(x, y, z)),
    };
    if cvars.try_zero_trajectory_transform().map_err(E::Cvar)? {
        *current = crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
        *next = SkeletonRootMatrix::known(*current);
    }
    // ef64 resolves to shared825187c0, whose sole instruction is BLR.
    // Preserve the live cvar read; its enabled call has no observable effect.
    let _display_trajectory = cvars
        .try_skeleton_display_trajectory_node()
        .map_err(E::Cvar)?;
    *inverse = skateboard_spawn_rigid_inverse(current);
    Ok(())
}

#[cfg(test)]
#[path = "skeleton_process_data_prefix_tests.rs"]
mod tests;
