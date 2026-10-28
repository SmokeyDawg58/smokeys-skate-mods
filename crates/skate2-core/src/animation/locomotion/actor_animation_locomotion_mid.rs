//! SkaterAnim::UpdateLocomotion82ca9bf8, between the two StepSelector harvests.
use super::{
    ActorAnimationConstruction,
    frame_phase::AnimationPhaseError,
    locomotion_create::{CreateTuningError, prediction_tuning},
    locomotion_create_finish::{CreatePredictionTuning, StepLaunchContinuation},
    locomotion_producer::TrajectoryProducerError,
    locomotion_update::{LocomotionUpdateError, lengths},
    physics_update_data::AnimationPhysicsDataError,
};
use crate::{input_cvar_runtime::InputCvarReferences, reckoning_numeric::HostReckoningNumeric};

#[derive(Debug)]
pub(crate) enum MidLocomotionError {
    Missing(&'static str),
    Phase(AnimationPhaseError),
    FirstFutureNotReady,
    Prediction(TrajectoryProducerError<CreateTuningError>),
    Tuning(CreateTuningError),
    IntegerDivision,
    UnknownFlags(u32),
    Action(AnimationPhysicsDataError),
    Length(LocomotionUpdateError),
    CoincidentQueriedFeet,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_animation_construction::{
        ActorAnimationCvars, ActorAnimationInputs, ActorAnimationInterface, ActorAnimationRegistry,
        AnimationLifetime,
    };
    use crate::input_cvar_runtime::InputCvarManager;
    use crate::motion_graph_entity_authority::{ActorGraphAuthority, MotionGraphEntityAuthority};
    use crate::skeleton_root_transforms::SkeletonRootVector as V;
    use std::{convert::Infallible, sync::Arc};
    struct Registry;
    impl ActorAnimationRegistry<u32> for Registry {
        type Error = Infallible;
        fn register_animation_interface(
            &mut self,
            _: &MotionGraphEntityAuthority<u32>,
            _: ActorAnimationInterface,
            _: &Arc<()>,
        ) -> Result<(), Infallible> {
            Ok(())
        }
        fn actor_player_index(
            &mut self,
            _: &MotionGraphEntityAuthority<u32>,
        ) -> Result<u32, Infallible> {
            Ok(0)
        }
    }
    struct Sprint;
    impl ActorAnimationCvars for Sprint {
        type Error = Infallible;
        fn use_sprint_button(&mut self) -> Result<u8, Infallible> {
            Ok(0)
        }
    }
    fn owner() -> ActorAnimationConstruction<u32, ()> {
        let authority = ActorGraphAuthority::new(0);
        let mut owner = ActorAnimationConstruction::new(
            &authority,
            ActorAnimationInputs {
                physics_world: Arc::new(()),
                life: AnimationLifetime::Transient,
                use_low_blend_cull_threshold: false,
                face_name: None,
            },
        );
        owner.try_mark_sim_allocation().unwrap();
        owner.try_construct_prefix(&mut Registry).unwrap();
        let vault = crate::attribute_runtime::tests::graph(
            crate::actor_animation_construction::ACTOR_ANIMATION_OFFBOARD_CLASS,
            Vec::new(),
        );
        owner
            .try_construct_vault_suffix(Arc::new(vault), &mut Sprint)
            .unwrap();
        owner.player.as_mut().unwrap().flags_bits |= 0x0010_0000;
        owner.before_sprint.as_mut().unwrap().foot_down = 0;
        let skater = owner.skater.as_mut().unwrap();
        skater.trajectory.current_state = 0;
        skater.trajectory.physical_foot_index = 0;
        skater.trajectory.input_feet[1].data.position = V {
            xyz: [1.0, 0.0, 0.0],
            w: None,
        };
        skater.trajectory.input_feet[2].data.position = V {
            xyz: [3.0, 0.0, 0.0],
            w: None,
        };
        skater.trajectory.physical_feet[0].states[0].action = 0;
        owner
    }
    #[test]
    fn inactive_and_pending_first_future_do_not_touch_prediction_or_second_future() {
        let mut owner = owner();
        let refs = InputCvarReferences::new_static_initialized();
        owner.skater.as_mut().unwrap().step_inputs[1].flags = 0xff;
        assert!(matches!(
            owner.try_mid_locomotion_prefix(&refs),
            Err(MidLocomotionError::FirstFutureNotReady)
        ));
        assert_eq!(owner.skater.as_ref().unwrap().step_inputs[1].flags, 0xff);
        owner.player.as_mut().unwrap().flags_bits &= !0x0010_0000;
        assert!(matches!(
            owner.try_mid_locomotion_prefix(&refs),
            Ok(MidLocomotionContinuation::Inactive)
        ));
        assert!(!owner.skater.as_ref().unwrap().step_results[1].ready);
    }
    #[test]
    fn invalid_first_result_predicts_but_never_launches_or_fabricates_readiness() {
        let mut owner = owner();
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        owner.skater.as_mut().unwrap().step_results[0].ready = true;
        assert!(matches!(
            owner.try_mid_locomotion_prefix(&refs),
            Ok(MidLocomotionContinuation::Active(None))
        ));
        owner
            .try_mid_locomotion_suffix(&mut HostReckoningNumeric)
            .unwrap();
        assert_eq!(
            owner.before_face.as_ref().unwrap().queried_feet[1],
            V {
                xyz: [3.0, 0.0, 0.0],
                w: None
            }
        );
        assert!(!owner.skater.as_ref().unwrap().step_results[1].ready);
    }
    #[test]
    fn valid_first_result_targets_only_second_future_and_defers_suffix_until_launch() {
        let mut owner = owner();
        let refs = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        let first = &mut owner.skater.as_mut().unwrap().step_results[0];
        first.ready = true;
        first.result.valid = true;
        first.result.position = V {
            xyz: [10.0, 0.0, 0.0],
            w: None,
        };
        let Ok(MidLocomotionContinuation::Active(Some(launch))) =
            owner.try_mid_locomotion_prefix(&refs)
        else {
            panic!("missing second launch");
        };
        assert_eq!((launch.first, launch.count), (1, 1));
        assert_eq!(
            owner.before_face.as_ref().unwrap().queried_feet[1].xyz,
            [0.0; 3]
        );
        assert!(!owner.skater.as_ref().unwrap().step_results[1].ready);
        owner
            .try_mid_locomotion_suffix(&mut HostReckoningNumeric)
            .unwrap();
        assert_eq!(
            owner.before_face.as_ref().unwrap().queried_feet[1],
            V {
                xyz: [12.0, 0.0, 0.0],
                w: None
            }
        );
        assert!(owner.skater.as_ref().unwrap().step_results[0].ready);
    }
    #[test]
    fn suffix_commits_second_query_position_before_zero_distance_assertion() {
        let mut owner = owner();
        let position = V {
            xyz: [4.0, 5.0, 6.0],
            w: None,
        };
        owner.skater.as_mut().unwrap().step_inputs[1].reference = position;
        owner.before_face.as_mut().unwrap().queried_feet[0] = position;
        assert!(matches!(
            owner.try_mid_locomotion_suffix(&mut HostReckoningNumeric),
            Err(MidLocomotionError::CoincidentQueriedFeet)
        ));
        assert_eq!(
            owner.before_face.as_ref().unwrap().queried_feet[1],
            position
        );
    }
}
pub(crate) enum MidLocomotionContinuation {
    Inactive,
    Active(Option<StepLaunchContinuation>),
}
impl<K, W> ActorAnimationConstruction<K, W> {
    /// Retains native writes before Launch. The caller releases this animation
    /// guard, launches the real second query, then invokes the suffix below.
    pub(crate) fn try_mid_locomotion_prefix(
        &mut self,
        cvars: &InputCvarReferences,
    ) -> Result<MidLocomotionContinuation, MidLocomotionError> {
        use MidLocomotionError as E;
        if !self.phase_flag(0x0010_0000).map_err(E::Phase)? {
            return Ok(MidLocomotionContinuation::Inactive);
        }
        let skater = self.skater.as_mut().ok_or(E::Missing("SkaterAnim"))?;
        if !skater.step_results[0].ready {
            return Err(E::FirstFutureNotReady);
        }
        let fields = self
            .before_sprint
            .as_ref()
            .ok_or(E::Missing("locomotion fields"))?;
        let mut tuning = prediction_tuning(cvars);
        skater.step_inputs[1].flags = 0;
        skater
            .trajectory
            .try_foot_prediction(
                fields.foot_down.wrapping_add(1),
                &mut skater.step_inputs[1],
                &mut tuning,
            )
            .map_err(E::Prediction)?;
        if !skater.step_results[0].result.valid {
            return Ok(MidLocomotionContinuation::Active(None));
        }
        // Replace only the reference. The already computed parameter/offset
        // and velocity remain retained, as in the native second-step query.
        skater.step_inputs[1].reference = skater.step_results[0].result.position;
        let turn = self
            .before_face
            .as_ref()
            .ok_or(E::Missing("locomotion suffix"))?
            .turn;
        let maximum = tuning.maximum_spread_degrees().map_err(E::Tuning)?;
        let frames = tuning.failure_frame_count().map_err(E::Tuning)?;
        let minimum = tuning.minimum_spread_degrees().map_err(E::Tuning)?;
        let failures = fields.step_failure_counts[1];
        skater.step_inputs[1].turn_intent = turn;
        // Both native integer traps follow the turn store and all three reads.
        if frames == 0 || (frames == -1 && failures == i32::MIN) {
            return Err(E::IntegerDivision);
        }
        let radians = f32::from_bits(0x3c8e_fa35);
        let maximum = maximum * radians;
        let minimum = minimum * radians;
        skater.step_inputs[1].spread_angle =
            ((failures.wrapping_rem(frames) as f32) * (maximum - minimum)) / (frames as f32)
                + minimum;
        let action = skater
            .trajectory
            .physics_action_probe()
            .map_err(E::Action)?;
        if fields.second_flags_known_mask & 0x4000_0000 == 0 {
            return Err(E::UnknownFlags(0x4000_0000));
        }
        let sprint =
            fields.second_flags_bits & 0x4000_0000 != 0 && matches!(action, 0 | 9 | 10 | 3 | 1);
        let input = &mut skater.step_inputs[1];
        input.flags = (input.flags & !0x10) | (u8::from(sprint) << 4);
        // IsPlayingAction re-reads the physical action after the flag store.
        let jump = skater
            .trajectory
            .physics_action_probe()
            .map_err(E::Action)?
            == 1;
        let player = self.player.as_ref().ok_or(E::Missing("player"))?;
        if player.flags_known_mask & 1 == 0 {
            return Err(E::UnknownFlags(1));
        }
        let world_only = player.flags_bits & 1 != 0;
        let height = tuning.maximum_step_height(jump).map_err(E::Tuning)?;
        input.max_y = height + input.reference.xyz[1];
        Ok(MidLocomotionContinuation::Active(Some(
            StepLaunchContinuation {
                first: 1,
                count: 1,
                world_only,
            },
        )))
    }

    /// 82ca9eb4..9f64 runs after successful Launch, or directly if result0 was
    /// invalid. Publish queried foot1 before the ordered positive-distance assert.
    pub(crate) fn try_mid_locomotion_suffix(
        &mut self,
        numeric: &mut HostReckoningNumeric,
    ) -> Result<(), MidLocomotionError> {
        use MidLocomotionError as E;
        let input = &self
            .skater
            .as_ref()
            .ok_or(E::Missing("SkaterAnim"))?
            .step_inputs[1];
        let suffix = self
            .before_face
            .as_mut()
            .ok_or(E::Missing("locomotion suffix"))?;
        let origin = input.reference.zip(input.reference_offset, |a, b| a + b);
        let position = origin.zip(input.parameter_vector, |a, b| a + b);
        suffix.queried_feet[1] = position;
        let delta = position.zip(suffix.queried_feet[0], |a, b| a - b);
        let [distance] = lengths([delta], numeric).map_err(E::Length)?;
        // Raw41990018 at9f50 is orderedGT, including rejection of NaN.
        if distance.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(E::CoincidentQueriedFeet);
        }
        Ok(())
    }
}
