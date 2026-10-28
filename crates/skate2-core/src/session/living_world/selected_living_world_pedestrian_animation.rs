//! Borrowed AnimMan services on the actual selected session owners.
use crate::{
    physical_player_assembly::PhysicalAssemblyWorld,
    living_world_pedestrian_entity::{PedestrianConstruction, factory::PedestrianAnimConstructInfo},
    living_world_pedestrian_memory::PedestrianMemory,
    simulation_animation_startup::SimulationAnimationStartup,
    input_cvar_runtime::InputCvarReferences,
    selected_living_world_pedestrian_visual::PedestrianPlaybackStartup,
};
use std::sync::{Arc, Mutex};
#[path = "selected_living_world_pedestrian_target.rs"]
mod target;
#[path = "selected_living_world_pedestrian_presentation.rs"]
pub(crate) mod presentation;

#[derive(Clone, Copy)]
pub(crate) struct PedestrianSessionAnimation<'a> {
    pub(crate) startup: &'a PedestrianPlaybackStartup,
    pub(crate) shared: &'a SimulationAnimationStartup,
    pub(crate) world: &'a Arc<Mutex<PhysicalAssemblyWorld>>,
}
impl PedestrianSessionAnimation<'_> {
    fn commands(&self) -> Result<Arc<Mutex<crate::actor_face_playback::FaceCommands>>, String> {
        Ok(Arc::clone(&self.startup.trajectory.as_ref().ok_or("pedestrian trajectory playback absent")?
            .lock().map_err(|_| "pedestrian trajectory playback lock")?.commands))
    }
    /// Manager.Init8271f83c/844 copies the real AnimMan construction references.
    pub(crate) fn initialize_construct_info(&self, info: &mut PedestrianAnimConstructInfo) -> Result<(), String> {
        if info.bind_pose.is_some() { return Ok(()); }
        if !self.startup.ready { return Err("pedestrian animation startup incomplete".into()); }
        let pose = self.startup.bind_pose.clone().ok_or("pedestrian bind pose absent")?;
        let hierarchy = self.startup.full_database.as_ref().ok_or("pedestrian PRES database absent")?
            .lock().map_err(|_| "pedestrian PRES database lock")?.parsed.as_ref()
            .ok_or("pedestrian PRES hierarchy absent")?.hierarchy.clone();
        info.initialize(pose, Arc::new(hierarchy));
        Ok(())
    }

    /// Called inside the existing factory's animation_and_motion slot.
    pub(crate) fn initialize_entity(&self, entity: &mut PedestrianConstruction,
        memory: &mut PedestrianMemory, motion_pool: u32, cvars: &InputCvarReferences,
        graphs: &crate::actor_construction::graph_startup::ActorGraphStartupOwners) -> Result<(), String> {
        entity.initialize_animation(memory, Arc::clone(self.world),
            self.startup.simulation.clone().ok_or("pedestrian playback absent")?,
            self.startup.bind_pose.clone().ok_or("pedestrian bind pose absent")?,
            self.shared.attributes.clone().ok_or("shared animation attributes absent")?,
            motion_pool, self.startup.simulation_fixed_matrix_id.ok_or("pedestrian fixed matrix absent")?)?;
        let animation = entity.animation.as_mut().expect("factory retained animation");
        // Constructor82e97ec4..ef0 -> tAnimData.ModelType82e93504.
        animation.model_category = entity.resources.as_ref().ok_or("pedestrian resources absent")?
            .model.try_scalar_word(0xf689_7cc7_b637_850c,0).map_err(|e|e.to_string())?.unwrap_or(0);
        let data = animation.selected_animation_data()?;
        let authored = data.try_cached_layout_text(0).map_err(|e| e.to_string())?
            .ok_or("pedestrian motion graph filename absent")?;
        let path = if cvars.try_pedestrian_use_test_motion_graph().map_err(|e| format!("{e:?}"))? {
            b"state/livingworldentities/pedestrian/motiongraph_test.xml".as_slice()
        } else { authored.bytes() };
        let enabled = graphs.living_world_enablement().map_err(|e| format!("pedestrian motion preload: {e:?}"))?;
        let route = crate::generated_onboard_state_graph::select_onboard_controller(path,true,false,enabled)
            .map_err(|e| e.to_string())?;
        let delegate = match route {
            crate::generated_onboard_state_graph::OnboardControllerSelection::GeneratedOther { delegate_index } =>
                crate::pedestrian_motion_delegate::MotionDelegate::from_delegate_index(delegate_index),
            _ => None,
        }.ok_or_else(|| format!("pedestrian motion graph backend absent: {route:?}"))?;
        animation.load_pedestrian_motion(&entity.authority, delegate)
    }
}

impl super::SelectedRecipeFrame<'_> {
    /// LWPedestrianMan.UpdateAnimPostAdjust82e83708, immediately after the
    /// complete head-tracking pass (Manager.UpdatePostTransforms82703260).
    pub(crate) fn update_pedestrian_post_adjust(&self,world:&PhysicalAssemblyWorld,
        cvars:&InputCvarReferences,dt:f32)->Result<(),String>{
        let mut living=world.grind_services.living_world().lock()
            .map_err(|_|"post-adjust LivingWorld lock")?;
        let roster=living.pedestrian_entities.as_deref_mut().ok_or("post-adjust roster absent")?;
        for id in roster.ids(){
            let entity=roster.get_mut(id).ok_or("post-adjust roster member absent")?;
            entity.update_animation_post_adjust(dt,cvars)?;
        }
        Ok(())
    }
    /// LWPedestrianMan.UpdateHeadTracking82e86c58, after collision transforms.
    pub(crate) fn update_pedestrian_head_tracking(&self,world:&mut PhysicalAssemblyWorld,
        cvars:&InputCvarReferences,local:&crate::actor_construction::ActorConstruction,dt:f32)
        ->Result<(),String>{
        let owner=Arc::clone(world.grind_services.living_world());
        let mut living=owner.lock().map_err(|_|"head tracking LivingWorld lock")?;
        let ids=living.pedestrian_entities.as_deref().ok_or("head tracking roster absent")?.ids();
        if ids.is_empty(){return Ok(());}
        let actors=world.entity_roster.as_ref().ok_or("head tracking Actor roster absent")?
            .snapshot().map_err(|e|format!("head tracking Actor roster: {e:?}"))?;
        let triggers=self.triggers.lock().map_err(|_|"head tracking trigger lock")?;
        let avoid=self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
        drop(triggers);
        let painter=self.painter.lock().map_err(|_|"head tracking painter lock")?;
        for id in ids{
            crate::pedestrian_graph::backend::frame::with_entity(&mut living,world,id,cvars,
                &actors,&painter,self.vault,&avoid,|entity,frame|
                    frame.update_head_tracking(entity,dt,local))?;
        }
        Ok(())
    }

    /// Manager.UpdateAnimManager82725538 -> AnimMan.Update82723cd0.
    /// The list is the complete native hash roster, not the AI execution group.
    pub(crate) fn update_pedestrian_animation(&mut self,
        world: &mut PhysicalAssemblyWorld, cvars: &InputCvarReferences,
        simulation_center: [f32;4], camera_position: [f32;4],
        tokens: &mut crate::actor_face_playback::blend_pose_stream::BlendPoseTokenPool,
    ) -> Result<(), String> {
        use crate::reckoning_numeric::HostReckoningNumeric;
        use crate::actor_animation_construction::hands_face::runtime;
        let owner = Arc::clone(world.grind_services.living_world());
        let mut living = owner.lock().map_err(|_| "pedestrian animation LivingWorld lock")?;
        let roster = living.pedestrian_entities.as_deref().ok_or("pedestrian animation roster absent")?;
        let ids = roster.ids();
        let nearby = roster.find_near([camera_position[0],camera_position[2]],30.,40,
            &mut HostReckoningNumeric).map_err(|e|format!("pedestrian animation proximity: {e:?}"))?;
        let mut active = self.pedestrian_animation.startup.active_motion_pool.lock()
            .map_err(|_| "pedestrian motion pool lock")?;
        for &id in &ids {
            living.pedestrian_entities.as_deref_mut().expect("roster retained").get_mut(id)
                .ok_or("pedestrian animation member absent")?.animation.as_mut()
                .ok_or("pedestrian animation absent")?.update_animation_data()?;
        }
        if !ids.is_empty() {
            let actors = world.entity_roster.as_ref().ok_or("pedestrian Actor roster absent")?
                .snapshot().map_err(|e|format!("pedestrian Actor roster: {e:?}"))?;
            let triggers=self.triggers.lock().map_err(|_| "pedestrian trigger lock")?;
            let avoid = self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
            drop(triggers);
            let painter = self.painter.lock().map_err(|_| "pedestrian painter lock")?;
            let pools = cvars.try_enable_pedestrian_motion_graph_pools().map_err(|e|format!("{e:?}"))?;
            let feet = cvars.try_generate_foot_tags_for_all_peds().map_err(|e|format!("{e:?}"))?;
            for &id in &ids {
                crate::pedestrian_graph::backend::frame::with_entity(&mut living,world,id,cvars,
                    &actors,&painter,self.vault,&avoid,|entity,frame| {
                        // AnimUpdateTestLOSim82e94ce0 asks the actual Chasee first.
                        let chasee = entity.state.as_ref().ok_or("pedestrian state absent")?.chaser.chasee;
                        let losim = if frame.big_black_has_threat(entity)? || frame.with_chasee(entity,chasee,|s|s.is_some())? { 0 } else {
                            test_losim(entity,simulation_center,&mut HostReckoningNumeric)?
                        };
                        crate::pedestrian_motion_delegate::frame::update(entity,
                            f32::from_bits(0x3c88_8889),*active,pools,losim,feet,cvars,
                            &mut HostReckoningNumeric,frame)?;
                        Ok(())
                    })?;
            }
            // Resolve the shared job manager before holding any ACS lock.
            let mut source = crate::actor_construction::body_session::ActorBodyAnimationServices::try_from_startup(
                self.pedestrian_animation.shared,cvars).map_err(|e|e.to_string())?;
            let commands = self.pedestrian_animation.commands()?;
            runtime::begin_command_set(&commands).map_err(|e|format!("pedestrian BeginCommandSet: {e:?}"))?;
            let threshold = cvars.try_pedestrian_blend_cull_threshold().map_err(|e|format!("{e:?}"))?;
            let scratch = self.pedestrian_animation.startup.simulation_scratch.as_ref()
                .ok_or("pedestrian full skeleton scratch absent")?;
            for &id in &ids {
                let entity = living.pedestrian_entities.as_deref_mut().expect("roster retained")
                    .get_mut(id).ok_or("pedestrian animation member absent")?;
                let losim = entity.core.as_ref().ok_or("pedestrian core absent")?.current_losim;
                let animation = entity.animation.as_mut().ok_or("pedestrian animation absent")?;
                animation.queue_simulation_pose(losim,animation.model_category==1,true,threshold,
                    Some(cvars.clone()),scratch)?;
            }
            runtime::end_command_set(&commands,&mut source)
                .map_err(|e|format!("pedestrian EndCommandSet: {e:?}"))?;
        }
        // UpdateBlendPoseBuffers826f4470 resets the SAME global token pool.
        // cPedestrianAnim ctor82e903f4 assigns pool0; recursive blends use pool1.
        tokens.reset();
        for (id,_) in nearby {
            let Some(entity) = living.pedestrian_entities.as_deref_mut().expect("roster retained").get_mut(id) else {continue};
            let animation = entity.animation.as_mut().ok_or("pedestrian animation absent")?;
            if entity.renderable && animation.has_current_animation() {
                let mut buffer = tokens.buffer(0).map_err(|e|e.to_string())?;
                animation.core.try_eval_hand_face_tokens(&mut buffer,tokens).map_err(|e|e.to_string())?;
                animation.blend_pose_buffer = Some(buffer);
                animation.frame.valid_blend_pose_buffer = true;
            } else {
                animation.frame.valid_blend_pose_buffer = false;
            }
        }
        *active = active.wrapping_sub(1)&1;
        Ok(())
    }

    /// Manager.UpdateSyncAnim82722588. Target transforms follow this fence.
    pub(crate) fn sync_pedestrian_animation(&self,world:&PhysicalAssemblyWorld,
        cvars:&InputCvarReferences)->Result<(),String> {
        let owner = world.grind_services.living_world().lock().map_err(|_|"pedestrian sync LivingWorld lock")?;
        if owner.pedestrian_entities.as_deref().ok_or("pedestrian sync roster absent")?
            .captured_member_count()==0 {return Ok(());}
        drop(owner);
        let mut source = crate::actor_construction::body_session::ActorBodyAnimationServices::try_from_startup(
            self.pedestrian_animation.shared,cvars).map_err(|e|e.to_string())?;
        crate::actor_animation_construction::hands_face::runtime::sync_command_set(
            &self.pedestrian_animation.commands()?,&mut source)
            .map_err(|e|format!("pedestrian Sync: {e:?}"))?;
        let mut living=world.grind_services.living_world().lock().map_err(|_|"pedestrian target LivingWorld lock")?;
        let living=&mut *living;
        let roster=living.pedestrian_entities.as_deref_mut().ok_or("pedestrian target roster absent")?;
        let dmos=living.dmo_objects.as_deref_mut().ok_or("pedestrian target DMO owner absent")?;
        for id in roster.ids(){
            roster.get_mut(id).ok_or("pedestrian target member absent")?
                .update_animation_target_transform(world,cvars,f32::from_bits(0x3c88_8889),dmos)?;
        }
        Ok(())
    }
}

/// LWBasicEntity.TestLOSim82e50180, also shared by the execution scheduler.
pub(crate) fn test_losim<N:crate::reckoning_numeric::ReckoningNumeric>(entity:&PedestrianConstruction,
    center:[f32;4],numeric:&mut N)->Result<u32,String> where N::Error:std::fmt::Debug {
    entity.core.as_ref().ok_or("pedestrian LOSim core absent")?
        .test_losim(center, &entity.attrib_cache, numeric)
        .map_err(|e|format!("pedestrian LOSim distance: {e:?}"))
}
