# GroundAnimation slide friction and collision response source packet

2026-09-08. Bounded read-only original IDA/raw/PDB and
original db.big audit. No implementation or Cargo execution by this author.
Confidence: **exact** source addresses, loads, branches, attributes and write
order; **high** meaningful arithmetic below, subject to the existing explicit
VMX estimate/reduction/exceptional-value host boundaries. No live skating claim.

## Source identity and callers

| Function | Half-open span | Original-byte SHA256 |
|---|---|---|
| `Sk8::Physics::Skateboard::ToolKit_CalcSlideFriction(float)` | `82b28260..82b28560` | `c14aa2494242a9ff4c025ad280123fac1b586268c135aa4a9315760fbdf13a85` |
| `Sk8::Physics::Skateboard::Toolkit_CalcCollisionForce(SkateboardForce*,Vector3*,Vector3*)` | `82b27530..82b27adc` | `179e406e5fee95d4c94fa48af1b762a4f725a7a68bf5b1ae66938622dc807b92` |

Decorated symbols are
`?ToolKit_CalcSlideFriction@Skateboard@Physics@Sk8@@QAA?AUSkateboardForce@123@M@Z`
and
`?Toolkit_CalcCollisionForce@Skateboard@Physics@Sk8@@QAA_NPAUSkateboardForce@123@PAVVector3@vpu@math@rw@@1@Z`.
Slide's hidden output is r3 and Skateboard is r4; decompiler labels reverse
these. Collision is an ordinary bool member call, r3 Skateboard, r4 force,
r5 required torque output, r6 optional new velocity output.

Slide callers: GroundAnimation `82d8a8ec`, PhysicsGround `82d9a720`, Skitching
`82da09c0`. Collision callers: GroundAnimation `82d8a958`, SlideGround
`82d8e920`, Grind_Common `82d902c0`, Nonspecific `82d910fc`, PhysicsAir
`82d99144`, PhysicsGround `82d9a52c`.

GroundAnimation non-jump calculates front, rear, brake, slide, linear drag,
collision, in that order. Slide receives actual processed state timer;
collision receives caller-zeroed force, writable discarded torque, null new
velocity. True enqueues only collision type15 and suppresses the drag write.
False enqueues front4, rear5, slide1, brake2 then writes drag. The existing
`physics.rs::update_ground_animation_non_jump` is the wrapper reference;
these two actual helpers are still missing runtime implementations. Prior
ordinary force calculations cannot be skipped on a later collision result.

## Actual owners and dependencies

Neither helper reads a cvar. Neither performs a World line query, body lookup,
queue insertion or rigid-body force write. Both consume the retained current
processed owner at Skateboard+110. Collision additionally reads retained
Reckoning at Skateboard+104. All vector operations below are full four lanes
except dot3 and scalar lane extraction. Do not fabricate W for retained
processed vectors whose producers only establish XYZ.

| Owner offset | PDB field / use |
|---|---|
| processed+40 | `mDeckTransform` first row/right vector |
| processed+190 | `mVelocity` |
| processed+1a0 | `mSurfaceVelocity` (different from velocity) |
| processed+1c0 | `mZAxisVelocityDir` |
| processed+1d0 | `mGroundNormal` |
| processed+220 | `mUpVector` |
| processed+2e0 | `mCollisionHookToHips` |
| processed+8c8 word mask40000 | `mIsSkeletonInCollision` |
| processed+904 | actual `Attrib::Gen::physics_surfaces* mSurfacePhysicsCollection` |
| processed+93c | `mTimeDelta` |
| processed+96c | `mSurfaceSpeed` |
| processed+970 | `mSkateboardMass` |
| processed+9c8 | `mWheelHardness` |
| Reckoning+1f0 | `mGroundNormal`, distinct from processed normal |

Global singleton address is `830591b8`. Its +b8 friction, +fc heading, +9c
wheels and +10c collision pointers are separately sampled at the sites below.
Global curves use collection `default=d7edbd362d7d2152`. Surface tuning uses
**the current processed surface collection**, not global/default or board
physicsdeck. Immutable runtime providers must retain these actual instances.

| Collection class | Class hash | Generated constructor | Cached layout/default size |
|---|---|---|---|
| physics_friction | `2574e307ae61a71e` | `828b0d98` | 160 hex |
| physics_heading | `3a2a8b50453622bb` | `828b1248` | 250 hex |
| physics_collision | `7379c4467aa75df9` | `828b1338` | 70 hex |
| physics_surfaces | `f59fe715b7ae3426` | `82dd5fe8` | 10c hex |
| physicswheels | `46add2eea90cc062` | existing global owner | no layout used here |

Constructors use actual FindCollection/Instance and only select generated
DefaultDataArea(size) when cached layout is absent. Scalar lookup uses
GetAttributePointer(key,index0), then DefaultDataArea4 only if absent. Missing
singleton assertion/debug break is not an empty tuning substitute. No tuning
should be copied into runtime constants from the corpus values below.

## Slide arithmetic and staged sampling

Let `select(s,a,b)` mean native fsel: a for nonnegative (including both zeros),
b for negative or unordered. Preserve original bit-negations and separate
f32 products unless a fused operation is explicitly stated.

1. `28294..282fc`: sample processed owner, ground normal N at 282b4, velocity V at 282bc,
   then FIRST dot3(V,N) at 282c0. Only after that dot read ground normal
   again at 282cc for its Y lane, then deck right R at 282d8. Compute
   product N*dot, tangent V-product, dot3(tangent,R), and retained side
   velocity R*dot. A first-dot failure must not perform those later reads.
   Clamp normal.Y with native VMX max(-1,nY), then min(1,result). Call
   XMVectorACos at `28300`. This is slope based on normal.Y, not an angle
   between R and V. Do not normalize N/R here.
2. `28328..28344`: angle*bits3f22f983 (2/pi), bit-negate; capture global
   friction at 28334 BEFORE lower/upper fsel. Then lower-select zero,
   then upper-select one: `a=select(-q,0,q); u=select(1-a,a,1)`.
   Native VMX min/max NaN selection remains a documented numeric boundary;
   scalar fsel semantics must not become Rust clamp semantics.
3. Global friction sampled `28334`, layout+0 `SlideVsNormal` 16 points:
   X+0,Y+40. Evaluate(u) at `28378`, then A=1-result.
4. Re-sample global friction `2837c`, then processed speed `283ac..283bc`.
   Evaluate `SlideVsSpeed` 8 points, X+120,Y+140, at speed*bits3dcccccd (.1),
   call `283cc`; B=1-result. Select `C=select(A-B,A,B)` at `283fc` (max for
   ordinary finite values; unordered selects B).
5. `283d4..28404`: re-sample current processed surface instance, its cached
   layout+ec `StraightenOut_ForceTime=T`; lower-bound time using
   `lo=select(-time,0,time)`, then `bounded=select(T-lo,lo,T)` and scalar
   `ratio=bounded/T`. No positive-T validation or divide-zero avoidance.
6. Global heading sampled `283dc` before those calculations; after its
   required singleton check, evaluate `SideFrictionFactor` 16 points
   X+80,Y+c0 at ratio, call `2843c`, retaining H.
7. Re-sample processed owner `28440`; global wheels `28444`; sample wheel
   hardness W at `28450`; lookup key `b72814ec6e1a8f2e` at `2848c`, load K
   at `284a0`. Native assertion is only ordered K<0; `4098001c` BO4 BI24
   skips assertion on LT-not-set, including NaN. Do not reject NaN here.
8. `284c4`: D=1-K. Re-sample processed surface `284c8/284ec`, layout pointer
   `28504`, scalar +f0 `SideFriction_Scalar=S` at `28508`. Form fused
   `blend=D.mul_add(W,K)` at `28500`, then ordinary blend*S, *H, *C,
   followed by bit-negation. Final force is retained side velocity*negative
   magnitude (`28540`) and stored full4 at `28544`.

No output writes before the late surface-scalar read/product. Local position
is written full4 zero at `28514`, then full4 zero again at `2851c` between
scalar multiplies. Force remains caller's previous buffer until `28544`.
A staged provider/numeric error earlier must retain that exact prefix.

## Collision arithmetic, branches and output prefix

1. `2755c..27570`: read processed collision flag; absent returns false with
   **all outputs untouched** and no global/numeric calls.
2. Global collision lookup/read A=`8b170055606891de` at `275c4/275dc`, then
   lookup B=`07c67f7618d46d82` at `2761c`; read B at `27654` after sampling
   current processed and Reckoning owners. A is the later magnitude limit;
   B is local force Y offset. Both scalar pointers use index0/default4.
3. `27658..276c4`: load Reckoning ground G at 27658, processed hook-to-hips
   H at 27660, then FIRST dot3(H,G) at 27668. Only after that dot read
   surface velocity V at 27674. Compute P=H-G*dot, q=dot3(P,P) at 27688, reciprocal-sqrt
   estimate and **two** original refinements, then length=q*refinedRoot,
   selecting full zero if q==0. This is the existing length graph, not host
   sqrt and not NormalizeSafe. Ordered length< bits37800000 (2^-16) returns
   false at `276d0`, still without writing any output. NaN does not take it.
4. `276f4..27720`: scalar inverse=1/length (fdivs), N=P*inverse;
   projection=dot3(V,N). Capture global collision `276d8`; after the dot,
   lookup/read C=`7c591f34661bd15d` at `2775c/27774`; re-sample global and
   lookup/read D=`fd2d593ddff8da8f` at `277b4/277cc`.
5. `27818..27824`: normalPart=N*projection; tangent=V-normalPart;
   newVelocity=N.mul_add(D,tangent), full4 fused. If optional output exists,
   store it at `2782c` **before every subsequent magnitude/cap/torque step**.
   Difference=newVelocity-V then impulse=difference*C at `27860`.
   Only ordered projection>D replaces impulse with known full4 zero.
6. `27884..27958`: re-sample processed, qI=dot3(impulse,impulse). Sample
   time delta `278ac`, mass `278c0`, both AFTER impulse dot27890.
   Rsqrt callback278e4 follows these reads. Reciprocal callback27918 follows
   first rsqrt error calculation2790c but precedes its first correction27920.
   Calculate original length L from qI
   with rsqrt estimate/two refinements, interleaved with reciprocal estimate
   of time delta and two reciprocal refinements; scaled force is
   refinedReciprocal*(impulse*mass). The time reciprocal executes even
   when impulse is zero or the later magnitude branch returns false.
7. `27960..279c0`: compare **unscaled impulse length L**, not scaled-force
   length, with bits3c23d70a (.01). Ordered L<.01 selects known full4 zero
   force and false. Otherwise (including unordered) return flag remains
   true; scale factor is A/L only if ordered L>A, else 1. Multiply the
   already mass/time-scaled force by that scalar. This is not a conventional
   clamp of final force magnitude.
8. `279fc`: write full4 force; `27a0c`: local `[+0,B,+0,+0]`; `27a10`:
   required torque output full4 zero. These writes also happen in the
   **late false** branch. Early false and late false are distinct.
9. Regardless of that late bool, re-sample processed at `27a14`, call
   Math::AngleBetweenVectors(processed ZAxisVelocityDir,N,processed UpVector)
   at `27a20`, then Math::Angle::Normalize at `27a28`. Let angle be result;
   bit-absolute(angle)*bits3ea2f983 (1/pi) is graph input. Re-sample global
   collision `27a34`; evaluate layout `CollisionTorqueVsAngle` at `27a80`.
   It is PointNegGraphData8 at +20, but this call directly uses its eight
   X values at +30 and Y at +50; **no generic negative-graph extrapolator**.
10. `27a84`: bit-negate graph result. Re-sample processed and its UpVector
    at `27a8c/27a9c`, sign-select graph result with fsel(angle,+value,-value)
    (NaN chooses negative), multiply Up full4 and overwrite required torque
    at `27ac0`; return retained bool. GroundAnimation discards this torque
    but must still execute the calls and preserve their failure prefix.

| Branch word/site | Source condition | Unordered path |
|---|---|---|
| `4198fea0` at 276d0, BO12 BI24 | ordered length<epsilon | continue |
| `40990020` at 27864, BO4 BI25 | GT-not-set bypasses impulse zeroing | retain impulse |
| `40980028` at 27964, BO4 BI24 | LT-not-set enters non-small branch | flag true |
| `4099000c` at 27990, BO4 BI25 | GT-not-set selects factor1 | factor1 |

Required torque writes must not be dropped merely because the current caller
ignores it. A numeric/attribute failure after force/local/zero-torque writes
keeps that prefix. No automatic output reset or success substitution.

## Existing numerical implementations and remaining bridge

Reuse `reckoning_numeric::{try_acos,try_signed_angle,try_normalize_angle,
length_from_squared_and_estimate}`, `simulation_drive_jacobian` reciprocal
refinement, and `physics::PointGraph::evaluate`; keep their explicit host
estimate, reduction and exceptional-value limitations. Source leaf identities:

| Leaf | Span | SHA256 |
|---|---|---|
| XMVectorACos | 824b0278..824b0360 | 42b27df78bb508db683e03504520f7268bc4e58be651eab91359b75df76e19a5 |
| PointGraphEval::Evaluate | 824e2c40..824e2d58 | 3d0ee920d3fb971062becf10d438ea2eaf134d6422b76a5e0235ea840de492ee |
| Math::AngleBetweenVectors | 829b0628..829b0790 | fc7c6fa26aab3692128b6b3691f287bfb1ea630dad631dd33861d2fcd8eeac61 |
| Math::Angle::Normalize | 825ece78..825ecef0 | 26089a8fd553675e4ddcb823c8be1ff2ad759d4d2fdb508276e1dac75989bb79 |

Minimum integration needs typed, staged processed/retained Reckoning reads,
actual current surface instance binding, global friction/heading/wheels and
collision attributes, and full-lane force/torque/new-velocity outputs. Curves
must be fetched/evaluated at their source stage, not hoisted into tunings.
No new physics algorithm or DMO expansion is needed. Check numeric callback
interleaving in collision rather than wrapping an entire impulse algorithm
behind an opaque service. Source vectors use native VMX128 extended operands;
raw VA/VB decoding corrects IDA's misleading v94/v95 etc aliases as in the
foot packet. This packet does not authorize assuming unknown W is zero.

## Original archive validation anchors

Read existing extracted original `skater.vlt`/`skater.bin` under
`%TEMP%/skate2-foot-vault`, from
`<game folder>/data/db/db.big`.
Used the already-built inspect_vault executable (no Cargo) to resolve actual
class/default collections and raw Python byte reads as a separate scalar/
curve oracle. VLT SHA256 is
`496215b210c3ce8bda3f7a64b3cc3d59c56546e8d3026dbf1da4a72f93fa7dde`.
All below default collections have no parent.

| Scalar key | VLT node start | +8 raw float bits | Node/type |
|---|---|---|---|
| 8b170055606891de | 5c170 | 41000000 | inline40/type2 |
| 07c67f7618d46d82 | 5c130 | bd4ccccd | inline40/type2 |
| 7c591f34661bd15d | 5c0b0 | 3f800000 | inline40/type2 |
| fd2d593ddff8da8f | 5c1c0 | 3f800000 | inline40/type2 |
| b72814ec6e1a8f2e | 5b648 | 40000000 | inline40/type0 |

Definitions are float type `e22228fbb3c209d8`, size4, maxcount1, flags4,
alignment2. Semantic names for these five scalar hashes were not established
here; descriptive roles above derive from actual arithmetic, not invented
Vault field names. Surface selection/value is deliberately not inferred from
this global corpus and must come from the current contact/processed owner.

| Global curve | BIN start of X (then Y contiguous) | Bytes | SHA256 |
|---|---|---|---|
| friction SlideVsNormal | 9bd00 | 128 | b0767636a87b08db55b280cf2c3f325deec0a79843041ef8364b39119346fa2f |
| friction SlideVsSpeed | 9be20 | 64 | 500f96a70f7c0d2bf181d8ae36ec3681a2c0ac23d719f8e34a020c55d17fa3bc |
| heading SideFrictionFactor | 9ce90 | 128 | bc0db3a9eea2a09ac3f7049ecb16f559acfa5eb0802ae41b59afdc03d666d7a8 |
| collision CollisionTorqueVsAngle | 9b450 | 64 | ad04cc5e25cbad5c14093779cec1e45595d7bcfb2cc6a9d9450e72c2ea8fb79f |

Resolved layout bases are friction BIN9bd00, heading BIN9ce10, collision
BIN9b420. Corpus anchors validate actual authored payloads; they do not
replace runtime Vault access or establish end-to-end grounded skating parity.


## Assigned native leaf implementation

The later authorized implementation adds `skateboard_slide_collision.rs` and
`skateboard_slide_collision_attributes.rs`, with separate focused test files.
`SkateboardSlideCollisionServices` takes staged input/attribute/numeric
services; `try_slide(time,force_out)` and
`try_collision(force_out,torque_out,optional_new_velocity_out)` preserve output
prefixes and return typed errors. Force records reuse `SkateboardFootForce`
and all vectors retain `SkeletonRootVector` W knowledge. The only shared
numeric change is `try_signed_angle_with_wind`: old `try_signed_angle`
delegates unchanged, while the new entry obtains wind only after the existing
ACos graph. Degenerate axes and earlier numeric failures do not read wind.

The actual immutable provider binds global friction/heading/wheels/collision
instances and uses distinct captured tokens. Scalar pointers and selected
surface tokens carry provider occurrence identity; another provider bound to
the same Vault cannot substitute its tokens. `bind_selected_surface(key)`
requires the key supplied by the actual current processed surface-selection
producer. It provides no guessed surface/default selection. Cached generated
layouts and missing scalar defaults retain source behavior; found malformed
storage remains an error. No global reload/replacement semantics are claimed.

Collision's rsqrt and reciprocal estimate callbacks retain source order, with
both before the magnitude branch. Pure arithmetic refinement instructions
are grouped into the existing shared DAGs instead of duplicated instruction
interleaving. Observation of native FP status between those pure instructions
is outside the existing host numeric boundary. NaN branches themselves are
preserved, including late true on unordered impulse length and factor1 for
unordered limit comparisons. FullW scratch supplied to XYZ-only numeric
reductions is explicitly unobserved and never becomes retained/output W.

Fifteen focused kernel tests cover finite authored arithmetic, both first-dot
read boundaries, negative/NaN wheel assertion paths, NaN time/impulse/limit,
early versus late false, optional-velocity prefix, later angle/curve failure
prefixes, late wind, and unknown W. Six ordinary provider tests cover global
class/default semantics, cached curves, actual distinct selected surfaces,
foreign tokens, inherited scalar payload and malformed found storage. One
ignored mounted original test checks five scalar keys/words and every knot
of the four curves against independent raw db.big bytes. The reusable helper
is `skateboard_slide_collision_attributes::tests::check_original(vault,archive)`.
The author performed rustfmt only; root owns Cargo validation and caller
integration. This leaf/provider work does not establish a playable frame.


Selected-surface integration additionally exposes source layout fields
WheelStaticFriction+e0, WheelDynamicFriction+e4, and BrakingScalar+108.
`SkateboardSelectedSurface::try_float` reads the already retained immutable
instance directly for the brake owner bridge. The mutable provider's
`surface_float` checks occurrence identity before delegating to that same
decoder. `capture_surface_layout` retains the exact immutable instance and
provider identity without eager storage validation; `layout_float` performs
the later checked read. This preserves material-constructor smooth-layout
capture6370 before fifth restitution read6374/store6378. Captured foreign
owners are rejected, and malformed storage fails at scalar read rather than
at pointer capture. No per-force surface rebind or authored constants added.
