//! `DriveNormalizer` (0x827b2738), called by `Island::Step_Solver2` at0x827b3610.
//! Every Active-ring occurrence participates, independently of body state or
//! dynamics type. Host numeric primitives remain explicit non-Xenon models.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Drive normalization awaits manager frame-owner integration"
    )
)]

use super::{SimulationDriveId, SimulationDriveList, SimulationDriveNode, SimulationDrivePool};
use std::fmt;

#[cfg(test)]
#[path = "simulation_drive_normalization_tests.rs"]
mod tests;

/// Intrinsic boundary, ordered normA,normB,seedA,seedB per nonnull frame.
/// Callbacks must be pure with respect to pool/frame state and nonreentrant:
/// the current frame's write lock is held, and must not be reacquired here.
/// Vector results retain all lanes, including injected per-lane test seeds.
pub(crate) trait SimulationDriveNormalizationNumeric {
    type Error;
    fn norm4(&mut self, quaternion: [f32; 4]) -> Result<[f32; 4], Self::Error>;
    fn rsqrt_estimate(&mut self, squared_norm: [f32; 4]) -> Result<[f32; 4], Self::Error>;
}

/// Nested host FMA reduction and `sqrt().recip()` estimates. This is not proof
/// of Xenon vmsum4 rounding, estimate bits, denormals or exceptional payloads.
pub(crate) struct HostSimulationDriveNormalizationNumeric;
impl SimulationDriveNormalizationNumeric for HostSimulationDriveNormalizationNumeric {
    type Error = std::convert::Infallible;
    fn norm4(&mut self, q: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        let sum = q[0].mul_add(q[0], q[1].mul_add(q[1], q[2].mul_add(q[2], q[3] * q[3])));
        Ok([sum; 4])
    }
    fn rsqrt_estimate(&mut self, norm: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(norm.map(|value| value.sqrt().recip()))
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum SimulationDriveNormalizationError<E> {
    CorruptTopology,
    FrameLockPoisoned { drive: SimulationDriveId },
    Numeric { drive: SimulationDriveId, source: E },
}
impl<E: fmt::Display> fmt::Display for SimulationDriveNormalizationError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CorruptTopology => write!(f, "Drive normalization ring topology is corrupt"),
            Self::FrameLockPoisoned { drive } => write!(
                f,
                "Drive {} normalization frame lock poisoned",
                drive.index()
            ),
            Self::Numeric { drive, source } => write!(
                f,
                "Drive {} normalization numeric operation: {source}",
                drive.index()
            ),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for SimulationDriveNormalizationError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Numeric { source, .. } => Some(source),
            Self::CorruptTopology | Self::FrameLockPoisoned { .. } => None,
        }
    }
}

impl SimulationDrivePool {
    /// Active-ring `for_each(DriveNormalizer)`, 0x827b30c8/0x827b2738.
    ///
    /// Topology validation is host policy and occurs before numeric/effects.
    /// It deliberately does not require bodies, dynamics, spy, occurrence or
    /// frame pointers: native normalization reads none of those except its
    /// nullable frame pointer. Existing Add/Build invariants remain unchanged.
    ///
    /// One write guard protects the current quaternion pair through callbacks
    /// and both stores. Other owners may be updated before their turn; no
    /// whole-batch snapshot or alias deduplication occurs. Numeric callbacks
    /// must not reenter the pool or acquire the current owner's lock. Errors
    /// leave the current pair unchanged and preserve earlier completed pairs.
    /// A panic poisons its current lock; it does not roll back prior pairs.
    pub(crate) fn try_normalize_active_drive_frames<N: SimulationDriveNormalizationNumeric>(
        &mut self,
        numeric: &mut N,
    ) -> Result<(), SimulationDriveNormalizationError<N::Error>> {
        use SimulationDriveNormalizationError::{CorruptTopology, FrameLockPoisoned, Numeric};
        if !self.normalization_topology_is_valid() {
            return Err(CorruptTopology);
        }
        let anchor = SimulationDriveNode::Anchor(SimulationDriveList::Active);
        let mut node = self.anchors[SimulationDriveList::Active.index()].right;
        while node != anchor {
            let SimulationDriveNode::Drive(id) = node else {
                return Err(CorruptTopology);
            };
            let slot = &self.slots[id.0 as usize];
            if let Some(owner) = &slot.frames {
                let mut frame = owner.write().map_err(|_| FrameLockPoisoned { drive: id })?;
                let qa = frame.quaternion_a;
                let norm_a = numeric
                    .norm4(qa)
                    .map_err(|source| Numeric { drive: id, source })?;
                let qb = frame.quaternion_b;
                let norm_b = numeric
                    .norm4(qb)
                    .map_err(|source| Numeric { drive: id, source })?;
                let seed_a = numeric
                    .rsqrt_estimate(norm_a)
                    .map_err(|source| Numeric { drive: id, source })?;
                let seed_b = numeric
                    .rsqrt_estimate(norm_b)
                    .map_err(|source| Numeric { drive: id, source })?;
                let (a, b) = refine_pair(norm_a, norm_b, seed_a, seed_b);
                let normalized_a = std::array::from_fn(|lane| qa[lane] * a[lane]);
                let normalized_b = std::array::from_fn(|lane| qb[lane] * b[lane]);
                frame.quaternion_a = normalized_a;
                frame.quaternion_b = normalized_b;
            }
            // Native iterator loads right AFTER the current functor returns.
            node = self.slots[id.0 as usize].links.right;
        }
        Ok(())
    }

    /// Allocation-free topology-only check. The complete two-ring partition
    /// is checked, but no native payload fields or body capabilities are read.
    fn normalization_topology_is_valid(&self) -> bool {
        let mut total = 0_usize;
        for list in SimulationDriveList::ALL {
            let anchor = SimulationDriveNode::Anchor(list);
            let anchor_links = self.anchors[list.index()];
            let mut node = anchor_links.right;
            let mut previous = anchor;
            let mut count = 0_usize;
            while node != anchor {
                let SimulationDriveNode::Drive(id) = node else {
                    return false;
                };
                let Some(slot) = self.slots.get(id.0 as usize) else {
                    return false;
                };
                if count >= self.slots.len()
                    || slot.id != id
                    || slot.membership != list
                    || slot.links.left != previous
                {
                    return false;
                }
                previous = node;
                node = slot.links.right;
                count += 1;
            }
            if anchor_links.left != previous || count != self.counts.get(list) as usize {
                return false;
            }
            // Disjoint membership and reciprocal previous links exclude
            // duplicate occurrences; total coverage excludes orphan slots.
            let Some(next_total) = total.checked_add(count) else {
                return false;
            };
            total = next_total;
        }
        total == self.slots.len()
    }
}

fn refine_pair(
    norm_a: [f32; 4],
    norm_b: [f32; 4],
    mut a: [f32; 4],
    mut b: [f32; 4],
) -> ([f32; 4], [f32; 4]) {
    for _ in 0..2 {
        let square_a = a.map(|value| value * value);
        let half_a = a.map(|value| value * 0.5);
        let square_b = b.map(|value| value * value);
        let half_b = b.map(|value| value * 0.5);
        let residual_a: [f32; 4] =
            std::array::from_fn(|lane| (-norm_a[lane]).mul_add(square_a[lane], 1.0));
        let residual_b: [f32; 4] =
            std::array::from_fn(|lane| (-norm_b[lane]).mul_add(square_b[lane], 1.0));
        a = std::array::from_fn(|lane| half_a[lane].mul_add(residual_a[lane], a[lane]));
        b = std::array::from_fn(|lane| half_b[lane].mul_add(residual_b[lane], b[lane]));
    }
    (a, b)
}
