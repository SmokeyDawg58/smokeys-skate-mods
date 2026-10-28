//! Native PropMan waypoint payloads and reservations. The sole Prop EntityMan
//! owns groups; references are (actual prop ID, native vector index), not a
//! second standalone waypoint registry.
use crate::{
    living_world_grind_services::{entities::{EntityMan, SpatialEntity}, entity_core::CommonEntityCore},
    simulation_inner_registry_asset::SimulationInnerRegistryAsset,
    simulation_normal_arena_plan::SimulationRegistryAssetKind,
    world::LivingWorldMemStoreSlots,
};
use std::{collections::BTreeMap, sync::Arc};
#[path="pedestrian_plugin.rs"]
pub(crate) mod plugin;
#[path = "pedestrian_prop_spawn.rs"]
pub(crate) mod spawn;
#[path = "pedestrian_actor_tracker.rs"]
pub(crate) mod actor_tracker;
#[path = "pedestrian_spectators.rs"]
pub(crate) mod spectators;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WaypointRef { pub(crate) prop: u32, pub(crate) index: u32 }

/// Source tWayPointData.Fixup829c6090 resolves offsets relative to this same
/// retained record; it does NOT use Arena::IdToObject for its internal arrays.
#[derive(Clone)]
pub(crate) struct WaypointAsset { pub(crate) asset: Arc<SimulationInnerRegistryAsset> }
fn word(bytes:&[u8],at:usize)->Result<u32,String>{
    Ok(u32::from_be_bytes(bytes.get(at..at.checked_add(4).ok_or("waypoint offset overflow")?)
        .ok_or("waypoint record truncated")?.try_into().expect("four bytes")))
}
fn wide(bytes:&[u8],at:usize)->Result<u64,String>{Ok((u64::from(word(bytes,at)?)<<32)|u64::from(word(bytes,at+4)?))}
fn vector(bytes:&[u8],at:usize)->Result<[f32;4],String>{Ok([
    f32::from_bits(word(bytes,at)?),f32::from_bits(word(bytes,at+4)?),
    f32::from_bits(word(bytes,at+8)?),f32::from_bits(word(bytes,at+12)?)])}
impl WaypointAsset {
    pub(crate) fn new(asset:Arc<SimulationInnerRegistryAsset>)->Result<Self,String>{
        if asset.kind!=SimulationRegistryAssetKind::Waypoint{return Err("wrong waypoint asset kind".into());}
        let this=Self{asset}; this.count()?; Ok(this)
    }
    fn bytes(&self)->Result<&[u8],String>{self.asset.bytes().map_err(|e|format!("waypoint asset: {e:?}"))}
    pub(crate) fn count(&self)->Result<u32,String>{word(self.bytes()?,0)}
    pub(crate) fn group(&self,index:u32)->Result<WaypointGroupSource,String>{
        if index>=self.count()?{return Err("waypoint group index out of range".into());}
        let bytes=self.bytes()?;
        let at=(word(bytes,12)? as usize).checked_add((index as usize).checked_mul(96).ok_or("group index overflow")?)
            .ok_or("group offset overflow")?;
        bytes.get(at..at.checked_add(96).ok_or("group size overflow")?).ok_or("waypoint group truncated")?;
        Ok(WaypointGroupSource::Asset{asset:self.clone(),offset:at})
    }
}
#[derive(Clone)]
pub(crate) enum WaypointGroupSource {
    Asset { asset:WaypointAsset, offset:usize },
    Hotpoint(Arc<HotpointWaypointSource>),
}
pub(crate) struct HotpointWaypointSource {
    entity_key:u64, centroid:[f32;4], location:[f32;4], orientation:[f32;4],
}
impl WaypointGroupSource {
    /// InitializeHotpoints82d38ec0 allocates one group and one point per
    /// non-hand hotpoint. Both GUIDs are explicitly zero, not generated IDs.
    pub(crate) fn from_hotpoint(entity_key:u64,transform:[f32;16])->Self{
        let location=std::array::from_fn(|i|transform[12+i]);
        Self::Hotpoint(Arc::new(HotpointWaypointSource{entity_key,
            centroid:location.map(|value|0.+value),location,
            orientation:std::array::from_fn(|i|transform[8+i])}))
    }
    pub(crate) fn global_guid(&self)->Result<u64,String>{match self{
        Self::Asset{asset,offset}=>wide(asset.bytes()?,offset+48),Self::Hotpoint(_)=>Ok(0)}}
    /// WaypointGroup.GetGUID82e68e98 returns LOCAL guid+56, not global+48.
    pub(crate) fn guid(&self)->Result<u64,String>{self.local_guid()}
    pub(crate) fn local_guid(&self)->Result<u64,String>{match self{
        Self::Asset{asset,offset}=>wide(asset.bytes()?,offset+56),Self::Hotpoint(_)=>Ok(0)}}
    pub(crate) fn entity_key(&self)->Result<u64,String>{match self{
        Self::Asset{asset,offset}=>wide(asset.bytes()?,offset+72),Self::Hotpoint(data)=>Ok(data.entity_key)}}
    pub(crate) fn centroid(&self)->Result<[f32;4],String>{match self{
        Self::Asset{asset,offset}=>vector(asset.bytes()?,*offset),Self::Hotpoint(data)=>Ok(data.centroid)}}
    pub(crate) fn point_count(&self)->Result<u32,String>{match self{
        Self::Asset{asset,offset}=>word(asset.bytes()?,offset+80),Self::Hotpoint(_)=>Ok(1)}}
    fn point(&self,index:u32)->Result<([f32;4],[f32;4]),String>{
        if index>=self.point_count()?{return Err("waypoint source index out of range".into());}
        let (asset,offset)=match self{
            Self::Asset{asset,offset}=>(asset,*offset),
            Self::Hotpoint(data)=>return Ok((data.location,data.orientation)),
        };
        let bytes=asset.bytes()?;
        let base=word(bytes,offset+84)? as usize;
        if base==0{return Err("nonnull waypoint count with null point data".into());}
        let at=base.checked_add((index as usize).checked_mul(48).ok_or("point index overflow")?)
            .ok_or("point offset overflow")?;
        Ok((vector(bytes,at)?,vector(bytes,at+16)?))
    }
    /// UpdateContainedPropTransforms82d2e510 indexes the live waypoint vector
    /// by the authored point ID at+32, not the source enumeration ordinal.
    pub(crate) fn point_with_id(&self,index:u32)->Result<(u32,[f32;4],[f32;4]),String>{
        let (location,orientation)=self.point(index)?;
        let id=match self {
            Self::Hotpoint(_)=>0,
            Self::Asset{asset,offset}=>{
                let bytes=asset.bytes()?;
                let base=word(bytes,offset+84)? as usize;
                let at=base.checked_add((index as usize).checked_mul(48).ok_or("point index overflow")?)
                    .and_then(|at|at.checked_add(32)).ok_or("point ID offset overflow")?;
                word(bytes,at)?
            }
        };
        Ok((id,location,orientation))
    }
}

/// LWPropMemoryMan.Init82e64740. Store2 deliberately has255 blocks, store6
/// has1020. Shared MemStore preserves original free-list order and exhaustion.
pub(crate) struct PropMemory {
    pub(crate) entity:LivingWorldMemStoreSlots<256>,
    pub(crate) core:LivingWorldMemStoreSlots<256>,
    pub(crate) group:LivingWorldMemStoreSlots<255>,
    pub(crate) conversation:LivingWorldMemStoreSlots<32>,
    pub(crate) gathering:LivingWorldMemStoreSlots<8>,
    pub(crate) actor_tracker:LivingWorldMemStoreSlots<1>,
    pub(crate) point:LivingWorldMemStoreSlots<1020>,
}
impl PropMemory {
    pub(crate) fn new()->Self{Self{entity:LivingWorldMemStoreSlots::new(),core:LivingWorldMemStoreSlots::new(),
        group:LivingWorldMemStoreSlots::new(),conversation:LivingWorldMemStoreSlots::new(),
        gathering:LivingWorldMemStoreSlots::new(),actor_tracker:LivingWorldMemStoreSlots::new(),point:LivingWorldMemStoreSlots::new()}}
}
pub(crate) struct Waypoint {
    pub(crate) allocation:u32,
    pub(crate) source_index:u32,
    pub(crate) location:[f32;4],
    pub(crate) orientation:[f32;4],
    pub(crate) locking_entity:Option<u32>,
}
impl Waypoint {
    pub(crate) fn lock(&mut self,pedestrian:u32)->Result<(),String>{
        if self.locking_entity.is_some(){return Err("Waypoint.LockWaypoint native already-locked assertion".into());}
        self.locking_entity=Some(pedestrian);Ok(())
    }
    ///82b983b0 ignores the caller identity and always clears the pointer.
    pub(crate) fn unlock(&mut self){self.locking_entity=None;}
}
pub(crate) struct WaypointGroup {
    pub(crate) allocation:u32,
    pub(crate) source:Option<WaypointGroupSource>,
    pub(crate) centroid:[f32;4],
    pub(crate) points:Vec<Waypoint>,
}
impl WaypointGroup {
    pub(crate) fn construct(memory:&mut PropMemory)->Result<Self,String>{
        let allocation=memory.group.allocate().ok_or("Prop waypoint-group store exhausted")?;
        Ok(Self{allocation,source:None,centroid:[0.;4],points:Vec::new()})
    }
    /// SetData82e68b08 frees old points, clears centroid, retains source, then
    /// allocates each point. Native failed point allocations skip that entry;
    /// source_index still records its original array index, not vector index.
    pub(crate) fn set_data(&mut self,source:Option<WaypointGroupSource>,memory:&mut PropMemory)->Result<(),String>{
        for point in self.points.drain(..){
            if !memory.point.free(point.allocation){return Err("waypoint point allocation owner mismatch".into());}
        }
        self.centroid=[0.;4];self.source=source;
        let Some(source)=&self.source else{return Ok(());};
        self.centroid=source.centroid()?;
        let count=source.point_count()?;
        self.points.try_reserve(count as usize).map_err(|_|"waypoint vector allocation".to_string())?;
        for index in 0..count {
            if let Some(allocation)=memory.point.allocate(){
                // Retain a native constructor prefix before any fallible data projection.
                self.points.push(Waypoint{allocation,source_index:index,location:[0.;4],orientation:[0.;4],locking_entity:None});
                let (location,orientation)=source.point(index)?;
                let point=self.points.last_mut().expect("point retained");point.location=location;point.orientation=orientation;
            }
        }
        Ok(())
    }
    pub(crate) fn guid(&self)->Result<u64,String>{self.source.as_ref().map_or(Ok(0),WaypointGroupSource::guid)}
    pub(crate) fn first_unlocked(&self)->Option<u32>{self.points.iter().position(|p|p.locking_entity.is_none()).map(|i|i as u32)}
    pub(crate) fn closest_unlocked(&self,position:[f32;4])->Option<u32>{
        let mut best=None;let mut distance=f32::MAX;
        for (index,point) in self.points.iter().enumerate(){
            if point.locking_entity.is_some(){continue;}
            let delta=std::array::from_fn(|i|point.location[i]-position[i]);
            let candidate=crate::reckoning_numeric::host_dot3(delta,delta);
            if candidate<=distance{distance=candidate;best=Some(index as u32);}
        }
        best
    }
}

/// Actual common Prop payload. Runtime graph construction remains mandatory
/// before admission; the group is not a substitute for its base Prop owner.
pub(crate) struct Prop {
    pub(crate) actor_tracker:Option<actor_tracker::ActorTrackerData>,
    pub(crate) transferable:Option<Arc<plugin::load::ProcessedTransferable>>,
    pub(crate) behaviour_sinks:Vec<u32>,
    pub(crate) behaviour_copies:Vec<Arc<()>>,
    pub(crate) containing_entity:Option<u32>,
    pub(crate) contained_props:Vec<u32>,
    pub(crate) attrib_cache:Option<Arc<crate::living_world_grind_services::attrib_cache::LwAttribCache>>,
    pub(crate) id:u32,
    pub(crate) allocation:u32,
    pub(crate) core_allocation:Option<u32>,
    pub(crate) core:Option<CommonEntityCore>,
    pub(crate) characteristics:Option<crate::attribute_runtime::AttributeRuntimeInstance>,
    pub(crate) flags:u8,
    pub(crate) pedestrian_spawn_offset:Option<f32>,
    pub(crate) pedestrian_spawn_probability:Option<f32>,
    pub(crate) default_pedestrian_spawn_timeout:Option<f32>,
    /// MemStore clear and Prop ctor retain zero at+e0 before frame updates.
    pub(crate) pedestrian_spawn_timer:f32,
    pub(crate) transfer_radius:Option<f32>,
    pub(crate) test_nav_mesh_offset:Option<f32>,
    pub(crate) waypoint:Option<WaypointGroup>,
}
impl SpatialEntity for Prop {
    fn core_position_xz(&self)->[f32;2]{self.core.as_ref().expect("published Prop core").position_xz}
    fn core_data_valid(&self)->bool{self.core.is_some()}
}
impl crate::living_world_grind_services::execution::ExecutionEntity for Prop {
    fn execution_core(&self)->Option<&CommonEntityCore>{self.core.as_ref()}
    fn execution_core_mut(&mut self)->Option<&mut CommonEntityCore>{self.core.as_mut()}
}
pub(crate) struct PropMan {
    /// Manager.mpActorTracker points into this same Prop EntityMan.
    pub(crate) actor_tracker:Option<u32>,
    pub(crate) entities:Box<EntityMan<256,Prop>>,
    pub(crate) memory:PropMemory,
    pub(crate) execution:crate::living_world_grind_services::execution::ExecutionMan<256,Prop>,
    pub(crate) unfinished:Option<Prop>,
    pub(crate) failed_containers:Vec<Prop>,
    pub(crate) culls:Vec<Prop>,
    pub(crate) waypoint_guids:WaypointGuidMap,
    pub(crate) waypoint_chunks:WaypointChunkMap,
    pub(crate) factory_callbacks:Option<[bool;11]>,
}
impl PropMan {
    /// LWBehaviourSink.UpdateContainedPropTransforms82e503b0. Share this base
    /// pass with ordinary pedestrians and the DMO waypoint specialization.
    pub(crate) fn update_contained_prop_transforms(
        &mut self, children: &[u32], transform: [[f32; 4]; 4],
    ) -> Result<(), String> {
        if children.len() > 8 {
            return Err("native contained Prop capacity8 exceeded".into());
        }
        // Retained IDs represent non-null native children; a missing ID is a
        // lifetime error, not the optional null pointer skipped by the source.
        for child in children {
            self.entities.get_mut(*child).and_then(|p| p.core.as_mut())
                .ok_or("contained transform Prop core absent")?
                .set_transform::<String>(transform)
                .map_err(|e| format!("contained transform: {e:?}"))?;
        }
        Ok(())
    }

    /// Prop82e67150 then WaypointGroup82e68d90. This completes the actual
    /// no-graph branch (used by shipped waypoint_patrol). Graph-bearing props
    /// stop at their real missing loader, retaining allocations and prefix;
    /// parsed graph metadata is not mistaken for an instantiated controller.
    pub(crate) fn construct_prop_retained(&mut self,retained:&mut Option<Prop>,
        ids:&mut crate::world::LivingWorldEntityIdAllocator,
        vault:&Arc<crate::attribute_runtime::AttributeRuntimeVault>,entity_key:u64,model_key:u64,waypoint:bool,
        graphs:&crate::actor_construction::graph_startup::ActorGraphStartupOwners,
    )->Result<bool,String>{
        if retained.is_some(){return Err("Prop factory construction already retained".into());}
        let Some(allocation)=self.memory.entity.allocate() else{return Ok(false);};
        let id=ids.allocate(crate::world::LivingWorldEntityType::from_raw(4));
        *retained=Some(Prop{actor_tracker:None,transferable:None,behaviour_sinks:Vec::new(),behaviour_copies:Vec::new(),containing_entity:None,contained_props:Vec::new(),attrib_cache:None,id,allocation,core_allocation:None,core:None,characteristics:None,
            flags:0,pedestrian_spawn_offset:None,pedestrian_spawn_probability:None,pedestrian_spawn_timer:0.,
            default_pedestrian_spawn_timeout:None,transfer_radius:None,test_nav_mesh_offset:None,waypoint:None});
        let prop=retained.as_mut().expect("Prop retained");
        prop.core_allocation=Some(self.memory.core.allocate().ok_or("Prop core store exhausted")?);
        let mut core=CommonEntityCore::new();core.entity_type=4;core.entity_key=entity_key;core.model_key=model_key;
        prop.core=Some(core);
        let entity=vault.instance(0x60d7_03c7_7cab_1631,entity_key);
        let key=entity.try_ref_spec_collection(0xa815_26f5_cf7b_d9c1).map_err(|e|e.to_string())?.unwrap_or(0);
        prop.characteristics=Some(vault.instance(0x5cac_3784_5e0c_4722,key));
        let attributes=prop.characteristics.as_ref().expect("characteristics retained");
        let layout_byte=|at|->Result<u8,String>{Ok(attributes.try_cached_layout_field(at,1)
            .map_err(|e|e.to_string())?.map_or(0,|data|data.bytes()[0]))};
        let scalar=|key|->Result<f32,String>{match attributes.try_get_data(key,0).map_err(|e|e.to_string())?{
            None=>Ok(0.),Some(data)=>Ok(f32::from_be_bytes(data.bytes().get(..4).ok_or("Prop scalar truncated")?
                .try_into().expect("four bytes")))}};
        let layout_scalar=|at|->Result<f32,String>{match attributes.try_cached_layout_field(at,4).map_err(|e|e.to_string())?{
            None=>Ok(0.),Some(data)=>Ok(f32::from_be_bytes(data.bytes().try_into().expect("four bytes")))}};
        if layout_byte(20)?!=0{
            let path=attributes.try_get_text(0x48bd_5e5a_1c5e_e7a1,0).map_err(|e|e.to_string())?
                .ok_or("Prop behaviour descriptor path absent")?;
            let path=std::str::from_utf8(path.bytes()).map_err(|e|e.to_string())?.trim_end_matches('\0');
            prop.transferable=Some(plugin::load::load(graphs,path)?);
            let mut sources=graphs.plugin_sources.lock().map_err(|_|"BehaviourMan source list lock")?;
            if sources.len()==1024{return Err("BehaviourMan source list native node capacity".into());}
            sources.push(id);
        }
        if attributes.try_get_data(0x9c94_6211_beb7_673c,0).map_err(|e|e.to_string())?
            .is_some_and(|data|data.bytes().first().copied().unwrap_or(0)!=0){
            return Err("Prop requires concrete AI graph loader".into());
        }
        prop.flags=(prop.flags&!0x40)|((layout_byte(9)?<<6)&0x40);
        prop.pedestrian_spawn_offset=Some(scalar(0x5762_9021_3ce1_ce69)?);
        prop.pedestrian_spawn_probability=Some(layout_scalar(16)?);
        prop.default_pedestrian_spawn_timeout=Some(layout_scalar(12)?);
        prop.transfer_radius=Some(layout_scalar(4)?);
        prop.test_nav_mesh_offset=Some(scalar(0x4321_9fe4_7ad6_da93)?);
        prop.flags=(prop.flags&!0x20)|((layout_byte(8)?<<5)&0x20);
        let final_flag=attributes.try_get_data(0xee83_af04_1d2a_7661,0).map_err(|e|e.to_string())?
            .and_then(|data|data.bytes().first().copied()).unwrap_or(0);
        prop.flags=(prop.flags&!0x80)|(final_flag<<7);
        if waypoint {prop.waypoint=Some(WaypointGroup::construct(&mut self.memory)?);}
        Ok(true)
    }
    pub(crate) fn initialize()->Result<Self,String>{
        let mut entities=EntityMan::try_construct().map_err(|e|format!("PropMan: {e:?}"))?;
        entities.try_initialize().map_err(|e|format!("PropMan: {e:?}"))?;
        Ok(Self{actor_tracker:None,entities,memory:PropMemory::new(),execution:crate::living_world_grind_services::execution::ExecutionMan::new(),unfinished:None,
            waypoint_guids:BTreeMap::new(),waypoint_chunks:BTreeMap::new(),factory_callbacks:None,culls:Vec::new(),failed_containers:Vec::new()})
    }
    pub(crate) fn group(&self,id:u32)->Result<&WaypointGroup,String>{
        self.entities.get(id).ok_or("waypoint Prop absent")?.waypoint.as_ref().ok_or("Prop is not a waypoint group".into())
    }
    /// Manager.RegisterWaypointData attaches source data to the actual spawned
    /// Prop. Duplicate GUIDs preserve the previous map entry after SetData.
    pub(crate) fn attach_waypoint_data(&mut self,id:u32,source:WaypointGroupSource,
        guids:&mut WaypointGuidMap)->Result<(),String>{
        let prop=self.entities.get_mut(id).ok_or("spawned waypoint Prop absent")?;
        let group=prop.waypoint.as_mut().ok_or("spawned Prop lacks IWaypointGroup")?;
        group.set_data(Some(source),&mut self.memory)?;
        guids.entry(group.guid()?).or_insert(id);
        Ok(())
    }
    /// NotifyPedsAboutWaypointDeRegister runs over the one actual pedestrian
    /// map, in native bucket order, before any waypoint data is released.
    pub(crate) fn notify_waypoint_deregister(&mut self,guid:u64,pedestrians:&mut EntityMan<40,
        crate::living_world_pedestrian_entity::PedestrianConstruction>)->Result<(),String>{
        for id in pedestrians.ids(){
            pedestrians.get_mut(id).ok_or("pedestrian map changed during waypoint notification")?
                .notify_waypoint_deregister(guid,self)?;
        }
        Ok(())
    }
    /// AudioSystem.CacheCrowdPodInfos824fd6c0 over GetPropsWithinRange82e698a8 (at most thirty
    /// props within 1000 in x and z): each waypoint_spectator group (LWBasicEntity.GetType) with
    /// at least two locked waypoints, as its centroid and that count.
    pub(crate) fn crowd_pod_infos(&self,position:[f32;2])->Result<Vec<([f32;4],i32)>,String>{
        let found=self.entities.find_near(position,1000.,30,&mut crate::reckoning_numeric::HostReckoningNumeric)
            .map_err(|e|format!("crowd pod props: {e:?}"))?;
        let spectator=skate2_formats::attrib::string_hash64("waypoint_spectator");
        let mut pods=Vec::new();
        for (id,_) in found{
            let Some(prop)=self.entities.get(id) else{continue;};
            let Some(group)=prop.waypoint.as_ref() else{continue;};
            if prop.core.as_ref().map(|core|core.entity_key)!=Some(spectator){continue;}
            let locked=group.points.iter().filter(|point|point.locking_entity.is_some()).count();
            if locked>=2{pods.push((group.centroid,locked as i32));}
        }
        Ok(pods)
    }
    pub(crate) fn group_mut(&mut self,id:u32)->Result<&mut WaypointGroup,String>{
        self.entities.get_mut(id).ok_or("waypoint Prop absent")?.waypoint.as_mut().ok_or("Prop is not a waypoint group".into())
    }
    pub(crate) fn point(&self,reference:WaypointRef)->Result<&Waypoint,String>{
        self.group(reference.prop)?.points.get(reference.index as usize).ok_or("waypoint index absent".into())
    }
    pub(crate) fn point_mut(&mut self,reference:WaypointRef)->Result<&mut Waypoint,String>{
        self.group_mut(reference.prop)?.points.get_mut(reference.index as usize).ok_or("waypoint index absent".into())
    }
}

/// Manager's existing GUID/chunk maps. Values reference the one Prop roster.
pub(crate) type WaypointGuidMap=BTreeMap<u64,u32>;
pub(crate) type WaypointChunkMap=BTreeMap<u64,WaypointAsset>;
