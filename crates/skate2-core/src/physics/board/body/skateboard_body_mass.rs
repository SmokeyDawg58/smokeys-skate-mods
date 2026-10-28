//! `SkateboardBody::GetMass`, 0x82b243e8..0x82b244b4. Reads the cached
//! inverse mass of each actual attached Part body, then fdivs/fadds in ascending
//! Part order. Neither construction tuning nor deck-only mass substitutes here.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "actual board mass awaits native toolkit scheduling"
    )
)]
use crate::{
    dynamic_assembly_session::part_contact_spy::DynamicAssemblyPartContactSpyError,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_assembly::{SkateboardAssemblySession, SkateboardSimulationBackend},
};

#[derive(Debug)]
pub(crate) enum SkateboardBodyMassError {
    Part {
        part: usize,
        source: DynamicAssemblyPartContactSpyError,
    },
    Body {
        part: usize,
        source: SimulationRigidBodyPoolError,
    },
}
impl std::fmt::Display for SkateboardBodyMassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Part { part, source } => write!(f, "board mass Part {part}: {source}"),
            Self::Body { part, source } => write!(f, "board mass body {part}: {source}"),
        }
    }
}

/// The source unrolls four Parts then handles the remainder, retaining the
/// same scalar accumulator and ascending single-precision arithmetic order.
/// Zero, signed zero, negative values and NaNs receive no replacement policy.
#[expect(
    clippy::assign_op_pattern,
    reason = "source operand order adds mass before the accumulator at group entry and tail"
)]
pub(crate) fn try_skateboard_body_mass(
    board: &SkateboardAssemblySession,
    backend: &SkateboardSimulationBackend,
) -> Result<f32, SkateboardBodyMassError> {
    let count = board.template().part_payloads().len();
    let read_inverse = |part| {
        let observation = board
            .try_part_contact_spy_observation(backend, part)
            .map_err(|source| SkateboardBodyMassError::Part { part, source })?;
        backend
            .collision_body_pool()
            .try_contact_response_cached_inverse_mass(observation)
            .map_err(|source| SkateboardBodyMassError::Body { part, source })
    };
    let mut total = 0.0_f32;
    let mut part = 0;
    while count - part >= 4 {
        let inverse = [
            read_inverse(part)?,
            read_inverse(part + 1)?,
            read_inverse(part + 2)?,
            read_inverse(part + 3)?,
        ];
        let masses = inverse.map(|value| 1.0_f32 / value);
        total = masses[0] + total;
        total += masses[1];
        total += masses[2];
        total += masses[3];
        part += 4;
    }
    while part < count {
        total = (1.0_f32 / read_inverse(part)?) + total;
        part += 1;
    }
    Ok(total)
}
