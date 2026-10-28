//! Native selected-list `WriteData827dbf98` and retained uncompressed backing view.
use super::{
    PresentationAnimation,
    read::PresentationReadFrame,
    write::{PresentationDataSubmission, PresentationWriteError, PresentationWriteScratch},
};
use crate::{
    actor_construction::presentation_frame_source::{
        ActorPresentationFrameSource, PresentationFrameSourceError,
    },
    animation::AnimationMatrix,
    game_input_command::GameInputCommandNumeric,
    world::try_simulation_quaternion_candidate,
};
#[path = "../presentation_data_buffer.rs"]
pub(crate) mod buffer;
#[cfg(test)]
#[path = "skater_presentation_frame_corpus.rs"]
pub(crate) mod corpus;
#[path = "skater_presentation_frame_read.rs"]
pub(crate) mod read_pass;
#[path = "skater_presentation_selected_runtime.rs"]
pub(crate) mod selected_runtime;
use buffer::{GatheredPresentationFrame, PresentationBufferError, SelectedPresentationBuffers};
#[derive(Debug)]
pub(crate) enum PresentationFrameError<N, P> {
    Missing(&'static str),
    Source(PresentationFrameSourceError),
    Numeric(N),
    Write(PresentationWriteError<N, P>),
    Submission(P),
    ScratchLimit,
    HeaderBounds,
    Buffer(PresentationBufferError),
}
impl<N: std::fmt::Display, P: std::fmt::Display> std::fmt::Display
    for PresentationFrameError<N, P>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "frame missing {s}"),
            Self::Source(e) => e.fmt(f),
            Self::Numeric(e) => e.fmt(f),
            Self::Write(e) => e.fmt(f),
            Self::Submission(e) => e.fmt(f),
            Self::ScratchLimit => f.write_str("native WriteData scratch count assertion"),
            Self::HeaderBounds => f.write_str("submitted frame header outside recorded payload"),
            Self::Buffer(e) => e.fmt(f),
        }
    }
}
/// Once-zero header scratch selects unspecified native stack contents for PC.
/// Each call updates only reached native fields (including explicit damage zero).
#[derive(Clone, Copy)]
pub(crate) enum RootPositionWTransport {
    RequireKnown,
    /// Only readers proven to ignore/overwrite this lane are admitted.
    /// Bytes are a host carrier, never a reconstructed physical W value.
    ReadSkaterOpaque {
        bits: u32,
    },
}
pub(crate) struct SubmittedPresentationFrame {
    gather: GatheredPresentationFrame,
    root_position_w: Vec<Option<u32>>,
}
pub(crate) struct PresentationHeaderScratch {
    bytes: [u8; 208],
    root_w_policy: RootPositionWTransport,
    root_position_w: Vec<Option<u32>>,
}
impl PresentationHeaderScratch {
    pub(crate) const fn new_host_zero_policy() -> Self {
        Self {
            bytes: [0; 208],
            root_w_policy: RootPositionWTransport::RequireKnown,
            root_position_w: Vec::new(),
        }
    }
    /// `WriteData827dc314/c328` copies position; retain its unknown W separately.
    fn root_position(
        &mut self,
        value: crate::skeleton_root_transforms::SkeletonRootVector,
    ) -> Result<Option<u32>, &'static str> {
        let semantic = value.w.map(f32::to_bits);
        let carrier = match (semantic, self.root_w_policy) {
            (None, RootPositionWTransport::RequireKnown) => return Err("known root position W"),
            (Some(bits), _) | (None, RootPositionWTransport::ReadSkaterOpaque { bits }) => bits,
        };
        for (lane, value) in value.xyz.into_iter().enumerate() {
            self.set_word(144 + 4 * lane, value.to_bits());
        }
        self.set_word(156, carrier);
        Ok(semantic)
    }
    fn word(&self, offset: usize) -> u32 {
        u32::from_be_bytes(
            self.bytes[offset..offset + 4]
                .try_into()
                .expect("fixed header word"),
        )
    }
    fn set_word(&mut self, offset: usize, value: u32) {
        self.bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }
    fn bits(&mut self, offset: usize, mask: u32, value: u32) {
        self.set_word(offset, (self.word(offset) & !mask) | (value & mask));
    }
    fn vector(&mut self, offset: usize, value: [f32; 4]) {
        for (i, v) in value.into_iter().enumerate() {
            self.set_word(offset + 4 * i, v.to_bits());
        }
    }
    fn matrix(&mut self, offset: usize, value: AnimationMatrix) {
        for column in 0..4 {
            self.vector(
                offset + 16 * column,
                std::array::from_fn(|lane| value[4 * column + lane]),
            );
        }
    }
}
impl PresentationAnimation {
    /// Selected actual Actor list is supplied by the caller.
    /// This does not stand in for `DoSimSubmit` roster/`ShouldSubmit` filtering.
    #[expect(
        clippy::too_many_lines,
        reason = "preserve ordered native header getters and payload publication"
    )]
    pub(crate) fn try_write_data<N: GameInputCommandNumeric, P: PresentationDataSubmission>(
        &self,
        sources: &[ActorPresentationFrameSource<'_>],
        header: &mut PresentationHeaderScratch,
        scratch: &mut PresentationWriteScratch<'_>,
        numeric: &mut N,
        provider: &mut P,
    ) -> Result<(), PresentationFrameError<N::Error, P::Error>> {
        use PresentationFrameError as E;
        let count = u32::try_from(sources.len()).map_err(|_| E::HeaderBounds)?;
        provider
            .submit_data(&count.to_be_bytes(), 4)
            .map_err(E::Submission)?;
        header.root_position_w.clear();
        header
            .root_position_w
            .try_reserve(sources.len())
            .map_err(|_| E::Buffer(PresentationBufferError::Allocation))?;
        let offsets = self.offsets.as_ref().ok_or(E::Missing("InitOffsets"))?;
        if offsets.pose_bytes == 0 {
            return Err(E::Missing("nonzero pose buffer size"));
        }
        header.bytes[168..182].fill(0); // native c024, not a whole-header clear.
        for (index, source) in sources.iter().enumerate() {
            let player = source.player_id().map_err(E::Source)?;
            header.bits(188, 0xf0, player.wrapping_shl(4));
            header.bytes[160..168]
                .copy_from_slice(&source.peer_id().map_err(E::Source)?.to_be_bytes());
            header.bits(
                192,
                0x8000,
                u32::from(source.is_local().map_err(E::Source)?) << 15,
            );
            header.bits(
                192,
                0x4000,
                u32::from(source.is_collidable().map_err(E::Source)?) << 14,
            );
            let opacity = source.opacity().map_err(E::Source)? * 255.0;
            let opacity = numeric
                .truncate_i64(opacity)
                .map_err(E::Numeric)?
                .to_be_bytes()[7];
            header.bits(188, 0xff00, u32::from(opacity) << 8);
            header.bits(
                192,
                0x2000,
                u32::from(source.opacity_main_camera_only().map_err(E::Source)?) << 13,
            );
            header.bits(
                192,
                0x1000,
                u32::from(source.was_teleported().map_err(E::Source)?) << 12,
            );
            header.set_word(184, source.board_speed().map_err(E::Source)?.to_bits());
            let root = source.root_matrix().map_err(E::Source)?;
            let semantic_w = header
                .root_position(root.columns()[3])
                .map_err(E::Missing)?;
            header.root_position_w.push(semantic_w); // native c328 precedes quaternion estimates/store.
            let root_q = try_simulation_quaternion_candidate(root.xyz_scratch(), |magnitudes| {
                let mut seeds = [0.0; 4];
                for i in 0..4 {
                    seeds[i] = numeric.reciprocal_sqrt_estimate(magnitudes[i])?;
                }
                Ok(seeds)
            })
            .map_err(E::Numeric)?;
            header.vector(128, root_q);
            {
                let local = source.header_local_pose_owner().map_err(E::Source)?;
                let local = local
                    .lock()
                    .map_err(|_| E::Source(PresentationFrameSourceError::Lock))?;
                let local = local
                    .local_pose
                    .as_ref()
                    .ok_or(E::Missing("header local pose"))?;
                for (slot, bone) in [(0, self.hips), (64, self.skateboard)] {
                    let bone = bone.ok_or(E::Missing("InitAnimation bone"))?;
                    let i = usize::try_from(bone)
                        .map_err(|_| E::Source(PresentationFrameSourceError::Bone(bone)))?;
                    header.matrix(
                        slot,
                        *local
                            .get(i)
                            .ok_or(E::Source(PresentationFrameSourceError::Bone(bone)))?,
                    );
                }
                if crate::diagnostics::lean_trace::enabled() {
                    for (bone, matrix) in local.iter().enumerate() {
                        crate::diagnostics::lean_trace::record(
                            "local_pose",
                            player as usize,
                            bone as u64,
                            matrix,
                        );
                    }
                }
            }
            header.bytes[168..182].copy_from_slice(&source.character_damage().map_err(E::Source)?);
            let wiping = if source.is_wiping_out().map_err(E::Source)? {
                u32::from_ne_bytes(source.time_in_state().map_err(E::Source)?.to_ne_bytes())
            } else {
                0
            };
            header.bits(192, 0xffff_0000, wiping.wrapping_shl(16));
            let pose_offset = 208_usize
                .checked_mul(sources.len() - index)
                .and_then(|n| {
                    offsets
                        .pose_bytes
                        .checked_mul(index)
                        .and_then(|p| n.checked_add(p))
                })
                .ok_or(E::HeaderBounds)?;
            let offset = u32::try_from(pose_offset).map_err(|_| E::HeaderBounds)?;
            header.bits(188, 0xffff_0000, offset.wrapping_shl(16));
            provider
                .submit_data(&header.bytes, 16)
                .map_err(E::Submission)?;
        }
        if offsets.matrix_rotations.len() + offsets.quaternion3_count >= 120
            || offsets.translation_count >= 120
        {
            return Err(E::ScratchLimit);
        }
        for source in sources {
            let pose = source
                .actor
                .try_capture_presentation_pose()
                .map_err(|e| E::Source(PresentationFrameSourceError::Pose(e)))?;
            pose.try_write(offsets, scratch, numeric, provider)
                .map_err(E::Write)?;
        }
        Ok(())
    }
}
/// Retained byte lifecycle only; it owns no second animation or pose allocation.
pub(crate) struct SelectedPresentationFrameProducer {
    pub(crate) buffers: SelectedPresentationBuffers,
    header: PresentationHeaderScratch,
    opaque_root_bytes: [Vec<std::ops::Range<usize>>; 2],
    matrix_rotations: Vec<[u16; 4]>,
    rotations: Vec<[u16; 3]>,
    translations: Vec<[u16; 3]>,
}
impl SelectedPresentationFrameProducer {
    pub(crate) fn new_uncompressed_host_policy(
        animation: &PresentationAnimation,
        submission_capacity: usize,
        gather_capacity: usize,
        provider: u32,
        root_w_policy: RootPositionWTransport,
    ) -> Result<Self, PresentationBufferError> {
        let offsets = animation
            .offsets
            .as_ref()
            .ok_or(PresentationBufferError::SubmissionState)?;
        Ok(Self {
            buffers: SelectedPresentationBuffers::new_uncompressed_host_policy(
                submission_capacity,
                gather_capacity,
                provider,
            )?,
            header: PresentationHeaderScratch {
                root_w_policy,
                ..PresentationHeaderScratch::new_host_zero_policy()
            },
            opaque_root_bytes: [Vec::new(), Vec::new()],
            matrix_rotations: try_scratch(offsets.matrix_rotations.len())?,
            rotations: try_scratch(offsets.quaternion3_count)?,
            translations: try_scratch(offsets.translation_count)?,
        })
    }
    pub(crate) fn try_produce<N: GameInputCommandNumeric>(
        &mut self,
        animation: &PresentationAnimation,
        sources: &[ActorPresentationFrameSource<'_>],
        numeric: &mut N,
    ) -> Result<SubmittedPresentationFrame, PresentationFrameError<N::Error, PresentationBufferError>>
    {
        for ranges in &mut self.opaque_root_bytes {
            ranges
                .try_reserve(sources.len())
                .map_err(|_| PresentationFrameError::Buffer(PresentationBufferError::Allocation))?;
        }
        self.buffers
            .begin_submission()
            .map_err(PresentationFrameError::Buffer)?;
        let mut scratch = PresentationWriteScratch {
            matrix_rotations: &mut self.matrix_rotations,
            rotations: &mut self.rotations,
            translations: &mut self.translations,
        };
        animation.try_write_data(
            sources,
            &mut self.header,
            &mut scratch,
            numeric,
            &mut self.buffers,
        )?;
        let gather = self
            .buffers
            .finish_uncompressed()
            .map_err(PresentationFrameError::Buffer)?;
        let frame = SubmittedPresentationFrame {
            gather,
            root_position_w: std::mem::take(&mut self.header.root_position_w),
        };
        let ranges = &mut self.opaque_root_bytes[frame.gather.buffer_index];
        // Native writes cover the prefix; untouched tail keeps old knowledge.
        ranges.retain_mut(|range| {
            range.start = range.start.max(frame.gather.payload.end);
            range.start < range.end
        });
        for (index, w) in frame.root_position_w.iter().enumerate() {
            if w.is_none() {
                let start = frame.gather.payload.start + 16 + 208 * index + 156;
                ranges.push(start..start + 4);
            }
        }
        Ok(frame)
    }
    /// Same-pointer `UpdateSkater827da108..a1cc` root only. The native position
    /// W overwrite is proven; distinct-frame interpolation/model dirt is not
    /// inferred. Actual frame identity and generation are checked by `read_frame`.
    pub(crate) fn same_frame_root(
        &self,
        frame: &SubmittedPresentationFrame,
        index: usize,
    ) -> Result<
        AnimationMatrix,
        PresentationFrameError<std::convert::Infallible, PresentationBufferError>,
    > {
        let view = self.read_frame(frame, index)?;
        let lane = |offset| {
            f32::from_bits(u32::from_be_bytes(
                view.read_base[offset..offset + 4]
                    .try_into()
                    .expect("validated header"),
            ))
        };
        let q = std::array::from_fn(|i| lane(128 + 4 * i));
        let p = [lane(144), lane(148), lane(152), 1.0];
        Ok(super::move_root::rigid_matrix(q, p))
    }
    /// Returned W keeps native knowledge; carrier bytes are not decoded as W.
    /// Future consumers must require `known_lanes` or prove a native overwrite.
    pub(crate) fn root_position(
        &self,
        frame: &SubmittedPresentationFrame,
        index: usize,
    ) -> Result<
        crate::skeleton_root_transforms::SkeletonRootVector,
        PresentationFrameError<std::convert::Infallible, PresentationBufferError>,
    > {
        let view = self.read_frame(frame, index)?; // validates exact gather generation/header
        let w = *frame
            .root_position_w
            .get(index)
            .ok_or(PresentationFrameError::HeaderBounds)?;
        let xyz = std::array::from_fn(|i| {
            f32::from_bits(u32::from_be_bytes(
                view.read_base[144 + 4 * i..148 + 4 * i]
                    .try_into()
                    .expect("validated root lane"),
            ))
        });
        Ok(crate::skeleton_root_transforms::SkeletonRootVector {
            xyz,
            w: w.map(f32::from_bits),
        })
    }
    pub(crate) fn read_frame(
        &self,
        frame: &SubmittedPresentationFrame,
        index: usize,
    ) -> Result<
        PresentationReadFrame<'_>,
        PresentationFrameError<std::convert::Infallible, PresentationBufferError>,
    > {
        use PresentationFrameError as E;
        let backing = self.buffers.backing_for(&frame.gather).map_err(E::Buffer)?;
        if frame.gather.payload.len() < 4 {
            return Err(E::HeaderBounds);
        }
        let count = u32::from_be_bytes(
            backing
                .get(..4)
                .ok_or(E::HeaderBounds)?
                .try_into()
                .expect("four bytes"),
        );
        if index >= usize::try_from(count).map_err(|_| E::HeaderBounds)? {
            return Err(E::HeaderBounds);
        }
        let start = index
            .checked_mul(208)
            .and_then(|n| n.checked_add(16))
            .ok_or(E::HeaderBounds)?;
        let end = start.checked_add(208).ok_or(E::HeaderBounds)?;
        if end > frame.gather.payload.len() {
            return Err(E::HeaderBounds);
        }
        let header = backing.get(start..end).ok_or(E::HeaderBounds)?;
        let matrix = |offset: usize| {
            std::array::from_fn(|i| {
                f32::from_bits(u32::from_be_bytes(
                    header[offset + 4 * i..offset + 4 * i + 4]
                        .try_into()
                        .expect("header matrix"),
                ))
            })
        };
        let compressed_offset = u16::from_be_bytes([header[188], header[189]]);
        let remaining = usize::try_from(count).map_err(|_| E::HeaderBounds)? - index;
        let header_bytes = remaining.checked_mul(208).ok_or(E::HeaderBounds)?;
        // Admission proof: compressed reads cannot reach opaque header lanes.
        if usize::from(compressed_offset) < header_bytes {
            return Err(E::HeaderBounds);
        }
        let read_start = frame.gather.payload.start + start + usize::from(compressed_offset);
        let size = self.matrix_rotations.len() * 8
            + self.rotations.len() * 6
            + self.translations.len() * 6;
        let lookahead = if self.translations.is_empty() && self.rotations.is_empty() {
            0
        } else {
            2
        };
        let read_end = read_start
            .checked_add(size)
            .and_then(|n| n.checked_add(lookahead))
            .ok_or(E::HeaderBounds)?;
        if self.opaque_root_bytes[frame.gather.buffer_index]
            .iter()
            .any(|range| range.start < read_end && read_start < range.end)
        {
            return Err(E::Missing(
                "known compressed bytes: retained opaque root W overlaps native lookahead",
            ));
        }
        Ok(PresentationReadFrame {
            read_base: &backing[start..],
            compressed_offset,
            local_hips: matrix(0),
            local_skateboard: matrix(64),
        })
    }
}
fn try_scratch<const N: usize>(count: usize) -> Result<Vec<[u16; N]>, PresentationBufferError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| PresentationBufferError::Allocation)?;
    values.resize(count, [0; N]);
    Ok(values)
}
#[cfg(test)]
#[path = "skater_presentation_frame_tests.rs"]
mod tests;
