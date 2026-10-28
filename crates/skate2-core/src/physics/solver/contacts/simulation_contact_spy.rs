//! XYZ projection of solved `ContactSpy` output, `SpyContactJacobians`
//! 0x82ae4a88..0x82ae4bf8. The sealed batch supplies the original Contact and
//! A/B body occurrences; no numeric body tokens become native addresses.
//!
//! Native frame W lanes contain Body pointers/tag bits. Position and force W
//! arithmetic inherits that packed provenance, so this projection omits W
//! rather than inventing canonical values. Ordinary host f32/FMA arithmetic
//! is not a Xenon denormal or NaN-payload parity claim. The actual exporter
//! owns source selection, B-before-A COM reads, ordered compaction, and the
//! one-export-per-completed-frame boundary; this is not Island list creation.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained solved-spy output awaits Island contact-list consumer"
    )
)]

use super::{
    SimulationContactJacobian,
    network_preparation::{
        SimulationContactIdentity, SimulationContactTableEndpoint,
        SimulationManagerContactAuthority, SimulationManagerContactDomain,
    },
};
use crate::simulation_rigid_body_pool::{
    SimulationRigidBodyFrameIdentity, SimulationRigidBodyObservation, SimulationRigidBodyPool,
    SimulationRigidBodyPoolAuthority, SimulationRigidBodyPoolError,
};
use std::fmt;

#[cfg(test)]
#[path = "simulation_contact_spy_tests.rs"]
mod tests;

/// Retained solved values with original body-role and Contact provenance.
/// Cloning a record does not grant authority to construct a validated frame.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SimulationContactSpy {
    contact: SimulationContactIdentity,
    body_a: SimulationRigidBodyObservation,
    body_b: SimulationRigidBodyObservation,
    frame: [[f32; 3]; 4],
    normal_force: [f32; 3],
    friction_force: [f32; 3],
    tag: u32,
}

impl SimulationContactSpy {
    /// Raw branches 0x82ae4af4/4b0c: a spy bit and strictly positive normal
    /// impulse. Unordered values skip; infinity is not a separate refusal.
    pub(super) fn selects(jacobian: &SimulationContactJacobian) -> bool {
        jacobian.nb_word & 8 != 0 && jacobian.position_impulse[0] > 0.0
    }

    /// Original A/B identity is supplied by the sealed compiled record, never
    /// inferred from reordered solver RF roles. COM values were read B then A
    /// by the exporter. `frequency_squared` was evaluated once for the batch.
    pub(super) fn from_solved(
        contact: SimulationContactIdentity,
        body_a: SimulationRigidBodyObservation,
        body_b: SimulationRigidBodyObservation,
        jacobian: &SimulationContactJacobian,
        frequency_squared: f32,
        center_a: [f32; 3],
        center_b: [f32; 3],
    ) -> Self {
        let vectors = solved_vectors(jacobian, frequency_squared, center_a, center_b);
        Self {
            contact,
            body_a,
            body_b,
            frame: vectors.frame,
            normal_force: vectors.normal_force,
            friction_force: vectors.friction_force,
            tag: jacobian.tag_word,
        }
    }

    pub(crate) fn contact(&self) -> SimulationContactIdentity {
        self.contact
    }
    pub(crate) fn body_a(&self) -> SimulationRigidBodyObservation {
        self.body_a
    }
    pub(crate) fn body_b(&self) -> SimulationRigidBodyObservation {
        self.body_b
    }
    pub(crate) fn frame(&self) -> &[[f32; 3]; 4] {
        &self.frame
    }
    pub(crate) fn normal_force(&self) -> &[f32; 3] {
        &self.normal_force
    }
    pub(crate) fn friction_force(&self) -> &[f32; 3] {
        &self.friction_force
    }
    pub(crate) fn tag(&self) -> u32 {
        self.tag
    }
}

struct SpyVectors {
    frame: [[f32; 3]; 4],
    normal_force: [f32; 3],
    friction_force: [f32; 3],
}

/// Shared by the retained producer and private numeric discriminators. This
/// helper cannot produce a Contact/body identity or frame publication.
fn solved_vectors(
    jacobian: &SimulationContactJacobian,
    frequency_squared: f32,
    center_a: [f32; 3],
    center_b: [f32; 3],
) -> SpyVectors {
    let point_b: [f32; 3] =
        std::array::from_fn(|lane| center_b[lane] + jacobian.attachment_offset_b[lane]);
    let point_a: [f32; 3] =
        std::array::from_fn(|lane| center_a[lane] + jacobian.attachment_offset_a[lane]);
    let weighted_b: [f32; 3] = point_b.map(|value| value * jacobian.inverse_mass_b);
    let numerator: [f32; 3] = std::array::from_fn(|lane| {
        point_a[lane].mul_add(jacobian.inverse_mass_a, weighted_b[lane])
    });
    let inverse = 1.0 / (jacobian.inverse_mass_b + jacobian.inverse_mass_a);
    let position = numerator.map(|value| value * inverse);
    let normal_impulse = jacobian
        .contact_right
        .map(|value| value * jacobian.position_impulse[0]);
    let normal_force = normal_impulse.map(|value| value * frequency_squared);
    let tangent_z: [f32; 3] = jacobian
        .contact_at
        .map(|value| value * jacobian.position_impulse[2]);
    let friction_impulse: [f32; 3] = std::array::from_fn(|lane| {
        jacobian.contact_up[lane].mul_add(jacobian.position_impulse[1], tangent_z[lane])
    });
    let friction_force = friction_impulse.map(|value| value * frequency_squared);
    SpyVectors {
        frame: [
            jacobian.contact_right,
            jacobian.contact_up,
            jacobian.contact_at,
            position,
        ],
        normal_force,
        friction_force,
    }
}

/// Actual exported frame ownership; no public arbitrary-record constructor.
/// Native compacts destructively into CJ storage; retaining this immutable Vec
/// instead preserves values safely without claiming that storage alias model.
#[derive(Debug)]
pub(crate) struct SimulationContactSpyFrame {
    manager: SimulationManagerContactAuthority,
    body_pool: SimulationRigidBodyPoolAuthority,
    frame: SimulationRigidBodyFrameIdentity,
    records: Vec<SimulationContactSpy>,
}

impl SimulationContactSpyFrame {
    pub(super) fn new(
        manager: SimulationManagerContactAuthority,
        body_pool: SimulationRigidBodyPoolAuthority,
        frame: SimulationRigidBodyFrameIdentity,
        records: Vec<SimulationContactSpy>,
    ) -> Self {
        Self {
            manager,
            body_pool,
            frame,
            records,
        }
    }

    pub(crate) fn records(&self) -> &[SimulationContactSpy] {
        &self.records
    }

    /// Revalidates manager, pool, frame, and every retained original A/B
    /// occurrence. This is host lifetime safety, not a native address check.
    pub(crate) fn try_validate_current(
        &self,
        manager: &SimulationManagerContactDomain,
        pool: &SimulationRigidBodyPool,
    ) -> Result<(), SimulationContactSpyError> {
        if self.manager != manager.assembly_pool_authority() {
            return Err(SimulationContactSpyError::ManagerMismatch);
        }
        if self.body_pool != pool.authority() {
            return Err(SimulationContactSpyError::BodyPoolMismatch);
        }
        if pool.frame_identity() != Some(self.frame) {
            return Err(SimulationContactSpyError::FrameMismatch);
        }
        for record in &self.records {
            for (side, body) in [
                (SimulationContactTableEndpoint::BodyA, record.body_a),
                (SimulationContactTableEndpoint::BodyB, record.body_b),
            ] {
                pool.try_validate_observation(body).map_err(|source| {
                    SimulationContactSpyError::Body {
                        contact: record.contact,
                        side,
                        source,
                    }
                })?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SimulationContactSpyError {
    ManagerMismatch,
    BodyPoolMismatch,
    FrameMismatch,
    PhaseMismatch,
    BatchMismatch,
    Allocation {
        count: usize,
    },
    Body {
        contact: SimulationContactIdentity,
        side: SimulationContactTableEndpoint,
        source: SimulationRigidBodyPoolError,
    },
}

impl fmt::Display for SimulationContactSpyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManagerMismatch => f.write_str("ContactSpy manager mismatch"),
            Self::BodyPoolMismatch => f.write_str("ContactSpy body pool mismatch"),
            Self::FrameMismatch => f.write_str("ContactSpy frame mismatch"),
            Self::PhaseMismatch => {
                f.write_str("ContactSpy requires the completed solve/integration phase")
            }
            Self::BatchMismatch => {
                f.write_str("ContactSpy requires the exact completed Contact batch")
            }
            Self::Allocation { count } => {
                write!(f, "ContactSpy allocation failed for {count} records")
            }
            Self::Body {
                contact,
                side,
                source,
            } => write!(f, "ContactSpy {contact:?} {side:?}: {source}"),
        }
    }
}
impl std::error::Error for SimulationContactSpyError {}
