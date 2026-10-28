//! Immutable loaded Vault read graph and retained collection instances.
//!
//! `Instance::GetAttributePointer` (0x82976308), `Collection::GetNode/GetData`
//! (0x8297b658/0x8297b7e8), and `Node::GetPointer` (0x82979ac8). This is a
//! selected-class, unconstructed scalar/layout read subset, not a mutable
//! editor, notification service, array/static-storage implementation, or
//! `Instance::Change`.
//! Checked malformed/unsupported-input errors are explicit safe-host policy.
//! Construction materializes a valid post-load graph: it is not native export
//! registration/constructor sequencing. In particular, forward parent references
//! resolve after the selected collections are collected. Native immediate
//! parent `AddRef` requires that parent to be registered already at that point.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "loaded attribute reader is staged for retained World and skateboard runtime owners"
    )
)]

use skate2_formats::attrib::{
    AttributeChunkError, AttributeCollectionLoadData, AttributeDefinition, AttributeExportKind,
    AttributePointerLocation, AttributePointerValue, AttributeVaultMetadata,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    sync::Arc,
};

#[cfg(test)]
#[path = "attribute_runtime_tests.rs"]
pub(crate) mod tests;

#[path = "attribute_runtime_mounted.rs"]
pub(crate) mod mounted;

#[path = "attribute_runtime_array_length.rs"]
mod array_length;
#[path = "attribute_runtime_reference_arrays.rs"]
pub(crate) mod reference_arrays;
#[path = "attribute_runtime_refspec.rs"]
pub(crate) mod refspec;
#[path = "attribute_runtime_pedestrian_animation.rs"]
mod pedestrian_animation;
pub(crate) use pedestrian_animation::{PedestrianSpeedMapping, PedestrianAnimAttributes, PedestrianBranchWindow};
#[path = "attribute_runtime_vector_arrays.rs"]
mod vector_arrays;

const PHYSICS_CLASS: u64 = 0x0429_3d0f_2adc_fbb4;

/// Exact dependency binding. Bytes are retained without relocation writes.
#[derive(Clone, Debug)]
pub(crate) struct AttributeRuntimeDependency {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) is_asset: bool,
    pub(crate) bytes: Arc<[u8]>,
}

#[derive(Debug)]
pub(crate) enum AttributeRuntimeError {
    Metadata(AttributeChunkError),
    DependencyBinding {
        index: usize,
    },
    DuplicateClass {
        class: u64,
    },
    DuplicateDefinition {
        class: u64,
        key: u64,
    },
    DuplicateCollection {
        class: u64,
        collection: u64,
    },
    DuplicateNode {
        class: u64,
        collection: u64,
        key: u64,
    },
    ClassIdentity {
        exported: u64,
        stored: u64,
    },
    MissingClass {
        class: u64,
    },
    MissingParent {
        class: u64,
        collection: u64,
        parent: u64,
    },
    ParentCycle {
        class: u64,
        collection: u64,
    },
    MissingDefinition {
        class: u64,
        key: u64,
    },
    InvalidNodeTypeIndex {
        key: u64,
        index: u16,
    },
    NodeTypeMismatch {
        key: u64,
        node_type: u64,
        definition_type: u64,
    },
    UnsupportedArray {
        key: u64,
    },
    UnsupportedConstructed {
        key: u64,
    },
    UnsupportedClassStaticStorage {
        class: u64,
        key: u64,
    },
    UnsupportedExportPointer {
        slot: AttributePointerLocation,
    },
    MissingRelocation {
        slot: AttributePointerLocation,
        raw_word: u32,
    },
    NullRelativeBase {
        origin: AttributeRuntimeDataOrigin,
        offset: usize,
    },
    ByteRange {
        dependency: u16,
        offset: usize,
        size: usize,
    },
    InlineSize {
        key: u64,
        size: usize,
    },
    ScalarSize {
        size: usize,
    },
    PayloadSize {
        key: u64,
        expected: usize,
        actual: usize,
    },
    RelocatedPayload {
        slot: AttributePointerLocation,
    },
    UnterminatedText {
        dependency: u16,
        offset: usize,
    },
}

impl From<AttributeChunkError> for AttributeRuntimeError {
    fn from(error: AttributeChunkError) -> Self {
        Self::Metadata(error)
    }
}

impl std::fmt::Display for AttributeRuntimeError {
    #[expect(
        clippy::too_many_lines,
        reason = "exhaustive messages for distinct Vault failure boundaries"
    )]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnterminatedText { dependency, offset } => write!(
                formatter,
                "unterminated attribute text at dependency {dependency}+{offset}"
            ),
            Self::Metadata(error) => write!(formatter, "attribute metadata: {error}"),
            Self::DependencyBinding { index } => {
                write!(formatter, "invalid dependency binding {index}")
            }
            Self::DuplicateClass { class } => write!(formatter, "duplicate class {class:016x}"),
            Self::DuplicateDefinition { class, key } => {
                write!(formatter, "duplicate definition {class:016x}/{key:016x}")
            }
            Self::DuplicateCollection { class, collection } => write!(
                formatter,
                "duplicate collection {class:016x}/{collection:016x}"
            ),
            Self::DuplicateNode {
                class,
                collection,
                key,
            } => write!(
                formatter,
                "duplicate node {class:016x}/{collection:016x}/{key:016x}"
            ),
            Self::ClassIdentity { exported, stored } => write!(
                formatter,
                "class identity mismatch {exported:016x}/{stored:016x}"
            ),
            Self::MissingClass { class } => write!(formatter, "missing class {class:016x}"),
            Self::MissingParent {
                class,
                collection,
                parent,
            } => write!(
                formatter,
                "missing parent {parent:016x} for {class:016x}/{collection:016x}"
            ),
            Self::ParentCycle { class, collection } => write!(
                formatter,
                "cyclic parents for {class:016x}/{collection:016x}"
            ),
            Self::MissingDefinition { class, key } => {
                write!(formatter, "missing definition {class:016x}/{key:016x}")
            }
            Self::InvalidNodeTypeIndex { key, index } => {
                write!(formatter, "invalid type index {index} for node {key:016x}")
            }
            Self::NodeTypeMismatch {
                key,
                node_type,
                definition_type,
            } => write!(
                formatter,
                "node {key:016x} type {node_type:016x} differs from definition {definition_type:016x}"
            ),
            Self::UnsupportedArray { key } => write!(formatter, "unsupported array {key:016x}"),
            Self::UnsupportedConstructed { key } => {
                write!(formatter, "unsupported constructed value {key:016x}")
            }
            Self::UnsupportedClassStaticStorage { class, key } => write!(
                formatter,
                "unimplemented registered class-static storage for {class:016x}/{key:016x}"
            ),
            Self::UnsupportedExportPointer { slot } => {
                write!(formatter, "unsupported export pointer at {slot:?}")
            }
            Self::MissingRelocation { slot, raw_word } => {
                write!(formatter, "unresolved word {raw_word:08x} at {slot:?}")
            }
            Self::NullRelativeBase { origin, offset } => {
                write!(formatter, "null {origin:?} base with offset {offset}")
            }
            Self::ByteRange {
                dependency,
                offset,
                size,
            } => write!(
                formatter,
                "invalid dependency {dependency} byte range {offset}+{size}"
            ),
            Self::InlineSize { key, size } => write!(
                formatter,
                "inline value {key:016x} exceeds four bytes: {size}"
            ),
            Self::PayloadSize {
                key,
                expected,
                actual,
            } => write!(
                formatter,
                "attribute {key:016x} requires {expected} bytes, found {actual}"
            ),
            Self::ScalarSize { size } => {
                write!(formatter, "expected four-byte scalar, found {size} bytes")
            }
            Self::RelocatedPayload { slot } => write!(
                formatter,
                "unsupported relocated pointer within returned payload at {slot:?}"
            ),
        }
    }
}

impl std::error::Error for AttributeRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Metadata(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AttributeRuntimeDataOrigin {
    Inline,
    CollectionLayout,
    StoredPointer,
}

/// A retained source range, not a guest or host raw pointer.
#[derive(Clone, Debug)]
pub(crate) struct AttributeRuntimeData {
    buffer: Arc<[u8]>,
    range: Range<usize>,
    origin: AttributeRuntimeDataOrigin,
}

impl AttributeRuntimeData {
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.buffer[self.range.clone()]
    }
    pub(crate) const fn origin(&self) -> AttributeRuntimeDataOrigin {
        self.origin
    }
}

#[derive(Clone, Copy, Debug)]
struct PointerSlot {
    location: AttributePointerLocation,
    raw_word: u32,
}

#[derive(Clone, Debug)]
struct RuntimeNode {
    flags: u8,
    value: u32,
    type_key: Option<u64>,
    type_index: u16,
    // Explicit nodes retain the original serialized word's address; layout
    // nodes are constructed from a Definition and have no serialized node.
    stored_word: Option<PointerSlot>,
}

#[derive(Debug)]
struct RuntimeClass {
    definitions: BTreeMap<u64, AttributeDefinition>,
    layout_nodes: BTreeMap<u64, RuntimeNode>,
}

#[derive(Debug)]
struct RuntimeCollection {
    parent: u64,
    layout: PointerSlot,
    nodes: BTreeMap<u64, RuntimeNode>,
}

/// All selected classes' collections are retained, including their parents.
/// Unselected class type construction is neither attempted nor claimed.
#[derive(Debug)]
pub(crate) struct AttributeRuntimeVault {
    buffers: Vec<Arc<[u8]>>,
    pointers: BTreeMap<AttributePointerLocation, AttributePointerValue>,
    classes: BTreeMap<u64, RuntimeClass>,
    collections: BTreeMap<(u64, u64), RuntimeCollection>,
    refspec_cache: refspec::RefSpecCache,
    /// Every exported class and collection key, selected or not. These answer
    /// only `Database::GetClass` 0x82977868 and `FindCollection` 0x82976528
    /// existence; no unselected class or collection is constructed.
    exported_classes: BTreeSet<u64>,
    exported_collections: BTreeSet<(u64, u64)>,
}

impl AttributeRuntimeVault {
    pub(crate) fn try_load(
        vlt: Arc<[u8]>,
        dependencies: &[AttributeRuntimeDependency],
        selected_classes: &[u64],
    ) -> Result<Arc<Self>, AttributeRuntimeError> {
        let metadata = AttributeVaultMetadata::parse(&vlt)?;
        let buffers = bind_dependencies(vlt, dependencies, &metadata)?;
        let sizes = buffers
            .iter()
            .map(|buffer| buffer.len())
            .collect::<Vec<_>>();
        let pointers = metadata.resolve_pointers(&sizes)?;
        let slices = buffers.iter().map(AsRef::as_ref).collect::<Vec<&[u8]>>();
        let selected = selected_classes.iter().copied().collect::<BTreeSet<_>>();
        let mut classes = BTreeMap::new();
        let mut collections = BTreeMap::new();
        let mut exported_classes = BTreeSet::new();
        let mut exported_collections = BTreeSet::new();
        for export in &metadata.exports {
            match export.kind() {
                AttributeExportKind::Class => {
                    exported_classes.insert(export.object_id);
                    if !selected.contains(&export.object_id) {
                        continue;
                    }
                    let header = export.class_load_data(&buffers[0])?;
                    if header.class_id != export.object_id {
                        return Err(AttributeRuntimeError::ClassIdentity {
                            exported: export.object_id,
                            stored: header.class_id,
                        });
                    }
                    let class = build_class(
                        header.class_id,
                        export.class_definitions(&slices, &pointers)?,
                    )?;
                    if classes.insert(header.class_id, class).is_some() {
                        return Err(AttributeRuntimeError::DuplicateClass {
                            class: header.class_id,
                        });
                    }
                }
                AttributeExportKind::Collection => {
                    // Metadata decoding is not construction of an unrelated
                    // class's object types; unsupported node forms stay lazy.
                    let collection = export.collection_load_data(&buffers[0])?;
                    exported_collections.insert((collection.class_id, collection.collection_id));
                    if !selected.contains(&collection.class_id) {
                        continue;
                    }
                    let key = (collection.class_id, collection.collection_id);
                    let loaded = build_collection(export.data_offset, &collection)?;
                    if collections.insert(key, loaded).is_some() {
                        return Err(AttributeRuntimeError::DuplicateCollection {
                            class: key.0,
                            collection: key.1,
                        });
                    }
                }
                _ => {}
            }
        }
        let vault = Self {
            buffers,
            pointers,
            classes,
            collections,
            refspec_cache: refspec::RefSpecCache::default(),
            exported_classes,
            exported_collections,
        };
        vault.validate_parents()?;
        Ok(Arc::new(vault))
    }

    /// `Database::GetClass` 0x82977868 returns non-null.
    pub(crate) fn has_class(&self, class: u64) -> bool {
        self.exported_classes.contains(&class)
    }

    /// `Attrib::FindCollection` 0x82976528 returns non-null.
    pub(crate) fn has_collection(&self, class: u64, collection: u64) -> bool {
        self.exported_collections.contains(&(class, collection))
    }

    pub(crate) fn collection_parent(
        &self,
        class: u64,
        collection: u64,
    ) -> Result<u64, AttributeRuntimeError> {
        self.collections
            .get(&(class, collection))
            .map(|c| c.parent)
            .ok_or(AttributeRuntimeError::MissingParent {
                class,
                collection,
                parent: collection,
            })
    }

    /// AttribSysUtils::GetLeafCollections82ab9710/82ab98c8: sorted class keys
    /// minus all parent keys, then strict IsDescendantOf82ab8be8 filtering.
    /// The seed itself is not a descendant. This uses the loaded graph already
    /// validated for missing parents/cycles; no second hierarchy is retained.
    pub(crate) fn leaf_descendants_sorted(
        &self,
        class: u64,
        seed: u64,
    ) -> Result<Vec<u64>, AttributeRuntimeError> {
        if !self.classes.contains_key(&class) {
            return Err(AttributeRuntimeError::MissingClass { class });
        }
        self.collection_parent(class, seed)?;
        let mut leaves = Vec::new();
        for key in self.leaf_collections_sorted(class)? {
            let mut parent = self.collection_parent(class, key)?;
            while parent != 0 && parent != seed {
                parent = self.collection_parent(class, parent)?;
            }
            if parent == seed {
                leaves.push(key);
            }
        }
        Ok(leaves)
    }

    /// GetCollections82ab9510: every collection, ordered by its full key.
    pub(crate) fn collections_sorted(&self, class:u64)->Result<Vec<u64>,AttributeRuntimeError> {
        if !self.classes.contains_key(&class) { return Err(AttributeRuntimeError::MissingClass{class}); }
        Ok(self.collections.range((class,0)..=(class,u64::MAX)).map(|(&(_,key),_)|key).collect())
    }
    /// Unfiltered GetLeafCollections82ab9710: sorted class keys minus parents.
    /// Also shared by the existing descendant-filtered navigation caller.
    pub(crate) fn leaf_collections_sorted(&self, class: u64) -> Result<Vec<u64>, AttributeRuntimeError> {
        if !self.classes.contains_key(&class) {
            return Err(AttributeRuntimeError::MissingClass { class });
        }
        let rows = self.collections.range((class, 0)..=(class, u64::MAX));
        let parents = rows.clone().map(|(_, c)| c.parent).collect::<BTreeSet<_>>();
        Ok(rows.filter_map(|(&(_, key), _)| (!parents.contains(&key)).then_some(key)).collect())
    }

    /// GetCollections82ab95e8 filters the complete sorted class collection list,
    /// including non-leaf descendants. The seed itself is excluded.
    pub(crate) fn descendants_sorted(&self, class: u64, seed: u64)
        -> Result<Vec<u64>, AttributeRuntimeError> {
        if !self.classes.contains_key(&class) {
            return Err(AttributeRuntimeError::MissingClass { class });
        }
        self.collection_parent(class, seed)?;
        let mut result = Vec::new();
        for (&(_, key), collection) in self.collections.range((class, 0)..=(class, u64::MAX)) {
            let mut parent = collection.parent;
            while parent != 0 && parent != seed {
                parent = self.collection_parent(class, parent)?;
            }
            if parent == seed { result.push(key); }
        }
        Ok(result)
    }

    /// `FindCollection`, 0x82976528, and Instance ctor, 0x82975bb0.
    /// Missing class/collection binds null, without retrying `default`.
    pub(crate) fn instance(
        self: &Arc<Self>,
        class: u64,
        collection: u64,
    ) -> AttributeRuntimeInstance {
        let key = (class, collection);
        let bound = self
            .classes
            .contains_key(&class)
            .then(|| self.collections.get(&key))
            .flatten();
        AttributeRuntimeInstance {
            vault: Arc::clone(self),
            collection: bound.map(|_| key),
            cached_layout: bound.map(|collection| collection.layout),
        }
    }

    /// Generated physics constructor 0x827b2660 binds its class internally.
    pub(crate) fn physics(self: &Arc<Self>, collection: u64) -> AttributeRuntimePhysics {
        AttributeRuntimePhysics {
            instance: self.instance(PHYSICS_CLASS, collection),
        }
    }

    fn validate_parents(&self) -> Result<(), AttributeRuntimeError> {
        for &(class, original) in self.collections.keys() {
            if !self.classes.contains_key(&class) {
                return Err(AttributeRuntimeError::MissingClass { class });
            }
            let mut visited = BTreeSet::new();
            let mut current = original;
            loop {
                if !visited.insert(current) {
                    return Err(AttributeRuntimeError::ParentCycle {
                        class,
                        collection: original,
                    });
                }
                let collection = &self.collections[&(class, current)];
                if collection.parent == 0 {
                    break;
                }
                if !self.collections.contains_key(&(class, collection.parent)) {
                    return Err(AttributeRuntimeError::MissingParent {
                        class,
                        collection: current,
                        parent: collection.parent,
                    });
                }
                current = collection.parent;
            }
        }
        Ok(())
    }

    fn pointer(
        &self,
        slot: PointerSlot,
    ) -> Result<Option<AttributePointerLocation>, AttributeRuntimeError> {
        match self.pointers.get(&slot.location) {
            Some(AttributePointerValue::Null) => Ok(None),
            Some(AttributePointerValue::Direct {
                dependency_index,
                offset,
            }) => {
                let offset =
                    u32::try_from(*offset).map_err(|_| AttributeRuntimeError::ByteRange {
                        dependency: *dependency_index,
                        offset: usize::MAX,
                        size: 0,
                    })?;
                Ok(Some(AttributePointerLocation {
                    dependency_index: *dependency_index,
                    offset,
                }))
            }
            Some(AttributePointerValue::Export { .. }) => {
                Err(AttributeRuntimeError::UnsupportedExportPointer {
                    slot: slot.location,
                })
            }
            None if slot.raw_word == 0 => Ok(None),
            None => Err(AttributeRuntimeError::MissingRelocation {
                slot: slot.location,
                raw_word: slot.raw_word,
            }),
        }
    }

    fn range(
        &self,
        location: AttributePointerLocation,
        relative: usize,
        size: usize,
        origin: AttributeRuntimeDataOrigin,
    ) -> Result<AttributeRuntimeData, AttributeRuntimeError> {
        let start = usize::try_from(location.offset)
            .ok()
            .and_then(|offset| offset.checked_add(relative));
        let error = || AttributeRuntimeError::ByteRange {
            dependency: location.dependency_index,
            offset: start.unwrap_or(usize::MAX),
            size,
        };
        let start = start.ok_or_else(error)?;
        let end = start.checked_add(size).ok_or_else(error)?;
        let buffer = self
            .buffers
            .get(usize::from(location.dependency_index))
            .ok_or_else(error)?;
        if end > buffer.len() {
            return Err(error());
        }
        if size != 0 {
            // This read subset returns plain serialized data only. A fixup
            // anywhere in the returned range denotes runtime pointer bytes
            // that we deliberately do not fabricate from source placeholders.
            let first = u32::try_from(start.saturating_sub(3)).map_err(|_| error())?;
            let last = u32::try_from(end - 1).map_err(|_| error())?;
            let lower = AttributePointerLocation {
                dependency_index: location.dependency_index,
                offset: first,
            };
            let upper = AttributePointerLocation {
                dependency_index: location.dependency_index,
                offset: last,
            };
            if let Some((&slot, _)) = self.pointers.range(lower..=upper).next() {
                return Err(AttributeRuntimeError::RelocatedPayload { slot });
            }
        }
        Ok(AttributeRuntimeData {
            buffer: Arc::clone(buffer),
            range: start..end,
            origin,
        })
    }

    fn relative_data(
        &self,
        base: PointerSlot,
        offset: usize,
        size: usize,
        origin: AttributeRuntimeDataOrigin,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        match self.pointer(base)? {
            Some(location) => self.range(location, offset, size, origin).map(Some),
            None if offset == 0 => Ok(None),
            None => Err(AttributeRuntimeError::NullRelativeBase { origin, offset }),
        }
    }
}

fn bind_dependencies(
    vlt: Arc<[u8]>,
    dependencies: &[AttributeRuntimeDependency],
    metadata: &AttributeVaultMetadata,
) -> Result<Vec<Arc<[u8]>>, AttributeRuntimeError> {
    if dependencies.len() != metadata.dependencies.len() {
        return Err(AttributeRuntimeError::DependencyBinding {
            index: metadata.dependencies.len(),
        });
    }
    let mut buffers = vec![vlt];
    for (index, expected) in metadata.dependencies.iter().enumerate() {
        let mut matching = dependencies.iter().filter(|dependency| {
            dependency.id == expected.id
                && dependency.name == expected.name
                && dependency.is_asset == expected.is_asset
        });
        let supplied = matching
            .next()
            .ok_or(AttributeRuntimeError::DependencyBinding { index })?;
        if matching.next().is_some() {
            return Err(AttributeRuntimeError::DependencyBinding { index });
        }
        buffers.push(Arc::clone(&supplied.bytes));
    }
    Ok(buffers)
}

fn pointer_slot(
    export: u32,
    relative: u32,
    raw_word: u32,
) -> Result<PointerSlot, AttributeRuntimeError> {
    let offset = export
        .checked_add(relative)
        .ok_or(AttributeRuntimeError::ByteRange {
            dependency: 0,
            offset: usize::MAX,
            size: 4,
        })?;
    Ok(PointerSlot {
        location: AttributePointerLocation {
            dependency_index: 0,
            offset,
        },
        raw_word,
    })
}

fn build_class(
    class: u64,
    definitions: Vec<AttributeDefinition>,
) -> Result<RuntimeClass, AttributeRuntimeError> {
    let mut loaded = RuntimeClass {
        definitions: BTreeMap::new(),
        layout_nodes: BTreeMap::new(),
    };
    for definition in definitions {
        let key = definition.key;
        // ClassPrivate ctor 0x82978d50: no blanket default nodes for ordinary
        // wheel definitions (flags4); static-layout flag8 excludes this table.
        if definition.flags & 2 != 0 && definition.flags & 8 == 0 {
            let flags = 0x90
                | if definition.flags & 1 != 0 { 2 } else { 0 }
                | if definition.flags & 0x20 != 0 { 8 } else { 0 };
            loaded.layout_nodes.insert(
                key,
                RuntimeNode {
                    flags,
                    value: u32::from(definition.offset),
                    type_key: Some(definition.type_id),
                    type_index: 0,
                    stored_word: None,
                },
            );
        }
        if loaded.definitions.insert(key, definition).is_some() {
            return Err(AttributeRuntimeError::DuplicateDefinition { class, key });
        }
    }
    Ok(loaded)
}

fn build_collection(
    export: u32,
    data: &AttributeCollectionLoadData,
) -> Result<RuntimeCollection, AttributeRuntimeError> {
    let mut nodes = BTreeMap::new();
    for (index, entry) in data.entries.iter().enumerate() {
        let relative = 48_usize + usize::from(data.types_length) * 8 + index * 16 + 8;
        let relative = u32::try_from(relative).map_err(|_| AttributeRuntimeError::ByteRange {
            dependency: 0,
            offset: usize::MAX,
            size: 4,
        })?;
        let node = RuntimeNode {
            flags: entry.node_flags | 0x80,
            value: entry.data_pointer,
            type_key: data.types.get(usize::from(entry.type_index)).copied(),
            type_index: entry.type_index,
            stored_word: Some(pointer_slot(export, relative, entry.data_pointer)?),
        };
        // Loaded Collection passes ptrIsRaw=false: layout/static offsets must
        // NOT be normalized. entry_flags is not consumed by Node construction.
        if nodes.insert(entry.key, node).is_some() {
            return Err(AttributeRuntimeError::DuplicateNode {
                class: data.class_id,
                collection: data.collection_id,
                key: entry.key,
            });
        }
    }
    Ok(RuntimeCollection {
        parent: data.parent_id,
        layout: pointer_slot(export, 40, data.layout_pointer)?,
        nodes,
    })
}

/// Retains one exact collection and cached layout identity. No reload/rebind or
/// `Instance::Change` exists here; a null binding stays null for its lifetime.
#[derive(Clone, Debug)]
pub(crate) struct AttributeRuntimeInstance {
    vault: Arc<AttributeRuntimeVault>,
    collection: Option<(u64, u64)>,
    cached_layout: Option<PointerSlot>,
}

impl AttributeRuntimeInstance {
    /// Read only a plain-data field of the retained cached layout. Pointer
    /// relocations elsewhere in the layout are not part of this read; any fixup
    /// overlapping this field is still rejected by the normal checked range.
    /// Missing layout returns `None`; the generated caller owns its fallback.
    pub(crate) fn try_cached_layout_field(
        &self,
        offset: usize,
        size: usize,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        let Some(layout) = self.cached_layout else {
            return Ok(None);
        };
        let Some(base) = self.vault.pointer(layout)? else {
            return Ok(None);
        };
        self.vault
            .range(
                base,
                offset,
                size,
                AttributeRuntimeDataOrigin::CollectionLayout,
            )
            .map(Some)
    }

    /// Resolve a text pointer in the retained cached layout through the SAME
    /// loaded relocation table. No node/parent fallback or string decoding.
    /// Missing layout returns None; the generated caller owns its proven default.
    pub(crate) fn try_cached_layout_text(
        &self,
        offset: u32,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        let Some(layout) = self.cached_layout else {
            return Ok(None);
        };
        let Some(base) = self.vault.pointer(layout)? else {
            return Ok(None);
        };
        let slot_offset =
            base.offset
                .checked_add(offset)
                .ok_or(AttributeRuntimeError::ByteRange {
                    dependency: base.dependency_index,
                    offset: usize::MAX,
                    size: 4,
                })?;
        let slot = AttributePointerLocation {
            dependency_index: base.dependency_index,
            offset: slot_offset,
        };
        self.vault.text_at(slot)
    }
    /// Raw retained cached-layout route for class-bound generated wrappers.
    /// This deliberately bypasses generic node/inheritance lookups. A wrapper
    /// must supply its source layout size and its own proven null fallback.
    pub(crate) fn try_cached_layout(
        &self,
        size: usize,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        self.cached_layout
            .map(|layout| {
                self.vault.relative_data(
                    layout,
                    0,
                    size,
                    AttributeRuntimeDataOrigin::CollectionLayout,
                )
            })
            .transpose()
            .map(Option::flatten)
    }

    pub(crate) const fn is_null(&self) -> bool {
        self.collection.is_none()
    }

    /// Attrib::Instance::GetCollection: expose the actual retained collection
    /// identity without re-resolving references or substituting a parent.
    pub(crate) fn collection_key(&self) -> Option<u64> {
        self.collection.map(|(_, key)| key)
    }

    /// Every scalar request reruns the source node search; values are not
    /// cached. Non-array index is ignored by 0x8297b7e8.
    pub(crate) fn try_get_data(
        &self,
        key: u64,
        index: u32,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        self.try_projected_data(key, index, text::Projection::Plain)
    }
    /// Resolve a scalar text pointer using the SAME generic node/parent/layout
    /// search. Null values end the search; no cached-layout substitution.
    pub(crate) fn try_get_text(
        &self,
        key: u64,
        index: u32,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        self.try_projected_data(key, index, text::Projection::Text)
    }
    fn try_projected_data(
        &self,
        key: u64,
        _index: u32,
        projection: text::Projection,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        match self.resolved_node(key)? {
            Some((class_key, class, collection, node)) => {
                self.node_data(class_key, class, collection, node, key, projection)
            }
            None => Ok(None),
        }
    }

    /// Shared Collection.GetNode parent/layout resolution. A found null node
    /// terminates lookup; callers must not fall through to a parent value.
    fn resolved_node(
        &self,
        key: u64,
    ) -> Result<Option<(u64, &RuntimeClass, &RuntimeCollection, &RuntimeNode)>, AttributeRuntimeError>
    {
        let Some((class_key, original_key)) = self.collection else {
            return Ok(None);
        };
        let class = &self.vault.classes[&class_key];
        let original = &self.vault.collections[&(class_key, original_key)];
        let mut current = original;
        loop {
            if let Some(node) = current.nodes.get(&key) {
                // A found node returns immediately, including a null pointer;
                // it must not resume the parent/layout search on None/error.
                return Ok(Some((class_key, class, current, node)));
            }
            if current.parent == 0 {
                break;
            }
            current = &self.vault.collections[&(class_key, current.parent)];
        }
        if let Some(node) = class.layout_nodes.get(&key) {
            // No parent-layout traversal: only the original child's layout.
            if self.vault.pointer(original.layout)?.is_some() {
                return Ok(Some((class_key, class, original, node)));
            }
        }
        Ok(None)
    }

    /// `GetAttributePointer` 0x82976308 for a generated `ClassRefSpec_*` node,
    /// returning its leading collection key (`ld r4, 0(r3)`). The type
    /// handler's Construct is an empty `blr` (e.g. vtable 0x82167a48 +4), so
    /// the loaded 16 bytes are the runtime value. Same node/parent search as
    /// `try_get_data`; a null result is the caller's `DefaultDataArea`.
    pub(crate) fn try_class_ref_spec_collection(
        &self,
        key: u64,
        type_id: u64,
    ) -> Result<Option<u64>, AttributeRuntimeError> {
        self.try_stored_reference_collection(key, type_id, 0)
    }

    /// RefSpec's authored collection word lives at+8. Callers which only ask
    /// GetAttributePointer then read that word do not construct/cache a pointer.
    pub(crate) fn try_ref_spec_collection(
        &self,
        key: u64,
    ) -> Result<Option<u64>, AttributeRuntimeError> {
        self.try_stored_reference_collection(key, 0x97c5_113d_e9bd_1ad6, 8)
    }

    /// Resolve a stored RefSpec through the same immutable session Vault.
    /// Pedestrian ctor82e9820c/829766f0 use both full-width keys, rather than
    /// assuming the referenced collection belongs to the caller's class.
    /// The host retains the Vault, so no native collection pointer is stored.
    pub(crate) fn try_ref_spec_instance(
        &self,
        key: u64,
    ) -> Result<Option<Self>, AttributeRuntimeError> {
        let Some(collection) = self.try_ref_spec_collection(key)? else {
            return Ok(None);
        };
        if collection == 0 { return Ok(None); }
        let Some(class) = self.try_stored_reference_collection(key, 0x97c5_113d_e9bd_1ad6, 0)? else {
            return Ok(None);
        };
        if class == 0 { return Ok(None); }
        Ok(self.vault.has_collection(class, collection)
            .then(|| self.vault.instance(class, collection)))
    }

    fn try_stored_reference_collection(
        &self,
        key: u64,
        type_id: u64,
        collection_offset: usize,
    ) -> Result<Option<u64>, AttributeRuntimeError> {
        let Some((class_key, original_key)) = self.collection else {
            return Ok(None);
        };
        let class = &self.vault.classes[&class_key];
        let definition =
            class
                .definitions
                .get(&key)
                .ok_or(AttributeRuntimeError::MissingDefinition {
                    class: class_key,
                    key,
                })?;
        if definition.type_id != type_id {
            return Err(AttributeRuntimeError::NodeTypeMismatch {
                key,
                node_type: type_id,
                definition_type: definition.type_id,
            });
        }
        let size = usize::from(definition.size);
        if size < collection_offset + 8 {
            return Err(AttributeRuntimeError::PayloadSize {
                key,
                expected: collection_offset + 8,
                actual: size,
            });
        }
        let mut current = &self.vault.collections[&(class_key, original_key)];
        loop {
            if let Some(node) = current.nodes.get(&key) {
                if node.flags & 2 != 0 {
                    return Err(AttributeRuntimeError::UnsupportedArray { key });
                }
                if node.type_key != Some(type_id) {
                    return Err(AttributeRuntimeError::NodeTypeMismatch {
                        key,
                        node_type: node.type_key.unwrap_or(0),
                        definition_type: type_id,
                    });
                }
                // Explicit stored-pointer nodes only: inline, layout-relative
                // and class-static forms never carry a 16-byte ClassRefSpec.
                if node.flags & 0x70 != 0 {
                    return Err(AttributeRuntimeError::UnsupportedConstructed { key });
                }
                let slot = node
                    .stored_word
                    .expect("explicit collection node retains its stored word");
                return self
                    .vault
                    .relative_data(slot, collection_offset, 8, AttributeRuntimeDataOrigin::StoredPointer)
                    .map(|data| {
                        data.map(|data| {
                            u64::from_be_bytes(
                                data.bytes()
                                    .try_into()
                                    .expect("checked eight bytes"),
                            )
                        })
                    });
            }
            if current.parent == 0 {
                return Ok(None);
            }
            current = &self.vault.collections[&(class_key, current.parent)];
        }
    }

    pub(crate) fn try_scalar_word(
        &self,
        key: u64,
        index: u32,
    ) -> Result<Option<u32>, AttributeRuntimeError> {
        let Some(data) = self.try_get_data(key, index)? else {
            return Ok(None);
        };
        let bytes: [u8; 4] =
            data.bytes()
                .try_into()
                .map_err(|_| AttributeRuntimeError::ScalarSize {
                    size: data.bytes().len(),
                })?;
        Ok(Some(u32::from_be_bytes(bytes)))
    }

    fn node_data(
        &self,
        class_key: u64,
        class: &RuntimeClass,
        collection: &RuntimeCollection,
        node: &RuntimeNode,
        key: u64,
        projection: text::Projection,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        if node.flags & 2 != 0 {
            return Err(AttributeRuntimeError::UnsupportedArray { key });
        }
        if node.flags & 8 != 0 {
            return Err(AttributeRuntimeError::UnsupportedConstructed { key });
        }
        let definition =
            class
                .definitions
                .get(&key)
                .ok_or(AttributeRuntimeError::MissingDefinition {
                    class: class_key,
                    key,
                })?;
        let node_type = node
            .type_key
            .ok_or(AttributeRuntimeError::InvalidNodeTypeIndex {
                key,
                index: node.type_index,
            })?;
        if node_type != definition.type_id {
            return Err(AttributeRuntimeError::NodeTypeMismatch {
                key,
                node_type,
                definition_type: definition.type_id,
            });
        }
        if definition.flags & 0x20 != 0 {
            return Err(AttributeRuntimeError::UnsupportedConstructed { key });
        }
        let size = usize::from(definition.size);
        // Node::GetPointer precedence is exactly inline, layout, static,
        // stored. Do not inspect a losing branch's pointer/offset.
        if node.flags & 0x40 != 0 {
            if size > 4 {
                return Err(AttributeRuntimeError::InlineSize { key, size });
            }
            let slot = node
                .stored_word
                .expect("only explicit nodes carry inline flag");
            return self.vault.project_data(
                slot.location,
                0,
                size,
                AttributeRuntimeDataOrigin::Inline,
                projection,
            );
        }
        if node.flags & 0x30 != 0 {
            // The loaded ctor reads the already-relocated union word at
            // 0x8297bef8. A fixup there changes a relative node's offset too;
            // it is not an ignorable losing stored-pointer alternative.
            // This plain-data subset cannot fabricate that runtime word.
            if let Some(slot) = node
                .stored_word
                .filter(|slot| self.vault.pointers.contains_key(&slot.location))
            {
                return Err(AttributeRuntimeError::RelocatedPayload {
                    slot: slot.location,
                });
            }
        }
        if node.flags & 0x10 != 0 {
            return self.vault.project_relative(
                collection.layout,
                node.value as usize,
                size,
                AttributeRuntimeDataOrigin::CollectionLayout,
                projection,
            );
        }
        if node.flags & 0x20 != 0 {
            // ClassExportPolicy::Initialize (0x82979098) first copies into
            // registered ClassStaticDesc storage. The serialized static-data
            // pointer is NOT the runtime base used by Node::GetPointer.
            // Its registry/copy owner is outside this read subset.
            return Err(AttributeRuntimeError::UnsupportedClassStaticStorage {
                class: class_key,
                key,
            });
        }
        let slot = node
            .stored_word
            .expect("constructed layout node always has layout flag");
        self.vault.project_relative(
            slot,
            0,
            size,
            AttributeRuntimeDataOrigin::StoredPointer,
            projection,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AttributePhysicsLayoutOrigin {
    CachedLayout,
    DefaultZero32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AttributePhysicsLayout {
    pub(crate) raw_bytes: [u8; 32],
    pub(crate) origin: AttributePhysicsLayoutOrigin,
}

/// Only this class-bound wrapper exposes the generated physics layout route.
/// It does not call generic `WorldGravity` `GetData`, even for explicit overrides.
#[derive(Clone, Debug)]
pub(crate) struct AttributeRuntimePhysics {
    instance: AttributeRuntimeInstance,
}

impl AttributeRuntimePhysics {
    pub(crate) fn try_physics_layout(
        &self,
    ) -> Result<AttributePhysicsLayout, AttributeRuntimeError> {
        let data = self
            .instance
            .cached_layout
            .map(|layout| {
                self.instance.vault.relative_data(
                    layout,
                    0,
                    32,
                    AttributeRuntimeDataOrigin::CollectionLayout,
                )
            })
            .transpose()?
            .flatten();
        match data {
            Some(data) => Ok(AttributePhysicsLayout {
                raw_bytes: data.bytes().try_into().expect("checked exact32 byte range"),
                origin: AttributePhysicsLayoutOrigin::CachedLayout,
            }),
            // Generated ctor0x827b2660 substitutes DefaultDataArea(32) when
            // cached layout is null; live first32bytes at0x83059d30 are zero.
            None => Ok(AttributePhysicsLayout {
                raw_bytes: [0; 32],
                origin: AttributePhysicsLayoutOrigin::DefaultZero32,
            }),
        }
    }
}

#[path = "attribute_runtime_text.rs"]
mod text;
