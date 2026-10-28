//! Unregistered PROXY single-Texture resident builder draft.
//!
//! The existing resident load factory calls `build` before its collection
//! callback. A failure here stops that load; this module never acknowledges a
//! parent callback or performs Presentation registration on its own.

use crate::gpu_renderer::{GpuTexture, GpuTextureUploader};
use skate2_core::{
    catalog_resident_load_factory::CollectionAssetBuilderSessionFactory,
    collection_resident_streamer::CollectionId,
    decoded_arena_asset::{DecodedArenaAsset, DecodedArenaAssetBuilder},
    game_stream::StreamKind,
    mounted_character_scene::own_texture_arena,
    owned_resident_collection::ResidentAssetIdentity,
    resident_collection_load_driver::{CollectionAssetBuildInput, CollectionAssetBuilder},
    world_stream_catalog::{
        AssetRegistrationSnapshot, CollectionRegistrationSnapshot, WorldStreamCatalog,
    },
};
use skate2_formats::{
    material::{TEXTURE_TYPE_ID, Texture},
    pegasus::{TABLE_OF_CONTENTS_TYPE_ID, TableOfContents},
    render::GPU_BUFFER_RESOURCE_TYPE_ID,
};
use std::{error::Error, fmt, ops::Range, ptr::NonNull};

// The TOC row type tested by ProcessArena at 0x82d60058. Distinct from the
// arena dictionary Texture type ID.
const TOC_TEXTURE_TYPE: u32 = 0xac46_2e4a;

#[derive(Debug)]
pub(crate) struct Kind0BuildError(String);

impl Kind0BuildError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for Kind0BuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for Kind0BuildError {}

/// Stable, distinct host allocation corresponding to one native aliased
/// Texture. `gpu` owns the actual uploaded levels. The decoded source remains
/// beside this Box under the same resident asset allocation.
pub(crate) struct Kind0AliasProduct {
    pub(crate) outer_asset_id: u64,
    pub(crate) texture_asset_id: u64,
    /// Both ranges address the `decoded.resources` kept in the same resident.
    pub(crate) source_texture: ResidentObjectLocator,
    pub(crate) source_gpu_buffer: ResidentObjectLocator,
    /// Original serialized Texture words, including its ObjectId fetch link.
    /// These are not the post-fixup Xbox pointer used by GetAliasedParameters.
    pub(crate) source_parameters: Texture,
    /// Same-format alias leaves the source dimensions/format unchanged; the
    /// native alias flag is added separately from the unknown full flags word.
    pub(crate) aliased_parameters: SameFormatAliasFacts,
    pub(crate) gpu: GpuTexture,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResidentObjectLocator {
    pub(crate) dictionary_index: u32,
    pub(crate) resource_index: usize,
    pub(crate) object_range: Range<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SameFormatAliasFacts {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) depth: u32,
    pub(crate) format: u32,
    pub(crate) alias_flag_bit: u32,
}

/// One processed kind-0 arena allocation. The physical loaded-asset cache
/// stores this behind one `Arc` per full outer arena ID; a Presentation owner
/// may retain that same `Arc` while its collection resident hold is live.
pub(crate) struct Kind0ProcessedArena {
    // Rust drops fields in declaration order. Release the alias/GPU allocation
    // before the decoded source resources it references symbolically.
    pub(crate) alias: Box<Kind0AliasProduct>,
    pub(crate) decoded: DecodedArenaAsset,
}

/// Compatibility name for the existing parent-only builder's output. The
/// shared loaded-asset cache, rather than this builder, supplies `Arc` cloning
/// and cross-collection canonicalization.
pub(crate) type Kind0ResidentArenaAsset = Kind0ProcessedArena;

impl Kind0ProcessedArena {
    /// Stable identity to pass to the already drafted shader texture table.
    /// The caller must remove it from that table before destroying this asset.
    pub(crate) fn alias_identity(&self) -> NonNull<Kind0AliasProduct> {
        NonNull::from(self.alias.as_ref())
    }
}

impl ResidentAssetIdentity for Kind0ProcessedArena {
    fn resident_asset_id(&self) -> CollectionId {
        CollectionId(self.decoded.asset_id)
    }
}

#[derive(Clone)]
pub(crate) struct Kind0ResidentBuilderSessions {
    catalog: WorldStreamCatalog,
    parent_id: CollectionId,
    uploader: GpuTextureUploader,
}

impl Kind0ResidentBuilderSessions {
    pub(crate) fn new(
        catalog: WorldStreamCatalog,
        parent_id: CollectionId,
        uploader: GpuTextureUploader,
    ) -> Self {
        Self {
            catalog,
            parent_id,
            uploader,
        }
    }
}

impl CollectionAssetBuilderSessionFactory for Kind0ResidentBuilderSessions {
    type Asset = Kind0ResidentArenaAsset;
    type Builder = Kind0ResidentBuilder;
    type Error = Kind0BuildError;

    fn begin_session(
        &mut self,
        collection: &CollectionRegistrationSnapshot,
    ) -> Result<Self::Builder, Self::Error> {
        if collection.id != self.parent_id.0 {
            return Err(Kind0BuildError::new(format!(
                "kind-0 builder received non-parent collection {:#018x}",
                collection.id
            )));
        }
        let registered = self
            .catalog
            .collection(collection.id)
            .map_err(|error| Kind0BuildError::new(error.to_string()))?
            .ok_or_else(|| Kind0BuildError::new("parent collection is not registered"))?;
        if &registered != collection {
            return Err(Kind0BuildError::new(
                "parent collection registration snapshot changed",
            ));
        }
        if registered.asset_count == 0 {
            return Err(Kind0BuildError::new("kind-0 parent has zero assets"));
        }
        let file = self
            .catalog
            .file(&collection.owner)
            .map_err(|error| Kind0BuildError::new(error.to_string()))?
            .ok_or_else(|| Kind0BuildError::new("parent stream owner is absent"))?;
        if file.kind != StreamKind::Presentation {
            return Err(Kind0BuildError::new("parent stream is not Presentation"));
        }
        Ok(Kind0ResidentBuilder {
            decoder: DecodedArenaAssetBuilder,
            uploader: self.uploader.clone(),
        })
    }
}

pub(crate) struct Kind0ResidentBuilder {
    decoder: DecodedArenaAssetBuilder,
    uploader: GpuTextureUploader,
}

impl CollectionAssetBuilder<AssetRegistrationSnapshot> for Kind0ResidentBuilder {
    type Asset = Kind0ResidentArenaAsset;
    type Error = Kind0BuildError;

    fn build(
        &mut self,
        input: CollectionAssetBuildInput<'_, AssetRegistrationSnapshot>,
    ) -> Result<Option<Self::Asset>, Self::Error> {
        let decoded = self
            .decoder
            .build(input)
            .map_err(|error| Kind0BuildError::new(error.to_string()))?
            .ok_or_else(|| Kind0BuildError::new("arena decoder returned no asset"))?;
        process_decoded_kind0(decoded, &self.uploader).map(Some)
    }
}

/// The reusable producer after the existing decoder has built the one source
/// allocation. A later unified Presentation builder can dispatch to this for
/// observed parent *or spatial* single-Texture arenas after classifying its
/// decoded TOC, without calling the decoder or uploader a second time.
pub(crate) fn process_decoded_kind0(
    decoded: DecodedArenaAsset,
    uploader: &GpuTextureUploader,
) -> Result<Kind0ProcessedArena, Kind0BuildError> {
    let resources = &decoded.resources.resources;
    let arena = &decoded.arena;
    // Native ProcessArena walks exported objects and selects its first TOC.
    // The current RW4 parser exposes the dictionary, not an exported-object
    // iterator. Requiring exactly one dictionary TOC is a narrower admission
    // observed for parent and spatial Texture arenas, not general
    // exported-order parity.
    let mut toc_rows = None;
    for entry in &arena.dictionary {
        if entry.type_id == TABLE_OF_CONTENTS_TYPE_ID {
            if toc_rows.is_some() {
                return Err(Kind0BuildError::new("arena has multiple TOC objects"));
            }
            let bytes = arena
                .object_bytes(resources, entry)
                .ok_or_else(|| Kind0BuildError::new("TOC object range is invalid"))?;
            toc_rows = Some(
                TableOfContents::parse(bytes)
                    .map_err(|error| Kind0BuildError::new(error.to_string()))?
                    .entries,
            );
        }
    }
    let rows = toc_rows.ok_or_else(|| Kind0BuildError::new("arena has no TOC"))?;
    if rows.len() != 1 || rows[0].object_type != TOC_TEXTURE_TYPE {
        return Err(Kind0BuildError::new(
            "arena does not select the native single-Texture branch",
        ));
    }
    let row = &rows[0];
    let texture_entry = arena
        .local_entry(row.object)
        .filter(|entry| entry.type_id == TEXTURE_TYPE_ID)
        .ok_or_else(|| Kind0BuildError::new("TOC Texture locator is invalid"))?;
    let texture_bytes = arena
        .object_bytes(resources, texture_entry)
        .ok_or_else(|| Kind0BuildError::new("Texture object range is invalid"))?;
    let texture =
        Texture::parse(texture_bytes).map_err(|error| Kind0BuildError::new(error.to_string()))?;
    let gpu_entry = arena
        .local_entry(texture.data)
        .filter(|entry| entry.type_id == GPU_BUFFER_RESOURCE_TYPE_ID)
        .ok_or_else(|| Kind0BuildError::new("Texture GPU-buffer locator is invalid"))?;
    arena
        .object_bytes(resources, gpu_entry)
        .ok_or_else(|| Kind0BuildError::new("Texture GPU-buffer range is invalid"))?;
    let source_texture = ResidentObjectLocator {
        dictionary_index: row.object.entry_index(),
        resource_index: texture_entry.resource_index,
        object_range: texture_entry.object_range.clone(),
    };
    let source_gpu_buffer = ResidentObjectLocator {
        dictionary_index: texture.data.entry_index(),
        resource_index: gpu_entry.resource_index,
        object_range: gpu_entry.object_range.clone(),
    };

    let mut owned = own_texture_arena(arena, resources)
        .map_err(|error| Kind0BuildError::new(error.to_string()))?;
    if owned.textures.len() != 1 {
        return Err(Kind0BuildError::new(
            "single-Texture arena decoded to a different count",
        ));
    }
    let source = owned.textures.pop().expect("one Texture checked");
    if source.guid != row.guid || source.dictionary_index != row.object.entry_index() {
        return Err(Kind0BuildError::new(
            "decoded Texture disagrees with TOC locator",
        ));
    }
    let gpu = uploader
        .upload_native_texture(&source)
        .map_err(|error| Kind0BuildError::new(error.to_string()))?;
    let alias = Box::new(Kind0AliasProduct {
        outer_asset_id: decoded.asset_id,
        texture_asset_id: row.guid,
        source_texture,
        source_gpu_buffer,
        source_parameters: texture,
        aliased_parameters: SameFormatAliasFacts {
            width: texture.width,
            height: texture.height,
            depth: texture.depth,
            format: texture.pixel_format,
            alias_flag_bit: 1,
        },
        gpu,
    });
    Ok(Kind0ProcessedArena { alias, decoded })
}
