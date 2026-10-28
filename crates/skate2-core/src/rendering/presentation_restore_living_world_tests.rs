use super::*;

fn yaw(angle: f32, translation: [f32; 3]) -> [f32; 16] {
    let (sine, cosine) = angle.sin_cos();
    let rows = [
        [cosine, 0.0, -sine, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [sine, 0.0, cosine, 0.0],
        [translation[0], translation[1], translation[2], 1.0],
    ];
    std::array::from_fn(|index| rows[index / 4][index % 4])
}

fn heading(transform: &AnimationMatrix) -> f32 {
    (-transform[2]).atan2(transform[0])
}

fn frame(id: u32, transform: [f32; 16]) -> LivingWorldSubmission {
    LivingWorldSubmission {
        pedestrians: vec![EntityReplay::new(id, transform)],
        ..LivingWorldSubmission::default()
    }
}

#[test]
fn a_new_entity_starts_at_its_next_record_and_then_blends() {
    let previous = frame(7, yaw(0.0, [100.0, 0.0, 200.0]));
    let next = frame(7, yaw(0.1, [101.0, 0.0, 200.0]));
    let mut restore = LivingWorldRestore::default();
    restore.restore(Some(&previous), &next, 0.5).unwrap();
    let created = restore.pedestrian(7).unwrap();
    assert!((created[12] - 101.0).abs() < 1e-4);
    restore.restore(Some(&previous), &next, 0.5).unwrap();
    let blended = restore.pedestrian(7).unwrap();
    assert!((blended[12] - 100.5).abs() < 1e-4, "{}", blended[12]);
    assert!(
        (heading(&blended) - 0.05).abs() < 1e-3,
        "{}",
        heading(&blended)
    );
}

#[test]
fn an_entity_the_previous_frame_lacks_stays_at_its_next_record() {
    let previous = frame(3, yaw(0.0, [0.0; 3]));
    let next = frame(7, yaw(0.4, [5.0, 0.0, 5.0]));
    let mut restore = LivingWorldRestore::default();
    restore.restore(Some(&previous), &next, 0.25).unwrap();
    restore.restore(Some(&previous), &next, 0.25).unwrap();
    let transform = restore.pedestrian(7).unwrap();
    assert!((transform[12] - 5.0).abs() < 1e-5);
    assert!((heading(&transform) - 0.4).abs() < 1e-4);
    assert!(restore.pedestrian(3).is_none());
}

#[test]
fn stored_wheel_angles_keep_their_steps() {
    let vehicle = VehicleReplay::new(1, yaw(0.0, [0.0; 3]), 1.0, -0.3, 0x80 | 8);
    assert!((vehicle.spin() - 1.0).abs() < 2e-4, "{}", vehicle.spin());
    assert!((vehicle.turn() + 0.3).abs() < 7e-3, "{}", vehicle.turn());
    assert_eq!(vehicle.wheels().lights, [0.0, 1.0, 0.0, 1.0]);
}

#[test]
fn wheels_slerp_their_spin_and_turn_between_frames() {
    let previous = VehicleReplay::new(1, yaw(0.0, [0.0; 3]), 0.2, 0.1, 2);
    let next = VehicleReplay::new(1, yaw(0.0, [0.0; 3]), 0.6, 0.3, 0);
    let wheels = VehicleWheels::restore(&previous, &next, 0.5);
    let spin = 2.0 * wheels.spin[0].atan2(wheels.spin[3]);
    assert!((spin - 0.4).abs() < 1e-3, "{spin}");
    assert_eq!(wheels.lights, [0.0; 4]);
    let turn = [0.0, (0.1_f32).sin(), 0.0, (0.1_f32).cos()];
    let expected = compose(turn, wheels.spin);
    for lane in 0..4 {
        assert!((wheels.front[lane] - expected[lane]).abs() < 2e-3, "{lane}");
    }
}

#[test]
fn a_dynamic_object_blends_its_part() {
    let object = |x: f32| LivingWorldSubmission {
        objects: vec![ObjectReplay::new(
            9,
            yaw(0.0, [x, 0.0, 0.0]),
            yaw(0.0, [x, 1.0, 0.0]),
        )],
        ..LivingWorldSubmission::default()
    };
    let (previous, next) = (object(10.0), object(12.0));
    let mut restore = LivingWorldRestore::default();
    restore.restore(Some(&previous), &next, 0.25).unwrap();
    restore.restore(Some(&previous), &next, 0.25).unwrap();
    let part = restore.dynamic_object(9).unwrap();
    assert!((part[12] - 10.5).abs() < 1e-5);
    assert!((part[13] - 1.0).abs() < 1e-6);
}
