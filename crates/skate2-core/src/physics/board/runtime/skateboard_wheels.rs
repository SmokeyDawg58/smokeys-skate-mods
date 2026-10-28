//! Fresh wheel definitions from `SkateboardBody::CreateWheels` (`0x82b344f0`).
//!
//! Four definition-ordered parts share one initialized collision volume. This
//! owner is not a seven-part skateboard, an arena object, or a registered body.
//! Placement, trucks, deck, joints, drives and island admission remain separate.
//! Scalar grouping and branch choices are recovered; host reciprocal estimates
//! are an explicit approximation, not a Xenon floating-point oracle.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "fresh wheel definitions awaiting seven-part runtime body integration"
    )
)]

use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    simulation_rigid_body_pool::SimulationInertia,
};
use skate2_formats::collision::{AssemblyPartMaterial, Volume, VolumeType};
use std::sync::Arc;

#[cfg(test)]
#[path = "skateboard_wheels_corpus.rs"]
mod corpus;

pub(crate) const FRESH_VOLUME_TRANSFORM: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];
const WHEEL_COUNT: usize = 4;
const DEBUG_OBJECT_CAPACITY: u32 = 32;
const MINIMUM_RADIUS: f32 = f32::from_bits(0x33d6_bf95);

/// The seven scalar inputs used by `CreateWheels`, in snapshot storage order.
/// Names of the six wheel fields hash exactly to the recovered keys. The
/// original spelling of the world mass multiplier has not been recovered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkateboardWheelAttribute {
    CapsuleLength,
    Radius,
    DynamicFriction,
    StaticFriction,
    Restitution,
    Mass,
    WorldMassMultiplier,
}

impl SkateboardWheelAttribute {
    pub(crate) const ALL: [Self; 7] = [
        Self::CapsuleLength,
        Self::Radius,
        Self::DynamicFriction,
        Self::StaticFriction,
        Self::Restitution,
        Self::Mass,
        Self::WorldMassMultiplier,
    ];

    pub(crate) const fn class_key(self) -> u64 {
        match self {
            Self::WorldMassMultiplier => 0x485a_76b6_8ca6_5d6f,
            _ => 0x46ad_d2ee_a90c_c062,
        }
    }

    pub(crate) const COLLECTION_KEY: u64 = 0xd7ed_bd36_2d7d_2152; // default
    pub(crate) const ELEMENT_INDEX: u32 = 0;

    pub(crate) const fn key(self) -> u64 {
        match self {
            Self::CapsuleLength => 0x20a5_4376_b6f2_c399,
            Self::Radius => 0x1264_6f5e_842c_4db3,
            Self::DynamicFriction => 0x2a10_82dd_feef_3ea7,
            Self::StaticFriction => 0x7d77_7041_dbfa_9234,
            Self::Restitution => 0x5d09_5e63_6526_0b84,
            Self::Mass => 0xa356_746a_6fe7_dc00,
            Self::WorldMassMultiplier => 0x4650_7e40_c853_72fc,
        }
    }

    const fn ordinal(self) -> usize {
        match self {
            Self::CapsuleLength => 0,
            Self::Radius => 1,
            Self::DynamicFriction => 2,
            Self::StaticFriction => 3,
            Self::Restitution => 4,
            Self::Mass => 5,
            Self::WorldMassMultiplier => 6,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkateboardWheelTuningOrigin {
    ResolvedAttribute,
    DefaultDataArea,
}

/// The retained wheel/world instance slice of `GlobalAttributeCollection`
/// (`0x828b1608`), consumed by `CreateWheels` (`0x82b344f0`).
///
/// The loaded read graph is immutable. This retains original collection/layout
/// bindings and performs fresh node lookup when sampling, but does not implement
/// the global collection's other instances, editor writes or board replacement.
pub(crate) struct SkateboardWheelAttributes {
    world: AttributeRuntimeInstance,
    wheels: AttributeRuntimeInstance,
}

impl SkateboardWheelAttributes {
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            // GlobalAttributeCollection binds world at 0x828b2324 before
            // wheels at 0x828b2380.
            world: vault.instance(
                SkateboardWheelAttribute::WorldMassMultiplier.class_key(),
                SkateboardWheelAttribute::COLLECTION_KEY,
            ),
            wheels: vault.instance(
                SkateboardWheelAttribute::Mass.class_key(),
                SkateboardWheelAttribute::COLLECTION_KEY,
            ),
        }
    }

    /// Immutable construction sample, not native mutable sampling timing.
    /// Only native missing pointers select zero; unsupported data propagates
    /// as an error rather than becoming fabricated tuning.
    pub(crate) fn try_snapshot(
        &self,
    ) -> Result<SkateboardWheelTuningSnapshot, AttributeRuntimeError> {
        let mut words = [None; 7];
        for (slot, attribute) in words.iter_mut().zip(SkateboardWheelAttribute::ALL) {
            let instance = if attribute == SkateboardWheelAttribute::WorldMassMultiplier {
                &self.world
            } else {
                &self.wheels
            };
            *slot = instance
                .try_scalar_word(attribute.key(), SkateboardWheelAttribute::ELEMENT_INDEX)?;
        }
        Ok(SkateboardWheelTuningSnapshot::from_resolved_words(words))
    }
}

/// Immutable resolved values, not a replacement for attribute lookup.
///
/// A loader must resolve the exact class/collection/key/index (including its
/// normal inheritance) before constructing this snapshot. A missing pointer
/// is represented by `None`: native calls `DefaultDataArea(4)` at `0x828b5340`,
/// whose zero-filled first word supplies +0. Each source bit pattern and its
/// supplied/default origin are retained without finite-value sanitization.
///
/// Snapshotting is host ownership policy. Native rereads length/radius and
/// per-part mass/material attributes. This owner only claims their equivalent
/// values while the resolved tuning remains immutable throughout construction.
/// Specifically, native dereferences the retained length pointer after the
/// radius lookup and the mass pointer after the world multiplier lookup;
/// dynamic friction is loaded before static lookup, and debug radius is read
/// per part. No callback-based mutable attribute-service parity is claimed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SkateboardWheelTuningSnapshot {
    words: [u32; 7],
    origins: [SkateboardWheelTuningOrigin; 7],
}

impl SkateboardWheelTuningSnapshot {
    pub(crate) fn from_resolved_words(words: [Option<u32>; 7]) -> Self {
        Self {
            words: words.map(|word| word.unwrap_or(0)),
            origins: words.map(|word| {
                if word.is_some() {
                    SkateboardWheelTuningOrigin::ResolvedAttribute
                } else {
                    SkateboardWheelTuningOrigin::DefaultDataArea
                }
            }),
        }
    }

    pub(crate) const fn word(&self, attribute: SkateboardWheelAttribute) -> u32 {
        self.words[attribute.ordinal()]
    }

    pub(crate) const fn origin(
        &self,
        attribute: SkateboardWheelAttribute,
    ) -> SkateboardWheelTuningOrigin {
        self.origins[attribute.ordinal()]
    }

    fn value(&self, attribute: SkateboardWheelAttribute) -> f32 {
        f32::from_bits(self.word(attribute))
    }
}

/// Estimate visitation order is Y, X, Z for each of four ascending parts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkateboardWheelInertiaAxis {
    X,
    Y,
    Z,
}

/// The only injected numeric operation: native `vrefp128` at
/// `0x82ae9d30/38/40`. All subsequent two-refinement arithmetic stays local.
pub(crate) trait SkateboardWheelMassNumeric {
    fn reciprocal_estimate(&mut self, axis: SkateboardWheelInertiaAxis, denominator: f32) -> f32;
}

/// Ordinary host reciprocal seed; no exact Xenon estimate/FPSCR claim.
pub(crate) struct HostSkateboardWheelMassNumeric;

impl SkateboardWheelMassNumeric for HostSkateboardWheelMassNumeric {
    fn reciprocal_estimate(&mut self, _axis: SkateboardWheelInertiaAxis, denominator: f32) -> f32 {
        denominator.recip()
    }
}

/// Native debug primitive is SPHERE (3), even for a capsule collision volume.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkateboardWheelDebugSphere {
    pub(crate) radius: f32,
    pub(crate) index: u32,
}

impl SkateboardWheelDebugSphere {
    pub(crate) const NATIVE_TYPE: u32 = 3;

    pub(crate) const fn end_index(self) -> u32 {
        self.index + 1
    }
}

/// One authored part. No public fields can turn caller-supplied numbers into
/// a trusted wheel definition or a simulation body occurrence.
#[derive(Debug)]
pub(crate) struct SkateboardWheelPartDefinition {
    ordinal: u8,
    volume: Arc<Volume>,
    inertia: SimulationInertia,
    inverse_tensor_w: f32,
    inverse_body_transform: Option<[f32; 16]>,
    is_static_body: bool,
    material: AssemblyPartMaterial,
    debug: SkateboardWheelDebugSphere,
}

impl SkateboardWheelPartDefinition {
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
        self.inverse_tensor_w
    }
    pub(crate) const fn inverse_body_transform(&self) -> Option<&[f32; 16]> {
        self.inverse_body_transform.as_ref()
    }
    pub(crate) const fn is_static_body(&self) -> bool {
        self.is_static_body
    }
    pub(crate) const fn material(&self) -> AssemblyPartMaterial {
        self.material
    }
    pub(crate) const fn debug(&self) -> SkateboardWheelDebugSphere {
        self.debug
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkateboardWheelDefinitionError {
    DebugCapacity { initial_count: u32 },
}

/// Trusted fresh four-wheel definition owner, extensible by an eventual
/// seven-part skateboard owner without fabricating serialized arena bytes.
/// Retains its immutable tuning provenance and shared geometry. It deliberately
/// issues neither pool handles nor guest-resource addresses.
#[derive(Debug)]
pub(crate) struct SkateboardWheelDefinitions {
    tuning: SkateboardWheelTuningSnapshot,
    parts: [SkateboardWheelPartDefinition; WHEEL_COUNT],
    debug_end: u32,
}

impl SkateboardWheelDefinitions {
    /// `CreateWheels`, `0x82b344f0..0x82b34a8c`, on freshly initialized parts.
    ///
    /// The debug-capacity refusal is host safety policy before any estimate
    /// calls. Native uses an unchecked 32-element array. Arc allocation failure
    /// follows Rust allocation policy, not a reconstructed native error return.
    pub(crate) fn try_create(
        tuning: SkateboardWheelTuningSnapshot,
        initial_debug_count: u32,
        numeric: &mut impl SkateboardWheelMassNumeric,
    ) -> Result<Self, SkateboardWheelDefinitionError> {
        if initial_debug_count > DEBUG_OBJECT_CAPACITY - 4 {
            return Err(SkateboardWheelDefinitionError::DebugCapacity {
                initial_count: initial_debug_count,
            });
        }
        let length = tuning.value(SkateboardWheelAttribute::CapsuleLength);
        let radius = tuning.value(SkateboardWheelAttribute::Radius);
        // Raw BO=4, BI=25 at 0x82b345c8 branches when !GT, including unordered.
        let (shape, half_height) = if length > 0.0 {
            (VolumeType::Capsule, length * f32::from_bits(0x3f00_0000))
        } else {
            (VolumeType::Sphere, 0.0)
        };
        let volume = Arc::new(Volume {
            transform: FRESH_VOLUME_TRANSFORM,
            // This existing semantic Volume API uses the type discriminant;
            // it is not a fabricated guest vtable pointer or arena resource.
            vtable_offset: shape as u32,
            // Inactive native union words are uninitialized. Canonical zero
            // host storage has no native-byte/observable-padding parity claim.
            shape_words: [half_height.to_bits(), 0, 0],
            radius,
            group_id: 5,
            surface_id: 0,
            flags: 1,
        });
        let dynamic_friction = tuning.value(SkateboardWheelAttribute::DynamicFriction);
        let mut static_friction = tuning.value(SkateboardWheelAttribute::StaticFriction);
        // Raw !LT branch at 0x82b34858 retains static on equality or unordered.
        if static_friction < dynamic_friction {
            static_friction = dynamic_friction;
        }
        let parts = std::array::from_fn(|index| {
            let mut inertia = SimulationInertia::native_default();
            inertia.max_omega = f32::from_bits(0x476a_5fff);
            let material = AssemblyPartMaterial {
                static_friction,
                dynamic_friction,
                restitution: tuning.value(SkateboardWheelAttribute::Restitution),
            };
            let mass = tuning.value(SkateboardWheelAttribute::Mass)
                * tuning.value(SkateboardWheelAttribute::WorldMassMultiplier);
            let inverse_body_transform = apply_fresh_mass_properties(
                shape,
                radius,
                half_height,
                mass,
                &mut inertia,
                numeric,
            );
            SkateboardWheelPartDefinition {
                ordinal: u8::try_from(index).expect("four wheel indices fit u8"),
                volume: Arc::clone(&volume),
                inverse_tensor_w: inertia.inverse_tensor[0],
                inertia,
                inverse_body_transform,
                is_static_body: false,
                material,
                debug: SkateboardWheelDebugSphere {
                    radius: tuning.value(SkateboardWheelAttribute::Radius),
                    index: initial_debug_count
                        + u32::try_from(index).expect("four indices fit u32"),
                },
            }
        });
        Ok(Self {
            tuning,
            parts,
            debug_end: initial_debug_count + 4,
        })
    }

    pub(crate) const fn tuning(&self) -> &SkateboardWheelTuningSnapshot {
        &self.tuning
    }
    pub(crate) const fn parts(&self) -> &[SkateboardWheelPartDefinition; WHEEL_COUNT] {
        &self.parts
    }
    pub(crate) const fn debug_end(&self) -> u32 {
        self.debug_end
    }
}

/// Scalar `fsel` input is the already rounded subtraction, not `f32::max`.
fn select_nonnegative(condition: f32, nonnegative: f32, negative_or_nan: f32) -> f32 {
    if condition >= 0.0 {
        nonnegative
    } else {
        negative_or_nan
    }
}

/// Sphere branch of `ComputeSimpleInertia`, `0x82ae941c..0x82ae9474`.
pub(crate) fn sphere_unit_mass(radius: f32) -> ([f32; 3], f32) {
    let radius = select_nonnegative(radius - MINIMUM_RADIUS, radius, MINIMUM_RADIUS);
    let squared = radius * radius;
    let diagonal = squared * f32::from_bits(0x3ecc_cccd);
    let volume = (squared * radius) * f32::from_bits(0x4086_0a92);
    ([diagonal; 3], volume)
}

/// `ComputeCapsuleInertia`, `0x82ae8100..0x82ae81d0`. Axis is local Z.
/// This exact scalar grouping is intentionally not a generic capsule formula.
pub(crate) fn capsule_unit_mass(radius: f32, half_height: f32) -> ([f32; 3], f32) {
    let height_squared = half_height * half_height;
    let minimum_by_height = half_height * f32::from_bits(0x3d4c_cccd);
    let three_height = half_height * 3.0;
    let three_quarter_height = half_height * 0.75;
    let two_height = half_height * 2.0;
    let height_cubed = height_squared * half_height;
    let four_height_squared = height_squared * 4.0;
    let radius = select_nonnegative(radius - MINIMUM_RADIUS, radius, MINIMUM_RADIUS);
    let radius = select_nonnegative(radius - minimum_by_height, radius, minimum_by_height);
    let radial_term = radius.mul_add(f32::from_bits(0x3fcc_cccd), three_quarter_height);
    let denominator = radius.mul_add(4.0, three_height);
    let radius_squared = radius * radius;
    let volume_length = radius.mul_add(f32::from_bits(0x3faa_aaab), two_height);
    let partial = radial_term.mul_add(radius, four_height_squared);
    let axial_diagonal = radius_squared * f32::from_bits(0x3ecc_cccd);
    let volume_partial = volume_length * radius;
    let numerator = partial.mul_add(radius, height_cubed);
    let volume_partial = volume_partial * radius;
    let transverse_diagonal = numerator / denominator;
    let volume = volume_partial * f32::from_bits(0x4049_0fdb);
    (
        [transverse_diagonal, transverse_diagonal, axial_diagonal],
        volume,
    )
}

/// Fresh identity-volume subset of `ComputeMassProperties`, `0x82ae9a50`.
/// No arbitrary-frame diagonalization or volume-integral fallback is invented.
pub(crate) fn apply_fresh_mass_properties(
    shape: VolumeType,
    radius: f32,
    half_height: f32,
    supplied_mass: f32,
    inertia: &mut SimulationInertia,
    numeric: &mut impl SkateboardWheelMassNumeric,
) -> Option<[f32; 16]> {
    let (diagonal, volume) = match shape {
        VolumeType::Sphere => sphere_unit_mass(radius),
        VolumeType::Capsule => capsule_unit_mass(radius, half_height),
        _ => unreachable!("fresh wheel owner only constructs sphere/capsule"),
    };
    let threshold = ((diagonal[1] + diagonal[2]) + diagonal[0]) * f32::from_bits(0x3586_37be);
    // Fresh translation is zero and fresh axes are exactly I/J/K. The source
    // !LT branch admits equality/unordered to inverse construction. In that
    // arm its transpose and Z/Y/X translation products yield this same matrix.
    let inverse_body_transform = if 0.0 < threshold {
        None
    } else {
        Some(FRESH_VOLUME_TRANSFORM)
    };
    apply_unit_density_mass_to_inertia(diagonal, volume, supplied_mass, inertia, numeric);
    inverse_body_transform
}

/// Shared mass-application suffix of `ComputeMassProperties`,
/// `0x82ae9cd8..0x82ae9dc0`, for the wheel/truck/deck density-one call sites.
///
/// The caller supplies the source principal unit-mass diagonal and volume;
/// this does not choose geometry, diagonalize or construct an inverse frame.
/// Existing maxima and drag fields remain unchanged. Estimate order and both
/// fused refinement rounds are identical for primitive and aggregate callers.
pub(crate) fn apply_unit_density_mass_to_inertia(
    diagonal: [f32; 3],
    volume: f32,
    supplied_mass: f32,
    inertia: &mut SimulationInertia,
    numeric: &mut impl SkateboardWheelMassNumeric,
) {
    // Source !LT skips fallback for unordered, unlike a `mass >= minimum` test.
    let mass = if supplied_mass < f32::MIN_POSITIVE {
        volume * 1.0
    } else {
        supplied_mass
    };
    inertia.inverse_mass = 1.0 / mass;
    let scaled = diagonal.map(|value| value * mass);
    let denominator = [scaled[1], scaled[0], scaled[2]];
    let mut estimates = [
        numeric.reciprocal_estimate(SkateboardWheelInertiaAxis::Y, denominator[0]),
        numeric.reciprocal_estimate(SkateboardWheelInertiaAxis::X, denominator[1]),
        numeric.reciprocal_estimate(SkateboardWheelInertiaAxis::Z, denominator[2]),
    ];
    // All three residuals precede all three corrections on each refinement.
    for _ in 0..2 {
        let residuals: [f32; 3] =
            std::array::from_fn(|axis| (-denominator[axis]).mul_add(estimates[axis], 1.0));
        estimates =
            std::array::from_fn(|axis| estimates[axis].mul_add(residuals[axis], estimates[axis]));
    }
    inertia.inverse_tensor = [estimates[1], estimates[0], estimates[2]];
    let [x, y, z] = inertia.inverse_tensor;
    let minimum_xy = if x < y { x } else { y };
    let minimum = if minimum_xy < z { minimum_xy } else { z };
    inertia.spherical = 1.0 / minimum;
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHIPPED_WORDS: [u32; 7] = [
        0,
        0x3cfd_f3b6,
        0x3f33_3333,
        0x3f4c_cccd,
        0,
        0x3da9_fbe7,
        0x40a0_0000,
    ];

    fn tuning() -> SkateboardWheelTuningSnapshot {
        SkateboardWheelTuningSnapshot::from_resolved_words(SHIPPED_WORDS.map(Some))
    }

    #[derive(Default)]
    struct TraceNumeric {
        calls: Vec<(SkateboardWheelInertiaAxis, u32)>,
    }

    impl SkateboardWheelMassNumeric for TraceNumeric {
        fn reciprocal_estimate(
            &mut self,
            axis: SkateboardWheelInertiaAxis,
            denominator: f32,
        ) -> f32 {
            self.calls.push((axis, denominator.to_bits()));
            denominator.recip()
        }
    }

    #[test]
    fn exact_keys_and_default_zero_policy_preserve_source_bits() {
        use skate2_formats::attrib::string_hash64;
        let names = [
            "WheelCapsuleLength",
            "WheelRadius",
            "WheelDynamicFriction",
            "WheelStaticFriction",
            "WheelRestitution",
            "WheelMass",
        ];
        for (attribute, name) in SkateboardWheelAttribute::ALL.into_iter().zip(names) {
            assert_eq!(attribute.key(), string_hash64(name));
            assert_eq!(attribute.class_key(), string_hash64("physicswheels"));
            assert_eq!(
                SkateboardWheelAttribute::COLLECTION_KEY,
                string_hash64("default")
            );
            assert_eq!(SkateboardWheelAttribute::ELEMENT_INDEX, 0);
        }
        assert_eq!(
            SkateboardWheelAttribute::WorldMassMultiplier.class_key(),
            string_hash64("physics_world")
        );
        assert_eq!(
            SkateboardWheelAttribute::WorldMassMultiplier.key(),
            0x4650_7e40_c853_72fc
        );
        let mut values = [None; 7];
        values[1] = Some(0x8000_0000);
        values[3] = Some(0x7fc0_1234);
        let snapshot = SkateboardWheelTuningSnapshot::from_resolved_words(values);
        assert_eq!(snapshot.word(SkateboardWheelAttribute::Radius), 0x8000_0000);
        assert_eq!(
            snapshot.word(SkateboardWheelAttribute::StaticFriction),
            0x7fc0_1234
        );
        assert_eq!(
            snapshot.origin(SkateboardWheelAttribute::Radius),
            SkateboardWheelTuningOrigin::ResolvedAttribute
        );
        assert_eq!(
            snapshot.origin(SkateboardWheelAttribute::Mass),
            SkateboardWheelTuningOrigin::DefaultDataArea
        );
        assert_eq!(snapshot.word(SkateboardWheelAttribute::Mass), 0);
    }

    #[test]
    fn shipped_four_parts_share_one_sphere_with_exact_defaults_and_debug_ranges() {
        let mut numeric = TraceNumeric::default();
        let definitions =
            SkateboardWheelDefinitions::try_create(tuning(), 2, &mut numeric).unwrap();
        assert_eq!(definitions.debug_end(), 6);
        assert_eq!(definitions.tuning(), &tuning());
        assert_eq!(numeric.calls.len(), 12);
        for (index, part) in definitions.parts().iter().enumerate() {
            assert_eq!(usize::from(part.ordinal()), index);
            assert!(Arc::ptr_eq(part.volume(), definitions.parts()[0].volume()));
            assert_eq!(part.volume().volume_type(), Some(VolumeType::Sphere));
            assert_eq!(
                part.volume().transform.map(f32::to_bits),
                FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
            );
            assert_eq!(part.volume().radius.to_bits(), SHIPPED_WORDS[1]);
            assert_eq!(
                (
                    part.volume().group_id,
                    part.volume().surface_id,
                    part.volume().flags
                ),
                (5, 0, 1)
            );
            assert!(!part.is_static_body());
            assert!(part.inverse_body_transform().is_none());
            assert_eq!(part.inertia().inverse_mass.to_bits(), 0x401a_3785);
            assert_eq!(part.inertia().max_velocity.to_bits(), 0x7f7f_ffff);
            assert_eq!(part.inertia().max_omega.to_bits(), 0x476a_5fff);
            assert_eq!(part.inertia().linear_drag.to_bits(), 0);
            assert_eq!(part.inertia().angular_drag.to_bits(), 0);
            assert_eq!(
                part.inverse_tensor_w().to_bits(),
                part.inertia().inverse_tensor[0].to_bits()
            );
            assert_eq!(part.material().static_friction.to_bits(), SHIPPED_WORDS[3]);
            assert_eq!(part.material().dynamic_friction.to_bits(), SHIPPED_WORDS[2]);
            assert_eq!(part.material().restitution.to_bits(), 0);
            assert_eq!(part.debug().radius.to_bits(), SHIPPED_WORDS[1]);
            assert_eq!(part.debug().index, 2 + u32::try_from(index).unwrap());
            assert_eq!(part.debug().end_index(), 3 + u32::try_from(index).unwrap());
            assert_eq!(
                &numeric.calls[index * 3..index * 3 + 3],
                &[
                    (SkateboardWheelInertiaAxis::Y, 0x3927_466f),
                    (SkateboardWheelInertiaAxis::X, 0x3927_466f),
                    (SkateboardWheelInertiaAxis::Z, 0x3927_466f),
                ]
            );
        }
        assert_eq!(SkateboardWheelDebugSphere::NATIVE_TYPE, 3);
    }

    #[test]
    fn raw_branch_unordered_and_signed_zero_cases_are_not_generic_max_tests() {
        for length in [0.0_f32, -0.0, -1.0, f32::NAN] {
            let mut words = SHIPPED_WORDS;
            words[0] = length.to_bits();
            let definitions = SkateboardWheelDefinitions::try_create(
                SkateboardWheelTuningSnapshot::from_resolved_words(words.map(Some)),
                0,
                &mut HostSkateboardWheelMassNumeric,
            )
            .unwrap();
            assert_eq!(
                definitions.parts()[0].volume().volume_type(),
                Some(VolumeType::Sphere)
            );
        }
        for (static_word, dynamic_word, expected) in [
            (0x8000_0000, 0, 0x8000_0000),
            (0x7fc0_1234, 0x3f80_0000, 0x7fc0_1234),
            (0x3f80_0000, 0x7fc0_5678, 0x3f80_0000),
            (0x3f00_0000, 0x3f80_0000, 0x3f80_0000),
        ] {
            let mut words = SHIPPED_WORDS;
            words[2] = dynamic_word;
            words[3] = static_word;
            let definitions = SkateboardWheelDefinitions::try_create(
                SkateboardWheelTuningSnapshot::from_resolved_words(words.map(Some)),
                0,
                &mut HostSkateboardWheelMassNumeric,
            )
            .unwrap();
            assert_eq!(
                definitions.parts()[0].material().static_friction.to_bits(),
                expected
            );
        }
    }

    #[test]
    fn sphere_scalar_packet_and_radius_fsel_match_source_constants() {
        let (diagonal, volume) = sphere_unit_mass(f32::from_bits(SHIPPED_WORDS[1]));
        assert_eq!(diagonal.map(f32::to_bits), [0x39c9_894b; 3]);
        assert_eq!(volume.to_bits(), 0x3902_d997);
        let minimum = sphere_unit_mass(MINIMUM_RADIUS);
        for radius in [-1.0_f32, 0.0, -0.0, f32::NAN] {
            let actual = sphere_unit_mass(radius);
            assert_eq!(actual.0.map(f32::to_bits), minimum.0.map(f32::to_bits));
            assert_eq!(actual.1.to_bits(), minimum.1.to_bits());
        }
    }

    #[test]
    fn capsule_uses_z_axis_and_keeps_debug_sphere_distinct() {
        let mut words = SHIPPED_WORDS;
        words[0] = 0.5_f32.to_bits();
        let definitions = SkateboardWheelDefinitions::try_create(
            SkateboardWheelTuningSnapshot::from_resolved_words(words.map(Some)),
            28,
            &mut HostSkateboardWheelMassNumeric,
        )
        .unwrap();
        let part = &definitions.parts()[0];
        assert_eq!(part.volume().volume_type(), Some(VolumeType::Capsule));
        assert_eq!(part.volume().shape_words[0], 0.25_f32.to_bits());
        let gp = crate::narrow_phase::GpPrimitive::from_volume(part.volume(), None).unwrap();
        assert_eq!(gp.edge_directions[0].map(f32::to_bits), [0, 0, 0x3f80_0000]);
        assert_eq!(part.debug().radius.to_bits(), SHIPPED_WORDS[1]);
        assert_eq!(definitions.debug_end(), 32);
        let (diagonal, volume) = capsule_unit_mass(f32::from_bits(SHIPPED_WORDS[1]), 0.25);
        assert_eq!(diagonal[0].to_bits(), diagonal[1].to_bits());
        assert!(diagonal[0] > diagonal[2]);
        assert_eq!(diagonal[2].to_bits(), 0x39c9_894b);
        // Independent rational-intermediate source evaluator and CRT fmaf
        // packet agree; neither is a captured Xenon runtime observation.
        assert_eq!(
            diagonal.map(f32::to_bits),
            [0x3cdd_3abf, 0x3cdd_3abf, 0x39c9_894b]
        );
        assert_eq!(volume.to_bits(), 0x3ad6_36cc);
    }

    #[test]
    fn mass_fallback_is_strict_and_nan_mass_is_retained() {
        let radius = f32::from_bits(SHIPPED_WORDS[1]);
        let (_, volume) = sphere_unit_mass(radius);
        for mass in [-1.0_f32, 0.0, -0.0, f32::from_bits(1)] {
            let mut inertia = SimulationInertia::native_default();
            apply_fresh_mass_properties(
                VolumeType::Sphere,
                radius,
                0.0,
                mass,
                &mut inertia,
                &mut HostSkateboardWheelMassNumeric,
            );
            assert_eq!(inertia.inverse_mass.to_bits(), (1.0 / volume).to_bits());
        }
        let mut inertia = SimulationInertia::native_default();
        apply_fresh_mass_properties(
            VolumeType::Sphere,
            radius,
            0.0,
            f32::MIN_POSITIVE,
            &mut inertia,
            &mut HostSkateboardWheelMassNumeric,
        );
        assert_eq!(
            inertia.inverse_mass.to_bits(),
            (1.0 / f32::MIN_POSITIVE).to_bits()
        );
        apply_fresh_mass_properties(
            VolumeType::Sphere,
            radius,
            0.0,
            f32::NAN,
            &mut inertia,
            &mut HostSkateboardWheelMassNumeric,
        );
        assert!(inertia.inverse_mass.is_nan());
    }

    #[test]
    fn debug_capacity_refuses_before_any_numeric_work() {
        let mut numeric = TraceNumeric::default();
        for initial in [29, 32, u32::MAX] {
            assert_eq!(
                SkateboardWheelDefinitions::try_create(tuning(), initial, &mut numeric)
                    .unwrap_err(),
                SkateboardWheelDefinitionError::DebugCapacity {
                    initial_count: initial
                }
            );
        }
        assert!(numeric.calls.is_empty());
    }

    #[test]
    fn injected_estimates_distinguish_two_refinements_and_spherical_reciprocal() {
        struct SuppliedEstimates {
            calls: Vec<(SkateboardWheelInertiaAxis, u32)>,
        }
        impl SkateboardWheelMassNumeric for SuppliedEstimates {
            fn reciprocal_estimate(
                &mut self,
                axis: SkateboardWheelInertiaAxis,
                denominator: f32,
            ) -> f32 {
                self.calls.push((axis, denominator.to_bits()));
                match axis {
                    SkateboardWheelInertiaAxis::Y => 0.5,
                    SkateboardWheelInertiaAxis::X => 1.0,
                    SkateboardWheelInertiaAxis::Z => 2.0,
                }
            }
        }
        let mut numeric = SuppliedEstimates { calls: Vec::new() };
        let mut inertia = SimulationInertia::native_default();
        let inverse = apply_fresh_mass_properties(
            VolumeType::Sphere,
            1.0,
            0.0,
            2.0,
            &mut inertia,
            &mut numeric,
        );
        assert!(inverse.is_none());
        assert_eq!(
            numeric.calls,
            [
                (SkateboardWheelInertiaAxis::Y, 0x3f4c_cccd),
                (SkateboardWheelInertiaAxis::X, 0x3f4c_cccd),
                (SkateboardWheelInertiaAxis::Z, 0x3f4c_cccd),
            ]
        );
        // Frozen independently evaluated single-precision FMA packet. The
        // unequal seed residuals rule out replacing the seam with division.
        assert_eq!(
            inertia.inverse_tensor.map(f32::to_bits),
            [0x3f9f_be77, 0x3f8b_4396, 0x3f8b_4395]
        );
        assert_eq!(inertia.spherical.to_bits(), 0x3f6b_4b4c);
        assert_eq!(inertia.inverse_mass.to_bits(), 0x3f00_0000);
    }

    #[test]
    fn unordered_com_threshold_constructs_inverse_fresh_frame_with_zero_w() {
        let mut words = SHIPPED_WORDS;
        words[0] = f32::INFINITY.to_bits();
        let definitions = SkateboardWheelDefinitions::try_create(
            SkateboardWheelTuningSnapshot::from_resolved_words(words.map(Some)),
            0,
            &mut HostSkateboardWheelMassNumeric,
        )
        .unwrap();
        let inverse = definitions.parts()[0]
            .inverse_body_transform()
            .expect("NaN threshold takes !LT branch");
        assert_eq!(
            inverse.map(f32::to_bits),
            FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
        );
        assert_eq!(inverse[15].to_bits(), 0);
    }

    #[test]
    fn spherical_minimum_selects_later_zero_or_unordered_operand() {
        struct AxisSeeds([f32; 3]);
        impl SkateboardWheelMassNumeric for AxisSeeds {
            fn reciprocal_estimate(&mut self, axis: SkateboardWheelInertiaAxis, _: f32) -> f32 {
                self.0[match axis {
                    SkateboardWheelInertiaAxis::X => 0,
                    SkateboardWheelInertiaAxis::Y => 1,
                    SkateboardWheelInertiaAxis::Z => 2,
                }]
            }
        }
        for (seeds, expected) in [
            ([-0.0, -0.0, 0.0], f32::INFINITY.to_bits()),
            ([0.0, 0.0, -0.0], f32::NEG_INFINITY.to_bits()),
        ] {
            let mut inertia = SimulationInertia::native_default();
            apply_fresh_mass_properties(
                VolumeType::Sphere,
                1.0,
                0.0,
                2.0,
                &mut inertia,
                &mut AxisSeeds(seeds),
            );
            assert_eq!(inertia.spherical.to_bits(), expected);
        }
        let mut inertia = SimulationInertia::native_default();
        apply_fresh_mass_properties(
            VolumeType::Sphere,
            1.0,
            0.0,
            2.0,
            &mut inertia,
            &mut AxisSeeds([1.0, 1.0, f32::NAN]),
        );
        assert!(
            inertia.spherical.is_nan(),
            "unordered final comparison selects Z"
        );
    }
}
