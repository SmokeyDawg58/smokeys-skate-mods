//! Selected serialized per-group cache8298cb10 and `LastInput`826dfbd8.
//! Four published device records enter explicitly; this is not hardware polling
//! or a reconstruction of concurrent manager-half sampling.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "selected input owners await platform polling and Actor scheduling"
    )
)]
use crate::input_pad::{InputPad, InputPadError};
use std::fmt;

/// One actual published device record: native count followed by24 values.
/// A disconnected sample explicitly supplies count0; absence is not a sample.
#[derive(Clone, Copy)]
pub(crate) struct PublishedInputPadSample {
    pub(crate) count: i32,
    pub(crate) values: [f32; 24],
}
impl PublishedInputPadSample {
    const CONSTRUCTOR_ZERO: Self = Self {
        count: 0,
        values: [0.0; 24],
    };
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum InputPadCacheError {
    InitializationRequired,
    AlreadyInitialized,
    DeviceIndex(usize),
    Pad { device: usize, error: InputPadError },
}
impl fmt::Display for InputPadCacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitializationRequired => write!(f, "input group cache requires Init"),
            Self::AlreadyInitialized => write!(f, "input group cache already initialized"),
            Self::DeviceIndex(i) => write!(f, "input device index {i} outside four slots"),
            Self::Pad { device, error } => write!(f, "input device {device}: {error}"),
        }
    }
}
/// One group owns its cache and four distinct retained native pad projections.
/// SIM and PRES require separate instances. No implicit drain or history reset.
pub(crate) struct InputPadCache {
    records: [[PublishedInputPadSample; 4]; 30],
    read: usize,
    write: usize,
    pads: Option<[InputPad; 4]>,
}
impl InputPadCache {
    /// Cache ctor8298cb10: cleared records, read0 then write0.
    pub(crate) const fn new() -> Self {
        Self {
            records: [[PublishedInputPadSample::CONSTRUCTOR_ZERO; 4]; 30],
            read: 0,
            write: 0,
            pads: None,
        }
    }
    /// Selected four-device route of cache Init8298cba8 followed by group pad
    /// construction in `InputManager::Init`826e2788. Actual manager/device
    /// construction is a caller prerequisite, not synthesized by this method.
    pub(crate) fn try_initialize(&mut self) -> Result<(), InputPadCacheError> {
        if self.pads.is_some() {
            return Err(InputPadCacheError::AlreadyInitialized);
        }
        self.write = 1;
        self.read = 0;
        self.pads = Some(std::array::from_fn(|_| InputPad::new()));
        Ok(())
    }
    fn require_initialized(&self) -> Result<(), InputPadCacheError> {
        if self.pads.is_none() {
            Err(InputPadCacheError::InitializationRequired)
        } else {
            Ok(())
        }
    }
    /// `CacheInput`8298cc40 serialized copy boundary. No count validation here:
    /// native copies all0x64 bytes and later `Pad.Update` owns count assertions.
    pub(crate) fn try_cache_input(
        &mut self,
        samples: &[PublishedInputPadSample; 4],
    ) -> Result<(), InputPadCacheError> {
        self.require_initialized()?;
        for (slot, sample) in self.records[self.write].iter_mut().zip(samples) {
            *slot = *sample;
        }
        self.write = (self.write + 1) % 30;
        Ok(())
    }
    fn next_input(&mut self) -> bool {
        let next = (self.read + 1) % 30;
        if next == self.write {
            return false;
        }
        self.read = next;
        true
    }
    /// `LastInput`826dfbd8 drains first, then count/value/Update for each device.
    /// A late invalid record retains the drain and already updated earlier pads.
    pub(crate) fn try_last_input(&mut self) -> Result<bool, InputPadCacheError> {
        self.require_initialized()?;
        if !self.next_input() {
            return Ok(false);
        }
        while self.next_input() {}
        for device in 0..4 {
            let count = self.records[self.read][device].count;
            let values = &self.records[self.read][device].values;
            self.pads.as_mut().expect("initialized group")[device]
                .try_update(values, count)
                .map_err(|error| InputPadCacheError::Pad { device, error })?;
        }
        Ok(true)
    }
    /// Flush8298cd88 discards queued positions, preserving bytes and pad history.
    pub(crate) fn try_flush(&mut self) -> Result<(), InputPadCacheError> {
        self.require_initialized()?;
        self.read = (self.write + 29) % 30;
        Ok(())
    }
    pub(crate) fn pad(&self, device: usize) -> Result<&InputPad, InputPadCacheError> {
        self.pads
            .as_ref()
            .ok_or(InputPadCacheError::InitializationRequired)?
            .get(device)
            .ok_or(InputPadCacheError::DeviceIndex(device))
    }
    /// Borrow the same retained pad for the existing input-map provider.
    pub(crate) fn pad_mut(&mut self, device: usize) -> Result<&mut InputPad, InputPadCacheError> {
        self.pads
            .as_mut()
            .ok_or(InputPadCacheError::InitializationRequired)?
            .get_mut(device)
            .ok_or(InputPadCacheError::DeviceIndex(device))
    }
}
#[cfg(test)]
#[path = "input_pad_cache_tests.rs"]
mod tests;
