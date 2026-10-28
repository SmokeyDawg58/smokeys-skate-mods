//! Integer `RAW_DISCRETIZE` fetch for the original face channel selectors.
//!
//! Reference: `Vector_ExtractPackedN` 0x82d6f838. This is the integer
//! extraction stage, not `Clip::SetFrame`, ACS time quantization or interpolation.
use super::{
    AndaleClip, AndaleClipPart, AndaleCompressionHeader, AndaleError, AndaleHierarchy, AndaleSqt,
    decompress_raw_packed_quaternion, identity_sqt, read_be_f32, read_be_u32,
};
use std::fmt;

#[cfg(test)]
#[path = "andale_raw_tests.rs"]
mod tests;

#[derive(Debug, PartialEq)]
pub enum AndaleRawError {
    Format(AndaleError),
    UnsupportedSelectors,
    BoneCount { part: usize, header: usize },
    MaskOutsideBones,
    Stride { declared: usize, channels: usize },
    PayloadFrames { bytes: usize, stride: usize },
    Frame { requested: usize, count: usize },
    PartFrameCount { clip: u32, part: usize },
    UnsupportedHierarchyCoverage,
    UnsupportedDeltaChannels,
}
impl From<AndaleError> for AndaleRawError {
    fn from(value: AndaleError) -> Self {
        Self::Format(value)
    }
}
impl fmt::Display for AndaleRawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RAW_DISCRETIZE: {self:?}")
    }
}
impl std::error::Error for AndaleRawError {}

impl AndaleClipPart {
    /// Reference: 0x82d6fbd0 / 0x82d6fd24 select `data + stride * frame`.
    /// Masks 0/2/4 select interleaved float3 scale, rotation and float3
    /// translation. Selector+28 chooses float4 (0) or packed-u32 (1) rotation.
    ///
    /// # Errors
    /// Rejects other selectors, inconsistent masks/stride and absent frames.
    pub fn sample_raw_frame_sqt(&self, frame: usize) -> Result<Vec<AndaleSqt>, AndaleRawError> {
        let AndaleCompressionHeader::RawDiscretize(header) = &self.compression else {
            return Err(AndaleError::NotRawDiscretizeCompression.into());
        };
        if !matches!(header.format_flags, [0, 0] | [0, 1])
            || header.channel_masks[1] != 0
            || header.channel_masks[3] != 0
            || header.channel_masks[5] != 0
        {
            return Err(AndaleRawError::UnsupportedSelectors);
        }
        let bones = self.bone_count();
        let header_bones = usize::from(header.last_bone_index) + 1;
        if bones != header_bones || bones > 32 {
            return Err(AndaleRawError::BoneCount {
                part: bones,
                header: header_bones,
            });
        }
        let valid = u32::MAX >> (32 - bones);
        if header.channel_masks.iter().any(|mask| mask & !valid != 0) {
            return Err(AndaleRawError::MaskOutsideBones);
        }
        let [scale, _, rotation, _, translation, _] = header.channel_masks;
        let channels = (scale.count_ones() * 12
            + rotation.count_ones() * if header.format_flags[1] == 0 { 16 } else { 4 }
            + translation.count_ones() * 12) as usize;
        let stride = usize::from(header.frame_stride);
        if stride != channels {
            return Err(AndaleRawError::Stride {
                declared: stride,
                channels,
            });
        }
        // Empty authored parts have zero stride: every selected frame retains
        // the extractor's initialized SQT identity without reading a payload.
        if stride == 0 && self.compressed_data.is_empty() {
            return Ok(vec![identity_sqt(); bones]);
        }
        if stride == 0 || self.compressed_data.len() % stride != 0 {
            return Err(AndaleRawError::PayloadFrames {
                bytes: self.compressed_data.len(),
                stride,
            });
        }
        let count = self.compressed_data.len() / stride;
        if frame >= count {
            return Err(AndaleRawError::Frame {
                requested: frame,
                count,
            });
        }
        Ok(decode_raw_channels(
            &self.compressed_data,
            frame * stride,
            bones,
            [scale, rotation, translation],
            header.format_flags[1] != 0,
        )?)
    }
}

// Shared by integer clip frames and static bind poses. Callers validate the
// selectors, masks and byte count before selecting this RAW layout.
pub(super) fn decode_raw_channels(
    bytes: &[u8],
    mut cursor: usize,
    bones: usize,
    [scale, rotation, translation]: [u32; 3],
    packed_rotation: bool,
) -> Result<Vec<AndaleSqt>, AndaleError> {
    let mut pose = vec![identity_sqt(); bones];
    for (bone, sqt) in pose.iter_mut().enumerate() {
        if scale & (1 << bone) != 0 {
            sqt.scale = read_float3(bytes, cursor, 1.0)?;
            cursor += 12;
        }
        if rotation & (1 << bone) != 0 {
            // Vector_ExtractPackedN82d6fc60 selects 16 or 4 bytes. The
            // unpacked path copies XYZW, without quaternion normalization.
            if packed_rotation {
                sqt.rotation = decompress_raw_packed_quaternion(read_be_u32(bytes, cursor)?);
                cursor += 4;
            } else {
                sqt.rotation = [read_be_f32(bytes, cursor)?, read_be_f32(bytes, cursor + 4)?,
                    read_be_f32(bytes, cursor + 8)?, read_be_f32(bytes, cursor + 12)?];
                cursor += 16;
            }
        }
        if translation & (1 << bone) != 0 {
            sqt.translation = read_float3(bytes, cursor, 1.0)?;
            cursor += 12;
        }
    }
    Ok(pose)
}

fn read_float3(bytes: &[u8], cursor: usize, w: f32) -> Result<[f32; 4], AndaleError> {
    Ok([
        read_be_f32(bytes, cursor)?,
        read_be_f32(bytes, cursor + 4)?,
        read_be_f32(bytes, cursor + 8)?,
        w,
    ])
}

impl AndaleClip {
    /// Materializes an integer RAW frame in retained hierarchy bone order.
    /// This requires the full ordered part set, as authored by the face files.
    ///
    /// # Errors
    /// Rejects incomplete hierarchies, inconsistent frame counts and unsupported
    /// RAW layouts; it does not manufacture missing transforms.
    pub fn sample_raw_frame_sqt(
        &self,
        hierarchy: &AndaleHierarchy,
        frame: usize,
    ) -> Result<Vec<AndaleSqt>, AndaleRawError> {
        if self.flags & (1 << 29) != 0 {
            return Err(AndaleRawError::UnsupportedDeltaChannels);
        }
        if frame as f64 >= f64::from(self.frame_count) {
            return Err(AndaleRawError::Frame { requested: frame, count: self.frame_count as usize });
        }
        if self.parts.len() != hierarchy.parts.len() {
            return Err(AndaleError::ClipHierarchyPartCountMismatch {
                clip: self.parts.len(),
                hierarchy: hierarchy.parts.len(),
            }
            .into());
        }
        let mut next_bone = 0_u32;
        for (part, hierarchy_part) in self.parts.iter().zip(&hierarchy.parts) {
            if hierarchy_part.bone_offset != next_bone
                || u16::from(part.part_name_index) != hierarchy_part.name_index
            {
                return Err(AndaleRawError::UnsupportedHierarchyCoverage);
            }
            next_bone = next_bone
                .checked_add(hierarchy_part.bone_count)
                .ok_or(AndaleError::IntegerOverflow)?;
        }
        if usize::try_from(next_bone).ok() != Some(hierarchy.bone_names.len()) {
            return Err(AndaleRawError::UnsupportedHierarchyCoverage);
        }
        let mut pose = vec![identity_sqt(); hierarchy.bone_names.len()];
        for (index, (part, hierarchy_part)) in self.parts.iter().zip(&hierarchy.parts).enumerate() {
            let decoded = part.sample_raw_frame_sqt(frame)?;
            let AndaleCompressionHeader::RawDiscretize(header) = &part.compression else {
                unreachable!("sampler already validated RAW header");
            };
            let count = if header.frame_stride == 0 { self.frame_count as usize }
                else { part.compressed_data.len() / usize::from(header.frame_stride) };
            let count_u32 = u32::try_from(count).map_err(|_| AndaleError::IntegerOverflow)?;
            if f64::from(count_u32).to_bits() != f64::from(self.frame_count).to_bits() {
                return Err(AndaleRawError::PartFrameCount {
                    clip: self.frame_count.to_bits(),
                    part: count,
                });
            }
            let expected = usize::try_from(hierarchy_part.bone_count)
                .map_err(|_| AndaleError::IntegerOverflow)?;
            if decoded.len() != expected {
                return Err(AndaleError::ClipHierarchyPartMismatch {
                    index,
                    clip_bones: decoded.len(),
                    hierarchy_bones: expected,
                }
                .into());
            }
            let start = usize::try_from(hierarchy_part.bone_offset)
                .map_err(|_| AndaleError::IntegerOverflow)?;
            let end = start
                .checked_add(decoded.len())
                .ok_or(AndaleError::IntegerOverflow)?;
            let target = pose
                .get_mut(start..end)
                .ok_or(AndaleError::ClipPartOutsideHierarchy {
                    index,
                    offset: start,
                    bones: decoded.len(),
                    hierarchy_bones: hierarchy.bone_names.len(),
                })?;
            target.copy_from_slice(&decoded);
        }
        Ok(pose)
    }
}
