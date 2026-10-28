//! Owned hat geometry from `SkeletonBody::CreateHatVolume` (0x82b4fae8).
//!
//! Cylinder construction precedes the caller's raw/state/effective mass stores;
//! sphere completion follows them. This module does not implement those stores,
//! body admission, allocation callbacks, or mutable Vault notifications.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "hat geometry awaits physical skeleton construction"
    )
)]

use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    octree::AabbWords,
    serialised_assembly_bbox::{
        HostSerialisedAssemblyBBoxNumeric, SerialisedAssemblyBBoxMinMaxNumeric,
        SerialisedAssemblyPartBBoxNumeric, try_get_primitive_bbox,
    },
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};
use skate2_formats::collision::{Volume, VolumeType};
use std::sync::Arc;

#[cfg(test)]
#[path = "skeleton_hat_tests.rs"]
mod tests;

/// Actual generated `physics_hat` binding (0x82b3e6a8); layout is cached by
/// Instance construction, not looked up as generic scalar attributes.
pub(crate) struct HatAttributes {
    instance: AttributeRuntimeInstance,
}

impl HatAttributes {
    pub(crate) const CLASS: u64 = 0x03df_76a4_7692_efff;

    pub(crate) const fn collection(hat_type: u32) -> u64 {
        if hat_type == 4 {
            0x3e30_b6a3_73a5_8559
        } else {
            0x2d64_3131_0b6e_1662
        }
    }

    /// Selection is the original ctor branch, including None and unknown raw
    /// enum values. Whether to create a hat is the caller's separate branch.
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>, hat_type: u32) -> Self {
        Self {
            instance: vault.instance(Self::CLASS, Self::collection(hat_type)),
        }
    }

    pub(crate) fn try_layout(&self) -> Result<HatLayout, AttributeRuntimeError> {
        let retained = self.instance.try_cached_layout(48)?;
        // Original generated constructor chooses DefaultDataArea(48). The
        // retained Vault is immutable: copying this range does not introduce a
        // live-reload snapshot policy or move observable scalar fetches.
        let mut bytes = [0; 48];
        if let Some(data) = retained {
            bytes.copy_from_slice(data.bytes());
        }
        Ok(HatLayout { bytes })
    }
}

/// All cached bytes, including uninterpreted tail padding and vector W lanes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HatLayout {
    bytes: [u8; 48],
}

impl HatLayout {
    #[cfg(test)]
    pub(crate) const fn from_bytes_for_test(bytes: [u8; 48]) -> Self {
        Self { bytes }
    }
    pub(crate) const fn bytes(&self) -> &[u8; 48] {
        &self.bytes
    }

    fn float(&self, offset: usize) -> f32 {
        f32::from_be_bytes(
            self.bytes[offset..offset + 4]
                .try_into()
                .expect("fixed hat field"),
        )
    }

    fn vector(&self, offset: usize) -> [f32; 4] {
        std::array::from_fn(|lane| self.float(offset + lane * 4))
    }
}

pub(crate) trait HatNumeric: SerialisedAssemblyPartBBoxNumeric {
    fn sin_cos(&mut self, angle: f32) -> Result<(f32, f32), Self::Error>;
}

/// Host trig, dot, estimate and min/max policies, not Xenon numeric parity.
#[derive(Default)]
pub(crate) struct HostHatNumeric;

impl SerialisedAssemblyBBoxMinMaxNumeric for HostHatNumeric {
    type Error = core::convert::Infallible;
    fn minimum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        HostSerialisedAssemblyBBoxNumeric.minimum(left, right)
    }
    fn maximum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        HostSerialisedAssemblyBBoxNumeric.maximum(left, right)
    }
}
impl SerialisedAssemblyPartBBoxNumeric for HostHatNumeric {
    fn sum_three_products(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        HostSerialisedAssemblyBBoxNumeric.sum_three_products(left, right)
    }
    fn reciprocal_square_root_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        HostSerialisedAssemblyBBoxNumeric.reciprocal_square_root_estimate(value)
    }
}
impl HatNumeric for HostHatNumeric {
    fn sin_cos(&mut self, angle: f32) -> Result<(f32, f32), Self::Error> {
        Ok(angle.sin_cos())
    }
}

/// Active debug hat fields only. Other native debug bytes remain unspecified.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HatCylinderDebug {
    pub(crate) transform: [f32; 16],
    pub(crate) radius: f32,
    pub(crate) half_length: f32,
}

/// Staged source cylinder, through the second collision-enable read. Native
/// `HatPresent` becomes true before the first sample; this successful owned
/// stage represents presence without exposing partial native mutation.
#[derive(Debug)]
pub(crate) struct HatCylinder {
    cylinder: Volume,
    initial_collision_enabled: bool,
    debug: HatCylinderDebug,
}

impl HatCylinder {
    #[expect(
        clippy::similar_names,
        reason = "axis-labelled sine/cosine products preserve the recovered Euler graph"
    )]
    pub(crate) fn try_create<N: HatNumeric>(
        layout: &HatLayout,
        enable_collision: &mut impl FnMut() -> bool,
        numeric: &mut N,
    ) -> Result<Self, N::Error> {
        let initial_collision_enabled = enable_collision();
        let radius = layout.float(0x24);
        let half_length = layout.float(0x20) * 0.5;
        // Cylinder's unused third union word is host canonical storage, not a
        // native initializer store. The active radius/half-height are exact.
        let mut cylinder = fresh_volume(
            VolumeType::Cylinder,
            0.0,
            [half_length.to_bits(), radius.to_bits(), 0],
        );
        let angles = layout.vector(0x10);
        let (sx, cx) = numeric.sin_cos(angles[0])?;
        let (sy, cy) = numeric.sin_cos(angles[1])?;
        let (sz, cz) = numeric.sin_cos(angles[2])?;
        let cx_sz = cx * sz;
        let cx_cz = cx * cz;
        let sx_cz = sx * cz;
        let sx_sz = sx * sz;
        let sy_cx_sz = sy * cx_sz;
        let cy_sz = cy * sz;
        let position = layout.vector(0);
        let cy_cz = cy * cz;
        let cy_cx = cy * cx;
        let cy_sx = cy * sx;
        cylinder.transform[12..16].copy_from_slice(&position);
        let sy_sx_cz = sy * sx_cz;
        let y_y = sy.mul_add(sx_sz, cx_cz);
        let z_x = sy.mul_add(cx_cz, sx_sz);
        let z_y = sy_cx_sz - sx_cz;
        let y_x = sy_sx_cz - cx_sz;
        let negative_sy = f32::from_bits(sy.to_bits() ^ 0x8000_0000);
        // Original full-lane stores are translation, X, Z, Y. W repeats X.
        cylinder.transform[0..4].copy_from_slice(&[cy_cz, cy_sz, negative_sy, cy_cz]);
        cylinder.transform[8..12].copy_from_slice(&[z_x, z_y, cy_cx, z_x]);
        cylinder.transform[4..8].copy_from_slice(&[y_x, y_y, cy_sx, y_x]);
        let debug = HatCylinderDebug {
            transform: cylinder.transform,
            radius,
            half_length,
        };
        let current_enabled = enable_collision();
        cylinder.flags = (cylinder.flags & !1) | u32::from(current_enabled);
        Ok(Self {
            cylinder,
            initial_collision_enabled,
            debug,
        })
    }

    pub(crate) const fn cylinder(&self) -> &Volume {
        &self.cylinder
    }
    pub(crate) const fn initial_collision_enabled(&self) -> bool {
        self.initial_collision_enabled
    }
    pub(crate) const fn debug(&self) -> &HatCylinderDebug {
        &self.debug
    }

    /// Call only after the caller's source raw/state/effective mass operations
    /// and sphere-radius calculation (0x82b4fdac..0x82b4feac). Sphere union
    /// storage is inactive and canonical zero here, not a native byte claim.
    pub(crate) fn try_finish<N: SerialisedAssemblyPartBBoxNumeric>(
        self,
        sphere_radius: f32,
        numeric: &mut N,
    ) -> Result<HatAggregate, N::Error> {
        let sphere = fresh_volume(VolumeType::Sphere, sphere_radius, [0; 3]);
        // Update includes both children, regardless of cylinder enabled state.
        // First child overwrites bounds; subsequent child unions all four lanes.
        let first =
            try_get_primitive_bbox(&self.cylinder, None, 0, numeric)?.expect("owned cylinder");
        let second = try_get_primitive_bbox(&sphere, None, 0, numeric)?.expect("owned sphere");
        let minimum = numeric.minimum(first.min_f32(), second.min_f32())?;
        let maximum = numeric.maximum(first.max_f32(), second.max_f32())?;
        Ok(HatAggregate {
            children: [self.cylinder, sphere],
            bounds: AabbWords::from_f32(minimum, maximum),
            outer: HatOuterVolume {
                transform: FRESH_VOLUME_TRANSFORM,
                radius: 0.0,
                group_id: 0,
                surface_id: 0,
                flags: 1,
            },
            initial_collision_enabled: self.initial_collision_enabled,
            debug: self.debug,
        })
    }
}

/// Aggregate outer header; the containing owner supplies its actual inner
/// relation. There is deliberately no serialized ID or fabricated pointer.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HatOuterVolume {
    pub(crate) transform: [f32; 16],
    pub(crate) radius: f32,
    pub(crate) group_id: u32,
    pub(crate) surface_id: u32,
    pub(crate) flags: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct HatAggregate {
    children: [Volume; 2],
    bounds: AabbWords,
    outer: HatOuterVolume,
    initial_collision_enabled: bool,
    debug: HatCylinderDebug,
}

impl HatAggregate {
    pub(crate) const fn children(&self) -> &[Volume; 2] {
        &self.children
    }
    pub(crate) const fn bounds(&self) -> AabbWords {
        self.bounds
    }
    pub(crate) const fn outer(&self) -> &HatOuterVolume {
        &self.outer
    }
    pub(crate) const fn outer_metadata(&self) -> &HatOuterVolume {
        &self.outer
    }
    pub(crate) const fn frame(&self) -> &[f32; 16] {
        &self.outer.transform
    }
    pub(crate) const fn initial_collision_enabled(&self) -> bool {
        self.initial_collision_enabled
    }
    pub(crate) const fn debug(&self) -> &HatCylinderDebug {
        &self.debug
    }
}

fn fresh_volume(kind: VolumeType, radius: f32, shape_words: [u32; 3]) -> Volume {
    Volume {
        transform: FRESH_VOLUME_TRANSFORM,
        vtable_offset: kind as u32,
        shape_words,
        radius,
        group_id: 0,
        surface_id: 0,
        flags: 1,
    }
}
