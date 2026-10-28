//! Native `GameCameraManager::UpdateReferencePoints82e36008`.
use crate::physics_state_ids::PHYSICS_CATEGORY_GROUNDTRICK;
use crate::{
    camera_subject::{CameraSubject, ZERO},
    physical_player_phys_out::{
        PhysOutSystemVector, PhysicalPlayerPhysOut, construct::ConcretePhysOutError,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::convert::Infallible;

#[derive(Debug)]
pub(crate) enum CameraReferenceError<E> {
    Component(ConcretePhysOutError<Infallible>),
    Camera(E),
}

impl CameraSubject {
    /// The deferred camera read occurs after the native component captures, and
    /// all nine publications occur only after the last ground/grind input read.
    pub(crate) fn try_update_reference_points<E>(
        &mut self,
        output: &PhysicalPlayerPhysOut,
        camera_anchor_output: impl FnOnce() -> Result<Option<SkeletonRootVector>, E>,
    ) -> Result<(), CameraReferenceError<E>> {
        use CameraReferenceError::{Camera, Component};
        let head = output
            .try_skeleton_output()
            .map_err(Component)?
            .skeleton_head_pos;
        let hips = output
            .try_skeleton_output()
            .map_err(Component)?
            .skeleton_hips_pos;
        let centre = output
            .try_camera_system_vector(PhysOutSystemVector::CentreOfMass)
            .map_err(Component)?;
        let centre_again = output
            .try_camera_system_vector(PhysOutSystemVector::CentreOfMass)
            .map_err(Component)?;
        let mut board = output
            .try_camera_board_output()
            .map_err(Component)?
            .deck_position;
        let root = output
            .try_skeleton_output()
            .map_err(Component)?
            .effective_skeleton_root_matrix
            .columns()[3];
        let damped_centre = output
            .try_camera_system_output()
            .map_err(Component)?
            .damped_centre_of_mass;
        let camera_anchor = camera_anchor_output().map_err(Camera)?.unwrap_or(ZERO);
        if !output.try_air_output().map_err(Component)?.is_traj_valid
            && output
                .try_state_output()
                .map_err(Component)?
                .physics_state_category
                != PHYSICS_CATEGORY_GROUNDTRICK
        {
            let ground = output
                .try_ground_output()
                .map_err(Component)?
                .last_ground_pos;
            // Ordered groundY > boardY chooses the original board lane;
            // false (including unordered) selects the sampled ground lane.
            board = lower_reference_height(board, ground);
        }
        let grinds = output.try_camera_grinds_output().map_err(Component)?;
        let grind = if grinds.is_grinding {
            grinds.cam_target_root
        } else {
            board
        };
        self.reference_positions[0] = head;
        self.reference_positions[1] = hips;
        self.reference_positions[2] = grind;
        self.reference_positions[3] = centre;
        self.reference_positions[4] = board;
        self.reference_positions[5] = centre_again;
        self.reference_positions[6] = camera_anchor;
        self.reference_positions[7] = root;
        self.reference_positions[8] = damped_centre;
        Ok(())
    }
}

#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "native unordered compare selects ground Y"
)]
fn lower_reference_height(
    mut board: SkeletonRootVector,
    ground: SkeletonRootVector,
) -> SkeletonRootVector {
    if !(ground.xyz[1] > board.xyz[1]) {
        board.xyz[1] = ground.xyz[1];
    }
    board
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_player_phys_out::construct::PhysOutAllocation;
    #[test]
    fn last_reference_reads_damped_centre_instead_of_centre_velocity() {
        let mut output = PhysicalPlayerPhysOut::new();
        output
            .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        let guard = output.sample_write_guard().unwrap();
        output
            .publish_system_vector(
                &guard,
                PhysOutSystemVector::CentreOfMassVelocity,
                SkeletonRootVector::known([17.0; 4]),
            )
            .unwrap();
        let mut subject = CameraSubject::try_construct().unwrap();
        subject
            .try_update_reference_points(&output, || Ok::<_, ()>(None))
            .unwrap();
        assert_eq!(
            subject.reference_positions[8],
            output
                .try_camera_system_output()
                .unwrap()
                .damped_centre_of_mass
        );
        assert_ne!(
            subject.reference_positions[8],
            SkeletonRootVector::known([17.0; 4])
        );
    }
    #[test]
    fn source_height_selection_changes_only_y_and_keeps_unordered_selection() {
        let board = SkeletonRootVector {
            xyz: [1.0, 4.0, 3.0],
            w: None,
        };
        let lower = lower_reference_height(board, SkeletonRootVector::known([9.0, 2.0, 8.0, 7.0]));
        assert_eq!(
            lower.xyz.map(f32::to_bits),
            [1.0_f32.to_bits(), 2.0_f32.to_bits(), 3.0_f32.to_bits()]
        );
        assert!(lower.w.is_none());
        let nan = f32::from_bits(0x7fc0_1234);
        let unordered =
            lower_reference_height(board, SkeletonRootVector::known([0.0, nan, 0.0, 0.0]));
        assert_eq!(unordered.xyz[1].to_bits(), nan.to_bits());
        let signed_zero = lower_reference_height(
            SkeletonRootVector::known([0.0; 4]),
            SkeletonRootVector::known([0.0, -0.0, 0.0, 0.0]),
        );
        assert_eq!(signed_zero.xyz[1].to_bits(), (-0.0_f32).to_bits());
    }
}
