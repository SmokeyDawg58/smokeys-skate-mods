//! Native channel dictionary order for the selected six-channel owner.
//! Init82d7cc68, insert82d7c198, rehash82d7b520, erase82d71d40.
use crate::play_animation::FastString36;

const MAX_CHANNELS: usize = 6;
const MAX_BUCKETS: usize = 11;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ChannelDictionaryError {
    /// Outside the actual selected InitChannelAnimatable(maxChannels=6) domain.
    Capacity,
}

struct Node {
    name: FastString36,
    channel: usize,
    next: Option<usize>,
}

/// Host storage does not model EASTL heap addresses. Active bucket count,
/// chain order and rehash history are native semantic state and are retained.
pub(crate) struct ChannelDictionary {
    nodes: [Option<Node>; MAX_CHANNELS],
    heads: [Option<usize>; MAX_BUCKETS],
    buckets: usize,
    next_resize: usize,
    len: usize,
}

impl ChannelDictionary {
    pub(crate) fn new() -> Self {
        Self {
            nodes: std::array::from_fn(|_| None),
            heads: [None; MAX_BUCKETS],
            buckets: 1,
            next_resize: 0,
            len: 0,
        }
    }
    pub(crate) const fn len(&self) -> usize {
        self.len
    }
    pub(crate) const fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// `ResetChannelAnimatable` 0x82d7d230..260 frees all entries and
    /// empties buckets while retaining the allocated bucket count/policy.
    pub(crate) fn clear_preserving_buckets(&mut self) {
        self.nodes.fill_with(|| None);
        self.heads.fill(None);
        self.len = 0;
    }
    fn bucket(name: &FastString36, count: usize) -> usize {
        let hash = name.words.iter().copied().fold(0_u32, u32::wrapping_add);
        usize::try_from(hash % u32::try_from(count).expect("at most eleven buckets"))
            .expect("bucket fits usize")
    }
    fn find(&self, name: &FastString36) -> Option<usize> {
        let mut cursor = self.heads[Self::bucket(name, self.buckets)];
        while let Some(index) = cursor {
            let node = self.nodes[index].as_ref().expect("linked node");
            if node.name == *name {
                return Some(index);
            }
            cursor = node.next;
        }
        None
    }
    pub(crate) fn get(&self, name: &FastString36) -> Option<&usize> {
        let index = self.find(name)?;
        Some(&self.nodes[index].as_ref().expect("found node").channel)
    }
    pub(crate) fn contains_key(&self, name: &FastString36) -> bool {
        self.find(name).is_some()
    }

    /// Native dictionary operator[] followed by channel-pointer assignment.
    /// Existing keys retain position and do not consult the resize policy.
    pub(crate) fn try_insert(
        &mut self,
        name: FastString36,
        channel: usize,
    ) -> Result<Option<usize>, ChannelDictionaryError> {
        if let Some(index) = self.find(&name) {
            let node = self.nodes[index].as_mut().expect("found node");
            return Ok(Some(std::mem::replace(&mut node.channel, channel)));
        }
        let index = self
            .nodes
            .iter()
            .position(Option::is_none)
            .ok_or(ChannelDictionaryError::Capacity)?;
        self.prepare_insert();
        let bucket = Self::bucket(&name, self.buckets);
        self.nodes[index] = Some(Node {
            name,
            channel,
            next: self.heads[bucket],
        });
        self.heads[bucket] = Some(index);
        self.len += 1;
        Ok(None)
    }

    /// `GetRehashRequired82bd2438` specialized to the source-owned load1/growth2
    /// and <=6 entries. All involved integer/f32 conversions, division by1,
    /// multiplication by2 and ceil are exact within this domain.
    fn prepare_insert(&mut self) {
        let total = self.len + 1;
        if total <= self.next_resize {
            return;
        }
        let effective = if self.buckets == 1 { 0 } else { self.buckets };
        if total <= effective {
            self.next_resize = effective;
            return;
        }
        let target = total.max(effective * 2);
        // Actual gPrimeNumberArray822852f0 prefix, not arbitrary host primes.
        let count = [2, 3, 5, 7, 11]
            .into_iter()
            .find(|&p| p >= target)
            .expect("six-channel domain requires at most eleven buckets");
        self.next_resize = count;
        self.rehash(count);
    }
    fn rehash(&mut self, count: usize) {
        let mut heads = [None; MAX_BUCKETS];
        for bucket in 0..self.buckets {
            while let Some(index) = self.heads[bucket] {
                let node = self.nodes[index].as_mut().expect("linked node");
                self.heads[bucket] = node.next;
                let destination = Self::bucket(&node.name, count);
                node.next = heads[destination];
                heads[destination] = Some(index);
            }
        }
        self.heads = heads;
        self.buckets = count;
    }

    /// Erase unlinks only this node. Surviving chains and resize history stay.
    pub(crate) fn remove(&mut self, name: &FastString36) -> Option<usize> {
        let bucket = Self::bucket(name, self.buckets);
        let mut cursor = self.heads[bucket];
        let mut previous: Option<usize> = None;
        while let Some(index) = cursor {
            let node = self.nodes[index].as_ref().expect("linked node");
            let next = node.next;
            if node.name == *name {
                if let Some(previous) = previous {
                    self.nodes[previous].as_mut().expect("previous node").next = next;
                } else {
                    self.heads[bucket] = next;
                }
                self.len -= 1;
                return Some(self.nodes[index].take().expect("removed node").channel);
            }
            previous = Some(index);
            cursor = next;
        }
        None
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&FastString36, &usize)> {
        let mut bucket = 0;
        let mut cursor = None;
        std::iter::from_fn(move || {
            while cursor.is_none() {
                if bucket == self.buckets {
                    return None;
                }
                cursor = self.heads[bucket];
                bucket += 1;
            }
            let node = self.nodes[cursor.expect("nonempty bucket")]
                .as_ref()
                .expect("linked node");
            cursor = node.next;
            Some((&node.name, &node.channel))
        })
    }
    pub(crate) fn keys(&self) -> impl Iterator<Item = &FastString36> {
        self.iter().map(|(name, _)| name)
    }
    pub(crate) fn values(&self) -> impl Iterator<Item = &usize> {
        self.iter().map(|(_, channel)| channel)
    }
}

impl std::ops::Index<&FastString36> for ChannelDictionary {
    type Output = usize;
    fn index(&self, name: &FastString36) -> &Self::Output {
        self.get(name).expect("channel dictionary key")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn colliding(n: u32) -> FastString36 {
        FastString36 {
            words: [n, 55 - n, 0, 0, 0, 0],
        }
    }
    #[test]
    fn growth_reverses_colliding_chains_in_native_order() {
        let mut map = ChannelDictionary::new();
        for (n, count) in [(1, 2), (2, 2), (3, 5), (4, 5), (5, 5), (6, 11)] {
            map.try_insert(colliding(n), usize::try_from(n).unwrap())
                .unwrap();
            assert_eq!(map.buckets, count);
        }
        assert_eq!(
            map.values().copied().collect::<Vec<_>>(),
            [6, 2, 1, 3, 4, 5]
        );
    }
    #[test]
    fn erase_and_reinsert_keep_grown_buckets_and_survivor_order() {
        let mut map = ChannelDictionary::new();
        for n in 1..=6 {
            map.try_insert(colliding(n), usize::try_from(n).unwrap())
                .unwrap();
        }
        assert_eq!(map.remove(&colliding(1)), Some(1));
        assert_eq!(map.remove(&colliding(6)), Some(6));
        assert_eq!(map.remove(&colliding(5)), Some(5));
        assert_eq!(map.values().copied().collect::<Vec<_>>(), [2, 3, 4]);
        for n in [2, 3, 4] {
            map.remove(&colliding(n));
        }
        assert!(map.is_empty());
        map.try_insert(colliding(1), 9).unwrap();
        assert_eq!((map.buckets, map.next_resize), (11, 11));
        assert_eq!(map.get(&colliding(1)), Some(&9));
    }
    #[test]
    fn wrapping_hash_preserves_full_name_equality_and_bucket_iteration() {
        let mut map = ChannelDictionary::new();
        let a = FastString36 {
            words: [u32::MAX, 1, 0, 0, 0, 0],
        };
        let b = FastString36 { words: [0; 6] };
        let c = FastString36 {
            words: [1, 0, 0, 0, 0, 0],
        };
        map.try_insert(a, 1).unwrap();
        map.try_insert(b, 2).unwrap();
        map.try_insert(c, 3).unwrap();
        assert_eq!(map.values().copied().collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(map.try_insert(b, 8), Ok(Some(2)));
        assert_eq!(map.values().copied().collect::<Vec<_>>(), [1, 8, 3]);
        assert_eq!(map.remove(&colliding(44)), None);
        assert_eq!(map.len(), 3);
    }
    #[test]
    fn duplicate_assignment_at_capacity_does_not_rehash_or_reorder() {
        let mut map = ChannelDictionary::new();
        for n in 1..=6 {
            map.try_insert(colliding(n), usize::try_from(n).unwrap())
                .unwrap();
        }
        assert_eq!(map.try_insert(colliding(3), 9), Ok(Some(3)));
        assert_eq!(
            map.try_insert(colliding(7), 7),
            Err(ChannelDictionaryError::Capacity)
        );
        assert_eq!(
            map.values().copied().collect::<Vec<_>>(),
            [6, 2, 1, 9, 4, 5]
        );
    }
}
