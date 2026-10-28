use crate::physics_state_ids::{PHYSICS_CATEGORY_GROUND, PHYSICS_STATE_PHYSICS_GROUND};
use super::*;
use crate::{
    attribute_runtime::tests::{collection, graph, set_dependency_bytes},
    debug_overlay::DebugOverlaySlot,
    input_cvar_runtime::InputCvarManager,
    physical_player_phys_out::{
        PhysOutMotionVector, PhysOutOrigin, PhysOutValue, construct::PhysOutAllocation,
    },
    skeleton_root_transforms::SkeletonRootVector as V,
};
struct Numeric {
    calls: Vec<&'static str>,
    fail: Option<usize>,
}
impl Numeric {
    fn call(&mut self, name: &'static str) -> Result<(), &'static str> {
        self.calls.push(name);
        if self.fail == Some(self.calls.len()) {
            Err("numeric")
        } else {
            Ok(())
        }
    }
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.call("dot")?;
        Ok((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
    }
    fn reciprocal_sqrt_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        self.call("rsqrt")?;
        Ok(1.0 / x.sqrt())
    }
    fn reciprocal_estimate(&mut self, x: f32) -> Result<f32, Self::Error> {
        self.call("reciprocal")?;
        Ok(1.0 / x)
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no conversion")
    }
}
fn numeric() -> Numeric {
    Numeric {
        calls: vec![],
        fail: None,
    }
}
fn output(velocity: [f32; 4]) -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out.concrete
        .collision
        .as_mut()
        .unwrap()
        .average_ground_normal = V::known([0., 1., 0., 0.]);
    out.motion_vectors[PhysOutMotionVector::DeckVelocity as usize] = Some(PhysOutValue {
        value: V::known(velocity),
        origin: PhysOutOrigin::Publication,
    });
    out
}
fn linked() -> InputCvarReferences {
    let refs = InputCvarReferences::new_static_initialized();
    InputCvarManager::new().try_link(&refs).unwrap();
    refs
}
fn global() -> AttributeRuntimeInstance {
    global_with_x([0.0, 0.25, 0.75, 1.0])
}
fn global_with_x(points: [f32; 4]) -> AttributeRuntimeInstance {
    const CLASS: u64 = 0x74bc_9c93_affe_6aac;
    const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;
    let mut vault = graph(CLASS, vec![]);
    collection(&mut vault, CLASS, DEFAULT, 0, Some(0), vec![]);
    let mut bytes = vec![0; 0x220];
    for (offset, value) in [(0x170, 0.8_f32), (0x190, 0.4_f32)] {
        for (i, x) in points.into_iter().enumerate() {
            bytes[offset + i * 4..offset + i * 4 + 4].copy_from_slice(&x.to_be_bytes());
            bytes[offset + 16 + i * 4..offset + 20 + i * 4].copy_from_slice(&value.to_be_bytes());
        }
    }
    set_dependency_bytes(&mut vault, 1, bytes);
    Arc::new(vault).instance(CLASS, DEFAULT)
}
#[test]
fn degenerate_landing_still_calls_both_estimates_and_side_dot_but_no_cvars() {
    let mut out = output([0.; 4]);
    let refs = InputCvarReferences::new_static_initialized();
    let mut num = numeric();
    out.try_calc_landing_quality(&LandingCvars(&refs), None, &mut num)
        .unwrap();
    assert_eq!(
        num.calls,
        vec!["dot", "dot", "dot", "dot", "dot", "rsqrt", "rsqrt", "dot"]
    );
    let a = out.concrete.animation.as_ref().unwrap();
    assert!(a.just_analyzed_sketchyness);
    assert_eq!(a.sketchyness_anim_result, 0);
    assert_eq!(a.sketchyness.to_bits(), 0);
    assert_eq!(a.sketchyness_cosine.to_bits(), 0);
    assert_eq!(a.sketchyness_side_speed.to_bits(), 0);
}
#[test]
fn active_landing_requires_real_global_layout_after_reciprocal_calls() {
    let mut out = output([4., 0., 4., 0.]);
    let refs = linked();
    let mut num = numeric();
    assert!(matches!(
        out.try_calc_landing_quality(&LandingCvars(&refs), None, &mut num),
        Err(LandingQualityError::MissingGlobalAnimation)
    ));
    assert_eq!(&num.calls[8..], &["reciprocal", "reciprocal"]);
    assert!(
        !out.concrete
            .animation
            .as_ref()
            .unwrap()
            .just_analyzed_sketchyness
    );
}
#[test]
fn complete_active_landing_uses_two_actual_layout_graphs_and_spin_direction() {
    let refs = linked();
    let global = global();
    for (body_spin, result, scalar) in [(1.0_f32, 2, -0.8_f32), (-1., 1, 0.8), (0., 1, 0.4)] {
        let mut out = output([4., 0., 4., 0.]);
        out.concrete.air.as_mut().unwrap().body_spin_speed = body_spin;
        out.try_calc_landing_quality(&LandingCvars(&refs), Some(&global), &mut numeric())
            .unwrap();
        let a = out.concrete.animation.as_ref().unwrap();
        assert!(a.just_analyzed_sketchyness);
        assert_eq!(a.sketchyness_anim_result, result);
        assert_eq!(a.sketchyness.to_bits(), scalar.to_bits());
        assert_eq!(a.sketchyness_anim_scalar.to_bits(), scalar.to_bits());
        assert_eq!(a.sketchyness_cosine.to_bits(), 4.0_f32.to_bits());
        assert_eq!(a.sketchyness_side_speed.to_bits(), 4.0_f32.to_bits());
    }
}
#[test]
fn failed_numeric_keeps_all_animation_fields_unpublished() {
    let mut out = output([4., 0., 4., 0.]);
    let before = out.concrete.animation.as_ref().unwrap().clone();
    let refs = linked();
    let mut num = numeric();
    num.fail = Some(6);
    assert!(matches!(
        out.try_calc_landing_quality(&LandingCvars(&refs), None, &mut num),
        Err(LandingQualityError::Numeric("numeric"))
    ));
    assert_eq!(out.concrete.animation.as_ref().unwrap(), &before);
}
#[test]
fn unordered_geometry_reaches_live_tuning_and_preserves_unpublished_output_on_error() {
    let mut out = output([f32::NAN, 0., 1., 0.]);
    let before = out.concrete.animation.as_ref().unwrap().clone();
    let refs = InputCvarReferences::new_static_initialized();
    assert!(matches!(
        out.try_calc_landing_quality(&LandingCvars(&refs), None, &mut numeric()),
        Err(LandingQualityError::Cvar(_))
    ));
    assert_eq!(out.concrete.animation.as_ref().unwrap(), &before);
}
#[test]
fn late_force_spin_threshold_controls_small_angle_result() {
    let refs = linked();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    let global = global();
    for (force, result) in [(b"1".as_slice(), 0), (b"0".as_slice(), 2)] {
        let mut ini = b"[Config]\nkStraightAngleThresh=2\nkForceSpinThresh=".to_vec();
        ini.extend_from_slice(force);
        ini.push(b'\n');
        manager.try_deserialize_startup(Some(&ini)).unwrap();
        let mut out = output([4., 0., 4., 0.]);
        out.concrete.air.as_mut().unwrap().body_spin_speed = 1.;
        out.try_calc_landing_quality(&LandingCvars(&refs), Some(&global), &mut numeric())
            .unwrap();
        assert_eq!(
            out.concrete
                .animation
                .as_ref()
                .unwrap()
                .sketchyness_anim_result,
            result
        );
    }
}
#[test]
fn actual_continuation_uses_retained_history_gate_without_eager_inputs() {
    let mut out = output([0.; 4]);
    out.concrete.state.as_mut().unwrap().physics_state = PHYSICS_STATE_PHYSICS_GROUND;
    out.concrete.state.as_mut().unwrap().physics_state_category = PHYSICS_CATEGORY_GROUND;
    let refs = linked();
    let mut owner = Conditioners::before_constructor(&out);
    owner
        .try_construct(&mut out, &Arc::new(graph(0, vec![])), &refs, &mut |_| {
            Ok::<_, ()>(true)
        })
        .unwrap();
    let mut num = numeric();
    let before = owner
        .try_update_physics_prefix(&mut out, &refs, &mut num)
        .unwrap();
    let before = owner
        .try_update_physics_damped_com(
            before,
            &mut out,
            &refs,
            &DebugOverlaySlot::default(),
            &mut num,
        )
        .unwrap();
    let before = owner
        .try_update_physics_foot_side(before, &mut out, &mut num)
        .unwrap();
    owner
        .physics
        .published_value_mut()
        .unwrap()
        .payload
        .last_filtered_state = FilteredPhysicsState::Ground;
    let calls = num.calls.len();
    let unlinked = InputCvarReferences::new_static_initialized();
    let after = owner
        .try_update_physics_landing_quality(before, &mut out, &unlinked, None, &mut num)
        .unwrap();
    assert!(after.matches_output(&out));
    assert_eq!(num.calls.len(), calls);
    assert!(
        !out.concrete
            .animation
            .as_ref()
            .unwrap()
            .just_analyzed_sketchyness
    );
}

#[test]
fn graph_unordered_interval_width_selects_upper_without_interpolation() {
    let global = global_with_x([0.0, f32::NAN, 2.0, 3.0]);
    let mut out = output([4.0, 0.0, 4.0, 0.0]);
    out.concrete.air.as_mut().unwrap().body_spin_speed = -1.0;
    out.try_calc_landing_quality(&LandingCvars(&linked()), Some(&global), &mut numeric())
        .unwrap();
    assert_eq!(
        out.concrete
            .animation
            .as_ref()
            .unwrap()
            .sketchyness
            .to_bits(),
        0.8f32.to_bits()
    );
}
