# Animated VBR runtime numeric review

September12 active integration review; this is not an Xbox numeric parity claim.

The new actual tree Eval exposes the older formats animated sampler to runtime
physics poses. Root freshly reviewed `VBRDecompressor::DecompressFrameBlock`
82edb790, `Init`82edc410, `InitIDctTablesInv`82edb3d0 and
`XMVectorCos`824b0130 through IDA source/disassembly.

Two concrete corrections in formats `andale.rs`:

- Native82edbb24..bb70 forms separate four-coefficient reductions and adds
  them. At82edbb74..80 it adds the retained `vhalf` vector, before the fused
  range/minimum mapping at82edbb90. `vhalf` is zero in the unloaded IDB data;
  its actual dynamic initializer82fabf10 (table82fd6da4) converts signed1 with
  scale1, yielding0.5 in all lanes. The previous scalar decoder omitted0.5.
  Zero coefficients therefore map to the channel midpoint, not its minimum.
- Native Init82edc6e8..728 stores each initial quantization row then increments
  its same f32 scale by0.2. The previous `1 + index*0.2` shortcut differs in
  later rows. The range application82edba6c..78 is fused: range squared times
  the initial table, plus rounded range/32768. Preserve that grouping.

The existing zero-coefficient test now checks the source-derived midpoint.
Frame agent independently confirmed the half initializer, preweighted IDCT
order, range FMA and recurrent row scale from fresh raw instructions. Fresh
integration gates are still required; the agent is now implementing the
remaining table/rounding graph under root independent review.

Numeric work identified before the following implementation:

- InitIDctTablesInv uses the actual XMVectorCos polynomial, not host `cos`.
  Coefficients are at823cfe40..60, pi constants823cfea0. It multiplies the
  sample's half-offset by precomputed frequency constants; DC is additionally
  halved after its cosine/quarter multiplication.
- Init82edc650..6a0 uses the XMVectorLog polynomial and `vlogefp` exponent
  extraction on frequencies2..9, followed by ln2 and1/32768. The current host
  `ln` remains an acknowledged numerical boundary.
- FrameBlock82edba88/90 pre-multiplies quantization by IDCT weights before
  coefficient reductions. The current scalar helper still applies a generic
  eight-term inverse DCT after dequantization; its rounding graph must be
  reconstructed separately, with an explicit VMX horizontal-reduction policy.
- No native per-frame reference traces establish subnormal, NaN payload,
  hardware estimate or bit-for-bit animated output parity.

The independently validated constant bind-pose path has no animated coefficient
rows and is separate from these corrections.

## Native numerical helpers implemented for review

The frame-schedule child exclusively changed the three scalar VBR functions in
`crates/skate2-formats/src/andale.rs` and added `andale_vbr_numeric.rs`. Other
animation formats and codec handling remain outside that scope. Confidence is
**high** for native operation grouping and constant extraction, with the host
numerical boundaries below. Root independent review and fresh gates are pending
for this addition; the author ran no Cargo during concurrent integration.

### IDCT table construction

Fresh `InitIDctTablesInv82edb3d0` raw disassembly loads frequency constants,
in coefficient order, with exact words:

```
00000000 3ec90fdb 3f490fdb 3f96cbe4
3fc90fdb 3ffb53d2 4016cbe4 402fede0
```

`HALF_PI_187` was independently read as `3fc90fdb`. Source converts sample
index to f32 then adds0.5 at82edb458, multiplies by each frequency constant,
calls `XMVectorCos824b0130`, and multiplies each result by0.25. It finally
multiplies the stored DC result by0.5 at82edb5c8. Runtime immutable table reuse
is safe here: native per-decompressor tables have only these fixed inputs.

`XMVectorCos` reads `g_XMPiConstants0` at823cfea0. Lane1=`40c90fdb` (2pi),
lane3=`3e22f983` (inverse2pi). It multiplies angle by inverse2pi, nearest-even
rounds with VRFIN824b01a4, and uses negative fused subtraction at824b01a8:

```
x = -two_pi.mul_add(nearest_even(angle*inverse_two_pi), -angle)
```

This preserves negative-rounded-FMA zero behavior. Original coefficient tables
823cfe40/50/60 contain:

```
3f800000 bf000000 3d2aaaab bab60b61
37d00d01 b493f27e 310f76c8 ad49cba5
29573f9f a53413c3 20f2a15d 9c8671cb
```

Source separately constructs x2=x*x, x4=x2*x2, x6=x4*x2, x8=x4*x4,
x10=x6*x4, x12=x6*x6, x14=x8*x6, x16=x8*x8, x18=x10*x8,
x20=x10*x10, x22=x12*x10. Starting at1, it adds coefficient times power
through ascending FMAs (`824b01b4..0200`). Host `cos` is no longer used by VBR.

### Quantization table construction

Fresh Init82edc588..6a0 identifies `XMVectorLog` local C0 at820f0fb0 and C1
at820f0fa0. Their exact words are:

```
C0 3fb8aa0e bf389e52 3ef5162d beb1d204
C1 3e77adbd be0cd4fb 3d5541c6 bc188b0b
```

Inputs are solely the integers2..9 (`82edc600..654`). Source VSEL keeps their
mantissa and substitutes the exponent/sign of1. Let x=mantissa-1, x2=x*x,
x3=x*x2, x4=x2*x2. Each coefficient array makes a cubic in the source grouping:

```
linear = fma(x,C[1],C[0])
quadratic = fma(x2,C[2],linear)
cubic = fma(x3,C[3],quadratic)
combined = fma(x4,cubic1,cubic0)
log2 = fma(x,combined,exponent)
```

Here exponent is floor(VLOGEFP(input)). For these eight positive normal inputs,
the implementation uses their binary exponents1,1,2,2,2,2,3,3. This constrained
projection avoids introducing a substitute general logarithm service. Exact
hardware estimate edge behavior still needs native traces. Source then
multiplies log2 by `gLn2`=`3f317218` at8227e950 and separately by `38000000`
(1/32768), before the recurrent table scale multiplication. Runtime range
application retains rounded range squared and the existing fused graph.

### Coefficient reconstruction and remaining host policy

`DecompressFrameBlock82edb790` loads the selected IDCT row from
`this+30+sample*32` at82edb828..850. Final quantization halves are built/cache
stored at82edba6c..80, then multiplied by those IDCT halves at82edba88/ba90.
Only afterward do dot4 operations multiply by decoded coefficient values
(`82edbb24..5c`). The two reductions are added, then0.5, then fused range/min.
The new implementation follows that grouping. It does not feed dequantized
coefficients through a generic eight-term transform.

Each dot4 explicitly uses separately rounded f32 products and left-associated
f32 sums as the **host** horizontal-reduction policy; the source merely proves
one VMSUM4FP per half. No claim is made that this scalar association matches
Xenon intermediate precision, denormal flushing, NaN payloads, FPSCR state,
or estimate behavior. The existing host abs/max exceptional-value selection
for DCT range remains another boundary. All callers still receive these
limitations rather than a bitwise Xbox parity claim.

### Focused validation artifacts

`artifacts/vbr-source-numeric-review.py` separately evaluates the captured
operation graph using Python f32 rounding and UCRT fmaf. This is a host check,
not an independent author's review or a native trace. It produced regression
oracles for row7 IDCT, all eight logarithms, recurrent quantization row17, and
a mixed signed coefficient vector. For coefficients
`[32767,-16384,73,12001,-32000,919,-11111,22222]`, table17, DCT range[-1,.5]
and channel range[-2,3], native prescale grouping yields sample1=`40b2659c`
and sample3=`c165dd32`. Dequantizing coefficients first instead yields
`40b2659a` and `c165dd33` under the same host policy.

Three focused tests freeze those source-graph host oracles and verify the
separate-half reduction with a cancellation case. Existing basis/midpoint
tests remain. Fresh workspace/corpus validation and independent source review
are owned by the root integrator.
