//! Selected serialized device event, not the native whole application timer.
//! A real retained target supplies manager/caches; OS calls stay in this app.
use crate::windows_pad_platform::WindowsPadPlatform;
use skate2_core::{SelectedDeviceInputEvents, SelectedInputGroup};

/// Native `TimerCallback82729770` device/cache order. Actual host scheduling,
/// front-end hotplug delivery and actuator startup remain caller prerequisites.
pub(crate) fn poll_and_cache<T: SelectedDeviceInputEvents<WindowsPadPlatform>>(
    target: &mut T,
    platform: &mut WindowsPadPlatform,
) -> Result<(), T::Error> {
    target.poll_hardware(platform)?;
    target.cache_input(SelectedInputGroup::Simulation)?;
    target.cache_input(SelectedInputGroup::Presentation)
}

/// The actual SIM advancement result gates later selected application work.
/// This does not replace preceding native network Idle or later Flow/vibration.
pub(crate) fn consume_sim<T: SelectedDeviceInputEvents<WindowsPadPlatform>>(
    target: &mut T,
) -> Result<bool, T::Error> {
    target.last_input(SelectedInputGroup::Simulation)
}
