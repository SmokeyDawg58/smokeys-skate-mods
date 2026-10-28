//! `Character::CASUnlockPartDB` (`casunlockpartdb.cpp`): the part types each
//! unlock group (`progression_asset_group_unlocks`/`_sponsors` key) unlocks.
//! `CASUnlockPartDB::Load` (`0x8259eb68`) reads
//! `data\content/recipe/createacharacter/` + the part database name +
//! `.unlocks`; `CASDLC::Init` (`0x8259e080`) names it `cas_db` and `_%03d` of
//! the on-disk part database version, `cas_db_000` without downloaded content.

/// The file the shipped game loads.
pub const PATH: &str = "data\\content/recipe/createacharacter/cas_db_000.unlocks";

/// `CASUnlockPartDB::Header::mVersion`.
const VERSION: u32 = 1;

/// The resolved `Header` (`mVersion`, `mDataSize`, `mRecipeName`,
/// `mNumUnlockEntries`, `mUnlockEntries`) and its 12-byte
/// `UnlockGroupEntry` records (`mUnlockGroupKey`, `mUnlockTypes`, padding),
/// sorted by key.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CasUnlockPartDb {
    entries: Vec<(u64, u16)>,
}

fn word(data: &[u8], offset: usize) -> Result<u32, String> {
    data.get(offset..offset + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| format!("unlock part database ends before {offset:#x}"))
}

impl CasUnlockPartDb {
    /// `CASUnlockPartDB::Resolve` (`0x82585b88`): version 1, `mDataSize` the
    /// whole file, then the entry offset relocated.
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        let version = word(data, 0)?;
        if version != VERSION {
            return Err(format!("unlock part database version {version}"));
        }
        let size = word(data, 4)?;
        if usize::try_from(size).ok() != Some(data.len()) {
            return Err(format!("unlock part database size {size} for {} bytes", data.len()));
        }
        let count = usize::try_from(word(data, 12)?).map_err(|e| e.to_string())?;
        let start = usize::try_from(word(data, 16)?).map_err(|e| e.to_string())?;
        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let at = start + 12 * i;
            let key = (u64::from(word(data, at)?) << 32) | u64::from(word(data, at + 4)?);
            let types = data.get(at + 8..at + 10).ok_or("unlock part database entry truncated")?;
            entries.push((key, u16::from_be_bytes([types[0], types[1]])));
        }
        Ok(Self { entries })
    }

    /// `CASUnlockPartDB::GetUnlockFlags` (`0x82585b08`): the binary search
    /// over the entries; 0 without data or for an absent key.
    #[must_use]
    pub fn unlock_flags(&self, key: u64) -> u16 {
        let Ok(count) = i32::try_from(self.entries.len()) else { return 0 };
        let (mut low, mut high) = (0_i32, count - 1);
        while low <= high {
            let middle = (low + high) / 2;
            let Some(&(entry, types)) = usize::try_from(middle).ok().and_then(|m| self.entries.get(m)) else { return 0 };
            if key > entry {
                low = middle + 1;
            } else if key < entry {
                high = middle - 1;
            } else {
                return types;
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database(entries: &[(u64, u16)]) -> Vec<u8> {
        let start = 20 + 11;
        let size = start + 12 * entries.len();
        let mut data = Vec::new();
        for value in [VERSION, u32::try_from(size).unwrap(), 20, u32::try_from(entries.len()).unwrap(), u32::try_from(start).unwrap()] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        data.extend_from_slice(b"cas_db_000\0");
        for &(key, types) in entries {
            data.extend_from_slice(&key.to_be_bytes());
            data.extend_from_slice(&types.to_be_bytes());
            data.extend_from_slice(&[0, 0]);
        }
        data
    }

    #[test]
    fn finds_each_key_and_rejects_others() {
        let entries = [(3, 1), (9, 24), (0x8000_0000_0000_0001, 16), (u64::MAX, 31)];
        let db = CasUnlockPartDb::parse(&database(&entries)).unwrap();
        for (key, types) in entries {
            assert_eq!(db.unlock_flags(key), types);
        }
        assert_eq!(db.unlock_flags(4), 0);
        assert_eq!(CasUnlockPartDb::default().unlock_flags(3), 0);
        let mut wrong = database(&entries);
        wrong[3] = 2;
        assert!(CasUnlockPartDb::parse(&wrong).is_err());
    }
}
