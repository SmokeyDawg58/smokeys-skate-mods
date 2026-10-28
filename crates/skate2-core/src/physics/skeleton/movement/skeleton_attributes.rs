//! Original `SkeletonBody::InitAttribData` (0x82b3c3b8) inputs.
//!
//! Produces an immutable construction snapshot, not in-place body reload or
//! mutable Vault notification behavior. Errors do not publish a partial body.
//! Record padding and raw boolean bytes survive layout copies. Root drive
//! padding is explicitly unknown after the native constructor, not zeroed.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "physical skeleton attributes await actual body construction"
    )
)]

use crate::attribute_runtime::{
    AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault,
};
use std::{fmt, sync::Arc};

#[cfg(test)]
#[path = "skeleton_attributes_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkeletonAttributeClass {
    Skeleton,
    Bones,
    Drives,
    Joints,
}

impl SkeletonAttributeClass {
    #[must_use]
    pub const fn key(self) -> u64 {
        match self {
            Self::Skeleton => 0x9f86_2c2e_aa01_6ce5,
            Self::Bones => 0xf60d_fc08_2242_b489,
            Self::Drives => 0xfdf6_1860_0d39_6c3b,
            Self::Joints => 0x9ceb_4dce_6a21_3c4e,
        }
    }

    const fn layout_size(self) -> usize {
        match self {
            Self::Skeleton => 112,
            Self::Bones => 576,
            Self::Drives => 736,
            Self::Joints => 440,
        }
    }
}

/// Lookup and float dereference are separate: the source retains both friction
/// handles before loading either. Layout handles capture one immutable range.
pub trait SkeletonAttributeRead {
    type Pointer;
    type Layout: AsRef<[u8]>;
    type Error;

    /// # Errors
    /// Returns the provider's unsupported-storage or lookup failure.
    fn lookup(
        &mut self,
        class: SkeletonAttributeClass,
        key: u64,
    ) -> Result<Self::Pointer, Self::Error>;
    /// # Errors
    /// Returns the provider's invalid scalar-range or size failure.
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
    /// # Errors
    /// Returns the provider's cached-layout resolution failure.
    fn layout(&mut self, class: SkeletonAttributeClass) -> Result<Self::Layout, Self::Error>;

    /// # Errors
    /// Propagates lookup or delayed scalar-load failure.
    fn read_float(&mut self, class: SkeletonAttributeClass, key: u64) -> Result<f32, Self::Error> {
        let pointer = self.lookup(class, key)?;
        self.float(&pointer)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkeletonMaterial {
    pub static_friction: f32,
    pub dynamic_friction: f32,
    pub restitution: f32,
}

/// PDB `SkeletonBoneData` (24 bytes); booleans retain the original byte.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkeletonBoneAttributes {
    pub mass_factor: f32,
    pub ragdoll_mass_factor: f32,
    pub has_collision: u8,
    pub use_root_drive: u8,
    pub padding: [u8; 2],
    pub volume_type: u32,
    pub volume_scalar: f32,
    pub num_parents: u32,
}

impl SkeletonBoneAttributes {
    fn from_record(bytes: &[u8]) -> Self {
        Self {
            mass_factor: float_at(bytes, 0),
            ragdoll_mass_factor: float_at(bytes, 4),
            has_collision: bytes[8],
            use_root_drive: bytes[9],
            padding: [bytes[10], bytes[11]],
            volume_type: word_at(bytes, 12),
            volume_scalar: float_at(bytes, 16),
            num_parents: word_at(bytes, 20),
        }
    }
}

/// PDB `SkeletonDriveData` (32 bytes). `None` padding means unwritten native
/// storage, as in the root drive left untouched by `InitAttribData`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkeletonDriveAttributes {
    pub ragdoll_strength_start: f32,
    pub ragdoll_strength: f32,
    pub ragdoll_strength_controlled: f32,
    pub ragdoll_strength_end: f32,
    pub collision_strength: f32,
    pub collision_strength_root: f32,
    pub collision_compliant: u8,
    pub padding: Option<[u8; 3]>,
    pub collision_priority: f32,
}

impl SkeletonDriveAttributes {
    /// Actual constructor stores at 0x82b623cc..0x82b623ec; padding is not written.
    #[must_use]
    pub const fn constructor_value() -> Self {
        Self {
            ragdoll_strength_start: 0.0,
            ragdoll_strength: 0.0,
            ragdoll_strength_controlled: 0.0,
            ragdoll_strength_end: 0.0,
            collision_strength: 0.0,
            collision_strength_root: 0.0,
            collision_compliant: 0,
            padding: None,
            collision_priority: 0.0,
        }
    }

    fn from_record(bytes: &[u8]) -> Self {
        Self {
            ragdoll_strength_start: float_at(bytes, 0),
            ragdoll_strength: float_at(bytes, 4),
            ragdoll_strength_controlled: float_at(bytes, 8),
            ragdoll_strength_end: float_at(bytes, 12),
            collision_strength: float_at(bytes, 16),
            collision_strength_root: float_at(bytes, 20),
            collision_compliant: bytes[24],
            padding: Some([bytes[25], bytes[26], bytes[27]]),
            collision_priority: float_at(bytes, 28),
        }
    }

    fn scale(&mut self, ragdoll: f32, collision: f32) {
        // 0x82b3c9b0..0x82b3c9ec: root collision evaluates first; stores are
        // start/ordinary/controlled/end/collision/root. No clamp or fused ops.
        let root = self.collision_strength_root * collision;
        let start = ragdoll * self.ragdoll_strength_start;
        let ordinary = ragdoll * self.ragdoll_strength;
        let controlled = ragdoll * self.ragdoll_strength_controlled;
        let end = ragdoll * self.ragdoll_strength_end;
        let collision = collision * self.collision_strength;
        self.ragdoll_strength_start = start;
        self.ragdoll_strength = ordinary;
        self.ragdoll_strength_controlled = controlled;
        self.ragdoll_strength_end = end;
        self.collision_strength = collision;
        self.collision_strength_root = root;
    }
}

/// PDB `SkeletonJointData` (20 bytes), including raw boolean/padding bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkeletonJointAttributes {
    pub ball_joint: u8,
    pub padding: [u8; 3],
    pub swing_angle: f32,
    pub twist_angle: f32,
    pub swing_ragdoll: f32,
    pub twist_ragdoll: f32,
}

impl SkeletonJointAttributes {
    fn from_record(bytes: &[u8]) -> Self {
        Self {
            ball_joint: bytes[0],
            padding: [bytes[1], bytes[2], bytes[3]],
            swing_angle: float_at(bytes, 4),
            twist_angle: float_at(bytes, 8),
            swing_ragdoll: float_at(bytes, 12),
            twist_ragdoll: float_at(bytes, 16),
        }
    }
}

#[derive(Debug)]
pub enum SkeletonAttributeLoadError<E> {
    Read(E),
    LayoutSize {
        class: SkeletonAttributeClass,
        expected: usize,
        actual: usize,
    },
}

impl<E: fmt::Display> fmt::Display for SkeletonAttributeLoadError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(f, "skeleton attribute read: {error}"),
            Self::LayoutSize {
                class,
                expected,
                actual,
            } => write!(
                f,
                "{class:?} layout requires {expected} bytes, found {actual}"
            ),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for SkeletonAttributeLoadError<E> {}

/// Completed `InitAttribData` snapshot. The separate Zero material is not
/// modified by this function and is intentionally not manufactured here.
#[derive(Clone, Debug, PartialEq)]
pub struct SkeletonBodyAttributeData {
    pub normal: SkeletonMaterial,
    pub ragdoll: SkeletonMaterial,
    pub ragdoll_head: SkeletonMaterial,
    pub drives: [SkeletonDriveAttributes; 24],
    pub bones: [SkeletonBoneAttributes; 24],
    pub joints: [SkeletonJointAttributes; 22],
}

impl SkeletonBodyAttributeData {
    /// Source 0x82b3c3b8..0x82b3d0a8. The supplied root drive survives unchanged;
    /// initial construction supplies `SkeletonDriveAttributes::constructor_value`.
    ///
    /// # Errors
    /// Propagates source-reader failures or rejects a malformed layout size.
    /// No partial snapshot is returned; native intermediate body stores are
    /// outside this immutable construction boundary.
    pub fn try_read<R: SkeletonAttributeRead>(
        reader: &mut R,
        root_drive: SkeletonDriveAttributes,
    ) -> Result<Self, SkeletonAttributeLoadError<R::Error>> {
        use SkeletonAttributeClass::{Bones, Drives, Joints};
        use SkeletonAttributeLoadError::Read;
        let normal = read_material(reader, 0x8a43_3894_d234_21e6, 0xfd1e_0898_d6f2_f5b8, false)
            .map_err(Read)?;
        let ragdoll = read_material(reader, 0x5e3e_f6cc_8922_8e59, 0xb8d8_ecea_a7b3_b603, false)
            .map_err(Read)?;
        let ragdoll_head =
            read_material(reader, 0xbac7_c1be_3809_0b31, 0xb086_b9c7_da14_b441, true)
                .map_err(Read)?;

        let drive_layout = reader.layout(Drives).map_err(Read)?;
        let drive_bytes = checked_layout::<R::Error>(Drives, drive_layout.as_ref())?;
        let mut drives = [root_drive; 24];
        for (part, record) in DRIVE_RECORDS.into_iter().enumerate() {
            drives[part + 1] =
                SkeletonDriveAttributes::from_record(&drive_bytes[record * 32..][..32]);
        }
        let ragdoll_scale = reader
            .read_float(Drives, 0x851d_fc1d_6128_976f)
            .map_err(Read)?;
        let collision_scale = reader
            .read_float(Drives, 0x057e_ddfa_341f_9492)
            .map_err(Read)?;
        for drive in &mut drives[1..] {
            drive.scale(ragdoll_scale, collision_scale);
        }

        let bone_layout = reader.layout(Bones).map_err(Read)?;
        let bone_bytes = checked_layout::<R::Error>(Bones, bone_layout.as_ref())?;
        let bones = BONE_RECORDS
            .map(|record| SkeletonBoneAttributes::from_record(&bone_bytes[record * 24..][..24]));
        let joint_layout = reader.layout(Joints).map_err(Read)?;
        let joint_bytes = checked_layout::<R::Error>(Joints, joint_layout.as_ref())?;
        let joints = JOINT_RECORDS
            .map(|record| SkeletonJointAttributes::from_record(&joint_bytes[record * 20..][..20]));
        Ok(Self {
            normal,
            ragdoll,
            ragdoll_head,
            drives,
            bones,
            joints,
        })
    }
}

// Fixed generated-layout field order -> original Part/joint order, independently
// checked against PDB member names and every unrolled InitAttribData copy.
const DRIVE_RECORDS: [usize; 23] = [
    12, 13, 18, 19, 21, 16, 8, 9, 11, 6, 0, 1, 2, 3, 15, 20, 17, 14, 5, 10, 7, 4, 22,
];
const BONE_RECORDS: [usize; 24] = [
    4, 13, 14, 19, 20, 22, 17, 9, 10, 12, 7, 0, 1, 2, 3, 16, 21, 18, 15, 6, 11, 8, 5, 23,
];
const JOINT_RECORDS: [usize; 22] = [
    12, 1, 16, 18, 14, 2, 9, 11, 7, 0, 3, 4, 5, 19, 17, 15, 13, 21, 10, 8, 6, 20,
];

fn read_material<R: SkeletonAttributeRead>(
    reader: &mut R,
    restitution_key: u64,
    friction_key: u64,
    first_load_first: bool,
) -> Result<SkeletonMaterial, R::Error> {
    let class = SkeletonAttributeClass::Skeleton;
    let restitution = reader.read_float(class, restitution_key)?;
    let first = reader.lookup(class, friction_key)?;
    let second = reader.lookup(class, friction_key)?;
    let (static_friction, dynamic_friction) = if first_load_first {
        let dynamic_friction = reader.float(&first)?;
        (reader.float(&second)?, dynamic_friction)
    } else {
        let static_friction = reader.float(&second)?;
        (static_friction, reader.float(&first)?)
    };
    Ok(SkeletonMaterial {
        static_friction,
        dynamic_friction,
        restitution,
    })
}

fn checked_layout<E>(
    class: SkeletonAttributeClass,
    bytes: &[u8],
) -> Result<&[u8], SkeletonAttributeLoadError<E>> {
    let expected = class.layout_size();
    if bytes.len() != expected {
        return Err(SkeletonAttributeLoadError::LayoutSize {
            class,
            expected,
            actual: bytes.len(),
        });
    }
    Ok(bytes)
}
fn word_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("checked record"),
    )
}
fn float_at(bytes: &[u8], offset: usize) -> f32 {
    f32::from_bits(word_at(bytes, offset))
}

/// Retained immutable generated-class bindings, in original ctor order. No
/// message port, live Change/reload or editor notification implementation.
pub(crate) struct SkeletonAttributes {
    instances: [AttributeRuntimeInstance; 4],
}

impl SkeletonAttributes {
    pub(crate) const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;

    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>, collection: u64) -> Self {
        Self {
            instances: [
                SkeletonAttributeClass::Skeleton,
                SkeletonAttributeClass::Bones,
                SkeletonAttributeClass::Drives,
                SkeletonAttributeClass::Joints,
            ]
            .map(|class| vault.instance(class.key(), collection)),
        }
    }

    fn instance(&self, class: SkeletonAttributeClass) -> &AttributeRuntimeInstance {
        &self.instances[match class {
            SkeletonAttributeClass::Skeleton => 0,
            SkeletonAttributeClass::Bones => 1,
            SkeletonAttributeClass::Drives => 2,
            SkeletonAttributeClass::Joints => 3,
        }]
    }
}

pub(crate) struct SkeletonAttributeLayout {
    retained: Option<AttributeRuntimeData>,
    size: usize,
}

impl AsRef<[u8]> for SkeletonAttributeLayout {
    fn as_ref(&self) -> &[u8] {
        // Generated ctors 828b10e0/82b3e5b8/828b0bb8/82b3e630 select
        // DefaultDataArea(112/576/736/440); source first736bytes are zero.
        self.retained
            .as_ref()
            .map_or(&[0; 736][..self.size], AttributeRuntimeData::bytes)
    }
}

impl SkeletonAttributeRead for SkeletonAttributes {
    type Pointer = Option<AttributeRuntimeData>;
    type Layout = SkeletonAttributeLayout;
    type Error = AttributeRuntimeError;

    fn lookup(
        &mut self,
        class: SkeletonAttributeClass,
        key: u64,
    ) -> Result<Self::Pointer, Self::Error> {
        self.instance(class).try_get_data(key, 0)
    }
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        let Some(data) = pointer else {
            return Ok(0.0);
        };
        let bytes: [u8; 4] =
            data.bytes()
                .try_into()
                .map_err(|_| AttributeRuntimeError::ScalarSize {
                    size: data.bytes().len(),
                })?;
        Ok(f32::from_be_bytes(bytes))
    }
    fn layout(&mut self, class: SkeletonAttributeClass) -> Result<Self::Layout, Self::Error> {
        let size = class.layout_size();
        Ok(SkeletonAttributeLayout {
            retained: self.instance(class).try_cached_layout(size)?,
            size,
        })
    }
}
