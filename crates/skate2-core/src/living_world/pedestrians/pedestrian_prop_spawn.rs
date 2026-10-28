//! Prop census construction and publication over the one shared PropMan.
use super::{PropMan,Prop};
use crate::{attribute_runtime::AttributeRuntimeVault,input_cvar_runtime::InputCvarReferences,
    living_world_rain::geometry::Transform,world::LivingWorldEntityIdAllocator};
use std::sync::Arc;
#[derive(Clone,Copy,PartialEq,Eq)]
pub(crate) enum PropCallback{Default,Waypoint,ActorTracker,Gathering,Null}
impl PropMan {
    /// RequestExplicitCull82e675b0 -> Prop.OnCullRequested82e65bf0 ->
    /// LWBehaviourSink82e4c500 / DeInitContainedProps82e57098. Published
    /// no-graph Props have no behaviour-source registrations or sinks.
    pub(crate) fn request_explicit_cull(&mut self,id:u32,
        events:&crate::hermes_game_sim_queue::GameSimEventRoute)->Result<(),String>{
        self.entities.get_mut(id).and_then(|p|p.core.as_mut())
            .ok_or("culling Prop core absent")?.allow_culling=true;
        if !self.entities.get(id).ok_or("culling Prop absent")?.behaviour_sinks.is_empty(){
            return Err("Prop cull requires concrete attached plugin sink deregistration".into());
        }
        loop {
            let child=self.entities.get(id).ok_or("culling Prop absent")?.contained_props.first().copied();
            let Some(child)=child else{break;};
            self.entities.get_mut(child).ok_or("contained Prop absent")?.containing_entity=None;
            self.request_explicit_cull(child,events)?;
            self.entities.get_mut(id).ok_or("culling container absent")?.contained_props.remove(0);
        }
        events.send(crate::hermes_game_sim_queue::LivingWorldMessage::RequestCull(id))?;
        let losim=self.entities.get(id).and_then(|p|p.core.as_ref()).ok_or("culling Prop core absent")?.current_losim;
        self.execution.remove(losim,id).map_err(|e|format!("Prop detach: {e:?}"))?;
        self.culls.push(self.entities.remove(id).ok_or("detaching Prop absent")?);
        Ok(())
    }
    /// Callback identities from InitCallbackMap82e694f0. Conversation's
    /// descendants have the same null callback; query original parent links,
    /// not a second attribute hierarchy. Cvars are captured at manager Init.
    pub(crate) fn factory_callback(&self,key:u64,vault:&Arc<AttributeRuntimeVault>)->Result<PropCallback,String>{
        use skate2_formats::attrib::string_hash64 as hash;
        let flags=self.factory_callbacks.ok_or("Prop factory callback map not initialized")?;
        if key==hash("actortracker_spectator"){return Ok(PropCallback::ActorTracker);}
        for (index,name) in ["waypoint_spectator","waypoint_patrol","waypoint_sit","waypoint_vendingmachine",
            "waypoint_atm","waypoint_waterfountain","waypoint_newspaperbox","waypoint_usetrashbin","waypoint_lookat"].iter().enumerate(){
            if flags[index]&&key==hash(name){return Ok(PropCallback::Waypoint);}
        }
        if flags[9]&&key==hash("gathering_01"){return Ok(PropCallback::Gathering);}
        if flags[10]{
            let target=hash("waypoint_conversation");let mut at=key;
            while at!=0{
                if at==target{return Ok(PropCallback::Null);}
                let entity=vault.instance(0x60d7_03c7_7cab_1631,at);
                if entity.is_null(){break;}
                at=vault.collection_parent(0x60d7_03c7_7cab_1631,at).map_err(|e|e.to_string())?;
            }
        }
        Ok(PropCallback::Default)
    }
}

pub(crate) struct PropSpawnInfo {
    pub(crate) entity_key:u64,
    pub(crate) model_key:u64,
    pub(crate) recipe_id:u64,
    pub(crate) transform:Transform,
    pub(crate) allow_culling:bool,
}
impl Prop {
    ///SpawnsHandPropsOnUse82e65090 reads the actual livingworld_props member
    ///array length, not the currently held object or general spawn flag.
    pub(crate) fn spawns_hand_props_on_use(&self)->Result<bool,String>{
        Ok(self.characteristics.as_ref().ok_or("Prop characteristics absent")?
            .try_length(0xe15e_856f_2ca9_b96b).map_err(|e|e.to_string())?!=0)
    }
    ///DecideToSpawnPedestrian82e64938 consumes global Math::BernoulliRandom,
    ///and resets the retry timer only when the draw rejects the spawn.
    pub(crate) fn decide_to_spawn_pedestrian(&mut self)->Result<bool,String>{
        let probability=self.pedestrian_spawn_probability.ok_or("Prop spawn probability absent")?;
        let word=crate::native_skater_random::rw_math::global_next()
            .map_err(|e|format!("Prop spawn Bernoulli RNG: {e:?}"))?;
        let accepted=crate::native_skater_random::rw_math::bernoulli_choice(probability,word);
        if !accepted {
            self.pedestrian_spawn_timer=self.default_pedestrian_spawn_timeout
                .ok_or("Prop spawn timeout absent")?;
        }
        Ok(accepted)
    }
    ///827217e8..82721924: admission before point-census/category selection.
    ///Return actual first unlocked point and original offset position; later
    ///spawn failure must retain this prefix's RNG/timer side effects.
    pub(crate) fn prepare_pedestrian_spawn(&mut self)->Result<Option<(u32,[f32;4],[f32;4])>,String>{
        if self.waypoint.is_none()||!self.can_offer_behaviour()||self.flags&0x40==0
            ||self.pedestrian_spawn_timer>0. {return Ok(None);}
        if !self.decide_to_spawn_pedestrian()?{return Ok(None);}
        let group=self.waypoint.as_ref().expect("checked waypoint owner");
        let Some(index)=group.first_unlocked() else{return Ok(None);};
        let point=&group.points[index as usize];
        let offset=self.pedestrian_spawn_offset.ok_or("Prop pedestrian spawn offset absent")?;
        //Separate vector multiply/add, not fused:82721914 then82721924.
        let position=std::array::from_fn(|i|point.location[i]+point.orientation[i]*offset);
        Ok(Some((index,position,point.orientation)))
    }
}
impl PropMan {
    /// LWPropCensusMan.Spawn82e64f18, then AttachToManagers82e67a00.
    /// Caller resolves the actual factory callback and original recipe/model
    /// gates first. This path constructs the concrete WaypointGroup callback.
    pub(crate) fn attach_spawned_prop(&mut self, pending:&mut Option<Prop>)->Result<u32,String>{
        let prop=pending.as_ref().ok_or("Prop construction absent")?;
        let id=prop.id;
        let losim=prop.core.as_ref().ok_or("Prop core absent")?.current_losim;
        self.entities.add_retained(id,pending).map_err(|e|format!("Prop AddEntity: {e:?}"))?;
        if pending.is_some(){return Err("duplicate Prop ID retains unpublished entity".into());}
        self.execution.add(losim,id).map_err(|e|format!("Prop execution admission: {e:?}"))?;
        Ok(id)
    }
    pub(crate) fn spawn_prop_into(&mut self,pending:&mut Option<Prop>,info:&PropSpawnInfo,waypoint:bool,
        graphs:&crate::actor_construction::graph_startup::ActorGraphStartupOwners,
        ids:&mut LivingWorldEntityIdAllocator,vault:&Arc<AttributeRuntimeVault>,
        _cvars:&InputCvarReferences)->Result<Option<u32>,String>{
        if !self.construct_prop_retained(pending,ids,vault,info.entity_key,info.model_key,waypoint,graphs)?{return Ok(None);}
        if self.factory_callback(info.entity_key,vault)?==PropCallback::ActorTracker {
            let allocation=self.memory.actor_tracker.allocate().ok_or("ActorTracker store5 exhausted")?;
            pending.as_mut().ok_or("ActorTracker base construction absent")?.actor_tracker=
                Some(super::actor_tracker::ActorTrackerData{allocation,recognizers:Vec::with_capacity(16)});
        }
        let prop=pending.as_mut().ok_or("Prop factory returned no retained entity")?;
        let core=prop.core.as_mut().ok_or("Prop core absent")?;
        core.recipe_id=info.recipe_id;
        core.set_target_transform::<std::convert::Infallible>(info.transform).map_err(|e|format!("Prop target transform: {e:?}"))?;
        // SetPropPosition82e66a38 -> SetPosition82e57770 changes translation
        // and XZ lookup only; target orientation is NOT copied to current.
        core.transform[3]=info.transform[3];
        core.position_xz=[info.transform[3][0],info.transform[3][2]];
        core.allow_culling=info.allow_culling;
        Ok(Some(prop.id))
    }
}
