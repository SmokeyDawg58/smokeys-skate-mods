# Rig framing VMX operand proof

Read-only operand audit of `Iris::Rig::DoFramingUpdate82e42ee0`, 2026-09-23. This amends the earlier camera framing packet. The IDA VMX128 display aliases arithmetic operands by +32 in several instructions; the raw words and the established decoder convention in `rqd-create-raw-register-audit-2026-09-22.md` govern the statements below. There was no target runtime trace, so the host numeric policy is still a validation boundary.

## Quaternion to Euler

The Rig target quaternion at `+0x80` loads at `82e42f34`. `82e42f3c..82e42fbc` is the same sqrt-two quaternion expansion as `SkeletonData::Init82b56ab8..82b56b58`, already expressed in `simulation_drive_jacobian::drive_basis_host`. Both build diagonal, paired products, W products, and three columns by the same `vpermwi 0x60/0x84`, `vperm`, and `vrlimi` sequence. `DoFramingUpdate` stores **v54** to the `EulerXYZFromNormalizedVector` input stack address at `82e42fbc`; the corresponding `SkeletonData::Init` result is **v49**, stored at matrix column `+0x20` at `82b56b48`. Hence this is the quaternion's **third (forward) basis column**, not its X column. Its four lanes in existing host notation are `[sum[2], difference[1], diagonal[0], sum[0]]`. `Math::EulerXYZFromNormalizedVector825f3e20` receives this vector with roll `+0.0` at `82e42fc8`.

After Euler returns, raw `82e42fe0 1a40ff35` splats result X into v50. Raw `82e42fe4 16329b35` XORs that v50 with the sign mask made by `82e42fdc`; IDA calls the v50 input `v82`. Raw `82e42fe8 1be88f15` inserts only lane X (`mask 8`) back into the Euler vector. **The target pitch is negated; yaw is untouched.** Existing `camera_compass::runtime::math::euler_angles` consumes this same normalized-vector source, so the retained scalar inputs are `(-pitch, yaw)`; Math::Angle normalization still occurs afterward at `82e42ff4`, `82e43130/44/54/68`, and before both Tracker Update calls. This proof is high for ordinary finite values; exact nonfinite payloads and signed-zero bits need a PPC trace.

## Three humanized matrix columns

The `shotEditor` switch is callee r5, copied into r28 at `82e42f30`, and tested at `82e4346c`. True selects target Euler X/Y, false selects pitch/yaw tracker output; Z is +0 in both. Three distinct `XMVectorSinCos` calls issue in X, Y, Z order at `82e434e4/f4/82e43504`.

At the final VMX stage let `(sx,cx)`, `(sy,cy)`, `(sz,cz)` be those three results. The raw vector arithmetic is:

```text
82e43534 v43=cx*sz       82e4353c v12=sx*sz
82e43540 v42=sx*cz       82e43548 v13=cx*cz
82e43550 v40=cy*sz       82e43558 v38=cy*cz
82e43560 v37=cy*sx       82e43568 v41=cy*cx
82e43580 v36=sy*v43      82e4358c v35=sy*v42
82e43594 v11=sy*v13+v12  82e43598 v10=sy*v12+v13
82e435a4 v33=v36-v42     82e435ac v32=v35-v43
```

Here the **raw** arithmetic inputs correct IDA's +32 aliases (`v93` means loaded v61=`cx`, `v94` means v62=`sx`, `v95` means v63=`cy`, `v68` means v36, `v67` means v35). Native `vmaddfp` is `VA*VC+VB`, which is decisive for v11/v10. Raw `82e4359c 144641e5` makes first column from v38/v40; raw `82e435a0 18423f15` inserts negated `sy` into Z. Raw `82e435b0 17eb09c5` makes third column from v11/v33; raw `82e435b8 1be24f15` inserts v41 into Z. Raw `82e435b4 17c051e4` makes second column from v32/v10; raw `82e435bc 1bc22f15` inserts v37 into Z. The three stores at `82e435a8/c0/c4` are therefore, in Rig offset order `+0x40,+0x50,+0x60`:

```text
X = [ cy*cz,                    cy*sz,                    -sy,    cy*cz ]
Y = [ sy*sx*cz - cx*sz,        sy*sx*sz + cx*cz,        cy*sx,  Y.x   ]
Z = [ sy*cx*cz + sx*sz,        sy*cx*sz - sx*cz,        cy*cx,  Z.x   ]
```

This is already the exact expression/W-lane shape of `camera_frame_composition_math::framing_basis`; that helper should be reused with camera-internal visibility. The source still calls SinCos for Z=+0, so no trigonometric shortcut is allowed.

## Roll rotation and remaining boundary

At `82e435c8..d4`, the source multiplies Rig roll `+0x1b0` by framing mirror `+0x17c`; at `82e435e4` it calls `Math::RotateAroundArbitraryAxis82647348` with the newly built three-column matrix, its Z column as axis, and that product as angle. The helper first calls `XMVectorCos826473?` and then `XMVectorSin824b0060`; it splats axis X/Y/Z, builds its rotation columns, then transforms input X/Y/Z columns with X multiply, Y FMA, Z FMA in that order (`8264739c..826474a4`). Its rotation columns and multiply graph are already transcribed for identity input in `skeleton_normal_displacement::try_rotated_identity`; this caller needs the general input-column version of the same graph. All three results overwrite Rig `+0x40/+0x50/+0x60` at `82e435e8..fc`. W lanes remain the computed X lanes, as in the source's permute/insert construction; they are not forced zero.

The scalar/tracker and matrix graph is sufficiently identified for an **unregistered draft**. Confidence: high for input owners, branch/parameter/cvar order, columns and roll graph; medium for host bit parity of VMX/FMA/trig on exceptional values pending per-tick PPC trace. No registered Rust or Cargo run was made here.

Independent review correction, 2026-09-23: `Rig::EvaluateUmbra82e3cc58` uses **two ordered less-than gates**, not negated greater-or-equal gates. Raw `82e3cc78 fcmpu` / `82e3cc7c blt` selects the return-one arm only for `value < umbra`; `82e3cca4 fcmpu` / `82e3cca8 blt` selects interpolation only for `value < penumbra`. An unordered/NaN value passes neither BLT and returns zero at `82e3ccf8..fc`. The unregistered draft and earlier packet now preserve this source ordering.
