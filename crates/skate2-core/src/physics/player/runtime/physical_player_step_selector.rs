//! World-owned StepSelector827b11ac..120c, Launch82d82618/Update82d82490.
use super::SimulationAssemblyConstraintSession;
use crate::{
    actor_animation_construction::AnimationStepInput,
    actor_construction::animation_mount::MountedActorAnimation,
    debug_overlay::DebugOverlaySlot,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    simulation_assembly_backend::constraint_session::{
        grind_world::PhysicalGrindServices,
        world_line_query::{
            SimulationWorldLineLeaseError, SimulationWorldLineQueries,
            step::{SimulationWorldStepBatch, WorldStepError},
        },
    },
    simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    static_ground_pool::StaticGroundPool,
    walking_step_query::StepTrajectoryResult,
    walking_step_results::{StepResultServices, calculate_results},
    walking_step_selection::{
        NativeStepTrajectoryDistance, StepSelectionError, StepSelectionResult, get_step_position,
    },
    walking_step_tuning::{WalkingTuning, WalkingTuningError},
};
use std::sync::{Arc, Mutex};

type SelectionError =
    StepSelectionError<ReckoningHostNumericError, WalkingTuningError, ReckoningHostNumericError>;
#[derive(Debug)]
pub(crate) enum StepSelectorError {
    AlreadyConstructed,
    Missing(&'static str),
    Poisoned,
    ForeignManager,
    QueueFull,
    DuplicateFuture,
    AlreadyReady,
    InputRange,
    Lease(SimulationWorldLineLeaseError),
    World(WorldStepError<ReckoningHostNumericError, WalkingTuningError>),
    Selection(SelectionError),
    Cvar(InputCvarError),
}
impl std::fmt::Display for StepSelectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StepSelector: {self:?}")
    }
}
#[derive(Default)]
pub(crate) struct StepSelectorSlot {
    attempted: bool,
    pub(crate) value: Option<Arc<PhysicalStepSelector>>,
}
pub(crate) struct PhysicalStepSelector {
    manager: SimulationManagerContactAuthority,
    cvars: InputCvarReferences,
    queue: Mutex<StepQueue>,
}
struct StepQueue {
    slots: [Option<PendingStep>; 18],
    len: usize,
}
struct PendingStep {
    animation: Arc<Mutex<MountedActorAnimation>>,
    first: usize,
    count: usize,
    batch: SimulationWorldStepBatch,
}

impl<M, C> SimulationAssemblyConstraintSession<M, C> {
    /// Source distinct allocation after Predictor; null stays null.
    pub(crate) fn try_initialize_step_selector(
        &mut self,
        allocated: bool,
        cvars: &InputCvarReferences,
    ) -> Result<(), StepSelectorError> {
        if self.step_selector.attempted {
            return Err(StepSelectorError::AlreadyConstructed);
        }
        self.step_selector.attempted = true;
        if allocated {
            self.step_selector.value = Some(Arc::new(PhysicalStepSelector {
                manager: self.contacts.assembly_pool_authority(),
                cvars: cvars.clone(),
                queue: Mutex::new(StepQueue {
                    slots: std::array::from_fn(|_| None),
                    len: 0,
                }),
            }));
        }
        Ok(())
    }
    pub(crate) fn step_selector_owner(&self) -> Option<&Arc<PhysicalStepSelector>> {
        self.step_selector.value.as_ref()
    }
    pub(crate) fn try_launch_step_selector(
        &mut self,
        animation: &Arc<Mutex<MountedActorAnimation>>,
        first: usize,
        count: usize,
        world_only: bool,
        ground: &StaticGroundPool,
    ) -> Result<(), StepSelectorError> {
        let owner = self
            .step_selector
            .value
            .as_ref()
            .ok_or(StepSelectorError::Missing("World.StepSelector"))?;
        let queries = self
            .world_line_queries
            .as_mut()
            .ok_or(StepSelectorError::Missing("World query owner"))?;
        let dmos = self
            .dmos
            .as_ref()
            .ok_or(StepSelectorError::Missing("World DMO pool"))?;
        owner.launch(
            self.contacts.assembly_pool_authority(),
            animation,
            first,
            count,
            world_only,
            queries,
            ground,
            dmos,
            &self.debug_overlay,
        )
    }
    pub(crate) fn try_update_step_selector(&mut self) -> Result<(), StepSelectorError> {
        let owner = self
            .step_selector
            .value
            .as_ref()
            .ok_or(StepSelectorError::Missing("World.StepSelector"))?;
        let queries = self
            .world_line_queries
            .as_mut()
            .ok_or(StepSelectorError::Missing("World query owner"))?;
        owner.update(
            self.contacts.assembly_pool_authority(),
            queries,
            &self.grind_services,
            &self.debug_overlay,
        )
    }
}

struct ResultServices<'a> {
    grind: &'a PhysicalGrindServices,
    cvars: &'a InputCvarReferences,
    tuning: WalkingTuning<'a>,
    numeric: HostReckoningNumeric,
}
impl StepResultServices for ResultServices<'_> {
    type Error = StepSelectorError;
    fn get_step_position(
        &mut self,
        input: &AnimationStepInput,
        candidate: &StepTrajectoryResult,
        result: &mut StepSelectionResult,
    ) -> Result<bool, Self::Error> {
        get_step_position(
            input,
            candidate,
            result,
            self.grind,
            self.cvars,
            &mut self.numeric,
            &mut self.tuning,
            &mut NativeStepTrajectoryDistance,
        )
        .map_err(StepSelectorError::Selection)
    }
    fn death_jump_distance(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_death_jump_distance()
            .map_err(StepSelectorError::Cvar)
    }
    fn maximum_time(&mut self) -> Result<f32, Self::Error> {
        self.cvars
            .try_step_maximum_time()
            .map_err(StepSelectorError::Cvar)
    }
}
impl PhysicalStepSelector {
    pub(crate) fn launch(
        &self,
        manager: SimulationManagerContactAuthority,
        animation: &Arc<Mutex<MountedActorAnimation>>,
        first: usize,
        count: usize,
        world_only: bool,
        queries: &mut SimulationWorldLineQueries,
        ground: &StaticGroundPool,
        dmos: &SimulationDmoAssemblyPool,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), StepSelectorError> {
        use StepSelectorError as E;
        if self.manager != manager {
            return Err(E::ForeignManager);
        }
        let mut queue = self.queue.lock().map_err(|_| E::Poisoned)?;
        if queue.len == 18 {
            return Err(E::QueueFull);
        }
        if queue.slots[..queue.len]
            .iter()
            .flatten()
            .any(|p| Arc::ptr_eq(&p.animation, animation) && p.first == first)
        {
            return Err(E::DuplicateFuture);
        }
        let mut anim = animation.lock().map_err(|_| E::Poisoned)?;
        let (inputs, futures) = anim
            .step_storage_mut()
            .ok_or(E::Missing("SkaterAnim step storage"))?;
        let end = first
            .checked_add(count)
            .filter(|end| *end <= inputs.len())
            .ok_or(E::InputRange)?;
        let batch = queries
            .try_acquire_step(dmos, "StepSelector", if world_only { 1 } else { 7 })
            .map_err(E::Lease)?;
        queries.try_begin_step(&batch).map_err(E::Lease)?;
        let _accepted = queries
            .try_add_step::<ReckoningHostNumericError, WalkingTuningError>(
                &batch,
                &inputs[first..end],
            )
            .map_err(E::World)?;
        let mut tuning = WalkingTuning {
            cvars: &self.cvars,
            overlay,
        };
        queries
            .try_end_step(&batch, ground, dmos, &mut HostReckoningNumeric, &mut tuning)
            .map_err(E::World)?;
        for future in &mut futures[first..end] {
            future.ready = false;
        }
        let index = queue.len;
        queue.slots[index] = Some(PendingStep {
            animation: Arc::clone(animation),
            first,
            count,
            batch,
        });
        queue.len += 1;
        Ok(())
    }
    fn update(
        &self,
        manager: SimulationManagerContactAuthority,
        queries: &mut SimulationWorldLineQueries,
        grind: &PhysicalGrindServices,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), StepSelectorError> {
        use StepSelectorError as E;
        if self.manager != manager {
            return Err(E::ForeignManager);
        }
        let mut queue = self.queue.lock().map_err(|_| E::Poisoned)?;
        for pending in queue.slots[..queue.len].iter().flatten() {
            let mut animation = pending.animation.lock().map_err(|_| E::Poisoned)?;
            let (inputs, futures) = animation
                .step_storage_mut()
                .ok_or(E::Missing("SkaterAnim step storage"))?;
            let end = pending.first + pending.count;
            if futures[pending.first].ready {
                return Err(E::AlreadyReady);
            }
            for future in &mut futures[pending.first..end] {
                future.result.valid = false;
            }
            queries.try_sync_step(&pending.batch).map_err(E::Lease)?;
            let result = queries
                .try_get_step_result(&pending.batch, 0)
                .map_err(E::Lease)?;
            let mut services = ResultServices {
                grind,
                cvars: &self.cvars,
                tuning: WalkingTuning {
                    cvars: &self.cvars,
                    overlay,
                },
                numeric: HostReckoningNumeric,
            };
            if calculate_results(
                &inputs[pending.first..end],
                &mut futures[pending.first..end],
                Some(&result),
                &mut services,
            )? {
                queries.try_release_step(&pending.batch).map_err(E::Lease)?;
            }
        }
        let length = queue.len;
        for slot in &mut queue.slots[..length] {
            *slot = None;
        }
        queue.len = 0;
        Ok(())
    }
}
