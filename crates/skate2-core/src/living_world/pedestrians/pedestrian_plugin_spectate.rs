//! Spectate callbacks826fdef0..826fedc0 and generated delegate828f84c0.
//! Mutable payloads belong to each copied controller; external owners are borrowed.
use super::{Family,PluginController,direct::{self,DirectLeaf,SavedSwitch,SavedSwitchInstance}};
use crate::{pedestrian_graph::{PedestrianGraph,PedestrianGraphBackend,backend::frame::FrameServices,
    colliding::{CollidingBehaviour,CollidingInstance}},
    generated_action_load::runtime::ActionUpdateError as Error,generated_graph_selection::Mask,
    living_world_pedestrian_entity::PedestrianConstruction as Entity,
    living_world_chase::TargetableId,pedestrian_navigation::NavigationTargetMode,
    pedestrian_adjust_owners::HeadTrackingMode,
    reckoning_numeric::{HostReckoningNumeric,ReckoningNumeric,try_length,try_normalize_safe}};
use std::sync::Arc;
enum Leaf{Direct(DirectLeaf),Switch(SavedSwitch),Lock,Override(u64),Collision(CollidingBehaviour),Unlock,Turn,Spectate,Watch,
    Register,Recognition,Velocity(crate::pedestrian_graph::locomotion::SuggestVelocity)}
pub(crate) struct SpectateBindings{family:Family,leaves:Vec<Leaf>,boredom:f32,long:i32,short:i32,max_short:f32,min_short:f32,wait:f32,angle:f32,predict:f32}
impl SpectateBindings{
    pub(crate) fn construct(family:Family)->Result<Self,String>{
        if !matches!(family,Family::Spectate|Family::ActortrackerSpectate){return Err("unsupported spectate family".into());}
        let mut leaves=Vec::new();
        for (i,d) in family.declarations().behaviours.iter().enumerate(){
            leaves.push(if let Some(value)=DirectLeaf::bind(family,i as u16)?{Leaf::Direct(value)}
                else if let Some(value)=SavedSwitch::bind(family,i as u16)?{Leaf::Switch(value)}
                else{match d.name{
                    "LockAndTargetWayPoint"=>Leaf::Lock,"OverrideAnimData"=>{
                        let section=d.attributes;let attributes=&family.attributes()[section.index as usize..section.index as usize+section.count as usize];
                        let name=attributes.iter().find(|a|a.key=="overrideEntityName").ok_or("native animation override-name assertion")?.value;
                        Leaf::Override(skate2_formats::attrib::string_hash64(name))},
                    "PedestrianColliding"=>Leaf::Collision(CollidingBehaviour{warn_chance:0.7,broadcast_radius:20.}),
                    "UnlockWaypoint"=>Leaf::Unlock,"TurnToFaceSkater"=>Leaf::Turn,"Spectate"=>Leaf::Spectate,
                    "StandAndWatchSkater"=>Leaf::Watch,"RegisterRecognizer"=>Leaf::Register,"PlayerRecognition"=>Leaf::Recognition,
                    "SuggestVelocity"=>{let section=d.attributes;Leaf::Velocity(crate::pedestrian_graph::locomotion::SuggestVelocity::from_attributes(
                        &family.attributes()[section.index as usize..section.index as usize+section.count as usize])?)},
                    name=>return Err(format!("spectate leaf absent: {name}")),
                }});
        }
        // Authored attributes plus constructor826ebae0/826ec560 defaults.
        let get=|name:&str,default:f32|->Result<f32,String>{family.attributes().iter().find(|a|a.key==name)
            .map_or(Ok(default),|a|a.value.parse::<f32>().map_err(|e|e.to_string()))};
        let max_short=get("maxCheerTime",7.)?;let min_short=get("minCheerTime",3.)?;
        if !(max_short>=min_short){return Err("native Spectate cheer-range assertion".into());}
        Ok(Self{family,leaves,boredom:get("boredomTime",60.)?,long:get("longCheerThreshold",0.)? as i32,
            short:get("shortCheerThreshold",0.)? as i32,max_short,min_short,wait:get("waitTimeAfterCheer",2.)?,
            angle:get("maxAngle",0.4)?,predict:get("predictTime",2.)?})
    }
}
enum Instance{Switch(SavedSwitchInstance),Collision(CollidingInstance),Cheer{delay:f32,duration:f32,kind:u32,flags:u8},Watch([f32;4])}
pub(crate) struct SpectateController{pub(crate) controller:PluginController,pub(crate) bindings:Arc<SpectateBindings>,instances:Vec<Option<Instance>>}
impl SpectateController{
    pub(crate) fn construct(bindings:Arc<SpectateBindings>)->Self{Self{controller:PluginController::construct(bindings.family),
        instances:(0..bindings.leaves.len()).map(|_|None).collect(),bindings}}
    pub(crate) fn run(&mut self,entity:&mut Entity,source:Option<u32>,frame:&mut FrameServices<'_>,dt:Option<f32>)->Result<(),String>{
        let mut backend=Backend{entity,source,frame,bindings:&self.bindings,instances:&mut self.instances,dt:dt.unwrap_or(0.),failure:None};
        let result=match dt{Some(dt)=>self.controller.update(dt,&mut backend),None=>self.controller.end_all(&mut backend)};
        result.map_err(|e|backend.failure.unwrap_or_else(||format!("spectate graph: {e:?}")))
    }
}
struct Backend<'a,'f>{entity:&'a mut Entity,source:Option<u32>,frame:&'a mut FrameServices<'f>,bindings:&'a SpectateBindings,
    instances:&'a mut [Option<Instance>],dt:f32,failure:Option<String>}
impl Backend<'_,'_>{
    fn local(&self)->Result<TargetableId,String>{Ok(TargetableId{kind:0,id:self.frame.world.local_skater_identity.as_ref()
        .ok_or("spectate SkaterManager absent")?.try_local_index().map_err(|e|e.to_string())?})}
    fn centre(&self)->Result<[f32;4],String>{self.frame.target_centre_of_mass(self.entity,self.local()?)?.ok_or("spectate local Actor absent".into())}
    fn position(&self)->Result<[f32;4],String>{Ok(self.entity.core.as_ref().ok_or("spectate core absent")?.transform[3])}
    fn timer(&self,id:u32)->Result<f32,String>{Ok(self.entity.state.as_ref().ok_or("spectate AI state absent")?.timer(id))}
    fn set_timer(&mut self,id:u32,time:f32)->Result<(),String>{self.entity.state.as_mut().ok_or("spectate AI state absent")?.set_timer(id,time).map_err(|e|e.to_string())}
    fn push(&mut self)->Result<(),String>{self.entity.head_tracking.as_mut().ok_or("spectate head owner absent")?.push(HeadTrackingMode::SkaterCenterOfMass)}
    fn pop(&mut self)->Result<(),String>{self.entity.head_tracking.as_mut().ok_or("spectate head owner absent")?.pop();Ok(())}
    fn target(&mut self,p:[f32;4])->Result<(),String>{self.frame.with_navigation(self.entity,None,|entity,facts,world,numeric|
        entity.target_explicit(p,NavigationTargetMode::ExplicitNoSeekNoAstar,facts,world,numeric))}
    fn begin_leaf(&mut self,i:usize)->Result<(),String>{match &self.bindings.leaves[i]{
        Leaf::Register=>self.frame.props.register_recognizer(self.entity.id),
        Leaf::Recognition=>self.recognize(),
        Leaf::Velocity(_)=>Ok(()),
        Leaf::Direct(DirectLeaf::ExitPlugin)=>direct::exit_plugin(self.entity,self.source,self.frame.props),
        Leaf::Direct(value)=>value.begin(self.entity,self.source),
        Leaf::Switch(value)=>{let Some(Instance::Switch(instance))=&mut self.instances[i] else{return Err("spectate switch instance absent".into());};value.begin(instance,self.entity)},
        Leaf::Lock=>direct::begin_lock_and_target_waypoint(self.entity,self.source,self.frame.props),
        Leaf::Unlock=>direct::unlock_waypoint(self.entity,self.frame.props),
        Leaf::Override(key)=>{let anim=self.entity.animation.as_mut().ok_or("spectate animation absent")?;
            anim.use_override_animation_data=true;
            anim.override_animation_data=Some(self.frame.vault.instance(0x53e3_32f6_7b81_24b2,*key));Ok(())},
        Leaf::Collision(value)=>{let Some(Instance::Collision(instance))=&mut self.instances[i] else{return Err("spectate collision instance absent".into());};value.begin(instance,self.entity,self.frame)},
        Leaf::Turn=>{self.push()?;self.entity.switch_navigation_mode(1);Ok(())},
        Leaf::Spectate=>self.set_timer(9,self.bindings.boredom),
        Leaf::Watch=>{self.push()?;let position=self.position()?;let facing=self.entity.core.as_ref().unwrap().transform[2];
            self.instances[i]=Some(Instance::Watch(std::array::from_fn(|n|facing[n]*4.+position[n])));Ok(())},
    }}
    fn end_leaf(&mut self,i:usize)->Result<(),String>{match &self.bindings.leaves[i]{
        Leaf::Register=>self.frame.props.unregister_recognizer(self.entity.id),
        Leaf::Recognition=>self.pop(),
        Leaf::Velocity(value)=>{value.end(self.entity.navigation_mut()?);Ok(())},
        Leaf::Direct(value)=>value.end(self.entity),
        Leaf::Switch(value)=>{let Some(Instance::Switch(instance))=&self.instances[i] else{return Err("spectate switch instance absent".into());};value.end(instance,self.entity)},
        Leaf::Override(_)=>{self.entity.animation.as_mut().ok_or("spectate animation absent")?.use_override_animation_data=false;Ok(())},
        Leaf::Collision(value)=>{let Some(Instance::Collision(instance))=&mut self.instances[i] else{return Err("spectate collision instance absent".into());};value.end(instance,self.entity,self.frame.world)},
        Leaf::Turn|Leaf::Watch=>self.pop(),
        Leaf::Spectate=>{self.pop()?;if !(self.timer(9)?>0.){self.entity.state_out.state=26;}Ok(())},
        Leaf::Lock|Leaf::Unlock=>Ok(()),
    }}
    fn update_leaf(&mut self,i:usize)->Result<(),String>{match &self.bindings.leaves[i]{
        Leaf::Velocity(value)=>{value.update(self.entity.navigation_mut()?);Ok(())},
        Leaf::Lock=>{let source=self.source;self.frame.with_navigation(self.entity,None,|entity,facts,world,numeric|
            direct::update_lock_and_target_waypoint(entity,source,facts,world,numeric))},
        Leaf::Collision(value)=>{let Some(Instance::Collision(instance))=&mut self.instances[i] else{return Err("spectate collision instance absent".into());};value.update(instance,self.entity,self.dt,self.frame)},
        Leaf::Turn=>{let centre=self.centre()?;self.target(centre)},
        Leaf::Spectate=>self.cheer(i),
        Leaf::Watch=>{
            let local=self.local()?;if self.frame.actor(local)?.is_none(){return Err("StandAndWatch local Actor absent".into());}
            self.entity.navigation_mut()?.seek_speed=0.;
            let Some(Instance::Watch(target))=&self.instances[i] else{return Err("stand-and-watch instance absent".into());};let target=*target;
            self.target(target)?;let p=self.position()?;let mut centre=self.centre()?;
            // Both COM and position are flattened before subtraction; the
            // saved COM used by the final prediction remains flattened too.
            centre[1]=0.;
            let a=try_normalize_safe([centre[0]-p[0],0.,centre[2]-p[2],0.],[0.;4],&mut HostReckoningNumeric).map_err(|e|format!("spectate normalize: {e:?}"))?;
            let b=try_normalize_safe([target[0]-p[0],0.,target[2]-p[2],0.],[0.;4],&mut HostReckoningNumeric).map_err(|e|format!("spectate normalize: {e:?}"))?;
            if HostReckoningNumeric.dot3(a,b).map_err(|e|format!("spectate dot: {e:?}"))?<self.bindings.angle{
                let velocity=self.frame.target_velocity(self.entity,local)?.ok_or("spectate Actor velocity absent")?;
                self.instances[i]=Some(Instance::Watch(std::array::from_fn(|n|velocity[n].mul_add(self.bindings.predict,centre[n]))));
            }Ok(())
        },
        _=>Ok(()), // all other generated Update callbacks are folded empty.
    }}
    fn cheer(&mut self,i:usize)->Result<(),String>{
        let Some(Instance::Cheer{delay,duration,kind,flags})=self.instances[i].as_ref() else{return Err("spectate cheer instance absent".into());};
        let (mut delay,mut duration,mut kind,mut flags)=(*delay,*duration,*kind,*flags);
        // Restore the same occurrence payload even when an external callback fails.
        let result=(||{
            self.entity.state_out.state=1;
            if flags&0x20!=0{delay-=self.dt;if !(delay>0.){self.frame.request_channel(self.entity,"SpectateCheerPositive")?;
                flags=(flags&0x5f)|0x80;self.set_timer(10,duration)?;return Ok(());}}
            if flags&0x80!=0{
                let done=if flags&0x40!=0{self.entity.state_out.state=23;
                    self.entity.animation.as_ref().ok_or("spectate animation absent")?.channels.channel.status==crate::pedestrian_motion_delegate::channel::Status::Finished
                }else if self.timer(10)?>0.{self.entity.state_out.state=24;false}else{true};
                if done{flags&=!0x80;self.set_timer(23,self.bindings.wait)?;}
            }
            self.entity.navigation_mut()?.seek_speed=0.;self.entity.navigation_rig_enabled=false;
            let event=self.entity.perception.last_spectate_event;
            if event.active&&!(self.timer(23)?>0.){
                let mut short_time=0.;let mut positive=false;let mut negative=false;kind=1;
                if event.wiped_out{negative=true;self.set_timer(9,self.bindings.boredom)?;kind=0;self.entity.state_out.state=25;}
                else if (event.score as i32)>self.bindings.short{positive=true;self.set_timer(9,self.bindings.boredom)?;
                    if (event.score as i32)>self.bindings.long{kind=3;}else{kind=2;
                        let random=self.random()?;short_time=(self.bindings.max_short-self.bindings.min_short)*random+self.bindings.min_short;}}
                flags=(flags&0xe7)|(u8::from(positive)<<3)|(u8::from(negative)<<4);
                if flags&0x80!=0{if kind==0{self.entity.state_out.state=25;self.set_timer(10,0.)?;}}
                else if kind!=1&&flags&0x20==0{duration=short_time;flags=(flags&!0x40)|(u8::from(kind==3)<<6);
                    let random=self.random()?;let max=self.frame.cvars.try_spectate_max_delay().map_err(|e|format!("spectate tuning: {e:?}"))?;
                    flags|=0x20;delay=max*random;}
            }else{flags&=0xe7;}
            Ok(())
        })();self.instances[i]=Some(Instance::Cheer{delay,duration,kind,flags});result
    }
    fn random(&mut self)->Result<f32,String>{self.frame.random.try_living_world_range(&crate::native_skater_random::NativeRandomGate::production_shared(),0.,1.).map_err(|e|format!("spectate random: {e:?}"))}
    /// PlayerRecognition.Begin82717410 remaps even if the channel is busy;
    /// only the actual PlayChannelAnim request is conditional on Finished.
    fn recognize(&mut self)->Result<(),String>{
        self.entity.navigation_rig_enabled=false;
        self.entity.state_out.state=if (self.random()?*2.) as i64==0{45}else{42};
        let animation=self.entity.animation.as_mut().ok_or("recognition animation absent")?;
        let random=&mut self.frame.random;let mut failure=None;
        let mapped=crate::pedestrian_animation::remap_animation("GreetChannel",&animation.motion_intents.registered_clips,
            animation.selected_animation_data()?,||match random.try_next(&crate::native_skater_random::NativeRandomGate::production_shared()){
                Ok(word)=>word,Err(e)=>{failure=Some(e);0}
            }).map_err(|e|e.to_string())?;
        if let Some(e)=failure{return Err(format!("recognition RNG: {e:?}"));}
        if animation.channels.channel.status==crate::pedestrian_motion_delegate::channel::Status::Finished{
            let name=crate::play_animation::FastString36::encode(&mapped).map_err(|e|format!("recognition channel: {e:?}"))?;
            animation.channels.play(name,0.2,0.2,false)?;
        }
        self.push()
    }
    fn facing_skater(&self)->Result<bool,String>{
        let centre=self.centre()?;let p=self.position()?;
        let direction=try_normalize_safe([centre[0]-p[0],0.,centre[2]-p[2],0.],[0.;4],&mut HostReckoningNumeric).map_err(|e|format!("spectate facing: {e:?}"))?;
        let facing=self.entity.core.as_ref().ok_or("spectate core absent")?.transform[2];
        let cos=crate::reckoning_sin_cos::try_cosine(0.8,&mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric).unwrap_or_else(|never|match never{});
        Ok(HostReckoningNumeric.dot3(direction,facing).map_err(|e|format!("spectate facing: {e:?}"))?>cos)
    }
    fn actor_condition(&mut self,index:u16)->Result<bool,String>{Ok(match index{
        0|2=>match self.frame.props.recognizer_count()?{None=>false,Some(count)=>{
            let exceeded=count>if index==0{4}else{5};
            self.entity.can_recognize_player()?&&exceeded
        }},
        1=>crate::pedestrian_graph::conditions::entity_condition("IsOnRoad",self.entity)?.ok_or("recognition road condition absent")?,
        3=>self.facing_skater()?,
        4=>!self.entity.animation.as_ref().ok_or("recognition animation absent")?.ai_input.locomotion_transition_complete,
        5=>!self.entity.animation.as_ref().ok_or("recognition animation absent")?.ai_input.locomotion_turn_complete,
        6=>{let chance=20_f32*0.01_f32;let random=self.frame.simulation_random.as_deref_mut().ok_or("SimpleRandom requires actual Simulation random")?
            .try_range(0.,1.).map_err(|e|format!("SimpleRandom: {e:?}"))?;chance>=random},
        7=>self.timer(9)?<=0.,
        8=>{let local=self.local()?;if self.frame.actor(local)?.is_none(){false}else{
            let centre=self.centre()?;let p=self.entity.navigation_mut()?.position;
            let delta=std::array::from_fn(|n|centre[n]-p[n]);
            try_length(delta,&mut HostReckoningNumeric).map_err(|e|format!("skater radius: {e:?}"))?<25.
        }},
        _=>return Err("actortracker condition index absent".into()),
    })}
    fn evaluate(&mut self,index:u16)->Result<bool,String>{
        if self.bindings.family==Family::ActortrackerSpectate{return self.actor_condition(index);}
        Ok(match index{
        0=>if let Some(reference)=self.entity.current_waypoint{let point=self.frame.props.point(reference)?.location;let p=self.position()?;
            try_length([point[0]-p[0],0.,point[2]-p[2],0.],&mut HostReckoningNumeric).map_err(|e|format!("waypoint distance: {e:?}"))?<3.
        }else{false},
        1=>if let Some(reference)=self.entity.current_waypoint{self.frame.props.point(reference)?.locking_entity.is_some()}else{false},
        2=>self.facing_skater()?,
        3|4=>crate::pedestrian_graph::conditions::entity_condition(if index==3{"NeedsToBeginColliding"}else{"IsPedestrianColliding"},self.entity)?.ok_or("spectate condition binding absent")?,
        _=>return Err("spectate condition index absent".into()),
    })}
}
impl PedestrianGraphBackend for Backend<'_,'_>{
    fn condition(&mut self,_:&PedestrianGraph,_:Mask,index:u16,_:u16)->Result<bool,Error>{self.evaluate(index).map_err(|e|{self.failure=Some(e);Error::PendingExpression(index)})}
    fn enabled(&self,index:u16)->Result<bool,Error>{if (index as usize)<self.bindings.leaves.len(){Ok(true)}else{Err(Error::MissingInstance(index))}}
    fn create_instance(&mut self,_:&PedestrianGraph,index:u16)->Result<(),Error>{
        let leaf=self.bindings.leaves.get(index as usize).ok_or(Error::MissingInstance(index))?;
        self.instances[index as usize]=match leaf{Leaf::Switch(_)=>Some(Instance::Switch(SavedSwitchInstance::new())),
            Leaf::Collision(_)=>Some(Instance::Collision(CollidingInstance::new())),
            Leaf::Spectate=>Some(Instance::Cheer{delay:0.,duration:0.,kind:1,flags:0}),Leaf::Watch=>Some(Instance::Watch([0.;4])),_=>None};Ok(())
    }
    fn begin(&mut self,_:&PedestrianGraph,index:u16)->Result<(),Error>{self.begin_leaf(index as usize).map_err(|e|{self.failure=Some(e);Error::MissingInstance(index)})}
    fn end_and_remove(&mut self,_:&PedestrianGraph,index:u16)->Result<(),Error>{self.end_leaf(index as usize).map_err(|e|{self.failure=Some(e);Error::MissingInstance(index)})}
    fn transition_hook(&mut self,_:&PedestrianGraph,index:u16,_:u16)->Result<(),Error>{Err(Error::PendingTransitionHook(index))}
    fn generated_update(&mut self,_:&PedestrianGraph,address:u32,active:&[u16])->Result<(),Error>{
        let expected:&[u16]=match address{0x825187c0=>&[],0x828f80d8=>&[0,1,2],0x828f8068=>&[3,4,5],0x828f8148=>&[0,1,2,7],0x828f81d0=>&[6],
            0x828f84c0|0x828f8558=>&[0,1,2,3,4],0x828f8560=>&[0,1,2,3,4,5],
            0x828f85b0=>&[0,1,2,3,4,6],0x828f8600=>&[0,1,2,3,4,7,8],0x828f8660=>&[0,1,2,3,4,9,10],
            0x828f86c0=>&[0,1,2,3,4,6,11],0x828f8720=>&[0,1,2,3,4,6,12,13],_=>return Err(Error::PendingStateUpdate(address))};
        if active!=expected{return Err(Error::PendingStateUpdate(address));}
        for &i in active{self.update_leaf(i as usize).map_err(|e|{self.failure=Some(e);Error::PendingStateUpdate(address)})?;}Ok(())
    }
}
