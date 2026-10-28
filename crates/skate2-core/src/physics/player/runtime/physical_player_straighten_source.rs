//! live adapter for source `StraightenOutTorque82b28560`.
use super::physical_heading::runtime::HeadingAttributes;
use crate::{
    attribute_runtime::AttributeRuntimeError,
    skateboard_attributes::{
        SkateboardAttributeClass, SkateboardAttributeError, SkateboardAttributeRead,
        SkateboardAttributes,
    },
    skateboard_brake_drag::SkateboardBrakeDragInputRead,
    skateboard_force_runtime::straighten::StraightenSource,
    skateboard_slide_collision::SkateboardSlideCollisionInputRead,
    skateboard_slide_collision_attributes::{
        SkateboardSelectedSurface, SkateboardSlideCollisionAttributeError, SlideSurfaceField,
    },
    skeleton_processed_contact::ground_force_input::{GroundForceInput, GroundForceInputError},
    skeleton_root_transforms::SkeletonRootVector,
};
#[derive(Debug)]
pub(crate) enum StraightenRuntimeError {
    Input(GroundForceInputError),
    BoardAttribute(SkateboardAttributeError),
    HeadingAttribute(AttributeRuntimeError),
    Surface(SkateboardSlideCollisionAttributeError),
}
impl std::fmt::Display for StraightenRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(e) => e.fmt(f),
            Self::BoardAttribute(e) => e.fmt(f),
            Self::HeadingAttribute(e) => e.fmt(f),
            Self::Surface(e) => e.fmt(f),
        }
    }
}
pub(crate) struct StraightenRuntime<'a> {
    pub(crate) input: GroundForceInput<'a>,
    pub(crate) board_attributes: &'a mut SkateboardAttributes,
    pub(crate) heading: &'a HeadingAttributes,
}
impl StraightenSource for StraightenRuntime<'_> {
    type Error = StraightenRuntimeError;
    type Surface = SkateboardSelectedSurface;
    fn wheel_hardness(&mut self) -> Result<f32, Self::Error> {
        SkateboardSlideCollisionInputRead::wheel_hardness(&mut self.input)
            .map_err(Self::Error::Input)
    }
    fn wheel_minimum(&mut self) -> Result<f32, Self::Error> {
        self.board_attributes
            .read_float(SkateboardAttributeClass::Wheels, 0xdf47_4d0a_659a_7ae3)
            .map_err(Self::Error::BoardAttribute)
    }
    fn capture_surface(&mut self) -> Result<Self::Surface, Self::Error> {
        SkateboardBrakeDragInputRead::capture_surface(&mut self.input).map_err(Self::Error::Input)
    }
    fn surface_duration(&mut self, surface: &Self::Surface) -> Result<f32, Self::Error> {
        surface
            .try_float(SlideSurfaceField::StraightenOutForceTime)
            .map_err(Self::Error::Surface)
    }
    fn heading_graph(&mut self, ratio: f32) -> Result<f32, Self::Error> {
        Ok(self
            .heading
            .capture_torque_graph::<16>(0)
            .map_err(Self::Error::HeadingAttribute)?
            .evaluate(ratio))
    }
    fn body_z(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.input.body_z_axis().map_err(Self::Error::Input)
    }
    fn velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        SkateboardSlideCollisionInputRead::velocity(&mut self.input).map_err(Self::Error::Input)
    }
    fn surface_strength(&mut self, surface: &Self::Surface) -> Result<f32, Self::Error> {
        surface
            .try_straighten_strength()
            .map_err(Self::Error::Surface)
    }
    fn ground_normal(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        SkateboardSlideCollisionInputRead::ground_normal(&mut self.input)
            .map_err(Self::Error::Input)
    }
    fn turn(&mut self) -> Result<f32, Self::Error> {
        let p = self
            .input
            .processed
            .lock()
            .map_err(|_| Self::Error::Input(GroundForceInputError::Poisoned))?;
        p.animation_attribute_fields()
            .turn
            .ok_or(Self::Error::Input(GroundForceInputError::Missing(
                "turn980",
            )))
    }
    fn maximum_turn(&mut self) -> Result<f32, Self::Error> {
        self.heading
            .torque_scalar(0xb494_e9b5_8d1c_8ba2)
            .map_err(Self::Error::HeadingAttribute)
    }
}
