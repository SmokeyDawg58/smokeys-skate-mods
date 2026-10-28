# Simulation Environment unlock router

## Scope

This slice reconstructs the state and lower-world effect ordering of
`UnlockableLocationMan` for Simulation kind-10 Environment instances. It does
not reconstruct the missing career/progression producer of context records or
the Simulation lock-status word, native fixed-hash insertion history, complete
SimReg/DReg message delivery, collision queries, or Presentation behavior.

## Provenance and layout

The reference inputs are:

- XEX SHA-256
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB SHA-256
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
- MAP SHA-256
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`.

Raw PPC, PDB types, and the linker map establish an
`UnlockableLocationMan` size of `0x1068`. Its fixed map begins at `+0x08`; the
stored `eastl::bitset<16>` begins at `+0x1060` and physically occupies eight
bytes. Each eight-byte `tUnlockableEnviroInstanceContext` has:

| Field | Offset | Reconstructed representation |
|---|---:|---|
| `mpInstance` | `+0x00` | non-owning weak Environment source |
| `mChallengeIndex` | `+0x04` | signed `i8` |
| `mIsUnlockedEnvInst` | `+0x05` | raw `u8`; zero false, nonzero true |
| `mIsInWorld` | `+0x06` | canonical state bit |

The map type is
`fixed_hash_map<uint64_t, Context, 128, 129, overflow=false>`. Raw allocator
initialization exposes 128 inline `0x18`-byte nodes. The constructor's bucket
hint is 129, while the selected live bucket count is 127. Lookup hashes the low
32 key bits but compares the complete 64-bit key. Native re-evaluation walks
ascending buckets and follows each collision chain; it is not insertion order
or a generic host hash-map order.

The base constructor at `0x828b0408` creates an empty map and clears the full
64-bit status word twice. Base `Init` (`0x825187c0`) and Sim `Init`
(`0x827d45e0`) are no-ops. `Simulation::Workspace::InitWorkspace`
(`0x828a7d30`) constructs the Sim manager. No context insertion or writer for
the Simulation status word was recovered.

## Lookup and admission

Private `AddEnviroInstance` (`0x828b0068`) and `RemEnviroInstance`
(`0x828b0188`) look up `tInstance.m_uiGuidLocal` at instance offset `+0x68`.
This differs from `ProcessInstanceList`, which uses the primary GUID at `+0x60`
for kind-10 registry identity.

Both Add (`0x828b00ec`) and re-evaluation (`0x828afed8`) load the complete raw
64-bit bitset word. A signed challenge index selects a bit only after an
unsigned `<16` check, so exactly `0..15` are valid; every negative value and
every value `>=16` gets `mask_bit=false`. Upper 48 stored bits are retained but
cannot affect admission. The exact truth function is
`mask_bit == !is_unlocked`, equivalently `mask_bit != is_unlocked`:

| Mask bit | Unlock byte nonzero | Desired in world |
|---:|---:|---:|
| 0 | 0 | false |
| 0 | 1 | true |
| 1 | 0 | true |
| 1 | 1 | false |

## Mutation and effect order

`Simulation::RegArena` (`0x828a01e0`) walks forward, stops at the first null,
and routes kind 10 through public Add (`0x828b02d8`), which always forces the
immediate rather than queued path.

- Missing context: call AddToWorld immediately without inserting or changing a
  context.
- Found context: compute admission, overwrite the stored input pointer and
  `mIsInWorld`, then call AddToWorld only when admitted. Repeated registration
  overwrites the pointer without removing the prior instance.

`UpdateEnviroInstanceLocks` (`0x828afe60`) walks native map order, skips null
stored pointers and unchanged states, calls AddToWorld or RemFromWorld first,
and only then writes the new `mIsInWorld`. This deliberately differs from the
initial registration ordering.

`Simulation::DRegArena` (`0x828a05f0`) also walks forward. Rem clears a found
context's pointer and in-world state first, then unconditionally calls
RemFromWorld with the input instance—not the formerly stored instance. This
occurs for missing contexts and unlock-suppressed registrations too.

`Flush` (`0x828b0230`) walks native map order and invokes Rem only for a
nonnull stored pointer with nonzero in-world state. Rem performs the clear
before the lower removal; nodes remain allocated. `DeInit` (`0x828b02f8`)
flushes, frees all nodes, zeros the element count, then clears all 64 status
bits. The base destructor does not flush. `Simulation::Purge` calls DeInit
before physics-world shutdown, while workspace shutdown later destroys the
manager. Instances admitted through the missing-context branch are not in the
map, so Flush and DeInit cannot discover or remove them; ordinary arena
deregistration remains responsible for those lower entries.

The Rust router intentionally has no custom `Drop` hook. Ordinary field
destruction releases its owned host context and pool storage, but it is not
native Flush, DeInit, or a downstream removal-callback sequence. Lifecycle code
must invoke `deinitialize` explicitly when reconstructing the proven Purge
phase; dropping the value is only memory-safe host cleanup.

## Readable ownership and explicit unknown input

`SimulationEnvironmentUnlockRouter::try_new_empty` represents the exact empty,
zero-status constructor. The separately named
`try_new_with_external_seed` is an evidence-honest import boundary for the
unrecovered producer. Its vector must already be in native bucket/chain
iteration order; the router preserves that order with bounded linear lookup
instead of claiming a recovered insertion history.

The concrete router owns `StaticGroundPool`, so callers cannot omit or reorder
its lower effects. Resident code retains each `Arc<StaticEnvironmentSource>`;
contexts hold `Weak` handles, matching the native non-owning pointer without
extending resident lifetime. A stale handle errors only when a changed state
requires a lower call. Unchanged states do not dereference it.

The 128-record limit and duplicate-key rejection protect the imported map
invariant. Pool capacity preflight, typed allocation/capacity failures, and
stale-handle errors are safe host policies because native failure behavior is
not recovered. For found initial registration, preflight occurs after the two
proven upper stores, so failure retains the attached source and in-world state;
an absent-context failure remains untracked. A re-evaluation failure preserves
its completed ordered prefix, leaves the current context state unchanged, and
skips later contexts. Stale-handle Flush/DeInit failures likewise preserve the
unprocessed context/status state as safe host fail-stop behavior, not recovered
native failure semantics. The current diagnostic app now reaches this router
through the bounded static arena adapter, but still does so during asset build
and explicit host `Drop` teardown. It is not presented as native SimReg/DReg
message timing.

## Corpus boundary

All 26 accepted Rob's Park Environment instances have local GUID
`0xffffffffffffffff`; their primary GUIDs are 26 distinct values. This makes
the local-versus-primary identity distinction observable. It does not by
itself prove absence of an `0xffffffffffffffff` context, so the runtime path
continues to require explicit context input rather than assuming admission.

An independent literal-reference scanner searched those 26 primary GUIDs as
big-endian bytes, little-endian bytes, lowercase hex, and uppercase hex across
22 archives (23,749 logical entries / 4,143,132,392 logical bytes) plus six
loose databases (7,901,428 bytes). It found 43 references, all big-endian and
all confined to the four source BAM Simulation XSFs; every primary GUID was
present and no literal occurred in the scanned mission, ChallengeCore,
progression, living-world, or FE-location data. This is exact negative literal
evidence, not proof against hashed, name-based, or indirect associations.

The scanner source SHA-256 is
`A3BF2685863B55A93D9014BA7ADC3FBDB5A6205D47DD1549709896E5DCBCCA49`;
its `Cargo.toml` is
`8778B8771D17AA84BCEE4BDC6BEC2CF7F51953D5FB086C7DF8321F19840043ED`,
lockfile is
`A7D0A9DF7E1CD74B1ABE14EEF515FD8667F350A51070CB1E03EA6FF6A2221538`,
and frozen output is
`CB9A718EAE4341ED149D1DFE942FC29E60EA42B9D4BC8584E532A36277C21526`.
The source is external at
`unlock-context-audit`; no proprietary data or output is
stored in the repository.

Focused tests gate the four admission rows, raw noncanonical unlock bytes,
signed challenge bounds, retained upper status bits, full-width GUID lookup,
absent/found/suppressed/duplicate registration, deferred input effects,
ordered-prefix failures, remove-before-add capacity reuse, stale weak handles,
unconditional deregistration, Flush, DeInit, and the 128-entry seed boundary.

## Confidence and open work

Type layout, lookup identity, predicate, branch behavior, mutation/call order,
iteration requirements, constructor, Flush, and DeInit are **exact**. The
`Arc`/`Weak`, linear seeded lookup, and typed failure translation are **high**
safe-host representations. Context population, challenge/unlock producers,
Simulation status mutation, native fixed-hash insertion history, live message
timing, downstream query batches, and Presentation integration remain open.
