//! Private Assembly/ground `GPTriangle` cache chronology.
//!
//! `RipTriangles<BBoxTestRipFunctor<AssemblyTriangleIntersectionTester>>` at
//! `0x827cd7c8..0x827cd960` batches exactly twenty accepted triangles before
//! calling `RunIntersectionTests` at `0x827cd1e0`. This owner reconstructs
//! that local cache boundary without constructing a triangle, a Contact, or a
//! public collision producer.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the reconstructed Assembly/ground producer"
    )
)]

const ASSEMBLY_GROUND_TRIANGLE_CACHE_CAPACITY: usize = 20;

/// Private fixed cache embedded in the native
/// `AssemblyTriangleIntersectionTester` at offsets `+0x3c/+0x40`.
///
/// `total_accepted` corresponds to `numTriangles +0x00`; it is diagnostic
/// bookkeeping and wraps as the native 32-bit increment does. Stale values
/// above `pending_count` deliberately remain unobservable after a flush,
/// matching the native count reset without a memory clear.
pub(super) struct AssemblyGroundTriangleCache<T: Copy> {
    slots: [Option<T>; ASSEMBLY_GROUND_TRIANGLE_CACHE_CAPACITY],
    pending_count: usize,
    total_accepted: u32,
}

/// Borrowed initialized prefix passed to the value-level intersection seam.
/// This view aliases the actual fixed cache; it does not manufacture a second
/// twenty-record copy or expose stale values above the native count.
#[derive(Clone, Copy)]
pub(super) struct AssemblyGroundTriangleBatch<'a, T: Copy> {
    slots: &'a [Option<T>],
}

impl<'a, T: Copy> AssemblyGroundTriangleBatch<'a, T> {
    pub(super) const fn len(self) -> usize {
        self.slots.len()
    }

    pub(super) fn iter(self) -> impl Iterator<Item = &'a T> {
        self.slots.iter().map(|slot| {
            slot.as_ref()
                .expect("the accepted cache prefix is initialized")
        })
    }
}

impl<T: Copy> AssemblyGroundTriangleCache<T> {
    pub(super) const fn new() -> Self {
        Self {
            slots: [None; ASSEMBLY_GROUND_TRIANGLE_CACHE_CAPACITY],
            pending_count: 0,
            total_accepted: 0,
        }
    }

    /// Applies the accepted-instance suffix recovered from `RipTriangles`.
    ///
    /// Native order is debug publication, old-cache intersection at exactly
    /// twenty, unconditional count reset, count increment, current triangle
    /// copy, and total increment. The callbacks are infallible because both
    /// native operations communicate no failure to this wrapper. In
    /// `sendBackTriangles == 0` mode the caller supplies a no-op debug hook,
    /// representing the entire disabled optional branch. In
    /// particular, a Contact-capacity early return from `RunIntersectionTests`
    /// still returns `void`, so the old cache is discarded and the triggering
    /// triangle becomes slot zero.
    pub(super) fn accept_triangle(
        &mut self,
        triangle: T,
        publish_debug: impl FnOnce(&T),
        run_intersections: impl FnOnce(AssemblyGroundTriangleBatch<'_, T>),
    ) {
        publish_debug(&triangle);
        if self.pending_count == ASSEMBLY_GROUND_TRIANGLE_CACHE_CAPACITY {
            self.with_pending(run_intersections);
            self.pending_count = 0;
        }

        let index = self.pending_count;
        self.pending_count += 1;
        self.slots[index] = Some(triangle);
        self.total_accepted = self.total_accepted.wrapping_add(1);
    }

    /// Models the unconditional final job-tail `RunIntersectionTests` call.
    /// Native does not reset this count afterward because the stack tester's
    /// lifetime ends. Consuming the host owner enforces that same terminal
    /// boundary and prevents replaying the final cache.
    pub(super) fn finish(self, run_intersections: impl FnOnce(AssemblyGroundTriangleBatch<'_, T>)) {
        self.with_pending(run_intersections);
    }

    fn with_pending(&self, run_intersections: impl FnOnce(AssemblyGroundTriangleBatch<'_, T>)) {
        run_intersections(AssemblyGroundTriangleBatch {
            slots: &self.slots[..self.pending_count],
        });
    }

    const fn pending_count(&self) -> usize {
        self.pending_count
    }

    const fn total_accepted(&self) -> u32 {
        self.total_accepted
    }

    fn pending(&self) -> impl Iterator<Item = &T> {
        self.slots[..self.pending_count].iter().map(|slot| {
            slot.as_ref()
                .expect("the accepted cache prefix is initialized")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn first_twenty_accumulate_without_running_intersections() {
        let mut cache = AssemblyGroundTriangleCache::new();
        let trace = RefCell::new(Vec::new());
        for triangle in 0_u32..20 {
            cache.accept_triangle(
                triangle,
                |value| trace.borrow_mut().push(format!("debug:{value}")),
                |_| panic!("the first twenty do not flush"),
            );
        }

        assert_eq!(cache.pending_count(), 20);
        assert_eq!(cache.total_accepted(), 20);
        assert_eq!(
            cache.pending().copied().collect::<Vec<_>>(),
            (0..20).collect::<Vec<_>>()
        );
        assert_eq!(trace.borrow().len(), 20);
    }

    #[test]
    fn twenty_first_debugs_then_flushes_old_cache_and_becomes_slot_zero() {
        let mut cache = AssemblyGroundTriangleCache::new();
        for triangle in 0_u32..20 {
            cache.accept_triangle(triangle, |_| {}, |_| unreachable!());
        }
        let trace = RefCell::new(Vec::new());
        cache.accept_triangle(
            20,
            |value| trace.borrow_mut().push(format!("debug:{value}")),
            |old| {
                trace
                    .borrow_mut()
                    .push(format!("run:{:?}", old.iter().copied().collect::<Vec<_>>()));
            },
        );

        assert_eq!(trace.borrow()[0], "debug:20");
        assert_eq!(
            trace.borrow()[1],
            format!("run:{:?}", (0_u32..20).collect::<Vec<_>>())
        );
        assert_eq!(cache.pending().copied().collect::<Vec<_>>(), [20]);
        assert_eq!(cache.pending_count(), 1);
        assert_eq!(cache.total_accepted(), 21);
        cache.finish(|remaining| {
            assert_eq!(remaining.len(), 1);
            assert_eq!(remaining.iter().copied().collect::<Vec<_>>(), [20]);
        });
    }

    #[test]
    fn void_intersection_stop_cannot_prevent_reset_and_trigger_append() {
        let mut cache = AssemblyGroundTriangleCache::new();
        for triangle in 100_u32..120 {
            cache.accept_triangle(triangle, |_| {}, |_| unreachable!());
        }
        let visited_prefix = RefCell::new(Vec::new());
        cache.accept_triangle(
            999,
            |_| {},
            |old| {
                // Models RunIntersectionTests returning early at its first
                // unavailable Contact slot. The wrapper receives no status.
                visited_prefix
                    .borrow_mut()
                    .push(*old.iter().next().expect("old cache"));
            },
        );

        assert_eq!(*visited_prefix.borrow(), [100]);
        assert_eq!(cache.pending().copied().collect::<Vec<_>>(), [999]);
        assert_eq!(cache.pending_count(), 1);
        assert_eq!(cache.total_accepted(), 21);
    }

    #[test]
    fn job_tail_consumes_empty_partial_or_exact_twenty_once() {
        let empty = AssemblyGroundTriangleCache::<u32>::new();
        empty.finish(|values| assert_eq!(values.len(), 0));

        let mut partial = AssemblyGroundTriangleCache::new();
        partial.accept_triangle(7, |_| {}, |_| unreachable!());
        partial.accept_triangle(8, |_| {}, |_| unreachable!());
        partial.finish(|values| {
            assert_eq!(values.iter().copied().collect::<Vec<_>>(), [7, 8]);
        });

        let mut exact = AssemblyGroundTriangleCache::new();
        for triangle in 0_u32..20 {
            exact.accept_triangle(triangle, |_| {}, |_| unreachable!());
        }
        exact.finish(|values| {
            assert_eq!(values.len(), 20);
            assert_eq!(
                values.iter().copied().collect::<Vec<_>>(),
                (0..20).collect::<Vec<_>>()
            );
        });
    }

    #[test]
    fn total_triangle_counter_wraps_while_retaining_the_accepted_triangle() {
        let mut cache = AssemblyGroundTriangleCache::new();
        cache.total_accepted = u32::MAX;
        cache.accept_triangle(42, |_| {}, |_| unreachable!());

        assert_eq!(cache.pending().copied().collect::<Vec<_>>(), [42]);
        assert_eq!(cache.total_accepted(), 0);
    }
}
