//! The existing CharacterModel definition's native material/mass initialization.
//! cLivingWorldCollisionBody.Create82e5cff0 writes materials, calls SetMasses,
//! then creates Parts. This module implements the definition writes only.
use super::{CharacterModel, ModelCollisionDefinition, ModelCollisionGeometry, ModelCollisionPart};
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    simulation_volume_integral::{
        SimulationVolumeIntegralNumeric,
        arena::{ArenaIntegralError, try_vehicle_mapped_integral},
        simple_volume_principal, try_diagonalise_volume_integral,
    },
    skeleton_body_mass::SkeletonBodyMassNumeric,
};
use skate2_formats::collision::AssemblyPartMaterial;

#[derive(Debug)]
pub(crate) enum CharacterMassError<N> {
    PartCount { parts: usize, masses: usize },
    MissingRoot(usize),
    UnsupportedPrimitive(u32),
    Integral(ArenaIntegralError<N>),
    Numeric(N),
}

impl ModelCollisionPart {
    /// ComputeMassProperties82ae9a50 with density1. Existing BodyMassState
    /// preserves inactive inverse-frame storage and all packed inertia lanes.
    pub(crate) fn try_compute_mass_properties<N: SimulationVolumeIntegralNumeric>(
        &mut self,
        mass: f32,
        integral_numeric: &mut N,
        mass_numeric: &mut impl SkeletonBodyMassNumeric,
    ) -> Result<bool, CharacterMassError<N::Error>> {
        let principal = match &self.geometry {
            ModelCollisionGeometry::Primitive(volume) => simple_volume_principal(volume).ok_or(
                CharacterMassError::UnsupportedPrimitive(volume.vtable_offset),
            )?,
            ModelCollisionGeometry::VehicleMappedArray {
                source,
                root,
                extra_box,
            } => {
                let volume = &source
                    .aggregate_roots
                    .get(*root)
                    .ok_or(CharacterMassError::MissingRoot(*root))?
                    .volume;
                let mut mapped =
                    try_vehicle_mapped_integral(source, volume, extra_box, integral_numeric)
                        .map_err(CharacterMassError::Integral)?;
                // A recognized mapped array's outer ComputeVolumeIntegral
                // succeeds regardless of its inner success flag82ae9924.
                try_diagonalise_volume_integral(&mut mapped.integral, integral_numeric)
                    .map_err(CharacterMassError::Numeric)?
            }
        };
        self.mass.apply_principal(
            principal.diagonal,
            principal.volume,
            principal.body_to_part,
            mass,
            mass_numeric,
        );
        Ok(true)
    }
}

impl ModelCollisionDefinition {
    /// SetMaterialProperties82e4c818 writes all Parts, including disabled ones.
    pub(crate) fn set_material_properties(&mut self, material: AssemblyPartMaterial) {
        for part in &mut self.parts {
            part.material = material;
        }
    }

    /// SetMasses82e50c10 asserts exact count before visiting Parts in order.
    pub(crate) fn try_set_masses<N: SimulationVolumeIntegralNumeric>(
        &mut self,
        masses: &[f32],
        integral_numeric: &mut N,
        mass_numeric: &mut impl SkeletonBodyMassNumeric,
    ) -> Result<(), CharacterMassError<N::Error>> {
        if masses.len() != self.parts.len() {
            return Err(CharacterMassError::PartCount {
                parts: self.parts.len(),
                masses: masses.len(),
            });
        }
        for (part, &mass) in self.parts.iter_mut().zip(masses) {
            part.try_compute_mass_properties(mass, integral_numeric, mass_numeric)?;
        }
        Ok(())
    }
}

impl CharacterModel {
    /// FillSkaterCollisionMasses82d285f8 (also used by FillWorldCollisionMasses).
    /// Source samples the density before reading model dimensions.
    pub(crate) fn try_vehicle_collision_mass(
        &self,
        cvars: &InputCvarReferences,
    ) -> Result<f32, InputCvarError> {
        let density = cvars.try_vehicle_collision_density()?;
        Ok(((self.dimensions[0] * self.dimensions[1]) * self.dimensions[2]) * density)
    }
}
