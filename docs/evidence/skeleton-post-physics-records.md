# Skeleton post-physics records and hook displacement

2026-09-08. Continues68185. Native source graph and owner integration, not
captured Xenon traces or completed Actor/GeneralUpdate scheduling. Confidence
exact for decoded control flow, field layout and store ordering; high for host
arithmetic with explicit primitive reduction/estimate/exception boundaries.

## Source and lifecycle

All spans use exclusive ends; SHA256 hashes cover original bytes read via IDA.

| Symbol | Span | SHA256 |
|---|---|---|
| SkeletonState::CalculateErrors | 82b3d408..82b3d498 | da2457d00a244ce42d0ad10dabc0fd2e6568c29f1fe3d4fd7064dbb406cfc010 |
| Skeleton::UpdateRecords | 82b54118..82b54214 | 4616b157ccfe78cd73ea1914f1494bce2eb26fa46da16088c8e7a79cd2b3c55a |
| Skeleton::CalculateHookToHips | 82b45778..82b45b5c | bbbf61be25733cb25c95d0f89816fa95965470f8e736ffb807ab012d8b95a79f |
| Skeleton::CalculateHookToHipsWithProxy | 82b4d688..82b4d8dc | dd4dfb0bee64451532fc5ca142f27f31c4d087f5e86203661de797c12f5b9d3b |
| SkeletonBody::UpdateVolumesStates | 82b479d0..82b47aa4 | 13c97a45021a9da4480f8b34de24c5ce93e19939ef67484dc723ad12302f106e |
| Skeleton::UpdatePostPhysics | 82b575d0..82b578b4 | 75202f926bc7035f871f25d4c332fb8ab29d80f80a1de98e9010596279fc896f |

UpdatePostPhysics reads OLD retained COM velocity+3d20 at577e0 into current
collision info577e8. Collision.Update57888 precedes volume-state update57890,
then UpdateRecords5789c produces NEW retained velocity. It feeds later landing
and the NEXT collision. PhysicsRecord's mass-weighted velocity and processed
input+260 are different sources. Do not retime this derivative into the earlier
UpdatePhysicsRecords576d8 call.

SkeletonState ctor initializes24 error vectors0. Skeleton ctor stores COM,
previousCOM, velocity and HookToHips all0 (5732c..5739c), proxyEnabled false574a8.
Init repeats proxyfalse62b08; implemented Set/Init do not call true setters.
No ctor MaxDisplacement+3dd0 store was found; host retains None until produced.
ResetSkeleton45bf0 and TeleportReset54228 preserve the three retained COM vectors.
True proxy mode writers (OffboardCollision4f238 and CollideWorldOnlyOffboard5431c)
remain unconnected; no invented public flag setter or inference from ragdoll.

## Existing owners and source prefixes

New non-Clone SkeletonPostPhysicsRecords resides on the existing normal
Skeleton owner. It borrows actual physics history, completed adjusted targets,
roots, collision normals and existing proxy target. No second mass/pose/body/
collision authority is created. The proxy target's Option is existing host
evidence that Set supplied the value; native embedded ctor zero differs from
that host availability contract and is not described as uninitialized memory.

CalculateErrors first stores error0 = actual proxy position - proxy target,
full4. Ascending Parts1..23 store PhysicsRecord.TRANSFORMS[part].P minus the
current adjusted position transformed through AnimToWorld, X-FMA intoP/YFMA/
ZFMA. Separate PhysicsRecord.positions are not used. Unknown AnimToWorld W
remains unknown in affected errors even when coefficients are0.

Then Prev=oldCOM54174. Reciprocal-dt estimate54184 precedes Current=newCOM54190;
a typed seed failure retains errors+Prev only. Twice refine r with fused
residual 1-r*dt, then r=FMA(r,residual,r); full4 velocity=(Current-Prev)*r at541b4.
Only then flags8c8&1000 restamps Prev=Current; it neither clears nor recomputes
velocity. Hook suffix follows. Missing source inputs/services preserve each
completed prefix. Debug paths explicitly refuse at their source stages.

Volume states use actual existing Option-valued activity/counters and retained
volume headers. Known inactive returns without reading counters. Unknown active
or a reached unknown counter refuses at its read. Active visits24 signed counts
ascending: positive decrements then enables flagbit0 at0 or clears it while
stillpositive; nonpositive entries leave headers unchanged. Final activity OR
commits last. A later refusal keeps prior counter/header writes and oldactivity.
The next collision publication reads these same headers.

Hook paths examine errors[23,17,18,16,21,22,20,1]. First max-length comparison
ties choose17; later strict-greater candidates keep ties. All15 source dot calls
and rsqrt, including zero input, remain; two refinements produce s*rsqrt(s),
followed by explicit s==0 selection. MaxDisplacement stores before hookservices.
Normal hook samples draw, then actual retained Reckoning.GroundNormal+1f0
(not Up+1b0), computes8 normal displacements in that order, then a second
max-vector reduction (14 source dots) whose initial tie chooses23. Second draw
read precedes final HookToHips store. Proxy path samples groundnormal, normals
toggle, optional projection, scaleY, stores hook, THEN draw. Other lanes survive
Y scaling. The existing Reckoning now exposes its actual ground-normal field.

## Normal-displacement source graph

SkeletonCollision::CalculateNormalDisplacement82b3e960..82b3ec78 reads the
actual retained normal_info/num_normals. Plane projection writes output before
NormalizeSafe; later numeric/refusal errors preserve that output. Magnitude
gate uses source0.1 (3dcccccd), and only reached normal rows are validated.
Normals are projected and safely normalized; signed angles are wrapped through
fractional turns and floating floor, with strict (-pi/2,pi/2) acceptance.
Angles select min/max; midpoint gives the envelope rotation and cosine factor.
No accepted normals still executes rotation with angle0 before multiplying by0.

Standalone Cos and Sin use their own recovered power/FMA graphs; the combined
SinCos implementation is numerically different and is not substituted. Rotation
calls Cos then Sin separately; an accepted angular envelope also calls factor
Cos first. Constants tables atfe10..fe90 and permutation82fd6e90 were read from
the reference. Raw Rodrigues columns each have W=X from permutation[0,5,0,0]
and Z-only inserts. Full identity composition still executes its zero terms.
Final rotation consumes only projectedXYZ, so completed outputW can be known
even when inputW was unknown. Early projected return retains unknownW.

| Numeric callee | Span | SHA256 |
|---|---|---|
| CalculateNormalDisplacement | 82b3e960..82b3ec78 | abb687998c6f6ce3c9f5d6ab7ad39ddb25e6d635efbf030fba7ffc3c39b9cbe5 |
| NormalizeSafe | 824b18e0..824b19b8 | ac52873fecced4947cc521de7a587d566c710d637780a9a943c9e0d160643e09 |
| Signed AngleBetweenVectors | 829b0628..829b0790 | fc7c6fa26aab3692128b6b3691f287bfb1ea630dad631dd33861d2fcd8eeac61 |
| ACos | 824b0278..824b0360 | 42b27df78bb508db683e03504520f7268bc4e58be651eab91359b75df76e19a5 |
| Sin | 824b0060..824b0130 | c26e709d492360b6e9b9572d0d5eb7b67a8ab9e1c2a903c7d0adfa4ed647beee |
| Cos | 824b0130..824b0208 | 6fa06e3a5c04ff91ae16eede7f791f1ad5cd931b8e3fcc34e3554aafb16253a5 |
| RotateAroundArbitraryAxis | 82647348..826474bc | c1f5542a29ebc6468b2637b1727a72338c5ab1afb18bd91fa5c2e4a23d972740 |

Existing NormalizeSafe/signedAngle/ACos are reused after independent raw audit.
Dot reductions, reciprocal/rsqrt seeds, exceptional-result bits, denormals and
FP status remain host policies. Source polynomial graph fidelity is not a
claim of captured Xenon arithmetic results.

## Corrected floating-branch interpretation

The new raw BO/BI audit caught an error in the earlier independently accepted
offset checkpoint. Full gate41cbc=40990258 and Y gate41f1c=40990084 are BO4/BI25:
branch when CR6.GT is false, INCLUDING unordered. Active counts require >0;
NaN is inactive, positive infinity active. Previous NaN-active implementation,
tests and evidence are withdrawn. Both flags still clear last, and another
active counter can cause application despite its partner beingNaN.

Likewise normal-angle lower rejectioneb18=4099002c is NOT GT and upper rejection
eb24=40980020 is NOT LT; NaN angles are rejected. Initial implementation/review
misread mnemonic aliases; author caught the raw-bit issue before acceptance
and reviewer independently confirmed it. Corrected tests include a NaN injected
at ACos, proving only two rotation rounding calls and no factorCos. Other
vector-comparison gates have different semantics and must be audited separately.

## Validation and scope

Implementation authors remain separate from the source/code reviewer. Focused
tests cover source numerical ordering, failure prefixes, known/unknownW, both
hook branches, tie rules, zero/NaN behavior, current counter state and lazy
services. The paired terrain corpus uses the same current-animation/root and
physics owners, and independently checks error/COM/derivative state plus a
geometric f64 hook oracle with disclosed tolerance. Strict results and any
corrections are recorded in RESEARCH_LOG after the gate finishes.

Accepted33408/3e1672 EXIT0: fmt check, 2779 units (2551 core,187 formats,38 app,
3 symbols), six doc tests, all-target workspace Clippy-Dwarnings, all nine core
original corpus tests and diff check PASS. Added 3 volume-state, 13 records and
11 normal-displacement tests; offset tests increased7->9. Initial lint findings
were corrected before acceptance. Independent review PASS for source,
production, tests, connected corpus and final lint-only changes. The bounded
nearby raw numeric-gate audit found no additional implemented mismatch beyond
the two corrected NaN families. It is not a whole-repository parity audit.

Source debug drawing, true proxy-mode producer, earlier world-to-animation
physics-history suffix, full UpdatePostPhysics guards/output preparation,
Actor/state scheduling and original input controls remain unfinished. The
portable window/GPU/camera and live session remain required for playability.

## Attached Part proxy target provenance correction (2026-09-12)

Original corpus 30 exposed an invalid host assertion that the physical skate-root
translation always had known W. `Skeleton::UpdateHookPositions` raw
`82b4d5b0..82b4d5b4` copies the complete native vector from `Skeleton+0x3cb0`
to the retained State proxy target at `Skeleton+0x2c80`; it does not establish a
canonical W. The existing `SkeletonRootVector` now passes through that same
retained target, constructor transfer, and post-record consumer. Native explicit
constructor/reset zero stores remain known four-lane zeros, and the authored
full-matrix Set producer retains all four supplied lanes.

`CalculateErrors` raw `82b3d420..82b3d444` loads State proxy target `+0x1340`
and proxy position `+0x10b0`, subtracts position minus target, and stores error
`+0x11c0`. The implementation computes known XYZ and preserves unknown target
W in the existing typed error record. No zero or homogeneous-coordinate value
is substituted. Root independently reviewed these raw stores and subtraction.
Confidence: high for recovered ownership/provenance and operation ordering;
this correction does not establish complete physics numerical parity.

Focused tests preserve unknown W through hook publication into the same target
and through proxy error calculation, while retaining the existing noncanonical
known-W subtraction case. Formatting and scoped diff checks pass; the root's
library compilation passed. Full test/lint and original-corpus gates are pending
at the time of this packet.
