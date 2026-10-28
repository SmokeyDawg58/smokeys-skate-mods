//! Unregistered construct child; one guarded borrow of actual Collision output.
use super::{
    super::{
        PhysOutAccessError, PhysOutComponent, PhysOutWriteGuard, PhysicalPlayerPhysOut, collision,
    },
    ConcretePhysOutError,
};
use crate::{
    skeleton_collision_data::fill_audio::{
        AudioBoneFlag, AudioBoneScalar, AudioBoneVector, AudioFlag, AudioScalar,
        SkeletonAudioOutput, SkeletonAudioRecord,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::convert::Infallible;
pub(crate) struct SkeletonAudioRecordBorrow<'a>(&'a mut collision::SkeletonCollision);
#[derive(Debug)]
pub(crate) enum SkeletonAudioOutputError {
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
}
impl std::fmt::Display for SkeletonAudioOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(e) => write!(f, "output access: {e:?}"),
            Self::Component(e) => write!(f, "output component: {e:?}"),
        }
    }
}
impl SkeletonAudioOutput for PhysicalPlayerPhysOut {
    type Error = SkeletonAudioOutputError;
    type Guard = PhysOutWriteGuard;
    type Record<'a> = SkeletonAudioRecordBorrow<'a>;
    fn sample_guard(&mut self) -> Result<Self::Guard, Self::Error> {
        self.sample_write_guard().map_err(Self::Error::Access)
    }
    fn begin(&mut self, guard: Self::Guard) -> Result<Self::Record<'_>, Self::Error> {
        self.check_write_guard(&guard)
            .map_err(Self::Error::Access)?;
        self.require_complete_component(PhysOutComponent::Collision)
            .map_err(Self::Error::Component)?;
        Ok(SkeletonAudioRecordBorrow(
            &mut self
                .concrete
                .collision
                .as_mut()
                .expect("guarded Collision")
                .skeleton,
        ))
    }
}
impl SkeletonAudioRecord for SkeletonAudioRecordBorrow<'_> {
    fn flag(&mut self, field: AudioFlag, value: u8) {
        let value = value != 0;
        match field {
            AudioFlag::Unrideable => self.0.unrideable_contact = value,
            AudioFlag::PartialRagdoll => self.0.is_partial_ragdoll = value,
            AudioFlag::Impaled => self.0.is_impaled_on_fence = value,
            AudioFlag::Face => self.0.audio.is_face_contact = value,
            AudioFlag::Groin => self.0.audio.is_groin_contact = value,
            AudioFlag::RecentGroin => self.0.has_groin_contacted = value,
            AudioFlag::RecentFace => self.0.has_face_contacted = value,
        }
    }
    fn scalar(&mut self, field: AudioScalar, value: f32) {
        match field {
            AudioScalar::ContactTotal => self.0.contact_force_during_wipeout = value,
            AudioScalar::SlideTotal => self.0.slide_force_during_wipeout = value,
            AudioScalar::TimeWipingOut => self.0.time_wiping_out = value,
            AudioScalar::Vehicle => self.0.audio.vehicle_impact = value,
            AudioScalar::Skater => self.0.audio.skater_impact = value,
            AudioScalar::Pedestrian => self.0.audio.pedestrian_impact = value,
        }
    }
    fn bone_vector(&mut self, part: usize, field: AudioBoneVector, value: SkeletonRootVector) {
        let bone = &mut self.0.bones[part];
        match field {
            AudioBoneVector::Position => bone.position = value,
            AudioBoneVector::Normal => bone.main.normal = value,
            AudioBoneVector::Slide => bone.main.slide = value,
            AudioBoneVector::SubNormal => bone.sub.normal = value,
            AudioBoneVector::SubSlide => bone.sub.slide = value,
        }
    }
    fn bone_scalar(&mut self, part: usize, field: AudioBoneScalar, value: f32) {
        let bone = &mut self.0.bones[part];
        match field {
            AudioBoneScalar::Acceleration => bone.main.acceleration = value,
            AudioBoneScalar::Force => bone.main.force = value,
            AudioBoneScalar::SubAcceleration => bone.sub.acceleration = value,
            AudioBoneScalar::SubForce => bone.sub.force = value,
        }
    }
    fn bone_surface(&mut self, part: usize, sub: bool, value: u32) {
        let bone = &mut self.0.bones[part];
        if sub {
            bone.sub.surface_id = value;
        } else {
            bone.main.surface_id = value;
        }
    }
    fn bone_flag(&mut self, part: usize, field: AudioBoneFlag, value: u8) {
        let bone = &mut self.0.bones[part];
        let value = value != 0;
        match field {
            AudioBoneFlag::Vehicle => bone.hit_vehicle = value,
            AudioBoneFlag::Pedestrian => bone.hit_pedestrian = value,
            AudioBoneFlag::Skater => bone.hit_skater = value,
            AudioBoneFlag::World => bone.hit_world = value,
        }
    }
    fn slide_speed(&mut self, category: usize, value: f32) {
        self.0.audio.slide_speed[category] = value;
    }
    fn contact_force(&mut self, category: usize, value: f32) {
        self.0.audio.contact_force[category] = value;
    }
    fn surface(&mut self, category: usize, value: u32) {
        self.0.audio.audio_surface_type[category] = i32::from_ne_bytes(value.to_ne_bytes());
    }
}
