//! Live CensusMan adapter; entities and RNG remain the existing Manager owners.
use super::SelectedRecipeFrame;
use crate::{living_world_grind_services::{LivingWorldGrindServices,
    census::pedestrian::{PedestrianCensusFrame,PedestrianCensusServices,PedestrianCullEntry,PedestrianSpawnInfo}},
    physical_player_assembly::PhysicalAssemblyWorld,input_cvar_runtime::InputCvarReferences,
    reckoning_numeric::ReckoningNumeric};
use std::sync::Arc;

struct Services<'a,'b> {
    frame: &'a SelectedRecipeFrame<'b>,
    living: &'a mut LivingWorldGrindServices,
    world: &'a mut PhysicalAssemblyWorld,
    cvars: &'a InputCvarReferences,
    painter: &'a crate::world_painter_layer::WorldPainterLayerMan,
    ground:&'a crate::static_ground_pool::StaticGroundPool,
    simulation_random:Option<&'a mut crate::native_skater_random::SimulationCommandRandom>,
}
impl<N:ReckoningNumeric> PedestrianCensusServices<N> for Services<'_, '_>
where N::Error:std::fmt::Debug {
    fn count(&self)->usize {self.living.pedestrian_entities.as_ref().map_or(0,|p|p.captured_member_count())}
    fn contains_model(&self,recipe:u64)->bool {self.living.dmo_models.character_model(recipe).is_some()}
    fn request_spawn(&mut self,info:PedestrianSpawnInfo,numeric:&mut N)->Result<Option<u32>,String>{
        self.frame.spawn_pedestrian(self.living,self.world,&info,self.cvars,self.painter,numeric)
    }
    fn cull_entries(&self)->Vec<PedestrianCullEntry>{
        let Some(entities)=self.living.pedestrian_entities.as_ref() else{return vec![]};
        entities.ids().into_iter().map(|id|{
            let entity=entities.get(id).expect("retained map order");
            let core=entity.core.as_ref().expect("published pedestrian core");
            PedestrianCullEntry{id,position:core.transform[3],allow_culling:core.allow_culling,
                security_guard:entity.security_guard.is_some()}
        }).collect()
    }
    fn request_cull(&mut self,id:u32,random:&mut crate::native_skater_random::NativeSkaterRandom)->Result<(),String>{
        self.frame.request_pedestrian_cull(self.living,self.world,id,self.cvars,self.painter,random)
    }
    fn set_security_outside_cull_radius(&mut self,id:u32,outside:bool)->Result<(),String>{
        self.living.pedestrian_entities.as_mut().and_then(|p|p.get_mut(id))
            .and_then(|p|p.security_guard.as_mut()).ok_or("pedestrian security cull owner absent")?
            .set_is_outside_cull_radius(outside);
        Ok(())
    }
    fn prop_driven_spawn(&mut self,census:&mut PedestrianCensusFrame<'_>,center:[f32;4],inner:f32,outer:f32,initial:bool,numeric:&mut N)->Result<(),String>{
        use crate::living_world_grind_services::census::targets::{PopulationKind,prop_target};
        let found=self.living.props.as_ref().ok_or("PropMan absent")?.entities
            .find_near([center[0],center[2]],outer,256,numeric).map_err(|e|format!("Prop census query: {e:?}"))?;
        let ignore=self.cvars.try_ignore_min_dist_for_prop_spawn().map_err(|e|format!("prop spawn cvar: {e:?}"))?;
        let mut remaining=if initial{600}else{1};
        for (source,distance) in found {
            if remaining==0{break;}
            if (initial||!ignore)&&distance<inner{continue;}
            let Some(prop)=self.living.props.as_mut().ok_or("PropMan absent")?.entities.get_mut(source) else{continue;};
            let Some((_point,position,orientation))=prop.prepare_pedestrian_spawn()? else{continue;};
            let offset=prop.pedestrian_spawn_offset.ok_or("Prop spawn offset absent")?;
            let source_key=prop.core.as_ref().ok_or("Prop core absent")?.entity_key;
            let count=self.living.pedestrian_entities.as_ref().ok_or("pedestrian roster absent")?.captured_member_count() as u32;
            let point=census.census.point_info::<std::convert::Infallible>(PopulationKind::Pedestrian,
                [position[0],position[1],position[2]],census.vault,census.painter,census.cvars)
                .map_err(|e|format!("prop point census: {e:?}"))?;
            if point.spawn_minimum==0||count>=point.spawn_minimum||point.group==0{continue;}
            let Some(key)=prop_target(point.group,source_key,census.vault)? else{continue;};
            let look=std::array::from_fn(|i|if offset>0.{position[i]-orientation[i]}else{position[i]+orientation[i]});
            let transform=crate::reckoning_numeric::try_look_at_up_eye(look,[0.,1.,0.,0.],position,numeric)
                .map_err(|e|format!("prop pedestrian transform: {e:?}"))?;
            let allow_hand_prop=!self.living.props.as_ref().ok_or("PropMan absent")?.entities.get(source)
                .ok_or("Prop disappeared")?.spawns_hand_props_on_use()?;
            let Some(id)=census.spawn(self,key,transform,initial,true,true,true,false,allow_hand_prop,numeric)?
                else{continue;};
            let actors=self.world.entity_roster.as_ref().ok_or("prop spawn Actor roster absent")?
                .snapshot().map_err(|e|format!("prop spawn Actor roster: {e:?}"))?;
            let triggers=self.frame.triggers.lock().map_err(|_|"prop spawn triggers lock")?;
            let avoid=self.frame.runtime.challenge_avoid_volumes.resolve(&triggers)?;
            drop(triggers);
            let accepted=crate::pedestrian_graph::backend::frame::with_entity_and_random(
                self.living,self.world,id,self.cvars,&actors,self.painter,self.frame.vault,&avoid,census.random,
                |entity,frame|{
                    if !super::transfer::check_transfer_conditions(frame,entity,source,self.ground,self.simulation_random.as_deref_mut())?{
                        return Ok(false);
                    }
                    frame.props.transfer_behaviour_to_pedestrian(source,entity)?;
                    Ok(true)
                })?;
            if accepted {
                let prop=self.living.props.as_mut().ok_or("PropMan absent")?.entities.get_mut(source).ok_or("Prop disappeared")?;
                prop.pedestrian_spawn_timer=prop.default_pedestrian_spawn_timeout.ok_or("Prop timeout absent")?;
                remaining-=1;
            } else {
                self.frame.request_pedestrian_cull(self.living,self.world,id,self.cvars,self.painter,census.random)?;
            }
        }
        Ok(())
    }
}
impl SelectedRecipeFrame<'_> {
    pub(crate) fn evaluate_pedestrians(&self,world:&mut PhysicalAssemblyWorld,
        cvars:&InputCvarReferences,inputs:&crate::living_world_grind_services::dmo_objects::DmoCensusInputs,
        cull:bool,spawn:bool,ground:&crate::static_ground_pool::StaticGroundPool,
        simulation_random:Option<&mut crate::native_skater_random::SimulationCommandRandom>)->Result<(),String>{
        let owner=Arc::clone(world.grind_services.living_world());
        let mut living=owner.lock().map_err(|_|"LivingWorld lock")?;
        if living.pedestrian_entities.is_none(){return Err("pedestrian EntityMan absent".into());}
        // Only the immutable configuration is sampled. The sole RNG is moved
        // temporarily for disjoint mutable borrows and restored on failure too.
        let census=living.census.clone();
        let mut random=living.random.take().ok_or("CensusMan random absent")?;
        let result=(||{
            let painter=self.painter.lock().map_err(|_|"WorldPainter lock")?;
            let mut services=Services{frame:self,living:&mut living,world,cvars,painter:&painter,ground,simulation_random};
            PedestrianCensusFrame{census:&census,random:&mut random,recipe_man:&self.runtime.recipes,
                vault:self.vault,cvars,painter:&painter,override_model_name:false}
                .evaluate(&mut services,inputs,cull,spawn,&mut crate::reckoning_numeric::HostReckoningNumeric)
        })();
        living.random=Some(random);
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        static CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        if *TRACE.get_or_init(||std::env::var_os("SKATE2_TRACE_PEDESTRIANS").is_some()) {
            let n=CALLS.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
            if inputs.initial || n % 120 == 0 || result.is_err() {
                std::eprintln!("pedestrian census call={n} initial={} spawn={spawn} cull={cull} enable={:#x} max={:?} reserve={} roster={} center={:?} result={result:?}",
                    inputs.initial,census.enable,census.max_pedestrians,census.pedestrian_reserve,
                    living.pedestrian_entities.as_ref().map_or(0,|p|p.captured_member_count()),inputs.center);
            }
        }
        result
    }
    /// `ILWCensusMan::SpawnPedestrian` (CensusMan vtable +4, SpawnPedestrian82720fe0) for a
    /// caller outside the census update, on the same Manager owners and RNG.
    pub(crate) fn spawn_census_pedestrian(&self,world:&mut PhysicalAssemblyWorld,
        cvars:&InputCvarReferences,ground:&crate::static_ground_pool::StaticGroundPool,
        entity_key:u64,transform:crate::living_world_rain::geometry::Transform,initial:bool,
        allow_culling:bool,force_spawn:bool,allow_world_collisions:bool,ignore_navmesh_ground:bool,
        allow_hand_prop:bool)->Result<Option<u32>,String>{
        let owner=Arc::clone(world.grind_services.living_world());
        let mut living=owner.lock().map_err(|_|"LivingWorld lock")?;
        let census=living.census.clone();
        let mut random=living.random.take().ok_or("CensusMan random absent")?;
        let result=(||{
            let painter=self.painter.lock().map_err(|_|"WorldPainter lock")?;
            let mut services=Services{frame:self,living:&mut living,world,cvars,painter:&painter,ground,simulation_random:None};
            PedestrianCensusFrame{census:&census,random:&mut random,recipe_man:&self.runtime.recipes,
                vault:self.vault,cvars,painter:&painter,override_model_name:false}
                .spawn(&mut services,entity_key,transform,initial,allow_culling,force_spawn,
                    allow_world_collisions,ignore_navmesh_ground,allow_hand_prop,
                    &mut crate::reckoning_numeric::HostReckoningNumeric)
        })();
        living.random=Some(random);
        result
    }
}
