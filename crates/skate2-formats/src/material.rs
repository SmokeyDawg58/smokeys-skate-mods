//! Native presentation-material and texture metadata.
//!
//! The layouts and pointer rules come from `tRMaterialData::Fixup` and the
//! render-engine texture serializer. Texture payloads remain original external
//! asset bytes; this module does not expose an Xbox rendering API.

use std::fmt;

use crate::pegasus::ObjectId;

pub const R_MATERIAL_DATA_TYPE_ID: u32 = 0x00eb_0005;
pub const TEXTURE_TYPE_ID: u32 = 0x0002_00e8;
pub const GPU_TEXTURE_RESOURCE_TYPE_ID: u32 = 0x0001_0031;
pub const PIXEL_FORMAT_A8R8G8B8: u32 = 405_275_014;
pub const PIXEL_FORMAT_DXT1: u32 = 438_305_106;
pub const PIXEL_FORMAT_DXT5: u32 = 438_305_108;
/// `renderengine::PixelFormat::LIN_DXT5`: the same BC3 blocks without tiling.
pub const PIXEL_FORMAT_LIN_DXT5: u32 = 0x1a20_0054;
pub const PIXEL_FORMAT_R5G6B5: u32 = 673_710_404;

const MATERIAL_DATA_SIZE: usize = 20;
const MATERIAL_SIZE: usize = 12;
const MATERIAL_SIZE_U32: u32 = 12;
const CHANNEL_SIZE: usize = 32;
const CHANNEL_SIZE_U32: u32 = 32;
const TEXTURE_SIZE: usize = 52;
const CHANNEL_SCALAR_CONSTANT: u16 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct RenderMaterialData {
    pub materials_offset: u32,
    pub channels_offset: u32,
    pub string_list_offset: u32,
    pub materials: Vec<RenderMaterial>,
    pub channels: Vec<MaterialChannel>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderMaterial {
    pub flags: u32,
    pub first_channel_index: usize,
    pub channel_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialChannel {
    pub shader_input: String,
    pub flags: u16,
    pub image_channel: u16,
    pub value: MaterialChannelValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MaterialChannelValue {
    Raster {
        guid: u64,
        stream_name: String,
        serialized_stream_pointer: u32,
    },
    ScalarConstant([f32; 4]),
}

impl RenderMaterialData {
    /// Parses the material, channel, and object-relative string tables.
    ///
    /// # Errors
    ///
    /// Returns [`MaterialError`] for truncated records, invalid channel spans,
    /// or malformed strings.
    pub fn parse(bytes: &[u8]) -> Result<Self, MaterialError> {
        require(bytes, 0, MATERIAL_DATA_SIZE)?;
        let material_count = as_usize(be_u32(bytes, 0)?)?;
        let channel_count = as_usize(be_u32(bytes, 4)?)?;
        let materials_offset = be_u32(bytes, 8)?;
        let channels_offset = be_u32(bytes, 12)?;
        let string_list_offset = be_u32(bytes, 16)?;
        let material_bytes = counted_range(
            bytes,
            as_usize(materials_offset)?,
            material_count,
            MATERIAL_SIZE,
        )?;
        let channel_bytes = counted_range(
            bytes,
            as_usize(channels_offset)?,
            channel_count,
            CHANNEL_SIZE,
        )?;
        require(bytes, as_usize(string_list_offset)?, 0)?;

        let mut channels = Vec::with_capacity(channel_count);
        for index in 0..channel_count {
            let offset = index * CHANNEL_SIZE;
            let flags = be_u16(channel_bytes, offset + 4)?;
            let value = if flags & CHANNEL_SCALAR_CONSTANT != 0 {
                MaterialChannelValue::ScalarConstant(be_f32_array(channel_bytes, offset + 16)?)
            } else {
                MaterialChannelValue::Raster {
                    guid: be_u64(channel_bytes, offset + 16)?,
                    stream_name: object_string(bytes, be_u32(channel_bytes, offset + 24)?)?,
                    serialized_stream_pointer: be_u32(channel_bytes, offset + 28)?,
                }
            };
            channels.push(MaterialChannel {
                shader_input: object_string(bytes, be_u32(channel_bytes, offset)?)?,
                flags,
                image_channel: be_u16(channel_bytes, offset + 6)?,
                value,
            });
        }

        let mut materials = Vec::with_capacity(material_count);
        for index in 0..material_count {
            let offset = index * MATERIAL_SIZE;
            let count = as_usize(be_u32(material_bytes, offset)?)?;
            let channel_offset = be_u32(material_bytes, offset + 8)?;
            let relative = channel_offset
                .checked_sub(channels_offset)
                .ok_or(MaterialError::InvalidChannelOffset(channel_offset))?;
            if relative % CHANNEL_SIZE_U32 != 0 {
                return Err(MaterialError::InvalidChannelOffset(channel_offset));
            }
            let first = as_usize(relative / CHANNEL_SIZE_U32)?;
            if first
                .checked_add(count)
                .is_none_or(|end| end > channels.len())
            {
                return Err(MaterialError::InvalidChannelSpan { first, count });
            }
            materials.push(RenderMaterial {
                flags: be_u32(material_bytes, offset + 4)?,
                first_channel_index: first,
                channel_count: count,
            });
        }
        Ok(Self {
            materials_offset,
            channels_offset,
            string_list_offset,
            materials,
            channels,
        })
    }

    #[must_use]
    pub fn channels_for(&self, material: RenderMaterial) -> &[MaterialChannel] {
        &self.channels
            [material.first_channel_index..material.first_channel_index + material.channel_count]
    }

    /// Resolves an arena subreference offset to an inline material record.
    #[must_use]
    pub fn material_at_object_offset(&self, offset: u32) -> Option<RenderMaterial> {
        let relative = offset.checked_sub(self.materials_offset)?;
        if relative % MATERIAL_SIZE_U32 != 0 {
            return None;
        }
        self.materials
            .get(usize::try_from(relative / MATERIAL_SIZE_U32).ok()?)
            .copied()
    }

    /// Returns the object-relative offset of an inline material record.
    #[must_use]
    pub fn material_object_offset(&self, index: usize) -> Option<u32> {
        u32::try_from(index)
            .ok()?
            .checked_mul(MATERIAL_SIZE_U32)?
            .checked_add(self.materials_offset)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TextureType {
    OneDimensional,
    TwoDimensional,
    TwoDimensionalArray,
    Cube,
    ThreeDimensional,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TextureEndian {
    None,
    EightInSixteen,
    EightInThirtyTwo,
    SixteenInThirtyTwo,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Texture {
    pub data: ObjectId,
    pub texture_type: TextureType,
    pub endian: TextureEndian,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mip_levels: u32,
    pub pixel_format: u32,
    pub packed_mips: bool,
    pub mip_data_offset: u32,
    pub fetch_words: [u32; 6],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinearTextureLevel {
    pub level: u32,
    pub face: u32,
    pub width: u32,
    pub height: u32,
    pub width_in_blocks: u32,
    pub height_in_blocks: u32,
    pub bytes_per_block: u32,
    pub row_pitch: u32,
    pub data: Vec<u8>,
}

impl Texture {
    /// Parses the serialized texture fetch constant using the exact reference
    /// bit extraction performed by `Texture::{GetWidth,GetHeight,GetDepth,
    /// GetFormat}`.
    ///
    /// # Errors
    ///
    /// Returns [`MaterialError`] when the 52-byte record is truncated.
    pub fn parse(bytes: &[u8]) -> Result<Self, MaterialError> {
        require(bytes, 0, TEXTURE_SIZE)?;
        let mut fetch_words = [0_u32; 6];
        for (index, word) in fetch_words.iter_mut().enumerate() {
            *word = be_u32(bytes, 28 + index * 4)?;
        }
        let texture_type = texture_type(fetch_words);
        let dimensions = texture_dimensions(fetch_words, texture_type);
        Ok(Self {
            data: ObjectId(fetch_words[1] >> 12),
            texture_type,
            endian: texture_endian(fetch_words),
            width: dimensions[0],
            height: dimensions[1],
            depth: dimensions[2],
            mip_levels: ((fetch_words[4] >> 6) & 0x0f) + 1,
            pixel_format: texture_pixel_format(fetch_words),
            packed_mips: fetch_words[5] & 0x800 != 0,
            mip_data_offset: fetch_words[5] & 0xffff_f000,
            fetch_words,
        })
    }

    /// Converts the tiled top mip into ordinary row-major blocks.
    ///
    /// BC1/BC3 output remains block-compressed. All output is normalized from
    /// the fetch constant's GPU surface-endian mode into native byte order.
    ///
    /// # Errors
    ///
    /// Returns [`MaterialError`] for unsupported formats/types, arithmetic
    /// overflow, or a payload too small for an addressed source block.
    pub fn decode_top_level(
        self,
        encoded_data: &[u8],
    ) -> Result<LinearTextureLevel, MaterialError> {
        if self.texture_type != TextureType::TwoDimensional {
            return Err(MaterialError::UnsupportedTextureType(self.texture_type));
        }
        self.decode_top_levels(encoded_data)?
            .into_iter()
            .next()
            .ok_or(MaterialError::IntegerOverflow)
    }

    /// Converts every top-level surface into row-major blocks. Two-dimensional
    /// textures produce one surface and cube textures produce six faces.
    ///
    /// # Errors
    ///
    /// Returns [`MaterialError`] for unsupported types/formats or truncated
    /// tiled payloads.
    pub fn decode_top_levels(
        self,
        encoded_data: &[u8],
    ) -> Result<Vec<LinearTextureLevel>, MaterialError> {
        self.decode_levels(encoded_data).map(|levels| {
            levels
                .into_iter()
                .filter(|level| level.level == 0)
                .collect()
        })
    }

    /// Converts every mip level and face into ordinary row-major native blocks.
    /// Independent mip surfaces and the shared packed mip tail are represented
    /// as named level/face records; no GPU fetch state survives this boundary.
    ///
    /// # Errors
    ///
    /// Returns [`MaterialError`] for unsupported types/formats or truncated
    /// tiled payloads.
    pub fn decode_levels(
        self,
        encoded_data: &[u8],
    ) -> Result<Vec<LinearTextureLevel>, MaterialError> {
        let layout = texture_block_layout(self.pixel_format)?;
        if self.pixel_format & 0x100 == 0 {
            return decode_linear_base_level(self, encoded_data, layout).map(|level| vec![level]);
        }
        let face_count = match self.texture_type {
            TextureType::TwoDimensional => 1,
            TextureType::Cube => 6,
            other => return Err(MaterialError::UnsupportedTextureType(other)),
        };
        let tail_first_level = self.packed_mips.then(|| mip_tail_first_level(self));
        let mut source_cursor = if tail_first_level == Some(0) {
            0
        } else {
            usize::try_from(self.mip_data_offset).map_err(|_| MaterialError::IntegerOverflow)?
        };
        let mut levels = Vec::with_capacity(
            usize::try_from(self.mip_levels)
                .ok()
                .and_then(|levels| levels.checked_mul(face_count))
                .ok_or(MaterialError::IntegerOverflow)?,
        );

        if tail_first_level != Some(0) {
            decode_independent_level(self, encoded_data, layout, 0, 0, face_count, &mut levels)?;
        }

        let independent_end = tail_first_level.unwrap_or(self.mip_levels);
        let independent_start = u32::from(independent_end != 0);
        for level in independent_start..independent_end.min(self.mip_levels) {
            let allocation = level_surface_allocation(self, layout, level)?;
            decode_independent_level(
                self,
                encoded_data,
                layout,
                level,
                source_cursor,
                face_count,
                &mut levels,
            )?;
            source_cursor = source_cursor
                .checked_add(
                    allocation
                        .checked_mul(face_count)
                        .ok_or(MaterialError::IntegerOverflow)?,
                )
                .ok_or(MaterialError::IntegerOverflow)?;
        }

        if let Some(tail_level) = tail_first_level.filter(|level| *level < self.mip_levels) {
            let tail_allocation = level_surface_allocation(self, layout, tail_level)?;
            for level in tail_level..self.mip_levels {
                let [origin_x, origin_y] = mip_tail_level_origin(self, layout, tail_level, level)?;
                for face in 0..face_count {
                    let face_offset = tail_allocation
                        .checked_mul(face)
                        .and_then(|offset| source_cursor.checked_add(offset))
                        .ok_or(MaterialError::IntegerOverflow)?;
                    levels.push(decode_level_surface(
                        self,
                        encoded_data,
                        layout,
                        level,
                        u32::try_from(face).map_err(|_| MaterialError::IntegerOverflow)?,
                        face_offset,
                        tail_allocation,
                        [origin_x, origin_y],
                    )?);
                }
            }
        }

        levels.sort_by_key(|level| (level.level, level.face));
        Ok(levels)
    }
}

#[derive(Clone, Copy)]
struct TextureBlockLayout {
    width: u32,
    height: u32,
    bytes: u32,
}

/// Original boot `logo_watermark.Texture` is an unbordered, single-level
/// linear 2D surface. GetSurfaceLayout82c2ef30 (82c2f048) obtains its texel
/// pitch from fetch.Pitch * 32; GetTextureDesc82c2f188 multiplies the byte
/// pitch by the block height at82c2f598. Do not pass these rows to the tiler.
fn decode_linear_base_level(
    texture: Texture,
    source: &[u8],
    layout: TextureBlockLayout,
) -> Result<LinearTextureLevel, MaterialError> {
    if texture.texture_type != TextureType::TwoDimensional
        || texture.mip_levels != 1
        || texture.packed_mips
        || texture.mip_data_offset != 0
        || texture.fetch_words[1] & 0x400 != 0
        || texture.fetch_words[3] & 0x8000_0000 != 0
    {
        return Err(MaterialError::UnsupportedLinearTextureLayout);
    }
    let width_in_blocks = texture.width.div_ceil(layout.width);
    let height_in_blocks = texture.height.div_ceil(layout.height);
    let pitch_texels = ((texture.fetch_words[0] >> 22) & 0x1ff) * 32;
    if pitch_texels < texture.width || pitch_texels % layout.width != 0 {
        return Err(MaterialError::UnsupportedLinearTextureLayout);
    }
    let source_pitch = (pitch_texels / layout.width)
        .checked_mul(layout.bytes)
        .ok_or(MaterialError::IntegerOverflow)?;
    let row_pitch = width_in_blocks
        .checked_mul(layout.bytes)
        .ok_or(MaterialError::IntegerOverflow)?;
    let size = row_pitch
        .checked_mul(height_in_blocks)
        .ok_or(MaterialError::IntegerOverflow)?;
    let mut data = Vec::with_capacity(as_usize(size)?);
    for y in 0..height_in_blocks {
        let offset = as_usize(y.checked_mul(source_pitch).ok_or(MaterialError::IntegerOverflow)?)?;
        let size = as_usize(row_pitch)?;
        let end = offset.checked_add(size).ok_or(MaterialError::IntegerOverflow)?;
        let row = source.get(offset..end).ok_or(MaterialError::TexturePayloadTooSmall {
            offset,
            size,
            actual: source.len(),
        })?;
        data.extend_from_slice(row);
    }
    normalize_surface_endian(&mut data, texture.endian);
    Ok(LinearTextureLevel {
        level: 0,
        face: 0,
        width: texture.width,
        height: texture.height,
        width_in_blocks,
        height_in_blocks,
        bytes_per_block: layout.bytes,
        row_pitch,
        data,
    })
}

fn decode_independent_level(
    texture: Texture,
    encoded_data: &[u8],
    layout: TextureBlockLayout,
    level: u32,
    source_offset: usize,
    face_count: usize,
    levels: &mut Vec<LinearTextureLevel>,
) -> Result<(), MaterialError> {
    let allocation = level_surface_allocation(texture, layout, level)?;
    for face in 0..face_count {
        let face_offset = allocation
            .checked_mul(face)
            .and_then(|offset| source_offset.checked_add(offset))
            .ok_or(MaterialError::IntegerOverflow)?;
        levels.push(decode_level_surface(
            texture,
            encoded_data,
            layout,
            level,
            u32::try_from(face).map_err(|_| MaterialError::IntegerOverflow)?,
            face_offset,
            allocation,
            [0, 0],
        )?);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn decode_level_surface(
    texture: Texture,
    encoded_data: &[u8],
    layout: TextureBlockLayout,
    level: u32,
    face: u32,
    source_offset: usize,
    allocation: usize,
    origin: [u32; 2],
) -> Result<LinearTextureLevel, MaterialError> {
    let width = (texture.width >> level).max(1);
    let height = (texture.height >> level).max(1);
    let width_in_blocks = width.div_ceil(layout.width);
    let height_in_blocks = height.div_ceil(layout.height);
    let source =
        encoded_data
            .get(source_offset..)
            .ok_or(MaterialError::TexturePayloadTooSmall {
                offset: source_offset,
                size: allocation,
                actual: encoded_data.len(),
            })?;
    let mut decoded = untile_surface_region(
        source,
        width_in_blocks,
        height_in_blocks,
        layout.bytes,
        origin,
    )?;
    decoded.level = level;
    decoded.face = face;
    decoded.width = width;
    decoded.height = height;
    normalize_surface_endian(&mut decoded.data, texture.endian);
    Ok(decoded)
}

fn level_surface_allocation(
    texture: Texture,
    layout: TextureBlockLayout,
    level: u32,
) -> Result<usize, MaterialError> {
    let width = (texture.width >> level).max(1).div_ceil(layout.width);
    let height = (texture.height >> level).max(1).div_ceil(layout.height);
    tiled_surface_allocation(width, height, layout.bytes)
}

fn mip_tail_first_level(texture: Texture) -> u32 {
    ceil_log2(texture.width)
        .min(ceil_log2(texture.height))
        .saturating_sub(4)
        .min(texture.mip_levels.saturating_sub(1))
}

fn mip_tail_level_origin(
    texture: Texture,
    layout: TextureBlockLayout,
    tail_first_level: u32,
    level: u32,
) -> Result<[u32; 2], MaterialError> {
    let relative_level = level
        .checked_sub(tail_first_level)
        .ok_or(MaterialError::IntegerOverflow)?;
    let tail_width = 1_u32
        .checked_shl(ceil_log2(texture.width).saturating_sub(tail_first_level))
        .ok_or(MaterialError::IntegerOverflow)?;
    let tail_height = 1_u32
        .checked_shl(ceil_log2(texture.height).saturating_sub(tail_first_level))
        .ok_or(MaterialError::IntegerOverflow)?;
    let width_is_strictly_major = tail_width > tail_height;
    let mut origin = [0_u32; 2];
    if relative_level < 3 {
        let coordinate = 16 >> relative_level;
        origin[usize::from(width_is_strictly_major)] = coordinate;
    } else {
        let coordinate = if width_is_strictly_major {
            tail_height
        } else {
            tail_width
        } >> (relative_level - 2);
        origin[usize::from(!width_is_strictly_major)] = coordinate;
    }
    origin[0] /= layout.width;
    origin[1] /= layout.height;
    Ok(origin)
}

fn ceil_log2(value: u32) -> u32 {
    u32::BITS - value.saturating_sub(1).leading_zeros()
}

fn texture_block_layout(pixel_format: u32) -> Result<TextureBlockLayout, MaterialError> {
    let (width, height, bytes) = match pixel_format {
        PIXEL_FORMAT_A8R8G8B8 => (1, 1, 4),
        PIXEL_FORMAT_DXT1 => (4, 4, 8),
        PIXEL_FORMAT_DXT5 | PIXEL_FORMAT_LIN_DXT5 => (4, 4, 16),
        PIXEL_FORMAT_R5G6B5 => (1, 1, 2),
        _ => return Err(MaterialError::UnsupportedTextureFormat(pixel_format)),
    };
    Ok(TextureBlockLayout {
        width,
        height,
        bytes,
    })
}

#[cfg(test)]
fn untile_surface(
    source: &[u8],
    width: u32,
    height: u32,
    bytes_per_block: u32,
) -> Result<LinearTextureLevel, MaterialError> {
    untile_surface_region(source, width, height, bytes_per_block, [0, 0])
}

fn untile_surface_region(
    source: &[u8],
    width: u32,
    height: u32,
    bytes_per_block: u32,
    origin: [u32; 2],
) -> Result<LinearTextureLevel, MaterialError> {
    let row_pitch = width
        .checked_mul(bytes_per_block)
        .ok_or(MaterialError::IntegerOverflow)?;
    let output_size = row_pitch
        .checked_mul(height)
        .and_then(|size| usize::try_from(size).ok())
        .ok_or(MaterialError::IntegerOverflow)?;
    let mut data = vec![0; output_size];
    for y in 0..height {
        for x in 0..width {
            let source_offset = tiled_surface_offset(
                x.checked_add(origin[0])
                    .ok_or(MaterialError::IntegerOverflow)?,
                y.checked_add(origin[1])
                    .ok_or(MaterialError::IntegerOverflow)?,
                width,
                bytes_per_block,
            )?;
            let destination_offset = y
                .checked_mul(row_pitch)
                .and_then(|offset| {
                    x.checked_mul(bytes_per_block)
                        .and_then(|x| offset.checked_add(x))
                })
                .and_then(|offset| usize::try_from(offset).ok())
                .ok_or(MaterialError::IntegerOverflow)?;
            let block_size =
                usize::try_from(bytes_per_block).map_err(|_| MaterialError::IntegerOverflow)?;
            let source_block = source
                .get(source_offset..source_offset + block_size)
                .ok_or(MaterialError::TexturePayloadTooSmall {
                    offset: source_offset,
                    size: block_size,
                    actual: source.len(),
                })?;
            data[destination_offset..destination_offset + block_size].copy_from_slice(source_block);
        }
    }
    Ok(LinearTextureLevel {
        level: 0,
        face: 0,
        width,
        height,
        width_in_blocks: width,
        height_in_blocks: height,
        bytes_per_block,
        row_pitch,
        data,
    })
}

/// Converts the four Xbox GPU surface-endian modes into the byte order used by
/// native texture APIs. This is deliberately expressed in terms of components,
/// not platform instructions or an emulated fetch unit.
fn normalize_surface_endian(data: &mut [u8], endian: TextureEndian) {
    match endian {
        TextureEndian::None => {}
        TextureEndian::EightInSixteen => {
            for word in data.chunks_exact_mut(2) {
                word.swap(0, 1);
            }
        }
        TextureEndian::EightInThirtyTwo => {
            for word in data.chunks_exact_mut(4) {
                word.reverse();
            }
        }
        TextureEndian::SixteenInThirtyTwo => {
            for word in data.chunks_exact_mut(4) {
                word.swap(0, 2);
                word.swap(1, 3);
            }
        }
    }
}

fn tiled_surface_allocation(
    width: u32,
    height: u32,
    bytes_per_block: u32,
) -> Result<usize, MaterialError> {
    let aligned_width = width
        .checked_add(31)
        .map(|value| value & !31)
        .ok_or(MaterialError::IntegerOverflow)?;
    let aligned_height = height
        .checked_add(31)
        .map(|value| value & !31)
        .ok_or(MaterialError::IntegerOverflow)?;
    aligned_width
        .checked_mul(aligned_height)
        .and_then(|blocks| blocks.checked_mul(bytes_per_block))
        .and_then(|bytes| bytes.checked_add(4095))
        .map(|bytes| bytes & !4095)
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or(MaterialError::IntegerOverflow)
}

/// Byte offset of a block in the Xbox tiled 2D surface layout. This is the
/// closed-form address decomposition used by the reference `UntileSurface`:
/// 32x32 macro tiles, 8x8 micro tiles, then memory-bank interleaving.
fn tiled_surface_offset(
    x: u32,
    y: u32,
    width: u32,
    bytes_per_block: u32,
) -> Result<usize, MaterialError> {
    if !matches!(bytes_per_block, 2 | 4 | 8 | 16) {
        return Err(MaterialError::UnsupportedTextureBlockSize(bytes_per_block));
    }
    let aligned_width = width
        .checked_add(31)
        .map(|value| value & !31)
        .ok_or(MaterialError::IntegerOverflow)?;
    let log_bytes = bytes_per_block.trailing_zeros();
    let tile_group_base = ((x >> 5) + (y >> 5) * (aligned_width >> 5)) << (log_bytes + 7);
    let intra_tile = ((x & 7) + ((y & 6) << 2)) << log_bytes;
    let tiled = tile_group_base
        + ((intra_tile & !0x0f) << 1)
        + (intra_tile & 0x0f)
        + ((y & 8) << (3 + log_bytes))
        + ((y & 1) << 4);
    let bank = ((((y & 8) >> 2) + (x >> 3)) & 3) << 6;
    let offset = ((tiled & !0x01ff) << 3)
        + ((tiled & 0x01c0) << 2)
        + (tiled & 0x003f)
        + ((y & 16) << 7)
        + bank;
    usize::try_from(offset).map_err(|_| MaterialError::IntegerOverflow)
}

fn texture_type(words: [u32; 6]) -> TextureType {
    match (words[5] >> 9) & 3 {
        0 => TextureType::OneDimensional,
        1 if words[1] & 0x400 == 0 => TextureType::TwoDimensional,
        1 => TextureType::TwoDimensionalArray,
        2 => TextureType::ThreeDimensional,
        3 => TextureType::Cube,
        _ => unreachable!(),
    }
}

fn texture_endian(words: [u32; 6]) -> TextureEndian {
    match (words[1] >> 6) & 3 {
        0 => TextureEndian::None,
        1 => TextureEndian::EightInSixteen,
        2 => TextureEndian::EightInThirtyTwo,
        3 => TextureEndian::SixteenInThirtyTwo,
        _ => unreachable!(),
    }
}

fn texture_dimensions(words: [u32; 6], texture_type: TextureType) -> [u32; 3] {
    match texture_type {
        TextureType::OneDimensional => [(words[2] & 0x00ff_ffff) + 1, 1, 1],
        TextureType::TwoDimensional | TextureType::TwoDimensionalArray | TextureType::Cube => {
            [(words[2] & 0x1fff) + 1, ((words[2] >> 13) & 0x1fff) + 1, 1]
        }
        TextureType::ThreeDimensional => [
            (words[2] & 0x07ff) + 1,
            ((words[2] >> 11) & 0x07ff) + 1,
            (words[2] >> 22) + 1,
        ],
    }
}

fn texture_pixel_format(words: [u32; 6]) -> u32 {
    let mut value = (words[3] >> 7) & 0x3f;
    value = (value << 3) | ((words[3] >> 4) & 7);
    value = (value << 3) | ((words[3] >> 1) & 7);
    value = (value << 1) | (words[3] & 1);
    value = (value << 2) | ((words[0] >> 8) & 3);
    value = (value << 2) | ((words[0] >> 6) & 3);
    value = (value << 2) | ((words[0] >> 4) & 3);
    value = (value << 2) | ((words[0] >> 2) & 3);
    value = (value << 1) | (words[0] >> 31);
    value = (value << 2) | ((words[1] >> 6) & 3);
    (value << 6) | (words[1] & 0x3f)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaterialError {
    Truncated {
        offset: usize,
        required: usize,
        actual: usize,
    },
    IntegerOverflow,
    InvalidChannelOffset(u32),
    InvalidChannelSpan {
        first: usize,
        count: usize,
    },
    UnterminatedString {
        offset: usize,
    },
    InvalidUtf8 {
        offset: usize,
    },
    UnsupportedTextureFormat(u32),
    UnsupportedTextureType(TextureType),
    UnsupportedTextureBlockSize(u32),
    UnsupportedLinearTextureLayout,
    TexturePayloadTooSmall {
        offset: usize,
        size: usize,
        actual: usize,
    },
}

impl fmt::Display for MaterialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Skate 2 material object: {self:?}")
    }
}

impl std::error::Error for MaterialError {}

fn object_string(bytes: &[u8], offset: u32) -> Result<String, MaterialError> {
    let start = as_usize(offset)?;
    let tail = bytes.get(start..).ok_or(MaterialError::Truncated {
        offset: start,
        required: start,
        actual: bytes.len(),
    })?;
    let length = tail
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(MaterialError::UnterminatedString { offset: start })?;
    std::str::from_utf8(&tail[..length])
        .map(str::to_owned)
        .map_err(|_| MaterialError::InvalidUtf8 { offset: start })
}

fn counted_range(
    bytes: &[u8],
    start: usize,
    count: usize,
    stride: usize,
) -> Result<&[u8], MaterialError> {
    let size = count
        .checked_mul(stride)
        .ok_or(MaterialError::IntegerOverflow)?;
    require(bytes, start, size)
}

fn require(bytes: &[u8], offset: usize, size: usize) -> Result<&[u8], MaterialError> {
    let end = offset
        .checked_add(size)
        .ok_or(MaterialError::IntegerOverflow)?;
    bytes.get(offset..end).ok_or(MaterialError::Truncated {
        offset,
        required: end,
        actual: bytes.len(),
    })
}

fn as_usize(value: u32) -> Result<usize, MaterialError> {
    usize::try_from(value).map_err(|_| MaterialError::IntegerOverflow)
}

fn be_u16(bytes: &[u8], offset: usize) -> Result<u16, MaterialError> {
    require(bytes, offset, 2).map(|field| u16::from_be_bytes(field.try_into().unwrap()))
}

fn be_u32(bytes: &[u8], offset: usize) -> Result<u32, MaterialError> {
    require(bytes, offset, 4).map(|field| u32::from_be_bytes(field.try_into().unwrap()))
}

fn be_u64(bytes: &[u8], offset: usize) -> Result<u64, MaterialError> {
    require(bytes, offset, 8).map(|field| u64::from_be_bytes(field.try_into().unwrap()))
}

fn be_f32_array(bytes: &[u8], offset: usize) -> Result<[f32; 4], MaterialError> {
    let mut values = [0.0; 4];
    for (index, value) in values.iter_mut().enumerate() {
        *value = f32::from_bits(be_u32(bytes, offset + index * 4)?);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    #[test]
    fn parses_material_channel_spans_and_values() {
        let mut bytes = vec![0; 104];
        put_u32(&mut bytes, 0, 1);
        put_u32(&mut bytes, 4, 2);
        put_u32(&mut bytes, 8, 20);
        put_u32(&mut bytes, 12, 32);
        put_u32(&mut bytes, 16, 96);
        put_u32(&mut bytes, 20, 2);
        put_u32(&mut bytes, 28, 32);
        put_u32(&mut bytes, 32, 96);
        put_u32(&mut bytes, 32 + 24, 100);
        put_u32(&mut bytes, 64, 96);
        put_u16(&mut bytes, 64 + 4, CHANNEL_SCALAR_CONSTANT);
        put_u32(&mut bytes, 64 + 16, 1.0_f32.to_bits());
        bytes[96..].copy_from_slice(b"in\0tex\0\0");

        let data = RenderMaterialData::parse(&bytes).unwrap();
        assert_eq!(data.materials[0].channel_count, 2);
        assert_eq!(data.material_object_offset(0), Some(20));
        assert_eq!(data.material_at_object_offset(20), Some(data.materials[0]));
        assert_eq!(data.material_at_object_offset(21), None);
        assert!(matches!(
            data.channels[0].value,
            MaterialChannelValue::Raster { .. }
        ));
        assert_eq!(
            data.channels[1].value,
            MaterialChannelValue::ScalarConstant([1.0, 0.0, 0.0, 0.0])
        );
    }

    #[test]
    fn decodes_reference_texture_fields() {
        let mut bytes = vec![0; TEXTURE_SIZE];
        put_u32(&mut bytes, 32, (7 << 12) | 0x12);
        put_u32(&mut bytes, 36, 255 | (127 << 13));
        put_u32(&mut bytes, 44, 3 << 6);
        put_u32(&mut bytes, 48, 1 << 9);
        let texture = Texture::parse(&bytes).unwrap();
        assert_eq!(texture.data, ObjectId(7));
        assert_eq!(texture.texture_type, TextureType::TwoDimensional);
        assert_eq!(texture.endian, TextureEndian::None);
        assert_eq!(
            [texture.width, texture.height, texture.depth],
            [256, 128, 1]
        );
        assert_eq!(texture.mip_levels, 4);
    }

    #[test]
    fn normalizes_gpu_endian_modes() {
        let original = [0, 1, 2, 3, 4, 5, 6, 7];
        let cases = [
            (TextureEndian::None, [0, 1, 2, 3, 4, 5, 6, 7]),
            (TextureEndian::EightInSixteen, [1, 0, 3, 2, 5, 4, 7, 6]),
            (TextureEndian::EightInThirtyTwo, [3, 2, 1, 0, 7, 6, 5, 4]),
            (TextureEndian::SixteenInThirtyTwo, [2, 3, 0, 1, 6, 7, 4, 5]),
        ];
        for (endian, expected) in cases {
            let mut actual = original;
            normalize_surface_endian(&mut actual, endian);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn decodes_every_surface_from_a_packed_bc1_mip_tail() {
        let texture = Texture {
            data: ObjectId(0),
            texture_type: TextureType::TwoDimensional,
            endian: TextureEndian::None,
            width: 16,
            height: 16,
            depth: 1,
            mip_levels: 5,
            pixel_format: PIXEL_FORMAT_DXT1,
            packed_mips: true,
            mip_data_offset: 0,
            fetch_words: [0; 6],
        };
        let layout = texture_block_layout(texture.pixel_format).unwrap();
        let mut encoded = vec![0_u8; 8192];
        let mut occupied = std::collections::BTreeSet::new();
        for level in 0..texture.mip_levels {
            let origin = mip_tail_level_origin(texture, layout, 0, level).unwrap();
            let width = (texture.width >> level).max(1).div_ceil(layout.width);
            let height = (texture.height >> level).max(1).div_ceil(layout.height);
            for y in 0..height {
                for x in 0..width {
                    let offset =
                        tiled_surface_offset(origin[0] + x, origin[1] + y, width, layout.bytes)
                            .unwrap();
                    assert!(occupied.insert(offset));
                    encoded[offset..offset + 8].fill(u8::try_from(level + 1).unwrap());
                }
            }
        }
        let levels = texture.decode_levels(&encoded).unwrap();
        assert_eq!(levels.len(), 5);
        for (level, decoded) in levels.iter().enumerate() {
            assert_eq!(decoded.level, u32::try_from(level).unwrap());
            assert_eq!(decoded.face, 0);
            assert!(
                decoded
                    .data
                    .iter()
                    .all(|byte| *byte == u8::try_from(level + 1).unwrap())
            );
        }
    }

    #[test]
    fn packed_mip_tail_origins_follow_reference_axis_rules() {
        let square = Texture {
            data: ObjectId(0),
            texture_type: TextureType::TwoDimensional,
            endian: TextureEndian::None,
            width: 256,
            height: 256,
            depth: 1,
            mip_levels: 9,
            pixel_format: PIXEL_FORMAT_DXT1,
            packed_mips: true,
            mip_data_offset: 0x8000,
            fetch_words: [0; 6],
        };
        let layout = texture_block_layout(square.pixel_format).unwrap();
        assert_eq!(mip_tail_first_level(square), 4);
        let origins = (4..9)
            .map(|level| mip_tail_level_origin(square, layout, 4, level).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(origins, [[4, 0], [2, 0], [1, 0], [0, 2], [0, 1]]);

        let wide = Texture {
            width: 256,
            height: 128,
            ..square
        };
        assert_eq!(mip_tail_first_level(wide), 3);
        assert_eq!(mip_tail_level_origin(wide, layout, 3, 3).unwrap(), [0, 4]);
    }

    #[test]
    fn untile_reconstructs_linear_block_order() {
        let mut tiled = vec![0_u8; 32 * 32 * 4];
        for y in 0..32 {
            for x in 0..32 {
                let offset = tiled_surface_offset(x, y, 32, 4).unwrap();
                let value = (y * 32 + x).to_be_bytes();
                tiled[offset..offset + 4].copy_from_slice(&value);
            }
        }
        let linear = untile_surface(&tiled, 32, 32, 4).unwrap();
        for (index, block) in linear.data.chunks_exact(4).enumerate() {
            assert_eq!(
                u32::from_be_bytes(block.try_into().unwrap()),
                u32::try_from(index).unwrap()
            );
        }
    }

    #[test]
    fn tiled_offsets_are_a_bijection_for_supported_block_sizes() {
        for bytes in [2, 4, 8, 16] {
            let mut offsets = std::collections::BTreeSet::new();
            for y in 0..32 {
                for x in 0..32 {
                    offsets.insert(tiled_surface_offset(x, y, 32, bytes).unwrap());
                }
            }
            assert_eq!(offsets.len(), 32 * 32);
            assert!(offsets.iter().all(|offset| offset % bytes as usize == 0));
            let allocation = (32 * 32 * usize::try_from(bytes).unwrap()).next_multiple_of(4096);
            assert!(offsets.iter().all(|offset| *offset < allocation));
        }
    }
}
