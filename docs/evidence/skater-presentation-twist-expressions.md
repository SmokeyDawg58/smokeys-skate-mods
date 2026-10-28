# Actual twist-bone expression correction

Source: `ComputeExpressions827d9db0`, `ComputeExpressions_Individual827d99a0`, `TwistThatBone827d9540..9998`. Fresh disassembly was decoded with actual VMX128 operand extensions; displayed high register names alone are not trusted. The new unregistered files `skater_presentation_twist.rs` and `skater_presentation_expressions.rs`, with test children, draft this next mandatory producer. Parent registration and the actual synchronized allocation mutation seam remain root-owned.

## Ownership and live attributes

The individual borrows actual bind SQT and replay[2*i]. The existing InitAnimation owner retains fourteen indices, in order: LeftArm, LeftArmTwist, LeftForeArm, LeftForeArmTwist, LeftForeArmTwist1, LeftUpLeg, LeftUpLegTwist, followed by the corresponding seven Right indices. The same stored mapping drives the draft; there is no hardcoded numeric bone index or 36-to-148 overlay.

The four ordered ratio reads are Arm38317705e5a4571a at9a14, Forearm246fc198b6f1624d at9a6c, ForearmSecond5cdb40ca8c5a5044 at9ac4, and UpperLeg3eb708886e6e4607 at9b1c. All four are sampled before any twist mutation; right-side calls reuse those samples. They are plain Attribute Instance queries at index0, not cached-layout members. Missing attribute data follows DefaultDataArea(4); a missing global allocation remains an error.

`PresentationTwistAttributes::bind_global` retains the actual selected GlobalAttributeCollection+d8 instance, classbabb3ec110849e08, collection2db7709645a19ce8, constructor828b27cc..2824. Root must mount this once in the real global owner and provide it for each individual call. The optional reference adapter checks the actual available owner on every read. Immutable Vault captures do not claim singleton replacement/editor reload behavior.

Eight calls follow in source order: LeftArm9b84, LeftForeArm9bd0, LeftForeArmTwist1 9c1c, LeftUpLeg9c68, RightArm9cb4, RightForeArm9d00, RightForeArmTwist1 9d4c, RightUpLeg9d98. The enclosing ComputeExpressions loop visits all mCount entries without a model-presence bypass.

## Twist arithmetic and temporal reads

1. Read actual bind-source quaternion955c and dot4 it with itself9564. Only then read current source quaternion9570. Conjugate uses sign-bit XOR in XYZ, preserving W. Actual reciprocal estimate95c4 and two negative-after-round VNMSUB refinements yield inverse bind-source. Compose inverse-bind-source with current source at95f4..9620. Quaternion XYZ uses native cross products, then current*inverse.W plus cross, then inverse*current.W; scalar W is separately rounded W product minus the source dot3.
2. Rotate the original X axis using the source double-cross graph9624..9658. This is not a normalized quaternion assumption. Compute Y*Y9668; issue reciprocal estimate of X966c; FMA Z*Z+Y²9670; issue rsqrt estimate9678 and two refinements. The length is squared*refined inverse root, selected to positive zero when squared==0 at96f8. The X reciprocal uses only one refinement. Its product with length includes the source fused +0 at96fc before the actual XMVectorATan9700.
3. After atan, issue the length reciprocal estimate970c, even at zero. The first reciprocal refinement precedes XMVectorSin9760; the second follows that call at9770/74. Atan quadrant adjustment uses OR of the numerator sign into original pi/half-pi bits, ordered denominator<0 and equality-to-zero masks. Half-angle multiplies by .5. Inverse-length*sine builds the transverse factor; XMVectorCos9780 is a distinct later call. The inverse swing quaternion is [0, direction.Z*factor, sign-flipped direction.Y*factor, cosine]. Compose it with the retained relative quaternion97f0..9818.
4. Both signed representatives are evaluated: atan(twist.X / twist.W) using a single reciprocal refinement at9824..983c, then atan(sign-flipped X / sign-flipped W) at9878..989c. Each calls the actual existing atan graph, including all its estimate callbacks. Only after both are complete are the quadrant masks applied; original W<0 selects the sign-flipped representative at9924. Multiply the selected half-angle by the live ratio9928.
5. XMVectorCos9930 precedes XMVectorSin993c. Their results form [sin,0,0,cos], stored to the actual destination quaternion at9950. Only **after that store** is bind-destination loaded9954. Compose bind-destination with this correction and store the final result9984. Consequently a checked missing bind-destination or final numeric failure must retain the intermediate store; earlier individual/bone writes are not rolled back.

Scale and translation are not changed by TwistThatBone. The new API borrows actual SQT slices and performs only quaternion writes. Root's allocation mutation guard must mark successful publication/generation only after the full correction completes, while retaining native partial writes on failure.

## Reused numeric implementations and unresolved boundary

The draft reuses existing `skeleton_query_rotation::try_atan` for XMVectorATan82581658, `reckoning_sin_cos::sine::try_sine` for the separate XMVectorSin824b0060, and `reckoning_sin_cos::try_cosine` for XMVectorCos824b0130. It does **not** reuse the combined SinCos helper: separate sine constructs odd powers successively from the previous odd power times squared angle. Fresh source confirmed that distinction. Four-lane bind norm uses the local PresentationMatrixNumeric contract already drafted for general inverse; dot3 and estimate contracts remain the existing ReckoningNumeric, rounding remains ReckoningSinCosNumeric.

Source review noted existing cosine/atan reciprocal helpers with algebraically negative FMA spelling rather than negative-after-round VNMSUB. Those shared helpers are not edited in this draft. Root was notified to independently assess/correct them before claiming source arithmetic closure.

**Material startup numeric limit:** the source has no zero-transverse-length bypass after selecting length0 at96f8. It still estimates 1/length at970c and performs both refinements across the sine call. Ordinary host reciprocal0=Infinity leads to NaN in these steps, including a bind-equal-current fixture. The draft preserves that observed graph and does not insert an identity correction, skip ratios equal to zero, or canonicalize nonfinite values. Native VMX estimate/nonfinite behavior must be established before this result can be admitted as a valid first-frame render pose. This is a concrete validation blocker, not a claim that the native game emits NaN.

## Saved validation and integration

Seven unregistered tests cover removal of nonzero swing before scaling X twist, the final intermediate-store boundary, a late missing bind destination, explicit host zero-length behavior, all eight destinations using a shuffled fourteen-index mapping with opposite right-side twists, all four live reads preceding mutations, and missing-global versus missing-attribute distinction. Source/conversion ordering and payload comparisons are checked without a GPU or fabricated Actor pose. Tests are saved but not run; root owns Cargo and independent review.

Register `move_root`, `twist`, and `expressions` under the actual presentation animation owner only after their source review. The expressions entry is `try_compute_expressions(bind, replay, &actual_twist_indices, attributes, numeric, rounding)`. It consumes the actual bind/replay allocations via a synchronized mutable range borrow. No shared parent, global owner, command admission, or rendering file is changed by these drafts. Subsequent actual SetMatrix/PoseWithMatMult/ExtractMatrix/Sync and model palette publication remain the next queue integration stage described in the UpdateBodies packet.

## Independent zero-length re-audit

The second raw pass confirms the following extended register operands. D is the destination, A/B the decoded source registers; accumulating instructions also consume the old D. v124 contains the selected transverse length, v126 is one, and v121 is repurposed only after the half-angle multiplication. The intervening XMVectorSin uses low vector registers and preserves these nonvolatile values.

| Address / raw word | Decoded operation |
|---|---|
| 970c / 1800e633 | VREFP D0, B124: estimate 1/length |
| 9718 / 14bce6f7 | copy length124 to37 |
| 9720 / 149ef6f7 | copy one126 to36 |
| 973c / 17c002d4 | copy seed0 to62 |
| 9744 / 149e2975 | VNMSUB D36, A62, B37: negative rounded(seed*length-one) |
| 974c / 14800114 | VMADDCFP D36, A0, B0: seed*oldD36+seed |
| 9758 / 172422fd | copy first inverse36 to121 |
| 9760 / 4bcd6901 | XMVectorSin, result v1 |
| 9764 / 17b9cef7 | copy inverse121 to61 |
| 9768 / 179ef6f7 | copy one126 to60 |
| 976c / 1779cef7 | copy inverse121 to59 |
| 9770 / 179de177 | VNMSUB D60, A61, B124: negative rounded(firstInverse*length-one) |
| 9774 / 1779e4f5 | VMADDFP D59, A121, B60: firstInverse*residual+oldD59 |
| 9778 / 179b08bc | VMUL D124, A59, B1: secondInverse*sine |

There is no missed zero select, swapped length operand, or preserved finite denominator in this sequence. The prior select96f8 explicitly supplies positive zero when transverse_squared==0. The cached external Xenia directory available to earlier reversing sessions currently contains no source files, so it cannot establish additional behavior in this audit.

Primary architectural verification: the [Freescale/NXP AltiVec Programming Environments Manual, Rev.3](https://www.nxp.com/docs/en/reference-manual/ALTIVECPEM.pdf), sections3.2.4.2 and3.2.4.3 (printed3-13), specifies signed infinity for reciprocal estimates of signed zero and a quiet NaN for infinity multiplied by zero. Section3.2.1.2 limits the non-Java mode distinction to flushing denormal inputs/results; it provides no finite-reciprocal or infinity-times-zero exception. Section4.2.2.1.1 warns that Newton refinement needs an explicit guard where the estimate is infinite. These are architectural rules, not a Xenon hardware trace.

Therefore exact zero length reaches a NaN residual at9744 under the documented baseline semantics. It propagates through the second refinement and factor. This is not remedied by a more accurate ordinary reciprocal seed. There is no evidence supporting an identity fallback, finite seed clamp, skipped zero ratio, or extra source gate; no code correction is proposed.

The remaining **actual startup** question is narrower: do the retained original bind/current quaternion pairs produce exactly zero length through this rounded graph? Mathematical bind equality does not establish bitwise zero after reciprocal refinements and fused cross products. The synthetic identity fixture proves the corner case, not that original startup reaches it. Minimal next validation is to record the length operand of each actual twist call's sixth reciprocal-estimate request (after bind-norm, direction-X, and the three first-atan estimates), together with source/destination indices, using the original synchronized bind/replay owners. This distinguishes a reached native degenerate input from a hypothetical corner without changing arithmetic or fabricating a pose.
