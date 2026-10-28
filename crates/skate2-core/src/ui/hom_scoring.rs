//! Retained Hall of Meat HUD data and native screen notifications.
//! HOMScoringHUD826b5860/826be050/826be118; PhysOutModule_Hud827c73b0.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum ScreenState {
    Show = 1,
    Hide = 2,
    Refresh = 3,
    ChallengesCompleted = 999,
    ChallengesCleared = 1000,
}

/// Native HOMScoringHUDData (244 bytes); Reset826b1c38 clears all values.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HomHudData {
    pub dirty: bool,
    pub score: u32,
    pub multiplier: u32,
    pub rotation: f32,
    pub rotation_score: f32,
    pub wipeout_speed_score: f32,
    pub max_air_drop_score: f32,
    pub max_air_time_score: f32,
    pub wipeout_duration_score: f32,
    pub wipeout_speed: f32,
    pub max_air_drop: f32,
    pub max_air_time: f32,
    pub wipeout_duration: f32,
    pub new_car_collisions: u32,
    pub new_ped_collisions: u32,
    pub new_dmo_collisions: u32,
    pub new_skater_collisions: u32,
    pub new_body_tweak: bool,
    pub car_collision_score: [f32; 10],
    pub ped_collision_score: [f32; 10],
    pub dmo_collision_score: [f32; 10],
    pub skater_collision_score: [f32; 10],
    pub body_tweak_score: f32,
}

impl HomHudData {
    /// HOM_GetBonusScores826ba688: scores are truncated signed APT integers.
    pub fn bonus_scores(&self) -> [i32; 5] {
        [
            self.wipeout_speed_score,
            self.max_air_drop_score,
            self.max_air_time_score,
            self.wipeout_duration_score,
            self.rotation_score,
        ]
        .map(|value| value as i32)
    }

    /// HOM_GetBonusData826ba760; same order as bonus_scores.
    pub fn bonus_data(&self) -> [f32; 5] {
        [
            self.wipeout_speed,
            self.max_air_drop,
            self.max_air_time,
            self.wipeout_duration,
            self.rotation,
        ]
    }

    /// ReactivateScoring827c7230 resets only these eight values, not the score.
    pub fn reactivate_scoring(&mut self) {
        self.wipeout_speed_score = 0.0;
        self.max_air_drop_score = 0.0;
        self.max_air_time_score = 0.0;
        self.wipeout_duration_score = 0.0;
        self.wipeout_speed = 0.0;
        self.max_air_drop = 0.0;
        self.max_air_time = 0.0;
        self.wipeout_duration = 0.0;
        self.dirty = true;
    }

    /// UpdateHoM827c73b0 writes all scalar metrics each eligible simulation tick.
    /// Collision arrays retain entries beyond this tick's newly reported hits.
    pub fn update_bail(&mut self, incoming: &Self) {
        self.score = incoming.score;
        self.rotation = incoming.rotation;
        self.rotation_score = incoming.rotation_score;
        self.wipeout_speed_score = incoming.wipeout_speed_score;
        self.max_air_drop_score = incoming.max_air_drop_score;
        self.max_air_time_score = incoming.max_air_time_score;
        self.wipeout_duration_score = incoming.wipeout_duration_score;
        self.wipeout_speed = incoming.wipeout_speed;
        self.max_air_drop = incoming.max_air_drop;
        self.max_air_time = incoming.max_air_time;
        self.wipeout_duration = incoming.wipeout_duration;
        self.new_car_collisions = incoming.new_car_collisions;
        self.new_ped_collisions = incoming.new_ped_collisions;
        self.new_dmo_collisions = incoming.new_dmo_collisions;
        for (count, output, input) in [
            (
                self.new_car_collisions,
                &mut self.car_collision_score,
                &incoming.car_collision_score,
            ),
            (
                self.new_ped_collisions,
                &mut self.ped_collision_score,
                &incoming.ped_collision_score,
            ),
            (
                self.new_dmo_collisions,
                &mut self.dmo_collision_score,
                &incoming.dmo_collision_score,
            ),
        ] {
            assert!(count <= 10, "native HOM collision array capacity exceeded");
            output[..count as usize].copy_from_slice(&input[..count as usize]);
        }
        self.new_body_tweak = incoming.new_body_tweak;
        self.body_tweak_score = incoming.body_tweak_score;
        // Native827c7820/827c783c explicitly suppress skater collision output.
        self.new_skater_collisions = 0;
        self.skater_collision_score[0] = 0.0;
        // UpdateHoM does not assign mMultiplier.
        self.dirty = true;
    }
}

/// Completed scoring output, sampled by PhysOutModule_Hud once per physics tick.
#[derive(Clone, Debug, Default)]
pub struct HomPhysicsFrame {
    pub just_wiped_out: bool,
    /// Conjunction of native kIsHoMHUDEnabled and mHoM_IsEffectEnabled.
    pub effect_enabled: bool,
    pub bail_in_progress: bool,
    pub bail_completed: bool,
    pub just_reactivated: bool,
    pub data: HomHudData,
}

#[derive(Clone, Debug, Default)]
pub struct HomFrame {
    pub commands: Vec<ScreenState>,
    pub data: HomHudData,
    pub visible: bool,
}

#[derive(Debug, Default)]
pub struct HomScoringHud {
    visible: bool,
    completed_challenges: u32,
    data: HomHudData,
}

impl HomScoringHud {
    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn data(&self) -> &HomHudData {
        &self.data
    }

    /// Show826be050 only sends transitions; repeated Show never restarts a clip.
    pub fn show(&mut self, visible: bool, commands: &mut Vec<ScreenState>) {
        if self.visible != visible {
            commands.push(if visible {
                ScreenState::Show
            } else {
                ScreenState::Hide
            });
            self.visible = visible;
        }
    }

    /// HOM vtable820ace60+8 selects shared Refresh826be0b8.
    /// Hidden HUDs retain dirtiness, while challenge edges are visibility independent.
    pub fn update(&mut self, completed_challenges: u32, commands: &mut Vec<ScreenState>) {
        if self.visible && self.data.dirty {
            commands.push(ScreenState::Refresh);
            self.data.dirty = false;
        }
        if completed_challenges != 0 && self.completed_challenges == 0 {
            commands.push(ScreenState::ChallengesCompleted);
        } else if completed_challenges == 0 && self.completed_challenges != 0 {
            commands.push(ScreenState::ChallengesCleared);
        }
        self.completed_challenges = completed_challenges;
    }

    /// PhysOutModule_Hud order: StartWipeout, UpdateHoM, ReactivateScoring,
    /// then the frontend's HOMScoringHUD::Update.
    pub fn advance(&mut self, frame: &HomPhysicsFrame, completed_challenges: u32) -> HomFrame {
        let mut commands = Vec::new();
        if frame.just_wiped_out {
            if self.data.score != 0 {
                self.data.score = 0;
                self.data.dirty = true;
            }
            self.show(frame.effect_enabled, &mut commands);
        }
        if frame.effect_enabled && (frame.bail_in_progress || frame.bail_completed) {
            self.data.update_bail(&frame.data);
        }
        if frame.just_reactivated {
            self.show(false, &mut commands);
            self.data.reactivate_scoring();
        }
        self.update(completed_challenges, &mut commands);
        HomFrame {
            commands,
            data: self.data.clone(),
            visible: self.visible,
        }
    }
}
