//! Native constructed collision definitions -> existing shared Part lifecycle.
//! cLivingWorldCollisionBody.Create82e5cff0, PhysicsUtility.CreateAssembly82e4a540.
//! Island membership, serialized publication and collision callbacks are later
//! phases. No DMO arena or Xenon source pointer is invented for these models.
use super::{CharacterModel, SharedCollisionDefinition, collision_mass::CharacterMassError};
use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyBodyOnlyPartSession, DynamicAssemblyPartAttachmentMetadata,
        DynamicAssemblyPartAttachmentTemplate, DynamicAssemblyPartPayload,
        DynamicAssemblySessionCreateError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    serialised_assembly_template::{
        SERIALISED_ASSEMBLY_PART_CAPACITY, SERIALISED_ASSEMBLY_VOLUME_CAPACITY,
        SerialisedAssemblyTemplateError, SerialisedAssemblyVolumeSource,
        append_constructed_mapped_volume_sources, append_volume_source,
    },
    simulation_assembly_backend::SimulationAssemblyBackend,
    simulation_assembly_payload::SimulationAssemblyInertia,
    simulation_joint_pool::{SimulationJointFrames, SimulationJointLimits},
    simulation_rigid_body_pool::{
        SimulationMutableInertia, SimulationRigidBodyPoolAuthority, SimulationRigidBodyPoolError,
    },
    simulation_volume_integral::SimulationVolumeIntegralNumeric,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_body_mass::SkeletonBodyMassNumeric,
};
use skate2_formats::collision::{AssemblyPartMaterial, BitTable};
use std::sync::Arc;

pub(crate) type CharacterCollisionBackend =
    SimulationAssemblyBackend<AssemblyPartMaterial, SimulationJointFrames, SimulationJointLimits>;
type Attachment =
    DynamicAssemblyPartAttachmentTemplate<SimulationAssemblyInertia, AssemblyPartMaterial>;

/// One binding per shared model Definition. Every admitted body's Inertia
/// payload refers to these cells, as native Parts refer to Definition memory.
#[derive(Debug)]
pub(crate) struct ModelPartBinding {
    backend: SimulationRigidBodyPoolAuthority,
    inertias: Vec<SimulationMutableInertia>,
    metadata: Vec<DynamicAssemblyPartAttachmentMetadata>,
    materials: Vec<AssemblyPartMaterial>,
    attachment: Arc<Attachment>,
}

pub(crate) struct CharacterCollisionAssembly {
    backend: SimulationRigidBodyPoolAuthority,
    pub(crate) definition: SharedCollisionDefinition,
    pub(crate) parts: DynamicAssemblyBodyOnlyPartSession<CharacterCollisionBackend>,
    pub(crate) group: u32,
    pub(crate) part_user_data: Vec<u32>,
    pub(crate) culling: Arc<BitTable>,
}

#[derive(Debug)]
pub(crate) enum CharacterAssemblyError<N> {
    PoisonedDefinition,
    ForeignBackend,
    Allocation,
    PartCapacity(usize),
    Mass(CharacterMassError<N>),
    MissingInverseFrame(usize),
    DefinitionPartCount {
        previous: usize,
        current: usize,
    },
    /// Dynamic changes to these Definition fields need live Part pointer
    /// propagation. Refuse explicitly rather than leaving existing Parts stale.
    LiveDefinitionLayoutChange(usize),
    Inertia(SimulationRigidBodyPoolError),
    Session(DynamicAssemblySessionCreateError),
}

impl CharacterCollisionAssembly {
    pub(crate) fn backend_authority(&self) -> SimulationRigidBodyPoolAuthority {
        self.backend
    }

    pub(crate) fn try_create<N: SimulationVolumeIntegralNumeric>(
        definition: &SharedCollisionDefinition,
        backend: &mut CharacterCollisionBackend,
        group: u32,
        masses: &[f32],
        material: AssemblyPartMaterial,
        integral_numeric: &mut N,
        mass_numeric: &mut impl SkeletonBodyMassNumeric,
    ) -> Result<Self, CharacterAssemblyError<N::Error>> {
        let mut current = definition
            .lock()
            .map_err(|_| CharacterAssemblyError::PoisonedDefinition)?;
        if current
            .binding
            .as_ref()
            .is_some_and(|binding| binding.backend != backend.authority())
        {
            return Err(CharacterAssemblyError::ForeignBackend);
        }
        current.set_material_properties(material);
        current
            .try_set_masses(masses, integral_numeric, mass_numeric)
            .map_err(CharacterAssemblyError::Mass)?;
        let count = current.parts.len();
        if count > SERIALISED_ASSEMBLY_PART_CAPACITY {
            return Err(CharacterAssemblyError::PartCapacity(count));
        }
        let mut metadata = Vec::new();
        metadata
            .try_reserve_exact(count)
            .map_err(|_| CharacterAssemblyError::Allocation)?;
        for (index, part) in current.parts.iter().enumerate() {
            let mass = &part.mass;
            let inverse_body_transform = if mass.has_inverse_body_transform {
                mass.inverse_body_transform_storage
                    .ok_or(CharacterAssemblyError::MissingInverseFrame(index))?
            } else {
                // Inactive storage is never consumed, same existing skeleton
                // attachment convention; no claim native initialized it.
                mass.inverse_body_transform_storage
                    .unwrap_or(FRESH_VOLUME_TRANSFORM)
            };
            metadata.push(DynamicAssemblyPartAttachmentMetadata {
                inverse_body_transform,
                has_inverse_body_transform: u32::from(mass.has_inverse_body_transform),
                is_static_body: u32::from(part.is_static_body),
            });
        }
        if let Some(binding) = &current.binding {
            if binding.metadata.len() != count {
                return Err(CharacterAssemblyError::DefinitionPartCount {
                    previous: binding.metadata.len(),
                    current: count,
                });
            }
            for (index, part) in current.parts.iter().enumerate() {
                let old = binding.metadata[index];
                let new = metadata[index];
                if old.has_inverse_body_transform != new.has_inverse_body_transform
                    || old.is_static_body != new.is_static_body
                    || (new.has_inverse_body_transform != 0
                        && old.inverse_body_transform.map(f32::to_bits)
                            != new.inverse_body_transform.map(f32::to_bits))
                    || !same_material(binding.materials[index], part.material)
                {
                    return Err(CharacterAssemblyError::LiveDefinitionLayoutChange(index));
                }
                backend
                    .try_write_living_world_definition_inertia(
                        &binding.inertias[index],
                        part.mass.inertia,
                        part.mass.inverse_tensor_w,
                    )
                    .map_err(CharacterAssemblyError::Inertia)?;
            }
        } else {
            let mut inertias = Vec::new();
            let mut payloads = Vec::new();
            let mut materials = Vec::new();
            inertias
                .try_reserve_exact(count)
                .map_err(|_| CharacterAssemblyError::Allocation)?;
            payloads
                .try_reserve_exact(count)
                .map_err(|_| CharacterAssemblyError::Allocation)?;
            materials
                .try_reserve_exact(count)
                .map_err(|_| CharacterAssemblyError::Allocation)?;
            for part in &current.parts {
                let inertia = SimulationMutableInertia::new_packed(
                    part.mass.inertia,
                    part.mass.inverse_tensor_w,
                );
                payloads.push(DynamicAssemblyPartPayload {
                    inertia: SimulationAssemblyInertia::mutable(inertia.clone()),
                    material: part.material,
                });
                materials.push(part.material);
                inertias.push(inertia);
            }
            let attachment = Arc::new(
                DynamicAssemblyPartAttachmentTemplate::try_from_parts(metadata.clone(), payloads)
                    .expect("one Definition metadata and inertia payload per Part"),
            );
            current.binding = Some(Arc::new(ModelPartBinding {
                backend: backend.authority(),
                inertias,
                metadata,
                materials,
                attachment,
            }));
        }
        let mut initial = Vec::new();
        let mut part_user_data = Vec::new();
        initial
            .try_reserve_exact(count)
            .map_err(|_| CharacterAssemblyError::Allocation)?;
        part_user_data
            .try_reserve_exact(count)
            .map_err(|_| CharacterAssemblyError::Allocation)?;
        // Part.Initialize82ae8fc8 and Create82e5d08c..d0a0. Placement is a later
        // AddToWorld operation, not a guessed transform in this constructor.
        initial.resize(count, FRESH_VOLUME_TRANSFORM);
        part_user_data.resize(count, group);
        let parts = DynamicAssemblyBodyOnlyPartSession::try_new(
            Arc::clone(&current.binding.as_ref().expect("created above").attachment),
            initial,
        )
        .map_err(CharacterAssemblyError::Session)?;
        Ok(Self {
            backend: backend.authority(),
            definition: Arc::clone(definition),
            parts,
            group,
            part_user_data,
            culling: Arc::clone(&current.culling_table),
        })
    }
}

#[derive(Debug)]
pub(crate) enum CharacterVolumeSelectionError {
    PoisonedDefinition,
    Allocation,
    MissingRoot(usize),
    Template(SerialisedAssemblyTemplateError),
}

impl CharacterCollisionAssembly {
    /// SerialisedAssembly.Create82e0c850 and Update82e0cf30 select the same
    /// volumes for these programmatic shapes: top-level primitives, or a fresh
    /// SimpleMappedArray around the vehicle's original volume and extra box.
    /// Existing arena Create/Update walks now share this SAME child selector.
    /// Returning geometry is not itself a body/collision publication.
    pub(crate) fn try_volume_sources(
        &self,
    ) -> Result<Vec<SerialisedAssemblyVolumeSource>, CharacterVolumeSelectionError> {
        let definition = self
            .definition
            .lock()
            .map_err(|_| CharacterVolumeSelectionError::PoisonedDefinition)?;
        let mut sources = Vec::new();
        sources
            .try_reserve_exact(SERIALISED_ASSEMBLY_VOLUME_CAPACITY)
            .map_err(|_| CharacterVolumeSelectionError::Allocation)?;
        for (ordinal, part) in definition.parts.iter().enumerate() {
            match &part.geometry {
                super::ModelCollisionGeometry::Primitive(volume) => {
                    append_volume_source(ordinal, volume, &mut sources)
                        .map_err(CharacterVolumeSelectionError::Template)?;
                }
                super::ModelCollisionGeometry::VehicleMappedArray {
                    source,
                    root,
                    extra_box,
                } => {
                    let volume = &source
                        .aggregate_roots
                        .get(*root)
                        .ok_or(CharacterVolumeSelectionError::MissingRoot(*root))?
                        .volume;
                    append_constructed_mapped_volume_sources(
                        source,
                        ordinal,
                        [volume, extra_box],
                        &mut sources,
                    )
                    .map_err(CharacterVolumeSelectionError::Template)?;
                }
            }
        }
        Ok(sources)
    }
}

#[derive(Debug)]
pub(crate) enum VehicleCollisionCreateError<N> {
    Cvar(InputCvarError),
    Assembly(CharacterAssemblyError<N>),
}

/// cLWVehicleCollision.InitCollisionBodies82d28eb8. Model-null is the caller's
/// early return. Source creates only the group10 skater body, including when
/// its fallback model has a separate world definition. Both material callbacks
/// point at noop825187c0, preserving these caller-initialized zero coefficients.
pub(crate) fn try_create_vehicle_skater_body<N: SimulationVolumeIntegralNumeric>(
    model: &CharacterModel,
    backend: &mut CharacterCollisionBackend,
    cvars: &InputCvarReferences,
    integral_numeric: &mut N,
    mass_numeric: &mut impl SkeletonBodyMassNumeric,
) -> Result<CharacterCollisionAssembly, VehicleCollisionCreateError<N::Error>> {
    let mass = model
        .try_vehicle_collision_mass(cvars)
        .map_err(VehicleCollisionCreateError::Cvar)?;
    CharacterCollisionAssembly::try_create(
        &model.skater_collision,
        backend,
        10,
        &[mass],
        AssemblyPartMaterial {
            static_friction: 0.,
            dynamic_friction: 0.,
            restitution: 0.,
        },
        integral_numeric,
        mass_numeric,
    )
    .map_err(VehicleCollisionCreateError::Assembly)
}

#[cfg(test)]
#[path = "living_world_character_collision_assembly_tests.rs"]
pub(crate) mod tests;

fn same_material(a: AssemblyPartMaterial, b: AssemblyPartMaterial) -> bool {
    [a.static_friction, a.dynamic_friction, a.restitution].map(f32::to_bits)
        == [b.static_friction, b.dynamic_friction, b.restitution].map(f32::to_bits)
}
