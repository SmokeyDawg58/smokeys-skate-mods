//! Pure record-kernel fixtures, not fabricated assembly/publication authority.
//! Actual attached-owner and original-mass tests live in the integration layer.
use super::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Acquire,
    Get(usize),
    Cvar,
    Up,
    Dot,
}

struct Input {
    events: Rc<RefCell<Vec<Event>>>,
    xyz: Result<u8, &'static str>,
    up: Result<[f32; 4], &'static str>,
}
impl SkeletonHistoryInput for Input {
    type Error = &'static str;
    type Up = ();
    fn acquire_up_vector(&mut self) -> Result<(), Self::Error> {
        self.events.borrow_mut().push(Event::Acquire);
        Ok(())
    }
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error> {
        self.events.borrow_mut().push(Event::Cvar);
        self.xyz
    }
    fn read_up_vector(&mut self, (): &()) -> Result<[f32; 4], Self::Error> {
        self.events.borrow_mut().push(Event::Up);
        self.up
    }
}
fn input(xyz: u8, up: [f32; 4]) -> Input {
    Input {
        events: Rc::default(),
        xyz: Ok(xyz),
        up: Ok(up),
    }
}
fn matrix(p: [f32; 4]) -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, p[0], p[1], p[2], p[3],
    ]
}
fn initialized() -> SkeletonPhysicsHistory {
    let mut masses = [0.0; 24];
    masses[0] = 1.0;
    let mut history = SkeletonPhysicsHistory::new(masses);
    history.try_initialize_masses().unwrap();
    history
}
fn assert_words<const N: usize>(actual: [f32; N], expected: [f32; N]) {
    assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
}

#[test]
fn constructor_retains_raw_masses_but_initializes_the_actual_record() {
    let mut masses = [0.0; 24];
    masses[3] = f32::from_bits(0x8000_0000);
    masses[7] = f32::from_bits(0x7fc0_1234);
    let history = SkeletonPhysicsHistory::new(masses);
    assert_eq!(
        history.part_masses().map(f32::to_bits),
        masses.map(f32::to_bits)
    );
    assert_eq!(history.total_mass().to_bits(), 0);
    assert_eq!(history.time_delta().to_bits(), 0);
    assert_eq!(history.fractional_part_masses().map(f32::to_bits), [0; 24]);
    for transform in history.record().transforms() {
        assert_eq!(
            transform.map(f32::to_bits),
            [
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
                0
            ]
        );
    }
    for vectors in [
        history.record().positions(),
        history.record().velocities(),
        history.record().velocity_deltas(),
    ] {
        for vector in vectors {
            assert_eq!(vector.map(f32::to_bits), [0; 4]);
        }
    }
    for vector in [
        history.record().centre_of_mass(),
        history.record().centre_of_mass_velocity(),
        history.record().proxy_volume_position(),
    ] {
        assert_eq!(vector.map(f32::to_bits), [0; 4]);
    }
}

#[test]
fn mass_sum_is_ascending_f32_and_failure_retains_total_not_new_fractions() {
    let mut masses = [0.0; 24];
    masses[..4].copy_from_slice(&[16_777_216.0, 1.0, -16_777_216.0, 2.0]);
    let mut history = SkeletonPhysicsHistory::new(masses);
    history.try_initialize_masses().unwrap();
    assert_eq!(history.total_mass().to_bits(), 2.0_f32.to_bits());
    assert_words(
        std::array::from_fn(|i| history.fractional_part_masses()[i]),
        [8_388_608.0, 0.5, -8_388_608.0, 1.0],
    );
    let before = *history.fractional_part_masses();
    for bad in [0.0, -1.0, f32::NAN] {
        history.part_masses = [0.0; 24];
        history.part_masses[0] = bad;
        assert_eq!(
            history.try_initialize_masses(),
            Err(SkeletonPhysicsMassError)
        );
        assert_words(*history.fractional_part_masses(), before);
        if bad.is_nan() {
            assert!(history.total_mass().is_nan());
        } else {
            assert_eq!(history.total_mass().to_bits(), bad.to_bits());
        }
    }
    // Positive infinity is not rejected by the source strict-positive gate.
    history.part_masses[0] = f32::INFINITY;
    history.try_initialize_masses().unwrap();
    assert!(history.fractional_part_masses()[0].is_nan());
}

#[test]
fn two_unchanged_samples_clear_velocity_but_leave_negative_first_velocity_delta() {
    let mut history = initialized();
    let root = matrix([1.0, -2.0, 3.0, -4.0]);
    let mut service = input(0xff, [f32::NAN; 4]);
    let events = Rc::clone(&service.events);
    let mut get = |part| {
        events.borrow_mut().push(Event::Get(part));
        Ok::<_, &'static str>(if part == 0 {
            matrix([90.0, 80.0, 70.0, 60.0])
        } else {
            root
        })
    };
    let mut numeric = HostSkeletonPhysicsHistoryNumeric;
    for sample in 0..2 {
        service.acquire_up_vector().unwrap();
        history
            .try_update(root, &(), &mut get, &mut service, &mut numeric)
            .unwrap();
        let scale = f32::from_bits(0x426f_ffff);
        let first = [scale, -2.0 * scale, 3.0 * scale, -4.0 * scale];
        for part in 0..24 {
            assert_words(history.record().positions()[part], [1.0, -2.0, 3.0, -4.0]);
            assert_words(
                history.record().velocities()[part],
                if sample == 0 { first } else { [0.0; 4] },
            );
            assert_words(
                history.record().velocity_deltas()[part],
                if sample == 0 {
                    first
                } else {
                    first.map(|v| -v)
                },
            );
        }
        assert_words(
            history.record().proxy_volume_position(),
            [90.0, 80.0, 70.0, 60.0],
        );
    }
    assert_eq!(history.time_delta().to_bits(), 0x3c88_8889);
    assert_words(history.record().centre_of_mass(), [1.0, -2.0, 3.0, -4.0]);
    assert_words(history.record().centre_of_mass_velocity(), [0.0; 4]);
    let one = std::iter::once(Event::Acquire)
        .chain((0..24).map(Event::Get))
        .chain([Event::Cvar])
        .collect::<Vec<_>>();
    assert_eq!(*service.events.borrow(), [one.clone(), one].concat());
}

#[test]
fn fractions_use_one_rounded_reciprocal_not_individual_divisions() {
    let mut masses = [0.0; 24];
    masses[0] = 3.0;
    masses[1] = 4.0;
    let mut history = SkeletonPhysicsHistory::new(masses);
    history.try_initialize_masses().unwrap();
    assert_eq!(history.total_mass().to_bits(), 0x40e0_0000);
    // round32(3 * round32(1/7)) differs from round32(3/7) by one ULP.
    assert_eq!(history.fractional_part_masses()[0].to_bits(), 0x3edb_6db8);
    assert_eq!((3.0_f32 / 7.0).to_bits(), 0x3edb_6db7);
}

#[test]
fn centre_of_mass_retains_fused_cancellation_with_source_initialized_fractions() {
    let mut masses = [0.0; 24];
    masses[0] = 1.0;
    masses[1] = 8_388_607.0;
    let mut history = SkeletonPhysicsHistory::new(masses);
    history.try_initialize_masses().unwrap();
    let root = matrix([-8_388_608.0, 0.0, 0.0, 0.0]);
    let second = matrix([f32::from_bits(0x3f80_0001), 0.0, 0.0, 0.0]);
    let mut service = input(1, [f32::NAN; 4]);
    history
        .try_update(
            root,
            &(),
            &mut |part| Ok::<_, &'static str>(if part == 1 { second } else { root }),
            &mut service,
            &mut HostSkeletonPhysicsHistoryNumeric,
        )
        .unwrap();
    // Part0 contributes -1; Part1 is (1+2^-23)*(1-2^-23).
    // Fused accumulation preserves -2^-46; split multiply/add would yield zero.
    assert_eq!(history.record().centre_of_mass()[0].to_bits(), 0xa880_0000);
    let split = second[12] * history.fractional_part_masses()[1] - 1.0;
    assert_eq!(split.to_bits(), 0);
}

#[test]
fn projected_com_reads_up_after_cvar_uses_unnormalized_full_w_and_unprojected_velocity() {
    struct Numeric {
        events: Rc<RefCell<Vec<Event>>>,
    }
    impl SkeletonPhysicsHistoryNumeric for Numeric {
        fn dot3(&mut self, displacement: [f32; 4], up: [f32; 4]) -> f32 {
            self.events.borrow_mut().push(Event::Dot);
            assert_words(displacement, [1.0, 2.0, 3.0, 4.0]);
            assert_words(up, [0.0, 2.0, 0.0, 3.0]);
            4.0
        }
    }
    let mut masses = [0.0; 24];
    masses[1] = 1.0;
    let mut history = SkeletonPhysicsHistory::new(masses);
    history.try_initialize_masses().unwrap();
    let root = matrix([10.0, 20.0, 30.0, 40.0]);
    let other = matrix([11.0, 22.0, 33.0, 44.0]);
    let mut service = input(0, [0.0, 2.0, 0.0, 3.0]);
    let events = Rc::clone(&service.events);
    let mut numeric = Numeric {
        events: Rc::clone(&events),
    };
    let mut get = |part| {
        events.borrow_mut().push(Event::Get(part));
        Ok::<_, &'static str>(other)
    };
    history
        .try_update(root, &(), &mut get, &mut service, &mut numeric)
        .unwrap();
    assert_words(history.record().centre_of_mass(), [10.0, 28.0, 30.0, 52.0]);
    let scale = f32::from_bits(0x426f_ffff);
    assert_words(
        history.record().centre_of_mass_velocity(),
        [11.0 * scale, 22.0 * scale, 33.0 * scale, 44.0 * scale],
    );
    assert_eq!(
        *events.borrow(),
        (0..24)
            .map(Event::Get)
            .chain([Event::Cvar, Event::Up, Event::Dot])
            .collect::<Vec<_>>()
    );
}

#[test]
fn validity_ignores_w_accepts_infinity_and_matrix_validation_precedes_full_copy() {
    let mut history = initialized();
    let mut valid = matrix([f32::INFINITY, 2.0, 3.0, f32::NAN]);
    valid[3] = f32::from_bits(0x7fc0_1234);
    history.update_part::<(), ()>(0, valid).unwrap();
    assert_eq!(
        history.record.transforms[0].map(f32::to_bits),
        valid.map(f32::to_bits)
    );
    assert!(history.record.positions[0][3].is_nan());
    let before = history.record.transforms[1];
    let mut bad = matrix([5.0, 6.0, 7.0, 8.0]);
    bad[8] = f32::NAN;
    assert_eq!(
        history.update_part::<(), ()>(1, bad),
        Err(SkeletonPhysicsHistoryError::InvalidVector {
            part: Some(1),
            field: SkeletonPhysicsVectorField::TransformColumn(2)
        })
    );
    assert_words(history.record.transforms[1], before);
    // The second infinite position produces infinity-infinity in delta.
    valid[13] = 9.0;
    assert_eq!(
        history.update_part::<(), ()>(0, valid),
        Err(SkeletonPhysicsHistoryError::InvalidVector {
            part: Some(0),
            field: SkeletonPhysicsVectorField::VelocityDelta
        })
    );
    assert_eq!(
        history.record.transforms[0][13].to_bits(),
        9.0_f32.to_bits()
    ); // matrix already copied
    assert_eq!(history.record.positions[0][1].to_bits(), 2.0_f32.to_bits()); // history did not advance
}

#[test]
fn getter_failure_preserves_root_proxy_and_completed_rows_without_com_publication() {
    let mut history = initialized();
    let root = matrix([1.0; 4]);
    let mut service = input(1, [0.0; 4]);
    let mut get = |part| {
        if part == 2 {
            Err("part2")
        } else {
            Ok(matrix([if part == 0 { 10.0 } else { 11.0 }; 4]))
        }
    };
    assert_eq!(
        history.try_update(
            root,
            &(),
            &mut get,
            &mut service,
            &mut HostSkeletonPhysicsHistoryNumeric
        ),
        Err(SkeletonPhysicsHistoryError::LivePart {
            part: 2,
            error: "part2"
        })
    );
    assert_eq!(history.time_delta().to_bits(), 0x3c88_8889);
    assert_words(history.record.positions[0], [1.0; 4]);
    assert_words(history.record.positions[1], [11.0; 4]);
    assert_words(history.record.positions[2], [0.0; 4]);
    assert_words(history.record.proxy_volume_position, [10.0; 4]);
    assert_words(history.record.centre_of_mass, [0.0; 4]);
    assert!(service.events.borrow().is_empty());
}

#[test]
fn cvar_and_lazy_up_failures_keep_completed_record_but_not_com() {
    for fail_up in [false, true] {
        let mut history = initialized();
        let root = matrix([2.0; 4]);
        let mut service = input(0, [0.0; 4]);
        if fail_up {
            service.up = Err("up");
        } else {
            service.xyz = Err("cvar");
        }
        let result = history.try_update(
            root,
            &(),
            &mut |_| Ok::<_, &'static str>(root),
            &mut service,
            &mut HostSkeletonPhysicsHistoryNumeric,
        );
        assert_eq!(
            result,
            Err(if fail_up {
                SkeletonPhysicsHistoryError::UpVector("up")
            } else {
                SkeletonPhysicsHistoryError::Cvar("cvar")
            })
        );
        assert_words(history.record.positions[23], [2.0; 4]);
        assert_words(history.record.centre_of_mass, [0.0; 4]);
        assert_eq!(
            *service.events.borrow(),
            if fail_up {
                vec![Event::Cvar, Event::Up]
            } else {
                vec![Event::Cvar]
            }
        );
    }
}

#[test]
fn com_store_precedes_com_velocity_assertion_and_dot_host_ignores_w() {
    let mut history = initialized();
    history.record.transforms[0] = matrix([2.0, 3.0, 4.0, 5.0]);
    history.record.velocities[0][0] = f32::NAN;
    let mut service = input(1, [f32::NAN; 4]);
    assert_eq!(
        history.calculate_centre_of_mass::<(), _, _>(
            &(),
            &mut service,
            &mut HostSkeletonPhysicsHistoryNumeric
        ),
        Err(SkeletonPhysicsHistoryError::InvalidVector {
            part: None,
            field: SkeletonPhysicsVectorField::CentreOfMassVelocity
        })
    );
    assert_words(history.record.centre_of_mass, [2.0, 3.0, 4.0, 5.0]);
    assert_words(history.record.centre_of_mass_velocity, [0.0; 4]);
    assert_eq!(
        HostSkeletonPhysicsHistoryNumeric
            .dot3([2.0, 3.0, 4.0, f32::NAN], [5.0, 6.0, 7.0, f32::INFINITY])
            .to_bits(),
        56.0_f32.to_bits()
    );
}
