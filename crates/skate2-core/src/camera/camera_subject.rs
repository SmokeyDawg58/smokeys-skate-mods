//! Actual `Iris::Subject82e404c0` constructed by `GameCameraManager82e37c08`.
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};

pub(crate) const ZERO: SkeletonRootVector = SkeletonRootVector::known([0.0; 4]);
const UP: SkeletonRootVector = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
const FORWARD: SkeletonRootVector = SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]);
pub(crate) fn identity() -> SkeletonRootMatrix {
    SkeletonRootMatrix::from_columns([
        SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]),
        UP,
        FORWARD,
        ZERO,
    ])
}

pub(crate) struct CameraSubject {
    pub(crate) anchor_positions: Vec<SkeletonRootVector>,
    pub(crate) anchor_velocities: Vec<SkeletonRootVector>,
    pub(crate) anchor_angular_velocities: Vec<SkeletonRootVector>,
    pub(crate) reference_positions: Vec<SkeletonRootVector>,
    pub(crate) compass_norths: Vec<f32>,
    pub(crate) transform: SkeletonRootMatrix,
    pub(crate) board_effective_transform: SkeletonRootMatrix,
    pub(crate) skeleton_root: SkeletonRootMatrix,
    pub(crate) board_position: SkeletonRootVector,
    pub(crate) board_velocity: SkeletonRootVector,
    pub(crate) board_angular_velocity: SkeletonRootVector,
    pub(crate) trajectory_start_position: SkeletonRootVector,
    pub(crate) trajectory_start_normal: SkeletonRootVector,
    pub(crate) trajectory_end_position: SkeletonRootVector,
    pub(crate) trajectory_end_normal: SkeletonRootVector,
    pub(crate) trajectory_apex_position: SkeletonRootVector,
    pub(crate) trajectory_target_heading: SkeletonRootVector,
    pub(crate) grind_direction: SkeletonRootVector,
    pub(crate) hips_look: SkeletonRootVector,
    pub(crate) hips_position: SkeletonRootVector,
    pub(crate) ground_normal: SkeletonRootVector,
    pub(crate) last_valid_ground_normal: SkeletonRootVector,
    pub(crate) up: SkeletonRootVector,
    pub(crate) look_stick: SkeletonRootVector,
    pub(crate) pumping_acceleration: f32,
    pub(crate) centre_of_mass_to_deck: f32,
    pub(crate) animation_input_lean: f32,
    pub(crate) animation_turn: f32,
    pub(crate) input_turn: f32,
    pub(crate) input_kickturn: f32,
    pub(crate) trajectory_elapsed: f32,
    pub(crate) trajectory_end_time: f32,
    pub(crate) trajectory_apex_time: f32,
    pub(crate) painted_region_blend: f32,
    pub(crate) skitch_grab_position: f32,
    pub(crate) flags: SubjectFlags,
}
#[expect(
    clippy::struct_excessive_bools,
    reason = "native independent Subject flags"
)]
pub(crate) struct SubjectFlags {
    pub(crate) trajectory_valid: bool,
    pub(crate) teleported: bool,
    pub(crate) wiping_out: bool,
    pub(crate) wipeout_over: bool,
    pub(crate) starting_to_jump: bool,
    pub(crate) riding_fakie: bool,
    pub(crate) ground_animation: bool,
    pub(crate) ground_physics: bool,
    pub(crate) anticipating: bool,
    pub(crate) manual: bool,
    pub(crate) physically_pushing: bool,
    pub(crate) pushable_speed: bool,
    pub(crate) mirrored: bool,
    pub(crate) grinding: bool,
    pub(crate) grind_forwards: bool,
    pub(crate) offboard: bool,
    pub(crate) running_out: bool,
}
impl CameraSubject {
    /// CameraMan.GetTrajectoryCollisionTime82e3bfb0: total end time only
    /// while valid, not elapsed or remaining flight time.
    pub(crate) fn trajectory_collision_time(&self) -> f32 {
        if self.flags.trajectory_valid { self.trajectory_end_time } else { 0.0 }
    }
    /// Actual caller supplies six anchors, nine references and six compass slots.
    pub(crate) fn try_construct() -> Result<Self, std::collections::TryReserveError> {
        let anchor_positions = allocated(6, ZERO)?;
        let anchor_velocities = allocated(6, ZERO)?;
        let anchor_angular_velocities = allocated(6, ZERO)?;
        let reference_positions = allocated(9, ZERO)?;
        let compass_norths = allocated(6, 0.0)?;
        Ok(Self {
            anchor_positions,
            anchor_velocities,
            anchor_angular_velocities,
            reference_positions,
            compass_norths,
            transform: identity(),
            board_effective_transform: identity(),
            skeleton_root: identity(),
            board_position: ZERO,
            board_velocity: ZERO,
            board_angular_velocity: ZERO,
            trajectory_start_position: ZERO,
            trajectory_start_normal: UP,
            trajectory_end_position: ZERO,
            trajectory_end_normal: UP,
            trajectory_apex_position: ZERO,
            trajectory_target_heading: FORWARD,
            grind_direction: FORWARD,
            hips_look: FORWARD,
            hips_position: ZERO,
            ground_normal: UP,
            last_valid_ground_normal: UP,
            up: UP,
            look_stick: ZERO,
            pumping_acceleration: 0.0,
            centre_of_mass_to_deck: 0.0,
            animation_input_lean: 0.0,
            animation_turn: 0.0,
            input_turn: 0.0,
            input_kickturn: 0.0,
            trajectory_elapsed: 0.0,
            trajectory_end_time: 0.0,
            trajectory_apex_time: 0.0,
            painted_region_blend: 0.0,
            skitch_grab_position: 0.0,
            flags: SubjectFlags {
                trajectory_valid: false,
                teleported: false,
                wiping_out: false,
                wipeout_over: false,
                starting_to_jump: false,
                riding_fakie: false,
                ground_animation: false,
                ground_physics: false,
                anticipating: false,
                manual: false,
                physically_pushing: false,
                pushable_speed: false,
                mirrored: false,
                grinding: false,
                grind_forwards: false,
                offboard: false,
                running_out: false,
            },
        })
    }
}
fn allocated<T: Clone>(
    count: usize,
    value: T,
) -> Result<Vec<T>, std::collections::TryReserveError> {
    let mut output = Vec::new();
    output.try_reserve_exact(count)?;
    output.resize(count, value);
    Ok(output)
}

#[cfg(test)]
mod trajectory_collision_tests {
    use super::*;
    #[test]
    fn invalid_trajectory_returns_zero_and_valid_uses_total_not_remaining_time() {
        let mut subject = CameraSubject::try_construct().unwrap();
        subject.trajectory_end_time = f32::NAN;
        assert_eq!(subject.trajectory_collision_time().to_bits(), 0);
        subject.flags.trajectory_valid = true;
        subject.trajectory_end_time = 1.25;
        subject.trajectory_elapsed = 0.75;
        assert_eq!(subject.trajectory_collision_time(), 1.25);
        subject.trajectory_end_time = -0.0;
        assert_eq!(subject.trajectory_collision_time().to_bits(), (-0.0f32).to_bits());
    }
}
