//! Retained native mapping member: ctor82cd4bd8, Init82ccca08, lookup82cc11b0.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "selected manager member awaits full application scheduling"
    )
)]
use crate::{
    actor_graph_construction::ActorGraphIntents, generated_action_load::construct::GestureGroup,
    play_animation::FastString36,
};
use std::sync::Arc;
#[path = "gesture_trick_mapping_debug.rs"]
pub(crate) mod debug;
#[path = "gesture_trick_mapping_records.rs"]
mod records;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TrickIntents {
    pub(crate) ordinary: FastString36,
    pub(crate) mirrored: FastString36,
}
struct Node<V> {
    name: FastString36,
    value: V,
    next: Option<usize>,
}
struct NativeMap<V, const B: usize, const C: usize> {
    nodes: [Option<Node<V>>; C],
    heads: [Option<usize>; B],
    len: usize,
}
impl<V, const B: usize, const C: usize> NativeMap<V, B, C> {
    fn new() -> Self {
        Self {
            nodes: std::array::from_fn(|_| None),
            heads: [None; B],
            len: 0,
        }
    }
    fn bucket(name: &FastString36) -> usize {
        usize::try_from(name.words.iter().copied().fold(0u32, u32::wrapping_add))
            .expect("u32 host index")
            % B
    }
    fn get(&self, name: &FastString36) -> Option<&V> {
        let mut next = self.heads[Self::bucket(name)];
        while let Some(i) = next {
            let n = self.nodes[i].as_ref().expect("linked node");
            if n.name == *name {
                return Some(&n.value);
            }
            next = n.next;
        }
        None
    }
    fn insert(&mut self, name: FastString36, value: V) -> Result<bool, ()> {
        if self.get(&name).is_some() {
            return Ok(false);
        }
        if self.len == C {
            return Err(());
        }
        let bucket = Self::bucket(&name);
        let index = self.len;
        self.nodes[index] = Some(Node {
            name,
            value,
            next: self.heads[bucket],
        });
        self.heads[bucket] = Some(index);
        self.len += 1;
        Ok(true)
    }
    fn iter(&self) -> impl Iterator<Item = (&FastString36, &V)> {
        let mut bucket = 0;
        let mut next = None;
        std::iter::from_fn(move || {
            while next.is_none() {
                if bucket == B {
                    return None;
                }
                next = self.heads[bucket];
                bucket += 1;
            }
            let node = self.nodes[next.expect("head")]
                .as_ref()
                .expect("linked node");
            next = node.next;
            Some((&node.name, &node.value))
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MappingAllocation {
    Host,
    Null,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MappingError<E> {
    Collaborator(E),
    AlreadyStarted,
    NotInitialized,
    Capacity,
    NullPair,
}
impl<E: std::fmt::Debug> std::fmt::Display for MappingError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "gesture mapping: {self:?}")
    }
}

/// Actual selected member, not a successful whole `SkaterManager` constructor.
pub(crate) struct GestureTrickMapping {
    groups: [NativeMap<Option<Arc<TrickIntents>>, 43, 45>; 7],
    held: NativeMap<FastString36, 5, 4>,
    started: bool,
    complete: bool,
}
fn group_index(group: GestureGroup) -> usize {
    match group {
        GestureGroup::Nose => 0,
        GestureGroup::Tail => 1,
        GestureGroup::Nose90 => 2,
        GestureGroup::Tail90 => 3,
        GestureGroup::NoseN90 => 4,
        GestureGroup::TailN90 => 5,
        GestureGroup::Square => 6,
    }
}
impl GestureTrickMapping {
    pub(crate) fn is_initialized(&self) -> bool {
        self.complete
    }
    /// Seven inline fixed maps then held map; Init is the next constructor stage.
    pub(crate) fn new() -> Self {
        Self {
            groups: std::array::from_fn(|_| NativeMap::new()),
            held: NativeMap::new(),
            started: false,
            complete: false,
        }
    }
    pub(crate) fn try_initialize<E>(
        &mut self,
        mut allocate: impl FnMut(usize, usize, u32) -> Result<MappingAllocation, E>,
    ) -> Result<(), MappingError<E>> {
        if self.started {
            return Err(MappingError::AlreadyStarted);
        }
        self.started = true;
        for (index, &(group, key, ordinary, mirrored)) in records::RECORDS.iter().enumerate() {
            let value = match allocate(index, 0x30, 1).map_err(MappingError::Collaborator)? {
                MappingAllocation::Host => Some(Arc::new(TrickIntents {
                    ordinary: ActorGraphIntents::native_name(ordinary.as_bytes()),
                    mirrored: ActorGraphIntents::native_name(mirrored.as_bytes()),
                })),
                MappingAllocation::Null => None,
            };
            self.groups[group_index(group)]
                .insert(ActorGraphIntents::native_name(key.as_bytes()), value)
                .map_err(|()| MappingError::Capacity)?;
        }
        for &(key, value) in &records::HELD {
            self.held
                .insert(
                    ActorGraphIntents::native_name(key.as_bytes()),
                    ActorGraphIntents::native_name(value.as_bytes()),
                )
                .map_err(|()| MappingError::Capacity)?;
        }
        self.complete = true;
        Ok(())
    }
    /// Output copy precedes the live debug read. None matches native null output.
    pub(crate) fn try_gesture_intents<D: debug::GestureMappingDebug>(
        &self,
        intents: &ActorGraphIntents,
        group: GestureGroup,
        output: Option<&mut TrickIntents>,
        debug: &mut D,
    ) -> Result<bool, MappingError<D::Error>> {
        if !self.complete {
            return Err(MappingError::NotInitialized);
        }
        let Some((key, value)) = self.groups[group_index(group)]
            .iter()
            .find(|(key, _)| intents.get(key).is_some())
        else {
            return Ok(false);
        };
        if let Some(output) = output {
            *output = **value.as_ref().ok_or(MappingError::NullPair)?;
        }
        if debug.enabled().map_err(MappingError::Collaborator)? {
            // Native string preparation dereferences the pair even if output was NULL.
            let pair = value.as_deref().ok_or(MappingError::NullPair)?;
            debug
                .after_strings(key, pair)
                .map_err(MappingError::Collaborator)?;
        }
        Ok(true)
    }
    pub(crate) fn try_held_intent(
        &self,
        name: &FastString36,
        output: &mut FastString36,
    ) -> Result<bool, MappingError<std::convert::Infallible>> {
        if !self.complete {
            return Err(MappingError::NotInitialized);
        }
        let Some(value) = self.held.get(name) else {
            return Ok(false);
        };
        *output = *value;
        Ok(true)
    }
}
#[cfg(test)]
#[path = "gesture_trick_mapping_tests.rs"]
mod tests;
