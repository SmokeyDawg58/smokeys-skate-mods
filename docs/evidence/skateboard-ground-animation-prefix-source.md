# GroundAnimation prefix: actual remaining integration

2026-09-08, original debug XEX/PDB inspected read-only. Confidence
**exact** for the raw caller ordering and small leaves below; **high** for the
dependency plan. This is a source audit, not a new runtime parity claim. No Cargo
or implementation was run for this packet.

## Actual caller and retained owners

`Sk8::Physics::PhysState_GroundAnimation::Update` at `82d98d68..82d98dd0`
first clears state `mJumpedThisFrame+40` at `8d80`. It then calls
`Reckoning::UpdateGroundStatesFixedHeading` (`82ddd468`) at `8d94`, passing
the SAME processed owner `+10` ground normal `+1d0` and dynamic up `+210`.
At `8da4` it calls `Skeleton::UpdateAnimatedSkateboard` with that owner and
fast blend true. Only after this returns does it set skeleton
`mAddSkateboardErrorToSkeleton+3dd4=true` (`8db4`) and call
`UpdateSkateboard` (`8db8`). PDB confirms the skeleton field name.

The actual animated-board function is **82b5bc58**, not `82d9f7c8` (the latter
is the PhysicsAir callsite currently cited by a collaborator method in
`physics.rs`). It validates retained matrices, calls `UpdateRootTransforms`
at `82b5be90`, composes the animation board pose using retained animation
volumes/root transforms and predicted board position, publishes processed
matrix `+0..+30` at `82b5c32c..338`, sets flag `0x80000` at `c340`, and stores
the skeleton skate-root matrix. It obtains the actual physical board at
`c364`, calls `Skateboard::ApplyBoardAnimation` at `c378`, stores returned
physics-board matrix, then calls **GeneralUpdate at c3a4**. Finally it resets
the next-trajectory transform to identity axes/zero origin.

Therefore the existing retained GeneralUpdate/IK/Contact/DriveFrames path
belongs inside this actual prefix; separately running it again would duplicate
the source update. `UpdateRootTransforms`, `ApplyBoardAnimation`, and the
preceding fixed-heading Reckoning operation remain distinct integrations.
Fixed-heading is substantial: BodySpin update, global and own Reckoning
curves/attributes, ground-normal update, retained ground filters and up-vector
state. A constant heading/normal is not an implementation of it.

## Steering executes even for zero target

`PhysState_GroundAnimation::UpdateSkateboard` (`82d8a7b0`) calls
`Skateboard::UpdateSteeringTilt(0.0)` at `82d8a7d8`, then material application
at `a7e0`, then ground-jump calculation at `a7ec`. There is no jump/steering
gate before these first two calls.

`UpdateSteeringTilt`, `82b29de0..82b29edc`, resolves global attributes
`physics_steering_default` via global owner `+b4`. Raw key construction at
`29e2c..44` gives **c3249a9df26600c9**. Lookup at `e48`, missing
`DefaultDataArea(4)` at `e58`, and scalar load at `e60` precede:

1. `weightedTarget = attr * target` (`fmuls e64`).
2. Capture processed pointer `+110` (`e68`), read old board `mDeckTilt+e0`
   (`e6c`), subtract `1.0-attr` (`fsubs e74`).
3. Fused `(1-attr)*oldTilt + weightedTarget` (`fmadds e78`), store tilt (`e7c`).
4. Read flags `processed+8c4` (`e80`), extracting board-flipped bit20, then
   read `processed+8c8` (`e88`).
5. Nonflipped sends bit28 to native r5/front timer and bit27 to r6/back timer;
   flipped swaps those. The bools are in r5/r6 because the float occupies the
   ABI argument slot; decompiler pseudo-signatures here are misleading.
6. Read actual body `board+0c`, call `SetForceTurnAngle` at `29ebc`.

`SkateboardBody::SetForceTurnAngle(float,bool,bool)`, `82b24cf0..82b24e10`,
updates retained front timer `+2084` and angle `+1df4`, then back timer `+2088`
and angle `+1df8`. Contact true: timer not ordered greater than threshold increments by the
fixed update time, clamps timer-scaled blend using the explicit two `fsel`
operations, then FMA-blends old angle and requested angle; otherwise it writes
requested angle directly. Raw branches `24d34/24da8` are BO4 BI25
(GT-not-set), so NaN timer follows the blend branch. Its two fsel operations
produce blend1, but final FMA still includes zero times retained angle (an
infinite old angle therefore yields NaN). Contact
false: it damps the old angle only when old timer equals zero (signed zeros
equal; NaN does not), then clears the timer. Both back branches set
`mForceTurn+1df0=true`; this also happens when requested tilt is zero.

Raw constants: threshold `3e28f5c3`, reciprocal blend time `40c1f07c`, airborne
damping `3f7ba5e3`, one `3f800000`, zero `00000000`; fixed timestep comes from
`kUpdateTime_53`. Preserve source `fsel` and single-precision/FMA semantics,
not generic clamp rewriting for unusual values. Front updates precede back;
back-contact true publishes force-turn before angle work (`24da4`), false
publishes it after resetting back timer (`24e08`).

**Required downstream continuation:** `skateboard_truck_drives.rs` is explicitly
the fresh force-turn-false profile. Its source packet documents the separate
true-force HARD_DRIVE branch as not covered. Connecting steering while retaining
the old false-force drive behavior would change the actual subsequent physics,
even for zero target. Reuse retained truck slots, numeric services and frame
setters; recover and integrate the live force-turn branch and native scheduling.

## Selected material is an actual four-part mutation

`SetWheelMaterialForSurface`, `82b29fb8..82b29ff8`, loads body `+0c`, processed
`+110`, assembly `body+18`, and selected material `processed+908`. It assigns
the SAME material pointer to actual parts 0,1,2,3 at part `+50`, stride `60`:
stores `29fcc`, `29fd8`, `29fe4`, `29ff0`. It reloads assembly/part list between
stores. No attribute lookup, material reconstruction, or all-seven-part write
occurs here.

Reuse `skateboard_processed_surface.rs` and its five source-constructed material
triples and selected projection, fed by actual completed board contact selection
and the separate next-frame publication boundary. The backend currently retains
authored part payload material; this audit found no implemented runtime wheel
material setter. Add the actual mutable material authority consumed by contact
generation, retaining source part order and earlier writes on a later failure.
A value copy is only equivalent across an explicitly immutable material-owner
lifetime; it must not hide source pointer ownership/reload behavior.

## Jump decision, existing calculation reuse, and missing handoff

`Toolkit_CalcGroundJump`, `82b315a8..82b3212c`, reads the live
`kMaxJumpError` target scalar at **31600 before clearing result storage**. It
then stores do-jump=false (`31604`), COM height +0 (`31608`), and full four-lane
zero new velocity (`3160c`). Only then does it read processed `+110`, flags
`+8c4` (`31614`), and test mask **00400000**, PDB `mStartJump`, at `31620`.
Inactive goes to `3210c`, redundantly stores false and returns. No jump Vault
lookup or `mJumpDV` mutation occurs on that branch. Thus a failing cvar read
must retain prior output; a missing start-jump producer cannot be silently
interpreted as false.

Active jump uses processed jump normal `+240`, prepared velocity `+2c0`, current
velocity `+190`, speed, COM/ground position, Reckoning up, gravity, jump strength
`+950`, adjustment X/Z `+954/+958`, body axis, global physics-jump curves and
nine dynamic attributes. `physics-state-machine.md` already records the curves,
keys and math; **reuse** `physics.rs::calculate_ground_jump_tangential_velocity`,
`calculate_ground_jump_vertical`, and `calculate_ground_jump`. Do not create a
second guessed solver. These are XYZ domain calculations supplied tuning/curves
eagerly, not the actual staged mutable provider. Their `.clamp`, comparisons,
host normalization and nonfused expressions require raw unusual-value/numeric
review before advertising full native parity. Preserve unknown vector W until
actual source derivation, not the compatibility +0 W used by the older handoff.

Active source publishes do-jump at `31dd0`, velocity at `31dfc`, performs the
remaining compensation/adjustment and stores board `mJumpDV+0b0` at `320d0`.
Late draw-jump gate at `320f0` can reach overlay assertion after these mutations.
The exact output COM-height write graph beyond its initial zero is a remaining
focused audit item; do not assume the domain helper's calculated height equals
the native result field simply because the quantities share a name.

Caller publishes result full velocity to state+20 (`a7f8`), COM height+30
(`a800`), do-jump+34 (`a80c`), then tests it (`a814`). False enters the already
implemented six-force tail at `a8a0`. True calls actual SetVelocity (`a82c`),
sets jumped-this-frame+40 (`a83c`), sets linear drag zero (`a840`), constructs
TrajectoryLaunchInfo (`a848`), obtains Skeleton launch info (`a854`), writes
playerJumped (`a860`) then full start velocity (`a86c`), calls selector Launch
(`a870`) and Update (`a878`, return ignored), copies selector+600 to state+50
(`a888..88c`), and returns without the six-force calculations.

`GroundAnimationState`/`prepare_ground_animation_jump` (`physics.rs` around
2090/2125) retain useful named state/handoff shapes. They currently accept a
precomputed solution and preobtained skeleton launch record, set jumped state
before any actual SetVelocity callback, and manufacture compatibility W=+0.
Consequently they cannot stand in for this failure-prefix-preserving caller.
Adapt/reuse their semantic types where possible while moving actual reads and
writes to native order. `update_ground_animation_non_jump` around1775 is the
older callback-only queue wrapper; reuse the now-integrated staged six-force
runtime instead of adding another calculation path.

`ProcessedPhysicsInput` around3174 is the semantic reset/animation handoff
model, not the full native ProcessedPhysIn layout or actual Actor scheduler.
It does not by itself certify the live wheel flags, prepared jump velocity,
selected surface/material, or Reckoning/current-body inputs. The current actual
toolkit projection and contact publisher are the producers to extend and bind;
animation intent must continue through the real animation/raw attribute path.

## Shortest completion route and validation

1. Retain actual steering timers/angles/force-turn and bind the global steering
   scalar at the source read site; implement its actual truck-drive consumer.
2. Apply the already-produced selected material to four real wheel material
   owners and verify a subsequent real contact consumes it.
3. Add the ground-jump staged decision/provider around the existing math, with
   actual start-jump production, then real SetVelocity/launch-info/selector
   handoff. Preserve inactive and active branches, not a forced inactive gate.
4. Connect the outer fixed-heading Reckoning and animated-board route on the
   SAME retained owner, nesting existing GeneralUpdate once. Only then call the
   whole GroundAnimation update integrated; Actor/input scheduling remains an
   additional producer/caller milestone.

Focused checks should cover nonzero retained steering histories, bool swap,
zero target still force-turn true, dynamic wheel material reaching the solver,
max-error read failure before result clear, inactive no jump-attribute reads,
and failures after velocity/jumped/drag/launch stores retaining their prefix.
Use original tuning and actual physical owners for the corpus. Deterministic
reference per-tick traces are still required for final skating parity.

Raw SHA256 evidence (half-open function ranges):

### Truck dynamics implementation supplement

Fresh raw `SetTruckDriveDynamics82b2cb08..82b2d0e8` hash remains
`8146f9dc1f0e3007db282fc44c2c18b4d4ed25817cc947c4f72746d4ddb8533e`.
The initial global deck key is `103fdc19514d4246`; byte read precedes drive
count test. For each drive it then samples retained force-turn.
True-deck/forced reads global `physicstrucks_drives_default+ a0` three pointer
keys in order: strength `b926e54f968134e0` (cc08), damping
`0b99673c3f054766` (cc60), spring `2374d23fa26b3a7b` (ccb8). All use
missing DefaultDataArea(4). Dereference order is spring (cccc), strength
(ccd0), damping (ccd8); spring multiplication between those latter two reads
uses raw `426fffff` (59.999996). Stores type HARD=2 (ccdc), spring (cce0),
damping (cce4), then common strength times `4560fffe` (ce70).
False-deck/forced reads only that same forced strength key (cf10), then stores
spring literal `45610000` (cf2c), damping literal `4279999a` (cf30), scaled
strength (cf34), and only then type SOFT=1 (d0c0). These literals are source
constants, not replacement Vault tuning.

Both unforced branches use existing four normal keys in the existing lookup
order. True-deck dereferences speed, multiplier, base, strength, spring and
stores type/spring/damping/strength. False-deck instead dereferences strength
(d094), speed (d098), multiplier (d0a0), base (d0a4), spring (d0a8), then
stores spring/damping/strength/type. Damping is a single FMA. Native count is
reloaded at each loop end. Host exclusive owner cannot change count from a
callback; this retained-count boundary remains explicit.

Contact setter fixed timestep was independently read from original
`kUpdateTime_53=8225c488`, word `3c888889`. The existing fresh constructor
remains guarded against false deck profile; the runtime dynamics method can
support both profiles without changing fresh construction admission behavior.

### Live truck publication occurs after the solve

Code xrefs give one live caller for each truck update: the END of
`SkateboardBody::UpdatePostPhysics82b36a58`, **dynamics at82b374c0 THEN frames
at82b374c8**. The other callers are construction (`37bd8/37be0` and
`37ed4/37edc`). Do not move these calls to immediately after steering or
before queued-force application merely because the leaf APIs are available.

Before these calls, body PostPhysics has performed actual collision acquisition,
winner processing, wheel-line normal fallback, per-part acceleration, surface
vote (`3709c`), wheel-contact count, wheel angular-drag writes (`372e0`),
ground/contact normal construction, and finally time-since-wheels-touching
update `374b8`. The new surface-only projection does not complete those sibling
stages or authorize skipping directly to the truck tail as a full native call.
`mSpeedDampingFactor+1dfc` is not written anywhere in this body PostPhysics
function. Its explicit setter is `Skateboard::SetTruckDamping82b29dd0`:
read actual body+0c then store input float to+1dfc (`29dd4`). Code xrefs are
SlideGround `82d8e2e4` (after Toolkit_CalcTruckDamping), PhysicsGround
`82d9a118`, and Skitching `82da0b24/82da0ce8`; GroundAnimation does not write
it. Therefore retain its old value across this state rather than reset it.

Outer `Skateboard::UpdatePrePhysics` calls `StartWheelLineTest` at37cfc then
queued `ApplyForces` at37d04. Outer `Skateboard::UpdatePostPhysics` first ends
wheel line tests (`37d48`), checks bad-data/strange acceleration with possible
reset paths, then calls body PostPhysics (`37e5c`) and `UpdateUpVector`
(`37e64`). The actual dynamics/frame changes therefore feed a subsequent solve,
not retroactively the solve that produced their contact state.

| Range | SHA256 |
|---|---|
| 82d8a7b0..82d8a9e0 | 960b4cf9fdfc2fdf4808f2c639acc7bbedc00a2af77aaec5265b415e51b18e5c |
| 82d98d68..82d98dd0 | deb930e1eccf6d7dbe933c97e6473d48ddc144620686b58b326b440bfbdcf569 |
| 82b29de0..82b29edc | dd0a47df145d4c2c00403292e8d90d33f8c3cd50288242ea7ec516faec9e85fd |
| 82b29fb8..82b29ff8 | 9d0fb9a07afafab1a83dba1ee6d7e4979fa45c07494b54bba0ad5fdf9a578ee5 |
| 82b24cf0..82b24e10 | 141864932169634691b612a11b6a34e5830b9906688858362408e829280927f1 |
| 82b315a8..82b3212c | c8d44e0949e87773f2955c4d1a3c5c7864f1cdacea9b5d6375030a0d599f2c97 |

## Root steering attribute/constructor addendum

Generated physics_steering ctor828b0d20 builds low2f0d4d06 with lis/ori0d34/40,
high3b76fd33 with0d38/44, inserts high32 at0d4c, then FindCollection0d50.
Class hash is3b76fd332f0d4d06. Global construction calls this at828b2570,
using shared default collection r28, and retains pointer+b4 at257c.
Outer Skateboard constructor stores positive-zero f0 to deckTilt+e0 at82b38b08,
before force queue construction. Root implementation retains tilt in the same
outer runtime, uses the immutable actual global Vault instance and scalar key
c3249a9df26600c9. Missing lookup alone selects positive zero; malformed present
scalar is an error. No literal tuning value replaces runtime lookup.

The bounded UpdateSteeringTilt implementation preserves coefficient*target,
processed-pointer capture before old tilt read, remaining1-coefficient,
remaining.mul_add(old,weighted), tilt store before current flag reads, flip
contact-bit swap and unconditional actualbody callback even for targetzero.
Truck dynamics/frame publication remains the separate postphysics stage.
Root tests and independent review are pending for this new integration.

Integrated validation update: checkpoint24084/31b724 PASS; workspace3266 units
(3023core), sixdocs, originala15c4c eleven PASS, strictClippy/fmt/diff PASS.
Independent source/code/owner/corpus review passes. Selected
component/collaborator and controlled-schedule limits above remain explicit;
this does not establish a native full HiLOD frame or a playable app.
