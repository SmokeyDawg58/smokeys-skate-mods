use super::*;

struct Input {
    value: Result<u8, &'static str>,
    reads: usize,
}
impl SkeletonAnimationRecordInput for Input {
    type Error = &'static str;
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
        self.reads += 1;
        self.value
    }
}

fn input(value: u8) -> Input {
    Input {
        value: Ok(value),
        reads: 0,
    }
}

fn masses(values: [f32; 24]) -> SkeletonPhysicsHistory {
    let mut result = SkeletonPhysicsHistory::new(values);
    result.try_initialize_masses().unwrap();
    result
}

fn one_part_mass() -> SkeletonPhysicsHistory {
    let mut values = [0.0; 24];
    values[1] = 1.0;
    masses(values)
}

fn pose() -> [AnimationMatrix; 24] {
    [FRESH_VOLUME_TRANSFORM; 24]
}

fn words(value: SkeletonRootVector) -> ([u32; 3], Option<u32>) {
    (value.xyz.map(f32::to_bits), value.w.map(f32::to_bits))
}

#[test]
fn constructor_uses_literal_source_identity_and_zero_vectors() {
    let record = SkeletonAnimationRecord::new();
    let identity = [
        0x3f80_0000,
        0,
        0,
        0,
        0,
        0x3f80_0000,
        0,
        0,
        0,
        0,
        0x3f80_0000,
        0,
        0,
        0,
        0,
        0,
    ];
    for matrix in record.poses() {
        assert_eq!(matrix.map(f32::to_bits), identity);
    }
    assert_eq!(record.centre_of_mass().map(f32::to_bits), [0; 4]);
    assert_eq!(record.centre_of_mass_delta().map(f32::to_bits), [0; 4]);
    assert_eq!(words(record.com_to_deck()), ([0; 3], Some(0)));
    assert_eq!(words(record.com_to_deck_delta()), ([0; 3], Some(0)));
    assert_eq!(record.reset_scalar().to_bits(), 0);
}

#[test]
fn invalid_mass_copies_all_rows_before_refusing_without_cvar() {
    let mut values = pose();
    values[23][15] = f32::from_bits(0x7fc0_0123);
    let mut record = SkeletonAnimationRecord::new();
    let mut source = input(1);
    let history = SkeletonPhysicsHistory::new([1.0; 24]);
    assert_eq!(
        record.try_update(
            &values,
            &history,
            &SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM),
            &mut source
        ),
        Err(SkeletonAnimationRecordError::InvalidTotalMass),
    );
    assert_eq!(record.poses()[23][15].to_bits(), 0x7fc0_0123);
    assert_eq!(source.reads, 0);
    assert_eq!(record.reset_scalar().to_bits(), 0);
    assert_eq!(record.centre_of_mass().map(f32::to_bits), [0; 4]);
}

#[test]
fn projection_retains_deck_xzw_and_current_mode_changes_full_com() {
    let mut values = pose();
    values[0][12..].copy_from_slice(&[2.0, 4.0, 6.0, 8.0]);
    values[1][12..].copy_from_slice(&[10.0, 12.0, 14.0, 16.0]);
    let history = one_part_mass();
    let initial_masses = history.fractional_part_masses().map(f32::to_bits);
    let matrix = SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM);
    let mut record = SkeletonAnimationRecord::new();
    let mut source = input(0);
    record
        .try_update(&values, &history, &matrix, &mut source)
        .unwrap();
    assert_eq!(
        record.centre_of_mass().map(f32::to_bits),
        [2.0_f32, 12.0, 6.0, 8.0].map(f32::to_bits)
    );
    assert_eq!(
        record.centre_of_mass_delta().map(f32::to_bits),
        [2.0_f32, 12.0, 6.0, 8.0].map(f32::to_bits)
    );
    assert_eq!(
        words(record.com_to_deck()),
        ([0, 8.0_f32.to_bits(), 0], Some(0))
    );
    assert_eq!(words(record.com_to_deck_delta()), ([0; 3], Some(0)));
    assert_eq!(record.reset_scalar().to_bits(), 0x3f80_0000);
    source.value = Ok(255);
    record
        .try_update(&values, &history, &matrix, &mut source)
        .unwrap();
    assert_eq!(
        record.centre_of_mass().map(f32::to_bits),
        [10.0_f32, 12.0, 14.0, 16.0].map(f32::to_bits)
    );
    assert_eq!(
        record.centre_of_mass_delta().map(f32::to_bits),
        [8.0_f32, 0.0, 8.0, 8.0].map(f32::to_bits)
    );
    assert_eq!(
        words(record.com_to_deck_delta()),
        ([8.0_f32.to_bits(), 0, 8.0_f32.to_bits()], Some(0))
    );
    assert_eq!(source.reads, 2);
    assert_eq!(
        history.fractional_part_masses().map(f32::to_bits),
        initial_masses
    );
}

#[test]
fn weighted_com_uses_ascending_fma_including_w() {
    // 4097*4097 = 16785409 exactly. Separate f32 multiplication rounds down
    // before subtracting 16785408; the source fused accumulation retains 1.
    let mut mass_values = [0.0; 24];
    mass_values[..3].copy_from_slice(&[1.0, 4097.0, -4097.0]);
    let history = masses(mass_values);
    let mut values = pose();
    values[0][12..].fill(-16_785_408.0);
    values[1][12..].fill(4097.0);
    let mut record = SkeletonAnimationRecord::new();
    record
        .try_update(
            &values,
            &history,
            &SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM),
            &mut input(1),
        )
        .unwrap();
    assert_eq!(record.centre_of_mass().map(f32::to_bits), [0x3f80_0000; 4]);
    assert_eq!((4097.0_f32 * 4097.0 - 16_785_408.0).to_bits(), 0);
}

#[test]
fn transformed_points_keep_separate_translation_rounding_and_old_delta() {
    let history = one_part_mass();
    let mut values = pose();
    values[1][12] = 1.0;
    let mut translated = FRESH_VOLUME_TRANSFORM;
    translated[12] = 33_554_432.0;
    let mut record = SkeletonAnimationRecord::new();
    record
        .try_update(
            &values,
            &history,
            &SkeletonRootMatrix::known(translated),
            &mut input(1),
        )
        .unwrap();
    // Both transformed X coordinates round to 2^25. Transforming COM-deck as
    // a direction instead would produce 1 and change retained history.
    assert_eq!(record.com_to_deck().xyz[0].to_bits(), 0);
    record
        .try_update(
            &values,
            &history,
            &SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM),
            &mut input(1),
        )
        .unwrap();
    assert_eq!(record.com_to_deck().xyz[0].to_bits(), 0x3f80_0000);
    assert_eq!(record.com_to_deck_delta().xyz[0].to_bits(), 0x3f80_0000);
    assert_eq!(record.centre_of_mass_delta().map(f32::to_bits), [0; 4]);
}

#[test]
fn cvar_failure_keeps_new_pose_and_all_previous_com_history() {
    let history = one_part_mass();
    let matrix = SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM);
    let mut values = pose();
    values[1][13] = 2.0;
    let mut record = SkeletonAnimationRecord::new();
    record
        .try_update(&values, &history, &matrix, &mut input(1))
        .unwrap();
    let old_com = record.centre_of_mass().map(f32::to_bits);
    let old_delta = record.centre_of_mass_delta().map(f32::to_bits);
    let old_deck = words(record.com_to_deck());
    let old_deck_delta = words(record.com_to_deck_delta());
    values[1][13] = 7.0;
    values[23][0] = 23.0;
    let mut source = Input {
        value: Err("unavailable"),
        reads: 0,
    };
    assert_eq!(
        record.try_update(&values, &history, &matrix, &mut source),
        Err(SkeletonAnimationRecordError::Cvar("unavailable"))
    );
    assert_eq!(source.reads, 1);
    assert_eq!(record.poses()[23][0].to_bits(), 23.0_f32.to_bits());
    assert_eq!(record.centre_of_mass().map(f32::to_bits), old_com);
    assert_eq!(record.centre_of_mass_delta().map(f32::to_bits), old_delta);
    assert_eq!(words(record.com_to_deck()), old_deck);
    assert_eq!(words(record.com_to_deck_delta()), old_deck_delta);
    assert_eq!(record.reset_scalar().to_bits(), 0x3f80_0000);
}

#[test]
fn physics_reset_preserves_pose_and_scalar_and_next_delta_is_unsuppressed() {
    let history = one_part_mass();
    let matrix = SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM);
    let mut values = pose();
    values[1][13] = 2.0;
    let mut record = SkeletonAnimationRecord::new();
    record
        .try_update(&values, &history, &matrix, &mut input(1))
        .unwrap();
    let mut proxy_target = Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
        [9.0; 4],
    ));
    record.reset_after_physics_reset(&mut proxy_target);
    assert_eq!(
        proxy_target
            .unwrap()
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0; 4]
    );
    assert_eq!(record.poses()[1][13].to_bits(), 2.0_f32.to_bits());
    assert_eq!(record.reset_scalar().to_bits(), 0x3f80_0000);
    assert_eq!(record.centre_of_mass().map(f32::to_bits), [0; 4]);
    assert_eq!(record.centre_of_mass_delta().map(f32::to_bits), [0; 4]);
    assert_eq!(words(record.com_to_deck()), ([0; 3], Some(0)));
    assert_eq!(words(record.com_to_deck_delta()), ([0; 3], Some(0)));
    record
        .try_update(&values, &history, &matrix, &mut input(1))
        .unwrap();
    assert_eq!(
        record.com_to_deck_delta().xyz[1].to_bits(),
        2.0_f32.to_bits()
    );
}

#[test]
fn full_outer_w_is_computed_and_unknown_w_cannot_cancel() {
    let mut columns = *SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM).columns();
    for (column, w) in columns.iter_mut().zip([2.0, 3.0, 5.0, 7.0]) {
        column.w = Some(w);
    }
    let result = transform_point(&columns, [2.0, 3.0, 4.0, f32::NAN]);
    assert_eq!(
        words(result),
        (
            [2.0_f32, 3.0, 4.0].map(f32::to_bits),
            Some(40.0_f32.to_bits())
        )
    );
    for unknown in 0..4 {
        let mut uncertain = columns;
        uncertain[unknown].w = None;
        let result = transform_point(&uncertain, [0.0; 4]);
        assert!(result.w.is_none());
        assert!(subtract(result, result).w.is_none());
    }
}

#[test]
fn first_reset_scalar_zero_does_not_sanitize_nan() {
    let mut values = pose();
    values[1][12] = f32::NAN;
    let mut record = SkeletonAnimationRecord::new();
    record
        .try_update(
            &values,
            &one_part_mass(),
            &SkeletonRootMatrix::known(FRESH_VOLUME_TRANSFORM),
            &mut input(1),
        )
        .unwrap();
    assert!(record.centre_of_mass()[0].is_nan());
    assert!(record.com_to_deck_delta().xyz[0].is_nan());
    assert_eq!(record.reset_scalar().to_bits(), 0x3f80_0000);
}
