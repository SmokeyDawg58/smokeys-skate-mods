//! Actual board ground/contact output 0x82b30288..0x82b30658.
use super::super::{
    PhysOutAccessError, PhysOutComponent, PhysOutGroundVector, PhysicalPlayerPhysOut,
};
use super::ConcretePhysOutError;
use crate::{
    skeleton_collision_numeric::SkeletonCollisionNumeric,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::convert::Infallible;
pub(crate) trait BoardFillGroundSource {
    type Error;
    type Body;
    fn capture_body(&mut self) -> Result<Self::Body, Self::Error>;
    fn time_since_wheels(&mut self, body: &Self::Body) -> Result<f32, Self::Error>;
    fn dynamic_up(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn last_valid_normal(&mut self, body: &Self::Body) -> Result<SkeletonRootVector, Self::Error>;
    fn contact_acceleration(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn average_normal(&mut self, body: &Self::Body) -> Result<SkeletonRootVector, Self::Error>;
    fn num_wheels(&mut self, body: &Self::Body) -> Result<u8, Self::Error>;
    fn in_contact(&mut self, body: &Self::Body, part: usize) -> Result<bool, Self::Error>;
    fn deck_normal(&mut self, body: &Self::Body) -> Result<SkeletonRootVector, Self::Error>;
    fn deck_acceleration(&mut self, body: &Self::Body) -> Result<SkeletonRootVector, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum BoardGroundOutputError<S, N> {
    Source(S),
    Numeric(N),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for BoardGroundOutputError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "board ground source: {e:?}"),
            Self::Numeric(e) => write!(f, "board ground numeric: {e:?}"),
            Self::Access(e) => write!(f, "board ground guard: {e:?}"),
            Self::Component(e) => write!(f, "board ground component: {e:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    #[expect(
        clippy::too_many_lines,
        reason = "single ordered native output block preserves distinct captures and assertion positions"
    )]
    pub(crate) fn try_fill_board_ground<S: BoardFillGroundSource, N: SkeletonCollisionNumeric>(
        &mut self,
        source: &mut S,
        numeric: &mut N,
    ) -> Result<(), BoardGroundOutputError<S::Error, N::Error>> {
        use BoardGroundOutputError as E;
        let body = source.capture_body().map_err(E::Source)?; //30294
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let time = source.time_since_wheels(&body).map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Air)
            .map_err(E::Component)?;
        self.concrete
            .air
            .as_mut()
            .expect("guarded Air")
            .time_since_wheels_touching = time;
        let guard = self.sample_write_guard().map_err(E::Access)?; //302c4
        self.check_write_guard(&guard).map_err(E::Access)?;
        let up = source.dynamic_up().map_err(E::Source)?; //302ec
        self.publish_ground_vector(&guard, PhysOutGroundVector::DynamicUp, up)
            .map_err(E::Access)?;
        let body = source.capture_body().map_err(E::Source)?; //30304
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let up = source.last_valid_normal(&body).map_err(E::Source)?;
        self.publish_ground_vector(&guard, PhysOutGroundVector::GroundUp, up)
            .map_err(E::Access)?;
        let body = source.capture_body().map_err(E::Source)?; //30344 fresh call, no reuse
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let up = source.last_valid_normal(&body).map_err(E::Source)?;
        self.publish_ground_vector(&guard, PhysOutGroundVector::LastValidGroundUp, up)
            .map_err(E::Access)?;
        self.check_current_guard().map_err(E::Access)?; //30374
        self.require_complete_component(PhysOutComponent::Ground)
            .map_err(E::Component)?;
        let acceleration = source.contact_acceleration().map_err(E::Source)?; //30398
        let guard = self.sample_write_guard().map_err(E::Access)?; //303a0
        let up = self.ground_vectors[PhysOutGroundVector::GroundUp as usize]
            .ok_or(E::Access(PhysOutAccessError::NotConstructed))?
            .value;
        let force = numeric.dot3(acceleration.xyz, up.xyz).map_err(E::Numeric)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.concrete
            .ground
            .as_mut()
            .expect("guarded Ground")
            .ground_force = force;
        let body = source.capture_body().map_err(E::Source)?; //303e8
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let normal = source.average_normal(&body).map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.require_complete_component(PhysOutComponent::Collision)
            .map_err(E::Component)?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded Collision")
            .average_ground_normal = normal;
        let body = source.capture_body().map_err(E::Source)?; //30428
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let count = source.num_wheels(&body).map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded Collision")
            .wheels_in_contact = i32::from(count);
        let body = source.capture_body().map_err(E::Source)?; //30468
        let back = source.in_contact(&body, 5).map_err(E::Source)?;
        let truck = if back {
            true
        } else {
            let body = source.capture_body().map_err(E::Source)?; //30488 only if false
            source.in_contact(&body, 4).map_err(E::Source)?
        };
        self.check_current_guard().map_err(E::Access)?; //304a0 after short circuit
        self.concrete
            .collision
            .as_mut()
            .expect("guarded Collision")
            .a_truck_in_contact = truck;
        let body = source.capture_body().map_err(E::Source)?; //304d8
        let guard = self.sample_write_guard().map_err(E::Access)?;
        let deck = source.in_contact(&body, 6).map_err(E::Source)?;
        self.check_write_guard(&guard).map_err(E::Access)?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded Collision")
            .deck_in_contact = deck;
        // Native reads already published fields, short-circuiting their guards.
        self.check_current_guard().map_err(E::Access)?;
        let wheels = self
            .concrete
            .collision
            .as_ref()
            .expect("guarded Collision")
            .wheels_in_contact;
        let anything = if wheels > 0 {
            true
        } else {
            self.check_current_guard().map_err(E::Access)?;
            if self
                .concrete
                .collision
                .as_ref()
                .expect("guarded Collision")
                .a_truck_in_contact
            {
                true
            } else {
                self.check_current_guard().map_err(E::Access)?;
                self.concrete
                    .collision
                    .as_ref()
                    .expect("guarded Collision")
                    .deck_in_contact
            }
        };
        self.check_current_guard().map_err(E::Access)?;
        self.concrete
            .collision
            .as_mut()
            .expect("guarded Collision")
            .anything_in_contact = anything;
        let body = source.capture_body().map_err(E::Source)?; //305d8 fresh validity
        let contact_force = if source.in_contact(&body, 6).map_err(E::Source)? {
            let normal = source.deck_normal(&body).map_err(E::Source)?;
            let acceleration = source.deck_acceleration(&body).map_err(E::Source)?;
            let dot = numeric
                .dot3(acceleration.xyz, normal.xyz)
                .map_err(E::Numeric)?;
            deck_force_from_dot(dot)
        } else {
            0.0
        };
        self.check_current_guard().map_err(E::Access)?; //30630 after calculation
        self.concrete
            .collision
            .as_mut()
            .expect("guarded Collision")
            .deck_contact_force = contact_force;
        Ok(())
    }
}
fn deck_force_from_dot(dot: f32) -> f32 {
    let magnitude = f32::from_bits(dot.to_bits() & 0x7fff_ffff);
    let scaled = magnitude * f32::from_bits(0x3aa3_d70a);
    let positive = if -scaled >= 0.0 { 0.0 } else { scaled };
    if 1.0 - positive >= 0.0 { positive } else { 1.0 }
}
#[cfg(test)]
#[path = "physical_player_phys_out_board_fill_ground_tests.rs"]
mod tests;
