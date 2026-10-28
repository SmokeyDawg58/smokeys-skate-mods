use super::*;
use crate::{
    input_cvar_runtime::InputCvarReferences,
    input_pad::device::tests::{Platform, Remap},
    reckoning_numeric::HostReckoningNumeric,
    skeleton_collision_numeric::HostSkeletonCollisionNumeric,
};
type Runtime = SelectedDeviceInputRuntime<u32, HostSkeletonCollisionNumeric, Remap>;
fn owners(reset: bool) -> (Runtime, Platform, Arc<SelectedInputMap>) {
    let mut devices = InputDeviceManager::new();
    devices.initialize().unwrap();
    let mut platform = Platform::new();
    if reset {
        for device in devices.devices_mut().unwrap() {
            device
                .reset_actuators(
                    &mut platform,
                    &InputCvarReferences::new_static_initialized(),
                    &mut HostReckoningNumeric,
                )
                .unwrap();
        }
    }
    devices.set_external_remap(Some(&42));
    let mut sim = InputPadCache::new();
    sim.try_initialize().unwrap();
    let mut pres = InputPadCache::new();
    pres.try_initialize().unwrap();
    let map =
        Arc::new(SelectedInputMap::parse(&crate::input_map::tests::config("Button0")).unwrap());
    let runtime = Runtime::from_owners(
        devices,
        sim,
        pres,
        Arc::clone(&map),
        HostSkeletonCollisionNumeric,
        Remap,
    );
    (runtime, platform, map)
}
#[test]
fn direct_gameplay_sample_is_current_half_not_sim_pad_and_pres_drain_is_independent() {
    let (mut runtime, mut platform, map) = owners(true);
    assert!(Arc::ptr_eq(runtime.map(), &map));
    runtime.poll_hardware(&mut platform).unwrap();
    for group in [
        SelectedInputGroup::Simulation,
        SelectedInputGroup::Presentation,
    ] {
        <Runtime as SelectedDeviceInputEvents<Platform>>::cache_input(&mut runtime, group).unwrap();
    }
    assert!(
        <Runtime as SelectedDeviceInputEvents<Platform>>::last_input(
            &mut runtime,
            SelectedInputGroup::Simulation
        )
        .unwrap()
    );
    assert_eq!(
        runtime
            .cache(SelectedInputGroup::Simulation)
            .pad(0)
            .unwrap()
            .num_buttons(),
        24
    );
    assert_eq!(
        runtime
            .cache(SelectedInputGroup::Presentation)
            .pad(0)
            .unwrap()
            .num_buttons(),
        0
    );
    // A later actual disconnected sample publishes while SIM still holds its
    // earlier consumed pad. PlayerUI must copy the current manager count0.
    platform.status = 1;
    runtime.poll_hardware(&mut platform).unwrap();
    assert_eq!(runtime.raw_current_cache_line(0).unwrap().count, 0);
    assert_eq!(
        runtime
            .cache(SelectedInputGroup::Simulation)
            .pad(0)
            .unwrap()
            .num_buttons(),
        24
    );
    assert!(
        !<Runtime as SelectedDeviceInputEvents<Platform>>::last_input(
            &mut runtime,
            SelectedInputGroup::Simulation
        )
        .unwrap()
    );
    assert!(
        <Runtime as SelectedDeviceInputEvents<Platform>>::last_input(
            &mut runtime,
            SelectedInputGroup::Presentation
        )
        .unwrap()
    );
    assert_eq!(
        runtime
            .cache(SelectedInputGroup::Presentation)
            .pad(0)
            .unwrap()
            .num_buttons(),
        24
    );
    // Native disconnected count does not clear the reused half's old values.
    runtime.poll_hardware(&mut platform).unwrap();
    let raw = runtime.raw_current_cache_line(0).unwrap();
    assert_eq!(raw.count, 0);
    assert_eq!(raw.values[0].to_bits(), 0.75_f32.to_bits());
    assert!(matches!(
        runtime.raw_current_cache_line(4),
        Err(InputPadCacheError::DeviceIndex(4))
    ));
}
#[test]
fn successful_hardware_state_without_an_actuator_reset_reads_the_pad_capabilities_once() {
    let (mut runtime, mut platform, _) = owners(false);
    runtime.poll_hardware(&mut platform).unwrap();
    assert_eq!(runtime.raw_current_cache_line(0).unwrap().count, 24);
    let capabilities = |platform: &Platform| platform.calls.iter().filter(|&&(call, _)| call == 1).count();
    assert_eq!(capabilities(&platform), 4);
    platform.calls.clear();
    runtime.poll_hardware(&mut platform).unwrap();
    assert_eq!(capabilities(&platform), 0);
}
