//! Skate 3's `EB` (version 3) BIG archives, as its `parkassets.big` is: a 48-byte header, a
//! 16-byte entry per file (offset in alignment units, stored size, size, name hash), a byte per
//! file, then fixed-width name records (a folder index and the file name) and fixed-width
//! folder names. A stored size of 0 is a file kept as it is; otherwise its data is `chunkref`:
//! a 40-byte header (`chunkref`, version, size, chunk size, chunk count, alignment), then each
//! chunk's stored size, its kind and its `RefPack` data, each chunk's data aligned.
//!
//! Every field is big-endian. Only what reading named files needs is kept.

use std::{
    collections::HashMap,
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

const HEADER_SIZE: usize = 0x30;
const ENTRY_SIZE: usize = 16;
const CHUNK_HEADER_SIZE: usize = 0x28;

#[derive(Clone, Copy, Debug)]
struct Entry {
    offset: u64,
    stored: u32,
    size: u32,
}

/// An opened `EB` archive's table: each file by its path (lower case, backslashes).
#[derive(Clone, Debug)]
pub struct EbArchive {
    path: PathBuf,
    embedded: Option<&'static [u8]>,
    entries: HashMap<String, Entry>,
}

fn be16(bytes: &[u8], at: usize) -> Result<u16, String> {
    Ok(u16::from_be_bytes(bytes.get(at..at + 2).ok_or("EB archive truncated")?.try_into().map_err(|_| "EB field")?))
}

fn be32(bytes: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_be_bytes(bytes.get(at..at + 4).ok_or("EB archive truncated")?.try_into().map_err(|_| "EB field")?))
}

fn text(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&byte| byte == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// A path as the table keys it.
fn key(path: &str) -> String {
    path.replace('/', "\\").to_ascii_lowercase()
}

impl EbArchive {
    /// Reads the archive's table.
    ///
    /// # Errors
    ///
    /// The file cannot be read or is not an `EB` version 3 archive.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_owned();
        let mut file = File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::read_table(path, None, &mut file)
    }

    /// Opens a bundled archive directly from the executable, without writing game files.
    pub fn open_embedded(bytes: &'static [u8]) -> Result<Self, String> {
        Self::read_table(PathBuf::from("bundled parkassets.big"), Some(bytes), &mut Cursor::new(bytes))
    }

    fn read_table(path: PathBuf, embedded: Option<&'static [u8]>, file: &mut impl Read) -> Result<Self, String> {
        let mut fixed = [0_u8; HEADER_SIZE];
        file.read_exact(&mut fixed).map_err(|e| format!("{}: {e}", path.display()))?;
        if &fixed[0..2] != b"EB" || be16(&fixed, 2)? != 3 {
            return Err(format!("{} is not an EB version 3 archive", path.display()));
        }
        let count = be32(&fixed, 4)? as usize;
        let shift = u32::from(fixed[10]);
        let names_offset = be32(&fixed, 12)? as usize;
        let names_size = be32(&fixed, 16)? as usize;
        let (name_width, folder_width) = (usize::from(fixed[20]), usize::from(fixed[21]));
        let folders = usize::from(be16(&fixed, 22)?);
        if name_width < 3 || count.checked_mul(name_width).is_none_or(|names| names + folders * folder_width > names_size) {
            return Err(format!("{}: EB name table does not fit", path.display()));
        }
        let mut table = vec![0_u8; names_offset + names_size];
        table[..HEADER_SIZE].copy_from_slice(&fixed);
        file.read_exact(&mut table[HEADER_SIZE..]).map_err(|e| format!("{}: {e}", path.display()))?;
        let folder_names: Vec<String> = (0..folders)
            .map(|folder| text(&table[names_offset + count * name_width + folder * folder_width..][..folder_width]))
            .collect();
        let mut entries = HashMap::with_capacity(count);
        for index in 0..count {
            let at = HEADER_SIZE + index * ENTRY_SIZE;
            let entry = Entry { offset: u64::from(be32(&table, at)?) << shift, stored: be32(&table, at + 4)?, size: be32(&table, at + 8)? };
            let record = &table[names_offset + index * name_width..][..name_width];
            let folder = folder_names.get(usize::from(be16(record, 0)?)).ok_or_else(|| format!("{}: EB folder index", path.display()))?;
            entries.insert(key(&format!("{folder}/{}", text(&record[2..]))), entry);
        }
        Ok(Self { path, embedded, entries })
    }

    /// How many files the archive holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether the archive holds `path` (`data\content\...`, either slash, any case).
    #[must_use]
    pub fn contains(&self, path: &str) -> bool {
        self.entries.contains_key(&key(path))
    }

    /// One file, expanded.
    ///
    /// # Errors
    ///
    /// The file is absent, cannot be read, or its chunks do not expand to its size.
    pub fn read_path(&self, path: &str) -> Result<Vec<u8>, String> {
        let entry = *self.entries.get(&key(path)).ok_or_else(|| format!("{path} is not in {}", self.path.display()))?;
        let mut stored = vec![0_u8; if entry.stored == 0 { entry.size } else { entry.stored } as usize];
        if let Some(bytes) = self.embedded {
            let mut file = Cursor::new(bytes);
            file.seek(SeekFrom::Start(entry.offset)).map_err(|e| format!("{path}: {e}"))?;
            file.read_exact(&mut stored).map_err(|e| format!("{path}: {e}"))?;
        } else {
            let mut file = File::open(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?;
            file.seek(SeekFrom::Start(entry.offset)).map_err(|e| format!("{path}: {e}"))?;
            file.read_exact(&mut stored).map_err(|e| format!("{path}: {e}"))?;
        }
        if entry.stored == 0 {
            return Ok(stored);
        }
        let bytes = if stored.starts_with(b"chunkref") { expand_chunks(&stored).map_err(|e| format!("{path}: {e}"))? } else { crate::refpack::decompress(&stored).map_err(|e| format!("{path}: {e:?}"))? };
        if bytes.len() != entry.size as usize {
            return Err(format!("{path}: expanded to {} bytes, not {}", bytes.len(), entry.size));
        }
        Ok(bytes)
    }
}

/// A `chunkref` file's chunks, each `RefPack` (or kept as it is), joined.
fn expand_chunks(stored: &[u8]) -> Result<Vec<u8>, String> {
    let size = be32(stored, 12)? as usize;
    let count = be32(stored, 20)? as usize;
    let align = (be32(stored, 24)? as usize).max(1);
    let mut out = Vec::with_capacity(size);
    let mut cursor = CHUNK_HEADER_SIZE;
    for chunk in 0..count {
        let length = be32(stored, cursor)? as usize;
        let data = stored.get(cursor + 8..cursor + 8 + length).ok_or_else(|| format!("chunk {chunk} outside the file"))?;
        if crate::refpack::decompressed_size(data).is_ok() {
            out.extend(crate::refpack::decompress(data).map_err(|e| format!("chunk {chunk}: {e:?}"))?);
        } else {
            out.extend_from_slice(data);
        }
        // The next chunk's data (after its 8-byte stored size and kind) is what is aligned.
        cursor = (cursor + 8 + length + 8).div_ceil(align) * align - 8;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every file in Skate 3's `parkassets.big` expands to the bytes bigfile.exe extracted.
    #[test]
    #[ignore = "requires Skate 3's parkassets.big (SKATE3_PARKASSETS_BIG) and its extraction (SKATE3_PARKASSETS)"]
    fn parkassets_expand_like_bigfile() {
        let archive = EbArchive::open(std::env::var("SKATE3_PARKASSETS_BIG").unwrap()).unwrap();
        let root = PathBuf::from(std::env::var("SKATE3_PARKASSETS").unwrap());
        let mut checked = 0;
        for name in archive.entries.keys() {
            let wanted = std::fs::read(root.join(name.replace('\\', "/"))).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(archive.read_path(name).unwrap(), wanted, "{name}");
            checked += 1;
        }
        eprintln!("EB RESULT {checked} files match");
        assert_eq!(checked, 1296);
    }
}
