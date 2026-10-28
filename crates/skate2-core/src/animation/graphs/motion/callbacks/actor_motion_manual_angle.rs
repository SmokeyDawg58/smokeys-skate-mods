//! SetManualAngle: Create82ca2be8, Begin82caf320, Update82cc2920.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_motion_load::MotionLoadError,
    physics::PointGraph,
    play_animation::{FastString30, FastString36},
};
use std::sync::Arc;

pub(crate) struct ManualAngleAttributes {
    vault: Arc<AttributeRuntimeVault>,
    instance: AttributeRuntimeInstance,
}
impl ManualAngleAttributes {
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        // Global+160 constructor828b30e8..310c, anim_motion class.
        let instance = vault.instance(0x90c3_decf_0105_631e, 0x5b83_4c31_0118_79cd);
        instance.try_cached_layout(0x5c0)?;
        Ok(Self {
            vault: Arc::clone(vault),
            instance,
        })
    }
    fn scalar(&self, key: u64) -> Result<f32, E> {
        self.instance
            .try_scalar_word(key, 0)
            .map(|v| f32::from_bits(v.unwrap_or(0)))
            .map_err(|e| E::Load(MotionLoadError::AttributeRuntime(e)))
    }
    fn target(&self, input: f32) -> Result<f32, E> {
        let layout = self
            .instance
            .try_cached_layout(0x5c0)
            .map_err(|e| E::Load(MotionLoadError::AttributeRuntime(e)))?;
        let word = |offset: usize| {
            layout.as_ref().map_or(0.0, |d| {
                f32::from_be_bytes(
                    d.bytes()[offset..offset + 4]
                        .try_into()
                        .expect("checked layout"),
                )
            })
        };
        let value = PointGraph::<8> {
            x: std::array::from_fn(|i| word(0x420 + i * 4)),
            y: std::array::from_fn(|i| word(0x440 + i * 4)),
        }
        .evaluate(input.abs());
        Ok(if input >= 0.0 { value } else { -value })
    }
}
// Match fsel subtraction/selection ordering, including unordered operands.
fn bounded(value: f32, limit: f32) -> f32 {
    let lower = if -limit - value >= 0.0 { -limit } else { value };
    if limit - lower >= 0.0 { lower } else { limit }
}
impl ActorConstruction {
    pub(crate) fn motion_manual_angle_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        name: FastString30,
        angle: &mut f32,
        velocity: &mut f32,
    ) -> Result<(), E> {
        let animation = self.lifecycle_animation(0)?;
        let input = self
            .motion_runtime_intent_value(
                context,
                &FastString36::encode("Manual").expect("native intent"),
            )?
            .unwrap_or(0.0);
        let services = self
            .action_runtime_services()
            .map_err(E::Actor)?
            .ok_or(E::MissingOwner("SetManualAngle services"))?;
        let attributes = &services.manual_angle;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        if !Arc::ptr_eq(&attributes.vault, &physical.owner().inputs().vault) {
            return Err(E::ForeignOwner("SetManualAngle global Vault"));
        }
        let target = attributes.target(input)?;
        let acceleration_limit = attributes.scalar(0xaed4_1843_fd93_af02)?;
        let velocity_limit = attributes.scalar(0xa353_ce16_70d3_aa40)?;
        let wanted_velocity = bounded(target - *angle, velocity_limit);
        *velocity += bounded(wanted_velocity - *velocity, acceleration_limit);
        *angle += *velocity;
        if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
            eprintln!(
                "known air: manual angle intent={input} target={target} angle={} velocity={} limits=({acceleration_limit}, {velocity_limit})",
                *angle, *velocity
            );
        }
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(name, *angle, false, -1)
            .map_err(E::AnimationWrite)
    }
}
