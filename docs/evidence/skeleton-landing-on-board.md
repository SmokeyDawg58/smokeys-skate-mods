# Skeleton landing on skateboard adjustment

2026-09-08. Accepted68261/aeb660 EXIT0 after records checkpoint33408. Strict2813
units/six docs/all nine original core/fmt/Clippy/diff and independent
source/code/tests/shared/corpus review PASS. It is not a native Tick trace.

## Source ownership and activation

`Skeleton::UpdateLandingOnSkateboardAdjust` is called only from `ProcessData`
at82b5f1a4, after Offboard atf17c. The caller reads retained processed8cc:
CanLandOnSkateboard bit200 selects this helper if OnDeck bit40000 is clear.
CanLand+OnDeck skips both landing and GrindAir; absent CanLand selects GrindAir.

The helper first compares zero > retained Skeleton COM velocity+3d20.Y with an
ordered vector predicate. Signed zero, positive values and NaN skip. Only then
does it read processed8d0 bit10 (LandOnSkateboardFlipBoard); set skips. Neither
inactive exit reads attributes/cvars or writes retained state. Raw VA at41818
is60, despite the decompiler printing92; use the corrected VMX128 decoder.

This COM velocity is the prior completed UpdateRecords derivative, produced
after collision and volume states. It is not processed+260, which the separate
later UpdateLandingAdjust reads with processed+220 Up. Constructor zero and
Reset/TeleportReset preservation were independently verified. The actual owner
passes retained velocity only to this helper; processed velocity remains separate.

Active inputs are the current mapped Skateboard volume0, same-owner WorldToAnim,
processed DeckTransform+40, LandingOnDeckAdjust+360, and OB_TimeToLand+a0c.
WorldToAnim is constructor identity or the sole rigid-inverse producer; both
establish all four W lanes. Other root matrices may retain unknown W.

## Curve, geometry and retained stores

Compute desired rotation from WorldToAnim * processedDeck. Rotate the adjustment
vector through WorldToAnim using XYZ coefficients and full-four-lane multiply/
FMA order. Replace its Y with zero unless desiredDeck.P.Y > mapped.P.Y, then
use that difference. Desired P becomes mapped.P + adjustment, full four lanes.

Fetch original physics_animation.default key6aea4c4efbba747b on each active call.
The native type is PointNegGraphData8,80 bytes: metadata0..10 is ignored; pass
x[8] at10 and y[8] at30 to PointGraphEval824e2c40 with TimeToLand. Null pointer
uses DefaultDataArea50; root read its first80 bytes at83059d30 and confirmed all
zero. Runtime provider reruns the retained Vault lookup, preserves inheritance
and unsupported-data errors, and checks payload size before decoding big endian.
It does not freeze authored tuning into runtime constants. The provider retains
separate global and Skeleton-owned instances, although both bind the same
immutable default here: global+CC at82b40234 differs from Skeleton+14 constructed
at82b56e00. Native editor mutation and complete constructor chronology remain open.

Interpolate mapped to desired, then calculate offset as interpolated OUTER *
rigidInverse(mapped) INNER. Query/interpolation math completes before debug and
negate-Y cvar reads. ShowLandingAdjust reference8308e450 reads target+20/bytecd;
its surrounding apparent draw calls resolve to BLR stub825187c0. The intervening
GetDebugOverlay82581e40 still requires its actual singleton/availability service.
NegateY reference8308ea74 has default1 and controls multiplication by +/-1.

Save signed vertical offset locally, set full-transform P.Y to+0, write the full
matrix and full JustSet, then sample frames-to-blend reference8308e9d4. First
failure retains matrix/flag and old count. Sample that cvar again before writing
the saved Y, Y JustSet and Y count. Second failure retains first full count and
P.Y zero. The following original offset stage applies the result to mapped0 and
the same four IK targets before animation record/COM/root consumers.

## Affine interpolation and rotation query

InterpolateMatrix44Affine first computes each full4 P lane as rounded(to.P*t)
then FMA(from.P,1-t,that product). It calls SLerp and overwrites SLerp P with
this value. Translation extrapolates even when rotation takes an endpoint.

SLerp copies To rotation for ordered t>=1. Otherwise it builds To rotation *
transpose(From rotation), with ZERO inner translation but all source zero-term
FMAs still executed. QueryRotate computes axis, angle AND center before the
lower endpoint gate0>=t. No identity or t<=0 shortcut may omit Query callbacks.
Small angle below3d0efa35 blends columns and independently normalizes them;
ALL three dots precede ALL three reciprocal-square-root estimates. It is not
Gram-Schmidt normalization. Larger angles use the accepted combined SinCos
graph and full source axis rotation. NaN follows the original ordered gates.

QueryRotate raw XYZ axis is[Y.z-Z.y,Z.x-X.z,X.y-Y.x]; positive-length W comes
from unwritten stack data and remains unknown. Dot3 and rsqrt execute even for
zero; two refinements and squared*refined give length, with final zero selection.
Positive length normalizes via reciprocal with two refinements; else axis is
known zero. Axis stores before later math. TraceMinus1 is((X.x+Y.y)+Z.z)-1;
its reciprocal uses ONE refinement, then ratio=FMA(length,reciprocal,0).

ATan always runs. Its reduction numerator is FMA(y,3f3b67af,y)-1, not a rounded
y*sqrt3 substitute. Three reciprocal estimates run even when masked by branch
selection; first-refinement self-equality controls NaN fallback. The recovered
rational polynomial uses its source FMA order. Angle adds signed PI for negative
TraceMinus1, then overrides with signed halfPI if TraceMinus1 equals zero.

Degenerate-axis selection requires ordered length<=bits00200000 and
TraceMinus1<=0. Strict diagonal selection preserves the original ties. The
principal candidate is squared(1+diagonal), not textbook2*(1+diagonal); other
components are symmetric sums. Source permutations establish W=X. Full4
normalization completes before replacing the earlier axis.

Center always computes c=axis cross P, d=c cross axis,
s=refinedReciprocal(2-TraceMinus1)*(0.5*length), then FMA(d,0.5,c*s), including
identity cases with NaNs. Axis, angle, optional replacement and center stores
retain their source failure prefixes. SLerp discards center only after it runs.

## Source hashes and current confidence

Spans have exclusive ends. Hashes cover original bytes read through IDA.

| Function | Span | SHA256 |
|---|---|---|
| Landing adjustment | 82b417c8..82b41c80 | edae581c4f8d98406cf61e6805d48d44f58318ff2ccc7c4654aab4a777d7290b |
| InterpolateMatrix44Affine | 82e4acf8..82e4ad7c | c50cbfc5a884ecac6061daeb54813e1da21bd8edbaedd0cd233cf1114f2007b5 |
| SLerp | 82b4b418..82b4b86c | 834eddfd5e4f2326c8b70a8d6e0933a53e0148e0d6df1ad7b3eaeb44769ca80d |
| QueryRotate | 82b4b128..82b4b414 | 97253a46872350088ecf1fbacc9373d491558f4d887319ee60795d7d29848862 |
| ATan | 82581658..82581808 | c4a6190eae0dcf4b16d345587ce0bfc43b946a069dd38a0f422ba13a0cb41fa9 |
| Degenerate axis | 82806b80..82806e24 | fd8fa364c88588b5964dfc3a4945120cab2133911392f2b5993be10341859a8f |

Independent production review passed for all three new modules.
Source layout, literal bits, gates, call/store order: exact. Host arithmetic:
high with explicit estimate/reduction, denormal, exceptional payload, fused-zero
and FP-status boundaries. Positive query-axis W remains typed unknown; it is
not made homogeneous zero. Root core11592/b917ef passed2585 units, including
30 new kernel tests and four new pipeline/Vault tests. Independent review passed
all tests, shared integration and the added corpus helper. Final strict68261
passed. The first original-data run found an old unsupported-landing expectation
in the preceding offset fixture; it now tests selected inactive landing followed
by a deliberate later debug failure, preserving the prior state/count assertions.

The active corpus runs after its second existing solve/root preparation and
before its second records update, when the first actual downward COM derivative
is still retained. An opposite-sign processed velocity proves source ownership.
It chooses an interior partial blend from the original authored graph, checks
nondegenerate rotation with an independent f64 geometric oracle (relative
tolerance0.002), tests both blend-count failure prefixes, then verifies all24
mapped/record rows, four IK targets, COM/history and root input against independent
source-form consumers. The next post-physics records consume those adjusted rows.

This does not connect native IK solving or drive updates. Root inspected the
sole SkeletonDrives.Update caller in GeneralUpdate82b5ba88: UpdateHookPositions
atbb18 and possible ResetSkeletonbb2c precede UpdateIKDatabb64/IK.Updatebb6c
(except processed state8e8==2be), then Drives.Updatebb7c reads buffer+3040.
Those prerequisites and full scheduling remain open; current raw-drive corpus
work remains explicitly controlled.
