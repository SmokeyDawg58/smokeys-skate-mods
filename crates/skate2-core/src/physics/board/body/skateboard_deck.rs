//! Authored deck geometry from `SkateboardBody::CreateTriangleDeck` (0x82b32e08).
//!
//! This owns actual child volumes and their computed aggregate mass, not an
//! arena identifier or a registered body. Platform trig/estimate arithmetic is
//! explicit; host results are not captured Xenon numeric observations.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "authored deck awaits seven-part board admission")
)]

use crate::{
    octree::AabbWords,
    serialised_assembly_bbox::{
        SerialisedAssemblyBBoxMinMaxNumeric, SerialisedAssemblyPartBBoxNumeric,
        try_get_primitive_bbox,
    },
    simulation_rigid_body_pool::SimulationInertia,
    simulation_volume_integral::{
        HostSimulationVolumeIntegralNumeric, SimulationVolumeIntegral,
        SimulationVolumeIntegralError, SimulationVolumeIntegralNumeric,
        SimulationVolumePrincipalMass, SimulationVolumeReciprocalSite,
        try_compute_mapped_array_integral, try_diagonalise_volume_integral,
    },
    skateboard_attributes::{SkateboardAttributeClass, SkateboardAttributeRead},
    skateboard_wheels::{
        FRESH_VOLUME_TRANSFORM, SkateboardWheelInertiaAxis, SkateboardWheelMassNumeric,
        apply_unit_density_mass_to_inertia,
    },
};
use skate2_formats::collision::{AssemblyPartMaterial, Volume, VolumeType};
use std::{fmt, sync::Arc};

#[cfg(test)]
#[path = "skateboard_deck_tests.rs"]
mod tests;

pub(crate) const WIDTH: u64 = 0xd76d_9db5_ee1f_d008;
pub(crate) const LENGTH: u64 = 0xc94d_6509_1dcb_828b;
pub(crate) const THICKNESS: u64 = 0xceda_721a_4f02_55b6;
pub(crate) const EXTENT: u64 = 0x49f4_29ae_2e66_f063;
pub(crate) const SUBDIVISIONS: u64 = 0x4bcc_b0af_113e_6953;
pub(crate) const BOX_TRIANGLE_ENABLE: u64 = 0x3273_d82d_b269_c8ca;
pub(crate) const CAPSULE_ENABLE: u64 = 0x3322_84c4_e878_a7c5;
pub(crate) const POSITIVE_TILT: u64 = 0x8020_3de9_f9b7_32ca;
pub(crate) const NEGATIVE_TILT: u64 = 0x00e9_49f6_d567_bb10;
pub(crate) const DYNAMIC_FRICTION: u64 = 0xac32_e9a7_f005_6c6d;
pub(crate) const STATIC_FRICTION: u64 = 0xaae1_43f5_64f2_63aa;
pub(crate) const RESTITUTION: u64 = 0x254d_958a_72d5_9a2d;
pub(crate) const ANGULAR_DRAG: u64 = 0x8d30_65ce_d858_a7df;
pub(crate) const MASS: u64 = 0x01d8_0275_8fa1_c38a;
pub(crate) const WORLD_MASS_MULTIPLIER: u64 = 0x4650_7e40_c853_72fc;

/// Calls preserve source chronology, including the unused initial `SinCos`.
/// Dot3/rsqrt/division/sqrt/min/max and mass estimates come from the two
/// shared numeric contracts. None of these methods may silently clamp tuning.
pub(crate) trait SkateboardDeckNumeric:
    SimulationVolumeIntegralNumeric + SkateboardWheelMassNumeric
{
    fn cosine(&mut self, angle: f32) -> Result<f32, Self::Error>;
    fn sine(&mut self, angle: f32) -> Result<f32, Self::Error>;
    fn sine_cosine(&mut self, angle: f32) -> Result<(f32, f32), Self::Error>;
    fn capsule_half_reciprocal_estimate(&mut self, denominator: f32) -> Result<f32, Self::Error>;
}

/// Ordinary host math, not original `XMVector` trig or Xenon estimate bits.
#[derive(Default)]
pub(crate) struct HostSkateboardDeckNumeric;

impl SerialisedAssemblyBBoxMinMaxNumeric for HostSkateboardDeckNumeric {
    type Error = core::convert::Infallible;
    fn minimum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        HostSimulationVolumeIntegralNumeric.minimum(left, right)
    }
    fn maximum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        HostSimulationVolumeIntegralNumeric.maximum(left, right)
    }
}
impl SerialisedAssemblyPartBBoxNumeric for HostSkateboardDeckNumeric {
    fn sum_three_products(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        HostSimulationVolumeIntegralNumeric.sum_three_products(left, right)
    }
    fn reciprocal_square_root_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        HostSimulationVolumeIntegralNumeric.reciprocal_square_root_estimate(value)
    }
}
impl SimulationVolumeIntegralNumeric for HostSkateboardDeckNumeric {
    fn reciprocal_estimate(
        &mut self,
        site: SimulationVolumeReciprocalSite,
        value: f32,
    ) -> Result<f32, Self::Error> {
        SimulationVolumeIntegralNumeric::reciprocal_estimate(
            &mut HostSimulationVolumeIntegralNumeric,
            site,
            value,
        )
    }
    fn scalar_sqrt(&mut self, value: f32) -> Result<f32, Self::Error> {
        HostSimulationVolumeIntegralNumeric.scalar_sqrt(value)
    }
    fn scalar_divide(&mut self, numerator: f32, denominator: f32) -> Result<f32, Self::Error> {
        HostSimulationVolumeIntegralNumeric.scalar_divide(numerator, denominator)
    }
}
impl SkateboardWheelMassNumeric for HostSkateboardDeckNumeric {
    fn reciprocal_estimate(&mut self, _axis: SkateboardWheelInertiaAxis, value: f32) -> f32 {
        value.recip()
    }
}
impl SkateboardDeckNumeric for HostSkateboardDeckNumeric {
    fn cosine(&mut self, angle: f32) -> Result<f32, Self::Error> {
        Ok(angle.cos())
    }
    fn sine(&mut self, angle: f32) -> Result<f32, Self::Error> {
        Ok(angle.sin())
    }
    fn sine_cosine(&mut self, angle: f32) -> Result<(f32, f32), Self::Error> {
        Ok(angle.sin_cos())
    }
    fn capsule_half_reciprocal_estimate(&mut self, denominator: f32) -> Result<f32, Self::Error> {
        Ok(denominator.recip())
    }
}

/// Active debug payload only: untouched native union/point bytes are not
/// fabricated. This fresh-board owner inherits constructor identity offsets.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SkateboardDeckDebugShape {
    Box { dimensions: [f32; 4] },
    Capsule { radius: f32, half_length: f32 },
    Triangle { points: [[f32; 4]; 3], radius: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SkateboardDeckDebugObject {
    pub(crate) index: u32,
    pub(crate) transform: [f32; 16],
    pub(crate) shape: SkateboardDeckDebugShape,
}

/// Actual mapped-array ownership replaces the original aggregate pointer.
/// No synthetic `Volume::aggregate_object_id` is introduced.
#[derive(Debug)]
pub(crate) struct SkateboardDeckAggregate {
    children: Arc<[Volume]>,
    bounds: AabbWords,
    tag_bits: u32,
}

#[expect(
    clippy::unused_self,
    reason = "fresh owned aggregate exposes its invariant fields through the same instance view as variable fields"
)]
impl SkateboardDeckAggregate {
    pub(crate) fn children(&self) -> &[Volume] {
        &self.children
    }
    pub(crate) const fn bounds(&self) -> AabbWords {
        self.bounds
    }
    pub(crate) const fn tag_bits(&self) -> u32 {
        self.tag_bits
    }
    pub(crate) const fn transform(&self) -> &[f32; 16] {
        &FRESH_VOLUME_TRANSFORM
    }
    pub(crate) const fn flags(&self) -> u32 {
        1
    }
    pub(crate) const fn group_id(&self) -> u32 {
        0
    }
    pub(crate) const fn surface_id(&self) -> u32 {
        0
    }
    pub(crate) const fn radius(&self) -> f32 {
        0.0
    }
}

#[derive(Debug)]
pub(crate) struct SkateboardDeckDefinition {
    aggregate: SkateboardDeckAggregate,
    category_counts: [u32; 3],
    category_first_indices: [u32; 3],
    material: AssemblyPartMaterial,
    inertia: SimulationInertia,
    integral: SimulationVolumeIntegral,
    principal_mass: SimulationVolumePrincipalMass,
    debug: Vec<SkateboardDeckDebugObject>,
    debug_start: u32,
    debug_end: u32,
}

#[derive(Debug)]
pub(crate) enum SkateboardDeckError<A, N> {
    Attribute(A),
    Numeric(N),
    Integral(SimulationVolumeIntegralError<N>),
    /// Explicit safe-host boundary; native unchecked negative arithmetic is
    /// not reinterpreted as a zero-subdivision deck.
    NegativeSubdivisions(i32),
    CountOverflow,
    DebugCapacity {
        initial: u32,
        children: u32,
    },
    ChildIndexCapacity(u32),
    UnsupportedChildBounds(usize),
}

impl<A: fmt::Debug, N: fmt::Debug> fmt::Display for SkateboardDeckError<A, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attribute(error) => write!(f, "deck attribute: {error:?}"),
            Self::Numeric(error) => write!(f, "deck numeric: {error:?}"),
            Self::Integral(error) => write!(f, "deck integral: {error:?}"),
            Self::NegativeSubdivisions(n) => {
                write!(f, "negative deck subdivisions {n} exceed safe owner domain")
            }
            Self::CountOverflow => write!(f, "deck child count overflow"),
            Self::DebugCapacity { initial, children } => {
                write!(f, "deck debug range {initial} + {children} exceeds 32")
            }
            Self::ChildIndexCapacity(count) => {
                write!(f, "deck child count {count} exceeds native index domain")
            }
            Self::UnsupportedChildBounds(index) => {
                write!(f, "authored deck child {index} has no primitive bounds")
            }
        }
    }
}

impl SkateboardDeckDefinition {
    pub(crate) fn aggregate(&self) -> &SkateboardDeckAggregate {
        &self.aggregate
    }
    pub(crate) const fn category_counts(&self) -> [u32; 3] {
        self.category_counts
    }
    pub(crate) const fn category_first_indices(&self) -> [u32; 3] {
        self.category_first_indices
    }
    pub(crate) const fn material(&self) -> &AssemblyPartMaterial {
        &self.material
    }
    pub(crate) const fn inertia(&self) -> &SimulationInertia {
        &self.inertia
    }
    pub(crate) const fn inverse_tensor_w(&self) -> f32 {
        self.inertia.inverse_tensor[0]
    }
    /// Source post-mass override at 0x82b33d70. Present even for symmetric COM.
    #[expect(
        clippy::unused_self,
        clippy::unnecessary_wraps,
        reason = "part view retains source present/absent frame semantics; this constructor forces presence and identity"
    )]
    pub(crate) const fn inverse_body_transform(&self) -> Option<&[f32; 16]> {
        Some(&FRESH_VOLUME_TRANSFORM)
    }
    #[expect(
        clippy::unused_self,
        reason = "uniform part view exposes native nonstatic constructor invariant"
    )]
    pub(crate) const fn is_static_body(&self) -> bool {
        false
    }
    pub(crate) const fn integral(&self) -> &SimulationVolumeIntegral {
        &self.integral
    }
    pub(crate) const fn principal_mass(&self) -> &SimulationVolumePrincipalMass {
        &self.principal_mass
    }
    pub(crate) fn debug_objects(&self) -> &[SkateboardDeckDebugObject] {
        &self.debug
    }
    pub(crate) const fn debug_range(&self) -> std::ops::Range<u32> {
        self.debug_start..self.debug_end
    }

    /// Actual runtime lookups and construction order, 0x82b32e08..0x82b33da4.
    /// Safe-host capacity validation occurs after the initial five reads and
    /// before unchecked native storage writes. No partial owner is published.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn try_create<A: SkateboardAttributeRead, N: SkateboardDeckNumeric>(
        attributes: &mut A,
        numeric: &mut N,
        initial_debug_count: u32,
    ) -> Result<Self, SkateboardDeckError<A::Error, N::Error>> {
        use SkateboardAttributeClass::{Deck, World};
        let width = attributes
            .read_float(Deck, WIDTH)
            .map_err(SkateboardDeckError::Attribute)?
            * 0.5;
        let length = attributes
            .read_float(Deck, LENGTH)
            .map_err(SkateboardDeckError::Attribute)?
            * 0.5;
        let thickness = attributes
            .read_float(Deck, THICKNESS)
            .map_err(SkateboardDeckError::Attribute)?
            * 0.5;
        let extent = attributes
            .read_float(Deck, EXTENT)
            .map_err(SkateboardDeckError::Attribute)?;
        let count_pointer = attributes
            .lookup(Deck, SUBDIVISIONS)
            .map_err(SkateboardDeckError::Attribute)?;
        let signed_count = attributes
            .integer(&count_pointer)
            .map_err(SkateboardDeckError::Attribute)?;
        let count = u32::try_from(signed_count)
            .map_err(|_| SkateboardDeckError::NegativeSubdivisions(signed_count))?;
        let total = count
            .checked_add(3)
            .and_then(|first| first.checked_add(count))
            .ok_or(SkateboardDeckError::CountOverflow)?;
        if total > u32::from(u16::MAX) {
            return Err(SkateboardDeckError::ChildIndexCapacity(total));
        }
        let debug_end = initial_debug_count
            .checked_add(total)
            .filter(|end| *end <= 32)
            .ok_or(SkateboardDeckError::DebugCapacity {
                initial: initial_debug_count,
                children: total,
            })?;
        let mut children = Vec::with_capacity(total as usize);
        let mut debug = Vec::with_capacity(total as usize);

        let rounded_radius = thickness * f32::from_bits(0x3f66_6666);
        let mut central_box = fresh_volume(
            VolumeType::Box,
            rounded_radius,
            [
                width - rounded_radius,
                thickness - rounded_radius,
                length - rounded_radius,
            ]
            .map(f32::to_bits),
        );
        set_enabled(
            &mut central_box,
            read_enable(attributes, BOX_TRIANGLE_ENABLE).map_err(SkateboardDeckError::Attribute)?,
        );
        children.push(central_box);
        debug.push(SkateboardDeckDebugObject {
            index: initial_debug_count,
            transform: FRESH_VOLUME_TRANSFORM,
            shape: SkateboardDeckDebugShape::Box {
                dimensions: [width, thickness, length, 0.0],
            },
        });

        let _unused = numeric
            .sine_cosine(f32::from_bits(0x3fc9_0fdb))
            .map_err(SkateboardDeckError::Numeric)?;
        let lateral = width - thickness;
        add_capsule(
            &mut children,
            &mut debug,
            initial_debug_count,
            [[lateral, 0.0, length, 0.0], [lateral, 0.0, -length, 0.0]],
            thickness,
            attributes,
            numeric,
        )?;
        let opposite_lateral = thickness - width;
        add_capsule(
            &mut children,
            &mut debug,
            initial_debug_count,
            [
                [opposite_lateral, 0.0, length, 0.0],
                [opposite_lateral, 0.0, -length, 0.0],
            ],
            thickness,
            attributes,
            numeric,
        )?;

        let mut previous_negative =
            trig_pair(0.0, numeric).map_err(SkateboardDeckError::Numeric)?;
        let mut previous_positive =
            trig_pair(0.0, numeric).map_err(SkateboardDeckError::Numeric)?;
        let positive_angle = attributes
            .read_float(Deck, POSITIVE_TILT)
            .map_err(SkateboardDeckError::Attribute)?
            * f32::from_bits(0x3c8e_fa35);
        let positive_tilt =
            trig_pair(positive_angle, numeric).map_err(SkateboardDeckError::Numeric)?;
        let negative_angle = attributes
            .read_float(Deck, NEGATIVE_TILT)
            .map_err(SkateboardDeckError::Attribute)?
            * f32::from_bits(0x3c8e_fa35);
        let negative_tilt =
            trig_pair(negative_angle, numeric).map_err(SkateboardDeckError::Numeric)?;
        if count > 0 {
            let end_extent = extent - thickness;
            #[expect(
                clippy::cast_precision_loss,
                reason = "source converts signed subdivision count to f32"
            )]
            let count_float = signed_count as f32;
            for index in 1..=count {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "source converts bounded signed loop index to f32"
                )]
                let index_float = index as f32;
                let angle = numeric
                    .scalar_divide(index_float * f32::from_bits(0xc049_0fdb), count_float)
                    .map_err(SkateboardDeckError::Numeric)?;
                let current = trig_pair(angle, numeric).map_err(SkateboardDeckError::Numeric)?;
                add_triangle(
                    &mut children,
                    &mut debug,
                    initial_debug_count,
                    [
                        [0.0, 0.0, -length, 0.0],
                        negative_rim(
                            previous_negative,
                            negative_tilt,
                            lateral,
                            end_extent,
                            length,
                        ),
                        negative_rim(current, negative_tilt, lateral, end_extent, length),
                    ],
                    thickness,
                    attributes,
                )
                .map_err(SkateboardDeckError::Attribute)?;
                previous_negative = current;
                let angle = numeric
                    .scalar_divide(index_float * f32::from_bits(0x4049_0fdb), count_float)
                    .map_err(SkateboardDeckError::Numeric)?;
                let current = trig_pair(angle, numeric).map_err(SkateboardDeckError::Numeric)?;
                add_triangle(
                    &mut children,
                    &mut debug,
                    initial_debug_count,
                    [
                        [0.0, 0.0, length, 0.0],
                        positive_rim(
                            previous_positive,
                            positive_tilt,
                            lateral,
                            end_extent,
                            length,
                        ),
                        positive_rim(current, positive_tilt, lateral, end_extent, length),
                    ],
                    thickness,
                    attributes,
                )
                .map_err(SkateboardDeckError::Attribute)?;
                previous_positive = current;
            }
        }
        debug_assert_eq!(children.len(), total as usize);
        let mut bounds: Option<AabbWords> = None;
        for (index, child) in children.iter().enumerate() {
            let child_bounds = try_get_primitive_bbox(child, None, 0, numeric)
                .map_err(SkateboardDeckError::Numeric)?
                .ok_or(SkateboardDeckError::UnsupportedChildBounds(index))?;
            bounds = Some(if let Some(previous) = bounds {
                let minimum = numeric
                    .minimum(previous.min_f32(), child_bounds.min_f32())
                    .map_err(SkateboardDeckError::Numeric)?;
                let maximum = numeric
                    .maximum(previous.max_f32(), child_bounds.max_f32())
                    .map_err(SkateboardDeckError::Numeric)?;
                AabbWords::from_f32(minimum, maximum)
            } else {
                child_bounds
            });
        }
        let aggregate = SkateboardDeckAggregate {
            children: children.into(),
            bounds: bounds.expect("authored deck always has three initial primitives"),
            tag_bits: u32::BITS - (total + 1).leading_zeros(),
        };

        let dynamic_pointer = attributes
            .lookup(Deck, DYNAMIC_FRICTION)
            .map_err(SkateboardDeckError::Attribute)?;
        let static_pointer = attributes
            .lookup(Deck, STATIC_FRICTION)
            .map_err(SkateboardDeckError::Attribute)?;
        let dynamic_friction = attributes
            .float(&dynamic_pointer)
            .map_err(SkateboardDeckError::Attribute)?;
        let static_friction = attributes
            .float(&static_pointer)
            .map_err(SkateboardDeckError::Attribute)?;
        let mut material = AssemblyPartMaterial {
            static_friction,
            dynamic_friction,
            restitution: 0.0,
        };
        material.restitution = attributes
            .read_float(Deck, RESTITUTION)
            .map_err(SkateboardDeckError::Attribute)?;
        let mut inertia = SimulationInertia::native_default();
        inertia.angular_drag = attributes
            .read_float(Deck, ANGULAR_DRAG)
            .map_err(SkateboardDeckError::Attribute)?
            * f32::from_bits(0x426f_ffff);
        let mass_pointer = attributes
            .lookup(Deck, MASS)
            .map_err(SkateboardDeckError::Attribute)?;
        let world_pointer = attributes
            .lookup(World, WORLD_MASS_MULTIPLIER)
            .map_err(SkateboardDeckError::Attribute)?;
        let world_mass = attributes
            .float(&world_pointer)
            .map_err(SkateboardDeckError::Attribute)?;
        let mass = attributes
            .float(&mass_pointer)
            .map_err(SkateboardDeckError::Attribute)?
            * world_mass;
        let mapped = try_compute_mapped_array_integral(
            aggregate.children(),
            Some(aggregate.transform()),
            numeric,
        )
        .map_err(SkateboardDeckError::Integral)?;
        let integral = mapped.integral;
        let principal_mass = try_diagonalise_volume_integral(&mut integral.clone(), numeric)
            .map_err(SkateboardDeckError::Numeric)?;
        apply_unit_density_mass_to_inertia(
            principal_mass.diagonal,
            principal_mass.volume,
            mass,
            &mut inertia,
            numeric,
        );
        // The caller forcibly replaces the intermediate principal inverse frame.
        // General ComputeMassProperties inversion side effects are not modeled.
        Ok(Self {
            aggregate,
            category_counts: [count, count, 3],
            category_first_indices: [4, 3, 0],
            material,
            inertia,
            integral,
            principal_mass,
            debug,
            debug_start: initial_debug_count,
            debug_end,
        })
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

fn set_enabled(volume: &mut Volume, enabled: bool) {
    volume.flags = (volume.flags & !1) | u32::from(enabled);
}

fn read_enable<A: SkateboardAttributeRead>(attributes: &mut A, key: u64) -> Result<bool, A::Error> {
    let pointer = attributes.lookup(SkateboardAttributeClass::Deck, key)?;
    Ok(attributes.byte(&pointer)? != 0)
}

/// Returns retained (sine, cosine), while calling cosine first.
fn trig_pair<N: SkateboardDeckNumeric>(
    angle: f32,
    numeric: &mut N,
) -> Result<(f32, f32), N::Error> {
    let cosine = numeric.cosine(angle)?;
    let sine = numeric.sine(angle)?;
    Ok((sine, cosine))
}

fn negative_rim(
    (sine, cosine): (f32, f32),
    tilt: (f32, f32),
    lateral: f32,
    extent: f32,
    length: f32,
) -> [f32; 4] {
    let sweep = sine * extent;
    [
        cosine * lateral,
        tilt.0 * -sweep,
        sweep.mul_add(tilt.1, -length),
        0.0,
    ]
}

fn positive_rim(
    (sine, cosine): (f32, f32),
    tilt: (f32, f32),
    lateral: f32,
    extent: f32,
    length: f32,
) -> [f32; 4] {
    let sweep = sine * extent;
    [
        cosine * lateral,
        tilt.0 * sweep,
        sweep.mul_add(tilt.1, length),
        0.0,
    ]
}

/// `AddTriangle`, 0x82b2b8c8. No eager normal calculation.
fn add_triangle<A: SkateboardAttributeRead>(
    children: &mut Vec<Volume>,
    debug: &mut Vec<SkateboardDeckDebugObject>,
    debug_start: u32,
    points: [[f32; 4]; 3],
    radius: f32,
    attributes: &mut A,
) -> Result<(), A::Error> {
    let mut triangle = fresh_volume(VolumeType::Triangle, 0.0, [-1.0_f32; 3].map(f32::to_bits));
    triangle.transform = [0.0; 16];
    triangle.transform[..12].copy_from_slice(points.as_flattened());
    triangle.flags = 0x1e3;
    set_enabled(&mut triangle, read_enable(attributes, BOX_TRIANGLE_ENABLE)?);
    triangle.radius = radius;
    triangle.flags |= 0x200;
    triangle.shape_words[0] = 1.0_f32.to_bits();
    triangle.shape_words[2] = 1.0_f32.to_bits();
    debug.push(SkateboardDeckDebugObject {
        index: debug_start + u32::try_from(children.len()).expect("validated child capacity"),
        transform: FRESH_VOLUME_TRANSFORM,
        shape: SkateboardDeckDebugShape::Triangle { points, radius },
    });
    children.push(triangle);
    Ok(())
}

/// `AddCapsule` (0x82b2ba10) and endpoint initialization (0x82e4b258).
fn add_capsule<A: SkateboardAttributeRead, N: SkateboardDeckNumeric>(
    children: &mut Vec<Volume>,
    debug: &mut Vec<SkateboardDeckDebugObject>,
    debug_start: u32,
    points: [[f32; 4]; 2],
    radius: f32,
    attributes: &mut A,
    numeric: &mut N,
) -> Result<(), SkateboardDeckError<A::Error, N::Error>> {
    let mut capsule =
        initialize_capsule(points, radius, numeric).map_err(SkateboardDeckError::Numeric)?;
    set_enabled(
        &mut capsule,
        read_enable(attributes, CAPSULE_ENABLE).map_err(SkateboardDeckError::Attribute)?,
    );
    let difference = std::array::from_fn(|lane| points[0][lane] - points[1][lane]);
    let length = refined_length(difference, numeric).map_err(SkateboardDeckError::Numeric)?;
    debug.push(SkateboardDeckDebugObject {
        index: debug_start + u32::try_from(children.len()).expect("validated child capacity"),
        transform: capsule.transform,
        shape: SkateboardDeckDebugShape::Capsule {
            radius,
            half_length: length * 0.5,
        },
    });
    children.push(capsule);
    Ok(())
}

fn initialize_capsule<N: SkateboardDeckNumeric>(
    points: [[f32; 4]; 2],
    radius: f32,
    numeric: &mut N,
) -> Result<Volume, N::Error> {
    let delta: [f32; 4] = std::array::from_fn(|lane| points[1][lane] - points[0][lane]);
    let mut reciprocal = numeric.capsule_half_reciprocal_estimate(2.0)?;
    for _ in 0..2 {
        let residual = (-reciprocal).mul_add(2.0, 1.0);
        reciprocal = reciprocal.mul_add(residual, reciprocal);
    }
    let half_delta = delta.map(|value| reciprocal * value);
    let center: [f32; 4] = std::array::from_fn(|lane| points[0][lane] + half_delta[lane]);
    let half_height = refined_length(half_delta, numeric)?;
    // Only the active capsule union word is native-initialized. Other words
    // are a canonical host inactive representation, never native byte claims.
    let mut capsule = fresh_volume(VolumeType::Capsule, radius, [half_height.to_bits(), 0, 0]);
    let inverse_height = numeric.scalar_divide(1.0, half_height)?;
    let [nx, ny, nz] = std::array::from_fn(|lane| inverse_height * half_delta[lane]);
    let horizontal = numeric.scalar_sqrt(ny.mul_add(ny, nx * nx))?;
    if horizontal > f32::from_bits(0x3780_0000) {
        let inverse_horizontal = numeric.scalar_divide(1.0, horizontal)?;
        capsule.transform[..12].copy_from_slice(&[
            -(inverse_horizontal * ny),
            inverse_horizontal * nx,
            0.0,
            0.0,
            -((inverse_horizontal * nz) * nx),
            -((inverse_horizontal * nz) * ny),
            horizontal,
            0.0,
            nx,
            ny,
            nz,
            0.0,
        ]);
    }
    capsule.transform[12..].copy_from_slice(&[center[0], center[1], center[2], 0.0]);
    Ok(capsule)
}

fn refined_length<N: SkateboardDeckNumeric>(
    value: [f32; 4],
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let squared = numeric.sum_three_products(value, value)?;
    let mut estimate = numeric.reciprocal_square_root_estimate(squared)?;
    for _ in 0..2 {
        let square = estimate * estimate;
        let half = estimate * 0.5;
        let residual = (-squared).mul_add(square, 1.0);
        estimate = half.mul_add(residual, estimate);
    }
    Ok(if squared == 0.0 {
        0.0
    } else {
        squared * estimate
    })
}
