use crate::physics_state_ids::{
    PHYSICS_CATEGORY_GROUND, PHYSICS_STATE_PHYSICS_GROUND, PHYSICS_STATE_SLIDE_GROUND,
};
use super::*;
use crate::{
    input_cvar_runtime::InputCvarManager,
    physical_player_phys_out::{
        PhysOutMotionScalar, PhysOutOrigin, PhysOutValue, construct::PhysOutAllocation,
    },
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector as V},
};
fn output(speed: f32, nose: f32) -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out.concrete.state.as_mut().unwrap().physics_state = PHYSICS_STATE_PHYSICS_GROUND;
    out.concrete.state.as_mut().unwrap().physics_state_category = PHYSICS_CATEGORY_GROUND;
    out.collision_type = Some(PhysOutValue {
        value: 8,
        origin: PhysOutOrigin::Publication,
    });
    out.motion_scalars[PhysOutMotionScalar::DeckSpeed as usize] = Some(PhysOutValue {
        value: speed,
        origin: PhysOutOrigin::Publication,
    });
    out.concrete
        .skateboard_reckoning
        .as_mut()
        .unwrap()
        .transform = SkeletonRootMatrix::from_columns([
        V::known([1., 0., 0., 0.]),
        V::known([0., 1., 0., 0.]),
        V {
            xyz: [0., nose, 1.],
            w: None,
        },
        V::known([0.; 4]),
    ]);
    out
}
fn linked() -> InputCvarReferences {
    let refs = InputCvarReferences::new_static_initialized();
    InputCvarManager::new().try_link(&refs).unwrap();
    refs
}
#[test]
fn early_exits_need_no_cvars_and_do_not_clear_a_prior_flag() {
    for exit in 0..3 {
        let mut out = output(10., 1.);
        out.concrete
            .animation
            .as_mut()
            .unwrap()
            .should_disallow_tricks_due_to_stairs = true;
        match exit {
            0 => out.concrete.state.as_mut().unwrap().physics_state = PHYSICS_STATE_SLIDE_GROUND,
            1 => out.concrete.state.as_mut().unwrap().is_manual = true,
            _ => out.collision_type.as_mut().unwrap().value = 0,
        }
        out.try_watch_for_stairs(
            &InputCvarReferences::new_static_initialized(),
            &DebugOverlaySlot::default(),
        )
        .unwrap();
        assert!(
            out.concrete
                .animation
                .as_ref()
                .unwrap()
                .should_disallow_tricks_due_to_stairs
        );
    }
}
#[test]
fn equality_exits_at_each_threshold_without_overlay() {
    let refs = linked();
    for (speed, nose) in [(1.5_f32, 1.0_f32), (2.0, f32::from_bits(0x3e4c_cccd))] {
        let mut out = output(speed, nose);
        out.try_watch_for_stairs(&refs, &DebugOverlaySlot::default())
            .unwrap();
        assert!(
            !out.concrete
                .animation
                .as_ref()
                .unwrap()
                .should_disallow_tricks_due_to_stairs
        );
    }
}
#[test]
fn active_store_survives_missing_overlay() {
    let mut out = output(2., -0.5);
    assert!(matches!(
        out.try_watch_for_stairs(&linked(), &DebugOverlaySlot::default()),
        Err(StairsError::Overlay(
            crate::debug_overlay::DebugOverlayError::Missing
        ))
    ));
    assert!(
        out.concrete
            .animation
            .as_ref()
            .unwrap()
            .should_disallow_tricks_due_to_stairs
    );
}
#[test]
fn unordered_speed_exits_before_flag_and_overlay() {
    let overlay = DebugOverlaySlot::default();
    let mut out = output(f32::NAN, f32::NAN);
    out.try_watch_for_stairs(&linked(), &overlay).unwrap();
    assert!(
        !out.concrete
            .animation
            .as_ref()
            .unwrap()
            .should_disallow_tricks_due_to_stairs
    );
}
#[test]
fn actual_seven_call_chain_returns_physics_completion_for_same_output() {
    let mut out = output(0., 0.);
    out.collision_type.as_mut().unwrap().value = 0;
    let refs = linked();
    let overlay = DebugOverlaySlot::default();
    let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
    let mut owner = Conditioners::before_constructor(&out);
    owner
        .try_construct(
            &mut out,
            &Arc::new(crate::attribute_runtime::tests::graph(0, vec![])),
            &refs,
            &mut |_| Ok::<_, ()>(true),
        )
        .unwrap();
    let before = owner
        .try_update_physics_prefix(&mut out, &refs, &mut numeric)
        .unwrap();
    let before = owner
        .try_update_physics_damped_com(before, &mut out, &refs, &overlay, &mut numeric)
        .unwrap();
    let before = owner
        .try_update_physics_foot_side(before, &mut out, &mut numeric)
        .unwrap();
    let before = owner
        .try_update_physics_landing_quality(before, &mut out, &refs, None, &mut numeric)
        .unwrap();
    let done = owner
        .try_finish_physics_update(before, &mut out, &refs, &overlay)
        .unwrap();
    assert!(done.matches_output(&out));
    assert!(!done.matches_output(&output(0., 0.)));
    assert!(
        out.concrete
            .animation
            .as_ref()
            .unwrap()
            .just_analyzed_sketchyness
    );
}
