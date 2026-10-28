# Actual Air animated Skeleton continuation — 2026-09-22

Status: independently source-approved unregistered implementation and artifact patch. No Cargo or
registered-file changes by implementation agent. Root integration/gates required.
Confidence: high for control/store order and ordinary finite arithmetic; not a
Xenon per-tick floating-point trace claim.

## Scope and reused state

Native Air.Update82d9f7c8 selects Skeleton.UpdateAnimatedSkateboard82b5bc58(false)
when its retained use-COM-velocity flag is clear. The original Ground100 entry
already takes this branch. This change connects that call to the actual attached
board/Skeleton pair, current mapped pose, same processed owner, actual Part0,
existing GeneralUpdate hooks/IK/drives, and all 24 Skeleton body displacements.
KnownAir82b5d408 remains separate and unsupported by this entry.

Retained roots stay in SkeletonRootTransforms, general matrices stay in
SkeletonGeneralUpdate, board quaternion/blend flag stay in force_runtime.steering.
The existing StateChangerAttributes capability accesses the same global
physics_airstates.default layout; no second gameplay owner or copied live tuning
is introduced. Existing GroundedWorldServices supplies actual contact queries.

## Reference ordering

UpdateAnimatedSkateboard82b5bc58 checks animation-to-board and predicted-position
XYZ for NaN, calls UpdateRootTransforms82b42938 at82b5be90, then checks prediction
and animation-to-world again. It composes raw animation-to-board with mapped pose0
and translates by predicted position; this deliberately does not substitute the
normalized animation-to-world matrix. It checks resulting XYZ, writes processed
orientation82b5c32c..338, sets processed8c4 bit80000 atc340, then stores skate-root
c344..350. ApplyBoardAnimation82b357f0(false) precedes physics-board stores
c388..3a0, then GeneralUpdate c3a4; next-trajectory identity stores c3c4..3d8
occur only after GeneralUpdate succeeds.

The old grounded implementation already contained GeneralUpdate; the patch
extracts its prefix and complete continuation without copying their arithmetic.
The 24-Part force loop no longer owns caller-specific next-trajectory reset.
Ground's bounded special-state path and complete ordinary path each retain one
reset after success. Animated resets afterward; future KnownAir must not reset.
Root update gains an entry that does not force Ground's update_heading=true.

ApplyBoardAnimation repeats processed orientation XYZ assertion and bit80000
assertion at82b35984/9ac. It copies the orientation into a return temporary,
reads outer-board+100 blend flag at35a00, optionally invokes BlendToAnimatedBoard,
then sets actual board assembly Part0 transform at35a70. This is not deckPart6
and not whole-board SetTransform. Existing typed Part API preserves unknown W.

## BlendToAnimatedBoard82b25bd8

Initial angle is ACos(retained quaternion.W)*2. It multiplies by inverse2pi,
uses FLOOR, subtracts one only if fractional turn>0.5, multiplies by2pi, clears
sign. It then fetches current global+104 cached layout graph: normal X+10/Y+30;
fast X+60/Y+80, eight points. Generated physics_airstates constructor828b0c74..88
establishes DefaultDataArea(130) on null, so zero fallback is source-backed.
Graph result ordered less than bits3f733333 continues. Equal, greater, and NaN
clear blend flag at25d18 and return input matrix without changing quaternion.

Continuing path does dot4(q,identity), flips q and dot if0>dot, and always computes
Cos(bits3db2b8c3 @8225c474). Near branch recomputes dot4 and selects linear signed
interpolation, dot4 length, rsqrt estimate and two refinements. Far branch calls
ACos, Sin; makes TWO reciprocal estimates of that Sin before further Sin calls,
refines each once, calls Sin(t*angle), refines second again, forms identity weight,
calls Sin((1-t)*angle), refines first again, then FMA(q,weight,identity-weighted).
Quaternion store82b25ed0 happens only after all fallible operations succeed.
Normalization reuses existing board physics-error literal-VNMSUB refinement.

Quaternion-to-matrix uses sqrt2 bits3fb504f3 @8227e8e0. qScaled=sqrt2*q;
h=.5-qScaled² via literal negated fused subtraction. Pair products and W products
build plus/minus terms; diagonal sums use h+permYZX(h). Rotation columnsXYZ are
[diagY,plusX,minusZ], [minusX,diagZ,plusY], [plusZ,minusY,diagX]. Composition uses
inputX product then inputY and inputZ FMA; translation still executes three
zero-coefficient FMAs. Source writes temporary translation,Z,X,Y; exclusive
temporary publication permits building the completed typed matrix atomically.

## Validation and limits

Seven new tests distinguish inactive lazy path; exact threshold/NaN completion;
graph failure after inverse trig; second reciprocal failure before later sine;
two estimates/four trig-stage ordering and unknown-W preservation; near branch;
zero-times-infinity translation semantics. Existing grounded unit/corpus checks
must remain green after shared extraction. Root must run authored Air frame to
exercise actual pair/Part0/GeneralUpdate producer-to-consumer path.

The implementation retains existing injected numeric and trigonometric services.
Dot4 uses existing injected XYZ host reduction plus separately rounded W product
and addition. VRFIM uses host floor. Xenon reduction rounding, denormals, NaN
payloads, exceptional FP status and estimate bit parity remain explicit limits.
No arbitrary finite guards, fake collision misses, synchronous trajectory-ready
stores, or forced blend completion are introduced.

Artifacts: `artifacts/animated-skeleton-2026-09-22.patch`, generated from current
registered sources by `artifacts/build-animated-skeleton-2026-09-22.py`.
New production drafts: `skateboard_animated_blend.rs`,
`dynamic_assembly_skeleton_animated.rs`, `dynamic_assembly_skater_pool_animated.rs`,
`physical_player_animated_skeleton.rs`; focused tests in
`skateboard_animated_blend_tests.rs`. Rustfmt of these drafts and git apply --check
passed. No Cargo results are claimed here.

Independent reviewer approved fresh source ordering, exact Ground reset count,
actual pair/processed/steering/Part bindings, quaternion branches and matrix graph.
Reviewer also confirmed mapped volume0 at Skeleton+3040, skate-root+3c80,
physics-board+2fc0, and final identity zero-W vector in the raw instructions.
