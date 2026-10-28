//! Synthetic pool/numeric discriminators, not native runtime traces.
#![expect(
    clippy::float_cmp,
    reason = "exact source/copied values; signed zero, exceptional payloads and fused discriminators use bit comparisons"
)]
use super::*;
use crate::simulation_drive_pool::{
    SimulationDriveDynamics, SimulationDriveDynamicsParams, SimulationDriveFrames,
    SimulationDriveHandle,
};
use crate::simulation_rigid_body_pool::{SimulationRigidBodyHandle, SimulationRigidBodyPool};
use std::sync::{Arc, RwLock};

struct Fixture {
    bodies: SimulationRigidBodyPool,
    body: [SimulationRigidBodyHandle; 2],
    pool: SimulationDrivePool,
    handles: Vec<SimulationDriveHandle>,
    frames: Vec<Arc<RwLock<SimulationDriveFrames>>>,
    dynamics: Vec<Arc<RwLock<SimulationDriveDynamics>>>,
}
fn fixture(count: usize) -> Fixture {
    let mut bodies = SimulationRigidBodyPool::try_new(2).unwrap();
    let identity = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    // Neither endpoint has the Build Active bit. They remain real public-Add
    // occurrences; normalization is not allowed to inspect/filter their states.
    let body = std::array::from_fn(|_| {
        bodies
            .try_add_rigid_body(identity, None, 1)
            .unwrap()
            .unwrap()
    });
    let mut pool = SimulationDrivePool::try_new(count, &bodies).unwrap();
    let mut frames = Vec::new();
    let mut dynamics = Vec::new();
    let mut handles = Vec::new();
    for index in 0..count {
        let scale = f32::from(u16::try_from(index + 2).unwrap());
        let frame = Arc::new(RwLock::new(SimulationDriveFrames {
            quaternion_a: [0.0, 0.0, 0.0, scale],
            position_a: [f32::from_bits(0x7fc0_1234), -0.0, 3.0, 4.0],
            quaternion_b: [scale, 0.0, 0.0, 0.0],
            position_b: [5.0, 6.0, -0.0, f32::from_bits(0xffc0_5678)],
        }));
        let params = SimulationDriveDynamicsParams {
            spring: -0.0,
            damping: f32::from_bits(0x7fc0_9876),
            strength: 9.0,
            raw_type: 0,
        };
        let dynamic = Arc::new(RwLock::new(SimulationDriveDynamics {
            linear: params,
            angular: params,
        }));
        handles.push(
            pool.try_add_drive(&bodies, body[0], body[1], &frame, &dynamic)
                .unwrap()
                .unwrap(),
        );
        frames.push(frame);
        dynamics.push(dynamic);
    }
    Fixture {
        bodies,
        body,
        pool,
        handles,
        frames,
        dynamics,
    }
}
#[derive(Clone, Debug, PartialEq)]
enum Event {
    Norm([u32; 4]),
    Seed([u32; 4]),
}
#[derive(Default)]
struct Numeric {
    events: Vec<Event>,
    seed: Option<[f32; 4]>,
    fail: Option<usize>,
}
impl SimulationDriveNormalizationNumeric for Numeric {
    type Error = &'static str;
    fn norm4(&mut self, q: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        let ordinal = self.events.len();
        self.events.push(Event::Norm(q.map(f32::to_bits)));
        if self.fail == Some(ordinal) {
            return Err("numeric");
        }
        Ok(HostSimulationDriveNormalizationNumeric.norm4(q).unwrap())
    }
    fn rsqrt_estimate(&mut self, n: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        let ordinal = self.events.len();
        self.events.push(Event::Seed(n.map(f32::to_bits)));
        if self.fail == Some(ordinal) {
            return Err("numeric");
        }
        Ok(self
            .seed
            .unwrap_or_else(|| n.map(|value| value.sqrt().recip())))
    }
}
fn bits(frame: &Arc<RwLock<SimulationDriveFrames>>) -> [[u32; 4]; 4] {
    let f = frame.read().unwrap();
    [f.quaternion_a, f.position_a, f.quaternion_b, f.position_b]
        .map(|value| value.map(f32::to_bits))
}
fn dynamics_bits(owner: &Arc<RwLock<SimulationDriveDynamics>>) -> [[u32; 4]; 2] {
    let d = owner.read().unwrap();
    [d.linear, d.angular].map(|p| {
        [
            p.spring.to_bits(),
            p.damping.to_bits(),
            p.strength.to_bits(),
            p.raw_type,
        ]
    })
}

#[test]
fn empty_ring_has_no_numeric_calls_or_metadata_changes() {
    let mut f = fixture(0);
    let mut numeric = Numeric::default();
    let before = f.pool.snapshot();
    f.pool
        .try_normalize_active_drive_frames(&mut numeric)
        .unwrap();
    assert!(numeric.events.is_empty());
    assert_eq!(f.pool.snapshot(), before);
}

#[test]
fn all_active_drives_normalize_even_static_endpoints_and_no_drive_dynamics() {
    let mut f = fixture(1);
    let mut numeric = Numeric::default();
    assert!(f.pool.try_active_build_order(&f.bodies).unwrap().is_empty());
    let before = bits(&f.frames[0]);
    let dynamics = dynamics_bits(&f.dynamics[0]);
    let metadata = f.pool.snapshot();
    f.pool
        .try_normalize_active_drive_frames(&mut numeric)
        .unwrap();
    assert_eq!(
        numeric.events,
        vec![
            Event::Norm([0, 0, 0, 2.0_f32.to_bits()]),
            Event::Norm([2.0_f32.to_bits(), 0, 0, 0]),
            Event::Seed([4.0_f32.to_bits(); 4]),
            Event::Seed([4.0_f32.to_bits(); 4])
        ]
    );
    let after = bits(&f.frames[0]);
    assert_eq!(after[0], [0, 0, 0, 1.0_f32.to_bits()]);
    assert_eq!(after[2], [1.0_f32.to_bits(), 0, 0, 0]);
    assert_eq!(after[1], before[1]);
    assert_eq!(after[3], before[3]);
    assert_eq!(dynamics_bits(&f.dynamics[0]), dynamics);
    assert_eq!(f.pool.snapshot(), metadata);
}

#[test]
fn null_frame_and_unrelated_missing_fields_do_not_inherit_build_validation() {
    let mut f = fixture(2);
    let mut numeric = Numeric::default();
    // Deliberately malformed payload fixtures only: public Add cannot produce
    // these. They distinguish native normalization reads from Build checks.
    f.pool.slots[0].frames = None;
    for slot in &mut f.pool.slots {
        slot.dynamics = None;
        slot.body_a = None;
        slot.body_b = None;
        slot.spy = None;
        slot.occurrence = None;
    }
    assert!(f.pool.validate_integrity().is_err()); // existing invariant NOT weakened
    f.pool
        .try_normalize_active_drive_frames(&mut numeric)
        .unwrap();
    assert_eq!(numeric.events.len(), 4);
    assert_eq!(numeric.events[0], Event::Norm([0, 0, 0, 3.0_f32.to_bits()]));
    assert_eq!(bits(&f.frames[0])[0][3], 2.0_f32.to_bits());
}

#[test]
fn both_refinement_rounds_and_vector_lanes_have_frozen_exact_dyadic_discriminators() {
    // norm4, seed1/4: first round11/32, second7117/16384. These
    // dyadic intermediate calculations are exact in f32, not captured Xenon seeds.
    let (a, b) = refine_pair(
        [4.0; 4],
        [4.0; 4],
        [0.25, 0.5, 0.0, 1.0],
        [0.5, 0.25, 1.0, 0.0],
    );
    assert_eq!(
        a.map(f32::to_bits),
        [0x3ede_6800, 0x3f00_0000, 0, 0xbf00_0000]
    );
    assert_eq!(
        b.map(f32::to_bits),
        [0x3f00_0000, 0x3ede_6800, 0xbf00_0000, 0]
    );
    assert_ne!(a[0], 0.25);
    assert_ne!(a[0], 11.0 / 32.0);
    let mut f = fixture(1);
    let mut numeric = Numeric {
        seed: Some([0.25; 4]),
        ..Numeric::default()
    };
    f.pool
        .try_normalize_active_drive_frames(&mut numeric)
        .unwrap();
    assert_eq!(bits(&f.frames[0])[0][3], 0x3f5e_6800);
}

#[test]
fn free_occurrences_are_skipped_even_while_their_old_frame_owner_is_retained() {
    let mut f = fixture(2);
    f.pool.try_remove_drive(f.handles[0]).unwrap();
    let first = bits(&f.frames[0]);
    let mut numeric = Numeric::default();
    f.pool
        .try_normalize_active_drive_frames(&mut numeric)
        .unwrap();
    assert_eq!(bits(&f.frames[0]), first);
    assert_eq!(numeric.events.len(), 4);
    assert_eq!(numeric.events[0], Event::Norm([0, 0, 0, 3.0_f32.to_bits()]));
}

#[test]
fn fifo_readd_and_shared_owner_are_visited_twice_using_then_current_values() {
    let mut f = fixture(2);
    f.pool.try_remove_drive(f.handles[0]).unwrap();
    let new = f
        .pool
        .try_add_drive(
            &f.bodies,
            f.body[0],
            f.body[1],
            &f.frames[1],
            &f.dynamics[1],
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        f.pool.snapshot().active_order,
        vec![f.handles[1].id(), new.id()]
    );
    let mut numeric = Numeric {
        seed: Some([0.1; 4]),
        ..Numeric::default()
    };
    let original = *f.frames[1].read().unwrap();
    let norm = HostSimulationDriveNormalizationNumeric
        .norm4(original.quaternion_a)
        .unwrap();
    let (refined, _) = refine_pair(norm, norm, [0.1; 4], [0.1; 4]);
    let once: [f32; 4] = std::array::from_fn(|lane| original.quaternion_a[lane] * refined[lane]);
    f.pool
        .try_normalize_active_drive_frames(&mut numeric)
        .unwrap();
    assert_eq!(numeric.events.len(), 8);
    assert_eq!(numeric.events[4], Event::Norm(once.map(f32::to_bits)));
    assert_ne!(bits(&f.frames[1])[0], once.map(f32::to_bits));
    assert_eq!(bits(&f.frames[0])[0][3], 2.0_f32.to_bits());
}

#[test]
fn later_numeric_failure_preserves_prefix_and_never_half_publishes_current_pair() {
    for failed_call in 4..8 {
        let mut f = fixture(2);
        let before = bits(&f.frames[1]);
        let mut numeric = Numeric {
            fail: Some(failed_call),
            ..Numeric::default()
        };
        assert_eq!(
            f.pool.try_normalize_active_drive_frames(&mut numeric),
            Err(SimulationDriveNormalizationError::Numeric {
                drive: f.handles[1].id(),
                source: "numeric"
            })
        );
        assert_eq!(bits(&f.frames[0])[0][3], 1.0_f32.to_bits());
        assert_eq!(bits(&f.frames[1]), before);
    }
}

#[test]
fn later_frame_poison_retains_prefix_and_poisoned_dynamics_is_not_read() {
    let mut f = fixture(2);
    let owner = Arc::clone(&f.frames[1]);
    let _ = std::panic::catch_unwind(move || {
        let _guard = owner.write().unwrap();
        panic!("synthetic poison");
    });
    let dynamic = Arc::clone(&f.dynamics[0]);
    let _ = std::panic::catch_unwind(move || {
        let _guard = dynamic.write().unwrap();
        panic!("unrelated poison");
    });
    let mut numeric = Numeric::default();
    assert_eq!(
        f.pool.try_normalize_active_drive_frames(&mut numeric),
        Err(SimulationDriveNormalizationError::FrameLockPoisoned {
            drive: f.handles[1].id()
        })
    );
    assert_eq!(bits(&f.frames[0])[0][3], 1.0_f32.to_bits());
    assert_eq!(numeric.events.len(), 4);
}

#[test]
fn topology_corruption_refuses_before_any_callback_without_using_payload_integrity() {
    let mut f = fixture(1);
    let before = bits(&f.frames[0]);
    f.pool.slots[0].links.left = SimulationDriveNode::Drive(f.handles[0].id());
    let mut numeric = Numeric::default();
    assert_eq!(
        f.pool.try_normalize_active_drive_frames(&mut numeric),
        Err(SimulationDriveNormalizationError::CorruptTopology)
    );
    assert!(numeric.events.is_empty());
    assert_eq!(bits(&f.frames[0]), before);
}

#[test]
fn zero_and_nonfinite_quaternions_are_not_replaced_with_identity_or_rejected() {
    for qa in [
        [0.0; 4],
        [f32::INFINITY, 0.0, 0.0, 0.0],
        [f32::NAN, 0.0, 0.0, 1.0],
    ] {
        let mut f = fixture(1);
        f.frames[0].write().unwrap().quaternion_a = qa;
        f.pool
            .try_normalize_active_drive_frames(&mut HostSimulationDriveNormalizationNumeric)
            .unwrap();
        let frame = f.frames[0].read().unwrap();
        assert!(frame.quaternion_a.iter().all(|value| value.is_nan()));
        assert_eq!(frame.quaternion_b, [1.0, 0.0, 0.0, 0.0]);
    }
}

#[test]
fn host_norm_policy_includes_w_and_unit_quaternion_signed_zeros_survive() {
    let mut host = HostSimulationDriveNormalizationNumeric;
    assert_eq!(host.norm4([1.0, 2.0, 3.0, 4.0]).unwrap(), [30.0; 4]);
    assert_eq!(
        host.rsqrt_estimate([1.0, 4.0, 16.0, 64.0]).unwrap(),
        [1.0, 0.5, 0.25, 0.125]
    );
    let mut f = fixture(1);
    f.frames[0].write().unwrap().quaternion_a = [-0.0, 0.0, -0.0, 1.0];
    f.pool.try_normalize_active_drive_frames(&mut host).unwrap();
    assert_eq!(
        bits(&f.frames[0])[0],
        [0x8000_0000, 0, 0x8000_0000, 0x3f80_0000]
    );
}

#[test]
fn both_quaternion_refinement_graphs_preserve_fusion() {
    // Independent frozen discriminator already established by world.rs.
    let norm = f32::from_bits(0x68d3_92ce);
    let seed = norm.sqrt().recip();
    let (a, b) = refine_pair([norm; 4], [norm; 4], [seed; 4], [seed; 4]);
    assert_eq!(a.map(f32::to_bits), [0x2ac7_1ea9; 4]);
    assert_eq!(b.map(f32::to_bits), [0x2ac7_1ea9; 4]);
    let mut split = seed;
    for _ in 0..2 {
        let square = split * split;
        let half = split * 0.5;
        let residual = 1.0 - norm * square;
        split += half * residual;
    }
    assert_eq!(split.to_bits(), 0x2ac7_1ea8);
}
