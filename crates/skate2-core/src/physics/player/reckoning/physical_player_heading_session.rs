//! Consecutive Update1 heading call3aa8 and publication3ab4.
#[path = "physical_player_heading_runtime.rs"]
pub(crate) mod runtime;
use crate::{
    attribute_runtime::AttributeRuntimeError,
    debug_overlay::DebugOverlayError,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::ReckoningNumeric,
};
use std::{fmt, sync::Arc};
#[derive(Debug)]
pub(crate) enum HeadingError<N> {
    Missing(&'static str),
    Poisoned,
    Pair(SimulationSkeletonJumpPairError),
    Numeric(N),
    Attribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    InvalidAngle,
}
impl<N: fmt::Debug> fmt::Display for HeadingError<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "heading missing {v}"),
            Self::Poisoned => f.write_str("heading owner lock poisoned"),
            Self::Pair(e) => write!(f, "heading pair: {e:?}"),
            Self::Numeric(e) => write!(f, "heading numeric: {e:?}"),
            Self::Attribute(e) => e.fmt(f),
            Self::Cvar(e) => e.fmt(f),
            Self::Overlay(e) => e.fmt(f),
            Self::InvalidAngle => f.write_str("source heading angle nonnegative assertion failed"),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_heading<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), HeadingError<N::Error>> {
        use HeadingError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        // vrefp2f8 precedes the actual ground-normal load328.
        let reciprocal = numeric
            .reciprocal_estimate(f32::from_bits(0x3fc9_0fdb))
            .map_err(E::Numeric)?;
        let normal_y = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .ground_normal()
            .ok_or(E::Missing("ground normal+1d0"))?
            .value
            .xyz[1];
        let angle = runtime::normalized_angle(normal_y, reciprocal, numeric).map_err(E::Numeric)?;
        if angle
            .partial_cmp(&0.0)
            .is_none_or(std::cmp::Ordering::is_lt)
        {
            return Err(E::InvalidAngle);
        }
        let angle = if angle > 1.2 { 0.0 } else { angle };
        let angular_factor = owner
            .heading_attributes()
            .graph(0x190, angle)
            .map_err(E::Attribute)?;
        // Source reads ZSpeed only after the first graph result.
        let speed = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .z_speed()
            .ok_or(E::Missing("ZSpeed+944"))?
            .value;
        let maximum = owner
            .heading_attributes()
            .speed_limit()
            .map_err(E::Attribute)?;
        let clamped = runtime::clamp_speed(speed, maximum);
        let denominator = owner
            .heading_attributes()
            .speed_limit()
            .map_err(E::Attribute)?;
        let speed_factor = owner
            .heading_attributes()
            .graph(0x150, clamped / denominator)
            .map_err(E::Attribute)?;
        if cvars.try_draw_vert_turning_input().map_err(E::Cvar)? {
            self.debug_overlay.require().map_err(E::Overlay)?;
        }
        processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields_mut()
            .heading_adjust_factor = Some(speed_factor * angular_factor);
        Ok(())
    }
}
