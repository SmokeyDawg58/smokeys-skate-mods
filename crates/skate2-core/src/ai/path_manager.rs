//! `AIPath::PathManager`: the path database the world arenas fill (registry kind 27) and its
//! queries. A challenge's paths come from one static package (`LoadStaticChallengePaths`),
//! `data\content\aipaths\AIPATH_Challenge_<id>.rx2` in `aipaths.big`.
//!
//! `mPathMap` is an `xfixed_hash_map<AIPathID, AIPathAlias, 512, 256>`: 251 buckets
//! (`GetPrevBucketCountOnly(256)`), a maximum load factor of 10000 so it never rehashes, new
//! nodes at the head of their bucket and iteration bucket by bucket. The queries walk it in
//! that order and stop at their result cap, so the order is kept.
use super::path_id::{self, PathFilter};
use crate::{
    selected_simulation_asset_services::SelectedSimulationAssetServices,
    simulation_normal_arena_plan::AI_PATH_DATA_TYPE_ID,
};
use skate2_formats::{
    ai_path::{AiPath, AiPathData, AiPathError, AiPathId},
    hash::rw_hash64_string,
    rw4::{Arena, ArenaError},
};
use std::sync::Arc;

const BUCKET_COUNT: usize = 251;

/// One loaded `tAIPathData`: the registry occurrence it came from (none for a challenge
/// package) and its paths.
pub(crate) struct LoadedPaths {
    asset_id: u64,
    occurrence: Option<Arc<crate::simulation_inner_registry_asset::SimulationInnerRegistryAsset>>,
    data: AiPathData,
}

impl std::fmt::Debug for LoadedPaths {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedPaths")
            .field("asset_id", &self.asset_id)
            .field("paths", &self.data.paths.len())
            .finish_non_exhaustive()
    }
}

/// A `Path*`: the loaded data and the path's index in it.
#[derive(Clone, Debug)]
pub(crate) struct PathRef {
    loaded: Arc<LoadedPaths>,
    index: usize,
}

impl PathRef {
    pub(crate) fn path(&self) -> &AiPath {
        &self.loaded.data.paths[self.index]
    }

    /// Pointer identity.
    pub(crate) fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.loaded, &other.loaded) && self.index == other.index
    }
}

/// `eastl::hash_node<pair<AIPathID const, AIPathAlias>>`; `AIPathAlias::mPaths` keeps the
/// paths sharing one ID in insertion order and `GetAIPath` answers the first.
#[derive(Debug)]
struct PathMapNode {
    id: AiPathId,
    paths: Vec<PathRef>,
}

/// `EPathQueryType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PathQueryType {
    Start,
    StartAllowFallback,
    Any,
}

/// `AIPath::PathQueryResult`.
#[derive(Clone, Debug)]
pub(crate) struct PathQueryResult {
    pub(crate) path: PathRef,
    pub(crate) node_index: i32,
    pub(crate) dist_sqr: f32,
}

/// What `Path::IsUsableBySkater` 0x824c1d10 reads from a `SkaterProfile`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PathSkaterAbilities {
    /// `ai_skater_profiles` 0xa390be5fc0dda71c: the skater's bit in `m_AllowedSkaters`.
    pub(crate) allowed_skater_flag: i32,
    pub(crate) can_skate_race: bool,
    pub(crate) can_skate_vert: bool,
    pub(crate) can_skate_street: bool,
}

#[derive(Debug)]
pub(crate) enum PathManagerError {
    Registry(crate::selected_simulation_asset_services::SelectedAssetError),
    Source(crate::simulation_inner_registry_asset::InnerAssetError),
    Data(AiPathError),
    /// `AIPathAlias::RemoveAIPath` asserted: the path was not in its alias.
    MissingAlias,
    /// A challenge path package that is no arena, or holds no `tAIPathData`.
    Package(Option<ArenaError>),
    /// `Path::GetNearestNodeIndex` asserted on a path with fewer than two nodes.
    NoNearestNode,
}

impl std::fmt::Display for PathManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Registry(e) => write!(f, "AI path registry: {e:?}"),
            Self::Source(e) => write!(f, "AI path asset: {e:?}"),
            Self::Data(e) => write!(f, "AI path data: {e}"),
            Self::MissingAlias => f.write_str("AI path removed from an alias it was not in"),
            Self::Package(e) => write!(f, "AI path package: {e:?}"),
            Self::NoNearestNode => f.write_str("AI path has no nearest node"),
        }
    }
}

impl std::error::Error for PathManagerError {}

/// `Sk8::AIPath::PathManager`.
#[derive(Debug)]
pub(crate) struct PathManager {
    buckets: Vec<Vec<PathMapNode>>,
    path_count: i32,
    loaded: Vec<Arc<LoadedPaths>>,
    universal_filter_id: AiPathId,
    ambient_filter: PathFilter,
    /// `mStaticPathsAssetID` and `mBinaryLoadCount`.
    static_asset: Option<u64>,
    binary_load_count: i32,
}

impl Default for PathManager {
    fn default() -> Self {
        Self::new()
    }
}

/// `vmsum3fp128` of the difference: the host dot3 policy.
pub(crate) fn distance_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    crate::reckoning_numeric::host_dot3(
        [a[0] - b[0], a[1] - b[1], a[2] - b[2], 0.0],
        [a[0] - b[0], a[1] - b[1], a[2] - b[2], 0.0],
    )
}

impl PathManager {
    /// `PathManager::PathManager` 0x824d3650.
    pub(crate) fn new() -> Self {
        Self {
            buckets: (0..BUCKET_COUNT).map(|_| Vec::new()).collect(),
            path_count: 0,
            loaded: Vec::new(),
            universal_filter_id: path_id::universal_filter_id(),
            ambient_filter: PathFilter::Single(path_id::ambient_filter_id()),
            static_asset: None,
            binary_load_count: 0,
        }
    }

    /// `mAmbientFilter`, the `SingleFilter` ambient skaters wander with.
    pub(crate) fn ambient_filter(&self) -> &PathFilter {
        &self.ambient_filter
    }

    /// `mUniversalFilterPathID`, which a fresh `SingleFilter` copies.
    pub(crate) fn universal_filter_id(&self) -> AiPathId {
        self.universal_filter_id
    }

    pub(crate) fn path_count(&self) -> i32 {
        self.path_count
    }

    fn bucket(id: &AiPathId) -> usize {
        path_id::hash(id) as usize % BUCKET_COUNT
    }

    fn nodes(&self) -> impl Iterator<Item = &PathMapNode> {
        self.buckets.iter().flatten()
    }

    /// `AddAsset` 0x824d4890 / `DoAddAsset` 0x824d4458: the registry's data for `asset_id`,
    /// each path into the database.
    pub(crate) fn try_add_asset(
        &mut self,
        asset_id: u64,
        registry: &SelectedSimulationAssetServices,
    ) -> Result<(), PathManagerError> {
        let Some(occurrence) = registry.try_inner(asset_id).map_err(PathManagerError::Registry)? else {
            return Ok(());
        };
        let data = AiPathData::parse(occurrence.bytes().map_err(PathManagerError::Source)?)
            .map_err(PathManagerError::Data)?;
        let loaded = Arc::new(LoadedPaths { asset_id, occurrence: Some(occurrence), data });
        for index in 0..loaded.data.paths.len() {
            self.add_path_to_database(PathRef { loaded: Arc::clone(&loaded), index });
        }
        self.loaded.push(loaded);
        Ok(())
    }

    /// `RemoveAsset` 0x824d0f78 / `DoRemoveAsset` 0x824d08d0.
    pub(crate) fn try_remove_asset(
        &mut self,
        asset_id: u64,
        registry: &SelectedSimulationAssetServices,
    ) -> Result<(), PathManagerError> {
        let Some(occurrence) = registry.try_inner(asset_id).map_err(PathManagerError::Registry)? else {
            return Ok(());
        };
        let Some(at) = self
            .loaded
            .iter()
            .position(|loaded| {
                loaded.asset_id == asset_id
                    && loaded.occurrence.as_ref().is_some_and(|held| Arc::ptr_eq(held, &occurrence))
            })
        else {
            return Ok(());
        };
        let loaded = self.loaded.remove(at);
        for index in 0..loaded.data.paths.len() {
            self.remove_path_from_database(&PathRef { loaded: Arc::clone(&loaded), index })?;
        }
        Ok(())
    }

    /// `LoadStaticChallengePaths` 0x824d48f8 with `kbStaticPathBinaryLoading` (1): the held
    /// package goes, then `LoadBinaryPathPackage` 0x824b8f28 requests
    /// `data\content\aipaths\AIPATH_Challenge_<GetFriendlyString(id, true)>.rx2`, which becomes
    /// `mStaticPathsAssetID` with one more load in flight. Returns the file the loader answers
    /// for.
    pub(crate) fn load_static_challenge_paths(&mut self, id: &AiPathId) -> Result<String, PathManagerError> {
        self.unload_static_paths()?;
        let file = format!("data\\content\\aipaths\\AIPATH_Challenge_{}.rx2", path_id::friendly_string(id, true));
        self.static_asset = Some(rw_hash64_string(&file));
        self.binary_load_count += 1;
        Ok(file)
    }

    /// The package's arena arriving: its `tAIPathData`'s paths join the database (`DoAddAsset`
    /// 0x824d4458), then `AssetAdded` 0x824b1d60 counts the load done. A package unloaded while
    /// loading only counts.
    pub(crate) fn try_add_binary_path_package(&mut self, file: &str, bytes: &[u8]) -> Result<(), PathManagerError> {
        self.binary_load_count -= 1;
        let asset_id = rw_hash64_string(file);
        if self.static_asset != Some(asset_id) {
            return Ok(());
        }
        let resources = Arena::split_file_resources(bytes).map_err(|e| PathManagerError::Package(Some(e)))?;
        let arena = Arena::parse(&resources).map_err(|e| PathManagerError::Package(Some(e)))?;
        let object = arena
            .dictionary
            .iter()
            .find(|entry| entry.type_id == AI_PATH_DATA_TYPE_ID)
            .and_then(|entry| arena.object_bytes(&resources, entry))
            .ok_or(PathManagerError::Package(None))?;
        let data = AiPathData::parse(object).map_err(PathManagerError::Data)?;
        let loaded = Arc::new(LoadedPaths { asset_id, occurrence: None, data });
        for index in 0..loaded.data.paths.len() {
            self.add_path_to_database(PathRef { loaded: Arc::clone(&loaded), index });
        }
        self.loaded.push(loaded);
        Ok(())
    }

    /// `AssetMissing` 0x824b1dc0: the package's arena never arrives.
    pub(crate) fn binary_path_package_missing(&mut self) {
        self.binary_load_count -= 1;
        self.static_asset = None;
    }

    /// `UnloadStaticPaths` 0x824d0e40 with `kbStaticPathBinaryLoading` (1): the package's arena
    /// goes, taking its paths out of the database.
    pub(crate) fn unload_static_paths(&mut self) -> Result<(), PathManagerError> {
        let Some(asset_id) = self.static_asset.take() else { return Ok(()) };
        let Some(at) = self.loaded.iter().position(|loaded| loaded.asset_id == asset_id && loaded.occurrence.is_none()) else {
            return Ok(());
        };
        let loaded = self.loaded.remove(at);
        for index in 0..loaded.data.paths.len() {
            self.remove_path_from_database(&PathRef { loaded: Arc::clone(&loaded), index })?;
        }
        Ok(())
    }

    /// `ChallengeAIManager::AreStaticAIPathsLoaded` 0x825b2c50: no package load in flight.
    pub(crate) fn are_static_paths_loaded(&self) -> bool {
        self.binary_load_count == 0
    }

    /// `AddPathToDatabase` 0x824d4370: a new ID inserts an alias at its bucket's head and
    /// counts it; a known ID appends the path to its alias.
    fn add_path_to_database(&mut self, path: PathRef) {
        let id = path.path().id;
        let bucket = &mut self.buckets[Self::bucket(&id)];
        if let Some(node) = bucket.iter_mut().find(|node| node.id == id) {
            node.paths.push(path);
        } else {
            bucket.insert(0, PathMapNode { id, paths: vec![path] });
            self.path_count += 1;
        }
    }

    /// `RemovePathFromDatabase` 0x824d0798: drop the path from its alias, erasing the alias
    /// once it is empty.
    fn remove_path_from_database(&mut self, path: &PathRef) -> Result<(), PathManagerError> {
        let id = path.path().id;
        let bucket = &mut self.buckets[Self::bucket(&id)];
        let Some(at) = bucket.iter().position(|node| node.id == id) else {
            return Ok(());
        };
        let node = &mut bucket[at];
        let Some(entry) = node.paths.iter().position(|candidate| candidate.same(path)) else {
            return Err(PathManagerError::MissingAlias);
        };
        node.paths.remove(entry);
        if node.paths.is_empty() {
            bucket.remove(at);
            self.path_count -= 1;
        }
        Ok(())
    }

    /// The map's aliases in bucket order, each by its first path (`AIPathAlias::mPaths[0]`).
    pub(crate) fn first_paths(&self) -> impl Iterator<Item = &PathRef> {
        self.nodes().filter_map(|node| node.paths.first())
    }

    /// `GetAIPath` 0x824c7ec0.
    pub(crate) fn get_ai_path(&self, id: &AiPathId) -> Option<PathRef> {
        self.buckets[Self::bucket(id)]
            .iter()
            .find(|node| node.id == *id)
            .and_then(|node| node.paths.first().cloned())
    }

    /// `ShouldSkipPath` 0x824c26e0: outside the filter, locked, or unusable by the skater.
    pub(crate) fn should_skip_path(
        path: &AiPath,
        filter: Option<&PathFilter>,
        skater: Option<PathSkaterAbilities>,
        challenge_round: Option<i32>,
        locked: &mut LockedPathTest<'_>,
    ) -> bool {
        filter.is_some_and(|filter| !filter.includes(&path.id))
            || locked.is_path_locked(path)
            || !is_usable_by_skater(path, skater, challenge_round)
    }

    /// `GetPathsInRadius` 0x824ca710: usable paths whose query node lies within `radius`, up to
    /// `max_results`. With `StartAllowFallback` and none found, the nearest usable path within
    /// 6 m or nearer than every unusable one, else the nearest unusable one.
    #[expect(clippy::too_many_arguments, reason = "native query signature")]
    pub(crate) fn try_get_paths_in_radius(
        &self,
        pos: [f32; 3],
        radius: f32,
        max_results: usize,
        query: PathQueryType,
        filter: Option<&PathFilter>,
        skater: Option<PathSkaterAbilities>,
        challenge_round: Option<i32>,
        locked: &mut LockedPathTest<'_>,
    ) -> Result<Vec<PathQueryResult>, PathManagerError> {
        let radius_sqr = radius * radius;
        let start_only = matches!(query, PathQueryType::Start | PathQueryType::StartAllowFallback);
        let fallback = query == PathQueryType::StartAllowFallback;
        let mut results = Vec::new();
        let mut best_usable: Option<PathQueryResult> = None;
        let mut best_unusable: Option<PathQueryResult> = None;
        for node in self.nodes() {
            let path_ref = &node.paths[0];
            let path = path_ref.path();
            if filter.is_some_and(|filter| !filter.includes(&path.id)) || locked.is_path_locked(path) {
                continue;
            }
            let usable = is_usable_by_skater(path, skater, challenge_round);
            let node_index = if start_only { 0 } else { nearest_node_index(path, pos)? };
            let dist_sqr = distance_squared(path.nodes[node_index as usize].position, pos);
            if usable && dist_sqr <= radius_sqr {
                results.push(PathQueryResult { path: path_ref.clone(), node_index, dist_sqr });
                if results.len() == max_results {
                    break;
                }
            } else if fallback && results.is_empty() {
                let best = if usable { &mut best_usable } else { &mut best_unusable };
                if best.as_ref().is_none_or(|best| dist_sqr < best.dist_sqr) {
                    *best = Some(PathQueryResult { path: path_ref.clone(), node_index, dist_sqr });
                }
            }
        }
        if fallback && results.is_empty() {
            match (best_usable, best_unusable) {
                (Some(usable), unusable)
                    if unusable.as_ref().is_none_or(|unusable| usable.dist_sqr < unusable.dist_sqr)
                        || usable.dist_sqr < 36.0 =>
                {
                    results.push(usable);
                }
                (_, Some(unusable)) => results.push(unusable),
                _ => {}
            }
        }
        Ok(results)
    }

    /// `GetPathsInDistanceRange` 0x824cae20: paths whose query node (node 0 for the start
    /// types, else the nearest) lies between `min` and `max` from `pos`.
    #[expect(clippy::too_many_arguments, reason = "native query signature")]
    pub(crate) fn try_get_paths_in_distance_range(
        &self,
        pos: [f32; 3],
        min: f32,
        max: f32,
        max_results: usize,
        query: PathQueryType,
        filter: Option<&PathFilter>,
        skater: Option<PathSkaterAbilities>,
        challenge_round: Option<i32>,
        locked: &mut LockedPathTest<'_>,
    ) -> Result<Vec<PathQueryResult>, PathManagerError> {
        let min_sqr = min * min;
        let max_sqr = max * max;
        let mut results = Vec::new();
        for node in self.nodes() {
            let path_ref = &node.paths[0];
            let path = path_ref.path();
            if Self::should_skip_path(path, filter, skater, challenge_round, locked) {
                continue;
            }
            let node_index = if matches!(query, PathQueryType::Start | PathQueryType::StartAllowFallback) {
                0
            } else {
                nearest_node_index(path, pos)?
            };
            let node = &path.nodes[node_index as usize];
            let dist_sqr = distance_squared(node.position, pos);
            if dist_sqr <= max_sqr && dist_sqr >= min_sqr {
                results.push(PathQueryResult { path: path_ref.clone(), node_index, dist_sqr });
                if results.len() == max_results {
                    break;
                }
            }
        }
        Ok(results)
    }
}

/// `Path::GetNearestNodeIndex` 0x824b70f8: the nearest node from index 1 on; node 0 never
/// competes.
pub(crate) fn nearest_node_index(path: &AiPath, pos: [f32; 3]) -> Result<i32, PathManagerError> {
    let mut best = None;
    let mut best_sqr = f32::MAX;
    for (index, node) in path.nodes.iter().enumerate().skip(1) {
        let dist_sqr = distance_squared(node.position, pos);
        if dist_sqr < best_sqr {
            best = Some(index as i32);
            best_sqr = dist_sqr;
        }
    }
    best.ok_or(PathManagerError::NoNearestNode)
}

/// `Path::IsUsableBySkater` 0x824c1d10 with the default `kbEnableSkaterFlagPathLocking` 1 and
/// `kbEnableRVSFlagPathLocking` 0: the skater's allowed bit, then the challenge round's
/// exclusion bit `round + 9`.
pub(crate) fn is_usable_by_skater(
    path: &AiPath,
    skater: Option<PathSkaterAbilities>,
    challenge_round: Option<i32>,
) -> bool {
    if let Some(skater) = skater {
        let bit = skater.allowed_skater_flag;
        if !(0..64).contains(&bit) || path.allowed_skaters & (1 << bit) == 0 {
            return false;
        }
    }
    match challenge_round {
        None | Some(-1) => true,
        Some(round) => path.bit_flags & (1u32.wrapping_shl((round + 9) as u32)) == 0,
    }
}

/// `PathManager::IsPathLocked` 0x824c25f8 and its `TriggerVolumeGroupData`: with
/// `kbEnablePathLockTest` and path flag 0x200, a path is locked when its box overlaps a
/// locked-out challenge's trigger volume within 40 m of the query point. The volumes are
/// gathered once per query (`ChallengeAIManager::GetLockedOutChallenges` 0x825c12d8).
pub(crate) struct LockedPathTest<'a> {
    pub(crate) center: [f32; 3],
    pub(crate) gather: &'a mut dyn FnMut([f32; 3], f32) -> Vec<([f32; 4], [f32; 4])>,
    gathered: Option<Vec<([f32; 4], [f32; 4])>>,
}

impl<'a> LockedPathTest<'a> {
    pub(crate) fn new(center: [f32; 3], gather: &'a mut dyn FnMut([f32; 3], f32) -> Vec<([f32; 4], [f32; 4])>) -> Self {
        Self { center, gather, gathered: None }
    }

    fn is_path_locked(&mut self, path: &AiPath) -> bool {
        if path.bit_flags & 0x200 == 0 {
            return false;
        }
        let center = self.center;
        let volumes = self.gathered.get_or_insert_with(|| (self.gather)(center, 40.0));
        volumes.iter().any(|(min, max)| boxes_overlap((path.bbox_min, path.bbox_max), (*min, *max)))
    }
}

/// `DoBBoxesOverlap` 0x824b8178: per axis `b.max >= a.min` and `a.max >= b.min`.
fn boxes_overlap(a: ([f32; 4], [f32; 4]), b: ([f32; 4], [f32; 4])) -> bool {
    (0..3).all(|axis| b.1[axis] >= a.0[axis] && a.1[axis] >= b.0[axis])
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::ai_path::AiPathNode;

    fn path(id: u8, positions: &[[f32; 3]]) -> AiPath {
        let mut data = [0; 16];
        data[4] = id;
        AiPath {
            bbox_min: [0.; 4],
            bbox_max: [0.; 4],
            id: AiPathId(data),
            nodes: positions.iter().map(|&position| AiPathNode { position, ..Default::default() }).collect(),
            has_extra_node_data: false,
            branch_groups: Vec::new(),
            bit_flags: 0,
            allowed_skaters: u64::MAX,
            skill_level: 0,
            extra_data: [0; 3],
        }
    }

    #[test]
    fn the_nearest_node_search_starts_at_node_one() {
        let path = path(1, &[[0., 0., 0.], [5., 0., 0.], [3., 0., 0.]]);
        assert_eq!(nearest_node_index(&path, [0., 0., 0.]).unwrap(), 2);
        assert!(nearest_node_index(&self::path(2, &[[0.; 3]]), [0.; 3]).is_err());
    }

    #[test]
    fn usability_checks_the_allowed_bit_and_the_challenge_round() {
        let mut path = path(1, &[[0.; 3]]);
        path.allowed_skaters = 1 << 5;
        let skater = |bit| PathSkaterAbilities {
            allowed_skater_flag: bit,
            can_skate_race: true,
            can_skate_vert: true,
            can_skate_street: true,
        };
        assert!(is_usable_by_skater(&path, Some(skater(5)), None));
        assert!(!is_usable_by_skater(&path, Some(skater(4)), None));
        path.bit_flags = 1 << 11;
        assert!(!is_usable_by_skater(&path, None, Some(2)));
        assert!(is_usable_by_skater(&path, None, Some(1)));
    }
}
