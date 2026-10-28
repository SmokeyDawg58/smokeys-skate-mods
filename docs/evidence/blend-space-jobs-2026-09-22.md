# BlendSpace parameterization job connection ? 2026-09-22

Status: independently SOURCE/CODE APPROVED and registered; Cargo execution pending.

## Source and owners

SetAttributes 0x82d79a00, Initiate3 0x82d742c0, Complete3 0x82d777a8,
Register 0x82d6b948, worker 0x82d73520, Post 0x82d79030 and Wait
0x82d79118 are connected in the proposed packet. Independent raw evidence is
retained in artifacts/blendspace-job-lifecycle-independent-2026-09-22 and
artifacts/blendspace-parameterization-independent-2026-09-22.

The existing initialized Animatable job, entry point, scheduler and actual
body/channel tree pools remain the owners. The existing node's projected and
coefficient allocations move into a shared parameter block retained by each
queue descriptor. The initialized queue replaces the previous count-only
placeholder; its length is the single active count. Native pointers are
represented by retained typed owners, not truncated host addresses.

Register checks capacity eight before setting needed and appending. It never
deduplicates: repeated initiation overwrites the same parameters, and a second
queued occurrence can fail the native needed assertion after the first clears
it. Worker count is captured once; each descriptor is read when its iteration
runs, and the parameter pointer is read again after computation. Typed worker
failure survives scheduler propagation. Wait preserves queue and started state
on failure. Successful Post completes entries before clearing count/started.

Complete uses the old volume's first child GetTime then GetLength, scalar
division, and ascending new-child GetLength/multiply/SetTime. Current volume
publishes only after all children. No ready guard is introduced. Print and Jobs
are resampled at their original call sites. Four live boolean cvars retain
original defaults, registration order and GUI title ordering.

## Limits and validation

Non-optimized dimensions, debug overlay calls, IsDone, and BlendSpace pose
and attribute evaluation remain explicit missing work. Native debugLine += 12
is recorded as debug accounting debt. CPU host reciprocal estimates and other
VMX numeric boundaries do not establish unconditional Xenon bit parity.

Proposed tests exercise asynchronous shared output, duplicate-node assertion,
capacity failure after initiation, and partial new-child retiming without early
volume publication. Root must run compilation, strict checks, workspace tests
and the original-data frame corpus after independent approval and integration.
Confidence: high for reviewed source ordering; runtime validation pending.

Root validation: strict clippy-bs-jobs-3 PASS44.13s; workspace-bs-jobs-2 4,626 unit plus six doc PASS. Original-bs-jobs-1 reaches real worker but fails VolumeBounds at neutral frame2 (18/20 overall). Real serialized geometry interpretation remains under investigation; synthetic worker tests are not a real-data parity claim.
