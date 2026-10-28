//! Retained native target iterators and SwitchTargetingMode82e6d548.
use super::{ActiveNavigation, PedestrianConstruction};
use crate::{
    input_cvar_runtime::InputCvarReferences,
    living_world_mesh_store::MeshMan,
    living_world_rain::RainMan,
    native_skater_random::{NativeRandomGate, NativeSkaterRandom},
    pedestrian_navigation::{NavigationTargetMode, mesh::{MeshNavigation, NavigationEntityFacts, NavigationTargetIterator},
        road::{RoadNetworkTargetIterator, RoadIteratorFrame, RoadNavigationContext},
        wander::{NoRoadWanderTargetIterator, NoRoadIteratorFrame, WanderAvoidVolumes}},
    reckoning_numeric::{ReckoningNumeric, try_normalize_safe, cross_full4},
};
use std::fmt::Debug;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TargetIteratorKind { Fixed, Waypoint, Road, Chase, Flee, VehicleFlee, NoRoad, Random }

pub(crate) struct PointTargetIterator {
    pub(crate) target: [f32; 4],
    pub(crate) cost_threshold: f32,
    pub(crate) pedestrian: Option<u32>,
}
impl PointTargetIterator {
    fn new(cost_threshold: f32) -> Self {
        Self { target: [0.; 4], cost_threshold, pedestrian: None }
    }
}
pub(crate) struct TargetIterators {
    pub(crate) road: RoadNetworkTargetIterator,
    pub(crate) no_road: NoRoadWanderTargetIterator,
    pub(crate) fixed: PointTargetIterator,
    pub(crate) random: PointTargetIterator,
    pub(crate) flee: PointTargetIterator,
    pub(crate) flee_direction: Option<[f32; 4]>,
    pub(crate) vehicle_flee: PointTargetIterator,
    pub(crate) vehicle_id: i32,
    pub(crate) waypoint_index: u32,
    pub(crate) waypoint_cost_threshold: f32,
    pub(crate) waypoint_pedestrian: Option<u32>,
    pub(crate) chase_pedestrian: Option<u32>,
    pub(crate) chase_cost_threshold: f32,
    pub(crate) chase_alternate: Option<[f32; 4]>,
    /// The two navigation owners each retain their own iterator pointer.
    pub(crate) mesh_current: Option<TargetIteratorKind>,
    pub(crate) physics_current: Option<TargetIteratorKind>,
}
impl TargetIterators {
    /// SecurityGuard.PreStateGraphUpdate82e9f5d8..610 addresses the mesh
    /// iterator independently of whichever navigation is currently active.
    pub(crate) fn set_mesh_cost_threshold(&mut self, value: f32) {
        let Some(kind) = self.mesh_current else { return; };
        *match kind {
            TargetIteratorKind::Fixed => &mut self.fixed.cost_threshold,
            TargetIteratorKind::Waypoint => &mut self.waypoint_cost_threshold,
            TargetIteratorKind::Road => &mut self.road.cost_threshold,
            TargetIteratorKind::Chase => &mut self.chase_cost_threshold,
            TargetIteratorKind::Flee => &mut self.flee.cost_threshold,
            TargetIteratorKind::VehicleFlee => &mut self.vehicle_flee.cost_threshold,
            TargetIteratorKind::NoRoad => &mut self.no_road.cost_threshold,
            TargetIteratorKind::Random => &mut self.random.cost_threshold,
        } = value;
    }

    pub(crate) fn new() -> Self {
        Self { road: RoadNetworkTargetIterator::new(), no_road: NoRoadWanderTargetIterator::new(),
            fixed: PointTargetIterator::new(0.), random: PointTargetIterator::new(0.),
            flee: PointTargetIterator::new(0.), flee_direction: None,
            vehicle_flee: PointTargetIterator::new(0.3), vehicle_id: -1,
            waypoint_index: 0, waypoint_cost_threshold: 0., waypoint_pedestrian: None, chase_pedestrian: None,
            chase_cost_threshold: 1., chase_alternate: None,
            mesh_current: None, physics_current: None }
    }
}

/// Resolved wants[10] targetable owner, not a cached invented flee target.
pub(crate) struct FleeSource { pub(crate) position: [f32; 4], pub(crate) velocity: [f32; 4] }
pub(crate) struct TargetingWorld<'a, A: WanderAvoidVolumes> {
    pub(crate) dmos: &'a mut crate::living_world_grind_services::dmo_objects::DmoObjectMan,
    pub(crate) mesh: &'a MeshMan,
    pub(crate) rain: &'a mut RainMan,
    pub(crate) random: &'a mut NativeSkaterRandom,
    pub(crate) avoid: &'a A,
    pub(crate) cvars: &'a InputCvarReferences,
    /// Result of resolving this entity's wants[10] through real target owners.
    pub(crate) flee_source: Option<FleeSource>,
    /// Resolved Chaser.GetChasee target position for this operation.
    pub(crate) chasee_position: Option<[f32; 4]>,
    pub(crate) props: &'a mut crate::living_world_grind_services::waypoints::PropMan,
}

struct WaypointFrame<'a> {
    props:&'a mut crate::living_world_grind_services::waypoints::PropMan,
    current:&'a mut Option<crate::living_world_grind_services::waypoints::WaypointRef>,
    index:&'a mut u32,
    pedestrian:Option<u32>,
    cost:&'a mut f32,
}
impl WaypointFrame<'_> {
    fn advance(&mut self)->Result<(),String>{
        let pedestrian=self.pedestrian.ok_or("waypoint iterator pedestrian pointer absent")?;
        let Some(current)=*self.current else{return Ok(());};
        let group=self.props.group_mut(current.prop)?;
        //82e77d0c: missing current-index point exits without resetting the index.
        let Some(point)=group.points.get_mut(*self.index as usize) else{return Ok(());};
        point.unlock();
        let count=group.points.len() as u32;
        for _ in 0..count {
            *self.index=self.index.wrapping_add(1);
            if *self.index>=count{*self.index=0;}
            let next=group.points.get_mut(*self.index as usize).ok_or("waypoint next index absent")?;
            if next.locking_entity.is_none(){
                next.lock(pedestrian)?;
                *self.current=Some(crate::living_world_grind_services::waypoints::WaypointRef{prop:current.prop,index:*self.index});
                break;
            }
        }
        Ok(())
    }
}
impl<N:ReckoningNumeric> NavigationTargetIterator<N> for WaypointFrame<'_> {
    fn has_target(&self)->bool{true}
    fn target(&self)->Result<[f32;4],String>{
        if self.pedestrian.is_none(){return Ok([0.;4]);}
        let Some(current)=*self.current else{return Ok([0.;4]);};
        let group=self.props.group(current.prop)?;
        Ok(group.points.get(*self.index as usize).map_or([0.;4],|point|point.location))
    }
    fn increment(&mut self,_position:[f32;4],_dt:f32,_numeric:&mut N)->Result<(),String>{self.advance()}
    fn reset_cost_threshold(&mut self){*self.cost=0.;}
    fn cost_threshold(&mut self)->Result<f32,String>{Ok(*self.cost)}
    fn allow_incomplete_path(&self)->bool{true}
    fn allow_off_mesh(&self)->bool{false}
    fn alternate_target(&self)->Option<[f32;4]>{None}
    fn on_pathing_failure(&mut self,_case:i32,_position:[f32;4])->Result<bool,String>{self.advance()?;Ok(true)}
}

struct ChaseFrame<'a> {
    cost: &'a mut f32,
    alternate: &'a Option<[f32;4]>,
    state: &'a mut crate::living_world_pedestrian_state::PedestrianStateStorage,
    pedestrian_present: bool,
    position: [f32;4],
    chasee: Option<[f32;4]>,
    cvars: &'a InputCvarReferences,
    allow_incomplete: bool,
}
fn compare_distance_xz(a:[f32;4],b:[f32;4],radius:f32)->f32 {
    let delta=[a[0]-b[0],0.,a[2]-b[2],0.];
    crate::reckoning_numeric::host_dot3(delta,delta)-radius*radius
}
impl<N: ReckoningNumeric> NavigationTargetIterator<N> for ChaseFrame<'_> {
    fn has_target(&self)->bool{true}
    // Native vtable823a4648+c resolves to zero-vector getter82e6ce60.
    fn target(&self)->Result<[f32;4],String>{Ok([0.;4])}
    fn increment(&mut self,_position:[f32;4],_dt:f32,_numeric:&mut N)->Result<(),String>{Ok(())}
    fn reset_cost_threshold(&mut self){*self.cost=1.;}
    fn cost_threshold(&mut self)->Result<f32,String>{Ok(*self.cost)}
    fn allow_incomplete_path(&self)->bool{self.allow_incomplete}
    fn allow_off_mesh(&self)->bool{true}
    fn alternate_target(&self)->Option<[f32;4]>{*self.alternate}
    fn on_pathing_failure(&mut self,case:i32,_position:[f32;4])->Result<bool,String>{
        if (2..=3).contains(&case) && self.pedestrian_present {
            if let Some(chasee)=self.chasee {
                let radius=self.cvars.try_pedestrian_chase_nav_fail_distance()
                    .map_err(|e| format!("chase failure distance: {e:?}"))?;
                let close=compare_distance_xz(self.position,chasee,radius)<=0.;
                self.state.flags[4]=(self.state.flags[4]&!0x20)|if close{0x20}else{0};
            }
        }
        Ok(true)
    }
}

fn increment_flee<N: ReckoningNumeric>(iterator: &mut PointTargetIterator,
    direction: &mut Option<[f32; 4]>, position: [f32; 4], source: Option<&FleeSource>,
    numeric: &mut N) -> Result<(), String> where N::Error: Debug {
    let mut value = [1.,0.,0.,0.];
    if iterator.pedestrian.is_some() {
        if let Some(source) = source {
            let velocity = try_normalize_safe([source.velocity[0],0.,source.velocity[2],0.], [0.;4], numeric)
                .map_err(|e| format!("flee velocity: {e:?}"))?;
            let delta = [position[0]-source.position[0],0.,position[2]-source.position[2],0.];
            let dot = numeric.dot3(velocity, delta).map_err(|e| format!("flee dot: {e:?}"))?;
            if dot < 0. {
                value = try_normalize_safe(delta,[0.;4],numeric).map_err(|e| format!("flee delta: {e:?}"))?;
            }
            else {
                let mut side = cross_full4(velocity,[0.,1.,0.,0.]);
                if numeric.dot3(side,delta).map_err(|e| format!("flee side: {e:?}"))? < 0. {
                    side = side.map(|v| -v);
                }
                let away = try_normalize_safe(delta,[0.;4],numeric).map_err(|e| format!("flee delta: {e:?}"))?;
                value = std::array::from_fn(|i| (side[i]+away[i])*0.5);
            }
        }
    }
    *direction = Some(value);
    iterator.target = std::array::from_fn(|i| value[i].mul_add(3.,position[i]));
    Ok(())
}

struct FleeFrame<'a> {
    iterator: &'a mut PointTargetIterator,
    direction: &'a mut Option<[f32;4]>,
    source: Option<&'a FleeSource>,
}
impl<N: ReckoningNumeric> NavigationTargetIterator<N> for FleeFrame<'_> where N::Error: Debug {
    fn has_target(&self)->bool {true}
    fn target(&self)->Result<[f32;4],String>{Ok(self.iterator.target)}
    fn increment(&mut self,position:[f32;4],_dt:f32,numeric:&mut N)->Result<(),String>{
        increment_flee(self.iterator,self.direction,position,self.source,numeric)
    }
    fn reset_cost_threshold(&mut self){self.iterator.cost_threshold=0.3;}
    fn cost_threshold(&mut self)->Result<f32,String>{Ok(self.iterator.cost_threshold)}
    fn allow_incomplete_path(&self)->bool{false}
    fn allow_off_mesh(&self)->bool{false}
    fn alternate_target(&self)->Option<[f32;4]>{None}
    fn on_pathing_failure(&mut self,_case:i32,_position:[f32;4])->Result<bool,String>{Ok(false)}
}

struct PointFrame<'a> {
    iterator: &'a mut PointTargetIterator,
    kind: TargetIteratorKind,
    random: &'a mut NativeSkaterRandom,
}
impl<N: ReckoningNumeric> NavigationTargetIterator<N> for PointFrame<'_> {
    fn has_target(&self) -> bool { true }
    fn target(&self) -> Result<[f32; 4], String> { Ok(self.iterator.target) }
    fn increment(&mut self, position: [f32; 4], _dt: f32, _numeric: &mut N) -> Result<(), String> {
        if self.kind == TargetIteratorKind::Random {
            let value = self.random.try_living_world_range(&NativeRandomGate::production_shared(), 0., 1.)
                .map_err(|e| format!("pedestrian random direction: {e:?}"))?;
            // RandomDirection.Increment82e6cf30: no normalization of diagonals.
            let directions: [[f32; 4]; 8] = [[1.,0.,0.,0.],[-1.,0.,0.,0.],
                [0.,0.,1.,0.],[0.,0.,-1.,0.],[-1.,0.,-1.,0.],
                [1.,0.,-1.,0.],[-1.,0.,1.,0.],[1.,0.,1.,0.]];
            let direction = directions.get((value * 8.) as usize).copied().unwrap_or([0.;4]);
            self.iterator.target = std::array::from_fn(|i| direction[i].mul_add(40., position[i]));
        }
        Ok(())
    }
    fn reset_cost_threshold(&mut self) {
        self.iterator.cost_threshold = if self.kind == TargetIteratorKind::VehicleFlee {0.3} else {0.};
    }
    fn cost_threshold(&mut self) -> Result<f32, String> { Ok(self.iterator.cost_threshold) }
    fn allow_incomplete_path(&self) -> bool { false }
    fn allow_off_mesh(&self) -> bool { false }
    fn alternate_target(&self) -> Option<[f32;4]> { None }
    fn on_pathing_failure(&mut self, _case:i32, _position:[f32;4]) -> Result<bool,String> { Ok(false) }
}

impl PedestrianConstruction {
    /// BlockChasee.Begin826ff0e0 and PursueChasee.Begin826ff1a8.
    /// Preserve targeting-before-navigation switching and the second forced
    /// Seek; switching targeting already performs its own nonforced Seek.
    pub(crate) fn begin_chase_navigation<N: ReckoningNumeric, A: WanderAvoidVolumes>(
        &mut self, facts: &NavigationEntityFacts, world: &mut TargetingWorld<'_, A>, numeric: &mut N,
    ) -> Result<(), String> where N::Error: Debug {
        self.navigation_mut()?.target_mode = NavigationTargetMode::Explicit;
        self.switch_targeting_mode(3, facts, world, numeric)?;
        self.switch_navigation_mode(1);
        self.navigation_smoothing_enabled = false;
        self.seek_current_target(true, facts, world, numeric)
    }

    /// ITargetIterator.SetAlternateTarget slot56: all concrete iterators except
    /// PedestrianChase use native empty825187c0. Chase82e6ce90 only replaces
    /// its previous alternate when the target has a valid current mesh cell.
    pub(crate) fn set_chase_alternate_target(&mut self, target: [f32; 4],
        mesh: &MeshMan, cvars: &InputCvarReferences) -> Result<(), String> {
        if self.current_target_iterator() != Some(TargetIteratorKind::Chase) { return Ok(()); }
        let tuning = cvars.try_pedestrian_mesh_tuning().map_err(|e| format!("chase mesh tuning: {e:?}"))?;
        let cell = mesh.store.try_get_cell_with_tolerance(
            crate::skeleton_root_transforms::SkeletonRootVector::known(target), tuning.y_tolerance)
            .map_err(|e| format!("chase alternate target cell: {e:?}"))?;
        if cell.is_valid() { self.target_iterators.chase_alternate = Some(target); }
        Ok(())
    }

    /// Exact selector values0..6. Road fallback is internal, not mode7.
    /// The final Seek(false) also occurs for unrecognized values. No-road
    /// explicitly performs Seek(true) first. Iterator assignment itself has
    /// no reset side effects (SetCurrentTargetIterator82e6bb68).
    pub(crate) fn switch_targeting_mode<N: ReckoningNumeric,A: WanderAvoidVolumes>(
        &mut self, mode: u32, facts: &NavigationEntityFacts,
        world: &mut TargetingWorld<'_,A>, numeric:&mut N,
    )->Result<(),String> where N::Error: Debug {
        let exists = match self.active_navigation {ActiveNavigation::Mesh=>self.mesh_navigation.is_some(),
            ActiveNavigation::Physics=>self.physics_navigation.is_some()};
        if !exists {return Ok(());}
        let selected=match mode {
            0=>Some(TargetIteratorKind::Fixed), 1=>Some(TargetIteratorKind::Waypoint),
            2=>Some(if !world.rain.chunks.is_empty() && world.cvars.try_pedestrian_do_rain_iteration()
                .map_err(|e| format!("RAIN iteration cvar: {e:?}"))? {TargetIteratorKind::Road}else{TargetIteratorKind::Random}),
            3=>Some(TargetIteratorKind::Chase),4=>Some(TargetIteratorKind::Flee),
            5=>Some(TargetIteratorKind::VehicleFlee),6=>Some(TargetIteratorKind::NoRoad),_=>None,
        };
        if let Some(selected)=selected {
            match self.active_navigation {ActiveNavigation::Mesh=>self.target_iterators.mesh_current=Some(selected),
                ActiveNavigation::Physics=>self.target_iterators.physics_current=Some(selected)}
            match selected {
                TargetIteratorKind::Fixed=>{
                    let point=self.state.as_ref().ok_or("pedestrian AI state absent")?.fixed_target
                        .ok_or("native fixed target has no producer")?;
                    self.target_iterators.fixed.target=[point[0],point[1],point[2],0.];
                }
                TargetIteratorKind::Waypoint=>self.target_iterators.waypoint_pedestrian=Some(self.id),
                TargetIteratorKind::Road|TargetIteratorKind::Random=>self.navigation_mut()?.arrival_radius=2.,
                TargetIteratorKind::Chase=>{
                    self.target_iterators.chase_alternate=None;
                    self.navigation_mut()?.arrival_radius=2.;
                    self.target_iterators.chase_pedestrian=Some(self.id);
                }
                TargetIteratorKind::Flee=>{
                    self.navigation_mut()?.target_mode=NavigationTargetMode::Auto;
                    self.navigation_mut()?.arrival_radius=4.;
                    self.target_iterators.flee.pedestrian=Some(self.id);
                    increment_flee(&mut self.target_iterators.flee,&mut self.target_iterators.flee_direction,
                        facts.position,world.flee_source.as_ref(),numeric)?;
                }
                TargetIteratorKind::VehicleFlee=>self.target_iterators.vehicle_id=
                    self.state.as_ref().ok_or("pedestrian AI state absent")?.honking_vehicle_id,
                TargetIteratorKind::NoRoad=>{
                    self.navigation_mut()?.target_mode=NavigationTargetMode::Auto;
                    self.navigation_mut()?.arrival_radius=4.;
                    self.seek_current_target(true,facts,world,numeric)?;
                }
            }
        }
        self.seek_current_target(false,facts,world,numeric)
    }
    pub(crate) fn current_target_iterator(&self) -> Option<TargetIteratorKind> {
        match self.active_navigation { ActiveNavigation::Mesh => self.target_iterators.mesh_current,
            ActiveNavigation::Physics => self.target_iterators.physics_current }
    }

    /// InvalidateNavTargetsOnEnd826eed40 refreshes the retained no-road
    /// iterator, then explicitly targets the current position on active nav.
    /// It neither switches iterators nor clears all navigation state.
    pub(crate) fn invalidate_navigation_targets<N: ReckoningNumeric, A: WanderAvoidVolumes>(
        &mut self, facts: &NavigationEntityFacts,
        world: &mut TargetingWorld<'_, A>, numeric: &mut N,
    ) -> Result<(), String> where N::Error: Debug {
        // Native IncrementTarget receives bits3e2aaa7e; this iterator's
        // implementation ignores dt. Reuse its existing adapter unchanged.
        self.with_target_iterator(TargetIteratorKind::NoRoad, facts, world, numeric,
            |_, iterator, numeric, _| iterator.increment(facts.position,
                f32::from_bits(0x3e2aaa7e), numeric))?;
        self.target_explicit(facts.position, NavigationTargetMode::Explicit, facts, world, numeric)
    }

    /// Shared virtual TargetExplicit82e6be90 over the active navigation owner.
    pub(crate) fn target_explicit<N: ReckoningNumeric, A: WanderAvoidVolumes>(
        &mut self, position: [f32; 4], mode: NavigationTargetMode,
        facts: &NavigationEntityFacts, world: &mut TargetingWorld<'_, A>, numeric: &mut N,
    ) -> Result<(), String> where N::Error: Debug {
        self.navigation_mut()?.target_mode = mode;
        match self.active_navigation {
            ActiveNavigation::Mesh => {
                let tuning = world.cvars.try_pedestrian_mesh_tuning()
                    .map_err(|e| format!("mesh tuning: {e:?}"))?;
                self.mesh_navigation.as_mut().ok_or("pedestrian mesh navigation absent")?
                    .set_destination(position, world.mesh, tuning, numeric)?;
            }
            ActiveNavigation::Physics => self.physics_navigation.as_mut()
                .ok_or("pedestrian physics navigation absent")?
                .set_seek_destination(position, numeric)
                .map_err(|e| format!("explicit destination: {e:?}"))?,
        }
        if mode != NavigationTargetMode::ExplicitNoSeekNoAstar {
            self.seek_current_target(true, facts, world, numeric)?;
        }
        Ok(())
    }

    /// Flee.Begin826ed228: enable rig, switch target4, then remember want10.
    pub(crate) fn begin_flee<N: ReckoningNumeric, A: WanderAvoidVolumes>(
        &mut self, facts: &NavigationEntityFacts, world: &mut TargetingWorld<'_, A>, numeric: &mut N,
    ) -> Result<(), String> where N::Error: Debug {
        self.navigation_rig_enabled = true;
        self.switch_targeting_mode(4, facts, world, numeric)?;
        self.ignore_avoid_target = self.state.as_ref().ok_or("Flee state absent")?.wants[10].target;
        Ok(())
    }

    /// Flee.Update826ed288 uses IChaser.GetMaxChaseSpeed, then increments
    /// the active iterator before targeting its result explicitly.
    pub(crate) fn update_flee<N: ReckoningNumeric, A: WanderAvoidVolumes>(
        &mut self, facts: &NavigationEntityFacts, world: &mut TargetingWorld<'_, A>, numeric: &mut N,
    ) -> Result<(), String> where N::Error: Debug {
        let speed = self.max_chase_speed()?;
        self.navigation_mut()?.seek_speed = speed;
        let current = self.current_target_iterator().ok_or("Flee current iterator absent")?;
        let target = self.with_target_iterator(current, facts, world, numeric,
            |_, iterator, numeric, _| {
                iterator.increment(facts.position, f32::from_bits(0x3e2aaa7e), numeric)?;
                iterator.target()
            })?;
        self.target_explicit(target, NavigationTargetMode::Explicit, facts, world, numeric)
    }
    /// Road vtable823a4520+40 and no-road share90-degree82e77b38;
    /// other original target iterators use base180-degree82e6a6c8.
    pub(crate) fn current_target_max_turn_threshold(&self) -> Option<f32> {
        self.current_target_iterator().map(|kind| match kind {
            TargetIteratorKind::Road|TargetIteratorKind::NoRoad=>90., _=>180.,
        })
    }

    /// Ordinary road/random/fixed/vehicle/no-road modes run the shared mesh
    /// Seek kernel using their retained concrete iterator. Physics Seek only
    /// refreshes position/heading82e6c308; it does not advance the iterator.
    pub(crate) fn seek_current_target<N: ReckoningNumeric, A: WanderAvoidVolumes>(
        &mut self, force: bool, facts: &NavigationEntityFacts,
        world: &mut TargetingWorld<'_, A>, numeric: &mut N,
    ) -> Result<(), String> where N::Error: Debug {
        if let ActiveNavigation::Physics = self.active_navigation {
            let nav = self.physics_navigation.as_mut().ok_or("pedestrian physics navigation absent")?;
            nav.heading = facts.heading; nav.position = facts.position;
            return Ok(());
        }
        let tuning = world.cvars.try_pedestrian_mesh_tuning().map_err(|e| format!("mesh tuning: {e:?}"))?;
        if self.target_iterators.mesh_current.is_none() {
            return self.mesh_navigation.as_mut().ok_or("pedestrian mesh navigation absent")?
                .seek(force, None::<&mut dyn NavigationTargetIterator<N>>, facts, world.mesh, tuning, numeric);
        }
        self.with_current_mesh_iterator(facts, world, numeric, |nav, iterator, numeric, mesh|
            nav.seek(force, Some(iterator), facts, mesh, tuning, numeric))
    }

    /// Also used by frame Update/ResolvePath so they borrow the same iterator
    /// state instead of constructing an unrelated per-frame target source.
    pub(crate) fn with_current_mesh_iterator<N: ReckoningNumeric, A: WanderAvoidVolumes>(
        &mut self, facts: &NavigationEntityFacts, world: &mut TargetingWorld<'_, A>, numeric: &mut N,
        execute: impl FnOnce(&mut MeshNavigation, &mut dyn NavigationTargetIterator<N>, &mut N, &MeshMan) -> Result<(), String>,
    ) -> Result<(), String> where N::Error: Debug {
        let current = self.target_iterators.mesh_current.ok_or("mesh target iterator absent")?;
        self.with_target_iterator(current, facts, world, numeric, execute)
    }

    /// Task_UpdatePositionalData82e72fd8: cell history before player distance.
    /// SetPathogenCell reads MESH's iterator even when physics nav is active,
    /// and calls its cost getter only when the cell identity changes.
    pub(crate) fn update_positional_data<N: ReckoningNumeric, A: WanderAvoidVolumes>(
        &mut self, player_position: [f32; 4], y_tolerance: f32,
        facts: &NavigationEntityFacts, world: &mut TargetingWorld<'_, A>, numeric: &mut N,
    ) -> Result<(), String> where N::Error: Debug {
        let position = self.core.as_ref().ok_or("pedestrian position core absent")?.transform[3];
        let mesh = world.mesh;
        let current = self.target_iterators.mesh_current;
        let mut rig = self.nav_rig.take().ok_or("pedestrian position rig absent")?;
        let result = rig.update_mesh_cell(position, position, y_tolerance, &mesh.store, || {
            match current {
                None => Ok(0.),
                Some(kind) => self.with_target_iterator(kind, facts, world, numeric,
                    |_, iterator, _, _| iterator.cost_threshold()),
            }
        });
        self.nav_rig = Some(rig);
        result?;
        self.core.as_mut().ok_or("pedestrian position core absent")?.player_distance
            .update(player_position, position, numeric)
            .map_err(|e| format!("pedestrian player distance: {e:?}"))
    }

    /// Task_NavigationDoUpdate82e73490 uses the execution target's timestep.
    /// PhysicsNavigation vtable823a4390+36 is folded empty825187c0.
    /// MeshNavigation82e7dc70 additionally updates direction and retained path.
    pub(crate) fn update_current_navigation<N: ReckoningNumeric, A: WanderAvoidVolumes>(
        &mut self, dt: f32, facts: &NavigationEntityFacts,
        world: &mut TargetingWorld<'_, A>, numeric: &mut N,
    ) -> Result<(), String> where N::Error: Debug {
        if let ActiveNavigation::Physics = self.active_navigation {
            return Ok(());
        }
        // The task's GetNavigation null check precedes any tuning reads.
        if self.mesh_navigation.is_none() { return Ok(()); }
        let tuning = world.cvars.try_pedestrian_mesh_tuning()
            .map_err(|e| format!("navigation update tuning: {e:?}"))?;
        if self.target_iterators.mesh_current.is_none() {
            return self.mesh_navigation.as_mut().expect("checked mesh navigation")
                .update(dt, None::<&dyn NavigationTargetIterator<N>>, facts, world.mesh, tuning, numeric);
        }
        self.with_current_mesh_iterator(facts, world, numeric, |nav, iterator, numeric, mesh|
            nav.update(dt, Some(&*iterator), facts, mesh, tuning, numeric))
    }

    /// Task_AStarNavigation82e733f8 dispatches ResolvePath on the active
    /// navigation. Physics navigation has the original empty implementation.
    pub(crate) fn resolve_current_navigation<N: ReckoningNumeric, A: WanderAvoidVolumes>(
        &mut self, facts: &NavigationEntityFacts,
        world: &mut TargetingWorld<'_, A>, numeric: &mut N,
    ) -> Result<(), String> where N::Error: Debug {
        if let ActiveNavigation::Physics = self.active_navigation { return Ok(()); }
        let Some(nav) = self.mesh_navigation.as_ref() else { return Ok(()); };
        let tuning = world.cvars.try_pedestrian_mesh_tuning()
            .map_err(|e| format!("navigation path tuning: {e:?}"))?;
        if !nav.should_astar || !tuning.allow_astar || !facts.can_astar { return Ok(()); }
        // ResolvePath82e761d4..dc returns when GetCurrentTargetIterator is null.
        // TrackProtectee can set an explicit destination without assigning one.
        // Preserve the pending A* flag and retained path, as the native does.
        if self.target_iterators.mesh_current.is_none() { return Ok(()); }
        // Temporarily split the same rig out for an immutable query while the
        // retained iterator/navigation are mutably borrowed. Restore on errors.
        let rig = self.nav_rig.take().ok_or("path resolution rig absent")?;
        let result = self.with_current_mesh_iterator(facts, world, numeric,
            |nav, iterator, numeric, mesh| nav.resolve_path(iterator, facts, mesh, tuning,
                &mut |point, numeric| rig.contains_point(point, numeric), numeric));
        self.nav_rig = Some(rig);
        result
    }

    /// Pedestrian::OnPathingFailure82e6e4a0: notify the active iterator;
    /// force Seek only when that iterator requests it. Unlike MeshNavigation's
    /// internal failure path this does not reset A* counters or feedback.
    pub(crate) fn on_target_pathing_failure<N: ReckoningNumeric, A: WanderAvoidVolumes>(
        &mut self, case: i32, facts: &NavigationEntityFacts,
        world: &mut TargetingWorld<'_, A>, numeric: &mut N,
    ) -> Result<(), String> where N::Error: Debug {
        let Some(current) = self.current_target_iterator() else { return Ok(()); };
        let seek = self.with_target_iterator(current, facts, world, numeric,
            |_, iterator, _, _| iterator.on_pathing_failure(case, facts.position))?;
        if seek { self.seek_current_target(true, facts, world, numeric)?; }
        Ok(())
    }

    fn with_target_iterator<N: ReckoningNumeric, A: WanderAvoidVolumes, R>(
        &mut self, current: TargetIteratorKind, facts: &NavigationEntityFacts,
        world: &mut TargetingWorld<'_, A>, numeric: &mut N,
        execute: impl FnOnce(&mut MeshNavigation, &mut dyn NavigationTargetIterator<N>, &mut N, &MeshMan) -> Result<R, String>,
    ) -> Result<R, String> where N::Error: Debug {
        let nav = self.mesh_navigation.as_mut().ok_or("pedestrian mesh navigation absent")?;
        let state = self.state.as_mut().ok_or("pedestrian AI state absent")?;
        match current {
            TargetIteratorKind::Road => {
                let mut restart = |duration| state.set_timer(0x29, duration).map_err(|e| e.to_string());
                let tuning = world.cvars.try_pedestrian_road_tuning().map_err(|e| format!("road tuning: {e:?}"))?;
                let facing = self.core.as_ref().ok_or("pedestrian core absent")?.transform[2];
                let mut frame = RoadIteratorFrame { iterator: &mut self.target_iterators.road,
                    context: RoadNavigationContext { rain: world.rain, mesh: &world.mesh.store,
                        random: world.random, position: facts.position, facing, entity_id: self.id },
                    tuning, restart_timer: &mut restart };
                execute(nav, &mut frame, numeric, world.mesh)
            }
            TargetIteratorKind::NoRoad => {
                let mut restart = |duration| state.set_timer(0x29, duration).map_err(|e| e.to_string());
                let tuning = world.cvars.try_pedestrian_wander_tuning().map_err(|e| format!("wander tuning: {e:?}"))?;
                let facing = self.core.as_ref().ok_or("pedestrian core absent")?.transform[2];
                let mut frame = NoRoadIteratorFrame { iterator: &mut self.target_iterators.no_road,
                    mesh: &world.mesh.store, avoid: world.avoid, facing, tuning, restart_timer: &mut restart };
                execute(nav, &mut frame, numeric, world.mesh)
            }
            TargetIteratorKind::Fixed | TargetIteratorKind::Random | TargetIteratorKind::VehicleFlee => {
                let iterator = match current { TargetIteratorKind::Fixed => &mut self.target_iterators.fixed,
                    TargetIteratorKind::Random => &mut self.target_iterators.random,
                    _ => &mut self.target_iterators.vehicle_flee };
                let mut frame = PointFrame { iterator, kind: current, random: world.random };
                execute(nav, &mut frame, numeric, world.mesh)
            }
            TargetIteratorKind::Flee => {
                let mut frame=FleeFrame {iterator:&mut self.target_iterators.flee,
                    direction:&mut self.target_iterators.flee_direction,source:world.flee_source.as_ref()};
                execute(nav,&mut frame,numeric,world.mesh)
            }
            TargetIteratorKind::Chase => {
                let present=self.target_iterators.chase_pedestrian.is_some();
                let mut allow=false;
                if world.cvars.try_pedestrian_allow_incomplete_path_in_chase()
                    .map_err(|e| format!("chase incomplete-path cvar: {e:?}"))? && present {
                    if let Some(chasee)=world.chasee_position {
                        let radius=world.cvars.try_pedestrian_chase_nav_fail_distance()
                            .map_err(|e| format!("chase failure distance: {e:?}"))?;
                        allow=compare_distance_xz(facts.position,chasee,radius)>0.;
                    }
                }
                let mut frame=ChaseFrame {cost:&mut self.target_iterators.chase_cost_threshold,
                    alternate:&self.target_iterators.chase_alternate,state,pedestrian_present:present,
                    position:facts.position,chasee:world.chasee_position,cvars:world.cvars,allow_incomplete:allow};
                execute(nav,&mut frame,numeric,world.mesh)
            }
            TargetIteratorKind::Waypoint=>{
                let mut frame=WaypointFrame{props:world.props,current:&mut self.current_waypoint,
                    index:&mut self.target_iterators.waypoint_index,
                    pedestrian:self.target_iterators.waypoint_pedestrian,
                    cost:&mut self.target_iterators.waypoint_cost_threshold};
                execute(nav,&mut frame,numeric,world.mesh)
            }
        }
    }
}
