# Shared SQT views and OverrideWith amendment

Unregistered draft, September 12, 2026. New child
`actor_animation_sqt_output_ranges.rs` and eight focused tests await root review.
`skater-presentation-sqt-range-amendment.patch` is a reviewable proposal against
`actor_animation_sqt_output.rs`; it has NOT been applied. No registered files
were edited for this task, and no Cargo command was run.

## Recovered operation

`Andale::PoseBufferExt<AnimCmdSys::SQTExt>::OverrideWith825920b0` iterates
SOURCE hierarchy parts in authored order. `GetPartIndex825910a0` scans the
DESTINATION hierarchy and returns the first matching packed part name. The
implementation uses the parser's actual `packed_name` equality, preserving its
complete representation; it does not compare display strings or name_index.
Missing destination parts are skipped. At `82592184`, the call to
`memcpy82ea0690` copies source_part.bone_count*48 bytes from source_part.offset
to the matched destination part.offset. It does not copy destination part
count, clamp to that count, or copy a generic whole full-rig buffer.

Native memcpy's aligned path copies forward. The bulk sequence
`82ea082c..82ea08a8` has at most three doublewords of source lookahead before
stores; the prefix/tail paths `07d0..07d8` and `0734..073c` also advance
forward. For SQT-aligned subranges in the same allocation, offsets differ by
multiples of48 bytes, exceeding that lookahead. Ascending record reads/writes
therefore preserve overlapping copies: copying records0..3 to1..4 propagates
record0, unlike memmove or an eagerly captured source snapshot. Later source
parts must likewise observe earlier destination writes in the same allocation.
This proof is restricted to the original aligned SQT storage and record-sized
view offsets, not arbitrary unaligned byte copies.

## Proposed shared amendment

The SAME OwnedAnimationSqtBuffer handle gains a checked allocation-relative
Range and an optional actual hierarchy Arc. allocate_unknown remains explicit
anonymous storage for existing controlled fixtures; allocate_for_hierarchy
retains the real caller-supplied descriptor. Production construction should use
the actual playback hierarchy, including hand/face initialization and the
presentation bind/replay buffers. No hierarchy is inferred from bone count.

try_subrange produces a checked same-Arc raw extraction view, without inventing
a subhierarchy. try_subrange_for_hierarchy optionally binds an actual supplied
matching-size descriptor for a native pose-buffer object with its own hierarchy.
The ordinary source Restore facial ExtractSQT pointer arithmetic only needs the
raw view. OverrideWith requires both actual descriptors and checks each matched
part's source and destination allocation range immediately before copying that
part. Earlier part copies survive a later host-range error.

Registered len/is_unwritten/extract/read must use the handle's range. The
allocation Vec, generation and Sync visibility remain shared. Extract mutates
only the view and bumps the same generation on the actual producer write;
complete_sync still requires all allocation values known, preserving the
selected bind-first/full-rig lifecycle. Untouched bones keep their values and
provenance. Unknown allocation values never become fabricated initialized SQTs.

The patch also distinguishes extraction_pending from synchronized. A fresh
unknown allocation and an active asynchronous extraction both currently have
synchronized=false; synchronous OverrideWith must not publish over an active
job. begin_extract sets pending and rejects another outstanding extraction on
the same allocation; actual successful complete_sync clears it. The selected
Restore schedule has one facial extraction after a synchronous bind copy.
Concurrent extraction into multiple views is a future explicit admission scope,
not silently accepted by clearing another job's state.

Generation/pending/Sync are HOST publication bookkeeping, not native object
fields or extra source stores. A synchronous OverrideWith increments generation
once upon its first actual record copy, hides visibility during mutation and
publishes at successful completion only if all allocation records are known.
No matching/nonempty copy leaves generation unchanged. Later host-range failure
preserves earlier copies and leaves the destination unreadable. The operation
locks identical Arcs once, and different Arcs in stable host allocation order;
no clone of source pose values is used to sidestep overlap or locking.

## Root integration checklist

1. Review/apply the narrow parent patch, format it, and register only this child.
2. Change presentation bind/replay allocation to allocate_for_hierarchy using
   the actual retained SkaterPresentationDB hierarchy; same for production hand
   and face buffers when their actual playback hierarchy is already available.
3. Correct the earlier unregistered Restore comment claiming all-bone copy:
   the actual operation is source-part matching, even on compatible full rigs.
4. Keep category2 scheduler admission and actual presentation caller separate;
   this patch does not supply those dependencies.

Tests cover nested same-Arc view writes and generation, range/descriptor checks,
source-count/missing-part behavior, positive overlap, authored part ordering,
late range failure with preserved earlier copies, anonymous metadata rejection,
and pending-extraction rejection. Draft files are formatted. Tests intentionally
require the proposed parent amendment and remain uncompiled until root's gate.
Confidence: high for the recovered part traversal and selected aligned overlap
semantics; full presentation publication remains unimplemented.
