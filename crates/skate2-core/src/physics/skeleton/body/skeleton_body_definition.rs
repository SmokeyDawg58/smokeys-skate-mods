//! Owned geometry and both mass caches from `SkeletonBody::Init` (0x82b61738).
//!
//! Stops immediately before InitJoints/assembly creation. These are actual
//! retained Volumes, not prepared arena identities or an admitted `SkeletonBody`.
//! `SkeletonData` and `InitAttribData` are immutable owners; native in-place writes
//! and partial failure publication are not reproduced by this host constructor.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "awaiting normal joints and live body admission")
)]

use crate::{
    ragdoll::{SKELETON_PART_COUNT, SkeletonPartIndex},
    simulation_volume_integral::{
        SimulationVolumeIntegralError, SimulationVolumeIntegralNumeric,
        SimulationVolumePrincipalMass, fat_box_unit_mass, try_compute_mapped_array_integral,
        try_diagonalise_volume_integral,
    },
    skateboard_wheels::{FRESH_VOLUME_TRANSFORM, capsule_unit_mass, sphere_unit_mass},
    skeleton_attributes::{SkeletonBodyAttributeData, SkeletonBoneAttributes},
    skeleton_body_attributes::{
        self as keys, SkeletonBodyAttributeRead, SkeletonBodyAttributeRoute as Route,
        SkeletonBodyDebugRead,
    },
    skeleton_body_mass::{BodyMassState, SkeletonBodyMassNumeric},
    skeleton_data::SkeletonData,
    skeleton_hat::{HatAggregate, HatCylinder, HatLayout, HatNumeric},
};
use skate2_formats::collision::{AssemblyPartMaterial, Volume, VolumeType};
use std::sync::Arc;

#[cfg(test)]
#[path = "skeleton_body_definition_tests.rs"]
pub(crate) mod tests;

#[derive(Debug)]
pub(crate) enum SkeletonPartGeometry {
    Primitive(Arc<Volume>),
    Hat(Arc<HatAggregate>),
}

impl SkeletonPartGeometry {
    fn try_principal<N: SimulationVolumeIntegralNumeric>(
        &self,
        numeric: &mut N,
    ) -> Result<SimulationVolumePrincipalMass, SimulationVolumeIntegralError<N::Error>> {
        match self {
            Self::Primitive(volume) => {
                let (diagonal, amount) = match volume.volume_type() {
                    Some(VolumeType::Sphere) => sphere_unit_mass(volume.radius),
                    Some(VolumeType::Capsule) => {
                        capsule_unit_mass(volume.radius, f32::from_bits(volume.shape_words[0]))
                    }
                    Some(VolumeType::Box) => {
                        fat_box_unit_mass(volume.shape_words.map(f32::from_bits), volume.radius)
                    }
                    _ => unreachable!("body constructor owns only its three source primitives"),
                };
                Ok(SimulationVolumePrincipalMass {
                    diagonal,
                    volume: amount,
                    body_to_part: volume.transform,
                })
            }
            Self::Hat(hat) => {
                let mut mapped =
                    try_compute_mapped_array_integral(hat.children(), Some(hat.frame()), numeric)?;
                try_diagonalise_volume_integral(&mut mapped.integral, numeric)
                    .map_err(SimulationVolumeIntegralError::Numeric)
            }
        }
    }
}

/// Only stores written by this shape branch, not fabricated constructor/debug
/// bytes. Box retains all four computed size lanes, though collision uses XYZ.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SkeletonPartDebugShape {
    Box { half_size: [f32; 4] },
    Sphere { radius: f32 },
    Capsule { radius: f32, half_length: f32 },
    HatSphere { radius: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkeletonPartMassCache {
    pub(crate) inverse_mass: f32,
    pub(crate) inverse_tensor: [f32; 4],
}

#[derive(Debug)]
pub(crate) struct SkeletonBodyPartDefinition {
    pub(crate) geometry: SkeletonPartGeometry,
    pub(crate) mass: BodyMassState,
    pub(crate) material: AssemblyPartMaterial,
    pub(crate) ragdoll: SkeletonPartMassCache,
    pub(crate) animated: SkeletonPartMassCache,
    pub(crate) unscaled_part_mass: f32,
    pub(crate) effective_mass: f32,
    pub(crate) debug: SkeletonPartDebugShape,
}

#[derive(Debug)]
pub(crate) struct SkeletonBodyDefinition {
    pub(crate) data: Arc<SkeletonData>,
    pub(crate) attributes: Arc<SkeletonBodyAttributeData>,
    pub(crate) parts: [SkeletonBodyPartDefinition; SKELETON_PART_COUNT],
    /// Initial allocation choice; later `InitJoints` performs additional reads.
    pub(crate) allocated_joint_count: usize,
    pub(crate) skateboard_part: SkeletonPartIndex,
    pub(crate) hat_type: u32,
}

#[derive(Debug)]
pub(crate) enum SkeletonBodyDefinitionError<A, I, H> {
    Allocation,
    Attribute(A),
    Integral(SimulationVolumeIntegralError<I>),
    Hat(H),
    MissingHatLayout,
    InvalidRagdollNumerator {
        bits: u32,
    },
    InvalidPartMass {
        part: SkeletonPartIndex,
        bits: u32,
    },
    UnsupportedBoneVolume {
        part: SkeletonPartIndex,
        volume_type: u32,
    },
}

impl SkeletonBodyDefinition {
    /// Original source field accesses are retained in order. Host allocation
    /// refusal occurs before callbacks; source unchecked allocator/debug/assert
    /// failures instead stop with typed errors and no partially published owner.
    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        clippy::type_complexity,
        reason = "one source constructor with explicit independent attribute and platform seams"
    )]
    pub(crate) fn try_create<R, I, H>(
        data: Arc<SkeletonData>,
        attributes: Arc<SkeletonBodyAttributeData>,
        reader: &mut R,
        hat_type: u32,
        hat_layout: Option<&HatLayout>,
        hat_enabled: &mut impl FnMut() -> bool,
        mass_numeric: &mut impl SkeletonBodyMassNumeric,
        integral_numeric: &mut I,
        hat_numeric: &mut H,
    ) -> Result<Self, SkeletonBodyDefinitionError<R::Error, I::Error, H::Error>>
    where
        R: SkeletonBodyAttributeRead,
        I: SimulationVolumeIntegralNumeric,
        H: HatNumeric,
    {
        use SkeletonBodyDefinitionError as Error;
        let mut parts = Vec::new();
        parts
            .try_reserve_exact(SKELETON_PART_COUNT)
            .map_err(|_| Error::Allocation)?;
        let capsule_scale = reader
            .float(Route::LocalBones, keys::CAPSULE_RADIUS_SCALE)
            .map_err(Error::Attribute)?;
        let allocated_joint_count = if reader
            .byte(Route::GlobalAnimation, keys::ENABLE_JOINTS)
            .map_err(Error::Attribute)?
            != 0
        {
            22
        } else {
            0
        };
        let ragdoll_numerator = reader
            .float(Route::GlobalWipeout, keys::RAGDOLL_INVERSE_MASS_NUMERATOR)
            .map_err(Error::Attribute)?;
        if ragdoll_numerator.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(Error::InvalidRagdollNumerator {
                bits: ragdoll_numerator.to_bits(),
            });
        }
        let root_bone = data.bone_ids()[0];
        let mut skateboard_part = SkeletonPartIndex::SkateboardRoot;
        for part in SkeletonPartIndex::ALL {
            let index = part.index();
            let bone = &attributes.bones[index];
            let size = data.parameters().bones[index].size;
            let (geometry, debug, unscaled_part_mass, effective_mass) =
                if part == SkeletonPartIndex::Neck1 && hat_type != 0 {
                    let layout = hat_layout.ok_or(Error::MissingHatLayout)?;
                    let cylinder = HatCylinder::try_create(layout, hat_enabled, hat_numeric)
                        .map_err(Error::Hat)?;
                    let raw = (size[0] * size[1]) * size[2];
                    let scale = reader
                        .float(Route::LocalSkeleton, keys::PART_MASS_SCALE)
                        .map_err(Error::Attribute)?;
                    let effective = scale * raw;
                    let radius = sphere_radius(size, bone.volume_scalar);
                    let aggregate = cylinder
                        .try_finish(radius, hat_numeric)
                        .map_err(Error::Hat)?;
                    (
                        SkeletonPartGeometry::Hat(Arc::new(aggregate)),
                        SkeletonPartDebugShape::HatSphere { radius },
                        raw,
                        effective,
                    )
                } else if data.bone_ids()[index] == root_bone {
                    skateboard_part = part;
                    let half_length = reader
                        .float(Route::LocalSkeleton, keys::ROOT_HALF_LENGTH)
                        .map_err(Error::Attribute)?;
                    let radius = reader
                        .float(Route::LocalSkeleton, keys::ROOT_RADIUS)
                        .map_err(Error::Attribute)?;
                    let mut volume =
                        fresh_volume(VolumeType::Capsule, radius, [half_length.to_bits(), 0, 0]);
                    volume.flags &= !1;
                    (
                        SkeletonPartGeometry::Primitive(Arc::new(volume)),
                        SkeletonPartDebugShape::Capsule {
                            radius,
                            half_length,
                        },
                        0.0,
                        f32::from_bits(0x3a83_126f),
                    )
                } else {
                    let (volume, debug) = ordinary_volume(size, bone, capsule_scale, reader)
                        .map_err(|error| match error {
                            OrdinaryVolumeError::Attribute(error) => Error::Attribute(error),
                            OrdinaryVolumeError::Unsupported(volume_type) => {
                                Error::UnsupportedBoneVolume { part, volume_type }
                            }
                        })?;
                    let raw = (size[0] * size[1]) * size[2];
                    let scale = reader
                        .float(Route::LocalSkeleton, keys::PART_MASS_SCALE)
                        .map_err(Error::Attribute)?;
                    (
                        SkeletonPartGeometry::Primitive(Arc::new(volume)),
                        debug,
                        raw,
                        scale * raw,
                    )
                };
            // 0x82b61e44: NORMAL DYNAMIC friction is written to BOTH fields.
            let material = AssemblyPartMaterial {
                static_friction: attributes.normal.dynamic_friction,
                dynamic_friction: attributes.normal.dynamic_friction,
                restitution: attributes.normal.restitution,
            };
            if effective_mass.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
                return Err(Error::InvalidPartMass {
                    part,
                    bits: effective_mass.to_bits(),
                });
            }
            let mut mass = BodyMassState::native_default();
            let ragdoll_mass = bone.ragdoll_mass_factor * effective_mass;
            let ragdoll_inverse_mass = ragdoll_numerator / ragdoll_mass;
            let principal = geometry
                .try_principal(integral_numeric)
                .map_err(Error::Integral)?;
            mass.apply_principal(
                principal.diagonal,
                principal.volume,
                principal.body_to_part,
                ragdoll_mass,
                mass_numeric,
            );
            let ragdoll = SkeletonPartMassCache {
                inverse_mass: ragdoll_inverse_mass,
                inverse_tensor: mass.packed_inverse_tensor(),
            };
            let animated_mass = bone.mass_factor * effective_mass;
            let animated_inverse_mass = 1.0 / animated_mass;
            // This is a SECOND source ComputeMassProperties, including the
            // aggregate integral/diagonalization and its numeric callbacks.
            let principal = geometry
                .try_principal(integral_numeric)
                .map_err(Error::Integral)?;
            mass.apply_principal(
                principal.diagonal,
                principal.volume,
                principal.body_to_part,
                animated_mass,
                mass_numeric,
            );
            if matches!(geometry, SkeletonPartGeometry::Hat(_)) {
                mass.inverse_body_transform_storage = Some(FRESH_VOLUME_TRANSFORM);
                mass.has_inverse_body_transform = true;
            }
            let mut debug_instance = reader.debug_instance().map_err(Error::Attribute)?;
            match debug_instance.mode().map_err(Error::Attribute)? {
                1 => mass.debug_multiply(
                    debug_instance.factor().map_err(Error::Attribute)?,
                    mass_numeric,
                ),
                2 => mass
                    .debug_multiply_isotropic(debug_instance.factor().map_err(Error::Attribute)?),
                _ => {}
            }
            let animated = SkeletonPartMassCache {
                inverse_mass: animated_inverse_mass,
                inverse_tensor: mass.packed_inverse_tensor(),
            };
            drop(debug_instance); // Source cache store precedes Instance destructor.
            parts.push(SkeletonBodyPartDefinition {
                geometry,
                mass,
                material,
                ragdoll,
                animated,
                unscaled_part_mass,
                effective_mass,
                debug,
            });
        }
        Ok(Self {
            data,
            attributes,
            parts: parts.try_into().expect("all24Parts constructed"),
            allocated_joint_count,
            skateboard_part,
            hat_type,
        })
    }
}

pub(crate) fn fresh_volume(shape: VolumeType, radius: f32, shape_words: [u32; 3]) -> Volume {
    Volume {
        transform: FRESH_VOLUME_TRANSFORM,
        vtable_offset: shape as u32,
        shape_words,
        radius,
        group_id: 0,
        surface_id: 0,
        flags: 1,
    }
}

fn sphere_radius(size: [f32; 4], volume_scalar: f32) -> f32 {
    let half = size.map(|lane| lane * 0.5);
    let selected = if half[0] - half[1] >= 0.0 {
        half[0]
    } else {
        half[1]
    };
    selected * volume_scalar
}

enum OrdinaryVolumeError<E> {
    Attribute(E),
    Unsupported(u32),
}

fn ordinary_volume<R: SkeletonBodyAttributeRead>(
    size: [f32; 4],
    bone: &SkeletonBoneAttributes,
    capsule_scale: f32,
    reader: &mut R,
) -> Result<(Volume, SkeletonPartDebugShape), OrdinaryVolumeError<R::Error>> {
    let early_scalar = bone.volume_scalar;
    match bone.volume_type {
        0 => {
            let half_size = size.map(|lane| lane * (early_scalar * 0.5));
            Ok((
                fresh_volume(
                    VolumeType::Box,
                    0.0,
                    [
                        half_size[0].to_bits(),
                        half_size[1].to_bits(),
                        half_size[2].to_bits(),
                    ],
                ),
                SkeletonPartDebugShape::Box { half_size },
            ))
        }
        1 => {
            let radius = sphere_radius(size, early_scalar);
            Ok((
                fresh_volume(VolumeType::Sphere, radius, [0; 3]),
                SkeletonPartDebugShape::Sphere { radius },
            ))
        }
        2 => {
            let base_radius = (size[0] * capsule_scale) * 0.5;
            let radius = base_radius * early_scalar;
            let remainder = 0.5 * size[2] - base_radius;
            let clamped = if -remainder >= 0.0 { 0.0 } else { remainder };
            let global = reader
                .float(Route::GlobalSkeleton, keys::CAPSULE_LENGTH_SCALE)
                .map_err(OrdinaryVolumeError::Attribute)?;
            let scaled_length = global * clamped;
            let late_scalar = bone.volume_scalar;
            let half_length = scaled_length * late_scalar;
            Ok((
                fresh_volume(VolumeType::Capsule, radius, [half_length.to_bits(), 0, 0]),
                SkeletonPartDebugShape::Capsule {
                    radius,
                    half_length,
                },
            ))
        }
        unsupported => Err(OrdinaryVolumeError::Unsupported(unsupported)),
    }
}
