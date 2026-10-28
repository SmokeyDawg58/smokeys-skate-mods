# Controller output and unconditional free-board yaw

September13 source packet/draft. Confidence high for recovered instruction
graph, field mapping and order; portable reciprocal/dot/ATan exceptional numeric
behavior retains existing host boundaries. No Cargo or native trace validation
by implementation author. Root owns corpus gates and independent source review.

Fresh raw/PDB: SkateboardController::FillPhysOut82dc8370..8664 called by
HiLOD.Update5 at82e04f78, after FootPlant, before Wipeout4f84. Writes actual
PhysOut+48 OffBoard: holding3f, yaw20, pitch24, hiding46, dropping47,
retrieving48, playerRetrieving49. The yaw is unconditional even when mounted.

Holding reads state1bc==1, then output guard, pointer, byte store. Next source
GetAnimPartTransformWorld82b3b878 takes PartIndex11 (Spine3). Raw `(part+c1)<<6`
selects mAnimVolumesPoseBuffer3040, not raw mAnimPoseBuffer3640 and not current
rigid-body Part11. Compose actual AnimToWorld2d80 with that volume row using
X multiply/Y FMA/Z FMA for axes; translation X FMA into P then Y/Z FMAs.
Existing SkeletonRootMatrix::compose implements this exact graph and W policy.
Existing ProcessData animation_targets.volume_row(11) is reused directly.
Do not call try_query_part_transform: its attribute-driven physics-history
branch is absent from this source getter.

Controller+1b4 is SAME Processed. Effective matrix is +c0 EffectiveAnimTransform,
produced by existing physical_player_process_input_ground.rs Update1 store35b0.
It is NOT +80 EffectiveTransform. Current controller state is sampled before
the matrix loads. In effective basis R/U/F/P, with S=Spine3 world position:

```
d = S - P
h = dot3(d,U)
projection = fma(U,h,P)
position = state==3 ? actual retrieval.start.P : actual Processed.deck.P
v = position - projection
denominator = dot3(v,F)
numerator = dot3(v,R)
seed = reciprocal_estimate(denominator)
residual = -fma(seed,denominator,-1)
inverse = fma(seed,residual,seed)
atan = existing_XMVectorATan(fma(numerator,inverse,+0))
```

Position is read after projection computation. No normalization of U or divide
by U length occurs. Matrix/vector W remains typed/unknown; dot3 consumes XYZ.
Native raw VMX128 VA correction:8408 v61=v62-v0;8418 dot v61/up;
8430 v58=v60-v0;8478 dot(v58,forward),8480 dot(v58,right).
8498 multiplies numerator by refined reciprocal with +0 before existing ATan.

After ATan, output guard84b8 is captured before quadrant selection. Negative
denominator adds sign-bit-OR PI to atan; zero denominator selects sign-bit-OR
HALF_PI regardless of prior reciprocal/atan nonfinite values. Constants are
PI823557fc=40490fdb and HALF_PI82355804=3fc90fdb. Do not use host atan2:
source negative denominator/numerator behavior and zero/NaN branches differ.
The guard is checked, actual OffBoard resolved, then yaw stored8508. Pitch is
literal +0 after another checked guard8538.

Hiding samples guard before state==3; dropping samples guard before current
Processed8cc mask400. Retrieving reads mask800 first, and only if clear reads
state==4; guard follows. PlayerRetrieving freshly reads same word and computes
mask800 || (mask1000 && fresh state==4), preserving short-circuit state reads.
Every successful preceding store survives later source/numeric/access failure.

Runtime uses real same-owner controller, Processed, Skeleton and token pair.
Missing actual fields refuse at their source read. No constructor state is
fabricated. Existing effective-animation field is read via the current mutable
accessor without modifying it; an optional future read-only accessor is routine.

Root registration: construct::controller_fill path
physical_player_phys_out_controller_fill.rs; constraint_session::update5_controller
path physical_player_update5_controller_session.rs. No new owner/getter needed.
world.try_continue_update5_controller(board,skeleton,before,numeric) consumes
update5_plants::Update5BeforeControllerFill and returns Update5BeforeWipeout,
retaining original owner and explicit grind policy. Remaining Wipeout/late
Update5/conditioner phases are not certified by the return.

Tests cover mandatory Spine3 after holding store, locked initial source read,
state3 retrieval selection, source zero-denominator and NaN selection, exact
short-circuit flag reads, projection failure before selected position access,
and nonunit Up/unknown W retention. Original-data corpus must follow root's
same-Actor output chain; these tests alone do not validate the live frame.
