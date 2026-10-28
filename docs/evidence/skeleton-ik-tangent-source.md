# Skeleton IK constructor tangent dependency

Read-only raw-source recovery, 2026-09-08. `XMVectorTan` at
`82581520..82581654` is called by `SkeletonIK::SkeletonIK` at `82b55374`.
SHA-256 of that half-open leaf span:
`5149a0c64c25b3a07a6c90904766e7f47df8d78f8620c2cbc229d1adb6b33f85`.
No scalar branches or callees occur in the leaf. The caller splats a scalar
to four lanes and consumes only X; the following is its exact per-lane DAG.
This is evidence for native Rust math, not a proposed instruction interpreter.

## Constants and operand recovery

All table words are big-endian f32 bits read from IDA:

| Table | Address | X | Y | Z | W |
| --- | --- | --- | --- | --- | --- |
| TanCoefficients1, B | `8203e090` | `3505bba8` | `be0895af` | `3b607415` | `b795d5b9` |
| TanCoefficients0, A | `8203e0a0` | `3f800000` | `beeef582` | `3cd23cf5` | `b9a37b25` |
| TanConstants, C | `8203e0b0` | `3fc90fdb` | `2e85a309` | `39800000` | `3f22f983` |

VMX128 VA fields must be decoded from the word: IDA's printed high-register
aliases are not reliable. In ordinary `vmaddfp D,A,C,B` listing syntax the
product is A*B, with C added. `FMA(a,b,c)` below denotes a single f32 fused
multiply-add; all standalone products/adds round to f32 separately.

## Range reduction and rational graph

```text
input_zero = (v == +0)                         // 154c, both signed zeros
n = round_nearest_even(v * C.w)                // 1570, 1594
absolute_n = clear_sign_bit(n)                 // 15a0
r0 = FMA(-n, C.x, v)                           // 15a4
integer_n = saturating_signed_i32(absolute_n)   // 15a8
r = FMA(-n, C.y, r0)                           // 15ac
even = (integer_n & 1) == 0                    // 15b0,15b4
z = r*r                                       // 15b8
mask = bounds_bits(r, C.z)                     // 15bc

p0 = FMA(z, B.x, A.w)                          // 15c0
p1 = FMA(z, B.w, B.z)                          // 15c4
p2 = FMA(z, p0, A.z)                           // 15c8
p3 = FMA(z, p1, B.y)                           // 15cc
p4 = FMA(z, p2, A.y)                           // 15d0
p5 = z*p3                                     // 15d4
denominator0 = FMA(z, p4, A.x)                 // 15d8
numerator0 = FMA(r, p5, r)                     // 15dc
denominator = bitselect(denominator0, 1, mask) // 15e0
numerator = bitselect(numerator0, r, mask)     // 15e4
```

`bitselect(a,b,m)` means `(bits(a)&~m)|(bits(b)&m)`, then reinterpret as f32.
The source does **not** turn `bounds_bits` into a whole-lane boolean before
these selections. Raw `15bc` is word `196db181`, `vcmpbfp128 v11,v13,v54`;
it feeds standard `vsel` directly. Replacing this with an epsilon shortcut
such as `if abs(r)<=epsilon { numerator=r; denominator=1 }` changes the graph.

Architectural bounds bits are zero in bounds, `80000000` above the positive
bound, `40000000` below its negative, and `c0000000` for unordered. Signed
vector conversion truncates and saturates; NaN converts to zero and positive
overflow to `7fffffff`. These semantics are documented by the
[NXP/Freescale AltiVec manual](https://www.nxp.com.cn/docs/en/reference-manual/ALTIVECPOWERISAPEM.pdf)
(printed 3-9, 3-10, and 6-95). VMX128-to-base-operation correspondence is
corroborated by the primary
[Xenia instruction implementation](https://raw.githubusercontent.com/xenia-project/xenia/master/src/xenia/cpu/ppc/ppc_emit_altivec.cc).
Its current bounds NaN comment and implementation disagree; use the vendor
definition for unordered, not that emulator expression. No emulator code is
proposed for the game runtime.

The actual constructor's preceding fsel clamp guarantees finite input within
`bits(3c23d70a)..bits(3fc7c82d)`, even for nonfinite fetched degree values.
Consequently its converted `absolute_n` is only 0 or 1. A general scalar leaf
should nevertheless implement the correct saturating conversion, rather than
reuse the existing checked scalar `ReckoningNumeric::truncate_i32`, which models
a different native instruction and would add incorrect failure cases.

## Two unconditional estimates, ordered refinements

The source gets BOTH seeds before either first refinement:

```text
denominator_seed = reciprocal_estimate(denominator)     // 15f0
negative_numerator = flip_sign_bit(numerator)            // 15f8
negative_numerator_seed = reciprocal_estimate(negative_numerator) // 1600

denominator_residual1 = FMA(-denominator, denominator_seed, 1) // 160c
negative_residual1 = FMA(-negative_numerator, negative_numerator_seed, 1) // 1614
denominator_first = FMA(denominator_seed, denominator_residual1, denominator_seed) // 1618
negative_first = FMA(negative_numerator_seed, negative_residual1, negative_numerator_seed) // 161c
denominator_residual2 = FMA(-denominator, denominator_first, 1) // 1620
denominator_valid = denominator_first == denominator_first   // 1624
negative_valid = negative_first == negative_first           // 1628
denominator_second = FMA(denominator_first, denominator_residual2, denominator_first) // 162c
negative_residual2 = FMA(-negative_numerator, negative_first, 1) // 1630
denominator_inverse = if denominator_valid then denominator_second else denominator_seed // 1634
negative_second = FMA(negative_first, negative_residual2, negative_first) // 1638
even_result = numerator * denominator_inverse               // 163c
negative_inverse = if negative_valid then negative_second else negative_numerator_seed // 1640
odd_result = denominator * negative_inverse                 // 1644
result = if even then even_result else odd_result            // 1648
result = if input_zero then +0 else result                   // 164c
```

The self-equality guard tests the FIRST refinement, not the second. Each
original seed survives if its first refinement is NaN. Zero input still
executes rounding and both reciprocal estimates; the final override produces
positive zero for either signed input zero. Neither parity selection nor the
zero override permits skipping the otherwise unused reciprocal path.

The leaf has no retained output stores before return. A fallible host service
therefore leaves the caller's old slope intact. Constructor geometry widths
and lengths have already committed by then. A generic mutable-output wrapper
may commit once after the entire leaf, never after just the first estimate.

## Smallest implementation interface and focused validation

Proposed interface (no Rust implementation authorized by this packet):

```rust
fn try_tangent<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
    value: f32,
    numeric: &mut N,
    rounding: &mut R,
) -> Result<f32, SkeletonIkTangentError<N::Error, R::Error>>;
```

Error variants need only distinguish rounding and reciprocal numeric failure.
Use the existing round-nearest-even service at `1594` and reciprocal service
at `15f0`, then again at `1600`. Local saturating conversion and raw bitmask
operations need no fallible host service. Preserve the same limited host
estimate/NaN-payload/denormal claims as the accepted ATan and SinCos graphs.
Calling host `tan`, dividing numerator by denominator directly, or merging
the ordered split range reduction is not equivalent source evidence.

Focused tests should freeze source-DAG results with controlled estimate seeds;
distinguish split reduction/FMA from reassociation; verify both parity paths,
zero's full callback sequence and +0 result, first-refinement fallback timing,
both estimate failure prefixes, range-rounding failure, and unusual mask bits.
Test generic saturation/NaN conversion independently of the bounded actual
constructor. For real original Vault geometry, compare against an independent
f64 tangent geometry oracle with explicit tolerance in addition to the exact
controlled-seed tests; a geometry oracle alone cannot validate the DAG.

A transient Python/`ucrtbase.fmaf` calculation independently evaluated this
written DAG using f32-rounded `1/x` estimate seeds. It produced the following
useful host-seed snapshots, not Xenon reference output:

| Input bits | Result bits | Raw bounds mask |
| --- | --- | --- |
| `00000000` or `80000000` | `00000000` | `00000000` |
| `3c23d70a` (ctor lower endpoint) | `3c23d871` | `80000000` |
| `3fc7c82d` (ctor upper endpoint) | `42c7fe57` | `40000000` |
| `3f000000` | `3f0bda7b` | `80000000` |
| `bf000000` | `bf0bda7b` | `40000000` |
| `3f800000` | `3fc75921` | `40000000` |
| `bf800000` | `bfc75921` | `80000000` |
| `39800000` | `39800000` | `00000000` |

The upper-endpoint result differs from f64 `tan` by about `4.42e-4` at a
slope near 100; this illustrates why an eventual geometric tolerance must
allow source split-reduction/estimate rounding near the vertical endpoint.

Confidence: **exact** raw constants, operand/order graph and call-site store
ordering; **high** VMX128 architectural mask/conversion interpretation,
corroborated independently. No original-data execution, Rust test, Cargo run,
or Xenon numeric trace was performed for this recovery. The implementation
interface is proposed, not currently integrated behavior.
