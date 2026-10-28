//! Same-manager source-qualified access to the existing body-only Part owner.
//! Native `Part::GetTransform` (0x826c3480) uses full packed body lanes;
//! Create input capture separately reads the retained cache, never this getter.

use super::DynamicAssemblyBodyOnlyPartSession;
use crate::{
    serialised_assembly_create::SerialisedAssemblyCreatePartInput,
    serialised_assembly_template::SerialisedAssemblyTemplate,
    simulation_assembly_backend::{
        SimulationAssemblyBackend,
        source::{SimulationAssemblySourceError, source_part_transform},
    },
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::fmt;

/// Read set of GrabSpline::Init 82d1a988: Part inertia and, when attached,
/// the actual body identity and GetBodyTransform (without the Part inverse).
#[derive(Clone, Copy, Debug)]
pub(crate) struct SourcePartGrabSnapshot {
    pub(crate) body: Option<crate::simulation_rigid_body_pool::SimulationRigidBodyObservation>,
    pub(crate) body_transform: Option<[f32; 16]>,
    pub(crate) velocity: Option<[f32; 4]>,
    pub(crate) inertia: crate::simulation_rigid_body_pool::SimulationInertia,
    pub(crate) inverse_tensor_w: f32,
}

#[cfg(test)]
#[path = "dynamic_assembly_source_parts_tests.rs"]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DynamicAssemblySourcePartError {
    ForeignBackend,
    NotAttached,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "native DMO getter parked before mesh publication")
    )]
    PartIndex {
        index: usize,
        count: usize,
    },
    PartCount {
        session: usize,
        template: usize,
    },
    UserDataCount {
        parts: usize,
        values: usize,
    },
    MissingBody {
        index: usize,
    },
    NotSourcePart {
        index: usize,
    },
    SourceMismatch {
        index: usize,
    },
    UnknownCache {
        index: usize,
    },
    Allocation {
        parts: usize,
    },
    Source(SimulationAssemblySourceError),
    Body(crate::simulation_rigid_body_pool::SimulationRigidBodyPoolError),
    MissingPackedInertia {
        index: usize,
    },
}
impl fmt::Display for DynamicAssemblySourcePartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(f, "source Part: {error}"),
            other => write!(f, "source Part access: {other:?}"),
        }
    }
}
impl std::error::Error for DynamicAssemblySourcePartError {}

impl<JF: Copy, JL: Copy>
    DynamicAssemblyBodyOnlyPartSession<SimulationAssemblyBackend<AssemblyPartMaterial, JF, JL>>
{
    /// Physics.Body.SetVelocity82de7c38: when attached, activate a frozen
    /// Part and clear cooldown BEFORE storing velocity. An unattached Part
    /// still receives velocity if its body exists; missing body is a no-op.
    pub(crate) fn try_set_body_part_velocity(
        &self, backend: &mut SimulationAssemblyBackend<AssemblyPartMaterial, JF, JL>,
        index: usize, velocity: [f32; 3],
    ) -> Result<(), DynamicAssemblySourcePartError> {
        use crate::dynamic_assembly_session::part_activation::DynamicAssemblyActivationBackend;
        if self.attached_authority.is_some_and(|authority| authority != backend.authority()) {
            return Err(DynamicAssemblySourcePartError::ForeignBackend);
        }
        let part = self.transaction.parts.get(index).ok_or(DynamicAssemblySourcePartError::PartIndex {
            index, count: self.transaction.parts.len(),
        })?;
        if self.attached_authority.is_some() {
            let body = part.body.as_ref().ok_or(DynamicAssemblySourcePartError::MissingBody { index })?;
            if backend.try_activation_state(body).map_err(DynamicAssemblySourcePartError::Body)? & 7 == 2 {
                backend.try_activate_frozen(body).map_err(DynamicAssemblySourcePartError::Body)?;
            }
            backend.try_clear_cooldown(body).map_err(DynamicAssemblySourcePartError::Body)?;
        }
        if let Some(body) = part.body.as_ref() {
            backend.try_set_part_linear_velocity(body.observation(), velocity)
                .map_err(DynamicAssemblySourcePartError::Body)?;
        }
        Ok(())
    }

    /// OnEntityCollision82e95888 reads this actual Part's RigidBody velocity
    /// and omega. A detached Part with no body is an error, never entity motion.
    /// W is padding for the host projection; the consumer uses dotXYZ only.
    pub(crate) fn try_live_part_velocities_host(&self,
        backend: &SimulationAssemblyBackend<AssemblyPartMaterial, JF, JL>, index: usize,
    ) -> Result<([f32; 4], [f32; 4]), DynamicAssemblySourcePartError> {
        if self.attached_authority.is_some_and(|authority| authority != backend.authority()) {
            return Err(DynamicAssemblySourcePartError::ForeignBackend);
        }
        let part = self.transaction.parts.get(index).ok_or(DynamicAssemblySourcePartError::PartIndex {
            index, count: self.transaction.parts.len(),
        })?;
        let body = part.body.ok_or(DynamicAssemblySourcePartError::MissingBody { index })?;
        let pool = backend.collision_body_pool();
        let observation = pool.try_observe(body).map_err(DynamicAssemblySourcePartError::Body)?;
        let velocity = pool.try_contact_spy_linear_velocity(observation).map_err(DynamicAssemblySourcePartError::Body)?;
        let omega = pool.try_phys_out_angular_velocity(observation).map_err(DynamicAssemblySourcePartError::Body)?;
        Ok(([velocity[0],velocity[1],velocity[2],0.], [omega[0],omega[1],omega[2],0.]))
    }
    pub(crate) fn try_grab_part_snapshot(
        &self,
        backend: &SimulationAssemblyBackend<AssemblyPartMaterial, JF, JL>,
        index: usize,
    ) -> Result<SourcePartGrabSnapshot, DynamicAssemblySourcePartError> {
        if self
            .attached_authority
            .is_some_and(|authority| authority != backend.authority())
        {
            return Err(DynamicAssemblySourcePartError::ForeignBackend);
        }
        let count = self.transaction.parts.len();
        let part = self
            .transaction
            .parts
            .get(index)
            .ok_or(DynamicAssemblySourcePartError::PartIndex { index, count })?;
        let payload = &self.transaction.template.payloads[index].inertia;
        let Some(source) = payload.as_source_routed() else {
            // Programmatically constructed LW Parts retain the SAME Definition
            // inertia cell. Read Part inertia, not the body's optional inertia
            // (a static Add may clear that pointer). Init82d1a988 copies all four
            // inverse-tensor lanes. Do not invent W for XYZ-only payloads.
            let packed = payload
                .try_read_packed()
                .map_err(DynamicAssemblySourcePartError::Body)?
                .ok_or(DynamicAssemblySourcePartError::MissingPackedInertia { index })?;
            let inertia = payload
                .try_read()
                .map_err(DynamicAssemblySourcePartError::Body)?;
            let (body, body_transform, velocity) = match part.body {
                Some(handle) => {
                    let pool = backend.collision_body_pool();
                    let observation = pool
                        .try_observe(handle)
                        .map_err(DynamicAssemblySourcePartError::Body)?;
                    let pose = pool
                        .try_rigid_body_pose(handle)
                        .map_err(DynamicAssemblySourcePartError::Body)?;
                    let motion = pool
                        .try_serialised_assembly_snapshot(observation)
                        .map_err(DynamicAssemblySourcePartError::Body)?;
                    // Existing authored-body host projection: no Xenon link
                    // pointer in the semantic velocity W lane.
                    (
                        Some(observation),
                        Some(pose),
                        Some([
                            motion.velocity[0],
                            motion.velocity[1],
                            motion.velocity[2],
                            0.,
                        ]),
                    )
                }
                None => (None, None, None),
            };
            return Ok(SourcePartGrabSnapshot {
                body,
                body_transform,
                velocity,
                inertia,
                inverse_tensor_w: packed[3],
            });
        };
        let (body, body_transform) = if let Some(body) = part.body {
            (
                Some(
                    backend
                        .try_source_body_observation(body, source.source_owner())
                        .map_err(DynamicAssemblySourcePartError::Source)?,
                ),
                Some(
                    backend
                        .try_source_body_pose(body, source.source_owner())
                        .map_err(DynamicAssemblySourcePartError::Source)?,
                ),
            )
        } else {
            (None, None)
        };
        let velocity = if let Some(body) = body {
            let context = backend
                .try_source_context(source.source_owner())
                .map_err(DynamicAssemblySourcePartError::Source)?;
            Some(
                crate::serialised_assembly_create::try_source_body_motion(
                    source.source_owner(),
                    context.body_pool,
                    context.inertia_owner,
                    context.image_domain,
                    body,
                )
                .map_err(|e| {
                    DynamicAssemblySourcePartError::Source(
                        SimulationAssemblySourceError::Transform(e),
                    )
                })?
                .velocity_and_right,
            )
        } else {
            None
        };
        let inertia = source.runtime_inertia();
        Ok(SourcePartGrabSnapshot {
            body,
            body_transform,
            velocity,
            inertia: **inertia.semantic(),
            inverse_tensor_w: f32::from_be_bytes(
                inertia.source().bytes()[12..16]
                    .try_into()
                    .expect("validated source inertia"),
            ),
        })
    }

    /// Attached full source-body pose then optional inverse-body composition;
    /// detached copies the actual Part cache. No body capability escapes.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "native DMO getter parked before mesh publication")
    )]
    pub(crate) fn try_native_part_transform(
        &self,
        backend: &SimulationAssemblyBackend<AssemblyPartMaterial, JF, JL>,
        index: usize,
    ) -> Result<[f32; 16], DynamicAssemblySourcePartError> {
        if self
            .attached_authority
            .is_some_and(|authority| authority != backend.authority())
        {
            return Err(DynamicAssemblySourcePartError::ForeignBackend);
        }
        let count = self.transaction.parts.len();
        let part = self
            .transaction
            .parts
            .get(index)
            .ok_or(DynamicAssemblySourcePartError::PartIndex { index, count })?;
        let Some(body) = part.body else {
            return part
                .part_transform
                .known()
                .copied()
                .ok_or(DynamicAssemblySourcePartError::UnknownCache { index });
        };
        let source = self.transaction.template.payloads[index]
            .inertia
            .as_source_routed()
            .ok_or(DynamicAssemblySourcePartError::NotSourcePart { index })?;
        let body = backend
            .try_source_body_pose(body, source.source_owner())
            .map_err(DynamicAssemblySourcePartError::Source)?;
        let metadata = self.transaction.template.metadata[index];
        Ok(source_part_transform(
            body,
            (metadata.has_inverse_body_transform != 0).then_some(&metadata.inverse_body_transform),
        ))
    }

    /// Owner-qualified Create/refresh inputs in Definition order. This captures
    /// the actual raw cache, material and caller-retained user words; the
    /// existing Create provider later produces attached body transforms.
    pub(crate) fn try_source_create_parts(
        &self,
        backend: &SimulationAssemblyBackend<AssemblyPartMaterial, JF, JL>,
        expected_template: &SerialisedAssemblyTemplate,
        user_data: &[u32],
    ) -> Result<Vec<SerialisedAssemblyCreatePartInput>, DynamicAssemblySourcePartError> {
        let authority = self
            .attached_authority
            .ok_or(DynamicAssemblySourcePartError::NotAttached)?;
        if authority != backend.authority() {
            return Err(DynamicAssemblySourcePartError::ForeignBackend);
        }
        let count = self.transaction.parts.len();
        if expected_template.parts().len() != count {
            return Err(DynamicAssemblySourcePartError::PartCount {
                session: count,
                template: expected_template.parts().len(),
            });
        }
        if user_data.len() != count {
            return Err(DynamicAssemblySourcePartError::UserDataCount {
                parts: count,
                values: user_data.len(),
            });
        }
        let mut inputs = Vec::new();
        inputs
            .try_reserve_exact(count)
            .map_err(|_| DynamicAssemblySourcePartError::Allocation { parts: count })?;
        for (index, (part, &user_data)) in self.transaction.parts.iter().zip(user_data).enumerate()
        {
            let payload = &self.transaction.template.payloads[index];
            let source = payload
                .inertia
                .as_source_routed()
                .ok_or(DynamicAssemblySourcePartError::NotSourcePart { index })?;
            if !source
                .runtime_inertia()
                .source()
                .is_same_backing_inertia(&expected_template.parts()[index].inertia_source)
            {
                return Err(DynamicAssemblySourcePartError::SourceMismatch { index });
            }
            let body = part
                .body
                .ok_or(DynamicAssemblySourcePartError::MissingBody { index })?;
            let observation = backend
                .try_source_body_observation(body, expected_template.owner())
                .map_err(DynamicAssemblySourcePartError::Source)?;
            inputs.push(SerialisedAssemblyCreatePartInput {
                body: observation,
                part_transform: *part
                    .part_transform
                    .known()
                    .ok_or(DynamicAssemblySourcePartError::UnknownCache { index })?,
                material: payload.material,
                user_data,
            });
        }
        Ok(inputs)
    }
}
