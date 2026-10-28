//! Draft child of processed `prior_output`: late UpdatePostPhysics57738..57880.
//! Uses actual retained inputs, not the controlled collision corpus snapshot.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_GROUNDTRICK, PHYSICS_CATEGORY_OFFBOARD, PHYSICS_STATE_BONELESS,
    PHYSICS_STATE_FOOTPLANT, PHYSICS_STATE_KNOWN_AIR, PHYSICS_STATE_LANDINGONDECK,
    PHYSICS_STATE_OFFBOARD_AIR,
};
use super::super::SkeletonProcessedContact;
use crate::{
    reckoning::Reckoning, skeleton_collision_data::update::SkeletonCollisionUpdateInput,
    skeleton_root_transforms::SkeletonRootVector,
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CollisionInputError {
    Missing(&'static str),
}

#[cfg(test)]
#[path = "skeleton_postphysics_collision_inputs_live_tests.rs"]
mod tests;
impl std::fmt::Display for CollisionInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "post-physics collision input missing {field}"),
        }
    }
}

impl SkeletonProcessedContact {
    /// Capture at576dc..76f8 BEFORE the Island contact-list query. Supplying
    /// borrowed actual fields avoids resampling the selection after that call.
    pub(crate) fn capture_postphysics_system_velocity(
        &self,
        history: &crate::skeleton_physics_history::SkeletonPhysicsHistory,
        outer: &crate::skeleton_assembly_construction::constructor::SkeletonOuterRemainder,
    ) -> Result<SkeletonRootVector, CollisionInputError> {
        let state = self
            .physics_state
            .physics
            .ok_or(CollisionInputError::Missing("physics state+8e4"))?;
        Ok(if state == PHYSICS_STATE_KNOWN_AIR {
            history.record().vector_typed(crate::skeleton_physics_history::lane_knowledge::RecordVectorField::CentreOfMassVelocity)
        } else {
            outer.skateboard_velocity
        })
    }

    /// Invoke after actual GetCollisions/GetRWSimulation. The processed owner
    /// stays borrowed throughout the source's captured pointer interval.
    /// Constructor amendment retains its OWN primary `GrabSpline` at+6c0.
    pub(crate) fn try_postphysics_collision_input<
        A: crate::reckoning_attributes::ReckoningAttributeRead,
    >(
        &self,
        system_velocity: SkeletonRootVector,
        reckoning: Option<&Reckoning<A>>,
        records: &crate::skeleton_post_physics_records::SkeletonPostPhysicsRecords,
        timestep: f32,
        request_partial_ragdoll: bool,
    ) -> Result<SkeletonCollisionUpdateInput, CollisionInputError> {
        use CollisionInputError as E;
        let grab = self
            .primary_grab
            .as_ref()
            .ok_or(E::Missing("constructed processed primary GrabSpline"))?;
        if grab.status_known_mask & 0x40 == 0 {
            return Err(E::Missing("primary GrabSpline status bit40"));
        }
        // 82b57744..7760: only status bit40 permits reading the actual body.
        // The existing collision consumer compares this occurrence with the
        // opposite spy body before accumulating collision reactions.
        let ignore_rigid_body = if grab.status & 0x40 != 0 {
            grab.collision_rigid_body()
        } else {
            None
        };
        let state = self
            .physics_state
            .physics
            .ok_or(E::Missing("physics state+8e4"))?;
        let mut ground_position = if matches!(state, PHYSICS_STATE_FOOTPLANT | PHYSICS_STATE_BONELESS) {
            self.ground_position
                .ground_position
                .ok_or(E::Missing("ground position+1e0"))?
        } else {
            self.prior_output
                .ground_position_all_states()
                .ok_or(E::Missing("ground position+1f0"))?
        };
        let flags = self.flags_8c8.ok_or(E::Missing("flags+8c8"))?;
        let category = self
            .physics_state
            .category
            .ok_or(E::Missing("category+8ec"))?;
        let mut ground_normal = reckoning
            .ok_or(E::Missing("constructed Reckoning"))?
            .ground_normal();
        let com = records.centre_of_mass_velocity(); //OLD retained+3d20
        let offboard_air = matches!(state, PHYSICS_STATE_OFFBOARD_AIR | PHYSICS_STATE_LANDINGONDECK);
        let offboard = category == PHYSICS_CATEGORY_OFFBOARD && !offboard_air;
        if offboard {
            let word = self
                .physics_state
                .flags_8d0
                .ok_or(E::Missing("offboard flags+8d0"))?;
            if word & 8 != 0 {
                ground_position = self
                    .prior_output
                    .input_fields
                    .next_step_position
                    .ok_or(E::Missing("next step position+370"))?;
                ground_normal = self
                    .prior_output
                    .input_fields
                    .next_step_normal
                    .ok_or(E::Missing("next step normal+380"))?;
            }
        }
        Ok(SkeletonCollisionUpdateInput {
            ignore_rigid_body,
            ground_position: ground_position.xyz,
            ground_normal: ground_normal.xyz,
            system_velocity: system_velocity.xyz,
            centre_of_mass_velocity: [com[0], com[1], com[2]],
            timestep,
            danger_zone: u8::from(flags & 0x0001_0000 != 0),
            ai_skater: u8::from(flags & 0x4000_0000 != 0),
            offboard: u8::from(offboard),
            offboard_air: u8::from(offboard_air),
            ground_trick: u8::from(category == PHYSICS_CATEGORY_GROUNDTRICK),
            request_partial_ragdoll: u8::from(request_partial_ragdoll),
        })
    }
}
