# Ground.Update to retained Reckoning

Status: adapters registered and existing numerical body refactored behind late reads; parent validation pending. Confidence high for caller/ownership/source-read mapping; host numeric reduction/estimate policy remains explicit, not a bitwise VMX claim.

## Consecutive boundary

Ground vtable8234a338+4 dispatches Update82da0de8. When spawnCoordinatesSet+f6 is false, read processed numWheels+90c; only signed >0 reads raw processed deck translation X+70, stores validSpawnX+b4, reads Z+78, stores flag+f6, then validSpawnZ+b8 (0e18..34). No W is consumed. At0e50 zero full antiFlipTorque+90;0e54 clear requestPushStop+f2;0e58 capture timeInState+a8; clear nudge+f3, pinning+f7, timeToSkitch+bc=-1, grabObject+f9. Raw0e74 BLE4099 is NOT GT, including NaN: only time>float3d4ccccd enables SAME Skeleton truck-tilt+3e41 at0e7c.

0e94 calls actual Reckoning.UpdateGroundStates82de0408 with pointers to processed ground+1d0, dynamicUp+210, deck.Z+60. The wrapper stores full dynamicUp to Reckoning+1a0 then full heading to+1e0 before UpdateGroundStatesFixedHeading82ddd468. This is not the animation-state fixed-heading entry.

The new world prefix ends immediately after this call. UpdateHandplants82d8d258 at0e9c, Skeleton.UpdateGroundSkateboard0ea8, Pumping1014 and Ground.UpdateSkateboard101c remain successors, not implied completed. Grinds being parked does not authorize omitting Handplant Reset/IK effects.

## Owners and reads

Reuse actual Ground fields/bindings, PhysicalPlayerAssemblyOwner.reckoning, the same processed Arc and existing Reckoning filters/body spin/attribute instances. No per-call new state, Vault binding, constant cvar provider or query result.

The existing grounded algorithm must be refactored in place behind fallible late readers, retaining controlled snapshot compatibility for leaf tests. Wrapper dynamic/heading writes survive later input failure. BodySpin consumes physicsBodySpin+9f8 before NormalizeSafe COM+2f0; ProcessData produces this full-known animation COM, distinct from hook-to-hips+2e0 and COM delta+300. Ground.Y can be read without W before its later complete filter input. Deck speed, wheel count, balance, absolute body speed, DT and individual flag bits are read only at their original sites/branches.

Toolkit owns raw deck+40 and effective+80 matrices; its live Part6 transform produces full known lanes. Prior-output ground and dynamic vectors retain provenance. A source full-vector retained store cannot establish missing W. Dot3-only scratch preserves known W and uses unknown scratch only without publishing it; effective-Z full W is needed only if the later cross-product consumer executes. No unrelated full flag-word admission.

## Live cvars

| Slot | Ref | Initializer | Static table | Section / label / title | Default |
|---|---|---|---|---|---|
|86|830db76c|82f98038|82fd5d90|Physics_Reckoning_BodySpin / kBodySpinDerivativeScalar / BodySpin Derivative|3fc00000 (1.5)|
|87|830db3f8|82f98098|82fd5d94|Physics_Reckoning_BodySpin / kBodySpinBlend / BodySpin Blend|3f800000 (1)|
|88|830db984|82f980f8|82fd5d98|Physics_Reckoning_BodySpin / kDerivativeThreshold / Derivative Threshold|3dcccccd (0.1)|
|89|830db374|82f981b8|82fd5da0|Physics_Reckoning_BodySpin / kBodySpinInputBlend / Input Blend|3e4ccccd (0.2)|
|90|830dc7a0|82f9c070|82fd6064|Physics_Reckoning / kShowReckoningInfo / Show Reckoning Info|false|
|91|830dd2d4|82f9c110|82fd606c|Physics_Reckoning / kShowCoffinInfo / Show Coffin Info|false|
|92|830dc078|82f9c0c0|82fd6068|Physics_Reckoning / kShowBodyTiltInfo / Show BodyTilt Info|false|

Raw float initializer r10=0 at98074/980d0/98134/981f4 confirms null callbacks (decompiler prototype mislabels these). GUI ranges are0..120 for derivative scalar and0..1 for the other floats; startup deserialize does not clamp. Native static order:86,87,88,89,90,92,91. BodySpin resolves derivative scalar, captures old histories, resolves blend, then separately reads both captured values; input blend resolves afresh twice. Adapter retains actual target Arcs rather than replacing the resolve/read pair with one eager getter. Existing explicit enabled-debug unsupported boundary remains honest until those drawing branches are reconstructed.

Parent owns independent review and all Cargo/original tests. Draft tests cover defaults, live mutation, relinking and retention of a previously captured target. Late-read tests now assert dynamic store survives missing heading without BodySpin entry; missing COM preserves actual completed BodySpin history/input mutation.

## Pending independent numerical audit

Parent raw review identified the pre-existing shared cross_full4 expression using a negated multiplicand FMA, while source cross uses VNMSUB final negation. Signed-zero behavior may differ. This batch deliberately preserves that shared numerical implementation pending exact raw operand mapping and NormalizeSafe review; it does not claim numerical closure from wiring the existing algorithm. Relevant code is reckoning_numeric.rs::cross_full4; this is separate from the accepted source-late adapter changes.


## Retained W propagation correction

The original same-pair run reached the Ground wrapper but exposed unknown attached
Part.GetTransform W in the processed deck. The packed source at 826c3480..35a4
uses RF/stasis/inertia/tag words; the existing host XYZ transform does not prove
those original address-image lanes. See simulation-xenon-address-image.md.
Toolkit now consumes the shared typed Part getter: detached cache lanes survive
verbatim; attached XYZ remains available with unknown W.

Reckoning matrices/vectors and the five SAME PID histories now retain
SkeletonRootMatrix/SkeletonRootVector. Wrapper0408 dynamic and heading copies
preserve knowledge instead of rejecting a source vector solely for unknown W.
All late reads remain at the existing positions. Vector PID8f20..90bc uses the
same rounded-product/FMA graph in one body, including W when known. Unknown W
propagates through zero coefficients (NaN/infinity cannot be excluded) and is
not erased by a value override that leaves histories intact.

Normalization/clamp share a lane-carrier body with their existing raw controlled
callers. Dot3 consumes XYZ only; its temporary operand retains known W or uses
scratch0 without publishing it. NormalizeSafe's actual fallback can establish
known W; successful normalization scales known W and preserves unknown W.
Cross XYZ uses the unchanged existing numerical graph; unknown cross W is never
published from scratch. Typed matrix composition propagates outer W knowledge;
the existing rigid inverse consumes XYZ and establishes source basis W0.
The previously recorded VNMSUB signed-zero audit remains open and was not mixed
into this representation correction. Host arithmetic is still not a claim of
bitwise Xbox parity.

Focused tests retain existing all-known raw-lane oracles and add unknown-W live
grounded completion, same-filter history persistence under zero coefficients,
and actual NormalizeSafe fallback knowledge. Validation is pending root gates.

Parent independent type-refactor review: retained vector scalar/FMA operations
use Option::zip/map for W and preserve XYZ order; no zero-coefficient shortcut.
PID integral, derivative-input, prior-output-delta and last-target stores remain
in the prior source order. Generic normalize/length/clamp share their previous
numerical bodies. NormalizeSafe returns the actual fallback carrier on its
source degenerate branch. Cross uses the previous scalar graph and marks W
unknown if either W is unknown. Matrix composition may establish known W only
from its actual outer coefficients; inner W is not a native coefficient.
Clippy38 exposed test type migrations and narrow lint findings, being fixed;
full workspace/original validation of this refactor remains pending.
