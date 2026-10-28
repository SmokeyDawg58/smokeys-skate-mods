# Tree Eval command boundary, 2026-09-23

The original queued fixture completes frames 0..20 and fails frame21 Actor Phase3
with `tree Eval command: UnsupportedSetup`. This is `AnimationEvalError::Command`,
so the failure occurs during command emission, before the later output-cardinality,
fetch execution, job submission and local/global pose work boundaries.

`tree-command-diagnostics-2026-09-23.patch` preserves all existing rejection
conditions and partial mutation. It identifies dynamic SQT allocation capacity,
binary/mirror/NBlend stack underflow and rejected coefficient bits. Fixed-SQT
owner/id admission still uses the original generic failure. Independent reviewer
approved the diagnostic-only change; no Cargo was run by the author or reviewer.

## Conditional next dependency

Fresh `GetNextLoc82a24c28` demonstrates that native SQT capacity exhaustion is not
unconditional failure or permission to increase capacity. At 82a24c68 it admits
only count < maximum. Failure yields -1 at 24c6c. After checking the existing error
and reuse count (24d6c..24d98), 24d9c..24dbc clears the allocation counter/current
maximum. The existing command graph from begin-node+1 is scanned at 24dd0..24e74;
unconsumed outputs (pid unsigned >=0x8000) excluding fixed Set/Reuse commands are
assigned dense output indices in original graph order. At 24e9c..24ea8 native
checks the required retained-output count against the same capacity. It reserves
retained-output-count+1 at 24ebc..24ec0, emits a reuse command carrying data kind,
then connects the popped graph dependency and increments reuse count at 2533c
(or empty dependency 253a8). The returned allocation ID is the retained count.
Raw branch/command-packing verification is required before implementation.

Existing body and RAW constructors already load the actual `ACSreuseBuffer`
entry point. Repository search finds no existing executor or command variant for
that entry. A capacity diagnostic would therefore identify a missing same-ACS
reuse-command producer/executor, not a reason to create another animation owner
or resize the native allocation. The reached diagnostic must decide whether this
continuation is actually required before code is added.

## Reuse executor and connected runtime seam

Fresh `ACSreuseBuffer_job82a22ac0` raw inspection: param1 is an array of pointer
pairs, param2 the record count, param3 the bone count. Each record is destination
then source (loads 22ae0/22ae4), equal pointers skip copying (22aec), otherwise
`XMemCpy` copies exactly 48*bone_count bytes (22af0..22b04). Records run in order,
advancing eight native bytes. This is a sequential copy, not a simultaneous
snapshot/permutation; later copies observe earlier stores. Zero records do no
reads or writes. No new quaternion/vector math is required.

`DispatchReuseBufferCmd82a2fc18` finds the prior root and scans graph nodes before
the reuse node. Live outputs are those with parent outside that segment
(2fd20..2fd30); already handled earlier-segment nodes and fixed Set/Reuse outputs
are skipped for copying. Destination is the compacted dynamic slot, source is
the old output encoded by the original command (2fdb4 and2fe68). It retains the
pair count and advances the segment boundary at2fe90/94. Nonzero copies turn the
existing latest null job into `ACSreuseBuffer` and set pointer/count/bones at
2fec4/2ff00/2ff3c/2ff70. It then flushes pending fetch work, aligns the next extract
range to128, and flushes SetData, SQTProcess, Pose in that order
(2ff74..30068), then increments the reuse count30078. Empty copy lists still
retain the flush/segment boundary; they do not require a fabricated copy job.

Existing connected owners:

- `BufferedBodyCommands.entries` already retains the loaded reuse entry; the
  same scheduler, fetch/blend pools and pending body work must be reused.
- `TreeCommands` currently retains separate fetches and operations, plus physical
  slot IDs in its stack. `BindPoseCommand` has no reuse command and there is no
  retained generation/segment representation. Merely resetting next_sqt would
  alias earlier fetch destinations and corrupt still-live outputs.
- `BodyPoseWork::run(FetchVbr)` executes all fetches, then the one SqtBlend job
  executes all operations. A reuse continuation therefore also needs ordered
  segment execution: prior fetch/blends, live-output copy, later fetch/blends.
- `AnimationSchedulerBackend::submit_body_tree` presently validates exactly one
  fetch and optional one blend before local/global pose. It must admit the
  concrete segmented dependency chain on the same backend; a standalone reuse
  leaf cannot make the reached producer/consumer path correct.

If capacity is confirmed, the smallest coherent packet is a same-TreeCommands
segment/reuse record with retained live-output mapping, the exact copy executor,
and per-segment work views sharing the existing BodyPoseWorkState/allocation.
Existing fetch/blend numeric functions and final local/global pose jobs remain
reused. Tests must force more fetches than the unchanged native capacity, verify
live-output preservation across at least two segments, retain fixed operands,
and discriminate sequential copy order. Full native arbitrary graph admission
must not be inferred from this selected tree path. No implementation was added
while waiting for the reached diagnostic.

## Confirmed reached branch and implementation packet

Root's original diagnostic confirmed both queued modes at frame21:
`DynamicSqtCapacity { next:20, maximum:20 }` (original suite19/22; workspace4863
unit tests plus6 doc tests). This establishes the actual bounded allocation
continuation; the packet never increases the20-slot descriptor.

Unregistered implementation files are `actor_animation_reuse_buffer.rs`,
`actor_animation_reuse_body_jobs.rs`, `actor_animation_reuse_buffer_tests.rs`,
and `actor_animation_reuse_body_jobs_tests.rs`.
Shared changes are in `artifacts/tree-reuse-buffer-2026-09-23.patch`.

The existing body collector explicitly admits reuse; other RAW/fixed-stream
callers retain their prior capacity boundary. Its live-output ledger follows
producer/graph order, including graph-live values after command-stack pops. A
reuse boundary retains fetch/operation ranges and ordered destination/source
pairs. The same bounded SQT vector survives across all segment work views. The
existing loaded code, job pools, scheduler and pending owner are reused; final
local/global pose publication and Sync remain the existing path. Later segment
fetches cannot execute before the prior segment's blend and copy jobs.

Pending NBlend dynamic operands are intentionally excluded from allocation
reuse. Native stores their list IDs before GetNextLoc, but independent review
found no native list-ID rewrite in EnQueueCmd/Schedule/EndCommands. The new typed
`ReusePendingOperands` boundary preserves that uncertainty instead of applying
an inferred remap. Ordinary future fetches and fixed-pair binary allocation do
not have this ambiguity. The copy leaf retains sequential alias behavior and
skips identity pointers before reading storage, matching native.

Focused tests exercise unchanged capacity at the first compaction, graph-order
rather than slot-order mapping, checked fixed/second-boundary refusal, a sequential copy
whose later source aliases an earlier destination, identity/zero-record laziness,
and the actual existing scheduler executing fetch→blend→copy→fetch→blend before
final local/global jobs. Author/reviewer do not run Cargo; root owns runtime
acceptance and the next original-data result.

Malformed/exhausted-live-set boundary: native accepts retained-count equal to
capacity before reserving one further slot. This host packet refuses equality
as checked missing storage after clearing its allocation counter. It does not
claim native invalid-capacity partial-state/trap parity. The supported successful
path requires retained-count strictly below capacity; neither fallback capacity
growth nor dropped live outputs are permitted.


Final independent source review narrows first-compaction admission to dynamic-only
roots. Native dispatch advances destination ordinal for fixed roots while skipping
their copy, and later compactions filter nodes using mStartBuffSegmentId and a
replacement graph node. This packet refuses a collector that has acquired a fixed
owner (`ReuseFixedRootsPending`) and refuses its second compaction
(`ReuseNextSegmentPending`) before those unproved mutations. It does not claim
repeated-compaction or fixed-root equivalence. Separate tests exercise these
checked prefixes. A further test runs actual SegmentWork copy and blend ranges,
checks final output extraction from the same allocation, and checks typed error
retention without publication when a copy source is missing.

Reuse boundary handles are lazily retained in the same BufferedBodyCommands,
reused across frames and cleared through the existing pending-job Sync path.
A later setup failure therefore cannot discard an already-created boundary handle.

## Follow-up read-only second-boundary audit

`BeginCommands82a229c0` records the last existing graph node (or0) as
mBeginCommandsNodeId and copies it to mStartBuffSegmentId at229fc/22a0c.
`EndCommandSet82a330c8` resets mNumReuseBufferCmds and mStartBuffSegmentId to0 at
3310c/33114 before ConstructRootList/Evaluate. Thus the dispatch-time segment
marker is rebuilt by reuse dispatch; it is not simply the emission-time marker.
`ConstructRootList82a22a38` collects final graph roots (unsigned pid>=0x8000)
in node order, storing each root's cid. `Evaluate82a33018` repeatedly scans nodes
ascending and schedules nodes whose remaining dependency count is zero.

Fresh raw `GetNextLoc24f1c..24fa4` confirms the inserted reuse command consumes
only the command-stack top. It copies that top's fixed bit and obtains its output
from the updated mOutputTable, encoding command kind15/data kind/output. The graph
insertion at252ac..25330 likewise pops one graph-stack entry, creates a new node
with one dependency, assigns the old node's pid to the new node, and pushes the
new node. Other live roots are not simply all replaced. The new requested
allocation returned from GetNextLoc is distinct (v19), while the inserted reuse
command's own output is the remapped former stack top.

Consequently repeated admission requires a retained producer node for the reuse
command itself, plus separate original command output fields versus mutable
mOutputTable references. The first-packet ledger only records live slot/order,
so it cannot represent all these distinctions. At dispatch, nodes at/before the
previous mStartBuffSegmentId advance the compacted destination ordinal but emit
no copy; only later eligible nodes produce destination/source pairs. Source IDs
come from each original command field, not the mutable output table. A generic
'recopy every live value' or resetting every producer ordinal is not proved.

No repeated-boundary code has been added. The next concrete step, if the original
run reaches it, is a minimal command-node/reference ledger on the same collector,
with source tests that distinguish the inserted reuse node from its newly
requested fetch allocation and distinguish original output fields from remapped
consumer references. This is an extension of the same retained ACS, not another
pose buffer or a capacity increase.

## September24 reached hand and repeated-boundary continuation

Controller-feedback push replay reached RAW HandsFace DynamicSqtCapacity18 at
frame205. The original hand StreamCommands never enabled the existing reuse
collector and its executor ran all fetches before all blends. Added segmented
RAW work over the same pending SQT owner, allocation and pools, reusing the
existing copy executor and operation-range execution. Empty-copy segments keep
chain ordering without a copy job. Final extraction belongs to the last producer.
Fresh GetNextLoc82a24c28, DispatchReuseBufferCmd82a2fc18 and copyjob82a22ac0
inspection agrees with the earlier evidence above.

A fixed owner can be retained after its fixed operands have been consumed;
that case now proceeds. Actual live fixed stack roots remain explicitly gated
because their destination ordinal requires separate graph handling. A synthetic
bounded hand program verifies consumed fixed operands, retained dynamic output,
and final-only extraction. Workspace5014 core plus other suites passed.

The next heldA replay passed205 but reached a second BODY reuse at214. Extended
the existing collector's live-output ledger to retain original command output
and producer order separately from mutable output-table slots. At each boundary,
copy selection excludes producers at/before the previous marker while still
advancing destinations. The inserted reuse producer replaces only stack top,
with its remapped encoded output; it is separate from the returned allocation.
The seven-reuse limit is retained. Existing unit coverage now discriminates
second-boundary copy selection and remapped references. Runtime validation of
this latest extension passed: heldA480ticks43.12s (both205/214 failures cleared),
three other push routes140.09s, focused full-left165ticks18.86s. Core5014 PASS.
This validates reached selected streams, not arbitrary fixed-root graph reuse.
See push-feedback-investigation checkpoint for final workspace/release status.

## Original-data acceptance after integration

Root combined gate:4879 unit tests plus6 doc tests passed. Original suite19/22
in12.68s: both queued modes pass frame21 reuse and Actor Phase3, then stop in
physical pre-solve at `Selection(PendingTransition(100))`. This proves the actual
reached20-slot first dynamic-only reuse path completes in the mounted caller.
It does not establish fixed-root, pending-NBlend or repeated-compaction support.
Root owns the next GroundAnimation103→Ground100 exit continuation.
