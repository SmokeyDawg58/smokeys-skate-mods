//! Pedestrian.RemoveHandProp82e70118 and DynamicObject.UnsetAsHandProp82d2ad88.
use super::PedestrianConstruction;
use crate::{physical_player_assembly::PhysicalAssemblyWorld,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::dmo_objects::{DmoObjectMan, DynamicObjectTransformType},
    simulation_assembly_backend::constraint_session::physical_island::vehicle::LivingWorldBodyKind};

/// Native tLWHandPropUsage. MemStore.Init82a6de38 and FreeBlock82a6df68
/// zero its POD fields; constructor82e97cd8..cfc clears both RefSpecs.
#[derive(Default)]
pub(crate) struct HandPropUsage {
    pub(crate) hand_prop_class: u64,
    pub(crate) hand_prop_collection: u64,
    pub(crate) drop_on_collision: f32,
    pub(crate) drop_on_large_avoid: f32,
    pub(crate) usage_class: u64,
    pub(crate) usage_collection: u64,
}

impl PedestrianConstruction {
    /// IPedestrian.MonitoredObjectMoved82e91fe0. This is the interacting
    /// plugin object, not the held hand prop. Large motion requests deferred
    /// plugin removal and may suggest the existing jump-avoid packet.
    pub(crate) fn monitored_object_moved(&mut self,object:u32,delta:[f32;3],
        cvars:&InputCvarReferences)->Result<(),String>{
        if self.interacting_object!=Some(object){return Ok(());}
        use crate::reckoning_numeric::{HostReckoningNumeric,ReckoningNumeric};
        let vector=[delta[0],delta[1],delta[2],0.];
        let square=HostReckoningNumeric.dot3(vector,vector).map_err(|e|format!("DMO movement length: {e:?}"))?;
        let tolerance=cvars.try_plugin_dmo_shift_tolerance().map_err(|e|format!("DMO movement tuning: {e:?}"))?;
        // PPC BLE is !GT, including unordered comparisons.
        if !(square-tolerance*tolerance>0.) {
            let state=self.state.as_mut().ok_or("DMO movement AI state absent")?;
            let shift=state.dmo_shift.as_mut().ok_or("DMO movement accumulator absent")?;
            state.flags[3]|=1;
            for i in 0..3 {shift[i]+=delta[i];}
        } else {
            self.plugin_removal_requested=true;
            if self.state.as_ref().ok_or("DMO movement AI state absent")?.flags[1]&1!=0 {
                let queue=self.intent_queue.as_mut().ok_or("DMO movement intent owner absent")?;
                // IntentGen+1416 is mAvoidJumpPacket.InQueue, not IsJumping.
                if !queue.packet(queue.avoid_jump)?.in_queue {
                    let packet=crate::pedestrian_navigation_modifiers::skater_avoidance::create_avoid_packet(queue,true,"Forward")?;
                    queue.suggest(packet,&mut self.animation.as_mut().ok_or("DMO movement animation absent")?.ai_input)?;
                }
            }
        }
        Ok(())
    }
    /// SetHandProp82e7b418 replaces the previous DMO through its real cull
    /// callbacks, then registers this pedestrian as one of its four monitors.
    pub(crate) fn set_hand_prop(&mut self,id:u32,world:&mut PhysicalAssemblyWorld,
        objects:&mut DmoObjectMan,
        locators:&mut crate::living_world_grind_services::dmo_locators::DynamicObjectLocatorMan,
        cvars:&InputCvarReferences,peers:&mut[(u32,&mut Self)],
    )->Result<(),String>{
        if let Some(old)=self.held_hand_prop {
            self.remove_hand_prop(world,objects,cvars)?;
            objects.try_request_explicit_cull_with_monitors(old,locators,
                &mut crate::living_world_grind_services::dmo_objects::FrameLocatorServices,
                |objects,monitor,id|self.notify_monitored_object_for_peers(peers,monitor,id,world,objects,cvars))
                .map_err(|e|format!("replaced hand prop cull: {e:?}"))?;
        }
        self.held_hand_prop=Some(id);
        self.state.as_mut().ok_or("hand prop state absent")?.flags[2]|=2;
        self.collision.as_mut().ok_or("hand prop collision absent")?.set_world_collision_group(world,12)?;
        let object=objects.object_mut(id).ok_or("new hand prop DMO absent")?;
        object.transform_type=DynamicObjectTransformType::HandProp;
        object.collision_ignore_gravity=true;
        world.try_set_living_world_collision_group(LivingWorldBodyKind::Dmo,&object.body,15)
            .map_err(|e|format!("hand prop collision group: {e:?}"))?;
        object.group=15;
        if object.monitoring_entities.len()<4 {object.monitoring_entities.push(self.id);}
        Ok(())
    }

    /// SpawnAndSetHandProp82e9cbd4..cd08. Native matches collection LOW32
    /// and only stops at a matching row with nonempty usage characteristics.
    pub(crate) fn initialize_hand_prop_usage(&mut self,key:u64,
        vault:&std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        random:&mut crate::native_skater_random::NativeSkaterRandom,
    )->Result<(),String>{
        self.state.as_mut().ok_or("hand prop state absent")?.flags[3]&=!0x20;
        let attributes=self.resources.as_ref().ok_or("hand prop entity attributes absent")?.entity.clone();
        const KEY:u64=0x864a_7084_ca22_0523;
        const TYPE:u64=0xd677_8895_b505_d1db;
        let field=|index,offset,size|->Result<Vec<u8>,String>{
            Ok(attributes.try_array_field_data(KEY,index,TYPE,56,offset..offset+size)
                .map_err(|e|e.to_string())?.map_or(vec![0;size],|d|d.bytes().to_vec()))
        };
        let word=|index,offset|->Result<u64,String>{Ok(u64::from_be_bytes(field(index,offset,8)?.try_into().expect("eight bytes")))};
        let scalar=|index,offset|->Result<f32,String>{Ok(f32::from_be_bytes(field(index,offset,4)?.try_into().expect("four bytes")))};
        for index in 0..attributes.try_length(KEY).map_err(|e|e.to_string())? {
            if self.state.as_ref().expect("checked state").flags[3]&0x20!=0 {break;}
            if word(index,8)? as u32!=key as u32 {continue;}
            self.hand_prop_usage=HandPropUsage{
                hand_prop_class:word(index,0)?,hand_prop_collection:word(index,8)?,
                drop_on_collision:scalar(index,24)?,drop_on_large_avoid:scalar(index,28)?,
                usage_class:word(index,32)?,usage_collection:word(index,40)?,
            };
            let usage=vault.instance(0x40c4_819c_bd5a_014b,self.hand_prop_usage.usage_collection);
            if usage.try_length(0xe4e9_c4e5_1498_1944).map_err(|e|e.to_string())?!=0 {
                self.state.as_mut().expect("checked state").flags[3]|=0x20;
            }
            self.choose_new_hand_prop_action(vault,random)?;
        }
        Ok(())
    }

    /// IsHandPropDisposable826fd500 and CanSitWithHandProp826fd558 read the
    /// authored layout bytes only when HasHandProp (including pending spawn).
    pub(crate) fn is_hand_prop_disposable(&self)->Result<bool,String>{
        if !self.has_hand_prop()? {return Ok(false);}
        self.hand_prop_layout_flag(60)
    }
    pub(crate) fn can_sit_with_hand_prop(&self)->Result<bool,String>{
        if !self.has_hand_prop()? {return Ok(true);}
        self.hand_prop_layout_flag(61)
    }
    fn hand_prop_layout_flag(&self,offset:usize)->Result<bool,String>{
        let attributes=self.hand_prop_attributes.as_ref().ok_or("hand prop layout absent")?;
        // Native826fd528/826fd580 load just one byte. The preceding
        // layout+56 is a relocated channel-name pointer, not POD payload.
        let data=attributes.try_cached_layout_field(offset,1).map_err(|e|e.to_string())?;
        Ok(data.as_ref().map_or(0,|d|d.bytes()[0])!=0)
    }

    /// Complete UpdateHandProp82e9d320 branch ordering. Explicit cull queues
    /// the existing DMO owner; removing a hand prop does not destroy its body.
    pub(crate) fn update_hand_prop(&mut self,world:&mut PhysicalAssemblyWorld,
        objects:&mut DmoObjectMan,
        locators:&mut crate::living_world_grind_services::dmo_locators::DynamicObjectLocatorMan,
        vault:&std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        cvars:&InputCvarReferences,random:&mut crate::native_skater_random::NativeSkaterRandom,
        peers:&mut [(u32,&mut Self)],
    )->Result<(),String> {
        let Some(id)=self.held_hand_prop else{return Ok(());};
        let state=self.state.as_ref().ok_or("hand prop state absent")?;
        if state.flags[2]&2==0 {return self.update_released_hand_prop(world,objects,cvars);}
        if state.flags[2]&1!=0 {
            self.update_hand_prop_throw(world,objects,cvars)?;
        }else if state.flags[4]&0x80!=0 {
            match state.tazer_state {
                0=>return Err("native UpdateHandProp tazer-state assertion".into()),
                1=>if self.hand_prop_channel_time()?<=0.2 {
                    self.request_channel_cycle("TazerCyc",true,random)?;
                    self.state.as_mut().expect("checked state").tazer_state=2;
                },
                3=>if self.hand_prop_channel_time()?<=0.2 {
                    let state=self.state.as_mut().expect("checked state");
                    state.tazer_state=0;
                    state.hand_prop_action=-1;
                    state.flags[4]&=!0x80;
                    state.flags[1]&=!0x20;
                    self.remove_hand_prop(world,objects,cvars)?;
                    objects.try_request_explicit_cull_with_monitors(id,locators,
                        &mut crate::living_world_grind_services::dmo_objects::FrameLocatorServices,
                        |objects,monitor,id|self.notify_monitored_object_for_peers(peers,monitor,id,world,objects,cvars))
                        .map_err(|e|format!("tazer cull: {e:?}"))?;
                },
                _=>{},
            }
        }else if state.flags[3]&0x20!=0 {
            self.update_hand_prop_action(vault,cvars,random)?;
        }
        // Native performs this even when the held branch released the object.
        if self.animation.as_ref().ok_or("hand prop animation absent")?.channels.channel.status
            ==crate::pedestrian_motion_delegate::channel::Status::Finished {
            self.request_hand_prop_idle(random)?;
        }
        Ok(())
    }

    /// ChooseNewHandPropAction82e9bed8. Raw insrdi operands establish the full
    /// class/attribute keys; original skater.vlt defines the 16-byte POD type.
    pub(crate) fn choose_new_hand_prop_action(&mut self,
        vault: &std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        random: &mut crate::native_skater_random::NativeSkaterRandom,
    ) -> Result<(), String> {
        if self.state.as_ref().ok_or("hand prop state absent")?.flags[3]&0x20==0 { return Ok(()); }
        let attributes=vault.instance(0x40c4_819c_bd5a_014b,self.hand_prop_usage.usage_collection);
        const KEY:u64=0xe4e9_c4e5_1498_1944;
        const TYPE:u64=0xe7c8_8694_4418_f681;
        let word=|index,offset| -> Result<[u8;4],String> {
            Ok(attributes.try_array_field_data(KEY,index,TYPE,16,offset..offset+4)
                .map_err(|e|e.to_string())?.map_or([0;4],|d|d.bytes().try_into().expect("four-byte field")))
        };
        let count=attributes.try_length(KEY).map_err(|e|e.to_string())?.min(16);
        let mut probabilities=Vec::with_capacity(count as usize);
        for index in 0..count { probabilities.push(f32::from_be_bytes(word(index,4)?)); }
        let draw=crate::native_skater_random::rw_math::global_next().map_err(|e|format!("hand prop discrete RNG: {e:?}"))?;
        let Some(index)=crate::living_world_grind_services::census::targets::discrete_choice(&probabilities,draw) else {
            self.state.as_mut().expect("checked state").flags[3]&=!0x20;
            return Ok(());
        };
        let index=index as u32;
        let maximum=f32::from_be_bytes(word(index,12)?);
        let minimum=f32::from_be_bytes(word(index,8)?);
        let delay=random.try_living_world_range(&crate::native_skater_random::NativeRandomGate::production_shared(),minimum,maximum)
            .map_err(|e|e.to_string())?;
        let action=i32::from_be_bytes(word(index,0)?);
        let state=self.state.as_mut().expect("checked state");
        state.hand_prop_action=action;
        state.set_timer(32,delay).map_err(|e|e.to_string())
    }

    fn hand_prop_channel_time(&self)->Result<f32,String> {
        let name=crate::play_animation::FastString36::encode("PedChannel").expect("native channel");
        self.animation.as_ref().ok_or("hand prop animation absent")?.core.time_left_in_channel(&name)
            .map_err(|e|format!("hand prop channel time: {e:?}"))
    }

    pub(crate) fn request_hand_prop_idle(&mut self,random:&mut crate::native_skater_random::NativeSkaterRandom)->Result<(),String> {
        let text=self.hand_prop_attributes.as_ref().ok_or("hand prop attributes absent")?
            .try_cached_layout_text(56).map_err(|e|e.to_string())?.ok_or("hand prop idle name absent")?;
        let name=std::str::from_utf8(text.bytes()).map_err(|e|e.to_string())?.to_owned();
        self.request_channel_cycle(&name,true,random)
    }

    /// UpdateHandPropAction82e9ce28. The action and phone/read cycle timers
    /// are separate; the plugin suppression flag delays starting by 15 seconds.
    pub(crate) fn update_hand_prop_action(&mut self,
        vault:&std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        cvars:&InputCvarReferences,random:&mut crate::native_skater_random::NativeSkaterRandom,
    )->Result<(),String> {
        let state=self.state.as_ref().ok_or("hand prop state absent")?;
        if !(state.timer(32)<=0.) {return Ok(());}
        let flags=state.flags[3];
        let action=state.hand_prop_action;
        if flags&8==0 {
            if flags&0x10!=0 {
                return self.state.as_mut().expect("checked state").set_timer(32,15.).map_err(|e|e.to_string());
            }
            let started=self.start_hand_prop_action(action,cvars,random)?;
            let state=self.state.as_mut().expect("checked state");
            state.flags[3]=(state.flags[3]&!8)|if started{8}else{0};
            return Ok(());
        }
        if !matches!(action,3|4) {
            if self.hand_prop_channel_time()?<=0.2 {
                self.request_hand_prop_idle(random)?;
                self.state.as_mut().expect("checked state").flags[3]&=!8;
                self.choose_new_hand_prop_action(vault,random)?;
            }
            return Ok(());
        }
        if flags&4==0 {
            if self.hand_prop_channel_time()?>0.2 {return Ok(());}
            let state=self.state.as_mut().expect("checked state");
            state.flags[3]|=4;
            return state.set_timer(33,if action==3{6.}else{2.}).map_err(|e|e.to_string());
        }
        let continuing=state.timer(33)>0.;
        self.request_channel_cycle(match(action,continuing){
            (3,true)=>"HandPropPhoneTalk",(3,false)=>"HandPropPhoneHangup",
            (_,true)=>"HandPropReadCyc",(_,false)=>"HandPropReadOut",
        },continuing,random)?;
        if !continuing {self.state.as_mut().expect("checked state").hand_prop_action=-1;}
        if action==3 {self.state_out.state=if continuing{48}else{50};}
        Ok(())
    }

    ///Shared short PlayChannelAnim82e6e830 ->82e91e08. The frame graph and
    ///hand-prop/plugin callbacks all use the same RemapAnim/channel owner.
    pub(crate) fn request_channel(&mut self,name:&str,random:&mut crate::native_skater_random::NativeSkaterRandom)->Result<(),String>{
        self.request_channel_cycle(name,false,random)
    }
    /// The full PlayChannelAnim82e91e08 with its cycle flag: UpdateHandProp82e9d6f8/
    /// UpdateHandPropAction82e9cf40 pass r8=1 for the prop's idle, as for HandPropReadCyc,
    /// HandPropPhoneTalk and TazerCyc, so those loop instead of finishing and restarting.
    pub(crate) fn request_channel_cycle(&mut self,name:&str,cycle:bool,
        random:&mut crate::native_skater_random::NativeSkaterRandom)->Result<(),String>{
        let animation=self.animation.as_mut().ok_or("channel animation owner absent")?;
        let mut failure=None;
        let mapped=crate::pedestrian_animation::remap_animation(name,&animation.motion_intents.registered_clips,
            animation.selected_animation_data()?,||match random.try_next(&crate::native_skater_random::NativeRandomGate::production_shared()){
                Ok(value)=>value,Err(error)=>{failure=Some(error);0}
            }).map_err(|e|e.to_string())?;
        if let Some(error)=failure{return Err(format!("channel Manager RNG: {error:?}"));}
        let name=crate::play_animation::FastString36::encode(&mapped).map_err(|e|format!("channel remapped name: {e:?}"))?;
        animation.channels.transition(name,0.2,0.2,0.2,cycle)
    }
    /// HasHandProp82e70040 includes the pending-spawn bit, while asserting that
    /// the real-held bit has its actual DMO owner.
    pub(crate) fn has_hand_prop(&self) -> Result<bool, String> {
        let state = self.state.as_ref().ok_or("hand prop AI state absent")?;
        if state.flags[2] & 2 != 0 && self.held_hand_prop.is_none() {
            return Err("native HasHandProp mpHandProp assertion".into());
        }
        Ok(state.flags[2] & 2 != 0 || state.flags[3] & 2 != 0)
    }
    /// ThrowHandPropAway82e9b528 schedules the release; UpdateHandProp consumes
    /// the flags and timer later. No immediate physics detach occurs here.
    pub(crate) fn throw_hand_prop_away(&mut self, cvars: &InputCvarReferences) -> Result<(), String> {
        let state = self.state.as_mut().ok_or("hand prop AI state absent")?;
        if state.flags[2] & 2 != 0 && state.flags[2] & 1 == 0 {
            state.flags[2] |= 1;
            state.flags[3] &= !0x40;
            state.flags[3] &= !0x80;
            state.set_timer(34,cvars.try_hand_prop_throw_timeout().map_err(|e| format!("throw timeout: {e:?}"))?)
                .map_err(|e| format!("hand prop timer: {e:?}"))?;
        }
        Ok(())
    }
    ///ThrowHandPropAtPosition82e9b5a0 retains the directed release request and
    ///starts the original directional channel animation; physics detach remains
    ///the existing UpdateHandProp consumer's responsibility.
    pub(crate) fn throw_hand_prop_at_position<N:crate::reckoning_numeric::ReckoningNumeric>(
        &mut self,position:[f32;4],attack:bool,cvars:&InputCvarReferences,
        random:&mut crate::native_skater_random::NativeSkaterRandom,numeric:&mut N
    )->Result<(),String> where N::Error:std::fmt::Debug {
        let state=self.state.as_mut().ok_or("directed throw AI state absent")?;
        if state.flags[2]&2==0{return Ok(());}
        state.flags[2]|=1;
        state.flags[3]|=0x40;
        state.hand_prop_throw_target=Some(position);
        state.flags[3]|=0x80;
        let centre=self.perception_centre()?;
        let delta=[position[0]-centre[0],0.,position[2]-centre[2],0.];
        let facing=self.core.as_ref().ok_or("directed throw facing absent")?.transform[2];
        let angle=crate::reckoning_numeric::try_signed_angle([facing[0],0.,facing[2],0.],delta,[0.,1.,0.,0.],numeric)
            .map_err(|e|format!("directed throw angle: {e:?}"))?;
        let cv=|e|format!("directed throw tuning: {e:?}");
        let timeout=if attack{cvars.try_hand_prop_attack_throw_timeout()}else{cvars.try_hand_prop_dispose_throw_timeout()}.map_err(cv)?;
        self.state.as_mut().expect("checked state").hand_prop_release_offset=timeout;
        let force=if attack{cvars.try_hand_prop_attack_throw_force()}else{cvars.try_hand_prop_dispose_force()}.map_err(cv)?;
        self.state.as_mut().expect("checked state").hand_prop_throw_velocity=Some(force);
        let timeout=if attack{cvars.try_hand_prop_attack_throw_timeout()}else{cvars.try_hand_prop_dispose_throw_timeout()}.map_err(cv)?;
        self.state.as_mut().expect("checked state").set_timer(34,timeout).map_err(|e|e.to_string())?;
        let name=if attack {
            if angle<cvars.try_hand_prop_attack_forward_angle().map_err(cv)?
                ||angle>6.2831855-cvars.try_hand_prop_attack_forward_angle().map_err(cv)? {"HandPropAttackThrow"}
            else if angle<3.1415927-cvars.try_hand_prop_attack_forward_angle().map_err(cv)? {"HandPropAttackThrowLeft"}
            else if angle>=cvars.try_hand_prop_attack_forward_angle().map_err(cv)?+3.1415927 {"HandPropAttackThrowRight"}
            else {"HandPropAttackThrow"}
        }else {
            if angle<0.78539819-cvars.try_hand_prop_throw_angle_tolerance().map_err(cv)?
                ||angle>cvars.try_hand_prop_throw_angle_tolerance().map_err(cv)?+5.4977875 {"HandPropThrowLightForward"}
            else if angle>=cvars.try_hand_prop_throw_angle_tolerance().map_err(cv)?+0.78539819 {
                if angle>=cvars.try_hand_prop_throw_angle_tolerance().map_err(cv)?+1.5707964 {
                    if angle>=4.712389-cvars.try_hand_prop_throw_angle_tolerance().map_err(cv)? {
                        if angle>=cvars.try_hand_prop_throw_angle_tolerance().map_err(cv)?+4.712389 {"HandPropThrowLightR45"}
                        else {"HandPropThrowLightR90"}
                    }else {"HandPropThrowLightR180"}
                }else {"HandPropThrowLightL90"}
            }else {"HandPropThrowLightL45"}
        };
        self.request_channel(name,random)
    }
    /// StartHandPropAction82e9bc28. Action numbers are the original PDB
    /// eHandPropActions values; audio writes use the existing StateOut owner.
    pub(crate) fn start_hand_prop_action(&mut self, action: i32,
        cvars: &InputCvarReferences, random: &mut crate::native_skater_random::NativeSkaterRandom,
    ) -> Result<bool, String> {
        let name = match action {
            0 => return Ok(false), //DoNothing leaves all fields alone.
            1 => "HandPropEat",
            2 => "HandPropDrink",
            3 => "HandPropPhoneAnswer",
            4 => "HandPropReadInto",
            5 => {
                let state = self.state.as_mut().ok_or("hand prop action state absent")?;
                state.flags[2] |= 1;
                state.flags[3] &= !0x40;
                let timeout = cvars.try_hand_prop_dispose_throw_timeout()
                    .map_err(|e|format!("hand prop litter timeout: {e:?}"))?;
                state.set_timer(34,timeout).map_err(|e|e.to_string())?;
                "HandPropLitter"
            }
            _ => {
                self.state.as_mut().ok_or("hand prop action state absent")?.flags[3] &= !0x20;
                return Ok(false);
            }
        };
        self.request_channel(name,random)?;
        if action == 3 { self.state_out.state = 49; } //PedestrianCellHello.
        if action == 5 { self.state_out.state = 67; } //PedstrianLitter.
        let state = self.state.as_mut().ok_or("hand prop action state absent")?;
        state.hand_prop_action = action;
        if matches!(action,3|4) { state.flags[3] &= !4; }
        Ok(true)
    }

    /// Pedestrian.ReleaseHandPropToPhysics82e700d0 -> DMO82d2ae28.
    /// Keep the held-object pointer and monitor until RemoveHandProp later.
    /// Raw DMO store+1080 is transform_type at complete-object+110c,
    /// NOT the monitor vector named by the decompiler's biased this pointer.
    pub(crate) fn release_hand_prop_to_physics(&mut self,
        world: &mut PhysicalAssemblyWorld, objects: &mut DmoObjectMan,
        cvars: &InputCvarReferences, apply_velocity: bool, velocity: [f32; 4],
    ) -> Result<(), String> {
        let Some(id) = self.held_hand_prop else { return Ok(()); };
        let state = self.state.as_mut().ok_or("hand prop release state absent")?;
        state.flags[2] &= !2;
        state.flags[2] |= 1;
        let object = objects.object_mut(id).ok_or("released hand prop DMO absent")?;
        object.transform_type = DynamicObjectTransformType::Physics;
        object.collision_ignore_gravity = false;
        object.hand_prop_cull_timer = cvars.try_dmo_hand_prop_cull_timer()
            .map_err(|e| format!("released hand prop cull timer: {e:?}"))?;
        if apply_velocity {
            world.try_set_dmo_part_velocity(&object.body, 0, [velocity[0], velocity[1], velocity[2]])?;
        }
        Ok(())
    }

    /// UpdateHandProp82e9d344..564 throw branch. The caller preserves the
    /// later channel-finished handling and other held-object action branches.
    pub(crate) fn update_hand_prop_throw(&mut self,
        world: &mut PhysicalAssemblyWorld, objects: &mut DmoObjectMan,
        cvars: &InputCvarReferences,
    ) -> Result<(), String> {
        let Some(id) = self.held_hand_prop else { return Ok(()); };
        let state = self.state.as_ref().ok_or("hand prop throw state absent")?;
        if state.flags[2] & 3 != 3 { return Ok(()); }
        let (remaining, threshold) = if state.flags[3] & 0x80 != 0 {
            let name = crate::play_animation::FastString36::encode("PedChannel")
                .expect("native pedestrian channel name");
            (self.animation.as_ref().ok_or("hand prop channel owner absent")?
                .core.time_left_in_channel(&name).map_err(|e|format!("hand prop channel time: {e:?}"))?,
                state.hand_prop_release_offset)
        } else { (state.timer(34), 0.) };
        if !(remaining <= threshold) { return Ok(()); }
        let object = objects.object(id).ok_or("hand prop throw DMO absent")?;
        let velocity = if state.flags[3] & 0x40 != 0 {
            let target = state.hand_prop_throw_target.ok_or("directed hand prop target unwritten")?;
            let speed = state.hand_prop_throw_velocity.ok_or("directed hand prop speed unwritten")?;
            let initial = std::array::from_fn(|i| object.transform[12+i]);
            let mut velocity = [0.;4];
            let mut time = -1.;
            crate::pedestrian_adjust_owners::jump_trajectory::solve_for_initial_velocity(
                initial, target, speed, &mut velocity, &mut time,
                &mut crate::reckoning_numeric::HostReckoningNumeric)?;
            self.state.as_mut().expect("checked hand prop state").set_timer(35,time)
                .map_err(|e|e.to_string())?;
            velocity
        } else {
            // Entity.GetVelocity, not facing: preserve the original inherited
            // motion multiplier even when the object is nearly stationary.
            let force = cvars.try_hand_prop_throw_force().map_err(|e|format!("hand prop force: {e:?}"))?;
            object.velocity.map(|lane|lane*force)
        };
        self.release_hand_prop_to_physics(world,objects,cvars,true,velocity)
    }

    /// UpdateHandProp82e9d704..860: released prop remains monitored until an
    /// absolute coordinate difference exceeds its lazy, per-axis threshold.
    pub(crate) fn update_released_hand_prop(&mut self,
        world: &mut PhysicalAssemblyWorld, objects: &mut DmoObjectMan,
        cvars: &InputCvarReferences,
    ) -> Result<(), String> {
        let Some(id)=self.held_hand_prop else{return Ok(());};
        if self.state.as_ref().ok_or("released prop state absent")?.flags[2]&2!=0{return Ok(());}
        let object=objects.object(id).ok_or("released prop DMO absent")?;
        let position=self.core.as_ref().ok_or("released prop pedestrian core absent")?.transform[3];
        let delta: [f32;3]=std::array::from_fn(|i|(object.transform[12+i]-position[i]).abs());
        let error=|e|format!("hand prop disown tuning: {e:?}");
        if delta[0]>cvars.try_hand_prop_disown_distance_x().map_err(error)?
            ||delta[1]>cvars.try_hand_prop_disown_distance_y().map_err(error)?
            ||delta[2]>cvars.try_hand_prop_disown_distance_z().map_err(error)? {
            self.remove_hand_prop(world,objects,cvars)?;
        }
        Ok(())
    }

    /// MonitoredObjectBeingCulled82e7ae88, hand-prop match before interaction.
    pub(crate) fn notify_monitored_object_for_peers<C>(&mut self,peers:&mut [(u32,&mut Self)],monitor:u32,id:u32,
        world:&mut crate::simulation_assembly_backend::constraint_session::SimulationAssemblyConstraintSession<skate2_formats::collision::AssemblyPartMaterial,C>,
        objects:&mut DmoObjectMan,cvars:&InputCvarReferences)->Result<(),String>{
        if monitor==self.id {self.monitored_object_being_culled(id,world,objects,cvars)}
        else if let Some((_,entity))=peers.iter_mut().find(|(id,_)|*id==monitor){entity.monitored_object_being_culled(id,world,objects,cvars)}
        else {Ok(())}
    }
    /// MonitoredObjectBeingCulled82e7ae88, hand-prop match before interaction.
    pub(crate) fn monitored_object_being_culled<C>(&mut self,id:u32,
        world:&mut crate::simulation_assembly_backend::constraint_session::SimulationAssemblyConstraintSession<skate2_formats::collision::AssemblyPartMaterial,C>,
        objects:&mut DmoObjectMan,cvars:&InputCvarReferences)->Result<(),String>{
        if self.held_hand_prop==Some(id){self.remove_hand_prop(world,objects,cvars)?;}
        else if self.interacting_object==Some(id){
            let object=objects.object_mut(id).ok_or("monitored interacting DMO absent")?;
            if let Some(index)=object.monitoring_entities.iter().position(|monitor|*monitor==self.id){object.monitoring_entities.remove(index);}
            // IPedestrian virtual112 = ClearNavRigIgnoreVolume82e94c80.
            self.nav_rig_ignore_entity=None;
            self.interacting_object=None;
        }
        Ok(())
    }
    pub(crate) fn remove_hand_prop<C>(&mut self, world: &mut crate::simulation_assembly_backend::constraint_session::SimulationAssemblyConstraintSession<skate2_formats::collision::AssemblyPartMaterial,C>,
        objects: &mut DmoObjectMan, cvars: &InputCvarReferences) -> Result<(), String> {
        self.collision.as_mut().ok_or("hand prop removal collision absent")?
            .set_world_collision_group(world, 11)?;
        let state = self.state.as_mut().ok_or("hand prop removal AI state absent")?;
        state.flags[2] &= !2;
        state.flags[2] &= !1;
        self.animation.as_mut().ok_or("hand prop removal animation absent")?.channels.stop(0.2);
        if let Some(id) = self.held_hand_prop {
            let object = objects.object_mut(id).ok_or("held hand prop DMO absent")?;
            object.transform_type = DynamicObjectTransformType::Physics;
            object.collision_ignore_gravity = false;
            let group = if object.model.mass >= 5.5 { 14 } else { 16 };
            world.try_set_living_world_collision_group(LivingWorldBodyKind::Dmo, &object.body, group)
                .map_err(|e| format!("hand prop collision group: {e:?}"))?;
            object.group = group;
            object.hand_prop_cull_timer = cvars.try_dmo_hand_prop_cull_timer().map_err(|e| format!("hand prop cull timer: {e:?}"))?;
            // DeregisterEntityMonitor82d35090 removes the FIRST matching entry.
            if let Some(index) = object.monitoring_entities.iter().position(|&entity| entity == self.id) {
                object.monitoring_entities.remove(index);
            }
            self.held_hand_prop = None;
        }
        Ok(())
    }
}
