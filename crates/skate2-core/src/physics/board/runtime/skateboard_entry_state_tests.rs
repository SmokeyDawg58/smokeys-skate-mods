use super::*;
use crate::simulation_drive_pool::SimulationDriveDynamicsParams;

#[test]
fn manual_scale_mutates_only_torque_and_integral_with_ieee_scalar() {
    let mut state = SkateboardEntryState::new();
    state.manual_torque = -0.0;
    state.manual_error_integral = 3.0;
    state.manual_last_target_angle = 4.0;
    state.manual_last_actual_angle = 5.0;
    state.manual_time_since_reset = 6.0;
    state.target_speed = 7.0;
    state.scale_manual_force(-2.0);
    assert_eq!(state.manual_torque.to_bits(), 0);
    assert_eq!(state.manual_error_integral.to_bits(), (-6.0_f32).to_bits());
    assert_eq!(
        [
            state.manual_last_target_angle,
            state.manual_last_actual_angle,
            state.manual_time_since_reset,
            state.target_speed
        ]
        .map(f32::to_bits),
        [4.0_f32, 5.0, 6.0, 7.0].map(f32::to_bits)
    );
    state.scale_manual_force(f32::INFINITY);
    assert!(state.manual_torque.is_nan());
    assert_eq!(
        state.manual_error_integral.to_bits(),
        f32::NEG_INFINITY.to_bits()
    );
}

#[test]
fn manual_reset_preserves_speed_and_animation_and_clears_all_five_scalars() {
    let mut state = SkateboardEntryState::new();
    state.target_speed = -7.0;
    state.animated = true;
    state.manual_error_integral = f32::NAN;
    state.manual_last_target_angle = -0.0;
    state.manual_last_actual_angle = 2.0;
    state.manual_torque = 3.0;
    state.manual_time_since_reset = 4.0;
    state.reset_manual();
    assert_eq!(
        [
            state.manual_error_integral,
            state.manual_last_target_angle,
            state.manual_last_actual_angle,
            state.manual_torque,
            state.manual_time_since_reset
        ]
        .map(f32::to_bits),
        [0; 5]
    );
    assert_eq!(state.target_speed.to_bits(), (-7.0_f32).to_bits());
    assert!(state.animated);
}

struct SpeedSource {
    events: Vec<&'static str>,
    fail_velocity: bool,
}
impl SkateboardSpeedModelSource for SpeedSource {
    type Error = &'static str;
    fn effective_transform(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootMatrix, Self::Error> {
        self.events.push("transform");
        Ok(
            crate::skeleton_root_transforms::SkeletonRootMatrix::from_columns([
                crate::skeleton_root_transforms::SkeletonRootVector {
                    xyz: [0.0; 3],
                    w: None,
                },
                crate::skeleton_root_transforms::SkeletonRootVector {
                    xyz: [0.0; 3],
                    w: None,
                },
                crate::skeleton_root_transforms::SkeletonRootVector {
                    xyz: [0.0, 0.0, -1.0],
                    w: Some(f32::NAN),
                },
                crate::skeleton_root_transforms::SkeletonRootVector {
                    xyz: [0.0; 3],
                    w: None,
                },
            ]),
        )
    }
    fn deck_velocity(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, Self::Error> {
        self.events.push("velocity");
        if self.fail_velocity {
            return Err("velocity");
        }
        Ok(crate::skeleton_root_transforms::SkeletonRootVector {
            xyz: [2.0, 3.0, 4.0],
            w: None,
        })
    }
}

#[test]
fn speed_model_samples_in_order_and_only_commits_after_success() {
    let mut state = SkateboardEntryState::new();
    state.target_speed = 7.0;
    let mut source = SpeedSource {
        events: vec![],
        fail_velocity: true,
    };
    let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
    assert_eq!(
        state.try_reset_speed_model(&mut source, &mut numeric),
        Err(ResetSpeedModelError::Source("velocity"))
    );
    assert_eq!(source.events, ["transform", "velocity"]);
    assert_eq!(state.target_speed().to_bits(), 7.0_f32.to_bits());
    source.fail_velocity = false;
    source.events.clear();
    state
        .try_reset_speed_model(&mut source, &mut numeric)
        .unwrap();
    assert_eq!(source.events, ["transform", "velocity"]);
    assert_eq!(state.target_speed().to_bits(), 4.0_f32.to_bits());
}

fn payload() -> Arc<RwLock<SimulationDriveDynamics>> {
    let params = SimulationDriveDynamicsParams {
        spring: 3.0,
        damping: 4.0,
        strength: 5.0,
        raw_type: 1,
    };
    Arc::new(RwLock::new(SimulationDriveDynamics {
        linear: params,
        angular: params,
    }))
}

#[test]
fn disable_uses_two_captures_and_mutates_the_same_retained_payload() {
    let mut state = SkateboardEntryState::new();
    assert!(!state.animated());
    assert_eq!(state.target_speed().to_bits(), 0);
    state.animated = true;
    state.target_speed = 7.0;
    let hook = payload();
    let observer = Arc::clone(&hook);
    let mut captures = 0;
    state
        .try_disable_board_animation(|| {
            captures += 1;
            if captures == 2 {
                let current = hook.read().unwrap();
                assert_eq!(current.linear.raw_type, 2);
                assert_eq!(current.linear.strength.to_bits(), 0);
                assert_eq!(current.angular.spring.to_bits(), 3.0_f32.to_bits());
            }
            Ok::<_, ()>(Arc::clone(&hook))
        })
        .unwrap();
    assert_eq!(captures, 2);
    assert!(!state.animated());
    assert_eq!(state.target_speed().to_bits(), 7.0_f32.to_bits());
    let current = observer.read().unwrap();
    for p in [&current.linear, &current.angular] {
        assert_eq!(p.raw_type, 2);
        assert_eq!([p.spring, p.damping, p.strength].map(f32::to_bits), [0; 3]);
    }
}

#[test]
fn capture_failures_preserve_distinct_animation_and_linear_prefixes() {
    let mut state = SkateboardEntryState::new();
    state.animated = true;
    assert_eq!(
        state.try_disable_board_animation(|| Err::<Arc<RwLock<SimulationDriveDynamics>>, _>(
            "first"
        )),
        Err(DisableBoardAnimationError::Body("first"))
    );
    assert!(state.animated());
    let hook = payload();
    let mut captures = 0;
    assert_eq!(
        state.try_disable_board_animation(|| {
            captures += 1;
            if captures == 2 {
                Err("second")
            } else {
                Ok(Arc::clone(&hook))
            }
        }),
        Err(DisableBoardAnimationError::Body("second"))
    );
    assert!(!state.animated());
    let current = hook.read().unwrap();
    assert_eq!(current.linear.raw_type, 2);
    assert_eq!(current.angular.raw_type, 1);
    assert_eq!(current.angular.spring.to_bits(), 3.0_f32.to_bits());
}

#[test]
fn poisoned_locks_retain_animated_and_completed_linear_stores() {
    for poison_on in [1, 2] {
        let mut state = SkateboardEntryState::new();
        state.animated = true;
        let hook = payload();
        let mut captures = 0;
        let result = state.try_disable_board_animation(|| {
            captures += 1;
            if captures == poison_on {
                let copy = Arc::clone(&hook);
                let _ = std::thread::spawn(move || {
                    let _guard = copy.write().unwrap();
                    panic!("intentional payload poison");
                })
                .join();
            }
            Ok::<_, ()>(Arc::clone(&hook))
        });
        assert_eq!(
            result,
            Err(if poison_on == 1 {
                DisableBoardAnimationError::LinearPoisoned
            } else {
                DisableBoardAnimationError::AngularPoisoned
            })
        );
        assert!(!state.animated());
        // Test-only raw inspection; production never recovers poisoned locks.
        let current = hook.read().unwrap_err().into_inner();
        assert_eq!(current.linear.raw_type, if poison_on == 1 { 1 } else { 2 });
        assert_eq!(current.angular.raw_type, 1);
    }
}

#[test]
fn update3_abort_clear_does_not_claim_constructor_initialization() {
    let mut state = super::SkateboardEntryState::new();
    assert_eq!(state.abort_push(), Some(false));
    state.clear_abort_push();
    assert_eq!(state.abort_push(), Some(false));
}
