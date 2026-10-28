# Authored skateboard deck geometry source

Date: 2026-09-07. Confidence: `exact` for checked addresses, keys, original
words, raw stores and operation/branch order; `high` for readable equations.
The implementation in `skateboard_deck.rs` has been authored from this packet.
Independent source review accepted the packet and constructor, including live
verification of all eight fingerprints below. Compilation, focused tests and
actual mounted corpus construction pass; full workspace results are in STATUS.
No reference-runtime numeric trace or XMVector bit parity is claimed.

This is the actual 13-child producer for the mass path documented in
`skateboard-volume-integral-source.md`, not board admission or a playable
skater. Original geometry must feed the new mass calculation; do not substitute
a central box, frozen diagonal, synthetic BAM or invented arena ownership.

## Existing reuse

Reuse `collision::Volume` shape storage and `VolumeType`, the accepted
primitive bbox calculations in `serialised_assembly_bbox.rs`, capsule
unit-mass calculations retained in `skateboard-wheels.md`, and the separately
audited aggregate-integral implementation.

Do not use `GpInstance::from_triangle_points` as the authored-volume constructor:
it does not retain this source volume's flags, radius and edge-cosine state.
`populate_triangle` already understands the volume's normal-dirty bit. An
authored owner must retain actual children directly, not invent arena IDs or
serialized bytes to satisfy an arena-backed template.

## Runtime inputs

Class `physicsdeck = 9ed291a274453fc0`; collection
`default = d7edbd362d7d2152`. Its instance pointer is
`GlobalAttributeCollection +94`; `physics_world` is `+98`, class
`485a76b68ca65d6f`.

Every lookup uses index zero. Missing pointers use `DefaultDataArea(4)`, except
the two enable attributes use `DefaultDataArea(1)`. Labels below describe
semantics, not verified original attribute spellings.

| Input | Full key | Original word |
|---|---|---|
| Width | `d76d9db5ee1fd008` | `3e75c28f` |
| Central length | `c94d65091dcb828b` | `3f170a3d` |
| Thickness | `ceda721a4f0255b6` | `3c75c28f` |
| End extent | `49f429ae2e66f063` | `3e28f5c3` |
| Signed subdivision count | `4bccb0af113e6953` | `00000005` |
| Box/triangle collision enable | `3273d82db269c8ca` | `01000000` |
| Capsule collision enable | `332284c4e878a7c5` | `01000000` |
| Positive-Z end angle, degrees | `80203de9f9b732ca` | `41500000` (13) |
| Negative-Z end angle, degrees | `00e949f6d567bb10` | `41480000` (12.5) |
| Dynamic friction | `ac32e9a7f0056c6d` | `3dcccccd` |
| Static friction | `aae143f564f263aa` | `3dcccccd` |
| Restitution | `254d958a72d59a2d` | `00000000` |
| Angular drag input | `8d3065ced858a7df` | `3ee66666` |
| Deck mass | `01d802758fa1c38a` | `3f99999a` |
| `physics_world` mass multiplier | `46507e40c85372fc` | `40a00000` |

Rechecked using existing `inspect_vault.exe` against original extracted
`skater.vlt/bin` under the supplied game's
`data/db/unpcaked#/data/db` directory. `physicsdeck.default` has no parent or
layout, and these are explicit inline entries. This inspection is not a new
mounted raw-archive equivalence test; retain the raw-versus-pre-relocated
provenance distinction in `skateboard-wheels.md`.

### Fetch chronology

For subdivision count five, there are 26 fetches:

1. Width: immediately load and halve (`82b32ec0`).
2. Length: immediately load and halve (`82b32f24`).
3. Thickness: immediately load and halve (`82b32f80`).
4. End extent: immediately load (`82b32fdc`).
5. Subdivision count: load signed integer (`82b33034`).
6. Box/triangle enable for central box (`82b331bc`).
7. Capsule enable for child 1 (`82b2bad8`).
8. Capsule enable again for child 2 (same helper call site).
9. Positive-Z tilt angle: multiply by degrees-to-radians, cosine then sine
   (`82b333b0`).
10. Negative-Z tilt angle: same (`82b3345c`).
11. Each of ten `AddTriangle` calls independently fetches box/triangle enable
    (`82b2b964`).
12. Dynamic friction pointer (`82b33b08`), then static friction pointer
    (`82b33b60`); only afterward dereference dynamic then static, and store
    static then dynamic.
13. Restitution (`82b33bcc`), then angular drag (`82b33c30`).
14. Deck mass pointer (`82b33ca0`), then world multiplier pointer (`82b33cf8`);
    dereference world first, deck mass second; multiply deck mass by world.

Do not collapse repeated enable reads or delayed pointer dereferences into a
silently immutable tuning snapshot. An explicitly immutable owner may snapshot
values only under a documented no-update-during-construction seam. Shared
runtime attribute access must preserve original fetching/default behavior.

## Geometry equations

All plain operations are f32. `fma(a,b,c)` and `fms(a,b,c)` denote fused
`a*b+c` and `a*b-c`. Negation preserves the source sign-bit operation.

```text
W = width * 0.5
L = length * 0.5
H = thickness * 0.5
E = end_extent
N = signed subdivision count
A = W-H
B = E-H
```

### Child 0: rounded central box

```text
rounded_radius = H * bits(3f666666)
half_extents = [W-rounded_radius, H-rounded_radius, L-rounded_radius]
```

Construct using `BoxVolume::Initialize` at `82adcc38`, then set radius to
`rounded_radius`. Initial transform is I/J/K/zero, including translation W
zero. Group/surface IDs are zero. Initialization flags are one; enable lookup
sets/clears bit zero while preserving other bits.

The debug box records `[W,H,L,0]` as dimensions, not reduced collision
half-extents. This call does not independently reset its pre-existing debug
offset. Board construction initializes debug offsets to identity/zero.

### Children 1 and 2: longitudinal capsules

Both receive radius H:

```text
child 1: p1=[W-H,+0,L,+0], p2=[W-H,+0,-L,+0]
child 2: p1=[H-W,+0,L,+0], p2=[H-W,+0,-L,+0]
```

The opposite lateral coordinate is `H-W`, not negation of the previously
rounded `W-H`. These go through `PhysicsUtility::InitializeCapsule` below.
Do not substitute half-height L and skip the original numeric path.

### Children 3 through 12: interleaved end fans

Constants at `8225c454`:

```text
+00 degrees_to_radians = 3c8efa35
+08 positive_pi        = 40490fdb
+10 half_pi            = 3fc90fdb
negative_pi literal   = c0490fdb
```

```text
tilt_plus  = positive_end_degrees * bits(3c8efa35)
tilt_minus = negative_end_degrees * bits(3c8efa35)
```

Call cosine before sine for each. Initialize two independent previous-angle
pairs by calling `Cos(+0)` then `Sin(+0)` for each. Each retained pair is
`(sin_phi, cos_phi)`.

For `i=1..=N`, only when signed N is positive:

```text
fi = f32 conversion of signed i
fn = f32 conversion of signed N
phi_minus = (fi * bits(c0490fdb)) / fn
phi_plus  = (fi * bits(40490fdb)) / fn
```

Multiplication precedes division. Convert N once, i once per iteration. Do not
replace this with `i*(pi/N)`.

For a negative-fan pair `(s,c)`:

```text
q = s*B
point_minus(s,c) = [
    c*A,
    sin(tilt_minus)*(-q),
    fms(q,cos(tilt_minus),L),
    +0
]
```

For a positive-fan pair:

```text
q = s*B
point_plus(s,c) = [
    c*A,
    sin(tilt_plus)*q,
    fma(q,cos(tilt_plus),L),
    +0
]
```

Each iteration:

1. Calculate `Cos(phi_minus)`, then `Sin(phi_minus)`.
2. Add negative triangle at `3+2*(i-1)`:
   `p1=[+0,+0,-L,+0]`, `p2=point_minus(previous_minus)`,
   `p3=point_minus(current_minus)`, radius H.
3. Retain current negative pair as previous.
4. Calculate `Cos(phi_plus)`, then `Sin(phi_plus)`.
5. Add positive triangle at `4+2*(i-1)`:
   `p1=[+0,+0,L,+0]`, `p2=point_plus(previous_plus)`,
   `p3=point_plus(current_plus)`, radius H.
6. Retain current positive pair as previous.

Child pairs are `(3,4),(5,6),(7,8),(9,10),(11,12)`. Recompute previous/current
rim points from retained trig pairs. Do not reverse winding or force final
endpoints to exact +/-A; native trig results remain inputs.

Negative Z uses `fmsubs` at `82b336e8/82b33784`; positive Z uses `fmadds` at
`82b3396c/82b33974`. These offsets must remain fused.

### Trig boundary and chronology

Native functions: `XMVectorCos=824b0130`, `XMVectorSin=824b0060`, and
`XMVectorSinCos=82584f60`.

Before the two capsules, `82b33200` calls `XMVectorSinCos(half_pi)`. Its temporary
outputs are not consumed by this geometry; later scratch writes replace them.
Do not misinterpret it as a capsule rotation. Two zero-angle Cos/Sin pairs
follow the capsules, then positive/negative tilt pairs, then interleaved
negative/positive sweep pairs.

For N=5 the source executes one SinCos, fourteen Cos and fourteen Sin calls.
A host trig backend is explicitly approximate, not original XMVector parity.

## AddTriangle state

`AddTriangle=82b2b8c8..82b2ba10` addresses child storage using low 16 bits of
the supplied index, then placement-constructs `TriangleVolume` at
`82ac8d98..82ac8e60`.

```text
transform columns 0,1,2 = p1,p2,p3, copying all four lanes
transform column 3 = [0,0,0,0]
radius = 0
group_id = surface_id = 0
flags = 0x1e3
edge_cos = [-1,-1,-1]
```

Then fetch enable, set/clear bit zero, write radius H, OR flags with `0x200`,
write edge cosines 0 and 2 to +1 (leave cosine 1 at -1), and append the debug
triangle with the same ordered points and radius.

Shipped flags are `0x3e3`, edge cosines `[1,-1,1]`. This is two-sided: `0x10`
is absent. All three convex-edge bits, edge-cosine use, vertex-0 disable and
normal-dirty bit are set. No cached normal is calculated here; fourth transform
vector remains zero. Eager normal calculation must not silently replace the
source cache/update semantics. Flag names are corroborated in
`triangle-contact-fixup.md`.

## AddCapsule and endpoint initialization

`AddCapsule=82b2ba10..82b2bc0c` uses low-16-bit child indexing, calls
`PhysicsUtility::InitializeCapsule=82e4b258..82e4b4f8`, fetches capsule enable,
updates bit zero, and appends the debug capsule.

```text
delta = p2-p1
half_delta = refined_reciprocal_estimate(2)*delta
center = p1+half_delta
squared = source_dot3(half_delta,half_delta)
```

The reciprocal estimate receives two residual/correction refinements. Derive
half-height using reciprocal-square-root estimate, two refinements, then
`squared*refined_estimate`; select exact zero when squared equals zero.
Each reciprocal-square-root refinement is:

```text
square = estimate*estimate
half_estimate = estimate*0.5
residual = fused(1-squared*square)
estimate = fma(half_estimate,residual,estimate)
```

Initialize actual capsule with radius and derived half-height.
`CapsuleVolume::Initialize=82ad7fa0` sets I/J/K/zero, flags one, group/surface
zero, radius and half-height. Inactive shape-union words are not initialized.

```text
inv_h = scalar(1/half_height)
nx = inv_h*half_delta.x
ny = inv_h*half_delta.y
nz = inv_h*half_delta.z
horizontal = sqrt(fma(ny,ny,nx*nx))
```

If `horizontal > bits(37800000)`:

```text
inv_horizontal = 1/horizontal
X = [-(inv_horizontal*ny), inv_horizontal*nx, 0, 0]
Y = [-((inv_horizontal*nz)*nx), -((inv_horizontal*nz)*ny), horizontal, 0]
Z = [nx,ny,nz,0]
```

Otherwise retain initializer I/J/K; unordered also skips. Finally store
`[center.x,center.y,center.z,0]` as translation. For longitudinal endpoints,
the horizontal test is zero and axes stay identity despite negative-Z delta.
Capsule symmetry does not justify changing that retained frame.

The debug helper separately recomputes length from `p1-p2`, using another
reciprocal-square-root estimate/refinement sequence, then multiplies by 0.5.
It copies collision capsule frame and stores radius/type/half-length. Collision
half-height and debug half-length have different arithmetic ordering. Preserve
dot/reciprocal/rsqrt exceptional behavior behind explicit numeric services.

## Aggregate bookkeeping and bounds

```text
mNumDeckPartVolumes = [N,N,3]
mDeckPartVolIndex   = [4,3,0]
mNumDeckVolumes     = (N+3)+N
```

First two categories are interleaved, not contiguous ranges. SkateboardBody
PDB offsets:

```text
mAggregate              +3ec
mNumDebugObjects        +410
mDebugObjects[32]       +420
mDebugObjectIndexStart +1a20
mDebugObjectIndexEnd   +1a3c
mVolumes[3]            +1e04
mNumDeckVolumes        +2010
mNumDeckPartVolumes[3] +2014
mDeckPartVolIndex[3]   +2020
```

`SimpleMappedArray::Initialize=82acf260` receives class size 0x40; child
storage follows aligned class storage. Tag-bit count is bit length of
`numVolumes+1`, hence four for 13.

After construction, assert final low-16-bit child index equals total, then call
`SimpleMappedArray::UpdateThis=82acf2b8..82acf388`:

- Visit every child ascending, including disabled children.
- Call `GetBBox(child,null,0)`.
- First result initializes aggregate bounds directly.
- Later results use `min(current_min,child_min)` and
  `max(current_max,child_max)`.
- Ignore bbox return values; reload child count after each iteration.
- Empty input leaves prior/uninitialized aggregate bounds unchanged.

Then allocate/initialize `AggregateVolume=82ad6ea0` and retain it in
`mVolumes[0]` and part 6. Wrapper is enabled, group/surface zero, radius zero,
identity axes/zero translation, and references actual mapped-array ownership.

Signed N<=0 skips triangles, but earlier allocation/count arithmetic and later
assertion are not replaced by a source clamp. Debug storage has only 32 entries
and native indexing is unchecked. Safe-host capacity/index rejection must be
explicit, not claimed as malformed native behavior. Fresh debug count advances
from six wheel/truck objects to nineteen: one box, two capsules, ten triangles.

## Material, mass and post-mass frame

After aggregate update/wrapping, store static/dynamic friction without wheel
friction arbitration; store restitution; angular drag is input multiplied by
`426fffff`, not exact 60. Multiply deck mass by world multiplier, then call
`ComputeMassProperties(part6,mass,1,null)` at `82b33d30`.

Force `m_hasInverseBodyLTM=1` and overwrite inverse-body transform with
I/J/K/zero at `82b33d70..82b33d84`, then publish debug range end. This later
override does not remove the actual 13-child aggregate mass dependency.

## Validation boundary and fingerprints

Raw mounted deck-input construction now passes: actual db.big Vault instances
feed four wheels, two trucks and the thirteen-child deck in source order.
Independent source, implementation and test review passed. Ten focused tests
in `skateboard_deck_tests.rs` distinguish
interleaved fan order, separate tilts, fused Z offsets, retained trig endpoint
values, all 26 fetches/delayed loads, byte truthiness, disabled-child bbox/mass
inclusion, capsule numeric chronology, final flags/edge cosines/dirty normal,
and forced post-mass identity. Root added analytic cancellation and independently
evaluated CRT fused/split discriminators for both rim Z offsets, plus aggregate
and debug-record checks. Host trig/estimate results are not Xenon bits; an actual
reference numeric trace remains pending.

Mounted host observations (not native output oracles): volume 0.011471862,
principal diagonal [0.08745143,0.0916046,0.0045132367], inverse tensor
[1.9058199,1.8194137,36.928413], debug range 6..19. All tuning comes from the
retained raw Vault graph; expected words are assertions, not constructor inputs.

The authored owner consumes the shared runtime `SkateboardAttributeRead`
interface, not a frozen geometry snapshot. It owns an actual `Arc<[Volume]>`
and aggregate metadata; it does not fabricate an arena aggregate object ID.
Capacity errors are a safe-host boundary after the initial five input reads.
Only active debug union fields are retained; capsule inactive shape words are
canonical host zeros, not claims about uninitialized source bytes.

The mass integration is explicitly the final-observable deck subset: compute
the actual aggregate integral, diagonalise it, retain principal outputs, apply
the shared unit-density mass/inertia suffix, and expose the caller-forced
identity inverse-body transform. It does not execute generic
`ComputeMassProperties`' potentially conditional intermediate inverse-frame
calculation that this caller immediately overwrites. Root approved this bounded
omission; it is not a claim of full generic mass-function numeric side effects.

End-exclusive spans and SHA-256:

```text
CreateTriangleDeck 82b32e08..82b33da4 5b20bc1dff54a2ae16b7be8042bbb0cc0b0590a2d67ffa0c0d268233952adb62
AddTriangle       82b2b8c8..82b2ba10 7745d332024e390818d7377216c169b404a8ecba905a4c4722d47882e073abdc
AddCapsule        82b2ba10..82b2bc0c a2ca3897d4ae8136857994ef03237a426dff51c7fe27e6ff31ca8359e6a9c891
InitializeCapsule 82e4b258..82e4b4f8 fc8a9859956fb678786200a72919c631b524a9429f451b11c05f8635b2efc856
TriangleCtor      82ac8d98..82ac8e60 8e7de310024acf171fbc857920bc4b22b20b4ac77880f0e618e4816a8d015994
MappedUpdate      82acf2b8..82acf388 ccc6c766ec3f2a150e541d1467915119d1d0fbac57677e4af1ce9211953ffded
MappedInitialize  82acf260..82acf2b4 7969792c746458f756ce807be8881b8827739c5e1e24c0fcd71fd7cc712fcdcd
AggregateInit     82ad6ea0..82ad6f30 e9c12b4a9376326f333e53768fe8a99ed75dae619a9d27d25443e26191ec632d
```
