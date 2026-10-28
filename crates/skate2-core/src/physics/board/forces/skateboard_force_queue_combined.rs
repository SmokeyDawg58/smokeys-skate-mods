//! Actual `GetCombinedForces82b26650`, hidden result r3 and board r4.
use super::SkateboardForceQueue;
use crate::skeleton_root_transforms::SkeletonRootVector;
impl SkateboardForceQueue {
    pub(crate) fn combined_forces(&self) -> SkeletonRootVector {
        // All four source scratch lanes explicitly cleared, including sp-4.
        let mut total = SkeletonRootVector::known([0.0; 4]);
        for record in &self.entries[..self.count] {
            let force = record
                .expect("occupied native force range")
                .force
                .world_space_force;
            total = total.zip(force, |a, b| a + b);
        }
        total
    }
}
#[cfg(test)]
mod tests {
    use super::SkateboardForceQueue;
    use crate::{
        skateboard_foot_forces::SkateboardFootForce,
        skeleton_root_transforms::SkeletonRootVector as V,
    };
    #[test]
    fn sums_every_kind_in_insertion_order_without_consuming() {
        let mut q = SkateboardForceQueue::new();
        assert_eq!(
            q.combined_forces().known_lanes().unwrap().map(f32::to_bits),
            [0; 4]
        );
        for (kind, x) in [(17, 16_777_216.0), (99, 1.0), (1, -16_777_216.0)] {
            q.add(
                kind,
                SkateboardFootForce {
                    world_space_force: V {
                        xyz: [x, 0.0, 0.0],
                        w: None,
                    },
                    local_space_position: V::known([f32::NAN; 4]),
                },
            );
        }
        assert_eq!(q.combined_forces().xyz[0].to_bits(), 0.0_f32.to_bits());
        assert_eq!(q.combined_forces().w, None);
        assert_eq!(q.len(), 3);
    }
}
