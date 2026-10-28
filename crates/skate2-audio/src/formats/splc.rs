//! Splicer banks (`SPLC` version 3, `SpliceManager::LoadSpliceBank` 0x82a7cd08 /
//! `AddToContainer` 0x82a7bf60). Big-endian.

use std::sync::Arc;

/// `Splicer::SPLICE_SampleRef` (72 bytes).
#[derive(Debug, Clone, PartialEq)]
pub struct SampleRef {
    pub sample_index: u16,
    pub splice_type: i8,
    pub voice_type: u8,
    pub volume: f32,
    pub pitch: f32,
    pub time_stretch: f32,
    pub az: f32,
    pub offset: f32,
    pub seek_point: f32,
    pub duration: f32,
    pub fade_in: f32,
    pub fade_out: f32,
    pub fade_curve_types: u8,
    pub rnd_vol: f32,
    pub rnd_pitch: f32,
    pub rnd_offset: f32,
    pub pitch_weighting: f32,
    pub priority: u8,
    pub roll_off_type: u8,
    pub probability_of_playing: f32,
    pub lfe_only: bool,
}

/// `Splicer::SPLICE_SampleCluster`: one layer of a splice, a choice among sample refs.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleCluster {
    /// `meRandomizationField`: the selection state `GenerateRandomIndex` keeps in the bank.
    pub randomization_field: u32,
    pub randomization_type: u8,
    pub refs: Vec<SampleRef>,
}

/// `Splicer::SPLICE_Data` (36 bytes).
#[derive(Debug, Clone, PartialEq)]
pub struct SpliceData {
    pub name_hash: u32,
    pub splice_index: u16,
    pub splice_type: i8,
    pub volume: f32,
    pub pitch: f32,
    pub rnd_pitch: f32,
    pub rnd_vol: f32,
    pub duration: f32,
    pub voice_type: u8,
    pub clusters: Vec<SampleCluster>,
}

/// `Splicer::SPLICE_SpliceClusters` (72 bytes): a random choice among splices.
#[derive(Debug, Clone, PartialEq)]
pub struct SpliceCluster {
    /// `muRandomHistory`: the selection state `GenerateRandomIndex` keeps in the bank.
    pub random_history: u32,
    pub splice_indices: Vec<u16>,
    pub randomization_type: u8,
}

/// `SpliceContainer::TableOfContentsEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TocEntry {
    pub sample_data: i32,
    pub seek_table: i32,
    pub hash: u32,
}

#[derive(Debug)]
pub struct SpliceBank {
    pub identifier: String,
    pub splices: Vec<SpliceData>,
    pub clusters: Vec<SpliceCluster>,
    pub toc: Vec<TocEntry>,
    /// The whole file; sample data starts at `sample_base`.
    pub bytes: Arc<[u8]>,
    pub sample_base: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplcError {
    Magic,
    Version(i32),
    Truncated,
}

struct Reader<'a> {
    data: &'a [u8],
}

impl Reader<'_> {
    fn bytes<const N: usize>(&self, at: usize) -> Result<[u8; N], SplcError> {
        self.data.get(at..at + N).and_then(|b| b.try_into().ok()).ok_or(SplcError::Truncated)
    }
    fn u8(&self, at: usize) -> Result<u8, SplcError> {
        Ok(self.bytes::<1>(at)?[0])
    }
    fn u16(&self, at: usize) -> Result<u16, SplcError> {
        Ok(u16::from_be_bytes(self.bytes(at)?))
    }
    fn i32(&self, at: usize) -> Result<i32, SplcError> {
        Ok(i32::from_be_bytes(self.bytes(at)?))
    }
    fn u32(&self, at: usize) -> Result<u32, SplcError> {
        Ok(u32::from_be_bytes(self.bytes(at)?))
    }
    fn f32(&self, at: usize) -> Result<f32, SplcError> {
        Ok(f32::from_be_bytes(self.bytes(at)?))
    }
}

fn sample_ref(r: &Reader<'_>, at: usize) -> Result<SampleRef, SplcError> {
    Ok(SampleRef {
        sample_index: r.u16(at)?,
        splice_type: r.u8(at + 2)? as i8,
        voice_type: r.u8(at + 3)?,
        volume: r.f32(at + 0x04)?,
        pitch: r.f32(at + 0x08)?,
        time_stretch: r.f32(at + 0x0C)?,
        az: r.f32(at + 0x10)?,
        offset: r.f32(at + 0x14)?,
        seek_point: r.f32(at + 0x18)?,
        duration: r.f32(at + 0x1C)?,
        fade_in: r.f32(at + 0x20)?,
        fade_out: r.f32(at + 0x24)?,
        fade_curve_types: r.u8(at + 0x28)?,
        rnd_vol: r.f32(at + 0x2C)?,
        rnd_pitch: r.f32(at + 0x30)?,
        rnd_offset: r.f32(at + 0x34)?,
        pitch_weighting: r.f32(at + 0x38)?,
        priority: r.u8(at + 0x3C)?,
        roll_off_type: r.u8(at + 0x3D)?,
        probability_of_playing: r.f32(at + 0x40)?,
        lfe_only: r.u8(at + 0x44)? != 0,
    })
}

impl SpliceBank {
    /// Parses a bank; `splice_type` is the slot it loads into (written into every sample ref,
    /// as `AddToContainer` does).
    pub fn parse(bytes: Arc<[u8]>, splice_type: i8) -> Result<Self, SplcError> {
        let r = Reader { data: &bytes };
        if r.bytes::<4>(0)? != *b"SPLC" {
            return Err(SplcError::Magic);
        }
        let version = r.i32(4)?;
        if version != 3 {
            return Err(SplcError::Version(version));
        }
        let meta_size = r.i32(8)?.max(0) as usize;
        let num_splices = r.i32(12)?.max(0) as usize;
        let num_clusters = r.i32(16)?.max(0) as usize;
        let num_loops = r.i32(20)?.max(0) as usize;
        let num_samples = r.i32(24)?.max(0) as usize;
        let identifier = String::from_utf8_lossy(&r.bytes::<32>(28)?).trim_end_matches('\0').to_owned();
        let meta = 60usize;
        let mut cursor = meta + 36 * num_splices + 72 * num_clusters + 24 * num_loops;
        let mut splices = Vec::with_capacity(num_splices);
        for i in 0..num_splices {
            let at = meta + 36 * i;
            let count = usize::from(r.u8(at + 7)?);
            let mut clusters = Vec::with_capacity(count);
            for _ in 0..count {
                let randomization_field = r.u32(cursor + 4)?;
                let refs_count = usize::from(r.u8(cursor + 8)?);
                let randomization_type = r.u8(cursor + 9)?;
                let mut refs = Vec::with_capacity(refs_count);
                for k in 0..refs_count {
                    let mut sample = sample_ref(&r, cursor + 12 + 72 * k)?;
                    sample.splice_type = splice_type;
                    refs.push(sample);
                }
                cursor += 12 + 72 * refs_count;
                clusters.push(SampleCluster { randomization_field, randomization_type, refs });
            }
            splices.push(SpliceData {
                name_hash: r.u32(at)?,
                splice_index: r.u16(at + 4)?,
                splice_type,
                volume: r.f32(at + 8)?,
                pitch: r.f32(at + 0x0C)?,
                rnd_pitch: r.f32(at + 0x10)?,
                rnd_vol: r.f32(at + 0x14)?,
                duration: r.f32(at + 0x18)?,
                voice_type: r.u8(at + 0x1C)?,
                clusters,
            });
        }
        let mut clusters = Vec::with_capacity(num_clusters);
        for i in 0..num_clusters {
            let at = meta + 36 * num_splices + 72 * i;
            let count = usize::from(r.u8(at + 0x44)?).min(32);
            let splice_indices = (0..count).map(|k| r.u16(at + 4 + 2 * k)).collect::<Result<_, _>>()?;
            clusters.push(SpliceCluster {
                random_history: r.u32(at)?,
                splice_indices,
                randomization_type: r.u8(at + 0x45)?,
            });
        }
        let mut toc_at = 60 + meta_size;
        toc_at += (4 - toc_at % 4) % 4;
        let toc = (0..num_samples)
            .map(|i| {
                let at = toc_at + 12 * i;
                Ok(TocEntry { sample_data: r.i32(at)?, seek_table: r.i32(at + 4)?, hash: r.u32(at + 8)? })
            })
            .collect::<Result<Vec<_>, SplcError>>()?;
        let mut sample_base = toc_at + 12 * num_samples;
        sample_base += (4 - sample_base % 4) % 4;
        Ok(Self { identifier, splices, clusters, toc, bytes, sample_base })
    }
}
