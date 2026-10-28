//! Same native Scorable as the output records. PhysicsScoring constructors use
//! new_noncleared; Sim allocation constructors retain Some(0) for unwritten
//! fields. Shared algorithms must not erase that allocation provenance.
pub(super) use super::super::scoring::{Scorable, TrickSettings};
