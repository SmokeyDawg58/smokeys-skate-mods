# Assembly pool refresh and repeated ground frames

2026-09-07. Confidence: `high` for the represented nonparallel source path;
source addresses and field/branch order are `exact` facts. Host arithmetic
remains the documented ordinary-finite backend, not a Xenon trace oracle.

`serialised_assembly_pool_update.rs` is a private child of the existing Create/
Update owner, not a replacement physics engine or generic frame wrapper.

## Source and implemented order

- `AssemblyPool::Params` constructor `0x82e0ad90` initializes
  updateInParallel=false at `0x82e0ada4`. Later overrides are not excluded.
- `AssemblyPool::BeginUpdate`, `0x82e0dc98`: perform serialized Update on every
  assembly before submitting the bounds job.
- `AssemblyPoolUpdateJob`, `0x827cdf78..0x827ce110`: visit every assembly,
  allocate 32*numVolumes bounds bytes then numVolumes validity bytes. In
  ascending volume order load the u16 body index, volume type, and selected
  body-table pointer (`0x827ce070`) BEFORE the Aggregate test. Aggregate is
  invalid for union, without body dereference or numeric work. Every other
  volume calls CalculatePartBBox (`0x827ce088`) with stored VolumeData geometry
  and transform plus current body motion. There are no frozen/static/enabled
  gates. Store results into separate pool bounds storage.
- `CalculatePartBBoxUnion`, `0x82e0b970`: positive-zero four-lane initialization,
  first valid box copied bit-for-bit, subsequent accumulator/current min/max
  in source order. Empty/all-Aggregate sets produce positive zero. The existing
  recovered helper is reused, including its numeric seam.
- `AssemblyPool::EndUpdate`, `0x82e0bb10`: after job completion publish BOTH
  16-byte bounds vectors into EVERY Header AABB (`+0x20`, stores
  `0x82e0bb7c..88`). Only afterward rescan live Part state bit 1 and append
  non-all-frozen assembly ordinals to the active list. Zero Parts are all
  frozen; all-static alone does not exclude an assembly from this list.

Root independently read the live job, EndUpdate and Union bodies; a separate
A reviewer checked implementation/source order. The primitive bounds,
four packed motion words, original image generation and occurrence-qualified
body snapshots reuse the existing implementations. No native pointer is
fabricated from a host address.

## Ownership, failure and freshness

The synchronous entry point borrows every prepared assembly mutably and the
body pool/image immutably throughout Update, calculation, publication and the
active rescan. Duplicate mutable publication and intervening body mutation
cannot occur within that call. Temporary bounds remain separate until all
calculations succeed. Host allocation/unsupported/malformed errors are explicit;
earlier VolumeData updates may remain visible after a later failure, but no
new Header AABBs are published before all bounds succeed. All input bounds
stamps are invalidated before work. This is not native malformed-memory fault
rollback or reproduction of worker scheduling.

Create initializes host volume/bounds frame stamps. Serialized Update changes
the volume stamp and invalidates bounds; completed pool publication stamps
bounds. Ground Run rejects a processed assembly whose stamps do not match its
exclusive body phase. These are semantic host guards, not native Header fields.
They do not detect arbitrary mutation within one frame or prove BeginFrame
was called (the pool already has initial identity 1). The caller must refresh
every assembly immediately before entering its collision phase. In particular,
native frozen/all-static skips precede the stamp check: a stale frozen Header
can still skip Run, so this guard is NOT universal freshness authority.

## Original-data validation

The existing real DMO Aggregate/Sphere versus Rob's Park corpus now calls the
complete pool refresh before its first ground job. Its one-contact fingerprint
`0e7c704e6a10f08a` is unchanged. It then retains the same real template/arena,
body occurrence, inertia owner, address-image generation, ground pool and
Contact manager across eight frames. Each later frame updates volumes,
recomputes/publishes bounds, produces contacts, compiles and solves them, then
integrates the body. All eight frames produce one contact and clear RF rows.
The second frame explicitly rejects stale bounds before refresh, then observes
changed Header bounds after refresh. Frame 8 COM is approximately
`[588.8619,-54.40114,996.5048]` under the controlled host test configuration.

Original assets/identity and controlled placement are detailed in
[ground integration](assembly-ground-run-integration.md). This regression
retains its original library gravity and four solver iterations for continuity;
neither is claimed as the actual World/Island gameplay setting. Island labels
are controlled pair-free test inputs. The test uses existing private Joint/
Drive test entry points and remains a corpus harness, not the production app
loop or a captured native gameplay trace. Actual World configuration, persistent
island labels/partitioning, and live owner scheduling remain next integrations.

Final root gate passes: seven focused synthetic tests, 2215 workspace unit
tests, six doctests, formatting and all-targets Clippy with warnings denied.
Both ignored core original-data tests were explicitly rerun and passed,
including the eight-frame path. Ordinary workspace testing still excludes
those two corpus tests and the pre-existing ignored app test.
