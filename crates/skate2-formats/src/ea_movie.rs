//! EA chunked movies (`data\movies\*_english_ntsc.vp6`), split as
//! `rw::movie::ContainerParser_EAChunk::ParseFunction` (`0x82b88ff8`) does:
//! every chunk is a four-byte tag and a little-endian size that includes the
//! eight header bytes. `MVhd` describes the video, chunks tagged `MV..` or
//! `MP..` carry video frames (`MV0K` key frames, `MV0F` the rest), and the
//! audio of the selected language comes in `SH`+language (header) and
//! `SD`+language (data) chunks; without a language filter, `SCHl` and
//! `SCDl`. Everything else (`SCEN`, `SEEN`, the other languages) is skipped.

use std::fmt;
use std::io::{self, Read};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EaMovieError {
    Truncated { offset: usize },
    InvalidChunkSize { offset: usize, size: u32 },
    MissingHeader,
}

impl fmt::Display for EaMovieError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid EA movie: {self:?}")
    }
}

impl std::error::Error for EaMovieError {}

/// The `MVhd` fields the parser stores in the container header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EaMovie {
    /// The codec FourCC at +8 (`vp60` in the shipped movies).
    pub codec: [u8; 4],
    pub width: u16,
    pub height: u16,
    pub frame_count: u32,
    /// The rate's numerator at +0x18 (32 bits) and denominator at +0x1C
    /// (the parser reads 16 bits of it).
    pub rate_numerator: u32,
    pub rate_denominator: u16,
}

impl EaMovie {
    /// The `MVhd` chunk at the start of a movie.
    ///
    /// # Errors
    ///
    /// Returns [`EaMovieError`] when the movie does not start with a
    /// complete `MVhd` chunk.
    pub fn parse_header(bytes: &[u8]) -> Result<Self, EaMovieError> {
        let chunk = chunks(bytes).next().ok_or(EaMovieError::MissingHeader)??;
        if &chunk.tag != b"MVhd" {
            return Err(EaMovieError::MissingHeader);
        }
        Self::from_chunk(&chunk)
    }

    /// The header fields of an `MVhd` chunk.
    ///
    /// # Errors
    ///
    /// Returns [`EaMovieError::Truncated`] for a chunk shorter than 0x20
    /// bytes.
    pub fn from_chunk(chunk: &Chunk<'_>) -> Result<Self, EaMovieError> {
        let p = chunk.payload;
        if p.len() < 0x18 {
            return Err(EaMovieError::Truncated {
                offset: chunk.offset,
            });
        }
        let u16_at = |at: usize| u16::from_le_bytes([p[at], p[at + 1]]);
        let u32_at = |at: usize| u32::from_le_bytes([p[at], p[at + 1], p[at + 2], p[at + 3]]);
        Ok(Self {
            codec: [p[0], p[1], p[2], p[3]],
            width: u16_at(4),
            height: u16_at(6),
            frame_count: u32_at(8),
            rate_numerator: u32_at(0x10),
            rate_denominator: u16_at(0x14),
        })
    }

    /// `fps`: the numerator over the denominator, as single floats.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "the native float division")]
    pub fn fps(&self) -> f32 {
        self.rate_numerator as f32 / f32::from(self.rate_denominator)
    }
}

/// What the parser does with a chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChunkKind {
    Header,
    Video,
    AudioHeader,
    Audio,
    Skipped,
}

/// One chunk: its tag, the bytes after the eight header bytes and its file
/// offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chunk<'a> {
    pub tag: [u8; 4],
    pub payload: &'a [u8],
    pub offset: usize,
}

impl Chunk<'_> {
    /// The chunk's stream for a two-letter audio language filter (`EN`,
    /// `FR` or `GE`, `AptMovieIntegration::Init` `0x82634c30`).
    #[must_use]
    pub fn kind(&self, language: [u8; 2]) -> ChunkKind {
        kind(self.tag, language)
    }
}

fn kind(tag: [u8; 4], language: [u8; 2]) -> ChunkKind {
    if &tag == b"MVhd" {
        ChunkKind::Header
    } else if tag == [b'S', b'H', language[0], language[1]] {
        ChunkKind::AudioHeader
    } else if tag[0] == b'M' && (tag[1] == b'V' || tag[1] == b'P') {
        ChunkKind::Video
    } else if tag == [b'S', b'D', language[0], language[1]] {
        ChunkKind::Audio
    } else {
        ChunkKind::Skipped
    }
}

/// The chunks of a movie held in memory.
pub fn chunks(bytes: &[u8]) -> impl Iterator<Item = Result<Chunk<'_>, EaMovieError>> {
    let mut offset = 0;
    std::iter::from_fn(move || {
        if offset >= bytes.len() {
            return None;
        }
        let Some(head) = bytes.get(offset..offset + 8) else {
            let error = EaMovieError::Truncated { offset };
            offset = bytes.len();
            return Some(Err(error));
        };
        let size = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
        let end = offset + size as usize;
        if size < 8 || end > bytes.len() {
            let error = EaMovieError::InvalidChunkSize { offset, size };
            offset = bytes.len();
            return Some(Err(error));
        }
        let chunk = Chunk {
            tag: [head[0], head[1], head[2], head[3]],
            payload: &bytes[offset + 8..end],
            offset,
        };
        offset = end;
        Some(Ok(chunk))
    })
}

/// A chunk read from a stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedChunk {
    pub tag: [u8; 4],
    pub payload: Vec<u8>,
}

impl OwnedChunk {
    #[must_use]
    pub fn kind(&self, language: [u8; 2]) -> ChunkKind {
        kind(self.tag, language)
    }
}

/// The next chunk of a movie stream, `None` at its end.
///
/// # Errors
///
/// Returns the reader's error, or [`io::ErrorKind::InvalidData`] for a chunk
/// size below eight bytes.
pub fn read_chunk(reader: &mut impl Read) -> io::Result<Option<OwnedChunk>> {
    let mut head = [0u8; 8];
    let mut filled = 0;
    while filled < 8 {
        let n = reader.read(&mut head[filled..])?;
        if n == 0 {
            if filled == 0 {
                return Ok(None);
            }
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        filled += n;
    }
    let size = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
    if size < 8 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("EA movie chunk size {size}"),
        ));
    }
    let mut payload = vec![0u8; size as usize - 8];
    reader.read_exact(&mut payload)?;
    Ok(Some(OwnedChunk {
        tag: [head[0], head[1], head[2], head[3]],
        payload,
    }))
}

/// The audio stream description `rw::movie::Snd::SNDI_patchtohdrgen`
/// (`0x82b8d508`) reads from an audio header chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioHeader {
    pub channels: u32,
    pub sample_rate: u32,
    pub total_samples: u32,
    /// `samplerep`: 10 is EA-XA (`AUDIO_FORMAT_GUID_EAXA_BLK`), 23
    /// EALayer3.
    pub codec: u32,
}

/// `samplerep` 10: the EA-XA block codec (`rw::movie::AudioDecoder_EAXA`).
pub const AUDIO_CODEC_EAXA: u32 = 10;

impl AudioHeader {
    /// The tagged patch after the chunk's first eight payload bytes
    /// (`GSTR` and a word): `SNDI_gettag` (`0x82b8d410`) skips 0xFC padding,
    /// stops at 0xFF or 0xFE, and stores the values of tags 0x80 to 0xA7;
    /// 0x82 is the channel count, 0x84 the rate, 0x85 the sample count and
    /// 0xA0 the codec. Defaults: one channel, 48000 Hz, codec 10.
    #[must_use]
    pub fn parse(payload: &[u8]) -> Self {
        let mut header = Self {
            channels: 1,
            sample_rate: 48000,
            total_samples: 0,
            codec: AUDIO_CODEC_EAXA,
        };
        let mut at = 8;
        loop {
            while payload.get(at) == Some(&0xFC) {
                at += 1;
            }
            let Some(&tag) = payload.get(at) else { break };
            if tag == 0xFF {
                break;
            }
            at += 1;
            if tag == 0xFD || tag == 0xFE {
                if tag == 0xFE {
                    break;
                }
                continue;
            }
            let Some(&size_byte) = payload.get(at) else {
                break;
            };
            let mut size = usize::from(size_byte);
            if size_byte == 0xFF {
                size = be_value(payload, at + 1, 4) as usize;
                at += 4;
            }
            at += 1;
            let value = if size <= 4 {
                be_value(payload, at, size)
            } else {
                0
            };
            at += size;
            match tag {
                0x82 => header.channels = value,
                0x84 => header.sample_rate = value,
                0x85 => header.total_samples = value,
                0xA0 => header.codec = value,
                _ => {}
            }
        }
        header
    }
}

/// `SNDI_getb`: a big-endian value of `size` bytes.
fn be_value(bytes: &[u8], at: usize, size: usize) -> u32 {
    (0..size).fold(0, |value, i| {
        (value << 8) | u32::from(bytes.get(at + i).copied().unwrap_or(0))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = tag.to_vec();
        bytes.extend_from_slice(&(u32::try_from(payload.len()).unwrap() + 8).to_le_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn reads_the_movie_header_and_classifies_chunks() {
        let mut header = b"vp60".to_vec();
        header.extend_from_slice(&1280u16.to_le_bytes());
        header.extend_from_slice(&720u16.to_le_bytes());
        header.extend_from_slice(&988u32.to_le_bytes());
        header.extend_from_slice(&0x0001_4954u32.to_le_bytes());
        header.extend_from_slice(&982_047u32.to_le_bytes());
        header.extend_from_slice(&32767u32.to_le_bytes());
        let mut bytes = chunk(b"MVhd", &header);
        for tag in [
            b"SHEN", b"SHFR", b"MV0K", b"SCEN", b"SDEN", b"SDGE", b"MV0F", b"SEEN",
        ] {
            bytes.extend(chunk(tag, &[0; 4]));
        }
        let movie = EaMovie::parse_header(&bytes).unwrap();
        assert_eq!(
            (movie.width, movie.height, movie.frame_count),
            (1280, 720, 988)
        );
        assert!((movie.fps() - 29.97).abs() < 0.001);
        let kinds: Vec<_> = chunks(&bytes).map(|c| c.unwrap().kind(*b"EN")).collect();
        use ChunkKind::*;
        assert_eq!(
            kinds,
            [
                Header,
                AudioHeader,
                Skipped,
                Video,
                Skipped,
                Audio,
                Skipped,
                Video,
                Skipped
            ]
        );
        let mut reader = &bytes[..];
        let mut count = 0;
        while read_chunk(&mut reader).unwrap().is_some() {
            count += 1;
        }
        assert_eq!(count, 9);
    }

    #[test]
    fn parses_the_tagged_audio_header() {
        let mut payload = b"GSTR\x01\0\0\0".to_vec();
        payload.extend_from_slice(&[
            0x00, 0x04, 0x1d, 0xf8, 0x51, 0xe0, 0x06, 0x01, 0x65, 0xfd, 0x80, 0x01, 0x03,
        ]);
        payload.extend_from_slice(&[
            0x85, 0x03, 0x18, 0x25, 0x2d, 0x82, 0x01, 0x06, 0xff, 0x00, 0x00,
        ]);
        let header = AudioHeader::parse(&payload);
        assert_eq!(
            header,
            AudioHeader {
                channels: 6,
                sample_rate: 48000,
                total_samples: 1_582_381,
                codec: 10
            }
        );
    }
}
