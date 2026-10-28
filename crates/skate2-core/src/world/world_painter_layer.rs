//! Actual `LayerMan` stream registration and region queries (`GetTOCEntry`:
//! tile, quad tree, dictionary, palette). Event dispatch remains an explicit
//! boundary.
use crate::{
    actor_face_playback::tree_database::clip_fetch::timing::low_integer_word,
    selected_simulation_asset_services::{SelectedAssetError, SelectedSimulationAssetServices},
    world_painter_layer_asset::{LayerAssetError, WorldPainterLayerAsset},
    world_painter_palette::{CAMERA_REGION_DATA, PALETTE_TOC},
};
use skate2_formats::attrib::string_hash64;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldPainterLayerError {
    OwnerLock,
    MissingAssetManager,
    Asset(SelectedAssetError),
    Source(LayerAssetError),
    NullRegistrationQuadTree,
    Vector2Assertion,
    IntegerConversion,
    ZeroTileIdAssertion,
    TileCapacity,
    LayerIndex(usize),
    /// A region painted on `eCameras_Race` without camera region data.
    MissingCameraRegionData(u32),
}
impl std::fmt::Display for WorldPainterLayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WorldPainter layer boundary: {self:?}")
    }
}
impl std::error::Error for WorldPainterLayerError {}

/// Full 64-bit GUIDs at native sLayerData83024690; registration compares `ld`
/// values, not the truncated decompiler expression. Index12 is `CamerasRace`.
pub(crate) const LAYER_GUIDS: [u64; 19] = [
    0x0000_0000_03e3_87c2,
    0x0000_0001_03e3_87c2,
    0x0000_0002_03e3_87c2,
    0x0000_0005_03e3_87c2,
    0x0000_000d_03e3_87c2,
    0x0000_000e_03e3_87c2,
    0x0000_0006_03e3_87c2,
    0x0000_000f_03e3_87c2,
    0x0000_0010_03e3_87c2,
    0x0000_0011_03e3_87c2,
    0x0000_0012_03e3_87c2,
    0x0000_0013_03e3_87c2,
    0x0000_0014_03e3_87c2,
    0x0000_0016_03e3_87c2,
    0x0000_0017_03e3_87c2,
    0x0000_0019_03e3_87c2,
    0x0000_001a_03e3_87c2,
    0x0000_001b_03e3_87c2,
    0x0000_001c_03e3_87c2,
];

/// `WorldPainter::eCameras_Race`.
pub(crate) const CAMERAS_RACE: usize = 12;

/// `LayerHandle` byte +1 (`bShouldConvertLayerStringToLower`) is set only for
/// `eFE_LocationTriggers` (5) and `eServices` (13) in `sLayerData`.
const LOWER_CASE_LAYERS: [usize; 2] = [5, 13];

struct TileHandle {
    offset: [f32; 4],
    layers: [Option<WorldPainterLayerAsset>; 32],
    // Native pLayerMan identity is implicit in this owning map, no second owner.
}
struct KeyCache {
    hint_node: Option<usize>,
    remaining_words: [Option<u32>; 3],
}
pub(crate) struct WorldPainterLayerMan {
    load_in_progress: Mutex<Vec<u32>>,
    tile_list: Vec<u32>,
    broadcaster_port: u32,
    broadcaster_queue: u32,
    key_cache: [KeyCache; 19],
    tiles: BTreeMap<u32, TileHandle>,
    assets: Option<Arc<SelectedSimulationAssetServices>>,
}
impl WorldPainterLayerMan {
    /// Constructor82ceb528: actual empty storage, queue9/hash, nineteen NULL
    /// hint nodes with untouched remaining key-cache words, NULL asset manager.
    pub(crate) fn construct() -> Self {
        Self {
            load_in_progress: Mutex::new(Vec::new()),
            tile_list: Vec::new(),
            broadcaster_port: skate2_formats::state_graph::state_graph_string_hash32(
                b"WorldPainterBroadcaster",
            ),
            broadcaster_queue: 9,
            key_cache: std::array::from_fn(|_| KeyCache {
                hint_node: None,
                remaining_words: [None; 3],
            }),
            tiles: BTreeMap::new(),
            assets: None,
        }
    }
    /// `Init82cea8b0` is exactly this source pointer store.
    pub(crate) fn initialize(&mut self, assets: Arc<SelectedSimulationAssetServices>) {
        self.assets = Some(assets);
    }
    fn lookup(&self, id: u64) -> Result<Option<WorldPainterLayerAsset>, WorldPainterLayerError> {
        let source = self
            .assets
            .as_ref()
            .ok_or(WorldPainterLayerError::MissingAssetManager)?
            .try_inner(id)
            .map_err(WorldPainterLayerError::Asset)?;
        source
            .map(WorldPainterLayerAsset::try_new)
            .transpose()
            .map_err(WorldPainterLayerError::Source)
    }
    /// `RegisterStreamedTile82ceb770`: lookup→bounds→tile/map publication→GUID→slot.
    /// Unknown GUIDs still leave the newly created tile handle published.
    pub(crate) fn try_register(&mut self, id: u64) -> Result<(), WorldPainterLayerError> {
        let Some(asset) = self.lookup(id)? else {
            return Ok(());
        };
        let (offset, width) = registration_coordinates(&asset)?;
        let key = tile_id(offset, width)?;
        if !self.tiles.contains_key(&key) {
            if self.tiles.len() == 128 {
                return Err(WorldPainterLayerError::TileCapacity);
            }
            self.tiles.insert(
                key,
                TileHandle {
                    offset,
                    layers: std::array::from_fn(|_| None),
                },
            );
        }
        let guid = asset.layer_guid().map_err(WorldPainterLayerError::Source)?;
        if let Some(layer) = LAYER_GUIDS.iter().position(|candidate| *candidate == guid) {
            self.tiles
                .get_mut(&key)
                .expect("existing or newly published tile")
                .layers[layer] = Some(asset);
        }
        Ok(())
    }
    /// `UnRegisterStreamedTile82ceb268` captures current registry winner again.
    /// It clears the selected slot without pointer-equality filtering, then
    /// removes the tile only when all32 source slots are NULL.
    pub(crate) fn try_unregister(&mut self, id: u64) -> Result<(), WorldPainterLayerError> {
        let Some(asset) = self.lookup(id)? else {
            return Ok(());
        };
        let (offset, width) = registration_coordinates(&asset)?;
        let key = tile_id(offset, width)?;
        let Some(tile) = self.tiles.get_mut(&key) else {
            return Ok(());
        };
        let guid = asset.layer_guid().map_err(WorldPainterLayerError::Source)?;
        let Some(layer) = LAYER_GUIDS.iter().position(|candidate| *candidate == guid) else {
            return Ok(());
        };
        tile.layers[layer] = None;
        if tile.layers.iter().all(Option::is_none) {
            self.tiles.remove(&key);
        }
        Ok(())
    }
    /// `GetRegionData` 0x82ceb228 on `eCameras_Race`, as
    /// `GlobalConditionData::SetWorldPainterRegionBlendValue` 0x82e36678 reads
    /// it: the type and blend value of the region painted at `position`.
    pub(crate) fn try_camera_region(
        &self,
        position: [f32; 2],
    ) -> Result<Option<(&'static str, f32)>, WorldPainterLayerError> {
        let Some(entry) = self.try_toc_entry(CAMERAS_RACE, position)? else {
            return Ok(None);
        };
        let region = PALETTE_TOC[entry].0;
        CAMERA_REGION_DATA
            .iter()
            .find(|(guid_high, ..)| *guid_high == region)
            .map(|&(_, kind, blend)| Some((kind, blend)))
            .ok_or(WorldPainterLayerError::MissingCameraRegionData(region))
    }
    /// `LayerMan::GetAttribData` 0x82ceaf50: tile (`GetTileHandle`, 128-unit
    /// tiles) -> layer -> quad tree leaf -> dictionary region word -> first
    /// palette entry whose GUID high word matches -> `Attrib::StringToKey`
    /// (`StringHash64`) of its name, lower-cased first for the layers whose
    /// handle asks. Every miss returns key 0, as native does.
    pub(crate) fn try_attrib_key(
        &self,
        layer: usize,
        position: [f32; 2],
    ) -> Result<u64, WorldPainterLayerError> {
        let Some(name) = self.try_region_name(layer, position)? else { return Ok(0); };
        Ok(if LOWER_CASE_LAYERS.contains(&layer) {
            // `StringToLowerCaseKey` 0x82976640: C `tolower` per byte.
            string_hash64(&name.to_ascii_lowercase())
        } else {
            string_hash64(name)
        })
    }
    /// `GetRegionName` 0x82ceb1e8 reads the same TOC entry as `GetAttribData`,
    /// retaining the original case and returning NULL for a missing region.
    pub(crate) fn try_region_name(
        &self, layer: usize, position: [f32; 2],
    ) -> Result<Option<&'static str>, WorldPainterLayerError> {
        Ok(self.try_toc_entry(layer, position)?.map(|entry| PALETTE_TOC[entry].1))
    }
    /// `GetTOCEntry` 0x82ceae30: the tile (`GetTileHandle`, 128-unit tiles),
    /// its layer, `DoQuadTreeLookup`, the dictionary region word, then the
    /// first of the 0x16B palette entries whose GUID high word matches. A
    /// missing tile, layer, tree, leaf or palette entry is NULL.
    fn try_toc_entry(
        &self, layer: usize, position: [f32; 2],
    ) -> Result<Option<usize>, WorldPainterLayerError> {
        if layer >= LAYER_GUIDS.len() {
            return Err(WorldPainterLayerError::LayerIndex(layer));
        }
        let key = tile_id(
            [position[0], position[1], position[0], position[1]],
            [128.0; 4],
        )?;
        if key == 0 {
            return Err(WorldPainterLayerError::ZeroTileIdAssertion);
        }
        let Some(asset) = self
            .tiles
            .get(&key)
            .and_then(|tile| tile.layers[layer].as_ref())
        else {
            return Ok(None);
        };
        let Some(index) = asset
            .dictionary_lookup(position)
            .map_err(WorldPainterLayerError::Source)?
        else {
            return Ok(None);
        };
        let region = asset
            .region_word(index)
            .map_err(WorldPainterLayerError::Source)?;
        Ok(PALETTE_TOC.iter().position(|(guid_high, _)| *guid_high == region))
    }
    #[cfg(test)]
    pub(crate) fn tile_count(&self) -> usize {
        self.tiles.len()
    }
    #[cfg(test)]
    pub(crate) fn populated_layer_count(&self, layer: usize) -> usize {
        self.tiles
            .values()
            .filter(|tile| tile.layers[layer].is_some())
            .count()
    }
}

fn registration_coordinates(
    asset: &WorldPainterLayerAsset,
) -> Result<([f32; 4], [f32; 4]), WorldPainterLayerError> {
    let (subtractor, half_width) = asset
        .registration_bounds()
        .map_err(WorldPainterLayerError::Source)?
        .ok_or(WorldPainterLayerError::NullRegistrationQuadTree)?;
    Ok((
        std::array::from_fn(|i| subtractor[i] - half_width[i]),
        std::array::from_fn(|i| half_width[i] * 2.0),
    ))
}

/// `GenTileId82ceabe0`, preserving rounded divisions→floor X/Y→width products,
/// separate X+1 and -100000 multiply, signed64 conversions and low32 subtraction.
fn tile_id(position: [f32; 4], width: [f32; 4]) -> Result<u32, WorldPainterLayerError> {
    // Raw vrlimi duplicates XY, then vcmpeq compares that vector to itself.
    // IDA misprints VA62 as v94; this is a NaN guard, not an XYXY assertion.
    if position[0].is_nan() || position[1].is_nan() {
        return Err(WorldPainterLayerError::Vector2Assertion);
    }
    let x_divided = position[0] / width[0];
    let y_divided = position[1] / width[1];
    let x_floor = x_divided.floor();
    let y_floor = y_divided.floor();
    let x = width[0] * x_floor;
    let y = width[1] * y_floor;
    let x_scaled = (x + 1.0) * -100_000.0;
    let y_word = low_integer_word(y).map_err(|_| WorldPainterLayerError::IntegerConversion)?;
    let x_word =
        low_integer_word(x_scaled).map_err(|_| WorldPainterLayerError::IntegerConversion)?;
    Ok(y_word.wrapping_sub(x_word))
}

#[cfg(test)]
#[path = "world_painter_layer_tests.rs"]
mod tests;
