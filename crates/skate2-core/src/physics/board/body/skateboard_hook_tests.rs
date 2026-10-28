//! Synthetic callback/error discriminators, not captured Xenon output traces.
use super::*;
use crate::skateboard_wheels::{
    HostSkateboardWheelMassNumeric, SkateboardWheelInertiaAxis as Axis,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Quaternion([u32; 4]),
    Mass(Axis, u32),
}
type Trace = Rc<RefCell<Vec<Event>>>;
struct Quaternion {
    trace: Trace,
    frames: Arc<RwLock<SimulationDriveFrames>>,
    dynamics: Arc<RwLock<SimulationDriveDynamics>>,
    observations: Vec<(SimulationDriveFrames, SimulationDriveDynamics)>,
    seeds: [f32; 2],
    fail: Option<usize>,
}
impl SkateboardTruckDriveNumeric for Quaternion {
    type Error = &'static str;
    fn sine_cosine(&mut self, _: f32) -> Result<(f32, f32), Self::Error> {
        panic!("CreateHookDrive does not call SinCos");
    }
    fn quaternion_reciprocal_square_root_estimate(
        &mut self,
        values: [f32; 4],
    ) -> Result<[f32; 4], Self::Error> {
        self.trace
            .borrow_mut()
            .push(Event::Quaternion(values.map(f32::to_bits)));
        let ordinal = self.observations.len();
        self.observations
            .push((*self.frames.read().unwrap(), *self.dynamics.read().unwrap()));
        if self.fail == Some(ordinal) {
            return Err("injected numeric refusal");
        }
        Ok([self.seeds[ordinal]; 4])
    }
}
struct Mass {
    trace: Trace,
    dynamics: Arc<RwLock<SimulationDriveDynamics>>,
}
impl SkateboardWheelMassNumeric for Mass {
    fn reciprocal_estimate(&mut self, axis: Axis, value: f32) -> f32 {
        self.trace
            .borrow_mut()
            .push(Event::Mass(axis, value.to_bits()));
        let dynamics = self.dynamics.read().unwrap();
        assert_params(dynamics.angular, 2);
        assert_params(dynamics.linear, 2);
        value.recip()
    }
}
fn fixture() -> (SkateboardHookConstruction, Quaternion, Mass) {
    let owner = SkateboardHookConstruction::new();
    let trace = Trace::default();
    let quaternion = Quaternion {
        trace: Rc::clone(&trace),
        frames: Arc::clone(owner.frames()),
        dynamics: Arc::clone(owner.dynamics()),
        observations: Vec::new(),
        seeds: [0.25, 0.375],
        fail: None,
    };
    let mass = Mass {
        trace,
        dynamics: Arc::clone(owner.dynamics()),
    };
    (owner, quaternion, mass)
}
fn assert_params(params: SimulationDriveDynamicsParams, kind: u32) {
    assert_eq!(params.raw_type, kind);
    assert_eq!(
        [params.spring, params.damping, params.strength].map(f32::to_bits),
        [0; 3]
    );
}
fn poison<T>(owner: Arc<RwLock<T>>) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _guard = owner.write().unwrap();
        panic!("explicit synthetic payload-lock poison");
    }));
    assert!(result.is_err());
}

#[test]
fn fresh_constructor_initializes_only_retained_payloads_without_a_completed_definition() {
    let owner = SkateboardHookConstruction::new();
    assert_eq!(owner.stage(), SkateboardHookStage::Fresh);
    assert!(owner.definition().is_none());
    assert!(owner.template().is_none());
    let frames = owner.frames().read().unwrap();
    assert_eq!(
        frames.quaternion_a.map(f32::to_bits),
        [0, 0, 0, 0x3f80_0000]
    );
    assert_eq!(
        frames.quaternion_b.map(f32::to_bits),
        [0, 0, 0, 0x3f80_0000]
    );
    assert_eq!(frames.position_a.map(f32::to_bits), [0; 4]);
    assert_eq!(frames.position_b.map(f32::to_bits), [0; 4]);
    let dynamics = owner.dynamics().read().unwrap();
    assert_params(dynamics.angular, 0);
    assert_params(dynamics.linear, 0);
}

#[test]
fn two_identity_setters_are_separate_child_then_parent_before_hard_dynamics_and_mass() {
    let (mut owner, mut quaternion, mut mass) = fixture();
    owner.try_prepare(&mut quaternion, &mut mass).unwrap();
    assert_eq!(
        *quaternion.trace.borrow(),
        vec![
            Event::Quaternion([0x4080_0000, 0, 0, 0]),
            Event::Quaternion([0x4080_0000, 0, 0, 0]),
            Event::Mass(Axis::Y, 0x3f2a_aaac),
            Event::Mass(Axis::X, 0x3f2a_aaac),
            Event::Mass(Axis::Z, 0x3f2a_aaac),
        ]
    );
    let first = quaternion.observations[0];
    let second = quaternion.observations[1];
    assert_eq!(
        first.0.quaternion_a.map(f32::to_bits),
        [0, 0, 0, 0x3f80_0000]
    );
    // Exact binary-rational discriminator: seed1/4 at magnitude4, two rounds
    // gives W=0.8687744140625. A literal identity or normalization would fail.
    assert_eq!(
        second.0.quaternion_a[3].to_bits(),
        0.868_774_4_f32.to_bits()
    );
    assert_eq!(
        second.0.quaternion_b.map(f32::to_bits),
        [0, 0, 0, 0x3f80_0000]
    );
    for observation in quaternion.observations {
        assert_params(observation.1.angular, 0);
        assert_params(observation.1.linear, 0);
        assert_eq!(observation.0.position_a.map(f32::to_bits), [0; 4]);
        assert_eq!(observation.0.position_b.map(f32::to_bits), [0; 4]);
    }
    let frames = owner.frames().read().unwrap();
    assert_ne!(
        frames.quaternion_a[3].to_bits(),
        frames.quaternion_b[3].to_bits()
    );
    assert_ne!(frames.quaternion_b[3].to_bits(), 1.0_f32.to_bits());
    assert_eq!(owner.stage(), SkateboardHookStage::Complete);
}

#[test]
fn disabled_half_extent_box_keeps_native_defaults_and_direct_mass_suffix() {
    let (mut owner, mut quaternion, mut mass) = fixture();
    owner.try_prepare(&mut quaternion, &mut mass).unwrap();
    let definition = owner.definition().unwrap();
    let volume = definition.volume();
    assert_eq!(volume.vtable_offset, VolumeType::Box as u32);
    assert_eq!(volume.shape_words, [0x3dcc_cccd; 3]);
    assert_eq!(
        volume.transform.map(f32::to_bits),
        FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
    );
    assert_eq!(volume.transform[15].to_bits(), 0);
    assert_eq!(
        (
            volume.radius.to_bits(),
            volume.group_id,
            volume.surface_id,
            volume.flags
        ),
        (0, 0, 0, 0)
    );
    assert!(!definition.is_static_body());
    assert!(definition.inverse_body_transform().is_none());
    let material = definition.material();
    assert_eq!(
        [
            material.static_friction,
            material.dynamic_friction,
            material.restitution
        ]
        .map(f32::to_bits),
        [0x3f00_0000, 0x3e99_999a, 0x3ecc_cccd]
    );
    let inertia = definition.inertia();
    assert_eq!(inertia.inverse_mass.to_bits(), 0x3c23_d70a);
    assert_eq!(
        [inertia.max_velocity, inertia.max_omega].map(f32::to_bits),
        [0x7f7f_ffff; 2]
    );
    assert_eq!(
        [inertia.linear_drag, inertia.angular_drag].map(f32::to_bits),
        [0; 2]
    );
    assert_eq!(
        definition.inverse_tensor_w().to_bits(),
        inertia.inverse_tensor[0].to_bits()
    );
    // Ordinary host policy checks only; not native estimate/output evidence.
    for value in inertia.inverse_tensor {
        assert!((value - 1.5).abs() < 0.000_001);
    }
    assert!((inertia.spherical - 2.0 / 3.0).abs() < 0.000_001);
    // Independent scalar f32 calculation at radius0: x*x, sum, multiply1/3;
    // source scalar producer fingerprints are checked independently of tests.
    let (diagonal, amount) = fat_box_unit_mass([HALF_EXTENT; 3], 0.0);
    assert_eq!(diagonal.map(f32::to_bits), [0x3bda_740f; 3]);
    assert_eq!(amount.to_bits(), 0x3c03_126f);
}

#[test]
fn template_retains_one_real_geometry_and_shared_inertia_but_no_definition_constraints() {
    let (mut owner, mut quaternion, mut mass) = fixture();
    let frames = Arc::clone(owner.frames());
    let dynamics = Arc::clone(owner.dynamics());
    owner.try_prepare(&mut quaternion, &mut mass).unwrap();
    assert!(Arc::ptr_eq(&frames, owner.frames()));
    assert!(Arc::ptr_eq(&dynamics, owner.dynamics()));
    let definition = Arc::clone(owner.definition().unwrap());
    let template = Arc::clone(owner.template().unwrap());
    assert!(Arc::ptr_eq(template.collision(), &definition));
    assert_eq!(template.part_payloads().len(), 1);
    assert!(Arc::ptr_eq(
        template.part_payloads()[0].inertia.as_immutable().unwrap(),
        definition.inertia()
    ));
    assert_eq!(template.part_payloads()[0].material, *definition.material());
    assert!(template.joints().is_empty());
    assert!(template.drives().is_empty());
    let before = quaternion.trace.borrow().clone();
    assert_eq!(
        owner.try_prepare(&mut quaternion, &mut mass),
        Err(SkateboardHookError::NotFresh {
            stage: SkateboardHookStage::Complete
        })
    );
    assert_eq!(*quaternion.trace.borrow(), before);
    assert!(Arc::ptr_eq(owner.template().unwrap(), &template));
    let other = SkateboardHookConstruction::new();
    assert!(!Arc::ptr_eq(other.frames(), &frames));
    assert!(!Arc::ptr_eq(other.dynamics(), &dynamics));
    drop(owner);
    assert_eq!(template.collision().volume().shape_words, [0x3dcc_cccd; 3]);
    assert_params(dynamics.read().unwrap().angular, 2);
}

#[test]
fn numeric_refusal_retains_child_prefix_and_permanently_refuses_retry() {
    for failed in 0..2 {
        let (mut owner, mut quaternion, mut mass) = fixture();
        quaternion.fail = Some(failed);
        assert_eq!(
            owner.try_prepare(&mut quaternion, &mut mass),
            Err(SkateboardHookError::Numeric("injected numeric refusal"))
        );
        let stage = if failed == 0 {
            SkateboardHookStage::ChildFrame
        } else {
            SkateboardHookStage::ParentFrame
        };
        assert_eq!(owner.stage(), stage);
        assert!(owner.definition().is_none());
        assert!(owner.template().is_none());
        let frames = *owner.frames().read().unwrap();
        assert_eq!(frames.quaternion_b[3].to_bits(), 1.0_f32.to_bits());
        assert_eq!(
            frames.quaternion_a[3].to_bits(),
            if failed == 0 {
                1.0_f32.to_bits()
            } else {
                0.868_774_4_f32.to_bits()
            }
        );
        assert_params(owner.dynamics().read().unwrap().angular, 0);
        assert_params(owner.dynamics().read().unwrap().linear, 0);
        let before = quaternion.trace.borrow().clone();
        quaternion.fail = None;
        assert_eq!(
            owner.try_prepare(&mut quaternion, &mut mass),
            Err(SkateboardHookError::NotFresh { stage })
        );
        assert_eq!(*quaternion.trace.borrow(), before);
    }
}

#[test]
fn frame_poison_refuses_at_position_store_before_any_numeric_callback() {
    let (mut owner, mut quaternion, mut mass) = fixture();
    poison(Arc::clone(owner.frames()));
    assert_eq!(
        owner.try_prepare(&mut quaternion, &mut mass),
        Err(SkateboardHookError::Poisoned {
            payload: SkateboardHookPayload::Frames
        })
    );
    assert_eq!(owner.stage(), SkateboardHookStage::Positions);
    assert!(quaternion.trace.borrow().is_empty());
    assert!(owner.template().is_none());
}

#[test]
fn dynamics_poison_is_not_preflighted_before_both_frame_setters() {
    // This provider does not read dynamics, allowing the source numeric work
    // to precede the later host poisoned-lock refusal as it must.
    struct Count(usize);
    impl SkateboardTruckDriveNumeric for Count {
        type Error = &'static str;
        fn sine_cosine(&mut self, _: f32) -> Result<(f32, f32), Self::Error> {
            panic!("no SinCos")
        }
        fn quaternion_reciprocal_square_root_estimate(
            &mut self,
            _: [f32; 4],
        ) -> Result<[f32; 4], Self::Error> {
            self.0 += 1;
            Ok([0.25; 4])
        }
    }
    let mut owner = SkateboardHookConstruction::new();
    poison(Arc::clone(owner.dynamics()));
    let mut numeric = Count(0);
    assert_eq!(
        owner.try_prepare(&mut numeric, &mut HostSkateboardWheelMassNumeric),
        Err(SkateboardHookError::Poisoned {
            payload: SkateboardHookPayload::Dynamics
        })
    );
    assert_eq!(numeric.0, 2);
    assert_eq!(owner.stage(), SkateboardHookStage::Dynamics);
    let frames = owner.frames().read().unwrap();
    assert_eq!(frames.quaternion_a[3].to_bits(), 0.868_774_4_f32.to_bits());
    assert_eq!(frames.quaternion_b[3].to_bits(), 0.868_774_4_f32.to_bits());
    assert!(owner.definition().is_none());
}

#[test]
fn zero_and_exceptional_seeds_are_not_replaced_by_identity_or_normalized() {
    let (mut owner, mut quaternion, mut mass) = fixture();
    quaternion.seeds = [0.0, f32::NAN];
    owner.try_prepare(&mut quaternion, &mut mass).unwrap();
    let frames = owner.frames().read().unwrap();
    assert_eq!(frames.quaternion_a.map(f32::to_bits), [0; 4]);
    assert!(frames.quaternion_b.into_iter().all(f32::is_nan));
    assert_eq!(frames.position_a.map(f32::to_bits), [0; 4]);
    assert_eq!(frames.position_b.map(f32::to_bits), [0; 4]);
    assert!(owner.template().is_some());
}
