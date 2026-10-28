//! LivingWorld::Manager::Reset 8271d3d8 at the game-unload boundary.
use super::{
    ActorConstruction, Arc, InputCvarReferences, PhysicalAssemblyWorld,
    SelectedLivingWorldDmo, SelectedLivingWorldDmoError,
};
use crate::selected_living_world_recipes::SelectedRecipeFrame;

impl SelectedLivingWorldDmo {
    /// GameUnload8273d994 -> ResetGame -> Simulation::Reset8289effc.
    /// Cull through the existing managers so collision bodies, road reservations,
    /// contained props, animation allocations and behaviour sources all retire.
    /// `network_game` is an online game in play: the original's sets `isNetworkGame` and
    /// `sGhostSystemEnabled` (`StateOnlineGame::SetupInitalGameInfo` 0x8275db08), which the
    /// port's, with its own netcode, leaves clear.
    pub(crate) fn try_reset_game(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        actor: &ActorConstruction,
        cvars: &InputCvarReferences,
        recipes: &SelectedRecipeFrame<'_>,
        network_game: bool,
    ) -> Result<(), SelectedLivingWorldDmoError> {
        use SelectedLivingWorldDmoError as E;
        let vault = Arc::clone(self.registered_vault().ok_or(E::MissingManagers)?);
        let backend = world.local_skater_identity.as_ref().ok_or(E::MissingManagers)?.backend();
        let spawn_enabled = !network_game
            && !backend.try_is_network_game().map_err(E::Identity)?
            && !backend.try_ghost_enabled().map_err(E::Identity)?;
        let owner = Arc::clone(world.grind_services.living_world());
        {
            let mut living = owner.lock().map_err(|_| E::LivingWorldLock)?;
            // IsLWSpawnEnabled8289e780: neither a network nor a ghost game.
            living.census.enable = (living.census.enable & 0x7f) | if spawn_enabled { 0x80 } else { 0 };
            living.chase.as_mut().ok_or(E::MissingManagers)?
                .reset(&vault).map_err(E::Pedestrians)?;
            living.mood.as_mut().ok_or(E::MissingManagers)?.reset();
            living.census_targets = crate::living_world_grind_services::attrib_cache::CensusTargetCache::new();

            // ForceAllPluginsUnregistered826f4d18 precedes ANY cull. Only
            // pedestrians own attached plugins in the admitted entity types.
            let ids = living.pedestrian_entities.as_ref().ok_or(E::MissingManagers)?.ids();
            {
                let living = &mut *living;
                let pedestrians = living.pedestrian_entities.as_mut().ok_or(E::MissingManagers)?;
                let props = living.props.as_mut().ok_or(E::MissingManagers)?;
                for &id in &ids {
                    pedestrians.get_mut(id).ok_or(E::MissingManagers)?
                        .remove_plugin(props).map_err(E::Pedestrians)?;
                }
            }
            let painter = recipes.painter.lock()
                .map_err(|_| E::Pedestrians("reset WorldPainter lock".into()))?;
            let mut random = living.random.take().ok_or(E::MissingManagers)?;
            let result = (|| {
                // RequestAllCull826f50d0: pedestrians, vehicles, DMOs, props;
                // false means include challenge-owned, non-census entities.
                for id in ids {
                    living.pedestrian_entities.as_mut().and_then(|p| p.get_mut(id))
                        .and_then(|p| p.core.as_mut()).ok_or(E::MissingManagers)?.allow_culling = true;
                    recipes.request_pedestrian_cull(&mut living, world, id, cvars, &painter, &mut random)
                        .map_err(E::Pedestrians)?;
                }
                Ok(())
            })();
            living.random = Some(random);
            result?;

            let ids = living.grind_vehicle_entities.as_ref().ok_or(E::MissingManagers)?.ids();
            for id in ids {
                living.grind_vehicle_entities.as_mut().and_then(|v| v.get_mut(id))
                    .ok_or(E::MissingManagers)?.core.allow_culling = true;
                living.request_vehicle_cull(id, cvars).map_err(E::Vehicles)?;
            }
            recipes.with_dmo_culls(&mut living, world, cvars, &painter, |objects, locators, cull| {
                for id in objects.ids() { cull(objects, locators, id)?; }
                Ok(())
            }).map_err(E::Pedestrians)?;

            let actors = world.entity_roster.as_ref().ok_or(E::MissingManagers)?
                .snapshot().map_err(|e| E::Pedestrians(format!("reset Actor roster: {e:?}")))?;
            let avoid = {
                let triggers = recipes.triggers.lock()
                    .map_err(|_| E::Pedestrians("reset trigger lock".into()))?;
                recipes.runtime.challenge_avoid_volumes.resolve(&triggers).map_err(E::Pedestrians)?
            };
            let mut random = living.random.take().ok_or(E::MissingManagers)?;
            let result = (|| {
                let ids = living.props.as_ref().ok_or(E::MissingManagers)?.entities.ids();
                for id in ids {
                    // A parent's explicit cull also detaches its contained props.
                    if living.props.as_ref().ok_or(E::MissingManagers)?.entities.get(id).is_some() {
                        recipes.prop_recipe_frame().request_contained_prop_cull(
                            &mut living, world, id, cvars, &painter, &mut random, &actors, &avoid,
                        ).map_err(E::Pedestrians)?;
                    }
                }
                Ok(())
            })();
            living.random = Some(random);
            result?;
        }

        // Census.Update8271d4f8 destroys the detached entities before the next
        // presentation pass can replay any of their last SIM command streams.
        Self::try_update_census(world, actor, cvars, &vault, Some(recipes))?;
        let mut living = owner.lock().map_err(|_| E::LivingWorldLock)?;
        living.big_black_entity = None;
        let props = living.props.as_mut().ok_or(E::MissingManagers)?;
        props.waypoint_guids.clear();
        let chunks: Vec<_> = props.waypoint_chunks.iter().map(|(&id, asset)| (id, asset.clone())).collect();
        for (id, asset) in chunks {
            recipes.prop_recipe_frame().register_waypoint_asset(&mut living, id, asset, false, cvars)
                .map_err(E::Pedestrians)?;
        }
        // The four component Reset slots call ResetAllEntities (82558fe8).
        // Only recreated waypoint Props remain; Prop's Reset is the folded
        // no-op825187c0 (LWBasicEntity vtable82394b00 + 0x5c).
        Ok(())
    }
}
