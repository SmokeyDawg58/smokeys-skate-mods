//! Pure route selection for Simulation collection registration.
//!
//! `cAssetStreamManager::NotifyCollectionSimReg` (`0x82d5c468`) emits
//! Simulation registration messages only for raw stream types zero and one.
//! Its deferred branch additionally requires the live defer cVar, the manager defer word at
//! `+0x2a834`, a false per-call `forceImmediate`, and an unsigned stream size
//! strictly greater than `0x7d000`. Every route still has a separate callback
//! message; the deferred producer emits its callback later from
//! `UpdateSimCollectionDefer`. This module accepts the already-sampled control values and lazily queries size only on the native size-test path. It performs no
//! I/O, queueing, registration, callback work, or loaded-count mutation. The
//! notifier's unconditional loaded-count increment occurs before this message
//! route and remains an outer lifecycle responsibility.

/// Strict native threshold for deferred Simulation collection registration.
pub const SIMULATION_COLLECTION_DEFER_SIZE_THRESHOLD: u32 = 0x7d000;

/// Control values needed before the conditional stream-size query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimulationCollectionAdmissionInputs {
    /// Raw registered stream type; only exact zero and one emit registration messages.
    pub stream_type_raw: u32,
    /// Live value of `kbEnableCollectionRegDefer` for this decision.
    pub defer_cvar_enabled: bool,
    /// Raw manager control word; any nonzero value enables its defer gate.
    pub manager_defer_raw: u32,
    /// Per-call immediate override.
    pub force_immediate: bool,
}

/// Producer sequence selected before any Simulation message is sent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationCollectionRegistrationRoute {
    /// No `RegCollection`/`RegArena` message; the callback is still sent.
    CallbackOnly,
    /// `RegCollection` with activation one, then `Callback`.
    ImmediateCollectionThenCallback,
    /// Deferred Arena sequence, activation-zero Collection, then `Callback`.
    DeferredCollectionThenCallback,
}

/// Message route for one Simulation collection deregistration notification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationCollectionDeregistrationRoute {
    /// No `DRegCollection` message; the callback is still sent.
    CallbackOnly,
    /// `DRegCollection`, then the separately sent `Callback`.
    DeregistrationThenCallback,
}

/// Selects the exact registration producer route from already-sampled inputs.
#[must_use]
pub fn select_simulation_collection_registration_route<F>(
    inputs: SimulationCollectionAdmissionInputs,
    stream_size: F,
) -> SimulationCollectionRegistrationRoute
where
    F: FnOnce() -> u32,
{
    match try_select_simulation_collection_registration_route(inputs, || {
        Ok::<u32, std::convert::Infallible>(stream_size())
    }) {
        Ok(route) => route,
        Err(never) => match never {},
    }
}

/// Fallible form of the exact registration route selector.
///
/// The size provider is reached only after the supported-type, cVar, manager,
/// and force-immediate gates pass. This lets an outer notifier preserve native
/// call ordering while retaining a typed host provider failure at that exact
/// boundary.
///
/// # Errors
///
/// Returns the size provider's error only when the native route reaches
/// `GetStreamSize`.
pub fn try_select_simulation_collection_registration_route<F, E>(
    inputs: SimulationCollectionAdmissionInputs,
    stream_size: F,
) -> Result<SimulationCollectionRegistrationRoute, E>
where
    F: FnOnce() -> Result<u32, E>,
{
    if inputs.stream_type_raw != 0 && inputs.stream_type_raw != 1 {
        return Ok(SimulationCollectionRegistrationRoute::CallbackOnly);
    }
    if inputs.defer_cvar_enabled
        && inputs.manager_defer_raw != 0
        && !inputs.force_immediate
        && stream_size()? > SIMULATION_COLLECTION_DEFER_SIZE_THRESHOLD
    {
        Ok(SimulationCollectionRegistrationRoute::DeferredCollectionThenCallback)
    } else {
        Ok(SimulationCollectionRegistrationRoute::ImmediateCollectionThenCallback)
    }
}

/// Selects the exact deregistration message route after stream-type lookup.
#[must_use]
pub const fn select_simulation_collection_deregistration_route(
    stream_type_raw: u32,
) -> SimulationCollectionDeregistrationRoute {
    if stream_type_raw == 0 || stream_type_raw == 1 {
        SimulationCollectionDeregistrationRoute::DeregistrationThenCallback
    } else {
        SimulationCollectionDeregistrationRoute::CallbackOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(
        stream_type_raw: u32,
        defer_cvar_enabled: bool,
        manager_defer_raw: u32,
        force_immediate: bool,
        stream_size: u32,
    ) -> SimulationCollectionRegistrationRoute {
        select_simulation_collection_registration_route(
            SimulationCollectionAdmissionInputs {
                stream_type_raw,
                defer_cvar_enabled,
                manager_defer_raw,
                force_immediate,
            },
            || stream_size,
        )
    }

    #[test]
    fn only_exact_stream_types_zero_and_one_emit_registration_messages() {
        for stream_type_raw in [2, 3, u32::MAX] {
            assert_eq!(
                route(stream_type_raw, true, 1, false, u32::MAX),
                SimulationCollectionRegistrationRoute::CallbackOnly
            );
        }
        for stream_type_raw in [0, 1] {
            assert_eq!(
                route(stream_type_raw, false, 0, true, 0),
                SimulationCollectionRegistrationRoute::ImmediateCollectionThenCallback
            );
        }
    }

    #[test]
    fn defer_requires_every_live_gate() {
        for stream_type_raw in [0, 1] {
            for defer_cvar_enabled in [false, true] {
                for manager_defer_raw in [0, 1, u32::MAX] {
                    for force_immediate in [false, true] {
                        let expected =
                            defer_cvar_enabled && manager_defer_raw != 0 && !force_immediate;
                        assert_eq!(
                            route(
                                stream_type_raw,
                                defer_cvar_enabled,
                                manager_defer_raw,
                                force_immediate,
                                SIMULATION_COLLECTION_DEFER_SIZE_THRESHOLD + 1,
                            ),
                            if expected {
                                SimulationCollectionRegistrationRoute::DeferredCollectionThenCallback
                            } else {
                                SimulationCollectionRegistrationRoute::ImmediateCollectionThenCallback
                            }
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn size_comparison_is_unsigned_and_strictly_greater() {
        for stream_size in [
            0,
            SIMULATION_COLLECTION_DEFER_SIZE_THRESHOLD - 1,
            SIMULATION_COLLECTION_DEFER_SIZE_THRESHOLD,
        ] {
            assert_eq!(
                route(1, true, 1, false, stream_size),
                SimulationCollectionRegistrationRoute::ImmediateCollectionThenCallback
            );
        }
        for stream_size in [
            SIMULATION_COLLECTION_DEFER_SIZE_THRESHOLD + 1,
            0x8000_0000,
            u32::MAX,
        ] {
            assert_eq!(
                route(1, true, 1, false, stream_size),
                SimulationCollectionRegistrationRoute::DeferredCollectionThenCallback
            );
        }
    }

    #[test]
    fn size_query_is_skipped_on_every_short_circuit() {
        for inputs in [
            SimulationCollectionAdmissionInputs {
                stream_type_raw: 2,
                defer_cvar_enabled: true,
                manager_defer_raw: 1,
                force_immediate: false,
            },
            SimulationCollectionAdmissionInputs {
                stream_type_raw: 1,
                defer_cvar_enabled: false,
                manager_defer_raw: 1,
                force_immediate: false,
            },
            SimulationCollectionAdmissionInputs {
                stream_type_raw: 1,
                defer_cvar_enabled: true,
                manager_defer_raw: 0,
                force_immediate: false,
            },
            SimulationCollectionAdmissionInputs {
                stream_type_raw: 1,
                defer_cvar_enabled: true,
                manager_defer_raw: 1,
                force_immediate: true,
            },
        ] {
            let mut queried = false;
            let _ = select_simulation_collection_registration_route(inputs, || {
                queried = true;
                u32::MAX
            });
            assert!(!queried);
        }
    }

    #[test]
    fn fallible_size_query_errors_only_at_the_native_call_site() {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        struct SizeError;

        for (inputs, expected) in [
            (
                SimulationCollectionAdmissionInputs {
                    stream_type_raw: 2,
                    defer_cvar_enabled: true,
                    manager_defer_raw: 1,
                    force_immediate: false,
                },
                SimulationCollectionRegistrationRoute::CallbackOnly,
            ),
            (
                SimulationCollectionAdmissionInputs {
                    stream_type_raw: 1,
                    defer_cvar_enabled: false,
                    manager_defer_raw: 1,
                    force_immediate: false,
                },
                SimulationCollectionRegistrationRoute::ImmediateCollectionThenCallback,
            ),
            (
                SimulationCollectionAdmissionInputs {
                    stream_type_raw: 1,
                    defer_cvar_enabled: true,
                    manager_defer_raw: 0,
                    force_immediate: false,
                },
                SimulationCollectionRegistrationRoute::ImmediateCollectionThenCallback,
            ),
            (
                SimulationCollectionAdmissionInputs {
                    stream_type_raw: 1,
                    defer_cvar_enabled: true,
                    manager_defer_raw: 1,
                    force_immediate: true,
                },
                SimulationCollectionRegistrationRoute::ImmediateCollectionThenCallback,
            ),
        ] {
            let selected =
                try_select_simulation_collection_registration_route(inputs, || Err(SizeError));
            assert_eq!(selected, Ok(expected));
        }

        let error = try_select_simulation_collection_registration_route(
            SimulationCollectionAdmissionInputs {
                stream_type_raw: 1,
                defer_cvar_enabled: true,
                manager_defer_raw: 1,
                force_immediate: false,
            },
            || Err(SizeError),
        )
        .unwrap_err();
        assert_eq!(error, SizeError);
    }

    #[test]
    fn deregistration_uses_the_same_exact_stream_type_class() {
        for stream_type_raw in [0, 1] {
            assert_eq!(
                select_simulation_collection_deregistration_route(stream_type_raw),
                SimulationCollectionDeregistrationRoute::DeregistrationThenCallback
            );
        }
        for stream_type_raw in [2, 3, u32::MAX] {
            assert_eq!(
                select_simulation_collection_deregistration_route(stream_type_raw),
                SimulationCollectionDeregistrationRoute::CallbackOnly
            );
        }
    }
}
