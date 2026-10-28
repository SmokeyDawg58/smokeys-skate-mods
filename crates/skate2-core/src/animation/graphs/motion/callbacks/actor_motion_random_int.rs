//! `Actor::GetRandomInt` 0x825ed0d0 (Entity vtable +0x1c): the Actor's
//! Motion graph (+0x1f8, vtable +0xd0) `SkaterMotionGraph::GetRandomInt`
//! 0x825fe1a0, which is `Random::RandValue` on the graph's RNG at +0xcf4.
use super::{ActorConstruction, MotionUpdateError as E};
use crate::native_skater_random::{NativeRandomGate, NativeSkaterRandom};

impl ActorConstruction {
    pub(crate) fn try_motion_random_int(&self) -> Result<u32, E> {
        let mut actor = self.prefix().map_err(E::Actor)?;
        let state = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .and_then(|g| g.state.as_mut())
            .ok_or(E::MissingOwner("Motion graph RNG"))?;
        let mut random = NativeSkaterRandom {
            seed: state.random_seed,
            index: state.random_index,
            values: state.random_values,
        };
        let value = random
            .try_next(&NativeRandomGate::production_shared())
            .map_err(|_| E::MissingOwner("native Random access is locked"))?;
        state.random_index = random.index;
        state.random_values = random.values;
        Ok(value)
    }
}
