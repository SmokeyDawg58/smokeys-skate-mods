//! Selected caller-owned raw records -> SAME processed/Skeleton adjustments.
//! AI/null choice and record production are explicit caller boundaries: this
//! does not establish Actor ownership of input or full `ProcessAnimAttributes`.
use crate::{
    dynamic_assembly_session::{
        skater_pool::{SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError},
        skeleton_normal::SkeletonNormalAssembly,
    },
    input_map::{InputMapPadValues, InputMapReadError, SelectedInputMap},
    physical_player_assembly::PhysicalAssemblyWorld,
    skeleton_jump_adjustments::SkeletonJumpAdjustmentError,
    skeleton_processed_contact::jump_events::{
        SkeletonJumpError, SkeletonJumpInput, SkeletonJumpVaultError,
    },
};
use std::{convert::Infallible, fmt, sync::Arc};

/// Explicit selected `GetInput` disposition, not a reconstructed Actor query.
pub(crate) enum PhysicalPlayerJumpInputBinding<'a, P: InputMapPadValues> {
    Human {
        map: &'a SelectedInputMap,
        pad: &'a mut P,
    },
    /// Caller explicitly selected native null `GetInput` for this AI boundary.
    AiNull,
    /// Human input cannot silently select the native null/AI path.
    MissingHuman,
}
#[derive(Debug)]
pub(crate) enum PhysicalPlayerJumpInputError<E> {
    MissingHuman,
    ForeignToken,
    Map(InputMapReadError<E>),
    Adjustment(SkeletonJumpAdjustmentError),
}
impl<E: fmt::Display> fmt::Display for PhysicalPlayerJumpInputError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHuman => f.write_str("human jump input is unavailable"),
            Self::ForeignToken => f.write_str("foreign captured jump input"),
            Self::Map(e) => write!(f, "jump mapping: {e}"),
            Self::Adjustment(e) => write!(f, "jump Skeleton adjustment: {e}"),
        }
    }
}
struct Input<'a, 'b, P: InputMapPadValues> {
    skeleton: &'a mut SkeletonNormalAssembly,
    binding: &'a mut PhysicalPlayerJumpInputBinding<'b, P>,
    identity: Arc<()>,
}
impl<P: InputMapPadValues> SkeletonJumpInput for Input<'_, '_, P> {
    type Input = Arc<()>;
    type Error = PhysicalPlayerJumpInputError<P::Error>;
    fn capture_input(&mut self) -> Result<Option<Self::Input>, Self::Error> {
        match self.binding {
            PhysicalPlayerJumpInputBinding::Human { .. } => Ok(Some(Arc::clone(&self.identity))),
            PhysicalPlayerJumpInputBinding::AiNull => Ok(None),
            PhysicalPlayerJumpInputBinding::MissingHuman => Err(Self::Error::MissingHuman),
        }
    }
    fn value(&mut self, token: &Self::Input, id: u32) -> Result<f32, Self::Error> {
        if !Arc::ptr_eq(token, &self.identity) {
            return Err(Self::Error::ForeignToken);
        }
        let PhysicalPlayerJumpInputBinding::Human { map, pad } = self.binding else {
            return Err(Self::Error::ForeignToken);
        };
        map.try_value(id, *pad).map_err(Self::Error::Map)
    }
    fn store_adjustment_x(&mut self, value: f32) -> Result<(), Self::Error> {
        self.skeleton
            .store_jump_adjustment_x(value)
            .map_err(Self::Error::Adjustment)
    }
    fn store_adjustment_z(&mut self, value: f32) -> Result<(), Self::Error> {
        self.skeleton
            .store_jump_adjustment_z(value)
            .map_err(Self::Error::Adjustment)
    }
    fn adjustment_x(&mut self) -> Result<f32, Self::Error> {
        self.skeleton
            .jump_adjustment_x()
            .map_err(Self::Error::Adjustment)
    }
    fn adjustment_z(&mut self) -> Result<f32, Self::Error> {
        self.skeleton
            .jump_adjustment_z()
            .map_err(Self::Error::Adjustment)
    }
}
#[derive(Debug)]
pub(crate) enum PhysicalPlayerJumpEventsError<E> {
    MissingSkaterPool,
    Pair(SimulationSkeletonJumpPairError),
    PoisonedReckoning,
    PoisonedProcessed,
    PoisonedAttributes,
    Event(SkeletonJumpError<PhysicalPlayerJumpInputError<E>, Infallible>),
    Finish(SkeletonJumpError<PhysicalPlayerJumpInputError<E>, SkeletonJumpVaultError>),
}
impl<E: fmt::Debug> fmt::Display for PhysicalPlayerJumpEventsError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSkaterPool => f.write_str("selected skater pool is not initialized"),
            Self::Pair(e) => write!(f, "selected jump pair: {e}"),
            Self::PoisonedReckoning => f.write_str("jump processed owner access poisoned"),
            Self::PoisonedProcessed => f.write_str("jump processed state poisoned"),
            Self::PoisonedAttributes => f.write_str("jump retained attribute owner poisoned"),
            Self::Event(e) => write!(f, "selected jump event: {e}"),
            Self::Finish(e) => write!(f, "selected jump finish: {e}"),
        }
    }
}
impl PhysicalAssemblyWorld {
    /// Executes selected jump events and the existing type3 hierarchy-ID prefix
    /// in caller record order. Other event semantics remain unimplemented; a
    /// successful result means only this selected projection completed. Native
    /// Processed Reset is a separate earlier stage and is not replayed here.
    pub(crate) fn try_process_pair_selected_jump_contact_events<P: InputMapPadValues>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        records: &[[u8; 0x40]],
        binding: &mut PhysicalPlayerJumpInputBinding<'_, P>,
    ) -> Result<(), PhysicalPlayerJumpEventsError<P::Error>> {
        use PhysicalPlayerJumpEventsError as E;
        self.skaters
            .as_mut()
            .ok_or(E::MissingSkaterPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |skeleton, owner| {
                let processed = Arc::clone(
                    owner
                        .reckoning()
                        .lock()
                        .map_err(|_| E::PoisonedReckoning)?
                        .processed(),
                );
                let mut processed = processed.lock().map_err(|_| E::PoisonedProcessed)?;
                // Retained immutable hierarchy belongs to this exact Skeleton body.
                let data = Arc::clone(&skeleton.session().template().collision().body().data);
                let mut input = Input {
                    skeleton,
                    binding,
                    identity: Arc::new(()),
                };
                processed.begin_jump_events();
                for record in records {
                    processed.apply_raw_attribute(&data, record);
                    processed
                        .try_apply_jump_event(record, &mut input)
                        .map_err(E::Event)?;
                }
                // Lock only when source traversal reaches its attribute suffix.
                // No upper endpoint or global data is prefetched before its gate.
                let mut attributes = owner
                    .jump_attributes()
                    .lock()
                    .map_err(|_| E::PoisonedAttributes)?;
                processed
                    .try_finish_jump_events(&mut input, &mut *attributes)
                    .map_err(E::Finish)
            })
            .map_err(E::Pair)?
    }
}
