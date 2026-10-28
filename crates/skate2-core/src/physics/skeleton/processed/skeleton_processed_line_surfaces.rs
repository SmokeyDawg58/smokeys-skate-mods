//! Retained `ProcessedPhysIn` left/right/hips surfaces, source +390/+3c0/+3f0.
use super::SkeletonProcessedContact;
use crate::{
    physical_player_assembly::hilod_tail::SkeletonLineResult,
    skeleton_root_transforms::SkeletonRootVector,
};

pub(crate) struct SkeletonProcessedLineSurfaces {
    pub(crate) left: Option<SkeletonLineResult>,
    pub(crate) right: Option<SkeletonLineResult>,
    pub(crate) hips: Option<SkeletonLineResult>,
    pub(crate) left_id: Option<i32>,
    pub(crate) right_id: Option<i32>,
}
impl SkeletonProcessedLineSurfaces {
    pub(crate) const fn new() -> Self {
        Self {
            left: None,
            right: None,
            hips: None,
            left_id: None,
            right_id: None,
        }
    }
    pub(crate) fn reset(&mut self) {
        let zero = SkeletonLineResult {
            position: SkeletonRootVector::known([0.0; 4]),
            normal: SkeletonRootVector::known([0.0; 4]),
            surface_id: 0,
            valid: false,
        };
        self.left = Some(zero);
        self.right = Some(zero);
        self.hips = Some(zero);
        self.left_id = Some(0);
        self.right_id = Some(0);
    }
}
impl SkeletonProcessedContact {
    pub(crate) fn line_surfaces(&self) -> &SkeletonProcessedLineSurfaces {
        &self.line_surfaces
    }
    /// `FillPhysIn82dfeca8` copies left, right, hips from retained PREVIOUS results.
    /// Subsequent Update1 at36dc/36f4 updates each surface ID only if valid.
    pub(crate) fn fill_skeleton_line_surfaces(&mut self, results: &[SkeletonLineResult; 3]) {
        self.line_surfaces.left = Some(results[1]);
        self.line_surfaces.right = Some(results[2]);
        self.line_surfaces.hips = Some(results[0]);
        if results[1].valid {
            self.line_surfaces.left_id = Some(results[1].surface_id);
        }
        if results[2].valid {
            self.line_surfaces.right_id = Some(results[2].surface_id);
        }
    }
}
