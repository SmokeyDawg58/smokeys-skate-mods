//! Native submission bytes over explicitly zero-initialized portable allocations.
//! This is the selected uncompressed host-admitted path, not all `DataMan` modes.
use super::super::write::PresentationDataSubmission;
use std::ops::Range;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PresentationBufferError {
    Allocation,
    Bounds {
        start: usize,
        size: usize,
        capacity: usize,
    },
    Alignment(usize),
    Size(usize),
    Provider(u32),
    SubmissionState,
    StaleFrame,
}
impl std::fmt::Display for PresentationBufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allocation => f.write_str("presentation allocation failed"),
            Self::Bounds {
                start,
                size,
                capacity,
            } => write!(
                f,
                "presentation write {start}+{size} exceeds capacity{capacity}"
            ),
            Self::Alignment(v) => write!(f, "presentation alignment{v} exceeds native field"),
            Self::Size(v) => write!(f, "presentation size{v} exceeds native field"),
            Self::Provider(v) => write!(f, "presentation provider{v} exceeds5 bits"),
            Self::SubmissionState => f.write_str("presentation submission is not allowed"),
            Self::StaleFrame => f.write_str("presentation gather allocation has been recycled"),
        }
    }
}
/// Backing is zero-initialized ONCE as explicit PC allocator policy. Reset only
/// touches metadata; unwritten tails retain prior native writes across frames.
pub(crate) struct PresentationDataBuffer {
    bytes: Vec<u8>,
    metadata: u32,
}
impl PresentationDataBuffer {
    pub(crate) fn new(capacity: usize) -> Result<Self, PresentationBufferError> {
        if capacity > i32::MAX as usize {
            return Err(PresentationBufferError::Size(capacity));
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| PresentationBufferError::Allocation)?;
        bytes.resize(capacity, 0);
        Ok(Self { bytes, metadata: 0 })
    }
    pub(crate) fn size(&self) -> usize {
        (self.metadata >> 7) as usize
    }
    pub(crate) fn alignment(&self) -> usize {
        ((self.metadata >> 1) & 63) as usize
    }
    pub(crate) fn reset_metadata(&mut self) {
        self.metadata &= 1;
    }
    pub(crate) fn backing(&self) -> &[u8] {
        &self.bytes
    }
    /// `AddData829900c8`: zero the reached gap before alignment publication and
    /// payload bounds checks. Only successful payload writes advance size.
    pub(crate) fn add(
        &mut self,
        data: &[u8],
        alignment: usize,
    ) -> Result<Range<usize>, PresentationBufferError> {
        use PresentationBufferError as E;
        let size = self.size();
        let padding = if alignment == 0 || size % alignment == 0 {
            0
        } else {
            alignment - size % alignment
        };
        let start = size.checked_add(padding).ok_or(E::Size(size))?;
        for i in size..start {
            let capacity = self.bytes.len();
            *self.bytes.get_mut(i).ok_or(E::Bounds {
                start: i,
                size: 1,
                capacity,
            })? = 0;
        }
        if alignment > self.alignment() {
            let packed = u32::try_from(alignment).map_err(|_| E::Alignment(alignment))?;
            self.metadata = (self.metadata & !0x7e) | ((packed.wrapping_mul(2)) & 0x7e);
            if self.alignment() != alignment {
                return Err(E::Alignment(alignment));
            }
        }
        if data.len() > i32::MAX as usize {
            return Err(E::Size(data.len()));
        }
        let end = start.checked_add(data.len()).ok_or(E::Size(data.len()))?;
        if !data.is_empty() {
            let capacity = self.bytes.len();
            self.bytes
                .get_mut(start..end)
                .ok_or(E::Bounds {
                    start,
                    size: data.len(),
                    capacity,
                })?
                .copy_from_slice(data);
        }
        // Native stores the low25-bit field without a later representability assertion.
        let end = u32::try_from(end).map_err(|_| E::Size(end))?;
        self.metadata = (end.wrapping_shl(7)) | (self.metadata & 0x7f);
        Ok(start..start + data.len())
    }
}
#[derive(Clone, Debug)]
pub(crate) struct GatheredPresentationFrame {
    pub(crate) payload: Range<usize>,
    pub(crate) buffer_index: usize,
    generation: u64,
    owner: std::sync::Arc<()>,
}
/// The caller explicitly admits uncompressed collection for the selected test.
/// Native compression defaults, other providers and `DataTracker` are not inferred.
pub(crate) struct SelectedPresentationBuffers {
    submission: PresentationDataBuffer,
    gathers: [PresentationDataBuffer; 2],
    provider: u32,
    allowed: bool,
    current: Option<GatheredPresentationFrame>,
    generations: [u64; 2],
    owner: std::sync::Arc<()>,
}
impl SelectedPresentationBuffers {
    pub(crate) fn new_uncompressed_host_policy(
        submission_capacity: usize,
        gather_capacity: usize,
        provider: u32,
    ) -> Result<Self, PresentationBufferError> {
        Ok(Self {
            submission: PresentationDataBuffer::new(submission_capacity)?,
            gathers: [
                PresentationDataBuffer::new(gather_capacity)?,
                PresentationDataBuffer::new(gather_capacity)?,
            ],
            provider,
            allowed: false,
            current: None,
            generations: [0; 2],
            owner: std::sync::Arc::new(()),
        })
    }
    pub(crate) fn begin_submission(&mut self) -> Result<(), PresentationBufferError> {
        if self.allowed || self.submission.size() != 0 {
            return Err(PresentationBufferError::SubmissionState);
        }
        self.allowed = true;
        Ok(())
    }
    /// `GatherData829908b8` is called with explicit `AllowCompression=false`.
    /// `PostGather82990b44..c88` writes subframe/payload then resets only metadata.
    pub(crate) fn finish_uncompressed(
        &mut self,
    ) -> Result<GatheredPresentationFrame, PresentationBufferError> {
        use PresentationBufferError as E;
        if !self.allowed {
            return Err(E::SubmissionState);
        }
        self.allowed = false;
        let size = self.submission.size();
        if size == 0 || size > 0x7ffff {
            return Err(E::Size(size));
        }
        if self.provider > 31 {
            return Err(E::Provider(self.provider));
        }
        let index = self.current.as_ref().map_or(0, |f| 1 - f.buffer_index);
        self.generations[index] = self.generations[index].wrapping_add(1);
        let gather = &mut self.gathers[index];
        gather.reset_metadata();
        let word = (u32::try_from(size).map_err(|_| E::Size(size))? << 13) | (self.provider << 8);
        let subframe = gather.add(&word.to_be_bytes(), 0)?;
        let payload = gather.add(
            &self.submission.backing()[..size],
            self.submission.alignment(),
        )?;
        let gap = payload.start - subframe.end;
        let packed = (word & 0xffff_ff01)
            | ((u32::try_from(gap).map_err(|_| E::Alignment(gap))? * 2) & 0xfe);
        gather.bytes[subframe].copy_from_slice(&packed.to_be_bytes());
        if gap > 127 {
            return Err(E::Alignment(gap));
        }
        self.submission.reset_metadata();
        let frame = GatheredPresentationFrame {
            payload,
            buffer_index: index,
            generation: self.generations[index],
            owner: std::sync::Arc::clone(&self.owner),
        };
        self.current = Some(frame.clone());
        Ok(frame)
    }
    pub(crate) fn backing_for(
        &self,
        frame: &GatheredPresentationFrame,
    ) -> Result<&[u8], PresentationBufferError> {
        if !std::sync::Arc::ptr_eq(&self.owner, &frame.owner)
            || self.generations.get(frame.buffer_index) != Some(&frame.generation)
        {
            return Err(PresentationBufferError::StaleFrame);
        }
        self.gathers
            .get(frame.buffer_index)
            .and_then(|g| g.backing().get(frame.payload.start..))
            .ok_or(PresentationBufferError::Bounds {
                start: frame.payload.start,
                size: frame.payload.len(),
                capacity: self.gathers[0].bytes.len(),
            })
    }
}
impl PresentationDataSubmission for SelectedPresentationBuffers {
    type Error = PresentationBufferError;
    fn submit_data(&mut self, bytes: &[u8], alignment: usize) -> Result<(), Self::Error> {
        if !self.allowed {
            return Err(PresentationBufferError::SubmissionState);
        }
        self.submission.add(bytes, alignment)?;
        Ok(())
    }
}
#[cfg(test)]
#[path = "presentation_data_buffer_tests.rs"]
mod tests;
