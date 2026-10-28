# Original authored truck definitions

Date: 2026-09-07. Independent source/code review, nine focused tests and
raw mounted wheel/truck/deck construction passed. Confidence:
`exact` for checked source keys, branches and instruction order; `high` for
the readable implementation. Host reciprocal estimates are not Xenon traces.

`SkateboardBody::CreateTrucks`, `82b33da8..82b344f0`, called after CreateWheels
and before CreateTriangleDeck by the actual board initialization path. SHA-256:
`6fe738fe6f8b28f85be7ea0427399c20e45a97c5eb536574a38782c125d38483`.
The fresh retained owner supplies parts 4/5 sharing one actual capsule; it is
not yet connected to the assembly registration, transforms or six joints.

## Runtime attributes and timing

All lookups use index zero and collection `default=d7edbd362d7d2152`.
Classes: trucks `af781d7b614dab50`, wheels `46add2eea90cc062`, world
`485a76b68ca65d6f`. Full keys and original extracted-data words:

| Class | Meaning | Key | Word |
|---|---|---|---|
| trucks | Radius multiplier | b803dfbe2e4b56fb | 3f09999a |
| trucks | Width multiplier | 03edf4ad8e1741f6 | 3faccccd |
| wheels | Radius | 12646f5e842c4db3 | 3cfdf3b6 |
| wheels | Half width | 5a84c435dc340af8 | 3dc28f5c |
| trucks | Collision enable | ddcaedd8fa8f5529 | 01000000 |
| trucks | Dynamic friction | 7e135b944834707e | 00000000 |
| trucks | Static friction | b34a64870f3f5944 | 3f666666 |
| trucks | Restitution | 329805a82950ab54 | 00000000 |
| trucks | Mass | 8e3a799a07260fd8 | 3ec7ae14 |
| world | Mass multiplier | 46507e40c85372fc | 40a00000 |

Exactly 15 lookups: the first five rows once; dynamic/static/restitution twice;
mass/world twice. First four float loads immediately follow their lookups.
Enable loads the **first byte**, not word truthiness (`82b34060=409a0008`).
Missing typed pointers select DefaultDataArea(1/4); unsupported representations
error rather than becoming a missing-value fallback.

For each material, obtain dynamic then static pointers, then load dynamic then
static. No friction clamp, NaN sanitization or per-part cache. Both materials
finish before either mass call. For each mass, obtain truck then world
pointers, load world then truck, multiply **world * truck** (operands at
`82b34394/82b3446c`). Calls at `82b343a8/82b34478` preserve separate samples.
Debug appends follow both mass calls.

Shared `skateboard_attributes.rs` distinguishes lookup handles from later loads.
Its retained class bindings follow native selected order deck, world, wheels,
trucks in GlobalAttributeCollection (`828b1608`), not a new editor lifecycle.
Trucks Find/Instance/publication are `828b2864/828b2874/828b2880`, member +dc.
The immutable concrete Vault graph and checked exact-size typed reads retain
the boundaries in `vault-runtime-fetch.md`. A scripted mutable test provider
is a timing discriminator, not evidence of implemented live Vault mutation.

## Geometry, mass and defaults

All ordinary operations below round to f32 individually:

```text
radius = (wheel_radius * truck_radius_multiplier) * 0.5
half_height = (truck_width_multiplier * wheel_half_width) * 0.5
```

Capsule initialization `82ad7fa0` sets local Z axis, I/J/K/zero transform with
all W lanes zero, radius +50, half-height +44, group +54 and surface +58 zero,
flags +5c one. Enable clears/retains bit zero. Inactive shape-union words are
canonical host storage only, not claims about native initialized bytes.

Reuse audited fresh capsule unit-mass and density-one suffix documented in
`skateboard-wheels.md`. No wheel-only maximum-omega override. Native part
initialization `82ae9058` retains maximum velocity/omega MAX, drag +0 and
nonstatic state. An ordinary positive finite fresh symmetric capsule does not
publish an inverse-body frame; zero/NaN threshold behavior remains in the
shared helper. Uninitialized inactive storage is represented by None. Inverse inertia
W retains X. Estimate callbacks remain Y, X, Z with the source refinement order.

Native debug constructor `82b383c8..82b38424` initializes 32 records: type zero,
identity/zero offset, zero size/radius/half-length and runtime White=[1;4].
White is initialized at `82f75a08..82f75a24`; static IDB zero bytes are not its
runtime value. Triangle points are untouched, represented as None. The truck
suffix `82b3447c..82b344d8` appends two CAPSULE=2 records and half-height/radius.
Fresh wheel count four becomes six. Host rejection of initial count >30 occurs
before callbacks, an explicit memory-safety policy, not native failure parity.

## Validation boundary

Nine scripted tests, independently reviewed, distinguish all lookup/load events, resampling,
delayed pointer mutation, enabled-byte semantics, unmodified material bits,
multiply-before-half overflow, shared geometry/defaults, debug values/capacity
and load-error propagation. The root mounted definition corpus exercises raw
db.big -> retained graph -> wheel/truck/deck constructors in order. Neither
test route registers a board or proves platform numeric equivalence.
