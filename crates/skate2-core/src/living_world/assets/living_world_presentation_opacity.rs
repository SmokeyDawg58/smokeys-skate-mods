//! LivingWorld presentation opacity: Update82811bd8 / EvaluateOpacityDistance828060f8.
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    camera_compass::runtime::math::unit_fraction,
    reckoning_numeric::{HostReckoningNumeric, try_length},
};

pub(crate) const MODELS_CLASS: u64 = 0x4e74_0a66_70f2_41a7;

/// PDB tLWOpacityDistance: opaque distance, transparent distance, then speed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LivingWorldOpacityDistance {
    pub opaque: f32,
    pub transparent: f32,
    pub speed: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LivingWorldOpacityParameters {
    pub far_min_speed: LivingWorldOpacityDistance,
    pub far_max_speed: LivingWorldOpacityDistance,
    pub near_min_speed: LivingWorldOpacityDistance,
    pub near_max_speed: LivingWorldOpacityDistance,
}

impl LivingWorldOpacityParameters {
    pub(crate) fn try_read(
        instance: &AttributeRuntimeInstance,
    ) -> Result<Self, AttributeRuntimeError> {
        fn read(
            instance: &AttributeRuntimeInstance,
            key: u64,
        ) -> Result<LivingWorldOpacityDistance, AttributeRuntimeError> {
            let Some(data) = instance.try_get_data(key, 0)? else {
                return Ok(LivingWorldOpacityDistance::default());
            };
            let bytes = data.bytes();
            if bytes.len() < 12 {
                return Err(AttributeRuntimeError::PayloadSize {
                    key,
                    expected: 12,
                    actual: bytes.len(),
                });
            }
            let word = |offset| {
                f32::from_be_bytes(bytes[offset..offset + 4].try_into().expect("checked size"))
            };
            Ok(LivingWorldOpacityDistance {
                opaque: word(0),
                transparent: word(4),
                speed: word(8),
            })
        }
        // Source captures the high-speed node before looking up the low-speed one.
        let far_max_speed = read(instance, 0x9fcf_dbea_56ba_4733)?;
        let far_min_speed = read(instance, 0x73b6_874c_7b46_c7c6)?;
        let near_max_speed = read(instance, 0x4e5e_99c0_ab48_f75f)?;
        let near_min_speed = read(instance, 0x8098_b1f0_7fd2_8112)?;
        Ok(Self {
            far_min_speed,
            far_max_speed,
            near_min_speed,
            near_max_speed,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LivingWorldOpacitySettings {
    pub enabled: bool,
    pub fade_in_frames: f32,
}

/// Per-presentation-entity state. The DMO creation path retains the base
/// constructor's spawn value1; it does not start a fabricated fade from zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LivingWorldOpacityState {
    pub opacity: f32,
    pub ghosted: bool,
    spawn_opacity: f32,
}

impl Default for LivingWorldOpacityState {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            ghosted: false,
            spawn_opacity: 1.0,
        }
    }
}

impl LivingWorldOpacityState {
    /// FullHandFacePedestrian constructor828127e4 overrides the base fade.
    pub fn with_spawn_opacity(spawn_opacity:f32)->Self {
        Self{spawn_opacity,..Self::default()}
    }
    /// Called once per presentation update, using camera matrix1's position
    /// and the actual category simulation-centre velocity (not DMO velocity).
    pub fn try_update(
        &mut self,
        settings: LivingWorldOpacitySettings,
        parameters: LivingWorldOpacityParameters,
        simulation_center_velocity: [f32; 4],
        bounds_center: [f32; 3],
        camera_position: [f32; 3],
    ) -> Result<(), String> {
        self.spawn_opacity = unit_fraction((1.0 / settings.fade_in_frames) + self.spawn_opacity);
        if !settings.enabled {
            self.opacity = 1.0;
            self.ghosted = false;
            return Ok(());
        }
        let mut numeric = HostReckoningNumeric;
        let speed = try_length(simulation_center_velocity, &mut numeric)
            .map_err(|error| format!("LivingWorld opacity speed: {error:?}"))?
            * f32::from_bits(0x4066_6666);
        let delta = [
            camera_position[0] - bounds_center[0],
            camera_position[1] - bounds_center[1],
            camera_position[2] - bounds_center[2],
            0.0,
        ];
        let distance = try_length(delta, &mut numeric)
            .map_err(|error| format!("LivingWorld opacity distance: {error:?}"))?;
        let far = evaluate_distance(speed, parameters.far_min_speed, parameters.far_max_speed);
        let near = evaluate_distance(speed, parameters.near_min_speed, parameters.near_max_speed);
        let distance_opacity = distance_opacity(distance, far, near);
        // Native FSEL minimum; NaNs follow its operand order.
        self.opacity = if distance_opacity - self.spawn_opacity >= 0.0 {
            self.spawn_opacity
        } else {
            distance_opacity
        };
        self.ghosted = self.opacity < 1.0;
        Ok(())
    }
}

/// Native max(delta,0) bounded by fabs(delta), preserving its FSEL order.
fn positive_distance(delta: f32) -> f32 {
    let positive = if -delta >= 0.0 { 0.0 } else { delta };
    let absolute = f32::from_bits(delta.to_bits() & 0x7fff_ffff);
    if absolute - positive >= 0.0 {
        positive
    } else {
        absolute
    }
}

fn evaluate_distance(
    speed: f32,
    low: LivingWorldOpacityDistance,
    high: LivingWorldOpacityDistance,
) -> LivingWorldOpacityDistance {
    if low.speed >= high.speed {
        return low;
    }
    let t = unit_fraction(positive_distance(speed - low.speed) / (high.speed - low.speed));
    LivingWorldOpacityDistance {
        opaque: (high.opaque - low.opaque).mul_add(t, low.opaque),
        transparent: (high.transparent - low.transparent).mul_add(t, low.transparent),
        speed: (high.speed - low.speed).mul_add(t, low.speed),
    }
}

#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "native BLE/BGE unordered fallthrough"
)]
fn distance_opacity(
    distance: f32,
    far: LivingWorldOpacityDistance,
    near: LivingWorldOpacityDistance,
) -> f32 {
    if !(distance <= far.opaque) {
        let width = far.transparent - far.opaque;
        let t = if !(width <= 0.0) {
            unit_fraction(positive_distance(distance - far.opaque) / width)
        } else {
            0.0
        };
        1.0 - t
    } else if !(distance >= near.opaque) {
        let width = near.opaque - near.transparent;
        if !(width <= 0.0) {
            unit_fraction(positive_distance(distance - near.transparent) / width)
        } else {
            1.0
        }
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_dmo_asset_distances_fade_between_45_and_55() {
        let far = LivingWorldOpacityDistance {
            opaque: 45.0,
            transparent: 55.0,
            speed: 0.0,
        };
        let near = LivingWorldOpacityDistance::default();
        assert_eq!(distance_opacity(0.0, far, near), 1.0);
        assert_eq!(distance_opacity(45.0, far, near), 1.0);
        assert_eq!(distance_opacity(50.0, far, near), 0.5);
        assert_eq!(distance_opacity(55.0, far, near), 0.0);
        assert_eq!(distance_opacity(60.0, far, near), 0.0);
        assert_eq!(evaluate_distance(90.0, far, near), far);
    }

    #[test]
    fn speed_interpolation_and_near_distance_use_authored_endpoints() {
        let low = LivingWorldOpacityDistance {
            opaque: 10.0,
            transparent: 20.0,
            speed: 0.0,
        };
        let high = LivingWorldOpacityDistance {
            opaque: 30.0,
            transparent: 40.0,
            speed: 20.0,
        };
        assert_eq!(
            evaluate_distance(10.0, low, high),
            LivingWorldOpacityDistance {
                opaque: 20.0,
                transparent: 30.0,
                speed: 10.0
            }
        );
        let near = LivingWorldOpacityDistance {
            opaque: 4.0,
            transparent: 2.0,
            speed: 0.0,
        };
        assert_eq!(distance_opacity(3.0, low, near), 0.5);
        assert_eq!(distance_opacity(1.0, low, near), 0.0);
        assert_eq!(LivingWorldOpacityState::default().spawn_opacity, 1.0);
    }
}
