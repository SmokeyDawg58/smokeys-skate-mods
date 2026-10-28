//! `MongoPushFootToFar::ActivationFunc`82caa870, using retained animation
//! attributes and the original PhysOut components read by `GetFootPos`825f5100.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, InputCvarReferences};
use crate::{
    generated_motion_load::runtime::MotionUpdateError, play_animation::FastString30,
    skeleton_root_transforms::SkeletonRootVector,
};

impl ActorConstruction {
    pub(crate) fn motion_runtime_mongo_push_foot_too_far(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        contact: FastString30,
        right_bone: FastString30,
        left_bone: FastString30,
        cvars: &InputCvarReferences,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        let Some(attribute) = self.motion_runtime_tree_attribute(context, contact)? else {
            return Ok(false);
        };
        let sample = attribute
            .value
            .contact_value()
            .map_err(E::ContactAttribute)?
            .ok_or(E::PendingAnimationOperation(
                "MongoPushFootToFar requires type-3 contact attribute",
            ))?;
        let selected = FastString30::encode(&sample.bone_name).map_err(|_| {
            E::PendingAnimationOperation("MongoPushFootToFar contact bone name encoding")
        })?;

        // The original checks GetIsMirrored before the right-name compare.
        // If that route does not match, it queries GetIsMirrored again before
        // the left-name compare. Both calls read the same retained player bit.
        let first_mirrored = self
            .runtime_is_mirrored_from_checked_context()
            .map_err(E::GestureCondition)?;
        if first_mirrored || selected != right_bone {
            let second_mirrored = self
                .runtime_is_mirrored_from_checked_context()
                .map_err(E::GestureCondition)?;
            if !second_mirrored || selected != left_bone {
                return Ok(false);
            }
        }

        let right = selected == right_bone;
        let forward_distance = self.motion_runtime_foot_pos(right)?[0];
        let threshold = cvars.try_mongo_push_to_antic_thresh().map_err(E::Cvar)?;
        Ok(threshold > forward_distance)
    }

    /// `SkaterMotionGraph::GetFootPos825f5100`, virtual +0xac. The returned
    /// Vector2 contains flipped deck-Z distance, then deck-Y height.
    /// Both MongoPushFootToFar and PushOut.Begin consume this same owner.
    pub(super) fn motion_runtime_foot_pos(
        &self,
        right: bool,
    ) -> Result<[f32; 2], MotionUpdateError> {
        use MotionUpdateError as E;
        // The native virtual asserts when a valid Actor lacks IPhysical.
        // The only zero-result branch is a null Actor, already rejected by
        // the selected Motion context guard.
        if self
            .prefix()
            .map_err(E::Actor)?
            .physical_interface
            .is_none()
        {
            return Err(E::MissingOwner("GetFootPos IPhysical"));
        }
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
        let skeleton = output.try_skeleton_output().map_err(E::PhysOut)?;
        let foot = if right {
            skeleton.skeleton_foot_right_pos
        } else {
            skeleton.skeleton_foot_left_pos
        };
        let board = output.try_camera_board_output().map_err(E::PhysOut)?;
        let displacement = foot.zip(board.deck_position, |a, b| a - b);
        let deck_y = board.transform.columns()[1];
        let deck_z = board.transform.columns()[2];
        let flipped = output
            .try_camera_motion_output()
            .map_err(E::PhysOut)?
            .skateboard_flipped;
        Ok(deck_foot_pos(displacement, deck_y, deck_z, flipped))
    }
}

fn deck_foot_pos(
    displacement: SkeletonRootVector,
    deck_y: SkeletonRootVector,
    deck_z: SkeletonRootVector,
    flipped: bool,
) -> [f32; 2] {
    let deck_z = if flipped { deck_z.map(|v| -v) } else { deck_z };
    [
        deck_forward_distance(displacement, deck_z),
        deck_forward_distance(displacement, deck_y),
    ]
}

fn deck_forward_distance(displacement: SkeletonRootVector, deck_z: SkeletonRootVector) -> f32 {
    let d = displacement.xyz;
    let z = deck_z.xyz;
    d[0] * z[0] + d[1] * z[1] + d[2] * z[2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foot_pos_uses_forward_and_height_not_sideways_distance() {
        // GetFootPos825f52f4 loads Matrix44Affine.yAxis (+0x10).
        // Raw VMX128: v56=dot(displacement,flippedZ), v57=dot(displacement,Y).
        // Permute bytes 00010203 14151617 select v56[0], v57[1].
        let displacement = SkeletonRootVector::known([3.0, 0.2, -0.75, 0.0]);
        let y = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
        let z = SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]);
        assert_eq!(deck_foot_pos(displacement, y, z, false), [-0.75, 0.2]);
        assert_eq!(deck_foot_pos(displacement, y, z, true), [0.75, 0.2]);
        // Rotating both world displacement and board preserves both distances.
        let turned = SkeletonRootVector::known([-0.75, 0.2, -3.0, 0.0]);
        let turned_z = SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]);
        assert_eq!(deck_foot_pos(turned, y, turned_z, false), [-0.75, 0.2]);
    }
}
