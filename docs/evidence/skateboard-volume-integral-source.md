# Authored skateboard deck volume-integral source packet

Date: 2026-09-07. Status: **implemented, independently reviewed, mounted path tested**.
This packet retains
live IDA/PDB/disassembly recovery for the shipped deck's aggregate mass path.
Addresses, byte hashes, field offsets, constants, and instruction/branch facts
are `exact` source facts; the readable arithmetic reconstruction is `high`
confidence. Independent implementation audit passed and actual mounted tuning
now creates the 13 children consumed by this calculation. Thirteen focused
tests pass. Host output is not a reference-runtime mass oracle; no such oracle
was obtained.

This is a dependency of the authored seven-part skateboard path documented in
`skateboard-authored-body-path.md`, not a claim of a registered skateboard or
playable skater. The original source-recovery phase did not modify runtime code;
the subsequent implementation is in `simulation_volume_integral.rs`, consumed
by `skateboard_deck.rs` and the original-data definition corpus.

## Scope and existing reuse

The shipped deck's 13 children contribute as follows:

- One rounded box through `ComputeFatBoxInertia`.
- Two capsules through the already reconstructed capsule unit-mass calculation.
- Ten triangles through their bounding boxes, then `ComputeFatBoxInertia` with
  radius zero.

The triangle approximation is original behavior, not a proposed simplification.
There is no triangle/tetrahedron volume integration, overlap subtraction, or
collision-enabled filter in this path. Overlapping primitive contributions are
added, not integrated as a geometric union.

Existing reusable calculations at inspection time:

- `crates/skate2-core/src/physics/board/runtime/skateboard_wheels.rs`: private `capsule_unit_mass`,
  sphere calculation, mass scaling, and reciprocal-refinement semantics.
  `apply_fresh_mass_properties` is explicitly limited to fresh identity-frame
  sphere/capsule definitions; it does not implement aggregate diagonalization.
- `crates/skate2-core/src/physics/assembly/serialised_assembly_bbox.rs`:
  `try_get_primitive_bbox` / `triangle_bbox`, including source min/max order and
  radius expansion.
- No existing `ConstructIntegral`, `TransformIntegral`, general
  diagonalization, or Jacobi implementation was found.
- `ragdoll_dynamics.rs` still delegates mass properties; it is not an
  implementation of this calculation.

## Source chain and type layout

All functions below are in `rw::physics` except Triangle `GetBBox`, which is in
`rw::collision::TriangleVolume`. Spans are end-exclusive.

| Function | Address/span |
|---|---|
| `ComputeFatBoxInertia` | `82ae7ee0..82ae8024` |
| `ComputeMappedArrayIntegral` | `82ae8028..82ae80fc` |
| `ComputeCapsuleInertia` | `82ae8100..82ae81d0` |
| `sym_schur2` | `82ae8270..82ae83c8` |
| `jacobi_update_cols` | `82ae83c8..82ae8764` |
| `jacobi_update_rows` | `82ae8768..82ae8b1c` |
| `JacobiRowCyclic` | `82ae8b20..82ae8e98` |
| `ConstructIntegral` | `82ae90f0..82ae91bc` |
| `TransformIntegral` | `82ae91c0..82ae93c0` |
| `ComputeSimpleInertia` | `82ae93c0..82ae9560` |
| `DiagonaliseVolumeIntegral` | `82ae9560..82ae981c` |
| `ComputeVolumeIntegral` | `82ae9878..82ae9a4c` |
| Triangle `GetBBox` | `82acdfe8..82ace0c8` |
| `ComputeMassProperties` | `82ae9a50..82ae9df4` |
| `Sk8::Physics::SkateboardBody::CreateTriangleDeck` | `82b32e08..82b33da4` |

The direct consumer is `ComputeMassProperties`: its aggregate integral call is
at `82ae9b60`, followed by diagonalization at `82ae9b80`. Its successful
primitive fast path bypasses aggregate diagonalization. `CreateTriangleDeck`
calls mass properties at `82b33d30` with density 1 and null volume-output
pointer, then overrides the retained inverse-body frame as described below.

Relevant PDB types agree with native loads/stores. Hexadecimal offsets/sizes:

- `Volume`: size `60`; transform `+00`, vtable `+40`, shape union `+44`, radius
  `+50`, group ID `+54`, surface ID `+58`, flags `+5c`.
- Box half-extents: `+44/+48/+4c`; capsule half-height `+44`.
- Volume vtable: type ID `+00`, `GetBBox` function `+04`.
- `Aggregate`: size `30`; AABB `+00`, vtable `+20`, tag-bit count `+24`, child
  count `+28`.
- `MappedArray`: size `40`; inherited aggregate followed by child pointer
  `+30`. Each child is a full `60`-byte `Volume`.
- Aggregate volume's aggregate pointer is in its shape union at `+44`.
- Recognized aggregate object types are `80002` and `80004`.
- `Matrix44`: four stored vectors at `00/10/20/30`, size `40`.
- `Matrix33`: three stored vectors at `00/10/20`, size `30`.

## Primitive and aggregate dispatch

`ComputeVolumeIntegral` performs this control flow:

1. NULL volume type returns false.
2. Successful `ComputeSimpleInertia`: construct integral, transform by that
   primitive's own frame, return true.
3. Recognized mapped aggregate: accumulate children. Transform by the aggregate
   volume's frame only if accumulation returned true. Return true even when
   accumulation returned false.
4. Triangle: attempt `GetBBox(volume, null, 0)`.
5. If that fails, or for other unsupported types, attempt
   `GetBBox(volume, &volume.transform, 0)`.
6. Successful bbox: calculate half-extents `(max-min)*0.5`, calculate fat-box
   inertia with radius `+0`, calculate center `min+half_extents`, construct the
   integral, and transform using identity axes plus that center.
7. Failed bbox returns false.

Triangle `GetBBox` always returns one. With a null parent, its calculation is:

```text
minimum = min(point0, min(point1, point2)) - radius
maximum = max(point0, max(point1, point2)) + radius
```

The three points occupy its first three transform vectors; its fourth vector
is not another point. The radius is expanded in the bbox before the subsequent
fat-box calculation receives its separate radius-zero argument. Existing bbox
numeric service boundaries remain relevant for VMX min/max exceptional values.

`ComputeMappedArrayIntegral` snapshots count/pointer, zeros all 16 result lanes,
visits children in ascending `60`-byte order, and adds successful child results
lane-by-lane using separate f32 additions. Failed children contribute nothing.
The return value reports whether any child succeeded. There is no enabled-flag
test, sorting, pairwise reduction, compensation, or overlap correction.

An empty recognized aggregate consequently produces a zero integral but is
reported successful by `ComputeVolumeIntegral`. Diagonalization has no
zero-volume guard. A safe-host unsupported/error policy must not be described
as native zero-volume handling. Native child traversal also has no recovered
recursion-depth or host-allocation guard.

## Fat-box arithmetic

Each ordinary operation below rounds to f32; `fma` denotes one source fused
operation. `select(d,a,b)` chooses `a` when the already-rounded `d >= 0`,
including signed zero; unordered chooses `b`. Do not replace this with generic
min/max or silently sanitize input values.

Constants, written as exact f32 bit patterns:

```text
eps     = 33d6bf95
0.05    = 3d4ccccd
2/3     = 3f2aaaab
1/3     = 3eaaaaab
2*pi    = 40c90fdb
```

Given input half-extents `x,y,z` and radius `r`:

```text
largest_yz = select(y-z, y, z)
largest    = select(x-largest_yz, x, largest_yz)
minimum    = (largest+r)*0.05
minimum    = select(minimum-eps, minimum, eps)
floor      = minimum-r

a = select(x-floor, x, floor)
b = select(y-floor, y, floor)
c = select(z-floor, z, floor)

radial = (r*r)*0.5

diagX = fma(fma((b+c)*r, 2, c*c) + b*b, 1/3, radial)
diagY = fma(fma((a+c)*r, 2, a*a) + c*c, 1/3, radial)
diagZ = fma(fma((a+b)*r, 2, a*a) + b*b, 1/3, radial)

rounded = fma(r, 2/3, a)
rounded = rounded+b
rounded = rounded+c
rounded = rounded*r
rounded = rounded*(2*pi)

faces = fma(b+c, a, b*c)
base  = ((a*b)*c)*8
volume = fma(fma(faces, 8, rounded), r, base)
```

The stored diagonal W lane is zero. The radius itself is not clamped here.
Even radius-zero thin triangle boxes receive the source's minimum-half-extent
treatment. This is the source formula, not an idealized rounded-box formula.
Capsule unit-mass equations and their original constants/grouping are retained
in `skateboard-wheels.md`; the aggregate path must reuse that calculation with
each capsule's actual local-Z half-height and radius.

## Construct and transform the integral

`ConstructIntegral(diagonal, volume)` creates a homogeneous second-moment
matrix, not an inertia tensor:

```text
Qxx = ((((Iz+Iy)-Ix)*volume)*0.5)
Qyy = ((((Iz+Ix)-Iy)*volume)*0.5)
Qzz = ((((Iy+Ix)-Iz)*volume)*0.5)
Qww = volume
all other entries = +0
```

Multiplication by volume precedes multiplication by half. Hex-Rays omits much
of this scalar calculation from its pseudocode; the equations above were
recovered from the scalar PPC operations and stack stores at `82ae90f0`.

`TransformIntegral(Q, frame)` is `H Q H-transpose` with this construction/order:

- Null frame is a no-op.
- Construct the columns of `H` from frame xyz components.
- Force axis W lanes to zero and translation W to one; incoming frame W lanes
  are ignored.
- First calculate each column of `H Q`, then each column of the final product.
- Each four-term column combination begins with the X product, followed by
  fused Y, Z, W additions in that order.
- Complete intermediates precede result stores; no in-place intermediate
  contamination occurs.

For example, the first stage uses this lane-wise column expression:

```text
B[column] = H[0] * Q[column].x
B[column] = fma(H[1], Q[column].y, B[column])
B[column] = fma(H[2], Q[column].z, B[column])
B[column] = fma(H[3], Q[column].w, B[column])
```

The second stage uses the same ordered graph with `B` columns and the columns
of `H-transpose`. This is a readable two-stage matrix calculation; no runtime
instruction interpreter or assembly-shaped implementation is required.

## Diagonalization

`DiagonaliseVolumeIntegral` performs these steps:

1. Store `volume = Q[3][3]`.
2. Obtain a reciprocal estimate of volume and apply two source
   residual/correction refinements.
3. Calculate center from Q's final vector xyz times that reciprocal.
4. Center Q in-place using `TransformIntegral` with identity axes and negative
   center.
5. Construct a symmetric tensor from the centered integral.
6. Run `JacobiRowCyclic(tensor, rotation, 10, tolerance)`, with tolerance bits
   `2b8cbccc`.
7. Transpose rotation into the output body-to-part frame; restore positive
   center.
8. Clamp each tensor diagonal against `largest_diagonal * bits(3ba3d70b)`.
9. Obtain a second independent volume reciprocal estimate, refine twice, and
   multiply the clamped diagonal by it.

The centered tensor, using hexadecimal byte offsets into Q:

```text
Txx = Q14 + Q28
Tyy = Q00 + Q28
Tzz = Q00 + Q14

Txy = (Q10 + Q04) * -0.5
Tyz = (Q24 + Q18) * -0.5
Txz = (Q20 + Q08) * -0.5
```

Both copies of each off-diagonal are stored; tensor W lanes are zero.
Largest/clamp ordering is:

```text
yz      = select(Tyy-Tzz, Tyy, Tzz)
largest = select(Txx-yz, Txx, yz)
floor   = largest * bits(3ba3d70b)
result_axis = select(axis-floor, axis, floor)
```

There is no eigenvalue sorting or positivity guard beyond this source clamp.
Do not reuse the first reciprocal estimate for the final normalization.

### Raw-lane handling

COM extraction uses permutation bytes at `82fd6e90`:

```text
00010203141516170001020300010203
```

It then uses `vrlimi128` mask 2 at `82ae9608`. Therefore packed COM is
`[x,y,z,x]`, not `[x,y,z,0]`. Negative-center construction flips sign bits.
Output transposed axes retain W equal to their Y component. `TransformIntegral`
ignores these frame W values, but the typed result must not claim they were
zero.

Jacobi initializes/updates only the nine meaningful rotation components. Its
temporary W lanes are not initialized. Final transposition uses xyz, so output
axes do not depend on those unspecified temporary W lanes. Host canonical
temporary storage, if used, must remain distinguished from native initialized
state.

## Jacobi equations and chronology

For this section, `A[row][lane]` denotes the three stored vectors' xyz
components. Initialize rotation xyz to identity.

The dynamic swizzle initializer is `82f7efe8`. Its target `83091fc0` is zero in
the static IDB because initialization occurs at runtime. The relevant generated
permutations at offsets `0/40/80` replace exactly X, Y, or Z using the other
vector's X component while preserving other lanes. Their meaning was checked
against the initializer's scalar constant stores, not assumed from static
zero-filled memory.

Before every sweep, compute squared diagonals and select the smallest, using
these branches:

```text
if yy_squared > xx_squared:
    scale = xx_squared if zz_squared > xx_squared else zz_squared
else:
    scale = yy_squared if zz_squared > yy_squared else zz_squared
```

Ties fall toward Y/Z, not X. These are ordered greater-than comparisons; NaNs
take their false arms. It is the minimum squared diagonal, not the maximum.

```text
off = fma(A21, A21, fma(A10, A10, A20*A20))
threshold = scale * (tolerance*tolerance)
```

Stop unless `off > threshold`; also stop when remaining iterations is zero.
Each sweep visits exactly:

```text
(p,q) = (1,0), (2,0), (2,1)
```

For each pair:

1. Calculate Schur sine/cosine from the current tensor.
2. Update rotation columns.
3. Update tensor columns.
4. Update tensor rows.

Then decrement remaining iterations. Return remaining iterations, not
iterations performed. `DiagonaliseVolumeIntegral` ignores this return value.

### Schur sine/cosine

```text
c = 1
s = 0
if A[p][q] == 0:
    return

inverse = reciprocal_estimate(A[p][q])
refine twice:
    residual = fused(1 - inverse*A[p][q])
    inverse  = fma(inverse, residual, inverse)

tau = inverse * ((A[q][q]-A[p][p])*0.5)

if tau > 0:
    t = 1 / (sqrt(fma(tau,tau,1)) + tau)
else:
    t = -1 / (sqrt(fma(tau,tau,1)) - tau)

c = 1 / sqrt(fma(t,t,1))
s = c*t
```

The raw branch at `82ae8374` tests not-GT for the second arm. Zero and unordered
take that arm. A nonzero off-diagonal with equal diagonals gives `tau=0` and
`t=-1`; a generic sign-based positive-root choice changes orientation.
The square roots/divisions here are scalar `fsqrts`/`fdivs`; they are not the
VMX reciprocal-estimate operation used earlier. The half multiplication of the
diagonal difference precedes its multiplication by the refined reciprocal.

### Column and row updates

Column rotation, using each row's old pair
`u=A[row][p], v=A[row][q]`:

```text
new_p = (u*c) - (v*s)      // Two rounded products, then subtraction.
new_q = fma(u, s, v*c)     // Rounded v*c, then fused operation.
```

Row rotation uses the same arithmetic on
`u=A[p][column], v=A[q][column]`.

Both helpers capture old values before stores, write three p entries before
three q entries, preserve untouched entries/W lanes, and do not explicitly
zero the eliminated off-diagonal. Do not replace them with a library
eigensolver or fuse the subtraction arm.

## Consumer boundary and next implementation slice

The next source-ready producer-to-consumer slice is:

```text
Authored deck children
  -> ordered volume integral
  -> source Jacobi diagonal/COM
  -> source mass application
```

Keep this connected to the authored-definition owner alongside current wheel
definitions. It does not require fabricated serialized assembly bytes. The
original geometry and Vault inputs remain those in the authored-body packet;
this mass packet does not replace runtime Vault fetching with corpus constants.

After `CreateTriangleDeck` computes mass properties, `82b33d70..82b33d84`
explicitly forces the retained inverse-body transform present and identity.
That later override does not permit skipping aggregate inertia calculation.

Remaining boundaries:

- Actual Xenon reciprocal-estimate bits, scalar square-root/division edge
  behavior, exceptional VMX arithmetic, denormals/FPSCR effects, and NaN payloads
  remain unvalidated. Use explicit numeric services and host honesty as in
  wheel mass; source FMA grouping does not establish universal bit parity.
- No reference-executed 13-child mass trace or frozen native result was
  obtained here. The subsequent independent implementation audit and actual
  mounted 13-child host construction now pass. These host calculations must
  not be labeled reference-runtime traces or universal numeric parity.
- General arbitrary-frame `ComputeMassProperties` inverse-frame construction
  still needs its own implementation review if generalized beyond this deck
  consumer.
- No rendering, board registration, joint/drive integration, or skater behavior
  is implied by this packet.

Focused discriminating tests should cover triangle bbox contribution rather
than zero triangle volume; degenerate triangle thickness floor; reordered
overlapping children; empty aggregate's unusual success; translated and
rotated integrals; Schur equal-diagonal sign; minimum-versus-maximum convergence
scale; fused-versus-unfused rotation arms; independent two volume-estimate
calls; and COM/axis W retention. Corpus tests must consume the actual 13 authored
children and original tuning, not a replacement box or a frozen diagonal used
as constructor input.

## Live byte-span fingerprints

SHA-256 values for the end-exclusive function spans listed above:

```text
FatBox         bea18d90fff2a8740294dfbc141dd4810447ef8b8282c658b349056fc1dcb553
MappedArray    1d084170a5cbf9d2c2caf25bb9a291fce3aa452756de904baf25f000c557fcc6
Capsule        cf4ab01148896f00b4642e4aa3bf38f249c4c3db6bd374c1d750de368b40b007
Schur          198e5fdb5459fdff9fd2a219f353221fb57c263993d992d61a423b5ab5eaab14
UpdateColumns  3c49fb1289d3c7d1b4ca2e6520b514d09b388cf2f571928972c6cff1cbdaea8b
UpdateRows     652af990581e49055089440f6c30aa959e7ea8633d743d6fb9af93d7799eefa3
Jacobi         72897042b8eaeb3aa0173dcb8fef6d338078d15e004e5e414e1303abdf9d3076
Construct      82d5b61e45e6d285cc29ca1a9d7d70faaf443c0d2a44a7edea50dacccc9ac3f0
Transform      80450e5cac44bbf36d70e0f35be4144f5af0e0a4d2b28f66b5d2c92f52ab8762
Diagonalise    b57a703d4d4caab7b050fec17c31421d9bcc092f98951f5db48de098a9214139
VolumeIntegral c6bfb9027d48903cc992be0344d8bf65a00fee2377577f0842da2992d9807259
```
