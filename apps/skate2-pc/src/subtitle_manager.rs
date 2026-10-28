//! Retained nested BIG archives owned by the reconstructed subtitle manager.
//!
//! `SubtitleManager::LoadBigSubtitleFile` synchronously loads a complete BIG
//! image and appends a wrapper to the manager-private list. It is deliberately
//! separate from `MountSet`: the native archive participates only in subtitle
//! lookup and is purged by `SubtitleManager::Purge`.

use skate2_formats::{
    big::{BigArchive, BigError},
    mount::{MountError, MountSet},
};
use std::{error::Error, fmt, sync::Arc};

#[derive(Debug)]
struct OwnedSubtitleBig {
    bytes: Vec<u8>,
    archive: BigArchive,
}

/// Manager-private ordered subtitle BIG ownership.
#[derive(Debug, Default)]
pub struct SubtitleArchiveStore {
    archives: Vec<OwnedSubtitleBig>,
}

impl SubtitleArchiveStore {
    /// Parses and appends one complete BIG image without duplicate suppression.
    ///
    /// # Errors
    ///
    /// Returns [`BigError`] when the supplied image has an invalid BIG table.
    pub fn append(&mut self, bytes: Vec<u8>) -> Result<(), BigError> {
        let archive = BigArchive::parse(&bytes)?;
        self.archives.push(OwnedSubtitleBig { bytes, archive });
        Ok(())
    }

    #[must_use]
    pub fn archive_count(&self) -> usize {
        self.archives.len()
    }

    #[must_use]
    pub fn entry_count(&self, archive_index: usize) -> Option<usize> {
        self.archives
            .get(archive_index)
            .map(|owned| owned.archive.entries.len())
    }

    #[must_use]
    pub fn backing_len(&self, archive_index: usize) -> Option<usize> {
        self.archives
            .get(archive_index)
            .map(|owned| owned.bytes.len())
    }

    /// Mirrors `SubtitleManager::Purge`'s complete private-list clear.
    pub fn purge(&mut self) {
        self.archives.clear();
    }
}

/// Mounted source plus the private subtitle archive store.
#[derive(Debug)]
pub struct MountedSubtitleManager {
    mounts: Arc<MountSet>,
    store: SubtitleArchiveStore,
}

impl MountedSubtitleManager {
    #[must_use]
    pub fn new(mounts: Arc<MountSet>) -> Self {
        Self {
            mounts,
            store: SubtitleArchiveStore::default(),
        }
    }

    /// Synchronously reads, parses, and retains one nested BIG image.
    ///
    /// Repeated paths append repeated private wrappers, matching the recovered
    /// function's lack of duplicate suppression.
    ///
    /// # Errors
    ///
    /// Returns a checked source-read or nested-BIG parse failure.
    pub fn load_big_subtitle_file(&mut self, path: &str) -> Result<(), SubtitleLoadError> {
        let bytes = self
            .mounts
            .read_path(path)
            .map_err(SubtitleLoadError::Read)?;
        self.store.append(bytes).map_err(SubtitleLoadError::Parse)
    }

    #[must_use]
    pub fn archives(&self) -> &SubtitleArchiveStore {
        &self.store
    }

    pub fn purge(&mut self) {
        self.store.purge();
    }
}

#[derive(Debug)]
pub enum SubtitleLoadError {
    Read(MountError),
    Parse(BigError),
}

impl fmt::Display for SubtitleLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(source) => write!(formatter, "subtitle BIG read failed: {source}"),
            Self::Parse(source) => write!(formatter, "subtitle BIG parse failed: {source}"),
        }
    }
}

impl Error for SubtitleLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read(source) => Some(source),
            Self::Parse(source) => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_entry_big() -> Vec<u8> {
        let name = b"English\\line.sub\0";
        let header_size = 16 + 8 + name.len();
        let payload = b"subtitle";
        let mut bytes = Vec::with_capacity(header_size + payload.len());
        bytes.extend_from_slice(b"BIG4");
        bytes.extend_from_slice(
            &u32::try_from(header_size + payload.len())
                .unwrap()
                .to_le_bytes(),
        );
        bytes.extend_from_slice(&1_u32.to_be_bytes());
        bytes.extend_from_slice(&u32::try_from(header_size).unwrap().to_be_bytes());
        bytes.extend_from_slice(&u32::try_from(header_size).unwrap().to_be_bytes());
        bytes.extend_from_slice(&u32::try_from(payload.len()).unwrap().to_be_bytes());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn private_store_retains_ordered_duplicate_archives_until_purge() {
        let bytes = one_entry_big();
        let mut store = SubtitleArchiveStore::default();

        store.append(bytes.clone()).unwrap();
        store.append(bytes).unwrap();

        assert_eq!(store.archive_count(), 2);
        assert_eq!(store.entry_count(0), Some(1));
        assert_eq!(store.entry_count(1), Some(1));
        assert_eq!(store.backing_len(0), store.backing_len(1));

        store.purge();
        assert_eq!(store.archive_count(), 0);
    }

    #[test]
    fn invalid_nested_big_does_not_append_a_private_wrapper() {
        let mut store = SubtitleArchiveStore::default();

        assert!(store.append(vec![0; 16]).is_err());
        assert_eq!(store.archive_count(), 0);
    }
}
