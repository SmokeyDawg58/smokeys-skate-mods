//! Constructor-owned data projection of `Sk8::Physics::SkeletonCollision`.
//!
//! Ctor 0x82b4c700, `InitSpecificBodyParts` 0x82b3ec78, Reset 0x82b3acc0.
//! This is NOT the native reference-bearing object: the eventual physical owner
//! must retain its actual SkeletonBody/SkeletonState relationships. No pointer,
//! address, placeholder owner token, or native padding bytes are manufactured.
//! Successful construction publishes complete data only; source partial writes
//! and allocation/assert failures are outside that safe-host return boundary.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "collision data awaits physical skeleton ownership"
    )
)]

use crate::{
    ragdoll::{SKELETON_PART_COUNT, SkeletonPartIndex},
    skeleton_collision_attributes::{
        FACE_RADIUS, GROIN_RADIUS, SkeletonCollisionAttributeRead, SkeletonCollisionOffset,
    },
};

#[cfg(test)]
#[path = "skeleton_collision_data_tests.rs"]
mod tests;

#[path = "skeleton_collision_update.rs"]
pub(crate) mod update;

/// Collision XYZ can be recovered even when native packed-pointer W cannot.
/// Keep that uncertainty on the actual state: no zero-fill, stale fourth lane,
/// indexing, or implicit conversion can turn a projection into a full vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkeletonCollisionVector {
    xyz: [f32; 3],
    fourth_bits: Option<u32>,
}

impl SkeletonCollisionVector {
    pub(crate) const ZERO: Self = Self::known([0.0; 4]);

    pub(crate) const fn known(lanes: [f32; 4]) -> Self {
        Self {
            xyz: [lanes[0], lanes[1], lanes[2]],
            fourth_bits: Some(lanes[3].to_bits()),
        }
    }

    pub(crate) const fn xyz_only(xyz: [f32; 3]) -> Self {
        Self {
            xyz,
            fourth_bits: None,
        }
    }

    pub(crate) const fn xyz(&self) -> &[f32; 3] {
        &self.xyz
    }

    /// Full-vector consumers must explicitly handle missing native W. Known
    /// NaNs/infinities/signed zeros are preserved; None is not a numeric value.
    pub(crate) const fn known_lanes(&self) -> Option<[f32; 4]> {
        match self.fourth_bits {
            Some(w) => Some([self.xyz[0], self.xyz[1], self.xyz[2], f32::from_bits(w)]),
            None => None,
        }
    }
}

/// Source32-byte entry: only normal and `PartIndex` are initialized/represented.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkeletonCollisionNormalInfo {
    pub(crate) normal: SkeletonCollisionVector,
    pub(crate) part: SkeletonPartIndex,
}

/// Active fields of native112-byte `BoneData`. Reset writes all these fields in
/// its original order, but never writes the twelve trailing padding bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkeletonCollisionBoneData {
    pub(crate) contact_normal: SkeletonCollisionVector,
    pub(crate) sub_part_contact_normal: SkeletonCollisionVector,
    pub(crate) slide_vector: SkeletonCollisionVector,
    pub(crate) sub_part_slide_vector: SkeletonCollisionVector,
    pub(crate) contact_position: SkeletonCollisionVector,
    pub(crate) contact_force: f32,
    pub(crate) sub_part_contact_force: f32,
    pub(crate) contact_surface_id: u32,
    pub(crate) sub_part_contact_surface_id: u32,
    pub(crate) hit_vehicle: u8,
    pub(crate) hit_pedestrian: u8,
    pub(crate) hit_skater: u8,
    pub(crate) hit_world: u8,
}

impl SkeletonCollisionBoneData {
    const fn constructor_value() -> Self {
        Self {
            contact_normal: SkeletonCollisionVector::ZERO,
            sub_part_contact_normal: SkeletonCollisionVector::ZERO,
            slide_vector: SkeletonCollisionVector::ZERO,
            sub_part_slide_vector: SkeletonCollisionVector::ZERO,
            contact_position: SkeletonCollisionVector::ZERO,
            contact_force: 0.0,
            sub_part_contact_force: 0.0,
            contact_surface_id: 0,
            sub_part_contact_surface_id: 0,
            hit_vehicle: 0,
            hit_pedestrian: 0,
            hit_skater: 0,
            hit_world: 0,
        }
    }

    fn reset(&mut self) {
        self.contact_normal = SkeletonCollisionVector::ZERO;
        self.sub_part_contact_normal = SkeletonCollisionVector::ZERO;
        self.slide_vector = SkeletonCollisionVector::ZERO;
        self.sub_part_slide_vector = SkeletonCollisionVector::ZERO;
        self.contact_position = SkeletonCollisionVector::ZERO;
        self.contact_surface_id = 0;
        self.contact_force = 0.0;
        self.sub_part_contact_surface_id = 0;
        self.sub_part_contact_force = 0.0;
        self.hit_vehicle = 0;
        self.hit_pedestrian = 0;
        self.hit_skater = 0;
        self.hit_world = 0;
    }
}

/// Original two configured body regions. Contact/recent-contact bytes have no
/// stores in ctor/InitSpecificBodyParts/Reset: None means unwritten, not false.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SkeletonSpecificBodyParts {
    pub(crate) parts: [SkeletonPartIndex; 2],
    pub(crate) world_positions: [[f32; 4]; 2],
    /// W in the raw host projection is usable only after a known-lane producer.
    pub(crate) world_position_w_known: [bool; 2],
    pub(crate) offsets: [[f32; 4]; 2],
    pub(crate) radius_squared: [f32; 2],
    pub(crate) contact: [Option<u8>; 2],
    pub(crate) recent_contact: [Option<u8>; 2],
}

impl SkeletonSpecificBodyParts {
    pub(crate) fn world_position_typed(
        &self,
        region: usize,
    ) -> crate::skeleton_root_transforms::SkeletonRootVector {
        let value = self.world_positions[region];
        crate::skeleton_root_transforms::SkeletonRootVector {
            xyz: [value[0], value[1], value[2]],
            w: self.world_position_w_known[region].then_some(value[3]),
        }
    }
    fn try_new<R: SkeletonCollisionAttributeRead>(reader: &mut R) -> Result<Self, R::Error> {
        let parts = [SkeletonPartIndex::Hips, SkeletonPartIndex::Neck1];
        let groin = reader.cached_offset(SkeletonCollisionOffset::Groin)?;
        let face = reader.cached_offset(SkeletonCollisionOffset::Face)?;
        let groin_radius_squared = read_radius_product(reader, GROIN_RADIUS)?;
        let face_radius_squared = read_radius_product(reader, FACE_RADIUS)?;
        Ok(Self {
            parts,
            offsets: [groin, face],
            radius_squared: [groin_radius_squared, face_radius_squared],
            world_positions: [[0.0; 4]; 2],
            world_position_w_known: [false; 2],
            contact: [None; 2],
            recent_contact: [None; 2],
        })
    }
}

fn read_radius_product<R: SkeletonCollisionAttributeRead>(
    reader: &mut R,
    key: u64,
) -> Result<f32, R::Error> {
    let first = reader.lookup(key)?;
    let second = reader.lookup(key)?;
    let second_value = reader.float(&second)?;
    let first_value = reader.float(&first)?;
    Ok(second_value * first_value)
}

/// Named semantic fields rather than a packed-memory clone. Raw bool bytes are
/// retained so Normal setup can copy original noncanonical compliance values.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SkeletonCollisionData {
    pub(crate) normal_info: [SkeletonCollisionNormalInfo; 20],
    pub(crate) time_since_collision: [f32; SKELETON_PART_COUNT],
    pub(crate) priority: [f32; SKELETON_PART_COUNT],
    pub(crate) just_collided: [u8; SKELETON_PART_COUNT],
    pub(crate) is_compliant: [u8; SKELETON_PART_COUNT],
    pub(crate) contact_force: [f32; 8],
    pub(crate) contact_friction_force: [f32; 8],
    pub(crate) slide_speed: [f32; 8],
    pub(crate) audio_surface_type: [u32; 8],
    pub(crate) contact_normal: [SkeletonCollisionVector; 8],
    pub(crate) most_vertical_contact_normal: SkeletonCollisionVector,
    pub(crate) most_vertical_toe_contact_normal: SkeletonCollisionVector,
    pub(crate) contact_bone: [Option<SkeletonPartIndex>; 8],
    pub(crate) bone_data: [SkeletonCollisionBoneData; SKELETON_PART_COUNT],
    pub(crate) specific_parts: SkeletonSpecificBodyParts,
    pub(crate) collision_timer: f32,
    pub(crate) drive_strength: f32,
    pub(crate) collision_priority: f32,
    pub(crate) contact_force_total_during_wipeout: f32,
    pub(crate) slide_force_total_during_wipeout: f32,
    pub(crate) time_in_wipeout: f32,
    pub(crate) skater_contact: f32,
    pub(crate) vehicle_contact: f32,
    pub(crate) pedestrian_contact: f32,
    pub(crate) unrideable_contact: u8,
    pub(crate) non_compliant_just_collided: u8,
    pub(crate) compliant_just_collided: u8,
    pub(crate) compliant_active: u8,
    pub(crate) hit_vehicle: u8,
    pub(crate) hit_world: u8,
    pub(crate) is_wiping_out: u8,
    pub(crate) opposing_contacts: u8,
    pub(crate) opposing_contacts_on_plane: u8,
    pub(crate) is_proxy_in_collision: u8,
    pub(crate) is_hook_to_hips_valid: u8,
    pub(crate) foot_hit_skateboard: u8,
    pub(crate) num_normals: i32,
}

impl SkeletonCollisionData {
    pub(crate) fn try_new<R: SkeletonCollisionAttributeRead>(
        reader: &mut R,
    ) -> Result<Self, R::Error> {
        let normal_info = [SkeletonCollisionNormalInfo {
            normal: SkeletonCollisionVector::ZERO,
            part: SkeletonPartIndex::SkateboardRoot,
        }; 20];
        let bone_data = [SkeletonCollisionBoneData::constructor_value(); SKELETON_PART_COUNT];
        // Native reference stores lie here; embedding physical owner supplies
        // those real relationships. No copied address substitutes are accepted.
        let specific_parts = SkeletonSpecificBodyParts::try_new(reader)?;
        let is_compliant = [0; SKELETON_PART_COUNT];
        let mut data = Self {
            normal_info,
            bone_data,
            specific_parts,
            is_compliant,
            time_since_collision: [0.0; SKELETON_PART_COUNT],
            priority: [0.0; SKELETON_PART_COUNT],
            just_collided: [0; SKELETON_PART_COUNT],
            contact_force: [0.0; 8],
            contact_friction_force: [0.0; 8],
            slide_speed: [0.0; 8],
            audio_surface_type: [0; 8],
            contact_normal: [SkeletonCollisionVector::ZERO; 8],
            most_vertical_contact_normal: SkeletonCollisionVector::ZERO,
            most_vertical_toe_contact_normal: SkeletonCollisionVector::ZERO,
            contact_bone: [None; 8],
            collision_timer: 0.0,
            drive_strength: 0.0,
            collision_priority: 0.0,
            contact_force_total_during_wipeout: 0.0,
            slide_force_total_during_wipeout: 0.0,
            time_in_wipeout: 0.0,
            skater_contact: 0.0,
            vehicle_contact: 0.0,
            pedestrian_contact: 0.0,
            unrideable_contact: 0,
            non_compliant_just_collided: 0,
            compliant_just_collided: 0,
            compliant_active: 0,
            hit_vehicle: 0,
            hit_world: 0,
            is_wiping_out: 0,
            opposing_contacts: 0,
            opposing_contacts_on_plane: 0,
            is_proxy_in_collision: 0,
            is_hook_to_hips_valid: 0,
            foot_hit_skateboard: 0,
            num_normals: 0,
        };
        data.reset();
        Ok(data)
    }

    /// Source selective Reset, 0x82b3acc0..0x82b3ae5c. Does not touch compliance,
    /// `NormalInfo`, `SpecificBodyParts`, or external owning relationships.
    pub(crate) fn reset(&mut self) {
        self.hit_vehicle = 0;
        self.hit_world = 0;
        self.opposing_contacts = 0;
        for part in 0..SKELETON_PART_COUNT {
            self.time_since_collision[part] = 0.0;
            self.priority[part] = 0.0;
            self.just_collided[part] = 0;
            self.bone_data[part].reset();
        }
        for contact in 0..8 {
            self.contact_force[contact] = 0.0;
            self.contact_friction_force[contact] = 0.0;
            self.slide_speed[contact] = 0.0;
            self.contact_normal[contact] = SkeletonCollisionVector::ZERO;
            self.contact_bone[contact] = None; // Native PartIndex -1, not NaN.
            self.audio_surface_type[contact] = 0;
        }
        self.contact_force_total_during_wipeout = 0.0;
        self.slide_force_total_during_wipeout = 0.0;
        self.unrideable_contact = 0;
        self.collision_timer = 0.0;
        self.drive_strength = 0.0;
        self.non_compliant_just_collided = 0;
        self.collision_priority = 0.0;
        self.compliant_just_collided = 0;
        self.time_in_wipeout = 0.0;
        self.compliant_active = 0;
        self.skater_contact = 0.0;
        self.hit_vehicle = 0;
        self.vehicle_contact = 0.0;
        self.hit_world = 0;
        self.pedestrian_contact = 0.0;
        self.is_wiping_out = 0;
        self.opposing_contacts = 0;
        self.opposing_contacts_on_plane = 0;
        self.is_proxy_in_collision = 0;
        self.is_hook_to_hips_valid = 0;
        self.foot_hit_skateboard = 0;
        self.num_normals = 0;
        // Full J vector at0x8227e900; W is original +0, not homogeneous1.
        self.most_vertical_contact_normal = SkeletonCollisionVector::known([0.0, 1.0, 0.0, 0.0]);
        self.most_vertical_toe_contact_normal =
            SkeletonCollisionVector::known([0.0, 1.0, 0.0, 0.0]);
    }

    pub(crate) fn set_compliance(&mut self, part: SkeletonPartIndex, raw_byte: u8) {
        self.is_compliant[part.index()] = raw_byte;
    }
    pub(crate) fn set_priority(&mut self, part: SkeletonPartIndex, value: f32) {
        self.priority[part.index()] = value;
    }
}

#[path = "skeleton_collision_fill_audio.rs"]
pub(crate) mod fill_audio;
