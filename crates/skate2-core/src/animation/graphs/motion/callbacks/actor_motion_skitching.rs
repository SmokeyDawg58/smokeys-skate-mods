//! Skitch entry/holding callbacks, verified against 82c95de8/82c95c80,
//! 82cb4b10/82cb48d0. Reuses the behaviour's existing anim_skitching Vault owner.
#[path = "actor_motion_skitching_kernel.rs"]
pub(crate) mod kernel;
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::{
    attribute_runtime::AttributeRuntimeInstance, generated_motion_load::MotionLoadError,
    input_cvar_runtime::InputCvarReferences, physics::PointGraph, play_animation::FastString30,
};
use kernel::{EnterInstance, HoldingInstance};

pub(super) fn scalar(a: &AttributeRuntimeInstance, key: u64) -> Result<f32, E> {
    a.try_scalar_word(key, 0)
        .map(|v| f32::from_bits(v.unwrap_or(0)))
        .map_err(|e| E::Load(MotionLoadError::AttributeRuntime(e)))
}
pub(super) fn absorb(a: &AttributeRuntimeInstance, speed: f32) -> Result<f32, E> {
    // GetAttributePointer82c8cca8: PointNegGraphData8, x+10/y+30;
    // missing data uses the original zeroed DefaultDataArea(0x50).
    let data = a
        .try_get_data(0x1c04_030e_5b00_a8ea, 0)
        .map_err(|e| E::Load(MotionLoadError::AttributeRuntime(e)))?;
    let word = |offset| -> Result<f32, E> {
        match &data {
            None => Ok(0.0),
            Some(d) => Ok(f32::from_be_bytes(
                d.bytes()
                    .get(offset..offset + 4)
                    .ok_or(E::MissingOwner("Skitch absorb graph data"))?
                    .try_into()
                    .unwrap(),
            )),
        }
    };
    let mut x = [0.0; 8];
    let mut y = [0.0; 8];
    for i in 0..8 {
        x[i] = word(0x10 + i * 4)?;
        y[i] = word(0x30 + i * 4)?;
    }
    Ok(PointGraph::<8> { x, y }.evaluate(speed))
}
impl ActorConstruction {
    pub(crate) fn motion_enter_skitch_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        data: &mut EnterInstance,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let p = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = p.owner().phys_out().lock().map_err(|_| E::Lock)?;
        data.begin(
            output
                .try_animation_output()
                .map_err(E::PhysOut)?
                .skitch_crouch,
        );
        Ok(())
    }
    pub(crate) fn motion_skitch_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        a: &AttributeRuntimeInstance,
        data: &mut HoldingInstance,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let p = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = p.owner().phys_out().lock().map_err(|_| E::Lock)?;
        data.initial_crouch = output
            .try_animation_output()
            .map_err(E::PhysOut)?
            .skitch_crouch;
        data.absorb_speed = absorb(
            a,
            output
                .try_ground_output()
                .map_err(E::PhysOut)?
                .skitch_contact_speed,
        )?;
        Ok(())
    }
    pub(super) fn skitch_debug_overlay(&self, enabled: bool) -> Result<(), E> {
        if enabled {
            self.action_runtime_services()
                .map_err(E::Actor)?
                .as_ref()
                .ok_or(E::MissingOwner("Skitch debug overlay"))?
                .overlay
                .require()
                .map_err(E::SkitchOverlay)?;
        }
        Ok(())
    }
    pub(crate) fn motion_enter_skitch_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        a: &AttributeRuntimeInstance,
        data: &mut EnterInstance,
        cvars: &InputCvarReferences,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let p = self.cached_physical_allocation().map_err(E::Physical)?;
        let animation = self.lifecycle_animation(0)?;
        let (height, contact_time) = {
            let output = p.owner().phys_out().lock().map_err(|_| E::Lock)?;
            let ground = output.try_ground_output().map_err(E::PhysOut)?;
            (ground.skitch_spline_height, ground.time_to_skitch)
        };
        let crouch = height + scalar(a, 0x6e6e_fe7c_f9be_f4b8)?;
        let long = scalar(a, 0xbdfd_21f2_a968_28fd)?;
        let short = scalar(a, 0x8738_e31e_171e_2a87)?;
        let mut animation = animation.lock().map_err(|_| E::Lock)?;
        let root = animation
            .base()
            .and_then(|base| base.body_root.as_ref())
            .ok_or(E::MissingOwner("EnterSkitching base animation"))?;
        let time = animation
            .body_reference_clock(root, true)
            .map_err(E::AnimationBody)?;
        let length = animation
            .body_reference_clock(root, false)
            .map_err(E::AnimationBody)?;
        if time > length {
            return Err(E::MissingOwner("EnterSkitching animation time <= length"));
        }
        let reach = kernel::reach(data, time, length, contact_time, short, long);
        self.skitch_debug_overlay(cvars.try_skitch_enter_debug().map_err(E::Cvar)?)?;
        animation
            .try_add_gesture_attribute(FastString30::encode("Crouch").unwrap(), crouch, true, -1)
            .map_err(E::AnimationWrite)?;
        animation
            .try_add_gesture_attribute(FastString30::encode("reachspeed").unwrap(), reach, true, -1)
            .map_err(E::AnimationWrite)
    }
    pub(crate) fn motion_skitch_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        a: &AttributeRuntimeInstance,
        data: &mut HoldingInstance,
        cvars: &InputCvarReferences,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let p = self.cached_physical_allocation().map_err(E::Physical)?;
        let animation = self.lifecycle_animation(0)?;
        let (height, speed) = {
            let output = p.owner().phys_out().lock().map_err(|_| E::Lock)?;
            let ground = output.try_ground_output().map_err(E::PhysOut)?;
            (ground.skitch_spline_height, ground.skitch_contact_speed)
        };
        let crouch = height + scalar(a, 0x6e6e_fe7c_f9be_f4b8)?;
        let target = absorb(a, speed)?;
        data.absorb_speed = kernel::limit(
            target,
            data.absorb_speed,
            cvars.try_skitch_absorb_delta().map_err(E::Cvar)?,
        );
        self.skitch_debug_overlay(cvars.try_skitch_draw_debug().map_err(E::Cvar)?)?;
        let mut animation = animation.lock().map_err(|_| E::Lock)?;
        animation
            .try_add_gesture_attribute(FastString30::encode("Crouch").unwrap(), crouch, true, -1)
            .map_err(E::AnimationWrite)?;
        animation
            .try_add_gesture_attribute(
                FastString30::encode("absorbspeed").unwrap(),
                data.absorb_speed,
                true,
                -1,
            )
            .map_err(E::AnimationWrite)
    }
}
