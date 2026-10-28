//! Test-only original mounted Vault provider and constructor-read observations.
//! Frozen words are assertions only. Every returned value comes from the real
//! immutable graph; this does not supply native shared-global Change/cvar state.
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeDataOrigin},
    attribute_runtime_corpus::mounted_physics_attributes_with_classes,
    skeleton_data::SkeletonData,
    skeleton_drive_attributes::{
        ANIMATION_DRIVES_CLASS, DEFAULT_COLLECTION, LOCAL_COLLECTION, LOCAL_ROOT_COLLECTION,
        SKELETON_DRIVES_CLASS, SkeletonDriveAttributeError, SkeletonDriveAttributes,
    },
    skeleton_drives::{SkeletonDriveAttributeRead, SkeletonDriveType},
};

const SOFT_KEYS: [u64; 4] = [
    0xe1a7_f2dc_fab0_b8d0,
    0xf300_ffd5_99e1_709e,
    0xf508_d452_b962_f68d,
    0x15a2_6b61_43e7_df97,
];
const SOFT_WORDS: [u32; 4] = [0x46c3_5000, 0x42c8_0000, 0x47c3_5000, 0x42c8_0000];
const ANIMATION_WORDS: [u32; 4] = [0x3f80_0000, 0x3f80_0000, 0x4080_0000, 0x4080_0000];

#[derive(Clone, Debug, Eq, PartialEq)]
enum ReadEvent {
    Change(SkeletonDriveType),
    Lookup(u64),
    Load(u32),
    Cached(SkeletonDriveType, usize, u32),
}

/// Pure observer around the concrete production provider. It neither replaces
/// attribute results nor combines repeated reads into cached test constants.
pub(crate) struct MountedSkeletonDriveAttributes {
    reader: SkeletonDriveAttributes,
    events: Vec<ReadEvent>,
}

/// Uses the existing raw-mounted db.big fingerprint/dependency proof. Original
/// animation `LOCAL_ROOT` VLT89b8/layoutslot89e0 -> `BIN2cbe8`; LOCAL `VLT89f8`/
/// layoutslot8a20 -> `BIN2cbf8`. Both own16 bytes, rather than inheriting default
/// `BIN2cbd8` zero16. Four ordinary scalar nodes are documented separately below.
pub(crate) fn mounted_skeleton_drive_attributes() -> MountedSkeletonDriveAttributes {
    let vault =
        mounted_physics_attributes_with_classes(&[ANIMATION_DRIVES_CLASS, SKELETON_DRIVES_CLASS]);
    for key in [LOCAL_COLLECTION, LOCAL_ROOT_COLLECTION] {
        let instance = vault.instance(ANIMATION_DRIVES_CLASS, key);
        assert!(!instance.is_null());
        let layout = instance.try_cached_layout(16).unwrap().unwrap();
        assert_eq!(
            layout.origin(),
            AttributeRuntimeDataOrigin::CollectionLayout
        );
        let actual: [u32; 4] = std::array::from_fn(|word| {
            u32::from_be_bytes(layout.bytes()[word * 4..word * 4 + 4].try_into().unwrap())
        });
        assert_eq!(actual, ANIMATION_WORDS);
    }
    let default = vault
        .instance(ANIMATION_DRIVES_CLASS, DEFAULT_COLLECTION)
        .try_cached_layout(16)
        .unwrap()
        .unwrap();
    assert_eq!(default.bytes(), &[0; 16]);
    // Original physics_skeleton_drives.default VLT5e088, explicit inline nodes
    // 5e0e0/5e0f0/5e110/5e120: not its unrelated generated736-byte layout.
    let global = vault.instance(SKELETON_DRIVES_CLASS, DEFAULT_COLLECTION);
    for (key, expected) in SOFT_KEYS.into_iter().zip(SOFT_WORDS) {
        let data = global.try_get_data(key, 0).unwrap().unwrap();
        assert_eq!(data.origin(), AttributeRuntimeDataOrigin::Inline);
        assert_eq!(
            u32::from_be_bytes(data.bytes().try_into().unwrap()),
            expected
        );
    }
    MountedSkeletonDriveAttributes {
        reader: SkeletonDriveAttributes::bind(&vault),
        events: Vec::new(),
    }
}

impl MountedSkeletonDriveAttributes {
    /// Called after the real producer, not used to generate its input events.
    /// The expected eligible Part sequence is derived from retained source
    /// parents, not from emitted Drive records under test.
    pub(crate) fn assert_constructor_reads(&self, data: &SkeletonData) {
        let mut expected = Vec::new();
        for _ in 0..24 {
            expected.push(ReadEvent::Change(SkeletonDriveType::Local));
            expected.push(ReadEvent::Change(SkeletonDriveType::LocalRoot));
            for (key, word) in SOFT_KEYS.into_iter().zip(SOFT_WORDS) {
                expected.push(ReadEvent::Lookup(key));
                expected.push(ReadEvent::Load(word));
            }
        }
        for parent in data.parents() {
            if parent.is_none() {
                continue;
            }
            for (kind, offsets) in [
                (SkeletonDriveType::Local, &[8_usize, 12][..]),
                (SkeletonDriveType::LocalRoot, &[8_usize, 12, 4, 0][..]),
            ] {
                for &offset in offsets {
                    expected.push(ReadEvent::Cached(kind, offset, ANIMATION_WORDS[offset / 4]));
                }
            }
        }
        assert_eq!(
            self.events, expected,
            "source InitAttribs/FETCH/normal-follow read chronology"
        );
    }
}

impl SkeletonDriveAttributeRead for MountedSkeletonDriveAttributes {
    type Pointer = Option<AttributeRuntimeData>;
    type Error = SkeletonDriveAttributeError;

    fn find_and_change_animation_drive(
        &mut self,
        kind: SkeletonDriveType,
    ) -> Result<(), Self::Error> {
        self.events.push(ReadEvent::Change(kind));
        self.reader.find_and_change_animation_drive(kind)
    }
    fn lookup_global(&mut self, key: u64) -> Result<Self::Pointer, Self::Error> {
        self.events.push(ReadEvent::Lookup(key));
        self.reader.lookup_global(key)
    }
    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        let value = self.reader.float(pointer)?;
        self.events.push(ReadEvent::Load(value.to_bits()));
        Ok(value)
    }
    fn animation_float(
        &mut self,
        kind: SkeletonDriveType,
        byte_offset: usize,
    ) -> Result<f32, Self::Error> {
        let value = self.reader.animation_float(kind, byte_offset)?;
        self.events
            .push(ReadEvent::Cached(kind, byte_offset, value.to_bits()));
        Ok(value)
    }
}
