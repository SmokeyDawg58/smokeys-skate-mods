//! PhysOutModule_Hud827c6ee0 and PlayerUI's DMO messages828aabd0..ad58.
use crate::physics_state_ids::PHYSICS_STATE_SKITCHING;
use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage};
use crate::selected_living_world_recipes::SelectedRecipeFrame;
use std::sync::Arc;
fn error(e: impl std::fmt::Debug) -> SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::SessionMarker, e)
}

impl<T, C> SelectedLiveSession<T, C> {
    pub(super) fn queue_prop_input(&mut self, input: [bool; 5]) {
        let Some(marker) = self.inner.session_marker.as_mut() else {
            return;
        };
        let Some(prop) = marker.hud.prop else {
            return;
        };
        // SignUpHUD.UpdateInput826bd7c0; add/remove are delivered even if dimmed.
        let action = if input[0] && !prop.params[0] {
            Some(0)
        } else if input[1] && !prop.params[1] {
            Some(1)
        } else if input[3] {
            Some(2)
        } else if input[2] {
            Some(3)
        } else {
            None
        };
        if let Some(action) = action {
            marker.prop_actions.push((prop.object, action));
        }
    }

    pub(super) fn try_update_prop_hud(&mut self) -> Result<(), SelectedLiveError> {
        let Some(marker) = self.inner.session_marker.as_mut() else {
            return Ok(());
        };
        let selected = &self
            .inner
            .player
            .as_ref()
            .ok_or_else(|| error("prop player absent"))?
            .selected;
        let actor = selected
            .actor
            .as_ref()
            .ok_or_else(|| error("prop Actor absent"))?;
        let physical = actor.cached_physical_allocation().map_err(error)?;
        let output = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| error("prop PhysOut lock"))?;
        let state = output.try_state_output().map_err(error)?;
        let offboard = output.try_offboard_output().map_err(error)?;
        let skitch = state.physics_state == PHYSICS_STATE_SKITCHING;
        let interactive = state.is_offboard
            && (offboard.is_grabbing_object || offboard.has_valid_interactable_object);
        let grabbing = offboard.is_grabbing_object;
        let spline = state.grab_spline_desc.object_id;
        let nearby = state.interactable_object_id;
        let offboard_only = state.is_offboard && !interactive;
        let skater = output
            .try_skeleton_output()
            .map_err(error)?
            .skeleton_root_matrix
            .columns()[3]
            .xyz;
        drop(output);
        if skitch {
            marker.hud.phone_hidden = true;
        } else if interactive {
            let world = self
                .inner
                .world
                .as_ref()
                .ok_or_else(|| error("prop world absent"))?
                .world
                .lock()
                .map_err(|_| error("prop world lock"))?;
            let living = world
                .grind_services
                .living_world()
                .lock()
                .map_err(|_| error("prop living world lock"))?;
            let objects = living
                .dmo_objects
                .as_deref()
                .ok_or_else(|| error("prop manager absent"))?;
            let id = if grabbing {
                objects.grab_spline_owner(spline as u32)
            } else {
                nearby
            };
            if id != 0 {
                let can_upright = if let Some(object) = objects.object(id) {
                    !object
                        .try_is_upright(
                            selected
                                .owners
                                .cvars
                                .try_dynamic_object_upright_angle()
                                .map_err(error)?,
                        )
                        .map_err(error)?
                } else {
                    false
                };
                let can_reset = crate::selected_living_world_recipes::dmo_reset::can_reset_dmo(
                    &living, &world, id,
                )
                .map_err(error)?;
                let exists = marker.marker.as_ref().is_some_and(|m| m.get().is_some());
                let member = objects.object(id).is_some_and(|o| {
                    marker
                        .marker
                        .as_ref()
                        .is_some_and(|m| m.props.contains(o.locator_id))
                });
                marker.hud.prop = Some(crate::ui::phonelist::PropHudSnapshot {
                    object: id,
                    params: [!can_upright, !can_reset, exists, member, false],
                });
                marker.hud.phone_hidden = true;
            }
        } else if let Some(prop) = offboard_only
            .then(|| {
                let exists = marker.marker.as_ref().is_some_and(|m| m.get().is_some());
                self.object_dropper.board_prop_hud(skater, exists)
            })
            .flatten()
        {
            // PC Object Dropper: a dropped board gets the same prop menu.
            marker.hud.prop = Some(prop);
            marker.hud.phone_hidden = true;
        } else {
            marker.hud.prop = None;
            marker.hud.phone_hidden = false;
        }
        marker.hud.prop_tick = marker.hud.prop_tick.wrapping_add(1);
        if let Some(career) = self.career.as_mut() {
            crate::challenge::signup_hud::update_prop(
                career.system.world_mut(),
                marker.hud.prop,
                marker.hud.phone_hidden,
            );
        }
        Ok(())
    }

    pub(super) fn apply_prop_actions(&mut self) -> Result<(), SelectedLiveError> {
        // PC Object Dropper: dropped boards' menu actions and marker restore go first; the
        // living-world code below never sees their object IDs.
        let Some(marker) = self.inner.session_marker.as_mut() else {
            return Ok(());
        };
        let (boards, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut marker.prop_actions)
            .into_iter()
            .partition(|&(object, _)| super::object_dropper::is_board_prop(object));
        marker.prop_actions = rest;
        let restore = marker.restore_props && marker.marker.is_some();
        let has_marker = marker.marker.is_some();
        if !boards.is_empty() || restore {
            self.apply_board_prop_actions(&boards, restore, has_marker)?;
        }
        let Some(marker) = self.inner.session_marker.as_mut() else {
            return Ok(());
        };
        if marker.prop_actions.is_empty()
            && !marker.restore_props
            && marker
                .marker
                .as_ref()
                .is_none_or(|m| m.props.cleared.is_empty())
        {
            return Ok(());
        }
        let world_owner = self
            .inner
            .world
            .as_ref()
            .ok_or_else(|| error("prop world absent"))?;
        let terrain = self
            .inner
            .terrain
            .as_ref()
            .ok_or_else(|| error("prop terrain absent"))?;
        let startup = terrain.terrain_for(&world_owner.world).map_err(error)?;
        let selected = &mut self
            .inner
            .player
            .as_mut()
            .ok_or_else(|| error("prop player absent"))?
            .selected;
        let actor = selected
            .actor
            .as_deref()
            .ok_or_else(|| error("prop Actor absent"))?;
        let owners = &mut selected.owners;
        let cvars = owners.cvars.clone();
        let recipes = Arc::clone(&owners.living_world_recipes);
        let mut runtime = recipes.lock().map_err(|_| error("prop recipes lock"))?;
        let graphs = Arc::clone(
            owners
                .graphs
                .as_ref()
                .ok_or_else(|| error("prop graphs absent"))?,
        );
        let graphs = graphs.lock().map_err(|_| error("prop graphs lock"))?;
        let frame=SelectedRecipeFrame {
            challenge: None,
            triggers:&startup.triggers,
            pedestrian_animation:crate::selected_living_world_recipes::pedestrian_animation::PedestrianSessionAnimation {
                startup:&owners.pedestrian_animation,shared:&owners.animation,world:&world_owner.world,
            },
            runtime:&mut runtime,
            registry:owners.recipe_registry.as_mut().ok_or_else(||error("prop registry absent"))?,
            requests:owners.recipe_load.as_mut().ok_or_else(||error("prop requests absent"))?.requests_mut(),
            mounts:&owners.mounts,vault:&owners.services.vault,graphs:&graphs,painter:&startup.layers,
        };
        let painter = startup
            .layers
            .lock()
            .map_err(|_| error("prop painter lock"))?;
        let mut world = world_owner
            .world
            .lock()
            .map_err(|_| error("prop world lock"))?;
        let living = Arc::clone(world.grind_services.living_world());
        let mut living = living.lock().map_err(|_| error("prop living world lock"))?;
        if let Some(session) = marker.marker.as_mut() {
            session.props.flush_cleared(&mut living).map_err(error)?;
        }
        for (id, action) in std::mem::take(&mut marker.prop_actions) {
            if std::env::var_os("SKATE2_TRACE_MARKER_UI").is_some() {
                eprintln!("prop action={action} object={id}");
            }
            match action {
                0 => {
                    if let Some(object) = living
                        .dmo_objects
                        .as_deref_mut()
                        .and_then(|m| m.object_mut(id))
                    {
                        if !object
                            .try_is_upright(
                                cvars.try_dynamic_object_upright_angle().map_err(error)?,
                            )
                            .map_err(error)?
                        {
                            object.flags |= 0x40;
                        }
                    }
                }
                1 => frame
                    .try_reset_unsafe_dmo(&mut living, &mut world, id, &cvars, &painter, actor)
                    .map_err(error)?,
                2 | 3 => {
                    if let Some(session) = marker.marker.as_mut() {
                        if let Some(locator) = living
                            .dmo_objects
                            .as_deref()
                            .and_then(|m| m.object(id))
                            .map(|o| o.locator_id)
                        {
                            if action == 2 {
                                session.props.add(locator, &mut living).map_err(error)?;
                            } else {
                                session.props.remove(locator);
                                session.props.flush_cleared(&mut living).map_err(error)?;
                            }
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        if std::mem::take(&mut marker.restore_props) {
            if let Some(session) = marker.marker.as_ref() {
                let saved: Vec<_> = session.props.iter().copied().collect();
                frame
                    .restore_marker_props(&mut living, &mut world, &saved, &cvars, &painter)
                    .map_err(error)?;
            }
        }
        Ok(())
    }
}
