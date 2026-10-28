//! Retained `SkaterPresEntity` cloth admission and X store, `827e5884..827e59d0`.
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
};
use std::sync::Arc;

const MARQUEE_CLASS: u64 = 0x91bc_a669_3efc_9aa7;
const CLOTH_EXCLUDED_MARQUEE: u64 = 0x2275_b900_5016_6969;
#[derive(Default)]
pub(crate) struct PresentationRopaState {
    attempted: bool,
    disabled: Option<bool>,
    vertex_x: Option<f32>,
    no_live_effects: bool,
}
#[derive(Debug)]
pub(crate) enum PresentationRopaError {
    AlreadyAttempted,
    Attribute(AttributeRuntimeError),
    MissingSchema,
    Cvar(InputCvarError),
    EnabledClothPending,
    NotConstructed,
}
impl std::fmt::Display for PresentationRopaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAttempted => f.write_str("ropa constructor already attempted"),
            Self::Attribute(e) => write!(f, "ropa schema: {e}"),
            Self::MissingSchema => f.write_str("ropa excluded marquee schema pointer unavailable"),
            Self::Cvar(e) => write!(f, "ropa enable cvar: {e:?}"),
            Self::EnabledClothPending => {
                f.write_str("native InitCloth must complete before vertex parameter store")
            }
            Self::NotConstructed => {
                f.write_str("ropa constructor has not reached vertex parameter store")
            }
        }
    }
}
impl PresentationRopaState {
    pub(crate) fn is_attempted(&self) -> bool {
        self.attempted
    }
    pub(crate) fn try_construct(
        &mut self,
        schema: &str,
        local: bool,
        supplied_disabled: bool,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
    ) -> Result<(), PresentationRopaError> {
        self.try_construct_with_effects(schema, local, supplied_disabled, vault, cvars, None)
    }
    pub(crate) fn try_construct_with_effects(
        &mut self,
        schema: &str,
        local: bool,
        supplied_disabled: bool,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
        has_live_effects: Option<bool>,
    ) -> Result<(), PresentationRopaError> {
        use PresentationRopaError as E;
        if self.attempted {
            return Err(E::AlreadyAttempted);
        }
        self.attempted = true;
        // Native comparison occurs even for a non-local/previously disabled entity.
        let excluded = vault
            .instance(MARQUEE_CLASS, CLOTH_EXCLUDED_MARQUEE)
            .try_cached_layout_text(0)
            .map_err(E::Attribute)?
            .ok_or(E::MissingSchema)?;
        let disabled = !local || supplied_disabled || schema.as_bytes() == excluded.bytes();
        self.disabled = Some(disabled);
        if !disabled && cvars.try_enable_ropa().map_err(E::Cvar)? {
            if has_live_effects.is_none() {
                return Err(E::EnabledClothPending);
            }
            self.no_live_effects=has_live_effects==Some(false);
        }else{
            self.no_live_effects=true;
        }
        // InitCloth's zero-effect path has no jobs or CPU skinning to update.
        self.vertex_x = Some(1.0);
        Ok(())
    }
    pub(crate) fn vertex_x(&self) -> Result<f32, PresentationRopaError> {
        self.vertex_x.ok_or(PresentationRopaError::NotConstructed)
    }
    pub(crate) fn cloth_disabled(&self) -> Option<bool> {
        self.disabled
    }
    pub(crate) fn has_no_live_effects(&self) -> bool {
        self.no_live_effects
    }
    /// Update827e8568 writes zero after the first successful ClothInit.
    pub(crate) fn cloth_initialized(&mut self){self.vertex_x=Some(0.0);}
}
