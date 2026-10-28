//! `Attrib::RefSpec::GetCollection829766f0` over the SAME loaded Vault.
use super::{AttributeRuntimeDataOrigin, AttributeRuntimeError, AttributeRuntimeInstance};
use skate2_formats::attrib::AttributePointerLocation;
use std::{collections::BTreeMap, sync::Mutex};

/// Native `RefSpec` cache fields belong to the shared layout, not each Instance.
/// Keys identify already retained collections; storing an Arc back to the Vault
/// here would create an ownership cycle. The existing Vault retains collections
/// for this immutable loaded session. Native absolute refcounts are not exposed.
#[derive(Debug, Default)]
pub(super) struct RefSpecCache(Mutex<BTreeMap<AttributePointerLocation, Option<(u64, u64)>>>);

#[derive(Debug)]
pub(crate) enum RefSpecError {
    Runtime(AttributeRuntimeError),
    CacheLock,
    OffsetOverflow,
    InitialCachedPointer {
        location: AttributePointerLocation,
        word: u32,
    },
}
impl From<AttributeRuntimeError> for RefSpecError {
    fn from(value: AttributeRuntimeError) -> Self {
        Self::Runtime(value)
    }
}
impl std::fmt::Display for RefSpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(error) => write!(f, "RefSpec source: {error}"),
            Self::CacheLock => f.write_str("shared Vault RefSpec cache poisoned"),
            Self::OffsetOverflow => f.write_str("RefSpec layout offset overflow"),
            Self::InitialCachedPointer { location, word } => write!(
                f,
                "RefSpec authored cached pointer {word:08x} at {location:?} is not admitted"
            ),
        }
    }
}

impl AttributeRuntimeInstance {
    /// Generated caller owns its null-layout default. A missing layout has the
    /// zero `RefSpec` of that default; no fallback collection name is invented.
    pub(crate) fn try_cached_layout_reference(
        &self,
        offset: u32,
    ) -> Result<Option<Self>, RefSpecError> {
        let Some(layout) = self.cached_layout else {
            return Ok(None);
        };
        let Some(base) = self.vault.pointer(layout)? else {
            return Ok(None);
        };
        let location = AttributePointerLocation {
            dependency_index: base.dependency_index,
            offset: base
                .offset
                .checked_add(offset)
                .ok_or(RefSpecError::OffsetOverflow)?,
        };
        let read = |relative, size| {
            self.vault.range(
                location,
                relative,
                size,
                AttributeRuntimeDataOrigin::CollectionLayout,
            )
        };
        // Raw82976704 LD+8 and CMPLDI preserve the ENTIRE 64-bit key.
        let collection = u64::from_be_bytes(read(8, 8)?.bytes().try_into().expect("eight bytes"));
        if collection == 0 {
            return Ok(None);
        }
        let mut cache = self
            .vault
            .refspec_cache
            .0
            .lock()
            .map_err(|_| RefSpecError::CacheLock)?;
        if let Some(Some((class, key))) = cache.get(&location) {
            return Ok(Some(self.vault.instance(*class, *key)));
        }
        if !cache.contains_key(&location) {
            let word = u32::from_be_bytes(read(16, 4)?.bytes().try_into().expect("four bytes"));
            if word != 0 {
                return Err(RefSpecError::InitialCachedPointer { location, word });
            }
        }
        let class = u64::from_be_bytes(read(0, 8)?.bytes().try_into().expect("eight bytes"));
        if class == 0 || !self.vault.classes.contains_key(&class) {
            return Ok(None);
        }
        let key = (class, collection);
        let found = self.vault.collections.contains_key(&key);
        // Native publishes even a null collection result. Null is retried on a
        // later call; only a successful pointer bypasses class/collection lookup.
        cache.insert(location, found.then_some(key));
        Ok(found.then(|| self.vault.instance(class, collection)))
    }
}

#[cfg(test)]
#[path = "attribute_runtime_refspec_tests.rs"]
mod tests;
