//! Completed local actor / streamed WorldPainter inputs for native HUDs.
use crate::physics_state_ids::PHYSICS_CATEGORY_OFFBOARD;
use super::*;

impl<T,C> SelectedLiveSession<T,C> {
    pub(super) fn try_minimap_frame(&self) -> Result<crate::ui::minimap::Frame, String> {
        let actor = self.inner.player.as_ref().and_then(|p|p.selected.actor.as_ref())
            .ok_or("minimap local actor absent")?;
        let physical = actor.cached_physical_allocation().map_err(|e|format!("{e:?}"))?;
        let output = physical.owner().phys_out().lock().map_err(|_|"minimap PhysOut lock")?;
        let skeleton = output.try_skeleton_output().map_err(|e|format!("{e:?}"))?;
        let actor_position = skeleton.skeleton_root_matrix.columns()[3].xyz;
        let actor_heading=crate::ui::minimap::entity_heading(skeleton.skeleton_root_matrix.columns()[2].xyz)?;
        let board = output.try_camera_board_output().map_err(|e|format!("{e:?}"))?;
        // FillSkaterData827f4c48 reads category OFFBOARD, Skeleton+0x50
        // (HipsMatY), and SystemReckoning+0x10 (UNDAMPED velocity).
        let state=output.try_state_output().map_err(|e|format!("{e:?}"))?;
        let offboard = state.physics_state_category==PHYSICS_CATEGORY_OFFBOARD;
        let look = if offboard {skeleton.skeleton_hips_mat_y.xyz}
            else {board.deck_nose_dir.xyz};
        let velocity = output.try_camera_system_vector(crate::physical_player_phys_out::PhysOutSystemVector::CentreOfMassVelocity)
            .map_err(|e|format!("{e:?}"))?.xyz;
        let wiping_out = state.is_wiping_out;
        drop(output);
        let hips = self.inner.presentation_runtime.as_ref().and_then(|r|r.published_hips_position())
            .ok_or("minimap presentation hips absent")?;
        let world = self.inner.world.as_ref().ok_or("minimap world absent")?;
        let terrain = self.inner.terrain.as_ref().ok_or("minimap terrain absent")?
            .terrain_for(&world.world).map_err(|e|format!("{e:?}"))?;
        let layers = terrain.layers.lock().map_err(|_|"minimap LayerMan lock")?;
        let position = [actor_position[0],actor_position[2]];
        let district_key = layers.try_attrib_key(5,position).map_err(|e|e.to_string())?;
        let security_key = layers.try_attrib_key(6,position).map_err(|e|e.to_string())?;
        drop(layers);
        let district_name = if matches!(district_key,0|u64::MAX|0xd7ed_bd36_2d7d_2152) {None} else {
            let assets = self.inner.assets.as_ref().ok_or("minimap assets absent")?;
            assets.vault.instance(crate::selected_original_assets_bootstrap::map_catalog::DISTRICT_CLASS,district_key)
                .try_cached_layout_text(0).map_err(|e|e.to_string())?
                .map(|text|String::from_utf8(text.bytes().to_vec()).map_err(|e|e.to_string())).transpose()?
                .filter(|text|!text.is_empty())
        };
        // KeyIsNoSkateZoneKey82cea848 compares FULL64-bit keys (cmpld).
        let no_skate_zone_key = ["HighAlert_Area","NoSkateZone_Area"].iter()
            .any(|name|skate2_formats::attrib::string_hash64(name)==security_key);
        let (mode,(_,_,security)) = {
            let simulation=world.world.lock().map_err(|_|"minimap world lock")?;
            let local=simulation.local_skater_identity.as_ref().ok_or("minimap local identity absent")?;
            (local.backend().try_game_mode().map_err(|e|format!("{e:?}"))?,
                local.backend().try_free_skate().map_err(|e|format!("{e:?}"))?)
        };
        // ReceiveMsg(cMsgWorldPainter_KeyChange)8263de80 drops the security key in free skate
        // (IsFreeSkateMode8255b808: modes 0/3/4) with security off; ShowNoSkateZoneMessage826389c0
        // never shows it online (4/5) or in Party Play (2).
        let no_skate_zone = no_skate_zone_key && (!matches!(mode,0|3|4) || security)
            && !matches!(mode,2|4|5);
        let mut dynamic_icons={
            let simulation=world.world.lock().map_err(|_|"minimap world lock")?;
            let route=simulation.sim_rate.lock().map_err(|_|"minimap sim rate lock")?.game_sim_events();
            if let Some(route)=route {
                let mut icons=route.minimap_icons.lock().map_err(|_|"minimap icon lock")?;
                let living=simulation.grind_services.living_world().lock().map_err(|_|"minimap LW lock")?;
                for (security,id,icon) in icons.iter_mut() {
                    if let Some(core)=living.pedestrian_entities.as_ref().and_then(|r|r.get(*id)).and_then(|e|e.core.as_ref()) {
                        // UpdateSecurityIcons82565b78 / UpdatePedChaserIcons82565f60.
                        icon.position=[core.transform[3][0],core.transform[3][2]];
                        if *security {icon.rotation=actor_heading-crate::ui::minimap::entity_heading(
                            core.transform[2][..3].try_into().expect("XYZ"))?;}
                    }
                }
                icons.iter().map(|(_,_,icon)|icon.clone()).collect()
            }else{Vec::new()}
        };
        if let Some(career)=self.career.as_ref() {
            dynamic_icons.extend(career.challenge_icons());
        }
        let main_challenge=self.career.as_ref().is_some_and(|career|career.main_challenge_active());
        Ok(crate::ui::minimap::Frame {position:[hips[0],hips[2]],look:[look[0],look[2]],velocity,
            wiping_out,district_key,district_name,main_challenge,no_skate_zone,dynamic_icons})
    }

    /// `LW::Man`'s event broadcaster, where the front end's handlers keep their state.
    fn game_sim_route(&self) -> Result<Option<std::sync::Arc<crate::hermes_game_sim_queue::GameSimEventRoute>>, String> {
        let Some(world) = self.inner.world.as_ref() else { return Ok(None) };
        let simulation = world.world.lock().map_err(|_| "chase arrow world lock")?;
        let route = simulation.sim_rate.lock().map_err(|_| "chase arrow sim rate lock")?.game_sim_events();
        Ok(route)
    }

    /// `FrontEndManager::Update` (0x82640130) in game: `ChaseArrowMan::Update` (0x82631328)'s count
    /// over the arrows `ReceiveChaseStateMsg` added. A pres pedestrian is a living world pedestrian
    /// with its animated presentation, as the pedestrian visuals take them.
    pub(super) fn try_update_chase_arrows(&self) -> Result<(), String> {
        let Some(world) = self.inner.world.as_ref() else { return Ok(()) };
        let simulation = world.world.lock().map_err(|_| "chase arrow world lock")?;
        let route = simulation.sim_rate.lock().map_err(|_| "chase arrow sim rate lock")?.game_sim_events();
        let Some(route) = route else { return Ok(()) };
        let living = simulation.grind_services.living_world().lock().map_err(|_| "chase arrow LW lock")?;
        let roster = living.pedestrian_entities.as_deref();
        route.chase_arrows.lock().map_err(|_| "chase arrow lock")?.update(crate::ui::chyron::UPDATE_MS, |id| {
            roster.and_then(|roster| roster.get(id)).is_some_and(|entity| entity.renderable
                && entity.animation.as_ref().is_some_and(|animation| animation.presentation.is_some()))
        });
        Ok(())
    }

    /// The arrows `ChaseArrowMan::Render` (0x82631638) draws.
    pub(super) fn try_chase_arrows(&self) -> Result<Vec<crate::ui::chase_arrows::Arrow>, String> {
        let Some(route) = self.game_sim_route()? else { return Ok(Vec::new()) };
        let arrows = route.chase_arrows.lock().map_err(|_| "chase arrow lock")?;
        Ok(arrows.arrows().to_vec())
    }

    /// `FrontEndManager::GameUnload` (0x82628c40): `ChaseArrowMan::RemoveAllArrows` (0x8261cfb8).
    pub(super) fn try_remove_all_chase_arrows(&self) -> Result<(), String> {
        if let Some(route) = self.game_sim_route()? {
            route.chase_arrows.lock().map_err(|_| "chase arrow lock")?.remove_all_arrows();
        }
        Ok(())
    }
}
