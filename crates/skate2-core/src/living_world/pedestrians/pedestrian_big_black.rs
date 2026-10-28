//! BigBlack82e98960 extends the retained Pedestrian; it owns no second entity.
use crate::{attribute_runtime::AttributeRuntimeInstance,living_world_chase::TargetableId,
    living_world_pedestrian_memory::{PedestrianMemory,PedestrianStore}};
use super::PedestrianConstruction;
pub(crate) const PROTECT_PARAMETERS:u64=0xaa97_a406_833b_a601;
pub(crate) const BLOCK_RADIUS:u64=0x9ba4_a240_febf_1ed9;
pub(crate) const IMMINENT_RADIUS:u64=0xc3c0_4de1_b983_58fb;
pub(crate) const MIN_RADIUS:u64=0xd1d1_3c93_f0f8_6a20;
pub(crate) const PROTECTION_RADIUS:u64=0x4e20_c322_d085_57a9;
pub(crate) const MAINTAIN_RADIUS:u64=0x4f3e_bdb0_5f85_4818;
pub(crate) const FIXED_REGION:u64=0x0417_be96_b255_d0f8;
#[derive(Clone,Copy)]
pub(crate) struct Threat {
    pub(crate) id:TargetableId,
    pub(crate) intercept:[f32;4],
    pub(crate) block:[f32;4],
    pub(crate) threat_intercept:[f32;4],
    pub(crate) distance:f32,
    pub(crate) threat_travel_time:f32,
    pub(crate) threat_intercept_time:f32,
    pub(crate) threat_block_time:f32,
    pub(crate) block_time:f32,
    pub(crate) intercept_time:f32,
    pub(crate) intercept_distance:f32,
    pub(crate) alive:bool,
    pub(crate) threat_can_intercept:bool,
    pub(crate) can_intercept:bool,
    pub(crate) can_block:bool,
    pub(crate) action:u32,
}
impl Threat {
    pub(crate) fn new(id:TargetableId)->Self {Self {id,intercept:[0.;4],block:[0.;4],
        threat_intercept:[0.;4],distance:0.,threat_travel_time:0.,threat_intercept_time:0.,
        threat_block_time:0.,block_time:0.,intercept_time:0.,intercept_distance:0.,
        alive:false,threat_can_intercept:false,can_intercept:false,can_block:false,action:0}}
    /// ShouldSwapThreat82e7b8c0: the significance argument is unused in this build.
    pub(crate) fn should_swap(&self,right:&Self,imminent:f32)->bool {
        if self.alive!=right.alive{return right.alive;}
        if (self.distance<imminent)!=(right.distance<imminent){return right.distance<imminent;}
        if self.threat_can_intercept!=right.threat_can_intercept{return right.threat_can_intercept;}
        if self.threat_can_intercept{return false;}
        !(self.distance<=right.distance)
    }
    /// EvaluateThreatAction82e7b778; every branch retains the native strict comparison.
    pub(crate) fn choose_action(&mut self,imminent:f32,block:f32){
        self.action=if !self.alive {if self.can_block{1}else{0}}
            else if self.distance<imminent {3}
            else if !self.threat_can_intercept {if self.distance<block{1}else{0}}
            else if !self.can_intercept || self.intercept_distance<imminent
                || self.threat_intercept_time<self.intercept_time {3}
            else if self.can_block && self.threat_block_time>=self.block_time {1}else{2};
    }
}
pub(crate) struct BigBlackData {
    pub(crate) assigned:TargetableId,
    pub(crate) wants:TargetableId,
    pub(crate) protectee:TargetableId,
    pub(crate) parameters:AttributeRuntimeInstance,
    pub(crate) threats:Vec<Threat>,
    pub(crate) zone_centre:[f32;4],
    pub(crate) zone_radius:f32,
    pub(crate) maintain_radius:f32,
}
impl BigBlackData {
    pub(crate) fn scalar(&self,key:u64)->Result<f32,String>{
        if self.parameters.is_null(){return Err("native BigBlack protection collection assertion".into());}
        Ok(f32::from_bits(self.parameters.try_scalar_word(key,0).map_err(|e|e.to_string())?.unwrap_or(0)))
    }
    pub(crate) fn fixed_region(&self)->Result<bool,String>{
        if self.parameters.is_null(){return Err("native BigBlack protection collection assertion".into());}
        Ok(self.parameters.try_get_data(FIXED_REGION,0).map_err(|e|e.to_string())?
            .is_some_and(|d|d.bytes().first().is_some_and(|v|*v!=0)))
    }
    /// IProtector.SetProtecteeId82e704e0.
    pub(crate) fn set_protectee(&mut self,id:TargetableId){
        if !id.is_valid() && self.wants==self.protectee {self.wants=TargetableId::INVALID;}
        self.protectee=id;
    }
}
impl PedestrianConstruction {
    pub(crate) fn initialize_big_black(&mut self,memory:&mut PedestrianMemory)->Result<(),String>{
        self.reserve(memory,PedestrianStore::BigBlack)?;
        let parameters=self.resources.as_ref().ok_or("BigBlack entity resources absent")?.entity
            .try_ref_spec_instance(PROTECT_PARAMETERS).map_err(|e|e.to_string())?
            .ok_or("BigBlack protection RefSpec absent")?;
        self.big_black=Some(BigBlackData {assigned:TargetableId::INVALID,wants:TargetableId::INVALID,
            protectee:TargetableId::INVALID,parameters,threats:Vec::with_capacity(20),
            zone_centre:[0.;4],zone_radius:0.,maintain_radius:0.});
        // Reset82e86300 extends Pedestrian.Reset; the base reset was already performed.
        self.state.as_mut().ok_or("BigBlack AI state absent")?.flags[1]|=0x20;
        Ok(())
    }
}
