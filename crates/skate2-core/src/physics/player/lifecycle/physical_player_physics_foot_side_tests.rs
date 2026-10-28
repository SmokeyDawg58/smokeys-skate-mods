use crate::physics_state_ids::{PHYSICS_CATEGORY_GROUND, PHYSICS_STATE_PHYSICS_GROUND};
use super::*;
use crate::{
    debug_overlay::DebugOverlaySlot,
    input_cvar_runtime::{InputCvarManager, InputCvarReferences},
    physical_player_phys_out::{
        PhysOutAccessError, PhysOutOrigin, PhysOutSystemVector,
        construct::{PhysOutAllocation, prior_input::PriorInputReadError},
    },
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector as V},
};
struct Numeric {
    result: Result<f32, &'static str>,
    operands: Vec<([f32; 4], [f32; 4])>,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.operands.push((a, b));
        self.result
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("no reciprocal")
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("no rsqrt")
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        panic!("no conversion")
    }
}
fn numeric(result: Result<f32, &'static str>) -> Numeric {
    Numeric {
        result,
        operands: vec![],
    }
}
fn output(axis: V) -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out.concrete
        .skateboard_reckoning
        .as_mut()
        .unwrap()
        .transform = SkeletonRootMatrix::from_columns([
        axis,
        V::known([0.; 4]),
        V::known([0.; 4]),
        V::known([0.; 4]),
    ]);
    let skeleton = out.concrete.skeleton.as_mut().unwrap();
    skeleton.skeleton_foot_left_look = V::known([1., 2., 3., 4.]);
    skeleton.skeleton_foot_right_look = V::known([5., 6., 7., 8.]);
    out
}
fn bits(vector: V) -> ([u32; 3], Option<u32>) {
    (vector.xyz.map(f32::to_bits), vector.w.map(f32::to_bits))
}
#[test]
fn positive_dot_publishes_actual_board_axis_and_uses_both_look_vectors() {
    let axis = V::known([2., -0., 3., f32::from_bits(0x7fc0_1234)]);
    let mut out = output(axis);
    let mut numeric = numeric(Ok(1.));
    out.try_calc_foot_side(&mut numeric).unwrap();
    assert_eq!(numeric.operands.len(), 1);
    assert_eq!(
        numeric.operands[0].0.map(f32::to_bits),
        [2., -0., 3., f32::from_bits(0x7fc0_1234)].map(f32::to_bits)
    );
    assert_eq!(
        numeric.operands[0].1.map(f32::to_bits),
        [6., 8., 10., 12.].map(f32::to_bits)
    );
    let published = out.system_vector(PhysOutSystemVector::Right).unwrap();
    assert_eq!(bits(published.value), bits(axis));
    assert_eq!(published.origin, PhysOutOrigin::Publication);
}
#[test]
fn negative_zero_and_unordered_dot_flip_all_lane_sign_bits() {
    let axis = V::known([2., -0., 3., f32::from_bits(0x7fc0_1234)]);
    for dot in [-1., 0., -0., f32::NAN] {
        let mut out = output(axis);
        out.try_calc_foot_side(&mut numeric(Ok(dot))).unwrap();
        assert_eq!(
            bits(out.system_vector(PhysOutSystemVector::Right).unwrap().value),
            bits(axis.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000)))
        );
    }
}
#[test]
fn unknown_board_w_is_not_manufactured_by_sign_selection() {
    for dot in [1., -1.] {
        let mut out = output(V {
            xyz: [1., 0., 0.],
            w: None,
        });
        out.try_calc_foot_side(&mut numeric(Ok(dot))).unwrap();
        assert_eq!(
            out.system_vector(PhysOutSystemVector::Right)
                .unwrap()
                .value
                .w,
            None
        );
    }
}
#[test]
fn numeric_and_first_guard_failures_do_not_publish() {
    let mut out = output(V::known([2., 3., 4., 5.]));
    let before = out.system_vector(PhysOutSystemVector::Right).unwrap();
    assert!(matches!(
        out.try_calc_foot_side(&mut numeric(Err("dot"))),
        Err(FootSideError::Numeric("dot"))
    ));
    assert_eq!(
        out.system_vector(PhysOutSystemVector::Right).unwrap(),
        before
    );
    out.locked = Some(true);
    let mut numeric = numeric(Ok(1.));
    assert!(matches!(
        out.try_calc_foot_side(&mut numeric),
        Err(FootSideError::Output(PriorInputReadError::Access(
            PhysOutAccessError::LockedForWrite
        )))
    ));
    assert!(numeric.operands.is_empty());
}
fn retained() -> (
    Conditioners,
    PhysicalPlayerPhysOut,
    PhysicsBeforeFootSideOfDeck,
) {
    let mut out = output(V::known([1., 0., 0., 0.]));
    out.concrete.state.as_mut().unwrap().physics_state = PHYSICS_STATE_PHYSICS_GROUND;
    out.concrete.state.as_mut().unwrap().physics_state_category = PHYSICS_CATEGORY_GROUND;
    let refs = InputCvarReferences::new_static_initialized();
    InputCvarManager::new().try_link(&refs).unwrap();
    let mut owner = Conditioners::before_constructor(&out);
    owner
        .try_construct(
            &mut out,
            &Arc::new(crate::attribute_runtime::tests::graph(0, vec![])),
            &refs,
            &mut |_| Ok::<_, ()>(true),
        )
        .unwrap();
    let mut numeric = numeric(Err("first prefix has no dot"));
    let before = owner
        .try_update_physics_prefix(&mut out, &refs, &mut numeric)
        .unwrap();
    let before = owner
        .try_update_physics_damped_com(
            before,
            &mut out,
            &refs,
            &DebugOverlaySlot::default(),
            &mut numeric,
        )
        .unwrap();
    (owner, out, before)
}
#[test]
fn actual_continuation_keeps_same_output_and_physics_histories() {
    let (mut owner, mut out, before) = retained();
    let history = owner.physics.published_value_mut().unwrap().payload.clone();
    let after = owner
        .try_update_physics_foot_side(before, &mut out, &mut numeric(Ok(1.)))
        .unwrap();
    assert!(after.matches_output(&out));
    assert_eq!(
        owner.physics.published_value_mut().unwrap().payload,
        history
    );
}
#[test]
fn actual_continuation_rejects_foreign_output_before_numeric() {
    let (mut owner, _out, before) = retained();
    let mut foreign = output(V::known([1., 0., 0., 0.]));
    let mut numeric = numeric(Ok(1.));
    assert!(matches!(
        owner.try_update_physics_foot_side(before, &mut foreign, &mut numeric),
        Err(FootSideError::ForeignOutput)
    ));
    assert!(numeric.operands.is_empty());
}
