# PROXY Environment admission diagnostic

The independently reviewed `artifacts/proxy-environment-admission-2026-09-23.patch`
was integrated. It changes the existing ignored `proxy_asset_census.rs` test and exposes the
already implemented `environment_is_hybrid` predicate as `pub(crate)` in
`simulation_static_arena_plan.rs`. The identical census replacement source is
`proxy_asset_census_environment_draft.rs`. The author made no registered edits
or Cargo run. Root ran the mounted corpus after integration and removed the
duplicate unregistered replacement draft, leaving one census owner.

## Source and decoded structure

Fresh `cProcessArenaAsset::ProcessInstanceList82d5ec38` decompilation shows
the Environment branch reading `tInstance` collision-model pointer at `+132`
first (`82d5eea0`), then its raw 32-bit `tCModelData::m_iNumMeshes` at `+4`
only when the pointer is nonnull (`82d5eeb0`). It then reads the render-model
pointer at `+128` (`82d5eec4`) and raw unsigned 16-bit
`tRModelData::m_iNumMeshes` at `+50` only when nonnull (`82d5eee0`). The
instance is skipped only if **both** model pointers exist and their respective
raw mesh counts are nonzero. Otherwise it calls `AddPresRegistryAsset` for kind
10 at `82d5ef1c`, adds the same asset to Simulation at `82d5ef28`, and appends
to the arena at `82d5ef34`. In particular collision count `0xffffffff`
remains nonzero. Confidence: high source control flow; exact decoded corpus
values after the pending probe.

`SimulationStaticArenaPlan::environment_is_hybrid` already performs precisely
those collision-then-render raw count checks using typed local RW4 references,
`CollisionModelData::raw_mesh_count`, and
`RenderModelData::raw_mesh_count`. The visibility change allows the diagnostic
to invoke that implementation directly against each retained `DecodedArenaAsset`
without copying the predicate or adding a parser. `InstanceData::parse`
supplies the existing decoded `Instance` fields (`guid`, `collision_model`,
`render_model`) from the same object bytes already visited by the census's
`InstanceData::record_table` view. The diagnostic checks the parsed record count
and each full GUID against that borrowed view before classifying it.

The prior required-collection corpus run found one Environment subtype
`acb31c9a` record in each of the 147 XSS spatial collections, and none in the
single parent. The diagnostic logs collection ID, arena ID, inline index, full
instance GUID, both model references, `hybrid`, and whether kind 10 would be
admitted for each spatial Environment record. It reports total records,
admissions and hybrid skips, and asserts that the observed record count still
equals the 147 XSS rows. This is a read-only processed-asset decision count;
it does not construct a processed Presentation asset, run registration, or
acknowledge proxy activation. The mounted result follows.

## Mounted result

`artifacts/proxy-environment-admission-corpus-2026-09-23.log` passes its
ignored original-data test. All 147 spatial Environment records passed the
same hybrid predicate for Presentation kind-10 admission; zero were skipped.
This is a corpus-bound classification of the observed records. It does not
perform processed-asset construction, registration or activation.
