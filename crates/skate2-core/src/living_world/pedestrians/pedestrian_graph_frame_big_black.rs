//! BigBlack's native protection and generated graph callbacks on the live roster.
use super::FrameServices;
use crate::{ai::vmx::{self,add,sub,scale,dot3,length3,xyz,v3},
    living_world_chase::TargetableId,
    living_world_pedestrian_entity::{PedestrianConstruction as Entity,big_black::{self,Threat}},
    pedestrian_adjust_owners::HeadTrackingMode,
    pedestrian_graph::behaviour_binding::LocalBehaviourBindings,
    pedestrian_navigation::NavigationTargetMode,
    reckoning_numeric::{HostReckoningNumeric,ReckoningVector}};
#[path = "pedestrian_big_black_geometry.rs"]
mod geometry;
impl FrameServices<'_>{
    /// BigBlack.TestLOSim82e968a8: retain full simulation while a live threat exists.
    pub(crate) fn big_black_has_threat(&self,entity:&Entity)->Result<bool,String>{
        match entity.big_black.as_ref().and_then(|data|data.threats.first()){
            Some(threat)=>self.target_exists(entity,threat.id),None=>Ok(false),
        }
    }
    pub(crate) fn protector_exists(&self,entity:&Entity,id:TargetableId)->bool{
        id.is_valid() && id.kind==1 && if id.id as u32==entity.id{entity.big_black.is_some()}
            else{self.pedestrian(id).is_some_and(|p|p.big_black.is_some())}
    }
    /// Actor.GetProtecteePosition825f9cd0 delegates through IActor+44 to GetPosition825f9df0.
    pub(crate) fn protected_position(&self,target:TargetableId)->Result<Option<[f32;4]>,String>{
        let Some(actor)=self.actor(target)? else{return Ok(None);};
        let allocation=actor.cached_physical_allocation().map_err(|e|format!("protectee physical: {e:?}"))?;
        let output=allocation.owner().phys_out().lock().map_err(|_|"protectee PhysOut lock")?;
        Ok(Some(output.try_skeleton_output().map_err(|e|format!("protectee position: {e:?}"))?
            .skeleton_root_matrix.columns()[3].operand()))
    }
    /// ProtectorOnCullRequested (IProtector+112) clears the reciprocal IProtectee ID.
    pub(crate) fn unprotect_big_black(&mut self,entity:&mut Entity)->Result<(),String>{
        let Some(data)=entity.big_black.as_ref() else{return Ok(());};
        if let Some(actor)=self.actor(data.protectee)?{
            let mut prefix=actor.prefix().map_err(|e|e.to_string())?;
            let fields=prefix.fields_mut().ok_or("protectee fields absent")?;
            fields.protector=TargetableId::INVALID;fields.protector_invalid=true;
        }
        Ok(())
    }
    /// BigBlack.PreStateGraphUpdate82e9f828 runs after the complete base pre-update.
    pub(crate) fn big_black_prestate(&mut self,entity:&mut Entity)->Result<(),String>{
        let Some(data)=entity.big_black.as_mut() else{return Ok(());};
        if !data.wants.is_valid() && data.assigned.is_valid(){
            data.wants=data.assigned;
            if !data.protectee.is_valid(){data.threats.clear();return Ok(());}
        }
        if !data.wants.is_valid(){return Ok(());}
        if !data.fixed_region()?{
            if let Some(position)=self.protected_position(data.protectee)?{
                data.zone_centre=position;entity.patrol_zone_start_point=position;
            }
        }
        data.zone_radius=data.scalar(big_black::PROTECTION_RADIUS)?;
        data.maintain_radius=data.scalar(big_black::MAINTAIN_RADIUS)?;
        Ok(())
    }
    fn big_black_seek(&mut self,entity:&mut Entity,position:[f32;4])->Result<(),String>{
        self.with_navigation(entity,None,|entity,facts,world,numeric|
            entity.target_explicit(position,NavigationTargetMode::Explicit,facts,world,numeric))
    }
    fn big_black_head(&mut self,entity:&mut Entity,target:TargetableId)->Result<(),String>{
        let exists=self.target_exists(entity,target)?;
        entity.head_tracking.as_mut().ok_or("BigBlack head tracking absent")?
            .push_targetable(HeadTrackingMode::TargetableVisibilityPoint,target,||Ok(exists))
    }
    pub(crate) fn big_black_callback(&mut self,entity:&mut Entity,bindings:&LocalBehaviourBindings,
        occurrence:u16,source:u32)->Result<bool,String>{
        match source{
            // ProtectProtectee.Begin8270d0a0: acquire only an unprotected IProtectee.
            0x8270d0a0=>{
                let target=entity.big_black.as_ref().ok_or("BigBlack owner absent")?.wants;
                if let Some(actor)=self.actor(target)?{
                    let protector=actor.prefix().map_err(|e|e.to_string())?.fields()
                        .ok_or("protectee fields absent")?.protector;
                    if !self.protector_exists(entity,protector){
                        let position=self.protected_position(target)?.ok_or("protectee disappeared")?;
                        let data=entity.big_black.as_mut().unwrap();
                        data.set_protectee(target);data.zone_centre=position;
                        let mut prefix=actor.prefix().map_err(|e|e.to_string())?;
                        let fields=prefix.fields_mut().ok_or("protectee fields absent")?;
                        fields.protector=TargetableId{kind:1,id:entity.id as i32};fields.protector_invalid=false;
                    }
                }
            },
            0x8270df98=>self.evaluate_big_black_threats(entity)?,
            0x8270e098=>{
                let data=entity.big_black.as_ref().ok_or("BigBlack owner absent")?;
                let target=data.threats.first().map(|t|t.id).unwrap_or(TargetableId::INVALID);
                let target=if self.target_exists(entity,target)?{target}else{TargetableId::INVALID};
                entity.state.as_mut().ok_or("BigBlack state absent")?.take_down_sink=target;
            },
            0x8270d1d8|0x8270dad8=>{
                let data=entity.big_black.as_ref().ok_or("BigBlack owner absent")?;
                let target=if source==0x8270d1d8{data.protectee}else{
                    data.threats.first().map(|t|t.id).unwrap_or(TargetableId::INVALID)};
                self.big_black_head(entity,target)?;
                entity.switch_navigation_mode(1);entity.navigation_rig_enabled=true;
            },
            0x8270d2a8=>{
                let target=entity.big_black.as_ref().ok_or("BigBlack owner absent")?.protectee;
                if let Some(position)=self.target_position(entity,target)?{self.big_black_seek(entity,position)?;}
            },
            0x8270d370=>{
                let target=entity.big_black.as_ref().ok_or("BigBlack owner absent")?.protectee;
                self.big_black_head(entity,target)?;entity.navigation_rig_enabled=false;
                if let Some(position)=self.target_centre_of_mass(entity,target)?{self.big_black_seek(entity,position)?;}
            },
            0x8270d478=>{
                let target=entity.big_black.as_ref().ok_or("BigBlack owner absent")?.protectee;
                if let Some(centre)=self.target_centre_of_mass(entity,target)?{
                    let fields=bindings.callback_fields(occurrence)?;
                    let angle=crate::pedestrian_graph::marquee::float(fields,"maxAngle",20.)?*0.017453292;
                    let time=crate::pedestrian_graph::marquee::float(fields,"predictTime",2.)?;
                    let core=entity.core.as_ref().ok_or("BigBlack core absent")?;
                    let angle_to=crate::reckoning_numeric::try_signed_angle(sub(centre,core.transform[3]),
                        core.transform[2],[0.,1.,0.,0.],&mut HostReckoningNumeric)
                        .map_err(|e|format!("BigBlack watch angle: {e:?}"))?;
                    let angle_to=if angle_to>std::f32::consts::PI{angle_to-std::f32::consts::TAU}else{angle_to};
                    if angle_to.abs()>angle{
                        let velocity=self.target_velocity(entity,target)?.ok_or("protectee velocity absent")?;
                        self.big_black_seek(entity,add(scale(velocity,time),centre))?;
                    }
                }
            },
            0x8270d670=>{entity.head_tracking.as_mut().ok_or("BigBlack head tracking absent")?.pop();},
            0x8270d6f8=>{
                let target=entity.big_black.as_ref().ok_or("BigBlack owner absent")?.threats.first()
                    .map(|t|t.id).unwrap_or(TargetableId::INVALID);
                entity.navigation_mut()?.ramped_seek_speed=6.;entity.navigation_rig_enabled=true;
                self.with_navigation(entity,None,|entity,facts,world,numeric|
                    entity.begin_chase_navigation(facts,world,numeric))?;
                self.big_black_head(entity,target)?;
            },
            0x8270d8b8=>{
                if let Some(threat)=entity.big_black.as_ref().ok_or("BigBlack owner absent")?.threats.first().copied(){
                    if self.target_exists(entity,threat.id)? && matches!(threat.action,2|3){
                        let mut point=threat.intercept;point[1]=entity.core.as_ref().ok_or("BigBlack core absent")?.transform[3][1];
                        let speed=entity.max_chase_speed()?;entity.navigation_rig_enabled=true;
                        entity.navigation_mut()?.seek_speed=speed;self.big_black_seek(entity,point)?;
                    }
                }
            },
            0x8270da28|0x8270dee8=>{
                entity.head_tracking.as_mut().ok_or("BigBlack head tracking absent")?.pop();
                entity.navigation_mut()?.use_strafe_target=false;
            },
            0x8270dba8=>self.big_black_block(entity)?,
            _=>return Ok(false),
        }
        Ok(true)
    }
    /// GetProtectorThreatBlockPoint82e82520 uses the assigned ID, with the
    /// protector's own position when that owner is absent.
    fn big_black_block_point(&self,entity:&Entity,threat:Threat)->Result<[f32;4],String>{
        let assigned=entity.big_black.as_ref().ok_or("BigBlack owner absent")?.assigned;
        let own=entity.core.as_ref().ok_or("BigBlack core absent")?.transform[3];
        Ok(self.protected_position(assigned)?.map_or(own,|p|add(threat.block,p)))
    }
    /// BigBlack.UpdateZoneInterests82e70a78. An intercept/block point may differ
    /// from the target's current position; a point outside the buffer is retained.
    pub(crate) fn big_black_zone_interests(&mut self,entity:&mut Entity)->Result<(),String>{
        let data=entity.big_black.as_ref().ok_or("BigBlack owner absent")?;
        let protectee=data.protectee;
        let mut point=entity.patrol_zone_start_point;
        if self.big_black_has_threat(entity)?{
            let threat=data.threats[0];
            entity.patrol_entity_of_interest=Some(threat.id);
            point=match threat.action{
                2|3=>threat.intercept,
                1=>self.big_black_block_point(entity,threat)?,
                _=>self.target_position(entity,threat.id)?.ok_or("protector threat position absent")?,
            };
        }else if self.actor(protectee)?.is_some(){
            entity.patrol_entity_of_interest=Some(protectee);
            point=self.target_position(entity,protectee)?.ok_or("protectee entity position absent")?;
        }else{
            entity.patrol_entity_of_interest=None;
        }
        if self.patrol_zone_contains(entity,point,true)?{entity.patrol_point_of_interest=point;}
        Ok(())
    }
    /// BigBlackBlock.Update8270dba8: run outside ApproachRadius, walk inside it,
    /// stop inside StopRadius; face the actual threat at close range.
    fn big_black_block(&mut self,entity:&mut Entity)->Result<(),String>{
        let data=entity.big_black.as_ref().ok_or("BigBlack owner absent")?;
        let threat=data.threats.first().copied();
        if let Some(threat)=threat.filter(|t|t.action==1){
            if self.target_exists(entity,threat.id)?{
                let own=entity.core.as_ref().ok_or("BigBlack core absent")?.transform[3];
                let point=self.big_black_block_point(entity,threat)?;
                let distance=length3(sub(point,own));
                self.big_black_seek(entity,point)?;
                let parameters=&entity.resources.as_ref().ok_or("BigBlack chase parameters absent")?.chase;
                let scalar=|key|->Result<f32,String>{Ok(f32::from_bits(parameters.try_scalar_word(key,0)
                    .map_err(|e|e.to_string())?.unwrap_or(0)))};
                // Full 64-bit keys checked in PPC GetAttributePointer call sites.
                let approach=scalar(0xa356_8f57_1760_c192)?;
                let stop=scalar(0xdb2a_bdfb_4eb7_ab4e)?;
                if distance>approach{
                    let speed=entity.max_chase_speed()?;entity.navigation_mut()?.seek_speed=speed;
                    entity.navigation_mut()?.use_strafe_target=false;
                }else{
                    if let Some(target)=self.target_position(entity,threat.id)?{
                        entity.navigation_mut()?.set_strafe_target(target);
                    }
                    entity.navigation_mut()?.seek_speed=if distance>stop{2.}else{0.};
                }
                return Ok(());
            }
        }
        entity.navigation_mut()?.seek_speed=0.;Ok(())
    }
    /// Determine/Evaluate/Sort/AutoSwitch/EvaluateAction82e96b00. Actor is the
    /// original live IProtectee owner; pedestrian and DMO objects expose no IProtectee.
    fn evaluate_big_black_threats(&mut self,entity:&mut Entity)->Result<(),String>{
        let data=entity.big_black.as_ref().ok_or("BigBlack owner absent")?;
        let protectee=data.protectee;
        let Some(mut protected)=self.protected_position(protectee)? else{
            entity.big_black.as_mut().unwrap().threats.clear();return Ok(());
        };
        let imminent=data.scalar(big_black::IMMINENT_RADIUS)?;
        let minimum=data.scalar(big_black::MIN_RADIUS)?;let block_radius=data.scalar(big_black::BLOCK_RADIUS)?;
        let ids=self.with_chasee(entity,protectee,|s|s.map(|s|s.chasers.iter().take(20).map(|c|c.id).collect::<Vec<_>>()))?
            .unwrap_or_default();
        let core=entity.core.as_ref().ok_or("BigBlack core absent")?;
        let own=core.transform[3];protected[1]=own[1];
        let max_speed=entity.max_chase_speed()?;
        let parameters=&entity.resources.as_ref().ok_or("BigBlack chase parameters absent")?.chase;
        let max_time=f32::from_bits(parameters.try_scalar_word(0x47c9_6c54_4337_ff32,0).map_err(|e|e.to_string())?.unwrap_or(0));
        let mut velocity=self.target_velocity(entity,protectee)?.ok_or("protectee velocity absent")?;velocity[1]=0.;
        let min_angle=self.chase.min_behind_intercept_angle()?;
        let mut threats=Vec::with_capacity(20);
        for id in ids{
            let mut threat=Threat::new(id);
            if let Some(mut position)=self.target_position(entity,id)?{
                position[1]=own[1];
                let speed=self.pedestrian(id).map_or(Ok(8.),|p|p.max_chase_speed())?;
                threat.alive=!self.target_wiped_out(entity,id)?.unwrap_or(false);
                threat.distance=length3(sub(protected,position));
                threat.threat_travel_time=vmx::reciprocal(speed)*threat.distance;
                let intercept=crate::living_world_chase::prediction::intercept_target(
                    xyz(protected),xyz(velocity),8.,xyz(position),speed,max_time,min_angle,&mut HostReckoningNumeric)
                    .map_err(|e|format!("BigBlack protectee interception: {e:?}"))?;
                threat.threat_intercept=v3(intercept.point);threat.threat_intercept_time=intercept.time;
                threat.threat_can_intercept=intercept.possible;
                // Original scales the displacement by inverse speed (82e8146c), not its length.
                let threat_velocity=scale(sub(threat.threat_intercept,position),vmx::reciprocal(speed));
                let intercept=crate::living_world_chase::prediction::intercept_target(
                    xyz(position),xyz(threat_velocity),speed,xyz(own),max_speed,max_time,min_angle,&mut HostReckoningNumeric)
                    .map_err(|e|format!("BigBlack threat interception: {e:?}"))?;
                threat.can_intercept=intercept.possible;threat.intercept=v3(intercept.point);threat.intercept_time=intercept.time;
                if intercept.possible && intercept.time<threat.threat_intercept_time{
                    threat.intercept_distance=length3(sub(threat.intercept,protected));
                }else{
                    let delta=sub(position,protected);
                    let direction=scale(delta,vmx::rsqrt(dot3(delta,delta)));
                    threat.intercept=vmx::madd(direction,[minimum;4],threat.threat_intercept);
                    threat.intercept_time=vmx::reciprocal(max_speed)*length3(sub(threat.intercept,own));
                    threat.intercept_distance=minimum;
                }
                if let Some(point)=geometry::block_point(protected,position,threat.threat_intercept,imminent,minimum)?{
                    threat.block=point;threat.can_block=true;
                    threat.threat_block_time=vmx::reciprocal(speed)*length3(sub(point,position));
                    threat.block_time=vmx::reciprocal(max_speed)*length3(sub(point,own));
                }else{threat.block=position;threat.threat_block_time=0.;threat.block_time=f32::MAX;}
                threat.block=sub(threat.block,protected);
            }
            threats.push(threat);
        }
        for start in 1..threats.len(){
            for index in (start+1..threats.len()).rev(){
                if threats[index-1].should_swap(&threats[index],imminent){threats.swap(index-1,index);}
            }
        }
        if threats.len()>1 && threats[0].should_swap(&threats[1],imminent){threats.swap(0,1);}
        if let Some(primary)=threats.first_mut(){
            if self.target_exists(entity,primary.id)?{primary.choose_action(imminent,block_radius);}
        }
        entity.big_black.as_mut().unwrap().threats=threats;
        Ok(())
    }
}
