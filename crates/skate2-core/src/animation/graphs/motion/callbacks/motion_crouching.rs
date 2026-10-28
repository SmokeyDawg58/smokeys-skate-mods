//! Crouching723: Create82cb3e88, Initialize82cb3f80, Update82cca698.
//! Live reads and stores remain ordered; no per-frame tuning snapshot.
#[cfg(test)]
#[path = "motion_crouching_tests.rs"]
mod tests;
#[derive(Debug, Default)]
pub(crate) struct CrouchingInstance {
    pub(crate) vertical_velocity: f32,
    pub(crate) smoothed_input: f32,
    pub(crate) crouch: f32,
    pub(crate) delta: f32,
    pub(crate) player_controlled: bool,
    pub(crate) auto_crouch: f32,
    pub(crate) mode: u32,
    pub(crate) mode_time: f32,
    pub(crate) previous_angle: f32,
    pub(crate) previous_magnitude: f32,
    pub(crate) pump_potential: f32,
    pub(crate) since_player: f32,
}
#[derive(Clone, Copy)]
pub(crate) enum Collection {
    Crouching,
    AutoPump,
}
#[derive(Clone, Copy)]
pub(crate) enum PhysicsValue {
    CentreOfMassToDeck,
    /// SkateboardMotion+0xa4 `mSkateboardSurfaceSpeed`: Initialize82cb4160,
    /// Update auto-pump82ccb02c and final disttocog82ccb76c (not +0xa0).
    SurfaceSpeed,
    VelocityY,
    Absorption,
    GroundNormal,
    MinimumCrouch,
    PumpPotential,
}
pub(crate) trait CrouchingSource {
    type Error;
    type Scalar;
    type Skeleton;
    fn resolve(&mut self, update: bool) -> Result<(), Self::Error>;
    fn scalar_capture(
        &mut self,
        collection: Collection,
        key: u64,
    ) -> Result<Self::Scalar, Self::Error>;
    fn scalar_read(&mut self, value: &Self::Scalar) -> Result<f32, Self::Error>;
    fn scalar(&mut self, collection: Collection, key: u64) -> Result<f32, Self::Error> {
        let value = self.scalar_capture(collection, key)?;
        self.scalar_read(&value)
    }
    fn graph(
        &mut self,
        collection: Collection,
        offset: usize,
        input: f32,
    ) -> Result<f32, Self::Error>;
    fn physics(&mut self, field: PhysicsValue) -> Result<f32, Self::Error>;
    fn skeleton_capture(&mut self) -> Result<Self::Skeleton, Self::Error>;
    fn deck_compression(&mut self, skeleton: &Self::Skeleton) -> Result<f32, Self::Error>;
    fn intent(&mut self, name: &str) -> Result<Option<f32>, Self::Error>;
    fn power_sliding(&mut self) -> Result<bool, Self::Error>;
    fn use_auto_pump(&mut self) -> Result<bool, Self::Error>;
    fn pumping_time(&mut self) -> Result<f32, Self::Error>;
    fn dt(&mut self) -> Result<f32, Self::Error>;
    fn publish_motion(&mut self, name: &str) -> Result<(), Self::Error>;
    fn publish_animation(&mut self, value: f32) -> Result<(), Self::Error>;
}
fn select(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
fn lower(value: f32, bound: f32) -> f32 {
    select(bound - value, bound, value)
}
fn upper(value: f32, bound: f32) -> f32 {
    select(bound - value, value, bound)
}
fn unit(value: f32) -> f32 {
    upper(select(-value, 0.0, value), 1.0)
}
#[expect(
    clippy::field_reassign_with_default,
    reason = "native constructor initializes fields before late cvar lookup"
)]
pub(crate) fn create<S: CrouchingSource>(
    s: &mut S,
    allocated: bool,
) -> Result<Option<CrouchingInstance>, S::Error> {
    if !allocated {
        return Ok(None);
    }
    let mut value = CrouchingInstance::default();
    value.since_player = s.pumping_time()?;
    Ok(Some(value))
}
pub(crate) fn begin<S: CrouchingSource>(
    s: &mut S,
    v: &mut CrouchingInstance,
) -> Result<(), S::Error> {
    // Initialize resets all preceding fields BEFORE this late cvar read.
    let prior_time = v.since_player;
    *v = CrouchingInstance::default();
    v.since_player = prior_time;
    v.since_player = s.pumping_time()?;
    s.resolve(false)?;
    let animation = s.physics(PhysicsValue::CentreOfMassToDeck)?;
    let speed = s.physics(PhysicsValue::SurfaceSpeed)?;
    let graph = s.graph(Collection::Crouching, 0x190, speed)?;
    let baseline = s.scalar(Collection::Crouching, 0x74ea_9539_2d41_5a5d)?;
    v.crouch = 1.0 - unit((animation - baseline) / (graph - baseline));
    Ok(())
}
#[allow(clippy::neg_cmp_op_on_partial_ord)] // Raw CR complement branches include unordered.
#[expect(
    clippy::too_many_lines,
    clippy::assign_op_pattern,
    reason = "original ordered callback preserves operand and partial-store sequence"
)]
pub(crate) fn update<S: CrouchingSource>(
    s: &mut S,
    v: &mut CrouchingInstance,
) -> Result<(), S::Error> {
    use Collection::{AutoPump as A, Crouching as C};
    use PhysicsValue::{
        Absorption, GroundNormal, MinimumCrouch, PumpPotential, SurfaceSpeed, VelocityY,
    };
    s.resolve(true)?;
    let bias = -s.graph(C, 0x450, 1.0 - v.crouch)?;
    let absorption = -s.physics(Absorption)?;
    let mut compression = upper(
        lower(absorption / s.scalar(C, 0xd0c3_1cb8_445c_940a)?, -1.0),
        1.0,
    );
    v.vertical_velocity = s.physics(VelocityY)? + v.vertical_velocity; // aa18, before scalar lookup.
    v.vertical_velocity = s.scalar(C, 0x4d4b_ac56_989f_5f2b)? * v.vertical_velocity;
    if v.vertical_velocity < 0.0 {
        compression = -compression;
    }
    let ground = -s.physics(GroundNormal)?;
    let secondary = upper(
        lower(ground / s.scalar(C, 0x6bde_80ba_bf0e_b9d5)?, -1.0),
        1.0,
    );
    if compression < 0.0 {
        if secondary > 0.0 {
            compression = secondary + compression;
        }
    } else {
        compression = select(compression - secondary, compression, secondary);
    }
    if compression < 0.0 {
        if !(compression < bias) {
            compression = bias;
        }
    } else {
        compression += bias;
    }
    let minimum = s.physics(MinimumCrouch)?;
    let old_angle = v.previous_angle;
    let old_magnitude = v.previous_magnitude;
    v.previous_angle = s.intent("AutoPumpAngle")?.unwrap_or(0.0);
    v.previous_magnitude = s.intent("AutoPumpMag")?.unwrap_or(0.0);
    let angle_limit = s.scalar(A, 0x132b_67ff_08fd_39c0)?;
    let magnitude_limit = s.scalar(A, 0xdba1_abca_593a_9f9b)?;
    let new_pump = old_angle.abs() < angle_limit
        && old_magnitude < magnitude_limit
        && v.previous_angle.abs() < angle_limit
        && v.previous_magnitude > magnitude_limit;
    let potential = s.physics(PumpPotential)?;
    let blend = s.scalar(A, 0x0908_ab96_fbbe_ee9a)?;
    v.pump_potential = (1.0 - blend).mul_add(v.pump_potential, blend * potential);
    let sufficient = !(v.pump_potential < s.scalar(A, 0x2b35_f6ae_c83d_a8f4)?);
    if new_pump {
        v.mode_time = 0.0;
        v.mode = 1;
    }
    let duration = s.scalar(A, 0xddcb_6014_e96d_27e5)?;
    if v.mode == 1 && v.mode_time > duration {
        v.mode = 2;
    }
    let trigger = s.scalar(A, 0x1056_991e_714f_6fa3)?;
    if matches!(v.mode, 1 | 2) && v.crouch > trigger && sufficient {
        v.mode = 3;
        s.publish_motion("NewAutoPump")?;
    }
    let finish = s.scalar(A, 0x9b4f_5525_98bb_121a)?;
    if matches!(v.mode, 2 | 3) && !(v.crouch > minimum + finish) {
        v.mode = 0;
    }
    match v.mode {
        0 => v.auto_crouch = 0.0,
        1 => {
            let speed = s.physics(SurfaceSpeed)?;
            let target = s.graph(A, 0x590, speed)?;
            let increment = s.scalar(A, 0x383e_9275_9fc0_115c)?;
            let duration = s.scalar(A, 0xddcb_6014_e96d_27e5)?;
            v.auto_crouch = upper(v.auto_crouch + increment, target);
            v.mode_time = s.dt()? + v.mode_time;
            v.mode_time = upper(v.mode_time, duration + 1.0);
        }
        2 | 3 => {
            let speed = s.scalar(
                A,
                if v.mode == 2 {
                    0xf056_44bf_b1ef_25c6
                } else {
                    0x8df4_151a_ccba_d2ed
                },
            )?;
            let value = v.auto_crouch - speed;
            v.auto_crouch = select(-value, 0.0, value);
        }
        _ => {}
    }
    let auto_crouch = v.auto_crouch;
    let mut player = s.intent("Crouch")?.unwrap_or(0.0);
    if !s.power_sliding()? {
        let hard = s.intent("HardTurnCrouch")?.unwrap_or(0.0);
        player = select(player - hard, player, hard);
    }
    let manual = s.intent("Manual")?;
    if s.power_sliding()? && manual.is_some_and(|x| x < 0.0) {
        player = select(player - 0.4, player, 0.4);
    }
    let auto = if s.use_auto_pump()? { auto_crouch } else { 0.0 };
    let target = select(player - auto, player, auto);
    let blend = unit(s.scalar(C, 0x4546_89d2_1d4f_7436)?);
    let mut smoothed = (1.0 - blend).mul_add(v.smoothed_input, blend * target);
    if smoothed.abs() < f32::from_bits(0x3ba3_d70a) {
        smoothed = 0.0;
    }
    v.smoothed_input = smoothed;
    let mut desired = s.scalar(C, 0x0f76_bbc4_5524_d46b)? * compression;
    if player > 0.0 || auto_crouch > 0.0 || (v.player_controlled && v.crouch > minimum) {
        v.player_controlled = true;
        let difference = upper(lower(smoothed, minimum), 1.0) - v.crouch;
        let limit = -s.scalar(C, 0x3176_b692_cfd4_664e)?;
        v.since_player = 0.0;
        desired = select(difference - limit, difference, limit);
    } else {
        v.player_controlled = false;
        v.since_player += f32::from_bits(0x3c88_8889);
    }
    if v.since_player < s.pumping_time()? {
        s.publish_motion("PlayerControlledPump")?;
    }
    let captured = s.scalar_capture(C, 0x29b4_4745_28c0_a68c)?;
    let curve = s.graph(C, 0x390, 1.0 - v.crouch)?;
    let minimum_delta = -(curve * s.scalar_read(&captured)?);
    let captured = s.scalar_capture(C, 0x29b4_4745_28c0_a68c)?;
    let curve = s.graph(C, 0x390, v.crouch)?;
    let maximum_delta = s.scalar_read(&captured)? * curve;
    desired = upper(lower(desired, minimum_delta), maximum_delta);
    let minimum_delta = v.delta - s.scalar(C, 0x5159_3962_368f_fdd4)?;
    let maximum_delta = s.scalar(C, 0x5159_3962_368f_fdd4)? + v.delta;
    v.delta = upper(lower(desired, minimum_delta), maximum_delta);
    v.crouch += v.delta;
    v.crouch = upper(lower(v.crouch, minimum), 1.0);
    let skeleton = s.skeleton_capture()?;
    let scale = s.scalar(C, 0xd209_9fe0_3d6f_afbb)?;
    let compression = s.deck_compression(&skeleton)? * scale;
    if compression > v.crouch {
        v.crouch = compression;
    }
    let scale = s.scalar(C, 0xdc5b_9355_4f2f_aebf)?;
    let output = unit(-v.crouch.mul_add(scale, -1.0));
    let speed = s.physics(SurfaceSpeed)?;
    let curve = s.graph(C, 0x190, speed)?;
    let baseline = s.scalar(C, 0x74ea_9539_2d41_5a5d)?;
    let distance = (curve - baseline).mul_add(output, baseline);
    if std::env::var_os("SKATE2_TRACE_SPEED").is_some() && speed.abs() > 8.0 {
        eprintln!(
            "speed crouch: surface_speed={speed} crouch={} disttocog={distance}",
            v.crouch
        );
    }
    if std::env::var_os("SKATE2_TRACE_VERT").is_some() {
        let body = s.physics(PhysicsValue::CentreOfMassToDeck)?;
        eprintln!(
            "vert crouch: crouch={} delta={} minimum={minimum} absorption={absorption} ground={ground} deck={compression} auto={auto_crouch} mode={} player={player} controlled={} surface_speed={speed} disttocog={distance} body_com_to_deck={body}",
            v.crouch, v.delta, v.mode, v.player_controlled
        );
    }
    s.publish_animation(distance)
}
