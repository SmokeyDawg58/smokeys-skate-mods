//! Retained selected construction on the existing camera notification owner.
//! The legacy filename identifies the reviewed construction packet; this code
//! is reused by the production selected capture, not copied into a second owner.
use super::{
    CameraStartupFields,
    selected_runtime::{CameraGlobalCondition, CameraSelectedInputs},
};
use crate::{
    attribute_notifications::{AttributeNotificationError, AttributeNotifications},
    attribute_runtime::AttributeRuntimeVault,
    camera_attributes::CameraAttributes,
    camera_collision::CameraCollision,
    camera_compass::{CameraAnchors, CameraCompass},
    camera_drop::{CameraDropPredictor, CameraGrindDetector},
    camera_graph::{CameraGraph, CameraGraphLoadError},
    camera_man::{CameraMan, CameraManDescriptor, CameraManError},
    camera_prediction::CameraObstacles,
    camera_shot_manager::CameraShotDatabase,
    camera_subject::CameraSubject,
    reckoning_numeric::ReckoningNumeric,
};
use std::sync::{Arc, Mutex};

/// Retained constructor components, not an initialized manager capability.
pub(crate) struct CameraConstructionState {
    pub(crate) parameters: Arc<Mutex<CameraAttributes>>,
    pub(crate) obstacles: Arc<Mutex<CameraObstacles>>,
    pub(crate) anchors: CameraAnchors,
    pub(crate) compass: CameraCompass,
    pub(crate) compass_output: [f32; 4],
    pub(crate) grind: Arc<Mutex<CameraGrindDetector>>,
    pub(crate) subject: Arc<Mutex<CameraSubject>>,
    pub(crate) camera_man: Option<Arc<Mutex<CameraMan>>>,
    pub(crate) graph: Option<CameraGraph>,
}
#[derive(Debug)]
pub(crate) enum CameraConstructionError<E> {
    AlreadyConstructed,
    Lock,
    Attribute(AttributeNotificationError),
    Allocation,
    Man(CameraManError<E>),
    Graph(CameraGraphLoadError),
}
impl CameraStartupFields {
    /// Constructor subobjects precede the first local-change callback. The same
    /// parameter owner survives partial notification registration and failure.
    pub(crate) fn try_construct_selected_components<E>(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        monitor: &AttributeNotifications,
    ) -> Result<(), CameraConstructionError<E>> {
        use CameraConstructionError as Error;
        if self.retained_parameters.is_some() {
            return Err(Error::AlreadyConstructed);
        }
        let parameters = Arc::new(Mutex::new(CameraAttributes::begin_init()));
        // Publish before the first binding/notification. This owner and attempt
        // marker survive partial Init and every later constructor failure.
        self.retained_parameters = Some(Arc::clone(&parameters));
        parameters
            .lock()
            .map_err(|_| Error::Lock)?
            .initialize(vault, monitor)
            .map_err(Error::Attribute)?;
        self.global = Some(CameraGlobalCondition::construct());
        let obstacles = Arc::new(Mutex::new(CameraObstacles::constructed()));
        let anchors = CameraAnchors::constructed();
        let compass = CameraCompass::constructed(Arc::clone(&parameters));
        // Native separate CameraCompassOutput37de4..df4 before GrindDetector.
        let compass_output = [0.0; 4];
        let grind = Arc::new(Mutex::new(CameraGrindDetector::constructed(Arc::clone(
            &parameters,
        ))));
        let subject = Arc::new(Mutex::new(
            CameraSubject::try_construct().map_err(|_| Error::Allocation)?,
        ));
        self.construction_state = Some(CameraConstructionState {
            parameters: Arc::clone(&parameters),
            obstacles: Arc::clone(&obstacles),
            anchors,
            compass,
            compass_output,
            grind: Arc::clone(&grind),
            subject: Arc::clone(&subject),
            camera_man: None,
            graph: None,
        });
        Ok(())
    }

    /// Native `CreateCameraManInstances` skips an already published man list.
    /// A graph failure preserves the man and cannot silently reconstruct it.
    pub(crate) fn try_create_selected_camera_instances<N: ReckoningNumeric>(
        &mut self,
        inputs: &CameraSelectedInputs,
        numeric: &mut N,
    ) -> Result<(), CameraConstructionError<N::Error>> {
        use CameraConstructionError as E;
        if self.camera_man_count != 0 {
            return Ok(());
        }
        if self.graph_count != 0 {
            return Err(E::AlreadyConstructed);
        }
        let retained = self
            .construction_state
            .as_ref()
            .ok_or(E::AlreadyConstructed)?;
        let parameters = Arc::clone(&retained.parameters);
        let obstacles = Arc::clone(&retained.obstacles);
        let grind = Arc::clone(&retained.grind);
        let subject = Arc::clone(&retained.subject);
        let predictor = Arc::clone(&inputs.predictor);
        let collision = Arc::new(Mutex::new(CameraCollision::constructed(
            inputs.world.clone(),
        )));
        let database = Arc::new(Mutex::new(CameraShotDatabase::constructed(Arc::clone(
            &inputs.vault,
        ))));
        let drop = Arc::new(Mutex::new(CameraDropPredictor::constructed(
            Arc::clone(&parameters),
            Arc::clone(&collision),
        )));
        let mut man = CameraMan::try_construct(
            CameraManDescriptor {
                parameters,
                collision,
                predictor,
                obstacles,
                database,
                grind,
                drop,
            },
            &inputs.mounts,
            numeric,
        )
        .map_err(E::Man)?;
        man.set_subject(&subject);
        let man = Arc::new(Mutex::new(man));
        let retained = self
            .construction_state
            .as_mut()
            .expect("retained constructor");
        retained.camera_man = Some(Arc::clone(&man));
        self.camera_man_count = 1;
        let graph = CameraGraph::try_construct(
            &inputs.cache,
            &inputs.vault,
            &inputs.mounts,
            self.actor.as_ref(),
            &man,
            &format!("script/camera/{}_cameragraph.xml", inputs.camera_set),
        )
        .map_err(E::Graph)?;
        retained.graph = Some(graph);
        self.graph_count = 1;
        Ok(())
    }
}
