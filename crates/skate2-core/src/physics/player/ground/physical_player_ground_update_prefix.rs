//! PhysicsGround.Update82da0de8 through Reckoning call0e94; Handplants is next.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences, reckoning::ReckoningCvarAdapter},
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_reckoning_owner::{
        PhysicalPlayerReckoningPhase, PhysicalPlayerReckoningUpdateError,
    },
    reckoning::grounded_input::{GroundInputError, GroundInputRead},
    reckoning_numeric::ReckoningNumeric,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum GroundUpdatePrefixError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignState,
    Pair(SimulationSkeletonJumpPairError),
    Input(GroundInputError),
    Reckoning(PhysicalPlayerReckoningUpdateError<std::convert::Infallible, InputCvarError, N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for GroundUpdatePrefixError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "Ground.Update missing {v}"),
            Self::Poisoned => f.write_str("Ground.Update owner lock poisoned"),
            Self::ForeignState => {
                f.write_str("Ground.Update requires the actual selected Ground owner")
            }
            Self::Pair(e) => write!(f, "Ground.Update pair: {e:?}"),
            Self::Input(e) => e.fmt(f),
            Self::Reckoning(e) => e.fmt(f),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_ground_reckoning<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<PhysicalPlayerReckoningPhase, GroundUpdatePrefixError<N::Error>> {
        use GroundUpdatePrefixError as E;
        let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
        let owner = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let enable_tilt = {
            let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            if state.current != CurrentPhysicalState::Ground {
                return Err(E::ForeignState);
            }
            let ground = state.ground.as_mut().ok_or(E::Missing("Ground state"))?;
            if !ground.bindings.board.same_as(board)
                || !ground.bindings.skeleton.same_as(skeleton)
                || !ground
                    .bindings
                    .owner
                    .upgrade()
                    .is_some_and(|v| Arc::ptr_eq(&v, &owner))
            {
                return Err(E::ForeignState);
            }
            let fields = &mut ground.fields;
            if !fields.spawn_coordinates_set {
                let input = processed.lock().map_err(|_| E::Poisoned)?;
                if input.wheels().map_err(E::Input)? > 0 {
                    fields.valid_spawn_x = input.ground_spawn_coordinate(0).map_err(E::Input)?;
                    let z = input.ground_spawn_coordinate(2).map_err(E::Input)?;
                    fields.spawn_coordinates_set = true;
                    fields.valid_spawn_z = z;
                }
            }
            fields.anti_flip_torque = Some(
                crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            ); //0e50
            fields.request_push_stop = false; //0e54
            let time = fields.time_in_state; //0e58, before the remaining clears
            fields.anti_flip_nudge = false;
            fields.is_pinning = false;
            fields.time_to_skitch = -1.0;
            fields.grab_object = false;
            // Raw BLE4099 is NOT GT: NaN preserves the previous truck-tilt byte.
            time > f32::from_bits(0x3d4c_cccd)
        };
        if enable_tilt {
            pool.try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.enable_physics_truck_tilt();
            })
            .map_err(E::Pair)?;
        }
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_update_grounded_live(&processed, &mut ReckoningCvarAdapter(cvars), numeric)
            .map_err(E::Reckoning)
    }
}
