//! DetermineTakeDownType82e7a6c0 and ChaseeTargetAvailable82e79dd0.
use super::FrameServices;
use crate::{living_world_chase::TargetableId,
    living_world_pedestrian_entity::{PedestrianConstruction as Entity,takedown_descriptor::{TakeDownDescriptor,TAKE_DOWN_ARRAY}},
    reckoning_numeric::{HostReckoningNumeric,ReckoningNumeric,try_normalize_safe,try_length,try_signed_angle}};
impl FrameServices<'_>{
    pub(crate) fn determine_take_down_type(&mut self,entity:&mut Entity,target:TargetableId)->Result<(),String>{
        let position=self.target_position(entity,target)?.ok_or("DetermineTakeDownType native target assertion")?;
        let own=entity.core.as_ref().ok_or("takedown core absent")?.transform[3];
        let difference=std::array::from_fn::<_,4,_>(|i|own[i]-position[i]);
        let mut numeric=HostReckoningNumeric;
        let error=|e|format!("takedown numeric: {e:?}");
        let direction=try_normalize_safe(difference,[0.;4],&mut numeric).map_err(error)?;
        let velocity=self.target_velocity(entity,target)?.ok_or("takedown target velocity absent")?;
        let relative=numeric.dot3(direction,velocity).map_err(error)?;
        let approach=if relative<=0. {2}else{1};
        entity.state.as_mut().ok_or("takedown state absent")?.take_down_type=None;
        let data=entity.resources.as_ref().ok_or("takedown resources absent")?.take_down.clone();
        let count=data.try_length(TAKE_DOWN_ARRAY).map_err(|e|e.to_string())?;
        for index in 0..count {
            let descriptor=entity.take_down_descriptor(index)?;
            if descriptor.approach_direction!=0 && !(descriptor.approach_direction==approach && descriptor.approach_speed<relative.abs()) {continue;}
            let width=f32::from_bits(data.try_scalar_word(0x8b45_ca81_23ad_04d4,0).map_err(|e|e.to_string())?.unwrap_or(0));
            let lag=f32::from_bits(data.try_scalar_word(0x98da_4760_b3c6_2ed7,0).map_err(|e|e.to_string())?.unwrap_or(0));
            if let Some((point,mirror))=self.chasee_target_available(entity,target,&descriptor,lag,width)? {
                let state=entity.state.as_mut().unwrap();
                state.take_down_target=[point[0],point[1],point[2]];
                state.take_down_type=Some(index);
                state.flags[0]=(state.flags[0]&0x7f)|(u8::from(mirror)<<7);
                break;
            }
        }
        Ok(())
    }
    /// Caller gets a target only on successful radial AND angular tests. The
    /// native failure output is scratch storage; Determine ignores it.
    pub(crate) fn chasee_target_available(&self,entity:&Entity,target:TargetableId,
        attack:&TakeDownDescriptor,lag:f32,width:f32)->Result<Option<([f32;4],bool)>,String>{
        let velocity=self.target_velocity(entity,target)?.ok_or("takedown availability target absent")?;
        let position=self.target_position(entity,target)?.ok_or("takedown availability position absent")?;
        let time=attack.time+lag;
        let mut predicted=std::array::from_fn::<_,4,_>(|i|position[i]+velocity[i]*time);
        let core=entity.core.as_ref().ok_or("takedown availability core absent")?;
        let own=core.transform[3];
        let tolerance=self.cvars.try_takedown_y_tolerance().map_err(|e|format!("takedown Y tuning: {e:?}"))?;
        if (predicted[1]-own[1]).abs()>tolerance {return Ok(None);}
        predicted[1]=own[1];
        let direction=std::array::from_fn::<_,4,_>(|i|predicted[i]-own[i]);
        let delta=std::array::from_fn::<_,4,_>(|i|direction[i]-core.transform[2][i]*width);
        let mut numeric=HostReckoningNumeric;
        let error=|e|format!("takedown availability numeric: {e:?}");
        let distance=try_length(delta,&mut numeric).map_err(error)?;
        //Both source speed Length evaluations are retained, before either comparison.
        let min=try_length(core.velocity,&mut numeric).map_err(error)?.mul_add(lag,attack.min_distance+width);
        let max=try_length(core.velocity,&mut numeric).map_err(error)?.mul_add(lag,attack.max_distance+width);
        if !(max>=distance && distance>min) {return Ok(None);}
        let angle=try_signed_angle(core.transform[2],direction,core.transform[1],&mut numeric).map_err(error)?;
        let angle=if angle>f32::from_bits(0x4049_0fdb){angle-f32::from_bits(0x40c9_0fdb)}else{angle};
        let right=attack.right_angle*0.017453292;
        let left=attack.left_angle*0.017453292;
        if angle>right && left>angle {return Ok(Some((predicted,false)));}
        if attack.can_be_mirrored && angle> -left && -right>angle {return Ok(Some((predicted,true)));}
        Ok(None)
    }
}
