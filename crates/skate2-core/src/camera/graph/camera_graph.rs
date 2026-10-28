//! Actual Default camera graph construction through the retained startup cache.
//! Uses the existing parser, structural processing and dynamic controller.
#[path = "camera_graph_update.rs"]
pub(crate) mod update;

use crate::{
    actor_construction::{ActorConstruction, graph_startup::CachedDynamicGraph},
    attribute_runtime::AttributeRuntimeVault,
    camera_graph_factory::{CameraFactoryError, CameraLeaf, CameraLeafFactory},
    camera_man::CameraMan,
    dynamic_state_graph_controller::{
        DynamicStateGraphController, DynamicStateGraphInvariantError,
    },
    graph_asset_cache::{
        GraphCacheLoadError, GraphCachePathError, StateGraphCache, compiled_graph_path,
    },
    state_graph_context::{StateGraphContext, StateGraphContextHandle},
    state_graph_runtime::{FactoryMissPolicy, StateGraphRuntime, StateGraphRuntimeError},
};
use skate2_formats::{
    hash::sk8_hash_value_32,
    mount::{MountError, MountSet},
    state_graph::{StateGraphBinaryDocument, StateGraphBinaryError},
};
use std::sync::{Arc, Mutex, Weak};

pub(crate) type CameraGraphCache = Arc<Mutex<StateGraphCache<CachedDynamicGraph>>>;
pub(crate) type SharedCameraGraph = Arc<Mutex<StateGraphRuntime<CameraLeaf>>>;
pub(crate) struct ControllerHandle(Weak<CameraGraphController>);
pub(crate) struct ActorHandle(Arc<ActorConstruction>);
pub(crate) struct CameraManHandle(Arc<Mutex<CameraMan>>);
impl StateGraphContextHandle for ControllerHandle {
    fn shallow_copy(&self) -> Self {
        Self(Weak::clone(&self.0))
    }
}
impl StateGraphContextHandle for ActorHandle {
    fn shallow_copy(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
impl StateGraphContextHandle for CameraManHandle {
    fn shallow_copy(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
pub(crate) type CameraContext =
    StateGraphContext<ControllerHandle, ActorHandle, (), CameraManHandle, (), ()>;
pub(crate) type CameraDynamicController =
    DynamicStateGraphController<CameraLeaf, update::SlowMotionInstance, CameraContext>;
pub(crate) struct CameraGraphController {
    pub(crate) fields: Mutex<CameraGraphControllerFields>,
}
pub(crate) struct CameraGraphControllerFields {
    pub(crate) dynamic: Option<CameraDynamicController>,
    pub(crate) root_name_hash: u32,
}
pub(crate) struct CameraGraph {
    pub(crate) controller: Arc<CameraGraphController>,
    pub(crate) camera_man: Arc<Mutex<CameraMan>>,
}
#[derive(Debug)]
pub(crate) enum CameraGraphDecodeError {
    Path(GraphCachePathError),
    Mount(MountError),
    Binary(StateGraphBinaryError),
    Runtime(StateGraphRuntimeError<CameraFactoryError>),
}
#[derive(Debug)]
pub(crate) enum CameraGraphLoadError {
    Lock,
    MissingGraph,
    Cache(GraphCacheLoadError<CameraGraphDecodeError>),
    Controller(DynamicStateGraphInvariantError),
}
impl CameraGraph {
    /// `CameraGraph82e32130`, `Hierarchical.Load82cef180` and initial Dynamic
    /// Reload82cf8cf8. The context controller is this actual hierarchical owner,
    /// held weakly; `CameraMan` does not own the graph, so no ownership cycle exists.
    pub(crate) fn try_construct(
        cache: &CameraGraphCache,
        vault: &Arc<AttributeRuntimeVault>,
        mounts: &MountSet,
        actor: Option<&Arc<ActorConstruction>>,
        camera_man: &Arc<Mutex<CameraMan>>,
        filename: &str,
    ) -> Result<Self, CameraGraphLoadError> {
        use CameraGraphLoadError as E;
        let owner = Arc::new(CameraGraphController {
            fields: Mutex::new(CameraGraphControllerFields {
                dynamic: None,
                root_name_hash: 0,
            }),
        });
        let context = StateGraphContext::new(
            Some(ControllerHandle(Arc::downgrade(&owner))),
            actor.map(|actor| ActorHandle(Arc::clone(actor))),
            None,
            Some(CameraManHandle(Arc::clone(camera_man))),
            None,
            None,
        );
        let graph = {
            let mut retained = cache.try_lock().map_err(|_| E::Lock)?;
            let loaded = retained
                .load_with(filename, false, |path| {
                    // Selected compiled-asset route. Missing binary/XML fallback is
                    // an explicit mount failure, never an empty successful graph.
                    let compiled =
                        compiled_graph_path(path).map_err(CameraGraphDecodeError::Path)?;
                    let bytes = mounts
                        .read_path(&compiled)
                        .map_err(CameraGraphDecodeError::Mount)?;
                    let document = StateGraphBinaryDocument::parse_native_prefix(&bytes)
                        .map_err(CameraGraphDecodeError::Binary)?;
                    let mut factory = CameraLeafFactory::new(Arc::clone(vault));
                    let mut graph = StateGraphRuntime::build(
                        &document.root,
                        &mut factory,
                        FactoryMissPolicy::Strict,
                    )
                    .map_err(CameraGraphDecodeError::Runtime)?;
                    graph
                        .process_after_build(&mut factory)
                        .map_err(CameraGraphDecodeError::Runtime)?;
                    Ok(Some(CachedDynamicGraph::Camera(Arc::new(Mutex::new(
                        graph,
                    )))))
                })
                .map_err(E::Cache)?
                .ok_or(E::MissingGraph)?;
            match loaded {
                CachedDynamicGraph::Camera(graph) => Arc::clone(graph),
            }
        };
        // Cache guard ends before creating the controller. Constructors above
        // invoke no graph execution/camera callbacks and cannot reenter the cache.
        let dynamic =
            DynamicStateGraphController::new_shared(graph, context).map_err(E::Controller)?;
        {
            let mut fields = owner.fields.lock().map_err(|_| E::Lock)?;
            fields.dynamic = Some(dynamic);
            let root_hash = {
                let graph = fields
                    .dynamic
                    .as_ref()
                    .expect("just published")
                    .graph()
                    .map_err(E::Controller)?;
                let root = graph.root().ok_or(E::MissingGraph)?;
                sk8_hash_value_32(&graph.states()[root.0].base.name)
            };
            fields.root_name_hash = root_hash;
        }
        Ok(Self {
            controller: owner,
            camera_man: Arc::clone(camera_man),
        })
    }
}
