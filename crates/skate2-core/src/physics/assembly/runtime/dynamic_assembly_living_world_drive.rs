//! LivingWorld collision target loop on the shared body-only Part session.
//! Source82e506d8. No new pool, body handles, integration, or position teleport.
use super::{DynamicAssemblyBodyOnlyPartSession, DynamicAssemblySimulationBackend};
use crate::{
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::collision_drive::{CollisionDriveError, try_part_drive},
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    simulation_assembly_backend::SimulationAssemblyBackend,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
};
use skate2_formats::collision::AssemblyPartMaterial;

#[derive(Debug)]
pub(crate) enum LivingWorldPartDriveError<N, T> {
    ForeignBackend,
    TargetCount {
        parts: usize,
        targets: usize,
    },
    MissingBody(usize),
    Numeric {
        part: usize,
        source: CollisionDriveError<N, T>,
    },
    Body {
        part: usize,
        source: SimulationRigidBodyPoolError,
    },
}

impl<JF: Copy, JL: Copy>
    DynamicAssemblyBodyOnlyPartSession<SimulationAssemblyBackend<AssemblyPartMaterial, JF, JL>>
{
    /// Caller establishes Island membership. A detached body is the original
    /// no-Island early return; extra transforms are permitted by the source
    /// assertion. Per-Part effects survive later failures, with no rollback.
    pub(crate) fn try_update_living_world_collision_parts<
        N: ReckoningNumeric,
        T: ReckoningSinCosNumeric,
    >(
        &self,
        backend: &mut SimulationAssemblyBackend<AssemblyPartMaterial, JF, JL>,
        targets: &[[f32; 16]],
        dt: f32,
        ignore_gravity: bool,
        cvars: &InputCvarReferences,
        numeric: &mut N,
        trig: &mut T,
    ) -> Result<(), LivingWorldPartDriveError<N::Error, T::Error>> {
        let Some(authority) = self.attached_authority else {
            return Ok(());
        };
        if authority != backend.authority() {
            return Err(LivingWorldPartDriveError::ForeignBackend);
        }
        let count = self.transaction.parts.len();
        if targets.len() < count {
            return Err(LivingWorldPartDriveError::TargetCount {
                parts: count,
                targets: targets.len(),
            });
        }
        for (index, part) in self.transaction.parts.iter().enumerate() {
            let body = part
                .body
                .as_ref()
                .ok_or(LivingWorldPartDriveError::MissingBody(index))?;
            let metadata = self.transaction.template.metadata[index];
            let inverse = (metadata.has_inverse_body_transform != 0)
                .then_some(&metadata.inverse_body_transform);
            let old = backend.rigid_body_part_transform(body, inverse);
            let drive = try_part_drive(
                std::array::from_fn(|axis| std::array::from_fn(|lane| old[axis * 4 + lane])),
                std::array::from_fn(|axis| {
                    std::array::from_fn(|lane| targets[index][axis * 4 + lane])
                }),
                dt,
                cvars,
                numeric,
                trig,
            )
            .map_err(|source| LivingWorldPartDriveError::Numeric {
                part: index,
                source,
            })?;
            backend
                .try_publish_living_world_part_drive(
                    body.observation(),
                    std::array::from_fn(|i| drive.velocity[i]),
                    std::array::from_fn(|i| drive.omega[i]),
                    ignore_gravity,
                )
                .map_err(|source| LivingWorldPartDriveError::Body {
                    part: index,
                    source,
                })?;
        }
        Ok(())
    }
}
