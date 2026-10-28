//! Storage projection of `SkateboardBody::CollisionInfo`, with explicit
//! source Reset (0x82b23630) rather than inferred constructor scheduling.
//! This contains no contact traversal, vote, body reads or frame publication.

use crate::skeleton_root_transforms::SkeletonRootVector;

#[cfg(test)]
#[path = "skateboard_contact_state_tests.rs"]
mod tests;

pub(crate) const UP: SkeletonRootVector = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
const ZERO: SkeletonRootVector = SkeletonRootVector::known([0.0; 4]);

/// Unknown bits remain unknown after a native read/modify/write. `value` has
/// meaning only under `known`; an uninitialized word is not a known zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SkateboardContactFlags {
    value: u32,
    known: u32,
}

impl SkateboardContactFlags {
    pub(crate) const fn uninitialized() -> Self {
        Self { value: 0, known: 0 }
    }

    pub(crate) const fn known(value: u32) -> Self {
        Self {
            value,
            known: u32::MAX,
        }
    }

    pub(crate) const fn known_mask(self) -> u32 {
        self.known
    }

    pub(crate) const fn read(self, mask: u32) -> Option<u32> {
        if self.known & mask == mask {
            Some(self.value & mask)
        } else {
            None
        }
    }

    pub(crate) fn and_mask(&mut self, mask: u32) {
        self.value &= mask;
        self.known |= !mask;
    }

    pub(crate) fn or_mask(&mut self, mask: u32) {
        self.value |= mask;
        self.known |= mask;
    }
}

/// Body+2f0: four validity bytes immediately followed by seven BE physics
/// words. Native seven-part validity access deliberately aliases word0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SkateboardContactSurfaceBytes {
    bytes: [Option<u8>; 32],
}

impl SkateboardContactSurfaceBytes {
    const fn uninitialized() -> Self {
        Self { bytes: [None; 32] }
    }

    pub(crate) fn validity(&self, part: usize) -> Option<u8> {
        assert!(part < 7, "native board has seven Parts");
        self.bytes[part]
    }

    pub(crate) fn set_validity(&mut self, part: usize, value: u8) {
        assert!(part < 7, "native board has seven Parts");
        self.bytes[part] = Some(value);
    }

    pub(crate) fn physics_type(&self, part: usize) -> Option<u32> {
        assert!(part < 7, "native board has seven Parts");
        let offset = 4 + part * 4;
        Some(u32::from_be_bytes([
            self.bytes[offset]?,
            self.bytes[offset + 1]?,
            self.bytes[offset + 2]?,
            self.bytes[offset + 3]?,
        ]))
    }

    pub(crate) fn set_physics_type(&mut self, part: usize, value: u32) {
        assert!(part < 7, "native board has seven Parts");
        let offset = 4 + part * 4;
        for (target, byte) in self.bytes[offset..offset + 4]
            .iter_mut()
            .zip(value.to_be_bytes())
        {
            *target = Some(byte);
        }
    }
}

pub(crate) struct SkateboardContactState {
    pub(crate) opposing_delta: Option<f32>,
    reset_completed: bool,
    pub(crate) average_normal: Option<SkeletonRootVector>,
    pub(crate) last_valid_normal: Option<SkeletonRootVector>,
    pub(crate) normal: [Option<SkeletonRootVector>; 7],
    pub(crate) position: [Option<SkeletonRootVector>; 7],
    pub(crate) velocity: [Option<SkeletonRootVector>; 7],
    pub(crate) acceleration: [Option<SkeletonRootVector>; 7],
    /// Body+220; Reset never writes this array.
    pub(crate) relative_velocity: [Option<SkeletonRootVector>; 7],
    pub(crate) in_contact: [Option<bool>; 7],
    pub(crate) surfaces: SkateboardContactSurfaceBytes,
    pub(crate) audio_type: [Option<u32>; 7],
    pub(crate) pattern_type: [Option<u32>; 7],
    pub(crate) flags: SkateboardContactFlags,
}

impl SkateboardContactState {
    pub(crate) const fn uninitialized() -> Self {
        Self {
            reset_completed: false,
            opposing_delta: None,
            average_normal: None,
            last_valid_normal: None,
            normal: [None; 7],
            position: [None; 7],
            velocity: [None; 7],
            acceleration: [None; 7],
            relative_velocity: [None; 7],
            in_contact: [None; 7],
            surfaces: SkateboardContactSurfaceBytes::uninitialized(),
            audio_type: [None; 7],
            pattern_type: [None; 7],
            flags: SkateboardContactFlags::uninitialized(),
        }
    }

    pub(crate) const fn reset_completed(&self) -> bool {
        self.reset_completed
    }

    /// Host completeness marker for a new in-place native Reset. A prior
    /// completed reset must not certify this one before its four-wheel tail.
    pub(crate) fn begin_in_place_reset(&mut self) {
        self.reset_completed = false;
    }

    pub(crate) fn complete_in_place_reset(&mut self) {
        self.reset_completed = true;
    }

    /// Native 0x82b236f0..23708, before shared wheel height is cleared.
    pub(crate) fn reset_part_before_height(&mut self, part: usize) {
        self.in_contact[part] = Some(false);
        self.normal[part] = Some(UP);
        self.position[part] = Some(ZERO);
        self.velocity[part] = Some(ZERO);
        self.acceleration[part] = Some(ZERO);
    }

    /// Native 0x82b23714..23720, after shared wheel height is cleared.
    pub(crate) fn reset_part_after_height(&mut self, part: usize) {
        self.opposing_delta = Some(0.0);
        self.audio_type[part] = Some(0);
        self.surfaces.set_physics_type(part, 0);
        self.pattern_type[part] = Some(0);
    }

    /// Applies Reset's writes to represented fields in native order. The
    /// enclosing actual body owns scheduling. Reset's max-contact scratch,
    /// line results, counters and booleans outside this projection are not
    /// certified by this method; neither is a completed current contact frame.
    pub(crate) fn reset(&mut self) {
        self.begin_in_place_reset();
        self.average_normal = Some(UP); // 236a4
        self.last_valid_normal = Some(UP); // 236ac
        self.flags.and_mask(0x17ff_ffff); // 236b4..236cc
        for part in 0..7 {
            self.reset_part_before_height(part);
            self.reset_part_after_height(part);
        }
        for wheel in 0..4 {
            self.surfaces.set_validity(wheel, 0); // 2374c
        }
        self.complete_in_place_reset();
    }
}
