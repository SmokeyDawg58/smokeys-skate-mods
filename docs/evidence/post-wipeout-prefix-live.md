# Retained Skeleton post-wipeout prefix

Source audit and bounded implementation, 2026-09-13. Confidence **high** for
ordinary source semantics; numeric exceptions retain the existing host
boundary. Not a full Update4 or playable-frame parity claim.

## Actual caller and ownership

`PhysicalPlayerHiLOD::Update4` executes Skeleton.UpdatePostPhysics at82e01d4c,
state virtual UpdatePostPhysics at1d60 (Ground calls the detector82d8e9b4),
then actual IsWipeoutRequested82e01d68. It retains that result, freshly reads
Processed+93c dt at1d7c, obtains the same Skeleton and passes result in r5 and
dt in f1 at1d94 to `Skeleton::UpdatePostPhysics_PostWipeoutCheck82b578b8`.
The decompiler's apparent extra boolean argument is misleading; raw proves
the single boolean. dt is not used by this particular prefix, but its caller
sampling remains mandatory. Do not substitute the prior Update5 decision.

Ground's remaining postdetector branch samples Processed+6b0 mask80000000 at
82d8e9bc..c8. The existing actual physics_state.ai_flags producer/reset is
usable. Only the set AI branch invokes UpdateAIPathFollowing82d8ea00.

New files are unregistered. Mount:

- physical_player_post_wipeout_prefix.rs as a child of
  dynamic_assembly_skeleton_normal.rs; method on the existing assembly.
- physical_player_post_wipeout_prefix_record.rs as a child of
  skeleton_physics_history.rs; mutation of the existing record only.
- physical_player_post_wipeout_prefix_wobble.rs as child `post_wipeout_update`
  of skeleton_skateboard_wobble.rs.

Root supplies PostWipeoutPrefixOwners using the same validated world pair,
same constructor.outer and constructor.wobble, same board constructor/backend,
and actual Processed collaborator. No detached owners or snapshots are created.
The current helper uses exclusive Rust borrows; native concurrent mutation
between repeated loads is outside this selected owner model. Root retains
the actual request-result/dt sampling order before calling the method.

## Error consumption and captured continuation

At578e4 bail=true skips the flag read. Otherwise sample8c4bit40000; only clear
calls UseSkateboardError82b4d8e0. Its own3dd4 flag may legitimately be true:
Ground.Update now marks it, and skeleton_postphysics_preparation_live.rs
produces actual3d50 error from physical deck minus predicted board position.
The helper reads up220 before error3d50, evaluates dot3, subtracts projected
error, then adds up*min(0,dot) using FMA. Dot reduction and VMX min's exceptional
NaN/signed-zero selection are explicit host numeric boundaries, not bit parity.

AddOffsetToPhysSkeleton82b3d7b0 adds all FOUR lanes to only transform translations
parts1..23, in ascending order. It does not modify root0, cached position,
velocity, COM, attached physical Parts or any other record. Only after all
stores does UseSkateboardError clear3dd4 atd958.

Next capture physics8e4 (air200/201), category8ec (offboard500), actual board
Body360 contact count, and current8e8 (teleport702); read actual Part6 transform
at579b0. Always call the same retained wobble.Update at579bc. Ignore its returned
active boolean: out.do_adjust controls the outer branch. On successful return
store outer3de4 tilt0 at579e0 then3de8 squish0 at579e8.

Returned PostWipeoutBeforeRootAdjustment is a continuation, not readiness.
For do_adjust=true, native579f4..57b94 MUST next rotate/translate actual root
PhysRecord0, publish real tilt/squish and replace the captured deck for IK;
that matrix graph remains unimplemented here. Actual ctor-inactive wobble
legitimately reaches the following IK condition without that branch.

## Retained wobble

SkateboardWobble.Update82b4af80 always fetches own live attribute
`4fc22a8e5b8b3676` before branching; absent field uses DefaultDataArea4. Rawafc4
captures actual activity BEFORE afc8 dereferences duration. Source constructor
82b4ae68 established inactive, time0, strength0, mirror1 on this same object.

Active branch samples currenttime then Evaluate(squish)afe8; reads strength
afec and writes squishaff4. It resamples time, evaluates tilt atb00c, reads
mirrorb010 then strengthb014, writes doAdjustb030 then tiltb038. Live debug
cvar is readb03c/b040 only on this branch; enabled path requires DebugOverlay
before its debug calls. The debug callback contract must preserve that owner
requirement; unimplemented enabled drawing should return an explicit error.
After debug, time increases by exact f32 `3c888889` (not caller dt), then an
`!(time <= duration)` comparison expires activity and resets time0 (rawb0d4
BLE skips expiry only on ordered less/equal).
The expiration call STILL produces do_adjust=true and its nonzero output.
NaN duration/time expire; NaN graph input returns its last Y at2c78.
Inactive writes doAdjustfalse, squish0, tilt0 and does not sample debug cvar.

One cvar requested/reserved202: kDebugDrawWobble; Physics_Systems_Wobble;
title `Debug Draw Wobble`; false; callbacknull; initializer82f7e0a8,
constructorcall82f7e0d4. No cvar registry/shared-file edits by this author.

PointGraphEval824e2c40 uses raw BLT at2c70 to enter the interval search, so an
unordered value or terminal X returns the final Y. A failed subsequent interval
search reaches assertion824e2ccc. Root September19 raw correction: width
guard2d10=40990030 is BO4/BI25 (!GT), including unordered. NaN width
selects upperY without arithmetic, matching the general PointGraph helper.
The earlier ordered-BLE interpretation was incorrect. Native graph min/max
metadata is not read by Evaluate.

## Mandatory remaining consumers and validation

Ordinary grounded/nonteleport continuation invokes foot IK82b55bf0 at57bd4.
It cannot be omitted or disabled: it fetches live physics_animation tuning
and uses retained IK on-deck/external-target histories. Air+contact also invokes
hand IK82b56310. IK.UpdatePostPhysics82b55398 is always called57bfc, samples
default-TRUE kDoNewPostPhysicsIK and clears12 target flags after its solver.
HeadTracking82dca850 always runs; retained invalid-target constructor state
does not excuse live layout/PID/cap/rotation-velocity updates. Finally
default-TRUE kUpdatePoseBuffer invokes82b4d970:24 global bone transforms,
reverse24 local transforms, followed by real truck/wheel/compression publication.
Those consumers remain required before declaring Update4 complete.

Focused tests added: PhysRecord offset excludes root and cached positions but
includes W; graph NaN input last-Y return and NaN-width upper-Y selection vs ordinary
interpolation. Standalone rustfmt passed. No Cargo, registration, real-corpus
execution or independent acceptance by this author; root owns those gates.

Independent review by runtime_path corrected the terminal graph guard and
wobble expiry unordered behavior above; decompiler relational spelling is not
the raw branch semantics. Corrections applied before registration.

Root follow-up source inspection (not yet implemented or independently reviewed):
active root adjustment82b579f4..57b94 copies actual PhysRecord0 before the
scalar-splat XMVectorSinCos call. Raw VMX128 operands at57aa4/aa8/aac construct
rotation columns [cos,sin,0,cos], [-sin,cos,0,-sin], [0,0,1,0] and zero
translation, using permutation bytes00010203141516170001020300010203.
57adc..57b30 compose the captured record with that rotation, retaining the
three translation FMAs for zero coefficients. Standard VMADD at57b34 then
computes adjustedY*squish+composedTranslation. SetTransform(record,0) at57b48
precedes tilt store57b5c, squish store57b64 and replacement of the captured
deck at57b70..94. Reuse the existing recovered SinCos polynomial and typed
matrix arithmetic; no canonicalization of unknown outer W. This inspection
does not authorize skipping the active branch or claim it is integrated.
