# Actual Skateboard UpdateUpVector proof packet

2026-09-08, raw debug XEX and PDB audit, before implementation. This corrects
the earlier final-continuation overview's dynamic-up field offsets. Function
82b26158..2661c SHA256
c8ee0b5f750ea86ef743fc640dac985195a0db1645d619f3c3a90e9513ac236b.

## Actual retained fields and initialization

PDB `Sk8::Physics::Skateboard` and raw addi2632c/26330 agree:

| Outer offset | PDB field | Actual constructor store/value |
|---|---|---|
|70|mUpVectorDynamic|38aa0 full [0,1,0,0] from stack var_F0|
|80|mUpVectorDynamicSpeed|38ab0 full [0,1,0,0] from stack var_E0|
|90|mContactAcceleration|38ac0 full zero from var_D0|
|a0|mAverageGroundNormal|38ad0 full zero from var_C0|

The speed's initial UP is intentional source evidence, not a typo to repair.
Raw389a0 loads f0=+0,389a8 loads f13=1;389cc/d4/dc and389e0/e8/f0 assemble
the two distinct UP vectors, with W integer-zero stores389e4/ec. Do not replace
the initial speed with zero or reuse the existing outer jump_normal+c0.

Outer TeleportReset360b8..361e0 does not store any of70/80/90/a0. It resets
other body/outer fields and jump_normal+c0; these up fields persist. Constructor
execution must initialize this projection explicitly on the same retained
outer board occurrence; allocation alone must not claim native scheduling.

## Branch and full-vector graph

Call actual GetSkateboardBody2618c, read numWheelsInContact Body+361 at26190.
If zero, store full zero ONLY to mContactAcceleration+90 at261c8 and return.
In particular, this branch does not clear dynamic-up-speed+80, and does not
write average-ground-normal+a0. It reads no attributes or processed speed.

Nonzero branch:

1. Capture Body CollisionInfo, initialize local accumulator full zero. Capture
   global physics_default+e0; read contact0 byte26238, then cached layout pointer
   and full WorldGravity26240/44 BEFORE branching on contact0. PDB generated
   physics layout: WorldGravity+0 Vector3, SkateboardStartPos+10; total20 hex.
2. Test contact0,1,2 only. Each true branch reads the corresponding full
   CollisionInfo acceleration+170,+180,+190 (=Body+1b0,+1c0,+1d0), performs
   separately rounded accumulator+acceleration, then subtracts the same gravity
   full vector. There is NO contact3 read/contribution. Raw26254..a8 confirms
   this despite the four-wheel count gate. NormalizeSafe(accumulator,full0)
   at262c4. Do not divide by contact count or publish the accumulator to+90.
3. Lookup global reckoning key A below, then scalar load26320. Read old speed
   full26344 and normalized target26348, multiply speed by scalar26354,
   store speed+80 at26358. This is the first retained write on nonzero path.
4. Capture global owner2635c, read dynamic-up+70 at26360, subtract it from
   normalized target26368. Lookup key B, scalar263bc. Capture processed owner
   pointer263c8, capture global263d0, read current surfaceSpeed+96c263e4 and
   fabs263e8. Multiply full correction by B263f4 (ordinary rounded multiply).
5. Lookup key C, scalar load2644c; rounded ratio=absSpeed/C26450. Capture
   global26448 before division. Lookup key D26490 but retain POINTER, with
   missing DefaultDataArea4 selection at264a0. Capture global264a4 separately;
   its reckoning cached layout264d8, graph X+460/Y+480, eight points264e4.
6. Compute dot3(correction,correction)264e8 BEFORE dereferencing retained D
   pointer264fc. cap=D*curveValue264fc..26504. Then reciprocal-sqrt estimate
   and two refinements2651c..48, length=squared*inverse2654c, exact-zero
   selection26550. Use separate primitive calls: an eager try_length would
   move the fallible D scalar read past the estimate.
7. Let positiveLength=fsel(-length,+0,length)2655c/60. NormalizeSafe the full
   scaled correction with full0 fallback26564 (a second independent dot and
   possible estimate, not reuse of the preceding length calculation).
   Rounded test=cap-positiveLength26568; selected=fsel(test,positiveLength,cap)
   26594. No extra lower-bound clamp on cap.
8. Read damped speed26574 before loading normalized correction2658c. Full4
   FMA(direction,selected,dampedSpeed)265b0; store speed+80 at265b8. Reread
   dynamic-up+70 at265bc; separately rounded full add(up,newSpeed)265c0 and
   store265c4. NormalizeSafe the now-retained up with full0 fallback265c8;
   normalized full store265d8. Finally actual Body getter265e8, load its
   average normal Body+40 at265f4 and copy to outer+a0 at265f8.

VMX128 IDA display aliases v87/v86/v93/v91/v89 are not fresh registers: raw
26354=1677a0b5 uses loaded speed v55;26368=17d6907d uses normalized target
v54;265b0=14ec48f5 is correction v44 * scalar v41 + old speed v39;
265c0=14853035 adds loaded up v37 + new speed v38. No dt multiply occurs.

All fsel tests choose the nonnegative arm for ordered >=0 (including either
signed zero), negative arm for unordered. Thus length NaN survives the first
fsel; cap-minus-length NaN chooses cap in the second. A negative cap is not
silently clamped to zero. Numeric graph exceptional payload/denormal behavior
remains the shared host numeric boundary; preserve operation and call order.

## Vault ownership and reusable implementation

All four FETCH keys use global physics_reckoning_default (singleton+e4), index0,
missing pointer DefaultDataArea4; malformed present values must refuse.

| Key | Lookup | Scalar read | Role in this graph |
|---|---|---|---|
|ab8f93ba080e0cb3|26308|26320|multiply old speed|
|9f15310dbbe96b8c|263a4|263bc|multiply target-minus-up correction|
|20f1eca637708494|26434|2644c|divide absolute processed surface speed|
|30bf4da4bd7ae00a|26490|264fc AFTER graph and dot|multiply graph cap|

PDB `Attrib::Gen::physics_reckoning::_LayoutStruct` names+460
DynamicSpeedMaxDeltaGraphZ (PointGraphData8); own/global selection is not a
physicsdeck attribute lookup. Reuse existing ReckoningVaultAttributes global
lookup/float and captured graph8 decoder where its immutable snapshot boundary
fits. If exposing exact singleton captures, extend a narrow retained global
token interface rather than claiming its combined scope+lookup call reproduces
separate source capture timing. Existing ground-normal global provider already
demonstrates capture/lookup/value tokens but lacks the graph layout read.

WorldGravity should reuse `AttributeRuntimePhysics::try_physics_layout` and
decode its first16 BE bytes, not generic WorldGravity FETCH. Existing runtime
class04293d0f2adcfbb4/defaultd7edbd362d7d2152 and generated ctor827b2660
already preserve missing cached layout -> DefaultDataArea20(hex). That wrapper
validates/copies the full32-byte immutable layout; this is a documented checked
host capture boundary, not a mutable native layout read.

Reuse `reckoning_numeric::try_normalize_safe`, staged dot/estimate plus
`length_from_squared_and_estimate`, and existing `physics::PointGraph<8>`
evaluation. Preserve unknown W via typed vectors; full-store source output W
cannot be manufactured from actual contact acceleration XYZ. Private XYZ-only
scratch is suitable for dot decisions; normalized full outputs need known-W
propagation or full-known fallback precisely at the source normalization gate.

## Actual caller and processed input

Only ordinary outer PostPhysics path37e20..64 reaches this after the full Body
PostPhysics suffix (including timer/dynamics/frames). Then common queue clear
37e78..80 occurs. Either up-stage failure must retain prior Body and up stores
and prevent queue clearing. Zero wheels is a real source branch, not a way to
bypass missing nonzero-wheel attribute/state producers.

Current mSurfaceSpeed is ProcessedPhysIn+96c (PDB verified), loaded late inside
this function. Its actual producer Update1_ProcessInput82e02710/714 copies
selected prior PhysOut_SkateboardMotion.mSkateboardSurfaceSpeed+a4. Existing
`SkeletonProcessedContact.prior_output().surface_speed()` exposes provenance
of that retained selected copy; its ordinary corpus currently demonstrates
constructor-origin motion, not a freshly solved motion publication. Do not
recompute speed from the just-solved board or substitute a cvar/default.

Minimum proposed API: a distinct retained outer up-state with explicit native
constructor initialization, plus staged input methods current wheel count,
contact flag(index), acceleration(index), processed surface speed, final
Body average normal. Use actual same collision owner for these reads and
late read services for globals/pointers/numerics. Keep no traversal beyond the
three source unrolled contact contributions. Root can bind the state once in
the actual retained outer runtime; no new board, Reckoning or collision owner.

Tests must discriminate speed-UP constructor from zero, zero-wheel clearing
only+90, contact3-only count with no acceleration contribution, source gravity
read even contact0false, late D decode after graph+dot but before estimate,
and partial stores at damped speed/new speed/up/normalization/final average.
Original-data nonzero-wheel corpus must consume actual acceleration and cached
Vault layouts, with controlled processed snapshot provenance stated explicitly.
