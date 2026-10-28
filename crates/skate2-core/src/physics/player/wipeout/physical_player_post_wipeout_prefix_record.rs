//! Child of skeleton_physics_history: actual PhysRecord offset82b3d7b0.
use super::SkeletonPhysicsHistory;

impl SkeletonPhysicsHistory {
    /// Only translations1..23 change. Cached positions/velocities and root0
    /// deliberately retain their values, exactly as82b3d7d8..d818.
    pub(crate) fn add_post_wipeout_skateboard_error(&mut self, offset: [f32; 4]) {
        for part in 1..24 {
            for (lane, value) in offset.into_iter().enumerate() {
                self.record.transforms[part][12 + lane] += value;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offset_excludes_root_and_cached_positions_but_includes_w() {
        let mut history = SkeletonPhysicsHistory::new([1.0; 24]);
        let root = history.record.transforms[0];
        let positions = history.record.positions;
        let before = history.record.transforms[23];
        history.add_post_wipeout_skateboard_error([2.0, 3.0, 4.0, 5.0]);
        assert_eq!(history.record.transforms[0], root);
        assert_eq!(history.record.positions, positions);
        assert_eq!(history.record.transforms[23][15], before[15] + 5.0);
        assert_eq!(history.record.transforms[1][12], root[12] + 2.0);
    }
}
