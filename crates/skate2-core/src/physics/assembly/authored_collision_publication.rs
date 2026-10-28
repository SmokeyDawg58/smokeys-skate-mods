//! Closed read view of actual authored Assembly collision publications.
//!
//! All producers retain their source-owned Create/Update records and real
//! occurrence-qualified body bindings. This adapter creates no rows, snapshots,
//! arena identities or scheduling policy: the existing CPU job (`0x827cdb98`)
//! and Run (`0x827cd1e0`) still perform their original common validation and
//! traversal. The serialized arena/image route remains a separate alternative.

use crate::{
    dynamic_assembly_session::{
        character_publication::CharacterCollisionPublication,
        skateboard_publication::SkateboardCollisionPublication,
        skeleton_collision_publication::SkeletonCollisionPublication,
    },
    octree::AabbWords,
    serialised_assembly_template::SerialisedAssemblyVolumeDataState,
    simulation_rigid_body_pool::{
        SimulationRigidBodyFrameIdentity, SimulationRigidBodyObservation,
        SimulationRigidBodyPoolAuthority,
    },
};
use skate2_formats::collision::BitTable;

/// Common semantic-XYZ projection used by programmatically constructed Parts.
/// Canonical pose W / zero motion-link W is the existing authored-body host
/// boundary, not manufactured Xenon pointer words. Source Part.GetTransform
/// 826c3480 and CalculatePartBBox82e0b760 consume these projections.
pub(crate) fn semantic_part_transform(
    snapshot: &crate::simulation_rigid_body_pool::SimulationRigidBodySerialisedAssemblySnapshot,
    metadata: &crate::dynamic_assembly_session::DynamicAssemblyPartAttachmentMetadata,
) -> [f32; 16] {
    let body_transform = [
        snapshot.right[0],
        snapshot.right[1],
        snapshot.right[2],
        0.,
        snapshot.up[0],
        snapshot.up[1],
        snapshot.up[2],
        0.,
        snapshot.at[0],
        snapshot.at[1],
        snapshot.at[2],
        0.,
        snapshot.center_of_mass[0],
        snapshot.center_of_mass[1],
        snapshot.center_of_mass[2],
        1.,
    ];
    if metadata.has_inverse_body_transform == 0 {
        body_transform
    } else {
        crate::skateboard_spawn_math::skateboard_spawn_compose(
            &body_transform,
            &metadata.inverse_body_transform,
        )
    }
}

pub(crate) fn semantic_motion(
    velocity: [f32; 3],
    omega: [f32; 3],
    force: [f32; 3],
    kine: f32,
    torque: [f32; 3],
    cooldown: u32,
) -> crate::serialised_assembly_bbox::SerialisedAssemblyBodyMotion {
    crate::serialised_assembly_bbox::SerialisedAssemblyBodyMotion {
        velocity_and_right: [velocity[0], velocity[1], velocity[2], 0.],
        omega_and_left: [omega[0], omega[1], omega[2], 0.],
        force_and_kine: [force[0], force[1], force[2], kine],
        torque_and_cool: [torque[0], torque[1], torque[2], f32::from_bits(cooldown)],
    }
}

/// Only a complete, retained producer can enter the authored ground path.
/// Copying this borrowed view neither duplicates its owner nor extends the
/// attached body's lifetime. Frame/authority/occurrence checks remain required.
#[derive(Clone, Copy)]
pub(crate) enum AuthoredCollisionPublication<'a> {
    Board(&'a SkateboardCollisionPublication),
    Skeleton(&'a SkeletonCollisionPublication),
    Character(&'a CharacterCollisionPublication),
}

impl<'a> From<&'a SkateboardCollisionPublication> for AuthoredCollisionPublication<'a> {
    fn from(value: &'a SkateboardCollisionPublication) -> Self {
        Self::Board(value)
    }
}

impl<'a> From<&'a SkeletonCollisionPublication> for AuthoredCollisionPublication<'a> {
    fn from(value: &'a SkeletonCollisionPublication) -> Self {
        Self::Skeleton(value)
    }
}

impl<'a> From<&'a CharacterCollisionPublication> for AuthoredCollisionPublication<'a> {
    fn from(value: &'a CharacterCollisionPublication) -> Self {
        Self::Character(value)
    }
}

impl<'a> AuthoredCollisionPublication<'a> {
    pub(crate) fn records(self) -> &'a [SerialisedAssemblyVolumeDataState] {
        match self {
            Self::Board(value) => value.records(),
            Self::Skeleton(value) => value.records(),
            Self::Character(value) => value.records(),
        }
    }

    pub(crate) fn rigid_bodies(self) -> &'a [SimulationRigidBodyObservation] {
        match self {
            Self::Board(value) => value.rigid_bodies(),
            Self::Skeleton(value) => value.rigid_bodies(),
            Self::Character(value) => value.rigid_bodies(),
        }
    }

    pub(crate) fn bounds(self) -> AabbWords {
        match self {
            Self::Board(value) => value.bounds(),
            Self::Skeleton(value) => value.bounds(),
            Self::Character(value) => value.bounds(),
        }
    }

    pub(crate) fn num_static(self) -> u32 {
        match self {
            Self::Board(value) => value.num_static(),
            Self::Skeleton(value) => value.num_static(),
            Self::Character(value) => value.num_static(),
        }
    }

    pub(crate) fn all_parts_frozen(self) -> bool {
        match self {
            Self::Board(value) => value.all_parts_frozen(),
            Self::Skeleton(value) => value.all_parts_frozen(),
            Self::Character(value) => value.all_parts_frozen(),
        }
    }

    pub(crate) fn bend_threshold(self) -> f32 {
        match self {
            Self::Board(value) => value.bend_threshold(),
            Self::Skeleton(value) => value.bend_threshold(),
            Self::Character(value) => value.bend_threshold(),
        }
    }

    pub(crate) fn body_pool_authority(self) -> SimulationRigidBodyPoolAuthority {
        match self {
            Self::Board(value) => value.body_pool_authority(),
            Self::Skeleton(value) => value.body_pool_authority(),
            Self::Character(value) => value.body_pool_authority(),
        }
    }

    pub(crate) fn has_bounds_for_frame(self, frame: SimulationRigidBodyFrameIdentity) -> bool {
        match self {
            Self::Board(value) => value.has_bounds_for_frame(frame),
            Self::Skeleton(value) => value.has_bounds_for_frame(frame),
            Self::Character(value) => value.has_bounds_for_frame(frame),
        }
    }

    /// Header `group` (+0xc), read by the skater DMO pipeline's pair culling.
    pub(crate) fn group(self) -> u32 {
        match self {
            Self::Board(value) => value.group(),
            Self::Skeleton(value) => value.group(),
            Self::Character(value) => value.group(),
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "self-collision table is distinct from ground pipeline culling"
        )
    )]
    pub(crate) fn culling_table(self) -> &'a BitTable {
        match self {
            Self::Board(value) => value.culling_table(),
            Self::Skeleton(value) => value.culling_table(),
            Self::Character(value) => value.culling_table(),
        }
    }
}
