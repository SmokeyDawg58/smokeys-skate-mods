use super::*;
use crate::actor_animation_construction::{
    ActorAnimationCvars, ActorAnimationInputs, ActorAnimationInterface, ActorAnimationRegistry,
    ActorGraphAuthority, AnimationLifetime, MotionGraphEntityAuthority,
};
use crate::input_cvar_runtime::InputCvarManager;
use crate::play_animation::FastString30;
use crate::skeleton_root_transforms::SkeletonRootVector as WalkVector;
use std::{convert::Infallible, sync::Arc};

struct Registry;
impl ActorAnimationRegistry<u32> for Registry {
    type Error = Infallible;
    fn register_animation_interface(
        &mut self,
        _: &MotionGraphEntityAuthority<u32>,
        _: ActorAnimationInterface,
        _: &Arc<()>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn actor_player_index(
        &mut self,
        _: &MotionGraphEntityAuthority<u32>,
    ) -> Result<u32, Self::Error> {
        Ok(0)
    }
}
struct Sprint;
impl ActorAnimationCvars for Sprint {
    type Error = Infallible;
    fn use_sprint_button(&mut self) -> Result<u8, Self::Error> {
        Ok(0)
    }
}
fn animation(sim: bool) -> ActorAnimationConstruction<u32, ()> {
    let authority = ActorGraphAuthority::new(0);
    let mut result = ActorAnimationConstruction::new(
        &authority,
        ActorAnimationInputs {
            physics_world: Arc::new(()),
            life: AnimationLifetime::Transient,
            use_low_blend_cull_threshold: false,
            face_name: None,
        },
    );
    if sim {
        result.try_mark_sim_allocation().unwrap();
    }
    result.try_construct_prefix(&mut Registry).unwrap();
    let vault = crate::attribute_runtime::tests::graph(
        crate::actor_animation_construction::ACTOR_ANIMATION_OFFBOARD_CLASS,
        Vec::new(),
    );
    result
        .try_construct_vault_suffix(Arc::new(vault), &mut Sprint)
        .unwrap();
    result
}
fn output() -> AnimOutPhysInOwner {
    let mut result = AnimOutPhysInOwner::new_sim_allocation();
    result.try_construct(0).unwrap();
    result
}

#[test]
fn locomotion_entry_missing_inputs_retains_resets_before_world_assertion() {
    use crate::actor_animation_construction::locomotion_lifecycle::LocomotionLifecycleError;
    let mut anim = animation(true);
    anim.before_sprint.as_mut().unwrap().speed = 12.0;
    anim.before_sprint.as_mut().unwrap().step_failure_counts = [5, 7];
    let trajectory = &mut anim.skater.as_mut().unwrap().trajectory;
    trajectory.time = 9.0;
    trajectory.physical_feet[0].initialized = true;
    trajectory.physical_feet[0].trailing_action_probe_word = Some(91);
    assert!(matches!(
        anim.try_set_is_in_locomotion(true, None, None),
        Err(LocomotionLifecycleError::Missing(
            "enter locomotion world assertion"
        ))
    ));
    let trajectory = &anim.skater.as_ref().unwrap().trajectory;
    assert_eq!(trajectory.current_state, -1);
    assert_eq!(trajectory.time, 0.0);
    assert!(!trajectory.physical_feet[0].initialized);
    assert_eq!(
        trajectory.physical_feet[0].trailing_action_probe_word,
        Some(91)
    );
    assert_eq!(
        anim.before_sprint.as_ref().unwrap().step_failure_counts,
        [0, 0]
    );
    assert_eq!(anim.before_sprint.as_ref().unwrap().speed, 12.0);
    assert_eq!(
        anim.player.as_ref().unwrap().flags_bits & 0x0014_0000,
        0x0014_0000
    );
}

#[test]
fn locomotion_entry_preserves_world_w_and_uses_mirrored_local_forward_speed() {
    use crate::skeleton_root_transforms::SkeletonRootMatrix as M;
    let mut anim = animation(true);
    let mut columns = *anim.before_face.as_ref().unwrap().world_position.columns();
    columns[2] = WalkVector {
        xyz: [1.1, 2.2, 3.3],
        w: None,
    };
    let world = M::from_columns(columns);
    let speed = WalkVector {
        xyz: [4.4, 5.5, 6.6],
        w: None,
    };
    let local = 3.3_f32.mul_add(6.6, 2.2_f32.mul_add(5.5, 1.1_f32 * 4.4));
    anim.player.as_mut().unwrap().flags_known_mask |= 0x4000_0000;
    anim.player.as_mut().unwrap().flags_bits |= 0x4000_0000;
    anim.try_set_is_in_locomotion(true, Some(world), Some(speed))
        .unwrap();
    assert_eq!(
        anim.before_face.as_ref().unwrap().world_position.columns()[2].w,
        None
    );
    assert_eq!(
        anim.before_sprint.as_ref().unwrap().speed.to_bits(),
        (local * f32::from_bits(0xbe73_cf3e)).to_bits()
    );
}

#[test]
fn locomotion_exit_removes_only_source_keys_and_preserves_trajectory_and_feet() {
    let mut anim = animation(true);
    for name in ["StartSequenceID", "StartSelected", "Retained"] {
        anim.motion_set_construction_attribute(
            FastString30::encode(name).unwrap(),
            FastString30::encode("Value").unwrap(),
        )
        .unwrap();
    }
    anim.skater.as_mut().unwrap().trajectory.current_state = 2;
    anim.before_face.as_mut().unwrap().feet[0].w = None;
    anim.before_face
        .as_mut()
        .unwrap()
        .time_since_leaving_locomotion = 7.0;
    anim.before_sprint.as_mut().unwrap().speed = 5.0;
    anim.try_set_is_in_locomotion(false, None, None).unwrap();
    assert_eq!(anim.skater.as_ref().unwrap().trajectory.current_state, 2);
    assert_eq!(anim.before_face.as_ref().unwrap().feet[0].w, None);
    assert_eq!(
        anim.before_face
            .as_ref()
            .unwrap()
            .time_since_leaving_locomotion,
        0.0
    );
    assert_eq!(anim.before_sprint.as_ref().unwrap().speed, 0.0);
    let map = &anim.base.as_ref().unwrap().construction_map;
    assert_eq!(map.len, 1);
    assert_eq!(
        map.slots[0].as_ref().unwrap().0,
        FastString30::encode("Retained").unwrap()
    );
}

#[test]
fn inactive_probe_uses_preceding_foot_padding_and_reset_preserves_its_origin() {
    let mut generic = animation(false);
    assert!(matches!(
        generic.physics_action_probe(),
        Err(AnimationPhysicsDataError::UnwrittenInactiveActionProbe)
    ));
    let trajectory = &mut generic.skater.as_mut().unwrap().trajectory;
    trajectory.input_feet[3].trailing_action_probe_word = Some(91);
    for (index, foot) in trajectory.physical_feet.iter_mut().enumerate() {
        foot.trailing_action_probe_word = Some(100 + u32::try_from(index).unwrap());
        foot.states[0].action = 1;
    }
    for (index, expected) in [
        (0, 91),
        (1, 100),
        (2, 101),
        (3, 102),
        (4, 91),
        (-1, 102),
        (-4, 91),
    ] {
        trajectory.physical_foot_index = index;
        assert_eq!(trajectory.physics_action_probe().unwrap(), expected);
    }
    trajectory.physical_foot_index = -5;
    assert!(matches!(
        trajectory.physics_action_probe(),
        Err(AnimationPhysicsDataError::FootIndex(-5))
    ));
    trajectory.physical_foot_index = 0;
    trajectory.input_feet[3].reset();
    assert_eq!(trajectory.physics_action_probe().unwrap(), 91);
    trajectory.current_state = 0;
    assert_eq!(trajectory.physics_action_probe().unwrap(), 1);
    let sim = animation(true);
    assert_eq!(sim.physics_action_probe().unwrap(), 0);
    let trajectory = &sim.skater.as_ref().unwrap().trajectory;
    assert_eq!(trajectory.current_state, -1);
    for foot in trajectory
        .input_feet
        .iter()
        .chain(&trajectory.physical_feet)
    {
        assert_eq!(foot.trailing_action_probe_word, Some(0));
    }
}

#[test]
fn is_landing_reads_flag_then_forced_land_action_codes() {
    let mut anim = animation(true);
    let player = anim.player.as_mut().unwrap();
    player.flags_known_mask |= 0x8000;
    player.flags_bits &= !0x8000;
    anim.skater.as_mut().unwrap().trajectory.current_state = 0;
    anim.skater.as_mut().unwrap().trajectory.physical_foot_index = 0;
    for (action, expected) in [(0, false), (13, true), (14, true), (12, false)] {
        anim.skater.as_mut().unwrap().trajectory.physical_feet[0].states[0].action = action;
        assert_eq!(anim.try_is_landing().unwrap(), expected);
    }
    anim.player.as_mut().unwrap().flags_bits |= 0x8000;
    assert!(anim.try_is_landing().unwrap());
}

#[test]
fn motion_running_out_queries_front_then_back() {
    let mut anim = animation(true);
    let player = anim.player.as_mut().unwrap();
    player.flags_known_mask |= 0x6000;
    player.flags_bits &= !0x6000;
    anim.skater.as_mut().unwrap().trajectory.current_state = 0;
    anim.skater.as_mut().unwrap().trajectory.physical_foot_index = 0;
    for (action, expected) in [(0, false), (15, true), (16, true), (14, false)] {
        anim.skater.as_mut().unwrap().trajectory.physical_feet[0].states[0].action = action;
        assert_eq!(anim.try_is_running_out().unwrap(), expected);
    }
    anim.player.as_mut().unwrap().flags_bits |= 0x4000;
    assert!(anim.try_is_running_out().unwrap());
}

#[test]
fn prefix_failure_keeps_time_ids_request_clear_and_does_not_fake_pose_output() {
    let cvars = InputCvarReferences::new_static_initialized();
    let mut anim = animation(false);
    let mut output = output();
    output.locomotion_flags = u32::MAX;
    output.foot_surface_ids = [4, 5];
    let result = anim.try_get_non_locomotion_physics_update(&mut output, -0.0, &cvars);
    assert!(matches!(
        result,
        Err(AnimationPhysicsDataError::UnwrittenInactiveActionProbe)
    ));
    assert_eq!(output.time_delta.to_bits(), (-0.0_f32).to_bits());
    assert_eq!(output.foot_surface_ids, [0; 2]);
    assert_eq!(output.locomotion_flags, !REQUEST_WIPEOUT);
    let mut anim = animation(true);
    anim.before_face.as_mut().unwrap().last_centre_of_mass = WalkVector::known([5.0; 4]);
    let result = anim.try_get_non_locomotion_physics_update(&mut output, 0.25, &cvars);
    assert!(matches!(
        result,
        Err(AnimationPhysicsDataError::Missing(
            "initialized body buffers"
        ))
    ));
    assert_eq!(
        anim.before_face
            .as_ref()
            .unwrap()
            .last_centre_of_mass
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0; 4]
    );
    assert_eq!(output.locomotion_flags & 0x0e00_0000, 0);
}

#[test]
fn suffix_uses_original_flags_repeated_queries_and_fixed_timer_delta() {
    let mut anim = animation(true);
    let player = anim.player.as_mut().unwrap();
    player.flags_bits = 0x8000_0000 | 0x2000_0000 | 0x0010_0000 | 0x0008_0000 | 0x400;
    player.relative_stance = 1;
    player.natural_stance = 0;
    anim.before_sprint
        .as_mut()
        .unwrap()
        .air_dismount_revert_frames = 19;
    anim.before_face
        .as_mut()
        .unwrap()
        .time_since_leaving_locomotion = 2.0;
    let mut output = output();
    output.locomotion_flags = 0x0000_4321;
    anim.publish_physics_state_flags(&mut output).unwrap();
    assert!(output.flags.board_flipped && !output.flags.mirrored && output.flags.riding_switch);
    assert!(
        output.flags.riding_fakie && output.flags.weight_forwards && output.flags.regular_stance
    );
    assert_eq!(output.locomotion_flags, 0xd000_4321);
    assert_eq!(output.air_dismount_revert_frames, 19);
    assert_eq!(anim.player.as_ref().unwrap().flags_bits & 0x400, 0);
    assert_eq!(
        anim.before_face
            .as_ref()
            .unwrap()
            .time_since_leaving_locomotion
            .to_bits(),
        (2.0 + f32::from_bits(0x3c88_8889)).to_bits()
    );
}

#[test]
fn nose_weight_lifecycle_flag_reaches_physics_with_fakie_inversion() {
    let mut anim = animation(true);
    let mut output = output();
    for fakie in [false, true] {
        anim.player.as_mut().unwrap().flags_bits = if fakie { 0x2000_0000 } else { 0 };
        for nose in [true, false] {
            let before = anim.player.as_ref().unwrap().flags_bits;
            anim.try_set_weight_on_nose(nose).unwrap();
            assert_eq!(
                anim.player.as_ref().unwrap().flags_bits & !0x1000_0000,
                before & !0x1000_0000
            );
            anim.publish_physics_state_flags(&mut output).unwrap();
            assert_eq!(output.flags.weight_forwards, nose != fakie);
        }
    }
}

#[test]
fn non_air_wipeout_reads_live_thresholds_and_preserves_prior_request() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let mut anim = animation(true);
    let mut output = output();
    anim.publish_air_wipeout_flags(&mut output, &refs).unwrap();
    assert_eq!(output.locomotion_flags & REQUEST_WIPEOUT, 0);
    manager
        .try_deserialize_startup(Some(
            b"[Config]\nkMinTimeInAirToWipeOut=-1\nkTimeToLandToWipeout=-2\n",
        ))
        .unwrap();
    anim.publish_air_wipeout_flags(&mut output, &refs).unwrap();
    assert_eq!(output.locomotion_flags & REQUEST_WIPEOUT, REQUEST_WIPEOUT);
    anim.player.as_mut().unwrap().flags_bits |= 0x200;
    anim.publish_air_wipeout_flags(&mut output, &refs).unwrap();
    assert_eq!(output.locomotion_flags & REQUEST_WIPEOUT, REQUEST_WIPEOUT);
    output.locomotion_flags = 0;
    anim.publish_air_wipeout_flags(&mut output, &refs).unwrap();
    assert_eq!(output.locomotion_flags, 0);
}

#[test]
fn source_debug_validation_ignores_w_and_accepts_infinity() {
    assert!(!xyz_nan(&[0.0, f32::INFINITY, -0.0, f32::NAN]));
    for axis in 0..3 {
        let mut vector = [0.0; 4];
        vector[axis] = f32::NAN;
        assert!(xyz_nan(&vector));
    }
}

#[test]
fn air_suffix_reads_physical_duration_and_keeps_nan_comparison_differences() {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let mut anim = animation(true);
    let trajectory = &mut anim.skater.as_mut().unwrap().trajectory;
    trajectory.current_state = 2;
    trajectory.physical_foot_index = 1;
    trajectory.time = 2.0;
    trajectory.input_feet[1].states[2].time = -50.0;
    trajectory.physical_feet[1].states[2].time = 3.0;
    let mut output = output();
    anim.publish_air_wipeout_flags(&mut output, &refs).unwrap();
    assert_eq!(output.locomotion_flags & 0x2800_0000, 0x2800_0000);
    let trajectory = &mut anim.skater.as_mut().unwrap().trajectory;
    trajectory.time = f32::NAN;
    trajectory.physical_feet[1].states[2].time = f32::NAN;
    output.locomotion_flags = 0;
    anim.publish_air_wipeout_flags(&mut output, &refs).unwrap();
    // BGT is false and BLE (NOT GT) skips the request on unordered values.
    assert_eq!(output.locomotion_flags & 0x2800_0000, 0);
    anim.skater.as_mut().unwrap().trajectory.physical_foot_index = 4;
    assert!(matches!(
        anim.publish_air_wipeout_flags(&mut output, &refs),
        Err(AnimationPhysicsDataError::FootIndex(4))
    ));
}

#[test]
fn full_locomotion_keeps_real_com_and_next_step_writes_before_missing_body_output() {
    let cvars = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&cvars).unwrap();
    manager
        .try_deserialize_startup(Some(b"[Config]\nkSA_UseWorldYForNormal=1\n"))
        .unwrap();
    let mut anim = animation(true);
    anim.player.as_mut().unwrap().flags_bits |= 0x0018_0000;
    anim.before_face
        .as_mut()
        .unwrap()
        .collision_trajectory_displacement = WalkVector::known([2.0, 3.0, 4.0, 5.0]);
    let trajectory = &mut anim.skater.as_mut().unwrap().trajectory;
    trajectory.current_state = 2;
    trajectory.time = 0.0;
    trajectory.physical_feet[0].states[2].time = 1.0;
    trajectory.physical_feet[0].initialized = true;
    trajectory.physical_feet[1].initialized = true;
    trajectory.physical_feet[1].data.position = WalkVector::known([10.0, 20.0, 30.0, 40.0]);
    let mut output = output();
    let result = anim.try_get_physics_update(
        &mut output,
        0.25,
        &cvars,
        &mut crate::reckoning_numeric::HostReckoningNumeric,
    );
    assert!(matches!(
        result,
        Err(locomotion::AnimationLocomotionError::Physics(
            AnimationPhysicsDataError::Missing("initialized body buffers")
        ))
    ));
    assert_eq!(
        output
            .center_of_mass_delta
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [2.0_f32, 3.0, 4.0, 5.0].map(f32::to_bits)
    );
    assert_eq!(
        output
            .center_of_mass_animation
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0; 4]
    );
    assert_eq!(
        output
            .next_step_position
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [10.0_f32, 20.0, 30.0, 40.0].map(f32::to_bits)
    );
    assert_eq!(
        output
            .next_step_normal
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0.0_f32, 1.0, 0.0, 0.0].map(f32::to_bits)
    );
    assert!(output.flags.next_step_valid);
}

#[test]
fn motion_locomotion_start_foot_targets_trajectory_and_named_selection_not_foot_down() {
    let mut anim = animation(true);
    anim.before_sprint.as_mut().unwrap().foot_down = 7;
    anim.before_sprint.as_mut().unwrap().last_foot_down = 8;
    anim.motion_starting_locomotion_foot(1).unwrap();
    let trajectory = &anim.skater.as_ref().unwrap().trajectory;
    assert_eq!(trajectory.foot_locked, 1);
    assert_eq!(trajectory.physical_foot_index, 1);
    assert_eq!(anim.before_sprint.as_ref().unwrap().foot_down, 7);
    assert_eq!(anim.before_sprint.as_ref().unwrap().last_foot_down, 8);
    let base = anim.base.as_ref().unwrap();
    assert!(
        base.construction_map.slots[..base.construction_map.len]
            .iter()
            .any(|a| a.as_ref()
                == Some(&(
                    FastString30::encode("StartSequenceID").unwrap(),
                    FastString30::encode("3").unwrap()
                )))
    );
    let left = WalkVector {
        xyz: [1.0, 2.0, 3.0],
        w: None,
    };
    let right = WalkVector::known([4.0, 5.0, 6.0, 7.0]);
    anim.motion_locomotion_feet(left, right).unwrap();
    assert_eq!(anim.before_face.as_ref().unwrap().feet[1], left);
    assert_eq!(anim.before_face.as_ref().unwrap().feet[0], right);
}
#[test]
fn motion_foot_down_uses_locked_trajectory_foot_with_mirrored_parity() {
    let mut anim = animation(true);
    anim.before_sprint.as_mut().unwrap().foot_down = 99; // different native field
    for (locked, state, time, mirrored, foot, expected) in [
        (0, 0, 0.0, false, 0, true),
        (0, 0, 0.0, false, 1, false),
        (0, 0, 0.0, true, 1, true),
        (1, 0, 0.0, true, 0, true),
        (1, 1, 0.0, false, 1, false),
        (1, 0, 0.1, false, 1, false),
        (1, 0, f32::NAN, false, 1, true),
        (-1, 0, 0.0, false, 1, false),
    ] {
        anim.motion_locomotion_flag(0x4000_0000, mirrored).unwrap();
        let trajectory = &mut anim.skater.as_mut().unwrap().trajectory;
        trajectory.foot_locked = locked;
        trajectory.current_state = state;
        trajectory.time = time;
        assert_eq!(anim.motion_locomotion_foot_down(foot).unwrap(), expected);
    }
    for (level, expected) in [(0.49, false), (0.5, true), (f32::NAN, false)] {
        anim.before_face.as_mut().unwrap().sprint_level = level;
        assert_eq!(anim.motion_trying_to_sprint().unwrap(), expected);
    }
}

#[test]
fn jump_air_and_apex_read_distinct_native_foot_buffers() {
    let mut anim = animation(true);
    anim.motion_locomotion_flag(0x0010_0000, true).unwrap();
    anim.motion_locomotion_flag(0x4000_0000, false).unwrap();
    let trajectory = &mut anim.skater.as_mut().unwrap().trajectory;
    trajectory.current_state = 2;
    trajectory.physical_foot_index = 1;
    trajectory.foot_locked = 0;
    trajectory.time = 99.0; // neither query reads this timer
    trajectory.physical_feet[1].states[2].time = 0.5;
    trajectory.input_feet[1].states[2].vectors[1].xyz[1] = 9.81;
    trajectory.input_feet[1].states[2].time = 0.5;
    assert!(anim.try_is_in_jump_air(0).unwrap());
    assert!(!anim.try_is_in_jump_air(1).unwrap());
    assert!(!anim.try_is_past_locomotion_apex().unwrap());
    anim.skater.as_mut().unwrap().trajectory.input_feet[1].states[2].time = 1.1;
    assert!(anim.try_is_past_locomotion_apex().unwrap());
    anim.motion_locomotion_flag(0x4000_0000, true).unwrap();
    assert!(anim.try_is_in_jump_air(1).unwrap());
    anim.skater.as_mut().unwrap().trajectory.physical_feet[1].states[2].time = 0.0;
    assert!(!anim.try_is_in_jump_air(1).unwrap()); // equality with default threshold
    anim.skater.as_mut().unwrap().trajectory.physical_feet[1].states[2].time = f32::NAN;
    assert!(anim.try_is_in_jump_air(1).unwrap()); // original BLE unordered behavior
    anim.motion_locomotion_flag(0x0010_0000, false).unwrap();
    anim.skater.as_mut().unwrap().trajectory.physical_foot_index = -1;
    assert!(!anim.try_is_past_locomotion_apex().unwrap()); // lazy index access
}
#[test]
fn motion_locomotion_condition_ordered_nan_and_threshold_gates() {
    let mut anim = animation(true);
    anim.motion_locomotion_flag(0x40000, true).unwrap();
    for (time, expected) in [(0.5, false), (0.5001, true), (f32::NAN, false)] {
        anim.before_face
            .as_mut()
            .unwrap()
            .time_since_leaving_locomotion = time;
        assert_eq!(anim.motion_locomotion_valid().unwrap(), expected);
    }
    for (speed, state, expected) in [
        (0.2, 0, true),
        (0.2001, 0, false),
        (f32::NAN, 0, true),
        (f32::NAN, 2, false),
    ] {
        let fields = anim.before_sprint.as_mut().unwrap();
        fields.speed = speed;
        fields.last_start_state = state;
        assert_eq!(anim.motion_locomotion_quit_allowed().unwrap(), expected);
    }
}
