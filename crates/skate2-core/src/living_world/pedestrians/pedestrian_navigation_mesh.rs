//! MeshNavigation Seek82e75b68/ResolvePath82e76120/Update82e7dc70. Target
//! iterator and entity facts are borrowed from their retained concrete owners.
use super::{NavigationTargetMode, PedestrianNavigation};
use crate::living_world_mesh_store::pathogen::NavPath;
use crate::{
    living_world_mesh_store::{MeshCell, MeshMan},
    reckoning_numeric::ReckoningNumeric,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::fmt::Debug;

pub(crate) trait NavigationTargetIterator<N: ReckoningNumeric> {
    fn has_target(&self) -> bool;
    fn target(&self) -> Result<[f32; 4], String>;
    fn increment(&mut self, position: [f32; 4], dt: f32, numeric: &mut N) -> Result<(), String>;
    fn reset_cost_threshold(&mut self);
    fn cost_threshold(&mut self) -> Result<f32, String>;
    fn allow_incomplete_path(&self) -> bool;
    fn allow_off_mesh(&self) -> bool;
    fn alternate_target(&self) -> Option<[f32; 4]>;
    fn on_pathing_failure(&mut self, case: i32, position: [f32; 4]) -> Result<bool, String>;
}
pub(crate) struct NavigationEntityFacts {
    pub(crate) position: [f32; 4],
    pub(crate) heading: f32,
    pub(crate) cell: MeshCell,
    pub(crate) max_astar_depth: u32,
    pub(crate) can_astar: bool,
    pub(crate) high_fidelity: bool,
    pub(crate) last_valid_cell_position: [f32; 4],
}
#[derive(Clone, Copy)]
pub(crate) struct MeshNavigationTuning {
    pub(crate) allow_astar: bool,
    pub(crate) y_tolerance: f32,
    pub(crate) off_mesh_anchor_radius: f32,
    pub(crate) boundary_tests: [bool; 3],
}
pub(crate) struct MeshNavigation {
    pub(crate) base: PedestrianNavigation,
    pub(crate) path: NavPath,
    pub(crate) seek_cell: MeshCell,
    pub(crate) should_astar: bool,
    pub(crate) pathing_failure_last_seek: bool,
    pub(crate) using_off_mesh: bool,
    pub(crate) off_mesh_target: Option<[f32; 4]>,
    pub(crate) off_mesh_anchor: [f32; 4],
    pub(crate) feedback_target_distance: [f32; 4],
    pub(crate) feedback_rating: [f32; 4],
    pub(crate) astars_for_destination: u32,
    pub(crate) resume_speed: f32,
}
fn error(e: impl Debug) -> String {
    format!("pedestrian mesh navigation: {e:?}")
}
impl MeshNavigation {
    pub(crate) fn release_tile(&mut self, tile: &std::sync::Arc<crate::living_world_mesh_store::asset::NavMeshAsset>) {
        if self.path.uses_tile(tile) {
            self.path.edges.clear();
            self.path.cells.clear();
            self.path.valid = false;
            self.path.start = MeshCell::Absent;
            self.path.end = MeshCell::Absent;
            self.should_astar = true;
        }
    }
    /// ValidateNavRig82e6c1a0 skips off-mesh mode only on the immediate path.
    pub(crate) fn validate_nav_rig<N: ReckoningNumeric>(
        &self,
        directions: &[[f32; 4]; 7],
        distances: &mut super::rig::NavRigDistances,
        numeric: &mut N,
    ) -> Result<(), String>
    where N::Error: Debug,
    {
        if !self.using_off_mesh {
            self.path.resolve_nav_rig(self.base.position, directions, distances, numeric)?;
        }
        Ok(())
    }

    /// AddNavRigPathBoundaryTestsToBatch82e6c2a8 has no off-mesh gate.
    pub(crate) fn add_nav_rig_tests(
        &self,
        directions: &[[f32; 4]; 7],
        distances: &super::rig::NavRigDistances,
        owner: u32,
        batch: &mut super::rig::NavRigBatch,
    ) -> Result<std::ops::Range<usize>, String> {
        self.path.add_nav_rig_tests(self.base.position, directions, distances, owner, batch)
    }

    /// Entity MemStore Init82a6de38 and FreeBlock82a6df68 clear the complete
    /// allocation, including flags not explicitly written by ctor82e85470.
    pub(crate) fn new(base: PedestrianNavigation) -> Self {
        Self {
            base,
            path: NavPath::reset(MeshCell::Absent, MeshCell::Absent),
            seek_cell: MeshCell::Absent,
            should_astar: false,
            pathing_failure_last_seek: false,
            using_off_mesh: false,
            off_mesh_target: None,
            off_mesh_anchor: [0.; 4],
            feedback_target_distance: [0.; 4],
            feedback_rating: [1.; 4],
            astars_for_destination: 0,
            resume_speed: 0.,
        }
    }
    pub(crate) fn reset(&mut self, position: [f32; 4], heading: f32) {
        self.seek_cell = MeshCell::Absent;
        self.base.reset(position, heading);
    }
    pub(crate) fn destination(&self) -> [f32; 4] {
        self.seek_cell
            .cost_and_centroid()
            .map_or(self.base.seek_destination, |(_, point)| point)
    }
    pub(crate) fn set_destination<N: ReckoningNumeric>(
        &mut self,
        point: [f32; 4],
        mesh: &MeshMan,
        tuning: MeshNavigationTuning,
        numeric: &mut N,
    ) -> Result<(), String>
    where
        N::Error: Debug,
    {
        self.base
            .set_seek_destination(point, numeric)
            .map_err(error)?;
        self.seek_cell = mesh
            .store
            .try_get_cell_with_tolerance(SkeletonRootVector::known(point), tuning.y_tolerance)
            .map_err(error)?;
        Ok(())
    }
    pub(crate) fn target_explicit<N: ReckoningNumeric>(
        &mut self,
        point: [f32; 4],
        mode: NavigationTargetMode,
        iterator: &mut (impl NavigationTargetIterator<N> + ?Sized),
        facts: &NavigationEntityFacts,
        mesh: &MeshMan,
        tuning: MeshNavigationTuning,
        numeric: &mut N,
    ) -> Result<(), String>
    where
        N::Error: Debug,
    {
        self.base.target_mode = mode;
        self.set_destination(point, mesh, tuning, numeric)?;
        if mode != NavigationTargetMode::ExplicitNoSeekNoAstar {
            self.seek(true, Some(iterator), facts, mesh, tuning, numeric)?;
        }
        Ok(())
    }
    pub(crate) fn seek<N: ReckoningNumeric>(
        &mut self,
        force: bool,
        iterator: Option<&mut (impl NavigationTargetIterator<N> + ?Sized)>,
        facts: &NavigationEntityFacts,
        mesh: &MeshMan,
        tuning: MeshNavigationTuning,
        numeric: &mut N,
    ) -> Result<(), String>
    where
        N::Error: Debug,
    {
        let force_astar = self.should_astar || force;
        self.pathing_failure_last_seek = false;
        self.base.position = facts.position;
        self.base.heading = facts.heading;
        if let Some(iterator) = iterator {
            if self.base.target_mode != NavigationTargetMode::Auto {
                if !self.path.valid {
                    self.should_astar = true;
                }
                if !self.path.valid || force {
                    iterator.reset_cost_threshold();
                }
            } else if !iterator.has_target() {
                iterator.increment(facts.position, f32::from_bits(0x3e2aaa7e), numeric)?;
                if iterator.has_target() {
                    self.set_destination(iterator.target()?, mesh, tuning, numeric)?;
                }
                self.should_astar = true;
                iterator.reset_cost_threshold();
            } else {
                let target = iterator.target()?;
                let arrived = self
                    .base
                    .has_arrived_xz(facts.position, target, self.base.arrival_radius, numeric)
                    .map_err(error)?;
                let destination = self.destination();
                let zero_destination = !destination[..3]
                    .iter()
                    .any(|v| v.abs() > f32::from_bits(0x34000000));
                if arrived || force || zero_destination {
                    iterator.increment(facts.position, f32::from_bits(0x3e2aaa7e), numeric)?;
                    iterator.reset_cost_threshold();
                    self.should_astar = true;
                    self.set_destination(iterator.target()?, mesh, tuning, numeric)?;
                } else {
                    let mut at_end = arrived;
                    if let Some((_, end)) = self.path.end.cost_and_centroid() {
                        at_end = self
                            .base
                            .has_arrived_xz(facts.position, end, self.base.arrival_radius, numeric)
                            .map_err(error)?;
                        if !self.path.cells.iter().any(|cell| {
                            cell.is_valid() && facts.cell.is_valid() && cell.same(&facts.cell)
                        }) {
                            self.should_astar = true;
                        }
                    }
                    self.should_astar |= at_end;
                }
            }
        }
        self.should_astar =
            force_astar || self.should_astar && self.base.speed() > 0. && facts.can_astar;
        Ok(())
    }
    fn failure<N: ReckoningNumeric>(
        &mut self,
        case: i32,
        iterator: &mut (impl NavigationTargetIterator<N> + ?Sized),
        facts: &NavigationEntityFacts,
        mesh: &MeshMan,
        tuning: MeshNavigationTuning,
        numeric: &mut N,
    ) -> Result<(), String>
    where
        N::Error: Debug,
    {
        self.feedback_target_distance = [0.; 4];
        self.astars_for_destination = 0;
        if iterator.on_pathing_failure(case, facts.position)? {
            self.seek(true, Some(iterator), facts, mesh, tuning, numeric)?;
        }
        self.pathing_failure_last_seek = true;
        Ok(())
    }
    pub(crate) fn resolve_path<N: ReckoningNumeric>(
        &mut self,
        iterator: &mut (impl NavigationTargetIterator<N> + ?Sized),
        facts: &NavigationEntityFacts,
        mesh: &MeshMan,
        tuning: MeshNavigationTuning,
        point_in_nav_rig: &mut impl FnMut([f32; 4], &mut N) -> Result<bool, String>,
        numeric: &mut N,
    ) -> Result<(), String>
    where
        N::Error: Debug,
    {
        if !self.should_astar || !tuning.allow_astar || !facts.can_astar {
            return Ok(());
        }
        let mut destination = self.destination();
        let start = facts.cell.clone();
        let mut end = self.seek_cell.clone();
        self.off_mesh_target = None;
        let allow_off_mesh = iterator.allow_off_mesh();
        let alternate = iterator.alternate_target();
        let mut solve = false;
        if !self.using_off_mesh {
            solve = true;
            if !start.is_valid() {
                solve = false;
                if allow_off_mesh {
                    self.using_off_mesh = true;
                } else {
                    self.failure(0, iterator, facts, mesh, tuning, numeric)?;
                }
            }
            if solve && !end.is_valid() {
                solve = false;
                if let Some(point) = alternate {
                    destination = point;
                    end = mesh
                        .store
                        .try_get_cell_with_tolerance(
                            SkeletonRootVector::known(point),
                            tuning.y_tolerance,
                        )
                        .map_err(error)?;
                    if end.is_valid() {
                        solve = true;
                        if allow_off_mesh && end.same(&start) {
                            self.using_off_mesh = true;
                            self.off_mesh_anchor = point;
                        }
                    }
                }
                if !solve {
                    self.failure(1, iterator, facts, mesh, tuning, numeric)?;
                }
            }
        } else {
            if start.is_valid() {
                let mut return_to_mesh = true;
                if !end.is_valid() && allow_off_mesh {
                    self.off_mesh_target = Some(destination);
                    return_to_mesh = false;
                    if let Some(point) = alternate {
                        let unchanged = !(0..3).any(|i| {
                            (point[i] - self.off_mesh_anchor[i]).abs() > f32::from_bits(0x37800000)
                        });
                        let arrived = self
                            .base
                            .has_arrived_xz(
                                facts.position,
                                point,
                                tuning.off_mesh_anchor_radius,
                                numeric,
                            )
                            .map_err(error)?;
                        if !unchanged && !arrived {
                            end = mesh
                                .store
                                .try_get_cell_with_tolerance(
                                    SkeletonRootVector::known(point),
                                    tuning.y_tolerance,
                                )
                                .map_err(error)?;
                            if end.is_valid() {
                                self.off_mesh_target = None;
                                destination = point;
                                return_to_mesh = true;
                            }
                        }
                    }
                }
                if return_to_mesh {
                    self.using_off_mesh = false;
                    solve = true;
                }
            }
            if !start.is_valid() || self.off_mesh_target.is_some() {
                if point_in_nav_rig(destination, numeric)? {
                    self.off_mesh_target = Some(destination);
                } else if end.is_valid() {
                    self.off_mesh_target = Some(facts.last_valid_cell_position);
                } else {
                    // This branch does not reset A* feedback or last-seek flag.
                    if iterator.on_pathing_failure(3, facts.position)? {
                        self.seek(true, Some(iterator), facts, mesh, tuning, numeric)?;
                    }
                    self.off_mesh_target = None;
                }
            }
        }
        if solve {
            self.path = NavPath::reset(start, end);
            mesh.solver.solve(
                &mesh.store,
                &mut self.path,
                facts.max_astar_depth,
                iterator.cost_threshold()?,
                iterator.allow_incomplete_path(),
                numeric,
            )?;
            self.should_astar = false;
            if self.path.valid {
                mesh.solver.boundary(
                    &mesh.store,
                    &mut self.path,
                    iterator.cost_threshold()?,
                    tuning.boundary_tests,
                )?;
                self.should_astar = false;
            } else {
                self.failure(2, iterator, facts, mesh, tuning, numeric)?;
                self.should_astar = false;
            }
        }
        Ok(())
    }
    pub(crate) fn update<N: ReckoningNumeric>(
        &mut self,
        dt: f32,
        iterator: Option<&(impl NavigationTargetIterator<N> + ?Sized)>,
        facts: &NavigationEntityFacts,
        mesh: &MeshMan,
        tuning: MeshNavigationTuning,
        numeric: &mut N,
    ) -> Result<(), String>
    where
        N::Error: Debug,
    {
        self.base.update_speed(dt);
        self.base.update_seek_direction(numeric).map_err(error)?;
        let Some(iterator) = iterator else {
            return Ok(());
        };
        if self.base.target_mode == NavigationTargetMode::ExplicitNoSeekNoAstar {
            return Ok(());
        }
        if self.base.target_mode == NavigationTargetMode::Auto
            && self
                .base
                .has_arrived_xz(
                    facts.position,
                    self.destination(),
                    self.base.arrival_radius,
                    numeric,
                )
                .map_err(error)?
        {
            self.should_astar = true;
            self.path.edges.clear();
            self.path.valid = false;
            self.set_destination(iterator.target()?, mesh, tuning, numeric)?;
            return Ok(());
        }
        self.path.high_fidelity_validation = facts.high_fidelity;
        if self.using_off_mesh {
            if let Some(point) = self.off_mesh_target {
                self.base.set_path_target(point, numeric).map_err(error)?;
            }
            return Ok(());
        }
        if !self.path.valid {
            self.should_astar = true;
            return Ok(());
        }
        if self
            .base
            .has_arrived_xz(
                facts.position,
                self.destination(),
                self.base.arrival_radius,
                numeric,
            )
            .map_err(error)?
        {
            self.should_astar = true;
        }
        if let Some(point) =
            self.path
                .furthest_target(self.base.position, self.base.seek_destination, numeric)?
        {
            self.base.set_path_target(point, numeric).map_err(error)?;
        }
        Ok(())
    }
}
