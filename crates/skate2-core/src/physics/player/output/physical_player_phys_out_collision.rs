//! Collision constructor82e2c690/reset82e24e50 and containing output82e24f20.
use super::simple::CollisionAudio;
use super::{SkeletonRootVector, UP, ZERO};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PartContact {
    pub(super) normal: SkeletonRootVector,
    pub(super) slide: SkeletonRootVector,
    pub(super) acceleration: f32,
    pub(super) force: f32,
    pub(super) surface_id: u32,
}
impl PartContact {
    fn reset_values() -> Self {
        Self {
            normal: ZERO,
            slide: ZERO,
            acceleration: 0.0,
            force: 0.0,
            surface_id: 0,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
pub(super) struct BoneContact {
    pub(super) position: SkeletonRootVector,
    pub(super) main: PartContact,
    pub(super) sub: PartContact,
    pub(super) hit_vehicle: bool,
    pub(super) hit_pedestrian: bool,
    pub(super) hit_skater: bool,
    pub(super) hit_world: bool,
}
impl BoneContact {
    fn reset_values() -> Self {
        Self {
            position: ZERO,
            main: PartContact::reset_values(),
            sub: PartContact::reset_values(),
            hit_vehicle: false,
            hit_pedestrian: false,
            hit_skater: false,
            hit_world: false,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
pub(super) struct SkeletonCollision {
    pub(super) audio: CollisionAudio,
    pub(super) contact_force_during_wipeout: f32,
    pub(super) slide_force_during_wipeout: f32,
    pub(super) time_wiping_out: f32,
    pub(super) has_groin_contacted: bool,
    pub(super) has_face_contacted: bool,
    pub(super) unrideable_contact: bool,
    pub(super) is_partial_ragdoll: bool,
    pub(super) is_impaled_on_fence: bool,
    pub(super) bones: [BoneContact; 24],
}
impl SkeletonCollision {
    pub(super) fn reset_values() -> Self {
        Self {
            audio: CollisionAudio {
                contact_force: [0.0; 8],
                slide_speed: [0.0; 8],
                audio_surface_type: [0; 8],
                is_groin_contact: false,
                is_face_contact: false,
                vehicle_impact: 0.0,
                skater_impact: 0.0,
                pedestrian_impact: 0.0,
            },
            contact_force_during_wipeout: 0.0,
            slide_force_during_wipeout: 0.0,
            time_wiping_out: 0.0,
            has_groin_contacted: false,
            has_face_contacted: false,
            unrideable_contact: false,
            is_partial_ragdoll: false,
            is_impaled_on_fence: false,
            bones: std::array::from_fn(|_| BoneContact::reset_values()),
        }
    }
}

/// `PhysicsSurfaceType` is excluded: the existing owner already retains that field.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source output flags retain their native meanings"
)]
pub(super) struct CollisionRemainder {
    pub(super) wheels_in_contact: i32,
    pub(super) front_truck_audio_surface: i32,
    pub(super) back_truck_audio_surface: i32,
    pub(super) deck_audio_surface: i32,
    pub(super) deck_slide_speed: f32,
    pub(super) deck_contact_force: f32,
    pub(super) average_ground_normal: SkeletonRootVector,
    pub(super) skeleton: SkeletonCollision,
    pub(super) wheel_in_contact: [bool; 4],
    pub(super) wheel_contact_position: [SkeletonRootVector; 4],
    pub(super) wheel_contact_normal: [SkeletonRootVector; 4],
    pub(super) wheel_audio_surface: [i32; 4],
    pub(super) wheel_surface_pattern: [i32; 4],
    pub(super) a_truck_in_contact: bool,
    pub(super) front_truck_in_contact: bool,
    pub(super) back_truck_in_contact: bool,
    pub(super) deck_in_contact: bool,
    pub(super) anything_in_contact: bool,
    pub(super) skeleton_in_contact: bool,
}
impl CollisionRemainder {
    pub(super) fn reset_values() -> Self {
        Self {
            wheels_in_contact: 0,
            front_truck_audio_surface: 0,
            back_truck_audio_surface: 0,
            deck_audio_surface: 0,
            deck_slide_speed: 0.0,
            deck_contact_force: 0.0,
            average_ground_normal: UP,
            skeleton: SkeletonCollision::reset_values(),
            wheel_in_contact: [false; 4],
            wheel_contact_position: [ZERO; 4],
            wheel_contact_normal: [UP; 4],
            wheel_audio_surface: [0; 4],
            wheel_surface_pattern: [0; 4],
            a_truck_in_contact: false,
            front_truck_in_contact: false,
            back_truck_in_contact: false,
            deck_in_contact: false,
            anything_in_contact: false,
            skeleton_in_contact: false,
        }
    }
}
