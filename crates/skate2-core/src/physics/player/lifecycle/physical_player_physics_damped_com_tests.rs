use crate::physics_state_ids::{PHYSICS_CATEGORY_GROUND, PHYSICS_STATE_PHYSICS_GROUND};
use super::*;
use crate::{
    input_cvar_runtime::InputCvarManager,
    physical_player_phys_out::{
        PhysOutOrigin, PhysOutSystemVector, PhysOutValue, construct::PhysOutAllocation,
    },
};
struct Numeric {
    result: Result<f32, &'static str>,
    calls: usize,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        self.calls += 1;
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
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out.system_vectors[PhysOutSystemVector::CentreOfMass as usize] = Some(PhysOutValue {
        value: V::known([4., 8., 12., 7.]),
        origin: PhysOutOrigin::Publication,
    });
    out.system_vectors[PhysOutSystemVector::CentreOfMassVelocity as usize] = Some(PhysOutValue {
        value: V::known([60., 120., 180., 11.]),
        origin: PhysOutOrigin::Publication,
    });
    out
}
fn linked() -> (InputCvarManager, InputCvarReferences) {
    let refs = InputCvarReferences::new_static_initialized();
    let mut manager = InputCvarManager::new();
    manager.try_link(&refs).unwrap();
    (manager, refs)
}
#[test]
fn first_update_needs_no_cvars_or_numeric_and_keeps_error_history() {
    let mut physics = Physics::reset_values();
    physics.accumulated_com_error = V::known([1., 2., 3., 4.]);
    let mut out = output();
    let refs = InputCvarReferences::new_static_initialized();
    let mut numeric = Numeric {
        result: Err("not called"),
        calls: 0,
    };
    physics
        .update_damped_com(
            &mut out,
            &DampedComCvars(&refs),
            &DebugOverlaySlot::default(),
            &mut numeric,
        )
        .unwrap();
    assert!(physics.last_smoothed_com_position_valid);
    assert_eq!(
        physics.last_damped_com_position,
        V::known([4., 8., 12., 7.])
    );
    assert_eq!(physics.accumulated_com_error, V::known([1., 2., 3., 4.]));
    assert_eq!(physics.heavily_damped_com_velocity, ZERO);
    assert_eq!(
        out.concrete.system.as_ref().unwrap().damped_centre_of_mass,
        physics.last_damped_com_position
    );
    assert_eq!(numeric.calls, 0);
}
#[test]
fn sustained_update_uses_live_axis_coefficients_and_fixed_step() {
    let (mut manager, refs) = linked();
    manager.try_deserialize_startup(Some(b"[Config]\nkDampedComBlend=0\nkDampedComBlendY=0\nkDampedComVelBlend=1\nkDampedComVelBlendY=0.5\nkDampedComErrorIterBlend=1\n")).unwrap();
    let mut physics = Physics::reset_values();
    physics.last_smoothed_com_position_valid = true;
    let mut out = output();
    let mut numeric = Numeric {
        result: Ok(0.),
        calls: 0,
    };
    physics
        .update_damped_com(
            &mut out,
            &DampedComCvars(&refs),
            &DebugOverlaySlot::default(),
            &mut numeric,
        )
        .unwrap();
    assert_eq!(
        physics.heavily_damped_com_velocity,
        V::known([60., 60., 180., 0.])
    );
    let dt = f32::from_bits(0x3c88_8889);
    let expected = V::known([
        60.0_f32.mul_add(dt, 0.),
        60.0_f32.mul_add(dt, 0.),
        180.0_f32.mul_add(dt, 0.),
        0.,
    ]);
    assert_eq!(physics.last_damped_com_position, expected);
    assert_eq!(
        physics.accumulated_com_error.xyz.map(f32::to_bits),
        std::array::from_fn(|i| ([4., 8., 12.][i] - expected.xyz[i]).to_bits())
    );
    assert_eq!(numeric.calls, 1);
}
#[test]
fn failed_dot_preserves_earlier_velocity_store_only() {
    let (_manager, refs) = linked();
    let mut physics = Physics::reset_values();
    physics.last_smoothed_com_position_valid = true;
    physics.last_damped_com_position = V::known([9.; 4]);
    let mut out = output();
    let mut numeric = Numeric {
        result: Err("dot"),
        calls: 0,
    };
    assert!(matches!(
        physics.update_damped_com(
            &mut out,
            &DampedComCvars(&refs),
            &DebugOverlaySlot::default(),
            &mut numeric
        ),
        Err(DampedComError::Numeric("dot"))
    ));
    assert_ne!(physics.heavily_damped_com_velocity, ZERO);
    assert_eq!(physics.last_damped_com_position, V::known([9.; 4]));
    assert_eq!(physics.accumulated_com_error, ZERO);
    assert_eq!(
        out.concrete.system.as_ref().unwrap().damped_centre_of_mass,
        ZERO
    );
}
#[test]
fn draw_gate_requires_actual_overlay_before_position_store() {
    let (mut manager, refs) = linked();
    manager
        .try_deserialize_startup(Some(b"[Config]\nkDampedComDraw=1\n"))
        .unwrap();
    let mut physics = Physics::reset_values();
    physics.last_smoothed_com_position_valid = true;
    let mut out = output();
    let mut numeric = Numeric {
        result: Ok(f32::NAN),
        calls: 0,
    };
    assert!(matches!(
        physics.update_damped_com(
            &mut out,
            &DampedComCvars(&refs),
            &DebugOverlaySlot::default(),
            &mut numeric
        ),
        Err(DampedComError::Overlay(DebugOverlayError::Missing))
    ));
    assert_ne!(physics.heavily_damped_com_velocity, ZERO);
    assert_eq!(physics.last_damped_com_position, ZERO);
    assert_eq!(physics.accumulated_com_error, ZERO);
}
#[test]
fn positive_error_correction_and_unknown_w_are_retained() {
    let (mut manager, refs) = linked();
    manager
        .try_deserialize_startup(Some(
            b"[Config]\nkDampedComBlend=1\nkDampedComBlendY=1\nkDampedComErrorBlend=0.5\n",
        ))
        .unwrap();
    let mut physics = Physics::reset_values();
    physics.last_smoothed_com_position_valid = true;
    physics.accumulated_com_error = V {
        xyz: [2., 4., 6.],
        w: None,
    };
    let mut out = output();
    let mut numeric = Numeric {
        result: Ok(1.),
        calls: 0,
    };
    physics
        .update_damped_com(
            &mut out,
            &DampedComCvars(&refs),
            &DebugOverlaySlot::default(),
            &mut numeric,
        )
        .unwrap();
    assert_eq!(
        physics.last_damped_com_position,
        V {
            xyz: [5., 10., 15.],
            w: None
        }
    );
    assert_eq!(physics.accumulated_com_error.w, None);
    assert_eq!(
        out.concrete
            .system
            .as_ref()
            .unwrap()
            .damped_centre_of_mass
            .w,
        None
    );
}
#[test]
fn actual_retained_continuation_rejects_foreign_output() {
    let (_manager, refs) = linked();
    let mut out = output();
    out.concrete.state.as_mut().unwrap().physics_state = PHYSICS_STATE_PHYSICS_GROUND;
    out.concrete.state.as_mut().unwrap().physics_state_category = PHYSICS_CATEGORY_GROUND;
    let mut owner = Conditioners::before_constructor(&out);
    owner
        .try_construct(
            &mut out,
            &Arc::new(crate::attribute_runtime::tests::graph(0, vec![])),
            &refs,
            &mut |_| Ok::<_, ()>(true),
        )
        .unwrap();
    let mut numeric = Numeric {
        result: Err("first frame has no dot"),
        calls: 0,
    };
    let before = owner
        .try_update_physics_prefix(&mut out, &refs, &mut numeric)
        .unwrap();
    let mut foreign = output();
    assert!(matches!(
        owner.try_update_physics_damped_com(
            before,
            &mut foreign,
            &refs,
            &DebugOverlaySlot::default(),
            &mut numeric
        ),
        Err(DampedComError::ForeignOutput)
    ));
    assert!(
        !owner
            .physics
            .published_value_mut()
            .unwrap()
            .payload
            .last_smoothed_com_position_valid
    );
}
