# Remaining PhysOut surface motion: actual producers and staged publication

2026-09-08, read-only source/repository audit. Fresh raw
82b2fdd8..82b301a0 in original FillPhysOut82b2f3e8 confirms this packet.
Whole function reference SHA256:
8c27321995ce79faf4d251aaf9c5aeac49182b7d076084730d04572b282f18c7.
No implementation, Cargo, new controlled runtime input or Actor claim.

## Concrete producer boundary

The selected actual Part6 linear/angular/deck-speed publication now exists in
physical_player_board_motion.rs and physical_player_phys_out_motion.rs. Its
DeckVelocity is the correct already-published input to this continuation.
Do not sample current rigid-body velocity again for surface velocity or ZSpeed.

Reckoning::ground_normal() already exposes the actual retained +1f0 field.
Reckoning::update_ground_normal82dd8178 updates the SAME filter: own layout
+4c0 control read, filter update, dot(raw,raw), raw normal store, rsqrt seed,
normalized normal store, filter set_value. A failed seed can leave raw normal;
an unqualified value getter is therefore not proof of a completed update.
try_update_grounded82ddd468/UpdateGroundStates82de0408 calls this producer.
Do not replace it with contact average/last-valid, outer dynamic up, filtered
Reckoning.up(), or PhysOut GroundUp even when numerical values coincide.

Reckoning also already retains ground_transform, native +20. Its existing
calculate_transform82dd3da0 computes the ground basis from ground_normal and
heading AFTER ordinary/flip transforms, writing X, Y=ground_normal, Z. No
public ground_transform getter currently exists. Add a narrow borrowed getter
on this same state when integrating; transform() returns a DIFFERENT matrix.
Although successful calculation makes ground Y equal the normal, retain the
fresh ground-transform read at its later source point and its producer readiness.

Actual production world methods currently take &Reckoning externally; they do
not own another paired copy. In the corpus RootCorpus owns one Reckoning and
PostRecordsCorpus exposes that SAME owner. It runs real retained algorithms
but with controlled ground input (including ground [0.6,0.8,0,0], fixed deck
axes/speeds and explicit cvars). This is not a runtime producer closed from
actual board contact/current animation. A surface component corpus may reuse
that explicitly controlled retained owner; it cannot claim native current
Reckoning ownership or substitute a freshly constructed UP-normal instance.
The missing runtime integration is same-pair retention/binding and source
scheduling of this existing producer with actual inputs, not another filter.

Processed physical deck Z already exists: SkeletonProcessedContact.deck,
written by try_sample_toolkit_prefix at PrepareToolkitInput26920..269e0.
Its deck_transform().columns()[2] supplies raw physical-deck +60. The second
body getter forms separately retained effective transform with late flip bits;
that effective Z is NOT the surface projection's input. Check actual completed
prefix/current owner; do not re-fetch a new body pose here or reuse toolkit
normalized body Z. Unknown W remains unknown through projection.

## Exact surface stages

1. Sample latchfdd8/checkfde0..fdf4 BEFORE obtaining writable Motion pointer
   fdf8. Then capture actual Reckoning pointerfe00 and processed pointerfe0c.
   Read G=Reckoning+1f0fe1c and V=published Motion+50fe24.
2. dot(V,G)fe2c; rounded product G*dotfe3c; S=V-productfe40. Only THEN read
   raw processed deck Zfe48; dot(Z,G)fe4c; rounded G*dotfe54; P=Z-productfe58.
   A first-dot failure must not read later deck Z.
3. NormalizeSafe(P,fullZERO)fe60 is mandatory, but its returned stack+90 vector
   is UNUSED. Reload the original projected P at stack+80fe74/fe8c. Preserve
   dot/estimate failure behavior and discard only the successful result.
4. dot(P,S)fe94. Ordered >0 keeps P; zero, negative and NaN XOR sign bits of
   all four P lanesfec8/fecc/fed0 (no arithmetic negation).
5. Fresh Reckoning pointerfed4, full ground transform copyfefc..ff28, with Y
   copiedff0c/ff14, BEFORE AngleBetweenVectors(P,S,windY)ff2c. Existing
   try_signed_angle reuses the exact shared graph. Here the caller already
   copies the matrix before the call: do not use its late-wind callback to
   move the actual matrix read after angle numeric failures. W can use an
   explicitly unobserved scratch lane for dot/cross XYZ, never publish it.
6. After angle returns, sample output latchff40/compareff4c. Compute
   t=angle*3e22f983ff54; floor(t)ff5c; fraction=t-floorff60; if ordered
   fraction>.5 subtract1ff64..6c; radians=wrapped*40c90fdbff70. Check sampled
   latchff78..8c, THEN SurfaceVelocity+60 storeff9c. A failed wrap primitive
   precedes write refusal. This is floor, not angle's truncating normalize.
7. Fresh latchffa0/checkffa8..bc; read wrapped scalarffc4 and store
   SurfaceAngle+acffdc. Its failure preserves SurfaceVelocity.
8. Reload retained local Sffe0, sample latchffe4 BEFORE dot(S,S)ffe8;
   compareffec, estimatefff0, two refinements30000..1c,
   squared*inverse30020, exact-zero selection30024. Check sampled latch3002c
   and publish SurfaceSpeed+a43004c. Zero still requests estimate. A failed
   dot/estimate preserves vector and angle but leaves prior speed.

The source guard samples in steps6/8 must not be delayed until after math.
Existing PhysOut sample_write_guard/check_write_guard/publish_motion_* cover
these stages and preserve Publication provenance. Every field gets its own
source guard. No generic eager all-input snapshot or single combined guard.

## Numeric reuse, without a competing implementation

Use existing typed NormalizeSafe graph with unknown-W propagation, existing
ReckoningNumeric dot/estimate, try_signed_angle, and the existing two-refinement
length_from_squared_and_estimate. No custom sqrt or normalization epsilon.

A floor-based wrap graph already exists inside Reckoning::calculate_animation_tilt
(reckoning.rs around316): turns-floor, >.5 subtraction, multiply2PI. That caller
first subtracts halfPi; Fill does not. Extract only the common turns-to-wrapped
radians suffix if sharing it. Existing host floor policy is explicit; a new
numerical service may expose source floor failure/exception boundaries if
required, but nearest-even cosine rounding and truncation are not substitutes.
The prior motion plan's statement that no floor graph existed was incomplete.

No new Vault scalar is directly fetched in this Fill continuation. Ground
filter controls belong to existing Reckoning's actual own/global provider;
retaining the actual state avoids hoisting values or rebinding another owner.

## ZSpeed is a separate later continuation

Before it, wobble pointer outer+10c30050 then latch30054, scalar+243005c,
check/store Motion+b03007c; repeat pointer30080/latch30084/scalar+283008c/
check/store+b4300ac. Next latch300b0/check BEFORE outer+e0 scalar300d4;
Motion+b8 store300e0. These are real intervening publication stages. Current
selected PhysOut motion fields do not represent these wobble/tilt outputs;
full Fill completion requires their producers/storage. Do not production-noop.

Fresh Body::GetPosition(6)300e8 supplies body transform, then current processed
+8c4 flag300f0 tested for0x00100000. If set, XOR sign bits in X and Z columns
30124/128; W remains unknown if body getter has not reconstructed it. Reuse
actual Body/GetBodyTransform path already recovered, not cached earlier
Toolkit.effective or GetTransform with a different inverse mapping.

First latch30134/check3013c precedes Motion pointer30154. SECOND latch30160/
compare30168 precedes effectiveZ load3016c, published DeckVelocity30170 and
**dot(V,effectiveZ)**30174. Check second sampled latch3017c..190; store
Motion.ZSpeed+a8301a0. This read is from output, not body. Actual current
processed flip flag production remains an explicit input dependency: current
SkeletonProcessedContact.ready(flags) accepts a caller-supplied snapshot,
which is not a retained full native +8c4 producer by itself.

## Minimum honest integration and tests

Bind SAME board/pair/output and completed processed deck owner; bind SAME
retained Reckoning occurrence plus its completed update provenance. Add its
narrow ground-transform getter. Implement selected surface vector/angle/speed
continuation with above exact callback boundaries and no new contact traversal.
Keep ZSpeed distinct until actual wobble/tilt and current flip stages are
represented, or explicitly expose a selected ZSpeed scope without full Fill
readiness. Native scheduling remains Update5 Fill then next Update1 priorcopy.

Existing skeleton_processed_prior_output.rs already copies surface velocity,
ZSpeed and SurfaceSpeed to retained +1a0/+964/+96c. No parallel consumer fields
or direct current getter in GroundAnimation are needed. Updating those actual
publications removes constructor defaults, but controlled Reckoning input in
a corpus must remain identified until the runtime producer chain is closed.

Focused tests should distinguish raw deck/effective Z, normal/up/last-valid,
projected P vs discarded normalized P, zero/NaN sign-selection, matrix read
before signed-angle failure, floor wrap near both halfturn edges, every guard
sample/check ordering, failed speed dot after vector/angle commit, current
late flip/GetPosition versus earlier transform, and actual output velocity
versus changed body velocity. Reject foreign/stale owners before publication.
Confidence exact for listed raw ordering and current code ownership; high for
minimal integration plan; complete Actor schedule remains unreconstructed.
