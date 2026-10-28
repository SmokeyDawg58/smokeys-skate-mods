# `EulerXYZFromMatrix33` source proof (2026-09-23)

Read-only IDA raw-word audit of `rw::math::vpu::EulerXYZFromMatrix33` `82e3df20..82e3e360` (size `0x440`, 272 instructions). `CameraMan::UpdateRig82e47e68` calls it at `82e48564` with the **old** `Frame.matrix` X/Y/Z columns staged at `82e48548..50`, then at `82e48588` with the new Rig `humanized_matrix` X/Y/Z columns staged at `82e4857c..84`. These are two separate calls in old-then-new order before camera angular velocity subtraction. Each ignores the input columns' W lanes and writes a complete four-lane result.

The raw VX128 arithmetic operand decoder used here is the one documented in `rqd-create-raw-register-audit-2026-09-22.md`. IDA displays many VA operands with a +32 alias; the equations below use raw words. `XMVectorATan` is the existing `skeleton_query_rotation::try_atan` scalar primitive. The source's single reciprocal refinement and signed π/2/π correction already occur in `camera_compass_math::heading82e34368` and can be factored into a reusable camera-owned `atan2` helper. `simulation_drive_jacobian::refine_drive_reciprocal` uses **two** refinements and is not suitable for the per-angle ratios here.

## Length and branch

Let matrix columns be `X`, `Y`, `Z`, and `h2 = X.x*X.x + X.y*X.y` in source order. `82e3df80` multiplies `X.y*X.y`; `82e3df8c` fuses `X.x*X.x + that product`. `82e3df90..c0` uses one `vrsqrtefp` seed and **two** fused Newton refinements, then computes `h = h2 * reciprocal_sqrt`. Raw `82e3dfc4 17d6eb7d` selects +0 for exactly zero `h2`, otherwise the computed h. `82e3dfc8` compares **ordered** `h > 0.001` (`0x3a83126f`); true takes `82e3dfe0`, false including unordered/NaN takes `82e3e1d8`. The zero select occurs after reciprocal-square-root work, so even a zero vector reaches that numeric operation.

For either branch, a source atan2-like operation on `(numerator, denominator)` issues a `vrefp128` seed for the denominator, performs **one** fused reciprocal refinement, computes `numerator * inverse + +0` with `vmaddcfp128`, and calls `XMVectorATan` on the ratio. Then it forms signed π and signed π/2 by OR-ing the numerator's sign bit with constants loaded at `82386e50` (`π=0x40490fdb`, `π/2=0x3fc90fdb`). A `vaddfp` forms `signed_pi + atan` unconditionally for each angle after the branch's atan calls; a `0 > denominator` mask then selects it versus `atan`, and a later `denominator == 0` mask overrides with signed pi/2. This order retains both ratio/atan calls and the pi addition even for a zero denominator. Unordered comparisons select neither correction; NaN payload bit parity still needs a Xenon trace.

## Ordinary and exceptional control graph

In the normal (`h>0.001`) branch the source issues three atan calls in this order:

1. `82e3dff8..82e3e058`: numerator `X.y`, denominator `X.x` → output Z. `82e3e150/194` compare denominator against zero for π and π/2 correction.
2. `82e3e05c..82e3e0b8`: numerator sign-XOR of `X.z`, denominator `h` → output Y. `82e3e128/154` compare denominator against zero. Raw `82e3e088 176e6b3d` XOR operands are `v46` (X.z splat) and `v45` (sign mask), so the numerator is **`-X.z`**.
3. `82e3e0bc..82e3e108`: numerator `Y.z`, denominator `Z.z` → output X. `82e3e130/160` compare denominator against zero.

The ordinary finite result is `(X,Y,Z) = (atan2(Y.z,Z.z), atan2(-X.z,h), atan2(X.y,X.x))`. The atan2 notation describes the source ratio, signed constants, and selects above; it is **not** permission to substitute host `f32::atan2`, which has different issue order and exceptional behavior.

In the near-vertical/NaN (`!(h>0.001)`) branch the source issues two atan calls:

1. `82e3e1d8..82e3e24c`: numerator `-X.z`, denominator `h` → output Y. Raw `82e3e210 17949b3d` XORs X.z with the sign mask.
2. `82e3e250..82e3e2b0`: numerator `-Z.y`, denominator `Y.y` → output X. Raw `82e3e294 176c733d` XORs Z.y with the sign mask.

It then selects the corrected angles and inserts **+0** in output Z at `82e3e33c`; ordinary result `(atan2(-Z.y,Y.y), atan2(-X.z,h), +0)`. In each branch `vperm128` with `VectorPermuteConstant<0,5,0,0>` puts selected X/Y in output X/Y, the following `vrlimi128` inserts Z only, and W remains a **bitwise copy of output X** (`82e3e1c8..d0` and `82e3e338..40`). No host code should force output W to zero or leave it unknown.

## Implementation and validation boundary

The smallest helper is a camera-owned `euler_from_matrix33([Vector;3], &mut ReckoningNumeric) -> Vector`, using `length_from_squared_and_estimate` for the two-refinement length, a factored one-refinement camera atan2 primitive, and `try_atan` for its native inner call. It should preserve call order (one RSQRT, then three or two angle reciprocal/atan sequences) and return W=X. The caller still requires ordered old/new calls, three `Math::Angle::Normalize` inserts, and later shake/publication work; this helper alone cannot mark the first frame complete. Confidence is **high** for source operands, branch, ordinary finite values, zero selection, call order and W. Exact VMX estimate, NaN payload, fused-result and FPSCR parity remains **medium** until an original runtime trace.
