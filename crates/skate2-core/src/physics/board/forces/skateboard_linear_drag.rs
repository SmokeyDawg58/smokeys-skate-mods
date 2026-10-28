//! Actual all-Part `Skateboard::SetLinearDrag` 0x82b23a98 -> 0x82de7de0.
use crate::{
    dynamic_assembly_session::part_contact_spy::DynamicAssemblyPartContactSpyError,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_assembly::{SkateboardAssemblySession, SkateboardSimulationBackend},
};

#[derive(Debug)]
pub(crate) enum SkateboardLinearDragError {
    Part {
        index: usize,
        source: DynamicAssemblyPartContactSpyError,
    },
    Inertia {
        first: usize,
        source: SimulationRigidBodyPoolError,
    },
}
impl std::fmt::Display for SkateboardLinearDragError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Part { index, source } => write!(f, "linear drag Part {index}: {source}"),
            Self::Inertia { first, source } => {
                write!(f, "linear drag inertia group starting {first}: {source}")
            }
        }
    }
}

pub(crate) fn try_set_skateboard_linear_drag(
    board: &SkateboardAssemblySession,
    backend: &mut SkateboardSimulationBackend,
    drag: f32,
) -> Result<(), SkateboardLinearDragError> {
    let count = board.template().part_payloads().len();
    let mut part = 0;
    let observe = |backend: &SkateboardSimulationBackend, index| {
        board
            .try_part_contact_spy_observation(backend, index)
            .map_err(|source| SkateboardLinearDragError::Part { index, source })
    };
    if count >= 4 {
        let scaled = drag * f32::from_bits(0x426f_ffff);
        while count - part >= 4 {
            let bodies = [
                observe(backend, part)?,
                observe(backend, part + 1)?,
                observe(backend, part + 2)?,
                observe(backend, part + 3)?,
            ];
            backend
                .try_set_part_group_linear_drag(&bodies, scaled)
                .map_err(|source| SkateboardLinearDragError::Inertia {
                    first: part,
                    source,
                })?;
            part += 4;
        }
    }
    if part < count {
        let scaled = drag * f32::from_bits(0x426f_ffff);
        while part < count {
            let body = observe(backend, part)?;
            backend
                .try_set_part_group_linear_drag(&[body], scaled)
                .map_err(|source| SkateboardLinearDragError::Inertia {
                    first: part,
                    source,
                })?;
            part += 1;
        }
    }
    Ok(())
}
