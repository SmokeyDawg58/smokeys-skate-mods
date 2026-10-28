//! Air.Update82d9f6f0 live gate and `AddCOMCorrection82d84290` on actual owners.
use crate::{
    attribute_runtime::AttributeRuntimeError,
    dynamic_assembly_session::{
        part_transform::DynamicAssemblyPartDisplacementError,
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
            SimulationSkeletonJumpPairError,
        },
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::ReckoningNumeric,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

pub(crate) const ENABLE: u64 = 0xfd9c_e5e1_f36c_cf2e;
#[derive(Debug)]
pub(crate) enum AirComCorrectionError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignState,
    Attribute(AttributeRuntimeError),
    Pair(SimulationSkeletonJumpPairError),
    Pool(SimulationSkaterPoolError),
    Apply(DynamicAssemblyPartDisplacementError<N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for AirComCorrectionError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("Air COM correction: Poisoned"),
            Self::ForeignState => f.write_str("Air COM correction: ForeignState"),
            Self::Missing(error) => write!(f, "Air COM correction Missing: {error:?}"),
            Self::Attribute(error) => write!(f, "Air COM correction Attribute: {error:?}"),
            Self::Pair(error) => write!(f, "Air COM correction Pair: {error:?}"),
            Self::Pool(error) => write!(f, "Air COM correction Pool: {error:?}"),
            Self::Apply(error) => write!(f, "Air COM correction Apply: {error:?}"),
        }
    }
}

pub(crate) fn decode_enable(bytes: Option<&[u8]>) -> Result<bool, AttributeRuntimeError> {
    // Generated scalar getter at6f8..700 uses DefaultDataArea(1), not layout.
    let bytes = bytes.unwrap_or(&[0]);
    if bytes.len() != 1 {
        return Err(AttributeRuntimeError::PayloadSize {
            key: ENABLE,
            expected: 1,
            actual: bytes.len(),
        });
    }
    Ok(bytes[0] != 0)
}

trait ComCorrectionSource {
    type Error;
    fn delta(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn time_delta(&mut self) -> Result<f32, Self::Error>;
    fn apply(&mut self, correction: SkeletonRootVector) -> Result<(), Self::Error>;
}
fn try_correction<S: ComCorrectionSource>(source: &mut S) -> Result<(), S::Error> {
    // GetCOMToDeckWorldDelta82b3b478 copies the live AnimRecord+2a20 full vector.
    let negated = source
        .delta()?
        .map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000));
    // 82d842f0 is scalar fdivs, NOT vector reciprocal estimate/refinement.
    let inverse_time = 1.0 / source.time_delta()?;
    let correction = negated.map(|v| v * inverse_time);
    source.apply(correction)
}

struct LiveSource<'a, N> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    skeleton: &'a SimulationSkaterAssemblyHandle,
    owner: Arc<PhysicalPlayerAssemblyOwner>,
    numeric: &'a mut N,
}
impl<N: ReckoningNumeric> ComCorrectionSource for LiveSource<'_, N> {
    type Error = AirComCorrectionError<N::Error>;
    fn delta(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        use AirComCorrectionError as E;
        self.world
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(
                &self.world.backend,
                self.board,
                self.skeleton,
                |normal, _| normal.current_animation().record().com_to_deck_delta(),
            )
            .map_err(E::Pair)
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        use AirComCorrectionError as E;
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let processed = processed.lock().map_err(|_| E::Poisoned)?;
        processed
            .physics_state_fields()
            .time_delta
            .ok_or(E::Missing("Processed.dt93c"))
    }
    fn apply(&mut self, correction: SkeletonRootVector) -> Result<(), Self::Error> {
        use AirComCorrectionError as E;
        let board = self
            .world
            .skaters
            .as_ref()
            .ok_or(E::Missing("skater pool"))?
            .session(self.board)
            .map_err(E::Pool)?;
        // ApplyImpulse82b36988 is the same fixed-time, inverse-mass, old-force
        // graph as the existing body displacement service. It affects Part6
        // force XYZ then cooldown, not all-Part velocity or the force queue.
        // Only XYZ leaves the service; scratch W is never retained as known.
        let scratch = [
            correction.xyz[0],
            correction.xyz[1],
            correction.xyz[2],
            correction.w.unwrap_or(0.0),
        ];
        board
            .try_apply_part_displacement(&mut self.world.backend, 6, scratch, self.numeric)
            .map_err(E::Apply)
    }
}

impl PhysicalAssemblyWorld {
    /// Caller releases its Air/Reckoning guards after `UpdateAirStates`, then calls
    /// this entry before sampling use-COM-velocity. False is the actual live gate.
    pub(crate) fn try_add_pair_air_com_correction<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        numeric: &mut N,
    ) -> Result<bool, AirComCorrectionError<N::Error>> {
        use AirComCorrectionError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            if state.current != CurrentPhysicalState::Air {
                return Err(E::ForeignState);
            }
            let air = state.air.as_ref().ok_or(E::Missing("Air state"))?;
            if !air.bindings.board.same_as(board)
                || !air.bindings.skeleton.same_as(skeleton)
                || !air
                    .bindings
                    .owner
                    .upgrade()
                    .is_some_and(|v| Arc::ptr_eq(&v, &owner))
            {
                return Err(E::ForeignState);
            }
            if !air.try_com_correction_enabled().map_err(E::Attribute)? {
                return Ok(false);
            }
        }
        try_correction(&mut LiveSource {
            world: self,
            board,
            skeleton,
            owner,
            numeric,
        })?;
        Ok(true)
    }
}

#[cfg(test)]
#[path = "physical_player_air_com_correction_tests.rs"]
mod tests;
