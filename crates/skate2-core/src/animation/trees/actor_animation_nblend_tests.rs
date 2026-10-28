use super::*;
fn collector() -> TreeCommands {
    let (program, _) = TreeCommands::fixed_test_collector();
    {
        let mut owner = program.owner.lock().unwrap();
        owner.in_commands = 1;
        owner.in_command_set = 1;
        owner.nblend.maximum_lists = 4;
        owner.nblend.maximum_other = 8;
        owner.nblend.maximum_commands = 20;
    }
    program
}
fn sqt(value: f32, w: f32) -> AndaleSqt {
    AndaleSqt {
        scale: [value; 4],
        rotation: [0., 0., 0., w],
        translation: [value; 4],
    }
}
#[test]
fn later_alias_preserves_contributing_pose() {
    let mut program = collector();
    program.nblends.push(NBlendRecord {
        native_index: 0,
        inputs: vec![0, 1],
        coefficients: vec![0.25, 0.75],
    });
    let mut values = vec![Some(vec![sqt(8., 1.)]), Some(vec![sqt(4., 1.)])];
    program.execute_n_blend(0, 2, 1, &mut values, 1).unwrap();
    let result = values[1].as_ref().unwrap()[0];
    assert_eq!(result.translation.map(f32::to_bits), [5_f32.to_bits(); 4]);
    assert_eq!(result.scale.map(f32::to_bits), [5_f32.to_bits(); 4]);
    assert_eq!(result.rotation[3].to_bits(), 1_f32.to_bits());
}
#[test]
fn walking_zero_first_weight_matches_nonaliased_blend() {
    let mut program = collector();
    program.nblends.push(NBlendRecord {
        native_index: 0,
        inputs: vec![0, 1, 2, 3],
        coefficients: vec![0., 1., 0., 0.],
    });
    let mut aliased = vec![
        Some(vec![sqt(8., 1.)]),
        Some(vec![sqt(4., 1.)]),
        Some(vec![sqt(2., 1.)]),
        Some(vec![sqt(1., 1.)]),
        None,
    ];
    let mut distinct = aliased.clone();
    program.execute_n_blend(0, 4, 1, &mut aliased, 1).unwrap();
    program.execute_n_blend(0, 4, 4, &mut distinct, 1).unwrap();
    let actual = aliased[1].as_ref().unwrap()[0];
    let expected = distinct[4].as_ref().unwrap()[0];
    assert_eq!(
        actual.rotation.map(f32::to_bits),
        expected.rotation.map(f32::to_bits)
    );
    assert_eq!(
        actual.translation.map(f32::to_bits),
        expected.translation.map(f32::to_bits)
    );
    assert_eq!(
        actual.scale.map(f32::to_bits),
        expected.scale.map(f32::to_bits)
    );
    assert!(actual.rotation.iter().all(|v| v.is_finite()));
}
#[test]
fn opposite_quaternions_cancel_without_binary_hemisphere_correction() {
    let mut program = collector();
    program.nblends.push(NBlendRecord {
        native_index: 0,
        inputs: vec![0, 1],
        coefficients: vec![0.5, 0.5],
    });
    let mut values = vec![Some(vec![sqt(2., 1.)]), Some(vec![sqt(4., -1.)]), None];
    program.execute_n_blend(0, 2, 2, &mut values, 1).unwrap();
    let result = values[2].as_ref().unwrap()[0];
    assert!(result.rotation.iter().all(|v| v.is_nan()));
    assert_eq!(result.translation.map(f32::to_bits), [3_f32.to_bits(); 4]);
}
#[test]
fn invalid_coefficients_publish_counters_without_popping_stack() {
    let mut program = collector();
    program.stack.extend([0, 1]);
    program
        .owner
        .lock()
        .unwrap()
        .commands
        .extend([Command::FetchPose { guid: 0, sqt: 0 }; 2]);
    assert!(matches!(
        program.emit_n_blend(&[f32::NAN, 1.]),
        Err(BindPoseError::NBlend(4))
    ));
    assert_eq!(program.stack, [0, 1]);
    let owner = program.owner.lock().unwrap();
    assert_eq!(owner.nblend.list_count, 1);
    assert_eq!(owner.nblend.other_count, 1);
    assert!(program.nblends.is_empty());
}
#[test]
fn list_count_spans_streams_and_usage_resets_only_at_end_set() {
    let mut first = collector();
    first.owner.lock().unwrap().nblend.maximum_lists = 0;
    first.stack.push(0);
    first
        .owner
        .lock()
        .unwrap()
        .commands
        .push(Command::FetchPose { guid: 0, sqt: 0 });
    assert!(matches!(
        first.emit_n_blend(&[1.]),
        Err(BindPoseError::NBlendListBounds)
    ));
    let mut second = TreeCommands::new(std::sync::Arc::clone(&first.owner));
    second.stack.push(0);
    second.owner.lock().unwrap().nblend.begin_commands();
    assert!(matches!(
        second.emit_n_blend(&[1.]),
        Err(BindPoseError::NBlend(64))
    ));
    let mut owner = first.owner.lock().unwrap();
    assert_eq!(owner.nblend.list_count, 1);
    assert_eq!(owner.nblend.other_count, 2);
    owner.nblend.end_command_set();
    assert_eq!(owner.nblend.list_count, 1);
    assert_eq!(owner.nblend.other_count, 0);
    owner.nblend.synchronized();
    assert_eq!(owner.nblend.list_count, 0);
}
#[test]
fn dynamic_input_at_index_zero_still_allocates_native_new_output() {
    let mut program = collector();
    program.next_sqt = 1;
    program
        .stack
        .extend([0, super::super::fixed::FIXED_SQT_BIT]);
    program
        .owner
        .lock()
        .unwrap()
        .commands
        .extend([Command::FetchPose { guid: 0, sqt: 0 }; 2]);
    program.emit_n_blend(&[0.5, 0.5]).unwrap();
    assert_eq!(program.output().unwrap(), 1);
    assert_eq!(program.dynamic_count(), 2);
    assert_eq!(
        program.nblends[0].inputs,
        [0, super::super::fixed::FIXED_SQT_BIT]
    );
}
#[test]
fn native_dimension_limit_and_rounded_coefficient_validation() {
    assert_eq!(coefficient_errors(&[0.25; 4]), 0);
    assert_eq!(coefficient_errors(&[]), 4);
    assert_eq!(coefficient_errors(&[1., 0., 0., 0., 0., 0.]), 4);
    assert_eq!(coefficient_errors(&[f32::INFINITY]), 4);
}

#[test]
fn weighted_add_retains_fused_cancellation_residual() {
    let mut program = collector();
    let epsilon = f32::from_bits(0x3400_0000); // 2^-23
    program.nblends.push(NBlendRecord {
        native_index: 0,
        inputs: vec![0, 1],
        coefficients: vec![epsilon, 1.0 - epsilon],
    });
    let mut values = vec![
        Some(vec![sqt(-8_388_608., 1.)]),
        Some(vec![sqt(1.0 + epsilon, 1.)]),
        None,
    ];
    program.execute_n_blend(0, 2, 2, &mut values, 1).unwrap();
    // (1+2^-23)*(1-2^-23)-1 = -2^-46. Separate multiply loses this.
    assert_eq!(
        values[2].as_ref().unwrap()[0].translation[0].to_bits(),
        0xa880_0000
    );
}

#[test]
fn late_unavailable_source_bone_retains_only_written_prefix() {
    let mut program = collector();
    program.nblends.push(NBlendRecord {
        native_index: 0,
        inputs: vec![0],
        coefficients: vec![1.],
    });
    let mut values = vec![Some(vec![sqt(7., 1.)]), None];
    assert!(matches!(
        program.execute_n_blend(0, 1, 1, &mut values, 2),
        Err(BindPoseError::InvalidHierarchy)
    ));
    assert_eq!(values[1].as_ref().unwrap().len(), 1);
    assert_eq!(
        values[1].as_ref().unwrap()[0].translation[0].to_bits(),
        7_f32.to_bits()
    );
}
