//! Ascending board wheel/contact output82b2f950..fc20, before deck-line output.
use super::super::{
    PhysOutAccessError, PhysOutComponent, PhysOutPublicationError, PhysicalPlayerPhysOut,
};
use super::ConcretePhysOutError;
use crate::skeleton_root_transforms::SkeletonRootVector;
use std::convert::Infallible;

pub(crate) trait BoardFillContactSource {
    type Error;
    type Contacts;
    fn capture_contacts(&mut self) -> Result<Self::Contacts, Self::Error>;
    fn in_contact(&mut self, contacts: &Self::Contacts, part: usize) -> Result<bool, Self::Error>;
    fn wheel_velocity(&mut self, wheel: usize) -> Result<SkeletonRootVector, Self::Error>;
    fn position(
        &mut self,
        contacts: &Self::Contacts,
        part: usize,
    ) -> Result<SkeletonRootVector, Self::Error>;
    fn normal(
        &mut self,
        contacts: &Self::Contacts,
        part: usize,
    ) -> Result<SkeletonRootVector, Self::Error>;
    fn audio(&mut self, contacts: &Self::Contacts, part: usize) -> Result<u32, Self::Error>;
    fn pattern(&mut self, contacts: &Self::Contacts, part: usize) -> Result<u32, Self::Error>;
    fn overall_surface(&mut self, contacts: &Self::Contacts) -> Result<u32, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum BoardFillContactsError<S> {
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
    Source(S),
    Surface(PhysOutPublicationError<S>),
}
impl<S: std::fmt::Debug> std::fmt::Display for BoardFillContactsError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(e) => write!(f, "board contact output guard: {e:?}"),
            Self::Component(e) => write!(f, "board contact output component: {e:?}"),
            Self::Source(e) => write!(f, "board contact output source: {e:?}"),
            Self::Surface(e) => write!(f, "board surface output: {e:?}"),
        }
    }
}

impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_board_contacts<S: BoardFillContactSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), BoardFillContactsError<S::Error>> {
        use BoardFillContactsError as E;
        let contacts = source.capture_contacts().map_err(E::Source)?;
        for wheel in 0..4 {
            self.try_fill_one_board_wheel(source, &contacts, wheel)?;
        }
        self.guard_board_collision()?;
        let front = source.in_contact(&contacts, 4).map_err(E::Source)?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded")
            .front_truck_in_contact = front;
        self.guard_board_collision()?;
        let back = source.in_contact(&contacts, 5).map_err(E::Source)?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded")
            .back_truck_in_contact = back;
        self.guard_board_collision()?;
        let front = source.audio(&contacts, 4).map_err(E::Source)?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded")
            .front_truck_audio_surface = signed_word(front);
        self.guard_board_collision()?;
        let back = source.audio(&contacts, 5).map_err(E::Source)?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded")
            .back_truck_audio_surface = signed_word(back);
        self.guard_board_collision()?;
        let deck = source.audio(&contacts, 6).map_err(E::Source)?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded")
            .deck_audio_surface = signed_word(deck);
        // Reuse the existing exact guarded fc20 publication and original owner.
        self.try_publish_collision_type(|| source.overall_surface(&contacts))
            .map_err(E::Surface)
    }

    fn try_fill_one_board_wheel<S: BoardFillContactSource>(
        &mut self,
        source: &mut S,
        contacts: &S::Contacts,
        wheel: usize,
    ) -> Result<(), BoardFillContactsError<S::Error>> {
        use BoardFillContactsError as E;
        // f994 latch precedes both f998 contact byte and f9ac body velocity.
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let contact = source.in_contact(contacts, wheel).map_err(E::Source)?;
        let velocity = source.wheel_velocity(wheel).map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Motion)
            .map_err(E::Component)?;
        self.concrete
            .motion
            .as_mut()
            .expect("guarded")
            .wheel_linear_velocity[wheel] = velocity;
        self.guard_board_collision()?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded")
            .wheel_in_contact[wheel] = contact;
        self.guard_board_collision()?;
        let position = if contact {
            source.position(contacts, wheel).map_err(E::Source)?
        } else {
            SkeletonRootVector::known([0.0; 4])
        };
        self.concrete
            .collision
            .as_mut()
            .expect("guarded")
            .wheel_contact_position[wheel] = position;
        self.guard_board_collision()?;
        let normal = if contact {
            source.normal(contacts, wheel).map_err(E::Source)?
        } else {
            SkeletonRootVector::known([0.0; 4])
        };
        self.concrete
            .collision
            .as_mut()
            .expect("guarded")
            .wheel_contact_normal[wheel] = normal;
        self.guard_board_collision()?;
        let audio = source.audio(contacts, wheel).map_err(E::Source)?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded")
            .wheel_audio_surface[wheel] = signed_word(audio);
        self.guard_board_collision()?;
        let pattern = source.pattern(contacts, wheel).map_err(E::Source)?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded")
            .wheel_surface_pattern[wheel] = signed_word(pattern);
        Ok(())
    }
    fn guard_board_collision<S>(&self) -> Result<(), BoardFillContactsError<S>> {
        self.check_current_guard()
            .map_err(BoardFillContactsError::Access)?;
        self.require_complete_component(PhysOutComponent::Collision)
            .map_err(BoardFillContactsError::Component)
    }
}
fn signed_word(value: u32) -> i32 {
    i32::from_ne_bytes(value.to_ne_bytes())
}

#[cfg(test)]
#[path = "physical_player_phys_out_board_fill_contacts_tests.rs"]
mod tests;
