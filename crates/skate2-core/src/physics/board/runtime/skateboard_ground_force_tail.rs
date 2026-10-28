//! Six real producer calls in `GroundAnimation`'s non-jump force tail,
//! 0x82d8a8b8..0x82d8a9cc. Steering, material update and jump decision precede
//! this stage; this is not the full state update or native scheduling.
use crate::{
    reckoning_numeric::ReckoningNumeric,
    skateboard_brake_attributes::SkateboardBrakeAttributeError,
    skateboard_brake_drag::{
        SkateboardBrakeDragError, SkateboardBrakeDragInputRead, SkateboardBrakeDragServices,
    },
    skateboard_foot_attributes::SkateboardFootAttributeError,
    skateboard_foot_forces::{
        SkateboardFootDebugRead, SkateboardFootForce, SkateboardFootForceError,
        SkateboardFootForceServices, SkateboardFootInputRead,
    },
    skateboard_force_runtime::SkateboardForceRuntime,
    skateboard_slide_collision::{
        SkateboardSlideCollisionError, SkateboardSlideCollisionInputRead,
        SkateboardSlideCollisionServices,
    },
    skateboard_slide_collision_attributes::{
        SkateboardSelectedSurface, SkateboardSlideCollisionAttributeError,
    },
    skeleton_root_transforms::SkeletonRootVector,
};

pub(crate) trait SkateboardGroundForceInput:
    SkateboardFootInputRead
    + SkateboardBrakeDragInputRead<Error = <Self as SkateboardFootInputRead>::Error>
    + SkateboardSlideCollisionInputRead<
        Error = <Self as SkateboardFootInputRead>::Error,
        Surface = SkateboardSelectedSurface,
    >
{
    fn state_timer(&mut self) -> Result<f32, <Self as SkateboardFootInputRead>::Error>;
}

#[derive(Debug)]
pub(crate) enum SkateboardGroundForceError<I, D, N> {
    Foot(SkateboardFootForceError<I, SkateboardFootAttributeError, D, N>),
    Brake(SkateboardBrakeDragError<I, SkateboardBrakeAttributeError>),
    StateTimer(I),
    SlideCollision(SkateboardSlideCollisionError<I, SkateboardSlideCollisionAttributeError, N>),
}
impl<I: std::fmt::Debug, D: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display
    for SkateboardGroundForceError<I, D, N>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Foot(error) => write!(f, "ground foot force: {error:?}"),
            Self::Brake(error) => write!(f, "ground brake/drag: {error:?}"),
            Self::StateTimer(error) => write!(f, "ground state timer: {error:?}"),
            Self::SlideCollision(error) => write!(f, "ground slide/collision: {error}"),
        }
    }
}

pub(crate) enum SkateboardGroundForceSelection {
    Collision(SkateboardFootForce),
    Ordinary {
        front: SkateboardFootForce,
        rear: SkateboardFootForce,
        slide: SkateboardFootForce,
        brake: SkateboardFootForce,
        drag: f32,
    },
}

impl SkateboardForceRuntime {
    #[expect(
        clippy::type_complexity,
        reason = "retains separate input, attribute, debug and numerical failure stages"
    )]
    pub(crate) fn try_calculate_grounded_force_tail<P, D, N>(
        &mut self,
        input: &mut P,
        debug: &mut D,
        numeric: &mut N,
    ) -> Result<
        SkateboardGroundForceSelection,
        SkateboardGroundForceError<<P as SkateboardFootInputRead>::Error, D::Error, N::Error>,
    >
    where
        P: SkateboardGroundForceInput,
        D: SkateboardFootDebugRead,
        N: ReckoningNumeric,
    {
        use SkateboardGroundForceError as E;
        // Host scratch initialization is not a native uninitialized-memory
        // claim. Each successful producer writes its complete returned record.
        let zero = SkateboardFootForce {
            world_space_force: SkeletonRootVector::known([0.0; 4]),
            local_space_position: SkeletonRootVector::known([0.0; 4]),
        };
        let mut front = zero;
        let mut rear = zero;
        let mut brake = zero;
        let mut slide = zero;
        {
            let mut feet = SkateboardFootForceServices {
                input,
                attributes: &mut self.foot_attributes,
                debug,
                numeric,
            };
            feet.try_front(1.0, 0.0, &mut front).map_err(E::Foot)?;
            feet.try_rear(1.0, 0.0, &mut rear).map_err(E::Foot)?;
        }
        SkateboardBrakeDragServices {
            input,
            attributes: &mut self.brake_attributes,
        }
        .try_braking(&mut brake)
        .map_err(E::Brake)?;
        let time = input.state_timer().map_err(E::StateTimer)?;
        SkateboardSlideCollisionServices {
            input,
            attributes: &mut self.slide_collision_attributes,
            numeric,
        }
        .try_slide(time, &mut slide)
        .map_err(E::SlideCollision)?;
        let drag = SkateboardBrakeDragServices {
            input,
            attributes: &mut self.brake_attributes,
        }
        .try_linear_drag()
        .map_err(E::Brake)?;
        // Source explicitly zeroes the collision record after drag. Required
        // torque output is evaluated even though GroundAnimation discards it.
        let mut collision = zero;
        let mut torque = SkeletonRootVector::known([0.0; 4]);
        let collided = SkateboardSlideCollisionServices {
            input,
            attributes: &mut self.slide_collision_attributes,
            numeric,
        }
        .try_collision(&mut collision, &mut torque, None)
        .map_err(E::SlideCollision)?;
        Ok(if collided {
            SkateboardGroundForceSelection::Collision(collision)
        } else {
            SkateboardGroundForceSelection::Ordinary {
                front,
                rear,
                slide,
                brake,
                drag,
            }
        })
    }
}
