# UpdateBodies and full render-pose continuation

Fresh source review: `Sk8::SkaterPresentation::UpdateBodies827de988`, `MoveRootToHips827d3a10`, `MoveRootToHips_Individual827d35d8`, `ComputeExpressions827d9db0`, `ComputeExpressions_Individual827d99a0`, and `TwistThatBone827d9540`. This packet records the path after actual Restore/Read; it does not admit a body36 overlay or a named-trick pose as full-rig publication.

## Ordered owners and calls

1. UpdateBodies asserts Q4+Q3<120 and T<120, then obtains actual `SkaterFull` playback index0 at de9f0. Its hierarchy and ACS are retained for both passes. It resolves actual Face_Offset0 metadata before BeginCommandSet dea80.
2. For each `i < mCount`, it uses the model index in that interpolated entry's high flag nibble. Only a present actual model executes BeginCommands, UpdateBody, ExtractSQT into **replay[2*i]**, and EndCommands. EndCommandSet deb14 and Sync deb1c finish the entire first pass.
3. MoveRootToHips and ComputeExpressions each loop **all mCount entries**, without the preceding model-presence test. Missing/nonpresent model slots cannot justify skipping these corrections or inventing an initialized replay buffer.
4. The second ACS pass again gates each entry on actual model presence. It SetSQT from the same corrected replay[2*i], SetMatrix from actual SkaterPresEntity+214 `InvBindPoseBuffer` (raw deb88), PoseWithMatMult deb94, ExtractMatrix into that entry's retained interpolated matrix allocation, EndCommands; then EndCommandSet debc8 and Sync debd0.
5. All count entries then copy retained mHipsTransform into matrix slot0, debfc..dec24. SceneManager singleton and actual PoseSaver are queried unconditionally at dec38..dec70, before the optional saving branch. Model StartJobs/EndJobs and per-part bone remapping/packing remain later consumers.

The actual InitAnimation draft already retains the full hierarchy, bind allocation, eighteen replay allocations, nine interpolated allocations, named hips/skateboard indices and fourteen twist indices. Shared SQT ranges and category2 affinity remain separate pending parent amendments. An allocation mutation must borrow the same synchronized range and preserve earlier writes on error while withholding successful generation publication until completion.

## MoveRootToHips draft

New unregistered files: `skater_presentation_move_root.rs`, its test child, `skater_presentation_matrix_inverse.rs` and its test child. Entry `try_move_root_to_hips(&mut [AndaleSqt], hips, skateboard, &mut hips_transform, &mut numeric)` borrows actual replay storage; it creates no retained replacement pose. Root owns registration and allocation mutation integration.

The hips record is addressed at 3628. Source loads quaternion+10 at3648 and translation+20 at3654, **never its scale**. Quaternion components are first multiplied by gSqrt2s at8227e8e0, four copies of bits3fb504f3. It computes rounded pair products and `.5 - component*component` via VNMSUB, then pair sums for the diagonal. XYZ masks clear basis W and translation W becomes one. This graph differs from the existing ACS `animation::sqt_to_local_matrix`, which forms quaternion products before doubling, so that helper is not reused here.

The source stores mHipsTransform translation3690 and axes3700/3704/3708, then mutates the **same hips SQT**: identity rotation370c, scale all ones3710, translation[0,0,0,1]3714. It subsequently loads the skateboard SQT, likewise ignoring scale and forcing matrix translation W to one. The actual general inverse call37c0 targets **82600f38**, not a rigid transpose inverse. It composes inverse(hips)*skateboard using the initial multiply and three successive four-lane FMAs. Skateboard scale is reset3820; the final quaternion candidate graph uses four rsqrt candidate lanes and two refinements, then strict trace/diagonal selection, matching the existing shared `world::try_simulation_quaternion_candidate`. There is no extra normalization. Final rotation39f4 and translation39f8 update the same skateboard record.

An invalid later skateboard index or fallible inverse service leaves already stored hips/reset state intact. A candidate estimate failure leaves the already reset skateboard scale intact. Actual production indices come from the retained full hierarchy; bad-index errors are checked host admission, not an alternative native path.

## General inverse arithmetic

`rw::math::vpu::Inverse82600f38` is a full 4x4 inverse. The new helper retains cyclic row cofactors, sign-bit XORs and the transpose. It does not substitute the existing Skeleton affine/rigid inverse.

For a lane r, the permute1230 family selects n=(r+1)%4, m=(r+2)%4 and l=(r+3)%4. Each 2x2 minor first computes `first.mul_add(second,+0)` and then negates the rounded fused subtract of the other product. Cofactor accumulation begins with a separately rounded multiply followed by two FMAs. Source cofactor column families are B*(C,D), C*(D,A), D*(A,B), A*(B,C), with even/odd sign-bit XOR masks alternating. This preserves source operand placement instead of reordering a generic determinant expansion.

| Dependency | Source |
|---|---|
| CD minors and first cofactor | 00fb0/00fcc/00fd4,0100c/0101c/01020,0103c/01044/01058 |
| DA minors and second cofactor | 00fc0/00fdc/00fe4,01018/01028/01030,01040/0104c/01060 |
| First cofactor sign XOR, dot4 determinant, reciprocal seed | 01068/0106c/01070 |
| AB minors and third cofactor | 00fec/01004/01038,01048/01054/010d4,01088/010e0/010f0 |
| BC minors and fourth cofactor | 00fc8/00ff4/00ffc,0105c/01064/01098,01090/010e4/010f4 |
| Two refinements, merge transpose, multiply/store columns | 0107c..01160 |

Each inverse column uses the same scalar-splat reciprocal seed and two repetitions of residual `-seed.mul_add(det,-1)`, then `seed.mul_add(residual,seed)`. The native function stores a determinant stack output at010c8; this caller does not read that local, so the draft returns only the used matrix. No singularity gate or identity repair is introduced.

**Numeric dependency:** existing ReckoningNumeric has dot3 but not dot4. The unregistered child therefore defines local `PresentationMatrixNumeric: ReckoningNumeric` with an explicit four-lane determinant reduction. A caller must supply that policy; it is not implemented by silently ignoring W or changing the shared trait. Host dot reduction, estimate seeds, denormals, NaN payloads and FP status remain explicit numeric limits. The quaternion helper similarly consumes all four candidate estimates even when some are later unselected.

Six saved tests cover a nonaffine matrix with nonzero bottom row against an independent rational inverse, singular propagation, numeric failure timing, same replay row changes with unrelated row preservation, ignored scale/translation-W, and partial hips writes after later failures. They are uncompiled/unrun drafts; root owns Cargo and independent verification.

## Mandatory following expressions

ComputeExpressions captures actual bind SQT and corrected replay[2*i]. For each individual it performs four ordered **live** GlobalAttributeCollection queries through instance+d8, each singleton read retained separately. Full raw keys, index0, and read order are:

| Ratio | 64-bit key | GetAttributePointer |
|---|---|---|
| Arm | 38317705e5a4571a | 827d9a14 |
| Forearm first twist | 246fc198b6f1624d | 827d9a6c |
| Forearm second twist | 5cdb40ca8c5a5044 | 827d9ac4 |
| Upper leg | 3eb708886e6e4607 | 827d9b1c |

Missing attribute pointers use actual DefaultDataArea(4); a missing global instance is an assertion boundary, not a ratio-zero shortcut. GlobalAttributeCollection constructor828b27cc..2824 allocates the retained 16-byte Instance and binds class **babb3ec110849e08**, collection **2db7709645a19ce8**, then stores instance+d8. This selected global binding is not currently retained by the presentation owner.

All four ratios are read before eight TwistThatBone calls: left arm9b84, left forearm9bd0, left forearm second9c1c, left upper-leg9c68, then corresponding right sides9cb4/9d00/9d4c/9d98. Each uses actual bind source quaternion, current source quaternion, bind destination quaternion and mutable current destination quaternion. TwistThatBone9540..9998 is not a generic quaternion blend: it computes inverse bind-source, swing/twist decomposition, reciprocal/rsqrt refinements and explicit atan/sin/cos graphs, then writes an intermediate twist quaternion before composing the bind destination. Existing atan/estimate/candidate helpers offer partial reuse; its complete raw numeric graph still requires lifting. The current SinCos helper cannot be presumed equivalent to its separate XMVectorSin/XMVectorCos calls without source comparison.

## Next integration scope

Root can mount MoveRoot only after source review plus the same-allocation synchronous mutation seam. The next concrete exclusive implementation should be `skater_presentation_expressions.rs` and `skater_presentation_twist.rs` with actual retained global Vault binding, existing fourteen hierarchy indices and original bind/replay borrows. It must implement the whole eight-call correction, not a callback that immediately returns Pending.

After those corrections, existing hierarchy composition and mesh remapping remain useful. However `animation::apply_inverse_bind` currently calls a generic host sum-product matrix multiply; it is not evidence for the source PoseWithMatMult fused graph. Actual SetMatrix/PoseWithMatMult command admission and execution need narrow amendments to the existing queue/job path, retaining entity+214 inverse-bind ownership and Extract/Sync publication. The final full148 pose, hips slot overwrite, actual model palette map and GPU row generation must remain one owner chain.
