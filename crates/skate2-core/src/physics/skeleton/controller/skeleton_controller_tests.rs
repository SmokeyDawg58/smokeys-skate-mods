use super::*;

// Explicit orchestration probe. Actual normal-state mutation/publication is
// supplied by the physical-pair bridge, not established by this recorder.
#[derive(Default)]
struct Collision {
    calls: Vec<&'static str>,
    fail: bool,
}
impl SkeletonControllerCollision for Collision {
    type Error = &'static str;
    fn set_ground_collision(&mut self) -> Result<(), Self::Error> {
        self.calls.push("ground");
        if self.fail { Err("collision") } else { Ok(()) }
    }
    fn collide_with_world_only(&mut self) -> Result<(), Self::Error> {
        self.calls.push("world-only");
        if self.fail { Err("collision") } else { Ok(()) }
    }
}
fn owner() -> SkeletonController {
    let mut value = SkeletonController::new();
    value.try_initialize_constructor::<&str>().unwrap();
    value
}

#[test]
fn air_init_publishes_need_air_before_a_failed_mode_effect() {
    let mut controller = owner();
    let mut collision = Collision::default();
    assert_eq!(
        controller.try_init_air_collision(&mut collision),
        Err(SkeletonControllerError::PendingCollisionMode(7))
    );
    assert_eq!(controller.current().unwrap().value, 7);
    assert!(controller.need_air_collision().unwrap().value);
}

#[test]
fn explicit_constructor_origins_and_native_startup_ground_branch() {
    let mut value = SkeletonController::new();
    assert_eq!(value.current(), None);
    assert_eq!(
        value.try_set_collision_state(6, &mut Collision::default()),
        Err(SkeletonControllerError::Unconstructed)
    );
    value.try_initialize_constructor::<&str>().unwrap();
    assert_eq!(
        value.current(),
        Some(SkeletonControllerValue::constructor(0))
    );
    assert_eq!(
        value.stored(),
        Some(SkeletonControllerValue::constructor(0))
    );
    assert_eq!(
        value.stored_valid(),
        Some(SkeletonControllerValue::constructor(false))
    );
    assert_eq!(
        value.world_only(),
        Some(SkeletonControllerValue::constructor(false))
    );
    assert_eq!(
        value.need_air_collision(),
        Some(SkeletonControllerValue::constructor(false))
    );
    let mut collision = Collision::default();
    value.try_set_collision_state(6, &mut collision).unwrap();
    assert_eq!(collision.calls, ["ground"]);
    assert_eq!(
        value.current(),
        Some(SkeletonControllerValue::transition(6))
    );
    assert_eq!(
        value.try_initialize_constructor::<&str>(),
        Err(SkeletonControllerError::AlreadyConstructed)
    );
}

#[test]
fn failed_ground_and_world_only_entries_retain_new_state_and_source_flag_prefixes() {
    let mut value = owner();
    let mut collision = Collision {
        calls: vec![],
        fail: true,
    };
    assert_eq!(
        value.try_set_collision_state(6, &mut collision),
        Err(SkeletonControllerError::Collision("collision"))
    );
    assert_eq!(value.current().unwrap().value, 6);
    assert_eq!(
        value.try_start_world_only(&mut collision),
        Err(SkeletonControllerError::Collision("collision"))
    );
    assert_eq!(value.current().unwrap().value, 2);
    assert_eq!(value.stored().unwrap().value, 6);
    assert!(value.stored_valid().unwrap().value);
    assert!(!value.world_only().unwrap().value);
    assert_eq!(
        value.try_set_collision_state(6, &mut collision),
        Err(SkeletonControllerError::CurrentWorldOnlyState(2))
    );
    assert_eq!(collision.calls, ["ground", "world-only"]);
}

#[test]
fn world_only_requested_state_is_stored_before_validation_and_maps_standard() {
    let mut value = owner();
    let mut collision = Collision::default();
    value.try_set_collision_state(6, &mut collision).unwrap();
    value.try_start_world_only(&mut collision).unwrap();
    value.try_set_collision_state(6, &mut collision).unwrap();
    assert_eq!(value.current().unwrap().value, 2);
    assert_eq!(value.stored().unwrap().value, 6);
    assert_eq!(
        value.try_set_collision_state(1, &mut collision),
        Err(SkeletonControllerError::InvalidWorldOnlyRequest(1))
    );
    assert_eq!(value.current().unwrap().value, 2);
    assert_eq!(value.stored().unwrap().value, 1);
    assert!(value.stored_valid().unwrap().value);
    assert_eq!(
        value.try_set_collision_state(8, &mut collision),
        Err(SkeletonControllerError::PendingCollisionMode(8))
    );
    assert_eq!(value.current().unwrap().value, 3);
    assert_eq!(value.stored().unwrap().value, 8);
    assert_eq!(collision.calls, ["ground", "world-only", "world-only"]);
}

#[test]
fn stop_clears_world_only_before_enter_but_keeps_stored_valid_on_failure() {
    let mut value = owner();
    let mut collision = Collision::default();
    value.try_set_collision_state(6, &mut collision).unwrap();
    value.try_start_world_only(&mut collision).unwrap();
    collision.fail = true;
    assert_eq!(
        value.try_stop_world_only(&mut collision),
        Err(SkeletonControllerError::Collision("collision"))
    );
    assert!(!value.world_only().unwrap().value);
    assert!(value.stored_valid().unwrap().value);
    assert_eq!(value.current().unwrap().value, 6);
    collision.fail = false;
    value.try_stop_world_only(&mut collision).unwrap();
    assert!(!value.stored_valid().unwrap().value);
    let mut fresh = owner();
    fresh.try_stop_world_only(&mut collision).unwrap();
    assert_eq!(
        fresh.stored_valid(),
        Some(SkeletonControllerValue::constructor(false))
    );
    assert_eq!(fresh.current().unwrap().value, 6);
}

#[test]
fn unrecovered_modes_refuse_after_state_publication_and_state_three_is_not_guarded() {
    let mut value = owner();
    let mut collision = Collision::default();
    assert_eq!(
        value.try_set_collision_state(4, &mut collision),
        Err(SkeletonControllerError::PendingCollisionMode(4))
    );
    assert_eq!(
        value.try_start_world_only(&mut collision),
        Err(SkeletonControllerError::PendingCollisionMode(1))
    );
    assert_eq!(value.stored().unwrap().value, 4);
    assert_eq!(value.current().unwrap().value, 1);
    assert!(!value.world_only().unwrap().value);
    let mut value = owner();
    assert_eq!(
        value.try_set_collision_state(3, &mut collision),
        Err(SkeletonControllerError::PendingCollisionMode(8))
    );
    value.try_set_collision_state(6, &mut collision).unwrap();
    assert_eq!(collision.calls, ["ground"]);
}

// Unlike the basic probe above, this supplies BOTH actual mode3 dependencies.
struct RagdollWorldProbe {
    calls: Vec<&'static str>,
    fail: Option<&'static str>,
}
impl SkeletonControllerCollision for RagdollWorldProbe {
    type Error = &'static str;
    fn set_ground_collision(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn set_ragdoll_collision(&mut self) -> Result<(), SkeletonControllerError<Self::Error>> {
        self.calls.push("ragdoll");
        if self.fail == Some("ragdoll") {
            Err(SkeletonControllerError::Collision("ragdoll"))
        } else {
            Ok(())
        }
    }
    fn collide_with_world_only(&mut self) -> Result<(), Self::Error> {
        self.calls.push("world-only");
        if self.fail == Some("world-only") {
            Err("world-only")
        } else {
            Ok(())
        }
    }
}

#[test]
fn mode_three_preserves_current_and_orders_both_failure_prefixes() {
    for failure in [Some("ragdoll"), Some("world-only"), None] {
        let mut controller = owner();
        let mut collision = RagdollWorldProbe {
            calls: vec![],
            fail: failure,
        };
        let result = controller.try_set_collision_state(3, &mut collision);
        assert_eq!(
            controller.current(),
            Some(SkeletonControllerValue::transition(3))
        );
        assert_eq!(
            result,
            failure.map_or(Ok(()), |e| Err(SkeletonControllerError::Collision(e)))
        );
        assert_eq!(
            collision.calls,
            if failure == Some("ragdoll") {
                vec!["ragdoll"]
            } else {
                vec!["ragdoll", "world-only"]
            }
        );
        assert!(!controller.world_only().unwrap().value);
    }
}

#[test]
fn actual_wipeout_request_eight_under_world_only_enters_three_and_retains_stored_eight() {
    for failure in [Some("ragdoll"), Some("world-only"), None] {
        let mut controller = owner();
        let mut collision = RagdollWorldProbe {
            calls: vec![],
            fail: None,
        };
        controller
            .try_set_collision_state(6, &mut collision)
            .unwrap();
        controller.try_start_world_only(&mut collision).unwrap();
        assert_eq!(controller.current().unwrap().value, 2);
        collision.calls.clear();
        collision.fail = failure;
        let result = controller.try_set_collision_state(8, &mut collision);
        assert_eq!(
            result,
            failure.map_or(Ok(()), |e| Err(SkeletonControllerError::Collision(e)))
        );
        assert_eq!(
            controller.current(),
            Some(SkeletonControllerValue::transition(3))
        );
        assert_eq!(
            controller.stored(),
            Some(SkeletonControllerValue::transition(8))
        );
        assert!(controller.stored_valid().unwrap().value);
        assert!(controller.world_only().unwrap().value);
        assert_eq!(
            collision.calls,
            if failure == Some("ragdoll") {
                vec!["ragdoll"]
            } else {
                vec!["ragdoll", "world-only"]
            }
        );
    }
}
