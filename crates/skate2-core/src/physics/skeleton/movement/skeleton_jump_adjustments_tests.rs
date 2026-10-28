use super::*;
use crate::{
    attribute_runtime::tests::{collection, definition, graph, node},
    skeleton_processed_contact::jump_events::{SkeletonJumpAttributes, SkeletonJumpGlobal},
};
#[test]
fn allocation_is_unknown_and_constructor_is_one_attempt() {
    let mut state = SkeletonJumpAdjustments::uninitialized();
    assert_eq!(
        state.x(),
        Err(SkeletonJumpAdjustmentError::ConstructorRequired)
    );
    assert!(state.store_x(2.0).is_err());
    state.initialize_constructor().unwrap();
    assert_eq!(state.x().unwrap().to_bits(), 0);
    assert_eq!(state.z().unwrap().to_bits(), 0);
    state.store_x(-0.0).unwrap();
    assert_eq!(
        state.initialize_constructor(),
        Err(SkeletonJumpAdjustmentError::AlreadyConstructed)
    );
    assert_eq!(state.x().unwrap().to_bits(), 0x8000_0000);
}
#[test]
fn independent_stores_preserve_other_lane_and_exact_bits() {
    let mut state = SkeletonJumpAdjustments::uninitialized();
    state.initialize_constructor().unwrap();
    state.store_x(f32::from_bits(0x7fc1_2345)).unwrap();
    assert_eq!(state.z().unwrap().to_bits(), 0);
    state.store_z(-0.0).unwrap();
    assert_eq!(state.x().unwrap().to_bits(), 0x7fc1_2345);
    assert_eq!(state.z().unwrap().to_bits(), 0x8000_0000);
}
#[test]
fn generated_binding_selects_jumping_not_default() {
    let class = 0x90c3_decf_0105_631e;
    let key = 0xb52e_6ee4_f322_a9f4;
    assert_eq!(skate2_formats::attrib::string_hash64("anim_motion"), class);
    assert_eq!(
        skate2_formats::attrib::string_hash64("jumping"),
        0xac26_0b44_fa3c_a0a4
    );
    assert_eq!(
        skate2_formats::attrib::string_hash64("physics_jump"),
        0xc2e0_a8b6_c139_90fd
    );
    let mut vault = graph(class, vec![definition(key, 4, 1)]);
    collection(
        &mut vault,
        class,
        0xac26_0b44_fa3c_a0a4,
        0,
        None,
        vec![node(key, 0x40, 0x0100_0000)],
    );
    collection(
        &mut vault,
        class,
        0xd7ed_bd36_2d7d_2152,
        0,
        None,
        vec![node(key, 0x40, 0)],
    );
    let vault = Arc::new(vault);
    let mut provider = SkeletonJumpVaultAttributes::bind_global(&vault);
    let token = provider.capture(SkeletonJumpGlobal::Jumping).unwrap();
    let pointer = provider.lookup(&token, key).unwrap();
    assert_eq!(provider.byte(&pointer).unwrap(), 1);
    let mut foreign = SkeletonJumpVaultAttributes::bind_global(&vault);
    assert!(foreign.lookup(&token, key).is_err());
    assert!(foreign.byte(&pointer).is_err());
}
