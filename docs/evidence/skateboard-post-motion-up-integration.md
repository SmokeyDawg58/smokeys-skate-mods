# Board post suffix, motion publication and outer up integration

Later accepted83337/00b30a: actual wheel-support fixture and source-reviewed
fixed-heading entry included. Workspace39655 passes3349 units/core3106, six
docs; strict Clippy and original12 pass; fmt/diff5f667c pass. Logs:
%TEMP%/skate2-support-fixed-workspace.txt, skate2-support-fixed-clippy2.txt,
skate2-support-fixed-original.txt. Earlier zero-wheel evidence below still holds
for that separate earlier fixture; the following support fixture closes the
nonzero-wheel branch gap without changing runtime contact values.

`physical_player_wheel_support_corpus.rs` derives requested deck lift0.08665848
from actual four wheel sphere radii0.031, LIVE Part transforms and mounted
vertical terrain hits. Actual registered SetTransform preserves current Part
displacements (not initial construction placements); sphere local offsets are
zero. Ten-percent-radius penetration is the requested fixture target, not a
claim of measured achieved penetration after numeric transformations.
Real next solve emits board Parts[3,6,6], selected wheel count1; normal, Body
suffix and nonzero-wheel Up execute. Native Up ignores wheel3 acceleration,
but still executes real gravity/FETCH/graph route for nonzero count. Actual
dynamic speed becomes[0,0.48935813,0], upUP and averageUP with unknownW;
processed prior surface-speed remains legitimately Reset-origin0. No values
are substituted or marked as live Actor input. This establishes that branch
on actual contacts; positive acceleration contributions from wheels0..2 remain
focused source-test evidence.

`reckoning_grounded.rs` now exposes direct FixedHeading82ddd468: wrapper
82de0408 keeps its two early stores then delegates, while direct caller keeps
heading and defers dynamic store until82ddd600 after initial graph/normalization.
Independent raw review passes; two tests distinguish direct flipping/early
numeric failure from wrapper prefix and retain heading on successful evolution.
This is the entry required by GroundAnimation82d98d94, not a new complete
GroundAnimation schedule.

Source/code review: independent reviewer, 2026-09-08. Confidence:
high for the represented source branches; no deterministic Xenon per-tick
numeric trace or whole Actor/FillPhysOut parity is claimed.

## Source and retained ownership

- Body PostPhysics82b36a58 tail3749c..374c8: actual wheel count selects retained
  timer + original dt or positive zero; then SAME constructed truck dynamics
  and frames run against their retained drive owners and Vault attributes.
  Earlier timer writes survive a later drive failure. Timer ctor386d8 is zero;
  the CollisionInfo Reset does not reset this timer.
- Selected FillPhysOut82b2f3e8 motion: early Part6 linear samplef550 is retained
  across omitted preceding Fill stages. Angular guard samplef714 precedes the
  actual late omega readf718, then check/store. Deck guard check precedes dot
  and retained-vector store. Speed guard sampling precedes estimate/refinement
  and its later check/store. SAME body observation, current solved-frame Arc and
  pair output occurrence seal the sample. Unknown vector W remains unknown.
  Existing prior Update1 copy consumes the SAME published records into the SAME
  bound processed owner; omitted Update1 collaborators remain diagnostic gaps.
- UpdateUpVector82b26158 uses the SAME board collision state and post fields,
  retained processed prior surface speed, shared global Reckoning provider and
  retained physics global. Read-only processed-output identity checks neither
  rebind an unbound owner nor prefetch source values. Ground-normal global and
  graph access share one provider occurrence; graph decoding reuses existing
  ReckoningCapturedLayout and PointGraph arithmetic. No payload tuning is copied
  into runtime constants. Immutable layout validation is a checked host capture
  boundary; mutable singleton/editor replacement remains unimplemented.
- Selected outer ctor38aa0..38ad0 initializes dynamic up AND dynamic speed to
  full UP, contact acceleration and average normal to full zero. These execute
  in the existing ForceRuntime constructor projection before jump-normal38af0
  and queue construction. Body Reset/TeleportReset never repeat these writes.
- Ordinary continuation37e64 calls UpdateUpVector only after current Body suffix
  completion. Success reaches SAME queue reset37e78..80. Failed updates preserve
  earlier writes and the queue, and host attempt stamps prevent replay. The
  entrypoint does not fabricate the earlier bad-data/accelerated/rewind decisions.

See the detailed [up source packet](skateboard-update-up-vector-source.md),
[motion packet](skateboard-physout-motion-runtime-plan.md), and
[Body suffix packet](skateboard-postphysics-final-continuation.md).

## Verification and limits

Focused tests discriminate constructor speed UP, zero-wheel +90-only clear,
three-contact accumulation excluding wheel3, gravity even with contact0 false,
late cap decode after graph/dot, source fsel behavior, unknown W and partial
stores. Attribute tests distinguish foreign equal-value physics/global/layout
owners and malformed-layout late reads. Body timer tests cover NaN and signed
zero behavior. Motion tests cover source guard/read/store and failure prefixes.

The original RobPark paired fixture has zero wheel contacts and actual Part6
linear velocity zero at this sample. This is not nonzero-wheel up or moving-board
evidence. Original corpus checks exact actual motion/provenance/prior copies,
zero-wheel retained up fields, current Body-to-Up admission, wrong processed
occurrence/replay refusal and genuine later-frame motion-sample rejection. Its
queue may already be empty; do not claim a nonempty queue drain from this fixture.
Nonzero-wheel state arithmetic is focused-test evidence pending a real wheel
contact fixture. Full surface motion publication, GroundAnimation and actual
Actor/input scheduling remain necessary for playable skating.

Accepted checkpoint57903/a6f2c1: strict all-target Clippy and original12 pass.
Workspace61868 passes3347 units (3104core+202formats+38app+3symbols), six docs.
Fmt/diff2324fe pass. First Clippy run found style/lint expectation issues;
corrected without numerical changes and reran strict checks. Logs are
%TEMP%/skate2-up-workspace.txt, skate2-up-clippy2.txt, skate2-up-original.txt.
Actual fixture linear XYZ are negative zero; angular XYZ are nonzero
[-0.25207967,-0.122273885,1.2659485] and preserved by publication/prior copy.
