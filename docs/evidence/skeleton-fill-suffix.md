# Skeleton Fill after part publication

Fresh independent IDA/raw review, 2026-09-12. Function
`Sk8::Physics::Skeleton::FillPhysOut` is **82b58230..82b59864**.
New unregistered files `physical_player_phys_out_skeleton_fill_suffix.rs` and
its test child draft the containing suffix. They are not a mounted or validated
complete Fill implementation. No Cargo was run for this draft.

## Ordered retained sources and destinations

1. 589b8..58a74 loads actual assembly body angular velocity (+30) from
   **assembly parts23,20,16**, publishing Skeleton+120,+130,+140. These differ
   from the preceding pose query indices19/15. Native assembly part stride60
   gives body pointers+8ec,+7cc,+64c. Reuse the existing rigid-body pool
   `try_phys_out_angular_velocity` and preserve unknown packed W.
2. 58a78 guard precedes Skeleton+3d80 hook load58aa4 -> output+160. The same
   retained `SkeletonPostPhysicsRecords::hook_to_hips` owns that source.
   Collision+fc5 is captured before its sampled guard is checked, then writes
   output Skeleton+249 at58b00.
3. Physics velocity base is Skeleton1940 + State.PhysRecord610 + Velocity780
   = **26d0**. Loads are2750(part8),2710(part4),27e0(part17),2820(part21).
   Subtract the **COM velocity held in v124 from585b0**, not a new COM sample.
   Dot order is part8,4,17,21. Reciprocal-square-root estimate order is
   part4,17,21,8. Each length has two source VNMSUB/FMA refinements then
   squared*inverse and an exact squared==0 positive-zero select. Guard8b24
   remains sampled across this work; scalar stores8c7c/8ca8/8cd4/8d08 target
   left leg, right leg, left arm, right arm respectively.
4. GetEffectiveAnimToWorldTransform is called twice:58d0c before first guard,
   yielding output+0 Z; second guard is checked before second call58d6c,
   yielding the full output+1b0 matrix. Do not cache the first result.
5. 58d98..594c8 performs **21** self-equality assertions: XYZ of published
   offsets10,20,30,40,90,a0,b0, each with a fresh write guard. These reject NaN
   but accept infinity. Packed W is not tested.
6. Fresh GetPartTransform(Hips23)594d8 precedes mandatory
   SkeletonIK.FillPhysOut82b56590 at594e4. The hips matrix survives that call.
7. Lean = unsigned angle(hips.I,worldJ), stored+1f0 at59540. Project worldJ
   off hips.I using dot(J,I), ordinary I*dot, ordinary subtraction; signed
   angle(projectedJ,hips.J,hips.I), add native pi, multiply native inverse2pi,
   floor wrap, store+1f4 at595f4.
8. Fresh shoulder queries are **parts6 then10**, calls595f8/5960c. Subtract
   right.P-left.P then NormalizeSafe with actual all-zero fallback. Capture
   actual Skeleton+3df4 Reckoning.up+1b0, then consume Processed+160 BodyZ
   inside AngleBetweenVectorsOnPlane. Processed.prior_output.up220 is a distinct
   owner and must not replace this normal.
   Wrap its result into signed turns and store output+1f8 at596d4.
9. Sample write guard596d8, read Collision.compliant_just_collided596e4,
   check the sampled guard596e8, then store actual
   Collision output+d75 `mIsSkeletonInContact`, then call mandatory
   **SkeletonCollision.FillPhysOut_AudioCollisionInfo82b3efc0** at59714.
   This produces simulation-owned collision data even if sound playback is parked.
10. Read IK+bd0 then+bd4; FSEL(left-right,right,left) selects the contact
    offset, preserving unordered behavior. Fresh Global+ec plain key
    **2f1f013c99079afb**, index0/default4, yields threshold. Only ordered
    selected<threshold reads Global+ec again and its layout+50/+60 four-point
    curve, then existing PointGraphEval824e2c40. Store+20c at5980c.
11. Read actual Skeleton.want_respawn+3e42. If set, guard output, write
    State+3b `mPhysicsRequestedTeleport`=true at59844, THEN clear the same
    source byte59848. An output failure must leave the request pending.

## Required parent amendment

Current `try_publish_skeleton_com` discards its captured585b0 velocity on return.
Carry that exact value through the existing prefix/part continuation token into
`try_fill_skeleton_suffix`. Do not reread either the original owner or mutable
output to simulate a retained register. This is temporary call state, not a new
retained simulation owner. No existing files were changed by this draft.

## Plane-angle helper and numeric boundaries

Fresh raw82b3a360..a43c: read normal, second vector, first vector; dot order
normal·second, normal·first, second², first². Form ordinary projected vectors,
but the branch compares **original first²*second² > 37800000**, not projected
lengths. Ordered failure returns the native zero constant loaded at a424;
success calls existing signed-angle829b0628. Raw corrected VX128 operands
confirm a3c4 multiplies v57(first²) and v58(second²).

Existing unsigned/signed-angle, NormalizeSafe, inverse-trig and wrap helpers are
reused. Their documented host reduction/estimate/min-max/exceptional payload and
VNMSUB exact-zero limitations remain; this draft does not claim new numerical
parity. New local limb-length refinements explicitly use negative-after-rounded
fused VNMSUB. Unknown W remains typed throughout outer vector operations.

## Mandatory IK child dependencies, not replaced with no-ops

82b56590 calls HandGrabInfo82b4a270, then FeetDeckInteraction82b53bc0, then
guarded source flags IK+c70/+c71 -> output+241/+242.

HandGrabInfo reads actual IK.part_to_deck[2].P (730) then [3].P (770),
requires Global, reads geometry half-width+c60 and total-half-length+c68,
adds Global.physics_skeleton_default layout+20 margin to [width,0,length,0].
Each hand tests strict ordered bounds Y upper/lower, X upper/lower, Z upper/lower;
lower bound uses sign-bit XOR. Both booleans are computed before either guarded
output store. No missing global default is authorized.

FeetDeckInteraction requires actual IK.mAttribs vector key (full64 not yet
recovered), actual State.PhysRecord deck matrix and foot transforms15/19,
same Processed dt, same normal physics record velocities15/19 (source27c0/2800),
and mutable SAME IK.last_left/right_foot_position_deck. It constructs a rigid
inverse from deck rows, transforms feet, computes finite differences with TWO
separate dt reciprocal estimate/refinement chains, writes last positions before
late output guards, then publishes deck/world velocities and strict foot-box
tests. Live kDrawContactInfo and required DebugOverlay reads follow even though
their terminal targets are shared BLR. This child requires a separate verified
arithmetic/owner draft; the containing suffix never bypasses it.

## Mandatory audio child dependencies

82b3efc0 unconditionally reads FOUR actual Global.physics_collision_default
attributes before output guard. It publishes24 bone collision records from
same Collision.bone_data and same Skeleton State record; then eight category
records with selected part indices, slide speed/material, contact normal and
mass. It updates **same Collision contact/slide force accumulated during
wipeout**, clamps them with source FSEL, and publishes recent face/groin and
vehicle/skater/pedestrian impact data. Live kDrawAudioCollisionInfo is sampled
inside the eight-category loop and again in the suffix; enabled branches still
require actual DebugOverlay despite terminal BLR calls. This is a substantial
mandatory producer, not an empty copy or an optional sound-device callback.

## Draft tests

Five focused tests cover exact angular/velocity source order, fresh effective
queries, retained COM subtraction and unknown W, late NaN failure before IK,
mandatory audio failure retaining earlier stores, source zero-length selection,
and contact-offset NaN skipping the curve. Fixture IK/audio successes are
explicit isolated scheduling doubles, not original runtime evidence. Tests
are saved but not compiled or run; registration remains root-owned.
