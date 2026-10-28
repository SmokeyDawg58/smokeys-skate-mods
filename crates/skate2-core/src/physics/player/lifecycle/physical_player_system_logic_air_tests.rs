use crate::physics_state_ids::{
    PHYSICS_STATE_GRIND_5050, PHYSICS_STATE_KNOWN_AIR, PHYSICS_STATE_OFFBOARD_AIR,
    PHYSICS_STATE_PHYSICS_AIR, PHYSICS_STATE_WIPEOUT_GROUND,
};
use super::*;

struct AirCvars {
    minimum: Result<f32, &'static str>,
    frames: Result<i32, &'static str>,
    calls: Vec<&'static str>,
}
impl StateChangerCvars for AirCvars {
        fn known_air_warning_time(&mut self) -> Result<f32, Self::Error> { Ok(0.04) }
        fn state_debug_warnings(&mut self) -> Result<bool, Self::Error> { Ok(false) }
        fn nonspecific_frames(&mut self, _: NonspecificFrames) -> Result<i32, Self::Error> {
            panic!("Air has no Nonspecific query")
        }
    type Error = &'static str;
    fn fall_through_world_reset_frames(&mut self) -> Result<i32, Self::Error> {
        Ok(400)
    }
    fn disable_skitching(&mut self) -> Result<bool, Self::Error> {
        panic!("Air has no skitch query")
    }
    fn minimum_air_time(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("minimum");
        self.minimum
    }
    fn grind_jump_air_frames(&mut self) -> Result<i32, Self::Error> {
        self.calls.push("frames");
        self.frames
    }
}
fn fixture() -> (SkeletonProcessedContact, StateChangerFields, AirCvars) {
    let mut source = SkeletonProcessedContact::new();
    source.try_initialize_constructor_fields().unwrap();
    (
        source,
        StateChangerFields::constructed(),
        AirCvars {
            minimum: Ok(0.1),
            frames: Ok(10),
            calls: Vec::new(),
        },
    )
}

#[test]
fn offboard_request_precedes_special_air_collision_and_timer() {
    let (mut source, fields, mut cvars) = fixture();
    source.physics_state_fields_mut().flags_8cc = Some(0x100);
    source.physics_state_fields_mut().state_timer = None;
    assert_eq!(
        select_air(&source, &fields, true, 0x100, &mut cvars).unwrap(),
        Selection::State(PHYSICS_STATE_OFFBOARD_AIR)
    );
    assert!(cvars.calls.is_empty());
}

#[test]
fn special_air_mode_collision_precedes_known_air_and_never_reads_cvars() {
    let (mut source, fields, mut cvars) = fixture();
    source.physics_state_fields_mut().state_timer = None;
    source.publish_trajectory_selector_ready(true);
    assert_eq!(
        select_air(&source, &fields, true, 0x100, &mut cvars).unwrap(),
        Selection::State(PHYSICS_STATE_WIPEOUT_GROUND)
    );
    assert_eq!(
        select_air(&source, &fields, false, 0x100, &mut cvars).unwrap(),
        Selection::State(PHYSICS_STATE_KNOWN_AIR)
    );
    assert!(cvars.calls.is_empty());
}

#[test]
fn timer_equal_or_unordered_skips_collision_and_grind_queries() {
    for (timer, minimum) in [(0.1, 0.1), (f32::NAN, 0.1), (0.2, f32::NAN)] {
        let (mut source, fields, mut cvars) = fixture();
        source.physics_state_fields_mut().state_timer = Some(timer);
        source.physics_state_fields_mut().grind_possible = None;
        cvars.minimum = Ok(minimum);
        source.publish_trajectory_selector_ready(true);
        assert_eq!(
            select_air(&source, &fields, true, 0, &mut cvars).unwrap(),
            Selection::State(PHYSICS_STATE_KNOWN_AIR)
        );
        assert_eq!(cvars.calls, ["minimum"]);
    }
}

#[test]
fn timer_failure_is_before_cvar_and_mature_collision_requests_actual_animation_query() {
    let (mut source, fields, mut cvars) = fixture();
    source.physics_state_fields_mut().state_timer = None;
    assert!(matches!(
        select_air(&source, &fields, true, 0, &mut cvars),
        Err(SystemLogicError::Missing("Processed state timer"))
    ));
    assert!(cvars.calls.is_empty());
    source.physics_state_fields_mut().state_timer = Some(0.2);
    assert_eq!(
        select_air(&source, &fields, true, 0, &mut cvars).unwrap(),
        Selection::AirLanding
    );
    assert_eq!(cvars.calls, ["minimum"]);
}

#[test]
fn grind_threshold_is_strict_and_selects_the_investigated_grind() {
    let (mut source, mut fields, mut cvars) = fixture();
    source.physics_state_fields_mut().state_timer = Some(0.2);
    source.physics_state_fields_mut().grind_possible = Some(true);
    fields.post_grind_jump_counter = 10;
    assert_eq!(
        select_air(&source, &fields, false, 0, &mut cvars).unwrap(),
        Selection::State(PHYSICS_STATE_PHYSICS_AIR)
    );
    fields.post_grind_jump_counter = 11;
    // The constructed investigation's grind type is zero: 5050, state 401.
    assert_eq!(
        select_air(&source, &fields, false, 0, &mut cvars).unwrap(),
        Selection::State(PHYSICS_STATE_GRIND_5050)
    );
    source.physics_state_fields_mut().flags_8d0 = Some(0x80000);
    assert_eq!(
        select_air(&source, &fields, false, 0, &mut cvars).unwrap(),
        Selection::State(PHYSICS_STATE_PHYSICS_AIR)
    );
}

#[test]
fn air_cvar_failure_retains_common_counter_prefix() {
    let (source, mut fields, mut cvars) = fixture();
    fields.begin(200);
    cvars.minimum = Err("minimum target unavailable");
    assert!(matches!(
        select_state(&mut fields, PHYSICS_STATE_PHYSICS_AIR, 3, &source, &mut cvars),
        Err(SystemLogicError::Cvar("minimum target unavailable"))
    ));
    assert_eq!(fields.post_grind_jump_counter, 1);
    assert_eq!(fields.three_wheel_counter, 1);
    assert!(!fields.request_teleport);
    assert_eq!(cvars.calls, ["minimum"]);
}
