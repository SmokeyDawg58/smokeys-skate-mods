//! Unregistered child of existing slide/collision attributes; reuses friction.
use super::{
    SkateboardSelectedSurface, SkateboardSlideCollisionAttributeError,
    SkateboardSlideCollisionAttributeRead, SkateboardSlideCollisionPointer,
    SkateboardSlideCollisionVaultAttributes, SlideCollisionGlobal,
};
use crate::{
    attribute_runtime::{
        AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
        AttributeRuntimeVault,
    },
    physics::PointGraph,
    skateboard_attributes::{
        SkateboardAttributeClass, SkateboardAttributeError, SkateboardAttributeRead,
        SkateboardAttributes,
    },
    skateboard_force_runtime::entry::speed_model_update::{SpeedGlobal, SpeedGraph},
};
use std::sync::Arc;
pub(crate) struct ConservationAttributes(AttributeRuntimeInstance);
impl ConservationAttributes {
    /// One selected global+c4 binding, constructor828b2698/26a8/26b4.
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self(vault.instance(0xa099_c166_f4be_639f, 0xd7ed_bd36_2d7d_2152))
    }
}
pub(crate) enum SpeedAttributePointer {
    Conservation(Option<AttributeRuntimeData>),
    Manual(Option<AttributeRuntimeData>),
    Friction(SkateboardSlideCollisionPointer),
}
pub(crate) struct SpeedGraphPointer {
    data: Option<AttributeRuntimeData>,
    offset: usize,
}
#[derive(Debug)]
pub(crate) enum SpeedAttributeError {
    Runtime(AttributeRuntimeError),
    Manual(SkateboardAttributeError),
    Friction(SkateboardSlideCollisionAttributeError),
}
impl std::fmt::Display for SpeedAttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(e) => e.fmt(f),
            Self::Manual(e) => e.fmt(f),
            Self::Friction(e) => e.fmt(f),
        }
    }
}
pub(crate) struct SpeedModelVault<'a> {
    pub(crate) conservation: &'a ConservationAttributes,
    pub(crate) manual: &'a mut SkateboardAttributes,
    pub(crate) friction: &'a mut SkateboardSlideCollisionVaultAttributes,
}
impl SpeedModelVault<'_> {
    pub(crate) fn capture(
        &mut self,
        global: SpeedGlobal,
        key: u64,
    ) -> Result<SpeedAttributePointer, SpeedAttributeError> {
        Ok(match global {
            SpeedGlobal::Conservation => SpeedAttributePointer::Conservation(
                self.conservation
                    .0
                    .try_get_data(key, 0)
                    .map_err(SpeedAttributeError::Runtime)?,
            ),
            SpeedGlobal::Manual => SpeedAttributePointer::Manual(
                self.manual
                    .lookup(SkateboardAttributeClass::Manual, key)
                    .map_err(SpeedAttributeError::Manual)?,
            ),
            SpeedGlobal::Friction => {
                let g = self
                    .friction
                    .capture_global(SlideCollisionGlobal::Friction)
                    .map_err(SpeedAttributeError::Friction)?;
                SpeedAttributePointer::Friction(
                    self.friction
                        .lookup(&g, key)
                        .map_err(SpeedAttributeError::Friction)?,
                )
            }
        })
    }
    pub(crate) fn value(
        &mut self,
        pointer: &SpeedAttributePointer,
    ) -> Result<f32, SpeedAttributeError> {
        match pointer {
            SpeedAttributePointer::Manual(p) => {
                self.manual.float(p).map_err(SpeedAttributeError::Manual)
            }
            SpeedAttributePointer::Friction(p) => self
                .friction
                .float(p)
                .map_err(SpeedAttributeError::Friction),
            SpeedAttributePointer::Conservation(p) => {
                let Some(p) = p else { return Ok(0.0) };
                let bytes = p.bytes().try_into().map_err(|_| {
                    SpeedAttributeError::Runtime(AttributeRuntimeError::ScalarSize {
                        size: p.bytes().len(),
                    })
                })?;
                Ok(f32::from_be_bytes(bytes))
            }
        }
    }
    pub(crate) fn capture_graph(
        &mut self,
        graph: SpeedGraph,
    ) -> Result<SpeedGraphPointer, SpeedAttributeError> {
        let global = self
            .friction
            .capture_global(SlideCollisionGlobal::Friction)
            .map_err(SpeedAttributeError::Friction)?;
        let data = global
            .instance
            .try_cached_layout(0x160)
            .map_err(SpeedAttributeError::Runtime)?;
        Ok(SpeedGraphPointer {
            data,
            offset: match graph {
                SpeedGraph::IdleFriction => 0x90,
                SpeedGraph::ManualFriction => 0xe0,
            },
        })
    }
}
impl SpeedGraphPointer {
    pub(crate) fn evaluate(&self, speed: f32) -> f32 {
        evaluate(self.data.as_ref(), self.offset, speed)
    }
}
impl SkateboardSelectedSurface {
    pub(crate) fn try_speed_model_friction(&self, speed: f32) -> Result<f32, SpeedAttributeError> {
        let data = self
            .instance
            .try_cached_layout(0x10c)
            .map_err(SpeedAttributeError::Runtime)?;
        Ok(evaluate(data.as_ref(), 0x60, speed))
    }
}
fn evaluate(data: Option<&AttributeRuntimeData>, offset: usize, input: f32) -> f32 {
    let read = |index| {
        data.map_or(0.0, |data| {
            f32::from_be_bytes(
                data.bytes()[offset + index * 4..offset + index * 4 + 4]
                    .try_into()
                    .expect("bounded source PointGraph8"),
            )
        })
    };
    PointGraph::<8> {
        x: std::array::from_fn(&read),
        y: std::array::from_fn(|i| read(i + 8)),
    }
    .evaluate(input)
}
