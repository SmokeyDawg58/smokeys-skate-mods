# Location kind-21 arena plan

## Scope

This note separates the producer/lifetime facts needed for streamed Location
registration from the still-unimplemented live `cLocationManager` container.
The readable `LocationArenaPlan` is a per-arena plan only. It does not claim the
outer collection acknowledgement, a native fixed-up allocation, or a complete
`Simulation::RegArena` implementation.

The reference inputs used for this boundary are:

| Input | SHA-256 |
|---|---|
| Xbox executable | `3706...C0FE2` |
| Debug PDB | `0B06...6EF0` |
| Linker MAP | `51E6...7E88` |

The abbreviated hashes identify the same pinned reference artifacts used by the
rest of this repository's evidence set; they are not new corpus fingerprints.

## Native producer boundary

`cProcessArenaAsset::ProcessLocationData` at `0x82d5f2e0` handles raw Pegasus
type `0x00eb0009`. This is distinct from trigger-container type `0x00eb0019`.
For each successful first-TOC entry it calls `AddSimRegistryAsset` with:

- the entry's complete 64-bit TOC GUID;
- Simulation kind `0x15` (21); and
- the exact complete `tLocationDescData*` at entry offset `+0x14`.

It then publishes that processed asset to the arena list. The unprocessor is the
ICF-shared routine at `0x82d55390` (also named `UnprocessSplineList` in the
debug material). It consumes the processed-list representation; it does not
justify flattening inline location descriptors into independent producer
assets.

The relevant serialized layouts, confirmed by PDB types and
`tLocationDescData::Fixup` at `0x829c5998`, are:

- `tLocationDescData`, 24 bytes: three counts at `+0/+4/+8` and object-relative
  location, sublocation, and string-list links at `+0x0c/+0x10/+0x14`;
- `tLocationDesc`, 128 bytes: transform at `+0`, AABB at `+0x40`, sublocation
  count/link at `+0x60/+0x64`, name/description at `+0x68/+0x6c`, and the full
  64-bit GUID at `+0x70`.

`LocationArenaPlan` therefore scans only the first used-dictionary TOC, accepts
only local section-0 `0x00eb0009` entries, validates the complete object before
construction, and emits one operation per container in authored TOC order. It
retains the existing immutable arena owner. `LocationDataHandle` identifies the
whole object by owner/resource/span; `LocationRecordHandle` adds the exact
128-byte record position. Aliased dictionary entries into one allocation compare
as the same object or record, while byte-equal data in separate arena owners does
not collapse.

These handles are safe lifetime authorities for a later writable fixed-up host
representation. Their serialized bytes are not presented as live native
pointers.

## Native manager boundary

`cLocationManager::RegArena` at `0x82a8eff8` and `DRegArena` at `0x82a8e430`
walk their supplied processed list forward until its first null entry. They
select only kind 21 and pass the complete asset ID to `AddLocationData` or
`RemoveLocationData`.

`AddLocationData` at `0x82a8e980` reacquires the whole object from the Simulation
registry by that complete ID, walks 128-byte records forward, and skips records
whose name pointer is null. Each remaining name is hashed with
`GetHashValue32` at `0x82a74490`. The manager inserts a duplicate-key multimap
node containing the hash, the current high-priority byte snapshot, the streamed
flag, and the exact descriptor pointer. Equal hash keys are accepted rather
than coalesced.

`RemoveLocationData` at `0x82a8de98` reacquires the same whole object, hashes
each non-null name, scans that hash's equal range for the exact descriptor
pointer, and erases only the first exact match. Teardown identity is therefore
`(name hash, descriptor pointer)`, not the location GUID, string value,
priority bit, or outer container ID alone. Add/Remove counts and native call
returns are ignored by the per-arena receivers. A missing registry object on
the native add path is not a recoverable false result.

The manager uses an EASTL fixed multimap with 1,024 inline nodes and overflow
enabled. Its allocator-failure aftermath is not sufficiently recovered to
model as a safe capacity error. None of these live manager effects are claimed
by the producer plan in this milestone.

## Independent whole-BAM gate

The optimized `--inventory-bam-sim` gate serializes an independently specified
FNV-1a stream beginning with
`SKATE2/BAM_SIM/LOCATION_KIND21_PLAN\0V1\0`. Big-endian integers, raw IEEE-754
bits, optional UTF-8 strings, object metadata, and every inline descriptor are
included. The supplied BAM Simulation corpus passes with signature
`024de4660f5fc535` and these frozen measurements:

| Measurement | Value |
|---|---:|
| Collections / arenas | 284 / 2,135 |
| Kind-21 containers / affected arenas | 107 / 107 |
| Empty containers | 83 |
| Locations / sublocations | 53 / 0 |
| Sum of serialized string counts | 0 |
| Container location-count histogram | `{0:83, 1:8, 2:8, 3:5, 4:1, 5:2}` |
| Source resource indices | `{0}` |
| Minimum / maximum object span | 32 / 778 bytes |

All 53 locations have names and descriptions. Container IDs, location GUIDs,
and names are nonzero/nonempty and unique in this corpus; there are no observed
name-hash collisions or aliased source spans. Those are corpus facts, not
assumptions baked into the plan: the owner-bound representation still preserves
duplicates and aliases.

## Streamed runtime adapter

The bounded `LocationArenaRegistrationAdapter` now reconstructs the streamed
kind-21 manager slice without pretending to be the complete native manager.
`PreparedLocationArena` validates producer metadata and exact raw name bytes
before effects. At runtime, every container registration and deregistration
performs a fresh `LocationArenaRuntimeSource::resolve_location_data` lookup with
the complete 64-bit asset ID, matching `GetRegAsset` rather than silently
reusing the producer-time source.

For each non-null name pointer, registration hashes the retained bytes with the
exact signed-byte/NUL-terminating `GetHashValue32` reconstruction, samples the
live raw high-priority byte immediately before insertion, sets streamed true,
and inserts after every existing equal hash. A null name skips hashing,
priority sampling, and insertion; a non-null empty string hashes to zero and is
inserted. Deregistration performs no priority samples. It freshly resolves the
container, rehashes each non-null name, scans the equal range forward, and
removes only the first pointer-equivalent record.

The host multimap is a hash-sorted vector whose entries own
`LocationRecordHandle`s. The representation is intentionally not claimed to be
the native red-black tree or 1,024-node inline allocator, but it preserves the
observable unsigned order, stable equal-key order, duplicates, and exact
descriptor identity. Retaining a handle in every node also prevents a stale-ID
reload mismatch from dangling the original descriptor: native-equivalent
removal quietly misses, while the old source remains owned by the surviving
entry.

Non-cloneable occurrence tokens, checked IDs, fallible host storage, caught
runtime-source unwinds, first-fault latching, and inert `Drop` are
high-confidence host safety policy. A fault retains the reached insertion or
removal prefix and performs no rollback. Missing registry data becomes a typed
terminal host fault; the native code instead reaches an unconditional null
dereference. The adapter excludes the constructor's nine `DebugSeedData`
nonstreamed locations, `GetLocationInfo`, and other nonstreamed producers, so it
does not claim complete `cLocationManager` equivalence.

An independently specified pair of FNV-1a V1 runtime oracles passes the full BAM
Simulation corpus with registration signature `2c83b407a844d816` and
deregistration signature `8b32aa3e9943123b`. The collection-scoped sweep
issues and consumes 2,135 arena occurrence tokens, performs 107 full-ID
container resolutions in each direction (including 83 empty containers), and
inserts/removes all 53 named descriptors. Alternating registration priority
samples produce 27 high and 26 low entries; teardown performs zero priority
samples, all 53 fresh record identities match, and the final map, registry, and
live-token set are empty. Synthetic tests cover null versus empty names, raw
priority bytes, repeated pointer duplicates, stable equal-hash order, stale
fresh-source mismatch, full-width IDs, foreign tokens, panic prefixes, inert
token drop, and successful overflow beyond 1,024 streamed entries.

Global interleaving with every normal Simulation kind and the outer
normal/Location/DMO collection receiver remains later work.
