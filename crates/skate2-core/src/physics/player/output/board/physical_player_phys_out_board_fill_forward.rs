//! Actual forward-speed output 0x82b300e4..0x82b301a0.
use super::super::{
    PhysOutAccessError, PhysOutMotionScalar, PhysOutMotionVector, PhysicalPlayerPhysOut,
};
use crate::{
    skeleton_collision_numeric::SkeletonCollisionNumeric,
    skeleton_root_transforms::SkeletonRootMatrix,
};
#[derive(Debug, PartialEq)]
pub(crate) enum BoardForwardOutputError<S, N> {
    Source(S),
    Access(PhysOutAccessError),
    Numeric(N),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for BoardForwardOutputError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "board forward transform: {e:?}"),
            Self::Access(e) => write!(f, "board forward output: {e:?}"),
            Self::Numeric(e) => write!(f, "board forward dot: {e:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    pub(crate) fn try_fill_board_forward<S, N: SkeletonCollisionNumeric>(
        &mut self,
        effective_transform: impl FnOnce() -> Result<SkeletonRootMatrix, S>,
        numeric: &mut N,
    ) -> Result<(), BoardForwardOutputError<S, N::Error>> {
        use BoardForwardOutputError as E;
        let transform = effective_transform().map_err(E::Source)?; //300e8..30130
        self.check_current_guard().map_err(E::Access)?; //30134..50
        // Native captures Motion pointer30154, then another latch30160 before
        // actual velocity/Z loads and dot3016c..78. Host component read follows
        // that second latch; no numeric callback precedes it.
        let guard = self.sample_write_guard().map_err(E::Access)?;
        // Direct retained field read: the public accessor would add a third
        // assertion before the source's dot, moving the actual second check.
        let velocity = self.motion_vectors[PhysOutMotionVector::DeckVelocity as usize]
            .ok_or(E::Access(PhysOutAccessError::NotConstructed))?;
        let dot = numeric
            .dot3(velocity.value.xyz, transform.columns()[2].xyz)
            .map_err(E::Numeric)?;
        self.publish_motion_scalar(&guard, PhysOutMotionScalar::ZSpeed, dot)
            .map_err(E::Access)
    }
}
#[cfg(test)]
#[path = "physical_player_phys_out_board_fill_forward_tests.rs"]
mod tests;
