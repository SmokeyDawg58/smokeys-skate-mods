//! PedestrianChaseTargetIterator vtable823a4648. The graph supplies explicit
//! target points; this iterator supplies native path policies and alternatives.
use crate::{
    living_world_mesh_store::MeshMan, living_world_pedestrian_state::PedestrianStateStorage,
    pedestrian_navigation::mesh::NavigationTargetIterator, reckoning_numeric::ReckoningNumeric,
    skeleton_root_transforms::SkeletonRootVector,
};

#[derive(Debug)]
pub(crate) struct PedestrianChaseTargetIterator {
    pub(crate) cost_threshold: f32,
    alternate: Option<[f32; 4]>,
}
impl Default for PedestrianChaseTargetIterator {
    fn default() -> Self {
        Self {
            cost_threshold: 1.0,
            alternate: None,
        }
    } //82e78140
}
impl PedestrianChaseTargetIterator {
    pub(crate) fn reset(&mut self) {
        self.alternate = None;
    } //82e6ce50
    /// SetAlternateTarget82e6ce90 only replaces a valid mesh point. A rejected
    /// candidate leaves the previous alternative intact, rather than clearing it.
    pub(crate) fn set_alternate_target(
        &mut self,
        point: [f32; 4],
        mesh: &MeshMan,
        y_tolerance: f32,
    ) -> Result<(), String> {
        let cell = mesh
            .store
            .try_get_cell_with_tolerance(SkeletonRootVector::known(point), y_tolerance)
            .map_err(|e| format!("chase alternate mesh cell: {e:?}"))?;
        if cell.is_valid() {
            self.alternate = Some(point);
        }
        Ok(())
    }
}
/// Borrowed adapter carries actual registered pedestrian/chasee positions and
/// state; it does not create an independent target or retain a stale entity ID.
pub(crate) struct ChaseIteratorView<'a> {
    pub(crate) iterator: &'a mut PedestrianChaseTargetIterator,
    pub(crate) state: &'a mut PedestrianStateStorage,
    pub(crate) pedestrian_position: Option<[f32; 4]>,
    pub(crate) chasee_position: Option<[f32; 4]>,
    pub(crate) allow_incomplete_path_in_chase: bool,
    pub(crate) chase_nav_fail_distance: f32,
}
impl ChaseIteratorView<'_> {
    fn compare_distance(&self) -> Option<f32> {
        let position = self.pedestrian_position?;
        let target = self.chasee_position?;
        let x = position[0] - target[0];
        let z = position[2] - target[2];
        Some(x * x + z * z - self.chase_nav_fail_distance * self.chase_nav_fail_distance)
    }
}
impl<N: ReckoningNumeric> NavigationTargetIterator<N> for ChaseIteratorView<'_> {
    fn has_target(&self) -> bool {
        true
    } //vtable+8 shared returntrue826c6d80
    fn target(&self) -> Result<[f32; 4], String> {
        Ok([0.0; 4])
    } //raw82e6ce60 zero Vector3
    fn increment(&mut self, _position: [f32; 4], _dt: f32, _numeric: &mut N) -> Result<(), String> {
        Ok(())
    } //native empty825187c0
    fn reset_cost_threshold(&mut self) {
        self.iterator.cost_threshold = 1.0;
    } //82e6a7e8
    fn cost_threshold(&mut self) -> Result<f32, String> {
        Ok(self.iterator.cost_threshold)
    }
    fn allow_incomplete_path(&self) -> bool {
        self.allow_incomplete_path_in_chase
            && self
                .compare_distance()
                .is_some_and(|distance| distance > 0.0)
    }
    fn allow_off_mesh(&self) -> bool {
        true
    } //vtable+60 shared returntrue
    fn alternate_target(&self) -> Option<[f32; 4]> {
        self.iterator.alternate
    }
    fn on_pathing_failure(&mut self, case: i32, _position: [f32; 4]) -> Result<bool, String> {
        //82e782b0 only NoPathFound2/IncompletePath3 change the AI flag;
        // missing pedestrian/chasee preserves its prior value.
        if (2..=3).contains(&case) {
            if let Some(distance) = self.compare_distance() {
                self.state.flags[4] =
                    (self.state.flags[4] & !0x20) | (u8::from(distance <= 0.0) << 5);
            }
        }
        Ok(true)
    }
}
