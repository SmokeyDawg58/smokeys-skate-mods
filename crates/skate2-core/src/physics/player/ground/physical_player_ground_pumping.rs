//! Ground.Update82da0eac..1014, after actual `GroundSkateboard` and before control.
use crate::{
    attribute_runtime::AttributeRuntimeError,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    input_cvar_runtime::{
        InputCvarReferences,
        pumping::{PumpingCvars, PumpingDebugError},
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_pumping::update::PumpingUpdateError,
    reckoning::grounded_input::{GroundInputError, GroundInputRead, GroundScalar},
    reckoning_numeric::{self, ReckoningNumeric},
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum GroundPumpingError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignState,
    Pair(SimulationSkeletonJumpPairError),
    Input(GroundInputError),
    Numeric(N),
    Update(PumpingUpdateError<AttributeRuntimeError, N, PumpingDebugError>),
}
impl<N: std::fmt::Debug> std::fmt::Display for GroundPumpingError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(value) => write!(f, "Ground Pumping requires {value}"),
            Self::Poisoned => f.write_str("Ground Pumping owner lock poisoned"),
            Self::ForeignState => f.write_str("Ground Pumping requires same selected state/pair"),
            Self::Pair(error) => write!(f, "Ground Pumping pair: {error:?}"),
            Self::Input(error) => error.fmt(f),
            Self::Numeric(error) => write!(f, "Ground Pumping numeric: {error:?}"),
            Self::Update(error) => error.fmt(f),
        }
    }
}

impl PhysicalAssemblyWorld {
    #[expect(
        clippy::too_many_lines,
        reason = "ordered late reads across retained Ground, Reckoning and Skeleton owners"
    )]
    pub(crate) fn try_update_pair_ground_pumping<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        references: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), GroundPumpingError<N::Error>> {
        use GroundPumpingError as E;
        let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
        let owner = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            let ground = state.ground.as_ref().ok_or(E::Missing("Ground state"))?;
            if state.current != CurrentPhysicalState::Ground
                || !ground.bindings.board.same_as(board)
                || !ground.bindings.skeleton.same_as(skeleton)
                || !ground
                    .bindings
                    .owner
                    .upgrade()
                    .is_some_and(|v| Arc::ptr_eq(&v, &owner))
            {
                return Err(E::ForeignState);
            }
        }
        let (processed, up) = {
            let reckoning = owner.reckoning().lock().map_err(|_| E::Poisoned)?;
            (
                Arc::clone(reckoning.processed()),
                reckoning.reckoning().ok_or(E::Missing("Reckoning"))?.up(),
            )
        };
        // The second axis is current Processed.orientation+20, not the skater
        // body transform and not a pre-GroundSkateboard snapshot.
        let (orientation_z, deck_z) = {
            let input = processed.lock().map_err(|_| E::Poisoned)?;
            (
                input
                    .skateboard_orientation()
                    .ok_or(E::Missing("orientation+0"))?
                    .columns()[2],
                input
                    .deck_transform()
                    .ok_or(E::Missing("deck+40"))?
                    .columns()[2],
            )
        };
        let deck_dot = numeric.dot_vector(deck_z, up).map_err(E::Numeric)?;
        let orientation_dot = numeric.dot_vector(orientation_z, up).map_err(E::Numeric)?;
        let deck_angle = if deck_dot.abs() > orientation_dot.abs() {
            let wind = owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .reckoning()
                .ok_or(E::Missing("Reckoning"))?
                .ground_transform()
                .columns()[0];
            let first = reckoning_numeric::try_signed_angle(deck_z, up, wind, numeric)
                .map_err(E::Numeric)?;
            let first =
                reckoning_numeric::wrap_turns_to_radians(first * f32::from_bits(0x3e22_f983));
            let second = reckoning_numeric::try_signed_angle(orientation_z, up, wind, numeric)
                .map_err(E::Numeric)?;
            let second =
                reckoning_numeric::wrap_turns_to_radians(second * f32::from_bits(0x3e22_f983));
            (first - second).abs()
        } else {
            0.0
        };
        let normal = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .reckoning()
            .ok_or(E::Missing("Reckoning"))?
            .ground_normal();
        let dt = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .scalar(GroundScalar::TimeDelta)
            .map_err(E::Input)?;
        // GetCOMToDeckWorld82b3b468 returns Skeleton+2a30 = State1940 +
        // mComToDeckWorldAnim10f0, the existing live animation record. Outer
        // mComToDeckWorld3cc0 is a distinct field and must not substitute for it.
        let com = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.current_animation().record().com_to_deck()
            })
            .map_err(E::Pair)?;
        let position = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .deck_transform()
            .ok_or(E::Missing("deck position+70"))?
            .columns()[3];
        let mut pumping = owner.pumping().lock().map_err(|_| E::Poisoned)?;
        let mut attributes = owner.pumping_attributes().lock().map_err(|_| E::Poisoned)?;
        let mut debug = PumpingCvars {
            references,
            overlay: &self.debug_overlay,
        };
        pumping
            .try_update(
                position,
                normal,
                com,
                deck_angle,
                dt,
                &mut *attributes,
                numeric,
                &mut debug,
            )
            .map_err(E::Update)?;
        if std::env::var_os("SKATE2_TRACE_VERT").is_some()
            && let Some(summary) = pumping.trace_summary()
        {
            eprintln!(
                "vert pump: normal={:?} deck_angle={deck_angle} deck_y={} {summary}",
                normal.xyz, position.xyz[1]
            );
        }
        Ok(())
    }
}
