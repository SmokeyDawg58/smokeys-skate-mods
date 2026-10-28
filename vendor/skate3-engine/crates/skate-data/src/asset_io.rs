//! Read-only assets embedded in a PE executable. No extraction or disk fallback
//! is performed for the reserved embedded root. Ordinary roots keep file I/O.
use std::{
    collections::BTreeMap,
    io,
    path::{Component, Path},
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

pub const ROOT: &str = "__skate3_embedded__/assets";
const MAGIC: &[u8; 8] = b"SK3PACK1";
static MOUNTED: OnceLock<Bundle<'static>> = OnceLock::new();
static DISK_READS: AtomicUsize = AtomicUsize::new(0);

pub fn disk_read_count() -> usize {
    DISK_READS.load(Ordering::Relaxed)
}

pub struct Bundle<'a> {
    source: &'a [u8],
    entries: BTreeMap<&'a str, &'a [u8]>,
    identity: String,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && !key.contains(['\\', ':', '\0'])
        && key
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

impl<'a> Bundle<'a> {
    pub fn parse(bytes: &'a [u8]) -> io::Result<Self> {
        struct Reader<'a> {
            bytes: &'a [u8],
            at: usize,
        }
        impl<'a> Reader<'a> {
            fn take(&mut self, count: usize) -> io::Result<&'a [u8]> {
                let end = self
                    .at
                    .checked_add(count)
                    .ok_or_else(|| invalid("Pack length overflow"))?;
                let data = self
                    .bytes
                    .get(self.at..end)
                    .ok_or_else(|| invalid("Truncated embedded pack"))?;
                self.at = end;
                Ok(data)
            }
            fn word(&mut self) -> io::Result<u32> {
                Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
            }
        }
        let mut reader = Reader { bytes, at: 0 };
        if reader.take(8)? != MAGIC {
            return Err(invalid("Unknown embedded asset pack"));
        }
        let count = usize::try_from(reader.word()?).map_err(|_| invalid("Pack count overflow"))?;
        if count > bytes.len().saturating_sub(reader.at) / 77 {
            return Err(invalid("Pack entry count exceeds remaining data"));
        }
        let mut entries = BTreeMap::new();
        for _ in 0..count {
            let key_len =
                usize::try_from(reader.word()?).map_err(|_| invalid("Pack path overflow"))?;
            let size = u64::from_le_bytes(reader.take(8)?.try_into().unwrap());
            let size = usize::try_from(size).map_err(|_| invalid("Pack data size overflow"))?;
            let digest = std::str::from_utf8(reader.take(64)?)
                .map_err(|_| invalid("Invalid pack digest"))?;
            let key = std::str::from_utf8(reader.take(key_len)?)
                .map_err(|_| invalid("Invalid pack path"))?;
            if !valid_key(key) {
                return Err(invalid(format!("Unsafe pack path {key:?}")));
            }
            let data = reader.take(size)?;
            if crate::sha256::digest(data) != digest {
                return Err(invalid(format!("Asset checksum mismatch: {key}")));
            }
            if entries.insert(key, data).is_some() {
                return Err(invalid(format!("Duplicate asset: {key}")));
            }
        }
        if reader.at != bytes.len() {
            return Err(invalid("Trailing embedded asset data"));
        }
        Ok(Self {
            source: bytes,
            entries,
            identity: crate::sha256::digest(bytes),
        })
    }

    pub fn get(&self, key: &str) -> Option<&'a [u8]> {
        self.entries.get(key).copied()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Bytes supplied by `include_bytes!` remain in the executable's mapped image.
pub fn mount(bytes: &'static [u8]) -> io::Result<&'static Path> {
    // The exact same immutable PE mapping was already fully validated.
    // A different mapping still goes through every checksum and identity check.
    if MOUNTED
        .get()
        .is_some_and(|bundle| std::ptr::eq(bundle.source, bytes))
    {
        return Ok(Path::new(ROOT));
    }
    let bundle = Bundle::parse(bytes)?;
    if let Some(current) = MOUNTED.get() {
        if current.identity != bundle.identity {
            return Err(invalid("A different embedded pack is already mounted"));
        }
    } else if let Err(bundle) = MOUNTED.set(bundle) {
        if MOUNTED.get().unwrap().identity != bundle.identity {
            return Err(invalid("Concurrent embedded packs differ"));
        }
    }
    Ok(Path::new(ROOT))
}

pub fn is_embedded_root(root: &Path) -> bool {
    root == Path::new(ROOT)
}

fn embedded_key(path: &Path) -> io::Result<Option<String>> {
    let Ok(relative) = path.strip_prefix(ROOT) else {
        return Ok(None);
    };
    let mut parts = Vec::new();
    for part in relative.components() {
        let Component::Normal(part) = part else {
            return Err(invalid("Invalid embedded asset path"));
        };
        parts.push(
            part.to_str()
                .ok_or_else(|| invalid("Non-UTF-8 embedded asset path"))?,
        );
    }
    let key = parts.join("/");
    if !valid_key(&key) {
        return Err(invalid("Invalid embedded asset key"));
    }
    Ok(Some(key))
}

pub fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let path = path.as_ref();
    if let Some(key) = embedded_key(path)? {
        return MOUNTED
            .get()
            .and_then(|bundle| bundle.get(&key))
            .map(<[u8]>::to_vec)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Embedded asset missing: {key}"),
                )
            });
    }
    DISK_READS.fetch_add(1, Ordering::Relaxed);
    std::fs::read(path)
}

pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    String::from_utf8(read(path)?).map_err(|error| invalid(error.to_string()))
}

pub fn is_file(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    match embedded_key(path) {
        Ok(Some(key)) => MOUNTED
            .get()
            .is_some_and(|bundle| bundle.get(&key).is_some()),
        Ok(None) => path.is_file(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (key, data) in entries {
            bytes.extend_from_slice(&(key.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&(data.len() as u64).to_le_bytes());
            bytes.extend_from_slice(crate::sha256::digest(data).as_bytes());
            bytes.extend_from_slice(key.as_bytes());
            bytes.extend_from_slice(data);
        }
        bytes
    }

    #[test]
    fn binary_assets_keep_exact_borrowed_bytes_and_integrity() {
        let bytes = pack(&[("private/a.bin", &[0, 255, 1]), ("private/empty", &[])]);
        let bundle = Bundle::parse(&bytes).unwrap();
        assert_eq!(bundle.len(), 2);
        assert_eq!(bundle.get("private/a.bin"), Some([0, 255, 1].as_slice()));
        let mut corrupt = bytes.clone();
        // Corrupt the first stored digest; no asset is accepted with wrong bytes.
        corrupt[24] ^= 1;
        assert!(Bundle::parse(&corrupt).is_err());
        assert!(Bundle::parse(&bytes[..bytes.len() - 1]).is_err());
        let mut trailing = bytes;
        trailing.push(0);
        assert!(Bundle::parse(&trailing).is_err());
    }

    #[test]
    fn duplicate_and_escaping_paths_are_rejected() {
        for key in [
            "../a",
            "private/../a",
            "/a",
            "C:/a",
            "private\\a",
            "private//a",
        ] {
            assert!(Bundle::parse(&pack(&[(key, b"data")])).is_err(), "{key}");
        }
        assert!(Bundle::parse(&pack(&[("a", b"1"), ("a", b"2")])).is_err());
    }

    #[test]
    fn mounted_reads_need_no_asset_directory_or_extraction() {
        let bytes =
            Box::leak(pack(&[("private/test.txt", b"inside executable")]).into_boxed_slice());
        let root = mount(bytes).unwrap();
        assert_eq!(
            read_to_string(root.join("private/test.txt")).unwrap(),
            "inside executable"
        );
        assert!(is_file(root.join("private/test.txt")));
        assert_eq!(
            read(root.join("missing")).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert!(read(root.join("../outside")).is_err());
        assert_eq!(mount(bytes).unwrap(), root);
        // A distinct immutable mapping still receives integrity validation.
        let mut damaged = bytes.to_vec();
        *damaged.last_mut().unwrap() ^= 1;
        assert!(mount(Box::leak(damaged.into_boxed_slice())).is_err());
        let other = Box::leak(pack(&[("private/other.txt", b"different pack")]).into_boxed_slice());
        assert!(mount(other).is_err());
    }
}
