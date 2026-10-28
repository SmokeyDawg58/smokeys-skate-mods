//! `SetTrickHeight::Begin`82cc6508 on the retained Motion, animation and
//! GlobalAttributeCollection owners.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_motion_load::MotionLoadError,
    input_cvar_runtime::InputCvarReferences,
    play_animation::{FastString30, FastString36},
};
use std::sync::Arc;

/// `GlobalAttributeCollection.anim_motion_jumping` at +0x124, shared with the
/// retained Skeleton jump path. Its two bool getters use the same Vault data.
pub(crate) struct TrickHeightAttributes {
    instance: AttributeRuntimeInstance,
}
impl TrickHeightAttributes {
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        let instance = vault.instance(0x90c3_decf_0105_631e, 0xac26_0b44_fa3c_a0a4);
        instance.try_cached_layout(0x5c0)?;
        Ok(Self { instance })
    }

    fn boolean(&self, key: u64) -> Result<bool, E> {
        let data = self
            .instance
            .try_get_data(key, 0)
            .map_err(|error| E::Load(MotionLoadError::AttributeRuntime(error)))?;
        // The generated getter points at a one-byte bool. Missing fields
        // point to zeroed DefaultDataArea(1), even with a cached layout.
        data.map_or(Ok(false), |value| {
            value
                .bytes()
                .first()
                .map(|byte| *byte != 0)
                .ok_or(E::MissingOwner("anim_motion jumping bool"))
        })
    }

    fn use_gesture_speed(&self) -> Result<bool, E> {
        self.boolean(0xe384_db9b_3cb5_a40d)
    }

    fn clamp_gesture_to_antic(&self) -> Result<bool, E> {
        self.boolean(0x7729_f2ea_8837_241e)
    }
}

impl ActorConstruction {
    pub(crate) fn motion_set_trick_height_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        from: FastString30,
        rename: FastString30,
        set_manually: bool,
        set_to_value: f32,
        cvars: Option<&InputCvarReferences>,
    ) -> Result<(), E> {
        // Native resolves IAnimatable before IMotionGraph, including the
        // manually supplied-value path.
        let animation = self.lifecycle_animation(0)?;
        self.check_motion_runtime_context(context)?;
        let value = if set_manually {
            set_to_value
        } else if let Some(value) = self.motion_runtime_intent_value(
            context,
            &FastString36::encode("TrickHeight").expect("native literal"),
        )? {
            value
        } else {
            let gesture = self
                .motion_runtime_intent_value(
                    context,
                    &FastString36::encode("GestureSpeed").expect("native literal"),
                )?
                .unwrap_or(0.0);
            let antic = match self.motion_runtime_tree_attribute(context, from)? {
                Some(attribute) => attribute
                    .scalar_value()
                    .ok_or(E::PendingAnimationOperation("SetTrickHeight tree scalar"))?,
                None => 0.0,
            };
            // The debug branch formats a value but its draw call is BLR in
            // this binary. The live target read still precedes Vault getters.
            let _draw_debug = cvars
                .ok_or(E::MissingOwner("SetTrickHeight debug cvar"))?
                .try_draw_trick_height_debug_info()
                .map_err(E::Cvar)?;
            let services = self
                .action_runtime_services()
                .map_err(E::Actor)?
                .ok_or(E::MissingOwner("SetTrickHeight globals"))?;
            let use_gesture = services.trick_height.use_gesture_speed()?;
            let clamp = services.trick_height.clamp_gesture_to_antic()?;
            if !use_gesture {
                antic
            } else if !clamp {
                gesture
            } else {
                // Two PPC fsel instructions, including their NaN and signed
                // zero behavior. f32::clamp/min/max choose different values.
                let floor = if -gesture >= 0.0 { 0.0 } else { gesture };
                if antic - floor >= 0.0 { floor } else { antic }
            }
        };
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(rename, value, true, -1)
            .map_err(E::AnimationWrite)
    }
}
