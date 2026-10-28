//! Actual kind6 Add828a0384/Remove828a0790 in the SAME normal arena loop.
//! Legacy void callbacks remain a separate explicit route, never a fallback.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "actual source-backed grind arena route is exercised by the retained original-data session; application scheduling remains pending"
    )
)]
use super::{
    NormalDeregistrationRunError, NormalRegistrationRunError, PreparedSimulationNormalArena,
    SimulationEnvironmentUnlockRouter, SimulationNormalArenaDeregistrationError,
    SimulationNormalArenaDispatch, SimulationNormalArenaFault, SimulationNormalArenaPlan,
    SimulationNormalArenaPrepareError, SimulationNormalArenaRegistration,
    SimulationNormalArenaRegistrationAdapter, SimulationNormalArenaRegistrationError,
    SimulationNormalEnvironmentDispatch, SimulationNormalOperation,
};
use crate::grind_data_runtime::{
    GrindDataError, GrindDataRuntime, GrindDataSourceError, GrindSplineSource, HostGrindDataNumeric,
};
use std::{
    convert::Infallible,
    fmt,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub(crate) enum NormalGrindPrepareError {
    Arena(SimulationNormalArenaPrepareError),
    Allocation,
    Source {
        operation_index: usize,
        source: GrindDataSourceError,
    },
}
impl fmt::Display for NormalGrindPrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arena(e) => write!(f, "normal grind arena preparation: {e}"),
            Self::Allocation => f.write_str("normal grind source storage allocation failed"),
            Self::Source {
                operation_index,
                source,
            } => write!(
                f,
                "normal grind source at operation {operation_index}: {source}"
            ),
        }
    }
}
/// Source validation occurs before registration effects, like the existing
/// prepared normal arena. `DReg` subsequently reads only the cached native GUID.
#[derive(Debug)]
pub(crate) struct PreparedNormalGrindArena {
    normal: PreparedSimulationNormalArena,
    sources: Arc<Vec<Option<GrindSplineSource>>>,
}
impl PreparedNormalGrindArena {
    pub(crate) fn try_new(
        plan: Arc<SimulationNormalArenaPlan>,
    ) -> Result<Self, NormalGrindPrepareError> {
        let normal =
            PreparedSimulationNormalArena::try_new(plan).map_err(NormalGrindPrepareError::Arena)?;
        let plan = normal.plan_owner();
        let mut sources = Vec::new();
        sources
            .try_reserve_exact(plan.operations().len())
            .map_err(|_| NormalGrindPrepareError::Allocation)?;
        for (operation_index, operation) in plan.operations().iter().enumerate() {
            sources.push(
                if matches!(operation, SimulationNormalOperation::Grind { .. }) {
                    Some(
                        GrindSplineSource::try_from_operation(Arc::clone(&plan), operation_index)
                            .map_err(|source| NormalGrindPrepareError::Source {
                            operation_index,
                            source,
                        })?,
                    )
                } else {
                    None
                },
            );
        }
        Ok(Self {
            normal,
            sources: Arc::new(sources),
        })
    }
}

#[derive(Debug)]
pub(crate) enum NormalGrindDispatchError {
    OwnerLock,
    MissingPreparedSource(usize),
    Data(GrindDataError<Infallible>),
}
impl fmt::Display for NormalGrindDispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("actual GrindData owner lock poisoned"),
            Self::MissingPreparedSource(index) => {
                write!(f, "normal operation {index} has no prepared grind source")
            }
            Self::Data(e) => e.fmt(f),
        }
    }
}
#[derive(Debug)]
pub(crate) enum NormalGrindRegistrationError {
    Arena(SimulationNormalArenaRegistrationError),
    Grind {
        fault: SimulationNormalArenaFault,
        source: NormalGrindDispatchError,
    },
}
impl fmt::Display for NormalGrindRegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arena(e) => e.fmt(f),
            Self::Grind { fault, source } => write!(
                f,
                "normal grind registration operation {}: {source}",
                fault.operation_index
            ),
        }
    }
}
#[derive(Debug)]
pub(crate) struct NormalGrindRegistration {
    native: SimulationNormalArenaRegistration,
    sources: Arc<Vec<Option<GrindSplineSource>>>,
}
#[derive(Debug)]
pub(crate) enum NormalGrindDeregistrationError {
    Arena {
        source: SimulationNormalArenaDeregistrationError,
        sources: Arc<Vec<Option<GrindSplineSource>>>,
    },
    Grind {
        fault: SimulationNormalArenaFault,
        source: NormalGrindDispatchError,
    },
}
impl NormalGrindDeregistrationError {
    pub(crate) fn into_registration(self) -> Option<NormalGrindRegistration> {
        match self {
            Self::Arena { source, sources } => source
                .into_registration()
                .map(|native| NormalGrindRegistration { native, sources }),
            Self::Grind { .. } => None,
        }
    }
}
impl fmt::Display for NormalGrindDeregistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arena { source, .. } => source.fmt(f),
            Self::Grind { fault, source } => write!(
                f,
                "normal grind deregistration operation {}: {source}",
                fault.operation_index
            ),
        }
    }
}

/// Private base adapter prevents source-backed tokens from entering the legacy
/// void grind route. The same world-owned `GrindData` occurrence is retained.
pub(crate) struct SimulationNormalGrindArenaAdapter<D, E = SimulationEnvironmentUnlockRouter> {
    normal: SimulationNormalArenaRegistrationAdapter<D, E>,
    grind: Arc<Mutex<GrindDataRuntime>>,
}
impl<D: SimulationNormalArenaDispatch, E: SimulationNormalEnvironmentDispatch>
    SimulationNormalGrindArenaAdapter<D, E>
{
    pub(crate) fn new(dispatch: D, environment: E, grind: Arc<Mutex<GrindDataRuntime>>) -> Self {
        Self {
            normal: SimulationNormalArenaRegistrationAdapter::with_environment(
                dispatch,
                environment,
            ),
            grind,
        }
    }
    pub(crate) fn new_selected_profile(
        dispatch: D,
        environment: E,
        grind: Arc<Mutex<GrindDataRuntime>>,
    ) -> Self {
        Self {
            normal: SimulationNormalArenaRegistrationAdapter::with_selected_profile(
                dispatch,
                environment,
            ),
            grind,
        }
    }
    pub(crate) fn selected_exclusions(
        &self,
    ) -> impl Iterator<Item = (Arc<SimulationNormalArenaPlan>, usize)> + '_ {
        self.normal.selected_exclusions()
    }
    pub(crate) fn dispatch(&self) -> &D {
        self.normal.dispatch()
    }
    pub(crate) fn register(
        &mut self,
        prepared: &PreparedNormalGrindArena,
        userdata_raw: u32,
    ) -> Result<NormalGrindRegistration, NormalGrindRegistrationError>  {
        self.register_with_waypoints(prepared, userdata_raw, None)
    }

    pub(crate) fn register_with_waypoints(
        &mut self,
        prepared: &PreparedNormalGrindArena,
        userdata_raw: u32,
        waypoint:Option<&mut dyn FnMut(u64,bool)->Result<(),String>>,
    ) -> Result<NormalGrindRegistration, NormalGrindRegistrationError> {
        let grind = &self.grind;
        self.normal
            .register_with_callbacks_and_waypoints(
                &prepared.normal,
                userdata_raw,
                |dispatch, source, asset_id| {
                    dispatch.register_dmo_model(source, asset_id);
                    Ok::<(), Infallible>(())
                },
                |_, _, operation_index, _| {
                    let source = prepared
                        .sources
                        .get(operation_index)
                        .and_then(Option::as_ref)
                        .ok_or(NormalGrindDispatchError::MissingPreparedSource(
                            operation_index,
                        ))?;
                    grind
                        .lock()
                        .map_err(|_| NormalGrindDispatchError::OwnerLock)?
                        .try_add(source.clone(), &mut HostGrindDataNumeric)
                        .map_err(NormalGrindDispatchError::Data)
                },
                waypoint,
            )
            .map(|native| NormalGrindRegistration {
                native,
                sources: Arc::clone(&prepared.sources),
            })
            .map_err(|error| match error {
                NormalRegistrationRunError::Arena(e) => NormalGrindRegistrationError::Arena(e),
                NormalRegistrationRunError::Grind { fault, source } => {
                    NormalGrindRegistrationError::Grind { fault, source }
                }
                NormalRegistrationRunError::DmoController { source, .. } => match source {},
            })
    }
    pub(crate) fn deregister(
        &mut self,
        registration: NormalGrindRegistration,
        userdata_raw: u32,
    ) -> Result<(), NormalGrindDeregistrationError>  {
        self.deregister_with_waypoints(registration, userdata_raw, None)
    }

    pub(crate) fn deregister_with_waypoints(
        &mut self,
        registration: NormalGrindRegistration,
        userdata_raw: u32,
        waypoint:Option<&mut dyn FnMut(u64,bool)->Result<(),String>>,
    ) -> Result<(), NormalGrindDeregistrationError> {
        let NormalGrindRegistration { native, sources } = registration;
        let grind = &self.grind;
        self.normal
            .deregister_with_callbacks_and_waypoints(
                native,
                userdata_raw,
                |dispatch, source, asset_id| {
                    dispatch.deregister_dmo_model(source, asset_id);
                    Ok::<(), Infallible>(())
                },
                |_, _, operation_index, _| {
                    let source = sources
                        .get(operation_index)
                        .and_then(Option::as_ref)
                        .ok_or(NormalGrindDispatchError::MissingPreparedSource(
                            operation_index,
                        ))?;
                    // DReg loads only the processed record's GUID; no spline parse,
                    // octree build or source numeric callback is introduced here.
                    grind
                        .lock()
                        .map_err(|_| NormalGrindDispatchError::OwnerLock)?
                        .try_remove(source.asset_id())
                        .map_err(NormalGrindDispatchError::Data)
                },
                waypoint,
            )
            .map_err(|error| match error {
                NormalDeregistrationRunError::Arena(source) => {
                    NormalGrindDeregistrationError::Arena { source, sources }
                }
                NormalDeregistrationRunError::Grind { fault, source } => {
                    NormalGrindDeregistrationError::Grind { fault, source }
                }
                NormalDeregistrationRunError::DmoController { source, .. } => match source {},
            })
    }
    pub(crate) const fn environment(&self) -> &E {
        self.normal.environment()
    }
    pub(crate) fn live_registration_count(&self) -> usize {
        self.normal.live_registration_count()
    }
    pub(crate) fn fault(&self) -> Option<SimulationNormalArenaFault> {
        self.normal.fault()
    }
}
