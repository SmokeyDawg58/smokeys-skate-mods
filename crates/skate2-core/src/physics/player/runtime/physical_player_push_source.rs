//! Live push reads from the same processed Arc and retained board entry owner.
//! Unregistered until the narrow entry setter and force child are admitted.
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    skateboard_force_runtime::{
        entry::SkateboardEntryState,
        push::{PushScalar, PushSource, attributes::PushAttributes},
    },
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum PushRuntimeError {
    Missing(&'static str),
    Poisoned,
    Attribute(AttributeRuntimeError),
}
impl std::fmt::Display for PushRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "push missing {field}"),
            Self::Poisoned => f.write_str("push processed lock poisoned"),
            Self::Attribute(error) => write!(f, "push attributes: {error}"),
        }
    }
}

pub(crate) struct PushRuntime<'a> {
    pub(crate) processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) entry: &'a mut SkateboardEntryState,
    pub(crate) attributes: &'a PushAttributes,
    pub(crate) owner: &'a crate::physical_player_assembly::PhysicalPlayerAssemblyOwner,
}
impl PushSource for PushRuntime<'_> {
    type Error = PushRuntimeError;
    type Pointer = (u64, Option<AttributeRuntimeData>);
    fn pushing(&mut self) -> Result<bool, Self::Error> {
        let input = self.processed.lock().map_err(|_| Self::Error::Poisoned)?;
        let (bits, known) = input.known_flags_8c4();
        if known & 0x0200_0000 == 0 {
            return Err(Self::Error::Missing("push flag8c4/02000000"));
        }
        Ok(bits & 0x0200_0000 != 0)
    }
    fn dont_push(&mut self) -> Result<bool, Self::Error> {
        let input = self.processed.lock().map_err(|_| Self::Error::Poisoned)?;
        Ok(input
            .known_flags_8c8()
            .ok_or(Self::Error::Missing("flags8c8"))?
            & 0x800
            != 0)
    }
    fn request_abort_push(&mut self) -> Result<(), Self::Error> {
        self.entry.request_abort_push();
        Ok(())
    }
    fn scalar(&mut self, field: PushScalar) -> Result<f32, Self::Error> {
        let input = self.processed.lock().map_err(|_| Self::Error::Poisoned)?;
        let (value, name) = match field {
            PushScalar::AnimationSpeed => (
                input.animation_attribute_fields().push_speed,
                "push speed940",
            ),
            PushScalar::ForwardSpeed => (
                input.prior_output().z_speed().map(|v| v.value),
                "Z speed944",
            ),
            PushScalar::AbsoluteSpeed => {
                (input.toolkit_state().abs_body_speed(), "absolute speed948")
            }
            PushScalar::Mass => (input.physical_mass(), "board mass970"),
            PushScalar::TimeDelta => (input.physics_state_fields().time_delta, "DT93c"),
        };
        value.ok_or(Self::Error::Missing(name))
    }
    fn capture(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        self.attributes.capture(key).map(|pointer|(key,pointer)).map_err(Self::Error::Attribute)
    }
    fn load(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        #[cfg(feature = "skate3-runtime")]
        if pointer.0 == crate::skateboard_force_runtime::push::CHANGE_LIMIT {
            let tuning = self.owner.skate3_tuning.lock().map_err(|_| Self::Error::Poisoned)?.clone();
            if let Some(tuning) = tuning {
                let signed_speed = self.scalar(PushScalar::ForwardSpeed)?;
                let maximum = PushAttributes::load(self.attributes.capture(crate::skateboard_force_runtime::push::SPEED_LIMIT)
                    .map_err(Self::Error::Attribute)?.as_ref()).map_err(Self::Error::Attribute)?;
                return Ok(tuning.push_limit(signed_speed, maximum));
            }
        }
        PushAttributes::load(pointer.1.as_ref()).map_err(Self::Error::Attribute)
    }
    /// `Toolkit_CalcPushAcceleration` 274dc/274ec loads Processed+160
    /// `mBodyZAxis` (flipped deck Z), not the flattened +170 direction.
    fn direction(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .toolkit_state()
            .body_z()
            .ok_or(Self::Error::Missing("body Z axis160"))
    }
}
