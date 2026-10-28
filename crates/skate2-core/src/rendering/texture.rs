//! Runtime-owned world textures and their independently streamed collections.

use crate::streaming::StreamFocusState;
use skate2_formats::material::{LinearTextureLevel, TextureEndian, TextureType};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct TextureScene {
    pub location_name: String,
    pub focus: [f32; 3],
    pub stream_focus: StreamFocusState,
    pub collections: Vec<TextureCollection>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextureCollection {
    pub id: u64,
    pub name: String,
    pub position: [f32; 3],
    pub distance_xz: f32,
    pub arenas: Vec<TextureArena>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextureArena {
    pub id: u32,
    pub textures: Vec<NativeTexture>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ResidentTextureHandle {
    pub collection_index: usize,
    pub arena_index: usize,
    pub texture_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeTexture {
    pub dictionary_index: u32,
    pub guid: u64,
    pub name: String,
    pub texture_type: TextureType,
    pub endian: TextureEndian,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mip_levels: u32,
    pub pixel_format: u32,
    pub packed_mips: bool,
    pub mip_data_offset: u32,
    /// Original encoded/tiled payload. A native decoder converts this before
    /// upload; no Xbox graphics object crosses this boundary.
    pub encoded_data: Vec<u8>,
    pub linear_levels: Vec<LinearTextureLevel>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextureSceneCounts {
    pub collections: usize,
    pub arenas: usize,
    pub textures: usize,
    pub encoded_bytes: usize,
    pub linear_bytes: usize,
}

impl TextureScene {
    #[must_use]
    pub fn current_collection(&self) -> Option<&TextureCollection> {
        let current_id = self.stream_focus.current_id()?;
        self.collections
            .iter()
            .find(|collection| collection.id == current_id)
    }

    #[must_use]
    pub fn counts(&self) -> TextureSceneCounts {
        TextureSceneCounts {
            collections: self.collections.len(),
            arenas: self
                .collections
                .iter()
                .map(|collection| collection.arenas.len())
                .sum(),
            textures: self
                .collections
                .iter()
                .flat_map(|collection| &collection.arenas)
                .map(|arena| arena.textures.len())
                .sum(),
            encoded_bytes: self
                .collections
                .iter()
                .flat_map(|collection| &collection.arenas)
                .flat_map(|arena| &arena.textures)
                .map(|texture| texture.encoded_data.len())
                .sum(),
            linear_bytes: self
                .collections
                .iter()
                .flat_map(|collection| &collection.arenas)
                .flat_map(|arena| &arena.textures)
                .flat_map(|texture| &texture.linear_levels)
                .map(|level| level.data.len())
                .sum(),
        }
    }

    #[must_use]
    pub fn resident_texture_index(&self) -> BTreeMap<u64, ResidentTextureHandle> {
        let mut index = BTreeMap::new();
        for (collection_index, collection) in self.collections.iter().enumerate() {
            for (arena_index, arena) in collection.arenas.iter().enumerate() {
                for (texture_index, texture) in arena.textures.iter().enumerate() {
                    index.insert(
                        texture.guid,
                        ResidentTextureHandle {
                            collection_index,
                            arena_index,
                            texture_index,
                        },
                    );
                }
            }
        }
        index
    }

    #[must_use]
    pub fn texture(&self, handle: ResidentTextureHandle) -> Option<&NativeTexture> {
        self.collections
            .get(handle.collection_index)?
            .arenas
            .get(handle.arena_index)?
            .textures
            .get(handle.texture_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::stream::FocusCollection;

    #[test]
    fn counts_runtime_owned_texture_payloads() {
        let mut focus = StreamFocusState::new([0.0; 3], false);
        focus.reconcile(&[FocusCollection {
            index: 0,
            id: 7,
            position: [0.0; 3],
            distance_xz: 0.0,
            stream_order: 0,
        }]);
        let scene = TextureScene {
            location_name: "test".to_owned(),
            focus: [0.0; 3],
            stream_focus: focus,
            collections: vec![TextureCollection {
                id: 7,
                name: "cell".to_owned(),
                position: [0.0; 3],
                distance_xz: 0.0,
                arenas: vec![TextureArena {
                    id: 9,
                    textures: vec![NativeTexture {
                        dictionary_index: 1,
                        guid: 11,
                        name: "texture".to_owned(),
                        texture_type: TextureType::TwoDimensional,
                        endian: TextureEndian::None,
                        width: 4,
                        height: 4,
                        depth: 1,
                        mip_levels: 1,
                        pixel_format: 0,
                        packed_mips: false,
                        mip_data_offset: 0,
                        encoded_data: vec![0; 16],
                        linear_levels: vec![LinearTextureLevel {
                            level: 0,
                            face: 0,
                            width: 4,
                            height: 4,
                            width_in_blocks: 4,
                            height_in_blocks: 4,
                            bytes_per_block: 1,
                            row_pitch: 4,
                            data: vec![0; 16],
                        }],
                    }],
                }],
            }],
        };
        assert_eq!(scene.current_collection().unwrap().name, "cell");
        assert_eq!(scene.counts().textures, 1);
        assert_eq!(scene.counts().encoded_bytes, 16);
        assert_eq!(scene.counts().linear_bytes, 16);
    }
}
