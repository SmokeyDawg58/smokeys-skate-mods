//! Registered Contact child; late actual `SpeedModel` prefix loads only.
use super::SkeletonProcessedContact;
use crate::{
    skateboard_force_runtime::entry::speed_model_prefix::SpeedModelPrefixSource,
    skateboard_slide_collision_attributes::{
        SkateboardSelectedSurface, SkateboardSlideCollisionAttributeError,
    },
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum SpeedModelInputError {
    Missing(&'static str),
    Poisoned,
    Surface(SkateboardSlideCollisionAttributeError),
}
impl std::fmt::Display for SpeedModelInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "speed model missing {field}"),
            Self::Poisoned => f.write_str("speed model processed lock poisoned"),
            Self::Surface(error) => write!(f, "speed model surface: {error}"),
        }
    }
}
pub(crate) struct SpeedModelInput<'a>(pub(crate) &'a Arc<Mutex<SkeletonProcessedContact>>);
impl SpeedModelInput<'_> {
    fn read<T>(
        &self,
        field: &'static str,
        get: impl FnOnce(&SkeletonProcessedContact) -> Option<T>,
    ) -> Result<T, SpeedModelInputError> {
        let input = self.0.lock().map_err(|_| SpeedModelInputError::Poisoned)?;
        get(&input).ok_or(SpeedModelInputError::Missing(field))
    }
}
impl SpeedModelPrefixSource for SpeedModelInput<'_> {
    type Error = SpeedModelInputError;
    type Surface = SkateboardSelectedSurface;
    fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
        self.read("force inhibit flags8c4", |p| {
            let (bits, known) = p.known_flags_8c4();
            (known & 0x6201_0000 == 0x6201_0000).then_some(bits)
        })
    }
    fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
        self.read("force inhibit flags8cc", |p| p.physics_state.flags_8cc)
    }
    fn wheel_count(&mut self) -> Result<i32, Self::Error> {
        self.read("numWheels90c", |p| p.physics_state.num_wheels_in_contact)
    }
    fn capture_surface(&mut self) -> Result<Self::Surface, Self::Error> {
        self.read("surface904", |p| p.surface_projection.selected.clone())
    }
    fn forward_speed(&mut self) -> Result<f32, Self::Error> {
        self.read("ZSpeed944", |p| p.prior_output.z_speed().map(|v| v.value))
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.read("DT93c", |p| p.physics_state.time_delta)
    }
    fn maximum_speed_difference(&mut self, surface: &Self::Surface) -> Result<f32, Self::Error> {
        surface
            .try_speed_model_maximum_difference()
            .map_err(Self::Error::Surface)
    }
}
