//! Source `Intents::Insert` (`0x824d0c48`) and its fixed 64-node map.
//! No generated graph execution or native allocator addresses are represented.
use super::ActorGraphConstructionError;
use crate::{action_graph_input_listener::ActionInputIntentSink, play_animation::FastString36};

#[cfg(test)]
#[path = "actor_graph_intents_tests.rs"]
mod tests;

const BUCKETS: usize = 61;
const CAPACITY: usize = 64;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ActorGraphIntentError {
    /// Native `DoAllocateNode` `0x824cdf78..80` dereferences null on exhaustion.
    /// This checked host error retains the prior map instead of crashing.
    Capacity,
}

pub(crate) struct ActorGraphIntent {
    pub(crate) name: FastString36,
    pub(crate) value: f32,
}
struct Node {
    intent: ActorGraphIntent,
    next: Option<usize>,
}

/// Constructor `0x825fd668`: previous prime of 65 is 61; max load factor
/// 10000 prevents rehash within the 64-node domain. Each new unique node is
/// prepended to its bucket (`0x824d0598..05b4`). No per-insert allocation.
pub(crate) struct ActorGraphIntents {
    nodes: Vec<Node>,
    heads: [Option<usize>; BUCKETS],
}
impl ActorGraphIntents {
    pub(crate) fn try_new<E>() -> Result<Self, ActorGraphConstructionError<E>> {
        let mut nodes = Vec::new();
        nodes
            .try_reserve_exact(CAPACITY)
            .map_err(|_| ActorGraphConstructionError::Allocation)?;
        Ok(Self {
            nodes,
            heads: [None; BUCKETS],
        })
    }
    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }
    #[expect(
        clippy::unused_self,
        reason = "fixed capacity is queried alongside the retained store length"
    )]
    pub(crate) const fn maximum(&self) -> usize {
        CAPACITY
    }

    /// `FastString<36>::operator=` `0x824cd990` recursively assigns six
    /// words via `FastString<6>::Assign` `0x824b1a78`. This source byte mapping
    /// sign-extends high bytes; it neither validates characters nor rejects
    /// long input. NUL terminates and input beyond 36 bytes is discarded.
    pub(crate) fn native_name(bytes: &[u8]) -> FastString36 {
        let mut words = [0_u32; 6];
        for (index, byte) in bytes.iter().copied().take(36).enumerate() {
            if byte == 0 {
                break;
            }
            let signed = i32::from(i8::from_ne_bytes([byte]));
            let digit = if signed >= 97 {
                signed - 86
            } else if signed <= 57 {
                signed - 47
            } else if signed <= 90 {
                signed - 54
            } else {
                signed - 58
            };
            let multiplier = 38_u32.pow(5 - u32::try_from(index % 6).expect("six-character word"));
            words[index / 6] = words[index / 6]
                .wrapping_add(u32::from_ne_bytes(digit.to_ne_bytes()).wrapping_mul(multiplier));
        }
        FastString36 { words }
    }

    fn bucket(name: &FastString36) -> usize {
        // Raw `0x824d04c0..050c`: six wrapping adds, then unsigned remainder.
        usize::try_from(name.words.iter().copied().fold(0_u32, u32::wrapping_add) % 61)
            .expect("bucket fits usize")
    }
    fn find(&self, name: &FastString36) -> Option<usize> {
        let mut current = self.heads[Self::bucket(name)];
        while let Some(index) = current {
            let node = &self.nodes[index];
            // Shared `DoFindNode` `0x824cf258` compares all six words.
            if node.intent.name == *name {
                return Some(index);
            }
            current = node.next;
        }
        None
    }
    /// Unique insertion `0x824d04a0`: a duplicate returns the existing node
    /// without changing value, chain position or size, even at full capacity.
    pub(crate) fn insert_unique(
        &mut self,
        name: FastString36,
        value: f32,
    ) -> Result<bool, ActorGraphIntentError> {
        if self.find(&name).is_some() {
            return Ok(false);
        }
        if self.nodes.len() == CAPACITY {
            return Err(ActorGraphIntentError::Capacity);
        }
        let bucket = Self::bucket(&name);
        let index = self.nodes.len();
        self.nodes.push(Node {
            intent: ActorGraphIntent { name, value },
            next: self.heads[bucket],
        });
        self.heads[bucket] = Some(index);
        Ok(true)
    }
    /// Source operator[] followed by store in Actor825fdb04/825fdb30. Existing
    /// nodes retain their bucket position; insertion still obeys fixed capacity.
    pub(crate) fn insert_or_assign(
        &mut self,
        name: FastString36,
        value: f32,
    ) -> Result<(), ActorGraphIntentError> {
        if let Some(index) = self.find(&name) {
            self.nodes[index].intent.value = value;
            return Ok(());
        }
        self.insert_unique(name, value).map(|_| ())
    }

    /// Lookup shared with `HasIntent` `0x825fa248`; preserves stored float bits.
    pub(crate) fn get(&self, name: &FastString36) -> Option<f32> {
        self.find(name).map(|index| self.nodes[index].intent.value)
    }
    /// Native fixed-map erase used by `CreateIntent`82cd5474 and
    /// `RemoveIntent`82cc1f1c. Preserve every surviving bucket-chain order.
    pub(crate) fn erase(&mut self, name: &FastString36) -> bool {
        let Some(index) = self.find(name) else {
            return false;
        };
        let successor = self.nodes[index].next;
        for head in &mut self.heads {
            if *head == Some(index) {
                *head = successor;
            }
        }
        for node in &mut self.nodes {
            if node.next == Some(index) {
                node.next = successor;
            }
        }
        // Host compaction changes storage indices only, never traversal order.
        self.nodes.remove(index);
        for link in self
            .heads
            .iter_mut()
            .chain(self.nodes.iter_mut().map(|node| &mut node.next))
        {
            if let Some(old) = *link {
                if old > index {
                    *link = Some(old - 1);
                }
            }
        }
        true
    }
    /// `DoFreeNodes` `0x82c969e0`, followed by caller count zero. Retains host
    /// capacity. Native freelist addresses are not observable through this API.
    pub(crate) fn clear(&mut self) {
        self.nodes.clear();
        self.heads.fill(None);
    }
    /// `GetSyncLog` `0x825f6340`: ascending buckets, following each head chain.
    /// This is traversal order, not insertion order or lexical key order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &ActorGraphIntent> {
        let mut bucket = 0;
        let mut next = None;
        std::iter::from_fn(move || {
            while next.is_none() {
                if bucket == BUCKETS {
                    return None;
                }
                next = self.heads[bucket];
                bucket += 1;
            }
            let node = &self.nodes[next.expect("nonempty bucket")];
            next = node.next;
            Some(&node.intent)
        })
    }
}
impl ActionInputIntentSink for ActorGraphIntents {
    type Error = ActorGraphIntentError;
    fn insert(&mut self, name: &str, value: f32) -> Result<(), Self::Error> {
        self.insert_unique(Self::native_name(name.as_bytes()), value)
            .map(|_| ())
    }
}
