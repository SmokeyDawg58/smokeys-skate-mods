//! Actual pool storage and synthetic authored input, not humanoid admission or
//! a native execution trace. Cosine callbacks deliberately inspect live owners.
#![expect(
    clippy::float_cmp,
    reason = "exact copied source values and representable test arithmetic"
)]
use super::*;
use crate::{
    simulation_rigid_body_pool::{SimulationInertia, SimulationRigidBodyPool},
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};
use std::sync::Arc;

fn initial_limits() -> SimulationJointLimits {
    SimulationJointLimits {
        p_prism: [1.0, 2.0, 3.0, 4.0],
        v_prism: [5.0, 6.0, 7.0, 8.0],
        v_twist: 9.0,
        v_swing: 10.0,
        swing_a: 11.0,
        twist_a: 12.0,
        swing_c: 13.0,
        twist_c: 14.0,
        raw_swing_type: 4,
        raw_twist_type: 2,
    }
}

fn inputs(children: &[usize]) -> (SkeletonData, SkeletonBodyAttributeData) {
    let (mut hierarchy, mut parameters, pose) = crate::skeleton_data_tests::fixture();
    hierarchy.parents.fill(-1);
    for child in children {
        hierarchy.parents[*child] = 23;
    }
    for bone in &mut parameters.bones {
        bone.swing = 2.0;
        bone.twist = 3.0;
    }
    let data = crate::skeleton_data_tests::build(hierarchy, parameters, &pose).unwrap();
    let (_, mut attributes) = crate::skeleton_body_definition::tests::inputs();
    for (i, row) in attributes.joints.iter_mut().enumerate() {
        row.swing_angle = f32::from(u8::try_from(i + 1).unwrap()) * 0.25;
        row.twist_angle = f32::from(u8::try_from(i + 1).unwrap()) * 0.5;
        row.ball_joint = 0xff; // Normal setup does not inspect this flag.
    }
    (data, attributes)
}

fn live_bindings(
    count: usize,
) -> (
    SimulationRigidBodyPool,
    SimulationJointPool<SimulationJointFrames, SimulationJointLimits>,
    Vec<SkeletonJointLimitBinding>,
) {
    let mut bodies = SimulationRigidBodyPool::try_new(2).unwrap();
    let inertia = Arc::new(SimulationInertia::native_default());
    let parent = bodies
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, Some(&inertia), 4)
        .unwrap()
        .unwrap();
    let child = bodies
        .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, Some(&inertia), 4)
        .unwrap()
        .unwrap();
    let mut pool = SimulationJointPool::try_new(count).unwrap();
    let frames = Arc::new(SimulationJointFrames {
        quaternion_a: [0.0, 0.0, 0.0, 1.0],
        position_a: [0.0; 4],
        quaternion_b: [0.0, 0.0, 0.0, 1.0],
        position_b: [0.0; 4],
        quaternion_l: [0.0, 0.0, 0.0, 1.0],
    });
    let bindings = (0..count)
        .map(|_| {
            let limits = SimulationMutableJointLimits::new(initial_limits());
            let joint = pool
                .try_add_joint_mutable_limits(parent, child, &frames, &limits)
                .unwrap()
                .unwrap();
            SkeletonJointLimitBinding { joint, limits }
        })
        .collect();
    (bodies, pool, bindings)
}

#[test]
fn callbacks_observe_source_partial_stores_and_ball_flags_do_not_gate_normal_limits() {
    let (data, attributes) = inputs(&[1, 2]);
    let (bodies, mut pool, bindings) = live_bindings(2);
    let topology = pool.snapshot();
    let body_snapshot = bodies.snapshot();
    let mut observations = Vec::new();
    try_set_normal_joint_limits(&data, &attributes, &mut pool, &bindings, &mut |angle| {
        observations.push((
            angle,
            bindings
                .iter()
                .map(|binding| binding.limits.try_read().unwrap())
                .collect::<Vec<_>>(),
        ));
        Ok::<_, ()>(angle + 100.0)
    })
    .unwrap();
    let old = initial_limits();
    let first_a = SimulationJointLimits {
        swing_a: 0.5,
        ..old
    };
    let first_b = SimulationJointLimits {
        swing_c: 100.5,
        twist_a: 1.5,
        ..first_a
    };
    let first_done = SimulationJointLimits {
        twist_c: 101.5,
        ..first_b
    };
    let second_a = SimulationJointLimits {
        swing_a: 1.0,
        ..old
    };
    let second_b = SimulationJointLimits {
        swing_c: 101.0,
        twist_a: 3.0,
        ..second_a
    };
    let second_done = SimulationJointLimits {
        twist_c: 103.0,
        ..second_b
    };
    assert_eq!(
        observations,
        [
            (0.5, vec![first_a, old]),
            (1.5, vec![first_b, old]),
            (1.0, vec![first_done, second_a]),
            (3.0, vec![first_done, second_b])
        ]
    );
    assert_eq!(pool.try_limits(bindings[0].joint).unwrap(), first_done);
    assert_eq!(pool.try_limits(bindings[1].joint).unwrap(), second_done);
    assert_eq!(pool.snapshot(), topology);
    assert_eq!(bodies.snapshot(), body_snapshot);
}

#[test]
fn cosine_failure_retains_exact_current_and_earlier_row_prefixes() {
    let (data, attributes) = inputs(&[1, 2]);
    for fail_at in 0..4 {
        let (_bodies, mut pool, bindings) = live_bindings(2);
        let mut calls = 0;
        let result =
            try_set_normal_joint_limits(&data, &attributes, &mut pool, &bindings, &mut |angle| {
                let current = calls;
                calls += 1;
                if current == fail_at {
                    Err("cosine")
                } else {
                    Ok(angle + 100.0)
                }
            });
        assert_eq!(calls, fail_at + 1);
        assert!(
            matches!(result, Err(SkeletonNormalJointLimitsError::Cosine { row, source: "cosine", .. }) if row == fail_at / 2)
        );
        for (row, binding) in bindings.iter().enumerate() {
            let mut expected = initial_limits();
            let first_call = row * 2;
            let scale = f32::from(u8::try_from(row + 1).unwrap());
            if fail_at >= first_call {
                expected.swing_a = 0.5 * scale;
            }
            if fail_at > first_call {
                expected.swing_c = expected.swing_a + 100.0;
                expected.twist_a = 1.5 * scale;
            }
            if fail_at > first_call + 1 {
                expected.twist_c = expected.twist_a + 100.0;
            }
            assert_eq!(binding.limits.try_read().unwrap(), expected);
        }
    }
}

#[test]
fn missing_dense_row_refuses_after_completed_prefix_without_using_emission_map() {
    let (data, attributes) = inputs(&[1, 2]);
    let (_bodies, mut pool, bindings) = live_bindings(1);
    let mut calls = 0;
    let result =
        try_set_normal_joint_limits(&data, &attributes, &mut pool, &bindings, &mut |angle| {
            calls += 1;
            Ok::<_, ()>(angle)
        });
    assert_eq!(
        result,
        Err(SkeletonNormalJointLimitsError::MissingInitializedRow {
            child: SkeletonPartIndex::Neck,
            row: 1
        })
    );
    assert_eq!(calls, 2);
    assert_eq!(
        bindings[0].limits.try_read().unwrap(),
        SimulationJointLimits {
            swing_a: 0.5,
            swing_c: 0.5,
            twist_a: 1.5,
            twist_c: 1.5,
            ..initial_limits()
        }
    );
}

#[test]
fn stale_or_wrong_owner_refusal_precedes_cosine_and_has_no_limit_writes() {
    let (data, attributes) = inputs(&[1]);
    for stale in [false, true] {
        let (_bodies, mut pool, mut bindings) = live_bindings(1);
        let original = bindings[0].limits.clone();
        if stale {
            pool.try_remove_joint(bindings[0].joint).unwrap();
        } else {
            bindings[0].limits = SimulationMutableJointLimits::new(initial_limits());
        }
        let result = try_set_normal_joint_limits(
            &data,
            &attributes,
            &mut pool,
            &bindings,
            &mut |_| -> Result<f32, ()> { panic!("refuse before cosine") },
        );
        assert!(matches!(
            result,
            Err(SkeletonNormalJointLimitsError::Pool { row: 0, .. })
        ));
        assert_eq!(original.try_read().unwrap(), initial_limits());
        assert_eq!(bindings[0].limits.try_read().unwrap(), initial_limits());
    }
}

#[test]
fn zero_eligible_parts_do_not_require_pool_rows_or_numeric_calls() {
    let (data, attributes) = inputs(&[]);
    let (_bodies, mut pool, bindings) = live_bindings(0);
    try_set_normal_joint_limits(
        &data,
        &attributes,
        &mut pool,
        &bindings,
        &mut |_| -> Result<f32, ()> { panic!("no eligible child") },
    )
    .unwrap();
}

#[test]
fn normal_floor_keeps_original_angle_and_selects_epsilon_for_unordered() {
    assert_eq!(minimum_angle(0.5), 0.5);
    assert_eq!(minimum_angle(f32::INFINITY), f32::INFINITY);
    for value in [f32::NAN, -0.0, 0.0, -1.0, f32::from_bits(0x3c23_d70a)] {
        assert_eq!(minimum_angle(value).to_bits(), 0x3c23_d70a);
    }
}
