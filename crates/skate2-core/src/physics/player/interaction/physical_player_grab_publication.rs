//! `PlayerGrabSpline.UpdateProcessedPhysIn`82dcbb18 on retained futures.
use super::PlayerGrabSpline;
use crate::skeleton_processed_contact::SkeletonProcessedContact;
use std::fmt;
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GrabPublicationError {
    MissingProcessedFlags,
    MissingPrimaryTarget,
    MissingAlternateTarget,
    FuturePoisoned,
}
impl fmt::Display for GrabPublicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingProcessedFlags => "grab publication lacks processed8d0",
            Self::MissingPrimaryTarget => "primary processed grab spline was not constructed",
            Self::MissingAlternateTarget => "alternate processed grab spline was not constructed",
            Self::FuturePoisoned => "grab data future lock poisoned",
        })
    }
}
fn publish_flag(
    processed: &mut SkeletonProcessedContact,
    mask: u32,
    value: bool,
) -> Result<(), GrabPublicationError> {
    let state = processed.physics_state_fields_mut();
    let word = state
        .flags_8d0
        .ok_or(GrabPublicationError::MissingProcessedFlags)?;
    state.flags_8d0 = Some((word & !mask) | if value { mask } else { 0 });
    Ok(())
}
impl PlayerGrabSpline {
    pub(crate) fn try_publish_processed(
        &mut self,
        processed: &mut SkeletonProcessedContact,
    ) -> Result<(), GrabPublicationError> {
        // Native pointer access is short-circuited by each actual future.ready.
        {
            let mut future = self
                .data
                .lock()
                .map_err(|_| GrabPublicationError::FuturePoisoned)?;
            let ready = future.ready && future.result.spline_data.is_some();
            if ready && !processed.publish_grab_spline(&future.result, false) {
                return Err(GrabPublicationError::MissingPrimaryTarget);
            }
            publish_flag(processed, 0x0100_0000, ready)?;
            future.ready = false;
        }
        {
            let mut future = self
                .alternate
                .lock()
                .map_err(|_| GrabPublicationError::FuturePoisoned)?;
            let ready = future.ready && future.result.spline_data.is_some();
            if ready && !processed.publish_grab_spline(&future.result, true) {
                return Err(GrabPublicationError::MissingAlternateTarget);
            }
            publish_flag(processed, 0x0080_0000, ready)?;
            future.ready = false;
        }
        let mut interactable = self
            .interactable
            .lock()
            .map_err(|_| GrabPublicationError::FuturePoisoned)?;
        if interactable.ready {
            let count = interactable.result_count;
            interactable.cursor = 0; //bc80 before count branch
            if count > 0 {
                interactable.cursor = 1;
                publish_flag(processed, 0x0040_0000, true)?;
                processed.physics_state_fields_mut().interactable_object_id =
                    Some(interactable.object_id);
            } else {
                publish_flag(processed, 0x0040_0000, false)?;
            }
        }
        // Unready branch preserves processed flag and prior object payload.
        interactable.ready = false;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::{GrabPublicationError, PlayerGrabSpline};
    use crate::{
        physical_player_assembly::tests::issued_owner_for_grind_tests,
        skeleton_processed_contact::SkeletonProcessedContact,
    };
    #[test]
    fn processed_collision_input_keeps_the_actual_grabbed_body_and_status_gate() {
        use crate::{
            attribute_runtime::tests::graph,
            reckoning::Reckoning,
            reckoning_attributes::{RECKONING_CLASS, ReckoningVaultAttributes},
            simulation_rigid_body_pool::SimulationRigidBodyPool,
            skateboard_wheels::FRESH_VOLUME_TRANSFORM,
            skeleton_post_physics_records::SkeletonPostPhysicsRecords,
            skeleton_root_transforms::SkeletonRootVector,
        };
        let mut bodies = SimulationRigidBodyPool::try_new(1).unwrap();
        let handle = bodies
            .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, 4)
            .unwrap()
            .unwrap();
        let body = bodies.try_observe(handle).unwrap();
        let mut spline = super::super::GrabSpline::in_sim_allocation();
        spline.rigid_body = Some(body);
        let mut processed = SkeletonProcessedContact::new();
        processed.try_initialize_constructor_fields().unwrap();
        let reckoning = Reckoning::try_new(ReckoningVaultAttributes::bind(&std::sync::Arc::new(
            graph(RECKONING_CLASS, vec![]),
        )))
        .unwrap();
        for valid in [false, true, false] {
            spline.status = if valid { 0x40 } else { 0 };
            assert!(processed.publish_grab_spline(&spline, false));
            let input = processed
                .try_postphysics_collision_input(
                    SkeletonRootVector::known([0.0; 4]),
                    Some(&reckoning),
                    &SkeletonPostPhysicsRecords::new(),
                    1.0 / 60.0,
                    false,
                )
                .unwrap();
            assert_eq!(input.ignore_rigid_body, valid.then_some(body));
        }
    }

    #[test]
    fn ready_splines_copy_into_existing_processed_members_then_keep_payload_when_unready() {
        let (_, owner) = issued_owner_for_grind_tests();
        let mut grab = PlayerGrabSpline::new_prefix(&owner, None);
        // Valid minimal tGrabData fixture; this test exercises pointer presence
        // and publication, not reachability or the geometry initializer.
        let mut bytes = vec![0u8; 0x70];
        bytes[0..4].copy_from_slice(&1u32.to_be_bytes());
        bytes[0x10..0x14].copy_from_slice(&0x20u32.to_be_bytes());
        let source = crate::living_world_grind_services::dmo_models::DmoGrabSpline {
            entry_index: 0,
            header_offset: 0x20,
            data: std::sync::Arc::new(skate2_formats::pegasus::GrabData::parse(&bytes).unwrap()),
        };
        for (future, id) in [(&grab.data, 17), (&grab.alternate, 29)] {
            let mut future = future.lock().unwrap();
            future.result.init_descriptor(2, id);
            future.result.spline_data = Some(source.clone());
            future.result.p1 = [id as f32; 4];
            future.ready = true;
        }
        let mut processed = SkeletonProcessedContact::new();
        processed.try_initialize_constructor_fields().unwrap();
        grab.try_publish_processed(&mut processed).unwrap();
        assert_eq!(
            processed.physics_state_fields().flags_8d0,
            Some(0x0180_0000)
        );
        assert_eq!(processed.grab_spline(false).unwrap().p1, [17.0; 4]);
        assert_eq!(processed.grab_spline(true).unwrap().p1, [29.0; 4]);
        grab.try_publish_processed(&mut processed).unwrap();
        assert_eq!(processed.physics_state_fields().flags_8d0, Some(0));
        assert_eq!(processed.grab_spline(false).unwrap().descriptor_object, 17);
        assert_eq!(processed.grab_spline(true).unwrap().descriptor_object, 29);
    }
    #[test]
    fn actual_constructor_idle_preserves_unready_interactable_flag() {
        let (_world, owner) = issued_owner_for_grind_tests();
        let mut grab = PlayerGrabSpline::new_prefix(&owner, None);
        let mut processed = SkeletonProcessedContact::new();
        processed.reset();
        processed.physics_state_fields_mut().flags_8d0 = Some(0xa1c0_0001);
        grab.try_publish_processed(&mut processed).unwrap();
        assert_eq!(
            processed.physics_state_fields().flags_8d0,
            Some(0xa040_0001)
        );
        assert!(
            !grab.data.lock().unwrap().ready
                && !grab.alternate.lock().unwrap().ready
                && !grab.interactable.lock().unwrap().ready
        );
    }
    #[test]
    fn ready_null_splines_consume_readiness_and_publish_actual_interactable_id() {
        let (_world, owner) = issued_owner_for_grind_tests();
        let mut grab = PlayerGrabSpline::new_prefix(&owner, None);
        grab.data.lock().unwrap().ready = true;
        grab.alternate.lock().unwrap().ready = true;
        grab.interactable.lock().unwrap().ready = true;
        grab.interactable.lock().unwrap().result_count = 1;
        grab.interactable.lock().unwrap().object_id = 0x0100_0042;
        let mut processed = SkeletonProcessedContact::new();
        processed.reset();
        assert_eq!(grab.try_publish_processed(&mut processed), Ok(()));
        assert!(!grab.data.lock().unwrap().ready && !grab.alternate.lock().unwrap().ready);
        assert!(!grab.interactable.lock().unwrap().ready);
        assert_eq!(
            processed.physics_state_fields().interactable_object_id,
            Some(0x0100_0042)
        );
        assert_eq!(grab.interactable.lock().unwrap().cursor, 1);
        assert_eq!(
            processed.physics_state_fields().flags_8d0,
            Some(0x0040_0000)
        );
        grab.interactable.lock().unwrap().result_count = 0;
        grab.interactable.lock().unwrap().ready = true;
        grab.try_publish_processed(&mut processed).unwrap();
        assert!(!grab.interactable.lock().unwrap().ready);
        assert_eq!(grab.interactable.lock().unwrap().cursor, 0);
        assert_eq!(processed.physics_state_fields().flags_8d0, Some(0));
    }
}
