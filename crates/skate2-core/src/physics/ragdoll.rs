//! Exact skeleton ragdoll setup recovered from the supplied debug build.
//!
//! This module reconstructs the fixed 24-part culling table and the state
//! mutation performed when an already-constructed skeleton enters ragdoll.
//! Ragdoll dimensions and mass-property generation remain outside this module;
//! setup accepts their prepared inverse values from its caller.

/// Number of entries in `Sk8::Physics::SkeletonDataBase::PartIndex`.
pub const SKELETON_PART_COUNT: usize = 24;

/// Number of packed words in a 24-by-24 `rw::BitTable`.
pub const SKELETON_CULLING_WORD_COUNT: usize = 18;

/// Number of undirected pairs cleared by `CreateRagdollCullingTable`.
pub const RAGDOLL_CULLING_CLEAR_PAIR_COUNT: usize = 68;

/// Raw invalid value declared by the PDB `PartIndex` enum.
pub const INVALID_SKELETON_PART_INDEX_RAW: i32 = -1;

/// Exact multiplier applied by `Body::SetLinearDrag` and
/// `Body::SetAngularDrag` before writing an inertia record.
///
/// Reference: raw float `0x426fffff` at `0x8225fd98`, consumed by the helpers
/// at `0x82de7de0` and `0x82de7f38`.
pub const SKELETON_DRAG_STORAGE_SCALE: f32 = f32::from_bits(0x426f_ffff);

/// Assembly group and per-part user-data value installed by plain ragdoll.
pub const RAGDOLL_ASSEMBLY_GROUP: u32 = 7;

/// Assembly group and per-part user-data value installed by the separate
/// collide-with-world-only operation.
pub const WORLD_ONLY_ASSEMBLY_GROUP: u32 = 3;

/// Volume group installed by the world-only collision helper.
pub const WORLD_ONLY_VOLUME_GROUP: u32 = 0;

/// Stable indices used by the reference skeleton assembly and culling tables.
///
/// Reference: PDB enum `Sk8::Physics::SkeletonDataBase::PartIndex` and the
/// 24-by-24 loops in `SkeletonBody::CreateRagdollCullingTable` at
/// `0x82b5f93c..0x82b5f9c0`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(i32)]
pub enum SkeletonPartIndex {
    SkateboardRoot = 0,
    Neck1 = 1,
    Neck = 2,
    LeftHand = 3,
    LeftForearm = 4,
    LeftArm = 5,
    LeftShoulder = 6,
    RightHand = 7,
    RightForearm = 8,
    RightArm = 9,
    RightShoulder = 10,
    Spine3 = 11,
    Spine2 = 12,
    Spine1 = 13,
    Spine = 14,
    LeftToeBase = 15,
    LeftFoot = 16,
    LeftLeg = 17,
    LeftUpLeg = 18,
    RightToeBase = 19,
    RightFoot = 20,
    RightLeg = 21,
    RightUpLeg = 22,
    Hips = 23,
}

impl SkeletonPartIndex {
    /// All skeleton parts in packed-table order.
    pub const ALL: [Self; SKELETON_PART_COUNT] = [
        Self::SkateboardRoot,
        Self::Neck1,
        Self::Neck,
        Self::LeftHand,
        Self::LeftForearm,
        Self::LeftArm,
        Self::LeftShoulder,
        Self::RightHand,
        Self::RightForearm,
        Self::RightArm,
        Self::RightShoulder,
        Self::Spine3,
        Self::Spine2,
        Self::Spine1,
        Self::Spine,
        Self::LeftToeBase,
        Self::LeftFoot,
        Self::LeftLeg,
        Self::LeftUpLeg,
        Self::RightToeBase,
        Self::RightFoot,
        Self::RightLeg,
        Self::RightUpLeg,
        Self::Hips,
    ];

    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    #[must_use]
    pub const fn from_index(index: usize) -> Option<Self> {
        if index < SKELETON_PART_COUNT {
            Some(Self::ALL[index])
        } else {
            None
        }
    }

    /// Converts the signed PDB enum representation without inventing a valid
    /// skeleton part for `INVALID_PART_INDEX` or unknown raw values.
    #[must_use]
    pub fn from_raw(raw: i32) -> Option<Self> {
        usize::try_from(raw).ok().and_then(Self::from_index)
    }
}

/// Undirected zero-bit pairs in native execution order.
///
/// `SkeletonBody::CreateRagdollCullingTable` explicitly clears `(a, b)` and
/// then `(b, a)` for each entry. The stores span `0x82b5f9f0..0x82b61728`.
pub const RAGDOLL_CULLING_CLEAR_PAIRS: [(SkeletonPartIndex, SkeletonPartIndex);
    RAGDOLL_CULLING_CLEAR_PAIR_COUNT] = [
    (SkeletonPartIndex::Neck1, SkeletonPartIndex::Spine3),
    (SkeletonPartIndex::LeftFoot, SkeletonPartIndex::RightFoot),
    (SkeletonPartIndex::LeftFoot, SkeletonPartIndex::RightLeg),
    (SkeletonPartIndex::LeftFoot, SkeletonPartIndex::RightUpLeg),
    (SkeletonPartIndex::RightFoot, SkeletonPartIndex::LeftUpLeg),
    (SkeletonPartIndex::RightFoot, SkeletonPartIndex::LeftLeg),
    (SkeletonPartIndex::LeftLeg, SkeletonPartIndex::RightLeg),
    (SkeletonPartIndex::LeftLeg, SkeletonPartIndex::RightUpLeg),
    (SkeletonPartIndex::RightLeg, SkeletonPartIndex::LeftUpLeg),
    (SkeletonPartIndex::LeftLeg, SkeletonPartIndex::RightHand),
    (SkeletonPartIndex::RightLeg, SkeletonPartIndex::LeftHand),
    (SkeletonPartIndex::LeftLeg, SkeletonPartIndex::RightArm),
    (SkeletonPartIndex::RightLeg, SkeletonPartIndex::LeftArm),
    (SkeletonPartIndex::LeftUpLeg, SkeletonPartIndex::RightHand),
    (SkeletonPartIndex::RightUpLeg, SkeletonPartIndex::LeftHand),
    (SkeletonPartIndex::LeftUpLeg, SkeletonPartIndex::RightArm),
    (SkeletonPartIndex::RightUpLeg, SkeletonPartIndex::LeftArm),
    (
        SkeletonPartIndex::LeftUpLeg,
        SkeletonPartIndex::RightForearm,
    ),
    (
        SkeletonPartIndex::RightUpLeg,
        SkeletonPartIndex::LeftForearm,
    ),
    (SkeletonPartIndex::LeftUpLeg, SkeletonPartIndex::LeftHand),
    (SkeletonPartIndex::RightUpLeg, SkeletonPartIndex::RightHand),
    (
        SkeletonPartIndex::RightUpLeg,
        SkeletonPartIndex::RightForearm,
    ),
    (SkeletonPartIndex::LeftUpLeg, SkeletonPartIndex::LeftForearm),
    (SkeletonPartIndex::LeftHand, SkeletonPartIndex::RightHand),
    (SkeletonPartIndex::LeftHand, SkeletonPartIndex::RightArm),
    (SkeletonPartIndex::RightHand, SkeletonPartIndex::LeftArm),
    (SkeletonPartIndex::LeftHand, SkeletonPartIndex::RightForearm),
    (SkeletonPartIndex::RightHand, SkeletonPartIndex::LeftForearm),
    (SkeletonPartIndex::LeftHand, SkeletonPartIndex::Neck1),
    (SkeletonPartIndex::RightHand, SkeletonPartIndex::Neck1),
    (SkeletonPartIndex::LeftHand, SkeletonPartIndex::Neck),
    (SkeletonPartIndex::RightHand, SkeletonPartIndex::Neck),
    (SkeletonPartIndex::LeftHand, SkeletonPartIndex::Spine),
    (SkeletonPartIndex::RightHand, SkeletonPartIndex::Spine),
    (SkeletonPartIndex::LeftHand, SkeletonPartIndex::Spine1),
    (SkeletonPartIndex::RightHand, SkeletonPartIndex::Spine1),
    (SkeletonPartIndex::LeftHand, SkeletonPartIndex::Spine2),
    (SkeletonPartIndex::RightHand, SkeletonPartIndex::Spine2),
    (SkeletonPartIndex::LeftHand, SkeletonPartIndex::Spine3),
    (SkeletonPartIndex::RightHand, SkeletonPartIndex::Spine3),
    (
        SkeletonPartIndex::RightForearm,
        SkeletonPartIndex::LeftForearm,
    ),
    (SkeletonPartIndex::RightForearm, SkeletonPartIndex::LeftArm),
    (SkeletonPartIndex::LeftForearm, SkeletonPartIndex::RightArm),
    (SkeletonPartIndex::LeftForearm, SkeletonPartIndex::Neck1),
    (SkeletonPartIndex::RightForearm, SkeletonPartIndex::Neck1),
    (SkeletonPartIndex::LeftForearm, SkeletonPartIndex::Neck),
    (SkeletonPartIndex::RightForearm, SkeletonPartIndex::Neck),
    (SkeletonPartIndex::LeftForearm, SkeletonPartIndex::Spine),
    (SkeletonPartIndex::RightForearm, SkeletonPartIndex::Spine),
    (SkeletonPartIndex::LeftForearm, SkeletonPartIndex::Spine1),
    (SkeletonPartIndex::RightForearm, SkeletonPartIndex::Spine1),
    (SkeletonPartIndex::LeftForearm, SkeletonPartIndex::Spine2),
    (SkeletonPartIndex::RightForearm, SkeletonPartIndex::Spine2),
    (SkeletonPartIndex::LeftForearm, SkeletonPartIndex::Spine3),
    (SkeletonPartIndex::RightForearm, SkeletonPartIndex::Spine3),
    (SkeletonPartIndex::LeftArm, SkeletonPartIndex::RightArm),
    (SkeletonPartIndex::LeftArm, SkeletonPartIndex::Neck1),
    (SkeletonPartIndex::RightArm, SkeletonPartIndex::Neck1),
    (SkeletonPartIndex::LeftArm, SkeletonPartIndex::Neck),
    (SkeletonPartIndex::RightArm, SkeletonPartIndex::Neck),
    (SkeletonPartIndex::LeftArm, SkeletonPartIndex::Spine),
    (SkeletonPartIndex::RightArm, SkeletonPartIndex::Spine),
    (SkeletonPartIndex::LeftArm, SkeletonPartIndex::Spine1),
    (SkeletonPartIndex::RightArm, SkeletonPartIndex::Spine1),
    (SkeletonPartIndex::LeftArm, SkeletonPartIndex::Spine2),
    (SkeletonPartIndex::RightArm, SkeletonPartIndex::Spine2),
    (SkeletonPartIndex::LeftArm, SkeletonPartIndex::Spine3),
    (SkeletonPartIndex::RightArm, SkeletonPartIndex::Spine3),
];

/// Six undirected zero-bit pairs used by the separate normal culling table.
///
/// Reference: `SkeletonBody::CreateNormalCullingTable` at
/// `0x82b5f53c..0x82b5f7c0`. This constant exists only to preserve the proven
/// distinction between normal and ragdoll tables; it does not model setup.
pub const NORMAL_CULLING_CLEAR_PAIRS: [(SkeletonPartIndex, SkeletonPartIndex); 6] = [
    (SkeletonPartIndex::Spine, SkeletonPartIndex::LeftForearm),
    (SkeletonPartIndex::Spine, SkeletonPartIndex::RightForearm),
    (SkeletonPartIndex::Hips, SkeletonPartIndex::LeftForearm),
    (SkeletonPartIndex::Hips, SkeletonPartIndex::RightForearm),
    (SkeletonPartIndex::LeftForearm, SkeletonPartIndex::LeftUpLeg),
    (
        SkeletonPartIndex::RightForearm,
        SkeletonPartIndex::RightUpLeg,
    ),
];

/// Exact packed result produced by the ragdoll-table construction path.
///
/// Words are row-major and LSB-first: bit `(row * 24 + column) & 31` of word
/// `(row * 24 + column) >> 5` represents that directed coordinate.
pub const DEBUG_REFERENCE_RAGDOLL_CULLING_WORDS: [u32; SKELETON_CULLING_WORD_COUNT] = [
    0x47ff_ffff,
    0xfc47_fff4,
    0x9b84_79ff,
    0x79bb_8479,
    0xffff_9f84,
    0xb987_c1ff,
    0xc1bb_87c1,
    0xffff_f987,
    0xfffc_45ff,
    0x47ff_fc47,
    0xfc47_fffc,
    0xffff_ffff,
    0x7f8f_ffff,
    0xfc67_8ffd,
    0xffff_ffcf,
    0xd7f8_ffff,
    0xfe47_f8ff,
    0xffff_fffc,
];

/// Exact packed result produced by the separate normal-table construction.
pub const DEBUG_REFERENCE_NORMAL_CULLING_WORDS: [u32; SKELETON_CULLING_WORD_COUNT] = [
    0xffff_ffff,
    0xffff_ffff,
    0xffff_ffff,
    0xff7b_bfff,
    0xffff_ffff,
    0xffff_ffff,
    0xff3f_bfff,
    0xffff_ffff,
    0xffff_ffff,
    0xffff_ffff,
    0xfeef_ffff,
    0xffff_ffff,
    0xffff_ffff,
    0xffef_ffff,
    0xffff_ffff,
    0xffff_ffff,
    0xfeff_ffff,
    0xfffe_efff,
];

/// Meaning of a packed skeleton culling-table bit.
///
/// Reference: `RunIntersectionTests` loads/tests the word at `0x827cbf30` and
/// `0x827cbf80`; its `beq` at `0x827cbf94` branches away for a set bit, while
/// a zero bit falls through to the intersection path at `0x827cbf98`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkeletonPairCulling {
    Eligible,
    Suppressed,
}

/// Fixed-size semantic representation of the reference table's packed words.
///
/// This omits the console `rw::BitTable` header and inventory identity; it
/// preserves the complete 24-by-24 bit contents and addressing behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SkeletonCullingTable {
    words: [u32; SKELETON_CULLING_WORD_COUNT],
}

impl SkeletonCullingTable {
    /// Constructs the supplied debug build's exact ragdoll culling table.
    ///
    /// `SkeletonBody::CreateRagdollCullingTable` at `0x82b5f7d0` first sets
    /// all 576 bits, including the diagonal, then performs the 68 symmetric
    /// clears in [`RAGDOLL_CULLING_CLEAR_PAIRS`].
    #[must_use]
    pub const fn debug_reference_ragdoll() -> Self {
        let mut table = Self::all_suppressed();
        let mut pair_index = 0;
        while pair_index < RAGDOLL_CULLING_CLEAR_PAIR_COUNT {
            let (first, second) = RAGDOLL_CULLING_CLEAR_PAIRS[pair_index];
            table.clear_directed(first, second);
            table.clear_directed(second, first);
            pair_index += 1;
        }
        table
    }

    /// Returns the exact 18 packed words without changing their bit layout.
    #[must_use]
    pub const fn words(&self) -> &[u32; SKELETON_CULLING_WORD_COUNT] {
        &self.words
    }

    /// Returns the culling disposition of one directed part pair.
    #[must_use]
    pub const fn pair_culling(
        &self,
        row: SkeletonPartIndex,
        column: SkeletonPartIndex,
    ) -> SkeletonPairCulling {
        let (word_index, mask) = packed_location(row, column);
        if self.words[word_index] & mask == 0 {
            SkeletonPairCulling::Eligible
        } else {
            SkeletonPairCulling::Suppressed
        }
    }

    const fn all_suppressed() -> Self {
        Self {
            words: [u32::MAX; SKELETON_CULLING_WORD_COUNT],
        }
    }

    const fn clear_directed(&mut self, row: SkeletonPartIndex, column: SkeletonPartIndex) {
        let (word_index, mask) = packed_location(row, column);
        self.words[word_index] &= !mask;
    }
}

const fn packed_location(row: SkeletonPartIndex, column: SkeletonPartIndex) -> (usize, u32) {
    let flat_index = row.index() * SKELETON_PART_COUNT + column.index();
    (flat_index >> 5, 1_u32 << (flat_index & 31))
}

/// Opaque identity of an already-resolved skeleton culling-table resource.
///
/// The reference setup dereferences `SkeletonBody::mCullingTableRagDoll` at
/// `+0xb7c` and copies its resource pointer into the assembly at `0x82b500c8`
/// through `0x82b500d8`. Resource lookup and lifetime remain caller-owned.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SkeletonCullingTableHandle(pub u32);

/// Material slot referenced by one skeleton assembly part.
///
/// `SetUpRagdoll` installs `RagdollBody` on parts 1 through 23, then replaces
/// parts 1, 3, and 7 with `RagdollHead` at `0x82b503b8..0x82b503e4`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkeletonMaterialSlot {
    Normal,
    RagdollBody,
    RagdollHead,
    Zero,
}

/// Mutable inertia fields touched by ragdoll setup.
///
/// Reference: PDB `rw::collision::Inertia`, whose inverse-inertia storage is
/// at `+0x00`, inverse mass at `+0x10`, spherical scalar at `+0x14`, linear
/// drag at `+0x20`, and angular drag at `+0x24`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkeletonInertiaSetupState {
    pub inverse_inertia: [f32; 4],
    pub inverse_mass: f32,
    pub spherical: f32,
    pub linear_drag: f32,
    pub angular_drag: f32,
}

/// Setup-visible state of one `0x60`-byte assembly part.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkeletonPartSetupState {
    pub material: SkeletonMaterialSlot,
    pub user_data: u32,
    pub inertia: SkeletonInertiaSetupState,
}

/// Setup-visible state of one skeleton collision volume.
///
/// Reference: PDB `rw::collision::Volume`, group ID at `+0x54` and flags at
/// `+0x5c`; the 24 disable counters begin at `SkeletonBody +0x1e00`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SkeletonVolumeSetupState {
    pub group_id: u32,
    pub flags: u32,
    pub disable_count: u32,
}

/// Complete mutable boundary needed by the recovered setup operation.
///
/// Proxy and root-volume fields are included so tests and callers can prove
/// that plain `SetUpRagdoll` leaves them untouched. The separate world-only
/// operation mutates them explicitly.
#[derive(Clone, Debug, PartialEq)]
pub struct SkeletonRagdollSetupState {
    pub assembly_culling_table: SkeletonCullingTableHandle,
    pub ragdoll_culling_table: SkeletonCullingTableHandle,
    pub assembly_group_id: u32,
    pub parts: [SkeletonPartSetupState; SKELETON_PART_COUNT],
    pub volumes: [SkeletonVolumeSetupState; SKELETON_PART_COUNT],
    pub volume_disable_count_active: bool,
    pub proxy_volume_enabled: bool,
    pub ragdoll_body_restitution: f32,
    pub ragdoll_head_restitution: f32,
    pub is_ragdoll: bool,
}

/// Precomputed dynamics installed by setup when the corresponding controls
/// are enabled.
///
/// `SkeletonBody::Init` creates these arrays at `0x82b61e98..0x82b62074`.
/// Their mass-property generation is deliberately not duplicated here.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedRagdollDynamics {
    pub inverse_mass: [f32; SKELETON_PART_COUNT],
    pub inverse_inertia: [[f32; 4]; SKELETON_PART_COUNT],
}

/// Object-local input and prepared mass properties consumed by setup.
///
/// `object_is_bouncy` is the byte at `SkeletonBody +0x1e62`, sampled once at
/// `0x82b5000c`. Every cVar and VLT lookup stays on
/// [`RagdollSetupCollaborator`] so its native short-circuit and per-part read
/// timing remain observable.
#[derive(Clone, Debug, PartialEq)]
pub struct RagdollSetupInputs {
    pub object_is_bouncy: bool,
    pub dynamics: PreparedRagdollDynamics,
}

/// Restitution source read at a particular native lookup site.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RagdollRestitutionSource {
    BodyAttribute,
    HeadAttribute,
    BouncyCVar,
}

/// Material restitution lane written by setup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RagdollRestitutionTarget {
    Body,
    Head,
}

/// Drag helper completed by setup after both live values have been sampled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RagdollDragTarget {
    Angular,
    Linear,
}

/// Native call site represented by an otherwise empty folded function.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RagdollFoldedCallSite {
    SetupPart(SkeletonPartIndex),
    SetupComplete,
    WorldOnlyPart(SkeletonPartIndex),
    WorldOnlyRoot,
}

/// External operations whose native ordering cannot be represented by the
/// final state alone.
///
/// Implementations perform cVar/VLT reads at the native call sites so their
/// short-circuiting and per-part timing remain observable. `folded_empty_call`
/// represents machine-level calls to `0x825187c0`, whose body is exactly
/// `blr`; it exists here so the reference edge ordering remains testable.
pub trait RagdollSetupCollaborator {
    /// Live `kSB_UseRagdollJointLimits` read at `0x82b4ffec..0x82b50004`.
    fn read_use_ragdoll_joint_limits(&mut self) -> bool;

    fn apply_ragdoll_joint_limits(&mut self);

    /// Live `kBouncyRagdollCheat` read at `0x82b50018..0x82b5002c`.
    /// The caller skips this entirely when the object-local bouncy byte is set.
    fn read_bouncy_ragdoll_cheat(&mut self) -> bool;

    fn read_restitution(&mut self, source: RagdollRestitutionSource) -> f32;

    fn restitution_written(&mut self, target: RagdollRestitutionTarget, value: f32);

    /// Live angular-drag VLT lookup at `0x82b50114..0x82b50170`.
    fn read_angular_drag(&mut self) -> f32;

    /// Live linear-drag VLT lookup at `0x82b50174..0x82b501cc`.
    fn read_linear_drag(&mut self) -> f32;

    /// Reports completion of the corresponding native body helper so its
    /// ordering relative to both live reads remains testable.
    fn drag_written(&mut self, target: RagdollDragTarget, stored_value: f32);

    /// Live hash `0xf1899bbc16050b02` lookup performed once for this part.
    fn read_install_inverse_mass(&mut self, part: SkeletonPartIndex) -> bool;

    /// Live hash `0xd6de526735151b3f` lookup performed independently once for
    /// this part, whether or not inverse-mass installation was enabled.
    fn read_install_inverse_inertia(&mut self, part: SkeletonPartIndex) -> bool;

    fn folded_empty_call(&mut self, site: RagdollFoldedCallSite);
}

/// Applies the exact setup-time state mutation for a constructed skeleton.
///
/// Reference: `Sk8::Physics::SkeletonBody::SetUpRagdoll` at
/// `0x82b4ffd8..0x82b503fc`. This is plain ragdoll setup; it intentionally does
/// not apply the later collide-with-world-only wrapper.
pub fn set_up_ragdoll(
    state: &mut SkeletonRagdollSetupState,
    inputs: &RagdollSetupInputs,
    collaborator: &mut impl RagdollSetupCollaborator,
) {
    if collaborator.read_use_ragdoll_joint_limits() {
        collaborator.apply_ragdoll_joint_limits();
    }

    let use_bouncy_restitution =
        inputs.object_is_bouncy || collaborator.read_bouncy_ragdoll_cheat();
    if use_bouncy_restitution {
        let head = collaborator.read_restitution(RagdollRestitutionSource::BouncyCVar);
        state.ragdoll_head_restitution = head;
        collaborator.restitution_written(RagdollRestitutionTarget::Head, head);

        let body = collaborator.read_restitution(RagdollRestitutionSource::BouncyCVar);
        state.ragdoll_body_restitution = body;
        collaborator.restitution_written(RagdollRestitutionTarget::Body, body);
    } else {
        let body = collaborator.read_restitution(RagdollRestitutionSource::BodyAttribute);
        state.ragdoll_body_restitution = body;
        collaborator.restitution_written(RagdollRestitutionTarget::Body, body);

        let head = collaborator.read_restitution(RagdollRestitutionSource::HeadAttribute);
        state.ragdoll_head_restitution = head;
        collaborator.restitution_written(RagdollRestitutionTarget::Head, head);
    }

    state.assembly_culling_table = state.ragdoll_culling_table;
    state.assembly_group_id = RAGDOLL_ASSEMBLY_GROUP;
    for part in &mut state.parts {
        part.user_data = RAGDOLL_ASSEMBLY_GROUP;
    }

    let angular_drag = collaborator.read_angular_drag();
    let linear_drag = collaborator.read_linear_drag();

    let stored_angular_drag = angular_drag * SKELETON_DRAG_STORAGE_SCALE;
    for part in &mut state.parts {
        part.inertia.angular_drag = stored_angular_drag;
    }
    collaborator.drag_written(RagdollDragTarget::Angular, stored_angular_drag);

    let stored_linear_drag = linear_drag * SKELETON_DRAG_STORAGE_SCALE;
    for part in &mut state.parts {
        part.inertia.linear_drag = stored_linear_drag;
    }
    collaborator.drag_written(RagdollDragTarget::Linear, stored_linear_drag);

    for part_index in SkeletonPartIndex::ALL.into_iter().skip(1) {
        let index = part_index.index();
        state.volumes[index].disable_count = 0;
        collaborator.folded_empty_call(RagdollFoldedCallSite::SetupPart(part_index));
        state.volumes[index].flags |= 1;
        state.volume_disable_count_active = false;
        state.parts[index].material = SkeletonMaterialSlot::RagdollBody;

        if collaborator.read_install_inverse_mass(part_index) {
            state.parts[index].inertia.inverse_mass = inputs.dynamics.inverse_mass[index];
        }
        if collaborator.read_install_inverse_inertia(part_index) {
            let inverse_inertia = inputs.dynamics.inverse_inertia[index];
            state.parts[index].inertia.inverse_inertia = inverse_inertia;
            state.parts[index].inertia.spherical = reciprocal_ppc_selected_minimum(inverse_inertia);
        }
    }

    for part_index in [
        SkeletonPartIndex::Neck1,
        SkeletonPartIndex::LeftHand,
        SkeletonPartIndex::RightHand,
    ] {
        state.parts[part_index.index()].material = SkeletonMaterialSlot::RagdollHead;
    }
    state.is_ragdoll = true;
    collaborator.folded_empty_call(RagdollFoldedCallSite::SetupComplete);
}

/// Applies the separate world-only collision wrapper used after ragdoll setup.
///
/// Reference: `SkeletonBody::CollideWithWorldOnly` at `0x82b4fa80` and
/// `Skeleton::CollideWithWorldOnly` at `0x82b54370`. Controller state 3 calls
/// this after [`set_up_ragdoll`]; controller state 8 does not.
pub fn set_ragdoll_collide_with_world_only(
    state: &mut SkeletonRagdollSetupState,
    collaborator: &mut impl RagdollSetupCollaborator,
) {
    for part_index in SkeletonPartIndex::ALL.into_iter().skip(1) {
        let volume = &mut state.volumes[part_index.index()];
        if volume.disable_count == 0 {
            collaborator.folded_empty_call(RagdollFoldedCallSite::WorldOnlyPart(part_index));
            volume.flags |= 1;
            volume.group_id = WORLD_ONLY_VOLUME_GROUP;
        }
    }

    state.assembly_group_id = WORLD_ONLY_ASSEMBLY_GROUP;
    for part in &mut state.parts {
        part.user_data = WORLD_ONLY_ASSEMBLY_GROUP;
    }
    state.proxy_volume_enabled = false;
    collaborator.folded_empty_call(RagdollFoldedCallSite::WorldOnlyRoot);
    state.volumes[SkeletonPartIndex::SkateboardRoot.index()].flags &= !1;
    state.volumes[SkeletonPartIndex::SkateboardRoot.index()].disable_count = 0;
}

/// Reproduces the two PPC `fcmpu`/branch selections at
/// `0x82b50354..0x82b5037c`, followed by `fdivs` at `0x82b5037c`.
///
/// Equality selects `y` at the first comparison and `z` at the second. An
/// unordered comparison is also false for `<`, matching the raw branch path:
/// the first comparison selects `y`, while the second selects `z`. Final NaN
/// payload/sign, subnormal handling, and PPC FPSCR versus host behavior remain
/// a differential-test boundary; this helper claims the proven branch order.
fn reciprocal_ppc_selected_minimum(inverse_inertia: [f32; 4]) -> f32 {
    let [x, y, z, _] = inverse_inertia;
    let xy = if x < y { x } else { y };
    let selected = if xy < z { xy } else { z };
    1.0 / selected
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, VecDeque};
    use std::mem::size_of;

    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum CollaboratorEvent {
        JointLimitsRead,
        JointLimits,
        BouncyCheatRead,
        RestitutionRead(RagdollRestitutionSource),
        RestitutionWrite(RagdollRestitutionTarget, u32),
        AngularDragRead,
        LinearDragRead,
        DragWritten(RagdollDragTarget, u32),
        InstallInverseMassRead(SkeletonPartIndex),
        InstallInverseInertiaRead(SkeletonPartIndex),
        Folded(RagdollFoldedCallSite),
    }

    struct RecordingCollaborator {
        joint_limit_values: VecDeque<bool>,
        bouncy_cheat_values: VecDeque<bool>,
        body_values: VecDeque<f32>,
        head_values: VecDeque<f32>,
        bouncy_values: VecDeque<f32>,
        angular_drag_values: VecDeque<f32>,
        linear_drag_values: VecDeque<f32>,
        install_inverse_mass_values: VecDeque<bool>,
        install_inverse_inertia_values: VecDeque<bool>,
        events: Vec<CollaboratorEvent>,
    }

    impl RecordingCollaborator {
        fn new(body: &[f32], head: &[f32], bouncy: &[f32]) -> Self {
            Self {
                joint_limit_values: [false].into(),
                bouncy_cheat_values: [false].into(),
                body_values: body.iter().copied().collect(),
                head_values: head.iter().copied().collect(),
                bouncy_values: bouncy.iter().copied().collect(),
                angular_drag_values: [f32::from_bits(0x3f00_0000)].into(),
                linear_drag_values: [f32::from_bits(0x0000_0000)].into(),
                install_inverse_mass_values: [false; SKELETON_PART_COUNT - 1].into(),
                install_inverse_inertia_values: [false; SKELETON_PART_COUNT - 1].into(),
                events: Vec::new(),
            }
        }
    }

    impl RagdollSetupCollaborator for RecordingCollaborator {
        fn read_use_ragdoll_joint_limits(&mut self) -> bool {
            self.events.push(CollaboratorEvent::JointLimitsRead);
            self.joint_limit_values
                .pop_front()
                .expect("test supplied the joint-limit read")
        }

        fn apply_ragdoll_joint_limits(&mut self) {
            self.events.push(CollaboratorEvent::JointLimits);
        }

        fn read_bouncy_ragdoll_cheat(&mut self) -> bool {
            self.events.push(CollaboratorEvent::BouncyCheatRead);
            self.bouncy_cheat_values
                .pop_front()
                .expect("test supplied the bouncy-cheat read")
        }

        fn read_restitution(&mut self, source: RagdollRestitutionSource) -> f32 {
            self.events.push(CollaboratorEvent::RestitutionRead(source));
            match source {
                RagdollRestitutionSource::BodyAttribute => self.body_values.pop_front(),
                RagdollRestitutionSource::HeadAttribute => self.head_values.pop_front(),
                RagdollRestitutionSource::BouncyCVar => self.bouncy_values.pop_front(),
            }
            .expect("test supplied every requested restitution value")
        }

        fn restitution_written(&mut self, target: RagdollRestitutionTarget, value: f32) {
            self.events
                .push(CollaboratorEvent::RestitutionWrite(target, value.to_bits()));
        }

        fn read_angular_drag(&mut self) -> f32 {
            self.events.push(CollaboratorEvent::AngularDragRead);
            self.angular_drag_values
                .pop_front()
                .expect("test supplied the angular-drag read")
        }

        fn read_linear_drag(&mut self) -> f32 {
            self.events.push(CollaboratorEvent::LinearDragRead);
            self.linear_drag_values
                .pop_front()
                .expect("test supplied the linear-drag read")
        }

        fn drag_written(&mut self, target: RagdollDragTarget, stored_value: f32) {
            self.events.push(CollaboratorEvent::DragWritten(
                target,
                stored_value.to_bits(),
            ));
        }

        fn read_install_inverse_mass(&mut self, part: SkeletonPartIndex) -> bool {
            self.events
                .push(CollaboratorEvent::InstallInverseMassRead(part));
            self.install_inverse_mass_values
                .pop_front()
                .expect("test supplied every inverse-mass control read")
        }

        fn read_install_inverse_inertia(&mut self, part: SkeletonPartIndex) -> bool {
            self.events
                .push(CollaboratorEvent::InstallInverseInertiaRead(part));
            self.install_inverse_inertia_values
                .pop_front()
                .expect("test supplied every inverse-inertia control read")
        }

        fn folded_empty_call(&mut self, site: RagdollFoldedCallSite) {
            self.events.push(CollaboratorEvent::Folded(site));
        }
    }

    fn sentinel_setup_state() -> SkeletonRagdollSetupState {
        SkeletonRagdollSetupState {
            assembly_culling_table: SkeletonCullingTableHandle(0x1111_1111),
            ragdoll_culling_table: SkeletonCullingTableHandle(0x2222_2222),
            assembly_group_id: 0x3333_3333,
            parts: std::array::from_fn(|index| {
                let index_u16 = u16::try_from(index).expect("24 parts fit in u16");
                let index_u32 = u32::from(index_u16);
                let index_float = f32::from(index_u16);
                SkeletonPartSetupState {
                    material: SkeletonMaterialSlot::Normal,
                    user_data: 0x4000_0000 | index_u32,
                    inertia: SkeletonInertiaSetupState {
                        inverse_inertia: [
                            100.0 + index_float,
                            200.0 + index_float,
                            300.0 + index_float,
                            400.0 + index_float,
                        ],
                        inverse_mass: 500.0 + index_float,
                        spherical: 600.0 + index_float,
                        linear_drag: 700.0 + index_float,
                        angular_drag: 800.0 + index_float,
                    },
                }
            }),
            volumes: std::array::from_fn(|index| {
                let index_u16 = u16::try_from(index).expect("24 parts fit in u16");
                let index_u32 = u32::from(index_u16);
                SkeletonVolumeSetupState {
                    group_id: 0x5000_0000 | index_u32,
                    flags: 0xa0,
                    disable_count: 90 + index_u32,
                }
            }),
            volume_disable_count_active: true,
            proxy_volume_enabled: true,
            ragdoll_body_restitution: -10.0,
            ragdoll_head_restitution: -20.0,
            is_ragdoll: false,
        }
    }

    fn prepared_dynamics() -> PreparedRagdollDynamics {
        PreparedRagdollDynamics {
            inverse_mass: std::array::from_fn(|index| {
                0.25 + f32::from(u16::try_from(index).expect("24 parts fit in u16"))
            }),
            inverse_inertia: std::array::from_fn(|index| {
                let base = f32::from(u16::try_from(index).expect("24 parts fit in u16")) + 1.0;
                [base + 2.0, base, base + 1.0, -base]
            }),
        }
    }

    fn setup_inputs(object_is_bouncy: bool) -> RagdollSetupInputs {
        RagdollSetupInputs {
            object_is_bouncy,
            dynamics: prepared_dynamics(),
        }
    }

    fn expected_setup_loop_events() -> Vec<CollaboratorEvent> {
        let mut events = Vec::new();
        for part in SkeletonPartIndex::ALL.into_iter().skip(1) {
            events.push(CollaboratorEvent::Folded(RagdollFoldedCallSite::SetupPart(
                part,
            )));
            events.push(CollaboratorEvent::InstallInverseMassRead(part));
            events.push(CollaboratorEvent::InstallInverseInertiaRead(part));
        }
        events.push(CollaboratorEvent::Folded(
            RagdollFoldedCallSite::SetupComplete,
        ));
        events
    }

    #[test]
    fn part_indices_follow_the_exact_pdb_order() {
        assert_eq!(size_of::<SkeletonPartIndex>(), size_of::<i32>());
        for (index, part) in SkeletonPartIndex::ALL.into_iter().enumerate() {
            assert_eq!(part.index(), index);
            assert_eq!(SkeletonPartIndex::from_index(index), Some(part));
            let raw = i32::try_from(index).expect("24 skeleton parts fit in i32");
            assert_eq!(SkeletonPartIndex::from_raw(raw), Some(part));
        }
        assert_eq!(
            SkeletonPartIndex::from_raw(INVALID_SKELETON_PART_INDEX_RAW),
            None
        );
        assert_eq!(SkeletonPartIndex::from_raw(i32::MIN), None);
        assert_eq!(SkeletonPartIndex::from_raw(24), None);
        assert_eq!(SkeletonPartIndex::from_raw(i32::MAX), None);
        assert_eq!(SkeletonPartIndex::from_index(SKELETON_PART_COUNT), None);
        assert_eq!(SkeletonPartIndex::from_index(usize::MAX), None);
    }

    #[test]
    fn exact_final_word_snapshot_matches_the_reference() {
        let table = SkeletonCullingTable::debug_reference_ragdoll();

        assert_eq!(table.words(), &DEBUG_REFERENCE_RAGDOLL_CULLING_WORDS);
    }

    #[test]
    fn clear_sequence_has_136_unique_in_range_directed_coordinates() {
        let mut directed = BTreeSet::new();
        let mut execution_order_fingerprint = 0_u64;

        for &(first, second) in &RAGDOLL_CULLING_CLEAR_PAIRS {
            assert!(first.index() < SKELETON_PART_COUNT);
            assert!(second.index() < SKELETON_PART_COUNT);
            assert_ne!(first, second);
            assert!(directed.insert((first.index(), second.index())));
            assert!(directed.insert((second.index(), first.index())));
            execution_order_fingerprint = execution_order_fingerprint
                .wrapping_mul(257)
                .wrapping_add(((first.index() << 5) | second.index()) as u64);
        }

        assert_eq!(RAGDOLL_CULLING_CLEAR_PAIRS.len(), 68);
        assert_eq!(directed.len(), 136);
        assert_eq!(execution_order_fingerprint, 0x7994_4995_4b5e_daf6);
    }

    #[test]
    fn final_table_has_exact_counts_and_symmetric_bits() {
        let table = SkeletonCullingTable::debug_reference_ragdoll();
        let suppressed = table
            .words()
            .iter()
            .map(|word| word.count_ones())
            .sum::<u32>();

        assert_eq!(suppressed, 440);
        assert_eq!(
            SKELETON_PART_COUNT * SKELETON_PART_COUNT - suppressed as usize,
            136
        );
        for row in SkeletonPartIndex::ALL {
            for column in SkeletonPartIndex::ALL {
                assert_eq!(
                    table.pair_culling(row, column),
                    table.pair_culling(column, row)
                );
            }
        }
    }

    #[test]
    fn every_diagonal_bit_remains_suppressed() {
        let table = SkeletonCullingTable::debug_reference_ragdoll();

        for part in SkeletonPartIndex::ALL {
            assert_eq!(
                table.pair_culling(part, part),
                SkeletonPairCulling::Suppressed
            );
        }
    }

    #[test]
    fn packing_crosses_word_boundaries_without_padding_rows() {
        assert_eq!(
            packed_location(SkeletonPartIndex::Neck1, SkeletonPartIndex::RightHand),
            (0, 1 << 31)
        );
        assert_eq!(
            packed_location(SkeletonPartIndex::Neck1, SkeletonPartIndex::RightForearm),
            (1, 1)
        );
        assert_eq!(
            packed_location(SkeletonPartIndex::LeftArm, SkeletonPartIndex::RightHand),
            (3, 1 << 31)
        );
        assert_eq!(
            packed_location(SkeletonPartIndex::LeftArm, SkeletonPartIndex::RightForearm),
            (4, 1)
        );
    }

    #[test]
    fn zero_is_eligible_and_one_is_suppressed() {
        let table = SkeletonCullingTable::debug_reference_ragdoll();

        assert_eq!(
            table.pair_culling(SkeletonPartIndex::Neck1, SkeletonPartIndex::Spine3),
            SkeletonPairCulling::Eligible
        );
        assert_eq!(
            table.pair_culling(SkeletonPartIndex::Neck1, SkeletonPartIndex::Neck),
            SkeletonPairCulling::Suppressed
        );
    }

    #[test]
    fn normal_and_ragdoll_clear_sets_remain_distinct() {
        let ragdoll = SkeletonCullingTable::debug_reference_ragdoll();
        let mut normal = SkeletonCullingTable::all_suppressed();
        for &(first, second) in &NORMAL_CULLING_CLEAR_PAIRS {
            normal.clear_directed(first, second);
            normal.clear_directed(second, first);
        }

        assert_eq!(NORMAL_CULLING_CLEAR_PAIRS.len(), 6);
        assert_eq!(normal.words(), &DEBUG_REFERENCE_NORMAL_CULLING_WORDS);
        assert_eq!(
            normal.pair_culling(SkeletonPartIndex::Neck1, SkeletonPartIndex::Spine3),
            SkeletonPairCulling::Suppressed
        );
        assert_eq!(
            ragdoll.pair_culling(SkeletonPartIndex::Neck1, SkeletonPartIndex::Spine3),
            SkeletonPairCulling::Eligible
        );
    }

    #[test]
    fn plain_setup_preserves_native_order_ranges_and_part_zero_exclusions() {
        let mut state = sentinel_setup_state();
        let before = state.clone();
        let inputs = setup_inputs(false);
        let mut collaborator = RecordingCollaborator::new(&[0.01], &[0.02], &[]);
        collaborator.joint_limit_values = [true].into();
        collaborator.install_inverse_mass_values = [true; SKELETON_PART_COUNT - 1].into();
        collaborator.install_inverse_inertia_values = [true; SKELETON_PART_COUNT - 1].into();

        set_up_ragdoll(&mut state, &inputs, &mut collaborator);

        assert_eq!(
            &collaborator.events[..11],
            &[
                CollaboratorEvent::JointLimitsRead,
                CollaboratorEvent::JointLimits,
                CollaboratorEvent::BouncyCheatRead,
                CollaboratorEvent::RestitutionRead(RagdollRestitutionSource::BodyAttribute),
                CollaboratorEvent::RestitutionWrite(
                    RagdollRestitutionTarget::Body,
                    0.01_f32.to_bits()
                ),
                CollaboratorEvent::RestitutionRead(RagdollRestitutionSource::HeadAttribute),
                CollaboratorEvent::RestitutionWrite(
                    RagdollRestitutionTarget::Head,
                    0.02_f32.to_bits()
                ),
                CollaboratorEvent::AngularDragRead,
                CollaboratorEvent::LinearDragRead,
                CollaboratorEvent::DragWritten(RagdollDragTarget::Angular, 0x41ef_ffff),
                CollaboratorEvent::DragWritten(RagdollDragTarget::Linear, 0),
            ]
        );
        let folded = collaborator
            .events
            .iter()
            .skip(11)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(folded, expected_setup_loop_events());

        assert_eq!(state.assembly_culling_table, state.ragdoll_culling_table);
        assert_eq!(state.assembly_group_id, RAGDOLL_ASSEMBLY_GROUP);
        assert_eq!(state.ragdoll_body_restitution.to_bits(), 0.01_f32.to_bits());
        assert_eq!(state.ragdoll_head_restitution.to_bits(), 0.02_f32.to_bits());
        assert!(!state.volume_disable_count_active);
        assert!(state.proxy_volume_enabled);
        assert!(state.is_ragdoll);

        let root = SkeletonPartIndex::SkateboardRoot.index();
        assert_eq!(state.volumes[root], before.volumes[root]);
        assert_eq!(state.parts[root].material, before.parts[root].material);
        assert_eq!(
            state.parts[root].inertia.inverse_mass.to_bits(),
            before.parts[root].inertia.inverse_mass.to_bits()
        );
        assert_eq!(
            state.parts[root].inertia.inverse_inertia.map(f32::to_bits),
            before.parts[root].inertia.inverse_inertia.map(f32::to_bits)
        );
        assert_eq!(
            state.parts[root].inertia.spherical.to_bits(),
            before.parts[root].inertia.spherical.to_bits()
        );

        for part_index in SkeletonPartIndex::ALL {
            let index = part_index.index();
            assert_eq!(state.parts[index].user_data, RAGDOLL_ASSEMBLY_GROUP);
            assert_eq!(
                state.parts[index].inertia.angular_drag.to_bits(),
                0x41ef_ffff
            );
            assert_eq!(state.parts[index].inertia.linear_drag.to_bits(), 0);
            if index == root {
                continue;
            }
            assert_eq!(state.volumes[index].disable_count, 0);
            assert_eq!(state.volumes[index].flags, 0xa1);
            assert_eq!(
                state.volumes[index].group_id,
                before.volumes[index].group_id
            );
            assert_eq!(
                state.parts[index].inertia.inverse_mass.to_bits(),
                inputs.dynamics.inverse_mass[index].to_bits()
            );
            assert_eq!(
                state.parts[index].inertia.inverse_inertia.map(f32::to_bits),
                inputs.dynamics.inverse_inertia[index].map(f32::to_bits)
            );
            assert_eq!(
                state.parts[index].inertia.spherical.to_bits(),
                (1.0 / (f32::from(u16::try_from(index).expect("24 parts fit in u16")) + 1.0))
                    .to_bits()
            );
            let expected_material = if [1, 3, 7].contains(&index) {
                SkeletonMaterialSlot::RagdollHead
            } else {
                SkeletonMaterialSlot::RagdollBody
            };
            assert_eq!(state.parts[index].material, expected_material);
        }
    }

    #[test]
    fn local_bouncy_byte_suppresses_cheat_read_and_reads_restitution_twice() {
        let mut state = sentinel_setup_state();
        let inputs = setup_inputs(true);
        let mut collaborator = RecordingCollaborator::new(&[], &[], &[0.75, 0.625]);

        set_up_ragdoll(&mut state, &inputs, &mut collaborator);

        assert_eq!(
            &collaborator.events[..5],
            &[
                CollaboratorEvent::JointLimitsRead,
                CollaboratorEvent::RestitutionRead(RagdollRestitutionSource::BouncyCVar),
                CollaboratorEvent::RestitutionWrite(
                    RagdollRestitutionTarget::Head,
                    0.75_f32.to_bits()
                ),
                CollaboratorEvent::RestitutionRead(RagdollRestitutionSource::BouncyCVar),
                CollaboratorEvent::RestitutionWrite(
                    RagdollRestitutionTarget::Body,
                    0.625_f32.to_bits()
                ),
            ]
        );
        assert!(
            !collaborator
                .events
                .contains(&CollaboratorEvent::BouncyCheatRead)
        );
        assert_eq!(state.ragdoll_head_restitution.to_bits(), 0.75_f32.to_bits());
        assert_eq!(
            state.ragdoll_body_restitution.to_bits(),
            0.625_f32.to_bits()
        );
    }

    #[test]
    fn cheat_bouncy_path_reads_cheat_then_restitution_head_and_body() {
        let mut state = sentinel_setup_state();
        let inputs = setup_inputs(false);
        let mut collaborator = RecordingCollaborator::new(&[], &[], &[0.75, 0.625]);
        collaborator.bouncy_cheat_values = [true].into();

        set_up_ragdoll(&mut state, &inputs, &mut collaborator);

        assert_eq!(
            &collaborator.events[..6],
            &[
                CollaboratorEvent::JointLimitsRead,
                CollaboratorEvent::BouncyCheatRead,
                CollaboratorEvent::RestitutionRead(RagdollRestitutionSource::BouncyCVar),
                CollaboratorEvent::RestitutionWrite(
                    RagdollRestitutionTarget::Head,
                    0.75_f32.to_bits()
                ),
                CollaboratorEvent::RestitutionRead(RagdollRestitutionSource::BouncyCVar),
                CollaboratorEvent::RestitutionWrite(
                    RagdollRestitutionTarget::Body,
                    0.625_f32.to_bits()
                ),
            ]
        );
    }

    #[test]
    fn disabled_mass_and_inertia_controls_preserve_existing_values() {
        let mut state = sentinel_setup_state();
        let before = state.clone();
        let inputs = setup_inputs(false);
        let mut collaborator = RecordingCollaborator::new(&[0.01], &[0.01], &[]);

        set_up_ragdoll(&mut state, &inputs, &mut collaborator);

        for index in 0..SKELETON_PART_COUNT {
            assert_eq!(
                state.parts[index].inertia.inverse_mass.to_bits(),
                before.parts[index].inertia.inverse_mass.to_bits()
            );
            assert_eq!(
                state.parts[index].inertia.inverse_inertia.map(f32::to_bits),
                before.parts[index]
                    .inertia
                    .inverse_inertia
                    .map(f32::to_bits)
            );
            assert_eq!(
                state.parts[index].inertia.spherical.to_bits(),
                before.parts[index].inertia.spherical.to_bits()
            );
        }
    }

    #[test]
    fn per_part_mass_and_inertia_gates_are_live_and_independent() {
        let mut state = sentinel_setup_state();
        let before = state.clone();
        let inputs = setup_inputs(false);
        let mass_enabled = (1..SKELETON_PART_COUNT)
            .map(|index| index % 2 == 0)
            .collect::<Vec<_>>();
        let inertia_enabled = (1..SKELETON_PART_COUNT)
            .map(|index| index % 3 == 0)
            .collect::<Vec<_>>();
        let mut collaborator = RecordingCollaborator::new(&[0.01], &[0.01], &[]);
        collaborator.install_inverse_mass_values = mass_enabled.iter().copied().collect();
        collaborator.install_inverse_inertia_values = inertia_enabled.iter().copied().collect();

        set_up_ragdoll(&mut state, &inputs, &mut collaborator);

        for index in 1..SKELETON_PART_COUNT {
            let expected_mass = if mass_enabled[index - 1] {
                inputs.dynamics.inverse_mass[index]
            } else {
                before.parts[index].inertia.inverse_mass
            };
            assert_eq!(
                state.parts[index].inertia.inverse_mass.to_bits(),
                expected_mass.to_bits()
            );

            if inertia_enabled[index - 1] {
                assert_eq!(
                    state.parts[index].inertia.inverse_inertia.map(f32::to_bits),
                    inputs.dynamics.inverse_inertia[index].map(f32::to_bits)
                );
            } else {
                assert_eq!(
                    state.parts[index].inertia.inverse_inertia.map(f32::to_bits),
                    before.parts[index]
                        .inertia
                        .inverse_inertia
                        .map(f32::to_bits)
                );
                assert_eq!(
                    state.parts[index].inertia.spherical.to_bits(),
                    before.parts[index].inertia.spherical.to_bits()
                );
            }
        }
    }

    #[test]
    fn ppc_minimum_preserves_tie_and_unordered_branch_behavior() {
        assert_eq!(
            reciprocal_ppc_selected_minimum([0.0, -0.0, 1.0, 99.0]).to_bits(),
            f32::NEG_INFINITY.to_bits()
        );
        assert_eq!(
            reciprocal_ppc_selected_minimum([1.0, -0.0, 0.0, 99.0]).to_bits(),
            f32::INFINITY.to_bits()
        );
        assert_eq!(
            reciprocal_ppc_selected_minimum([f32::NAN, 2.0, 3.0, 99.0]).to_bits(),
            0.5_f32.to_bits()
        );
        assert_eq!(
            reciprocal_ppc_selected_minimum([1.0, f32::NAN, 4.0, 99.0]).to_bits(),
            0.25_f32.to_bits()
        );
        assert!(reciprocal_ppc_selected_minimum([1.0, 2.0, f32::NAN, 99.0]).is_nan());
    }

    #[test]
    fn world_only_collision_is_a_separate_ordered_composition() {
        let mut state = sentinel_setup_state();
        let inputs = setup_inputs(false);
        let mut collaborator = RecordingCollaborator::new(&[0.01], &[0.01], &[]);
        set_up_ragdoll(&mut state, &inputs, &mut collaborator);
        let setup_event_count = collaborator.events.len();

        set_ragdoll_collide_with_world_only(&mut state, &mut collaborator);

        let world_only_events = &collaborator.events[setup_event_count..];
        let mut expected = SkeletonPartIndex::ALL
            .into_iter()
            .skip(1)
            .map(|part| CollaboratorEvent::Folded(RagdollFoldedCallSite::WorldOnlyPart(part)))
            .collect::<Vec<_>>();
        expected.push(CollaboratorEvent::Folded(
            RagdollFoldedCallSite::WorldOnlyRoot,
        ));
        assert_eq!(world_only_events, expected);
        assert_eq!(state.assembly_group_id, WORLD_ONLY_ASSEMBLY_GROUP);
        assert!(!state.proxy_volume_enabled);
        for part in state.parts {
            assert_eq!(part.user_data, WORLD_ONLY_ASSEMBLY_GROUP);
        }
        for volume in &state.volumes[1..] {
            assert_eq!(volume.group_id, WORLD_ONLY_VOLUME_GROUP);
            assert_eq!(volume.flags & 1, 1);
        }
        let root = &state.volumes[SkeletonPartIndex::SkateboardRoot.index()];
        assert_eq!(root.flags & 1, 0);
        assert_eq!(root.disable_count, 0);
    }

    #[test]
    fn world_only_helper_skips_nonzero_disable_counts() {
        let mut state = sentinel_setup_state();
        let skipped = SkeletonPartIndex::Neck;
        state.volumes[skipped.index()].flags = 0xa0;
        state.volumes[skipped.index()].group_id = 0xfeed_beef;
        state.volumes[skipped.index()].disable_count = 2;
        let mut collaborator = RecordingCollaborator::new(&[], &[], &[]);

        set_ragdoll_collide_with_world_only(&mut state, &mut collaborator);

        assert_eq!(state.volumes[skipped.index()].flags, 0xa0);
        assert_eq!(state.volumes[skipped.index()].group_id, 0xfeed_beef);
        assert_eq!(state.volumes[skipped.index()].disable_count, 2);
        assert!(!collaborator.events.contains(&CollaboratorEvent::Folded(
            RagdollFoldedCallSite::WorldOnlyPart(skipped)
        )));
    }
}
