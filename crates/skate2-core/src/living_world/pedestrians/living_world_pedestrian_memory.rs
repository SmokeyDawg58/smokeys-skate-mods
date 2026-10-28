//! LWPedestrianMemoryMan::Init82e6b070. Reuses the existing MemStore slot
//! ordering; these are allocation owners, not a second pedestrian roster.
use crate::world::LivingWorldMemStoreSlots;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub(crate) enum PedestrianStore {
    Entity = 0,
    Core = 1,
    Collision = 2,
    NavRig = 3,
    Intent = 4,
    AiState = 5,
    Animation = 6,
    PostAdjust = 7,
    HeadTracking = 8,
    SpatialAdjust = 9,
    SecurityGuard = 10,
    BigBlack = 11,
}

/// Original allocation sizes, retained for evidence/debug accounting. Rust
/// objects own typed fields; Xbox byte sizes are not host structure layouts.
pub(crate) const NATIVE_BLOCK_BYTES: [usize; 12] = [
    0x1a70, 0x120, 0x30, 0x150, 0x840, 0xcb0, 0x66e0, 0x160,
    0xb0, 0xe0, 0x70, 0xb20,
];

pub(crate) struct PedestrianMemory {
    common: [LivingWorldMemStoreSlots<40>; 10],
    guards: LivingWorldMemStoreSlots<8>,
    big_black: LivingWorldMemStoreSlots<1>,
}
impl PedestrianMemory {
    pub(crate) fn initialized() -> Self {
        Self {
            common: std::array::from_fn(|_| LivingWorldMemStoreSlots::new()),
            guards: LivingWorldMemStoreSlots::new(),
            big_black: LivingWorldMemStoreSlots::new(),
        }
    }
    pub(crate) fn request(&mut self, store: PedestrianStore) -> Option<u32> {
        match store {
            PedestrianStore::SecurityGuard => self.guards.allocate(),
            PedestrianStore::BigBlack => self.big_black.allocate(),
            _ => self.common[store as usize].allocate(),
        }
    }
    pub(crate) fn release(&mut self, store: PedestrianStore, slot: u32) -> bool {
        match store {
            PedestrianStore::SecurityGuard => self.guards.free(slot),
            PedestrianStore::BigBlack => self.big_black.free(slot),
            _ => self.common[store as usize].free(slot),
        }
    }
    pub(crate) fn available(&self, store: PedestrianStore) -> usize {
        match store {
            PedestrianStore::SecurityGuard => self.guards.available(),
            PedestrianStore::BigBlack => self.big_black.available(),
            _ => self.common[store as usize].available(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PedestrianClass { Regular, SecurityGuard, BigBlack }
impl PedestrianClass {
    /// LWPedestrianFactory::InitCallbackMap82e99c80. Full64-bit Attrib keys;
    /// unregistered keys use CreateRegularPedestrian82e99720, not a guard.
    pub(crate) fn for_entity_key(key: u64) -> Self {
        use skate2_formats::attrib::string_hash64;
        if key == string_hash64("security_nsz01") || key == string_hash64("security_nsz02") {
            Self::SecurityGuard
        } else if key == string_hash64("big_black_regular01") {
            Self::BigBlack
        } else {
            Self::Regular
        }
    }
}
