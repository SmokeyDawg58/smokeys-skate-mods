# Same-owner board PostPhysics continuation plan

2026-09-08. Read-only original debug XEX/PDB and original
Vault audit; no runtime implementation or Cargo run. This extends the source
proof in `skateboard-body-ground-normal-source.md`, with current repository
ownership checked rather than assumed.

## Result and minimum scope

Extend **the existing `SkateboardContactSurface` traversal** into a retained
selected Body PostPhysics continuation. Its completed-frame token already binds
the actual solved contact list, manager, pool and physical board. Do not run a
second normal-selection traversal or derive last-valid from the surface vote.
The first useful endpoint is the source normal commit through 82b37498, exposing
same-board average and last-valid to the existing FillPhysOut ground publisher.
This is not completion of all Body/outer PostPhysics or an Actor schedule.

Actual source sequence is outer EndWheelLineTest82b37d48, then outer checks,
then Body UpdatePostPhysics82b37e5c. The latter receives the retained outer
Reckoning up and processed dt. Preserve these arguments and the completed
wheel/deck batch occurrence. Wheel-line defaults are valid only after its actual
Sync/GetResult/Release sequence, not a shortcut for absent current results.

## Current owners and necessary changes

`skateboard_contact_surface.rs` currently retains seven in-contact bits and
winner normal **Y only**, physics type and overall voted type. Its actual
`PhysicalContactInfo` producer in `physical_player_contact_list.rs` retains
normal, position and relative velocity as **XYZ arrays**, not full vectors.
The earlier source packet describes native full-vector stores, not a completed
four-lane host producer. Extend winner/state with `SkeletonRootVector`; preserve
unknown W when importing these existing XYZ owners. No zero-W promotion is
justified by this audit. Arithmetic accepting unknown W must retain that
uncertainty while using known XYZ for dot/angle decisions.

Retain average and last-valid on this same board occurrence. Constructor and
TeleportReset use CollisionInfo::Reset82b23630; full up is independently proven
at8227e900, with average store236a4 and last-valid236ac. `new()` of a projection
must not implicitly claim native constructor scheduling: root must invoke the
appropriate same-body constructor/reset bridge. Ordinary frame entry preserves
last-valid and old per-Part winner fields while clearing in-contact.

Keep best-Y part index in traversal order (initial -1, Y=-2), four actual wheel
validity flags, seven normal/position/relative-velocity winners, current normal
publication readiness and the existing surface state. Invalidate readiness at
entry; failure retains already-written fields and cannot issue a current normal
publication token. Bind that token to the same body and completed contact frame.
The -1 best-index fallback explicitly selects retained last-valid, not an error.

The native seven initialization stores at Body+2f0..2f6 overlap the first
physics-surface word after the four PDB validity bytes. The seven-part grip loop
also reads those overlapping bytes at37020. A full flag/grip extension needs a
shared small layout-aware field projection or explicit alias semantics; a
fabricated bool[7] is wrong. Wheel accumulation itself reads only the four
non-overlapping flags. Preserve this as a blocking detail for a claimed full
prefix, not a reason to discard unrelated source stores.

## Required stage and failure ordering

1. Entry asserts dt>0, activates actual Assembly, calls actual
   CalculateAverageWheelCompressions, obtains the Island collision vector.
   These precede frame-selection initialization. Their real collaborators must
   run, or a typed unresolved-stage refusal must prevent later publication.
   Debug-object recoloring is interleaved with initialization and each accepted
   contact; retain an explicit debug stage if this owner has native objects.
2. Each contact first captures normal/position/relative velocity and tags. For
   deck tag6, dot(normal, passedUp) at36c14 updates two local opposing-contact
   extrema with rounded subtraction/fsel36c20..2c. Then **every contact**, even
   a later same-Entity skip, reads old retained Part velocity Body+140+i*10,
   computes dot(normal,velocity)36c48, sign-bit negates, compares with retained
   maxContactSpeed, and may store scalar36c70 then full maxContactVel36c8c.
   Raw36c5c=40990034 is BO4/BI25 GT-not-set, including unordered: only ordered newSpeed>old commits. A numeric error here must occur before owner reads and winner changes.
3. Non-null other: read other Assembly pointer36c98, other Entity36c9c,
   this Entity36ca0, compare36ca4, **read other Assembly group36ca8**, then
   skip same-Entity36cac. Existing surface-only owner callback intentionally
   omits the group read; a complete prefix must add it at this actual point.
   Null other performs no owner/group lookup.
4. Group4 executes deck-end geometry below before winner selection. Group15/16
   sets flags word bit20000000 at36d90 then clears selected validity byte36d94.
   Group14 sets bit10000000 without invalidating normal. These group effects
   apply even when this contact loses the later normal-Y comparison.
5. First accepted contact for a Part wins, otherwise ordered normalY>oldY.
   Store full normal36dd4, position36e00, relative velocity36e08, then pattern,
   physics and audio fields. Update global best part/Y from **retained winner**
   at36e5c/64 after each admitted contact; set inContact36e68. Equal/unordered
   preserve earlier winners; do not sort or rescan the seven parts afterward.
6. Missing-contact wheels0..3: capture LineDist target36ef0, read current
   wheel distance36ef4, read target+D8 scalar36ef8; only ordered distance<threshold
   copies the current line normal. LT-not-set, including unordered, clears
   validity. In-contact wheels skip all these reads. This does not set contact.
7. Before surface voting: opposing-contact delta and seven velocity/acceleration
   updates execute. Raw fdivs(1,dt) is **once at36f34**, not once per Part as
   decompiler indentation suggests. Each Part reads retained old velocity,
   then actual rigid-body velocity+20 (36fb4..bc), stores full difference to
   acceleration36fd0, stores difference*(1/dt)36fd8, then new velocity36fdc.
   Stair physics-type8 updates flag80000000; conditional grip bit40 reads
   in-contact and validity/overlap byte, then dot(normal, retained grip vector
   Body+3f0) and compares gripCos Body+400 before flag08000000. Preserve these
   interleavings or expose an explicit incomplete-stage continuation.
8. Existing ChoosePhysicsSurfaceType runs3709c **after** those seven updates.
   Then numPartsInContact stores37130. Its voted type can be published at its
   existing bounded scope; that alone does not certify stages1..7 complete.
9. Wheel-normal acceptance and angular-drag stores interleave as detailed in
   the existing ground-normal packet. Every wheel counts contact regardless of
   validity; valid wheel dot(normal,up)>cos is followed by unsigned angle
   (best normal first, wheel second)<quarterPi; accepted full normals add in
   wheel order. Each actual wheel's mutable inertia angular_drag store372e0
   occurs before the next wheel's numeric work. Publish wheel count372ec.
10. Contact-count>0 and sumSquared>2^-16: shared estimate/two-refinement graph,
    full sum*reciprocalLength; **average37378 then last-valid3737c**. Otherwise
    deck6/frontTruck4/rearTruck5 in-contact normals sum in that order; source
    length estimates execute even for zero, threshold .01 uses ordered GT,
    reciprocal refinement normalizes or full UP falls back. This writes only
    average, never last-valid. Time-since-wheels and truck drive dynamics/frames
    follow; keep a separate explicit continuation rather than claiming full
    Body PostPhysics when this normal endpoint is reached.

## Group4 geometry and cvar timing

GetBodyTransform on actual Part6 at36cc8, with output on stack: this is the
body transform, not cached Part-local transform or a guessed deck frame. Compute
full displacement=contactPosition-bodyTranslation36cec; read bodyY36cf0;
dot(bodyY,displacement)36d00; rounded bodyY*dot36d08; displacement-product36d0c.
NormalizeSafe with full-zero fallback36d14 using the existing shared graph.
Capture kMinDPWithZToBeConsideredDeckEnd target36d28 **before** reading bodyZ and
normalized vector36d2c/30 and their dot36d34; read target scalar36d38 after dot;
clear sign bit of dot36d3c, compare36d48. Raw branch36d4c is4099004c,
BO4/BI25 (GT-not-set), so only ordered absDot>threshold sets bit40000000.
Unordered does not set it. IDA VMX128 aliases v94/v95 are not distinct producers:
raw words36d00=167ef9b5,36d08=165e98b5,36d0c=163f9075 identify the reused
bodyY/displacement graph. Do not hoist cvar value before the dot capability.

## Original tuning and immutable provider ownership

All three references are in cvar section Physics_Skateboard. Defaults are
source initialization evidence, **not replacement runtime samples**:

| Reference | Initializer | Default bits / value | Source min/max |
|---|---|---|---|
| kMinDPWithZToBeConsideredDeckEnd8308dcdc |82f7b7f0|3f6b851f / .92000002|0 / 1|
| kFreeSkateboardWheelInertia8308da60 |82f7b8b0|3d23d70a / .039999999|0 / .1|
| kSkateboard_LineDistToUseNormal8308d8ac |82f7b960|3d8f5c29 / .07|0 / .5|

Raw default loads are b828/b8e8/b998. Resolved source constant addresses/words
are8225fe24=3f6b851f,82086a14=3d23d70a,82108430=3d8f5c29. The free-wheel cvar
is captured/read once37148..54 only when actual Body+2098 bit80 is set; otherwise
contact drag input is +0. Non-contact input3bc49ba6 and multiplier426fffff are
embedded source constants; do not normalize them to convenient decimal60.

After that conditional read, capture actual GlobalAttributeCollection37164,
select its physics_reckoning_default instance+e4 at37190, lookup full key
**0e508708cc7f5249**, index0 at371a8, source missing pointer DefaultDataArea(4),
then scalar read371cc. Multiply by source radians constant3c8efa35 before actual
XMVectorCos371ec. Retain separate capture/lookup/value stages and exact instance
identity. The existing ReckoningVaultAttributes supplies immutable global vs own
instances and ordinary lookup, but its current trait combines global selection
and lookup; add a narrow captured-global token rather than misrepresent that
combined operation as the original singleton lifetime. Reuse actual Vault and
checked scalar decode/default semantics; do not create a guessed layout offset.

Original class physics_reckoning=7ce1b6b581709c6f, default collection=
d7edbd362d7d2152, no parent,32 explicit entries. Existing inspect_vault executable
(read-only, no build) reports this key inline node40/type4/entry00; float type
e22228fbb3c209d8,size4,maxcount1,flags4,alignment2. Independent raw original
`<game folder>/data/db/db.big` has VLT at
archive256, node5d6f8 bytes **0e508708cc7f524942a0000000044000**; scalar
archive5d800 bits42a00000=80. VLT851568-byte SHA256
496215b210c3ce8bda3f7a64b3cc3d59c56546e8d3026dbf1da4a72f93fa7dde.
Semantic attribute name was not recovered; key and arithmetic role are exact.

## Proposed minimal typed ports and tests

Keep one staged update on the existing producer, with borrowed actual completed
contacts, retained wheel-line output token, passed up/dt, same-board services and
numeric/provider ports. Source collaborators should be individual operations:
activate Assembly; calculate compressions; read other Assembly group; read Part6
body transform; read actual Part velocity; write exact wheel angular drag;
capture cvar target and read its value; capture global instance/lookup/read scalar.
No production no-op callback may stand in for an unresolved preceding stage.
An explicit selected-component schedule can expose its scope without certifying
those omitted siblings, as the existing selected PhysOut owner does.

`SimulationRigidBodyPool::try_write_mutable_inertia` already checks exact mutable
inertia occurrence and pool, but writes a full copied inertia. Prefer a narrow
occurrence-checked angular-drag setter mutating only that scalar under the same
lock; do not overwrite a stale full inertia snapshot. It must run once per wheel
at372e0, preserve preceding numeric failures, and leave all other inertia fields.
A same-authority Part resolver should retrieve the actual body observation and
retained mutable inertia, without introducing a parallel wheel owner.

Reuse typed NormalizeSafe, unsigned angle, dot and estimate/refinement helpers.
Cosine requires a source numeric capability; ordinary host cos is not evidence
of XMVectorCos parity. Scalar reciprocal for seven velocities is source fdivs,
not the VMX reciprocal-estimate graph used in fallback normalization.

Tests should distinguish losing contacts' group invalidation; same-Entity numeric
prefix then skip; unknown W retention; inter-contact best-Y ties versus part-order
scan; no-contact line fallback strict/unordered gate; per-wheel angular-drag
failure after prior commits; count>0 but no accepted normals; -1 best sentinel;
last-valid unchanged on fallback/failed normalization; average-before-last-valid
commit; actual default Vault word and lazy missing-data path; current normal
publication refusing stale/wrong-board frames. Original corpus should use the
same board/list/wheel-batch and verify FillPhysOut copies its retained last-valid,
without introducing a second contact traversal or claiming playable scheduling.

Source hash rechecked afresh: 82b36a58..82b374d0 SHA256
2e999a33027dcdcf25dd3578a2ca48cac787f72f73b1d63fa1181d43400728ae.
Exact confidence for listed raw ordering/constants/Vault words; high for the
bounded integration proposal. Compression/debug/grip alias and full outer
schedule remain explicit dependencies, not completed implementations.


## Constructor/reset and concrete prefix closure (follow-up audit)

Fresh PDB CollisionInfo layout confirms the aliases above. Reset82b23630 is
concrete: maxContactVel initially zero2369c (subsequent source swizzle stores
236ec remain part of the seven iterations), average UP236a4, last-valid UP236ac,
part count0/ max speed0/wheel count0 at236b0/b8/bc. Flags are **masked with
17ffffff**, not overwritten zero. Its lower27 bits and bit28 survive. Initial
raw flags are therefore not wholly specified by Reset; retain typed semantic
bits/known-mask rather than fabricate an all-zero word. PostPhysics masks with
87ffffff before traversal, clearing group/grip bits; stair clears later36f40.

For each of seven slots Reset writes inContact=false236f0, normal fullUP236fc,
position zero23700, velocity zero23704, acceleration zero23708, then wheelHeight
and opposingDelta zero23710/14, audio0/physics0/pattern0 at23718/1c/20. **Relative
velocity[7] is not cleared by this Reset.** It becomes current only with a
winning contact. OverallPhysicsSurfaceType also has no Reset write here.
Finally for four wheels: line normalUP23740, distance0 23748, validityfalse2374c.
Retain unknown/unwritten relative-velocity winners until the first accepted
contact, rather than conflating their lifetime with reset-backed velocity.

PostPhysics initialization resets maxContactSpeed and full maxContactVel inside
**each** of seven validity/debug iterations (36b18/24), not once after that loop.
If debug storage access can fail, the prefix differs. It is straightforward to
perform real retained color writes in this loop rather than use an opaque gap.

Body ctor zeroes grip vector full4 at383a4 and gripCos383a8, clears Body+2098
bits80/40 at387bc..c4, and initializes both compression scalars zero387b4/b8.
These are sufficient first-session inactive-grip/free-board producers. Preserve
the bits and values in the retained owner; a later controller UpdateGrip82dcc7f8
must update that same owner when enabled, not silently keep constructor values.
Collision Reset/TeleportReset does not repeat the grip constructor writes.

The validity overlap has a small exact implementation: keep four validity bytes
and seven big-endian physics-type u32 words in the same owner. Native logical
validity indices4,5,6 map to bytes0,1,2 of physicsType[0]. At frame initialization
these three bytes become1; a later winning wheel0 physics type write replaces
all four bytes. Group15/16 clear on tag4..6 and grip reads on parts4..6 affect/read
those SAME bytes. This is not a separate seven-bool array. Other Parts' physics
words remain retained when not contacted; voting uses only inContact entries.

### Debug objects cannot be omitted

Native ctor constructs32 debug objects, numDebugObjects=0 at383ac, but its Init
then unconditionally creates one debug sphere per wheel and one per truck.
CreateWheels82b344f0 fills start/end[0..3], CreateTrucks82b33da8 fills[4..5],
CreateTriangleDeck82b32e08 fills[6] for its actual volumes. Existing authored
wheel/truck/deck definitions already retain these exact debug indices/ranges.
Thus 'debug absent' is false even without a renderer. Add retained color words
for those actual objects to the same board state, initialized from ctor White;
PostPhysics writes literal full4 idle[3dcccccd,3dcccccd,3e4ccccd,3f800000] and
contact[3f19999a,3f19999a,3f19999a,3f800000] over each actual range at the source
points. This needs no renderer service or allocation during PostPhysics.

### Wheel compression is concrete, but not a rigid inverse

CalculateAverageWheelCompressions82b2b538..82b2b818 reads actual Part6 BODY
transform first. Its inverse is an **inlined general 3x3 cofactor/determinant
inverse**, with determinant reciprocal estimate and two refinements, then
negative-translation multiply/FMA graph (roughly b57c..b668). It is not the
transpose-only `skateboard_spawn_rigid_inverse`, and no call to a separate inverse
symbol exists. Reuse shared cross/dot/refinement primitives with the actual raw
graph; recover a general-affine helper rather than substitute the rigid helper.
Then obtain actual body transforms of Parts0,1,2,3 in that order, retaining their
translations. Transform each to deck-body space with inverse translation then
X/Y/Z FMAs. Read actual retained wheel-distance Body+2094 atb744 after the four
transform calls, before remaining Y/Z projection arithmetic completes.

Front=(localY0+localY1)*.5-distance; back=(localY2+localY3)*.5-distance. Source
multiplies scalar .5 on the left. b7e4 writes front scratch, b7e8 loads it,
b7ec overwrites same scratch with back, b7f0 loads back; frontBody+208c storeb7f4
then backBody+2090 storeb7f8. Decompiler aliases both scratch names misleadingly;
the raw preserves distinct front/back values. Failure before these final stores
preserves both retained compression scalars.

The distance producer is ALREADY retained by SkateboardBodyConstruction:
Init37684/8c actual global physicstrucks key2be08dd7a9207622, exposed through
wheel_distance(). Original corpus independently asserts bitsbd676c8b. Pass this
same construction owner's field, not another attribute fetch at PostPhysics.

### Assembly activation: exact source and existing pool reuse

Assembly::Activate82ae09b8..82ae0a30 SHA256
979f41fa9b2531e5202c468b4ba5c5f381e123b0600378f8f6637080e849d2a5.
Reads count; zero returns. For ascending Part index and stride60, reads current
Part.body+4c then state+8c. If state&7==2, reads that body's actual stasis+4c and
calls Simulation::ActivateRigidBody82ae3228 at0a08. Then **rereads Part.body**
0a0c and always stores cooldown0 at0a18, including Static or already Active.
No native Part/body null guards exist. A host missing occurrence must refuse at
that Part and retain earlier activations/cooldown writes.

ActivateRigidBody82ae3228..82ae3298 SHA256
5f952a2c363ec17ad1f42071bbaf67267df7909a3cb470a1a7728c7f26a55247.
Unlink from current Frozen ring: right.left=left3238, left.right=right3244.
Append at Active anchor tail: body.right=anchor3248; body.left=anchor.left3250;
oldTail.right=body3258; anchor.left=body3260. State store326c=(oldState&8)|4;
all other bits discarded. Cooldown3278=simulationCooldown-1 wrapping; Frozen
count decrement328c, Active count increment3290. Assembly subsequently overwrites
cooldown with0. No inertia, RF rows, pose, force, torque, tag or occurrence reset.

Existing pool private unlink()/append_to_tail() already implement this same ring
order; occurrence validation/integrity checks and actual counts are available.
No activation API was found. Add the narrow frozen-body primitive plus scalar
cooldown clear under the SAME pool, and an ascending actual Part session loop.
The backend bridge must use current handles/observations, no detach/re-add or
new body allocation. Tests should prove Frozen tail order, flag8 preservation,
other-bit discard, cooldown underflow when configured0, subsequent Assembly
zero, active/static no-relink, and late invalid Part preserving earlier writes.

Smallest concrete prefix implementation is therefore retained Reset/grip/debug
state + current Part activation + general-affine compression + the existing
contact traversal extended with native numeric/group/alias/winner effects +
seven velocity/grip updates + existing surface vote + interleaved normal/drag.
These are calculable local stages over existing bodies and authored definitions;
none needs a synthetic physics service. Actual nonnull other-body Assembly group
still requires the current member's retained construction group (board group4 is
already retained), and enabling grip requires its same-owner controller producer.
This closes the normal endpoint; truck dynamics/frames and outer phase scheduling
remain separately owned continuations after that endpoint.


## Implementation-ready normal tail (second follow-up, read-only)

The bounded entry is immediately after same-owner surface choice3709c and seven
contact counts. It requires the completed preceding same-frame selection,
line-fallback and velocity/grip stages; it does not traverse contacts again.
Its completion is **GroundNormalsReady**, followed by time-since-wheel and truck
continuation. This keeps existing surface-only completion distinct.

Proposed method on the SAME SkateboardContactSurface:
`try_finish_ground_normals(&mut self, up: SkeletonRootVector, attributes: &mut A,
cvars: &mut C, bodies: &mut B, numeric: &mut N, rounding: &mut R)`.
Here N:ReckoningNumeric, R:ReckoningSinCosNumeric. Attribute/rounding/body errors
remain separate typed variants; do not force a shared error type or move any
captures before their source point. Same-owner state carries retained normals,
in-contact and wheel validity, best part sentinel, constructor/reset average and
last-valid, and source flags. No caller-supplied normal arrays or best index.
A private stage enum enforces the entry, and current normal readiness is issued
only after the last actual normal store. Failed attempts preserve field prefixes
and are not silently restarted against a partly updated body.

One concrete body operation suffices in this tail:
`try_set_wheel_angular_drag(ordinal:usize,value:f32)` resolves this same admitted
board's actual Part?body?mutable inertia at372b8/cc/d8/dc and writes only+24 at
372e0. Numeric rounding of value must occur BEFORE the late body resolution:
source reads Assembly pointer372b8, computes `dragInput * 426fffff`372bc, then
loads Part list/body/inertia. No eager validation of all four wheel inertias.
Within current immutable Assembly-storage lifetime, a getter can retain the
actual Assembly capability before multiplication without dereferencing Part/body
until the setter; it must not claim to model replaceable Assembly pointers.

Cvar interface uses separate capture/read operations, only when free-board
flag80 is set: capture target37150, value37154; false uses +0 without touching
that reference. The flag is read BEFORE numParts store37130 (raw37128), but the
cvar capture follows the store. Attribute interface is captured actual Global
instance37164/190, ordinary lookup(key0e508708cc7f5249,index0)371a8, then float
from retained pointer371cc. Missing pointer selects actual DefaultDataArea4.
No per-wheel tuning lookup is added. Numeric cosine follows the rounded
`attribute * 3c8efa35` product371e8. Constants drag+.006/60-neighbor load after
cosine returns, before the four-wheel loop.

### Exact per-wheel and finish dependencies

For i=0..3, read inContact37218 and select drag input (.006 or conditional
free-wheel sample), increment local count when contacted3722c. Read validity
37230. Invalid skips all normal/up/dot/angle reads and proceeds to inertia write.
Valid reads passed up3723c then current wheel normal37244; dot operands are
**normal,up**37248. Raw37258=GT-not-set rejects equal, smaller and unordered.
Only passing wheels address best normal via `Body+16*(bestPart+6)`3725c..6c:
-1 means Body+50 lastValidGroundNormal. Other legal best indices0..6 map to the
retained normal array. Do not read best normal before this dot gate.

Call unsigned angle(bestNormal,wheelNormal)37270. QuarterPi3f490fdb is read
after the helper returns3727c; vector compare3729c is **quarterPi>angle**.
Equal and unordered fail. On pass, reload wheel normal372b0, add **sum+normal**
372b4, retaining W Option arithmetic. Then execute this wheel's angular-drag
store. If numeric work for wheel2 fails, wheel0/1 drag writes survive, wheel2/3
writes and wheel-count publication do not occur. If wheel3 drag fails, even all
completed local normal arithmetic does not publish numWheels.

After all four stores, write numWheels372ec. If count<=0 skip the wheel-sum dot
entirely and enter fallback. Otherwise dot(sum,sum)37304; ordered strict >
37800000 at3731c admits normalization. Equal/unordered enter fallback. Estimate
37330 followed by existing TWO refinements yields inverse length; multiply
**sum * inverseLength**37370 and store average37378 then last-valid3737c. No
NormalizeSafe epsilon substitute, no contact-count divisor, no later extra norm.

Fallback starts full+0. Conditionally add normal6, then4, then5 based only on
inContact; no validity, angle, best or up reads. Dot(sum,sum)373d4, estimate
373f4 even when squared is zero, TWO refinements, product **squared*inverse**
37424 and exact squared==0 select+0. Reuse
`skeleton_collision_numeric::length_from_squared_and_estimate`, calling its
fallible dot/estimate at these original points. Ordered length>.01 enters
reciprocal estimate37460/TWO refinements and **reciprocal*sum**3747c; otherwise
fullUP. Fallback stores only average37480/98. Neither fallback arm changes
last-valid. Do not use a scale helper that reverses scalar-left operand order.
No fallible callback separates the paired average/last-valid success stores.

Constructor/reset provenance is above: both fullUP are source-established;
relative velocity has distinct unknown lifetime. Once wheel sum produces a
valid last-normal, later no-contact frames retain it even when average becomes
UP. Typed W on selected XYZ-only contacts remains unknown through accepted
normal sums, normalization and publication; fullUP fallback is genuinely known.
Dot/angle scratch W can be zero solely because those result computations do not
observe W, not as an update of retained winner lanes.

### Numeric reuse verified against fresh originals

`reckoning_sin_cos::try_cosine` now exists in the current repository. Reuse it,
not `try_sin_cos(...).1`: standalone XMVectorCos uses different rounded power
dependencies. Fresh function824b0130..824b0208 SHA256
6fa06e3a5c04ff91ae16eede7f791f1ad5cd931b8e3fcc34e3554aafb16253a5.
Turns=angle*3e22f983; one `round_nearest_even` at01a4; reduced-angle negative
FMA `(-40c90fdb)*nearest+angle`01a8. Construct x2=x*x, x4=x2*x2,
x6=x4*x2, x8=x4*x4, x10=x6*x4, x12=x6*x6, x14=x8*x6,
x16=x8*x8, x18=x10*x8, x20=x10*x10, x22=x12*x10. Start1 and
ascending coefficient*power+accumulator FMAs. There is no acos call here.
Original tables823cfe40/50/60 match existing constants exactly:
`3f800000 bf000000 3d2aaaab bab60b61`,
`37d00d01 b493f27e 310f76c8 ad49cba5`,
`29573f9f a53413c3 20f2a15d 9c8671cb`.
The first1 is constructed independently. Current numeric policy groups
independent polynomial powers but has no intervening callbacks/stores and
explicitly excludes native FP status/NaN payload/denormal parity. Host nearest
is the existing primitive, not a libc cosine substitution.

Existing `reckoning_numeric::try_unsigned_angle` matches829b0528..829b0628,
SHA256 ffe3cf54e5bf60c3fb10cf6fe7cae874dba8d2bc0ae1e1b480729b912169c147:
first dot(a,a)0540 gate >38d1b717; only passing then loads b0574 and
computes dot(b,b)0578 and same gate. Rejection returns+0 without estimates,
so it can pass the caller's quarterPi test. Then first estimate0590,
second0598, ONE refinement each, full normalized a/b, dot(aNorm,bNorm)05dc,
max(dot,-1) then min(...,+1), existing source-polynomial ACos824b0278 call05e8.
No Rust acos or extra gate. Existing helper's ordinary max/min exceptional
selection, dot reduction, seeds and negative-FMA zero-sign boundaries stay
explicit; no new parity claim. Its by-value input API is safe here under the
same exclusively borrowed immutable-during-call contact owner: a rejected first
dot has no side-effecting second getter. If a later interface adds fallible
getters, it must expose the helper's existing lazy second-load stage instead.

Refinement helpers in simulation_drive_jacobian are already used by this same
codebase and preserve separate rounded squares/products and fused corrections.
Reuse them at37350..6c,37404..20 and3746c..78. Do not copy a new local numerical
graph. The tests required now are stage/failure retention and actual owner
publication; the existing helper tests remain the numerical authority.

### Attribute integration now available

`ReckoningVaultAttributes` retains own and global instances separately from the
actual AttributeRuntimeVault. Ordinary lookup uses `try_get_data(key,0)` and
float decode checks exact4 bytes; None yields DefaultDataArea4. Its current
ReckoningAttributeRead combines scope selection with lookup, so a narrow
captured-global adapter over an actual retained instance is needed if capture
failure/order is observable. Prefer the existing scoped token pattern used by
foot/slide attributes: token retains exact instance occurrence, lookup returns
owner-bound pointer token, float validates same token owner before checked BE
scalar decode. Bind actual global class/default identified above, not own
Reckoning just because this original collection happens to have equal values.
Runtime value remains live provider data within the existing immutable Vault
scope; bits42a00000 belong only in original-data tests.

Suggested focused tests: first-wheel dot refusal before best/angle/drag; angle
first-length gate returning0; second-wheel drag refusal leaves first write;
all four writes before count; failed sum estimate after count retains old
average/last; no-contact fallback still performs zero estimate; strict cosine,
quarterPi, sum and fallback-length unordered gates; -1 selects priorlast; typed
unknownW propagation; same-owner original Vault42a00000 and no hoisted cvar
reads. Actual integrated corpus must reuse the same solved contacts and retained
wheel-line outputs and then verify FillPhysOut's last-valid copy.
