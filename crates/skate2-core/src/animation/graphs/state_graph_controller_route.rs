//! Lazy generated-versus-Dynamic `StateGraph` controller routing.
//!
//! Controller selection consumes the original logical request name. Only a
//! selected Dynamic fallback is normalized through the cache-path front and
//! resolved to its compiled `.stategraph` sibling. Keeping those identities
//! separate prevents the physical filename from incorrectly defeating an
//! enabled generated delegate.

use std::{error::Error, fmt};

use crate::{
    generated_onboard_state_graph::{
        GeneratedOnboardEnablement, GeneratedOnboardKind, GeneratedRequiredMiss,
        OnboardControllerSelection, select_onboard_controller,
    },
    graph_asset_cache::{
        GraphCacheKey, GraphCachePathError, compiled_graph_path, state_graph_cache_key,
    },
};

/// One caller-loaded Dynamic payload together with all recovered path identities.
#[derive(Debug)]
pub struct DynamicStateGraphAsset<T> {
    logical_request: String,
    cache_key: GraphCacheKey,
    compiled_path: String,
    payload: T,
}

impl<T> DynamicStateGraphAsset<T> {
    #[must_use]
    pub fn logical_request(&self) -> &str {
        &self.logical_request
    }

    #[must_use]
    pub const fn cache_key(&self) -> &GraphCacheKey {
        &self.cache_key
    }

    #[must_use]
    pub fn compiled_path(&self) -> &str {
        &self.compiled_path
    }

    #[must_use]
    pub const fn payload(&self) -> &T {
        &self.payload
    }

    #[must_use]
    pub fn into_payload(self) -> T {
        self.payload
    }

    /// Transforms only the caller-supplied payload while preserving all three
    /// route identities unchanged. Those identities do not certify the
    /// replacement payload's provenance.
    #[must_use]
    pub fn map_payload<U, F>(self, map: F) -> DynamicStateGraphAsset<U>
    where
        F: FnOnce(T) -> U,
    {
        DynamicStateGraphAsset {
            logical_request: self.logical_request,
            cache_key: self.cache_key,
            compiled_path: self.compiled_path,
            payload: map(self.payload),
        }
    }
}

/// Exact route selected before any generated or Dynamic controller exists.
#[derive(Debug)]
pub enum StateGraphControllerRoute<T> {
    GeneratedOnboard(GeneratedOnboardKind),
    GeneratedOther { delegate_index: usize },
    Dynamic(DynamicStateGraphAsset<T>),
}

#[derive(Debug)]
pub enum StateGraphControllerRouteError<E> {
    GeneratedRequired(GeneratedRequiredMiss),
    Path(GraphCachePathError),
    DynamicLoad(E),
}

impl<E: fmt::Display> fmt::Display for StateGraphControllerRouteError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GeneratedRequired(error) => error.fmt(formatter),
            Self::Path(error) => error.fmt(formatter),
            Self::DynamicLoad(error) => {
                write!(formatter, "Dynamic StateGraph load failed: {error}")
            }
        }
    }
}

impl<E: Error + 'static> Error for StateGraphControllerRouteError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::GeneratedRequired(error) => error,
            Self::Path(error) => error,
            Self::DynamicLoad(error) => error,
        })
    }
}

/// Reconstructs generated selection before one-shot Dynamic materialization,
/// plus the compiled-sibling derivation from the path front of
/// `StateGraph::Cache::Load` (`0x82CF8AF0`). Generated selection itself is in
/// `AbstractHierarchicalController::Create` (`0x829513F0`).
///
/// Generated selection is case-sensitive and byte-exact on the final basename
/// of the logical request's C-string prefix. It precedes all path normalization
/// and loader effects. Only Dynamic fallback constructs a cache key, resolves
/// the compiled sibling, and invokes `load_dynamic` once.
/// The returned payload is exactly the loader result; `T` is deliberately
/// unconstrained. This helper does not perform or claim a real cache lookup,
/// hit/flush/replacement/capacity behavior, old-controller deletion, Dynamic
/// controller construction/reload, cache sharing, controller lifetime, or
/// executable leaf coverage. The host input is UTF-8: a generated C-prefix hit
/// can return before a later embedded NUL is inspected by the Dynamic path,
/// while Dynamic normalization rejects that NUL.
///
/// # Errors
///
/// Returns a required-generated miss before path/loader work, a checked cache
/// path failure, or the Dynamic loader's exact error.
pub fn route_state_graph_controller<T, E, F>(
    logical_request: &str,
    allow_generated: bool,
    require_generated: bool,
    enablement: GeneratedOnboardEnablement,
    load_dynamic: F,
) -> Result<StateGraphControllerRoute<T>, StateGraphControllerRouteError<E>>
where
    F: FnOnce(&str) -> Result<T, E>,
{
    let selection = select_onboard_controller(
        logical_request.as_bytes(),
        allow_generated,
        require_generated,
        enablement,
    )
    .map_err(StateGraphControllerRouteError::GeneratedRequired)?;
    match selection {
        OnboardControllerSelection::GeneratedOnboard(kind) => {
            Ok(StateGraphControllerRoute::GeneratedOnboard(kind))
        }
        OnboardControllerSelection::GeneratedOther { delegate_index } => {
            Ok(StateGraphControllerRoute::GeneratedOther { delegate_index })
        }
        OnboardControllerSelection::DynamicFallback => {
            let cache_key = state_graph_cache_key(logical_request)
                .map_err(StateGraphControllerRouteError::Path)?;
            let compiled_path = compiled_graph_path(cache_key.normalized_path())
                .map_err(StateGraphControllerRouteError::Path)?;
            let payload = load_dynamic(&compiled_path)
                .map_err(StateGraphControllerRouteError::DynamicLoad)?;
            Ok(StateGraphControllerRoute::Dynamic(DynamicStateGraphAsset {
                logical_request: logical_request.to_owned(),
                cache_key,
                compiled_path,
                payload,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, convert::Infallible, error::Error};

    #[derive(Debug)]
    struct LoaderError;

    impl fmt::Display for LoaderError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("loader")
        }
    }

    impl Error for LoaderError {}

    const MOTION_LOGICAL: &str = "state/MotionGraph_OnBoard.xml";

    #[test]
    fn generated_motion_precedes_every_dynamic_path_and_loader_effect() {
        let calls = Cell::new(0);
        let route = route_state_graph_controller(
            MOTION_LOGICAL,
            true,
            false,
            GeneratedOnboardEnablement::onboard(1, 1),
            |_| -> Result<(), Infallible> {
                calls.set(calls.get() + 1);
                Ok(())
            },
        )
        .unwrap();
        assert!(matches!(
            route,
            StateGraphControllerRoute::GeneratedOnboard(GeneratedOnboardKind::Motion)
        ));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn dynamic_motion_retains_logical_cache_and_compiled_identities() {
        let calls = Cell::new(0);
        let route = route_state_graph_controller(
            MOTION_LOGICAL,
            true,
            false,
            GeneratedOnboardEnablement::onboard(1, 0),
            |path| {
                calls.set(calls.get() + 1);
                assert_eq!(path, r"data\state\MotionGraph_OnBoard.stategraph");
                Ok::<_, Infallible>(17_u32)
            },
        )
        .unwrap();
        let StateGraphControllerRoute::Dynamic(asset) = route else {
            panic!("disabled Motion delegate did not select Dynamic")
        };
        assert_eq!(calls.get(), 1);
        assert_eq!(asset.logical_request(), MOTION_LOGICAL);
        assert_eq!(
            asset.cache_key().normalized_path(),
            r"data\state\MotionGraph_OnBoard.xml"
        );
        assert_eq!(asset.cache_key().hash(), 0x07d5_e6bc);
        assert_eq!(
            asset.compiled_path(),
            r"data\state\MotionGraph_OnBoard.stategraph"
        );
        assert_eq!(asset.into_payload(), 17);
    }

    #[test]
    fn required_generated_miss_precedes_dynamic_loader() {
        let calls = Cell::new(0);
        let error = route_state_graph_controller(
            MOTION_LOGICAL,
            true,
            true,
            GeneratedOnboardEnablement::onboard(1, 0),
            |_| -> Result<(), Infallible> {
                calls.set(calls.get() + 1);
                Ok(())
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            StateGraphControllerRouteError::GeneratedRequired(GeneratedRequiredMiss { basename })
                if basename == b"MotionGraph_OnBoard.xml"
        ));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn generated_disabled_matrix_never_falls_through_a_required_miss() {
        let calls = Cell::new(0);
        let enabled = GeneratedOnboardEnablement::onboard(1, 1);
        let dynamic = route_state_graph_controller(MOTION_LOGICAL, false, false, enabled, |_| {
            calls.set(calls.get() + 1);
            Ok::<_, Infallible>(())
        })
        .unwrap();
        assert!(matches!(dynamic, StateGraphControllerRoute::Dynamic(_)));
        assert_eq!(calls.get(), 1);

        let error = route_state_graph_controller(
            MOTION_LOGICAL,
            false,
            true,
            enabled,
            |_| -> Result<(), Infallible> {
                calls.set(calls.get() + 1);
                Ok(())
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            StateGraphControllerRouteError::GeneratedRequired(_)
        ));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn dynamic_path_and_loader_failures_preserve_priority_and_source() {
        let calls = Cell::new(0);
        let too_long = format!("{}.xml", "a".repeat(252));
        let path_error = route_state_graph_controller(
            &too_long,
            false,
            false,
            GeneratedOnboardEnablement::onboard(0, 0),
            |_| -> Result<(), LoaderError> {
                calls.set(calls.get() + 1);
                Ok(())
            },
        )
        .unwrap_err();
        assert!(matches!(
            path_error,
            StateGraphControllerRouteError::Path(GraphCachePathError::TooLong { .. })
        ));
        assert_eq!(calls.get(), 0);

        let loader_error = route_state_graph_controller(
            MOTION_LOGICAL,
            false,
            false,
            GeneratedOnboardEnablement::onboard(0, 0),
            |_| Err::<(), _>(LoaderError),
        )
        .unwrap_err();
        assert!(matches!(
            loader_error,
            StateGraphControllerRouteError::DynamicLoad(LoaderError)
        ));
        assert_eq!(
            loader_error.to_string(),
            "Dynamic StateGraph load failed: loader"
        );
        assert_eq!(loader_error.source().unwrap().to_string(), "loader");
    }

    #[test]
    fn generated_c_prefix_hit_precedes_dynamic_nul_validation() {
        let route = route_state_graph_controller(
            "state/MotionGraph_OnBoard.xml\0ignored",
            true,
            false,
            GeneratedOnboardEnablement::onboard(1, 1),
            |_| -> Result<(), Infallible> { panic!("C-prefix generated hit loaded Dynamic") },
        )
        .unwrap();
        assert!(matches!(
            route,
            StateGraphControllerRoute::GeneratedOnboard(GeneratedOnboardKind::Motion)
        ));

        let error = route_state_graph_controller(
            "state/other.xml\0ignored",
            true,
            false,
            GeneratedOnboardEnablement::onboard(1, 1),
            |_| -> Result<(), Infallible> { panic!("embedded-NUL Dynamic path loaded") },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            StateGraphControllerRouteError::Path(GraphCachePathError::EmbeddedNul)
        ));
    }

    #[test]
    fn physical_compiled_name_is_not_a_generated_selection_identity() {
        let calls = Cell::new(0);
        let route = route_state_graph_controller(
            r"data\state\MotionGraph_OnBoard.stategraph",
            true,
            false,
            GeneratedOnboardEnablement::onboard(1, 1),
            |_| {
                calls.set(calls.get() + 1);
                Ok::<_, Infallible>(())
            },
        )
        .unwrap();
        assert!(matches!(route, StateGraphControllerRoute::Dynamic(_)));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn non_onboard_generated_delegate_remains_distinct_and_lazy() {
        let mut raw = [0_u8; 20];
        raw[2] = 0xff;
        let route = route_state_graph_controller(
            "state/livingworldentities/vehicle/Vehicle.xml",
            true,
            false,
            GeneratedOnboardEnablement::from_delegate_raw(raw),
            |_| -> Result<(), Infallible> { panic!("generated Vehicle loaded Dynamic") },
        )
        .unwrap();
        assert!(matches!(
            route,
            StateGraphControllerRoute::GeneratedOther { delegate_index: 2 }
        ));
    }
}
