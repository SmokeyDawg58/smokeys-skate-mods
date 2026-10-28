# Two-bone solver: full numerical dependency packet

2026-09-08, author audit. Read-only IDA work; this packet changes no
Rust and runs no Cargo. This is the active pre-physics IK dependency identified
in `skeleton-general-ik-next-source.md`, not an independently integrated IK
feature. The current General adapter completes state 0x2be, which skips IK;
other states still require the actual retained IK producer and consumers.

## Identity and ownership

| Routine | Half-open raw span | SHA256 |
|---|---|---|
| `Math::_TwoBoneIKSolve` | 829b1240..829b1e30 | 377269268ca1ea81cfd8e9a66d3b4bca4509bfa648bdb119cd827398c1988347 |
| `Math::TwoBoneIKSolve` | 829b1e30..829b1e88 | 21d8d17d0ad278d82482adb97d9a863aaa28e23439bbbbafd2c002fac4e7966b |
| Math constant initializers, including intervening initializers | 82f763c8..82f765d0 | dd8f134b3322887c06c9af70e96151b64857c85fa11c7651d38f4d5887fbc5da |

Decorated symbols:

```text
?_TwoBoneIKSolve@Math@@YA?AW4TwoBoneIKresult@1@ABVVector3@vpu@math@rw@@00PAV3456@1ABVVecFloat@456@2_NI@Z
?TwoBoneIKSolve@Math@@YA?AW4TwoBoneIKresult@1@VVector3@vpu@math@rw@@00PAV3456@1MM_NI@Z
```

The wrapper has two actual callers: pre-physics `SkeletonIK::DoTwoBoneIk_PrePhysics`
at82b49ce0 and post-physics `DoTwoBoneIk_PostPhysics` at82b53818. The private core
has only its wrapper and three tail-recursive call sites198c/1db0/1e04. PDB enum
`Math::TwoBoneIKresult`: reachable0, unreachable1, invalid2, alloptions3. This
routine returns only0/1/2.

Inputs are immutable original root A, middle B, end C; mutable middle output
Bpri; mutable **in/out desired end** Cpri; minimum/maximum angle degrees;
OverrideMaxWithOrig; and unsigned remaining-call budget. They are full four-lane
native Vector3 values, with XYZ reductions only where explicitly indicated.
The wrapper writes f1/f2 as f32 and splats those stored values into VecFloat
arguments. The decompiler's apparent `double` parameters and trailing arguments
are ABI artifacts. The remaining budget passes through the stack+54 slot.

The active pre-physics caller uses override=true and budget2. That caller owns
the actual original joint positions, target, and limits. The solver has no
Vault, cvar, owner, elapsed-time, or contact-service reads. It must not invent
those inputs or become an activation shortcut around the missing IK lifecycle.

## Exact constants

The mutable static slots are zero in the unloaded database; their dynamic
initializers establish the following splats. Do not read the zero backing
bytes as runtime tuning.

| Value | f32 bits | Runtime slot | Initializer / original literal |
|---|---|---|---|
| epsilon, 0.0005 | 3a03126f | 83080010 | 82f764a0, `__real_3a03126f` |
| zero | 00000000 | 8307fe70 | 82f763c8, `__real_00000000` |
| one | 3f800000 | 8307ff90 | 82f764c8 |
| negative one | bf800000 | 8307fed0 | 82f764f0 |
| two | 40000000 | 8307ff80 | 82f76540 |
| 180 degrees | 43340000 | 8307ff00 | 82f76568 |
| two PI | 40c90fdb | 8307ff10 | 82f76518, literal821f0c30 |
| radians to degrees | 42652ee1 | 8307fef0 | 82f76590, literal821f0c28 |
| degrees to radians | 3c8efa35 | 8307fec0 | 82f765b0, literal821f0c24 |

The entry assertion uses literal42b40000, 90 degrees. SIMD one/half/zero are
also constructed locally from integer splats; half is3f000000. The rotation
permute table82fd6e90 is `00010203 14151617 00010203 00010203`, selecting
`[first.X, second.Y, first.X, first.X]`.

## Arithmetic vocabulary and service boundaries

All values below are f32. `FMA(a,b,c)` is one fused operation; ordinary products,
adds and subtracts are separately rounded. `neg_bits` XORs80000000. `dot` ignores
W but retains the existing explicit Xenon reduction service boundary.

`recip(x)` obtains one reciprocal estimate and applies two accepted Newton
refinements, without a NaN fallback. `rsqrt(x)` obtains one inverse-square-root
estimate and applies two accepted refinements, without a fallback. These are
the existing `refine_drive_reciprocal` and `refine_drive_reciprocal_sqrt` graphs.
`length(q)` means `q * rsqrt(q)`, then select positive zero when `q == 0`; the
estimate and both refinements execute even at zero. `unit(v)` means
`v * rsqrt(dot(v,v))` across all four lanes, with **no zero selection**.

`cross(a,b)` is the existing full4 rounded-product/FMA/permute graph. Its XYZ
are the usual cross product; its W is the source cancellation graph and can
be nonzero or NaN. No normalization or cross operation repairs W.

`clamp01(x)` below names the source **VMX max(0,x), then min(1,result)** operand
graph, not Rust `clamp`, not a -1/+1 clamp. Native exceptional operand selection,
denormal handling, NaN payloads and exact-zero VNMSUB signs remain the previously
declared host numeric boundary. Ordinary finite behavior and operand order are
recovered; no new claim of Xenon exceptional bit parity is made.

`acos` is the accepted polynomial at824b0278 (`reckoning_numeric::try_acos`),
including its unconditional reciprocal-square-root service. `cos` is the
standalone824b0130 graph currently private in `skeleton_normal_displacement`;
its powers differ from combined `try_sin_cos`. Both standalone cos calls must
use that graph. The two later sin/cos calls use the accepted combined routine
at82584f60 and each performs its own nearest-even rounding service.

## Entry, length gates, and output prefixes

1. Assert ordered `90 >= minimum`. Native vector all-true reduction occurs at
   128c..12a4. Failure enters FailAssert/DebugBreak before reading the recursion
   budget. A checked host error belongs here, before any output store. NaN
   minimum fails; negative and negative-infinite minimum are not rejected.
2. If budget is zero, copy original C to Cpri at12d4, then original B to Bpri
   at12dc; return Invalid. This restores the original end even after an earlier
   recursive target adjustment. Do not leave the adjusted target on exhaustion.
3. Set `u=B-A`, `v=C-B`. Obtain dot(u,u), dot(v,v), then both estimates in that
   order. Compute the zero-selected lengths `a`, `b`; save `two_a=2*a`, `a2=a*a`,
   `b2=b*b` with separate products.
4. If ordered `epsilon > a`, or otherwise ordered `epsilon > b`, copy original
   B to Bpri at1e18 and return Invalid. Cpri stays unchanged. Equality passes;
   unordered lengths do not take either short-segment exit.

There are no additional coincident-target, collinear-chain, determinant, or
finite-value guards. Those cases execute the unsafe normalization and trig
graphs below. Numeric collaborator failures are host errors, not native
Invalid results; preserve already committed output prefixes.

## Plane frames and their inverse graphs

Let D be the current Cpri target. Build:

```text
original_forward = unit(C-A)
target_forward   = unit(D-A)
plane_normal     = unit(cross(u,v))
original_side    = cross(original_forward, plane_normal)
target_side      = cross(target_forward, plane_normal)
target_normal    = cross(target_side, target_forward)
```

The final line is real source computation at1558/15a8. The target frame uses
the recomputed normal, not the original plane normal. Neither side nor this
recomputed normal gets another normalization. Both frames have translation A:
original columns `[original_forward, original_side, plane_normal]`; target
columns `[target_forward, target_side, target_normal]`.

The source inverts each three-column basis by cofactors and determinant,
**not** by a rigid transpose. For frame columns X,Y,Z:

```text
c0 = cross(Y,Z)
c1 = cross(Z,X)
c2 = cross(X,Y)
inverse_determinant = recip(dot(X,c0))
inverse.X = [c0.x,c1.x,c2.x,c1.x] * inverse_determinant
inverse.Y = [c0.y,c1.y,c2.y,c1.y] * inverse_determinant
inverse.Z = [c0.z,c1.z,c2.z,c1.z] * inverse_determinant
inverse.P = inverse.X * neg_bits(A.x)
inverse.P = FMA(inverse.Y, neg_bits(A.y), inverse.P)
inverse.P = FMA(inverse.Z, neg_bits(A.z), inverse.P)
```

The fourth lane duplicates **Y**, proved by the high/low word merge sequence
15b0..15f8 and1634..16a4. It does not come from the input basis W. No identity
position or canonical homogeneous value may replace this computed translation.

Point application starts with FMA(X,point.x,P), followed by Y and Z FMAs. Apply
the original inverse to A,B,C, producing localA/localB/localC; apply the target
inverse to D, producing localD. Then:

```text
delta = localD-localA
d2 = FMA(delta.x, delta.x, delta.y*delta.y)
d = length(d2)
```

Only XY contribute to this distance. Do not replace it with dot3(delta,delta).
Both local bases and all four local points have known full4 lanes from known
input XYZ, even when the input W values are unknown. Inverse W duplicates Y
through all these arithmetic operations; source-derived known W is legitimate.

## Original angle, distance bounds, and recursion

Before bounds, calculate:

```text
original_denominator = a * neg_bits(b)
original_numerator   = dot(u, neg_bits(v))
original_degrees = acos(recip(original_denominator) * original_numerator)
                   * RAD_TO_DEG
effective_maximum = maximum
if original_degrees > maximum and override_original:
    effective_maximum = original_degrees

two_ab = two_a*b
sum_squares = a2+b2
minimum_cos = cos(minimum*DEG_TO_RAD)
minimum_distance = length(sum_squares - two_ab*minimum_cos)
maximum_cos = cos(effective_maximum*DEG_TO_RAD)
maximum_distance = length(sum_squares - two_ab*maximum_cos)
```

The double sign reversal in original angle is confirmed raw:1588 XORs b's sign;
16a8 multiplies a by that result;16b0 XORs v's sign;16c4 dots u against it. Do
not substitute a conventional joint-interior-angle formula. The override is
local to an invocation. Recursive calls pass the **original** maximum argument,
then repeat the original-angle/override calculation.

The minimum-distance comparison is computed before the maximum-distance
estimate/refinement, but its branch occurs only after that complete calculation.
Thus neither the maximum cos service nor maximum length seed may be skipped
when the target is too close.

If ordered `minimum_distance > d` or ordered `d > maximum_distance`, take the
outside branch below. Otherwise calculate:

```text
derived_degrees = acos(clamp01(recip(two_ab)*(sum_squares-d2))) * RAD_TO_DEG
```

If ordered `minimum >= derived_degrees`, normalize the WORLD delta D-A without
a zero fallback, calculate `newD=FMA(unit(D-A), minimum_distance+epsilon, A)`,
commit full Cpri at1988, then recurse with budget-1. This includes equality.
Return that recursive result directly; Bpri is not prewritten on this path.

Outside branch1c64:

- If ordered `d > (a+b)` and effective maximum is ordered-equal180, compute
  `local_middle=FMA(unit(delta),a,localA)` and
  `local_end=FMA(unit(delta),b,local_middle)`. Transform both using the TARGET
  frame, starting from A and executing X/Y/Z FMAs. Commit Bpri at1d18, then
  Cpri at1d1c; return Unreachable. This is the only direct return1 path.
- Otherwise, if ordered `d > maximum_distance`, compute
  `FMA(unit(D-A), maximum_distance-epsilon, A)`, commit Cpri at1dac and recurse.
- Otherwise compute `FMA(unit(D-A), minimum_distance+epsilon, A)`, commit Cpri
  at1e00 and recurse. Each recursive call decrements budget by one.

All vector branch conditions reduce scalar splats with the all-true CR test;
unordered GT/GE/EQ are false. There is no generic range containment check.
NaNs may pass the distance rejection tests and continue into the reachable
candidate graph, depending on the declared numeric service behavior.

## Reachable candidate graph

After the derived-angle recursion gate, the remaining graph is:

```text
orientation_angle = acos(clamp01(recip(d)*delta.x))
if (localB.y < 0 and localD.y > 0)
    or (localB.y > 0 and localD.y < 0):
    orientation_angle = -1 * orientation_angle

angle_denominator = two_a*d
angle_numerator = (a2+d2)-b2
angle_input = FMA(recip(angle_denominator), angle_numerator, orientation_angle)
candidate_angle = acos(clamp01(angle_input))
```

The angle is added **inside** the final acos input. At1a74 raw112a0aee is the
standard VMX form `vmaddfp v9,v10,v1,v11`, meaning `v10*v11 + v1`. This is not
`acos(cosine_law_ratio)+orientation_angle`. Also retain use of absolute localD.y
in the sign test, rather than delta.y. Zero and unordered comparisons do not
negate the orientation angle.

Obtain combined sin/cos for candidate_angle, then independently for
`TWO_PI-candidate_angle`; do not negate/reuse the first sine result. Build each
rotation from its scalar cosine c and sine s, using bit-sign negation for s:

```text
X = [c, s, 0, c]
Y = [neg_bits(s), c, 0, neg_bits(s)]
Z = [0, 0, 1, 0]
P = [0, 0, 0, 0]
```

The table82fd6e90 and Z-only inserts1aec..1af8/1b50..1b64 prove these W values.
Rotate `localC-localA`, starting with an X FMA into P, then Y and Z FMAs.
Normalize both rotated candidates without zero fallback; scale each by a with
FMA into localA. Transform each local middle through the TARGET frame, starting
with A then X/Y/Z FMAs. Both candidates fully execute before selection.

If ordered `localC.y > localB.y`, choose the TWO_PI-minus-angle candidate;
otherwise choose the candidate-angle result. Store only selected Bpri at1c50;
leave Cpri unchanged; return Reachable. Native stack stores of the two candidate
world points are private scratch, not output commits.

## Numeric callback order

This order matters for fallible host numeric services. Pure arithmetic may be
expressed through meaningful helpers, but sequential whole-vector normalize
helpers must not reorder these observations.

| Stage | External primitive calls in source order |
|---|---|
| Initial lengths | dot(u,u), dot(v,v), rsqrt seed(first), rsqrt seed(second) |
| Plane geometry | dot(C-A,C-A), dot(D-A,D-A), seed(first), dot(cross(u,v),cross(u,v)), seed(second), seed(normal) |
| Frame inversion and angle setup | dot(originalForward,originalCofactor0), reciprocal seed(original determinant), dot(targetForward,targetCofactor0), reciprocal seed(target determinant), dot(u,neg_bits(v)), rsqrt seed(d2), reciprocal seed(original-angle denominator), acos service |
| Bounds | standalone cos(min), rsqrt seed(minimum squared distance), standalone cos(max), rsqrt seed(maximum squared distance) |
| Interior gate | reciprocal seed(two_ab), acos service |
| Reachable angles | reciprocal seed(d), acos service, reciprocal seed(two_a*d), acos service, combined sincos(phi), combined sincos(TWO_PI-phi) |
| Both rotated candidates | dot(first,first), seed(first), dot(second,second), seed(second) |
| Each recursive clamp | dot(D-A,D-A), seed, Cpri store, next invocation's entry assertion |
| Direct unreachable | dot(delta,delta), seed, Bpri store, Cpri store |

Each acos service itself makes one reciprocal-square-root estimate request.
Each standalone cos / combined sincos makes one nearest-even rounding request.
The two reachable candidate normalization refinements are interleaved, but
there are no retained stores between them. Both estimates run before selected
Bpri is committed. Budget0 performs no numeric calls, after its assertion.

## Fourth-lane authority

Use the existing `SkeletonRootVector { xyz, w: Option<f32> }` meaning throughout.
Unknown input W never prevents these known XYZ calculations and branch tests.
Dot scratch W may be zero only when it is discarded and never published as a
known lane. Full4 add/subtract/FMA/cross propagate W knowledge explicitly.

The two inverse frames and their local points recover known W from cofactor XYZ
via word merges. The angle rotations also have known W from scalar trig. Final
world output uses the original A and target frame full4 columns, so unknown W
can reappear at the final world composition. Preserve it. The recursion target
FMA similarly keeps unknown W if any required input W is unknown. Do not set
returned points to W=1 or vectors to W=0, and do not use the existing rigid
inverse as a replacement for this cofactor graph.

Output pointer aliasing is not part of the proposed native Rust contract:
original chain points are copied, and the two outputs have exclusive mutable
borrows. Actual Skeleton callers use separate local input/output storage. A
general C ABI aliasing emulator is neither needed nor proposed.

## Proposed native Rust scope

Use one reusable math module, e.g. `two_bone_ik.rs`, with semantic helpers for
plane frames, cofactor inversion, distance bounds and candidate construction.
Do not expose a register context or instruction operations. A suitable API is:

```rust
struct TwoBoneChain {
    root: SkeletonRootVector,
    middle: SkeletonRootVector,
    end: SkeletonRootVector,
}
struct TwoBoneLimits {
    minimum_degrees: f32,
    maximum_degrees: f32,
    override_maximum_with_original: bool,
}
enum TwoBoneIkOutcome { Reachable, Unreachable, Invalid }
enum TwoBoneIkError<N, R> {
    MinimumAngleAssertion { bits: u32 },
    Numeric(N),
    Trigonometry(R),
}
fn try_solve_two_bone<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    chain: &TwoBoneChain,
    middle_output: &mut SkeletonRootVector,
    target_and_end_output: &mut SkeletonRootVector,
    limits: TwoBoneLimits,
    remaining_calls: u32,
    numeric: &mut N,
    rounding: &mut R,
) -> Result<TwoBoneIkOutcome, TwoBoneIkError<N::Error, R::Error>>;
```

Use a bounded tail loop instead of native recursive stack frames: the original
chain and limits remain fixed; only Cpri and remaining budget change. Re-run the
entry assertion on each iteration and retain every target/output commit before
the next fallible call. Do not return a newly constructed pair atomically: that
would erase the native recursion and failure prefixes.

Reuse the existing dot/estimate/refinement/acos/combined-trig graphs. Promote
the already audited standalone cosine to a shared numeric helper, rather than
copying or substituting the combined cosine. A private cofactor inverse helper
is required because the existing root inverse is a different graph. Typed full4
vector operations can be local semantic helpers or shared through root-owned
interfaces; there is no need for a new competing public matrix representation.

After this dependency is implemented, integration still requires the original
IK owner/modes/attributes/contact path and the PrePhysics caller's line-frame
rotations, angle-range clamp and ordered actual mapped-row writes. Those are
enumerated in the previous packet. Solver completion alone must not be reported
as active General IK completion. The post-physics caller can reuse this math
later, with its independently recovered calling chronology.

## Focused implementation verification

1. Entry minimum90 equality, next f32 above90 and NaN; assertion precedes zero
   budget and both outputs remain untouched on the assertion error.
2. Budget0 restores end then middle; short segment restores only middle; length
   epsilon equality does not take the short exit. Initial callback batching.
3. Reachable bent non-axis-aligned chain with frozen source-form oracle; include
   a case distinguishing cofactor inversion from rigid transpose and a case
   distinguishing angle addition inside acos from an angle sum afterward.
4. Signed orientation test uses localD.y; final candidate tie chooses the first
   candidate. Both combined trig calls and both candidate dots/seeds execute.
5. Too close, too far and interior-angle equality recursive paths; budget1 must
   demonstrate temporary target commit followed by original end restoration.
   Inject the next invocation's numeric failure to expose the committed target.
6. Maximum180 direct unreachable versus nearby maximum; end computation starts
   from the already rounded local middle. Freeze B and C full4 outputs and order.
7. Override false/true with original angle above limit; recursive invocations
   recompute the override against the original argument, without retaining it.
8. Both cos services and maximum-distance seed still run on too-close input;
   inject a failure at each numeric/trig boundary and verify output prefixes.
9. Mixed/noncanonical/NaN W and unknown W: XYZ unaffected by W-only changes;
   inverse local W is known and duplicates Y; final world unknown W remains
   unknown. Full4 reachable/unreachable outputs need independent frozen values.
10. Collinear chain, coincident desired target, NaN/Inf lengths and singular
    frame: preserve the unsafe arithmetic graph, all ordered gates, and explicit
    host numeric limitations. Do not silently turn them into a new Invalid exit.
11. Frozen cancellation cases distinguish ordinary products/subtractions from
    FMA in d2, angle_input and both world compositions. Trig fixtures must use
    the source polynomial graphs, not a libc sin/acos oracle.

## Confidence and remaining evidence limits

**High** for the full control flow, scalar-wrapper ABI, constant initialization,
true VMX operands, complete arithmetic dependency graphs, full4 merge/rotation
lanes, output ordering and callback ordering. Raw original-angle and nested-acos
operands are explicitly recorded because they differ from convenient textbook
substitutions. Independent source review is requested separately.

**Medium** for exceptional host numeric results: the inherited estimate, dot
reduction, VMX min/max NaN selection, denormal and VNMSUB-zero-sign boundaries
are still declared limitations. There is no new reference runtime trace and
no original-data solver corpus in this read-only packet. No extra geometric
callee remains unidentified for this solver; the unresolved full-active-IK
dependencies are its actual owner/caller stages, not a missing generic IK math
substitute.
