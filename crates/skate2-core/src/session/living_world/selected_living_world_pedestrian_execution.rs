//! Pedestrian execution phases over the sole live Manager/EntityMan owners.
use crate::{input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::HostReckoningNumeric};
use std::sync::Arc;

impl super::SelectedRecipeFrame<'_> {
    /// Task_UpdateStateGraphs82e73230: each target completes its pre/state/post
    /// sequence before the next target starts. Called after the whole list's
    /// positional updates and before any navigation task.
    pub(crate) fn update_pedestrian_preframe_states(
        &mut self, world: &mut PhysicalAssemblyWorld, cvars: &InputCvarReferences,
        ground: &crate::static_ground_pool::StaticGroundPool, local_player_index: i32,
        targets: &[crate::living_world_grind_services::execution::pedestrian::PedestrianExecutionTarget],
        mut simulation_random:Option<&mut crate::native_skater_random::SimulationCommandRandom>,
    ) -> Result<(), String> {
        if targets.is_empty() { return Ok(()); }
        let owner = Arc::clone(world.grind_services.living_world());
        let mut living = owner.lock().map_err(|_| "pedestrian state Manager lock")?;
        let actors = world.entity_roster.as_ref().ok_or("pedestrian state Actor roster absent")?
            .snapshot().map_err(|e| format!("pedestrian state Actor roster: {e:?}"))?;
        let triggers = self.triggers.lock().map_err(|_| "pedestrian state trigger lock")?;
        let avoid = self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
        drop(triggers);
        let painter = self.painter.lock().map_err(|_| "pedestrian state painter lock")?;
        for target in targets {
            crate::pedestrian_graph::backend::frame::with_entity_sim(
                &mut living, world, target.entity, cvars, &actors, &painter, self.vault, &avoid,
                simulation_random.as_deref_mut(),Some(ground),|entity, frame| {
                    frame.pedestrian_prestate_prefix(entity, target.dt, ground)?;
                    frame.update_threats(entity, local_player_index, ground)?;
                    frame.update_needs_and_wants(entity, local_player_index)?;
                    frame.update_chasee(entity, target.dt)?;
                    frame.update_zone_interests(entity)?;
                    frame.pedestrian_prestate_suffix(entity)?;
                    frame.security_guard_prestate_suffix(entity, target.dt)?;
                    frame.big_black_prestate(entity)?;
                    // BasicEntity82e4c1d0 checks the actual optional AI graph.
                    if entity.ai_bindings.is_some() { entity.update_ai_graph(target.dt, frame)?; }
                    // Pedestrian.UpdateState82e6eaf0 clears tAnimData+68,
                    // exactly cPedAnimData.MotionGraphInput.IntentsValid.
                    entity.animation.as_mut().ok_or("pedestrian state animation absent")?
                        .motion_intents.valid = false;
                    frame.pedestrian_poststate(entity);
                    Ok(())
                },
            )?;
        }
        Ok(())
    }

    /// Complete LWPedestrianExecutionMan.UpdatePreFrame82e96fc0, with one
    /// retained target list shared by every native task and query-send phase.
    pub(crate) fn update_pedestrian_preframe(
        &mut self, world: &mut PhysicalAssemblyWorld, cvars: &InputCvarReferences,
        ground: &crate::static_ground_pool::StaticGroundPool, center: [f32; 4],
        local_player_index: i32,
        simulation_random:Option<&mut crate::native_skater_random::SimulationCommandRandom>,
    ) -> Result<(), String> {
        let targets = self.prepare_pedestrian_preframe(world, cvars, center)?;
        self.update_pedestrian_preframe_states(world, cvars, ground, local_player_index, &targets, simulation_random)?;
        self.update_pedestrian_preframe_navigation(world, cvars, &targets)?;
        self.send_pedestrian_ground_queries(world, cvars, ground, &targets)?;
        self.send_pedestrian_volume_queries(world, cvars, &targets)?;
        self.send_pedestrian_ground_dmo_queries(world, cvars, ground, &targets)
    }

    /// CreateVolumeNavRigLineTestBatch82e868d8. The per-batch token table
    /// projects pointer equality only; it contains no entity payload or state.
    pub(crate) fn send_pedestrian_volume_queries(
        &mut self, world: &mut PhysicalAssemblyWorld, cvars: &InputCvarReferences,
        targets: &[crate::living_world_grind_services::execution::pedestrian::PedestrianExecutionTarget],
    ) -> Result<(), String> {
        use crate::living_world_chase::TargetableId;
        if !cvars.try_pedestrian_volume_tests().map_err(|e| format!("pedestrian volume tests: {e:?}"))? {
            return Ok(());
        }
        let owner = Arc::clone(world.grind_services.living_world());
        let mut living = owner.lock().map_err(|_| "pedestrian volume Manager lock")?;
        let mut batches = living.pedestrian_queries.take().ok_or("pedestrian query owner absent")?;
        let result = (|| {
            let batch = &mut batches.volume;
            batch.prepare_volumes();
            // Zero retains the native null-mask meaning. Tokens need only
            // survive this immutable volume/test batch: receive consumes
            // distance, never resolves a hit token back to an entity.
            let mut identities = Vec::<TargetableId>::new();
            let mut token = |identity: TargetableId| -> u32 {
                if let Some(index) = identities.iter().position(|entry| *entry == identity) {
                    return (index + 1) as u32;
                }
                identities.push(identity);
                identities.len() as u32
            };
            let roster = living.pedestrian_entities.as_deref().ok_or("pedestrian volume roster absent")?;
            for id in roster.ids() {
                let entity = roster.get(id).ok_or("pedestrian volume entity disappeared")?;
                let wiped_out = entity.state.as_ref().ok_or("pedestrian volume state absent")?.flags[1] & 4 != 0;
                let [velocity_scalar, max_velocity_shift] = cvars.try_pedestrian_avoidance_expansion()
                    .map_err(|e| format!("pedestrian volume expansion: {e:?}"))?;
                if !batch.add_pedestrian(entity, wiped_out, token(TargetableId { kind: 1, id: id as i32 }),
                    velocity_scalar, max_velocity_shift)? { break; }
            }
            batch.add_vehicles(&living)?;
            batch.add_dmos(living.dmo_objects.as_deref().ok_or("pedestrian volume DMO roster absent")?,
                self.vault, |id| token(TargetableId { kind: 2, id: id as i32 }))?;
            let actors = world.entity_roster.as_ref().ok_or("pedestrian volume Actor roster absent")?
                .snapshot().map_err(|e| format!("pedestrian volume Actor roster: {e:?}"))?;
            for actor in &actors {
                let id = actor.prefix().map_err(|e| e.to_string())?.fields()
                    .ok_or("pedestrian volume Actor fields absent")?.player_index;
                let physical = actor.cached_physical_allocation().map_err(|e| format!("pedestrian volume Actor physical: {e:?}"))?;
                let output = physical.owner().phys_out().lock().map_err(|_| "pedestrian volume PhysOut lock")?;
                let root = output.try_skeleton_output().map_err(|e| format!("pedestrian volume Actor root: {e:?}"))?
                    .skeleton_root_matrix;
                if !batch.add_skater(root, token(TargetableId { kind: 0, id: id as i32 })) { break; }
            }
            let triggers = self.triggers.lock().map_err(|_| "pedestrian volume trigger lock")?;
            let avoid = self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
            drop(triggers);
            avoid.add_to_pedestrian_query_batch(batch)?;
            batch.begin();
            let roster = living.pedestrian_entities.as_deref().ok_or("pedestrian volume roster absent")?;
            for target in targets {
                let entity = roster.get(target.entity).ok_or("pedestrian volume target disappeared")?;
                let owner_mask = token(TargetableId { kind: 1, id: entity.id as i32 });
                let ignore_mask = entity.nav_rig_ignore_entity.filter(|id| id.is_valid())
                    .map(&mut token).unwrap_or(0);
                let [fatness, y_clearance, start_offset] = cvars.try_pedestrian_volume_probe_tuning()
                    .map_err(|e| format!("pedestrian volume probe tuning: {e:?}"))?;
                if !batch.add_entity(entity, entity.navigation_rig_enabled, owner_mask, ignore_mask,
                    fatness, y_clearance, start_offset)? { break; }
            }
            batch.end()
        })();
        living.pedestrian_queries = Some(batches);
        result
    }

    /// CreateGroundDMOLineTestBatch82e72b08. Resolve taze data lazily for each
    /// entity, preserving the accepted submission prefix on checked failure.
    pub(crate) fn send_pedestrian_ground_dmo_queries(
        &mut self, world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences, ground: &crate::static_ground_pool::StaticGroundPool,
        targets: &[crate::living_world_grind_services::execution::pedestrian::PedestrianExecutionTarget],
    ) -> Result<(), String> {
        use crate::pedestrian_ground_dmo_batches::{GroundDmoProbeFacts, TazeProbeFacts};
        let owner = Arc::clone(world.grind_services.living_world());
        let mut living = owner.lock().map_err(|_| "pedestrian default-query Manager lock")?;
        // Temporarily separate the sole retained query owner from the roster
        // and callback borrows; restore it even when a callback fails.
        let mut batches = living.pedestrian_queries.take().ok_or("pedestrian query owner absent")?;
        let result = (|| {
            let (queries, _) = world.world_line_owners();
            batches.ground_dmo.begin(queries.ok_or("pedestrian world queries absent")?)?;
            if !targets.is_empty() {
                let actors = world.entity_roster.as_ref().ok_or("pedestrian default-query Actor roster absent")?
                    .snapshot().map_err(|e| format!("pedestrian default-query Actor roster: {e:?}"))?;
                let triggers = self.triggers.lock().map_err(|_| "pedestrian default-query trigger lock")?;
                let avoid = self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
                drop(triggers);
                let painter = self.painter.lock().map_err(|_| "pedestrian default-query painter lock")?;
                for target in targets {
                    let accepting = crate::pedestrian_graph::backend::frame::with_entity(
                        &mut living, world, target.entity, cvars, &actors, &painter, self.vault, &avoid,
                        |entity, frame| {
                            let tuning = cvars.try_pedestrian_ground_dmo_tuning()
                                .map_err(|e| format!("pedestrian default-query tuning: {e:?}"))?;
                            let state = entity.state.as_ref().ok_or("pedestrian default-query state absent")?;
                            let knocked_down = state.flags[1] & 4 != 0;
                            entity.positioning_test_start_height = Some(if knocked_down {
                                tuning.distance_up_knock_down
                            } else { tuning.distance_up_default });
                            let jump_enabled = entity.nav_rig.as_ref().ok_or("pedestrian default-query rig absent")?
                                .distances.do_jump_test && entity.navigation_rig_enabled;
                            let taze = if batches.ground_dmo.has_taze_capacity(jump_enabled)
                                && state.flags[4] & 0x80 != 0 && state.tazer_state == 2 {
                                let target = state.wants.get(usize::try_from(state.taze_result_target)
                                    .map_err(|_| "pedestrian taze want index negative")?)
                                    .ok_or("pedestrian taze want index outside table")?.target;
                                match frame.target_centre_of_mass(entity, target)? {
                                    Some(target_centre_of_mass) => Some(TazeProbeFacts {
                                        // tAnimData+6580 = SimLODBoneGlobal[3].translation.
                                        animation_origin: entity.animation.as_ref().ok_or("taze animation absent")?
                                            .sim_lod_bone_global.as_ref().ok_or("taze right-hand global pose absent")?.matrices[3][3],
                                        target_centre_of_mass,
                                    }),
                                    None => None,
                                }
                            } else { None };
                            let core = entity.core.as_ref().ok_or("pedestrian default-query core absent")?;
                            let rig = entity.nav_rig.as_ref().ok_or("pedestrian default-query rig absent")?;
                            let facts = GroundDmoProbeFacts {
                                pedestrian_id: entity.id,
                                core_position: core.target_transform[3],
                                position: core.transform[3], facing: core.transform[2],
                                knocked_down, ignore_nav_mesh: entity.ignore_nav_mesh_in_ground_test,
                                mesh_cell: &rig.mesh_info.cell,
                                jump_enabled,
                                taze,
                            };
                            let (queries, dmos) = frame.world.world_line_owners();
                            batches.ground_dmo.add_entity(&facts, &tuning,
                                queries.ok_or("pedestrian world queries absent")?, ground,
                                dmos.ok_or("pedestrian dynamic pool absent")?,
                                &mut crate::collision_line_numeric::HostCollisionLineNumeric)
                        },
                    )?;
                    if !accepting { break; }
                }
            }
            let (queries, dmos) = world.world_line_owners();
            batches.ground_dmo.end(queries.ok_or("pedestrian world queries absent")?, ground,
                dmos.ok_or("pedestrian dynamic pool absent")?,
                &mut crate::collision_line_numeric::HostCollisionLineNumeric)
        })();
        living.pedestrian_queries = Some(batches);
        result
    }

    /// UpdatePreFrame82e96fc0 prefix: retain the selected targets across the
    /// state-graph and navigation phases. All positional updates precede any
    /// PreStateGraphUpdate; the latter is a separate required phase.
    pub(crate) fn prepare_pedestrian_preframe(
        &mut self, world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences, center: [f32; 4],
    ) -> Result<Vec<crate::living_world_grind_services::execution::pedestrian::PedestrianExecutionTarget>, String> {
        let owner = Arc::clone(world.grind_services.living_world());
        let mut living = owner.lock().map_err(|_| "pedestrian preframe Manager lock")?;
        let mut targets = Vec::new();
        {
            let living = &mut *living;
            living.pedestrian_execution.collect_preframe(
                living.pedestrian_entities.as_deref_mut().ok_or("pedestrian preframe roster absent")?,
                center, &mut targets, &mut HostReckoningNumeric,
            ).map_err(|e| format!("pedestrian preframe targets: {e:?}"))?;
        }
        if targets.is_empty() { return Ok(targets); }
        let actors = world.entity_roster.as_ref().ok_or("pedestrian preframe Actor roster absent")?
            .snapshot().map_err(|e| format!("pedestrian preframe Actor roster: {e:?}"))?;
        let triggers = self.triggers.lock().map_err(|_| "pedestrian preframe trigger lock")?;
        let avoid = self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
        drop(triggers);
        let painter = self.painter.lock().map_err(|_| "pedestrian preframe painter lock")?;
        for target in &targets {
            crate::pedestrian_graph::backend::frame::with_entity(
                &mut living, world, target.entity, cvars, &actors, &painter, self.vault, &avoid,
                |entity, frame| frame.update_navigation_position(entity, center),
            )?;
        }
        Ok(targets)
    }

    /// UpdatePreFrame82e9704c..a4, AFTER every selected entity's
    /// PreStateGraphUpdate/UpdateState/PostStateGraphUpdate. Each task visits
    /// the entire retained list before the next task begins. Query creation
    /// must follow this method, using the same targets.
    pub(crate) fn update_pedestrian_preframe_navigation(
        &mut self, world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences,
        targets: &[crate::living_world_grind_services::execution::pedestrian::PedestrianExecutionTarget],
    ) -> Result<(), String> {
        let owner = Arc::clone(world.grind_services.living_world());
        let mut living = owner.lock().map_err(|_| "pedestrian navigation Manager lock")?;
        if !targets.is_empty() {
            let actors = world.entity_roster.as_ref().ok_or("pedestrian navigation Actor roster absent")?
                .snapshot().map_err(|e| format!("pedestrian navigation Actor roster: {e:?}"))?;
            let triggers = self.triggers.lock().map_err(|_| "pedestrian navigation trigger lock")?;
            let avoid = self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
            drop(triggers);
            let painter = self.painter.lock().map_err(|_| "pedestrian navigation painter lock")?;
            // Entity vslots40/44 (Pre/PostNavigation) are the folded empty
            // function for both Pedestrian and SecurityGuard.
            for phase in 0..3 {
                for target in targets {
                    crate::pedestrian_graph::backend::frame::with_entity(
                        &mut living, world, target.entity, cvars, &actors, &painter, self.vault, &avoid,
                        |entity, frame| match phase {
                            0 => frame.seek_navigation(entity),
                            1 => frame.resolve_navigation(entity),
                            _ => frame.update_navigation(entity, target.dt),
                        },
                    )?;
                }
            }
        }
        let living = &mut *living;
        let mesh = living.mesh.as_ref().ok_or("pedestrian navigation MeshMan absent")?;
        let roster = living.pedestrian_entities.as_deref_mut().ok_or("pedestrian navigation roster absent")?;
        let rotations = crate::pedestrian_navigation::rig::ray_sin_cos();
        for target in targets {
            let tuning = cvars.try_pedestrian_rig_tuning().map_err(|e| format!("pedestrian rig tuning: {e:?}"))?;
            roster.get_mut(target.entity).ok_or("pedestrian transform rig target disappeared")?
                .transform_nav_rig(&tuning, &rotations, &mut HostReckoningNumeric)?;
        }
        let mut batch = mesh.nav_validator.lock().map_err(|_| "pedestrian navigation validator lock")?;
        for target in targets {
            let tuning = cvars.try_pedestrian_rig_tuning().map_err(|e| format!("pedestrian rig tuning: {e:?}"))?;
            let entity = roster.get_mut(target.entity).ok_or("pedestrian validate rig target disappeared")?;
            entity.validate_nav_rig(entity.navigation_rig_enabled, &tuning, &mut batch, &mut HostReckoningNumeric)?;
        }
        // ProcessBatch is unconditional, even for an empty execution list.
        batch.process(&mut HostReckoningNumeric)?;
        for target in targets {
            roster.get_mut(target.entity).ok_or("pedestrian receive rig target disappeared")?
                .receive_nav_rig_batch(&batch)?;
        }
        drop(batch);
        for target in targets {
            let tuning = cvars.try_pedestrian_rig_tuning().map_err(|e| format!("pedestrian rig tuning: {e:?}"))?;
            let entity = roster.get_mut(target.entity).ok_or("pedestrian orient rig target disappeared")?;
            entity.orient_nav_rig(entity.navigation_rig_enabled, &tuning, &mesh.store, &mut HostReckoningNumeric)?;
        }
        Ok(())
    }

    /// CreateGroundLineTestBatch82e72320. This is the first of three query
    /// creation phases, after OrientNavRigToWorldTopology. The native cvar
    /// skips Begin as well as End, preserving the previous buffer contents.
    pub(crate) fn send_pedestrian_ground_queries(
        &mut self, world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences, ground: &crate::static_ground_pool::StaticGroundPool,
        targets: &[crate::living_world_grind_services::execution::pedestrian::PedestrianExecutionTarget],
    ) -> Result<(), String> {
        if !cvars.try_pedestrian_physics_tests().map_err(|e| format!("pedestrian physics tests: {e:?}"))? {
            return Ok(());
        }
        let owner = Arc::clone(world.grind_services.living_world());
        let mut living = owner.lock().map_err(|_| "pedestrian ground Manager lock")?;
        let living = &mut *living;
        let batches = &mut living.pedestrian_queries.as_mut().ok_or("pedestrian query owner absent")?.ground;
        let roster = living.pedestrian_entities.as_deref_mut().ok_or("pedestrian ground roster absent")?;
        let (queries, dmos) = world.world_line_owners();
        let queries = queries.ok_or("pedestrian world queries absent")?;
        let dmos = dmos.ok_or("pedestrian dynamic pool absent")?;
        let mut numeric = crate::collision_line_numeric::HostCollisionLineNumeric;
        batches.begin(queries)?;
        for target in targets {
            let entity = roster.get_mut(target.entity).ok_or("pedestrian ground target disappeared")?;
            let tuning = cvars.try_pedestrian_probe_tuning().map_err(|e| format!("pedestrian ground tuning: {e:?}"))?;
            let enabled = entity.navigation_rig_enabled;
            // BF_Ground has no owner-mask exclusion; this request field is
            // used only by VolumeLineBatch, never by the ground line owner.
            if !batches.add_entity(entity, enabled, 0, &tuning,
                queries, ground, dmos, &mut numeric)? { break; }
        }
        batches.end(queries, ground, dmos, &mut numeric)
    }

    /// ComponentMan.UpdateLOSStates82e82da8. TestLOSim82e94ce0 forces high
    /// simulation when the actual Chasee exists; otherwise use shared radii.
    pub(crate) fn update_pedestrian_los_states(&mut self,
        world: &mut PhysicalAssemblyWorld, cvars: &InputCvarReferences,
        center: [f32; 4],
    ) -> Result<(), String> {
        let owner = Arc::clone(world.grind_services.living_world());
        let mut living = owner.lock().map_err(|_| "pedestrian LOSim Manager lock")?;
        let ids = living.pedestrian_entities.as_ref().ok_or("pedestrian LOSim roster absent")?.ids();
        if ids.is_empty() { return Ok(()); }
        let actors = world.entity_roster.as_ref().ok_or("pedestrian LOSim Actor roster absent")?
            .snapshot().map_err(|e| format!("pedestrian LOSim roster: {e:?}"))?;
        let triggers = self.triggers.lock().map_err(|_| "pedestrian LOSim trigger lock")?;
        let avoid = self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
        drop(triggers);
        let painter = self.painter.lock().map_err(|_| "pedestrian LOSim painter lock")?;
        for id in ids {
            let (previous, next) = crate::pedestrian_graph::backend::frame::with_entity(
                &mut living, world, id, cvars, &actors, &painter, self.vault, &avoid,
                |entity, frame| {
                    let previous = entity.core.as_ref().ok_or("pedestrian LOSim core absent")?.current_losim;
                    let target = entity.state.as_ref().ok_or("pedestrian LOSim AI state absent")?.chaser.chasee;
                    let next = if frame.big_black_has_threat(entity)? || frame.with_chasee(entity, target, |chasee| chasee.is_some())? { 0 }
                        else { super::pedestrian_animation::test_losim(entity, center, &mut HostReckoningNumeric)? };
                    Ok((previous, next))
                })?;
            if previous != next {
                // LWCollideableObject.OnLOSimSwitch82e50580 has no state
                // writes and returns true after its optional collision getter.
                living.pedestrian_execution.remove(previous, id)
                    .map_err(|e| format!("pedestrian old LOSim group: {e:?}"))?;
                living.pedestrian_entities.as_deref_mut().ok_or("pedestrian LOSim roster absent")?
                    .get_mut(id).ok_or("pedestrian LOSim entity disappeared")?
                    .core.as_mut().ok_or("pedestrian LOSim core absent")?.current_losim = next;
                living.pedestrian_execution.add(next, id)
                    .map_err(|e| format!("pedestrian new LOSim group: {e:?}"))?;
            }
        }
        Ok(())
    }

    /// LWPedestrianExecutionMan.UpdateCurrentFrame82e96ef8. Query swaps and
    /// reception precede selection; modifiers run for the entire selected list
    /// before any entity consumes their recommendations in UpdateIntent.
    pub(crate) fn update_pedestrian_current_frame(
        &mut self, world: &mut PhysicalAssemblyWorld,
        cvars: &InputCvarReferences, center: [f32; 4],
    ) -> Result<(), String> {
        let owner = Arc::clone(world.grind_services.living_world());
        let mut living = owner.lock().map_err(|_| "pedestrian current Manager lock")?;
        let mesh = living.try_mesh_store().map_err(|e| format!("pedestrian current mesh: {e:?}"))?;
        let y_tolerance = cvars.try_pedestrian_default_mesh_y_tolerance()
            .map_err(|e| format!("pedestrian query mesh tolerance: {e:?}"))?;
        {
            let living = &mut *living;
            let (queries, _) = world.world_line_owners();
            living.pedestrian_queries.as_mut().ok_or("pedestrian query owner absent")?
                .receive_current_frame(
                    queries.ok_or("pedestrian world queries absent")?,
                    living.pedestrian_entities.as_deref_mut().ok_or("pedestrian current roster absent")?,
                    &mut living.pedestrian_culls,
                    &mesh, y_tolerance,
                )?;
        }
        let mut targets = Vec::new();
        living.pedestrian_execution.collect_current(
            living.pedestrian_entities.as_deref().ok_or("pedestrian current roster absent")?,
            center, &mut targets, &mut HostReckoningNumeric,
        ).map_err(|e| format!("pedestrian current targets: {e:?}"))?;
        if targets.is_empty() { return Ok(()); }
        let actors = world.entity_roster.as_ref().ok_or("pedestrian current Actor roster absent")?
            .snapshot().map_err(|e| format!("pedestrian current Actor roster: {e:?}"))?;
        let triggers = self.triggers.lock().map_err(|_| "pedestrian current trigger lock")?;
        let avoid = self.runtime.challenge_avoid_volumes.resolve(&triggers)?;
        drop(triggers);
        let painter = self.painter.lock().map_err(|_| "pedestrian current painter lock")?;
        for target in &targets {
            crate::pedestrian_graph::backend::frame::with_entity(
                &mut living, world, target.entity, cvars, &actors, &painter, self.vault, &avoid,
                |entity, frame| frame.evaluate_all_navigation_modifiers(entity),
            )?;
        }
        for target in &targets {
            living.pedestrian_entities.as_deref_mut().ok_or("pedestrian current roster absent")?
                .get_mut(target.entity).ok_or("pedestrian current target disappeared")?
                .update_intent(&mesh, cvars, &mut HostReckoningNumeric)?;
        }
        Ok(())
    }
}
