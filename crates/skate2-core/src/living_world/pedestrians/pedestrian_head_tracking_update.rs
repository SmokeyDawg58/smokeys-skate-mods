//! HeadTracking.Update82e85290 and native orientation helper82e5acc0.
use super::{HeadTracking,HeadTrackingMode};
use crate::{living_world_pedestrian_entity::PedestrianConstruction,
    pedestrian_graph::backend::frame::FrameServices,living_world_chase::TargetableId,
    actor_construction::ActorConstruction,
    reckoning_numeric::{HostReckoningNumeric,ReckoningNumeric,try_length,try_acos},
    simulation_drive_jacobian::refine_drive_reciprocal};

impl HeadTracking{
    /// IsHeadTracking82e74448 reads the live mode and retained graph input.
    pub(crate) fn is_head_tracking(&self,locomotion_turn_complete:bool)->Result<bool,String>{
        if self.current().mode==HeadTrackingMode::None{return Ok(false);}
        let enabled=self.attributes.try_cached_layout_field(0,1).map_err(|e|e.to_string())?
            .ok_or("head tracking layout absent")?.bytes()[0]!=0;
        Ok(enabled&&(self.stack_level>=0||locomotion_turn_complete))
    }
}

/// Native normalizes the 3D displacement before flattening it, then normalizes
/// again. Preserve both length/reciprocal refinements and the 0.0001 gates. The sign is
/// `fsel` 0x82e5af10 on `(forward x offset).y`: -1 when it is not negative, else 1.
pub(crate) fn bearing(orientation:[[f32;4];4],target:[f32;4])->Result<f32,String>{
    let numeric=&mut HostReckoningNumeric;
    let mut forward=orientation[2];forward[1]=0.;
    let normalize=|vector:[f32;4],numeric:&mut HostReckoningNumeric|->Result<Option<[f32;4]>,String>{
        let length=try_length(vector,numeric).map_err(|e|format!("head bearing length: {e:?}"))?;
        if !(length>0.0001){return Ok(None);}
        let seed=numeric.reciprocal_estimate(length).map_err(|e|format!("head bearing reciprocal: {e:?}"))?;
        let inverse=refine_drive_reciprocal(length,seed);
        Ok(Some(vector.map(|v|v*inverse)))
    };
    let Some(forward)=normalize(forward,numeric)? else{return Ok(0.)};
    let delta=std::array::from_fn(|i|target[i]-orientation[3][i]);
    let Some(mut delta)=normalize(delta,numeric)? else{return Ok(0.)};
    delta[1]=0.;
    let Some(delta)=normalize(delta,numeric)? else{return Ok(0.)};
    let dot=numeric.dot3(delta,forward).map_err(|e|format!("head bearing dot: {e:?}"))?;
    let angle=try_acos(dot.max(-1.).min(1.),numeric).map_err(|e|format!("head bearing acos: {e:?}"))?;
    let cross_y=forward[2]*delta[0]-forward[0]*delta[2];
    Ok((if cross_y>=0.{-1.}else{1.})*angle*57.29578_f32)
}

impl FrameServices<'_>{
    /// ITargetable default visibility825efa20 dispatches COM, including all
    /// four concrete types. DMO82d362a0 and Vehicle82d1d078 use position.
    pub(crate) fn target_centre_of_mass(&self,entity:&PedestrianConstruction,target:TargetableId)
        ->Result<Option<[f32;4]>,String>{
        if !self.target_exists(entity,target)?{return Ok(None);}
        let mut value=self.target_position(entity,target)?.ok_or("head target resolved position absent")?;
        // Actor825f1420 and Pedestrian82e7a970 both dispatch position and
        // add global-Y .75. Mode7 below separately reads the actual PhysOut COM.
        if target.kind==0||target.kind==1{value[1]+=0.75;}
        Ok(Some(value))
    }

    pub(crate) fn update_head_tracking(&mut self,entity:&mut PedestrianConstruction,dt:f32,
        local:&ActorConstruction)->Result<(),String>{
        if !entity.renderable{return Ok(());}
        let orientation=entity.core.as_ref().ok_or("head orientation absent")?.target_transform;
        let animation=entity.animation.as_ref().ok_or("head animation absent")?;
        let head_height=animation.sim_lod_pose.as_ref().ok_or("head SIM pose absent")?.local_bone(0)?[13];
        let point=animation.sim_lod_bone_global.as_ref().ok_or("head global pose absent")?.matrices[0][3];
        let selected=*entity.head_tracking.as_ref().ok_or("head tracking owner absent")?.current();
        let resolved=match selected.mode{
            HeadTrackingMode::TargetableCenterOfMass|HeadTrackingMode::TargetableVisibilityPoint=>{
                if let Some((kind,id))=selected.targetable{
                    self.target_centre_of_mass(entity,TargetableId{kind:kind as i32,id:id as i32})?
                }else{None}
            },
            HeadTrackingMode::SkaterCenterOfMass=>Some(actor_centre(local)?),
            HeadTrackingMode::BehaviourSource=>selected.behaviour_source
                .and_then(|id|self.props.entities.get(id))
                .map(|prop|prop.core.as_ref().map(|core|core.transform[3])
                    .ok_or("head behaviour-source core absent".to_string())).transpose()?,
            HeadTrackingMode::WanderDynamic=>{
                let target=entity.navigation_mut()?.path_target;
                if bearing(orientation,target)?.abs()>=80.{None}else{Some(target)}
            },
            HeadTrackingMode::Flee=>{
                let head=entity.head_tracking.as_mut().expect("retained head owner");
                let time=head.flee_look_back_time.ok_or("head flee timer has no producer")?;
                if time<=0.{
                    let looking=!head.flee_looking_back.ok_or("head flee mode has no producer")?;
                    head.flee_looking_back=Some(looking);
                    head.flee_look_back_time=Some(selected.flee_look_back_timer.ok_or("head flee stack timer absent")?);
                    if looking{Some(std::array::from_fn(|i|
                        orientation[1][i].mul_add(head_height,orientation[3][i])-orientation[2][i]))}else{None}
                }else{
                    head.flee_look_back_time=Some(time-dt);Some(head.resolved_target)
                }
            },
            HeadTrackingMode::None|HeadTrackingMode::StraightAhead=>None,
        };
        let head=entity.head_tracking.as_mut().expect("retained head owner");
        if let Some(target)=resolved{head.resolved_target=target;}else{head.set_straight_ahead(orientation,head_height);}
        head.point_of_interest=point;
        Ok(())
    }
}

/// Mode 7's PhysOut system centre of mass (`Update` 0x82e85390..0x82e853b8 copies all four
/// lanes). The port's PhysOut leaves the W lane unwritten; it reads 0.0 because no consumer
/// uses it (`bearing` and `PostAdjust::update` take three-lane lengths, dots and `cross[1]`).
pub(crate) fn actor_centre(actor:&ActorConstruction)->Result<[f32;4],String>{
    let allocation=actor.cached_physical_allocation().map_err(|e|format!("head Actor physical: {e:?}"))?;
    let output=allocation.owner().phys_out().lock().map_err(|_|"head Actor PhysOut lock")?;
    let centre=output.try_system_centre_and_damped_velocity().map_err(|e|format!("head Actor COM: {e:?}"))?.0;
    Ok([centre.xyz[0],centre.xyz[1],centre.xyz[2],centre.w.unwrap_or(0.0)])
}

#[cfg(test)]
mod bearing_tests {
    use super::bearing;

    /// `GetBearingToTarget` (0x82e5acc0): a target on the facing's +X side is a negative
    /// bearing, one on its -X side a positive one; height alone turns nothing.
    #[test]
    fn a_target_on_the_positive_x_side_is_a_negative_bearing() {
        let facing = [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]];
        let right = bearing(facing, [5., 0., 5., 1.]).unwrap();
        let left = bearing(facing, [-5., 0., 5., 1.]).unwrap();
        assert!((right + 45.).abs() < 0.01, "{right}");
        assert!((left - 45.).abs() < 0.01, "{left}");
        assert!(bearing(facing, [0., 2., 5., 1.]).unwrap().abs() < 0.01);
    }
}
