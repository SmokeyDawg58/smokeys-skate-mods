# Update5 final output callbacks

Native `PhysicalPlayer::HiLOD::Update5`82e04ea8 calls surface check at
82e061ec, `ProcessIntents`82e04de0, physical `AnimOut::Fill`82b3a1e8,
physical animation Reset and finally increments HiLOD update_count.

The registered selected Ground corpus continuation now executes this sequence
after push history/coffin/grab publication. Surface checks use the retained
Processed physics state8e4 (not current state8e8), current collision output,
encoded foot/footplant surface IDs, or the captured hips surface according to
the original state branches. Surface6 requests teleport and9 requests wipeout;
these are guarded true-only stores, including the original repeated guards.

ProcessIntents queries presence of the actual Motion map's native PUSHING key
before the output guard. Zero and NaN weights still count as presence. The
physical animation fill reads facing80000000 before its guard sample, guards
the five-word grind name copy, and preserves the four subsequent per-field
read/guard orders. Input absence40000000 is inverted; autopump20000000,
kickturn10000000 and anticipation08000000 are copied directly. Each flag read
requires its actual known bit rather than inventing missing producer values.

Only successful completion resets physical AnimOut and increments the actual
HiLOD counter with native wrapping. Late errors preserve earlier publications
and do not perform that reset/increment. Update5 itself does not lock output.

Independent runtime_path source review accepted surface semantics and final
callbacks after the root corrected the facing flag read order. Five surface
tests and four finish tests cover state selection, true-only writes, owner
capture, locked-output ordering, zero/NaN intent presence, and late failure
before Reset/count. Workspace/corpus validation is recorded in the rolling
session checkpoint after the gates complete.

Confidence: high for the reviewed selected callbacks. This completes the
selected Update5 output tail with the explicit grind exclusion; earlier
Update4/UpdateAll stages, sustained ticks and playable app integration remain
incomplete. Component and original-data tests are not per-tick Xenon parity.
