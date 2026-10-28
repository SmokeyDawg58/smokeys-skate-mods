//! Source-authored truck definitions, `CreateTrucks` (0x82b33da8..0x82b344f0).
//! Parts4/5 share one fresh capsule. This is a board-definition component,
//! not simulation admission, a placed truck, or a complete seven-part board.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "authored truck definitions await whole-board admission"
    )
)]

use crate::{
    simulation_rigid_body_pool::SimulationInertia,
    skateboard_attributes::{SkateboardAttributeClass as Class, SkateboardAttributeRead},
    skateboard_wheels::{
        FRESH_VOLUME_TRANSFORM, SkateboardWheelMassNumeric, apply_fresh_mass_properties,
    },
};
use skate2_formats::collision::{AssemblyPartMaterial, Volume, VolumeType};
use std::{fmt, sync::Arc};

#[cfg(test)]
#[path = "skateboard_trucks_tests.rs"]
mod tests;

const RADIUS_MULTIPLIER: u64 = 0xb803_dfbe_2e4b_56fb;
const WIDTH_MULTIPLIER: u64 = 0x03ed_f4ad_8e17_41f6;
const WHEEL_RADIUS: u64 = 0x1264_6f5e_842c_4db3;
const WHEEL_HALF_WIDTH: u64 = 0x5a84_c435_dc34_0af8;
const COLLISION_ENABLED: u64 = 0xddca_edd8_fa8f_5529;
const DYNAMIC_FRICTION: u64 = 0x7e13_5b94_4834_707e;
const STATIC_FRICTION: u64 = 0xb34a_6487_0f3f_5944;
const RESTITUTION: u64 = 0x3298_05a8_2950_ab54;
const MASS: u64 = 0x8e3a_799a_0726_0fd8;
const WORLD_MASS_MULTIPLIER: u64 = 0x4650_7e40_c853_72fc;

/// Constructor-initialized debug fields plus the truck-specific stores.
/// Native triangle-point fields are untouched and are not invented as zero.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SkateboardTruckDebugCapsule {
    pub(crate) index: u32,
    pub(crate) offset: [f32; 16],
    pub(crate) size: [f32; 4],
    pub(crate) radius: f32,
    pub(crate) half_length: f32,
    pub(crate) color: [f32; 4],
    pub(crate) triangle_points: Option<[[f32; 4]; 3]>,
}

impl SkateboardTruckDebugCapsule {
    pub(crate) const NATIVE_TYPE: u32 = 2;
}

#[derive(Debug)]
pub(crate) struct SkateboardTruckPartDefinition {
    ordinal: u8,
    volume: Arc<Volume>,
    inertia: SimulationInertia,
    inverse_body_transform: Option<[f32; 16]>,
    material: AssemblyPartMaterial,
    debug: SkateboardTruckDebugCapsule,
}

impl SkateboardTruckPartDefinition {
    pub(crate) const fn ordinal(&self) -> u8 {
        self.ordinal
    }
    pub(crate) fn volume(&self) -> &Arc<Volume> {
        &self.volume
    }
    pub(crate) const fn inertia(&self) -> &SimulationInertia {
        &self.inertia
    }
    pub(crate) const fn inverse_tensor_w(&self) -> f32 {
        self.inertia.inverse_tensor[0]
    }
    pub(crate) const fn inverse_body_transform(&self) -> Option<&[f32; 16]> {
        self.inverse_body_transform.as_ref()
    }
    pub(crate) const fn material(&self) -> AssemblyPartMaterial {
        self.material
    }
    #[expect(
        clippy::unused_self,
        reason = "uniform part view exposes native nonstatic constructor invariant"
    )]
    pub(crate) const fn is_static_body(&self) -> bool {
        false
    }
    pub(crate) const fn debug(&self) -> &SkateboardTruckDebugCapsule {
        &self.debug
    }
}

#[derive(Debug)]
pub(crate) enum SkateboardTruckDefinitionError<E> {
    Attribute(E),
    DebugCapacity { initial_count: u32 },
}

impl<E: fmt::Display> fmt::Display for SkateboardTruckDefinitionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attribute(error) => write!(f, "truck attribute: {error}"),
            Self::DebugCapacity { initial_count } => write!(
                f,
                "two truck debug objects exceed capacity32 from {initial_count}"
            ),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for SkateboardTruckDefinitionError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Attribute(error) => Some(error),
            Self::DebugCapacity { .. } => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct SkateboardTruckDefinitions {
    parts: [SkateboardTruckPartDefinition; 2],
    debug_end: u32,
}

impl SkateboardTruckDefinitions {
    /// Preserves all fifteen source lookups and delayed pointer loads. The
    /// concrete provider is presently immutable, but this constructor does not
    /// cache one material/mass sample for both parts. Maxima/drag stay native.
    /// Capacity refusal precedes callbacks as explicit host safety policy;
    /// native allocation/debug indexing and failure paths are unchecked.
    pub(crate) fn try_create<R: SkateboardAttributeRead>(
        attributes: &mut R,
        initial_debug_count: u32,
        numeric: &mut impl SkateboardWheelMassNumeric,
    ) -> Result<Self, SkateboardTruckDefinitionError<R::Error>> {
        if initial_debug_count > 30 {
            return Err(SkateboardTruckDefinitionError::DebugCapacity {
                initial_count: initial_debug_count,
            });
        }
        Self::read_and_create(attributes, initial_debug_count, numeric)
            .map_err(SkateboardTruckDefinitionError::Attribute)
    }

    fn read_and_create<R: SkateboardAttributeRead>(
        attributes: &mut R,
        initial: u32,
        numeric: &mut impl SkateboardWheelMassNumeric,
    ) -> Result<Self, R::Error> {
        let radius_multiplier = attributes.read_float(Class::Trucks, RADIUS_MULTIPLIER)?;
        let width_multiplier = attributes.read_float(Class::Trucks, WIDTH_MULTIPLIER)?;
        let wheel_radius = attributes.read_float(Class::Wheels, WHEEL_RADIUS)?;
        let radius = (wheel_radius * radius_multiplier) * 0.5;
        let wheel_half_width = attributes.read_float(Class::Wheels, WHEEL_HALF_WIDTH)?;
        let half_height = (width_multiplier * wheel_half_width) * 0.5;
        let mut volume = Arc::new(Volume {
            transform: FRESH_VOLUME_TRANSFORM,
            vtable_offset: VolumeType::Capsule as u32,
            // Inactive union words use canonical host storage, not native
            // initialized-byte claims.
            shape_words: [half_height.to_bits(), 0, 0],
            radius,
            group_id: 0,
            surface_id: 0,
            flags: 1,
        });
        let enabled_pointer = attributes.lookup(Class::Trucks, COLLISION_ENABLED)?;
        let enabled = attributes.byte(&enabled_pointer)?;
        if enabled == 0 {
            Arc::get_mut(&mut volume)
                .expect("new capsule not yet shared")
                .flags &= !1;
        }
        // Both materials precede both mass calls in the native constructor.
        let materials = [read_material(attributes)?, read_material(attributes)?];
        let masses = [
            read_mass(attributes, radius, half_height, numeric)?,
            read_mass(attributes, radius, half_height, numeric)?,
        ];
        // Debug records are appended only after both mass calls complete.
        let parts = std::array::from_fn(|index| {
            let (inertia, inverse_body_transform) = masses[index];
            SkateboardTruckPartDefinition {
                ordinal: u8::try_from(index + 4).expect("truck ordinals4/5"),
                volume: Arc::clone(&volume),
                inertia,
                inverse_body_transform,
                material: materials[index],
                debug: SkateboardTruckDebugCapsule {
                    index: initial + u32::try_from(index).expect("two indices fit u32"),
                    offset: FRESH_VOLUME_TRANSFORM,
                    size: [0.0; 4],
                    radius,
                    half_length: half_height,
                    color: [1.0; 4],
                    triangle_points: None,
                },
            }
        });
        Ok(Self {
            parts,
            debug_end: initial + 2,
        })
    }

    pub(crate) const fn parts(&self) -> &[SkateboardTruckPartDefinition; 2] {
        &self.parts
    }
    pub(crate) const fn debug_end(&self) -> u32 {
        self.debug_end
    }
}

fn read_material<R: SkateboardAttributeRead>(
    attributes: &mut R,
) -> Result<AssemblyPartMaterial, R::Error> {
    let dynamic_pointer = attributes.lookup(Class::Trucks, DYNAMIC_FRICTION)?;
    let static_pointer = attributes.lookup(Class::Trucks, STATIC_FRICTION)?;
    let dynamic_friction = attributes.float(&dynamic_pointer)?;
    let static_friction = attributes.float(&static_pointer)?;
    // No wheel-style clamp, even if dynamic exceeds static or is NaN.
    let restitution = attributes.read_float(Class::Trucks, RESTITUTION)?;
    Ok(AssemblyPartMaterial {
        static_friction,
        dynamic_friction,
        restitution,
    })
}

fn read_mass<R: SkateboardAttributeRead>(
    attributes: &mut R,
    radius: f32,
    half_height: f32,
    numeric: &mut impl SkateboardWheelMassNumeric,
) -> Result<(SimulationInertia, Option<[f32; 16]>), R::Error> {
    let mass_pointer = attributes.lookup(Class::Trucks, MASS)?;
    let world_pointer = attributes.lookup(Class::World, WORLD_MASS_MULTIPLIER)?;
    let world = attributes.float(&world_pointer)?;
    let mass = attributes.float(&mass_pointer)?;
    // Native fmuls operand order is world first, truck mass second.
    let supplied_mass = world * mass;
    let mut inertia = SimulationInertia::native_default();
    let inverse = apply_fresh_mass_properties(
        VolumeType::Capsule,
        radius,
        half_height,
        supplied_mass,
        &mut inertia,
        numeric,
    );
    Ok((inertia, inverse))
}
