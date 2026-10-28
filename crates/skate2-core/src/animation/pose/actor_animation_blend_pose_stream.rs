//! Native hand/face stream tokens from the actual retained animation tree.
//! Registration is under `actor_face_playback` so clip bytes retain their owner.
use super::FaceDatabase;
use crate::{
    actor_animation_tree::{
        AnimationNodeHandle, AnimationNodeKind, AnimationTreeError, AnimationTreePool,
    },
    actor_face_playback::tree_database::clip_fetch::timing::{FetchTimingError, low_integer_word},
    play_animation::FastString36,
};

const STREAMS: usize = 3;
const TOKENS: usize = 8000;
const COEFFICIENT_SCALE: f32 = 1_048_575.0;

#[cfg(test)]
#[path = "actor_animation_blend_pose_stream_tests.rs"]
mod tests;

#[derive(Debug)]
pub(crate) enum BlendPoseError {
    Tree(AnimationTreeError),
    Numeric(FetchTimingError),
    Missing(&'static str),
    Capacity,
    InvalidTokenStack,
    PoseNodeAssertion,
    SourceRange,
}
impl From<AnimationTreeError> for BlendPoseError {
    fn from(value: AnimationTreeError) -> Self {
        Self::Tree(value)
    }
}
impl std::fmt::Display for BlendPoseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tree(e) => write!(f, "blend-pose tree: {e}"),
            Self::Numeric(e) => write!(f, "blend-pose token: {e:?}"),
            Self::Missing(owner) => write!(f, "blend-pose requires {owner}"),
            other => write!(f, "blend-pose: {other:?}"),
        }
    }
}

/// Actual two pools of three 8000-token lanes. A buffer retains a lane slice,
/// while the pool tops remain shared through recursive alternating buffers.
pub(crate) struct BlendPoseTokenPool {
    lanes: [[Vec<u32>; STREAMS]; 2],
    tops: [[usize; STREAMS]; 2],
}
pub(crate) struct BlendPoseBuffer {
    names: [[FastString36; STREAMS]; 2],
    name_index: usize,
    pub(crate) pool_index: usize,
    count: usize,
    starts: [usize; STREAMS],
    lengths: [usize; STREAMS],
}
impl BlendPoseTokenPool {
    pub(crate) fn new() -> Self {
        Self {
            lanes: std::array::from_fn(|_| std::array::from_fn(|_| vec![0; TOKENS])),
            tops: [[0; STREAMS]; 2],
        }
    }
    /// DispatchHandsFaceAnim825f8864..887c and8a9c..8ab4 reset allocation tops.
    pub(crate) fn reset(&mut self) {
        self.tops = [[0; STREAMS]; 2];
    }

    /// Constructor825f7738 and Init825efb58; no pose or tokens are synthesized.
    pub(crate) fn buffer(&self, pool_index: usize) -> Result<BlendPoseBuffer, BlendPoseError> {
        let starts = *self.tops.get(pool_index).ok_or(BlendPoseError::Capacity)?;
        Ok(BlendPoseBuffer {
            names: [[FastString36 { words: [0; 6] }; STREAMS]; 2],
            name_index: 0,
            pool_index,
            count: 0,
            starts,
            lengths: [0; STREAMS],
        })
    }
    pub(crate) fn tokens<'a>(
        &'a self,
        buffer: &BlendPoseBuffer,
        stream: usize,
    ) -> Result<&'a [u32], BlendPoseError> {
        if stream >= STREAMS {
            return Err(BlendPoseError::Capacity);
        }
        let start = buffer.starts[stream];
        let end = start
            .checked_add(buffer.lengths[stream])
            .ok_or(BlendPoseError::Capacity)?;
        self.lanes[buffer.pool_index][stream]
            .get(start..end)
            .ok_or(BlendPoseError::Capacity)
    }
    fn push(
        &mut self,
        buffer: &mut BlendPoseBuffer,
        stream: usize,
        token: u32,
    ) -> Result<(), BlendPoseError> {
        if stream >= STREAMS || self.tops[buffer.pool_index][stream] >= TOKENS - 1 {
            return Err(BlendPoseError::Capacity);
        }
        let index = buffer.starts[stream] + buffer.lengths[stream];
        let slot = self.lanes[buffer.pool_index][stream]
            .get_mut(index)
            .ok_or(BlendPoseError::Capacity)?;
        *slot = token;
        buffer.lengths[stream] += 1;
        self.tops[buffer.pool_index][stream] += 1;
        Ok(())
    }
    /// `PushPose82d6b2c8`: shift is unmasked, preserving source low-word behavior.
    pub(crate) fn push_pose(
        &mut self,
        buffer: &mut BlendPoseBuffer,
        stream: usize,
        pose: u32,
    ) -> Result<(), BlendPoseError> {
        self.push(buffer, stream, pose << 23)
    }
    pub(crate) fn push_coefficient(
        &mut self,
        buffer: &mut BlendPoseBuffer,
        stream: usize,
        coefficient: f32,
    ) -> Result<(), BlendPoseError> {
        if stream >= STREAMS || self.tops[buffer.pool_index][stream] >= TOKENS - 1 {
            return Err(BlendPoseError::Capacity);
        }
        if stream >= STREAMS || buffer.lengths[stream] < 2 {
            return Err(BlendPoseError::InvalidTokenStack);
        }
        // Raw b454/b468 publishes the relative top and zero token before fctidz;
        // b494 publishes the global top only after conversion completes.
        let index = buffer.starts[stream] + buffer.lengths[stream];
        let slot = self.lanes[buffer.pool_index][stream]
            .get_mut(index)
            .ok_or(BlendPoseError::Capacity)?;
        buffer.lengths[stream] += 1;
        *slot = 0;
        let word =
            low_integer_word(coefficient * COEFFICIENT_SCALE).map_err(BlendPoseError::Numeric)?;
        self.tops[buffer.pool_index][stream] += 1;
        *slot = 0x4000_0000 | word;
        Ok(())
    }
    /// `PushBlendCommand82d6b4d8`: preserve its exact backwards duplicate scan.
    pub(crate) fn push_blend(
        &mut self,
        buffer: &mut BlendPoseBuffer,
        stream: usize,
        keys: usize,
    ) -> Result<(), BlendPoseError> {
        if stream >= STREAMS {
            return Err(BlendPoseError::Capacity);
        }
        let length = buffer.lengths[stream];
        let mut position = length
            .checked_sub(keys + 1)
            .ok_or(BlendPoseError::InvalidTokenStack)?;
        let mut same = false;
        if position >= keys {
            same = true;
            let tokens = self.tokens(buffer, stream)?;
            let first = tokens[position];
            for index in 0..keys {
                let token = tokens[position];
                if token != first || token & 0xc000_0000 != 0 {
                    same = false;
                    break;
                }
                if index + 1 < keys {
                    position = position
                        .checked_sub(1)
                        .ok_or(BlendPoseError::InvalidTokenStack)?;
                }
            }
        }
        if same {
            let remove = if keys == 2 { 2 } else { keys * 2 - 1 };
            if length <= remove || self.tops[buffer.pool_index][stream] < remove {
                return Err(BlendPoseError::InvalidTokenStack);
            }
            buffer.lengths[stream] -= remove;
            self.tops[buffer.pool_index][stream] -= remove;
            Ok(())
        } else {
            if length < 3 {
                return Err(BlendPoseError::InvalidTokenStack);
            }
            let keys = u32::try_from(keys).map_err(|_| BlendPoseError::Capacity)?;
            self.push(buffer, stream, 0x8000_0000 | (keys << 20))
        }
    }
    pub(crate) fn append_streams(&mut self,destination:&mut BlendPoseBuffer,source:&BlendPoseBuffer)->Result<(),BlendPoseError> {
        for stream in 0..source.count {
            let index=destination.create_stream(source.names[source.name_index][stream],source.names[1-source.name_index][stream])?;
            for token in self.tokens(source,stream)?.to_vec() {
                match token>>30 {
                    0=>self.push_pose(destination,index,(token>>23)&0x7f)?,
                    1=>self.push_coefficient(destination,index,decode_coefficient(token))?,
                    2=>self.push_blend(destination,index,((token>>20)&7) as usize)?,_=>{}
                }
            }
        }
        Ok(())
    }
    /// Blend82d78038 re-encodes each token and appends a pair blend for existing streams.
    pub(crate) fn blend(
        &mut self,
        destination: &mut BlendPoseBuffer,
        source: &BlendPoseBuffer,
        coefficient: f32,
    ) -> Result<(), BlendPoseError> {
        for stream in 0..source.count {
            let name = source.names[source.name_index][stream];
            let existing = destination.stream_index(&name);
            let index = match existing {
                Some(index) => index,
                None => {
                    destination.create_stream(name, source.names[1 - source.name_index][stream])?
                }
            };
            // Snapshot token words before writes: native source/destination pools differ.
            let tokens = self.tokens(source, stream)?.to_vec();
            for token in tokens {
                match token >> 30 {
                    0 => self.push_pose(destination, index, (token >> 23) & 0x7f)?,
                    1 => self.push_coefficient(destination, index, decode_coefficient(token))?,
                    2 => self.push_blend(destination, index, ((token >> 20) & 7) as usize)?,
                    _ => {}
                }
            }
            if existing.is_some() {
                self.push_coefficient(destination, index, coefficient)?;
                self.push_blend(destination, index, 2)?;
            }
        }
        Ok(())
    }
}
impl BlendPoseBuffer {
    pub(crate) fn mirror_streams(&mut self) {self.name_index=1-self.name_index;}
    pub(crate) fn stream_index(&self, name: &FastString36) -> Option<usize> {
        self.names[self.name_index][..self.count]
            .iter()
            .position(|candidate| candidate == name)
    }
    fn create_stream(
        &mut self,
        name: FastString36,
        mirrored: FastString36,
    ) -> Result<usize, BlendPoseError> {
        if let Some(index) = self.stream_index(&name) {
            return Ok(index);
        }
        if self.count >= STREAMS {
            return Err(BlendPoseError::Capacity);
        }
        let index = self.count;
        self.names[1 - self.name_index][index] = mirrored;
        self.count += 1;
        self.names[self.name_index][index] = name;
        Ok(index)
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "native exact float conversion of a 20-bit unsigned field"
)]
pub(crate) fn decode_coefficient(token: u32) -> f32 {
    (token & 0x000f_ffff) as f32 * f32::from_bits(0x3580_0008)
}

#[derive(Clone)]
pub(crate) struct ClipStream {
    pub(crate) name: FastString36,
    pub(crate) mirror: FastString36,
    pub(crate) keys: Vec<(i16, u16)>,
}
impl FaceDatabase {
    fn clip_blend_streams(&self, record_index: usize) -> Result<Vec<ClipStream>, BlendPoseError> {
        let parsed = self
            .parsed
            .as_ref()
            .ok_or(BlendPoseError::Missing("parsed clip database"))?;
        let record = parsed
            .records
            .get(record_index)
            .ok_or(BlendPoseError::SourceRange)?;
        if record.header.data_type != 2 {
            return Err(BlendPoseError::SourceRange);
        }
        let bytes = self
            .bytes
            .as_ref()
            .and_then(|bytes| bytes.get(record.data_range.clone()))
            .ok_or(BlendPoseError::SourceRange)?;
        decode_clip_streams(bytes)
    }
}
pub(crate) fn decode_clip_streams(bytes:&[u8])->Result<Vec<ClipStream>,BlendPoseError> {
        let count = (word(bytes, 52)? >> 13) & 31;
        let table = usize::try_from(word(bytes, 48)?).map_err(|_| BlendPoseError::SourceRange)?;
        let mut streams = Vec::new();
        for index in 0..count {
            let offset = usize::try_from(word(bytes, table + index as usize * 4)?)
                .map_err(|_| BlendPoseError::SourceRange)?;
            let mut names = [FastString36 { words: [0; 6] }; 2];
            for (name_index, name) in names.iter_mut().enumerate() {
                for (lane, lane_word) in name.words.iter_mut().enumerate() {
                    *lane_word = word(bytes, offset + name_index * 24 + lane * 4)?;
                }
            }
            let count = i32::from_ne_bytes(word(bytes, offset + 48)?.to_ne_bytes());
            let mut keys = Vec::new();
            for index in 0..count {
                let index = usize::try_from(index).map_err(|_| BlendPoseError::SourceRange)?;
                let key = word(bytes, offset + 52 + index * 4)?.to_be_bytes();
                keys.push((
                    i16::from_be_bytes([key[0], key[1]]),
                    u16::from_be_bytes([key[2], key[3]]),
                ));
            }
            streams.push(ClipStream {
                name: names[0],
                mirror: names[1],
                keys,
            });
        }
        Ok(streams)
}

fn word(bytes: &[u8], offset: usize) -> Result<u32, BlendPoseError> {
    let end = offset.checked_add(4).ok_or(BlendPoseError::SourceRange)?;
    Ok(u32::from_be_bytes(
        bytes
            .get(offset..end)
            .ok_or(BlendPoseError::SourceRange)?
            .try_into()
            .expect("four bytes"),
    ))
}

impl AnimationTreePool {
    /// `EvalBlendPose` consumes the SAME nodes as body Eval without changing their times.
    pub(crate) fn eval_blend_pose(
        &self,
        handle: &AnimationNodeHandle,
        buffer: &mut BlendPoseBuffer,
        pool: &mut BlendPoseTokenPool,
    ) -> Result<(), BlendPoseError> {
        match &self.node(handle)?.kind {
            AnimationNodeKind::Sequence(_) | AnimationNodeKind::Selector(_) => {
                self.eval_blend_pose(&self.selected_child(handle)?, buffer, pool)
            }
            AnimationNodeKind::BlendSpace(node) => {
                let count = node.active_count()?;
                for index in 0..count {
                    let child = self.blend_space(handle)?.active_child(index)?;
                    self.eval_blend_pose(&child, buffer, pool)?;
                }
                for index in 0..count {
                    let coefficient = self
                        .blend_space(handle)?
                        .parameters
                        .lock()
                        .map_err(|_| AnimationTreeError::Lock)?
                        .coefficients[index];
                    for stream in 0..STREAMS {
                        pool.push_coefficient(buffer, stream, coefficient)?;
                    }
                }
                for stream in 0..STREAMS {
                    pool.push_blend(buffer, stream, count)?;
                }
                Ok(())
            }
            AnimationNodeKind::Clip(clip) => {
                let header = clip
                    .header
                    .as_ref()
                    .ok_or(BlendPoseError::Missing("clip header"))?;
                let streams = header
                    .database
                    .lock()
                    .map_err(|_| BlendPoseError::Missing("clip database lock"))?
                    .clip_blend_streams(header.record_index)?;
                eval_clip_streams(&streams, clip.time, clip.length, buffer, pool)
            }
            AnimationNodeKind::Pose { .. } | AnimationNodeKind::FixedSqt { .. } => {
                Err(BlendPoseError::PoseNodeAssertion)
            }
            AnimationNodeKind::AddSqt { motion, .. } => self.eval_blend_pose(motion, buffer, pool),
            AnimationNodeKind::Mirror { child, .. } => {
                self.eval_blend_pose(
                    child
                        .as_ref()
                        .ok_or(BlendPoseError::Missing("mirror child"))?,
                    buffer,
                    pool,
                )?;
                buffer.name_index = 1 - buffer.name_index;
                Ok(())
            }
            AnimationNodeKind::PhaseBlend(phase) => {
                let first = phase
                    .child1
                    .as_ref()
                    .ok_or(BlendPoseError::Missing("phase first child"))?;
                let second = phase
                    .child2
                    .as_ref()
                    .ok_or(BlendPoseError::Missing("phase second child"))?;
                let (remapped, gate) = phase_stream_coefficients(
                    phase.blend,
                    phase
                        .children
                        .first()
                        .is_some_and(|(_, node)| node == first),
                    phase
                        .children
                        .last()
                        .is_some_and(|(_, node)| node == second),
                );
                // Raw BGE means NOT-LT, including unordered.
                if gate.partial_cmp(&1.0) != Some(std::cmp::Ordering::Less) {
                    return self.eval_blend_pose(second, buffer, pool);
                }
                self.eval_blend_pose(first, buffer, pool)?;
                if phase.blend > 0.0 {
                    let mut temporary = pool.buffer(1 - buffer.pool_index)?;
                    self.eval_blend_pose(second, &mut temporary, pool)?;
                    pool.blend(buffer, &temporary, remapped)?;
                }
                Ok(())
            }
        }
    }
}
pub(crate) fn eval_clip_streams(
    streams: &[ClipStream],
    time: f32,
    length: f32,
    buffer: &mut BlendPoseBuffer,
    pool: &mut BlendPoseTokenPool,
) -> Result<(), BlendPoseError> {
    let mut written = [false; STREAMS];
    for stream in streams {
        let index = buffer.create_stream(stream.name, stream.mirror)?;
        for (key_index, &(encoded, pose)) in stream.keys.iter().enumerate() {
            let end = length * (f32::from(encoded) * f32::from_bits(0x3900_0400));
            // Raw82d77f00 skips only ordered LT. Unordered selects this key.
            if end.partial_cmp(&time) != Some(std::cmp::Ordering::Less) {
                if key_index == 0 {
                    pool.push_pose(buffer, index, u32::from(pose))?;
                } else {
                    let (previous, previous_pose) = stream.keys[key_index - 1];
                    pool.push_pose(buffer, index, u32::from(previous_pose))?;
                    pool.push_pose(buffer, index, u32::from(pose))?;
                    let start = length * (f32::from(previous) * f32::from_bits(0x3900_0400));
                    pool.push_coefficient(buffer, index, 1.0 - (end - time) / (end - start))?;
                    pool.push_blend(buffer, index, 2)?;
                }
                written[index] = true;
                break;
            }
        }
    }
    for (index, wrote) in written.into_iter().enumerate() {
        if !wrote {
            pool.push_pose(buffer, index, 0)?;
        }
    }
    Ok(())
}

/// Raw82d793ec..94f4: smoothed coefficient selects children; remapped blends streams.
fn phase_stream_coefficients(blend: f32, first: bool, last: bool) -> (f32, f32) {
    let lower = f32::from_bits(0x3e99_999a);
    let upper = f32::from_bits(0x3f33_3333);
    let capped = if lower - blend >= 0.0 { lower } else { blend };
    let capped = if upper - capped >= 0.0 { capped } else { upper };
    let remapped = (capped - lower) * f32::from_bits(0x4020_0001);
    let squared = remapped * remapped;
    let gate = if remapped > 0.5 {
        if last {
            let four = remapped * 4.0;
            (8.0 - four).mul_add(squared, -four) + 1.0
        } else {
            blend
        }
    } else if first {
        -(remapped.mul_add(4.0, -4.0)) * squared
    } else {
        blend
    };
    (remapped, gate)
}
