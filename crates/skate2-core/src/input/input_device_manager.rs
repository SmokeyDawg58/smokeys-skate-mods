//! Serialized native device manager double buffer; group caches remain separate.
use crate::{
    input_pad::device::{PadPlatform, PadRemap, PadSampleError, XenonPadDevice},
    input_pad_cache::{InputPadCache, InputPadCacheError, PublishedInputPadSample},
    skeleton_collision_numeric::SkeletonCollisionNumeric,
};

#[derive(Debug, PartialEq)]
pub(crate) enum DeviceManagerError<P, N, R> {
    InitializationRequired,
    AlreadyInitialized,
    Sample {
        device: usize,
        error: PadSampleError<P, N, R>,
    },
}
pub(crate) struct InputDeviceManager<R> {
    devices: Option<[XenonPadDevice<R>; 4]>,
    records: [[PublishedInputPadSample; 4]; 2],
    active: Option<[[bool; 4]; 2]>,
    active_index: usize,
}
impl<R: Clone> InputDeviceManager<R> {
    pub(crate) const fn new() -> Self {
        Self {
            devices: None,
            records: [[PublishedInputPadSample {
                count: 0,
                values: [0.0; 24],
            }; 4]; 2],
            active: None,
            active_index: 0,
        }
    }
    /// Selected four successful native allocations; no failed allocation repair.
    pub(crate) fn initialize(&mut self) -> Result<(), DeviceManagerError<(), (), ()>> {
        if self.devices.is_some() {
            return Err(DeviceManagerError::AlreadyInitialized);
        }
        self.devices = Some(std::array::from_fn(|i| {
            XenonPadDevice::new(u32::try_from(i).expect("four indices"))
        }));
        self.active = Some([[false; 4]; 2]);
        self.records = [[PublishedInputPadSample {
            count: 0,
            values: [0.0; 24],
        }; 4]; 2];
        self.active_index = 0;
        Ok(())
    }
    pub(crate) fn devices_mut(&mut self) -> Option<&mut [XenonPadDevice<R>; 4]> {
        self.devices.as_mut()
    }
    pub(crate) fn set_external_remap(&mut self, remap: Option<&R>) {
        if let Some(devices) = self.devices.as_mut() {
            for device in devices {
                device.set_remap(remap.cloned());
            }
        }
    }
    #[expect(
        clippy::type_complexity,
        reason = "preserves separate platform, numeric and remap errors"
    )]
    pub(crate) fn poll<P: PadPlatform, N: SkeletonCollisionNumeric, M: PadRemap<R>>(
        &mut self,
        platform: &mut P,
        numeric: &mut N,
        remap: &mut M,
    ) -> Result<(), DeviceManagerError<P::Error, N::Error, M::Error>> {
        let devices = self
            .devices
            .as_mut()
            .ok_or(DeviceManagerError::InitializationRequired)?;
        let inactive = usize::from(self.active_index == 0);
        for (index, device) in devices.iter_mut().enumerate() {
            let record = &mut self.records[inactive][index];
            let count = device
                .sample(&mut record.values, platform, numeric, remap)
                .map_err(|error| DeviceManagerError::Sample {
                    device: index,
                    error,
                })?;
            record.count = count;
            self.active.as_mut().expect("initialized active flags")[inactive][index] = count != 0;
        }
        self.active_index = inactive;
        Ok(())
    }
    /// `GetRawInputCacheLine826dfc90`: one serialized active-half observation,
    /// then the selected complete count/24-channel record. This does not drain
    /// a group cache, update a pad, remap values or substitute disconnected zeros.
    pub(crate) fn raw_current_cache_line(
        &self,
        device: usize,
    ) -> Result<PublishedInputPadSample, InputPadCacheError> {
        if self.devices.is_none() {
            return Err(InputPadCacheError::InitializationRequired);
        }
        let active = self.active_index;
        self.records[active]
            .get(device)
            .copied()
            .ok_or(InputPadCacheError::DeviceIndex(device))
    }
    /// Same retained active half copied into an existing SIM or PRES cache.
    /// Caller schedules each group separately; polling never drains either.
    pub(crate) fn cache_into(&self, cache: &mut InputPadCache) -> Result<(), InputPadCacheError> {
        if self.devices.is_none() {
            return Err(InputPadCacheError::InitializationRequired);
        }
        cache.try_cache_input(&self.records[self.active_index])
    }
}
#[path = "input_selected_device_runtime.rs"]
pub(crate) mod selected_runtime;
#[cfg(test)]
#[path = "input_device_manager_tests.rs"]
mod tests;
