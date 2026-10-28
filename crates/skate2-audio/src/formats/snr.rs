//! EAAC sample headers ("SNR", `SndPlayer1::UnpackHeader` 0x82bf2390) read with
//! `rw::audio::core::BitGetter` (most significant bit first).

pub struct BitGetter<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> BitGetter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    /// `BitGetter::GetBits`, up to 32 bits.
    pub fn get(&mut self, count: u32) -> u32 {
        let mut value = 0u32;
        for _ in 0..count {
            let byte = self.data.get(self.position >> 3).copied().unwrap_or(0);
            value = (value << 1) | u32::from(byte >> (7 - (self.position & 7)) & 1);
            self.position += 1;
        }
        value
    }

    pub fn position(&self) -> usize {
        self.position
    }
}

/// `SndPlayer1::GetFileInfo` 0x82bf20a0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileInfo {
    pub codec: u8,
    pub channels: u8,
    pub sample_rate: u32,
    pub num_samples: u32,
}

pub fn file_info(data: &[u8]) -> FileInfo {
    let data = if data.first() == Some(&b'H') { data.get(4..).unwrap_or(&[]) } else { data };
    let mut bits = BitGetter::new(data);
    bits.get(4);
    let codec = bits.get(4) as u8;
    let channels = bits.get(6) as u8 + 1;
    let sample_rate = bits.get(18);
    bits.get(3);
    let num_samples = bits.get(29);
    FileInfo { codec, channels, sample_rate, num_samples }
}
