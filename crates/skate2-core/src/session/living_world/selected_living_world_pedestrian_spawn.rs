//! Native Spawn82e9a510 admission on existing session factory and roster owners.
use crate::{
    living_world_grind_services::{LivingWorldGrindServices, census::pedestrian::PedestrianSpawnInfo},
    living_world_pedestrian_entity::factory::construct_retained,
    living_world_mesh_store::MeshCell,
    reckoning_numeric::ReckoningNumeric,
    skeleton_root_transforms::SkeletonRootVector,
};
use super::SelectedRecipeFrame;

fn error(e: impl std::fmt::Debug) -> String { format!("pedestrian spawn: {e:?}") }
fn trace_admission(stage: &str, position: [f32;4], value: f32) {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    static CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    if *ENABLED.get_or_init(||std::env::var_os("SKATE2_TRACE_PEDESTRIANS").is_some()) {
        let n=CALLS.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
        if n<16 || n%120==0 {std::eprintln!("pedestrian admission={n} stage={stage} position={position:?} value={value}");}
    }
}
impl SelectedRecipeFrame<'_> {
    pub(crate) fn spawn_pedestrian<N: ReckoningNumeric>(&self,
        living: &mut LivingWorldGrindServices,
        world: &mut crate::physical_player_assembly::PhysicalAssemblyWorld,
        info: &PedestrianSpawnInfo, cvars: &crate::input_cvar_runtime::InputCvarReferences,
        painter: &crate::world_painter_layer::WorldPainterLayerMan,
        numeric: &mut N,
    ) -> Result<Option<u32>, String>
    where N::Error: std::fmt::Debug {
        if self.prepare_pedestrian_spawn(living,world,info,cvars,numeric)?.is_none() { return Ok(None); }
        let entity = living.pedestrian_spawn_unfinished.as_mut().expect("retained prepared spawn");
        // Spawn82e9aad4/aad8 loads ACTIVE_BODY (4) into both body-state
        // arguments. STATIC_BODY (1) prevents the admitted collision bodies
        // from integrating the animation-driven velocities as the ped walks.
        const ACTIVE_BODY: u32 = 4;
        entity.add_collision_models_to_world(world,info.allow_world_collisions,
            ACTIVE_BODY,ACTIVE_BODY,cvars,living.props.as_deref())?;
        entity.core.as_mut().ok_or("pedestrian core absent")?.allow_culling = info.allow_culling;
        let parent_id = entity.id;
        let contained_transform = entity.core.as_ref().ok_or("pedestrian core absent")?.target_transform;
        if cvars.try_allow_contained_props().map_err(error)? {
            let attributes = entity.resources.as_ref().ok_or("pedestrian resources absent")?.entity.clone();
            if attributes.try_length(0x3eb7_7fb8_ca81_c99b).map_err(error)? > 8 {
                return Err("pedestrian contained prop capacity8 exceeded".into());
            }
            let mut props = self.prop_recipe_frame();
            let mut index = 0;
            while index < attributes.try_length(0x3eb7_7fb8_ca81_c99b).map_err(error)? {
                let data = attributes.try_array_field_data(0x3eb7_7fb8_ca81_c99b,index,
                    0x97c5_113d_e9bd_1ad6,24,8..16).map_err(error)?;
                let key = data.map_or(0,|data|u64::from_be_bytes(data.bytes().try_into().expect("RefSpec collection")));
                let child = props.spawn_waypoint_prop(living,key,contained_transform,false,cvars)?
                    .ok_or("native InitContainedProps child spawn assertion")?;
                living.props.as_deref_mut().ok_or("PropMan absent")?.entities.get_mut(child)
                    .ok_or("contained Prop absent after admission")?.containing_entity = Some(parent_id);
                living.pedestrian_spawn_unfinished.as_mut().ok_or("retained pedestrian spawn absent")?
                    .contained_props.push(child);
                index += 1;
            }
        }
        let entity = living.pedestrian_spawn_unfinished.as_mut().ok_or("retained pedestrian spawn absent")?;
        // Ordinary Entity vslot0x94 ->825187c0 is empty. Guard overrides it.
        if entity.security_guard.is_some() {
            entity.post_initialize_security_guard(painter)?;
        }
        let id = entity.id;
        let losim = entity.core.as_ref().ok_or("pedestrian core absent")?.current_losim;
        // AttachToManagers82e92b30: map insertion precedes execution group.
        living.pedestrian_entities.as_mut().ok_or("pedestrian EntityMan absent")?
            .add_retained(id,&mut living.pedestrian_spawn_unfinished).map_err(error)?;
        if living.pedestrian_spawn_unfinished.is_some() { return Err("duplicate pedestrian ID".into()); }
        living.pedestrian_execution.add(losim,id).map_err(error)?;
        living.pedestrian_on_spawn_complete(id)?;
        // CensusMan.SpawnPedestrian82721424..2c sets entity byte6724 bit08
        // only after RequestSpawn has completed and returned the live entity.
        living.pedestrian_entities.as_deref_mut().ok_or("pedestrian EntityMan absent")?
            .get_mut(id).ok_or("completed pedestrian spawn disappeared")?
            .ignore_nav_mesh_in_ground_test = info.ignore_navmesh_ground;
        Ok(Some(id))
    }
    pub(crate) fn prepare_pedestrian_spawn<N: ReckoningNumeric>(&self,
        living: &mut LivingWorldGrindServices,
        world: &mut crate::physical_player_assembly::PhysicalAssemblyWorld,
        info: &PedestrianSpawnInfo, cvars: &crate::input_cvar_runtime::InputCvarReferences,
        numeric: &mut N,
    ) -> Result<Option<(MeshCell, crate::living_world_rain::geometry::Transform)>, String>
    where N::Error: std::fmt::Debug {
        if living.pedestrian_spawn_unfinished.is_some() {
            return Err("pedestrian factory has unfinished spawn".into());
        }
        let mesh = living.try_mesh_store().map_err(error)?;
        let cell = mesh.try_get_cell(SkeletonRootVector::known(info.transform[3])).map_err(error)?;
        if !info.force_spawn && (!cell.is_valid() || cell.score_with_threshold(0.) > 0.) {
            trace_admission(if cell.is_valid(){"nav-score"}else{"nav-absent"},info.transform[3],cell.score_with_threshold(0.));
            return Ok(None);
        }
        trace_admission("nav-accepted",info.transform[3],cell.score_with_threshold(0.));
        let mut transform = info.transform;
        if cell.is_valid() && !info.ignore_navmesh_ground {
            if let Some(y) = cell.height_at(info.transform[3], numeric)? { transform[3][1] = y; }
        }
        let point = [info.transform[3][0], info.transform[3][2]];
        let entities = living.pedestrian_entities.as_ref().ok_or("pedestrian EntityMan absent")?;
        let near = entities.find_near(point, cvars.try_pedestrian_spawn_density().map_err(error)?,40,numeric).map_err(error)?;
        if !near.is_empty() && !info.force_spawn {
            trace_admission("density",info.transform[3],near.len() as f32);
            return Ok(None);
        }
        if !info.force_spawn {
            let dmos = living.dmo_objects.as_ref().ok_or("DMO entity manager absent")?;
            for (id, _) in dmos.try_objects_within_range(point,
                cvars.try_pedestrian_spawn_dmo_check_radius().map_err(error)?,64).map_err(error)? {
                let Some(object) = dmos.object(id) else { continue; };
                let centered = object.model.bbox_transform(&object.transform);
                let delta = [transform[3][0]-centered[12],transform[3][1]-centered[13],transform[3][2]-centered[14],0.];
                // IDynamicObject vslot40 is GetTransform82d1d200, not an
                // inverse. Preserve native X multiply, Y FMA, Z FMA order.
                let basis = &object.transform;
                let x = basis[8].mul_add(delta[2],basis[4].mul_add(delta[1],basis[0]*delta[0]));
                let z = basis[10].mul_add(delta[2],basis[6].mul_add(delta[1],basis[2]*delta[0]));
                let padding = cvars.try_pedestrian_spawn_dmo_padding().map_err(error)?;
                if object.model.dimensions[0]*0.5+padding >= x.abs()
                    && object.model.dimensions[2]*0.5+padding >= z.abs() {
                    trace_admission("dmo-overlap",info.transform[3],0.);
                    return Ok(None);
                }
            }
        }
        self.pedestrian_animation.initialize_construct_info(&mut living.pedestrian_animation_construction)?;
        let cache = living.census_targets.get(info.model_key,self.vault).map_err(error)?;
        let construction = &living.pedestrian_animation_construction;
        let bind = construction.bind_pose.clone().ok_or("pedestrian construction bind pose absent")?;
        let hierarchy = construction.hierarchy.clone().ok_or("pedestrian construction hierarchy absent")?;
        let pool = construction.motion_graph_pool;
        if !construct_retained(&mut living.pedestrian_spawn_unfinished,
            living.pedestrian_memory.as_mut().ok_or("pedestrian memory absent")?,
            &mut living.entity_ids,self.vault,cache,info.entity_key,info.model_key,world,bind,hierarchy,
            |entity,memory,_world| self.pedestrian_animation.initialize_entity(entity,memory,pool,cvars,self.graphs),
            cvars,info.allow_hand_prop)? {
            trace_admission("factory",info.transform[3],0.);
            return Ok(None);
        }
        living.pedestrian_animation_construction.increment_motion_graph_pool();
        let entity = living.pedestrian_spawn_unfinished.as_mut().expect("retained pedestrian constructor");
        let has_model = entity.initialize_models(living.pedestrian_memory.as_mut().expect("retained memory"),
            info.recipe_id,self.vault,&living.dmo_models,&mut living.vehicle_collideables,world,
            |entity,path| entity.load_graph(self.graphs,path))?;
        if !has_model {
            // No model means InitCollisionBodies admitted no bodies; graph load
            // has not performed Begin. Native destroys this unpublished prefix.
            if let Some(collision) = &mut entity.collision { collision.destroy_bodies(world)?; }
            let memory = living.pedestrian_memory.as_mut().expect("retained memory");
            for &(store,slot) in entity.allocations.iter().rev() {
                if !memory.release(store,slot) { return Err("pedestrian rejected spawn allocation release".into()); }
            }
            living.pedestrian_spawn_unfinished = None;
            return Ok(None);
        }
        if cell.is_valid() {
            let position = entity.core.as_ref().ok_or("pedestrian core absent")?.transform[3];
            entity.nav_rig.as_mut().ok_or("pedestrian nav rig absent")?.mesh_info.set_cell(cell.clone(),position,||Ok(0.))?;
        }
        entity.core.as_mut().ok_or("pedestrian core absent")?.set_target_transform::<std::convert::Infallible>(transform).map_err(error)?;
        Ok(Some((cell,transform)))
    }
}
