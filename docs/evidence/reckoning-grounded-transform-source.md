# Grounded Reckoning and Skeleton transform producer

2026-09-08. Source confidence **high** for the recorded raw graphs/layouts.
Root plus independent source packets; no native per-tick numerical trace.
Reference remains the external symbolized Skate 2 debug XEX/PDB/IDA database.
This is the next prerequisite of accepted current-target drive checkpoint60267,
not proof that the grounded gameplay caller or full GeneralUpdate is running.

## Retained grounded implementation (2026-09-08)

`reckoning.rs` and `reckoning_grounded.rs` now implement the retained constructor,
selective Reset, complete grounded GroundStates/FixedHeading calculation and
GroundNormal/CalculateTransform/DynamicLean/Tilt helpers below. The SAME
BodySpin, three PIDs and attribute provider survive subsequent calls. Grounded
updates now produce current Up, transforms, lean and tilt using the source
branch, store and arithmetic ordering, not a matrix wrapper or generic PID.
Root authored; Laplace independent raw production audit PASS. Huygens reviewed
the focused tests and original-data integration. Native concurrent/aliased
input writes are excluded by the exclusive borrowed-input contract.

`ReckoningGroundInput` is a fully supplied immutable projection of the actual
consumed ProcessedPhysIn fields, NOT the missing production input generator or
a fake HiLOD. All current cvar values are supplied through services at their
source points; enabled debug branches return typed UnsupportedOverlay instead
of silently forcing them false. Flipping assertion and negative clamp maximum
also stop explicitly with completed writes retained. Successful native object
allocation is modeled, not nullable-allocation/debugger-resume behavior.

No production game loop/Actor currently calls this module. The verified next
connection is original Vault -> SAME grounded Reckoning state -> current
transform/Up/lean/tilt under controlled input, separate from the previously
accepted actual physics-pool/collision/drive feedback. Neither trajectory source
provenance nor native per-tick equivalence is proved by this component corpus.
Next connect the genuine processed-input/root-transform/adjusted-pose owners,
then moving hooks and GeneralUpdate reset/IK, preserving unknown packed W.

### Concrete original Vault and numerical graphs

`reckoning_attributes.rs` retains separate own/global generated instances and
scoped cached-layout handles. Ordinary lookup returns a data/default token;
float/signed-word loads are separate and lazy. Source dead lookup does not
decode a scalar. Missing cached layouts use only verified Default4d0; malformed
nonnull layouts or unsupported ordinary nodes remain errors. Whole-layout1232
and graph64 bounds/copies are disclosed immutable-host policies, not native
editor Change/global replacement or concurrent inter-read mutation parity.
Huygens authored; Laplace independently checked provider and original bytes.

Original `db.big`: VLT archiveoffset256/length851568, BIN851840/length812520.
The retained default Reckoning layout is BIN+9e110, length4d0, SHA256:
23c8bac085a9a5d945ea7d3ee48acfe6dc161f8acc63ce1ee1b676197ab581e6.
Both reviewers independently read original archive bytes, not producer outputs.
All140 words in three controls and eight complete graphs match the frozen test
oracles; six inline scalar values/node flags/types also match. Runtime tuning
still comes from Vault, never these assertions. PID controls bylayout:

```text
4a0 Slow  3d23d70a 3d23d70a 3e4ccccd 3e4ccccd
4b0 Fast  3e4ccccd 3ecccccd 3e4ccccd 3e4ccccd
4c0 Ground 3e4ccccd 3e4ccccd 3d4ccccd 3dcccccd
```

Ordinary raw VLT value locations:5d710(dead3df5c28f),5d730(wheels0),
5d6f0(rate3dcccccd),5d7c0(acceleration40000000),5d690(globaldamp3f19999a),
5d7b0(reversal3f000000). Wheel node typeindex5; float nodes typeindex4, flags40.
DeckAngle negativegraph metadata100..10f remains ignored; actual X110/Y130.

`reckoning_numeric.rs` implements full4 cross, safe/unsafe normalization,
length, clamp, signed and unsigned angles, checked angle wrapping and the
ACTUAL source inverse-trig polynomial. No libc acos/asin or endpoint repair.
Nietzsche authored; root independently inspected rawACos/Normalize, all12
coefficient words and the production graph against the proof packet.

Unsigned AngleBetween829b0528..0628 is used by all FIVE FixedHeading checks,
not signedAngle829b0628. Same lazy norm gates and one refinement/input, but no
wind/cross/2PI branch. Unsigned hash:
ffe3cf54e5bf60c3fb10cf6fe7cae874dba8d2bc0ae1e1b480729b912169c147.
Signed helper is used by lean, tilt and root heading. Angle normalization
performs finite/in-range host signed truncation; unsupported conversion is a
typed error, never Rust saturation or rem_euclid.

Inverse-trig coefficients at823cfe70/80/90:

```text
C0 bd6dd42d bed65553 3e663246 400b1889
C1 3f1dd7b6 408980bd bf983f2f c0d1360e
C2 bfaf4418 c08f6ad9 3fb58485 40af6ad8
```

For scalar replicated source lanes: square=x*x, abs=bitclearSign(x),
denominator=bits3f800001-abs, cube=square*abs, weightedInput=(-abs).mul_add(x,x).
Compute linear[i]=C0[i].mul_add(abs,C1[i]) in3/2/1/0 order; ONE estimate request
for denominator; halfDen=denominator*.5. Quadratic[i]=linear[i].mul_add(abs,C2[i])
in3/2/1/0. high=q2.mul_add(cube,q3),low=q0.mul_add(cube,q1),
correction=(-halfDen).mul_add(seed*seed,.5); base=x*high;
factor=weightedInput*low; refined=seed.mul_add(correction,seed);
ASin=factor.mul_add(refined,base). ACos=rounded(PI*.5)-ASin. No domain clamp.
RawACos824b0278..0360 hash42b27df78bb508db683e03504520f7268bc4e58be651eab91359b75df76e19a5;
ASin825e8ce0..8db0 hashfcf103a8beda22ab0bba1662d63b8fffe75d197cf21d50a0d1b0adbd81aed69e.
With explicit host seeds ACos(1) is bits35000000, NOT0. The flat-ground lean
test preserves that original polynomial residual. At.75 the fused ASin oracle
3f591a99 differs from independently defused3f591a9f.

Primitive boundaries remain explicit: host dot3 separately rounded products/
left-associated adds, host1/sqrt and1/x seeds, chosen NaN propagation for VMX
min/max, signed-zero/VNMSUB/FNMSUB/denormal/payload/FP-status differences. This
is recovered scalar/vector graph fidelity, not certified Xenon numeric bits.

Focused coverage:11 owner/control-flow tests,5 provider tests,8 numeric tests.
The owner tests cover repeated flat/sloped state, angle-only target vs dead
speed maximum, dead/reversal lazy pointers, nonzero W, selective reset retaining
actual seeded BodySpin words/history/cursor, four partial coefficient stores,
PID-only versus raw-normal failure prefixes, second basis failure after main
transform/inverse, NaN balance, retained tilt, coffin effective-axis doublecross,
and explicit debug/flipping stops. Numeric tests use independently frozen
polynomial seeds/FMA residuals, one-vs-two refinements and seven failure sites.

Two additional original-data tests verify raw Vault words and actual retained
grounded evolution: one flat call then24 sloped calls using shipped tuning,
controlled current input/cvars, source numeric graphs with host seeds. They
check finite evolving transform/Up, persistent filter histories and final Up
feedback into both filters. This is NOT original native-tick ground truth.
Final strict checkpoint and known remaining connections are in RESEARCH_LOG.

## Implemented bounded history kernels

`reckoning_pid.rs` implements PIDFilter<Vector3> constructor82e48e50 and
Update82dc8f20. `reckoning_body_spin.rs` implements BodySpin constructor82dcb940
and ONLY grounded Update82dcfaf8 prefix through82dcfc04, then82dd0058 suffix.
Both are now consumed by the retained grounded Reckoning calculation above;
the original ProcessedPhysIn producer and gameplay-session caller remain missing.
PID authored by Laplace and independently raw-reviewed by root. BodySpin
authored by root and independently raw-reviewed by Nietzsche. Host f32/FMA
does not establish Xenon denormal, NaN payload, FP status or exceptional-result
parity. No generic PID, timestep insertion or all-state reset is justified.

PID PDB size60: controls00, value10, lastTarget20, integral30, derivative40,
derivativeInput50. Controls are copied P/I/D/integral-blend floats at0/4/8/c.
Constructor copies initial full4 into value and lastTarget; three histories
are full4 positive zero. Update, per lane:

```text
error = target - oldValue
integral = oldIntegral.mul_add(1-blend, rounded(error*blend))
targetDelta = target - oldLastTarget
derivativeInput = oldDerivativeInput.mul_add(1-blend, rounded(targetDelta*blend))
base = integral.mul_add(I, oldValue)
proportional = error.mul_add(P, base)
next = (derivativeInput - oldDerivative).mul_add(D, proportional)
derivative = next - oldValue
value = next
lastTarget = target
```

Blend is reloaded after integral store; derivative subtracts prior OUTPUT delta,
not prior filtered input. No dt, clamp, normalization, estimate or callback.
Value override and coefficient replacement preserve all remaining histories.
Exclusive host ownership excludes concurrent/aliased native writes between
repeated loads. Tests cover repeated updates, nontrivial W, state overrides,
isolated NaN W, and finite FMA residual2^-46 discriminators.

BodySpin PDB sizeac: history[30] at0, input78, derivative7c, filtered80,
smoothed84, target88, speed8c, speedProp90, speedDeriv94, airTime98,
derivativeTime9c, clampAmounta0, autoSpina4, historyPosa8. Constructor clears
all30 entries,12 scalars and cursor to positive zero. Ground Update:

1. Store autoSpin, capture derivative-scalar cvar target, load old input and
   derivative, capture blend target. Load scalar then blend values.
2. delta=input-oldInput; oldContribution=(1-blend)*oldDerivative; store input;
   derivative=(delta*scalar).mul_add(blend,oldContribution); store derivative.
3. Capture threshold target; compute derivative*input, load threshold. Only
   ordered product>threshold keeps derivative; equality/NaN select positive0.
4. Store filtered, then lower=fsel(-1-filtered,-1,filtered),
   bounded=fsel(1-lower,lower,1); store filtered and history[oldPos], advance
   (pos+1)%30. Private valid cursor invariant matches native signed remainder.
5. Clear airTime, speed, speedProp in that order. Other air fields survive.
6. Capture/read InputBlend, store (1-blendA)*oldSmoothed; capture/read the SAME
   cvar again, store blendB.mul_add(input,intermediate).

Current cvar refs: scalar830db76c, blend830db3f8, threshold830db984,
inputBlend830db374. Each target is ref+28, scalar at target+d8. Target capture
is a separate service from value load; both derivative targets precede either
value load. All ten fallible capture/load prefixes are tested. Host errors stop
and retain completed writes, not native exception/debugger-resume emulation.
Two different InputBlend values,67 updates/ring wraps, untouched air fields,
strict/unordered threshold, infinities and fused residuals are tested.

## Actual Reckoning owner and initial state

### Shared authored-curve correction

The existing `physics::PointGraph::evaluate`, used by jump/landing/grind
calculations, did not reproduce Evaluate824e2c40 for finite FMA rounding or
unordered branches. Huygens recovered and implemented the correction; root
independently checked the complete raw function and source tests:

- First ordered value<x0 selects y0. Otherwise, value NOT ordered<lastX selects
  lastY, including NaN query or lastX. Preserve raw endpoint words.
- Search ascending upper1..N for first ordered value<xUpper. Interior NaN is
  skipped; never sort authored coordinates or substitute a sanitized curve.
- width=xUpper-xLower; only ordered width>0 evaluates interpolation. Otherwise
  return yUpper, including NaN width.
- delta=value-xLower; slope=(yUpper-yLower)/width; result=slope.mul_add(delta,yLower).
  The source uses fdivs then fmadds, not separately rounded multiply+add.

Five new tests cover finite fused negative residual-2^-46 (a8800000 versus old
split+0), NaN/raw endpoint preservation, interiorNaN/width behavior, duplicates/
descending coordinates, one sample and zero-sample host refusal. N0 hostassert
prevents the native invalid last-element read. Failed interval search is a host
panic at the native assertion, not a claim to debugger-resume/y0 behavior.
This fixes an EXISTING caller path; it does not make those callers a live player.

PhysicalPlayerHiLOD constructor allocates size370 at82e03fe8, passes existing
ProcessedPhysIn (HiLOD+2b0) at3ff4, calls Reckoning constructor3ff8 and stores
nullable result atHiLOD+2bc (4008). Reckoning retains input at10 and allocates
actual BodySpin sizeac at82dd7b0c, ctor7b1c, nullable pointer14. Allocation
failure cannot be repaired by silently inventing a second object.

Reckoning ctor82dd7ab8 initializes six matrices (Ground20,Transform60,NoFlipA0,
InverseE0,Flip120,BodySpin160) to I/J/K/P0 with ALL W lanes0. DynamicUp1a0,
Up1b0,UpTarget1c0,GroundNormal1f0=J; Heading1e0 and FlipAxis200=I, NOT K.
UpSpeed1d0,AnimTilt210,AnimTiltSpeed220=full4 positive0. Scalars350..368 and
flipping bytes36c/36d=0. Padding is unwritten.

Generated physics_reckoning ctor828b0ff0 binds class7ce1b6b581709c6f and default
collectiond7edbd362d7d2152, port0. Missing cached layout uses verified all-zero
DefaultDataArea4d0. PID construction copies current cached controls:
GroundNormalFilter230 fromlayout4c0 (call7de4), Slow290 from4a0 (7e04),
Fast2f0 from4b0 (7e24), in that order. Initial filter value/lastTarget=J.
Ongoing GroundNormal refreshes its four coefficients each call; FixedHeading
refreshes Slow/Fast. A copied constructor coefficient set is not immutable
runtime tuning or a replacement for actual ongoing Vault layout reads.

Reset82dd3c50 restores only eight vectors (Up/UpTarget/DynamicUp/GroundNormal=J,
Heading=I,UpSpeed/AnimTilt/AnimTiltSpeed=0) and BodySpinAngle/Speed/DynamicLean/
GroundBlend=0. It does NOT reset matrices, PIDs, BodySpin, flip axis/angles/
speeds/flags. ResetBodySpin82dd4068 clears only Reckoning speed thenangle.
ResetFlipping82dd4080 rotates Up and Heading through current Flip before
identity/flag/angle resets; TargetBodyFlipSpeed is retained. Actual ResetSystems
82dfff20 calls Reset then reads current board transform and invokes
UpdateGroundStates(J,J,board.Z) at82dfff78. Reset alone is not placement.

## Grounded caller and helper graphs

PhysicsGround::Update82da0de8 calls UpdateGroundStates82de0408 at0e94,
MANDATORY UpdateHandplants at0e9c, then UpdateGroundSkateboard82b5e468 at0ea8.
GroundStates stores full4 DynamicUp1a0 then Heading1e0, tailcalls
FixedHeading82ddd468. FixedHeading calls actual BodySpin.Update with current
ProcessedPhysIn+9f8, autoSpin+0, inAir0 at82ddd4dc. The recovered attribute/branch
supplement below supplies its provider contract; the implemented retained
grounded owner now follows that graph under the disclosed input/service scope.

UpdateGroundNormal82dd8178 captures cached layout pointer once, loads/stores
control4c0/4/8/c into samePID230/4/8/c, snapshots target, calls PID81d0.
Dot returnedXYZ81e8, store RAW resultGroundNormal81f4, estimate81f8, two
refinements/full4 scale; store normalized GroundNormal822c then samePID.val
240 at8230. No zero/finite gate. A failed estimate retains raw normal and
already-updated PID histories. Do not move dot after the first state store.

CalculateTransform82dd3da0: xRaw=cross(Up,oldHeading), zRaw=cross(xRaw,Up)
BEFORE either normalized. Dot/estimate X thenZ; normalize each independently
with two source fused refinements. Store Heading=zNorm, Transform X/Y/Z;
retain oldTransformP. Copy full16 toNoFlip; Transform=compose(Flip,NoFlip),
source stores X/P/Z/Y. Compute rigidInverse. Using NEW Heading and GroundNormal
repeat raw double-cross and independent normalizations for GroundTransform
X/Y/Z, preserving GroundTransformP. No Up normalization or safe-zero repair.
Cross graph is rounded positive product then negative-product FMA, then YZXW
permutation (including source residual W); VMX128 VA aliases need raw decoding.

CalculateDynamicLean82dd82d0 uses current FLIPPED Transform.Z, Up, DynamicUp.
Subtract Z*dot(U,Z) and Z*dot(D,Z), with separately rounded multiply/subtract.
Require projected Up lengthSquared>bits3727c5ac, THEN compute/test projected
DynamicUp similarly. Failure includingNaN writes lean+0. Success calls signed
AngleBetweenVectors(projectedUp,projectedDynamicUp,Z), then Angle::Normalize
825ece78 before scalar358 store. Normalize uses truncating integer conversion
and negative fused turn subtraction, not rem_euclid; exceptional conversion
remains a numeric boundary.

CalculateTilt82dd7e60 uses NoFlip.Y: projection=Y*dot(Y,J), perp=J-projection,
q=dot(perp,perp). Only ordered q>bits3a03126f proceeds; otherwise OLD tilt
survives and no graphs/cvar are read. If projectionXYZ==JXYZ,angle0; else unsafe
two-refinement normalizeperp then signedAngle(NoFlip.Z,perp,Y). Subtract halfPI,
multiply2PIreciprocal, frac viafloor, strict frac>.5 subtract1, multiply2PI,
clear signbit; rotation=(halfPI-absWrapped)*2overPI. Processed flags8c4 bit00100000
negates. ASin(Y.y), slope=(halfPI-asin)*2overPI. Evaluate8 rotationgraph atabsrot
THEN slopegraph atabsslope. fsel supplies rotation sign; current debug cvar
830dc078 target+20/byte+cd sampled before product. Enabled debug requires
actual overlay singleton83059210. Store AnimTilt=[slopeResult*signedRot,+0,+0,+0],
not AnimTiltSpeed. Constants halfPI3fc90fdb,2PI40c90fdb,recip3e22f983,
twoOverPI3f22f983. Acos/ASin/SinCos/estimates/dot reduction remain numeric services.

### FixedHeading attribute and scalar-branch supplement

Laplace raw packet82ddd468..82dde2f8, SHA256
311d784f91f1d332013d63cb855392deaef36688181db0630bad1c39c0f604c0.
Own generated instance atReckoning0/cachedpointer4 is distinct from global
GlobalAttributeCollection+e4, even if their records happen to match.
All graphs use8X then8Y atbase+20. Source layout offsets and sampling:

| Scope/field | Layout | Sampling |
|---|---:|---|
| Global DynamicUpVsGroundY |420|d574, supplied groundNormal.Y|
| Own GroundNormalSmoothing |4c0|UpdateGroundNormal d60c, replace samePID controls|
| Own GroundVectorBlendGraph |3e0|d890, clamped ground/J angle times inversePI|
| Own GroundVectorBlendVsSpeed |3a0|layout reloaded d894, evald8b4, deckSpeed*.1|
| Own UpVectorSmoothingSlow/Fast |4a0/4b0|one captured layoutda98, preserve histories|
| Own DeckAngleUsageVsSpeed |100|db18 withabsBodySpeed; negativegraph storage but ordinary X110/Y130 call|
| Own UpVectorSmoothingVsSpeed |1a0|layoutdd18, evaldd3c, deckSpeed*.1|
| Own UpVectorMaxDeltaVsSpeed |1e0|layoutddbc, evalddd0, SAME retained scaled speed|
| Own TiltVsRotGround/TiltVsSlopeGround |320/2a0|one capturee1a0, helpercall e1b0|

All ordinary FETCH sites use index0 and null DefaultDataArea4. Preserve lookup
separately from scalar interpretation, including the unused and lazy pointers:

| Scope/key | Lookup | Read/use |
|---|---|---|
| Own da9e831896b86ffc |d954|NEVER dereferenced; missing default resolution still executes|
| Own fae80499e7d5f3e4 |d984|raw signedwordd99c, wheel threshold|
| Own 6ab1ba8197414fb3 |d9d4|floatd9e8, per-call blendrate, no dt|
| Own 8b86e0174aee5d6e |dcfc|floatdd14, multiplied by current dt for max acceleration|
| Global 5366a548f61bbbea |df4c|floatdf6c, UpSpeed component parallel to Transform.X|
| Own af8a5d55853ad035 |dfe8|float ONLY e03c when reversal dot<0|

The first three only execute when groundnormal/J cross squared length is
ordered>bits3a83126f. Failure includingNaN copies GroundNormal toUpTarget,
skipping those keys and the two blendgraph evaluations.

Angle results a=angle(projected,GroundNormal)d734,b=angle(projected,J)d74c,
c=angle(GroundNormal,J)d764,d=angle(oldUp,J)d780,e=angle(oldUp,GroundNormal)d798.
Nearest endpoint GroundNormal iff a<b, otherwiseJ (ties/NaN). Retain projected
only when a<c AND b<c. Smoothing alpha=(deckSpeed+1)*.001; rounded candidate
contribution then FMA with oldUp. Bypass smoothing when d>c OR b>d (ordered).
Ground angle graph input c*bits3ea2f983, fsel lower0 thenupper1.

CRITICAL dead-result distinction: max(speedGraph,angleGraph) computed d8c8
goes ONLY to debug outputd90c..d934. ACTUAL blend target remains angleGraphf30.
The raw signed wheel count must be strictly greater than fetched threshold;
otherwise target+0. delta=target-oldGroundBlend BEFORE rateFETCH; then reloadold
blend. lower=fsel(-rate-delta,-rate,delta), limited=fsel(rate-lower,lower,rate),
newBlend=limited+oldAgain. No clamp validity check or dt multiplication.

Deck adjustment only signed wheels>=2 AND balance==0; NaN balance skips.
Max desired delta length is graph*.2 bits3e4ccccd. Selectionde9c is
fsel(actualLength-limit,limit,actualLength), not NaN-sanitizingmin. UpSpeed
commitdf08 precedes global dampingFETCH. Axis-parallel rounded product and
perpendicular remainder combine viaFMA; UpSpeeddfb0, Upadditiondfbc, then
NormalizeSafe with CURRENT Up itself asfallback. Reversal pointerlookup always
executes; actualfloatread only strictnegative dot, notNaN. Final Up copies to
SlowPID.vale18c THENFastPID.vale194, preserving histories. CalculateTransform,
CalculateDynamicLean, CalculateTilt run afterward in that order.

Provider must retain separate own/global instances, scoped layout capture,
explicit graph/control reads, ordinary retained lookup/defaulttoken and separate
float/signedword loads. Dead lookup cannot demand scalar validity; reversal
pointer cannot be eagerly decoded. Current ShowReckoning debug reads occur in
blend, deck-adjustment and final blocks; tilt/coffin debug are additional actual
services, not silently false substitutes. Immutable Vault limitations must stay
explicit. Root and independent review verified full vector flow before acceptance.

## Skeleton retained root producer: corrected heading and unknown W

UpdateRootTransforms82b42938 first stores OLD board P in LastBoardPos, then
reads actual paired board Part6 at429a0, stores matrix and inverse. If
prediction-valid3dec==0, resolves actual board AGAIN and reads velocity42a70,
predicts full4 velocity.mul_add(dt,currentBoardP), stores42aa4. Reads heading
byte3ded, then clears prediction-valid42aac. Heading uses CURRENT
SkateTransformAnim.Z at2ce0 with Y replaced+0, NOT prior AnimToWorld.Z.
SignedAngle with K aboutJ thenSinCos yields native yaw:

```text
X=[cos,+0,signxor(sin),cos], Y=[+0,1,+0,+0],
Z=[sin,+0,cos,sin], P=[+0;4]
```

Clear heading42be4; shiftedAnimation=AnimAngle composed with translation of
sign-XOR SkateTransformAnim.P (NOT arithmetic0-P). AnimToBoard=Reckoning.transform
composed with shiftedAnimation. Store complete then XYZ non-NaN assertion;
validate prediction; compose translation(prediction) with AnimToBoard into
AnimToWorld, store/assert, original OrthoNormalizeRotation/store/assert, inverse
WorldToAnim. Assertions accept infinity and ignore W, preserving completed
writes on typed host stop. Existing full4 spawn composition/inverse graphs apply;
do not simplify identity/zero terms away. No direct Vault/cvars in RootTransforms.

Constructor82b56d70 proves all relevant matrices I/J/K/P0 with all W0, dt0,
PredictedPos and LastBoardPos full0, heading1 and prediction-valid0. Skeleton.Init
82b62710 also initializes both24-row pose buffers identity before actual Set.
Existing controlled-target None rows are disclosed host readiness policy, not
native initialized bytes. TeleportReset82b54228 and ResetSkeleton82b45bf0 do
NOT reset root matrices/heading/prediction; ResetSkeleton places bodies from
CURRENT animation transforms. No repeated identity reset is source-equivalent.

GetVelocity82b244b8 returns native deck mVel full4 directly. W is packed
Right-link storage, unknown to the semantic pool. Prediction and world-position
W inherit that unknown; XYZ and validity branches do not depend on it. Require
typed unknown-W or an explicitly scoped semantic-lane projection, never a
fabricated native W0. Full4 parity is not established by current XYZ accessor.

SignedAngle829b0628: first dot and ordered gate>bits38d1b717, only then second
dot/gate. Failure inclNaN returns+0 before estimates/acos/wind. Estimatefirst
thensecond; ONE interleaved Newton refinement each, normalizefull4, dot and
native max(-1)/min(1), acos. Read wind only AFTERacos, cross normalizedinputs
and dotwind; orderednegative returns2PI-angle, elseangle. This is NOT the
existing two-refinement NormalizeSafe graph/threshold.

## Outer grounded Skeleton wrapper

### Original SinCos packet (now implemented for retained root updates)

The subsequent root checkpoint implements this graph in `reckoning_sin_cos.rs`
and connects it to SAME-owner paired root updates. Root independently reviewed
the raw implementation; see `skeleton-retained-root-transforms.md` for scope,
tests, and the remaining controlled-input/adjusted-pose boundary. The following
packet records the evidence used for that implementation.

Nietzsche read-only raw packet, 2026-09-08: Root call82b42b7c targets
XMVectorSinCos82584f60..825850e8, SHA256
418cfa6763a323a517a3b4ab4da4ce161306cab52016b9f536e1d5714c67bae0.
Root explicitly splats scalar angle X at42b78. Older Rust SinCos services
use disclosed libc approximations; the new root-specific helper implements
this polynomial without changing those separately scoped existing services.

Range reduction: turns=rounded(angle*bits3e22f983); native vrfin produces
nearest-even integer-valued FLOAT, not i32 truncation or ties-away round;
r=(-bits40c90fdb).mul_add(nearest,angle), with native VNMSUB exceptional/zero
semantics still an explicit numerical boundary. No source finite/domain guard.
Use a separate nearest-even-float primitive, not the existing truncate_i32.

Separately rounded source power DAG, in observed construction order:

```text
p2=r*r       p3=p2*r       p4=p2*p2
p5=p3*p2     p6=p3*p3      p8=p4*p4
p7=p4*p3     p9=p5*p4      p10=p5*p5
p11=p6*p5    p12=p6*p6     p13=p7*p6
p15=p8*p7    p14=p7*p7     p16=p8*p8
p17=p9*p8    p19=p10*p9    p18=p9*p9
p21=p11*p10  p20=p10*p10   p23=p12*p11
p22=p11*p11
```

Sine starts r, cosine1; ascending term accumulation uses coefficient.mul_add
(power, prior sum), with independent accumulators interleaved in native code.
This is NOT Horner, powi, or repeated multiplication by r. Native distinct
output stores are cos first825850dc then sin825850e0; a completed immutable
pair models this specific caller without inventing independently sampled lanes.
Raw coefficient words by ascending exponent:

```text
Sin823cfe10 exponent1/3/5/7:   3f800000 be2aaaab 3c088889 b9500d01
Sin823cfe20 exponent9/11/13/15:3638ef1d b2d7322b 2f309231 ab573f9f
Sin823cfe30 exponent17/19/21/23:274a963c a317a4da 1eb8dc78 9a3b0da1
Cos823cfe40 exponent0/2/4/6:   3f800000 bf000000 3d2aaaab bab60b61
Cos823cfe50 exponent8/10/12/14:37d00d01 b493f27e 310f76c8 ad49cba5
Cos823cfe60 exponent16/18/20/22:29573f9f a53413c3 20f2a15d 9c8671cb
```

Unknown-W dependency refinement for the next root owner: represent each column
as XYZ plus explicit Option<W>. Forward AnimToWorld.P.W inherits unknown
predicted velocity W. Affine composition never consumes INNER W; output W
depends on outer basis W and (translation only) outer P.W. A zero coefficient
cannot erase unknownness: packed native W could represent NaN/infinity.
Rigid inverse instead can recover KNOWN W despite unknown input P.W: it sets
transposed basis W to+0 and translation consumes only input P.XYZ. Compute its
W normally, including signed-zero/exceptional products. Private zero-W scratch
must never escape as purported known native lanes. XYZ assertions remain
independent of unknown W. The subsequent retained root state implements these
dependencies; this does not establish native packed-lane numerical parity.

### Grounded wrapper and GeneralUpdate ordering

UpdateGroundSkateboard82b5e468 sets heading1,e488; RootTransforms e490.
Validate predictionXYZ, then each AnimToWorldcolumnXYZ. Compose
C=AnimToBoard(+2d40) with CURRENT mapped/adjustedvolume0(+3040), then
D=translation(prediction) composed withC. Both execute full4 axis and translation
arithmetic, not simplified copies. ValidateD before any destinationmatrix writes.
Store full16 ProcessedPhysIn.orientation at0, then clear existingflags8c4 bit
00080000 (ANDfff7ffff). Only then resolve physical→board→body and fresh Part6
at e984; copy live matrix PhysicsBoard2fc0 THEN SkateRoot3c80. GeneralUpdatee9d8.
Reset NextTrajectory2e80 to identity ONLY AFTER GeneralUpdate returns (X/P/Y/Z).
Passed processed pointer and Skeleton retained processed pointer are separate
source reads; pointer identity cannot be invented. Controlled unadjusted mapped
row0 does not prove original intervening adjustments have run.

GeneralUpdate starts with AnimToWorld position velocity using current dt and
two reciprocal refinements, updates LastAnimWorldPos, then hook positions.
Hook0=AnimToWorld composed with adjustedHips; Hook1=SkateRoot. After hook/proxy
writes, teleport/large delta can require ResetSkeleton before IK/drives. Neither
this reset branch, handplants, IK nor original adjusted pose producer is optional.

## Reproducible raw function hashes (SHA-256)

```text
Reckoning ctor82dd7ab8 55194bba2121d7e4054e41890ce08dcede3d8f37b4be391b1f6832c6bb46ecbb
Reckoning Reset82dd3c50 681b6d7f2bc9c063fd930f8ab9f971c3d48eb170976b302477ce330636fbdb47
BodySpin ctor82dcb940 cb389ee7c313291eb92b6fc2e3f4b5f5ebf222d56f4e9a3424e5f4c817e7cc05
BodySpin prefix82dcfaf8..fc08 4738535ba35f8bd04e6f5a30a689ba2e3b9a4042dac326893ce14afb8cc093f1
BodySpin ground82dd0058..0098 63c2bfa614ca199a553ede74d4dc10f0e445f1120e3f992f233ff432645dc6b1
PID ctor82e48e50 ef43293371150e22d1cd738be1204a317551dcb7a46586faee9a426dc7f935ea
PID Update82dc8f20 9a3ebad7df8ab4568d85f5d50b1fa2302c5d2fa86d3af42a132a43212f9f78ce
Generated attrib ctor828b0ff0 8ca3b295caa123d2a20e2c30d717201b22ea2a6fdd1b74ba26d493e1858ad6fd
CalculateTransform82dd3da0 9c9654666145afbaad8bc593232bfa05cbfc4b6a966a49714ab97c2b0848ad06
UpdateGroundNormal82dd8178 0860563076e2e1e77f19c68eace73dd78054d9dbd7be2fa80bdea94486eb6137
CalculateDynamicLean82dd82d0 63d4fd1c77b227ac2a06b2d4aa7f128b644d2bae33b58d7669f9e5e24da91bf1
CalculateTilt82dd7e60 1104e35e95085c144e99c4f22f4b91cb10413a0d83a6cec2b6042f3671b2004a
Skeleton ctor82b56d70 2b18fc26c4b0b779768694008612f44554820e923aeb3e3459606204293adfc5
RootTransforms82b42938 60ad9a392a23d9677107db21428dbed49ce8a95345a71a11cd4536f0cc774bed
GroundSkateboard82b5e468 29adbc642357d07d39d80d6fea5b22d81708daee6affe25d19ba0d83905fe192
GeneralUpdate82b5ba88 6f9035906e5fc80631410e91625af3b7369602c7bb4c51e6b110cfdb0e8f8693
OrthoNormalizeRotation82b22c78 dfe744df56df5b2f68438a3f8f8e66bd42e33ed4b005d6db857978c3242bb281
SignedAngle829b0628 fc7c6fa26aab3692128b6b3691f287bfb1ea630dad631dd33861d2fcd8eeac61
GetVelocity82b244b8 c3d671de7f894bc16074419db47bfc99b794dd017c288a2fbf701e550635a517
PointGraph824e2c40 3d0ee920d3fb971062becf10d438ea2eaf134d6422b76a5e0235ea840de492ee
```

Full freeroam goal remains active/incomplete.
