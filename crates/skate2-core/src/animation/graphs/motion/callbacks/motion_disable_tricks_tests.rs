use super::*;
#[test]
fn begin_and_expiry_keep_native_failure_prefixes() {
    let mut state = Instance {
        timer: 8.0,
        completed: true,
    };
    assert!(state.begin(2.0, |_| Err(())).is_err());
    assert_eq!(state.timer.to_bits(), 8.0_f32.to_bits());
    assert!(state.completed);
    state
        .begin(2.0, |v| {
            assert!(!v);
            Ok::<_, ()>(())
        })
        .unwrap();
    assert!(!state.completed);
    state
        .update(2.0, |_| panic!("zero does not expire"))
        .unwrap_or_else(|e: ()| panic!("{e:?}"));
    assert!(
        state
            .update(0.25, |v| {
                assert!(v);
                Err(())
            })
            .is_err()
    );
    assert_eq!(state.timer.to_bits(), (-0.25_f32).to_bits());
    assert!(!state.completed);
    state
        .update(0.25, |v| {
            assert!(v);
            Ok::<_, ()>(())
        })
        .unwrap();
    assert!(state.completed);
    let before = state.timer.to_bits();
    state
        .update(f32::NAN, |_| panic!("completed"))
        .unwrap_or_else(|e: ()| panic!("{e:?}"));
    assert_eq!(state.timer.to_bits(), before);
}
#[test]
fn unordered_timer_does_not_query_motion() {
    let mut state = Instance::default();
    state.begin(f32::NAN, |_| Ok::<_, ()>(())).unwrap();
    state
        .update(0.1, |_| panic!("unordered skips"))
        .unwrap_or_else(|e: ()| panic!("{e:?}"));
    assert!(state.timer.is_nan());
    assert!(!state.completed);
}
