# Skeleton normal/world-contact response: next implementation evidence

## 2026-09-08 connected normal response (checkpoint57482)

This section supersedes the historical NEXT-work wording below. Confidence:
**high** for reviewed normal-mode control/data flow in the stated XYZ/host
numeric domain; native per-tick parity is not established. Full Update remains
incomplete for ragdoll and debug-triggered region configuration reload.

Production: `skeleton_collision_update.rs` mutates existing collision data;
`dynamic_assembly_skeleton_collision.rs` supplies actual retained owner/history,
body reads and attempt state. `physical_player_history.rs` performs ongoing
history, actual GetCollisions and response within one exclusive world borrow.
Pool checks require the exact completed history/export/list frame and reject
owner mismatches and replay; failures preserve already-written state. Historical
frame diagnostics are not standalone current-state certificates.

The six-route attribute provider retains the original Vault graph and separates
lookup from scalar load. Only genuine missing data defaults to +0; malformed or
unsupported found data is an error. Numeric helpers retain source NormalizeSafe,
length, clamp and negative-product-FMA cross graphs. Host dot reduction and
reciprocal estimates remain explicit, not Xenon NaN/denormal/zero-sign/FP-status
equivalence. Contact vectors retain unknown W rather than inventing its value.

Material raw-source corrections and ordering verified independently:

- All normal categories are implemented: group6 applies lazy AI/nonAI collision
  multiplier and skater maximum; group10 sets vehicle flag/maximum; group13
  pedestrian maximum. GLOBAL hitWorld is set for EVERY group except4, including
  6/10/13. Bone hitWorld instead excludes6/10/13; category bytes use raw OR.
- Timer/compliance/normal-count/world/vehicle prefix writes precede speed estimate
  at82b4b984; wipeout-time storeb990 follows that callback. Duration lookup/load
  precede selective bulk clear. This is not constructor Reset.
- Small-object threshold lookup is unconditional for response-reaching contacts,
  but float load requires otherMass>bits38d1b717. Raw branchc0dc40990014 skips on
  unordered as well; threshold comparison is strict ordered LT.
- Group winner performs slide dotc51c, friction productc540, force STOREc548,
  rsqrt estimatec554, then speed/friction/normal/Part/surface writes. Tag0 still
  reaches compliance/hook classification but skips Bone/group/timer reset.
- Ground helper computes alignment dot before signed-distance dot; alignment
  scalar load remains lazy. ShowGroundPlane follows predicate calculation.
- Entry drawBody flag invokes InitSpecificBodyParts natively, materially reloading
  Parts/offsets/radii. This branch is explicitly refused AFTER selective clears,
  BEFORE region updates. Later region debug checks follow world-position or
  contact-byte writes and model overlay availability; native nominal drawing
  call825187c0 is a folded no-op. Resumed assertion/global reload traces omitted.
- Final conflicting result precedes priority/drive-strength stores. DebugAlways
  forces strength0 and compliantJustCollided1 but DOES NOT recompute hook-valid.

Validation57482 EXIT0: fmt/check;2403core+187formats+38app+3symbols=2631unit;
6doc; all-target Clippy-Dwarnings; ALL7ignored original core checks; diff-check.
New23 tests: numeric9, attribute6, empty-contact update8, plus strengthened
existing original-body corpus. Failure-prefix tests cover lazy duration and
estimate failures, selective persistence, full4 region transforms, later overlay
failure, raw partial request, zero/NaN/out-of-range final strength and ragdoll
refusal. No fake ContactInfo/Spy/body identities were introduced for unit tests.

The mounted check uniformly translates only XYZ of original supplied rig pose;
axes and W stay unchanged. Real35-body/28-joint/49-drive pair, original resident
RobPark terrain and50-iteration solve yield positive nonroot skeleton contacts.
Controlled system velocity[0,-5,0], danger-zone bypass, on-board mode and current
cvars are explicit; gravity/timer come from original Vault. Same-frame history
and collision progress complete, source timers/flags/normal provenance pass.
An independently reviewed oracle derives eight grouped force/friction/slide/
normal/Part/surface winners from actual sealed spies, live pool masses/velocities
and UPDATED history, not actual response outputs; positive expected force is
mandatory. All24 accepted timers and ordered priority normals are checked.
It reuses separately audited host math and established Vault resolution: this
is integration/control evidence, not an independent native numeric trace.
The actual list<=16 cannot exercise the20-normal cap, and a real exact-force tie
is not guaranteed. Category6/10/13, small-object, ground-filtered and ragdoll
positive-branch corpus coverage is not claimed. Prior DMO/board trajectories
remain unchanged. Explicit teardown removes actual targets, lists and drives.

Next: current animation-volume target producer -> SkeletonDrives::Update ->
same retained frames/dynamics -> next solver Build. Source82b54ee0 reads the
existing collision drive strength; this feedback is not implemented here.

## 2026-09-08 prerequisite implementation (checkpoint82288)

The normal/world response below remains NEXT work, not completed Update.
Three prerequisite gates are now implemented and independently source reviewed:

- Ongoing SAME-owner history prefix uses actual paired board6->current input->
  existing24-Part kernel, with exact completed frame/list and replay guards.
  See `skeleton-physics-history-next-source.md`; suffix/world scheduling open.
- Existing authoritative collision-state vectors use SkeletonCollisionVector:
  private XYZ plus Option<u32> fourth-lane bits. Explicit known-four constructor,
  XYZ-only constructor and checked known_lanes()->Option prevent implicit W
  invention. Applies to NormalInfo, five BoneData fields, eight group normals
  and both most-vertical vectors. Specific-region offsets/positions remain
  known-four because their sources are full retained values/transforms.
  Source constructor/reset constants stay known. Selective Reset preserves
  unknown NormalInfo W but restores written BoneData/group/vertical fields.
  Two tests check known NaN/-0/Inf/subnormal bit roundtrip and unknown-reset
  persistence. There is no new shadow collision-state owner or response writer.
- Pool try_contact_response_cached_inverse_mass reads actual cached
  RigidBody.mInvm+7c, not mutable inertia owner. Separate existing try_raw_state
  preserves source lazy order: Astate82b4c038, cmp1c03c, BEQc040 skips massc044;
  Bstatec048/cmpc04c/BEQc050 skips massc054. Exactstate1 only, not Static9.
  Three real-pool tests cover six allocated rings/raw values, cache vs edited
  and poisoned owner, foreign/free/ABA/corrupt-ring refusal. No response caller
  currently uses the seam; its production dead-code annotation is narrow.

Strict82288 EXIT0:2608unit+6doc, Clippy, ALL7original core checks, unchanged prior
terrain trajectories. These tests are source-derived host/component evidence,
not complete collision response or Xenon numeric/per-tick trace parity.

2026-09-07. Read-only next-source packet. High-confidence raw computation graph;
not implemented or validated against native per-tick traces. This is the real
pull consumer, not Body::GetContacts (a no-op for board and skeleton).

Main Update82b4b8f0..82b4c6fc uses actual retained history, original unscaled
Part masses, current collision inputs, live rigid bodies and Island ContactInfo.
Reuse existing collision state but NOT a wholesale reset: some fields persist.
All `D(a,b)` below are native vmsum3fp128, excluding W. Host reduction/estimate
and exceptional float behavior remain explicit numeric services.

## Required kernels

NormalizeSafe824b18e0..824b19b8: squared=D(v,v); initialized epsilon bits358637bd
(initializer82f763a0..82f763c8); only `squared - epsilon*epsilon > 0` invokes
rsqrt estimate and TWO refinement rounds, then v*refined inverse. Else supplied
fallback (zero here). Each refinement: square=e*e, half=e*0.5,
residual=fused(1-input*square), e=fused(half*residual+e).

Other lengths always perform estimate/two refinements even for zero, THEN choose
+0 for exact-zero squared length or squared*refined_inverse otherwise.
Sites: system speed82b4b940..b9f0, scaled friction82b4c164..c1a4,
grouped slide82b4c51c..c598, clamp82e49480..94d4.

ClampVectorWithinMaxLength82e49430..82e4957c asserts only maximum<0, computes
length above, returns input when length<bits37800000. Otherwise fsel on
maximum-length selects length if nonnegative, maximum otherwise. Request
reciprocal estimate of length and refine twice; output is
`refinedReciprocal * (input * selected)`, not input*(selected/length).
Existing drive Jacobian reciprocal/rsqrt refiners match; generic physics.rs
normalize_safe3/clamp_vector_length3 do NOT preserve these graphs.

## Prefix and filtering

Normalize system velocity; independently calculate speed. Decrement collision
timer, clear per-update flags, increment wipeout time, copy ragdoll byte; fetch
physics_skeleton.default timer-duration key12ea5351a38fd32e index0.
All24 timers subtract dt without clamping; clear just-collided and represented
BoneData; clear grouped force/friction/slide/normal and contact-bone entries,
but NOT grouped audio surfaces. Most-vertical normal=-J (signed zeros), toe
normal=allzero. Non-ragdoll clears wipeout time/cumulative wipeout force.
Update two configured body regions; partial-ragdoll request sets timer=duration.

Visit contacts in retained order. Tag<24 required. Resolve actual other Body's
current Assembly group, null=>0. Group4 with tags15/16/19/20 sets foot-hit-board
BEFORE self-collision skip. Surface `(id&0xf80)==0x300` sets unrideable BEFORE
ignored-body skip. Choose actual opposite Spy body using isThisA.
Normal mode tests ground plane only when `!dangerZone || groundTrick`; if ground,
skip. Most-vertical normal Y uses strict>, toe similarly for tags15/19.
Check both specific regions. If numNormals>=20, skip ALL remaining response,
BEFORE greater-priority reset. Set Part timer0/justCollided1. Greater priority
resets count/running priority; equal appends normal/tag. Unordered does neither.

Ground-plane82b3e720..82b3e960:
distance=D(groundNormal, contactPosition-groundPosition) (SIGNED);
alignment=abs_bits(D(contactNormal,groundNormal));
ground iff distance<maximumDistance && (offboard || alignment>minimumAlignment).
Onboard distance lookup physics_collision key30ef97095d6fa22c; offboard current
kSCMaxDistOffBoard. Alignment keye6b372a5b8ecc397 lookup happens even offboard,
but dereference only after distance passes and offboard is false. Strict tests.

SpecificBodyParts82b3eeb0..82b3efc0 ascending2: recent=oldRecent&&wipingOut;
clear current contact; actual record transform of configured Part transforms
offset by axisX*offsetX+translation, then axisY*offsetY+p, then axisZ*offsetZ+p,
all FMAs/full4. Per contact/region sample kAllowAnyContact; if enabled or matching
tag, squared XYZ distance<radiusSquared sets contact/recent. Last matching region
wins selected-region index. Normal mode clears initially unknown recent bytes.

## Force and normal/world outputs

Read actual bodyA andB states and cached inverse masses:
inverse = state==1 ? bits3a83126f : cachedInverseMass. EXACT==1, not masked;
registered Static9 uses cached value. Separate massA=1/inverseA,massB=1/inverseB.
Other mass selected by isThisA. Fetch skeleton key9ab57b38d5bb9449 every
response-reaching contact, dereference only after otherMass>bits38d1b717;
small-object additionally requires strict otherMass<threshold.

Friction scale = ((massB+massA)/(massB*massA))*bits3991a2b5. Scale actual Spy
friction then refined XYZ length. Do not simplify into inverse-mass sum.
Normal on-board modeled velocity = normalizedSystem *
D(history.velocity[tag],normalizedSystem), clamped to system speed as above.
Offboard/offboardAir instead uses supplied COM velocity, overwrites Y=+0.
relative=oppositeBody.currentVelocity-modeled;
force=abs_bits(D(relative,contactNormal)). Small-object multiplies current
kSmallObjectForceScalarHack. Neither ContactInfo.relativeVel nor normal impulse
replaces these actual reads.

Group0 sets hitWorld. Tag0 sets proxy-in-collision without BoneData/category
update or timer reset. Tags1..23 compare force strictly against selected
ordinary/subpart BoneData and grouped maxima; ties keep earlier winners.
If either wins, slide=relative cross normal via source YZXW/FMA graph.
Bone winner writes force/normal/surface/slide; ordinary winner also position,
subpart does not. Hit-category bytes only update on BoneData win.
Group winner writes force, refined slide length, frictionSpeed*unscaledPartMass,
normal, Part and surfaceID&0x7f. Set timer=duration even if no maximum improved.
Then compliant/noncompliant classification uses raw Part compliance and small
object flag; hook-to-hips valid = compliantJustCollided || offboard.

Original kCollisionPart82262140:
[-1,0,1,2,2,2,1,3,3,3,1,1,1,1,1,6,6,4,4,7,7,5,5,1].
Nonworld/ragdoll branches remain outside this bounded graph. AI/nonAI collision
multiplier keys98dbb06a81ec4f67/3f3079e086067292 are additional requirements.

## Final normal-mode work

CheckForConflictingContacts82b3a9b8..82b3ab40 loops i0..7 THEN j0..7. Both
group forces>0 and selected Parts compliant, i!=j, D(normalI,normalJ)<bitsbf333333.
Then read actual LIVE Part transformI followed byJ; return true on
D(positionJ-positionI,normalI)<0. First pair wins; no unordered-pair optimization.
Store running priority. If duration==0, driveStrength=1. Otherwise:
ratio=timer/duration; lower=fsel(-ratio,+0,ratio);
upper=fsel(1-lower,lower,1); strength=1-upper. NaN ratio yields strength0.
Current debugAlwaysCollide overrides strength0 and compliantJustCollided1;
compliantActive = strength<1 strict. Do not use generic Rust clamp.

All inspected branch/helper reductions are dot3; vertical tests splatY and
cross shuffles YZXW. Unknown native packed W cannot affect XYZ/scalar decisions,
but DOES affect stored normal/position/slide W. An honest XYZ projection must
not claim equality of existing full4 arrays or unreviewed later consumers.

## Hashes / remaining services

```text
Update 4785efa3f4b358a9791877426f4e5beae73dc42698a65bf5020081ecbf05b0ae
UpdateSpecificBodyParts 69e122804b8312c899c55a6e4b7778a4e83d472db3709c2f60d0371e1e75d2a6
CalculateIsGroundPlane d88a2b15d72d9643acc26dfd33de293fc76b74422927232838f32151fb96edd3
NormalizeSafe ac52873fecced4947cc521de7a587d566c710d637780a9a943c9e0d160643e09
ClampVectorWithinMaxLength 90d8996ac7ee4b2e035c026b1adb1d6dffc62962f7a76674ac17b872a8327efd
CheckForConflictingContacts 3fba19ae429e543945a2552d086a868622c9bb5edea3dac8d4cdfde27501aede
```

Next implementation requires actual Island vectors and paired physical owner,
current skeleton-default reads/cvars, numeric seams and selective state writes.
Full UpdatePostPhysics additionally needs true current processed input/Reckoning,
ongoing history scheduling, bad-data reset/IK and later records. Controlled
component inputs are not proof of that full lifecycle.
