//! Unregistered live inputs for absorption/pump-force; uses existing global +f8.
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    physical_player_pumping::{
        PhysicalPlayerPumpingOwner, PumpingError, attributes::PumpingVaultAttributes,
    },
    skateboard_foot_attributes::{
        SkateboardFootAttributeError, SkateboardFootAttributePointer, SkateboardFootAttributeRead,
        SkateboardFootVaultAttributes,
    },
    skateboard_force_runtime::pumping_forces::{
        ABSORPTION_SCALE, AbsorptionSource, ENABLED, NO_INTENTION_SCALE, PumpForceSource,
    },
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};

pub(crate) trait PumpingForceDebug {
    type Error;
    fn draw_absorption(&mut self) -> Result<bool, Self::Error>;
    fn draw_intention(&mut self) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum PumpingForceRuntimeError<D> {
    Poisoned(&'static str),
    Missing(&'static str),
    Pumping(PumpingError),
    Feet(SkateboardFootAttributeError),
    Attribute(AttributeRuntimeError),
    Debug(D),
}
impl<D: std::fmt::Debug> std::fmt::Display for PumpingForceRuntimeError<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned(owner) => write!(f, "pump force {owner} lock poisoned"),
            Self::Missing(field) => write!(f, "pump force missing {field}"),
            Self::Pumping(error) => write!(f, "pump force owner: {error:?}"),
            Self::Feet(error) => write!(f, "pump force feet: {error}"),
            Self::Attribute(error) => write!(f, "pump force attribute: {error}"),
            Self::Debug(error) => write!(f, "pump force debug: {error:?}"),
        }
    }
}
pub(crate) struct PumpingForceRuntime<'a, D> {
    pub(crate) processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) pumping: &'a Arc<Mutex<PhysicalPlayerPumpingOwner>>,
    pub(crate) attributes: &'a Mutex<PumpingVaultAttributes>,
    pub(crate) feet: &'a mut SkateboardFootVaultAttributes,
    pub(crate) debug: &'a mut D,
}
impl<D: PumpingForceDebug> PumpingForceRuntime<'_, D> {
    fn read<T>(
        &self,
        field: &'static str,
        read: impl FnOnce(&SkeletonProcessedContact) -> Option<T>,
    ) -> Result<T, PumpingForceRuntimeError<D::Error>> {
        let input = self
            .processed
            .lock()
            .map_err(|_| PumpingForceRuntimeError::Poisoned("processed"))?;
        read(&input).ok_or(PumpingForceRuntimeError::Missing(field))
    }
}
impl<D: PumpingForceDebug> AbsorptionSource for PumpingForceRuntime<'_, D> {
    type Error = PumpingForceRuntimeError<D::Error>;
    type Pointer = SkateboardFootAttributePointer;
    fn crouch(&mut self) -> Result<f32, Self::Error> {
        self.read("crouch9d4", |p| p.input_controls().crouch_input)
    }
    fn crouch_delta(&mut self) -> Result<f32, Self::Error> {
        self.read("crouch delta9d8", |p| p.input_controls().crouch_input_delta)
    }
    fn capture_scale(&mut self) -> Result<Self::Pointer, Self::Error> {
        self.feet
            .lookup(ABSORPTION_SCALE)
            .map_err(Self::Error::Feet)
    }
    fn load_scale(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        self.feet.float(pointer).map_err(Self::Error::Feet)
    }
    fn absorption(&mut self) -> Result<f32, Self::Error> {
        self.pumping
            .lock()
            .map_err(|_| Self::Error::Poisoned("Pumping"))?
            .try_force_absorption()
            .map_err(Self::Error::Pumping)
    }
    fn draw_absorption(&mut self) -> Result<bool, Self::Error> {
        self.debug.draw_absorption().map_err(Self::Error::Debug)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.debug.require_overlay().map_err(Self::Error::Debug)
    }
}
impl<D: PumpingForceDebug> PumpForceSource for PumpingForceRuntime<'_, D> {
    type Error = PumpingForceRuntimeError<D::Error>;
    type Pointer = Option<AttributeRuntimeData>;
    fn player_controlled_pump(&mut self) -> Result<bool, Self::Error> {
        self.read("player controlled pump8cc/4", |p| {
            p.physics_state_fields().flags_8cc.map(|v| v & 4 != 0)
        })
    }
    fn acceleration(&mut self) -> Result<f32, Self::Error> {
        self.pumping
            .lock()
            .map_err(|_| Self::Error::Poisoned("Pumping"))?
            .try_force_acceleration()
            .map_err(Self::Error::Pumping)
    }
    fn draw_intention(&mut self) -> Result<bool, Self::Error> {
        self.debug.draw_intention().map_err(Self::Error::Debug)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.debug.require_overlay().map_err(Self::Error::Debug)
    }
    fn capture(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        self.attributes
            .lock()
            .map_err(|_| Self::Error::Poisoned("Pumping attributes"))?
            .lookup(key)
            .map_err(Self::Error::Attribute)
    }
    fn load_scalar(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        self.attributes.lock().map_err(|_| Self::Error::Poisoned("Pumping attributes"))?
            .load_for_player(pointer.as_ref(), NO_INTENTION_SCALE).map_err(Self::Error::Attribute)
    }
    fn load_enabled(&mut self, pointer: &Self::Pointer) -> Result<bool, Self::Error> {
        let Some(data) = pointer else {
            return Ok(false);
        };
        let bytes = data.bytes();
        let [byte] = bytes else {
            return Err(Self::Error::Attribute(AttributeRuntimeError::PayloadSize {
                key: ENABLED,
                expected: 1,
                actual: bytes.len(),
            }));
        };
        Ok(*byte != 0)
    }
    fn mass(&mut self) -> Result<f32, Self::Error> {
        self.read("mass970", SkeletonProcessedContact::physical_mass)
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.read("DT93c", |p| p.physics_state_fields().time_delta)
    }
    fn z_axis_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("Z axis velocity1b0", |p| {
            p.toolkit_state().z_axis_velocity()
        })
    }
}
