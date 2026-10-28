//! `FingerFlipOut` CreateInstance82712148 (0x0c, float +8 zeroed),
//! Begin82cbea20 (Initialize82cbe948) and Update82cca470; End is the shared
//! BLR 825187c0.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    generated_motion_load::MotionLoadError,
    physics::PointGraph,
    play_animation::{FastString30, FastString36},
};
use std::sync::Arc;

/// GlobalAttributeCollection+0x164 `anim_motion_finger_flip`, built at
/// 0x828b3130..3154 from collection 0x41db0c4f82003a15.
pub(crate) struct FingerFlipMotionAttributes {
    vault: Arc<AttributeRuntimeVault>,
    instance: AttributeRuntimeInstance,
}
impl FingerFlipMotionAttributes {
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        let instance = vault.instance(0x90c3_decf_0105_631e, 0x41db_0c4f_8200_3a15);
        instance.try_cached_layout(0x5c0)?;
        Ok(Self {
            vault: Arc::clone(vault),
            instance,
        })
    }

    fn check_vault(&self, vault: &Arc<AttributeRuntimeVault>) -> Result<(), Error> {
        if Arc::ptr_eq(&self.vault, vault) {
            Ok(())
        } else {
            Err(Error::ForeignOwner("Global anim_motion_finger_flip Vault"))
        }
    }

    /// Update82cca624..668: the timer clamped by layout +0x140 and +0x148,
    /// then the eight-point graph with X+0x150 and Y+0x170.
    fn blend(&self, timer: f32) -> Result<(f32, f32), Error> {
        let layout = self
            .instance
            .try_cached_layout(0x5c0)
            .map_err(|error| Error::Load(MotionLoadError::AttributeRuntime(error)))?;
        let word = |offset: usize| {
            layout.as_ref().map_or(0.0, |data| {
                f32::from_be_bytes(
                    data.bytes()[offset..offset + 4]
                        .try_into()
                        .expect("validated anim_motion layout"),
                )
            })
        };
        let timer = clamp(timer, word(0x140), word(0x148));
        let graph = PointGraph::<8> {
            x: std::array::from_fn(|i| word(0x150 + i * 4)),
            y: std::array::from_fn(|i| word(0x170 + i * 4)),
        };
        Ok((timer, graph.evaluate(timer)))
    }
}

/// Update82cca5d0..5f0: a held grab intent winds the timer down, otherwise up.
pub(crate) fn wind(timer: f32, held: bool, dt: f32) -> f32 {
    if held { timer - dt } else { dt + timer }
}

/// Update82cca63c..64c: fsel up to `lower`, then fsel down to `upper`. An
/// unordered timer keeps itself first, then takes `upper`.
pub(crate) fn clamp(timer: f32, lower: f32, upper: f32) -> f32 {
    let raised = if lower - timer >= 0.0 { lower } else { timer };
    if upper - raised >= 0.0 { raised } else { upper }
}

impl ActorConstruction {
    /// Begin82cbea20 calls Initialize82cbe948: IMotionGraph (asserted at
    /// MotionGraphBehaviours.cpp:6551), then the timer at instance+8 is zeroed.
    pub(crate) fn motion_finger_flip_out_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        timer: &mut f32,
    ) -> Result<(), Error> {
        self.check_motion_runtime_context(context)?;
        *timer = 0.0;
        Ok(())
    }

    /// Update82cca470: IAnimatable and IMotionGraph (asserted at
    /// MotionGraphBehaviours.cpp:6517), the grab intent's presence in the
    /// Motion intent map (IMotionGraph+0x10), the wound then clamped timer,
    /// and `AddSettableAtt` (IAnimatable+0x48) with normalized false, sequence -1.
    pub(crate) fn motion_finger_flip_out_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        timer: &mut f32,
        grab_intent: &FastString36,
        grab_blend: FastString30,
        attributes: Option<&FingerFlipMotionAttributes>,
        dt: f32,
    ) -> Result<(), Error> {
        self.check_motion_runtime_context(context)?;
        let animation = self.lifecycle_animation(0)?;
        let held = self
            .motion_runtime_intent_value(context, grab_intent)?
            .is_some();
        *timer = wind(*timer, held, dt);
        let attributes =
            attributes.ok_or(Error::MissingOwner("Global anim_motion_finger_flip+164"))?;
        let physical = self.cached_physical_allocation().map_err(Error::Physical)?;
        attributes.check_vault(&physical.owner().inputs().vault)?;
        let (clamped, value) = attributes.blend(*timer)?;
        *timer = clamped;
        animation
            .lock()
            .map_err(|_| Error::Lock)?
            .try_add_gesture_attribute(grab_blend, value, false, -1)
            .map_err(Error::AnimationWrite)
    }
}
