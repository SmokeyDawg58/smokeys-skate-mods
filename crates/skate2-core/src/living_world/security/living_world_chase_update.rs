//! Shared Chasee.UpdateByChasee82de5200 orchestration for Actor and Pedestrian.
use super::{ChaseeState,ChaserAssessment,TargetableId,prediction::predict_intersect_at_max_speed};
use crate::{living_world_pedestrian_entity::PedestrianConstruction,reckoning_numeric::HostReckoningNumeric};

pub(crate) trait ChaseeUpdateServices {
    fn chaser(&self,id:TargetableId)->Result<&PedestrianConstruction,String>;
    fn chaser_mut(&mut self,id:TargetableId)->Result<&mut PedestrianConstruction,String>;
    fn position(&self,id:TargetableId)->Result<[f32;4],String>;
    fn velocity(&self,id:TargetableId)->Result<[f32;4],String>;
    fn facing(&self,id:TargetableId)->Result<[f32;4],String>;
    fn switch_thresholds(&self)->Result<(f32,f32),String>;
    fn switch_time(&self)->Result<f32,String>;
    fn protector_distance(&self)->Result<Option<f32>,String>;
    fn group_end_reason(&self,stored:i32)->Result<i32,String>;
    fn ongoing_rate(&self)->Result<f32,String>;
    fn send_ongoing(&self,chaser:TargetableId,chasee:TargetableId,position:[f32;4])->Result<(),String>;
}
fn assessment(chaser:&PedestrianConstruction,id:TargetableId,time:f32)->Result<ChaserAssessment,String>{
    let state=chaser.state.as_ref().ok_or("chaser assessment AI state absent")?;
    Ok(ChaserAssessment{id,intercept_time:time,primary_timeout:state.timer(29),
        wiped_out:state.flags[1]&4!=0,colliding:chaser.collision.as_ref().ok_or("chaser collision owner absent")?.colliding,
        resting:state.flags[1]&2!=0,failed_takedown:state.flags[1]&0x80!=0})
}
pub(crate) fn update(chasee:&mut ChaseeState,id:TargetableId,dt:f32,services:&mut impl ChaseeUpdateServices)->Result<(),String>{
    if chasee.chasers.is_empty(){return Ok(());}
    chasee.update_chasee_timers(dt);
    let ids:Vec<_>=chasee.chasers.iter().map(|c|c.id).collect();
    for &id in &ids {services.chaser(id)?;}
    let position=services.position(id)?;
    let velocity=services.velocity(id)?;
    services.position(ids[0])?;
    services.velocity(ids[0])?;
    for (index,&chaser) in ids.iter().enumerate(){
        let position2=services.position(chaser)?;
        let speed=services.chaser(chaser)?.max_chase_speed()?;
        let result=predict_intersect_at_max_speed(xyz(position),xyz(velocity),xyz(position2),speed,&mut HostReckoningNumeric)
            .map_err(|e|format!("chaser intercept: {e:?}"))?;
        chasee.chasers[index].intercept_time=result.points.first().map_or(f32::MAX,|p|p.time);
    }
    if ids.len()>=2 {
        let assessments=chasee.chasers.iter().map(|c|assessment(services.chaser(c.id)?,c.id,c.intercept_time))
            .collect::<Result<Vec<_>,_>>()?;
        chasee.rank_secondaries(&assessments).map_err(|e|format!("chaser ranking: {e:?}"))?;
        let (timeout,intercept)=services.switch_thresholds()?;
        if let Some(old)=chasee.primary_switch_candidate(&assessments,timeout,intercept).map_err(|e|format!("chaser switch: {e:?}"))?{
            let requested=services.switch_time()?;
            let state=services.chaser_mut(old)?.state.as_mut().ok_or("primary AI state absent")?;
            let previous=state.timer(29);
            state.set_timer(29,if requested-previous>=0.{requested}else{previous}).map_err(|e|format!("primary timeout: {e:?}"))?;
            chasee.finish_primary_switch(old).map_err(|e|format!("primary switch: {e:?}"))?;
        }
    }
    chasee.evaluate_flank_orders(services.protector_distance()?);
    let ids:Vec<_>=chasee.chasers.iter().map(|c|c.id).collect();
    for &id in &ids {services.chaser(id)?;}
    let position=services.position(id)?;
    let positions=ids.iter().map(|&c|Ok((c,xyz(services.position(c)?)))).collect::<Result<Vec<_>,String>>()?;
    let velocity=services.velocity(id)?;
    let facing=services.facing(id)?;
    chasee.generate_ideal_positions(xyz(position),xyz(velocity),xyz(facing),&positions,&mut HostReckoningNumeric)
        .map_err(|e|format!("chaser formation: {e:?}"))?;
    for chaser in &mut chasee.chasers{chaser.recommended_position=chaser.ideal_position;}
    if services.group_end_reason(chasee.group_end_reason)?!=-1{return Ok(());}
    let rate=services.ongoing_rate()?;
    if !(chasee.nearby_event_timer<=0.){return Ok(());}
    for chaser in ids {
        if !chaser.is_valid()||chaser.kind!=1 {continue;}
        if services.chaser(chaser).is_err(){continue;}
        services.send_ongoing(chaser,id,services.position(id)?)?;
    }
    chasee.nearby_event_timer=rate;
    Ok(())
}
fn xyz(v:[f32;4])->[f32;3]{[v[0],v[1],v[2]]}
