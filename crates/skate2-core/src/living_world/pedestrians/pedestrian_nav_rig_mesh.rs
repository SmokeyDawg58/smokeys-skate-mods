//! Pedestrian::SetPathogenCell82e6e190 retained mesh history.
//! Child of pedestrian_navigation::rig; the rig remains the single owner.
use super::NavRig;
use crate::{
    living_world_mesh_store::{MeshCell, MeshStore, asset::NavMeshAsset},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

/// Native NavRig+240, tMeshInfo64 bytes. Its position fields are distinct
/// from NavRig::mPosition at+16, which SetPathogenCell does not write.
pub(crate) struct NavRigMeshInfo {
    pub(crate) cell: MeshCell,
    pub(crate) previous_cell: MeshCell,
    pub(crate) previous_cell_position: [f32; 4],
    pub(crate) cell_score: f32,
    pub(crate) previous_cell_score: f32,
    pub(crate) last_valid_cell: MeshCell,
    pub(crate) last_valid_cell_position: [f32; 4],
}
impl NavRigMeshInfo {
    /// NavRig ctor82e51870 explicitly nulls all three cell/handle pairs;
    /// surrounding MemStore construction supplies the cleared scalar storage.
    pub(crate) fn new() -> Self {
        Self { cell: MeshCell::Absent, previous_cell: MeshCell::Absent,
            previous_cell_position: [0.; 4], cell_score: 0., previous_cell_score: 0.,
            last_valid_cell: MeshCell::Absent, last_valid_cell_position: [0.; 4] }
    }
    /// SetPathogenCell82e6e190: query threshold only on an identity change.
    /// `threshold` must read MeshNavigation's current target iterator (zero
    /// when absent), regardless of which navigation implementation is active.
    pub(crate) fn set_cell(
        &mut self, cell: MeshCell, transform_position: [f32; 4],
        mut threshold: impl FnMut() -> Result<f32, String>,
    ) -> Result<(), String> {
        // Null cell and null handle compare equal in the native pointer pair.
        let same = matches!((&self.cell, &cell), (MeshCell::Absent, MeshCell::Absent))
            || self.cell.same(&cell);
        if !same {
            let threshold = threshold()?;
            self.cell_score = cell.score_with_threshold(threshold);
            self.previous_cell_score = self.cell.score_with_threshold(threshold);
            self.previous_cell = self.cell.clone();
            self.cell = cell.clone();
            if cell.is_valid() {
                self.last_valid_cell = cell.clone();
                self.last_valid_cell_position = transform_position;
            }
            self.previous_cell_position = transform_position;
        }
        self.cell = cell;
        Ok(())
    }
    /// ReleaseNavMeshTile82e6e0f8 clears current/previous pairs for that exact
    /// streamed handle. It intentionally retains last-valid, scores and positions.
    /// Caller also forwards release to the existing MeshNavigation owner.
    pub(crate) fn release_tile(&mut self, tile: &Arc<NavMeshAsset>) {
        let handle = Arc::downgrade(tile);
        for cell in [&mut self.cell, &mut self.previous_cell] {
            if matches!(cell, MeshCell::Present { tile, .. } if tile.ptr_eq(&handle)) {
                *cell = MeshCell::Absent;
            }
        }
    }
}
impl NavRig {
    /// Task_UpdatePositionalData82e7305c..30a8 obtains the real current cell
    /// with kDefaultNavMeshCheckYTolerance, then calls SetPathogenCell.
    /// The caller still performs LWBasicEntity::UpdatePositionalData afterward.
    pub(crate) fn update_mesh_cell(
        &mut self, position: [f32; 4], transform_position: [f32; 4],
        y_tolerance: f32, mesh: &MeshStore,
        threshold: impl FnMut() -> Result<f32, String>,
    ) -> Result<(), String> {
        let cell = mesh.try_get_cell_with_tolerance(SkeletonRootVector::known(position), y_tolerance)
            .map_err(|e| format!("pedestrian current mesh cell: {e:?}"))?;
        self.mesh_info.set_cell(cell, transform_position, threshold)
    }
}
