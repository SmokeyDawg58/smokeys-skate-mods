use super::*;
use crate::{
    input_cvar_runtime::InputCvarReferences,
    input_pad::device::tests::{Platform, Remap},
    reckoning_numeric::HostReckoningNumeric,
    skeleton_collision_numeric::HostSkeletonCollisionNumeric,
};
#[test]
fn incomplete_poll_preserves_inactive_prefix_and_does_not_publish() {
    let mut m = InputDeviceManager::<u32>::new();
    assert!(m.active.is_none());
    m.initialize().unwrap();
    let mut p = Platform::new();
    let refs = InputCvarReferences::new_static_initialized();
    for device in m.devices_mut().unwrap() {
        device
            .reset_actuators(&mut p, &refs, &mut HostReckoningNumeric)
            .unwrap();
    }
    p.fail_index = Some(2);
    assert!(matches!(
        m.poll(&mut p, &mut HostSkeletonCollisionNumeric, &mut Remap),
        Err(DeviceManagerError::Sample { device: 2, .. })
    ));
    assert_eq!(m.active_index, 0);
    assert_eq!(m.records[1][0].count, 24);
    assert_eq!(m.records[1][1].count, 24);
    assert_eq!(m.records[1][2].count, 0);
    assert_eq!(m.active.unwrap()[1], [true, true, false, false]);
    let mut sim = InputPadCache::new();
    sim.try_initialize().unwrap();
    m.cache_into(&mut sim).unwrap();
    sim.try_last_input().unwrap();
    assert_eq!(sim.pad(0).unwrap().num_buttons(), 0);
}
#[test]
fn poll_publication_enters_existing_independent_group_caches() {
    let mut m = InputDeviceManager::<u32>::new();
    m.initialize().unwrap();
    let mut p = Platform::new();
    let refs = InputCvarReferences::new_static_initialized();
    for device in m.devices_mut().unwrap() {
        device
            .reset_actuators(&mut p, &refs, &mut HostReckoningNumeric)
            .unwrap();
    }
    m.set_external_remap(Some(&42));
    m.poll(&mut p, &mut HostSkeletonCollisionNumeric, &mut Remap)
        .unwrap();
    let mut sim = InputPadCache::new();
    let mut pres = InputPadCache::new();
    sim.try_initialize().unwrap();
    pres.try_initialize().unwrap();
    m.cache_into(&mut sim).unwrap();
    m.cache_into(&mut pres).unwrap();
    sim.try_last_input().unwrap();
    assert_eq!(
        sim.pad(0).unwrap().value(0).unwrap().to_bits(),
        0.75f32.to_bits()
    );
    assert_eq!(pres.pad(0).unwrap().num_buttons(), 0);
    pres.try_last_input().unwrap();
    assert_eq!(pres.pad(0).unwrap().num_buttons(), 24);
    p.status = 1;
    m.poll(&mut p, &mut HostSkeletonCollisionNumeric, &mut Remap)
        .unwrap();
    m.cache_into(&mut sim).unwrap();
    sim.try_last_input().unwrap();
    assert_eq!(sim.pad(0).unwrap().num_buttons(), 0);
    assert_eq!(pres.pad(0).unwrap().num_buttons(), 24);
}
