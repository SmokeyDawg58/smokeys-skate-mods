//! RoadNetworkTargetIterator: original four-node lookahead, shared RAIN
//! topology, intersection reservations, Manager RNG and traffic-light clock.
use crate::{
    living_world_mesh_store::MeshStore,
    living_world_rain::{
        RainMan,
        ped_nodes::{NodeType, PedRainNode},
        traffic::Colour,
    },
    native_skater_random::{NativeRandomGate, NativeSkaterRandom},
    reckoning_numeric::{ReckoningNumeric, try_length, try_signed_angle},
};
use std::fmt::Debug;
fn error(e: impl Debug) -> String {
    format!("road navigation: {e:?}")
}
pub(crate) struct RoadNavigationContext<'a> {
    pub(crate) rain: &'a mut RainMan,
    pub(crate) mesh: &'a MeshStore,
    pub(crate) random: &'a mut NativeSkaterRandom,
    pub(crate) position: [f32; 4],
    pub(crate) facing: [f32; 4],
    pub(crate) entity_id: u32,
}
#[derive(Clone, Copy)]
pub(crate) struct RoadNavigationTuning {
    pub(crate) valid_distance: f32,
    pub(crate) valid_angle: f32,
    pub(crate) timer_speed: f32,
    pub(crate) crosswalk_time: f32,
    pub(crate) allow_incomplete: bool,
}
pub(crate) struct RoadNetworkTargetIterator {
    pub(crate) nodes: [PedRainNode; 4],
    pub(crate) target: [f32; 4],
    pub(crate) flags: u8,
    pub(crate) start_slot: Option<u32>,
    pub(crate) end_slot: Option<u32>,
    pub(crate) intersection_id: u64,
    pub(crate) current_road_id: u64,
    pub(crate) cost_threshold: f32,
}
impl RoadNetworkTargetIterator {
    pub(crate) fn new() -> Self {
        Self {
            nodes: [PedRainNode::default(); 4],
            target: [0.; 4],
            flags: 0x10,
            start_slot: None,
            end_slot: None,
            intersection_id: 0,
            current_road_id: 0,
            cost_threshold: 0.,
        }
    }
    pub(crate) fn has_target(&self) -> bool {
        self.flags & 0x20 != 0
    }
    fn disable(&mut self) {
        self.flags = self.flags & 0xcf | 0x10;
    }
    pub(crate) fn on_pathing_failure(&mut self, case: i32) -> bool {
        if case == 2 || case == 4 {
            self.disable();
        }
        true
    }
    fn choose(context: &mut RoadNavigationContext<'_>, choices: &[u8]) -> Result<u8, String> {
        let random = context
            .random
            .try_living_world_range(&NativeRandomGate::production_shared(), 0., 1.)
            .map_err(error)?;
        Ok(choices[((random * choices.len() as f32) as u32 % choices.len() as u32) as usize])
    }
    fn selection(
        node: PedRainNode,
        selection: u8,
        context: &mut RoadNavigationContext<'_>,
    ) -> Result<PedRainNode, String> {
        match selection {
            0..=3 => node
                .adjacent_crosswalk(
                    context.rain,
                    context.mesh,
                    selection >= 2,
                    (selection & 1) as usize,
                )
                .map_err(error),
            4..=7 => node
                .adjacent_road(context.rain, selection >= 6, (selection & 1) as usize)
                .map_err(error),
            _ => Ok(PedRainNode::default()),
        }
    }
    fn valid_selection(
        node: PedRainNode,
        selection: u8,
        context: &mut RoadNavigationContext<'_>,
    ) -> Result<bool, String> {
        Self::selection(node, selection, context)?
            .is_valid(context.rain, context.mesh)
            .map_err(error)
    }
    /// FindForward82e7e878/Backward82e7ea80 preserve candidate order and one draw.
    fn find_next(
        &mut self,
        index: usize,
        context: &mut RoadNavigationContext<'_>,
    ) -> Result<bool, String> {
        if index + 1 >= 4
            || !self.nodes[index]
                .is_valid(context.rain, context.mesh)
                .map_err(error)?
        {
            return Ok(false);
        }
        let node = self.nodes[index];
        let backward = self.flags & 0x40 != 0;
        let candidates = if backward {
            if node.kind == NodeType::Crosswalk || node.reversed {
                [0, 4, 5]
            } else {
                [3, 6, 7]
            }
        } else if node.kind == NodeType::Crosswalk || !node.reversed {
            [2, 6, 7]
        } else {
            [1, 4, 5]
        };
        let mut choices = Vec::new();
        for choice in candidates {
            if Self::valid_selection(node, choice, context)? {
                choices.push(choice);
            }
        }
        if choices.is_empty() {
            return Ok(false);
        }
        let choice = Self::choose(context, &choices)?;
        self.nodes[index + 1] = Self::selection(node, choice, context)?;
        self.nodes[index + 1].reversed = if backward {
            choice != 5 && choice != 6
        } else {
            choice == 7 || choice == 4
        };
        Ok(true)
    }
    fn init_current(&mut self, context: &mut RoadNavigationContext<'_>) -> Result<(), String> {
        let node = self.nodes[0];
        if !node.is_valid(context.rain, context.mesh).map_err(error)? {
            return Ok(());
        }
        if node.kind == NodeType::Crosswalk {
            self.start_slot = self.end_slot.take();
            if self.start_slot.is_none() {
                if let Some(target) = node
                    .request_nav_target(context.rain, context.mesh, node.reversed)
                    .map_err(error)?
                {
                    self.start_slot = Some(target.id);
                    target.entity_id = Some(context.entity_id & 0xffffff);
                }
            }
            if let Some(target) = node
                .request_nav_target(context.rain, context.mesh, !node.reversed)
                .map_err(error)?
            {
                self.end_slot = Some(target.id);
                target.entity_id = Some(context.entity_id & 0xffffff);
                self.current_road_id = 0;
            }
            self.intersection_id = node.this_crosswalk.id;
        } else {
            self.intersection_id = 0;
            self.current_road_id = node.this_road.id;
        }
        Ok(())
    }
    pub(crate) fn deinit_current(
        &mut self,
        context: &mut RoadNavigationContext<'_>,
    ) -> Result<(), String> {
        let node = self.nodes[0];
        if !node.is_valid(context.rain, context.mesh).map_err(error)? {
            return Ok(());
        }
        if node.kind == NodeType::Crosswalk {
            if let Some(id) = self.start_slot {
                node.free_nav_target(context.rain, context.mesh, node.reversed, id)
                    .map_err(error)?;
            }
            if let Some(id) = self.end_slot {
                node.free_nav_target(context.rain, context.mesh, !node.reversed, id)
                    .map_err(error)?;
            }
        } else {
            self.start_slot = None;
            self.end_slot = None;
        }
        Ok(())
    }
    pub(crate) fn target_position(
        &self,
        index: usize,
        start: bool,
        context: &mut RoadNavigationContext<'_>,
    ) -> Result<[f32; 4], String> {
        let node = *self.nodes.get(index).ok_or("road target index")?;
        if !node.is_valid(context.rain, context.mesh).map_err(error)? {
            return Ok(context.position);
        }
        let end = if start { node.reversed } else { !node.reversed };
        let slot = if start {
            self.start_slot
        } else {
            self.end_slot
        };
        if index == 0 && node.kind == NodeType::Crosswalk {
            if let Some(id) = slot {
                return Ok(node
                    .nav_target(context.rain, context.mesh, end, id)
                    .map_err(error)?
                    .and_then(|target| target.position)
                    .unwrap_or(context.position));
            }
        }
        node.endpoint(context.rain, context.mesh, end)
            .map_err(error)
    }
    fn check_and_set<N: ReckoningNumeric>(
        &mut self,
        check_angle: bool,
        context: &mut RoadNavigationContext<'_>,
        tuning: RoadNavigationTuning,
        numeric: &mut N,
    ) -> Result<Option<f32>, String>
    where
        N::Error: Debug,
    {
        self.target = self.target_position(0, self.flags & 0x80 != 0, context)?;
        let delta: [f32; 4] = std::array::from_fn(|i| self.target[i] - context.position[i]);
        if check_angle {
            let mut angle = try_signed_angle(delta, context.facing, [0., 1., 0., 0.], numeric)
                .map_err(error)?;
            if angle > f32::from_bits(0x40490fdb) {
                angle -= f32::from_bits(0x40c90fdb);
            }
            // Raw82e7f210 is fmul (double), not fmuls, before the compare.
            if f64::from(angle.abs())
                > f64::from(tuning.valid_angle) * f64::from(f32::from_bits(0x3c0efa35))
            {
                self.disable();
                return Ok(None);
            }
        }
        if self.has_target() {
            Ok(Some(
                try_length(delta, numeric).map_err(error)? * (1. / tuning.timer_speed),
            ))
        } else {
            Ok(None)
        }
    }
    fn start<N: ReckoningNumeric>(
        &mut self,
        context: &mut RoadNavigationContext<'_>,
        tuning: RoadNavigationTuning,
        numeric: &mut N,
    ) -> Result<(), String>
    where
        N::Error: Debug,
    {
        let backward = context
            .random
            .try_living_world_range(&NativeRandomGate::production_shared(), 0., 1.)
            .map_err(error)?
            > 0.5;
        self.flags = (self.flags & 0x9f) | if backward { 0x40 } else { 0 } | 0x20;
        if self.flags & 0x10 != 0 {
            self.disable();
            return Ok(());
        }
        self.nodes = [PedRainNode::default(); 4];
        let Some(id) = context
            .rain
            .nearest_intersection([context.position[0], context.position[2]])
            .map_err(error)?
        else {
            self.disable();
            return Ok(());
        };
        let Some(intersection) = context.rain.intersections.get(&id) else {
            self.disable();
            return Ok(());
        };
        let nodes = intersection.ped;
        let mut best = 0;
        let mut best_distance = 0.;
        for (i, node) in nodes.iter().enumerate() {
            let point = node
                .endpoint(context.rain, context.mesh, backward)
                .map_err(error)?;
            let delta: [f32; 4] = std::array::from_fn(|i| context.position[i] - point[i]);
            let distance = try_length(delta, numeric).map_err(error)?;
            if i == 0 || best_distance > distance {
                best = i;
                best_distance = distance;
            }
        }
        self.nodes[0] = nodes[best];
        self.nodes[0].reversed = backward;
        let start = self.nodes[0]
            .endpoint(context.rain, context.mesh, false)
            .map_err(error)?;
        let delta = std::array::from_fn(|i| start[i] - context.position[i]);
        if numeric.dot3(delta, delta).map_err(error)?
            - tuning.valid_distance * tuning.valid_distance
            <= 0.
        {
            for i in 0..3 {
                if !self.find_next(i, context)? {
                    break;
                }
            }
            self.flags |= 0x80;
            self.init_current(context)?;
        } else {
            self.disable();
        }
        Ok(())
    }
    fn recheck(&mut self, context: &mut RoadNavigationContext<'_>) -> Result<(), String> {
        let node = self.nodes[0];
        if !node.is_valid(context.rain, context.mesh).map_err(error)?
            || node.kind != NodeType::Crosswalk
            || self.start_slot.is_some() && self.end_slot.is_some()
        {
            return Ok(());
        }
        self.deinit_current(context)?;
        self.start_slot = None;
        self.end_slot = None;
        let mut choices = Vec::new();
        let extra = if node.reversed { 5 } else { 7 };
        if node.outgoing_road[0].id as u32 == 0
            && node.incoming_road[0].id as u32 == 0
            && Self::valid_selection(node, extra, context)?
        {
            choices.push(extra);
        }
        let first = if node.reversed { 6 } else { 4 };
        let links = if node.reversed {
            node.outgoing_road
        } else {
            node.incoming_road
        };
        for i in 0..2 {
            if self.current_road_id as u32 != links[i].id as u32
                && Self::valid_selection(node, first + i as u8, context)?
            {
                choices.push(first + i as u8);
            }
        }
        if choices.is_empty() && self.current_road_id as u32 != 0 {
            for i in 0..2 {
                if self.current_road_id as u32 == links[i].id as u32
                    && Self::valid_selection(node, first + i as u8, context)?
                {
                    choices.push(first + i as u8);
                    break;
                }
            }
        }
        if choices.is_empty() {
            self.disable();
            return Ok(());
        }
        let choice = Self::choose(context, &choices)?;
        self.nodes[0] = Self::selection(node, choice, context)?;
        let backward = self.flags & 0x40 != 0;
        let flip = if backward {
            choice == 5 || choice == 6
        } else {
            choice == 7 || choice == 4
        };
        self.nodes[0].reversed = backward ^ flip;
        for i in 0..3 {
            if !self.find_next(i, context)? {
                break;
            }
        }
        self.flags |= 0x80;
        self.init_current(context)
    }
    pub(crate) fn check_for_target<N: ReckoningNumeric>(
        &mut self,
        context: &mut RoadNavigationContext<'_>,
        tuning: RoadNavigationTuning,
        numeric: &mut N,
    ) -> Result<Option<f32>, String>
    where
        N::Error: Debug,
    {
        if !self.has_target()
            || !self.nodes[0]
                .is_valid(context.rain, context.mesh)
                .map_err(error)?
        {
            self.flags &= !0x10;
            self.start(context, tuning, numeric)?;
            self.recheck(context)?;
            if self.has_target()
                || self.nodes[0]
                    .is_valid(context.rain, context.mesh)
                    .map_err(error)?
            {
                return self.check_and_set(true, context, tuning, numeric);
            }
        }
        Ok(None)
    }
    pub(crate) fn increment<N: ReckoningNumeric>(
        &mut self,
        context: &mut RoadNavigationContext<'_>,
        tuning: RoadNavigationTuning,
        numeric: &mut N,
    ) -> Result<Option<f32>, String>
    where
        N::Error: Debug,
    {
        if self.has_target()
            && self.nodes[0]
                .is_valid(context.rain, context.mesh)
                .map_err(error)?
        {
            if self.flags & 0x80 == 0 {
                self.deinit_current(context)?;
                for i in 0..3 {
                    self.nodes[i] = self.nodes[i + 1];
                }
                if self.nodes[2]
                    .is_valid(context.rain, context.mesh)
                    .map_err(error)?
                {
                    self.find_next(2, context)?;
                }
                self.init_current(context)?;
                self.recheck(context)?;
            }
            if self.has_target()
                || self.nodes[0]
                    .is_valid(context.rain, context.mesh)
                    .map_err(error)?
            {
                self.flags ^= 0x80;
                return self.check_and_set(false, context, tuning, numeric);
            }
        } else {
            self.start(context, tuning, numeric)?;
            self.recheck(context)?;
            if self.has_target()
                || self.nodes[0]
                    .is_valid(context.rain, context.mesh)
                    .map_err(error)?
            {
                return self.check_and_set(false, context, tuning, numeric);
            }
        }
        Ok(None)
    }
    pub(crate) fn is_crossing(
        &self,
        context: &mut RoadNavigationContext<'_>,
    ) -> Result<bool, String> {
        Ok(self.nodes[0]
            .is_valid(context.rain, context.mesh)
            .map_err(error)?
            && self.nodes[0].kind == NodeType::Crosswalk
            && self.start_slot.is_some()
            && self.end_slot.is_some()
            && self.flags & 0x80 == 0)
    }
    pub(crate) fn can_proceed(
        &self,
        context: &mut RoadNavigationContext<'_>,
        tuning: RoadNavigationTuning,
    ) -> Result<bool, String> {
        let node = self.nodes[0];
        if !node.is_valid(context.rain, context.mesh).map_err(error)? {
            return Ok(false);
        }
        if node.kind != NodeType::Crosswalk {
            return Ok(true);
        }
        if self.start_slot.is_none() || self.end_slot.is_none() {
            return Ok(false);
        }
        if node.incoming_road[0].id as u32 == 0 && node.outgoing_road[0].id as u32 == 0 {
            return Ok(true);
        }
        let Some(intersection) = context
            .rain
            .intersection(node.this_crosswalk.id, context.mesh)
            .map_err(error)?
        else {
            return Ok(true);
        };
        if intersection.record.word(580).map_err(error)? == 0 || self.flags & 0x80 != 0 {
            return Ok(true);
        }
        let light = context
            .rain
            .traffic_lights
            .get(node.this_crosswalk.pin1 as usize)
            .ok_or("crosswalk light index")?
            .current(true)
            .map_err(error)?;
        Ok(light.colour == Colour::Green && light.remaining > tuning.crosswalk_time)
    }
}

pub(crate) struct RoadIteratorFrame<'a, 'world> {
    pub(crate) iterator: &'a mut RoadNetworkTargetIterator,
    pub(crate) context: RoadNavigationContext<'world>,
    pub(crate) tuning: RoadNavigationTuning,
    pub(crate) restart_timer: &'a mut dyn FnMut(f32) -> Result<(), String>,
}
impl<N: ReckoningNumeric> super::mesh::NavigationTargetIterator<N> for RoadIteratorFrame<'_, '_>
where
    N::Error: Debug,
{
    fn has_target(&self) -> bool {
        self.iterator.has_target()
    }
    fn target(&self) -> Result<[f32; 4], String> {
        Ok(self.iterator.target)
    }
    fn increment(&mut self, position: [f32; 4], _dt: f32, numeric: &mut N) -> Result<(), String> {
        self.context.position = position;
        if let Some(duration) = self
            .iterator
            .increment(&mut self.context, self.tuning, numeric)?
        {
            (self.restart_timer)(duration)?;
        }
        Ok(())
    }
    fn reset_cost_threshold(&mut self) {
        self.iterator.cost_threshold = 0.;
    }
    fn cost_threshold(&mut self) -> Result<f32, String> {
        Ok(if self.iterator.is_crossing(&mut self.context)? {
            0.26
        } else {
            self.iterator.cost_threshold
        })
    }
    fn allow_incomplete_path(&self) -> bool {
        self.tuning.allow_incomplete
    }
    fn allow_off_mesh(&self) -> bool {
        false
    }
    fn alternate_target(&self) -> Option<[f32; 4]> {
        None
    }
    fn on_pathing_failure(&mut self, case: i32, _position: [f32; 4]) -> Result<bool, String> {
        Ok(self.iterator.on_pathing_failure(case))
    }
}
