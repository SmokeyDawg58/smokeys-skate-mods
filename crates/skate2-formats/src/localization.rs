//! EA Localizer1.4 language categories and wide-character histograms.
//! The shipped language BIN payloads are little endian even on Xbox360:
//! AddCategory8297f128, LangPackAddCategory8297ede0, EndianSwap829825d0.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalizationError {
    Bounds,
    ChunkId(u32),
    CategoryName,
    RecordOrder,
    UnterminatedString,
    HistogramIndex(usize),
}
impl fmt::Display for LocalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid Localizer data: {self:?}")
    }
}
impl std::error::Error for LocalizationError {}

fn word(bytes: &[u8], offset: usize) -> Result<u32, LocalizationError> {
    let end = offset.checked_add(4).ok_or(LocalizationError::Bounds)?;
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..end)
            .ok_or(LocalizationError::Bounds)?
            .try_into()
            .unwrap(),
    ))
}
fn payload(bytes: &[u8], expected: u32) -> Result<&[u8], LocalizationError> {
    let id = word(bytes, 0)?;
    if id != expected {
        return Err(LocalizationError::ChunkId(id));
    }
    let size = word(bytes, 4)? as usize;
    bytes
        .get(8..8usize.checked_add(size).ok_or(LocalizationError::Bounds)?)
        .ok_or(LocalizationError::Bounds)
}

/// ICELITE::StringHash8297e018. Case-sensitive, seed -1, terminates at NUL.
/// Deliberately distinct from BIG's DJB2 (which starts from5381).
pub fn string_hash(label: &[u8]) -> u32 {
    label
        .iter()
        .copied()
        .take_while(|&v| v != 0)
        .fold(u32::MAX, |hash, byte| {
            hash.wrapping_mul(33).wrapping_add(u32::from(byte))
        })
}

#[derive(Clone, Debug)]
pub struct LanguageCategory<'a> {
    pub name: &'a str,
    records: Vec<(u32, &'a [u8])>,
}
impl<'a> LanguageCategory<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, LocalizationError> {
        let data = payload(bytes, 0x39000)?;
        let count = word(data, 0)? as usize;
        let records_start = word(data, 4)? as usize;
        let strings_start = word(data, 8)? as usize;
        let name = data.get(12..28).ok_or(LocalizationError::Bounds)?;
        let name_len = name.iter().position(|&v| v == 0).unwrap_or(name.len());
        let name =
            std::str::from_utf8(&name[..name_len]).map_err(|_| LocalizationError::CategoryName)?;
        let records_size = count.checked_mul(8).ok_or(LocalizationError::Bounds)?;
        let end = records_start
            .checked_add(records_size)
            .ok_or(LocalizationError::Bounds)?;
        let table = data
            .get(records_start..end)
            .ok_or(LocalizationError::Bounds)?;
        let strings = data.get(strings_start..).ok_or(LocalizationError::Bounds)?;
        let mut records = Vec::with_capacity(count);
        for record in table.chunks_exact(8) {
            let hash = word(record, 0)?;
            if records
                .last()
                .is_some_and(|&(previous, _)| previous >= hash)
            {
                return Err(LocalizationError::RecordOrder);
            }
            let offset = word(record, 4)? as usize;
            let string = strings.get(offset..).ok_or(LocalizationError::Bounds)?;
            let len = string
                .iter()
                .position(|&v| v == 0)
                .ok_or(LocalizationError::UnterminatedString)?;
            records.push((hash, &string[..=len]));
        }
        Ok(Self { name, records })
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
    /// Original sorted hash table, for a retained frontend string cache.
    pub fn packed_strings(&self) -> impl Iterator<Item = (u32, &'a [u8])> + '_ {
        self.records.iter().copied()
    }
    /// SearchForString8297e248. A miss remains a miss; the manager's database
    /// priority and localization-error policy belong to its caller.
    pub fn packed_string(&self, hash: u32) -> Option<&'a [u8]> {
        self.records
            .binary_search_by_key(&hash, |r| r.0)
            .ok()
            .map(|i| self.records[i].1)
    }
}

#[derive(Clone, Debug)]
pub struct Histogram {
    entries: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnpackedString {
    /// UCS2 code units, excluding the terminating zero.
    pub units: Vec<u16>,
    /// Native UnpackString reports false after substituting '_' for an
    /// invalid two-byte code. Keep that status rather than concealing it.
    pub valid: bool,
    pub terminated: bool,
}

impl Histogram {
    pub fn parse(bytes: &[u8]) -> Result<Self, LocalizationError> {
        let data = payload(bytes, 0x39001)?;
        let count = word(data, 0)? as usize;
        let end = 4usize
            .checked_add(count.checked_mul(2).ok_or(LocalizationError::Bounds)?)
            .ok_or(LocalizationError::Bounds)?;
        let table = data.get(4..end).ok_or(LocalizationError::Bounds)?;
        Ok(Self {
            entries: table
                .chunks_exact(2)
                .map(|v| u16::from_le_bytes([v[0], v[1]]))
                .collect(),
        })
    }
    fn entry(&self, index: usize) -> Result<u16, LocalizationError> {
        self.entries
            .get(index)
            .copied()
            .ok_or(LocalizationError::HistogramIndex(index))
    }
    /// LocalizerHistogram::UnpackString829824e8, including the native
    /// DisableWideStringHistogram option and the destination-size boundary.
    pub fn unpack(
        &self,
        packed: &[u8],
        capacity: usize,
        disabled: bool,
    ) -> Result<UnpackedString, LocalizationError> {
        let mut result = UnpackedString {
            units: Vec::new(),
            valid: true,
            terminated: false,
        };
        let mut cursor = 0;
        for _ in 0..capacity {
            let byte = *packed
                .get(cursor)
                .ok_or(LocalizationError::UnterminatedString)?;
            cursor += 1;
            let mut value = u16::from(byte);
            if byte >= 128 && !disabled {
                value = self.entry(usize::from(byte))?;
                if value < 128 {
                    if value == 0 {
                        value = u16::from(byte);
                    } else {
                        let second = *packed
                            .get(cursor)
                            .ok_or(LocalizationError::UnterminatedString)?;
                        cursor += 1;
                        value = if second < 128 {
                            0
                        } else {
                            self.entry(128 * usize::from(value) + usize::from(second) - 128)?
                        };
                    }
                }
                if value == 0 {
                    result.valid = false;
                    value = u16::from(b'_');
                }
            }
            if value == 0 {
                result.terminated = true;
                break;
            }
            result.units.push(value);
        }
        Ok(result)
    }
}

#[cfg(test)]
#[path = "localization_tests.rs"]
mod tests;
