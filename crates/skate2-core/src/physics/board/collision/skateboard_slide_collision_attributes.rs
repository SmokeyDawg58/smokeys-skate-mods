//! Actual immutable global and selected surface Vault owners for 0x82b28260/27530.
//! Singleton capture tokens preserve call staging; singleton replacement/editor
//! reload are outside this retained-instance scope. No authored runtime defaults.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "grounded board integration pending")
)]
use crate::{
    attribute_runtime::{
        AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance,
        AttributeRuntimeVault,
    },
    physics::PointGraph,
};
use std::{fmt, sync::Arc};

pub(crate) const COLLISION_LIMIT: u64 = 0x8b17_0055_6068_91de;
pub(crate) const COLLISION_LOCAL_Y: u64 = 0x07c6_7f76_18d4_6d82;
pub(crate) const COLLISION_RESPONSE: u64 = 0x7c59_1f34_661b_d15d;
pub(crate) const COLLISION_NORMAL_SPEED: u64 = 0xfd2d_593d_dff8_da8f;
pub(crate) const WHEEL_HARDNESS_BLEND: u64 = 0xb728_14ec_6e1a_8f2e;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlideCollisionGlobal {
    Friction,
    Heading,
    Wheels,
    Collision,
}
impl SlideCollisionGlobal {
    pub(crate) const fn class(self) -> u64 {
        match self {
            Self::Friction => 0x2574_e307_ae61_a71e,
            Self::Heading => 0x3a2a_8b50_4536_22bb,
            Self::Wheels => 0x46ad_d2ee_a90c_c062,
            Self::Collision => 0x7379_c446_7aa7_5df9,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlideCollisionCurve {
    SlideVsNormal,
    SlideVsSpeed,
    SideFrictionFactor,
    CollisionTorqueVsAngle,
}
impl SlideCollisionCurve {
    fn layout(self) -> (SlideCollisionGlobal, usize, usize, usize) {
        match self {
            Self::SlideVsNormal => (SlideCollisionGlobal::Friction, 0x160, 0, 16),
            Self::SlideVsSpeed => (SlideCollisionGlobal::Friction, 0x160, 0x120, 8),
            Self::SideFrictionFactor => (SlideCollisionGlobal::Heading, 0x250, 0x80, 16),
            Self::CollisionTorqueVsAngle => (SlideCollisionGlobal::Collision, 0x70, 0x30, 8),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlideSurfaceField {
    StraightenOutForceTime,
    SideFrictionScalar,
    BrakingScalar,
    WheelDynamicFriction,
    WheelStaticFriction,
    SlideOrientationStrength,
    SlideAngularDamping,
}
pub(crate) trait SkateboardSlideCollisionAttributeRead {
    type Global;
    type Pointer;
    type Surface;
    type Layout;
    type Error;
    fn capture_global(&mut self, kind: SlideCollisionGlobal) -> Result<Self::Global, Self::Error>;
    fn graph(
        &mut self,
        global: &Self::Global,
        curve: SlideCollisionCurve,
        input: f32,
    ) -> Result<f32, Self::Error>;
    fn lookup(&mut self, global: &Self::Global, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
    fn capture_surface_layout(
        &mut self,
        surface: &Self::Surface,
    ) -> Result<Self::Layout, Self::Error>;
    fn layout_float(
        &mut self,
        layout: &Self::Layout,
        field: SlideSurfaceField,
    ) -> Result<f32, Self::Error>;
    fn surface_float(
        &mut self,
        surface: &Self::Surface,
        field: SlideSurfaceField,
    ) -> Result<f32, Self::Error>;
}
#[derive(Clone, Debug)]
pub(crate) struct SkateboardSlideCollisionGlobal {
    identity: Arc<()>,
    kind: SlideCollisionGlobal,
    instance: AttributeRuntimeInstance,
}
#[derive(Clone, Debug)]
pub(crate) struct SkateboardSelectedSurface {
    identity: Arc<()>,
    instance: AttributeRuntimeInstance,
}
/// Retains the exact immutable cached-layout owner without eager data access.
#[derive(Clone, Debug)]
pub(crate) struct SkateboardSelectedSurfaceLayout {
    identity: Arc<()>,
    instance: AttributeRuntimeInstance,
}
impl SkateboardSelectedSurface {
    /// CalcPowerSlideForce82b28108: eight-point graph at layout0, x10/y30.
    pub(crate) fn try_power_slide_graph(&self, speed: f32) -> Result<f32, SkateboardSlideCollisionAttributeError> {
        let data = self.instance.try_cached_layout(0x10c).map_err(SkateboardSlideCollisionAttributeError::Runtime)?;
        let sample = |offset: usize| data.as_ref().map_or(0.0, |d| f32::from_be_bytes(d.bytes()[offset..offset+4].try_into().unwrap()));
        Ok(PointGraph::<8> { x: std::array::from_fn(|i| sample(0x10+i*4)), y: std::array::from_fn(|i| sample(0x30+i*4)) }.evaluate(speed))
    }

    /// Read through this already-authorized selected owner. No provider borrow
    /// or rebinding occurs; callers cannot manufacture a different instance.
    pub(crate) fn try_float(
        &self,
        field: SlideSurfaceField,
    ) -> Result<f32, SkateboardSlideCollisionAttributeError> {
        read_surface_float(&self.instance, field)
    }
}
fn read_surface_float(
    instance: &AttributeRuntimeInstance,
    field: SlideSurfaceField,
) -> Result<f32, SkateboardSlideCollisionAttributeError> {
    let Some(data) = instance
        .try_cached_layout(0x10c)
        .map_err(SkateboardSlideCollisionAttributeError::Runtime)?
    else {
        return Ok(0.0);
    };
    let offset = match field {
        SlideSurfaceField::StraightenOutForceTime => 0xec,
        SlideSurfaceField::SideFrictionScalar => 0xf0,
        SlideSurfaceField::BrakingScalar => 0x108,
        SlideSurfaceField::WheelDynamicFriction => 0xe4,
        SlideSurfaceField::WheelStaticFriction => 0xe0,
        SlideSurfaceField::SlideOrientationStrength => 0xf4,
        SlideSurfaceField::SlideAngularDamping => 0xf8,
    };
    Ok(f32::from_be_bytes(
        data.bytes()[offset..offset + 4]
            .try_into()
            .expect("bounded source surface scalar"),
    ))
}
pub(crate) struct SkateboardSlideCollisionPointer {
    identity: Arc<()>,
    data: Option<AttributeRuntimeData>,
}
#[derive(Debug)]
pub(crate) enum SkateboardSlideCollisionAttributeError {
    ForeignOwner,
    WrongGlobal,
    Runtime(AttributeRuntimeError),
}
impl fmt::Display for SkateboardSlideCollisionAttributeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "slide/collision attributes: {self:?}")
    }
}
impl std::error::Error for SkateboardSlideCollisionAttributeError {}
pub(crate) struct SkateboardSlideCollisionVaultAttributes {
    vault: Arc<AttributeRuntimeVault>,
    identity: Arc<()>,
    globals: [AttributeRuntimeInstance; 4],
}
impl SkateboardSlideCollisionVaultAttributes {
    pub(crate) const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
    pub(crate) const SURFACE_CLASS: u64 = 0xf59f_e715_b7ae_3426;
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            vault: Arc::clone(vault),
            identity: Arc::new(()),
            globals: [
                SlideCollisionGlobal::Friction,
                SlideCollisionGlobal::Heading,
                SlideCollisionGlobal::Wheels,
                SlideCollisionGlobal::Collision,
            ]
            .map(|kind| vault.instance(kind.class(), Self::DEFAULT_COLLECTION)),
        }
    }
    /// Only the actual retained processed surface-selection producer may supply
    /// this key. This API supplies no guessed/default contact surface selection.
    pub(crate) fn bind_selected_surface(&self, collection_key: u64) -> SkateboardSelectedSurface {
        SkateboardSelectedSurface {
            identity: Arc::clone(&self.identity),
            instance: self.vault.instance(Self::SURFACE_CLASS, collection_key),
        }
    }
    fn check(&self, identity: &Arc<()>) -> Result<(), SkateboardSlideCollisionAttributeError> {
        if Arc::ptr_eq(identity, &self.identity) {
            Ok(())
        } else {
            Err(SkateboardSlideCollisionAttributeError::ForeignOwner)
        }
    }
}
impl SkateboardSlideCollisionAttributeRead for SkateboardSlideCollisionVaultAttributes {
    type Global = SkateboardSlideCollisionGlobal;
    type Pointer = SkateboardSlideCollisionPointer;
    type Surface = SkateboardSelectedSurface;
    type Layout = SkateboardSelectedSurfaceLayout;
    type Error = SkateboardSlideCollisionAttributeError;
    fn capture_global(&mut self, kind: SlideCollisionGlobal) -> Result<Self::Global, Self::Error> {
        let index = match kind {
            SlideCollisionGlobal::Friction => 0,
            SlideCollisionGlobal::Heading => 1,
            SlideCollisionGlobal::Wheels => 2,
            SlideCollisionGlobal::Collision => 3,
        };
        Ok(SkateboardSlideCollisionGlobal {
            identity: Arc::clone(&self.identity),
            kind,
            instance: self.globals[index].clone(),
        })
    }
    fn graph(
        &mut self,
        global: &Self::Global,
        curve: SlideCollisionCurve,
        input: f32,
    ) -> Result<f32, Self::Error> {
        self.check(&global.identity)?;
        let (kind, size, offset, count) = curve.layout();
        if global.kind != kind {
            return Err(Self::Error::WrongGlobal);
        }
        let data = global
            .instance
            .try_cached_layout(size)
            .map_err(Self::Error::Runtime)?;
        // Generated class constructor selects zero DefaultDataArea only when absent.
        let read = |index: usize| {
            data.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(
                    data.bytes()[offset + index * 4..offset + index * 4 + 4]
                        .try_into()
                        .expect("bounded source graph"),
                )
            })
        };
        Ok(if count == 16 {
            PointGraph::<16> {
                x: std::array::from_fn(&read),
                y: std::array::from_fn(|i| read(i + 16)),
            }
            .evaluate(input)
        } else {
            PointGraph::<8> {
                x: std::array::from_fn(&read),
                y: std::array::from_fn(|i| read(i + 8)),
            }
            .evaluate(input)
        })
    }
    fn lookup(&mut self, global: &Self::Global, key: u64) -> Result<Self::Pointer, Self::Error> {
        self.check(&global.identity)?;
        Ok(SkateboardSlideCollisionPointer {
            identity: Arc::clone(&self.identity),
            data: global
                .instance
                .try_get_data(key, 0)
                .map_err(Self::Error::Runtime)?,
        })
    }
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        self.check(&pointer.identity)?;
        let Some(data) = &pointer.data else {
            return Ok(0.0);
        };
        let bytes: [u8; 4] = data.bytes().try_into().map_err(|_| {
            Self::Error::Runtime(AttributeRuntimeError::ScalarSize {
                size: data.bytes().len(),
            })
        })?;
        Ok(f32::from_be_bytes(bytes))
    }
    fn capture_surface_layout(
        &mut self,
        surface: &Self::Surface,
    ) -> Result<Self::Layout, Self::Error> {
        self.check(&surface.identity)?;
        Ok(SkateboardSelectedSurfaceLayout {
            identity: Arc::clone(&surface.identity),
            instance: surface.instance.clone(),
        })
    }
    fn layout_float(
        &mut self,
        layout: &Self::Layout,
        field: SlideSurfaceField,
    ) -> Result<f32, Self::Error> {
        self.check(&layout.identity)?;
        read_surface_float(&layout.instance, field)
    }
    fn surface_float(
        &mut self,
        surface: &Self::Surface,
        field: SlideSurfaceField,
    ) -> Result<f32, Self::Error> {
        self.check(&surface.identity)?;
        surface.try_float(field)
    }
}
#[cfg(test)]
#[path = "skateboard_slide_collision_attributes_tests.rs"]
pub(crate) mod tests;

#[path = "../control/skateboard_speed_model_surface.rs"]
pub(crate) mod speed_model_surface;

#[path = "../control/skateboard_speed_model_attributes.rs"]
pub(crate) mod speed_model;

#[path = "../control/skateboard_straighten_surface.rs"]
pub(crate) mod straighten_surface;
