//! StepSelector::CalculateResults82d820a0, after its validity-clear and Sync.
use crate::{
    actor_animation_construction::AnimationStepInput, collision_query_batches::step::StepResult,
    skeleton_root_transforms::SkeletonRootVector as V, walking_step_query::StepTrajectoryResult,
    walking_step_selection::StepSelectionResult,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StepSelectionFuture {
    pub(crate) result: StepSelectionResult,
    pub(crate) ready: bool,
}
impl StepSelectionFuture {
    pub(crate) const fn constructed() -> Self {
        Self {
            result: StepSelectionResult {
                trajectory_contact: V::known([0.0; 4]),
                position: V::known([0.0; 4]),
                normal: V::known([0.0; 4]),
                surface: 0,
                valid: false,
            },
            ready: false,
        }
    }
}
pub(crate) trait StepResultServices {
    type Error;
    fn get_step_position(
        &mut self,
        input: &AnimationStepInput,
        candidate: &StepTrajectoryResult,
        result: &mut StepSelectionResult,
    ) -> Result<bool, Self::Error>;
    fn death_jump_distance(&mut self) -> Result<f32, Self::Error>;
    fn maximum_time(&mut self) -> Result<f32, Self::Error>;
}

/// Caller clears all future.valid fields before the real batch.Sync/GetResult.
/// None corresponds to native null GetResult: no ready stores and no Release.
/// Returned true tells that caller to execute Release, even for zero tests.
pub(crate) fn calculate_results<S: StepResultServices>(
    inputs: &[AnimationStepInput],
    futures: &mut [StepSelectionFuture],
    batch: Option<&StepResult>,
    services: &mut S,
) -> Result<bool, S::Error> {
    let Some(batch) = batch else {
        return Ok(false);
    };
    assert!(inputs.len() <= batch.len() && inputs.len() <= futures.len());
    for (index, input) in inputs.iter().enumerate() {
        let future = &mut futures[index];
        let origin = input.reference.zip(input.reference_offset, |a, b| a + b);
        let candidates = &batch[index];
        let mut selected = false;
        for candidate in candidates {
            if candidate.priority != -1
                && services.get_step_position(input, candidate, &mut future.result)?
            {
                selected = true;
                break;
            }
        }
        if !selected {
            let mut had_collision = false;
            let mut death_jump = false;
            for candidate in candidates {
                if candidate.collision.time[0] >= 0.0 {
                    // Native writes +0 only here. Other result fields retain
                    // prior contents unless the following branch overwrites.
                    future.result.trajectory_contact = candidate.collision.position;
                    had_collision = true;
                    if origin.xyz[1] - candidate.collision.position.xyz[1]
                        > services.death_jump_distance()?
                    {
                        death_jump = true;
                        break;
                    }
                }
            }
            if death_jump {
                future.result.position = future.result.trajectory_contact;
                future.result.normal = V::known([0.0, 1.0, 0.0, 0.0]);
                // Source deliberately retains the old surface word.
                future.result.valid = true;
            } else if !had_collision {
                // 82d823d4 publishes validity before sampling maximum time.
                future.result.valid = true;
                let time = services.maximum_time()?;
                let time_squared = time * time;
                let destination = origin.zip(input.parameter_vector, |a, b| a + b);
                let velocity = input.speed_at_end;
                let first = velocity.zip(destination, |v, p| v.mul_add(time, p));
                // Math::VECTOR3_GRAVITY82f76360, distinct from worker -9.81.
                let half_gravity =
                    V::known([0.0, f32::from_bits(0xc11c_cccd), 0.0, 0.0]).map(|g| g * (1.0 * 0.5));
                let position = half_gravity.zip(first, |g, p| g.mul_add(time_squared, p));
                future.result.position = position;
                future.result.trajectory_contact = position;
                future.result.normal = V::known([0.0, 1.0, 0.0, 0.0]);
                future.result.surface = 0;
            }
        }
        future.ready = true;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Services {
        steps: usize,
        death: f32,
        fail_time: bool,
    }
    impl StepResultServices for Services {
        type Error = &'static str;
        fn get_step_position(
            &mut self,
            _: &AnimationStepInput,
            _: &StepTrajectoryResult,
            _: &mut StepSelectionResult,
        ) -> Result<bool, Self::Error> {
            self.steps += 1;
            Ok(false)
        }
        fn death_jump_distance(&mut self) -> Result<f32, Self::Error> {
            Ok(self.death)
        }
        fn maximum_time(&mut self) -> Result<f32, Self::Error> {
            if self.fail_time { Err("time") } else { Ok(2.0) }
        }
    }
    fn input() -> AnimationStepInput {
        AnimationStepInput {
            reference: V::known([0.0; 4]),
            reference_offset: V::known([0.0; 4]),
            parameter_vector: V::known([0.0; 4]),
            speed_at_end: V::known([1.0, 0.0, 0.0, 0.0]),
            parameter: 0.0,
            spread_angle: 0.0,
            turn_intent: 0.0,
            max_y: 0.0,
            flags: 0,
        }
    }
    #[test]
    fn shallow_collision_publishes_only_contact_and_ready() {
        let mut rows = [[StepTrajectoryResult::new(); 4]; 5];
        rows[0][0].collision.time = [0.0; 4];
        rows[0][0].collision.position = V {
            xyz: [1.0, -0.5, 2.0],
            w: None,
        };
        let mut future = StepSelectionFuture::constructed();
        future.result.surface = 17;
        future.result.position = V::known([9.0; 4]);
        let mut futures = [future];
        let mut services = Services {
            steps: 0,
            death: 2.0,
            fail_time: false,
        };
        assert!(calculate_results(&[input()], &mut futures, Some(&rows), &mut services).unwrap());
        assert!(futures[0].ready);
        assert!(!futures[0].result.valid);
        assert_eq!(futures[0].result.position, future.result.position);
        assert_eq!(futures[0].result.surface, 17);
        assert_eq!(futures[0].result.trajectory_contact.w, None);
        assert_eq!(services.steps, 0);
    }
    #[test]
    fn deep_collision_preserves_surface_and_no_collision_uses_math_gravity() {
        let mut rows = [[StepTrajectoryResult::new(); 4]; 5];
        rows[0][0].collision.time = [0.0; 4];
        rows[0][0].collision.position = V::known([0.0, -3.0, 0.0, 7.0]);
        let mut futures = [StepSelectionFuture::constructed(); 2];
        futures[0].result.surface = 42;
        let mut services = Services {
            steps: 0,
            death: 2.0,
            fail_time: false,
        };
        calculate_results(
            &[input(), input()],
            &mut futures,
            Some(&rows),
            &mut services,
        )
        .unwrap();
        assert!(futures.iter().all(|f| f.ready && f.result.valid));
        assert_eq!(futures[0].result.surface, 42);
        assert_eq!(futures[0].result.position.w, Some(7.0));
        assert_eq!(futures[1].result.position.xyz, [2.0, -19.6, 0.0]);
    }
    #[test]
    fn missing_batch_and_failed_tuning_do_not_publish_readiness() {
        let mut futures = [StepSelectionFuture::constructed()];
        let mut services = Services {
            steps: 0,
            death: 2.0,
            fail_time: true,
        };
        assert!(!calculate_results(&[input()], &mut futures, None, &mut services).unwrap());
        assert!(!futures[0].ready);
        let rows = [[StepTrajectoryResult::new(); 4]; 5];
        assert_eq!(
            calculate_results(&[input()], &mut futures, Some(&rows), &mut services),
            Err("time")
        );
        assert!(futures[0].result.valid);
        assert!(!futures[0].ready);
    }
}
