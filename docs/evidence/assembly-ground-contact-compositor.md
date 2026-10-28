# Assembly/ground Contact final-value compositor

This checkpoint reconstructs the private pure final-value graph inside
`AssemblyTriangleIntersectionTester::RunIntersectionTests` at
`0x827cd500..0x827cd778`. For one already accepted world-space contact pair it
builds all sixteen semantic rows of the native `0x100`-byte `Contact`. It does
not yet enumerate live volumes, reserve a temporary slot, append a Contact, or
authorize a simulation-frame producer.

## Authority

- Xbox 360 executable SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`
- full PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`
- linker MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`
- complete Run raw packet SHA-256:
  `B4FC2DF4223001B753A1B702BDB9478DF281C4CD9108B1C925A40273018C3A8E`
- full `0x827cd1e0..0x827cd7c8` native span SHA-256:
  `81C7E79EA8038EC1A61F45F03A3DC203A18B7742EFCA214EBA493C83B9FE4983`
- independently sliced `0x827cd500..0x827cd77c` span SHA-256:
  `090CABE5258A305D2271C47C80515E42D1944B131D9726841E38E03BDDDEDD5E`
- corroborating network-compositor packet SHA-256:
  `2010F8D2B8B4E3211FA5802F5252B5B2ABCD0CAE41EB228713CB18BA3BB64607`
- audited `simulation_contact_jacobian.rs` source SHA-256:
  `062B1B5DDF09A4CEE2AC04BA7BBF878B69D341C9DDA3C14BEE6711326AA0C649`

Here body A is the tested Assembly volume and body B is ground.

## Reconstructed numeric graph

The two contact radii are standalone subtractions:

```text
ra = position_a - body_a.center_of_mass
rb = position_b - body_b.center_of_mass
```

Point velocity uses the exact nested fused grouping below, where `YZX` and
`ZXY` are cyclic permutations, `FMA(a,b,c)` is fused `a*b+c`, and
`FNMS(a,b,c)` is fused `c-a*b`:

```text
point_velocity(radius, velocity, omega) =
    FMA(ZXY(radius), YZX(omega),
        FNMS(YZX(radius), ZXY(omega), velocity))

velocity_a = point_velocity(ra, body_a.velocity, body_a.omega)
velocity_b = point_velocity(rb, body_b.velocity, body_b.omega)
relative   = velocity_b - velocity_a
```

The dynamic tangent is `relative x normal` using the same nested FMA/FNMS
schedule. Its squared magnitude accumulates X, then Y, then Z with three fused
operations. Magnitude equal to `f32::MIN_POSITIVE` takes the dynamic path;
less-than or unordered takes the deterministic fallback.

The fallback computes both `axis_x - normal*normal.x` and the bit-negation of
`axis_z - normal*normal.z`. The X candidate is selected exactly when the fused
value `0.5 - normal.x*normal.x >= -0.0`; otherwise the Z candidate is used.
The selected scalar magnitude is passed to the sole, unrefined
`vrsqrtefp` at `0x827cd6cc`. Its four-lane result scales both the tangent and
the source-shaped `normal x tangent` binormal. There is no Newton refinement
and no second estimate.

The shared point-velocity, tangent-choice, and binormal helpers are also used
by the separately proven network compositor. Only those numeric kernels are
shared. The whole network builder is not reused because it transforms a local
point, applies displacement, assumes different roles, and clamps materials.

## Material and tag rules

The three source comparisons are independent and deliberately do not use host
`min`/`max` helpers:

```text
restitution = volume.res < ground.res ? volume.res : ground.res
dynamic     = volume.mud > ground.mud ? volume.mud : ground.mud
static      = volume.mus > ground.mus ? volume.mus : ground.mus
```

The `fcmpu` branch graph means equality and unordered comparisons choose the
ground operand, preserving its signed-zero or quiet-NaN payload. There is no
later restitution-to-one or friction-to-zero clamp.

The final tag does not come from `Sk8ComputeContactPoints` scratch tags:

```text
tag = ((volume.surface_id & 0xffff) << 16)
    | ((triangle.user_tag >> 16) & 0xffff)
```

## Complete final rows

The private output remains the exact existing `SimulationContactBuildInput`:

| row | XYZ | W word |
|---:|---|---|
| S0 | volume-side point | table body A |
| S1 | ground-side point | table body B |
| S2 | packed Sk8 normal | restitution |
| S3 | scaled tangent | static friction |
| S4 | scaled binormal | dynamic friction |
| S5 | ground point velocity minus volume point velocity | packed surface tag |
| S6/S7 | A/B center of mass | A/B RF-row binding |
| S8/S9 | A/B full inverse inertia | A/B inverse mass |
| S10/S11 | A/B split inverse inertia | A/B state word |
| S12/S13 | A/B force | A/B kinematic value |
| S14/S15 | A/B torque | A/B cooldown word |

The two table identities and two RF-row bindings use distinct private token
types and are supplied independently. The code neither derives one pair from
the other nor assumes that equal-looking roles can be collapsed.

## Native chronology outside this pure seam

Native reserves a temporary Contact before doing any of this numeric work and
immediately exits the whole Run invocation if reservation returns null. It
clears both 128-byte halves, initially writes body pointers into S0.w/S1.w,
and overwrites those two words with PPU table identities only at
`0x827cd768..0x827cd778`. It also rereads body COM rows and the ground table
identity late. The pure final-value function represents the observable final
rows, not those intermediate stores or live rereads.

Consequently, a future production owner must bind immutable body occurrences
before reservation and use an infallible estimate/compositor afterward, or
explicitly document any different host rollback policy. Computing the record
before reservation would incorrectly reorder numeric work around the native
flush/null boundary.

## Discriminating tests and nonclaims

Focused tests freeze:

- one literal sixteen-row packet with unrelated table-A, table-B, RF-A, and
  RF-B words and distinct numeric body rows;
- ground-winning signed-zero ties and unordered quiet-NaN payloads, plus
  end-to-end negative friction and above-one restitution values proving no
  compositor clamp;
- body-specific point velocities and exact B-minus-A relative-velocity sign;
- the manually packed tag halves;
- one lane-distinct estimate call and operand shared by tangent and binormal;
- the adjacent `0x3f3504f3`/`0x3f3504f4` normal-X inputs on opposite sides of
  the fused X/Z fallback cutoff;
- the existing shared dynamic/fallback, minimum-positive equality,
  point-velocity FMA, and binormal-association network oracles after factoring.

Exact Xenon estimate results, exceptional VMX arithmetic, denormal handling,
NaN payload propagation outside the explicit material selectors, vector W
intermediates, and FPSCR behavior remain open. No `Contact::IsValid`, sorting,
deduplication, or manifold reduction is added. The compositor types and
constructors remain module-private and expose no append or body capability.

The effective-padding graph is now separately reconstructed in the private Run
module. The remaining boundary is live triangle-major/volume-minor/point-minor
traversal: eligibility/culling, exact body and table occurrence binding,
reserve/write/null-abort behavior, and final tail publication through the
shared temporary buffer. The opaque reserve-first write capability is now
separately proven, but no live frame issues it or connects this compositor.
See `docs/evidence/assembly-ground-contact-write-port.md`.
