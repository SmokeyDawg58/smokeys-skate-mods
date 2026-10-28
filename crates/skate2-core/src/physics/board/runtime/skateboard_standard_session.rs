//! Actual constructed Body, Part inertia/material and volume writes for
//! `Skateboard::SetStandard` (0x82b355a0), through its continuing global Vault.
use super::{SkateboardBodyConstruction, SkateboardBodyMaterials, SkateboardBodyPostFields};
use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyPartMaterialError, part_contact_spy::DynamicAssemblyPartContactSpyError,
    },
    simulation_rigid_body_pool::{
        SimulationMutableInertia, SimulationRigidBodyObservation, SimulationRigidBodyPoolError,
    },
    skateboard_assembly::{
        SkateboardAssemblySession, SkateboardCollisionDefinition, SkateboardSimulationBackend,
        volume_flags::SkateboardVolumeFlagError,
    },
    skateboard_attributes::SkateboardAttributeError,
    skateboard_set_standard::{
        SkateboardSetStandardError, SkateboardSetStandardServices, SkateboardStandardMaterial,
        try_set_standard,
    },
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{fmt, sync::Arc};

#[derive(Debug)]
pub(crate) enum SkateboardStandardSessionError {
    ConstructionRequired,
    DefinitionMismatch,
    Part(DynamicAssemblyPartContactSpyError),
    Material(DynamicAssemblyPartMaterialError),
    Inertia(SimulationRigidBodyPoolError),
    Volume(SkateboardVolumeFlagError),
    DeckCount,
}
impl fmt::Display for SkateboardStandardSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConstructionRequired => {
                f.write_str("standard board requires completed constructor")
            }
            Self::DefinitionMismatch => {
                f.write_str("standard board constructor/session definitions differ")
            }
            Self::Part(e) => write!(f, "{e}"),
            Self::Material(e) => write!(f, "{e}"),
            Self::Inertia(e) => write!(f, "{e}"),
            Self::Volume(e) => write!(f, "standard board volume: {e:?}"),
            Self::DeckCount => f.write_str("deck volume count exceeds source signed count"),
        }
    }
}
pub(crate) type StandardError =
    SkateboardSetStandardError<SkateboardAttributeError, SkateboardStandardSessionError>;

impl SkateboardBodyConstruction {
    pub(crate) fn try_set_standard(
        &mut self,
        session: &mut SkateboardAssemblySession,
        backend: &mut SkateboardSimulationBackend,
    ) -> Result<(), StandardError> {
        self.with_standard_services(session, backend, |attributes, services| {
            try_set_standard(attributes, services)
        })
    }
    /// `SkateboardBody::SetAngularDamping` 0x82b24a78.
    pub(crate) fn try_set_angular_damping(
        &mut self,
        session: &mut SkateboardAssemblySession,
        backend: &mut SkateboardSimulationBackend,
        damping: f32,
    ) -> Result<(), StandardError> {
        self.with_standard_services(session, backend, |_, services| {
            let body = services
                .capture_body()
                .map_err(SkateboardSetStandardError::Service)?;
            crate::skateboard_set_standard::try_set_angular_damping(services, &body, damping)
                .map_err(SkateboardSetStandardError::Service)
        })
    }
    /// `SkateboardBody::SetStandardAngularDamping` 0x82b2b818.
    pub(crate) fn try_set_standard_angular_damping(
        &mut self,
        session: &mut SkateboardAssemblySession,
        backend: &mut SkateboardSimulationBackend,
    ) -> Result<(), StandardError> {
        self.with_standard_services(session, backend, |attributes, services| {
            let body = services
                .capture_body()
                .map_err(SkateboardSetStandardError::Service)?;
            crate::skateboard_set_standard::try_set_standard_angular_damping(
                attributes, services, &body,
            )
        })
    }
    fn with_standard_services<T>(
        &mut self,
        session: &mut SkateboardAssemblySession,
        backend: &mut SkateboardSimulationBackend,
        operation: impl FnOnce(
            &mut crate::skateboard_attributes::SkateboardAttributes,
            &mut Services<'_>,
        ) -> Result<T, StandardError>,
    ) -> Result<T, StandardError> {
        use SkateboardStandardSessionError as E;
        if !matches!(
            self.phase,
            super::SkateboardBodyConstructionPhase::Complete
                | super::SkateboardBodyConstructionPhase::HandedOff
        ) {
            return Err(SkateboardSetStandardError::Service(E::ConstructionRequired));
        }
        let collision = self
            .collision
            .as_ref()
            .ok_or(SkateboardSetStandardError::Service(E::ConstructionRequired))?;
        if !self
            .template
            .as_ref()
            .is_some_and(|template| Arc::ptr_eq(template, session.template()))
        {
            return Err(SkateboardSetStandardError::Service(E::DefinitionMismatch));
        }
        let post = self
            .post_fields
            .as_mut()
            .ok_or(SkateboardSetStandardError::Service(E::ConstructionRequired))?;
        operation(
            &mut self.attributes,
            &mut Services {
                session,
                backend,
                post,
                materials: &self.materials,
                collision,
            },
        )
    }
}

struct Services<'a> {
    session: &'a mut SkateboardAssemblySession,
    backend: &'a mut SkateboardSimulationBackend,
    post: &'a mut SkateboardBodyPostFields,
    materials: &'a SkateboardBodyMaterials,
    collision: &'a Arc<SkateboardCollisionDefinition>,
}

// These pointer captures are fixed by the single exclusive constructor/session
// borrow. Numeric fields, material identities and Part bodies are read only at
// their later source stages; no external callback can substitute another owner.
impl SkateboardSetStandardServices for Services<'_> {
    type Error = SkateboardStandardSessionError;
    type Body = ();
    type Assembly = ();
    type Parts = ();
    type PartBody = SimulationRigidBodyObservation;
    type Inertia = (SimulationRigidBodyObservation, SimulationMutableInertia);
    type Materials = [Arc<AssemblyPartMaterial>; 3];
    fn capture_body(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn capture_assembly(&mut self, (): &()) -> Result<(), Self::Error> {
        Ok(())
    }
    fn capture_parts(&mut self, (): &()) -> Result<(), Self::Error> {
        Ok(())
    }
    fn clear_free_wheel(&mut self, (): &()) -> Result<(), Self::Error> {
        self.post.free_wheel_enabled = false;
        Ok(())
    }
    fn capture_part_body(
        &mut self,
        (): &(),
        ordinal: usize,
    ) -> Result<Self::PartBody, Self::Error> {
        self.session
            .try_part_contact_spy_observation(self.backend, ordinal)
            .map_err(Self::Error::Part)
    }
    fn capture_inertia(&mut self, body: &Self::PartBody) -> Result<Self::Inertia, Self::Error> {
        Ok((
            *body,
            self.backend
                .try_capture_part_inertia(*body)
                .map_err(Self::Error::Inertia)?,
        ))
    }
    fn store_angular_drag(
        &mut self,
        inertia: &Self::Inertia,
        value: f32,
    ) -> Result<(), Self::Error> {
        self.backend
            .try_set_captured_angular_drag(inertia.0, &inertia.1, value)
            .map_err(Self::Error::Inertia)
    }
    fn capture_standard_materials(&mut self, (): &()) -> Result<Self::Materials, Self::Error> {
        Ok([
            Arc::clone(&self.materials.deck),
            Arc::clone(&self.materials.wheel),
            Arc::clone(&self.materials.truck),
        ])
    }
    fn store_material(
        &mut self,
        (): &(),
        ordinal: usize,
        materials: &Self::Materials,
        selected: SkateboardStandardMaterial,
    ) -> Result<(), Self::Error> {
        let index = match selected {
            SkateboardStandardMaterial::Deck => 0,
            SkateboardStandardMaterial::Wheel => 1,
            SkateboardStandardMaterial::Truck => 2,
        };
        self.session
            .try_set_part_material(self.backend, ordinal, Arc::clone(&materials[index]))
            .map_err(Self::Error::Material)
    }
    fn enable_top_volume(&mut self, (): &(), index: usize) -> Result<(), Self::Error> {
        self.collision
            .volume_flags
            .enable_top(index)
            .map_err(Self::Error::Volume)
    }
    fn deck_volume_count(&mut self, (): &()) -> Result<i32, Self::Error> {
        let count = self
            .collision
            .volume_flags
            .read()
            .map_err(Self::Error::Volume)?
            .deck_count();
        i32::try_from(count).map_err(|_| Self::Error::DeckCount)
    }
    fn enable_deck_volume(&mut self, (): &(), index: usize) -> Result<(), Self::Error> {
        self.collision
            .volume_flags
            .enable_deck(index)
            .map_err(Self::Error::Volume)
    }
}
