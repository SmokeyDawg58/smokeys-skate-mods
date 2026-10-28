//! Shared LWExecutionMan/LWExecutionGroup owner for existing typed EntityMan
//! rosters. Vehicle and pedestrian templates share rotation82711608 and sort.
use super::{entities::EntityMan, entity_core::CommonEntityCore};
use crate::{input_cvar_runtime::InputCvarError, reckoning_numeric::ReckoningNumeric};
use std::marker::PhantomData;
use crate::native_execution_sort as target_sort;
#[path = "../pedestrians/pedestrian_execution.rs"]
pub(crate) mod pedestrian;

pub(crate) trait ExecutionEntity {
    fn execution_core(&self) -> Option<&CommonEntityCore>;
    fn execution_core_mut(&mut self) -> Option<&mut CommonEntityCore>;
    fn execution_position(&self) -> Option<[f32;4]> {
        self.execution_core().map(|core| core.transform[3])
    }
    fn take_explicit_update(&mut self) -> Option<bool> {
        self.execution_core_mut().map(|core| std::mem::take(&mut core.explicit_update_requested))
    }
}
/// Access the existing entity owner; DMO's retained map predates EntityMan.
/// This adapter does not allocate or mirror a roster.
pub(crate) trait ExecutionRoster<E> {
    fn execution_get(&self, id:u32)->Option<&E>;
    fn execution_get_mut(&mut self, id:u32)->Option<&mut E>;
}
impl<const N:usize,E> ExecutionRoster<E> for EntityMan<N,E> {
    fn execution_get(&self,id:u32)->Option<&E>{self.get(id)}
    fn execution_get_mut(&mut self,id:u32)->Option<&mut E>{self.get_mut(id)}
}
impl<E> ExecutionRoster<E> for std::collections::HashMap<u32,E> {
    fn execution_get(&self,id:u32)->Option<&E>{self.get(&id)}
    fn execution_get_mut(&mut self,id:u32)->Option<&mut E>{self.get_mut(&id)}
}
impl<E,R:ExecutionRoster<E>> ExecutionRoster<E> for Box<R> {
    fn execution_get(&self,id:u32)->Option<&E>{self.as_ref().execution_get(id)}
    fn execution_get_mut(&mut self,id:u32)->Option<&mut E>{self.as_mut().execution_get_mut(id)}
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct ExecutionTarget { pub(crate) entity: u32, pub(crate) dt: f32 }
#[derive(Debug)]
pub(crate) enum ExecutionError {
    AlreadyInitialized, Cvar(InputCvarError), InvalidFrameRate(u32), Group(u32),
    NoAvailableBucket, BucketCapacity, TargetCapacity, MissingEntity(u32), MissingCore(u32),
}
#[derive(Debug)]
pub(crate) enum ExecutionTaskError<N> { Execution(ExecutionError), Numeric(N) }
pub(crate) struct ExecutionMan<const N: usize, E> {
    pub(crate) groups: Vec<ExecutionGroup>,
    entity: PhantomData<fn() -> E>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BucketState {
    PreFrame,
    CurrentFrame,
    Idle,
}
pub(crate) struct Bucket {
    pub(crate) entities: Vec<u32>,
    pub(crate) state: BucketState,
}
pub(crate) struct ExecutionGroup {
    pub(crate) buckets: Vec<Bucket>,
    pub(crate) bucket_order: Vec<usize>,
    pub(crate) dt: f32,
    pub(crate) preframe: usize,
    pub(crate) current: usize,
}
impl ExecutionGroup {
    pub(crate) fn initialize(rate: u32) -> Result<Self, ExecutionError> {
        let count = 60_u32
            .checked_div(rate)
            .ok_or(ExecutionError::InvalidFrameRate(rate))?;
        // Native asserts >1 (GUI permits values which deliberately assert).
        if count <= 1 {
            return Err(ExecutionError::InvalidFrameRate(rate));
        }
        let mut buckets: Vec<_> = (0..count)
            .map(|_| Bucket {
                entities: Vec::new(),
                state: BucketState::Idle,
            })
            .collect();
        buckets[0].state = BucketState::PreFrame;
        Ok(Self {
            buckets,
            bucket_order: (0..count as usize).collect(),
            dt: 1.0 / rate as f32,
            preframe: 0,
            current: 0,
        })
    }
    pub(crate) fn add(&mut self, id: u32) -> Result<(), ExecutionError> {
        // Native mutates the retained proxy order on EVERY insertion. It
        // sorts only by list size, then skips the CurrentFrame bucket.
        let lengths: Vec<_> = self.buckets.iter().map(|b| b.entities.len()).collect();
        sort_bucket_proxies(&mut self.bucket_order, &lengths);
        let index = self
            .bucket_order
            .iter()
            .copied()
            .find(|&i| self.buckets[i].state != BucketState::CurrentFrame)
            .ok_or(ExecutionError::NoAvailableBucket)?;
        let list = &mut self.buckets[index].entities;
        if list.len() >= 256 {
            return Err(ExecutionError::BucketCapacity);
        }
        list.push(id); // native fixed_list push_back, no duplicate suppression
        Ok(())
    }
    pub(crate) fn remove(&mut self, id: u32) {
        for bucket in &mut self.buckets {
            bucket.entities.retain(|&other| other != id);
        }
    }
    pub(crate) fn advance(&mut self) {
        self.buckets[self.current].state = BucketState::Idle;
        self.current = self.preframe;
        self.buckets[self.preframe].state = BucketState::CurrentFrame;
        self.preframe += 1;
        if self.preframe >= self.buckets.len() {
            self.preframe = 0;
        }
        self.buckets[self.preframe].state = BucketState::PreFrame;
    }
    /// GetPreFrameBucketContents82d271d8, BEFORE the manager's distance sort.
    fn append_preframe<E: ExecutionEntity>(
        &self,
        entities: &mut impl ExecutionRoster<E>,
        out: &mut Vec<ExecutionTarget>,
    ) -> Result<(), ExecutionError> {
        for bucket in &self.buckets {
            if bucket.state == BucketState::PreFrame {
                continue;
            }
            for &id in &bucket.entities {
                let entity = entities.execution_get_mut(id).ok_or(ExecutionError::MissingEntity(id))?;
                if entity.take_explicit_update().ok_or(ExecutionError::MissingCore(id))? {
                    append(out, id, self.dt)?;
                }
            }
        }
        let bucket = &self.buckets[self.preframe];
        if bucket.state == BucketState::PreFrame {
            for &id in &bucket.entities {
                // Native scheduled branch does NOT clear the explicit flag.
                entities.execution_get(id).ok_or(ExecutionError::MissingEntity(id))?;
                append(out, id, self.dt)?;
            }
        }
        Ok(())
    }
}
fn append(
    out: &mut Vec<ExecutionTarget>,
    entity: u32,
    dt: f32,
) -> Result<(), ExecutionError> {
    if out.len() >= 256 {
        return Err(ExecutionError::TargetCapacity);
    }
    out.push(ExecutionTarget { entity, dt });
    Ok(())
}
impl<const N: usize, E: ExecutionEntity> ExecutionMan<N, E> {
    pub(crate) const fn new() -> Self { Self { groups: Vec::new(), entity: PhantomData } }
    pub(crate) fn initialize_with(&mut self,
        read: impl FnOnce() -> Result<[u32; 3], InputCvarError>,
    ) -> Result<(), ExecutionError> {
        if !self.groups.is_empty() { return Err(ExecutionError::AlreadyInitialized); }
        let rates = read().map_err(ExecutionError::Cvar)?;
        self.initialize_rates(rates)
    }
    pub(crate) fn initialize_rates(&mut self, rates: [u32; 3]) -> Result<(), ExecutionError> {
        if !self.groups.is_empty() { return Err(ExecutionError::AlreadyInitialized); }
        for rate in rates { self.groups.push(ExecutionGroup::initialize(rate)?); }
        Ok(())
    }
    pub(crate) fn add(&mut self, losim: u32, id: u32) -> Result<(), ExecutionError> {
        self.groups.get_mut(losim as usize).ok_or(ExecutionError::Group(losim))?.add(id)
    }
    pub(crate) fn remove(&mut self, losim: u32, id: u32) -> Result<(), ExecutionError> {
        self.groups.get_mut(losim as usize).ok_or(ExecutionError::Group(losim))?.remove(id);
        Ok(())
    }
    pub(crate) fn advance(&mut self) { for group in &mut self.groups { group.advance(); } }
    pub(crate) fn collect_preframe_unsorted(&self, entities: &mut impl ExecutionRoster<E>,
        out: &mut Vec<ExecutionTarget>) -> Result<(), ExecutionError> {
        out.clear();
        for group in &self.groups { group.append_preframe(entities, out)?; }
        Ok(())
    }
    pub(crate) fn collect_current_unsorted(&self, entities: &impl ExecutionRoster<E>,
        out: &mut Vec<ExecutionTarget>) -> Result<(), ExecutionError> {
        out.clear();
        for group in &self.groups {
            let bucket = &group.buckets[group.current];
            if bucket.state != BucketState::CurrentFrame { continue; }
            for &id in &bucket.entities {
                entities.execution_get(id).ok_or(ExecutionError::MissingEntity(id))?;
                append(out, id, group.dt)?;
            }
        }
        Ok(())
    }
    pub(crate) fn collect_preframe<M: ReckoningNumeric>(&self, entities: &mut impl ExecutionRoster<E>,
        center: [f32; 4], out: &mut Vec<ExecutionTarget>, numeric: &mut M,
    ) -> Result<(), ExecutionTaskError<M::Error>> {
        self.collect_preframe_unsorted(entities, out).map_err(ExecutionTaskError::Execution)?;
        Self::sort_targets(entities, center, out, numeric)
    }
    pub(crate) fn collect_current<M: ReckoningNumeric>(&self, entities: &impl ExecutionRoster<E>,
        center: [f32; 4], out: &mut Vec<ExecutionTarget>, numeric: &mut M,
    ) -> Result<(), ExecutionTaskError<M::Error>> {
        self.collect_current_unsorted(entities, out).map_err(ExecutionTaskError::Execution)?;
        Self::sort_targets(entities, center, out, numeric)
    }
    fn sort_targets<M: ReckoningNumeric>(entities: &impl ExecutionRoster<E>, center: [f32; 4],
        out: &mut [ExecutionTarget], numeric: &mut M,
    ) -> Result<(), ExecutionTaskError<M::Error>> {
        let mut distance = |id| {
            let entity = entities.execution_get(id).ok_or(ExecutionTaskError::Execution(ExecutionError::MissingEntity(id)))?;
            let position = entity.execution_position().ok_or(ExecutionTaskError::Execution(ExecutionError::MissingCore(id)))?;
            let delta = std::array::from_fn(|i| center[i] - position[i]);
            numeric.dot3(delta, delta).map_err(ExecutionTaskError::Numeric)
        };
        target_sort::sort(out, &mut |a, b| Ok(distance(a.entity)? < distance(b.entity)?))
    }
}
/// EASTL quick_sort82d27b08: cutoff28, median/partition82d27630/82d33548,
/// then insertion82e86610/82e86688. Keeps equal-count proxy order from the
/// original algorithm; host sort_unstable would choose different buckets.
pub(crate) fn sort_bucket_proxies(order: &mut [usize], lengths: &[usize]) {
    if order.is_empty() {
        return;
    }
    let depth = 2 * (usize::BITS - order.len().leading_zeros()) as i32 - 2;
    quick(order, lengths, depth);
    // The unguarded tail has the sorted first28 as a sentinel. The same
    // insertion operations with a bounds guard preserve its exact result.
    for i in 1..order.len() {
        let value = order[i];
        let mut j = i;
        while j > 0 && lengths[value] < lengths[order[j - 1]] {
            order[j] = order[j - 1];
            j -= 1;
        }
        order[j] = value;
    }
}
fn quick(mut order: &mut [usize], lengths: &[usize], mut depth: i32) {
    while order.len() > 28 && depth > 0 {
        let first = order[0];
        let middle = order[order.len() / 2];
        let last = order[order.len() - 1];
        let pivot = if lengths[first] < lengths[middle] {
            if lengths[middle] < lengths[last] {
                middle
            } else if lengths[first] < lengths[last] {
                last
            } else {
                first
            }
        } else if lengths[first] < lengths[last] {
            first
        } else if lengths[middle] < lengths[last] {
            last
        } else {
            middle
        };
        depth -= 1;
        let mut low = 0;
        let mut high = order.len();
        let split = loop {
            while lengths[order[low]] < lengths[pivot] {
                low += 1;
            }
            high -= 1;
            while lengths[pivot] < lengths[order[high]] {
                high -= 1;
            }
            if low >= high {
                break low;
            }
            order.swap(low, high);
            low += 1;
        };
        let (left, right) = order.split_at_mut(split);
        quick(right, lengths, depth);
        order = left;
    }
    if depth == 0 {
        heap_sort(order, lengths);
    }
}
fn heap_sort(order: &mut [usize], lengths: &[usize]) {
    if order.len() < 2 {
        return;
    }
    for top in (0..=(order.len() - 2) / 2).rev() {
        let value = order[top];
        adjust(order, lengths, top, top, value);
    }
    for end in (1..order.len()).rev() {
        let value = order[end];
        order[end] = order[0];
        adjust(&mut order[..end], lengths, 0, 0, value);
    }
}
fn adjust(order: &mut [usize], lengths: &[usize], top: usize, mut hole: usize, value: usize) {
    let mut right = 2 * (hole + 1);
    while right < order.len() {
        // Native82e8671c..28 compares the bucket POINTERS here, unlike its
        // promote step. Buckets share one allocation: indices preserve that
        // ordering without inventing original addresses. Raw cmplw confirmed.
        if order[right] < order[right - 1] {
            right -= 1;
        }
        order[hole] = order[right];
        hole = right;
        right = 2 * (hole + 1);
    }
    if right == order.len() {
        order[hole] = order[right - 1];
        hole = right - 1;
    }
    // promote_heap82e86598 compares entity-list sizes, including equal ties.
    while hole > top {
        let parent = (hole - 1) / 2;
        if lengths[order[parent]] >= lengths[value] {
            break;
        }
        order[hole] = order[parent];
        hole = parent;
    }
    order[hole] = value;
}

