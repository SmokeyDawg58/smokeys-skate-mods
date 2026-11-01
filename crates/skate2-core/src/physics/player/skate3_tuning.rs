//! Source-backed difficulty settings scoped to one real human physical owner.
//! S2 keeps its solver and authored defaults; only selected, corresponding
//! physical parameters read S3's embedded physics_mode profile.
use crate::{physical_player_assembly::PhysicalPlayerAssemblyOwner, ui::gameplay::Difficulty};
use std::sync::{Arc, OnceLock};

pub(crate) struct AirAssistSettings {
    pub spin: skate3_core::air::body_spin::BodySpinSettings,
    pub flip: skate3_core::air::body_flip::BodyFlipSettings,
}
impl std::fmt::Debug for AirAssistSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("embedded source S3 air-assist settings")
    }
}
#[derive(Debug)]
pub(crate) struct Tuning {
    pub profile: &'static str,
    pub easy_body_spins: bool,
    pub perfect_body_flips: bool,
    pub auto_push_enabled: bool,
    pub auto_push_speed_threshold: f32,
    pub air_assists: Arc<AirAssistSettings>,
    pub jump_minimum: f32,
    pub jump_maximum: f32,
    pub push_changes: [f32; 2],
    pub motor_enabled: bool,
    pub motor_speed: f32,
    pub pump: f32,
    pub absorption: f32,
    pub unintentional_pump: f32,
    pub automatic_spin_speed: f32,
    pub ground_wipeout_xz: f32,
    pub grind_lock_distance: f32,
}
impl Tuning {
    pub(crate) fn select(difficulty: Difficulty) -> Result<Option<Arc<Self>>, String> {
        let index = match difficulty {
            Difficulty::Skate2Default => return Ok(None),
            Difficulty::Easy => 0,
            Difficulty::Normal => 1,
            Difficulty::Hardcore => 2,
            Difficulty::Motorized => 3,
            Difficulty::EasyMotorized => 4,
            Difficulty::HardcoreMotorized => 5,
            Difficulty::NormalMotorized => 6,
        };
        static PROFILES: OnceLock<Result<Vec<Arc<Tuning>>, String>> = OnceLock::new();
        let profiles = PROFILES
            .get_or_init(|| {
                let data = super::embedded_collections()?;
                let settings = crate::skate3_native::physics::air_reckoning::load_assist_settings(&data)?;
                let air_assists = Arc::new(AirAssistSettings {
                    spin: settings.body_spin,
                    flip: settings.body_flip,
                });
                let mut profiles = Vec::new();
                for (key, motor) in [
                    ("easy", false),
                    ("normal", false),
                    ("hardcore", false),
                    ("motorized", true),
                    ("easy", true),
                    ("hardcore", true),
                    ("normal", true),
                ] {
                    let f = |field| data.float("physics_mode", key, field);
                    let motor_key = if motor { "motorized" } else { key };
                    let profile = Self {
                        profile: if key == "easy" && motor {
                            "easy_motorized"
                        } else if key == "hardcore" && motor {
                            "hardcore_motorized"
                        } else if key == "normal" && motor {
                            "normal_motorized"
                        } else {
                            key
                        },
                        easy_body_spins: data.boolean("physics_mode", key, "EasyBodySpins")?,
                        perfect_body_flips: data.boolean("physics_mode", key, "PerfectBodyFlips")?,
                        auto_push_enabled: data.boolean("physics_mode", key, "AutoPushEnabled")?,
                        auto_push_speed_threshold: data.float("physics_push", "default", "Hash_501D5581043D7D3C")?,
                        air_assists: Arc::clone(&air_assists),
                        jump_minimum: f("JumpMinHeight")?,
                        jump_maximum: f("JumpMaxHeight")?,
                        push_changes: [f("MaxPushDVStart")?, f("MaxPushDVEnd")?],
                        motor_enabled: data.boolean("physics_mode", motor_key, "MotorEnabled")?,
                        motor_speed: data.float("physics_mode", motor_key, "MotorTopSpeed")?,
                        pump: f("PumpEffectFactor")?,
                        absorption: f("PumpEffectFactorAbsorption")?,
                        unintentional_pump: f("UnintentionalPumpScalar")?,
                        automatic_spin_speed: f("MaxAutoBodySpinSpeed")?,
                        ground_wipeout_xz: f("Wipeout_GroundXZAcceleration")?,
                        grind_lock_distance: f("GrindLockDist")?,
                    };
                    if [
                        profile.auto_push_speed_threshold,
                        profile.jump_minimum,
                        profile.jump_maximum,
                        profile.push_changes[0],
                        profile.push_changes[1],
                        profile.motor_speed,
                        profile.pump,
                        profile.absorption,
                        profile.unintentional_pump,
                        profile.automatic_spin_speed,
                        profile.ground_wipeout_xz,
                        profile.grind_lock_distance,
                    ]
                    .iter()
                    .any(|value| !value.is_finite() || *value < 0.)
                    {
                        return Err(format!("Invalid embedded difficulty {key}"));
                    }
                    profiles.push(Arc::new(profile));
                }
                Ok(profiles)
            })
            .as_ref()
            .map_err(Clone::clone)?;
        Ok(Some(Arc::clone(&profiles[index])))
    }
    /// TU3 Toolkit_CalcPushAcceleration82D94930..82D94998, the same
    /// ordered fsel tests and fused interpolation as riding::push.
    pub(crate) fn push_limit(&self, signed_speed: f32, maximum_speed: f32) -> f32 {
        let speed_fraction = signed_speed / maximum_speed;
        let nonnegative = if -speed_fraction >= 0. {
            0.
        } else {
            speed_fraction
        };
        let fraction = if 1. - nonnegative >= 0. {
            nonnegative
        } else {
            1.
        };
        (1. - fraction).mul_add(self.push_changes[0], fraction * self.push_changes[1])
    }
    pub(crate) fn pumping(&self, key: u64) -> Option<f32> {
        use skate2_formats::attrib::string_hash64 as hash;
        if key == hash("PumpEffectFactor") {
            Some(self.pump)
        } else if key == hash("PumpEffectFactorAbsorption") {
            Some(self.absorption)
        } else if key == hash("UnintentionalPumpScalar") {
            Some(self.unintentional_pump)
        } else {
            None
        }
    }
}
pub(crate) fn set_owner(
    owner: &PhysicalPlayerAssemblyOwner,
    tuning: Option<Arc<Tuning>>,
) -> Result<(), String> {
    let mut current = owner
        .skate3_tuning
        .lock()
        .map_err(|_| "native difficulty lock")?;
    let same = match (&*current, &tuning) {
        (None, None) => true,
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        _ => false,
    };
    if same {
        return Ok(());
    }
    owner
        .pumping_attributes()
        .lock()
        .map_err(|_| "native pumping difficulty lock")?
        .set_mod_tuning(tuning.clone());
    match &tuning {
        Some(tuning) => crate::diagnostics::test_log::event(
            "difficulty_profile",
            format_args!(
                "solver=skate2 selected={} jump={:?} push={:?} motor={} motor_speed={} grind_lock={}",
                tuning.profile,
                [tuning.jump_minimum, tuning.jump_maximum],
                tuning.push_changes,
                tuning.motor_enabled,
                tuning.motor_speed,
                tuning.grind_lock_distance
            ),
        ),
        None => crate::diagnostics::test_log::event(
            "difficulty_profile",
            format_args!("solver=skate2 selected=skate2 authored_defaults=true"),
        ),
    }
    *current = tuning;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn skate3_native_difficulty_push_limits_match_source_kernel() {
        use skate3_core::{
            math::Vector3,
            riding::push::{PushInput, PushLimits, calculate_acceleration},
        };
        assert!(Tuning::select(Difficulty::Skate2Default).unwrap().is_none());
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Normal,
            Difficulty::Hardcore,
            Difficulty::Motorized,
            Difficulty::EasyMotorized,
        ] {
            let tuning = Tuning::select(difficulty).unwrap().unwrap();
            for speed in [-5., 0., 1., 20.] {
                let result = calculate_acceleration(
                    PushInput {
                        flags_2468: 0x0200_0000,
                        flags_2472: 0,
                        target_speed: speed + 10.,
                        current_speed: speed,
                        absolute_body_speed: speed.abs(),
                        scale: 1.,
                        delta_seconds: 1.,
                        direction: Vector3::new(0., 0., 1.),
                    },
                    PushLimits {
                        maximum_pushable_speed: 100.,
                        low_speed_change: tuning.push_changes[0],
                        high_speed_change: tuning.push_changes[1],
                    },
                );
                assert_eq!(
                    result.vector.z.to_bits(),
                    tuning.push_limit(speed, 100.).to_bits(),
                    "{difficulty:?} speed={speed}"
                );
            }
        }
    }
}
