use super::*;

fn shot() -> CameraShot {
    let mut parent = CameraShot::constructed();
    parent.blend_points = [0.0, 1.0, 2.0];
    for name in &mut parent.blend_names {
        name[0] = Some(b'x');
    }
    parent.blend_shots = std::array::from_fn(|_| {
        let mut child = CameraShot::constructed();
        child.name[0] = Some(b'x');
        Some(Arc::new(Mutex::new(child)))
    });
    parent
}

#[test]
fn below_first_preserves_same_pointer_for_both_updates() {
    let mut parent = shot();
    parent.blend_value = -1.0;
    let chosen = select(&parent).unwrap();
    assert!(Arc::ptr_eq(
        chosen.first.as_ref().unwrap(),
        chosen.second.as_ref().unwrap()
    ));
    assert_eq!(chosen.fraction.to_bits(), 1.0_f32.to_bits());
}

#[test]
fn unordered_value_walks_to_last_pair() {
    let mut parent = shot();
    parent.blend_value = f32::NAN;
    let chosen = select(&parent).unwrap();
    assert!(Arc::ptr_eq(
        chosen.first.as_ref().unwrap(),
        parent.blend_shots[1].as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(
        chosen.second.as_ref().unwrap(),
        parent.blend_shots[2].as_ref().unwrap()
    ));
    assert_eq!(chosen.fraction.to_bits(), 1.0_f32.to_bits());
}

#[test]
fn empty_reference_name_stops_before_later_missing_pointer() {
    let mut parent = shot();
    parent.blend_names[1][0] = Some(0);
    parent.blend_shots[2] = None;
    parent.blend_value = 10.0;
    let chosen = select(&parent).unwrap();
    assert!(Arc::ptr_eq(
        chosen.first.as_ref().unwrap(),
        chosen.second.as_ref().unwrap()
    ));
}

#[test]
fn nonfinite_smoothing_uses_fsel_upper_endpoint() {
    assert_eq!(
        smooth_value(1.0, 0.0, f32::NAN, -2.0, 3.0).to_bits(),
        3.0_f32.to_bits()
    );
}
