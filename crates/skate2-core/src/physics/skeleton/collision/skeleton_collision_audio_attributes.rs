//! `CollisionUpdateAttributes` child: reuse its retained global+10c.
use super::{SkeletonCollisionUpdateAttributeRead, SkeletonCollisionUpdateAttributes};
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance},
    skeleton_collision_data::fill_audio::SkeletonAudioAttributes,
};
impl SkeletonAudioAttributes for SkeletonCollisionUpdateAttributes {
    type Error = AttributeRuntimeError;
    type Global = AttributeRuntimeInstance;
    type Pointer = Option<AttributeRuntimeData>;
    fn capture(&mut self) -> Result<Self::Global, Self::Error> {
        Ok(self.collision.clone())
    }
    fn lookup(&mut self, global: &Self::Global, key: u64) -> Result<Self::Pointer, Self::Error> {
        global.try_get_data(key, 0)
    }
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        SkeletonCollisionUpdateAttributeRead::float(self, pointer)
    }
}
