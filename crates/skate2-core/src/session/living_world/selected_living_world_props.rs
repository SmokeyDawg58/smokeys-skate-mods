//! Original SpawnProp8271c430 through retained RecipeMan and PropMan owners.
use super::SelectedRecipeFrame;
use crate::{input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::LivingWorldGrindServices,
    living_world_grind_services::waypoints::spawn::PropSpawnInfo,
    living_world_rain::geometry::Transform,
    living_world_recipe_manager::{attributes::RecipeAttributes,LIVING_WORLD_MODELS_CLASS_ID,LOAD_MODEL_ATTRIBUTE_ID}};

pub(crate) struct PropRecipeFrame<'a>{
    pub(crate) avoid_volumes:&'a crate::pedestrian_navigation::wander::ChallengeAvoidVolumes,
    pub(crate) graphs:&'a crate::actor_construction::graph_startup::ActorGraphStartupOwners,
    pub(crate) recipes:&'a crate::living_world_recipe_manager::runtime::RecipeMan,
    pub(crate) vault:&'a std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,
}
/// Split references to the same spawning owners. DMO construction already
/// borrows its map and locator manager; child creation must not reacquire them.
pub(crate) struct PropSpawnOwners<'a>{
    pub(crate) props:&'a mut crate::living_world_grind_services::waypoints::PropMan,
    pub(crate) entity_ids:&'a mut crate::world::LivingWorldEntityIdAllocator,
    pub(crate) census_targets:&'a mut crate::living_world_grind_services::attrib_cache::CensusTargetCache,
    pub(crate) models:&'a crate::living_world_grind_services::dmo_models::DmoModelMan,
    pub(crate) census_enable:u8,
}
impl SelectedRecipeFrame<'_>{
    pub(crate) fn prop_recipe_frame(&self)->PropRecipeFrame<'_>{PropRecipeFrame{avoid_volumes:&self.runtime.challenge_avoid_volumes,graphs:self.graphs,recipes:&self.runtime.recipes,vault:self.vault}}
}
impl PropRecipeFrame<'_>{
    /// Prop ComponentMan.UpdateLOSStates82e66168. Prop Entity vslot3c
    /// (82394be8+3c ->826c6d80) returns true for OnLOSimSwitch.
    pub(crate) fn update_los_states(&self, living: &mut LivingWorldGrindServices,
        center: [f32; 4]) -> Result<(), String> {
        let props = living.props.as_deref_mut().ok_or("Prop LOSim manager absent")?;
        let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
        for id in props.entities.ids() {
            let prop = props.entities.get_mut(id).ok_or("Prop LOSim entity absent")?;
            let cache = prop.attrib_cache.as_ref().ok_or("Prop LOSim attributes absent")?;
            let core = prop.core.as_mut().ok_or("Prop LOSim core absent")?;
            let next = core.test_losim(center, cache, &mut numeric)
                .map_err(|e|format!("Prop LOSim distance: {e:?}"))?;
            let previous = core.current_losim;
            if previous != next {
                props.execution.remove(previous, id).map_err(|e|format!("Prop LOSim remove: {e:?}"))?;
                core.current_losim = next;
                props.execution.add(next, id).map_err(|e|format!("Prop LOSim add: {e:?}"))?;
            }
        }
        Ok(())
    }
    /// Manager.InitPropData8271d610, after graph preload and manager creation.
    pub(crate) fn initialize_actor_tracker(&mut self,living:&mut LivingWorldGrindServices,
        cvars:&InputCvarReferences)->Result<(),String>{
        if !cvars.try_enable_actor_tracker().map_err(|e|format!("ActorTracker cvar: {e:?}"))?
            ||living.props.as_ref().ok_or("PropMan absent")?.actor_tracker.is_some(){return Ok(());}
        let transform=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.;4]];
        let id=self.spawn_waypoint_prop(living,skate2_formats::attrib::string_hash64("actortracker_spectator"),
            transform,false,cvars)?;
        living.props.as_mut().ok_or("PropMan absent")?.actor_tracker=id;
        if let Some(id)=id {
            // Native InitPropData8271d708 explicitly appends after SpawnProp;
            // preserve this even if the base Prop ctor already registered it.
            let mut sources=self.graphs.plugin_sources.lock().map_err(|_|"ActorTracker source list lock")?;
            if sources.len()==1024{return Err("ActorTracker source list capacity".into());}
            sources.push(id);
        }
        Ok(())
    }
    /// LWPropExecutionMan.UpdatePreFrame82e68928. Admitted Props currently
    /// have no AI graph; transferable templates belong to their sink copies.
    pub(crate) fn update_preframe(&self,living:&mut LivingWorldGrindServices,
        center:[f32;4],local_actor_position:Option<[f32;4]>)->Result<(),String>{
        let props=living.props.as_deref_mut().ok_or("PropMan absent")?;
        let mut targets=Vec::new();
        let mut numeric=crate::reckoning_numeric::HostReckoningNumeric;
        props.execution.collect_preframe(&mut props.entities,center,&mut targets,&mut numeric)
            .map_err(|e|format!("Prop preframe targets: {e:?}"))?;
        let player=local_actor_position.unwrap_or([0.;4]);
        for target in &targets {
            let core=props.entities.get_mut(target.entity).and_then(|p|p.core.as_mut())
                .ok_or("Prop preframe core absent")?;
            core.player_distance.update(player,core.transform[3],&mut numeric)
                .map_err(|e|format!("Prop positional data: {e:?}"))?;
        }
        for target in targets {
            let prop=props.entities.get_mut(target.entity).ok_or("Prop preframe entity absent")?;
            // ActorTracker82e64990 overrides (does not call) Prop prestate.
            // IProp.SetPropPosition changes current translation and XZ only.
            if prop.actor_tracker.is_some(){
                if let Some(position)=local_actor_position {
                    let core=prop.core.as_mut().ok_or("ActorTracker core absent")?;
                    core.transform[3]=position;
                    core.position_xz=[position[0],position[2]];
                }
                continue;
            }
            // PreStateGraphUpdate82e648c0: clamp only the old nonpositive
            // value; a positive timer may become negative on this frame.
            prop.pedestrian_spawn_timer=if prop.pedestrian_spawn_timer<=0. {0.}
                else {prop.pedestrian_spawn_timer-target.dt};
        }
        Ok(())
    }
    /// DeregisterWaypointData82722088 no-graph waypoint branch. Notification
    /// precedes point release; detach precedes batch census destruction.
    pub(crate) fn deregister_waypoint_asset(&mut self,living:&mut LivingWorldGrindServices,
        id:u64,asset:crate::living_world_grind_services::waypoints::WaypointAsset,
        send_message:bool,world:&mut crate::physical_player_assembly::PhysicalAssemblyWorld,
        cvars:&InputCvarReferences,painter:&crate::world_painter_layer::WorldPainterLayerMan,
        avoid:&crate::pedestrian_navigation::wander::ResolvedAvoidVolumes)->Result<(),String>{
        use crate::hermes_game_sim_queue::LivingWorldMessage;
        let events=living.game_sim_events.as_ref().ok_or("GameSim broadcaster absent")?;
        let props=living.props.as_mut().ok_or("PropMan absent")?;
        if props.waypoint_chunks.remove(&id).is_none(){return Err("native unregistered waypoint chunk assertion".into());}
        let pedestrians=living.pedestrian_entities.as_mut().ok_or("pedestrian EntityMan absent")?;
        for index in 0..asset.count()?{props.notify_waypoint_deregister(asset.group(index)?.local_guid()?,pedestrians)?;}
        let mut removed=Vec::new();
        for prop_id in props.entities.ids(){
            if props.entities.get(prop_id).and_then(|p|p.waypoint.as_ref()).is_none(){continue;}
            let guid=props.group(prop_id)?.guid()?;
            for index in 0..asset.count()?{
                if guid!=asset.group(index)?.local_guid()?{continue;}
                if send_message{events.send(LivingWorldMessage::WaypointGroupRemoved(guid))?;}
                if props.waypoint_guids.get(&guid)==Some(&prop_id){props.waypoint_guids.remove(&guid);}
                let lockers:Vec<_>=props.group(prop_id)?.points.iter().filter_map(|p|p.locking_entity).collect();
                for locker in lockers{
                    let ped=pedestrians.get_mut(locker).ok_or("waypoint locker pedestrian absent")?;
                    if ped.core.is_some(){ped.clear_current_waypoint(props)?;}
                }
                props.entities.get_mut(prop_id).and_then(|p|p.waypoint.as_mut()).ok_or("waypoint group absent")?
                    .set_data(None,&mut props.memory)?;
                removed.push(prop_id);
            }
        }
        let actors=world.entity_roster.as_ref().ok_or("waypoint cull Actor roster absent")?
            .snapshot().map_err(|e|format!("waypoint cull Actor roster: {e:?}"))?;
        let mut random=living.random.take().ok_or("waypoint cull Manager random absent")?;
        let result=(||{
            for prop_id in removed{
                self.request_contained_prop_cull(living,world,prop_id,cvars,painter,&mut random,&actors,avoid)?;
            }
            self.process_culls(living)
        })();
        living.random=Some(random);
        result
    }
    /// Shared Census.Update destructor for detached Props, also used after
    /// waypoint deregistration. No second cull list is introduced.
    pub(crate) fn process_culls(&mut self,living:&mut LivingWorldGrindServices)->Result<(),String>{
        let props=living.props.as_deref_mut().ok_or("PropMan absent")?;
        let pedestrians=living.pedestrian_entities.as_deref_mut().ok_or("pedestrian EntityMan absent")?;
        // Census.Update visits its retained detached list after all requests.
        while let Some(prop)=props.culls.first_mut(){
            if let Some(tracker)=prop.actor_tracker.take(){
                if !props.memory.actor_tracker.free(tracker.allocation){return Err("ActorTracker destructor allocation mismatch".into());}
                // ActorTracker::~ActorTracker82e66de0 releases store5. The
                // retained host manager must retire its ID with that payload;
                // the next GameLoad's InitPropData recreates the tracker.
                if props.actor_tracker==Some(prop.id){props.actor_tracker=None;}
            }
            if let Some(group)=prop.waypoint.as_mut(){
                group.set_data(None,&mut props.memory)?;
                if !props.memory.group.free(group.allocation){return Err("waypoint group destructor allocation mismatch".into());}
            }
            prop.waypoint=None;
            for id in pedestrians.ids(){pedestrians.get_mut(id).expect("native map order").notify_entity_culled(prop.id)?;}
            self.graphs.plugin_sources.lock().map_err(|_|"BehaviourMan source list lock")?.retain(|id|*id!=prop.id);
            let core_slot=prop.core_allocation.take().ok_or("Prop destructor core allocation absent")?;
            if !props.memory.core.free(core_slot){return Err("Prop core destructor allocation mismatch".into());}
            prop.core=None;
            if !props.memory.entity.free(prop.allocation){return Err("Prop destructor entity allocation mismatch".into());}
            props.culls.remove(0);
        }
        Ok(())
    }
    /// Caller has resolved the actual LWPropFactory CreateWaypointGroup
    /// callback. Model/recipe/load/limit gates remain the original shared ones.
    pub(crate) fn spawn_waypoint_prop(&mut self,living:&mut LivingWorldGrindServices,
        entity_key:u64,transform:Transform,allow_culling:bool,cvars:&InputCvarReferences,
    )->Result<Option<u32>,String>{
        let mut owners=PropSpawnOwners{
            props:living.props.as_deref_mut().ok_or("PropMan absent")?,
            entity_ids:&mut living.entity_ids,census_targets:&mut living.census_targets,
            models:&living.dmo_models,census_enable:living.census.enable,
        };
        self.spawn_waypoint_prop_with_owners(&mut owners,entity_key,transform,allow_culling,cvars)
    }
    pub(crate) fn spawn_waypoint_prop_with_owners(&mut self,owners:&mut PropSpawnOwners<'_>,
        entity_key:u64,transform:Transform,allow_culling:bool,cvars:&InputCvarReferences,
    )->Result<Option<u32>,String>{
        if owners.census_enable&0x80==0{return Ok(None);}
        let entity=self.vault.instance(0x60d7_03c7_7cab_1631,entity_key);
        if entity.is_null(){return Ok(None);}
        let parent=entity.try_cached_layout_field(8,8).map_err(|e|e.to_string())?
            .map_or(0,|value|u64::from_be_bytes(value.bytes().try_into().expect("eight bytes")));
        let attributes=RecipeAttributes::new(self.vault,cvars);
        let model_key=self.recipes.select_model(parent,&attributes).map_err(|e|format!("Prop model selection: {e:?}"))?;
        let model=self.vault.instance(LIVING_WORLD_MODELS_CLASS_ID,model_key);
        if model.is_null(){return Ok(None);}
        let load=model.try_get_data(LOAD_MODEL_ATTRIBUTE_ID,0).map_err(|e|e.to_string())?
            .is_some_and(|value|value.bytes().first().is_some_and(|&byte|byte!=0));
        let recipe_id=self.recipes.model_asset_id(model_key,&attributes).map_err(|e|e.to_string())?;
        if load&&(recipe_id==crate::living_world_grind_services::dmo_models::NULL_ASSET_ID
            || !owners.models.contains_model(recipe_id)){return Ok(None);}
        let limit=cvars.try_max_props().map_err(|e|format!("Prop limit: {e:?}"))?;
        let props=&mut *owners.props;
        if limit==0||limit as usize<=props.entities.captured_member_count(){return Ok(None);}
        let attrib_cache=owners.census_targets.get(model_key,self.vault).map_err(|e|format!("Prop attribute cache: {e:?}"))?;
        use crate::living_world_grind_services::waypoints::spawn::PropCallback;
        let callback=props.factory_callback(entity_key,self.vault)?;
        let waypoint=match callback{
            PropCallback::Null=>return Ok(None),
            PropCallback::Waypoint=>true,
            PropCallback::Default=>false,
            PropCallback::ActorTracker=>false,
            _=>return Err("Prop callback requires its concrete non-waypoint constructor".into()),
        };
        if props.unfinished.is_some() || !props.failed_containers.is_empty() {
            return Err("unfinished Prop construction retained".into());
        }
        let mut pending=None;
        let result=(||{
            let result=owners.props.spawn_prop_into(&mut pending,
                &PropSpawnInfo{entity_key,model_key,recipe_id,transform,allow_culling},waypoint,self.graphs,
                owners.entity_ids,self.vault,cvars)?;
            let Some(id)=result else{return Ok(None);};
            pending.as_mut().ok_or("Prop construction absent")?.attrib_cache=Some(attrib_cache);
            // InitContainedProps82e615a0 occurs before AttachToManagers.
            // The same CensusMan recursively creates each child with the
            // supplied transform and bAllowCull=false, then sets its parent.
            if cvars.try_allow_contained_props().map_err(|e|format!("contained Prop cvar: {e:?}"))? {
                const KEY:u64=0x3eb7_7fb8_ca81_c99b;
                if entity.try_length(KEY).map_err(|e|e.to_string())?>8 {
                    return Err("Prop contained array exceeds native capacity8".into());
                }
                let mut index=0;
                while index<entity.try_length(KEY).map_err(|e|e.to_string())? {
                    let data=entity.try_array_field_data(KEY,index,0x97c5_113d_e9bd_1ad6,24,8..16)
                        .map_err(|e|e.to_string())?;
                    let key=data.map_or(0,|data|u64::from_be_bytes(data.bytes().try_into().expect("RefSpec collection")));
                    let child=self.spawn_waypoint_prop_with_owners(owners,key,transform,false,cvars)?
                        .ok_or("native InitContainedProps child spawn assertion")?;
                    owners.props.entities.get_mut(child)
                        .ok_or("contained Prop absent after admission")?.containing_entity=Some(id);
                    pending.as_mut().ok_or("container construction absent")?.contained_props.push(child);
                    index+=1;
                }
            }
            owners.props.attach_spawned_prop(&mut pending)?;
            Ok(Some(id))
        })();
        if result.is_err(){
            if let Some(prop)=pending {
                // A failing nested constructor may already retain its own
                // prefix. Keep every ancestor without replacing that child.
                owners.props.failed_containers.push(prop);
            }
        }
        result
    }
    /// RegisterWaypointData82719e60. The actual stream callback supplies the
    /// retained registry asset; this entire operation is synchronous.
    pub(crate) fn register_waypoint_asset(&mut self,living:&mut LivingWorldGrindServices,
        id:u64,asset:crate::living_world_grind_services::waypoints::WaypointAsset,
        send_message:bool,cvars:&InputCvarReferences)->Result<(),String>{
        let props=living.props.as_mut().ok_or("PropMan absent")?;
        if !props.waypoint_chunks.contains_key(&id)&&props.waypoint_chunks.len()>=128{
            return Err("native waypoint chunk map capacity128".into());
        }
        props.waypoint_chunks.entry(id).or_insert_with(||asset.clone());
        for index in 0..asset.count()?{
            let source=asset.group(index)?;let key=source.entity_key()?;
            if living.props.as_ref().ok_or("PropMan absent")?.factory_callback(key,self.vault)?
                ==crate::living_world_grind_services::waypoints::spawn::PropCallback::Default {continue;}
            let transform=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],source.centroid()?];
            let Some(prop_id)=self.spawn_waypoint_prop(living,key,transform,false,cvars)? else{continue;};
            let props=living.props.as_mut().ok_or("PropMan absent")?;
            let group=props.entities.get_mut(prop_id).and_then(|prop|prop.waypoint.as_mut())
                .ok_or("spawned waypoint interface absent")?;
            group.set_data(Some(source.clone()),&mut props.memory)?;
            let guid=group.guid()?;
            if !props.waypoint_guids.contains_key(&guid)&&props.waypoint_guids.len()>=256{
                return Err("native waypoint GUID map capacity256".into());
            }
            props.waypoint_guids.entry(guid).or_insert(prop_id);
            if send_message{living.game_sim_events.as_ref().ok_or("Simulation GameSim route absent")?
                .send(crate::hermes_game_sim_queue::LivingWorldMessage::WaypointGroupAdded(source.local_guid()?))?;}
        }
        Ok(())
    }
}
