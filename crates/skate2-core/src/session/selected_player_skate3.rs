//! Explicit host selection, without any second player controller or solver.
use super::SelectedPlayerTick;
use crate::{
    physical_player_assembly::PhysicalAssemblyWorld,
    static_ground_pool::StaticGroundPool,
    ui::gameplay::{Difficulty, GameplaySettings},
};

impl SelectedPlayerTick {
    pub(crate) fn configure_skate3(
        &mut self,
        settings: GameplaySettings,
        raw: Option<crate::input_pad::XboxPadSample>,
    ) {
        self.mod_settings = settings;
        self.raw_controller = raw;
    }
    pub(crate) fn configure_skate3_focus(&mut self, focused: bool) {
        self.skate3_has_focus = focused;
        if !focused { self.raw_controller = Some(crate::input_pad::XboxPadSample { buttons: 0, left_trigger: 0, right_trigger: 0, left_x: 0, left_y: 0, right_x: 0, right_y: 0 }); }
    }
    pub(crate) fn allow_skate3_start(&mut self, allowed: bool) { self.allow_skate3_start = allowed; }
    pub(crate) fn configure_skate3_camera(&mut self, camera_type: i32) { self.camera_type = camera_type; }
    pub(crate) fn skate3_output(&self) -> Option<std::sync::Arc<crate::skate3_native::FrameOutput>> {
        let owner = self.mode_owner.as_ref()?;
        let mode = owner.skate3.lock().ok()?;
        mode.as_ref().map(|player| player.output_frame())
    }
    #[cfg(test)]
    pub(crate) fn skate3_ground_height(&self,point:[f32;3])->Result<f32,String> {
        let owner=self.mode_owner.as_ref().ok_or("ground fixture source owner")?;
        let player=owner.skate3.lock().map_err(|_|"ground fixture source lock")?;
        player.as_ref().ok_or("ground fixture source player")?.test_ground_height(point)
    }
    #[cfg(test)]
    pub(crate) fn skate3_spawn_wheel_gaps(&self)->Result<[f32;4],String> {
        let owner=self.mode_owner.as_ref().ok_or("wheel audit no selected owner")?;
        let player=owner.skate3.lock().map_err(|_|"wheel audit source lock")?;
        player.as_ref().ok_or("wheel audit no source player")?.test_spawn_wheel_gaps()
    }
    #[cfg(test)]
    pub(crate) fn skate3_spawn_stance(&self) -> (u32, i32) {
        let owner=self.mode_owner.as_ref().unwrap();
        let player=owner.skate3.lock().unwrap();
        player.as_ref().unwrap().test_spawn_stance()
    }
    #[cfg(test)]
    pub(crate) fn skate3_boneless_trace(&self) -> String {
        self.mode_owner.as_ref().and_then(|owner|owner.skate3.lock().ok())
            .and_then(|mode|mode.as_ref().map(|player|player.test_boneless_trace())).expect("source boneless")
    }
    #[cfg(test)]
    pub(crate) fn skate3_bail_control_probe(&self)->([f32;2],Option<[f32;2]>) {
        let owner=self.mode_owner.as_ref().unwrap();let mode=owner.skate3.lock().unwrap();
        mode.as_ref().unwrap().test_bail_control_probe()
    }
    #[cfg(test)]
    pub(crate) fn skate3_plant_control_probe(&self)->([f32;2],[f32;4],Option<[f32;2]>) {
        let owner=self.mode_owner.as_ref().unwrap();let mode=owner.skate3.lock().unwrap();
        mode.as_ref().unwrap().test_plant_control_probe()
    }
    #[cfg(test)]
    pub(crate) fn skate3_footplant_trace(&self) -> String {
        self.mode_owner.as_ref().and_then(|owner|owner.skate3.lock().ok())
            .and_then(|mode|mode.as_ref().map(|player|player.test_footplant_trace())).expect("source footplant")
    }
    #[cfg(test)]
    pub(crate) fn skate3_grab_audit(&self,world:&PhysicalAssemblyWorld)->Result<(usize,usize,usize),String> {
        let owner=self.mode_owner.as_ref().ok_or("grab audit no selected owner")?;
        let mode=owner.skate3.lock().map_err(|_|"grab audit source lock")?;
        mode.as_ref().ok_or("grab audit no source player")?.test_grab_scene(world)
    }
    #[cfg(test)]
    pub(crate) fn skate3_living_world_impulse(&self) -> (bool,[u32;4]) {
        self.mode_owner.as_ref().and_then(|owner|owner.skate3.lock().ok())
            .and_then(|mode|mode.as_ref().map(|player|player.test_living_world_impulse())).expect("source impulse")
    }
    #[cfg(test)]
    pub(crate) fn skate3_actor_input(&self) -> (bool,bool,Vec<String>,[f32;2]) {
        self.mode_owner.as_ref().and_then(|owner|owner.skate3.lock().ok())
            .and_then(|mode|mode.as_ref().map(|player|player.test_actor_input())).expect("source actor input")
    }
    #[cfg(test)]
    pub(crate) fn skate3_offboard_motion_trace(&self)->String {
        let owner=self.mode_owner.as_ref().unwrap();let mode=owner.skate3.lock().unwrap();
        mode.as_ref().unwrap().test_offboard_motion_trace()
    }
    #[cfg(test)]
    pub(crate) fn skate3_input_trace(&self) -> String {
        self.mode_owner.as_ref().and_then(|owner|owner.skate3.lock().ok())
            .and_then(|mode|mode.as_ref().map(|player|player.test_input_trace())).unwrap_or_default()
    }
    #[cfg(test)]
    pub(crate) fn skate3_world_body_count(&self) -> usize {
        self.mode_owner.as_ref().and_then(|owner| owner.skate3.lock().ok())
            .and_then(|mode| mode.as_ref().map(|player| player.world_body_count())).unwrap_or(0)
    }
    #[cfg(test)]
    pub(crate) fn test_reconcile_skate3(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        skaters: &[super::SelectedTickSkater<'_>],
        selected: usize,
        input_active: bool,
        settings: &crate::players_backend_settings::PlayersBackendSettings,
    ) -> Result<(),String> {
        self.try_reconcile_skate3(world,ground,skaters,selected,input_active,settings)
    }
    pub(super) fn try_reconcile_skate3(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        skaters: &[super::SelectedTickSkater<'_>],
        selected: usize,
        input_active: bool,
        settings: &crate::players_backend_settings::PlayersBackendSettings,
    ) -> Result<(), String> {
        let skater = skaters.get(selected).ok_or("Missing selected native player")?;
        let (actor, board, skeleton) = (skater.actor, skater.board, skater.skeleton);
        if self.gesture_preferences.is_none() || self.style_preferences.is_none() {
            let path=crate::edit_skater_profile::EditSkaterProfileStore::default_path().map_err(|e|e.to_string())?;
            let store=crate::edit_skater_profile::EditSkaterProfileStore::open(path).map_err(|e|e.to_string())?;
            if self.gesture_preferences.is_none() { self.gesture_preferences=Some(store.load_gesture_settings().map_err(|e|e.to_string())?); }
            if self.style_preferences.is_none() { self.style_preferences=Some(store.load_skate3_style().map_err(|e|e.to_string())?); }
        }
        actor.try_set_pc_gesture_preferences(self.gesture_preferences.unwrap())?;
        let physical = actor.cached_physical_allocation().map_err(|e| e.to_string())?;
        let owner = std::sync::Arc::clone(physical.owner());
        if let Some(previous) = self.mode_owner.as_ref().filter(|previous| !std::sync::Arc::ptr_eq(previous, &owner)) {
            crate::skate3_native::physics::player::tuning::set_owner(previous, None)?;
            previous.skate3_footplants.store(false,std::sync::atomic::Ordering::Relaxed);
            // Party Play changes the local Actor without reordering the skater list.
            // Return the previous Actor to its own solver before selecting another.
            let mut previous_mode = previous.skate3.lock().map_err(|_| "previous native physical mode lock")?;
            if let Some(player) = previous_mode.take() {
                let previous_skater = skaters.iter().find(|candidate| {
                    candidate.actor.cached_physical_allocation().ok().is_some_and(|physical|
                        std::sync::Arc::ptr_eq(physical.owner(), previous))
                });
                if let Some(skater) = previous_skater {
                    player.return_to_skate2(world, skater.actor, previous, skater.board, skater.skeleton)?;
                    skater.actor.try_select_skate3_movement(false).map_err(|e|e.to_string())?;
                } else {
                    player.release(world)?;
                }
            }
            self.active_difficulty = None;
            self.previous_offboard_button = false;
            crate::diagnostics::test_log::event("skate3_player_owner", format_args!("selected={selected} changed=true"));
        }
        self.mode_owner = Some(std::sync::Arc::clone(&owner));
        let mut mode = owner.skate3.lock().map_err(|_| "native physical mode lock")?;
        let (player_index, _, non_player_ai) = owner.actor_query.lock().map_err(|_| "native player query lock")?
            .as_ref().ok_or("native player query missing")?.try_scoring_identity().map_err(|error|error.to_string())?;
        let tuning = if non_player_ai { None } else {
            crate::skate3_native::physics::player::tuning::Tuning::select(self.mod_settings.difficulty)? };
        crate::skate3_native::physics::player::tuning::set_owner(&owner, tuning)?;
        let wants_footplant=!non_player_ai && !self.mod_settings.skate3.all && self.mod_settings.skate3.footplants;
        // Let an already-contacting independent plant release through its own
        // manager. Retiring it in the middle would discard its authored path.
        let finishing_footplant=!non_player_ai && !self.mod_settings.skate3.all
            && owner.skate3_footplants.load(std::sync::atomic::Ordering::Relaxed)
            && matches!(owner.physics_state().lock().map_err(|_|"footplant selection state lock")?.current,
                crate::physical_player_physics_state::CurrentPhysicalState::FootPlant
                    | crate::physical_player_physics_state::CurrentPhysicalState::Boneless);
        let enabled=wants_footplant || finishing_footplant;
        let was_enabled=owner.skate3_footplants.swap(enabled,std::sync::atomic::Ordering::Relaxed);
        if was_enabled && !enabled {
            owner.skate3_footplant_requested.store(false,std::sync::atomic::Ordering::Relaxed);
            let mut foot=owner.footplant().lock().map_err(|_|"footplant selection owner lock")?;
            let manager=foot.try_manager(&owner,skeleton).map_err(|error|format!("footplant selection: {error:?}"))?;
            if let Some(source)=manager.source3.as_mut() { source.reset(true,false); }
            manager.prediction.ready=false;
            crate::diagnostics::test_log::event("skate3_footplant_owner",format_args!("native_s2_solver=true enabled=false"));
        }
        let toggle_held = self.raw_controller.is_some_and(|pad|
            pad.buttons & (1 << 15) != 0 && pad.buttons & (1 << 8) == 0);
        let actor_policy=actor.try_skate3_actor_input().map_err(|error|error.to_string())?;
        let toggle_pressed = input_active && !actor_policy.pinned && toggle_held && !self.previous_offboard_button;
        self.previous_offboard_button = toggle_held;
        let host_offboard = owner.phys_out().lock().map_err(|_| "offboard selection output lock")?
            .try_state_output().map_err(|error|format!("offboard selection state: {error:?}"))?.is_offboard;
        let independent_offboard = !self.mod_settings.skate3.all && self.mod_settings.skate3.offboard;
        if let Some(player) = mode.as_mut() { player.observe_offboard_replacement(); }
        if independent_offboard && toggle_pressed {
            if let Some(player) = mode.as_mut() { player.begin_ready_offboard_replacement(); }
        }
        // Frontend admission defers construction; it is not an independent
        // Darkcatch mode selection and must not run that mode's error branch.
        if mode.is_none() && !self.allow_skate3_start {
            actor.try_select_skate3_movement(false).map_err(|e| e.to_string())?;
            return Ok(());
        }
        let use_source = self.mod_settings.skate3.all || (independent_offboard &&
            (host_offboard || toggle_pressed || mode.as_ref().is_some_and(|player|player.keep_offboard_replacement())));
        if !use_source || non_player_ai {
            actor.try_select_skate3_movement(false).map_err(|e| e.to_string())?;
            if !non_player_ai && self.mod_settings.skate3.darkcatch {
                return Err("Independent Darkcatch is not connected yet".into());
            }
            if let Some(player) = mode.take() {
                player.return_to_skate2(world, actor, &owner, board, skeleton)?;
                self.active_difficulty = None;
            }
            return Ok(());
        }
        // A Party Play location/character change can publish a new Actor
        // before its replacement world has any resident collision. The native
        // frontend preview remains its existing owner until admission; creating
        // a source solver against an empty scene used to terminate the game.
        if mode.is_none() && ground.published_volume_count() == 0 {
            crate::skate3_native::physics::player::tuning::set_owner(&owner, None)?;
            actor.try_select_skate3_movement(false).map_err(|error|error.to_string())?;
            self.active_difficulty = None;
            if self.tick.is_multiple_of(60) {
                crate::diagnostics::test_log::event("skate3_scene_wait",format_args!(
                    "tick={} selected={selected} reason=no_resident_terrain phase=player_admission",self.tick));
            }
            return Ok(());
        }
        let difficulty = self.mod_settings.difficulty;
        let profile = match difficulty {
            Difficulty::Easy | Difficulty::EasyMotorized => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Skate2Default => "custom",
            Difficulty::Hardcore => "hardcore",
            Difficulty::Motorized => "motorized",
        };
        let entering = mode.is_none();
        if entering {
            let entry = {
                let output = owner.phys_out().lock().map_err(|_| "native entry PhysOut lock")?;
                if output.try_state_output().map_err(|error|format!("native entry state: {error:?}"))?.is_offboard {
                    let root = output.try_skeleton_output().map_err(|error|format!("native entry skeleton: {error:?}"))?.skeleton_root_matrix;
                    let mut transform = crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
                    for (column, value) in root.columns().iter().enumerate() {
                        transform[column * 4..column * 4 + 3].copy_from_slice(&value.xyz);
                    }
                    let (_, velocity) = output.try_system_centre_and_damped_velocity().map_err(|error|format!("native entry velocity: {error:?}"))?;
                    let state = output.try_state_output().map_err(|error|format!("native entry state: {error:?}"))?.physics_state;
                    // S2's separate Locomotion504 is part of S3's BipedGround;
                    // S2's Jump503 reports the same air family as OffBoardAir502.
                    let state = match state { 501 => 502, 502 | 503 => 501, 505 => 503, 500 | 504 => 500,
                        other => return Err(format!("Unsupported offboard entry state {other}")) };
                    let holding_board = output.try_offboard_output().map_err(|error|format!("native entry board: {error:?}"))?.is_holding_skateboard;
                    let filtered = output.try_filtered_output().map_err(|error|format!("native entry filtered: {error:?}"))?;
                    if filtered.is_grinding { return Err("Offboard entry carries a conflicting active grind".into()); }
                    let category = |state| {
                        use crate::physics::FilteredPhysicsState as S2;
                        use skate3_core::physics::filtered_state::FilteredCategory as S3;
                        match state { S2::Invalid => S3::Invalid, S2::Ground => S3::Ground, S2::Air => S3::Air,
                            S2::Grind => S3::Grind, S2::Wipeout => S3::Wipeout, S2::Teleport => S3::Teleport,
                            S2::OffBoard => S3::Offboard, S2::OffBoardAir => S3::OffboardAir }
                    };
                    let filtered = skate3_core::physics::filtered_state::FilteredStateOutput {
                        category: category(filtered.filtered_physics_state),
                        previous_category: category(filtered.last_filtered_physics_state),
                        grinding: false, grind: Default::default(), last_grind_distance: filtered.last_grind_dist,
                    };
                    let camera = world.local_skater_identity.as_ref().ok_or("native entry local camera owner")?
                        .try_camera_producer_projection().map_err(|error|format!("native entry camera: {error:?}"))?;
                    let camera_basis = Some(std::array::from_fn(|column| camera.camera_matrix.columns()[column].xyz));
                    Some(crate::skate3_native::physics::player::PlayerEntry { transform, velocity: velocity.xyz,
                        on_board: false, state, holding_board, filtered: Some(filtered), camera_basis })
                } else { None }
            };
            *mode = Some(Box::new(crate::skate3_native::physics::player::Skate3Player::load(
                world, ground, board, skeleton, profile, entry,
            )?));
            if independent_offboard && !host_offboard {
                mode.as_mut().expect("loaded").request_offboard_replacement();
            }
        }
        // Profile stance is not the marker's requested leading foot: a switch
        // marker changes the latter without changing the skater's customization.
        mode.as_mut().expect("loaded").configure_character_stance(
            settings.try_stance_byte(player_index).map_err(|error|error.to_string())? != 0,
            actor.try_last_set_foot_forward().map_err(|error|error.to_string())?, entering)?;
        mode.as_mut().expect("loaded").configure_animation_style(self.style_preferences.expect("loaded"))?;
        if self.active_difficulty != Some(difficulty) {
            let player = mode.as_mut().expect("loaded");
            match difficulty {
                Difficulty::Skate2Default => player.set_skate2_profile(&owner.inputs().vault)?,
                Difficulty::EasyMotorized => {
                    player.set_motorized_profile(profile)?;
                }
                _ => player.set_profile(profile)?,
            }
            self.active_difficulty = Some(difficulty);
        }
        actor.try_select_skate3_movement(true).map_err(|e| e.to_string())?;
        let camera_type = match self.camera_type { 0 => 0, 1 => 1, other => return Err(format!("Invalid native camera angle {other}")) };
        mode.as_mut().expect("loaded").use_actor_gesture_channels();
        mode.as_mut().expect("loaded").set_camera_type(camera_type);
        mode.as_mut().expect("loaded").set_input_focus(self.skate3_has_focus);
        mode.as_mut().expect("loaded").set_airtime_slow_motion(self.mod_settings.airtime_slow_motion);
        // The entered online flow publishes this on the actual simulation owner.
        let online = world.sim_rate.lock().map_err(|_|"source online scoring world rate lock")?.online_game();
        mode.as_mut().expect("loaded").set_online_scoring(online);
        mode.as_mut().expect("loaded").sync_teleport(actor)?;
        Ok(())
    }
}
