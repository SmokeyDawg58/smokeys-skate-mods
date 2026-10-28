# Retained Actor animation phase and attributes

Scope: selected nonlocomotion body playback through the actual mutable
`ActorAnimationConstruction`, its body/channel tree pools, shared attributes
pool, named body ACS, and existing buffered scheduler. Implementation confidence
**high** for reviewed ordinary source behavior; no Xenon numerical parity claim.
Root integration, original corpus and independent review are separate gates.

## Phase1 opacity and holding prefix

`ActorConstruction::try_update_animation_phase1_prefix` implements
Actor825f41a8..42c0 on its actual cached physical and animation owners. Equality
with self rejects NaN dt, then UpdateSkaterOpacity825f1240 runs before either
holding-state sample. Native fade-in field Actor+238 is compared to the live
kSkaterFadeInTime target; when still fading, source queries actual PhysOut.State
collideWithWorldOnly and subtracts dt only when the timer is above half the
first sampled fade duration. Division re-reads the fade-in cvar target. Native
FSEL clamps are retained, including unordered and signed-zero behavior. Otherwise
source samples fade-out duration and uses one minus the clamped ratio while
fade-out remains active.

SetOpacity82cb75d0 is only two retained member writes, not a renderer call. It
receives the +52e0 ISkaterAnim subobject; raw+e68/+e4c are full-owner+6148/+612c.
It stores opacity and replaces player flag04000000 with mainCameraOnly. The
implemented fade branches passfalse. Only successful setter return increments
both original Actor timers. When both fading branches finish, native1368..13e0
requires actual Simulation.CameraManager, compares the returned camera Actor
identity, and samples GameCameraManager opacity on equality; that unmounted
branch returns an explicit dependency error before setting opacity or advancing
timers. It is not replaced with unconditional opacity1.

Live cvar references83047d78/83047da4 originate at initializers82f563b8/82f56418,
registration slots82fd0ff4/82fd0ff8. Both sectionRender_Skaters, exact labels
kSkaterFadeInTime/kSkaterFadeOutTime, titlesFade-in/Fade-out time (seconds),
default3f800000, GUI range0..5. Definitions/accessors and registration-order
oracle were integrated by the cvar owner after independent initializer review.

Holding samples PhysOut.State.isOffboard at825f424c then a fresh
PhysOut.OffBoard.isHoldingSkateboard at425c. Concrete SkaterAnim vtable82302d28
+d0=IsMounting82cb7668, +d4=SetIsHoldingBoard82cb7728: these are not the
decompiler's IsCurrentGrabType/SetGrabType. Native holding is
physicalHolding OR !offboard OR IsMounting, with short-circuit sampling.
IsMounting uses a big-endian halfword load atfull-owner+612c and lowbit,
i.e.player mask00010000. SetIsHoldingBoard replaces only secondflags+6130
mask80000000. The subsequent IsInLocomotion branch is sampled from the same
owner. True explicitly requires the trajectory update/future producer; false
returns to the outer caller's actual Action/Motion updates.

Tests cover source FSEL exceptional values and endpoint clamps; unknown mounting
flags are not read when the physical holding/offboard short circuit selects
true; opacity and holding preserve all other retained flags. Root owns the
original Actor prefix/cvar/physical integration gate.

## Mandatory remaining locomotion producer chain

Concrete vtable+C8=CreateLocomotionTrajectory82cac998(size1834),
+CC=UpdateLocomotionTrajectory82cb7d48(size1bd8). Caller825f4584..45ac passes
this=r3,dt=f1,attrs=r5,mgattrs=r6,hookToHips=r7,COMvelocity=r8,proxyCollision=r9,
worldOnly=r10; the decompiler's shifted pointer/bool arguments are incorrect.
Create first stores the world-only player lowbit and invokes
UpdateCollisionOffset82c8b3f0 with actual physical arguments. It consumes both
existing tree attributes and MG attributes, live anim_motion_locomotion Vault
fields, per-Actor offboard attrib curves, collision/turn/sprint cvars, and mutable
trajectory feet/state/action. It calls SetState82c8a828, SetSpeedCOMEnd82c8a708,
SetFootWorldPos82c92ab0, SetAction82c8a9a0, UpdateIntentData82c98878 and
GetFootPrediction82ca86c0. These producers must retain the existing feet and
state owners consumed by the already implemented COM/Air numerical methods.

Create unconditionally ends by deriving actual step_inputs[0], selecting jump
or run maximum step height, obtaining the actual world StepSelector, and
Launch82d82618(count1,sameinput,samefuture,worldOnly) at82cae1b0. Launch acquires
the real StepSelector batch with flagsworldOnly?1:7, Begin/AddTest/End, marks
result futures notready, then appends a query retaining input/result/batch
identity. Duplicate pending future identity asserts; fixed query capacity18
is checked. StepSelector.Update82d82490 calls CalculateResults82d820a0 in
query order and clears queries; Simulation SimUpdate1/2 call it at8289f908/
8289fd00. Next Phase1 Update first writes time-to-next-foot from
CalculateTimeToNextFoot82c8a8e8, then unconditionally requires future0.ready at
82cb7d80..7da4. Future1 readiness is additionally gated by the live assertion
cvar in its branch. Empty or guessed invalid future results cannot stand in for
that producer.

Existing query-family storage reserves StepSelector batches but does not yet
execute its geometry. Native route includes ClusteredMeshStepSelectorBatch
AddTest827be790, local job827c3600, RunStepSelectorTest827c2f00, trajectory
tests827c2910/2c10, geometry analyzer827c2808, and final
StepSelector.GetStepPos82d81278/CalculateResults82d820a0. Aggregate world
selection/reduction827bea70 must preserve actual selected world contributions.
The separate skeleton swept-line trajectory implementation does not close this
StepSelector route. Implementing the complete locomotion loop requires these
actual query results, not only the per-frame wrapper or current COM/Air helpers.

## Immediate production caller order

Concrete SkaterAnim vtable `82302d28`: AddTime +20=`82c8a558`, GetCurrAnim
+34=`82c8a498`, SetAttributes +4c=`82cb7be0`, ProcessTreeAttrs +74=`82d72238`.

* Actor UpdatePhase1 `825f41a8`: opacity, actual physical offboard/holding state,
  then if IsInLocomotion, UpdateLocomotionTrajectory `825f42f4`, AddTime `431c`,
  ProcessTreeAttrs `4340`, DispatchBodyAnim `434c`. Action update `4374` follows;
  MotionGraph ClearMgAttributes `43a0`, Motion update `43cc` follow Action.
* Actor UpdatePhase2 `825ed6b8`: SkaterAnim SetAttributes `825ed71c`, DispatchBSJob
  `724`, conditional WaitForBSJob `738`.
* Actor UpdatePhase3 `825f43e0`: when `!WasInLocomotion || !IsInLocomotion`,
  AddTime `825f448c`, ProcessTreeAttrs `44a0`. ToggleSkaterStates `44a8` always.
  If IsInLocomotion, real physical output/query/COM state feeds
  CreateLocomotionTrajectory `45ac`. Re-test the locomotion condition; when
  entering the dispatch branch, prior WasInLocomotion selects the named ACS
  Sync at `4674`, followed by DispatchBodyAnim `4680`.
* Existing manager physics boundary performs final named ACS Sync before
  SetUpPhysics/GetPhysUpdateData. Phase3 does not insert an extra final Sync.

The new phase2/phase3 methods must be mounted by the outer owner around actual
Action/Motion updates. They do not replace those controllers or the Actor phase1
physical/opacity work. A true IsInLocomotion branch stops explicitly at the
unmounted CreateLocomotionTrajectory collaborators, after its valid prefix.

## SetAttributes and retained consumption

SkaterAnim SetAttributes `82cb7be0` checks player+612c masks100000 and80000.
Only entering locomotion inserts, in order, ZPrevStride/ZNextStride=3fc00000
with sequence -1, YSpeed=bf851eb8 and ZSpeed=407eb852 with sequence0; all use
normalized=false. These are source code literals, not runtime Vault tuning.
Base SetAttributes `82d722b0` writes the actual AnimInstance/request callback
binding, resolves the current tree, calls SetAttributes, then clears the real
settable vector. A nonempty vector with NULL current tree stops at that native
dereference boundary. Empty vectors do nothing.

ChannelBlend SetAttributes `82d6f308` visits base then channel unconditionally.
AddSQT `825886a8` visits only mMotionOperand; Mirror `82c92928` visits its child.
Clip and Pose use the shared false-return leaf (`82784530`) in their vtable.
PhaseBlend `82d79528` visits all authored children before checking its own cached
name/value/normalized flag. Similarity is ordered abs(delta)<38d1b717 (constant
at8233e40c). Changed children or own attribute refresh each child coordinate via
GetAttribute flags15, then stable ordered bubble sort (`82d74bf8`). Normalized
target uses fmadds `82d79814`; lower_bound tests ordered BLT `79848`; both
clamps use source FSEL. Blend is stored before old-length validation and active
child stores. Final length uses first_length*(1-blend), then fused
second_length*blend+first_term `799b8`; SetTime preserves normalized old phase.

Supported node families do not request BlendSpace jobs. Phase2 checks the actual
retained BS data count before following DispatchBSJob `82d6b9e0`'s zero-work
branch. WaitForBSJob `82d79118` is a no-op only when the actual started flag is
false. Nonempty work or started jobs require their real executor explicitly.

## Tree attributes before Eval

ProcessTreeAttrs `82d72238` clears the actual output vector FIRST. NULL current
tree leaves it empty. Otherwise it samples the real shared/own attribute pool
and calls GetAttributes flags15. The output is copied into existing retained
`base.tree_attributes` for its existing physics consumers, including accepted
prefixes before a checked failure.

Clip `82d80808` iterates serialized attributes in source order. GetAttributeStatus
`82d6d3b0` uses the actual speed-scaled clip length and mutable time/old_time/loop
count. Static status6 bypasses filters for GetAttributes; timed attributes must
match both f&28 and f&3. A crossed loop asserts unless count1 and current<old.
Attribute Init `82d751a0` retains packed name/type/value; static endpoints=-1,
timed endpoints=normalized serialized endpoint*live length. Eval subsequently
commits old_time and clears loops, so moving this consumer after Eval is wrong.

Integrator independently reviewed raw status branches and corrected three
decompiler-induced NaN differences: no-loop d43c40990010 requires ordered
end>old, then NOT(begin>current); looping d458/d468 gives end>old OR
NOT(begin>current); d494 skips the loop assertion only for ordered current<old.
Tests include unordered end, previous time, begin and current time.

Pose vtable82044b48+30 points at shared blr825187c0. AddSQT `82588678` delegates
motion only. Mirror `82ca9a60` first visits child then mirrors all accepted output
attributes. Attribute Mirror `82ca9938` sign-XORs vector X (raw VMX128 aliases
corrected); contact payload replaces its packed bone name through the actual
hierarchy mirror table, preserving its scalar. Scalar attributes are unchanged.

PhaseBlend `82d7fab0` culls with raw blend versus cached current threshold. In
the middle branch it pushes two actual pool slots, queries child1 with caller
flags and child2 with31, then Attributes::Blend `82d7f3d0` intersects sorted
packed names, retaining left status and contact bone. Attribute MultBy
`82d6b0e8` then MultAdd `82d6b198` use separate products/addition for endpoints;
scalar/vector/contact values use right.mul_add(coefficient,left*(1-coefficient)).
Raw standard VMADD82d6b290=11606b2e gives VA0/VC12/VB13, consistent with this.

ChannelBlend `82d809f0` uses native useAttributes bit02000000, independently of
pose blend coefficient. Disabled delegates base. Enabled visits channel first;
empty channel delegates base. Otherwise it queries base and merges sorted names,
with the channel replacing equal names. No blend of attribute values occurs.

PoolStack<Attributes>::Push `82d79cc0` consumes7712 bytes, requires count<capacity
and new byte end STRICTLY below memory size. Live temporary typed attribute
vectors obey the actual retained pool's count/capacity; count is decremented only
at native successful Pops. Fixed output capacity120 is enforced per append.

ToggleSkaterStates `82ca9f70` tests attribute presence, not scalar value. Dynamic
initializers82f82138/68/80 produce exact names `animboardbackward`, `mirrored`,
`switch`. In that order, presence toggles player masks80000000/40000000, then
sets relative stance to (old stance==0). Other player state is retained.

## Validation and remaining limits

Focused tests cover live clip status/end/loop boundaries, sorted intersection
and contact scalar ownership, vector sign-XOR including signaling NaN payload,
strict scratch end capacity, and channel-over-base replacement. Root runs all
Cargo gates and selected original-data repeated-frame integration.

Unimplemented node families retain existing explicit admission failures.
Nonempty BS work, locomotion trajectory collaborator wiring, channel transition
completion/removal and buffered batch splits remain distinct boundaries. Runtime
attribute vectors retain existing decoded canonical FastString names; packed
comparisons are reconstructed from that parser-validated representation. Host
float/FMA exceptional policies and existing codec/VMX dot/reciprocal-square-root
policies require the separate deterministic reference-trace audit. No claim of
completed skating or complete Actor phase scheduling follows from these methods.
