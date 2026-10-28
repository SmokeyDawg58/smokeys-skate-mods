use super::*;
use crate::attribute_runtime::tests::{collection, graph, set_dependency_bytes};

fn vault() -> Arc<AttributeRuntimeVault> {
    Arc::new(graph(SPEED_WOBBLE_CLASS, vec![]))
}

#[test]
fn reset_preserves_same_instance_and_unknown_or_written_amplitude() {
    for amplitude in [None, Some(f32::from_bits(0xffc0_0123))] {
        let mut owner = SkateboardSpeedWobbleOwner::new();
        assert_eq!(owner.phase(), SkateboardSpeedWobblePhase::Unattempted);
        assert_eq!(
            owner.try_reset(),
            Err(SkateboardSpeedWobbleError::NotConstructed)
        );
        owner
            .try_construct(&vault(), SkateboardSpeedWobbleAllocation::Allocated)
            .unwrap();
        assert_eq!(owner.value().unwrap().state_bits(), (false, [0; 6], None));
        let attributes = Arc::clone(owner.value().unwrap().attributes());
        owner
            .value_mut_for_test()
            .unwrap()
            .seed_reset_fields_for_test(-7.0, amplitude);
        owner.try_reset().unwrap();
        assert_eq!(
            owner.value().unwrap().state_bits(),
            (false, [0; 6], amplitude.map(f32::to_bits))
        );
        assert!(Arc::ptr_eq(
            &attributes,
            owner.value().unwrap().attributes()
        ));
        assert_eq!(
            owner.try_construct(&vault(), SkateboardSpeedWobbleAllocation::Allocated),
            Err(SkateboardSpeedWobbleError::AlreadyAttempted)
        );
    }
}

#[test]
fn null_allocation_stays_null_and_never_becomes_successful_reset() {
    let mut owner = SkateboardSpeedWobbleOwner::new();
    owner
        .try_construct(&vault(), SkateboardSpeedWobbleAllocation::Null)
        .unwrap();
    assert_eq!(owner.phase(), SkateboardSpeedWobblePhase::AllocationNull);
    assert!(owner.value().is_none());
    assert_eq!(
        owner.try_reset(),
        Err(SkateboardSpeedWobbleError::NotConstructed)
    );
    assert_eq!(
        owner.try_construct(&vault(), SkateboardSpeedWobbleAllocation::Allocated),
        Err(SkateboardSpeedWobbleError::AlreadyAttempted)
    );
}

#[test]
fn null_cached_layout_uses_256_bytes_even_with_authored_parent() {
    for found in [false, true] {
        let mut data = graph(SPEED_WOBBLE_CLASS, vec![]);
        if found {
            collection(&mut data, SPEED_WOBBLE_CLASS, 7, 0, Some(64), vec![]);
            collection(
                &mut data,
                SPEED_WOBBLE_CLASS,
                DEFAULT_COLLECTION,
                7,
                None,
                vec![],
            );
            set_dependency_bytes(&mut data, 1, vec![0x7f; 64 + LAYOUT_SIZE]);
        }
        let wobble = SkateboardSpeedWobble::new(&Arc::new(data));
        let layout = wobble.attributes().try_capture_layout().unwrap();
        assert!(layout.uses_default());
        assert_eq!(layout.bytes(), &[0; LAYOUT_SIZE]);
    }
}

#[test]
fn constructor_and_reset_do_not_read_bad_layout_and_authored_bytes_stay_retained() {
    let mut bad = graph(SPEED_WOBBLE_CLASS, vec![]);
    collection(
        &mut bad,
        SPEED_WOBBLE_CLASS,
        DEFAULT_COLLECTION,
        0,
        Some(64),
        vec![],
    );
    set_dependency_bytes(&mut bad, 1, vec![0; 68]);
    let mut wobble = SkateboardSpeedWobble::new(&Arc::new(bad));
    wobble.reset();
    assert!(wobble.attributes().try_capture_layout().is_err());
    let mut data = graph(SPEED_WOBBLE_CLASS, vec![]);
    collection(
        &mut data,
        SPEED_WOBBLE_CLASS,
        DEFAULT_COLLECTION,
        0,
        Some(64),
        vec![],
    );
    let mut bytes = vec![0; 64 + LAYOUT_SIZE];
    bytes[64] = 0x80;
    bytes[64 + LAYOUT_SIZE - 1] = 0x7f;
    set_dependency_bytes(&mut data, 1, bytes);
    let wobble = SkateboardSpeedWobble::new(&Arc::new(data));
    let attributes = Arc::clone(wobble.attributes());
    drop(wobble);
    let layout = attributes.try_capture_layout().unwrap();
    assert!(!layout.uses_default());
    assert_eq!(layout.bytes().len(), LAYOUT_SIZE);
    assert_eq!(
        (layout.bytes()[0], layout.bytes()[LAYOUT_SIZE - 1]),
        (0x80, 0x7f)
    );
}
