//! Hash-only `StateGraph` and `BehaviourGraph` cache ownership.
//!
//! Semantic binary/XML decoding remains a supplied collaborator. These caches
//! own the recovered path, hit, flush, negative-entry, and replacement rules.

use skate2_formats::hash::sk8_hash_value_32;
use std::{collections::BTreeMap, error::Error, fmt, sync::Arc};

pub const GRAPH_CACHE_PATH_CAPACITY: usize = 255;
pub const STATE_GRAPH_CACHE_CAPACITY: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphCacheKey {
    normalized_path: String,
    hash: u32,
}

impl GraphCacheKey {
    #[must_use]
    pub fn normalized_path(&self) -> &str {
        &self.normalized_path
    }

    #[must_use]
    pub const fn hash(&self) -> u32 {
        self.hash
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphCachePathError {
    EmbeddedNul,
    MissingExtension,
    TooLong { bytes: usize, maximum: usize },
}

impl fmt::Display for GraphCachePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmbeddedNul => formatter.write_str("graph-cache path contains an embedded NUL"),
            Self::MissingExtension => formatter.write_str("graph-cache path has no extension"),
            Self::TooLong { bytes, maximum } => write!(
                formatter,
                "graph-cache path has {bytes} bytes; safe native buffer maximum is {maximum}"
            ),
        }
    }
}

impl Error for GraphCachePathError {}

#[derive(Debug)]
pub enum GraphCacheLoadError<E> {
    Path(GraphCachePathError),
    StateCapacity { maximum: usize },
    Backend(E),
}

impl<E: fmt::Display> fmt::Display for GraphCacheLoadError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(source) => source.fmt(formatter),
            Self::StateCapacity { maximum } => {
                write!(formatter, "StateGraph cache exhausted its {maximum} nodes")
            }
            Self::Backend(source) => write!(formatter, "graph decoder failed: {source}"),
        }
    }
}

impl<E: Error + 'static> Error for GraphCacheLoadError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Path(source) => Some(source),
            Self::Backend(source) => Some(source),
            Self::StateCapacity { .. } => None,
        }
    }
}

/// Reconstructs the cache-front path logic in `StateGraph::Cache::Load`
/// (`0x82cf8af0`) for NUL-free UTF-8 paths within the native buffer.
///
/// The `data\` test is a case-sensitive substring test performed before slash
/// conversion, not a prefix test.
///
/// # Errors
///
/// Rejects embedded NULs and normalized paths beyond the safe native buffer.
/// Native accepts arbitrary C-string bytes and has unsafe overflow behavior;
/// the UTF-8 input type and checked failures are host-safety boundaries.
pub fn state_graph_cache_key(filename: &str) -> Result<GraphCacheKey, GraphCachePathError> {
    let prefix = if filename.contains("data\\") {
        ""
    } else {
        "data\\"
    };
    checked_cache_key(&format!("{prefix}{filename}"))
}

/// Reconstructs the cache-front path logic in `BehaviourGraph::Cache::Load`
/// (`0x82cf6ca0`) for NUL-free UTF-8 paths within the native buffer. Behaviour
/// paths always receive another `data\` prefix.
///
/// # Errors
///
/// Rejects embedded NULs and normalized paths beyond the safe native buffer.
/// Native accepts arbitrary C-string bytes and has unsafe overflow behavior;
/// the UTF-8 input type and checked failures are host-safety boundaries.
pub fn behaviour_graph_cache_key(filename: &str) -> Result<GraphCacheKey, GraphCachePathError> {
    checked_cache_key(&format!("data\\{filename}"))
}

/// Reconstructs the compiled-sibling name made by both `ParseBinary`
/// implementations after their bounded 256-byte copy.
///
/// Native asserts on a missing final dot and then would copy through null if
/// execution resumed. Its extension copy is unbounded, so the safe host also
/// rejects an expanded sibling longer than 255 bytes.
///
/// # Errors
///
/// Rejects embedded NULs, a missing extension, or an expanded path beyond the
/// safe native buffer. The checked UTF-8 interface and errors replace native
/// C-string/assert/overflow behavior outside the valid path domain.
pub fn compiled_graph_path(normalized_path: &str) -> Result<String, GraphCachePathError> {
    if normalized_path.as_bytes().contains(&0) {
        return Err(GraphCachePathError::EmbeddedNul);
    }
    if normalized_path.len() > GRAPH_CACHE_PATH_CAPACITY {
        return Err(GraphCachePathError::TooLong {
            bytes: normalized_path.len(),
            maximum: GRAPH_CACHE_PATH_CAPACITY,
        });
    }
    let extension = normalized_path
        .rfind('.')
        .ok_or(GraphCachePathError::MissingExtension)?;
    let bytes = extension + ".stategraph".len();
    if bytes > GRAPH_CACHE_PATH_CAPACITY {
        return Err(GraphCachePathError::TooLong {
            bytes,
            maximum: GRAPH_CACHE_PATH_CAPACITY,
        });
    }
    let mut compiled = String::with_capacity(bytes);
    compiled.push_str(&normalized_path[..extension]);
    compiled.push_str(".stategraph");
    Ok(compiled.replace('/', "\\"))
}

fn checked_cache_key(path: &str) -> Result<GraphCacheKey, GraphCachePathError> {
    if path.as_bytes().contains(&0) {
        return Err(GraphCachePathError::EmbeddedNul);
    }
    let normalized_path = path.replace('/', "\\");
    let bytes = normalized_path.len();
    if bytes > GRAPH_CACHE_PATH_CAPACITY {
        return Err(GraphCachePathError::TooLong {
            bytes,
            maximum: GRAPH_CACHE_PATH_CAPACITY,
        });
    }
    let hash = sk8_hash_value_32(normalized_path.as_bytes());
    Ok(GraphCacheKey {
        normalized_path,
        hash,
    })
}

/// Readable owner for the native fixed 32-node `StateGraph` map.
#[derive(Debug, Default)]
pub struct StateGraphCache<T> {
    entries: BTreeMap<u32, Option<T>>,
}

impl<T> StateGraphCache<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Reconstructs `StateGraph::Cache::Flush` (`0x82cf3060`) by releasing
    /// cache ownership from the greatest `u32` key to the least.
    pub fn flush(&mut self) {
        while let Some(entry) = self.entries.pop_last() {
            drop(entry);
        }
    }

    /// Loads or returns one hash-keyed `StateGraph` entry.
    ///
    /// A present null entry is a cache hit. A true flush decodes and replaces
    /// it. For a 33rd unique hash, decoding occurs before the safe-host capacity
    /// fault because native parsing precedes its invalid fixed-pool insertion.
    ///
    /// # Errors
    ///
    /// Returns a checked path, backend, or fixed-capacity failure.
    pub fn load_with<E, F>(
        &mut self,
        filename: &str,
        flush_cache: bool,
        decode: F,
    ) -> Result<Option<&T>, GraphCacheLoadError<E>>
    where
        F: FnOnce(&str) -> Result<Option<T>, E>,
    {
        let key = state_graph_cache_key(filename).map_err(GraphCacheLoadError::Path)?;
        if !flush_cache && self.entries.contains_key(&key.hash) {
            return Ok(self.entries.get(&key.hash).and_then(Option::as_ref));
        }
        let decoded = decode(&key.normalized_path).map_err(GraphCacheLoadError::Backend)?;
        if !self.entries.contains_key(&key.hash) && self.entries.len() == STATE_GRAPH_CACHE_CAPACITY
        {
            drop(decoded);
            return Err(GraphCacheLoadError::StateCapacity {
                maximum: STATE_GRAPH_CACHE_CAPACITY,
            });
        }
        let previous = self.entries.insert(key.hash, decoded);
        drop(previous);
        Ok(self.entries.get(&key.hash).and_then(Option::as_ref))
    }
}

impl<T> Drop for StateGraphCache<T> {
    fn drop(&mut self) {
        self.flush();
    }
}

/// Readable owner for the `BehaviourGraph` cache's hash-only identity rules.
/// The native container is dynamically allocated and has no fixed node cap.
/// Native bucket-order destruction remains outside this cache-front slice.
#[derive(Debug, Default)]
pub struct BehaviourGraphCache<T> {
    entries: BTreeMap<u32, Option<Arc<T>>>,
}

impl<T> BehaviourGraphCache<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Loads or returns one hash-keyed transferable behaviour. The decoder
    /// closure owns any successful `Process(shared StateGraph cache)` effect
    /// before this cache stores the object.
    ///
    /// # Errors
    ///
    /// Returns a checked path or backend failure.
    pub fn load_with<E, F>(
        &mut self,
        filename: &str,
        flush_cache: bool,
        decode_and_process: F,
    ) -> Result<Option<Arc<T>>, GraphCacheLoadError<E>>
    where
        F: FnOnce(&str) -> Result<Option<T>, E>,
    {
        let key = behaviour_graph_cache_key(filename).map_err(GraphCacheLoadError::Path)?;
        if !flush_cache && self.entries.contains_key(&key.hash) {
            return Ok(self.entries.get(&key.hash).and_then(Clone::clone));
        }
        let decoded = decode_and_process(&key.normalized_path)
            .map_err(GraphCacheLoadError::Backend)?
            .map(Arc::new);
        let previous = self.entries.insert(key.hash, decoded.clone());
        drop(previous);
        Ok(decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, convert::Infallible, rc::Rc};

    #[derive(Debug)]
    struct DropRecord {
        value: u32,
        drops: Rc<RefCell<Vec<u32>>>,
    }

    impl Drop for DropRecord {
        fn drop(&mut self) {
            self.drops.borrow_mut().push(self.value);
        }
    }

    #[test]
    fn state_path_uses_preconversion_substring_while_behaviour_always_prefixes() {
        assert_eq!(
            state_graph_cache_key("folder/data\\state/one.xml")
                .unwrap()
                .normalized_path(),
            "folder\\data\\state\\one.xml"
        );
        assert_eq!(
            state_graph_cache_key("data/state/one.xml")
                .unwrap()
                .normalized_path(),
            "data\\data\\state\\one.xml"
        );
        assert_eq!(
            behaviour_graph_cache_key("data\\state/one.xml")
                .unwrap()
                .normalized_path(),
            "data\\data\\state\\one.xml"
        );
        assert_eq!(
            state_graph_cache_key("DATA\\state//one.xml")
                .unwrap()
                .normalized_path(),
            "data\\DATA\\state\\\\one.xml"
        );
    }

    #[test]
    fn safe_path_boundary_accepts_255_and_rejects_overflow_and_nul() {
        let exact = format!("data\\{}", "a".repeat(250));
        assert_eq!(exact.len(), GRAPH_CACHE_PATH_CAPACITY);
        assert!(state_graph_cache_key(&exact).is_ok());
        assert_eq!(
            state_graph_cache_key(&(exact + "a")),
            Err(GraphCachePathError::TooLong {
                bytes: 256,
                maximum: GRAPH_CACHE_PATH_CAPACITY,
            })
        );
        assert_eq!(
            state_graph_cache_key("state/a\0b.xml"),
            Err(GraphCachePathError::EmbeddedNul)
        );
    }

    #[test]
    fn compiled_sibling_replaces_last_extension_and_checks_expansion() {
        assert_eq!(
            compiled_graph_path("data\\dir.with.dot\\state.xml").unwrap(),
            "data\\dir.with.dot\\state.stategraph"
        );
        assert_eq!(
            compiled_graph_path("data/state.xml").unwrap(),
            "data\\state.stategraph"
        );
        assert_eq!(
            compiled_graph_path("data\\state"),
            Err(GraphCachePathError::MissingExtension)
        );
        assert_eq!(
            compiled_graph_path("data\\dir.with.dot\\state").unwrap(),
            "data\\dir.with.stategraph"
        );
        let expanding = format!("{}.x", "a".repeat(246));
        assert_eq!(expanding.len(), 248);
        assert_eq!(
            compiled_graph_path(&expanding),
            Err(GraphCachePathError::TooLong {
                bytes: 257,
                maximum: GRAPH_CACHE_PATH_CAPACITY,
            })
        );
        let overlong_with_early_dot = format!("a.{}", "b".repeat(254));
        assert_eq!(overlong_with_early_dot.len(), 256);
        assert_eq!(
            compiled_graph_path(&overlong_with_early_dot),
            Err(GraphCachePathError::TooLong {
                bytes: 256,
                maximum: GRAPH_CACHE_PATH_CAPACITY,
            })
        );
    }

    #[test]
    fn state_cache_negative_hits_and_flush_replacements_are_exact() {
        let mut cache = StateGraphCache::<u32>::new();
        let mut calls = Vec::new();
        assert_eq!(
            cache
                .load_with("state/a.xml", false, |path| {
                    calls.push(path.to_owned());
                    Ok::<_, Infallible>(None)
                })
                .unwrap(),
            None
        );
        assert_eq!(
            cache
                .load_with(
                    "state/a.xml",
                    false,
                    |_| -> Result<Option<u32>, Infallible> { panic!("negative hit decoded") }
                )
                .unwrap(),
            None
        );
        assert_eq!(
            cache
                .load_with("state/a.xml", true, |path| {
                    calls.push(path.to_owned());
                    Ok::<_, Infallible>(Some(7))
                })
                .unwrap(),
            Some(&7)
        );
        assert_eq!(calls, ["data\\state\\a.xml", "data\\state\\a.xml"]);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn state_cache_aliases_distinct_paths_with_the_same_native_hash() {
        let first = "data\\state\\collision_2305.xml";
        let second = "data\\state\\collision_60100.xml";
        assert_eq!(
            state_graph_cache_key(first).unwrap().hash(),
            state_graph_cache_key(second).unwrap().hash()
        );
        let mut cache = StateGraphCache::new();
        cache
            .load_with(first, false, |_| Ok::<_, Infallible>(Some(23)))
            .unwrap();
        assert_eq!(
            cache
                .load_with(second, false, |_| -> Result<Option<i32>, Infallible> {
                    panic!("hash alias decoded")
                })
                .unwrap(),
            Some(&23)
        );
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn state_capacity_fault_occurs_after_decoding_the_thirty_third_key() {
        let mut cache = StateGraphCache::new();
        for index in 0..STATE_GRAPH_CACHE_CAPACITY {
            cache
                .load_with(&format!("state/{index}.xml"), false, |_| {
                    Ok::<_, Infallible>(Some(index))
                })
                .unwrap();
        }
        let mut decoded = false;
        let result = cache.load_with("state/overflow.xml", false, |_| {
            decoded = true;
            Ok::<_, Infallible>(Some(99))
        });
        assert!(decoded);
        assert!(matches!(
            result,
            Err(GraphCacheLoadError::StateCapacity { maximum: 32 })
        ));
        assert_eq!(cache.len(), STATE_GRAPH_CACHE_CAPACITY);
    }

    #[test]
    fn state_flush_releases_entries_in_descending_hash_order() {
        let drops = Rc::new(RefCell::new(Vec::new()));
        let mut cache = StateGraphCache::new();
        let paths = ["state/a.xml", "state/b.xml", "state/c.xml"];
        for (value, path) in paths.into_iter().enumerate() {
            cache
                .load_with(path, false, |_| {
                    Ok::<_, Infallible>(Some(DropRecord {
                        value: u32::try_from(value).unwrap(),
                        drops: Rc::clone(&drops),
                    }))
                })
                .unwrap();
        }
        let mut expected = paths
            .into_iter()
            .enumerate()
            .map(|(value, path)| {
                (
                    state_graph_cache_key(path).unwrap().hash(),
                    u32::try_from(value).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        expected.sort_unstable_by_key(|(hash, _)| std::cmp::Reverse(*hash));

        cache.flush();

        assert_eq!(
            *drops.borrow(),
            expected
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>()
        );
        assert!(cache.is_empty());
    }

    #[test]
    fn state_flush_replaces_an_existing_key_even_at_capacity() {
        let mut cache = StateGraphCache::new();
        for index in 0..STATE_GRAPH_CACHE_CAPACITY {
            cache
                .load_with(&format!("state/{index}.xml"), false, |_| {
                    Ok::<_, Infallible>(Some(index))
                })
                .unwrap();
        }
        assert_eq!(
            cache
                .load_with("state/0.xml", true, |_| Ok::<_, Infallible>(Some(99)))
                .unwrap(),
            Some(&99)
        );
        assert_eq!(cache.len(), STATE_GRAPH_CACHE_CAPACITY);
    }

    #[test]
    fn state_negative_entries_consume_capacity_and_flush_restores_it() {
        let mut cache = StateGraphCache::<u32>::new();
        for index in 0..STATE_GRAPH_CACHE_CAPACITY {
            cache
                .load_with(&format!("state/{index}.xml"), false, |_| {
                    Ok::<_, Infallible>(None)
                })
                .unwrap();
        }
        let mut decoded = false;
        assert!(matches!(
            cache.load_with("state/overflow.xml", false, |_| {
                decoded = true;
                Ok::<_, Infallible>(None)
            }),
            Err(GraphCacheLoadError::StateCapacity { maximum: 32 })
        ));
        assert!(decoded);
        cache.flush();
        assert!(cache.is_empty());
        cache
            .load_with("state/reused.xml", false, |_| Ok::<_, Infallible>(Some(7)))
            .unwrap();
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn state_replacement_decodes_and_stores_before_releasing_old_value() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut cache = StateGraphCache::new();
        cache
            .load_with("state/a.xml", false, |_| {
                Ok::<_, Infallible>(Some(DropRecord {
                    value: 1,
                    drops: Rc::clone(&events),
                }))
            })
            .unwrap();
        cache
            .load_with("state/a.xml", true, |_| {
                events.borrow_mut().push(2);
                Ok::<_, Infallible>(Some(DropRecord {
                    value: 3,
                    drops: Rc::clone(&events),
                }))
            })
            .unwrap();
        assert_eq!(*events.borrow(), [2, 1]);
        drop(cache);
        assert_eq!(*events.borrow(), [2, 1, 3]);
    }

    #[test]
    fn behaviour_cache_processes_before_store_and_reuses_positive_hits() {
        let mut cache = BehaviourGraphCache::new();
        let mut paths = Vec::new();
        assert_eq!(
            cache
                .load_with("livingworld/a.xml", false, |path| {
                    paths.push(path.to_owned());
                    Ok::<_, Infallible>(Some(11))
                })
                .unwrap(),
            Some(Arc::new(11))
        );
        assert_eq!(paths, ["data\\livingworld\\a.xml"]);
        assert_eq!(
            cache
                .load_with(
                    "livingworld/a.xml",
                    false,
                    |_| -> Result<Option<i32>, Infallible> { panic!("hit decoded") },
                )
                .unwrap(),
            Some(Arc::new(11))
        );
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn behaviour_negative_entries_hit_and_true_flush_retries() {
        let mut cache = BehaviourGraphCache::<u32>::new();
        let mut calls = 0;
        assert_eq!(
            cache
                .load_with("livingworld/missing.xml", false, |_| {
                    calls += 1;
                    Ok::<_, Infallible>(None)
                })
                .unwrap(),
            None
        );
        assert_eq!(
            cache
                .load_with(
                    "livingworld/missing.xml",
                    false,
                    |_| -> Result<Option<u32>, Infallible> { panic!("negative hit decoded") },
                )
                .unwrap(),
            None
        );
        assert_eq!(
            cache
                .load_with("livingworld/missing.xml", true, |_| {
                    calls += 1;
                    Ok::<_, Infallible>(Some(5))
                })
                .unwrap(),
            Some(Arc::new(5))
        );
        assert_eq!(calls, 2);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn behaviour_cache_uses_hash_only_identity_for_positive_and_negative_hits() {
        let first = "collision_7434.xml";
        let second = "collision_240102.xml";
        assert_eq!(
            behaviour_graph_cache_key(first).unwrap().hash(),
            behaviour_graph_cache_key(second).unwrap().hash()
        );
        let mut positive = BehaviourGraphCache::new();
        positive
            .load_with(first, false, |_| Ok::<_, Infallible>(Some(23)))
            .unwrap();
        assert_eq!(
            positive
                .load_with(second, false, |_| -> Result<Option<i32>, Infallible> {
                    panic!("positive hash alias decoded")
                })
                .unwrap(),
            Some(Arc::new(23))
        );
        let mut negative = BehaviourGraphCache::<i32>::new();
        negative
            .load_with(first, false, |_| Ok::<_, Infallible>(None))
            .unwrap();
        assert_eq!(
            negative
                .load_with(second, false, |_| -> Result<Option<i32>, Infallible> {
                    panic!("negative hash alias decoded")
                })
                .unwrap(),
            None
        );
    }

    #[test]
    fn backend_failures_leave_cache_entries_unchanged() {
        let mut state = StateGraphCache::new();
        state
            .load_with("state/a.xml", false, |_| Ok::<_, &'static str>(Some(1)))
            .unwrap();
        assert!(matches!(
            state.load_with("state/a.xml", true, |_| Err::<Option<i32>, _>("state fail")),
            Err(GraphCacheLoadError::Backend("state fail"))
        ));
        assert_eq!(
            state
                .load_with(
                    "state/a.xml",
                    false,
                    |_| -> Result<Option<i32>, &'static str> { panic!("state entry was changed") }
                )
                .unwrap(),
            Some(&1)
        );

        let mut behaviour = BehaviourGraphCache::new();
        behaviour
            .load_with("livingworld/a.xml", false, |_| {
                Ok::<_, &'static str>(Some(2))
            })
            .unwrap();
        assert!(matches!(
            behaviour.load_with("livingworld/a.xml", true, |_| {
                Err::<Option<i32>, _>("behaviour fail")
            }),
            Err(GraphCacheLoadError::Backend("behaviour fail"))
        ));
        assert_eq!(
            behaviour
                .load_with(
                    "livingworld/a.xml",
                    false,
                    |_| -> Result<Option<i32>, &'static str> {
                        panic!("behaviour entry was changed")
                    },
                )
                .unwrap(),
            Some(Arc::new(2))
        );
    }

    #[test]
    fn behaviour_returns_owning_handles_that_survive_replacement_and_cache_drop() {
        let drops = Rc::new(RefCell::new(Vec::new()));
        let mut cache = BehaviourGraphCache::new();
        let first = cache
            .load_with("livingworld/a.xml", false, |_| {
                Ok::<_, Infallible>(Some(DropRecord {
                    value: 1,
                    drops: Rc::clone(&drops),
                }))
            })
            .unwrap()
            .unwrap();
        let second = cache
            .load_with("livingworld/a.xml", true, |_| {
                Ok::<_, Infallible>(Some(DropRecord {
                    value: 2,
                    drops: Rc::clone(&drops),
                }))
            })
            .unwrap()
            .unwrap();
        drop(cache);
        assert!(drops.borrow().is_empty());
        drop(second);
        assert_eq!(*drops.borrow(), [2]);
        drop(first);
        assert_eq!(*drops.borrow(), [2, 1]);
    }
}
