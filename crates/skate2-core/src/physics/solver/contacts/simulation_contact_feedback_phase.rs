//! Actual pool mutations between completed `ContactSpy` export and Island list
//! construction. `ResetStaticReactionForces` is 0x827b26d8..0x827b2738;
//! `FreezeIfNecessary` is 0x82e0be40..0x82e0bec8 and its `FreezeRigidBody`
//! consumer is 0x82ae3298..0x82ae3304.
//!
//! The caller owns `Step_Solver3` ordering: reset RF, sample the CURRENT freezing
//! byte, conditionally sample the manager's unsigned threshold and freeze, then
//! create contact lists. Neither method supplies a cvar/default or executes the
//! missing thaw/partition/scheduler paths. Exclusive pool borrowing prevents
//! these writes while a prepared solver phase still borrows this same pool.

use super::{
    SimulationRigidBodyId, SimulationRigidBodyList, SimulationRigidBodyNode,
    SimulationRigidBodyPool, SimulationRigidBodyPoolError,
};

#[cfg(test)]
#[path = "simulation_contact_feedback_phase_tests.rs"]
mod tests;

impl SimulationRigidBodyPool {
    /// Clears all sixteen RF words of each actual Static-ring body, including
    /// registered state9, following right links AFTER the current body's clear.
    /// No Body force, velocity, inertia, cooldown, state or occurrence changes.
    ///
    /// Native skips null Body.mId RF pointers. Published host slots instead own
    /// one permanent RF row each, matching Initialize's final per-body binding
    /// (0x82ae7c94/0x82ae7cc8); transient null bindings are not exposed. The host
    /// anchor has no Body/RF payload: an empty ring is inert, not a fabricated
    /// nullable row or a claim about uninitialized native tiny-capacity anchors.
    /// Whole topology preflight before writes is explicit safe-host policy.
    pub(crate) fn try_reset_static_reaction_forces(
        &mut self,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        self.validate_integrity()?;
        let anchor = SimulationRigidBodyNode::Anchor(SimulationRigidBodyList::Static);
        let mut node = self.anchors[SimulationRigidBodyList::Static.index()].right;
        while node != anchor {
            let SimulationRigidBodyNode::Body(id) = node else {
                unreachable!("validated Static ring contains only actual bodies");
            };
            let index = id.0 as usize;
            self.solver_delta_rows[index].vectors = [[0.0; 4]; 4];
            node = self.slots[index].links.right;
        }
        Ok(())
    }

    /// Captures the first at most100 qualifying Active bodies, then freezes
    /// that captured list in the same order. The comparison is unsigned >=.
    /// The 100th capture exits without reading its right link.
    ///
    /// `num_frames_before_freezing` is the caller's current manager Params+24
    /// sample, NOT an inferred alias of this pool's cooldown. Native initially
    /// copies that same parameter into Simulation.cooldown, but `FreezeRigidBody`
    /// separately reads CURRENT Simulation.cooldown for each selected body.
    /// Topology is checked before capture/mutation; the fixed capture array does
    /// not introduce allocation failures. The returned count is host observation.
    pub(crate) fn try_freeze_if_necessary(
        &mut self,
        num_frames_before_freezing: u32,
    ) -> Result<usize, SimulationRigidBodyPoolError> {
        self.validate_integrity()?;
        let mut captured = [None; 100];
        let mut count = 0;
        let anchor = SimulationRigidBodyNode::Anchor(SimulationRigidBodyList::Active);
        let mut node = self.anchors[SimulationRigidBodyList::Active.index()].right;
        while node != anchor {
            let SimulationRigidBodyNode::Body(id) = node else {
                unreachable!("validated Active ring contains only actual bodies");
            };
            let slot = &self.slots[id.0 as usize];
            if slot.cooldown >= num_frames_before_freezing {
                captured[count] = Some(id);
                count += 1;
                if count == captured.len() {
                    break;
                }
            }
            node = slot.links.right;
        }
        for id in captured[..count].iter().copied() {
            self.freeze_captured_active_body(id.expect("every captured entry is initialized"));
        }
        Ok(count)
    }

    /// Native `FreezeRigidBody` write set; no public arbitrary-id entry point.
    /// Selection and the exclusive pool borrow prove current Active membership.
    fn freeze_captured_active_body(&mut self, id: SimulationRigidBodyId) {
        let node = SimulationRigidBodyNode::Body(id);
        self.unlink(node);
        self.append_to_tail(node, SimulationRigidBodyList::Frozen);
        let slot = &mut self.slots[id.0 as usize];
        slot.state = (slot.state & 8) | 2;
        slot.cooldown = self.cooldown;
        self.counts.active = self.counts.active.wrapping_sub(1);
        self.counts.frozen = self.counts.frozen.wrapping_add(1);
    }
}
