use super::*;
fn initialized() -> InputPadCache {
    let mut cache = InputPadCache::new();
    cache.try_initialize().unwrap();
    cache
}
fn samples(value: f32) -> [PublishedInputPadSample; 4] {
    [PublishedInputPadSample {
        count: 24,
        values: [value; 24],
    }; 4]
}
#[test]
fn constructor_is_not_initialized_empty_ring() {
    let mut cache = InputPadCache::new();
    assert_eq!((cache.read, cache.write), (0, 0));
    assert_eq!(
        cache.try_last_input(),
        Err(InputPadCacheError::InitializationRequired)
    );
    cache.try_initialize().unwrap();
    assert_eq!((cache.read, cache.write), (0, 1));
    assert!(!cache.try_last_input().unwrap());
    assert_eq!(
        cache.try_initialize(),
        Err(InputPadCacheError::AlreadyInitialized)
    );
    assert_eq!(cache.pad(0).unwrap().num_buttons(), 0);
}
#[test]
fn collapse_updates_each_pad_only_once_and_empty_drain_preserves_history() {
    let mut cache = initialized();
    cache.try_cache_input(&samples(1.0)).unwrap();
    cache.try_cache_input(&samples(0.25)).unwrap();
    assert!(cache.try_last_input().unwrap());
    for device in 0..4 {
        assert_eq!(
            cache.pad(device).unwrap().value(0).unwrap().to_bits(),
            0.25_f32.to_bits()
        );
    }
    for _ in 0..8 {
        assert!(!cache.try_last_input().unwrap());
    }
    // First update advanced debounce once. Two more still suppress the press;
    // the fourth consumed update finally commits it.
    for _ in 0..2 {
        cache.try_cache_input(&samples(1.0)).unwrap();
        cache.try_last_input().unwrap();
        assert!(!cache.pad(0).unwrap().held(0).unwrap());
    }
    cache.try_cache_input(&samples(1.0)).unwrap();
    cache.try_last_input().unwrap();
    assert!(cache.pad(0).unwrap().pressed(0).unwrap());
}
#[test]
fn thirty_unconsumed_publications_alias_empty_without_overflow_repair() {
    let mut cache = initialized();
    for _ in 0..30 {
        cache.try_cache_input(&samples(1.0)).unwrap();
    }
    assert_eq!((cache.read, cache.write), (0, 1));
    assert!(!cache.try_last_input().unwrap());
    assert_eq!(cache.pad(0).unwrap().num_buttons(), 0);
    cache.try_cache_input(&samples(0.5)).unwrap();
    assert!(cache.try_last_input().unwrap());
    assert_eq!(
        cache.pad(0).unwrap().value(0).unwrap().to_bits(),
        0.5_f32.to_bits()
    );
}
#[test]
fn groups_consume_independently_and_flush_keeps_same_pad() {
    let mut sim = initialized();
    let mut pres = initialized();
    sim.try_cache_input(&samples(0.25)).unwrap();
    pres.try_cache_input(&samples(0.25)).unwrap();
    sim.try_last_input().unwrap();
    assert_eq!(pres.pad(0).unwrap().num_buttons(), 0);
    let pointer = std::ptr::from_ref(sim.pad(0).unwrap());
    sim.try_cache_input(&samples(1.0)).unwrap();
    sim.try_flush().unwrap();
    assert!(!sim.try_last_input().unwrap());
    assert_eq!(
        pointer,
        std::ptr::from_mut(sim.pad_mut(0).unwrap()).cast_const()
    );
    assert_eq!(
        sim.pad(0).unwrap().value(0).unwrap().to_bits(),
        0.25_f32.to_bits()
    );
    assert!(pres.try_last_input().unwrap());
}
#[test]
fn invalid_late_count_keeps_drain_and_earlier_pad_writes() {
    let mut cache = initialized();
    let mut input = samples(0.75);
    input[2].count = 25;
    cache.try_cache_input(&input).unwrap();
    assert_eq!(
        cache.try_last_input(),
        Err(InputPadCacheError::Pad {
            device: 2,
            error: InputPadError::CountTooLarge(25)
        })
    );
    assert_eq!(
        cache.pad(1).unwrap().value(0).unwrap().to_bits(),
        0.75_f32.to_bits()
    );
    assert_eq!(cache.pad(2).unwrap().num_buttons(), 0);
    assert_eq!(cache.pad(3).unwrap().num_buttons(), 0);
    assert!(!cache.try_last_input().unwrap());
}
#[test]
fn explicit_disconnect_and_reconnection_reuse_existing_growth_semantics() {
    let mut cache = initialized();
    for _ in 0..4 {
        cache.try_cache_input(&samples(1.0)).unwrap();
        cache.try_last_input().unwrap();
    }
    assert!(cache.pad(0).unwrap().held(0).unwrap());
    let disconnected = [PublishedInputPadSample {
        count: 0,
        values: [9.0; 24],
    }; 4];
    cache.try_cache_input(&disconnected).unwrap();
    cache.try_last_input().unwrap();
    assert_eq!(cache.pad(0).unwrap().num_buttons(), 0);
    cache.try_cache_input(&samples(1.0)).unwrap();
    cache.try_last_input().unwrap();
    assert!(!cache.pad(0).unwrap().held(0).unwrap());
}
