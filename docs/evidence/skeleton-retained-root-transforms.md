# Retained Skeleton root transforms

2026-09-08. Source confidence: high for recorded branch/arithmetic graphs and
ownership. No native per-tick trace or complete GroundSkateboard/GeneralUpdate
claim. This advances the root prerequisite after grounded Reckoning11283.

Subsequent checkpoint67613 replaces the original paired root corpus's selected
RAW-pose input with the same owner's mapped-pose/IK-target/inactive-adjustment/
animation-record producer. The root kernel itself remains unchanged. See
`skeleton-current-animation-producer.md` for this new bounded connection and
remaining full adjusted-animation/lifecycle gaps. The original85223 input
boundary and validation below describe that historical checkpoint.

## Original evidence and implementation

Root independently integrates `skeleton_root_transforms.rs` into the existing
`SkeletonNormalAssembly`, next to the SAME owner's histories/animation targets.
Laplace independently re-read RootTransforms and Ortho raw source and reviewed
the production graph. Native Skeleton constructor82b56d70 initializes root
matrices to I/J/K/P0 with ALL W0, positions0, prediction-valid0, heading1.
Neither ResetSkeleton nor TeleportReset resets them. The host assembly retains
one non-Clone root projection; its creation is not a claim that the complete
native Skeleton constructor/lifetime or its Reckoning pointer producer exists.

Native `Skeleton::UpdateRootTransforms`82b42938..82b43374 SHA256:
60ad9a392a23d9677107db21428dbed49ce8a95345a71a11cd4536f0cc774bed.
Its root order:

1. Save OLD SkateboardMtx.P to LastBoardPos3d30 at4296c.
2. Resolve actual paired physical->board->body, GetPosition Part6 at429a0;
   retain board matrix2f00 and rigid inverse2f40.
3. If prediction-valid3dec is false, resolve board/body AGAIN and GetVelocity
   at42a70. Store full4 velocity.mul_add(dt, currentBoardP) at3ce0/42aa4.
4. Capture heading3ded, clear prediction-valid42aac unconditionally. If heading,
   flatten CURRENT SkateTransformAnim.Z2ce0 Y to+0 (raw42b14; decompiler loses
   the first scratch write), SignedAngle(flattenedZ,K,J)42b58, SinCos42b7c,
   store yaw then clear heading42be4. Never use old AnimToWorld heading.
5. Sign-XOR current SkateTransformAnim.P, compose AnimAngle with its translation,
   then Reckoning.Transform with that result. Store AnimToBoard2d40, assert XYZ
   non-NaN; then assert prediction XYZ. Identity/zero terms are not simplified.
6. Compose translation(prediction) with AnimToBoard, store AnimToWorld2d80 and
   assert its XYZ. Call original OrthoNormalizeRotation on a temporary, copy
   returned matrix, assert again. Finally store WorldToAnim2dc0 rigid inverse.

Yaw X=[cos,0,signxor(sin),cos],Y=J,Z=[sin,0,cos,sin],P=0. Source matrix store
orders: yaw X/Z/P/Y; AnimToBoard P/X/Y/Z; world X/P/Y/Z; normalized X/Y/Z/P.
The exclusive host operations publish complete matrices between actual service
calls/assertions, not native concurrently observable per-column writes.
Assertions accept infinities and ignore W; typed stops retain preceding stores,
without claiming native debugger-resume behavior. Prediction-valid is NOT set
true by RootTransforms. The grounded entry exposes only82b5e488 heading=true
and82b5e490 Root call; the outer wrapper's remaining effects are not included.

## Reused numerical graphs and original SinCos

Existing `skateboard_spawn_compose` executes X multiply (or X FMA into P for
translation), Y FMA, Z FMA, all4 lanes. Existing rigid inverse transposes XYZ,
sets basis W+0, arithmetic0-P and Z multiply/Y FMA/X FMA for translation.
Laplace rechecked `skateboard_spawn_normalize` against
OrthoNormalizeRotation82b22c78..82b22d24, SHA256
dfe744df56df5b2f68438a3f8f8e66bd42e33ed4b005d6db857978c3242bb281.
It is reverse-axis CLASSICAL Gram-Schmidt Z/Y/X. Projection dots always use
the ORIGINAL axis; separately rounded projection products are subtracted from
the candidate. Every axis uses dot/estimate/TWO refinements, with no zero or
finite repair. Translation is restored unchanged after X completes. Native
helper output is temporary, not partial writes into retained AnimToWorld.
Its host reduction/seed choices remain disclosed existing numerical boundaries.

`reckoning_sin_cos.rs` implements original XMVectorSinCos82584f60..825850e8,
SHA256418cfa6763a323a517a3b4ab4da4ce161306cab52016b9f536e1d5714c67bae0.
Nietzsche authored; root independently read the complete raw instruction stream
and all six coefficient tables823cfe10..60 plus PI constants823cfea0.
Range reduction uses rounded angle*bits3e22f983, native nearest-even INTEGER
VALUED FLOAT (VRFIN, not i32), then (-bits40c90fdb).mul_add(nearest,angle).
The exact source power DAG and ascending fused sums are retained, not Horner,
powi, sequential powers or libc SinCos. See the full table/power packet in
`reckoning-grounded-transform-source.md`. Native output stores cos then sin;
Root splats one scalar at42b78 and has distinct stack outputs. A completed
scalar pair after one rounding service models this caller. Power construction
is grouped before independent sums; there are no intervening stores/services.
FP status, denormals, NaN payloads and native VNMSUB zero-sign remain open.

## Unknown packed W and honest input boundary

`SkeletonRootVector` retains XYZ plus Option<W>; a matrix has four such columns.
The checked live board velocity provides XYZ, while packed native Right-link W
is UNKNOWN. Even dt0 cannot turn that into known0: unknown bits could represent
NaN/infinity. Forward predicted position and AnimToWorld.P retain that unknown.
Affine composition never consumes INNER W. Unknown outer basis W propagates,
including zero coefficients; outer P.W affects translation only.
Rigid inverse instead recovers KNOWN W because its new basis W is+0 and its
translation only consumes input P.XYZ. Actual zero-term arithmetic still runs.
Private zero-W scratch is never published as native known output: affected
W is re-established or retained as unknown before storage. Ortho at this caller
has known basis W and ignores P arithmetically, so original P is preserved.

`SkeletonRootInput` explicitly supplies current dt and SkateTransformAnim; it
is not the missing ProcessData producer, adjusted pose or constructor pose.
The current Reckoning reference must be the retained caller collaborator, not
a matrix replacement. Native Skeleton's retained Reckoning pointer lifetime
is still not modeled by this physical-assembly constructor. These boundaries
must remain explicit until actual player/animation production is connected.

The paired bridge validates actual physical-owner identity/backend and mutates
the existing Skeleton root state. Board Part6 matrix and conditional separate
XYZ velocity reads use checked live session/body access, not a publication's
primitive occurrence6, whole-pool snapshot, fake body handle or supplied pose.
Root-only state mutation does not invalidate collision publications or require
a solved-contact phase not present in native RootTransforms.

## Independent review and exercised path

Root authored retained state; Laplace raw-source/production audit PASS and
authored11 focused tests. Nietzsche authored SinCos/six tests; root raw-code/
coefficient review PASS. Huygens authored paired bridges/original corpus;
Nietzsche independently audited both production ownership/access and corpus
oracles PASS, also reviewing root/tests. Constructor expected words were
strengthened to independent literal native identity words, not the production
constant. Six SinCos tests use frozen words computed before implementation
with separate exact integer-significand/f32 rounding arithmetic. At PI, fused
source sine b4a7bec4 differs from defused b294dd8b and sequential-powers
b2255165; range1000 fused/split reduction and nearest-even .5 differ too.
These are host graph oracles, not captured native execution bits.

Eleven root tests cover repeated original history, lazy prediction/heading
services, CURRENT flattened Z with W preserved, board/velocity/angle/SinCos
failure prefixes, actual noncanonical yaw W, unknown W even with dt0, known
inverse W recovery, ignored inner W, conservative unknown outer W at zero
coefficient, exact4097-squared FMA residuals and all four assertion stages.
The original infinity assertion fixture passes AnimToBoard's XYZ non-NaN
check, then identity zero-term arithmetic produces the later world NaN:
this proves the stages are distinct rather than generic finite validation.

`physical_player_roots_corpus.rs` is called INSIDE the existing original paired
construction/solver corpus, not a new duplicate factory or ignored wrapper.
The SAME mounted Vault now also selects the Reckoning class and its frozen
original Ground control words are checked. One external retained Reckoning
collaborator feeds two updates on the actual Skeleton's root projection after
two real0/28/49,50-iteration solved frames. Actual nonzero Part6 velocity,
changed second deck transform, evolving Reckoning Up and prediction distinct
from current deck position prevent vacuous identity checks. Expected root XYZ
is calculated from actual reads/current controlled inputs with already audited
primitives, not from retained root outputs. Original RAW boneIds[0]-selected
pose plus controlled translation is NOT bone-to-volume-mapped or adjusted
SkateTransformAnim. Inputs, pose and cvars remain explicitly controlled.

The corpus also verifies wrong member kind before numerical reads, a second
actual live prediction committed before a controlled late angle failure,
unknown predicted/world P.W, independently known inverse W, and unchanged
actual body occurrence/transform/velocity. All9 original core checks pass with
this extension; prior terrain/collision/drive paths remain green. Final full
workspace gate is separately recorded in RESEARCH_LOG, not a native parity claim.

## Remaining path

Root transforms alone do not update processed orientation, move hooks, run
GeneralUpdate velocity/history, reset the skeleton, apply IK/handplants or
produce fully adjusted drive targets. Current input/animation remains controlled
until original player producers exist. Full freeroam still requires gestures,
Actor scheduling, renderer/camera and deterministic native trace comparison.
Focused/corpus results and the final strict gate are recorded in RESEARCH_LOG
after completion, not inferred from this source packet.
