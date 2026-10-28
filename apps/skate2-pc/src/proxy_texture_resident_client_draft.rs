//! Unregistered renderer-bearing resident Texture load experiment.
//!
//! The caller supplies an already accepted PROXY catalog and one required
//! single-Texture collection ID. This does not schedule the PROXY closure,
//! register Presentation kind 0, or publish proxy activation. The draw list is
//! intentionally empty until a real Presentation consumer is connected.

use crate::{
    gpu_renderer::{GpuObject, GpuRenderer, GpuTexture, RenderResult},
    render_window::{RenderWindowClient, RenderWindowOptions},
};
use skate2_core::{
    catalog_resident_load_factory::CatalogCollectionAssetMetadata,
    collection_load_pump::CollectionStreamFormat,
    collection_resident_streamer::{CollectionId, ResidentCollectionStorage},
    decoded_arena_asset::{DecodedArenaAsset, DecodedArenaAssetBuilder, DecodedArenaAssetError},
    game_stream::StreamKind,
    mounted_character_scene::own_texture_arena,
    owned_resident_collection::{
        OwnedResidentCollection, OwnedResidentCollectionError, OwnedResidentCollectionStorage,
        ResidentAssetIdentity,
    },
    resident_collection_load_driver::{
        CollectionAssetBuildInput, CollectionAssetBuilder, CollectionDecompressionWake,
        MountedResidentCollectionLoadDriver, MountedResidentCollectionLoadSpec,
    },
    world_stream_catalog::{AssetRegistrationSnapshot, WorldStreamCatalog},
};
use skate2_formats::mount::MountSet;
use std::{sync::{Arc, Mutex}, time::Instant};
use winit::event::WindowEvent;

struct UploadedTexture {
    toc_id: u64,
    texture_dictionary_index: u32,
    gpu: GpuTexture,
}

/// The decoded source and its real GPU upload have the same resident lifetime.
struct ResidentTextureAsset {
    decoded: DecodedArenaAsset,
    uploaded: Mutex<Option<UploadedTexture>>,
}

impl ResidentAssetIdentity for ResidentTextureAsset {
    fn resident_asset_id(&self) -> CollectionId {
        CollectionId(self.decoded.asset_id)
    }
}

#[derive(Default)]
struct TextureResidentBuilder(DecodedArenaAssetBuilder);

impl CollectionAssetBuilder<AssetRegistrationSnapshot> for TextureResidentBuilder {
    type Asset = ResidentTextureAsset;
    type Error = DecodedArenaAssetError;

    fn build(
        &mut self,
        input: CollectionAssetBuildInput<'_, AssetRegistrationSnapshot>,
    ) -> Result<Option<Self::Asset>, Self::Error> {
        self.0.build(input).map(|asset| {
            asset.map(|decoded| ResidentTextureAsset {
                decoded,
                uploaded: Mutex::new(None),
            })
        })
    }
}

type TextureStorage = OwnedResidentCollectionStorage<
    ResidentTextureAsset,
    fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
    OwnedResidentCollectionError,
>;

fn identity(error: OwnedResidentCollectionError) -> OwnedResidentCollectionError {
    error
}

/// One explicitly selected, retained resident collection. A root caller must
/// supply the accepted PROXY catalog; no second XSM/XST loader is introduced.
pub(crate) struct ProxyTextureResidentClient {
    mounts: Arc<MountSet>,
    catalog: WorldStreamCatalog,
    collection_id: CollectionId,
    storage: TextureStorage,
    collection: Option<OwnedResidentCollection<ResidentTextureAsset>>,
    initialized: bool,
}

impl ProxyTextureResidentClient {
    pub(crate) fn new(
        mounts: Arc<MountSet>,
        catalog: WorldStreamCatalog,
        collection_id: CollectionId,
    ) -> Self {
        Self {
            mounts,
            catalog,
            collection_id,
            storage: TextureStorage::new(identity),
            collection: None,
            initialized: false,
        }
    }

    fn load_and_upload(&mut self, renderer: &GpuRenderer) -> RenderResult<()> {
        let descriptor = self.catalog.collection(self.collection_id.0)?
            .ok_or("selected resident Texture collection is absent")?;
        let file = self.catalog.file(&descriptor.owner)?
            .ok_or("selected resident Texture file owner is absent")?;
        if file.kind != StreamKind::Presentation || descriptor.asset_count == 0 {
            return Err("selected PROXY collection must be nonempty Presentation".into());
        }
        let count = i32::try_from(descriptor.asset_count)?;
        let collection = self.storage.create_empty(count)?;
        self.collection = Some(collection);
        let collection = self.collection.as_ref().expect("stored resident collection");
        let writer = self.storage.writer(collection)?;
        let wake = CollectionDecompressionWake::default();
        let mut driver = MountedResidentCollectionLoadDriver::new(
            Arc::clone(&self.mounts),
            MountedResidentCollectionLoadSpec {
                path: Arc::clone(&descriptor.path),
                payload_offset: descriptor.offset,
                request_id: 0,
                target_assets: count,
                format: CollectionStreamFormat(i32::try_from(descriptor.stream_format)?),
            },
            CatalogCollectionAssetMetadata::new(self.catalog.clone()),
            writer,
            TextureResidentBuilder::default(),
            wake.clone(),
        ).map_err(|error| format!("resident Texture start: {error}"))?;
        // The same producer/pump/decompression driver as the resident census.
        // This synchronous first packet leaves native asynchronous scheduling
        // and collection callbacks to the eventual PROXY runtime.
        while !driver.pump().is_complete() {
            driver.update_one_tick()
                .map_err(|error| format!("resident Texture update: {error}"))?;
            let _ = wake.request();
            std::thread::yield_now();
        }
        drop(driver);

        let mut uploaded_count = 0_usize;
        collection.resident_assets_view().try_for_each(|asset| -> RenderResult<()> {
            let mut arena = own_texture_arena(
                &asset.decoded.arena,
                &asset.decoded.resources.resources,
            )?;
            if arena.textures.len() != 1 {
                return Err(format!(
                    "arena {:016x} has {} textures, expected the native single-Texture branch",
                    asset.decoded.asset_id,
                    arena.textures.len()
                ).into());
            }
            let source = arena.textures.pop().expect("one texture checked");
            let gpu = renderer.upload_native_texture(&source)?;
            let mut slot = asset.uploaded.lock()
                .map_err(|_| "resident GPU texture slot is poisoned")?;
            if slot.is_some() {
                return Err("resident Texture uploaded twice".into());
            }
            *slot = Some(UploadedTexture {
                toc_id: source.guid,
                texture_dictionary_index: source.dictionary_index,
                gpu,
            });
            uploaded_count += 1;
            Ok(())
        })?;
        if uploaded_count != usize::try_from(descriptor.asset_count)? {
            return Err("resident Texture count differed from XSM".into());
        }
        Ok(())
    }
}

impl RenderWindowClient for ProxyTextureResidentClient {
    fn initialize(&mut self, renderer: &GpuRenderer) -> RenderResult<Vec<GpuObject>> {
        if self.initialized {
            return Err("resident Texture client initialized twice".into());
        }
        self.initialized = true;
        self.load_and_upload(renderer)?;
        Ok(Vec::new())
    }

    fn window_event(&mut self, _event: &WindowEvent) -> RenderResult<()> {
        Ok(())
    }

    fn frame(
        &mut self,
        _renderer: &GpuRenderer,
        _objects: &mut Vec<GpuObject>,
        _now: Instant,
    ) -> RenderResult<()> {
        Ok(())
    }
}

impl Drop for ProxyTextureResidentClient {
    fn drop(&mut self) {
        if let Some(collection) = self.collection.take() {
            // There is no Presentation manager registration in this packet.
            // Collection destruction drops each GPU upload and decoded arena.
            self.storage.destroy(collection);
        }
    }
}

/// Callable only once a root caller supplies the accepted PROXY catalog and a
/// collection selected from its actual parent closure. No `TextureScene` path.
pub(crate) fn run_with_catalog(
    mounts: Arc<MountSet>,
    catalog: WorldStreamCatalog,
    collection_id: CollectionId,
    options: RenderWindowOptions,
) -> RenderResult<()> {
    crate::render_window::run_with_options(
        ProxyTextureResidentClient::new(mounts, catalog, collection_id),
        options,
    )
}
