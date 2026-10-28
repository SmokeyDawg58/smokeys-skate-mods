use super::*;
#[test]
fn pointer_projection_rejects_overlapping_other_relocation() {
    let mut vault = graph(CLASS, vec![definition(KEY, 4, 4)]);
    // Flags4 does not create a class layout node; supply the actual searched node.
    collection(
        &mut vault,
        CLASS,
        CHILD,
        0,
        Some(0),
        vec![node(KEY, 0x10, 0)],
    );
    set_dependency_bytes(&mut vault, 1, vec![0; 256]);
    fixture_text_pointer(&mut vault, 1, 1, 128);
    let instance = Arc::new(vault).instance(CLASS, CHILD);
    assert!(
        matches!(instance.try_get_text(KEY,0),Err(AttributeRuntimeError::RelocatedPayload{slot}) if slot==location(1,1))
    );
    assert!(
        matches!(instance.try_cached_layout_text(0),Err(AttributeRuntimeError::RelocatedPayload{slot}) if slot==location(1,1))
    );
}

#[test]
fn generic_text_uses_parent_node_before_child_layout_and_plain_read_still_refuses_fixup() {
    let mut vault = graph(CLASS, vec![definition(KEY, 4, 4)]);
    collection(
        &mut vault,
        CLASS,
        PARENT,
        0,
        Some(0),
        vec![node(KEY, 0x10, 4)],
    );
    collection(&mut vault, CLASS, CHILD, PARENT, Some(64), vec![]);
    let mut bytes = vec![0; 256];
    bytes[128..135].copy_from_slice(b"parent\0");
    bytes[144..151].copy_from_slice(b"layout\0");
    set_dependency_bytes(&mut vault, 1, bytes);
    fixture_text_pointer(&mut vault, 1, 4, 128);
    fixture_text_pointer(&mut vault, 1, 64, 144);
    let instance = Arc::new(vault).instance(CLASS, CHILD);
    assert_eq!(
        instance.try_get_text(KEY, 0).unwrap().unwrap().bytes(),
        b"parent"
    );
    assert!(matches!(
        instance.try_get_data(KEY, 0),
        Err(AttributeRuntimeError::RelocatedPayload { .. })
    ));
    assert_eq!(
        instance.try_cached_layout_text(0).unwrap().unwrap().bytes(),
        b"layout"
    );
}
#[test]
fn null_parent_text_ends_search_without_child_layout_fallback() {
    let mut vault = graph(CLASS, vec![definition(KEY, 4, 4)]);
    collection(
        &mut vault,
        CLASS,
        PARENT,
        0,
        Some(0),
        vec![node(KEY, 0x10, 4)],
    );
    collection(&mut vault, CLASS, CHILD, PARENT, Some(64), vec![]);
    let mut bytes = vec![0; 256];
    bytes[144..151].copy_from_slice(b"layout\0");
    set_dependency_bytes(&mut vault, 1, bytes);
    fixture_text_pointer(&mut vault, 1, 64, 144);
    let instance = Arc::new(vault).instance(CLASS, CHILD);
    assert!(instance.try_get_text(KEY, 0).unwrap().is_none());
    assert!(instance.try_cached_layout_text(0).unwrap().is_some());
}
