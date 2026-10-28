//! Live OffboardTrajectorySelector adapter: existing owner, predictor and queries.
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_trajectory_owners::{
        OffboardLaunchInfo,
        offboard_runtime::{self, Frame, Services},
    },
    reckoning_numeric::ReckoningNumeric,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum OffboardTrajectoryServiceError {
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    Pair(SimulationSkeletonJumpPairError),
    Missing(&'static str),
    Poisoned,
}
pub(crate) type OffboardTrajectoryRuntimeError<N> =
    offboard_runtime::Error<OffboardTrajectoryServiceError, N>;

struct Live<'a> {
    cvars: &'a InputCvarReferences,
    overlay: &'a DebugOverlaySlot,
    processed: Arc<Mutex<SkeletonProcessedContact>>,
}
impl<N: ReckoningNumeric> Services<N> for Live<'_> {
    type Error = OffboardTrajectoryServiceError;
    fn maximum_time(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_ob_air_trajectory_max_time()
            .map_err(Self::Error::Cvar)
    }
    fn maximum_error_end(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_ob_air_trajectory_max_error_end()
            .map_err(Self::Error::Cvar)
    }
    fn maximum_error_start(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_ob_air_trajectory_max_error_start()
            .map_err(Self::Error::Cvar)
    }
    fn force_failure(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_ob_debug_trajectory_fail()
            .map_err(Self::Error::Cvar)
    }
    fn debug_draw_enabled(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_ob_air_debug_draw()
            .map_err(Self::Error::Cvar)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
    fn processed_up_vector(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .prior_output()
            .reckoning_up()
            .map(|value| value.value)
            .ok_or(Self::Error::Missing("Processed mUpVector+220"))
    }
    fn processed_dt(&mut self) -> Result<f32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .physics_state_fields()
            .time_delta
            .ok_or(Self::Error::Missing("Processed DT"))
    }
    fn world_gravity(&mut self) -> SkeletonRootVector {
        // Math::VECTOR3_GRAVITY initializer82f76360, separate from Vault gravity.
        SkeletonRootVector::known([0.0, f32::from_bits(0xc11c_cccd), 0.0, 0.0])
    }
}

impl PhysicalAssemblyWorld {
    #[expect(
        clippy::too_many_arguments,
        reason = "existing physical pair and actual collision/numeric services"
    )]
    pub(crate) fn try_launch_pair_offboard_trajectory<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        launch: OffboardLaunchInfo,
        ground: &StaticGroundPool,
        numeric: &mut N,
    ) -> Result<(), OffboardTrajectoryRuntimeError<N::Error>> {
        use offboard_runtime::Error as E;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(owner, actual)
            })
            .map_err(|error| E::Source(OffboardTrajectoryServiceError::Pair(error)))?;
        if !same {
            return Err(E::ForeignOwner);
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let predictor = self
            .predictor
            .value
            .as_ref()
            .ok_or(E::Missing("World predictor"))?;
        let mut services = Live {
            cvars: &owner.inputs().cvars,
            overlay: &self.debug_overlay,
            processed,
        };
        offboard_runtime::launch(
            owner,
            launch,
            &mut services,
            predictor,
            self.world_line_queries
                .as_mut()
                .ok_or(E::Missing("World queries"))?,
            ground,
            self.dmos.as_ref().ok_or(E::Missing("DMO collection"))?,
            numeric,
        )
    }

    pub(crate) fn try_pair_offboard_trajectory_frame<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        frame: i32,
        height_adjust: f32,
        numeric: &mut N,
    ) -> Result<Frame, OffboardTrajectoryRuntimeError<N::Error>> {
        use offboard_runtime::Error as E;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut services = Live {
            cvars: &owner.inputs().cvars,
            overlay: &self.debug_overlay,
            processed: Arc::clone(&processed),
        };
        let mut trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
        let selector = trajectories
            .offboard
            .as_mut()
            .ok_or(E::Missing("offboard selector"))?;
        if !selector.bindings.matches(owner, &processed) {
            return Err(E::ForeignOwner);
        }
        selector.frame(frame, height_adjust, &mut services, numeric)
    }
}
