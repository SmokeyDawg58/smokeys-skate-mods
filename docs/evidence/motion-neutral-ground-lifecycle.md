# Neutral-ground Motion1170 lifecycle mounting packet

2026-09-19. Fresh connected IDA `sk82_na_m.xex` raw vtables, direct update
thunks and targeted decompilation. **High** confidence for the small timer/
flag draft and table below; larger callback algorithms remain unimplemented.

## Small first batch and existing work

Reuse `actor_motion_attach_intent.rs`, `motion_attach_intent.rs` and its
tests. Their source interfaces are captured separately in native order;
missing intents do not publish an attribute. Applicable occurrences17..27,
91,92,1924. Do not recreate this kernel. Existing `motion_setting_body_tilt.rs`
plus Actor adapter/tests cover its instance/update, but BodyTiltTuning is a
late collaborator without an actual global/cvar provider. It is not ready
for active ApplyingBodyTilt merely because the inactive branch can return.

New unique `actor_motion_grounded_timers.rs`, intended child of
actor_construction::motion_runtime_session, implements three concrete
instance-free callbacks through the actual cached ISkaterMotionGraph,
publication check, UCOM occurrence identity and retained graph state:

* UpdateTimeSinceKickturn31 Update82ca1290 calls virtual825fe120:
  `fadds dt, old` at+0c98, no clamp. Pass actual retained controller dt.
* UpdateManualOutTimer724 Update82ca53f8 calls IsManualing825fe090 first
  (Motion+ca4 flag10000000). Only if false, update825fe1b0 subtracts dt from
  Motion+ca0 and uses fsel to keep the nonnegative result or +0. Unordered
  selects +0; -0 is retained. End82ca5500 calls ClearManualOutTimer825fe160,
  which stores +0 regardless of manual flag. Unknown flags remain errors.
* ApplyingBodyTilt1926 Begin82c9e098 / End82c9e190 invoke825fdfe0 with1/0,
  updating only Motion+ca4 flag80000000 and its reconstruction-known bit.

The new countdown test covers ordinary subtraction, clamp, negative dt,
negative zero, NaN inputs and infinity-minus-infinity. No numeric tuning is
baked. New helper only uses these verified masks and source immediate zero.

## Exact instance and callback slots

Vtable offsets2c/30/34/38 are CreateInstance/Begin/Update/End. `NULL` below
means shared82bb3da8, raw li0/stw result/blr; `BLR` means825187c0, raw4e800020.
These are verified source no-ops, not permission to skip other callbacks.

|Occurrence/type|Vtable|Create|Begin|Update|End|
|---|---|---|---|---|---|
|17..27 AttachIntent|82303360|NULL|BLR|82cc9f30|BLR|
|28 SettingBodyTilt|82302088|82caf1e0|BLR|82c9e288|BLR|
|29 PowerSliding|823006d8|82ca3088|BLR|82cc7478|BLR|
|30 BodySpin|8230001c|82c9eb10|BLR|82cc4600|BLR|
|31 UpdateTimeSinceKickturn|823003ec|NULL|BLR|82ca1290|BLR|
|32 StoreLandingData|823004e8|82ca1f78|BLR|82ca1d00|BLR|
|90 ForcePhysics|8230069c|NULL|82ca2f98|BLR|BLR|
|177/1923 SetScoreAugmentation|82301428|NULL|BLR|82ca8228|BLR|
|178 EndShimmy|823010d0|NULL|82c964c8|BLR|BLR|
|722 SetDistComToBoard|8230244c|NULL|82cb1208|BLR|BLR|
|723 Crouching|82302838|82cb3e88|82cb4240|82cca698|BLR|
|724 UpdateManualOutTimer|82300b38|NULL|BLR|82ca53f8|82ca5500|
|1413 SetTurning|823025b4|82cb27b8|82cb2798|82cc8408|BLR|
|1925 Pumping|82300058|82c9f388|BLR|82c9ecc0|82c9f2b0|
|1926 ApplyingBodyTilt|822fff20|NULL|82c9e098|BLR|82c9e190|
|1927 CreateAttribute|82301d40|NULL|82caba88|82ca92d8|82cabaa0|

2172 PlayAnimation already has a lifecycle implementation: reuse its actual
authored payload and retained animation/channel owner; do not replace it.

Instance constructor allocations all request alignment10 hex, preserve
allocation failure as absent instance, then use source fields:

* SettingBodyTilt: size14 hex, angle/delta +8/+c =0, byte+10=false.
* PowerSliding: size18, floats+8/+c/+10=0; only upper three bits of word+14
  are cleared. Low29 bits are not initialized by this constructor.
* BodySpin: size20, integer+8=0, floats+c/+10/+14/+18/+1c=0.
* StoreLandingData: size10, float+8=0, byte+c=false. Its CreateInstance is
  merged symbol DisableTricks82ca1f78; the StoreLandingData vtable proves use.
* Pumping: size18, float+8/+c/+14=0, signed integer+10=-1.
* SetTurning: size14, floats+8/+c=0, integer+10=2. Begin82cb2798 resets
  those same three values; it is not BLR.
* Crouching: size38, floats+8/+c/+10/+14/+1c/+24/+28/+2c/+30=0,
  byte+18=false, integer+20=0, float+34 reads live
  kTimeToConsiderPumpingMode only after successful allocation.

## Source update sequence for1170

Raw update thunks establish root-to-leaf callback order (some parent bodies
are inlined into descendant thunks). Execute each occurrence exactly once:

1. Root0 existing occurrences0..3.
2. OnBoard3, thunk828edf40: AttachIntent17..27, SettingBodyTilt28,
   PowerSliding29, BodySpin30, UpdateTimeSinceKickturn31, StoreLandingData32.
3. OnGround24,828ee770: ForcePhysics90 BLR Update, AttachIntent91 then92.
4. RidingIdle89:177 SetScoreAugmentation,178 EndShimmy BLR Update.
5. Riding277,828f04c8:722 SetDistComToBoard BLR,723 Crouching,724 timer.
6. Turning719:1413 SetTurning.
7. Idle1025,828f2d28:1923 score,1924 AttachIntent,1925 Pumping,
   1926 ApplyingBodyTilt BLR,1927 CreateAttribute.
8. Default1170,828f39b8 calls Idle1025 then2172 PlayAnimation.

Entry must still run real non-BLR Begin callbacks before the first update;
in particular1926 Begin enables tilt before ancestor28 Update. Therefore
neutral input cannot justify skipping active SettingBodyTilt tuning.
Source order is not equivalent to running only the newly selected leaf.

## Larger unimplemented dependencies

PowerSliding82cc7478 requires actual IPhysical, ISkaterAnim, IMotionGraph and
ISkaterMotionGraph. It reads both slide intents, updates the instance's
ground-transition/history bits, and on actual Ground reads live global
anim_motion_power_slide scalars/graphs even when no slide intent is present.
BodySpin82cc4600 resolves seven actual interfaces, live
kEnableLandingOnBoardBodySpin, physics filtered/state outputs, BodySpin
intent, mirrored animation flag and global anim_motion_body_spin graph plus
four scalar fields before its state-specific channel updates. Neutral state0
still ends FsBodySpinSkaterAnim/BsBodySpinSkaterAnim channels and stores state
and input. Neither callback can be represented by a neutral no-op.

StoreLandingData82ca1d00 resolves IPhysical then ISkaterMotionGraph. On first
Air frame it resets stored landing speed; records was_air; reads actual
PhysOut Skater section+24 hex, vectors+10/+50, takes native vmsum3fp dot;
when dot<0 AND dot<previous, publishes abs(dot); always stores new dot.
Raw82ca1f30=40980028 and1f3c=4098001c are NOT-LT skip branches, so NaN
does not publish but is stored. This needs source-accurate dot accumulation
and the actual output/landing-speed setter before mounting.

The remaining Begin/Update bodies in the table need separate bounded
implementation; the table is their precise mounting contract, not a claim
they have been recovered end-to-end. Keep pending callback failures until
each actual owner/provider route exists. Constructor-only admission must
not make missing callbacks silently succeed.

Validation: exact Test-Path checks before new files, rustfmt only; root owns
registration, Cargo, actual corpus execution and independent source review.

## BodyTilt live provider follow-up (unregistered draft)

`actor_motion_body_tilt_attributes.rs` now supplies the previously missing
BodyTiltTuning collaborator. Mount as input_cvar_runtime::body_tilt; existing
Actor adapter is expected at motion_runtime_session::setting_body_tilt.
No existing draft was edited. This supersedes the earlier provider-missing
status, but registration and actual corpus validation remain outstanding.

Global ctor alloc828b2f2c requests10 hex, constructs generated anim_motion
at2f50 with collection **ddcdc2a88e43a628**, stores Global+148 at2f5c.
Generated ctor828b1590 raw15a4..15bc binds class **90c3decf0105631e**;
FindCollection15c0, Instance15d0, missing cached layout substitutes
DefaultDataArea(5c0)15e4. The draft binds/validates once in shared
ActionRuntimeServices, exactly beside existing +128/+13c/+190 wrappers.
Store `body_tilt: BodyTiltMotionAttributes`, use try_bind on the SAME runtime
Vault Arc. Runtime checks its Vault identity against actual physical inputs.
No per-Actor private global replacement and no eager scalar tuning snapshot.

At each active update the graph is reread from the retained cached layout,
offset560 hex. Native raw82c9e604/e610 loads Global+148/layout+4; e614 selects
560, e618/e61c choose y+20/x+10, e620 calls PointGraphEval with4 points.
Thus provider x offsets570..57c/y580..58c, using existing PointGraph<4>.
Null layout alone uses the generated all-zero DefaultDataArea; absent runtime
service is an error rather than a null-bound generated wrapper.

Separate generic node/inheritance scalar reads, index0 and real missing-field
DefaultDataArea4 fallback, are freshly decoded from accessor raw instructions:

|Value|Accessor|Full attribute key|
|---|---|---|
|Air acceleration|82c8abf8|e7c877e220d6fbc0|
|Air velocity|82c8ac40|52855de42244601b|
|Ground acceleration|82c8ac88|68bf98bc935799b9|
|Ground velocity|82c8acd0|d2b50fd12925415d|

Source sampling sequence is actual interfaces, applying flag, instance edge
reset/was_applying store, actual animation output pointer, mirror, tilt X,
abs body-spin speed, graph, filtered state, selected acceleration, selected
velocity, integration stores, use-tilt ref/value, optionally set-tilt
ref/value, optionally override ref/value, publish. Provider view construction
does not validate or read either attributes/cvars, preserving late failures.
The integrator passes Option service/ref borrows and the actual physical
owner's input Vault Arc into BodyTiltRuntime, then passes it to the existing
motion_runtime_setting_body_tilt method. Preserve kernel stores on later
cvar/publication failures. Inactive update must not ask for any tuning.

Three root-coordinated new cvar indices (225/226 reserved for physical debug):

|Index|Label|Title|Default|Reference|Initializer/table slot|
|---|---|---|---|---|---|
|227|kUseTilt|Use Body Tilt|Boolean true|830a4de0|82f85a70 /82fd4ea0|
|228|kSetTilt|Hard Code Tilt|Boolean false|830a33ec|82f85ac0 /82fd4ea4|
|229|kTiltX|TiltX|Float0, GUI min-1 max1|830a3dec|82f85b10 /82fd4ea8|

All three section strings are **Animation_Skater_BodyTilt**; no update hook.
Register in native table order4ea0/4ea4/4ea8, before later table64a4/64a8.
Update definition count, registration permutation and GUI enumeration test
expectations in root. The provider's bool reads use the existing live
try_current_boolean; float reads current target under the existing locks.
It never caches captured targets across updates or samples an unneeded
override. GUI bounds are metadata, not permission to clamp runtime reads.

Next original Update blocker after active tilt is PowerSliding29. It is the
smaller next callback than BodySpin30, but still requires the actual shared
Global.anim_motion_power_slide generated wrapper, slide-control owner,
intent history bits, graph fields and live debug ref. BodySpin additionally
requires real channel commands; keep both explicit PendingCallback until
their complete active neutral route exists.

Follow-up validation: rustfmt check only. Existing four SettingBodyTilt kernel
tests cover edge-reset-before-failure, integrated state before cvar failure,
disabled publication after integration and fsel unusual values. New provider
requires root's real-Vault/cvar corpus gate; no standalone test claims that
the actual active actor route has already executed.

## September19 integration in progress

Root mounted the reviewed timers, BodyTilt, PowerSliding, BodySpin, landing
history, ForcePhysics, score augmentation, shimmy end, COM distance, Crouching,
SetTurning, Pumping and CreateAttribute lifecycles. Actual retained providers
bind once to the same physical Vault; cvars227..238 remain live at source reads.
ForcePhysics receives the already-borrowed real world and published Skeleton
handle, validates membership/owner, then writes the real Skeleton mode. It does
not reacquire the world mutex or keep a duplicate mode on Actor.

Fresh independent generated1170 proof gives update thunk828f39b8 and exactly
35 flattened slots: 0,1,2,3,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,
90,91,92,177,178,722,723,724,1413,1923,1924,1925,1926,1927,2172.
Ancestor thunks inline these slots; root executes this list once after all Enter
callbacks. State and thunk address and exact active set are checked together.
Existing Default6 admission remains separate; no other state is inferred.

The shared channel helper now accepts source blend lengths and last-frame flag.
Gesture retains .3/.3/false; BodySpin uses .2/.3/false and identical animation/
channel names; Pumping uses live lengths and last=true. Raw Init82d6d500 proves
last-frame40000000 and ordered blend_out>0 for08000000 (zero/negative/NaN false).
Allocation, tree, Init, free-list and dictionary ordering remain shared.
Original-data regression uses the disposable Actor after physical teardown.

Compiler integration corrected Crouching Begin's output member to actual
Animation+48 mCentreOfMassToDeck. Independent review also corrected SetTurning
smoothing FMA operand order and added a bitwise regression. A selected corpus
continuation consumes the actual output after all seven Physics and five
Animation callbacks. Workspace/original-data gates are still pending for this
batch; this is not yet an accepted sustained frame or playable build.
