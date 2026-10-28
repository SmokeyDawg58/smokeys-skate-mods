//! Lazy immutable-Vault provider for grounded `Reckoning` (0x82ddd468).
//!
//! Own/generated and global/generated instances are retained separately even
//! when both select the same collection. Generated constructor 0x828b0ff0
//! substitutes DefaultDataArea(0x4d0) for a null cached layout. Captures below
//! defer checked host range validation, but not source collection selection.
//! No editor Change, mutable global replacement or concurrent layout mutation
//! is modeled. Ordinary FETCH reruns lookup and does not decode its value until
//! the caller loads it; dead and conditional pointers therefore remain lazy.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "concrete Vault provider awaits the live Reckoning caller"
    )
)]

use crate::{
    attribute_runtime::{
        AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
        AttributeRuntimeVault,
    },
    physics::PointGraph,
};
use std::sync::Arc;

pub(crate) const RECKONING_CLASS: u64 = 0x7ce1_b6b5_8170_9c6f;
const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
const LAYOUT_SIZE: usize = 0x4d0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReckoningAttributeScope {
    Own,
    Global,
}

pub(crate) trait ReckoningAttributeRead {
    type Layout;
    type Pointer;
    type Error;
    fn capture_layout(
        &mut self,
        scope: ReckoningAttributeScope,
    ) -> Result<Self::Layout, Self::Error>;
    fn layout_float(&mut self, layout: &Self::Layout, offset: usize) -> Result<f32, Self::Error>;
    fn evaluate_graph8(
        &mut self,
        layout: &Self::Layout,
        offset: usize,
        input: f32,
    ) -> Result<f32, Self::Error>;
    fn lookup(
        &mut self,
        scope: ReckoningAttributeScope,
        key: u64,
    ) -> Result<Self::Pointer, Self::Error>;
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
    fn signed_word(&mut self, pointer: &Self::Pointer) -> Result<i32, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum ReckoningAttributeError {
    Runtime(AttributeRuntimeError),
    LayoutRange { offset: usize, size: usize },
}

impl From<AttributeRuntimeError> for ReckoningAttributeError {
    fn from(error: AttributeRuntimeError) -> Self {
        Self::Runtime(error)
    }
}
impl std::fmt::Display for ReckoningAttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(error) => write!(f, "Reckoning attribute: {error}"),
            Self::LayoutRange { offset, size } => write!(
                f,
                "Reckoning cached layout range {offset:#x}+{size:#x} is unaligned or outside {LAYOUT_SIZE:#x}"
            ),
        }
    }
}
impl std::error::Error for ReckoningAttributeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Runtime(error) => Some(error),
            Self::LayoutRange { .. } => None,
        }
    }
}

/// A captured actual range; None is the generated constructor's established
/// zero `DefaultDataArea4d0`, not arbitrary invalid data or a generic node fallback.
pub(crate) struct ReckoningCapturedLayout(Option<AttributeRuntimeData>);

#[derive(Clone)]
pub(crate) struct ReckoningVaultAttributes {
    body_spin: AttributeRuntimeInstance,
    own: AttributeRuntimeInstance,
    global: AttributeRuntimeInstance,
}

impl ReckoningVaultAttributes {
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            body_spin: vault.instance(0x05ec_9cb0_3a5b_2eee, DEFAULT_COLLECTION),
            own: vault.instance(RECKONING_CLASS, DEFAULT_COLLECTION),
            global: vault.instance(RECKONING_CLASS, DEFAULT_COLLECTION),
        }
    }

    fn instance(&self, scope: ReckoningAttributeScope) -> &AttributeRuntimeInstance {
        match scope {
            ReckoningAttributeScope::Own => &self.own,
            ReckoningAttributeScope::Global => &self.global,
        }
    }
}

impl ReckoningCapturedLayout {
    pub(crate) fn try_capture(
        instance: &AttributeRuntimeInstance,
    ) -> Result<Self, ReckoningAttributeError> {
        Ok(Self(instance.try_cached_layout(LAYOUT_SIZE)?))
    }
    pub(crate) fn evaluate_graph8(
        &self,
        offset: usize,
        input: f32,
    ) -> Result<f32, ReckoningAttributeError> {
        // Checked full-region snapshot of an immutable captured allocation.
        // Graph evaluation itself preserves original selected-read arithmetic.
        // DeckAngleUsageVsSpeed passes 0x110, not its negative-graph base0x100;
        // its four min/max metadata words are NOT consumed by this caller.
        ReckoningCapturedLayout::check_range(offset, 64)?;
        let graph = PointGraph {
            x: std::array::from_fn::<_, 8, _>(|index| {
                f32::from_be_bytes(
                    self.word(offset + index * 4)
                        .expect("checked graph X range"),
                )
            }),
            y: std::array::from_fn::<_, 8, _>(|index| {
                f32::from_be_bytes(
                    self.word(offset + 32 + index * 4)
                        .expect("checked graph Y range"),
                )
            }),
        };
        Ok(graph.evaluate(input))
    }

    fn check_range(offset: usize, size: usize) -> Result<(), ReckoningAttributeError> {
        if offset % 4 != 0 || offset.checked_add(size).is_none_or(|end| end > LAYOUT_SIZE) {
            return Err(ReckoningAttributeError::LayoutRange { offset, size });
        }
        Ok(())
    }

    fn word(&self, offset: usize) -> Result<[u8; 4], ReckoningAttributeError> {
        Self::check_range(offset, 4)?;
        Ok(self.0.as_ref().map_or([0; 4], |data| {
            data.bytes()[offset..offset + 4]
                .try_into()
                .expect("checked captured layout word")
        }))
    }
}

impl ReckoningAttributeRead for ReckoningVaultAttributes {
    type Layout = ReckoningCapturedLayout;
    type Pointer = Option<AttributeRuntimeData>;
    type Error = ReckoningAttributeError;

    fn capture_layout(
        &mut self,
        scope: ReckoningAttributeScope,
    ) -> Result<Self::Layout, Self::Error> {
        Ok(ReckoningCapturedLayout(
            self.instance(scope).try_cached_layout(LAYOUT_SIZE)?,
        ))
    }

    fn layout_float(&mut self, layout: &Self::Layout, offset: usize) -> Result<f32, Self::Error> {
        Ok(f32::from_be_bytes(layout.word(offset)?))
    }

    fn evaluate_graph8(
        &mut self,
        layout: &Self::Layout,
        offset: usize,
        input: f32,
    ) -> Result<f32, Self::Error> {
        layout.evaluate_graph8(offset, input)
    }

    fn lookup(
        &mut self,
        scope: ReckoningAttributeScope,
        key: u64,
    ) -> Result<Self::Pointer, Self::Error> {
        Ok(self.instance(scope).try_get_data(key, 0)?)
    }

    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        Ok(f32::from_be_bytes(pointer_word(pointer.as_ref())?))
    }

    fn signed_word(&mut self, pointer: &Self::Pointer) -> Result<i32, Self::Error> {
        Ok(i32::from_be_bytes(pointer_word(pointer.as_ref())?))
    }
}

fn pointer_word(
    pointer: Option<&AttributeRuntimeData>,
) -> Result<[u8; 4], ReckoningAttributeError> {
    let Some(data) = pointer else {
        return Ok([0; 4]);
    }; // Actual DefaultDataArea4.
    data.bytes().try_into().map_err(|_| {
        AttributeRuntimeError::ScalarSize {
            size: data.bytes().len(),
        }
        .into()
    })
}

#[cfg(test)]
#[path = "reckoning_attributes_tests.rs"]
mod tests;

#[path = "reckoning_body_spin_air_attributes.rs"]
mod body_spin_air;
