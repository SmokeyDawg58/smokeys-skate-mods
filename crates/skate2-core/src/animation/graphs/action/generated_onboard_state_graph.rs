//! Read-only generated on-board `StateGraph` descriptor and selection seam.
//!
//! The matched build uses generated Action/Motion controllers by default, but
//! their ticks require concrete generated conditions, behaviours, hooks, and
//! state-update functions. This module therefore exposes metadata and selection
//! only. It never substitutes the dynamic controller for a selected generated
//! controller and never invokes an opaque guest update entry.

use crate::living_world_state_graph_preload::GENERATED_STATE_GRAPH_DELEGATE_BASENAMES;
use std::{collections::BTreeSet, error::Error, fmt};

pub const GENERATED_STATE_DESC_SIZE: usize = 0x20;
pub const GENERATED_NULL_INDEX: u16 = u16::MAX;
pub const GENERATED_ARRAY_SECTION_SIZE: usize = 4;
pub const GENERATED_STATE_DESC_NAME_OFFSET: usize = 0x00;
pub const GENERATED_STATE_DESC_INTERRUPT_TYPE_OFFSET: usize = 0x04;
pub const GENERATED_STATE_DESC_ACTIVE_OFFSET: usize = 0x05;
pub const GENERATED_STATE_DESC_INTERRUPT_TARGET_OFFSET: usize = 0x06;
pub const GENERATED_STATE_DESC_PARENT_OFFSET: usize = 0x08;
pub const GENERATED_STATE_DESC_EXPRESSION_OFFSET: usize = 0x0a;
pub const GENERATED_STATE_DESC_CHILDREN_OFFSET: usize = 0x0c;
pub const GENERATED_STATE_DESC_BEHAVIOURS_OFFSET: usize = 0x10;
pub const GENERATED_STATE_DESC_TRANSITIONS_OFFSET: usize = 0x14;
pub const GENERATED_STATE_DESC_UPDATE_OFFSET: usize = 0x18;
pub const GENERATED_STATE_DESC_TRAILING_OFFSET: usize = 0x1c;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GeneratedOnboardKind {
    Action,
    Motion,
}

impl GeneratedOnboardKind {
    #[must_use]
    pub const fn metadata(self) -> GeneratedOnboardMetadata {
        match self {
            Self::Action => GeneratedOnboardMetadata {
                basename: b"ActionGraph_OnBoard.xml",
                root_name: b"Action",
                state_count: 224,
                state_accessor_address: 0x8292_8268,
                state_table_address: 0x8217_66b0,
                controller_allocation_size: 0x2e4,
                raw_table_sha256: "D410C2994AD587E734155BF0CD438338D39E520239B318B1477F4F72A2295D70",
                semantic_sha256: "5E868C5FBBD6274145D0D06398F30F697D77E5EF2F15DB0230968040322803E2",
                path_set_sha256: "5701D5FF92B2CD20E7F66B09660C0FDA6109D3754460941ED5AFA76FEA4DAF9E",
            },
            Self::Motion => GeneratedOnboardMetadata {
                basename: b"MotionGraph_OnBoard.xml",
                root_name: b"Motion",
                state_count: 1_354,
                state_accessor_address: 0x8292_a538,
                state_table_address: 0x8219_5fa0,
                controller_allocation_size: 0x39c,
                raw_table_sha256: "A070D6E419C8B34EC08CB50E27F0947826005678CAD29B98B96C0563CE6D526F",
                semantic_sha256: "FD631D7899DB0F14DB4E61A6ACAE0EE43A8B06B608D5783CE386AB1118558D97",
                path_set_sha256: "700A08158046CBB88FB851A85B643C9D38C80F5BBE219A9A24A069D7ED86D5A1",
            },
        }
    }

    /// Constructs a domain-tagged generated index after applying the exact
    /// Action/Motion State accessor bound.
    ///
    /// # Errors
    ///
    /// Rejects Action indices at or above 224 and Motion indices at or above
    /// 1,354. Native asserts and can continue; rejection is checked-host policy.
    pub const fn checked_index(
        self,
        index: u16,
    ) -> Result<GeneratedStateIndex, GeneratedStateIndexError> {
        let count = self.metadata().state_count;
        if index < count {
            Ok(GeneratedStateIndex { kind: self, index })
        } else {
            Err(GeneratedStateIndexError {
                kind: self,
                index,
                state_count: count,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratedOnboardMetadata {
    pub basename: &'static [u8],
    pub root_name: &'static [u8],
    pub state_count: u16,
    pub state_accessor_address: u32,
    pub state_table_address: u32,
    pub controller_allocation_size: u32,
    pub raw_table_sha256: &'static str,
    pub semantic_sha256: &'static str,
    pub path_set_sha256: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GeneratedStateIndex {
    kind: GeneratedOnboardKind,
    index: u16,
}

impl GeneratedStateIndex {
    #[must_use]
    pub const fn kind(self) -> GeneratedOnboardKind {
        self.kind
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.index
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratedStateIndexError {
    pub kind: GeneratedOnboardKind,
    pub index: u16,
    pub state_count: u16,
}

impl fmt::Display for GeneratedStateIndexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "generated {:?} State index {} is outside count {}",
            self.kind, self.index, self.state_count
        )
    }
}

impl Error for GeneratedStateIndexError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GeneratedArraySection {
    pub count: u16,
    pub index: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GeneratedStateDesc {
    pub name_guest_address: u32,
    pub interrupt_type_raw: u8,
    pub active_raw: u8,
    pub interrupt_target: u16,
    pub parent: u16,
    pub expression: u16,
    pub children: GeneratedArraySection,
    pub behaviours: GeneratedArraySection,
    pub transitions: GeneratedArraySection,
    /// Opaque metadata only. This boundary never invokes the guest entry.
    pub update_guest_address: u32,
    /// PDB-unassigned trailing bytes required by the exact `0x20` stride.
    pub trailing_bytes: [u8; 4],
}

impl GeneratedStateDesc {
    #[must_use]
    pub fn decode_guest_be(bytes: [u8; GENERATED_STATE_DESC_SIZE]) -> Self {
        Self {
            name_guest_address: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            interrupt_type_raw: bytes[0x04],
            active_raw: bytes[0x05],
            interrupt_target: u16::from_be_bytes([bytes[0x06], bytes[0x07]]),
            parent: u16::from_be_bytes([bytes[0x08], bytes[0x09]]),
            expression: u16::from_be_bytes([bytes[0x0a], bytes[0x0b]]),
            children: decode_section(&bytes, 0x0c),
            behaviours: decode_section(&bytes, 0x10),
            transitions: decode_section(&bytes, 0x14),
            update_guest_address: u32::from_be_bytes([
                bytes[0x18],
                bytes[0x19],
                bytes[0x1a],
                bytes[0x1b],
            ]),
            trailing_bytes: [bytes[0x1c], bytes[0x1d], bytes[0x1e], bytes[0x1f]],
        }
    }
}

fn decode_section(bytes: &[u8; GENERATED_STATE_DESC_SIZE], offset: usize) -> GeneratedArraySection {
    GeneratedArraySection {
        count: u16::from_be_bytes([bytes[offset], bytes[offset + 1]]),
        index: u16::from_be_bytes([bytes[offset + 2], bytes[offset + 3]]),
    }
}

/// Host-supplied access to legally mounted descriptor bytes and resolved names.
/// The core deliberately contains no generated table contents.
pub trait GeneratedOnboardDescriptorProvider {
    type Error;

    /// Returns one exact 32-byte big-endian guest descriptor record.
    ///
    /// # Errors
    ///
    /// Returns the provider's mounted-data failure.
    fn state_desc_bytes(
        &self,
        index: GeneratedStateIndex,
    ) -> Result<[u8; GENERATED_STATE_DESC_SIZE], Self::Error>;

    /// Resolves the descriptor's guest C-string name to its byte contents,
    /// excluding the terminator. Core rejects any returned embedded NUL.
    ///
    /// # Errors
    ///
    /// Returns the provider's mounted-data failure.
    fn state_name(
        &self,
        index: GeneratedStateIndex,
        name_guest_address: u32,
    ) -> Result<Vec<u8>, Self::Error>;
}

pub struct GeneratedStateDescView {
    pub index: GeneratedStateIndex,
    pub name: Vec<u8>,
    pub descriptor: GeneratedStateDesc,
}

#[derive(Debug)]
pub enum GeneratedOnboardDescriptorError<E> {
    Index(GeneratedStateIndexError),
    Provider(E),
    StateNameContainsNul {
        index: GeneratedStateIndex,
        name: Vec<u8>,
    },
    ParentCycle {
        kind: GeneratedOnboardKind,
        repeated_index: u16,
    },
}

impl<E: fmt::Display> fmt::Display for GeneratedOnboardDescriptorError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Index(error) => error.fmt(formatter),
            Self::Provider(error) => {
                write!(formatter, "generated descriptor provider failed: {error}")
            }
            Self::StateNameContainsNul { index, .. } => write!(
                formatter,
                "generated {:?} State name at index {} contains a NUL byte",
                index.kind(),
                index.get()
            ),
            Self::ParentCycle {
                kind,
                repeated_index,
            } => write!(
                formatter,
                "generated {kind:?} parent chain repeats State index {repeated_index}"
            ),
        }
    }
}

impl<E: Error + 'static> Error for GeneratedOnboardDescriptorError<E> {}

pub struct GeneratedOnboardDescriptorSource<P> {
    provider: P,
}

impl<P> GeneratedOnboardDescriptorSource<P> {
    #[must_use]
    pub const fn new(provider: P) -> Self {
        Self { provider }
    }

    #[must_use]
    pub const fn provider(&self) -> &P {
        &self.provider
    }
}

impl<P: GeneratedOnboardDescriptorProvider> GeneratedOnboardDescriptorSource<P> {
    /// Reads one checked generated descriptor and resolves its name.
    ///
    /// # Errors
    ///
    /// Returns an exact kind-specific bounds error or provider failure.
    pub fn state_desc(
        &self,
        kind: GeneratedOnboardKind,
        raw_index: u16,
    ) -> Result<GeneratedStateDescView, GeneratedOnboardDescriptorError<P::Error>> {
        let index = kind
            .checked_index(raw_index)
            .map_err(GeneratedOnboardDescriptorError::Index)?;
        let bytes = self
            .provider
            .state_desc_bytes(index)
            .map_err(GeneratedOnboardDescriptorError::Provider)?;
        let descriptor = GeneratedStateDesc::decode_guest_be(bytes);
        let name = self
            .provider
            .state_name(index, descriptor.name_guest_address)
            .map_err(GeneratedOnboardDescriptorError::Provider)?;
        if name.contains(&0) {
            return Err(GeneratedOnboardDescriptorError::StateNameContainsNul { index, name });
        }
        Ok(GeneratedStateDescView {
            index,
            name,
            descriptor,
        })
    }

    /// Builds the complete root-to-State path by following the descriptor's
    /// same-kind `parent` indices until the `0xffff` root sentinel.
    ///
    /// # Errors
    ///
    /// Returns an exact kind-specific bounds error or provider failure.
    pub fn state_path(
        &self,
        kind: GeneratedOnboardKind,
        raw_index: u16,
    ) -> Result<GeneratedStatePath, GeneratedOnboardDescriptorError<P::Error>> {
        let mut current = raw_index;
        let mut visited = BTreeSet::new();
        let mut components = Vec::new();
        loop {
            kind.checked_index(current)
                .map_err(GeneratedOnboardDescriptorError::Index)?;
            if !visited.insert(current) {
                return Err(GeneratedOnboardDescriptorError::ParentCycle {
                    kind,
                    repeated_index: current,
                });
            }
            let view = self.state_desc(kind, current)?;
            components.push(view.name);
            if view.descriptor.parent == GENERATED_NULL_INDEX {
                break;
            }
            current = view.descriptor.parent;
        }
        components.reverse();
        Ok(GeneratedStatePath { kind, components })
    }

    /// Searches generated-index order for the first exact root-to-State
    /// component sequence.
    ///
    /// # Errors
    ///
    /// Returns the first provider failure encountered during the bounded scan.
    pub fn find_state_path(
        &self,
        kind: GeneratedOnboardKind,
        components: &[Vec<u8>],
    ) -> Result<Option<GeneratedStateIndex>, GeneratedOnboardDescriptorError<P::Error>> {
        for raw_index in 0..kind.metadata().state_count {
            let index = GeneratedStateIndex {
                kind,
                index: raw_index,
            };
            let candidate = self.state_path(kind, raw_index)?;
            if candidate.components == components {
                return Ok(Some(index));
            }
        }
        Ok(None)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedStatePath {
    pub kind: GeneratedOnboardKind,
    pub components: Vec<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratedOnboardEnablement {
    delegate_raw: [u8; 20],
}

impl GeneratedOnboardEnablement {
    #[must_use]
    pub const fn from_delegate_raw(delegate_raw: [u8; 20]) -> Self {
        Self { delegate_raw }
    }

    #[must_use]
    pub const fn onboard(action_raw: u8, motion_raw: u8) -> Self {
        let mut delegate_raw = [0; 20];
        delegate_raw[0] = action_raw;
        delegate_raw[1] = motion_raw;
        Self { delegate_raw }
    }

    #[must_use]
    pub const fn delegate_raw(self, index: usize) -> Option<u8> {
        if index < self.delegate_raw.len() {
            Some(self.delegate_raw[index])
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OnboardControllerSelection {
    GeneratedOnboard(GeneratedOnboardKind),
    GeneratedOther { delegate_index: usize },
    DynamicFallback,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedRequiredMiss {
    pub basename: Vec<u8>,
}

impl fmt::Display for GeneratedRequiredMiss {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "generated StateGraph was required but basename {} had no enabled generated match",
            String::from_utf8_lossy(&self.basename)
        )
    }
}

impl Error for GeneratedRequiredMiss {}

/// Reconstructs `AbstractHierarchicalController::Create`'s generated choice.
/// The native slash precedence is preserved: the final forward slash wins if
/// any exists; only a path with no forward slash is searched for a final
/// backslash. Input is treated as a C-string prefix ending at the first NUL;
/// basename matching inside that prefix is byte-exact and case-sensitive.
///
/// # Errors
///
/// Returns a generated-required miss instead of constructing Dynamic when the
/// matched generated branch is unavailable.
pub fn select_onboard_controller(
    filename: &[u8],
    allow_generated: bool,
    require_generated: bool,
    enablement: GeneratedOnboardEnablement,
) -> Result<OnboardControllerSelection, GeneratedRequiredMiss> {
    let basename = final_basename(filename);
    if allow_generated {
        for (index, candidate) in GENERATED_STATE_GRAPH_DELEGATE_BASENAMES.iter().enumerate() {
            if basename == candidate.as_bytes()
                && enablement.delegate_raw(index).is_some_and(|raw| raw != 0)
            {
                return Ok(match index {
                    0 => OnboardControllerSelection::GeneratedOnboard(GeneratedOnboardKind::Action),
                    1 => OnboardControllerSelection::GeneratedOnboard(GeneratedOnboardKind::Motion),
                    _ => OnboardControllerSelection::GeneratedOther {
                        delegate_index: index,
                    },
                });
            }
        }
    }
    if require_generated {
        Err(GeneratedRequiredMiss {
            basename: basename.to_vec(),
        })
    } else {
        Ok(OnboardControllerSelection::DynamicFallback)
    }
}

fn final_basename(filename: &[u8]) -> &[u8] {
    let c_bytes = filename
        .iter()
        .position(|byte| *byte == 0)
        .map_or(filename, |nul| &filename[..nul]);
    if let Some(separator) = c_bytes.iter().rposition(|byte| *byte == b'/') {
        &c_bytes[separator + 1..]
    } else if let Some(separator) = c_bytes.iter().rposition(|byte| *byte == b'\\') {
        &c_bytes[separator + 1..]
    } else {
        c_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratedOnboardUnsupported {
    pub kind: GeneratedOnboardKind,
}

impl fmt::Display for GeneratedOnboardUnsupported {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "generated {:?} on-board StateGraph ticking is unsupported",
            self.kind
        )
    }
}

impl Error for GeneratedOnboardUnsupported {}

/// Explicit descriptor-only tick boundary. It never dispatches Dynamic.
///
/// # Errors
///
/// Always returns `GeneratedOnboardUnsupported` until the concrete generated
/// factories, direct helpers, state updates, and collaborators are recovered.
pub const fn tick_generated_onboard(
    kind: GeneratedOnboardKind,
    _dt: f32,
) -> Result<(), GeneratedOnboardUnsupported> {
    Err(GeneratedOnboardUnsupported { kind })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, collections::BTreeMap, convert::Infallible};

    #[derive(Default)]
    struct FixtureProvider {
        records: BTreeMap<(GeneratedOnboardKind, u16), ([u8; 32], Vec<u8>)>,
        calls: Cell<usize>,
    }

    impl GeneratedOnboardDescriptorProvider for FixtureProvider {
        type Error = Infallible;

        fn state_desc_bytes(&self, index: GeneratedStateIndex) -> Result<[u8; 32], Infallible> {
            self.calls.set(self.calls.get() + 1);
            Ok(self.records[&(index.kind(), index.get())].0)
        }

        fn state_name(
            &self,
            index: GeneratedStateIndex,
            _name_guest_address: u32,
        ) -> Result<Vec<u8>, Infallible> {
            self.calls.set(self.calls.get() + 1);
            Ok(self.records[&(index.kind(), index.get())].1.clone())
        }
    }

    fn descriptor_fixture() -> [u8; 32] {
        descriptor_with_parent(0x5678)
    }

    fn descriptor_with_parent(parent: u16) -> [u8; 32] {
        let mut bytes = [
            0x82, 0x17, 0x66, 0xb0, 0x02, 0x7f, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0x00, 0x03,
            0x11, 0x22, 0x00, 0x04, 0x33, 0x44, 0x00, 0x05, 0x55, 0x66, 0x82, 0x8e, 0xcb, 0xc0,
            0xde, 0xad, 0xbe, 0xef,
        ];
        bytes[GENERATED_STATE_DESC_PARENT_OFFSET..GENERATED_STATE_DESC_PARENT_OFFSET + 2]
            .copy_from_slice(&parent.to_be_bytes());
        bytes
    }

    fn insert_record(
        provider: &mut FixtureProvider,
        kind: GeneratedOnboardKind,
        index: u16,
        parent: u16,
        name: &[u8],
    ) {
        provider.records.insert(
            (kind, index),
            (descriptor_with_parent(parent), name.to_vec()),
        );
    }

    #[test]
    fn descriptor_layout_and_big_endian_scalars_are_exact() {
        assert_eq!(GENERATED_ARRAY_SECTION_SIZE, 4);
        assert_eq!(GENERATED_STATE_DESC_SIZE, 0x20);
        assert_eq!(GENERATED_STATE_DESC_NAME_OFFSET, 0x00);
        assert_eq!(GENERATED_STATE_DESC_INTERRUPT_TYPE_OFFSET, 0x04);
        assert_eq!(GENERATED_STATE_DESC_ACTIVE_OFFSET, 0x05);
        assert_eq!(GENERATED_STATE_DESC_INTERRUPT_TARGET_OFFSET, 0x06);
        assert_eq!(GENERATED_STATE_DESC_PARENT_OFFSET, 0x08);
        assert_eq!(GENERATED_STATE_DESC_EXPRESSION_OFFSET, 0x0a);
        assert_eq!(GENERATED_STATE_DESC_CHILDREN_OFFSET, 0x0c);
        assert_eq!(GENERATED_STATE_DESC_BEHAVIOURS_OFFSET, 0x10);
        assert_eq!(GENERATED_STATE_DESC_TRANSITIONS_OFFSET, 0x14);
        assert_eq!(GENERATED_STATE_DESC_UPDATE_OFFSET, 0x18);
        assert_eq!(GENERATED_STATE_DESC_TRAILING_OFFSET, 0x1c);
        let descriptor = GeneratedStateDesc::decode_guest_be(descriptor_fixture());
        assert_eq!(descriptor.name_guest_address, 0x8217_66b0);
        assert_eq!(
            (descriptor.interrupt_type_raw, descriptor.active_raw),
            (2, 0x7f)
        );
        assert_eq!(
            (descriptor.interrupt_target, descriptor.parent),
            (0x1234, 0x5678)
        );
        assert_eq!(descriptor.expression, 0x9abc);
        assert_eq!(
            descriptor.children,
            GeneratedArraySection {
                count: 3,
                index: 0x1122
            }
        );
        assert_eq!(
            descriptor.behaviours,
            GeneratedArraySection {
                count: 4,
                index: 0x3344
            }
        );
        assert_eq!(
            descriptor.transitions,
            GeneratedArraySection {
                count: 5,
                index: 0x5566
            }
        );
        assert_eq!(descriptor.update_guest_address, 0x828e_cbc0);
        assert_eq!(descriptor.trailing_bytes, [0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(GENERATED_NULL_INDEX, 0xffff);

        let mut empty_bytes = [0; GENERATED_STATE_DESC_SIZE];
        empty_bytes[0x0e..0x10].copy_from_slice(&0xffffu16.to_be_bytes());
        let sentinel_empty = GeneratedStateDesc::decode_guest_be(empty_bytes);
        let zero_empty = GeneratedStateDesc::decode_guest_be([0; GENERATED_STATE_DESC_SIZE]);
        assert_eq!(
            sentinel_empty.children,
            GeneratedArraySection {
                count: 0,
                index: 0xffff
            }
        );
        assert_eq!(
            zero_empty.children,
            GeneratedArraySection { count: 0, index: 0 }
        );
        assert_ne!(sentinel_empty.children, zero_empty.children);
    }

    #[test]
    fn kind_metadata_freezes_exact_accessors_bounds_and_digests() {
        let action = GeneratedOnboardKind::Action.metadata();
        assert_eq!(
            (action.state_count, action.state_table_address),
            (224, 0x8217_66b0)
        );
        assert_eq!(
            (action.state_accessor_address, action.root_name),
            (0x8292_8268, b"Action".as_slice())
        );
        assert_eq!(action.controller_allocation_size, 0x2e4);
        assert_eq!(
            action.raw_table_sha256,
            "D410C2994AD587E734155BF0CD438338D39E520239B318B1477F4F72A2295D70"
        );
        assert_eq!(
            action.semantic_sha256,
            "5E868C5FBBD6274145D0D06398F30F697D77E5EF2F15DB0230968040322803E2"
        );
        assert_eq!(
            action.path_set_sha256,
            "5701D5FF92B2CD20E7F66B09660C0FDA6109D3754460941ED5AFA76FEA4DAF9E"
        );

        let motion = GeneratedOnboardKind::Motion.metadata();
        assert_eq!(
            (motion.state_count, motion.state_table_address),
            (1_354, 0x8219_5fa0)
        );
        assert_eq!(
            (motion.state_accessor_address, motion.root_name),
            (0x8292_a538, b"Motion".as_slice())
        );
        assert_eq!(motion.controller_allocation_size, 0x39c);
        assert_eq!(
            motion.raw_table_sha256,
            "A070D6E419C8B34EC08CB50E27F0947826005678CAD29B98B96C0563CE6D526F"
        );
        assert_eq!(
            motion.semantic_sha256,
            "FD631D7899DB0F14DB4E61A6ACAE0EE43A8B06B608D5783CE386AB1118558D97"
        );
        assert_eq!(
            motion.path_set_sha256,
            "700A08158046CBB88FB851A85B643C9D38C80F5BBE219A9A24A069D7ED86D5A1"
        );
    }

    #[test]
    fn checked_bounds_precede_every_provider_call() {
        let source = GeneratedOnboardDescriptorSource::new(FixtureProvider::default());
        assert!(matches!(
            source.state_desc(GeneratedOnboardKind::Action, 224),
            Err(GeneratedOnboardDescriptorError::Index(
                GeneratedStateIndexError {
                    state_count: 224,
                    ..
                }
            ))
        ));
        assert!(matches!(
            source.state_path(GeneratedOnboardKind::Motion, 1_354),
            Err(GeneratedOnboardDescriptorError::Index(
                GeneratedStateIndexError {
                    state_count: 1_354,
                    ..
                }
            ))
        ));
        assert_eq!(source.provider().calls.get(), 0);
        assert!(GeneratedOnboardKind::Action.checked_index(223).is_ok());
        assert!(GeneratedOnboardKind::Motion.checked_index(1_353).is_ok());
        assert_ne!(
            GeneratedOnboardKind::Action.checked_index(2).unwrap(),
            GeneratedOnboardKind::Motion.checked_index(2).unwrap()
        );
    }

    #[test]
    fn provider_boundary_decodes_fixture_and_builds_root_path() {
        let mut provider = FixtureProvider::default();
        provider.records.insert(
            (GeneratedOnboardKind::Action, 0),
            (
                descriptor_with_parent(GENERATED_NULL_INDEX),
                b"Action".to_vec(),
            ),
        );
        let source = GeneratedOnboardDescriptorSource::new(provider);
        let view = source.state_desc(GeneratedOnboardKind::Action, 0).unwrap();
        assert_eq!(view.name, b"Action");
        assert_eq!(view.index.get(), 0);
        assert_eq!(view.descriptor.children.count, 3);
        assert_eq!(
            source.state_path(GeneratedOnboardKind::Action, 0).unwrap(),
            GeneratedStatePath {
                kind: GeneratedOnboardKind::Action,
                components: vec![b"Action".to_vec()]
            }
        );
    }

    #[test]
    fn parent_chain_reverses_to_root_and_find_compares_components() {
        let mut provider = FixtureProvider::default();
        insert_record(
            &mut provider,
            GeneratedOnboardKind::Action,
            0,
            GENERATED_NULL_INDEX,
            b"Action",
        );
        insert_record(
            &mut provider,
            GeneratedOnboardKind::Action,
            1,
            0,
            b"OnBoard",
        );
        insert_record(
            &mut provider,
            GeneratedOnboardKind::Action,
            2,
            1,
            b"Rolling",
        );
        let source = GeneratedOnboardDescriptorSource::new(provider);
        let components = vec![b"Action".to_vec(), b"OnBoard".to_vec(), b"Rolling".to_vec()];
        assert_eq!(
            source.state_path(GeneratedOnboardKind::Action, 2).unwrap(),
            GeneratedStatePath {
                kind: GeneratedOnboardKind::Action,
                components: components.clone()
            }
        );
        assert_eq!(
            source
                .find_state_path(GeneratedOnboardKind::Action, &components)
                .unwrap(),
            Some(GeneratedOnboardKind::Action.checked_index(2).unwrap())
        );
    }

    #[test]
    fn out_of_range_parent_is_rejected_before_another_provider_call() {
        let mut provider = FixtureProvider::default();
        insert_record(
            &mut provider,
            GeneratedOnboardKind::Action,
            0,
            224,
            b"Action",
        );
        let source = GeneratedOnboardDescriptorSource::new(provider);
        assert!(matches!(
            source.state_path(GeneratedOnboardKind::Action, 0),
            Err(GeneratedOnboardDescriptorError::Index(
                GeneratedStateIndexError {
                    kind: GeneratedOnboardKind::Action,
                    index: 224,
                    state_count: 224
                }
            ))
        ));
        assert_eq!(source.provider().calls.get(), 2);
    }

    #[test]
    fn self_and_indirect_parent_cycles_stop_before_repeated_provider_calls() {
        let mut self_cycle = FixtureProvider::default();
        insert_record(
            &mut self_cycle,
            GeneratedOnboardKind::Action,
            0,
            0,
            b"Action",
        );
        let source = GeneratedOnboardDescriptorSource::new(self_cycle);
        assert!(matches!(
            source.state_path(GeneratedOnboardKind::Action, 0),
            Err(GeneratedOnboardDescriptorError::ParentCycle {
                kind: GeneratedOnboardKind::Action,
                repeated_index: 0
            })
        ));
        assert_eq!(source.provider().calls.get(), 2);

        let mut indirect_cycle = FixtureProvider::default();
        insert_record(
            &mut indirect_cycle,
            GeneratedOnboardKind::Motion,
            10,
            11,
            b"Ten",
        );
        insert_record(
            &mut indirect_cycle,
            GeneratedOnboardKind::Motion,
            11,
            10,
            b"Eleven",
        );
        let source = GeneratedOnboardDescriptorSource::new(indirect_cycle);
        assert!(matches!(
            source.state_path(GeneratedOnboardKind::Motion, 10),
            Err(GeneratedOnboardDescriptorError::ParentCycle {
                kind: GeneratedOnboardKind::Motion,
                repeated_index: 10
            })
        ));
        assert_eq!(source.provider().calls.get(), 4);
    }

    #[test]
    fn resolved_names_with_nul_are_rejected_at_the_core_boundary() {
        let mut provider = FixtureProvider::default();
        insert_record(
            &mut provider,
            GeneratedOnboardKind::Action,
            0,
            GENERATED_NULL_INDEX,
            b"Act\0ion",
        );
        let source = GeneratedOnboardDescriptorSource::new(provider);
        assert!(matches!(
            source.state_desc(GeneratedOnboardKind::Action, 0),
            Err(GeneratedOnboardDescriptorError::StateNameContainsNul {
                index,
                name
            }) if index == GeneratedOnboardKind::Action.checked_index(0).unwrap()
                && name == b"Act\0ion"
        ));
        assert_eq!(source.provider().calls.get(), 2);
    }

    #[test]
    fn generated_selection_is_case_sensitive_and_strips_only_final_separator() {
        let both = GeneratedOnboardEnablement::onboard(1, 0xff);
        assert_eq!(
            select_onboard_controller(b"state/ActionGraph_OnBoard.xml", true, false, both),
            Ok(OnboardControllerSelection::GeneratedOnboard(
                GeneratedOnboardKind::Action
            ))
        );
        assert_eq!(
            select_onboard_controller(b"data\\state\\MotionGraph_OnBoard.xml", true, false, both),
            Ok(OnboardControllerSelection::GeneratedOnboard(
                GeneratedOnboardKind::Motion
            ))
        );
        assert_eq!(
            select_onboard_controller(b"state/actiongraph_onboard.xml", true, false, both),
            Ok(OnboardControllerSelection::DynamicFallback)
        );
        assert_eq!(
            select_onboard_controller(b"state/dir.ActionGraph_OnBoard.xml", true, false, both),
            Ok(OnboardControllerSelection::DynamicFallback)
        );
        assert_eq!(
            select_onboard_controller(b"a/b\\ActionGraph_OnBoard.xml", true, false, both),
            Ok(OnboardControllerSelection::DynamicFallback)
        );
        assert_eq!(
            select_onboard_controller(b"a\\b/ActionGraph_OnBoard.xml", true, false, both),
            Ok(OnboardControllerSelection::GeneratedOnboard(
                GeneratedOnboardKind::Action
            ))
        );
        assert_eq!(
            select_onboard_controller(
                b"ActionGraph_OnBoard.xml\0ignored/unknown.xml",
                true,
                false,
                both
            ),
            Ok(OnboardControllerSelection::GeneratedOnboard(
                GeneratedOnboardKind::Action
            ))
        );
    }

    #[test]
    fn every_enabled_generated_delegate_routes_without_dynamic_fallback() {
        let all_enabled = GeneratedOnboardEnablement::from_delegate_raw([1; 20]);
        for (index, basename) in GENERATED_STATE_GRAPH_DELEGATE_BASENAMES.iter().enumerate() {
            let expected = match index {
                0 => OnboardControllerSelection::GeneratedOnboard(GeneratedOnboardKind::Action),
                1 => OnboardControllerSelection::GeneratedOnboard(GeneratedOnboardKind::Motion),
                _ => OnboardControllerSelection::GeneratedOther {
                    delegate_index: index,
                },
            };
            assert_eq!(
                select_onboard_controller(basename.as_bytes(), true, false, all_enabled),
                Ok(expected),
                "delegate {index}: {basename}"
            );
        }

        let mut vehicle_only = [0; 20];
        vehicle_only[2] = 0x80;
        let vehicle_only = GeneratedOnboardEnablement::from_delegate_raw(vehicle_only);
        assert_eq!(
            select_onboard_controller(b"Vehicle.xml", true, false, vehicle_only),
            Ok(OnboardControllerSelection::GeneratedOther { delegate_index: 2 })
        );
        assert_eq!(
            select_onboard_controller(b"ActionGraph_OnBoard.xml", true, false, vehicle_only),
            Ok(OnboardControllerSelection::DynamicFallback)
        );
    }

    #[test]
    fn enable_allow_require_matrix_never_silently_substitutes_dynamic() {
        let disabled = GeneratedOnboardEnablement::onboard(0, 0);
        assert_eq!(
            select_onboard_controller(b"ActionGraph_OnBoard.xml", true, false, disabled),
            Ok(OnboardControllerSelection::DynamicFallback)
        );
        assert_eq!(
            select_onboard_controller(
                b"ActionGraph_OnBoard.xml",
                false,
                false,
                GeneratedOnboardEnablement::onboard(1, 1)
            ),
            Ok(OnboardControllerSelection::DynamicFallback)
        );
        assert!(matches!(
            select_onboard_controller(b"ActionGraph_OnBoard.xml", true, true, disabled),
            Err(GeneratedRequiredMiss { .. })
        ));
        assert!(matches!(
            select_onboard_controller(
                b"unknown.xml",
                true,
                true,
                GeneratedOnboardEnablement::onboard(1, 1)
            ),
            Err(GeneratedRequiredMiss { .. })
        ));
    }

    #[test]
    fn selected_generated_tick_is_explicitly_unsupported() {
        let selection = select_onboard_controller(
            b"ActionGraph_OnBoard.xml",
            true,
            false,
            GeneratedOnboardEnablement::onboard(1, 1),
        )
        .unwrap();
        assert_eq!(
            selection,
            OnboardControllerSelection::GeneratedOnboard(GeneratedOnboardKind::Action)
        );
        assert_eq!(
            tick_generated_onboard(GeneratedOnboardKind::Action, 1.0 / 60.0),
            Err(GeneratedOnboardUnsupported {
                kind: GeneratedOnboardKind::Action
            })
        );
    }
}
