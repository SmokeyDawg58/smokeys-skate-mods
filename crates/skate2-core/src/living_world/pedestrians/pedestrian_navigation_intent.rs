//! Stateful IntentGen navigation adapter; GetNavTargetDirection82e8b320.
use super::{NavigationModifiers,ORDER};
use crate::{
    living_world_grind_services::entity_core::CommonEntityCore,
    living_world_mesh_store::MeshStore,
    living_world_pedestrian_state::PedestrianStateStorage,
    pedestrian_adjust_owners::SpatialAdjust,
    pedestrian_intent_packet::{IntentStage,queue::{IntentQueue,AiGraphInput,
        PacketNavigation,BaseNavigation,StageRotation,StrafeDirection}},
    pedestrian_navigation::{PedestrianNavigation,NavigationTargetMode,rig::{NavRig,JumpTuning}},
    physical_player_trajectory_owners::selector_grind::{vmax,vmin},
    play_animation::FastString30,
    reckoning_numeric::{ReckoningNumeric,try_normalize_safe,try_normalize_unsafe,
        try_length,normalize_from_squared_and_estimate,try_acos,cross_full4},
    skeleton_root_transforms::SkeletonRootVector,
};
pub(crate) struct NavigationIntentState {
    pub(crate) smoothing: [[f32;4];5],
    pub(crate) smoothing_initialized: bool,
    pub(crate) explicit_direction: Option<[f32;4]>,
    pub(crate) distance_razor: f32,
}
impl NavigationIntentState {
    /// IntentGen82e8e3f0 clears initialized/explicit flags and razor. Pooled
    /// storage clears the samples; first direction call fills all with forward.
    pub(crate) fn new() -> Self {
        Self { smoothing:[[0.;4];5],smoothing_initialized:false,
            explicit_direction:None,distance_razor:0. }
    }
}
pub(crate) struct NavigationIntentTuning {
    pub(crate) jump: JumpTuning,
    pub(crate) jump_override: bool,
    pub(crate) distance_initial: f32,
    pub(crate) distance_scalar: f32,
    pub(crate) usage_velocity_threshold: f32,
    pub(crate) smooth_best_ray: bool,
    pub(crate) check_jump_mesh: bool,
    pub(crate) jump_mesh_y_tolerance: f32,
}
/// All references point at existing retained owners. Flags/max turn are real
/// entity interface reads; max turn uses180 only when active Navigation's
/// GetCurrentTargetIterator returns null, otherwise GetMaxTurnThreshold.
/// The caller chooses the original orientation argument.
pub(crate) struct NavigationIntentAdapter<'a,N:ReckoningNumeric> {
    pub(crate) state:&'a mut NavigationIntentState,
    pub(crate) core:&'a CommonEntityCore,
    pub(crate) orientation:&'a [[f32;4];4],
    pub(crate) navigation:&'a PedestrianNavigation,
    pub(crate) rig:&'a NavRig,
    pub(crate) modifiers:&'a NavigationModifiers,
    pub(crate) spatial:&'a mut SpatialAdjust,
    pub(crate) ai_state:&'a mut PedestrianStateStorage,
    pub(crate) mesh:&'a MeshStore,
    pub(crate) tuning:&'a NavigationIntentTuning,
    pub(crate) nav_rig_override:bool,
    pub(crate) smoothing_enabled:bool,
    pub(crate) maximum_turn:f32,
    pub(crate) numeric:&'a mut N,
}
impl<N:ReckoningNumeric> NavigationIntentAdapter<'_,N> where N::Error:std::fmt::Debug {
    pub(crate) fn navigation_direction(&mut self,queue:&mut IntentQueue,input:&mut AiGraphInput)
        -> Result<([f32;4],f32),String> {
        if !self.state.smoothing_initialized {
            self.state.smoothing=[self.orientation[2];5];
            self.state.smoothing_initialized=true;
        }
        let mut direction=self.navigation.seek_direction;
        let mut max_angle=4.25;
        for kind in ORDER {
            if !self.modifiers.active(kind) { continue; }
            let Some((target,maximum))=self.modifiers.suggested_target(kind) else {continue;};
            // Source self-equality checks only X/Z before full XYZ distance.
            if target[0]!=target[0] || target[2]!=target[2] {continue;}
            let delta:[f32;4]=std::array::from_fn(|i|target[i]-self.orientation[3][i]);
            let squared=self.numeric.dot3(delta,delta).map_err(|e|format!("intent target distance: {e:?}"))?;
            if squared-0.010000001_f32>0. {
                direction=try_normalize_unsafe(delta,self.numeric).map_err(|e|format!("intent target normalize: {e:?}"))?;
                max_angle=maximum;
                break;
            }
        }
        direction[1]=0.; // W survives native stack lane replacement.
        if self.navigation.target_mode==NavigationTargetMode::ExplicitNoSeekNoAstar
            || !self.nav_rig_override || self.spatial.jump.is_jumping
            || queue.packet(queue.jump)?.in_queue { return Ok((direction,max_angle)); }

        let mut ray=0;
        let mut over_turn=false;
        let mut requested_valid=self.rig.best_nav_ray(&mut ray,direction,
            self.state.distance_razor,self.maximum_turn,&mut over_turn,self.numeric)?;
        if !requested_valid && self.rig.distances.do_jump_test {
            let mut height=0.; let mut ledge=0.;
            if self.rig.distances.has_to_jump(&self.tuning.jump,&mut height,&mut ledge) {
                let facing=self.core.transform[2];
                let forward=try_normalize_safe([facing[0],0.,facing[2],0.],[0.;4],self.numeric)
                    .map_err(|e|format!("jump target facing: {e:?}"))?;
                let landing:[f32;4]=std::array::from_fn(|i| {
                    let leading=forward[i].mul_add(self.tuning.jump.distance,self.core.transform[3][i]);
                    leading+[0.,1.,0.,0.][i]*height
                });
                // Query occurs even when kCheckForJumpNavMesh is false.
                let cell=self.mesh.try_get_cell_with_tolerance(SkeletonRootVector::known(landing),
                    self.tuning.jump_mesh_y_tolerance).map_err(|e|format!("jump target mesh: {e:?}"))?;
                if !self.tuning.check_jump_mesh || cell.is_valid() && !(cell.score()>750.) {
                    self.spatial.set_jump_to_position(self.core.transform[3],facing,landing,ledge);
                    let id=queue.jump;
                    let packet=queue.packet_mut(id)?;
                    packet.reset_for_jump();
                    let mut stage=IntentStage::default();
                    stage.add_intent(FastString30::encode("ChaseJump").expect("native name"))?;
                    stage.rotate_to_target=false;
                    packet.add_stage(stage)?;
                    queue.suggest(id,input)?;
                    return Ok((direction,max_angle));
                }
            }
        }
        if !over_turn && !(self.navigation.speed()>0.) {
            let squared=self.numeric.dot3(self.core.velocity,self.core.velocity)
                .map_err(|e|format!("navigation usage velocity: {e:?}"))?;
            if !(squared-self.tuning.usage_velocity_threshold*self.tuning.usage_velocity_threshold>0.) {
                requested_valid=true;
            }
        }
        if over_turn {
            // Pedestrian::OnOverThresholdTurnRequired82e9aeb8, timer14=4.
            self.ai_state.set_timer(14,4.).map_err(|e|e.to_string())?;
        }
        let candidate=if requested_valid {direction} else {self.rig.directions[ray]};
        for i in (1..5).rev() { self.state.smoothing[i]=self.state.smoothing[i-1]; }
        self.state.smoothing[0]=candidate;
        let count=if self.core.current_losim==2 {3} else {5};
        // Raw vor82e8bc18 seeds candidate, then adds samples INCLUDING that
        // candidate again. This intentionally retains native extra weighting.
        let mut sum=candidate;
        for sample in &self.state.smoothing[..count] {
            for lane in 0..4 {sum[lane]+=sample[lane];}
        }
        direction=if self.tuning.smooth_best_ray && self.smoothing_enabled {
            let reciprocal=1./count as f32;
            sum.map(|lane|lane*reciprocal)
        } else {candidate};
        Ok((direction,max_angle))
    }
    fn rotation(&mut self,direction:[f32;4],maximum:f32)->Result<Option<StageRotation>,String> {
        if !direction[..3].iter().any(|v|v.abs()>f32::from_bits(0x3400_0000)) {return Ok(None);}
        let forward=self.orientation[2];
        let target_squared=self.numeric.dot3(direction,direction).map_err(|e|format!("turn target norm: {e:?}"))?;
        let forward_squared=self.numeric.dot3(forward,forward).map_err(|e|format!("turn forward norm: {e:?}"))?;
        let target_estimate=self.numeric.reciprocal_sqrt_estimate(target_squared).map_err(|e|format!("turn target estimate: {e:?}"))?;
        let forward_estimate=self.numeric.reciprocal_sqrt_estimate(forward_squared).map_err(|e|format!("turn forward estimate: {e:?}"))?;
        let target=normalize_from_squared_and_estimate(direction,target_squared,target_estimate);
        let forward=normalize_from_squared_and_estimate(forward,forward_squared,forward_estimate);
        let dot=self.numeric.dot3(target,forward).map_err(|e|format!("turn dot: {e:?}"))?;
        let mut bearing=try_acos(vmin(1.,vmax(-1.,dot)),self.numeric).map_err(|e|format!("turn acos: {e:?}"))?;
        if cross_full4(forward,target)[1]>0. {bearing *= -1.;}
        Ok(Some(StageRotation {direction,max_angle_degrees:maximum,bearing}))
    }
}
impl<N:ReckoningNumeric> PacketNavigation for NavigationIntentAdapter<'_,N> where N::Error:std::fmt::Debug {
    fn target_direction(&mut self,queue:&mut IntentQueue,input:&mut AiGraphInput)->Result<[f32;4],String> {
        Ok(self.navigation_direction(queue,input)?.0)
    }
    fn turn_mask_with_active_modifiers(&mut self)->Result<u32,String> {
        let mut mask=119;
        for kind in ORDER {if self.modifiers.active(kind) {mask &=self.modifiers.turn_mask(kind);}}
        Ok(mask)
    }
    fn rotate_to_target(&mut self,queue:&mut IntentQueue,input:&mut AiGraphInput)->Result<Option<StageRotation>,String> {
        let direction=match self.state.explicit_direction {Some(d)=>d,None=>self.navigation_direction(queue,input)?.0};
        // Raw82e8be64 reloads4.25 after GetNavTargetDirection; its modifier
        // maxRotateAngle output is not used by this method.
        self.rotation(direction,4.25)
    }
}
impl<N:ReckoningNumeric> BaseNavigation for NavigationIntentAdapter<'_,N> where N::Error:std::fmt::Debug {
    fn turn_mask(&mut self)->Result<u32,String> {Ok(119)}
    fn speed_after_clearance_update(&mut self)->Result<f32,String> {
        self.state.distance_razor=if self.tuning.jump_override && self.rig.distances.do_jump_test {self.tuning.jump.distance}
            else {
                let speed=try_length(self.core.velocity,self.numeric).map_err(|e|format!("clearance velocity: {e:?}"))?;
                self.tuning.distance_scalar.mul_add(speed,self.tuning.distance_initial)
            };
        Ok(self.rig.distances.speed(self.navigation,self.state.distance_razor))
    }
    fn minimum_velocity_override(&mut self)->Result<Option<f32>,String> {Ok(self.navigation.velocity_override)}
    fn modifier_count(&mut self)->Result<usize,String> {Ok(6)}
    fn modifier_active(&mut self,index:usize)->Result<bool,String> {Ok(self.modifiers.active(*ORDER.get(index).ok_or("modifier index")?))}
    fn modifier_turn_mask(&mut self,index:usize)->Result<u32,String> {Ok(self.modifiers.turn_mask(*ORDER.get(index).ok_or("modifier index")?))}
    fn modifier_speed(&mut self,index:usize)->Result<Option<f32>,String> {Ok(self.modifiers.suggested_speed(*ORDER.get(index).ok_or("modifier index")?))}
    fn strafing(&mut self)->Result<bool,String> {Ok(self.navigation.use_strafe_target)}
    fn strafe_rotate_to_target(&mut self,_queue:&mut IntentQueue,_input:&mut AiGraphInput)->Result<Option<StageRotation>,String> {
        let direction=if let Some(direction)=self.state.explicit_direction {direction} else {
            let delta:[f32;4]=std::array::from_fn(|i|self.navigation.strafe_target[i]-self.orientation[3][i]);
            try_normalize_safe(delta,[0.;4],self.numeric).map_err(|e|format!("strafe target: {e:?}"))?
        };
        self.rotation(direction,4.25)
    }
    fn strafe_direction(&mut self,queue:&mut IntentQueue,input:&mut AiGraphInput)->Result<StrafeDirection,String> {
        let direction=match self.state.explicit_direction {Some(d)=>d,None=>self.navigation_direction(queue,input)?.0};
        let rotation=self.rotation(direction,0.)?;
        let angle=rotation.as_ref().map_or(0.,|r|r.bearing)*f32::from_bits(0x4265_2ee1);
        let name=if angle>=160. || angle<=-160. {"MoveBackwards"}
            else if !(angle<115.) {"MoveRight135"} else if !(angle> -115.) {"MoveLeft135"}
            else if !(angle<70.) {"MoveRight90"} else if !(angle> -70.) {"MoveLeft90"}
            else if !(angle<25.) {"MoveRight45"} else if !(angle> -25.) {"MoveLeft45"}
            else {"MoveForwards"};
        Ok(StrafeDirection {rotation,intent:FastString30::encode(name).expect("native strafe literal")})
    }
}
