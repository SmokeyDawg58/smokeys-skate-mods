//! Shared native waypoint-interaction callbacks, over existing queue/nav owners.
use super::{Family,PluginController,direct::{self,DirectLeaf,SavedSwitch,SavedSwitchInstance}};
use crate::{pedestrian_graph::{PedestrianGraph,PedestrianGraphBackend},
    generated_action_load::{Attribute,construct::{Comparison,comparison_from_attributes},runtime::ActionUpdateError as Error},
    generated_graph_selection::Mask,living_world_pedestrian_entity::{PedestrianConstruction as Entity,targeting::TargetingWorld},
    pedestrian_navigation::{NavigationTargetMode,mesh::NavigationEntityFacts,wander::WanderAvoidVolumes},
    reckoning_numeric::{ReckoningNumeric,try_length},play_animation::FastString30,
    pedestrian_intent_packet::{IntentStage,PacketState},pedestrian_adjust_owners::HeadTrackingMode};
use std::sync::Arc;
fn fields(family:Family,section:crate::generated_action_load::Section)->Result<&'static [Attribute],String>{
    if section.count==0{return Ok(&[]);}
    family.attributes().get(section.index as usize..section.index as usize+section.count as usize)
        .ok_or("plugin attribute section absent".into())
}
fn text<'a>(fields:&'a [Attribute],key:&str)->Option<&'a str>{fields.iter().find(|a|a.key==key).map(|a|a.value)}
fn scalar(fields:&[Attribute],key:&str,default:f32)->Result<f32,String>{
    text(fields,key).map_or(Ok(default),|s|s.parse::<f32>().map_err(|e|e.to_string()))
}
fn hash(fields:&[Attribute])->Result<u32,String>{Ok(skate2_formats::hash::sk8_hash_value_32(
    text(fields,"intentName").filter(|n|!n.is_empty()).ok_or("native monitored intent-name assertion")?.as_bytes()))}
enum Leaf {
    Direct(DirectLeaf),Switch(SavedSwitch),LockFirst,Unlock,LockPosition,
    Turn{offset:f32},ExplicitTurn,Target{speed:f32,stop:f32,slide:f32,slide_speed:f32,y:f32},
    Intent{hash:u32,name:FastString30,stages:Vec<Option<FastString30>>},Increment(u32),Speech(u32),
    DisableSourceCollisions,OwnObject,SitTimer,
}
enum Condition {Locked,Distance{y:f32,comparison:Comparison},Facing(f32),Monitor(u32),Timer(u32),StandUp}
pub(crate) struct WaypointBindings {family:Family,leaves:Vec<Leaf>,conditions:Vec<Condition>}
impl WaypointBindings{
    pub(crate) fn construct(family:Family)->Result<Self,String>{
        let mut leaves=Vec::new();
        for (index,d) in family.declarations().behaviours.iter().enumerate(){
            let f=fields(family,d.attributes)?;
            let leaf=if let Some(leaf)=DirectLeaf::bind(family,index as u16)?{Leaf::Direct(leaf)}
            else if let Some(leaf)=SavedSwitch::bind(family,index as u16)?{Leaf::Switch(leaf)}
            else{match d.name{
                "LockFirstWaypoint"=>Leaf::LockFirst,"UnlockWaypoint"=>Leaf::Unlock,
                "LockToWaypointPosition"=>Leaf::LockPosition,
                "DisableCollisionsWithBehaviourSource"=>Leaf::DisableSourceCollisions,
                "OwnPluginObject"=>Leaf::OwnObject,"SetSitTimer"=>Leaf::SitTimer,
                "TurnToFaceWaypointOrientation"=>Leaf::Turn{offset:scalar(f,"targetOffset",1.)?},
                "SetExplicitTurnDirectionToWaypointOrientation"=>Leaf::ExplicitTurn,
                "TargetWaypoint"=>Leaf::Target{speed:scalar(f,"speed",2.)?,stop:scalar(f,"stopDistance",0.)?,
                    slide:scalar(f,"slideDistance",0.)?,slide_speed:scalar(f,"slideSpeed",0.)?,y:scalar(f,"yTolerance",1.8)?},
                "CreateSimpleMonitoredIntent"=>{
                    let count=scalar(f,"numberOfStages",1.)?;
                    if !(count>=1.)||count as u32>3{return Err("native monitored stage-count assertion".into());}
                    let name=FastString30::encode(text(f,"intentName").ok_or("intent name absent")?).map_err(|e|format!("intent: {e:?}"))?;
                    let mut stages=Vec::new();
                    for stage in 1..=count as u32{stages.push(text(f,&format!("stage{stage}Name")).filter(|s|!s.is_empty())
                        .map(FastString30::encode).transpose().map_err(|e|format!("intent stage: {e:?}"))?);}
                    Leaf::Intent{hash:hash(f)?,name,stages}
                }
                "IncrementMonitoredPacketStage"=>Leaf::Increment(hash(f)?),
                "SendSpeechEvent"=>{let value=scalar(f,"speechvalue",0.)? as i32;
                    if !(1..72).contains(&value){return Err("native speech-state assertion".into());}Leaf::Speech(value as u32)},
                name=>return Err(format!("waypoint plugin leaf unresolved: {name}")),
            }};
            leaves.push(leaf);
        }
        let mut conditions=Vec::new();
        for d in family.conditions(){let f=fields(family,d.attributes)?;conditions.push(match d.name{
            "HasWaypointLocked"=>Condition::Locked,
            "GoingToStandBackUp"=>Condition::StandUp,
            "DistanceFromWaypointXZ"=>Condition::Distance{y:scalar(f,"yTolerance",1.8)?,
                comparison:comparison_from_attributes(d.name,f).map_err(|e|format!("plugin comparison: {e:?}"))?},
            "IsFacingWaypointOrientation"=>Condition::Facing(scalar(f,"FOVAngle",0.8)?),
            "HasMonitoredIntent"=>Condition::Monitor(hash(f)?),
            "SimpleTimerExpired"=>Condition::Timer(crate::living_world_pedestrian_state::timer_type(text(f,"timerName").unwrap_or("TimedRangeRandom")).ok_or("plugin timer name")?),
            name=>return Err(format!("waypoint plugin condition unresolved: {name}")),
        });}
        Ok(Self{family,leaves,conditions})
    }
}
pub(crate) struct WaypointController {pub(crate) controller:PluginController,pub(crate) bindings:Arc<WaypointBindings>,instances:Vec<Option<SavedSwitchInstance>>}
impl WaypointController{
    pub(crate) fn construct(bindings:Arc<WaypointBindings>)->Self{
        let instances=(0..bindings.leaves.len()).map(|_|None).collect();
        Self{controller:PluginController::construct(bindings.family),bindings,instances}
    }
    pub(crate) fn update<N:ReckoningNumeric,A:WanderAvoidVolumes>(&mut self,dt:f32,entity:&mut Entity,
        source:Option<u32>,facts:&NavigationEntityFacts,world:&mut TargetingWorld<'_,A>,numeric:&mut N)->Result<(),String>
        where N::Error:std::fmt::Debug{
        let mut backend=Backend{entity,source,facts,world,numeric,bindings:&self.bindings,instances:&mut self.instances,failure:None};
        self.controller.update(dt,&mut backend).map_err(|e|backend.failure.unwrap_or_else(||format!("waypoint graph: {e:?}")))
    }
    pub(crate) fn end<N:ReckoningNumeric,A:WanderAvoidVolumes>(&mut self,entity:&mut Entity,
        source:Option<u32>,facts:&NavigationEntityFacts,world:&mut TargetingWorld<'_,A>,numeric:&mut N)->Result<(),String>
        where N::Error:std::fmt::Debug{
        let mut backend=Backend{entity,source,facts,world,numeric,bindings:&self.bindings,instances:&mut self.instances,failure:None};
        self.controller.end_all(&mut backend).map_err(|e|backend.failure.unwrap_or_else(||format!("waypoint graph end: {e:?}")))
    }
}
struct Backend<'a,'w,N,A:WanderAvoidVolumes>{entity:&'a mut Entity,source:Option<u32>,facts:&'a NavigationEntityFacts,
    world:&'a mut TargetingWorld<'w,A>,numeric:&'a mut N,bindings:&'a WaypointBindings,instances:&'a mut [Option<SavedSwitchInstance>],failure:Option<String>}
impl<N:ReckoningNumeric,A:WanderAvoidVolumes> Backend<'_,'_,N,A> where N::Error:std::fmt::Debug{
    fn waypoint(&self)->Result<Option<([f32;4],[f32;4],bool)>,String>{
        self.entity.current_waypoint.map(|r|self.world.props.point(r).map(|p|(p.location,p.orientation,p.locking_entity.is_some()))).transpose()
    }
    fn begin_leaf(&mut self,index:usize)->Result<(),String>{
        match &self.bindings.leaves[index]{
            Leaf::DisableSourceCollisions=>{
                self.entity.state.as_mut().ok_or("plugin state absent")?.flags[1]|=1;
                if let Some(source)=self.source{
                    let container=self.world.props.entities.get(source).ok_or("plugin source absent")?.containing_entity;
                    if let Some(id)=container.filter(|id|self.world.dmos.object(*id).is_some()){
                        self.entity.set_interacting_object(Some(id),self.world.dmos)?;
                    }
                }
            }
            Leaf::OwnObject=>self.entity.state.as_mut().ok_or("plugin state absent")?.flags[1]|=1,
            Leaf::SitTimer=>{
                let maximum=entity_scalar(self.entity,0x69f6_7c67_8b26_73c9)?;
                let minimum=entity_scalar(self.entity,0x1190_3263_71f1_a684)?;
                let time=self.world.random.try_living_world_range(&crate::native_skater_random::NativeRandomGate::production_shared(),minimum,maximum)
                    .map_err(|e|format!("sit timer random: {e:?}"))?;
                self.entity.state.as_mut().ok_or("plugin state absent")?.set_timer(24,time).map_err(|e|e.to_string())?;
            }
            Leaf::Direct(DirectLeaf::ExitPlugin)=>direct::exit_plugin(self.entity,self.source,self.world.props)?,
            Leaf::Direct(leaf)=>leaf.begin(self.entity,self.source)?,
            Leaf::Switch(leaf)=>leaf.begin(self.instances[index].as_mut().ok_or("saved switch instance absent")?,self.entity)?,
            Leaf::LockFirst=>{
                self.entity.navigation_mut()?.arrival_radius=0.2;
                if self.entity.current_waypoint.is_none(){if let Some(prop)=self.source{
                    let source=self.world.props.entities.get(prop).ok_or("plugin source absent")?;
                    if source.waypoint.as_ref().and_then(|g|g.points.first()).is_some_and(|p|p.locking_entity.is_none()){
                        let reference=crate::living_world_grind_services::waypoints::WaypointRef{prop,index:0};
                        self.entity.set_current_waypoint(Some(reference));
                        self.world.props.point_mut(reference)?.lock(self.entity.id)?;
                        self.entity.navigation_rig_enabled=true;
                    }
                }}
            }
            Leaf::Unlock=>direct::unlock_waypoint(self.entity,self.world.props)?,
            Leaf::LockPosition=>if let Some((position,_,_))=self.waypoint()?{
                let adjust=self.entity.spatial_adjust.as_mut().ok_or("plugin spatial adjust absent")?;
                adjust.position_locked=true;adjust.locked_position=position;
            },
            Leaf::Turn{offset}=>{
                self.entity.head_tracking.as_mut().ok_or("plugin head tracking absent")?.push(HeadTrackingMode::StraightAhead)?;
                if let Some((_,direction,_))=self.waypoint()?{
                    self.entity.navigation_intent.explicit_direction=Some(direction);
                    let position=self.entity.core.as_ref().ok_or("plugin core absent")?.transform[3];
                    let target=std::array::from_fn(|i|position[i]+direction[i]*offset);
                    self.entity.target_explicit(target,NavigationTargetMode::Explicit,self.facts,self.world,self.numeric)?;
                    self.entity.navigation_mut()?.seek_speed=0.;
                }
            }
            Leaf::ExplicitTurn=>if let Some((_,direction,_))=self.waypoint()?{self.entity.navigation_intent.explicit_direction=Some(direction);},
            Leaf::Target{..}=>{
                if let Some((p,_,_))=self.waypoint()?{self.entity.state.as_mut().ok_or("plugin state absent")?.fixed_target=Some([p[0],p[1],p[2]]);}
                self.entity.switch_navigation_mode(1);
                self.entity.switch_targeting_mode(0,self.facts,self.world,self.numeric)?;
            }
            Leaf::Intent{hash,name,stages}=>{
                let queue=self.entity.intent_queue.as_mut().ok_or("plugin intent queue absent")?;
                let id=queue.create_monitored(self.entity.state.as_mut().ok_or("plugin state absent")?,*hash)?;
                for suffix in stages{let mut stage=IntentStage::default();stage.add_intent(name.clone())?;
                    if let Some(suffix)=suffix{stage.add_intent(suffix.clone())?;}queue.packet_mut(id)?.add_stage(stage)?;}
                queue.suggest(id,&mut self.entity.animation.as_mut().ok_or("plugin animation absent")?.ai_input)?;
            }
            Leaf::Increment(hash)=>if let Some(&id)=self.entity.state.as_ref().ok_or("plugin state absent")?.intent_monitors.get(hash){
                self.entity.intent_queue.as_mut().ok_or("plugin intent queue absent")?.packet_mut(id)?.advance_stage()?;
            },
            Leaf::Speech(state)=>self.entity.state_out.state=*state,
        }Ok(())
    }
    fn end_leaf(&mut self,index:usize)->Result<(),String>{match &self.bindings.leaves[index]{
        Leaf::DisableSourceCollisions=>{
            self.entity.state.as_mut().ok_or("plugin state absent")?.flags[1]&=!1;
            self.entity.set_interacting_object(None,self.world.dmos)?;
            self.entity.state.as_mut().ok_or("plugin state absent")?.flags[2]&=!0x80;
            let timer=self.world.cvars.try_plugin_unlock_timer().map_err(|e|format!("plugin unlock cvar: {e:?}"))?;
            self.entity.state.as_mut().ok_or("plugin state absent")?.set_timer(42,timer).map_err(|e|e.to_string())?;
        }
        Leaf::OwnObject=>self.entity.state.as_mut().ok_or("plugin state absent")?.flags[1]&=!1,
        Leaf::Direct(leaf)=>leaf.end(self.entity)?,
        Leaf::Switch(leaf)=>leaf.end(self.instances[index].as_ref().ok_or("saved switch instance absent")?,self.entity)?,
        Leaf::Turn{..}=>{self.entity.head_tracking.as_mut().ok_or("plugin head tracking absent")?.pop();self.entity.navigation_intent.explicit_direction=None;},
        Leaf::ExplicitTurn=>self.entity.navigation_intent.explicit_direction=None,
        Leaf::LockPosition=>self.entity.spatial_adjust.as_mut().ok_or("plugin spatial adjust absent")?.position_locked=false,
        Leaf::Target{..}=>self.entity.spatial_adjust.as_mut().ok_or("plugin spatial adjust absent")?.sliding_to_position=false,
        Leaf::Intent{hash,..}=>{
            let state=self.entity.state.as_mut().ok_or("plugin state absent")?;
            if let Some(&id)=state.intent_monitors.get(hash){let queue=self.entity.intent_queue.as_mut().ok_or("plugin intent queue absent")?;
                queue.packet_mut(id)?.state=PacketState::CompleteNormal;queue.release_monitored(state,*hash)?;}
        }
        Leaf::LockFirst|Leaf::Unlock|Leaf::Increment(_)|Leaf::Speech(_)|Leaf::SitTimer=>{},//native empty End825187c0
    }Ok(())}
    fn update_leaf(&mut self,index:usize)->Result<(),String>{
        let Leaf::Target{speed,stop,slide,slide_speed,y}=&self.bindings.leaves[index]else{return Ok(());};
        self.entity.navigation_mut()?.seek_speed=*speed;
        if let Some((target,_,_))=self.waypoint()?{
            let p=self.entity.core.as_ref().ok_or("plugin core absent")?.transform[3];
            self.entity.target_explicit(target,NavigationTargetMode::Explicit,self.facts,self.world,self.numeric)?;
            if *y>(p[1]-target[1]).abs(){
                let distance=try_length([target[0]-p[0],0.,target[2]-p[2],0.],self.numeric).map_err(|e|format!("plugin distance: {e:?}"))?;
                if distance<*stop{self.entity.navigation_mut()?.seek_speed=0.;}
                let adjust=self.entity.spatial_adjust.as_mut().ok_or("plugin spatial adjust absent")?;
                if distance>=*slide{adjust.sliding_to_position=false;}else{
                    adjust.sliding_speed=*slide_speed;adjust.sliding_position=target;adjust.sliding_to_position=true;
                }
            }
        }Ok(())
    }
    fn evaluate(&mut self,index:usize)->Result<bool,String>{Ok(match &self.bindings.conditions[index]{
        Condition::StandUp=>{
            let chance=entity_scalar(self.entity,0xb040_d387_aba6_e24d)?;
            crate::native_skater_random::rw_math::bernoulli_choice(chance,crate::native_skater_random::rw_math::global_next().map_err(|e|format!("stand-up random: {e:?}"))?)
        }
        Condition::Locked=>self.waypoint()?.is_some_and(|p|p.2),
        Condition::Timer(timer)=>self.entity.state.as_ref().ok_or("plugin state absent")?.timer(*timer)<=0.,
        Condition::Monitor(hash)=>match self.entity.state.as_ref().ok_or("plugin state absent")?.intent_monitors.get(hash){
            Some(&id)=>self.entity.intent_queue.as_ref().ok_or("plugin queue absent")?.packet(id)?.in_queue,None=>false},
        Condition::Facing(angle)=>if let Some((_,orientation,_))=self.waypoint()?{
            let facing=self.entity.core.as_ref().ok_or("plugin core absent")?.transform[2];
            let dot=self.numeric.dot3(facing,orientation).map_err(|e|format!("plugin facing: {e:?}"))?;
            let cosine=crate::reckoning_sin_cos::try_cosine(*angle,&mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric).unwrap_or_else(|never|match never{});
            dot>cosine
        }else{false},
        Condition::Distance{y,comparison}=>if let Some((point,_,_))=self.waypoint()?{
            let p=self.entity.core.as_ref().ok_or("plugin core absent")?.transform[3];
            if *y>(p[1]-point[1]).abs(){
                let value=try_length([p[0]-point[0],0.,p[2]-point[2],0.],self.numeric).map_err(|e|format!("plugin distance: {e:?}"))?;
                crate::generated_action_load::conditions::compare(*comparison,value)
            }else{false}
        }else{false},
    })}
}
fn entity_scalar(entity:&Entity,key:u64)->Result<f32,String>{
    let attributes=&entity.resources.as_ref().ok_or("plugin resources absent")?.entity;
    if attributes.is_null(){return Err("plugin entity collection absent".into());}
    Ok(f32::from_bits(attributes.try_scalar_word(key,0).map_err(|e|e.to_string())?.unwrap_or(0)))
}
impl<N:ReckoningNumeric,A:WanderAvoidVolumes> PedestrianGraphBackend for Backend<'_,'_,N,A>where N::Error:std::fmt::Debug{
    fn condition(&mut self,_:&PedestrianGraph,_:Mask,index:u16,_:u16)->Result<bool,Error>{
        if index as usize>=self.bindings.conditions.len(){return Err(Error::PendingExpression(index));}
        self.evaluate(index as usize).map_err(|e|{self.failure=Some(e);Error::PendingExpression(index)})
    }
    fn enabled(&self,index:u16)->Result<bool,Error>{if (index as usize)<self.bindings.leaves.len(){Ok(true)}else{Err(Error::MissingInstance(index))}}
    fn create_instance(&mut self,_:&PedestrianGraph,index:u16)->Result<(),Error>{
        let instance=self.instances.get_mut(index as usize).ok_or(Error::MissingInstance(index))?;
        *instance=if matches!(self.bindings.leaves[index as usize],Leaf::Switch(_)){Some(SavedSwitchInstance::new())}else{None};Ok(())
    }
    fn begin(&mut self,_:&PedestrianGraph,index:u16)->Result<(),Error>{self.begin_leaf(index as usize).map_err(|e|{self.failure=Some(e);Error::MissingInstance(index)})}
    fn end_and_remove(&mut self,_:&PedestrianGraph,index:u16)->Result<(),Error>{self.end_leaf(index as usize).map_err(|e|{self.failure=Some(e);Error::MissingInstance(index)})}
    fn transition_hook(&mut self,_:&PedestrianGraph,index:u16,_:u16)->Result<(),Error>{Err(Error::PendingTransitionHook(index))}
    fn generated_update(&mut self,_:&PedestrianGraph,address:u32,active:&[u16])->Result<(),Error>{
        if !self.bindings.family.declarations().states.iter().any(|s|s.update_address==address){return Err(Error::PendingStateUpdate(address));}
        // Audited folded generated functions consume the active occurrence
        // vector in ascending declaration order. Only these two functions
        // invoke a nonempty Update, TargetWaypoint in slot0/slot1 respectively.
        let (count,target)=match address{
            0x825187c0=>(0,None),0x828f81d0=>(1,None),0x828f81e8=>(2,None),
            0x828f5cc8=>(3,None),0x828f83b8|0x828f84b8=>(6,None),
            0x828f8468=>(7,None),0x828f84c0|0x828f88a0=>(5,None),
            0x828f8810|0x828f8898=>(4,None),
            0x828f8798=>(1,Some(0)),0x828f87b0=>(2,Some(1)),
            _=>return Err(Error::PendingStateUpdate(address)),
        };
        if active.len()!=count{return Err(Error::PendingStateUpdate(address));}
        if let Some(slot)=target{
            let index=active[slot] as usize;
            if !matches!(self.bindings.leaves[index],Leaf::Target{..}){return Err(Error::PendingStateUpdate(address));}
            self.update_leaf(index).map_err(|e|{self.failure=Some(e);Error::PendingStateUpdate(address)})?;
        }Ok(())
    }
}
