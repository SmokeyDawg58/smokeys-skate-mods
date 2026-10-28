//! Original map/Vault + controlled raw events through the actual same pair.
//! This proves selected ownership and producers, not Actor gesture generation.
use super::PhysicalPlayerAssemblyConstruction;
use crate::{
    input_map::SelectedInputMap,
    input_pad::InputPad,
    simulation_assembly_backend::constraint_session::physical_jump_events::{
        PhysicalPlayerJumpEventsError, PhysicalPlayerJumpInputBinding, PhysicalPlayerJumpInputError,
    },
    skeleton_processed_contact::jump_events::{
        SkeletonJumpAttributes, SkeletonJumpError, SkeletonJumpGlobal,
    },
};
use skate2_formats::andale::PackedFastString;
use std::sync::Arc;
pub(super) fn record(name: &str, value: f32) -> [u8; 0x40] {
    let mut raw = [0; 0x40];
    raw[..4].copy_from_slice(&value.to_be_bytes());
    for (index, word) in PackedFastString::<5>::encode(name)
        .unwrap()
        .words
        .into_iter()
        .enumerate()
    {
        raw[0x20 + index * 4..0x24 + index * 4].copy_from_slice(&word.to_be_bytes());
    }
    raw[0x34] = 6; // controlled Mg-style status; type0 dispatch does not gate on it
    raw
}
#[expect(
    clippy::too_many_lines,
    reason = "one actual pair and retained source fields across controlled event passes"
)]
pub(crate) fn check(factory: &mut PhysicalPlayerAssemblyConstruction<'_>) {
    let mounts = crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_corpus();
    let bytes = mounts.read_path("data\\config\\input.cfg").unwrap();
    let map = SelectedInputMap::parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
    let owner = Arc::clone(factory.owner());
    let processed = Arc::clone(owner.reckoning().lock().unwrap().processed());
    let board = factory.board().unwrap().clone();
    let skeleton = factory.skeleton().unwrap().clone();
    // Actual retained providers, frozen payloads below are corpus assertions only.
    {
        let mut attrs = owner.jump_attributes().lock().unwrap();
        let jumping = attrs.capture(SkeletonJumpGlobal::Jumping).unwrap();
        let enable = attrs.lookup(&jumping, 0xb52e_6ee4_f322_a9f4).unwrap();
        assert_eq!(attrs.byte(&enable).unwrap(), 1);
        for (key, expected) in [
            (0x982f_499e_c977_35a9, 0x3efd_70a4),
            (0xa1f6_5bb8_c4d1_ba8b, 0x3f66_6666),
        ] {
            let pointer = attrs.lookup(&jumping, key).unwrap();
            assert_eq!(attrs.float(&pointer).unwrap().to_bits(), expected);
        }
        let adjustment = attrs.capture(SkeletonJumpGlobal::Adjustment).unwrap();
        let retain = attrs.lookup(&adjustment, 0x34bb_37c3_15bc_14b0).unwrap();
        assert_eq!(attrs.byte(&retain).unwrap(), 1);
    }
    let world = factory.completed_world_mut().unwrap();
    // Explicit controlled completed pad values. The original map/converter corpus
    // separately supplies raw hardware-equivalent integers and retained pad history.
    let mut pad = InputPad::new();
    let mut values = [0.0; 24];
    values[16] = 0.75;
    values[17] = 0.125;
    values[18] = 0.25;
    values[19] = 0.875;
    values[22] = 0.125; // different RStickY discriminates PrepareJump's ID75
    pad.try_update(&values, 24).unwrap();
    let expected_x = map.try_value(74, &mut pad).unwrap();
    let expected_z = map.try_value(75, &mut pad).unwrap();
    processed.lock().unwrap().reset(); // explicit earlier source Reset, no hidden bridge reset
    world
        .try_process_pair_selected_jump_contact_events(
            &board,
            &skeleton,
            &[
                record("PrepareJump", 0.0),
                record("MinJump", 0.25),
                record("TrickHeight", 0.625),
                record("jump", 0.0),
            ],
            &mut PhysicalPlayerJumpInputBinding::Human {
                map: &map,
                pad: &mut pad,
            },
        )
        .unwrap();
    let actual = world.skeleton(&skeleton).unwrap();
    assert_eq!(
        actual.jump_adjustment_x().unwrap().to_bits(),
        expected_x.to_bits()
    );
    assert_eq!(
        actual.jump_adjustment_z().unwrap().to_bits(),
        expected_z.to_bits()
    );
    {
        let state = processed.lock().unwrap();
        assert!(state.jump_events().complete());
        assert_eq!(
            state.jump_events().strength().unwrap().to_bits(),
            0.625_f32.to_bits()
        );
        let (x, z) = state.jump_events().adjustments();
        assert_eq!(x.unwrap().to_bits(), expected_x.to_bits());
        assert_eq!(z.unwrap().to_bits(), expected_z.to_bits());
    }
    // Missing human input refuses after the Prepare flag store, before adjustment writes.
    processed.lock().unwrap().reset();
    assert_eq!(
        processed.lock().unwrap().jump_events().known_flags().0 & 0x0020_0000,
        0
    );
    let mut missing = PhysicalPlayerJumpInputBinding::<InputPad>::MissingHuman;
    assert!(matches!(
        world.try_process_pair_selected_jump_contact_events(
            &board,
            &skeleton,
            &[record("PrepareJump", 0.0)],
            &mut missing
        ),
        Err(PhysicalPlayerJumpEventsError::Event(
            SkeletonJumpError::Input(PhysicalPlayerJumpInputError::MissingHuman)
        ))
    ));
    assert!(!processed.lock().unwrap().jump_events().complete());
    assert_eq!(
        processed.lock().unwrap().jump_events().known_flags().0 & 0x0020_0000,
        0x0020_0000
    );
    assert_eq!(
        world
            .skeleton(&skeleton)
            .unwrap()
            .jump_adjustment_x()
            .unwrap()
            .to_bits(),
        expected_x.to_bits()
    );
    assert_eq!(
        world
            .skeleton(&skeleton)
            .unwrap()
            .jump_adjustment_z()
            .unwrap()
            .to_bits(),
        expected_z.to_bits()
    );
    // Actual retained hippy bit overrides the endpoint result. AI-null is a
    // deliberately selected diagnostic GetInput branch, not an inferred Actor.
    processed.lock().unwrap().reset();
    let mut ai = PhysicalPlayerJumpInputBinding::<InputPad>::AiNull;
    world
        .try_process_pair_selected_jump_contact_events(
            &board,
            &skeleton,
            &[
                record("PrepareJump", 0.0),
                record("MinJump", 0.25),
                record("TrickHeight", 0.125),
                record("jump", 0.0),
                record("HippyJumping", 0.0),
            ],
            &mut ai,
        )
        .unwrap();
    assert_eq!(
        processed
            .lock()
            .unwrap()
            .jump_events()
            .strength()
            .unwrap()
            .to_bits(),
        0.125_f32.to_bits()
    );
    assert_eq!(
        world
            .skeleton(&skeleton)
            .unwrap()
            .jump_adjustment_z()
            .unwrap()
            .to_bits(),
        expected_z.to_bits()
    );
    println!(
        "original Xbox map and retained anim_motion.jumping/physics_jump.default -> SAME paired Skeleton Prepare adjustments and processed jump strength; missing-human refusal and actual HippyJumping bit override; controlled raw records, no Actor gesture claim"
    );
}
