//! Retained `SpeedWobble` constructor 0x82b2eaf8 and reset 0x82b26620.
//! No numeric wobble update or guessed amplitude is implemented here.
use crate::attribute_runtime::{
    AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use std::sync::Arc;

pub(crate) const SPEED_WOBBLE_CLASS: u64 = 0x5be4_6545_de48_5c98;
pub(crate) const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
const LAYOUT_SIZE: usize = 0x100;

pub(crate) struct SkateboardSpeedWobbleAttributes {
    instance: AttributeRuntimeInstance,
}
impl SkateboardSpeedWobbleAttributes {
    /// Generated wrapper 0x82b25408: bind once with message port zero. Null
    /// cached layout selects `DefaultDataArea(0x100)`, without parent lookup.
    /// The immutable instance fixes that choice here; checked byte-range
    /// validation is deferred until a consumer actually requests the layout.
    fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            instance: vault.instance(SPEED_WOBBLE_CLASS, DEFAULT_COLLECTION),
        }
    }
    pub(crate) fn try_capture_layout(
        &self,
    ) -> Result<SkateboardSpeedWobbleLayout, AttributeRuntimeError> {
        Ok(SkateboardSpeedWobbleLayout(
            self.instance.try_cached_layout(LAYOUT_SIZE)?,
        ))
    }
}
pub(crate) struct SkateboardSpeedWobbleLayout(Option<AttributeRuntimeData>);
impl SkateboardSpeedWobbleLayout {
    pub(crate) fn uses_default(&self) -> bool {
        self.0.is_none()
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        match &self.0 {
            Some(data) => data.bytes(),
            None => &[0; LAYOUT_SIZE],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkateboardSpeedWobbleAllocation {
    Allocated,
    Null,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkateboardSpeedWobblePhase {
    Unattempted,
    AllocationNull,
    Constructed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkateboardSpeedWobbleError {
    AlreadyAttempted,
    NotConstructed,
}

pub(crate) struct SkateboardSpeedWobble {
    attributes: Arc<SkateboardSpeedWobbleAttributes>,
    active: bool,
    time: f32,
    amplitude: Option<f32>,
    frequency: f32,
    last_tilt: f32,
    angle_position: f32,
    effect: f32,
    wobble_tilt: f32,
}
impl SkateboardSpeedWobble {
    fn new(vault: &Arc<AttributeRuntimeVault>) -> Self {
        // The actual instance construction precedes every represented scalar.
        let attributes = Arc::new(SkateboardSpeedWobbleAttributes::bind(vault));
        Self {
            attributes,
            active: false,
            time: 0.0,
            amplitude: None,
            frequency: 0.0,
            last_tilt: 0.0,
            angle_position: 0.0,
            effect: 0.0,
            wobble_tilt: 0.0,
        }
    }
    /// 0x82b26630..48 leaves amplitude+14 and the attribute instance untouched.
    fn reset(&mut self) {
        self.active = false;
        self.time = 0.0;
        self.frequency = 0.0;
        self.last_tilt = 0.0;
        self.angle_position = 0.0;
        self.effect = 0.0;
        self.wobble_tilt = 0.0;
    }
    pub(crate) fn attributes(&self) -> &Arc<SkateboardSpeedWobbleAttributes> {
        &self.attributes
    }
    pub(crate) const fn effect(&self) -> f32 {
        self.effect
    }
    pub(crate) const fn wobble_tilt(&self) -> f32 {
        self.wobble_tilt
    }
    /// Bit-preserving selected fields for inspection, including unknown amplitude.
    pub(crate) fn state_bits(&self) -> (bool, [u32; 6], Option<u32>) {
        (
            self.active,
            [
                self.time,
                self.frequency,
                self.last_tilt,
                self.angle_position,
                self.effect,
                self.wobble_tilt,
            ]
            .map(f32::to_bits),
            self.amplitude.map(f32::to_bits),
        )
    }
    #[cfg(test)]
    pub(crate) fn seed_reset_fields_for_test(&mut self, value: f32, amplitude: Option<f32>) {
        self.active = true;
        self.time = value;
        self.frequency = value;
        self.last_tilt = value;
        self.angle_position = value;
        self.effect = value;
        self.wobble_tilt = value;
        self.amplitude = amplitude;
    }
}

/// Actual outer+10c member: initial null at38b2c is distinct from the later
/// allocation result38ba4. Host allocation follows Rust policy; `Allocated`
/// selects the successful native branch, not allocator parity.
pub(crate) struct SkateboardSpeedWobbleOwner {
    phase: SkateboardSpeedWobblePhase,
    value: Option<Box<SkateboardSpeedWobble>>,
}
impl SkateboardSpeedWobbleOwner {
    pub(crate) fn new() -> Self {
        Self {
            phase: SkateboardSpeedWobblePhase::Unattempted,
            value: None,
        }
    }
    pub(crate) fn phase(&self) -> SkateboardSpeedWobblePhase {
        self.phase
    }
    pub(crate) fn value_mut(&mut self) -> Option<&mut SkateboardSpeedWobble> {
        self.value.as_deref_mut()
    }
    pub(crate) fn value(&self) -> Option<&SkateboardSpeedWobble> {
        self.value.as_deref()
    }
    pub(crate) fn try_construct(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        allocation: SkateboardSpeedWobbleAllocation,
    ) -> Result<(), SkateboardSpeedWobbleError> {
        if self.phase != SkateboardSpeedWobblePhase::Unattempted {
            return Err(SkateboardSpeedWobbleError::AlreadyAttempted);
        }
        match allocation {
            SkateboardSpeedWobbleAllocation::Null => {
                self.phase = SkateboardSpeedWobblePhase::AllocationNull;
            }
            SkateboardSpeedWobbleAllocation::Allocated => {
                self.value = Some(Box::new(SkateboardSpeedWobble::new(vault)));
                self.phase = SkateboardSpeedWobblePhase::Constructed;
            }
        }
        Ok(())
    }
    pub(crate) fn try_reset(&mut self) -> Result<(), SkateboardSpeedWobbleError> {
        self.value
            .as_mut()
            .ok_or(SkateboardSpeedWobbleError::NotConstructed)?
            .reset();
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn value_mut_for_test(&mut self) -> Option<&mut SkateboardSpeedWobble> {
        self.value.as_deref_mut()
    }
}

#[cfg(test)]
#[path = "skateboard_speed_wobble_tests.rs"]
mod tests;

#[path = "skateboard_speed_wobble_calculate.rs"]
pub(crate) mod calculate;
