//! Original want-target callbacks over the retained navigation/hand-prop owners.
use super::FrameServices;
use crate::{living_world_pedestrian_entity::PedestrianConstruction as Entity,
    living_world_guard_graph::binding, pedestrian_navigation::NavigationTargetMode as Mode,
    pedestrian_adjust_owners::HeadTrackingMode, reckoning_numeric::HostReckoningNumeric};
fn want(occurrence:u16)->Result<usize,String>{
    let name=binding::attribute(occurrence,"want")?.ok_or("native want attribute assertion")?;
    crate::pedestrian_graph::conditions::want_type(name).ok_or_else(||format!("native invalid want: {name}"))
}
impl FrameServices<'_>{
    pub(crate) fn dispatch_want_callback(&mut self,entity:&mut Entity,occurrence:u16,source:u32)->Result<(),String>{
        match source {
            0x826fe580=>self.with_navigation(entity,None,|entity,facts,world,numeric|
                entity.switch_targeting_mode(5,facts,world,numeric))?,
            0x826fe5e0=>self.run_from_honker(entity)?,
            0x82707d48|0x82707ef0=>{
                let want=want(occurrence)?;
                let target=entity.state.as_ref().ok_or("greet state absent")?.wants[want].target;
                let returning=binding::boolean(occurrence,"returnGreet",false)?;
                let deactivate=binding::boolean(occurrence,"deactivateWant",true)?;
                let exists=self.target_exists(entity,target)?;
                if source==0x82707d48 {
                    if exists {
                        self.request_channel(entity,"GreetChannel")?;
                        let time=self.cvars.try_greet_interest_timer().map_err(|e|format!("greet tuning: {e:?}"))?;
                        entity.state.as_mut().unwrap().set_timer(36,time).map_err(|e|e.to_string())?;
                        if !returning && target.kind==1 {
                            let event=crate::hermes_game_sim_queue::MoodEvent{event_type:16,
                                position:entity.core.as_ref().ok_or("greet core absent")?.transform[3].map(f32::to_bits),
                                perpetrator:crate::living_world_chase::TargetableId{kind:1,id:entity.id as i32},victim:target,
                                loudness_distance_factor:1f32.to_bits(),visibility_distance_factor:1f32.to_bits()};
                            let ground=self.ground.ok_or("greet detection ground owner absent")?;
                            self.receive_direct_mood(entity,target,&event,ground)?;
                        }
                    }else if deactivate {entity.state.as_mut().unwrap().set_want_active(want,false).map_err(|e|e.to_string())?;}
                }else{
                    if exists && target.kind==1 {
                        let other=if target.id as u32==entity.id {&*entity}else{self.pedestrian(target).ok_or("greet target vanished")?};
                        //PedStateOut.GetAudioCharacter82e78490 reads livingworld_models layout+4.
                        let model=&other.resources.as_ref().ok_or("greet model resource absent")?.model;
                        // GetAudioCharacter82e784e4 reads exactly layout+4.
                        // The preceding layout+0 is a relocated string pointer.
                        let data=model.try_cached_layout_field(4,4).map_err(|e|e.to_string())?.ok_or("greet model layout absent")?;
                        let character=u32::from_be_bytes(data.bytes().try_into().unwrap());
                        entity.state_out.target_greet_character=character;
                        entity.state_out.state=if returning {69}else{62};
                    }
                    if deactivate && entity.state.as_ref().unwrap().timer(36)<=0. {
                        entity.state.as_mut().unwrap().set_want_active(want,false).map_err(|e|e.to_string())?;
                    }
                }
            }
            //DrawDebugUpdateMarker826ff360 only requests the debug overlay and
            //reads Entity.GetPosition when enabled. No gameplay state changes.
            0x826ff360=>{},
            0x826ef2a8=>entity.release_hand_prop_to_physics(self.world,self.dmos,self.cvars,false,[0.;4])?,
            0x826eb7d0=>entity.head_tracking.as_mut().ok_or("alert head tracking absent")?.pop(),
            0x826ef3c0=>entity.ignore_collision_target=entity.state.as_ref().ok_or("ignore target state absent")?.wants[want(occurrence)?].target,
            0x826fde30=>{
                let target=entity.state.as_ref().ok_or("alert state absent")?.wants[want(occurrence)?].target;
                let exists=self.target_exists(entity,target)?;
                entity.head_tracking.as_mut().ok_or("alert head tracking absent")?
                    .push_targetable(HeadTrackingMode::TargetableVisibilityPoint,target,||Ok(exists))?;
            }
            0x826eb920|0x826ef4f0=>{
                let target=entity.state.as_ref().ok_or("approach state absent")?.wants[want(occurrence)?].target;
                if let Some(position)=self.target_position(entity,target)? {
                    self.with_navigation(entity,None,|entity,facts,world,numeric|
                        entity.target_explicit(position,Mode::Explicit,facts,world,numeric))?;
                    if source==0x826eb920 {entity.navigation_mut()?.seek_speed=binding::float(occurrence,"speed",Some(2.))?;}
                }
            }
            0x826eead0=>{
                let target=entity.state.as_ref().ok_or("throw state absent")?.wants[want(occurrence)?].target;
                self.throw_hand_prop_at_targetable(entity,target)?;
            }
            0x826eeb50=>{
                let state=entity.state.as_mut().ok_or("throw state absent")?;
                if state.flags[2]&2==0 && state.timer(35)<=0. {
                    state.set_want_active(want(occurrence)?,false).map_err(|e|e.to_string())?;
                }
            }
            0x8270e840=>{
                let point=entity.patrol_point_of_interest;
                let position=entity.core.as_ref().ok_or("zone approach core absent")?.transform[3];
                let navigation=entity.navigation_mut()?;
                let arrived=navigation.has_arrived_xz(position,point,navigation.arrival_radius,&mut HostReckoningNumeric)
                    .map_err(|e|format!("zone approach arrival: {e:?}"))?;
                if arrived {
                    let interest=entity.patrol_entity_of_interest.map(|id|self.target_position(entity,id)).transpose()?.flatten();
                    self.with_navigation(entity,None,|entity,facts,world,numeric|
                        entity.finish_return_to_patrol_zone(interest,facts,world,numeric))?;
                }else{
                    self.with_navigation(entity,None,|entity,facts,world,numeric|
                        entity.target_explicit(point,Mode::Explicit,facts,world,numeric))?;
                    entity.navigation_mut()?.seek_speed=1.8;
                }
            }
            _=>return Err(format!("want callback source {source:#x} absent")),
        }
        Ok(())
    }
    /// ThrowHandPropAtTargetable82e9b960 predicts both entities through the
    /// attack delay, uses shared native interception, then Manager GetRandomPos.
    fn throw_hand_prop_at_targetable(&mut self,entity:&mut Entity,target:crate::living_world_chase::TargetableId)->Result<(),String>{
        if !self.target_exists(entity,target)? {return Ok(());}
        let cv=|e|format!("throw target tuning: {e:?}");
        let delay=self.cvars.try_hand_prop_attack_throw_timeout().map_err(cv)?;
        let velocity=self.target_velocity(entity,target)?.ok_or("throw target velocity absent")?;
        let centre=self.target_centre_of_mass(entity,target)?.ok_or("throw target centre absent")?;
        //Native uses separate multiply and add for both delayed positions.
        let target_start=std::array::from_fn::<_,4,_>(|i|centre[i]+velocity[i]*delay);
        let own=crate::living_world_chase::TargetableId{kind:1,id:entity.id as i32};
        let own_velocity=self.target_velocity(entity,own)?.ok_or("throw self velocity absent")?;
        let own_centre=self.target_centre_of_mass(entity,own)?.ok_or("throw self centre absent")?;
        let start=std::array::from_fn::<_,4,_>(|i|own_centre[i]+own_velocity[i]*delay);
        let force=self.cvars.try_hand_prop_attack_throw_force().map_err(cv)?;
        let xyz=|v:[f32;4]|[v[0],v[1],v[2]];
        let prediction=crate::living_world_chase::prediction::predict_intersect_at_max_speed(
            xyz(target_start),xyz(velocity),xyz(start),force,&mut HostReckoningNumeric)
            .map_err(|e|format!("throw target prediction: {e:?}"))?;
        //Native no-intersection fallback is the thrower's predicted position.
        let mut point=prediction.points.first().map_or(start,|p|[p.point[0],p.point[1],p.point[2],0.]);
        let radius=self.cvars.try_hand_prop_attack_perturb_radius().map_err(cv)?;
        //Raw82e9bb80 skips all RNG draws when abs(radius)<=EPSILON.
        if radius.abs()>f32::from_bits(0x3400_0000) {
            point=crate::living_world_grind_services::census::random::random_position(self.random,
                &crate::native_skater_random::NativeRandomGate::production_shared(),point,0.,radius,&mut HostReckoningNumeric)
                .map_err(|e|format!("throw target perturbation: {e:?}"))?;
        }
        point[1]+=self.cvars.try_hand_prop_attack_y_offset().map_err(cv)?;
        entity.throw_hand_prop_at_position(point,true,self.cvars,self.random,&mut HostReckoningNumeric)
    }
    /// RunFromHonker.Update826fe5e0: flee perpendicular to the actual vehicle
    /// velocity on the same side of its movement plane as the pedestrian.
    fn run_from_honker(&mut self,entity:&mut Entity)->Result<(),String>{
        use crate::reckoning_numeric::{ReckoningNumeric,try_normalize_unsafe,cross_full4};
        let id=entity.state.as_ref().ok_or("honker state absent")?.honking_vehicle_id;
        if id == -1 {return Ok(());}
        let Some(vehicle)=self.vehicles.get(id as u32) else{return Ok(());};
        let position=entity.core.as_ref().ok_or("honker pedestrian core absent")?.transform[3];
        let vehicle_position=vehicle.core.transform[3];
        let mut numeric=HostReckoningNumeric;
        let err=|e|format!("honker vector: {e:?}");
        let velocity=try_normalize_unsafe(vehicle.core.velocity,&mut numeric).map_err(err)?;
        let side=cross_full4(velocity,[0.,-1.,0.,0.]).map(|v|v*f32::from_bits(0x3fe6_6666));
        let side=try_normalize_unsafe(side,&mut numeric).map_err(err)?;
        let displacement=side.map(|v|v*10.);
        //The source normalizes this unit direction a second time for its plane test.
        let normal=try_normalize_unsafe(side,&mut numeric).map_err(err)?;
        let dot=numeric.dot3(position,normal).map_err(err)?;
        let offset=normal[2].mul_add(vehicle_position[2],normal[0].mul_add(vehicle_position[0],normal[1]*vehicle_position[1]));
        let target=std::array::from_fn(|i|if dot-offset>0. {position[i]+displacement[i]}else{position[i]-displacement[i]});
        self.with_navigation(entity,None,|entity,facts,world,numeric|
            entity.target_explicit(target,Mode::Explicit,facts,world,numeric))?;
        //Virtual+184 reads Navigation.mPathTarget(+112), after SetTarget.
        let path=entity.navigation_mut()?.path_target;
        let delta=[path[0]-position[0],0.,path[2]-position[2],path[3]-position[3]];
        let squared=numeric.dot3(delta,delta).map_err(err)?;
        entity.navigation_mut()?.seek_speed=if squared-4.>0. {6.}else{3.};
        Ok(())
    }
}
