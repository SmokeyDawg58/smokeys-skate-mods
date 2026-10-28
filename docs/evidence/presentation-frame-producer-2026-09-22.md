# Selected presentation frame producer and retained bytes

Confidence: high for recovered selected-list field/call order and native buffer
writes; exact at the documented storage boundaries. Full native rendering and
allocator contents are not claimed. Root owns Cargo and original-data execution.

## Same owners and reuse

The new `ActorPresentationFrameSource` borrows the actual Actor and its retained
`PlayerAttributeServices`. Player ID, peer, local/local-sim index, actual manager
Ghost pointer state, SkaterAnim opacity/flag, and PhysOut Motion/State/Skeleton/
Scoring2 fields are sampled in native order. The header makes its own fresh
GetLocalPose call; root's existing WriteSkater later captures local, face, right
and left allocations anew. No parallel pose/backend/manager is constructed.

Existing WriteSkater compresses Q4/Q3/T and submits Q then T, alignment2. Existing
ReadSkater decodes those exact submitted bytes and mutates the existing replay
allocation. Existing StreamCommands, SetData, ExtractSQT, dispatch and Sync are
reused. Original-data driver retains a producer across real selected physics
frames, and keeps the prior isolated writer probe.

## Source and ordered layout

`Sk8::SkaterPresentation::WriteData827dbf98` submits count4 align4 at bfe8 before
nonzero pose-size assertion c010, then explicitly zeros damage14 at c024. Each
208-byte header is submitted align16 at c4e4; all headers precede all poses.

GetPlayerID c124, fresh GetPeerId c148, IsLocal c160, IsCollidable c184,
opacity conversion c1c0, camera flag c1e0, teleported c204, speed c22c, root c240,
position store c324 before quaternion estimates, independent local pose c3c0,
hips/board stores c3ec..434, damage c44c/45c, wiping c470 and lazy integer
state-count c490, relative pose offset c4e0. Count assertion c5c4 precedes all
WriteSkater calls c610. Header-relative offset is
`208*(count-index)+pose_bytes*index`, stored low16; no saturation.

| Header bytes | Actual fields |
|---|---|
| 0..64,64..128 | fresh local hips and skateboard matrices |
| 128..144,144..160 | root quaternion and full root position |
| 160..168 | signed64 peer ID |
| 168..182 | seven existing packed damage level/intensity pairs |
| 184..188 | board surface speed |
| word188 | high16 relative pose offset, bits8..15 opacity, bits4..7 player |
| word192 | high16 integer wiping-state count, bits15/14/13/12 local/collidable/camera/teleport |

Unused header bits/bytes are retained scratch, never per-frame cleared.
GetPeerId825f0068 calls backend8255aba8 (row+88); actual reset82561ec0 writes
full64 -1. Board agent owns that field/getter and actual offline producer.
IsCollidable825f01c8 queries actual Ghost owner, distinct from backend ghost
boolean; native null returns true, nonnull remains explicit unsupported Ghost
method. Board agent owns actual constructor-null/query patch.

## Explicit PC allocation and collection policy

See `presentation-data-buffer-boundary-2026-09-22.md`. Native ctor82990dd0
allocates without proven zero fill. This host explicitly zero-initializes full
submission/gather backing ONCE. Native metadata resets preserve bit0 and all
unwritten bytes across reuse. The host's two retained gather allocations have
independent generations; recycled views return a typed stale error.

AddData829900c8 zeroes reached alignment gaps before alignment field publication
and payload bounds checking. Maximum alignment is a six-bit field with native
post-store assertion. SimpleDataBuffer.WriteData8298fd70 copies exact payload;
25-bit size publication retains native truncation. Failure preserves reached
zero/store prefixes. PostGather82990b44..c88 writes subframe4 align0 then actual
payload at submission maximum alignment; patches gap bits and resets submission
metadata only. Native empty-gather no-op is outside this nonempty selected-list
path: WriteData always submits count first.

The entry is named `new_uncompressed_host_policy`: caller explicitly supplies
AllowCompression=false. It does not disable or claim reconstruction of native
DataMan default compression, provider registry, DataTracker or render scheduling.
ReadFrame borrows the SAME retained gather backing from header start through
allocation end. Adjacent decoder reads see real prior writes/host-initialized
contents. No concatenated block padding or fabricated decoder bytes are used.

## Read caller boundary

Fresh UpdateBody827de868 selects prior/next headers via actual frame map and
flag8000, and may blend. Full UpdateBodies827de988 also gates on actual model
presence. These producers remain subsequent work. The reusable lower-level
`try_decode_and_extract` explicitly takes admitted source/destination allocations;
it preserves current SkaterFull/Face_Offset0 lookup, BeginSet/BeginCommands,
ReadSkater, ExtractSQT, EndCommands/EndSet/Sync. The original driver explicitly
supplies actual selected Actor list and restored odd/base?even/output allocation;
it does not fabricate model presence, frame-map state or full rendered-frame
completion. MoveRootToHips/expressions/matrix extraction remain later consumers.

Host numerical profile retains existing estimates/conversions/XYZ reduction;
new XYZW reduction explicitly uses separately rounded products and left-associated
sums. Xenon estimate/reduction/NaN bit parity remains unproven.

## Validation

Focused tests cover retained tails/recycled views, gap writes and alignment stores
on late payload failure, exact subframe gap bits, header reserved-bit retention,
and ReadFrame borrowing the actual allocation with header-relative offset. The
original driver asserts synchronized nonempty finite reconstructed channels and
unchanged authored source allocation. Formatting and shared patch application
are checked by author; Cargo/original-data results are integrator-owned and pending.
