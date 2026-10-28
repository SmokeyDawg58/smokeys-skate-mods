use super::*;

#[test]
fn null_second_endpoint_preserves_first_update() {
    let first = Arc::new(Mutex::new(CameraShot::constructed()));
    let selection = blend_selection::BlendSelection {
        first: Some(Arc::clone(&first)),
        second: None,
        fraction: 0.5,
    };
    let result = update_endpoints(selection, |child| {
        child.lock().unwrap().blend_value = 7.0;
        Ok(())
    })
    .unwrap();
    assert!(result.is_none());
    assert_eq!(
        first.lock().unwrap().blend_value.to_bits(),
        7.0_f32.to_bits()
    );
}

#[test]
fn aliased_endpoints_update_twice_before_both_payload_reads() {
    let child = Arc::new(Mutex::new(CameraShot::constructed()));
    child.lock().unwrap().blend_value = 0.0;
    let selection = blend_selection::BlendSelection {
        first: Some(Arc::clone(&child)),
        second: Some(Arc::clone(&child)),
        fraction: 1.0,
    };
    let result = update_endpoints(selection, |child| {
        child.lock().unwrap().blend_value += 1.0;
        Ok(())
    })
    .unwrap()
    .unwrap();
    assert_eq!(result.0.blend_value.to_bits(), 2.0_f32.to_bits());
    assert_eq!(result.1.blend_value.to_bits(), 2.0_f32.to_bits());
}
