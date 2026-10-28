//! MoodMan.ReceiveAmbientMoodEventMsg at the GameSim subscriber boundary.
use crate::{hermes_game_sim_queue::MoodEvent, input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalAssemblyWorld,static_ground_pool::StaticGroundPool};
use std::sync::Arc;
pub(crate) struct SelectedMoodFrame<'a> {
    pub(crate) triggers:&'a Arc<std::sync::Mutex<crate::trigger_camera_group::CameraTriggerGroup>>,
    pub(crate) runtime:&'a super::SelectedLivingWorldRecipes,
    pub(crate) vault:&'a Arc<crate::attribute_runtime::AttributeRuntimeVault>,
    pub(crate) painter:&'a Arc<std::sync::Mutex<crate::world_painter_layer::WorldPainterLayerMan>>,
}
impl super::SelectedRecipeFrame<'_> {
    pub(crate) fn dispatch_game_sim(&mut self,world:&mut PhysicalAssemblyWorld,cvars:&InputCvarReferences,ground:&StaticGroundPool)->Result<(),String>{
        SelectedMoodFrame{triggers:self.triggers,runtime:self.runtime,vault:self.vault,painter:self.painter}
            .dispatch_game_sim(world,cvars,ground)
    }
}
impl SelectedMoodFrame<'_> {
    /// Simulation.Update1 and paused frontend share this registered-subscriber
    /// flush, with the same complete owners and a single ordered queue4.
    pub(crate) fn dispatch_game_sim(&mut self,world:&mut PhysicalAssemblyWorld,cvars:&InputCvarReferences,ground:&StaticGroundPool)->Result<(),String>{
        let rate=Arc::clone(&world.sim_rate);
        let events=rate.lock().map_err(|_|"GameSim rate lock")?.game_sim_events();
        let Some(events)=events else{return rate.lock().map_err(|_|"GameSim rate lock")?.try_dispatch().map_err(|e|e.to_string());};
        events.dispatch_with(|message| {
            use crate::hermes_game_sim_queue::LivingWorldMessage;
            match message.payload {
                LivingWorldMessage::SetSimRate{sim_time,ticks}=>rate.lock().map_err(|_|"GameSim rate lock")?
                    .try_on_request_set_sim_rate(crate::simulation_sim_rate::SetSimRate{sim_time:f32::from_bits(sim_time),ticks_of_slow_motion:ticks})
                    .map_err(|e|e.to_string())?,
                LivingWorldMessage::Mood(event) if events.mood_man_registered()=>self.deliver_ambient_mood(world,cvars,ground,&event)?,
                LivingWorldMessage::SecuritySpawned(id)|LivingWorldMessage::ChaseStart(id)=>{
                    let security=matches!(message.payload,LivingWorldMessage::SecuritySpawned(_));
                    let mut icons=events.minimap_icons.lock().map_err(|_|"minimap icon lock")?;
                    // AddSecurityIcon825658e0 / AddPedChaserIcon82565ce0:
                    // each pool has ten entries; duplicate entity IDs are ignored.
                    if icons.iter().filter(|(s,_,_)|*s==security).count()<10 &&
                        !icons.iter().any(|(s,i,_)|*s==security&&*i==id) {
                        let living=world.grind_services.living_world().lock().map_err(|_|"minimap pedestrian lock")?;
                        if let Some(core)=living.pedestrian_entities.as_ref().and_then(|r|r.get(id)).and_then(|e|e.core.as_ref()) {
                            let key=if security {0xd3de_fd45_3e19_109a} else {0x2877_117d_094b_a7bc};
                            let instance=self.vault.instance(crate::selected_original_assets_bootstrap::map_catalog::ICON_CLASS,key);
                            let name=instance.try_cached_layout_text(0).map_err(|e|e.to_string())?.ok_or("dynamic minimap icon texture absent")?;
                            let name=String::from_utf8(name.bytes().to_vec()).map_err(|e|e.to_string())?;
                            let maximum_range=f32::from_bits(instance.try_scalar_word(0x2cf2_5b33_b439_ddcd,0).map_err(|e|e.to_string())?.unwrap_or(0));
                            // This overload receives activity=null and preserves
                            // MapIcon ctor's miniMapVisible=true (82634548).
                            icons.push((security,id,crate::ui::minimap::Icon {
                                texture:format!("{name}.Texture"),position:[core.transform[3][0],core.transform[3][2]],
                                rotation:if security {crate::ui::minimap::entity_heading(core.transform[2][..3].try_into().expect("XYZ"))?}else{0.},
                                scale:1.,visible:true,fade_at_edge:true,maximum_range,pinch_point:false,
                            }));
                        }
                    }
                },
                LivingWorldMessage::SecurityCulled(id)|LivingWorldMessage::ChaseEnd(id)=>{
                    let security=matches!(message.payload,LivingWorldMessage::SecurityCulled(_));
                    events.minimap_icons.lock().map_err(|_|"minimap icon lock")?
                        .retain(|(s,i,_)|*s!=security||*i!=id);
                },
                LivingWorldMessage::WaypointGroupAdded(guid)=>{
                    if let Some(groups)=events.challenge_waypoint_groups.lock()
                        .map_err(|_|"challenge waypoint group lock")?.as_mut(){groups.push(guid);}
                },
                // FrontEndManager::StartInGame registers ReceiveChaseStateMsg82634db8 on
                // LW::Man's broadcaster.
                LivingWorldMessage::ChaserState{entity,state}=>events.chase_arrows.lock()
                    .map_err(|_|"chase arrow lock")?.add_arrow(entity,state),
                // This selected startup registers Simulation's rate handler
                // and MoodMan, plus the FE minimap and chase handlers above. Native
                // EventSys consumes unmatched career/Lua events.
                _=>{},
            }
            Ok(())
        })?;
        Ok(())
    }

    /// Deliver one registered Mood message, preserving the native category,
    /// local-victim/KD40 recipient selection and per-recipient mutation order.
    /// The queue owner controls when this callback runs and consumes the message.
    pub(crate) fn deliver_ambient_mood(&mut self,world:&mut PhysicalAssemblyWorld,
        cvars:&InputCvarReferences,ground:&StaticGroundPool,event:&MoodEvent)->Result<(),String>{
        let owner=Arc::clone(world.grind_services.living_world());
        let mut living=owner.lock().map_err(|_|"ambient mood Manager lock")?;
        let recipients=living.ambient_mood_recipients(event)?;
        if recipients.is_empty(){return Ok(());}
        let local=world.local_skater_identity.as_ref().ok_or("ambient mood SkaterManager absent")?
            .try_local_index().map_err(|e|format!("ambient mood local index: {e:?}"))?;
        let actors=world.entity_roster.as_ref().ok_or("ambient mood Actor roster absent")?
            .snapshot().map_err(|e|format!("ambient mood Actor roster: {e:?}"))?;
        let triggers=self.triggers.lock().map_err(|_|"ambient mood trigger lock")?;
        let avoid=self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
        drop(triggers);
        let painter=self.painter.lock().map_err(|_|"ambient mood painter lock")?;
        for id in recipients {
            // Native GetPedestrian may fail after the KD query; it skips only
            // that recipient and retains previous successful deliveries.
            if !living.pedestrian_entities.as_ref().is_some_and(|r|r.get(id).is_some()){continue;}
            crate::pedestrian_graph::backend::frame::with_entity(&mut living,world,id,cvars,&actors,
                &painter,self.vault,&avoid,|entity,frame|frame.receive_ambient_mood(entity,event,local,ground))?;
        }
        Ok(())
    }
}
