use super::*;
use crate::attribute_runtime::{
    AttributeRuntimeVault,
    tests::{collection, graph, set_dependency_bytes},
};
const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;
fn vault(class: u64, bytes: Vec<u8>) -> Arc<AttributeRuntimeVault> {
    let mut vault = graph(class, vec![]);
    collection(&mut vault, class, DEFAULT, 0, Some(64), vec![]);
    let mut data = vec![0; 64];
    data.extend(bytes);
    set_dependency_bytes(&mut vault, 1, data);
    Arc::new(vault)
}
#[test]
fn gravity_reads_actual_full_layout_and_rejects_equal_owner_substitution() {
    let words = [0x8000_0000_u32, 0xc11c_f5c3, 0, 0x7fc1_2345];
    let mut bytes = vec![0; 32];
    for (i, w) in words.iter().enumerate() {
        bytes[i * 4..i * 4 + 4].copy_from_slice(&w.to_be_bytes());
    }
    let vault = vault(0x0429_3d0f_2adc_fbb4, bytes);
    let physics = vault.physics(DEFAULT);
    let other = vault.physics(DEFAULT);
    let mut ground = SkateboardGroundNormalVaultAttributes::bind(&vault);
    let mut reader = SkateboardUpdateUpVaultAttributes::new(&mut ground, &physics);
    let global = reader.capture_physics_global().unwrap();
    let layout = reader.capture_physics_layout(&global).unwrap();
    let value = reader.world_gravity(&layout).unwrap();
    assert_eq!(value.xyz.map(f32::to_bits), words[..3]);
    assert_eq!(value.w.unwrap().to_bits(), words[3]);
    assert!(matches!(
        reader.capture_physics_layout(&SkateboardUpdateUpPhysicsToken(&other)),
        Err(SkateboardUpdateUpAttributeError::ForeignPhysics)
    ));
    assert!(matches!(
        reader.world_gravity(&SkateboardUpdateUpPhysicsToken(&other)),
        Err(SkateboardUpdateUpAttributeError::ForeignPhysics)
    ));
}
#[test]
fn shared_ground_occurrence_and_graph_decoder_are_reused() {
    let mut bytes = vec![0; 0x4d0];
    for i in 0u8..8 {
        let n = usize::from(i);
        bytes[0x460 + n * 4..0x464 + n * 4].copy_from_slice(&f32::from(i).to_be_bytes());
        bytes[0x480 + n * 4..0x484 + n * 4].copy_from_slice(&(f32::from(i) * 2.0).to_be_bytes());
    }
    let vault = vault(SkateboardGroundNormalVaultAttributes::CLASS, bytes);
    let physics = vault.physics(DEFAULT);
    let mut ground = SkateboardGroundNormalVaultAttributes::bind(&vault);
    let mut other = SkateboardGroundNormalVaultAttributes::bind(&vault);
    let global = ground.capture_global().unwrap();
    let foreign = other.capture_global().unwrap();
    let layout = {
        let mut reader = SkateboardUpdateUpVaultAttributes::new(&mut ground, &physics);
        assert!(matches!(
            reader.capture_reckoning_layout(&foreign),
            Err(SkateboardUpdateUpAttributeError::Ground(
                SkateboardGroundNormalAttributeError::ForeignGlobal
            ))
        ));
        let layout = reader.capture_reckoning_layout(&global).unwrap();
        assert_eq!(
            reader
                .evaluate_graph8(&layout, 0x460, 2.5)
                .unwrap()
                .to_bits(),
            5.0f32.to_bits()
        );
        layout
    };
    let mut reader = SkateboardUpdateUpVaultAttributes::new(&mut other, &physics);
    assert!(matches!(
        reader.evaluate_graph8(&layout, 0x460, 2.5),
        Err(SkateboardUpdateUpAttributeError::ForeignLayout)
    ));
}
#[test]
fn layout_capture_defers_malformed_range_until_actual_graph_read() {
    let vault = vault(SkateboardGroundNormalVaultAttributes::CLASS, vec![0; 4]);
    let physics = vault.physics(DEFAULT);
    let mut ground = SkateboardGroundNormalVaultAttributes::bind(&vault);
    let mut reader = SkateboardUpdateUpVaultAttributes::new(&mut ground, &physics);
    let global = reader.capture_global().unwrap();
    let layout = reader.capture_reckoning_layout(&global).unwrap();
    assert!(matches!(
        reader.evaluate_graph8(&layout, 0x460, 0.0),
        Err(SkateboardUpdateUpAttributeError::Graph(_))
    ));
}
