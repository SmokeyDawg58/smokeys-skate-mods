//! Handplant animation conditions/callbacks82ca7970,82ca7c30,82cb51d8.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    generated_motion_load::conditions::HandPlantAnimation, play_animation::FastString30,
};
const ANTIC: u64 = 0x673f_3b1b_d056_c39e;
pub(crate) const ANIMATION_CLASS: u64 = 0xf190_522e_738e_c5cb;
fn scalar(a: &AttributeRuntimeInstance, key: u64) -> Result<f32, E> {
    a.try_scalar_word(key, 0)
        .map(|v| f32::from_bits(v.unwrap_or(0)))
        .map_err(E::HandPlantAttribute)
}
impl ActorConstruction {
    fn handplant_animation_attributes(&self) -> Result<AttributeRuntimeInstance, E> {
        let p = self.cached_physical_allocation().map_err(E::Physical)?;
        // GlobalAttributeCollection+178 constructor828b32a0..32b8, full64 keys.
        Ok(p.owner()
            .inputs()
            .vault
            .instance(ANIMATION_CLASS, 0xd7ed_bd36_2d7d_2152))
    }
    fn handplant_animation_time(&self) -> Result<f32, E> {
        let p = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = p.owner().phys_out().lock().map_err(|_| E::Lock)?;
        Ok(output
            .try_air_output()
            .map_err(E::PhysOut)?
            .anim_hand_plant_info_struct
            .estimated_time_to_apex)
    }
    pub(crate) fn motion_should_play_handplant(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        desired: HandPlantAnimation,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        let time = self.handplant_animation_time()?;
        let a = self.handplant_animation_attributes()?;
        let antic = scalar(&a, ANTIC)?;
        let into = scalar(&a, 0x5123_42fb_1d4a_7d57)?;
        let out = scalar(&a, 0xb901_2b86_eb52_58fe)?;
        let time = time - f32::from_bits(0x3c88_8889);
        Ok(match desired {
            HandPlantAnimation::Antic => !(time >= antic),
            HandPlantAnimation::Into => time < into,
            HandPlantAnimation::Out => !(time >= -out),
        })
    }
    pub(crate) fn motion_new_handplant_position(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        let p = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = p.owner().phys_out().lock().map_err(|_| E::Lock)?;
        Ok(output
            .try_air_output()
            .map_err(E::PhysOut)?
            .anim_hand_plant_info_struct
            .flags
            & 0x4000_0000
            != 0)
    }
    pub(crate) fn motion_handplant_antic_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        name: FastString30,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let animation = self.lifecycle_animation(0)?;
        let time = self.handplant_animation_time()?;
        let lengths = self
            .handplant_shared
            .lock()
            .map_err(|_| E::Lock)?
            .clone()
            .ok_or(E::MissingOwner("SkaterAnimShared HandPlantAnimInfo"))?;
        let threshold = scalar(&self.handplant_animation_attributes()?, ANTIC)?;
        if !(lengths[1] > lengths[0]) {
            return Err(E::MissingOwner(
                "HandPlant antic must exceed late antic length",
            ));
        }
        let length = lengths[1] - (threshold - time);
        let fraction = (length - lengths[0]) / (lengths[1] - lengths[0]);
        let lower = if -fraction >= 0. { 0. } else { fraction };
        let value = if 1. - lower >= 0. { lower } else { 1. };
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(name, value, false, -1)
            .map_err(E::AnimationWrite)
    }
}
/// SkaterAnimShared.InitHandPlantAnimInfo82cb70e8: same playback and temporary
/// 0x100-byte/0x100-element pool, late/regular/into measurement order.
pub(crate) fn measure(
    playback: &crate::actor_face_playback::FacePlaybackData,
) -> Result<[f32; 3], crate::actor_animation_tree::AnimationTreeError> {
    use crate::{
        actor_animation_tree::{AnimationTreeError, AnimationTreePool},
        play_animation::FastString36,
    };
    let mut pool = AnimationTreePool::new(0x100, 0x100);
    let mut lengths = [0.; 3];
    for (index, name) in [
        "INVERT_HANDPLANT_FS_0_ANTIC_LATE",
        "INVERT_HANDPLANT_FS_0_ANTIC",
        "INVERT_HANDPLANT_FS_0_INTO",
    ]
    .into_iter()
    .enumerate()
    {
        let name = FastString36::encode(name).expect("native handplant name");
        let tree = playback
            .try_construct_animation_tree(&mut pool, &name, &[])?
            .ok_or(AnimationTreeError::MissingRecord)?;
        lengths[index] = pool.length(&tree)?;
        pool.clear();
    }
    Ok(lengths)
}
