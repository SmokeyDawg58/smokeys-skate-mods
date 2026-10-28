//! HippyJumpAntic82ca80f8 and AirDismounting82ca5db8/82cb4248.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::{
    attribute_runtime::AttributeRuntimeInstance, generated_motion_load::MotionLoadError,
    physics::PointGraph, play_animation::FastString30, reckoning_numeric::ReckoningNumeric,
};
impl ActorConstruction {
    pub(crate) fn motion_hippy_antic(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        name: &FastString30,
        attributes: &AttributeRuntimeInstance,
        time: &mut f32,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let animation = self.lifecycle_animation(0)?;
        *time += f32::from_bits(0x3c888889); //Native fixed update time, not controller dt.
        let data = attributes
            .try_get_data(0x5bdf_9f0a_81a2_76b0, 0)
            .map_err(|e| E::Load(MotionLoadError::AttributeRuntime(e)))?;
        let word = |offset: usize| -> Result<f32, E> {
            match &data {
                None => Ok(0.0),
                Some(d) => Ok(f32::from_be_bytes(
                    d.bytes()
                        .get(offset..offset + 4)
                        .ok_or(E::MissingOwner("HippyJumpAntic graph data"))?
                        .try_into()
                        .unwrap(),
                )),
            }
        };
        let mut x = [0.0; 8];
        let mut y = [0.0; 8];
        for i in 0..8 {
            x[i] = word(16 + i * 4)?;
            y[i] = word(48 + i * 4)?;
        }
        let value = PointGraph::<8> { x, y }.evaluate(*time);
        let result = animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(name.clone(), value, false, -1)
            .map_err(E::AnimationWrite);
        result
    }
    pub(crate) fn motion_air_dismounting(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        frames: &mut i32,
        begin: bool,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        if begin {
            let owner = self.lifecycle_animation(0)?;
            let animation = owner.lock().map_err(|_| E::Lock)?;
            let root = animation
                .base()
                .and_then(|b| b.body_root.as_ref())
                .ok_or(E::MissingOwner("AirDismounting base animation"))?;
            let time = animation
                .body_reference_clock(root, true)
                .map_err(E::AnimationBody)?;
            let length = animation
                .body_reference_clock(root, false)
                .map_err(E::AnimationBody)?;
            let remaining = (length - time) * f32::from_bits(0x426fffff);
            let value = if 1.0 - remaining >= 0.0 {
                1.0
            } else {
                remaining
            };
            *frames = crate::reckoning_numeric::HostReckoningNumeric
                .truncate_i32(value)
                .map_err(E::ConditionNumeric)?;
        } else {
            let owner = self.lifecycle_animation(2)?;
            let _animatable = self.lifecycle_animation(0)?;
            if self
                .runtime_required_tree_attribute_from_checked_context(
                    FastString30::encode("airdismountrevert").unwrap(),
                )?
                .is_some()
            {
                owner
                    .lock()
                    .map_err(|_| E::Lock)?
                    .try_request_air_dismount_revert(*frames)
                    .map_err(E::AnimationWrite)?;
            }
        }
        Ok(())
    }
}
