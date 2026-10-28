//! Concrete camera callbacks for the existing generic dynamic controller.
use super::{CameraContext, CameraGraph};
use crate::{
    camera_graph_factory::{CameraCondition, CameraLeaf},
    camera_session::selected_runtime::{CameraGlobalCondition, CameraSelectedInputs},
    camera_subject::CameraSubject,
    dynamic_state_graph_controller::{
        DynamicBehaviourCallbackRequest, DynamicBehaviourCreateRequest,
        DynamicConditionActivationRequest, DynamicStateGraphBackend,
        DynamicStateGraphControllerError, DynamicTransitionHookRequest,
    },
    input_cvar_runtime::InputCvarError,
    physical_player_phys_out::construct::ConcretePhysOutError,
    reckoning_numeric::ReckoningNumeric,
    trigger_camera_group::CameraTriggerError,
    world_painter_layer::WorldPainterLayerError,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

#[path = "camera_graph_conditions.rs"]
mod conditions;
#[path = "camera_graph_slow_motion.rs"]
mod slow_motion;
pub(crate) use slow_motion::SlowMotionInstance;

#[derive(Debug)]
pub(crate) enum CameraGraphLeafError<E> {
    OwnerLock,
    MissingSubject,
    MissingPhysOut,
    Payload(CameraCondition),
    Component(ConcretePhysOutError<Infallible>),
    Cvar(InputCvarError),
    Speed(crate::camera_rig::speed::SubjectSpeedError<E>),
    Shot(crate::camera_shot_manager::selection::ShotSelectionError<E>),
    Layer(WorldPainterLayerError),
    Trigger(CameraTriggerError),
    PendingVolumeName(Arc<str>),
    PendingBehaviour { phase: &'static str, name: Vec<u8> },
    MissingInstance,
    Attribute(crate::attribute_runtime::AttributeRuntimeError),
    Physical(crate::actor_construction::physical_mount::ActorPhysicalMountError),
    SimRate(crate::simulation_sim_rate::SimRateError),
}

pub(super) struct CameraGraphBackend<'a, N> {
    global: &'a mut CameraGlobalCondition,
    inputs: &'a CameraSelectedInputs,
    subject: &'a Arc<Mutex<CameraSubject>>,
    numeric: &'a mut N,
}

impl<N: ReckoningNumeric> DynamicStateGraphBackend<CameraLeaf, CameraContext>
    for CameraGraphBackend<'_, N>
{
    type Instance = SlowMotionInstance;
    type Error = CameraGraphLeafError<N::Error>;

    fn condition_activation(
        &mut self,
        request: DynamicConditionActivationRequest<'_, CameraLeaf, CameraContext>,
    ) -> Result<bool, Self::Error> {
        let CameraLeaf::Condition { class, data } = request.payload else {
            return Err(Self::Error::PendingBehaviour {
                phase: "condition",
                name: request.registered_name.to_vec(),
            });
        };
        self.condition(*class, data, request.context)
    }
    fn create_instance(
        &mut self,
        request: DynamicBehaviourCreateRequest<'_, CameraLeaf>,
    ) -> Result<Option<SlowMotionInstance>, Self::Error> {
        match request.payload {
            // Actual vtable82386db8 targets folded null82bb3da8.
            CameraLeaf::ChooseShot { .. } | CameraLeaf::PrintText(_) => Ok(None),
            CameraLeaf::SlowMotion { .. } => Ok(Some(SlowMotionInstance::construct())),
            _ => Err(Self::Error::PendingBehaviour {
                phase: "CreateInstance",
                name: request.registered_name.to_vec(),
            }),
        }
    }
    fn begin(
        &mut self,
        request: DynamicBehaviourCallbackRequest<'_, CameraLeaf, CameraContext, SlowMotionInstance>,
    ) -> Result<(), Self::Error> {
        if let CameraLeaf::PrintText(_) = request.payload {
            return Ok(());
        }
        if let CameraLeaf::SlowMotion { attributes } = request.payload {
            return self.slow_motion_begin(attributes, request.instance);
        }
        if let CameraLeaf::ChooseShot {
            shots,
            transition_in,
            transition_out,
        } = request.payload
        {
            let Some(camera) = request.context.camera_man() else {
                return Ok(());
            };
            if self
                .inputs
                .cvars
                .try_camera_lock_shot()
                .map_err(Self::Error::Cvar)?
            {
                return Ok(());
            }
            let mut man = camera.0.lock().map_err(|_| Self::Error::OwnerLock)?;
            man.shots.clear_choices();
            for shot in shots {
                man.shots
                    .try_add_choice(shot.as_bytes())
                    .map_err(Self::Error::Shot)?;
            }
            return man
                .try_make_shot_choice(*transition_in, *transition_out, self.numeric)
                .map_err(Self::Error::Shot);
        }
        Err(Self::Error::PendingBehaviour {
            phase: "Begin",
            name: request.registered_name.to_vec(),
        })
    }
    fn update(
        &mut self,
        request: DynamicBehaviourCallbackRequest<'_, CameraLeaf, CameraContext, SlowMotionInstance>,
    ) -> Result<(), Self::Error> {
        match request.payload {
            // Actual vtable82386dc0 ->825187c0 BLR.
            CameraLeaf::ChooseShot { .. } | CameraLeaf::PrintText(_) => Ok(()),
            // GetDeltaTime is the controller's own update dt.
            CameraLeaf::SlowMotion { attributes } => self.slow_motion_update(
                attributes,
                request.context,
                request.controller.delta_time,
                request.instance,
            ),
            _ => Err(Self::Error::PendingBehaviour {
                phase: "Update",
                name: request.registered_name.to_vec(),
            }),
        }
    }
    fn end(
        &mut self,
        request: DynamicBehaviourCallbackRequest<'_, CameraLeaf, CameraContext, SlowMotionInstance>,
    ) -> Result<(), Self::Error> {
        match request.payload {
            // Actual vtable82386dc4 ->825187c0 BLR.
            CameraLeaf::ChooseShot { .. } | CameraLeaf::PrintText(_) => Ok(()),
            CameraLeaf::SlowMotion { .. } => self.slow_motion_end(),
            _ => Err(Self::Error::PendingBehaviour {
                phase: "End",
                name: request.registered_name.to_vec(),
            }),
        }
    }
    fn on_transition(
        &mut self,
        request: DynamicTransitionHookRequest<'_, CameraLeaf, CameraContext>,
    ) -> Result<(), Self::Error> {
        Err(Self::Error::PendingBehaviour {
            phase: "OnTransition",
            name: request.registered_name.to_vec(),
        })
    }
}

#[derive(Debug)]
pub(crate) enum CameraGraphUpdateError<E> {
    OwnerLock,
    MissingDynamicController,
    Controller(DynamicStateGraphControllerError<CameraGraphLeafError<E>>),
}

impl CameraGraph {
    /// `UpdateCameraManInstances36384` calls this same controller with fixed1/60.
    pub(crate) fn try_update<N: ReckoningNumeric>(
        &self,
        global: &mut CameraGlobalCondition,
        inputs: &CameraSelectedInputs,
        subject: &Arc<Mutex<CameraSubject>>,
        numeric: &mut N,
    ) -> Result<(), CameraGraphUpdateError<N::Error>> {
        let mut fields = self
            .controller
            .fields
            .lock()
            .map_err(|_| CameraGraphUpdateError::OwnerLock)?;
        let controller = fields
            .dynamic
            .as_mut()
            .ok_or(CameraGraphUpdateError::MissingDynamicController)?;
        let mut backend = CameraGraphBackend {
            global,
            inputs,
            subject,
            numeric,
        };
        // Callbacks never reacquire controller fields or the world guard.
        controller
            .update(&mut backend, f32::from_bits(0x3c88_8889))
            .map_err(CameraGraphUpdateError::Controller)
    }
}

impl<E: std::fmt::Display> std::fmt::Display for CameraGraphLeafError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("camera graph owner lock"),
            Self::MissingSubject => f.write_str("camera graph missing Subject"),
            Self::MissingPhysOut => f.write_str("camera graph missing PhysOut"),
            Self::Payload(class) => write!(f, "camera condition payload {class:?}"),
            Self::Component(error) => write!(f, "camera condition component: {error:?}"),
            Self::Cvar(error) => write!(f, "camera graph cvar: {error}"),
            Self::Speed(error) => write!(f, "camera graph speed: {error}"),
            Self::Shot(error) => write!(f, "camera graph shot: {error}"),
            Self::Layer(error) => write!(f, "camera region: {error}"),
            Self::Trigger(error) => write!(f, "camera volume: {error:?}"),
            Self::PendingVolumeName(name) => write!(f, "pending populated camera volume {name}"),
            Self::PendingBehaviour { phase, name } => write!(
                f,
                "pending camera {phase} {}",
                String::from_utf8_lossy(name)
            ),
            Self::MissingInstance => f.write_str("camera behaviour instance missing"),
            Self::Attribute(error) => write!(f, "camera behaviour attributes: {error:?}"),
            Self::Physical(error) => write!(f, "camera behaviour IPhysical: {error:?}"),
            Self::SimRate(error) => write!(f, "camera behaviour: {error}"),
        }
    }
}
impl<E: std::fmt::Display> std::fmt::Display for CameraGraphUpdateError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerLock => f.write_str("camera controller lock"),
            Self::MissingDynamicController => f.write_str("camera dynamic controller absent"),
            Self::Controller(error) => write!(f, "camera controller: {error}"),
        }
    }
}
