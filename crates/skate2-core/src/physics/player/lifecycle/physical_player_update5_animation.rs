//! Actual retained input fields consumed by Update5 animation stores.
use crate::{
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::construct::update5_animation::{
        AnimationFlag, AnimationScalar, Update5AnimationError, Update5AnimationSource,
    },
    skeleton_collision_numeric::{HostSkeletonCollisionNumeric, SkeletonCollisionNumeric},
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum AnimationRuntimeError {
    Poisoned,
    Missing(&'static str),
}
impl std::fmt::Display for AnimationRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("Update5 animation owner poisoned"),
            Self::Missing(field) => write!(f, "Update5 animation requires {field}"),
        }
    }
}
struct Source<'a> {
    owner: &'a Arc<PhysicalPlayerAssemblyOwner>,
    carving_input: Option<Arc<Mutex<SkeletonProcessedContact>>>,
}
impl Source<'_> {
    fn processed<T>(
        &self,
        read: impl FnOnce(&SkeletonProcessedContact) -> Option<T>,
    ) -> Result<T, AnimationRuntimeError> {
        let processed = Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| AnimationRuntimeError::Poisoned)?
                .processed(),
        );
        let processed = processed
            .lock()
            .map_err(|_| AnimationRuntimeError::Poisoned)?;
        read(&processed).ok_or(AnimationRuntimeError::Missing(
            "required retained Processed field or mask",
        ))
    }
}
impl Update5AnimationSource for Source<'_> {
    type Error = AnimationRuntimeError;
    fn capture_carving_input(&mut self) -> Result<(), Self::Error> {
        self.carving_input = Some(Arc::clone(
            self.owner
                .reckoning()
                .lock()
                .map_err(|_| Self::Error::Poisoned)?
                .processed(),
        ));
        Ok(())
    }
    fn scalar(&mut self, field: AnimationScalar) -> Result<f32, Self::Error> {
        self.processed(|p| match field {
            AnimationScalar::Turn => p.animation_attribute_fields().turn,
            AnimationScalar::Spin => p.jump_events().spin(),
            AnimationScalar::TimeSinceInput => p.input_controls().time_since_last_input,
        })
    }
    fn flag(&mut self, field: AnimationFlag) -> Result<bool, Self::Error> {
        use AnimationFlag as F;
        if matches!(field, F::Carving) {
            let input = self
                .carving_input
                .as_ref()
                .ok_or(Self::Error::Missing("captured Processed carving input"))?
                .lock()
                .map_err(|_| Self::Error::Poisoned)?;
            let (bits, known) = input.known_flags_8c4();
            return if known & 0x8000_0000 != 0 {
                Ok(bits & 0x8000_0000 != 0)
            } else {
                Err(Self::Error::Missing("Processed carving8c4"))
            };
        }
        if matches!(
            field,
            F::Mirrored | F::Switch | F::Fakie | F::Regular | F::Jumping
        ) {
            let input = Arc::clone(
                self.owner
                    .animation_physics_input
                    .lock()
                    .map_err(|_| Self::Error::Poisoned)?
                    .as_ref()
                    .ok_or(Self::Error::Missing("actual AnimOutPhysIn2b4"))?,
            );
            let input = input.lock().map_err(|_| Self::Error::Poisoned)?;
            return Ok(match field {
                F::Mirrored => input.flags.mirrored,
                F::Switch => input.flags.riding_switch,
                F::Fakie => input.flags.riding_fakie,
                F::Regular => input.flags.regular_stance,
                _ => input.locomotion_flags & 0x0400_0000 != 0,
            });
        }
        self.processed(|p| match field {
            F::Carving | F::Pushing => {
                let (bits, known) = p.known_flags_8c4();
                let mask = if matches!(field, F::Carving) {
                    0x8000_0000
                } else {
                    0x0800_0000
                };
                (known & mask == mask).then_some(bits & mask != 0)
            }
            F::StandingJump => p
                .physics_state_fields()
                .flags_8cc
                .map(|bits| bits & 0x0010_0000 != 0),
            // Native lbz8c8 reads the high byte of the big-endian word.
            F::NoInput => p.known_flags_8c8().map(|bits| bits & 0x0100_0000 != 0),
            F::Transitioning | F::RunningOut => p.physics_state_fields().flags_8d0.map(|bits| {
                bits & if matches!(field, F::Transitioning) {
                    0x20
                } else {
                    4
                } != 0
            }),
            F::OffBoard => {
                let (bits, known) = p.physics_state_fields().flags_8d4;
                (known & 0x4000_0000 != 0).then_some(bits & 0x4000_0000 != 0)
            }
            _ => unreachable!("animation fields handled above"),
        })
    }
}
pub(crate) fn try_publish_update5_animation(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
) -> Result<
    (),
    Update5AnimationError<
        AnimationRuntimeError,
        <HostSkeletonCollisionNumeric as SkeletonCollisionNumeric>::Error,
    >,
> {
    let mut output = owner
        .phys_out()
        .lock()
        .map_err(|_| Update5AnimationError::Source(AnimationRuntimeError::Poisoned))?;
    output.try_publish_update5_animation(
        &mut Source {
            owner,
            carving_input: None,
        },
        &mut HostSkeletonCollisionNumeric,
    )
}
