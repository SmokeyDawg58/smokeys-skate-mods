//! Frame-local assembly-pair and thaw lists used by Step_Solver2.
//! Native identities are Assembly pointers. Here each borrowed publication is
//! the identity of its actual retained assembly, with its real Part bodies.
//! Host allocation/pipeline job order need not match Xenon pointer/job order.

use crate::{
    authored_collision_publication::AuthoredCollisionPublication,
    serialised_assembly_create::PreparedSerialisedAssembly,
    simulation_rigid_body_pool::SimulationRigidBodyObservation,
};

// World.Init827b0ee8/0f0c -> Params.maxAssemblyPairs(+14)=500.
// SimulationManager ctor82e0c7a8..7d4 allocates 1000 thaw entries.
const PAIR_CAPACITY: usize = 500;
const THAW_CAPACITY: usize = 1000;

#[derive(Clone, Copy)]
pub(crate) struct SimulationCollisionAssembly<'a> {
    key: usize,
    bodies: &'a [SimulationRigidBodyObservation],
}

impl<'a> SimulationCollisionAssembly<'a> {
    pub(crate) fn dmo(assembly: &'a PreparedSerialisedAssembly) -> Self {
        Self {
            key: assembly as *const _ as usize,
            bodies: assembly.rigid_bodies(),
        }
    }

    pub(crate) fn skater(assembly: AuthoredCollisionPublication<'a>) -> Self {
        let key = match assembly {
            AuthoredCollisionPublication::Board(value) => value as *const _ as usize,
            AuthoredCollisionPublication::Skeleton(value) => value as *const _ as usize,
            AuthoredCollisionPublication::Character(value) => value as *const _ as usize,
        };
        Self {
            key,
            bodies: assembly.rigid_bodies(),
        }
    }

    pub(crate) fn key(self) -> usize {
        self.key
    }
    pub(crate) fn bodies(self) -> &'a [SimulationRigidBodyObservation] {
        self.bodies
    }
}

#[derive(Default)]
pub(crate) struct SimulationAssemblyPairFrame<'a> {
    pairs: Vec<[SimulationCollisionAssembly<'a>; 2]>,
    thaw: Vec<SimulationCollisionAssembly<'a>>,
}

impl<'a> SimulationAssemblyPairFrame<'a> {
    pub(crate) fn remaining_pairs(&self) -> usize {
        PAIR_CAPACITY - self.pairs.len()
    }
    pub(crate) fn remaining_thaws(&self) -> usize {
        THAW_CAPACITY - self.thaw.len()
    }
    pub(crate) fn append_pair(
        &mut self,
        a: SimulationCollisionAssembly<'a>,
        b: SimulationCollisionAssembly<'a>,
    ) {
        if self.remaining_pairs() != 0 {
            // 827cc7b4..7e4 sorts the actual Assembly pointer pair before Store.
            self.pairs.push(if a.key < b.key { [a, b] } else { [b, a] });
        }
    }
    pub(crate) fn append_thaw(&mut self, assembly: SimulationCollisionAssembly<'a>) {
        if self.remaining_thaws() != 0 {
            self.thaw.push(assembly);
        }
    }
    pub(crate) fn pairs(&self) -> &[[SimulationCollisionAssembly<'a>; 2]] {
        &self.pairs
    }
    pub(crate) fn thaw(&self) -> &[SimulationCollisionAssembly<'a>] {
        &self.thaw
    }
}
