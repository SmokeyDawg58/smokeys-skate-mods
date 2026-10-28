# DMO moving-object animation continuation

Status: source-traced changes, UNCOMPILED and UNEXECUTED.
SOLO. The full load/render/collide/grab/move goal remains
active; this is not evidence that grabbing and moving a DMO works end to end.

## Existing owners reused

The physical Pushing501 flow already fills the actual PhysOut.OffBoard movement
Z/X, force Z/X, crouch and mass. The generated graph already loads MovingObject
Vault behaviours, IsMovingObject and AngleAdjustByAnimationEndLessThan condition
payloads. Their runtime consumers were absent. The SkaterAnim constructor already
owned the moving coefficient fields; these fields have been grouped into
MovingObjectsCoefficients, not duplicated in another component. Its existing
constructor test now inspects the same flag knowledge through that block.

Existing numeric implementations reused: refine_drive_reciprocal_sqrt,
camera_atan2_pending / XMVectorATan, and wrap_turns_to_radians. The camera's
atan2 helper matches this native ratio/refinement/quadrant graph; the name does
not imply camera data is read. Host estimate seeds and exceptional floating-point
payload behavior remain the existing numeric boundary, not proven Xenon bits.

## Source and implementation

- IsMovingObject.Activation82c9b808 / raw82c9b90c..ba98: reads real OffBoard+0 Z
  and +4 X. Squared length is scalar X*X + rounded Z*Z (FMADDS), two rsqrt
  refinements and a zero selection. atan2(-X,Z) still executes at zero length,
  with one reciprocal refinement and XMVectorATan. Only length <=0.1 returns
  false early. Sector edges inclusive; wrapped ranges and raw unordered branch
  results are retained. Nine existing condition instances share this consumer.
- InitMovingObjects.Begin82ca38d8, vtable823008b8: null Create82bb3da8,
  Begin82ca38d8, Update/End825187c0 BLR. Copies ten words, zeros seven floats
  and both direction words, clears only flag bit0x80. Existing SkaterAnim block
  is accessed through its same-entity interface and Get/reset/Set. Lower flag
  bits retain their knownness and values; no whole-component reset.
- PDB MovingObjectsCoefficients size0x28: floats lastFullAngle/lastAngle/
  lastSpeed/lastStrength/lastRotation/lastCrouch/blendTargetAngle at00..18,
  currentDirection1c, lastDirection20, handledPushPullAngleChange bit7 at24.
  Get82cb7768 returns this block, Set82cb7770 copies all ten words.
- MovingObject.Create82cb3948 allocates0x30/alignment16 and constructs the same
  coefficient block at+8, preserving null on allocation failure. Initialize
  82cb3a38 copies the current SkaterAnim block. Begin82cb3b40 copies current
  direction to last, sets the authored direction and clears bit0x80; resolves
  Physical and SkaterAnim, then sets mirror-next/offboard together according
  to the native left/right family and GetFootForward82cb78a0. Still clears both.
  SetMirrorNextAnim82c8b868 writes only player flags0x00600000, which the existing
  animation tree builder consumes. GetFootForward was extracted from the existing
  breadcrumb read so both consumers share it; the separate fakie modifier stays.
  End is the native BLR. Update82ca3a30 is now connected as detailed below.
- AngleAdjustByAnimationEndLessThan.Activation82c9db98: resolves Physical,
  Animatable, SkaterAnim; gets coefficients then base animation time and length.
  Remaining frames=(length-time)*bits426fffff, FSEL minimum1; actual amount is
  abs(wrapped(targetAngle-lastFullAngle))/frames, strict < authored max degrees
  times bits3c8efa35. Raw82c9de8c..98 contains the angle multiplication that
  HexRays drops! Shared floor-wrap retains the native +PI half-turn convention.

## Complete subtree inventory and dispatch

MovingObject104 has 91 states including its descendants, not just its26 direct
children. Four native update thunks cover them:

| Thunk | States | Added callbacks after the shared prefix |
| --- | ---: | --- |
| 828ef5f0 | 1 | five AttachIntent, CreateAttribute, InitMovingObjects BLR |
| 828f0640 | 32 | none; branch to828ef5f0 |
| 828f0648 | 26 | MovingObject, PlayAnimation |
| 828f1dd8 | 32 | MovingObject |

The prefix calls OffBoard828eddc8 once; it runs Motion0 then ten slots (eight
AttachIntent, ToggleBoard, CreateAttribute). MovingObject's seven slots follow
at positions14..20. Every generated state's actual ancestry/behaviour declaration
was checked for the matching count and class sequence before adding its exact
active-set list to generated_motion_runtime_moving_states.rs. No recursive
ancestor updates are added; callbacks still run once in the packed order.
StateBehaviourDirect_MovingObject_Update82ca6a78 is a simple Update82ca3a30 thunk.

All relevant expression cases5,8..21,278 were extracted independently from
Motion Pre8290aa50 and Sustain8291e218 with tools/ida_graph_switch_paths.py.
The exact paths are in dmo-motion-expression-paths-2026-09-28.json. Newly connected
cases preserve condition order and Sustain's distinct constant/default cases.
Static Python source comparison: **32 matched, 0 unported, 0 mismatches** for
these expressions. This checks decision expressions only, not gameplay.

## Validation and next work

Scoped rustfmt on new modules, condition file and the edited collision frame;
syntax-only rustfmt parsing on integration files. No Cargo compilation, unit
test execution or live game. Added unexecuted tests for sector edges/wrap/NaN,
atan sign/zero denominator, reset flag preservation and animation-end angle budget.

## Full MovingObject.Update continuation

`actor_motion_moving_objects.rs` now consumes the real PhysOut.OffBoard X/Z
movement, X/Z force and crouch. `actor_motion_moving_objects_math.rs` retains
the source arithmetic and branch graphs; it is not another physics producer.

- 82ca3c80..3e10: squared movement uses rounded products plus addition, unlike
  IsMovingObject's scalar FMADDS. Squared speed <= bits38d1b717 selects zero
  speed/strength. Otherwise speed uses rsqrt plus two refinements, force is
  projected onto movement using dot/squared length, and only nonnegative dot
  contributes its projected magnitude. Unordered branches retained.
- Graph key c3a15e5de438066f reconstructed at82ca3e28..40 from lis/ori/insrdi.
  PointGraphData8 has X at+0x10 and Y at+0x30, with DefaultDataArea0x50 on a
  missing pointer. Reuses existing PointGraph<8>::evaluate, not another curve
  evaluator. Raw source offsets are authoritative; other graph layouts differ.
  Read-only inspection of the mounted game's db.big/skater.vlt+bin confirms
  this attribute is80 bytes in anim_motion/default, BIN offset216672. X samples
  are0,390.879486,791.530884,1089.577026,1397.394043,1793.160034,2262.215088,
  2555.375; Y samples0,.157142907,.328571409,.467857212,.614285707,.807142913,1,1.
  Both the actual asset and IDA therefore support the16-byte header offsets.
- Strength clamped to previous +/- live delta and stored before subsequent
  cvar reads. Raw-speed, crouch and speed target pointers captured together
  at82ca3ee4..3f00, then values read in that order. Speed target saturates
  rawSpeed/rawMaximum, crouch and speed retain their native FSEL bounds.
- Changing between push1..6 and pull7..12 applies a wrapped PI offset once,
  gated by the retained bit0x80. Transition directions3..6/9..12 distribute
  wrapped target-current over remaining animation frames, optionally including
  the next full cycle. Uses actual base animation GetTime/GetLength and the
  bits426fffff multiplier with minimum1. This Update uses division; the separate
  angle condition uses reciprocal followed by multiplication.
- Ordinary directions use the existing atan2 numeric graph on (-X,Z), even
  for zero movement, and the existing kMOAngleMaxDelta degrees cvar.
- ClampAngle82c8dd98 selects circularly nearest sector endpoint; ties select
  end, unordered distance comparison selects start (raw bgelr, not HexRays '<').
  Push-left[-PI/2,0], push-right[0,PI/2], pull-left[PI,-PI/2] wrapped,
  pull-right[PI/2,PI]. Normalized angle follows original side-specific formulas.
  Still retains its previous normalized angle. lastRotation is unchanged.
- Final writes: authored angle, strength, smoothed crouch, global `Crouch`
  (unsmoothed), global `Speed` (smoothed), then the complete coefficient block
  back to SkaterAnim. All five use existing AddSettableAtt(false,-1). Native
  Update does not write the retained xSpeed/zSpeed/straferotation names.

Native constants: rawAngleMax830207a4=3fc90fdb;
forwardRightEnd830207a8/backRightStart830207ac=3fc90fdb;
backRightEnd830207b0/backLeftStart830207b4=40490fdb;
backLeftEnd830207b8/forwardLeftStart830207bc=bfc90fdb;
forwardRightStart822fb8c8/forwardLeftEnd822fb8cc=0.

Cvars added, null callbacks checked from raw r10=0:

| Index | Reference | Initializer/table | Default |
| --- | --- | --- | --- |
| 890 Strength Max Delta | 830a3754 | 82f86770 / 82fd4f34 | bits3c23d70a |
| 891 Speed Max Delta | 830a3e6c | 82f86830 / 82fd4f3c | bits3cf5c28f |
| 892 Raw Speed Max | 830a345c | 82f86890 / 82fd4f40 | 2.5 |
| 893 Crouch Max Delta | 830a4e78 | 82f868f0 / 82fd4f44 | bits3cf5c28f |

Existing angle index35/default2 is reused between strength and speed in native
registration order. Static check:894 entries, unique label expressions, full
registration permutation0..893, correct five MovingObject indices. The existing
registration-order test's expected title list was updated; test NOT executed.

Added unexecuted tests for projected force, opposing force, zero movement,
circular clamp/NaN, FSEL inverted bounds, and all12 direction families. Five
edited Rust files parsed with rustfmt (including lifecycle and cvar integration).
This is not type checking, compilation or runtime validation.

Camera condition audit: SkaterIsMovingObject.Activation82e34660 compares the
actual PhysOut physics state to501. Existing camera_graph_conditions.rs already
does exactly this; no duplicate condition added. Camera shot runtime still
requires gameplay evidence. Remaining DMO renderer work is documented separately
in dmo-render-2026-09-28.md. Full goal remains active.

Additional graph inventory: the91-state MovingObject subtree owns211 distinct
transitions (13..223). All211 declare zero transition hooks, so none needs a
new PendingTransitionHook exception. Guards are5,9,11,278 and the null expression;
only external targets are Locomotion33 and Mount34. Existing shared lifecycle
End handles MovingObject as the native BLR. This is static graph coverage, not a
claim that every reachable neighbouring gameplay state is implemented.
