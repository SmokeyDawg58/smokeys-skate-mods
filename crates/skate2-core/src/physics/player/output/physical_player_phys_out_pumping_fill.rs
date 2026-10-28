//! Pumping.Fill82dd4348..447c, actual Update5 call82e04f24.
use super::super::{PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut};
use super::ConcretePhysOutError;
use std::convert::Infallible;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PumpingFillField {
    Absorption,
    GroundAbsorption,
    MinimumCrouch,
    DeckAbsorption,
    Acceleration,
}
#[derive(Debug)]
pub(crate) enum PumpingFillError<E> {
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
    Source(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for PumpingFillError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(error) => write!(f, "Pumping.Fill guard: {error:?}"),
            Self::Component(error) => write!(f, "Pumping.Fill component: {error:?}"),
            Self::Source(error) => write!(f, "Pumping.Fill source: {error:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_pumping<S>(
        &mut self,
        mut read: impl FnMut(PumpingFillField) -> Result<f32, S>,
    ) -> Result<(), PumpingFillError<S>> {
        use PumpingFillError as E;
        self.check_current_guard().map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Skeleton)
            .map_err(E::Component)?;
        self.concrete.skeleton.as_mut().unwrap().pumping_compression = 0.0; //4394
        for field in [
            PumpingFillField::Absorption,
            PumpingFillField::GroundAbsorption,
            PumpingFillField::MinimumCrouch,
            PumpingFillField::DeckAbsorption,
            PumpingFillField::Acceleration,
        ] {
            // For the deck field441c source value precedes lock sampling4420.
            // Other four fields sample the lock byte first, then load source,
            // then act on the sampled assertion result (even when locked).
            let (guard, value) = if field == PumpingFillField::DeckAbsorption {
                let value = read(field).map_err(E::Source)?;
                (self.sample_write_guard().map_err(E::Access)?, value)
            } else {
                let guard = self.sample_write_guard().map_err(E::Access)?;
                (guard, read(field).map_err(E::Source)?)
            };
            self.check_write_guard(&guard).map_err(E::Access)?;
            let component = if field == PumpingFillField::Acceleration {
                PhysOutComponent::Ground
            } else {
                PhysOutComponent::Skeleton
            };
            self.require_complete_component(component)
                .map_err(E::Component)?;
            match field {
                PumpingFillField::Absorption => {
                    self.concrete
                        .skeleton
                        .as_mut()
                        .unwrap()
                        .absorption_compression = value;
                }
                PumpingFillField::GroundAbsorption => {
                    self.concrete
                        .skeleton
                        .as_mut()
                        .unwrap()
                        .ground_normal_compression = value;
                }
                PumpingFillField::MinimumCrouch => {
                    self.concrete
                        .skeleton
                        .as_mut()
                        .unwrap()
                        .ground_normal_min_crouch = value;
                }
                PumpingFillField::DeckAbsorption => {
                    self.concrete
                        .skeleton
                        .as_mut()
                        .unwrap()
                        .deck_angle_compression = value;
                }
                PumpingFillField::Acceleration => {
                    self.concrete.ground.as_mut().unwrap().pumping_acceleration = value;
                }
            }
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "physical_player_phys_out_pumping_fill_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "physical_player_phys_out_pumping_session_tests.rs"]
mod session_tests;
