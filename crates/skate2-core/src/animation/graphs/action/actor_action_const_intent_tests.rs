use super::*;
#[test]
fn null_begin_preserves_insert_before_byte_store_failure() {
    let mut inserted = false;
    let result = begin(None, || {
        inserted = true;
        Ok::<_, ()>(())
    });
    assert!(inserted);
    assert_eq!(result, Err(ConstIntentError::NullInstance));
}
#[test]
fn failed_begin_does_not_publish_just_begun() {
    let mut instance = ConstIntentInstance::new();
    assert_eq!(
        begin(Some(&mut instance), || Err("insert")),
        Err(ConstIntentError::Source("insert"))
    );
    assert!(!instance.just_begun);
}
#[test]
fn pulse_survives_first_update_then_removes_without_reinsertion() {
    let mut instance = ConstIntentInstance::new();
    let mut erased = 0;
    begin(Some(&mut instance), || Ok::<_, ()>(())).unwrap();
    update(Some(&mut instance), false, || {
        erased += 1;
        Ok::<_, ()>(())
    })
    .unwrap();
    assert_eq!(erased, 0);
    update(Some(&mut instance), false, || {
        erased += 1;
        Ok::<_, ()>(())
    })
    .unwrap();
    assert_eq!(erased, 1);
}
#[test]
fn persistent_update_never_calls_map_and_null_fails_at_final_store() {
    let mut instance = ConstIntentInstance::new();
    update(Some(&mut instance), true, || panic!("no map access"))
        .unwrap_or_else(|_: ConstIntentError<()>| panic!());
    assert_eq!(
        update(None, true, || Err("must not run")),
        Err(ConstIntentError::NullInstance)
    );
    assert_eq!(
        update(None, false, || Err("must not run")),
        Err(ConstIntentError::NullInstance)
    );
}
