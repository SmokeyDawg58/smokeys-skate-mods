//! Private frame-local queue for `Sk8::Physics::Island::NetworkContact`.
//!
//! PDB layout and `Island::AddNetworkContact` (`0x827b6f40..0x827b6fb4`)
//! prove a fixed capacity of 432 and a `0x40`-byte element containing Part at
//! `+0x00`, complete four-lane point at `+0x10`, complete four-lane normal at
//! `+0x20`, and displacement at `+0x30`. This host foundation replaces the
//! native Part pointer with an occurrence-qualified selected body and a copied
//! three-float Material value.
//!
//! Requiring the caller to resolve Part-to-body and Material before enqueue,
//! rather than at the later `AddNetworkContacts` drain, is explicit safe-host
//! policy until a checked Part registry exists. This module does not construct
//! Contact records, expose raw Contact bytes, drain or seal a frame, or
//! authorize any non-network producer.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "private foundation for the checked AddNetworkContacts producer"
    )
)]

use crate::simulation_rigid_body_pool::{
    SimulationRigidBodyFrameIdentity, SimulationRigidBodyObservation, SimulationRigidBodyPool,
    SimulationRigidBodyPoolAuthority, SimulationRigidBodyPoolError,
};
use std::{
    error::Error,
    fmt,
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

/// PDB `xfixed_vector<NetworkContact,432,...>` inline capacity.
const SIMULATION_NETWORK_CONTACT_CAPACITY: usize = 432;

static NEXT_NETWORK_CONTACT_QUEUE_AUTHORITY: AtomicU64 = AtomicU64::new(1);

/// Process-unique identity of one private queue owner.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct SimulationNetworkContactQueueAuthority(NonZeroU64);

/// Exact three-scalar `rw::physics::Material` value resolved for a request.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationNetworkContactMaterial {
    pub(crate) static_friction: f32,
    pub(crate) dynamic_friction: f32,
    pub(crate) restitution: f32,
}

/// One unpublished, enqueue-resolved network Contact request.
///
/// Point and normal retain all four copied words. Their W lanes are not
/// normalized to XYZ-only host vectors. Unlike native `NetworkContact`, this
/// value contains no live Part reference and therefore makes no claim of
/// drain-time Part mutation visibility.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SimulationResolvedNetworkContactRequest {
    pub(crate) selected_body: SimulationRigidBodyObservation,
    pub(crate) point: [f32; 4],
    pub(crate) normal: [f32; 4],
    pub(crate) displacement: f32,
    pub(crate) material: SimulationNetworkContactMaterial,
}

/// Typed safe-host refusal from the private network Contact queue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationNetworkContactQueueError {
    AllocationFailed,
    AuthorityExhausted,
    FrameIdentityUnavailable,
    BodyPoolMismatch,
    FrameMismatch,
    CapacityReached,
    Body(SimulationRigidBodyPoolError),
}

impl fmt::Display for SimulationNetworkContactQueueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllocationFailed => {
                formatter.write_str("could not allocate network Contact queue")
            }
            Self::AuthorityExhausted => {
                formatter.write_str("network Contact queue authority space is exhausted")
            }
            Self::FrameIdentityUnavailable => {
                formatter.write_str("rigid-body frame identity is unavailable")
            }
            Self::BodyPoolMismatch => {
                formatter.write_str("network Contact queue body pool does not match")
            }
            Self::FrameMismatch => {
                formatter.write_str("network Contact queue frame does not match")
            }
            Self::CapacityReached => formatter.write_str("network Contact queue is full"),
            Self::Body(source) => write!(formatter, "network Contact body: {source}"),
        }
    }
}

impl Error for SimulationNetworkContactQueueError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Body(source) => Some(source),
            _ => None,
        }
    }
}

/// One non-cloneable, fixed-capacity queue bound to one body frame.
pub(crate) struct SimulationNetworkContactQueue {
    authority: SimulationNetworkContactQueueAuthority,
    body_pool: SimulationRigidBodyPoolAuthority,
    frame: SimulationRigidBodyFrameIdentity,
    entries: Vec<SimulationResolvedNetworkContactRequest>,
}

impl SimulationNetworkContactQueue {
    /// Creates an empty queue for the body's current frame.
    pub(crate) fn try_new(
        body_pool: &SimulationRigidBodyPool,
    ) -> Result<Self, SimulationNetworkContactQueueError> {
        let frame = body_pool
            .frame_identity()
            .ok_or(SimulationNetworkContactQueueError::FrameIdentityUnavailable)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(SIMULATION_NETWORK_CONTACT_CAPACITY)
            .map_err(|_| SimulationNetworkContactQueueError::AllocationFailed)?;
        let authority = fresh_queue_authority()?;
        Ok(Self {
            authority,
            body_pool: body_pool.authority(),
            frame,
            entries,
        })
    }

    #[must_use]
    pub(crate) const fn authority(&self) -> SimulationNetworkContactQueueAuthority {
        self.authority
    }

    #[must_use]
    pub(crate) const fn body_pool_authority(&self) -> SimulationRigidBodyPoolAuthority {
        self.body_pool
    }

    #[must_use]
    pub(crate) const fn frame_identity(&self) -> SimulationRigidBodyFrameIdentity {
        self.frame
    }

    #[must_use]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub(crate) fn entries(&self) -> &[SimulationResolvedNetworkContactRequest] {
        &self.entries
    }

    /// Appends one fully copied private request in caller order.
    ///
    /// Pool/frame, selected-body occurrence, and capacity are all validated
    /// before the queue changes. This failure-atomic ordering is host safety
    /// policy; native fixed-vector growth and pointer stores are unchecked.
    pub(crate) fn try_enqueue(
        &mut self,
        body_pool: &SimulationRigidBodyPool,
        request: SimulationResolvedNetworkContactRequest,
    ) -> Result<(), SimulationNetworkContactQueueError> {
        if self.body_pool != body_pool.authority() {
            return Err(SimulationNetworkContactQueueError::BodyPoolMismatch);
        }
        if body_pool.frame_identity() != Some(self.frame) {
            return Err(SimulationNetworkContactQueueError::FrameMismatch);
        }
        body_pool
            .try_validate_observation(request.selected_body)
            .map_err(SimulationNetworkContactQueueError::Body)?;
        if self.entries.len() == SIMULATION_NETWORK_CONTACT_CAPACITY {
            return Err(SimulationNetworkContactQueueError::CapacityReached);
        }
        self.entries.push(request);
        Ok(())
    }
}

fn fresh_queue_authority()
-> Result<SimulationNetworkContactQueueAuthority, SimulationNetworkContactQueueError> {
    NEXT_NETWORK_CONTACT_QUEUE_AUTHORITY
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .ok()
        .and_then(NonZeroU64::new)
        .map(SimulationNetworkContactQueueAuthority)
        .ok_or(SimulationNetworkContactQueueError::AuthorityExhausted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_matrix() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }

    fn request(
        selected_body: SimulationRigidBodyObservation,
        discriminator: u32,
    ) -> SimulationResolvedNetworkContactRequest {
        SimulationResolvedNetworkContactRequest {
            selected_body,
            point: [
                f32::from_bits(discriminator),
                f32::from_bits(discriminator.wrapping_add(1)),
                f32::from_bits(discriminator.wrapping_add(2)),
                f32::from_bits(discriminator.wrapping_add(3)),
            ],
            normal: [
                f32::from_bits(discriminator.wrapping_add(4)),
                f32::from_bits(discriminator.wrapping_add(5)),
                f32::from_bits(discriminator.wrapping_add(6)),
                f32::from_bits(discriminator.wrapping_add(7)),
            ],
            displacement: f32::from_bits(discriminator.wrapping_add(8)),
            material: SimulationNetworkContactMaterial {
                static_friction: f32::from_bits(discriminator.wrapping_add(9)),
                dynamic_friction: f32::from_bits(discriminator.wrapping_add(10)),
                restitution: f32::from_bits(discriminator.wrapping_add(11)),
            },
        }
    }

    fn request_bits(request: SimulationResolvedNetworkContactRequest) -> [u32; 12] {
        [
            request.point[0].to_bits(),
            request.point[1].to_bits(),
            request.point[2].to_bits(),
            request.point[3].to_bits(),
            request.normal[0].to_bits(),
            request.normal[1].to_bits(),
            request.normal[2].to_bits(),
            request.normal[3].to_bits(),
            request.displacement.to_bits(),
            request.material.static_friction.to_bits(),
            request.material.dynamic_friction.to_bits(),
            request.material.restitution.to_bits(),
        ]
    }

    #[test]
    fn queue_is_frame_bound_unique_and_retains_all_copied_words_in_order() {
        assert_eq!(SIMULATION_NETWORK_CONTACT_CAPACITY, 432);
        let mut bodies = SimulationRigidBodyPool::try_new(1).expect("body pool");
        let body = bodies
            .try_add_rigid_body(identity_matrix(), None, 1)
            .expect("body add")
            .expect("capacity");
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let mut first = SimulationNetworkContactQueue::try_new(&bodies).expect("first queue");
        let second = SimulationNetworkContactQueue::try_new(&bodies).expect("second queue");
        assert_ne!(first.authority(), second.authority());
        assert_eq!(first.body_pool_authority(), bodies.authority());
        assert_eq!(
            first.frame_identity(),
            bodies.frame_identity().expect("frame")
        );
        assert!(first.is_empty());

        let requests = [
            request(body.observation(), 0x7fc0_0041),
            request(body.observation(), 0xff80_0101),
        ];
        for value in requests {
            first.try_enqueue(&bodies, value).expect("enqueue");
        }
        assert_eq!(first.len(), 2);
        assert_eq!(first.entries()[0].selected_body, body.observation());
        assert_eq!(request_bits(first.entries()[0]), request_bits(requests[0]));
        assert_eq!(request_bits(first.entries()[1]), request_bits(requests[1]));
        assert_eq!(first.entries()[0].point[3].to_bits(), 0x7fc0_0044);
        assert_eq!(first.entries()[1].normal[3].to_bits(), 0xff80_0108);
    }

    #[test]
    fn queue_accepts_inactive_body_state_and_refuses_entry_433_without_effect() {
        let mut bodies = SimulationRigidBodyPool::try_new(1).expect("body pool");
        let body = bodies
            .try_add_rigid_body(identity_matrix(), None, 1)
            .expect("static body add")
            .expect("capacity");
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let mut queue = SimulationNetworkContactQueue::try_new(&bodies).expect("queue");
        for index in 0..SIMULATION_NETWORK_CONTACT_CAPACITY {
            queue
                .try_enqueue(
                    &bodies,
                    request(
                        body.observation(),
                        u32::try_from(index).expect("capacity fits u32"),
                    ),
                )
                .expect("within fixed capacity");
        }
        let before = queue.entries().to_vec();
        assert_eq!(
            queue.try_enqueue(&bodies, request(body.observation(), u32::MAX)),
            Err(SimulationNetworkContactQueueError::CapacityReached)
        );
        assert_eq!(queue.entries(), before);
    }

    #[test]
    fn queue_rejects_foreign_stale_and_new_frame_contexts_before_effect() {
        let mut bodies = SimulationRigidBodyPool::try_new(1).expect("body pool");
        let old_body = bodies
            .try_add_rigid_body(identity_matrix(), None, 1)
            .expect("body add")
            .expect("capacity");
        bodies.begin_dynamic_update_frame(1.0 / 60.0);
        let mut queue = SimulationNetworkContactQueue::try_new(&bodies).expect("queue");

        let mut foreign = SimulationRigidBodyPool::try_new(1).expect("foreign pool");
        let foreign_body = foreign
            .try_add_rigid_body(identity_matrix(), None, 1)
            .expect("foreign body add")
            .expect("capacity");
        foreign.begin_dynamic_update_frame(1.0 / 60.0);
        assert_eq!(
            queue.try_enqueue(&foreign, request(foreign_body.observation(), 1)),
            Err(SimulationNetworkContactQueueError::BodyPoolMismatch)
        );
        assert!(queue.is_empty());

        bodies
            .try_remove_rigid_body(old_body)
            .expect("remove old body");
        assert_eq!(
            queue.try_enqueue(&bodies, request(old_body.observation(), 2)),
            Err(SimulationNetworkContactQueueError::Body(
                SimulationRigidBodyPoolError::BodyIsFree { slot: 0 }
            ))
        );
        assert!(queue.is_empty());

        let replacement = bodies
            .try_add_rigid_body(identity_matrix(), None, 1)
            .expect("replacement add")
            .expect("capacity");
        assert_eq!(replacement.id(), old_body.id());
        assert_eq!(
            queue.try_enqueue(&bodies, request(old_body.observation(), 3)),
            Err(SimulationNetworkContactQueueError::Body(
                SimulationRigidBodyPoolError::StaleHandle { slot: 0 }
            ))
        );
        assert!(queue.is_empty());

        assert_eq!(
            queue.try_enqueue(&bodies, request(foreign_body.observation(), 4)),
            Err(SimulationNetworkContactQueueError::Body(
                SimulationRigidBodyPoolError::ForeignHandle
            ))
        );
        assert!(queue.is_empty());

        queue
            .try_enqueue(&bodies, request(replacement.observation(), 5))
            .expect("queue remains reusable after refusal");
        let before_new_frame = queue.entries().to_vec();

        bodies.begin_dynamic_update_frame(1.0 / 120.0);
        assert_eq!(
            queue.try_enqueue(&bodies, request(replacement.observation(), 6)),
            Err(SimulationNetworkContactQueueError::FrameMismatch)
        );
        assert_eq!(queue.entries(), before_new_frame);
    }
}
