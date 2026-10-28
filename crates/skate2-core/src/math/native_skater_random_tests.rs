use super::*;
#[test]
fn reset_sequence_matches_source_integer_graph_and_seed_wraps_each_word() {
    let gate = NativeRandomGate::from_static_initializer();
    let mut random = NativeSkaterRandom::reset();
    let result = (0..4)
        .map(|_| random.try_next(&gate).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(result, [0x559a_51ed, 0x274e_a7f5, 0xe827_f7e0, 0x76b6_5632]);
    assert_eq!(
        random.values,
        [
            0x76b6_5632,
            0x8e8e_5e51,
            0xcdb5_0e66,
            0xde90_1484,
            0x5db2_2d13,
            0x6fdf_3b68
        ]
    );
    assert_eq!(random.index, 4);
    random.set_seed(u32::MAX);
    assert_eq!((random.seed, random.index), (u32::MAX, 0));
    assert_eq!(
        random.values,
        INITIAL_SEEDS.map(|value| value.wrapping_sub(1))
    );
}
#[test]
fn source_last_word_overflow_cascades_through_all_words_and_wraps_index() {
    let gate = NativeRandomGate::from_static_initializer();
    let mut random = NativeSkaterRandom {
        seed: 9,
        index: u32::MAX,
        values: [0, 0, 0, 0, 0, u32::MAX],
    };
    assert_eq!(random.try_next(&gate).unwrap(), 0);
    assert_eq!(random.values, [0; 6]);
    assert_eq!((random.seed, random.index), (9, 0));
}
#[test]
fn shared_global_gate_blocks_both_generators_without_mutation_and_reset_does_not_unlock() {
    let gate = NativeRandomGate::from_static_initializer();
    let mut simulation = SimulationCommandRandom::construct_field(Arc::clone(&gate));
    let mut other = NativeSkaterRandom::reset();
    gate.set_locked(true);
    simulation.set_seed_before_input_reset(7);
    assert_eq!(
        simulation.try_advance_for_discarded_range(),
        Err(NativeRandomLocked)
    );
    assert_eq!(other.try_next(&gate), Err(NativeRandomLocked));
    assert_eq!(other.values, INITIAL_SEEDS);
    assert_eq!(simulation.random.index, 0);
    simulation.unlock_for_commands();
    simulation.try_advance_for_discarded_range().unwrap();
    assert_eq!(other.try_next(&gate).unwrap(), 0x559a_51ed);
    assert_eq!(simulation.random.index, 1);
}
