# Prepared ragdoll dynamics evidence

This note scopes `ragdoll_dynamics.rs` to the mass/inertia-cache portion of
`Sk8::Physics::SkeletonBody::Init` in the supplied November 28, 2008 debug
reference. It does not claim to reproduce the unrecovered RenderWare or VMX
shape arithmetic.

## Symbols and layouts

The linker map places these functions in `physskeleton:skeleton_lib.obj` or
the named RenderWare object:

| Address | Symbol |
| --- | --- |
| `0x82b61738` | `Sk8::Physics::SkeletonBody::Init(SkeletonState&)` |
| `0x82b4fae8` | `SkeletonBody::CreateHatVolume(...)` |
| `0x82ae9a50` | `rw::physics::ComputeMassProperties(PartDefinition&, float, float, float*)` |
| `0x82e4ae50` | general box-volume creator |
| `0x82e4af08` | general sphere-volume creator |
| `0x82e4afa0` | general capsule-volume creator |
| `0x82e4b0e0` | debug inertia-tensor helper for mode 1 |
| `0x82e4b188` | debug inertia-tensor helper for mode 2 |

Relevant PDB facts are:

- `SkeletonBoneData` is 24 bytes: `MassFactor +0x00`,
  `RagdollMassFactor +0x04`, `HasCollision +0x08`, `UseRootDrive +0x09`,
  `VolumeType +0x0c`, `VolumeScalar +0x10`, and `NumParents +0x14`.
- `Andale::PhysicsParamBoneData` is 112 bytes. Its `mSize +0x20` is a
  16-byte `rw::math::vpu::Vector3`. The implementation preserves all four
  lanes as raw bits because the PDB name does not establish a semantic use
  for lane four.
- `SkeletonData::mBoneHandles[24]` starts at `+0x08` and
  `mBoneIds[24]` starts at `+0x1270`.
- `PartDefinition` is 160 bytes. Its volume pointer is at `+0x00`, its current
  inverse tensor begins at `+0x10`, and its normal material begins at `+0x80`:
  `m_mus`, `m_mud`, and `m_res` are at `+0x80/+0x84/+0x88`.
- The four `SkeletonBody` caches begin at `+0x7a4` (animated inverse mass),
  `+0x804` (ragdoll inverse mass), `+0x870` (animated inverse inertia), and
  `+0x9f0` (ragdoll inverse inertia). The common-install loads use
  `SkeletonBody +0xb84` for `mMaterialNormal.m_mud` and `+0xb88` for
  `mMaterialNormal.m_res`. Hat fields are `mHatAggregate +0x98`,
  `mHatPresent +0x9c`, and `mLastEnableHatCollision +0x9d`; the 24-entry
  `mVolumes` array begins at `+0xbb4`.
- `SkeletonState::mPartMass` begins at `+0x1100`.
- `PartDefinition::m_inverseBodyLTM` begins at `+0x40` and
  `m_hasInverseBodyLTM` is at `+0x8c`.
- A collision `Volume`'s `m_flags` field is at `+0x5c`.

Confidence: `exact` for the symbol addresses, type sizes, field offsets, and
24-entry iteration bound.

## Recovered ordering

`Init` reads hash `0x3689013d7adbdd2a` once at
`0x82b618a4..0x82b618d8` and requires an ordered value greater than positive
zero. It captures the physics bone ID at part zero, then visits all parts
`0..24` in order (`0x82b6195c..0x82b6207c`).

Shape selection has this exact priority:

1. Only when iteration reaches part 1 does the code read the live hat type.
   This is after every part-0 side effect; it is not a value sampled before
   the loop. A nonzero result calls `CreateHatVolume`
   (`0x82b61988..0x82b619c0`) and takes its out mass. This wins even if that
   part's physics bone ID equals the root ID. The collaborator's only valid
   successful typed result is `HatAggregate`; any other shape class is an
   impossible helper/result combination rejected by the host boundary.
2. Otherwise, a physics bone ID equal to part zero's ID uses the root path.
   The matching part index is stored first. Canonical high-word-first VLT key
   `0x5e43a9ebd2bebbde` supplies half-length, followed by key
   `0x7e90113b05bacda6` for radius. The separate capsule-construction
   collaborator receives radius before half-length and returns a prepared
   volume identity. Immediately after construction, `m_flags &=
   !VOLUMEFLAG_ISENABLED` clears exactly low bit 0
   (`0x82b61de4..0x82b61dec`); no other flag bit changes. Only after that
   mutation does the code write positive zero to
   `SkeletonState::mPartMass[part]`.
   Its effective mass is raw float `0x3a83126f`. Every later duplicate root
   bone ID repeats these root-index, construction, and mass-write side effects.
3. Otherwise, raw `VolumeType` 0/1/2 chooses box/sphere/capsule. The complete
   16-byte size, scalar, and part are sent to the shape-preparation boundary.
   Values greater than 2 reach a native debug invariant; the Rust boundary
   rejects them rather than modeling the unsafe continuation.

General paths write the VMX-produced unscaled integral to
`SkeletonState::mPartMass[part]` before reading hash
`0x9f4a03c75637e7e9` at `0x82b61cf8..0x82b61d40`. Only general shapes multiply
by that value in the outer `Init` path. `CreateHatVolume` owns its internal
unscaled-part-mass write and live lookup/scaling by the same key; the outer
orchestrator must not repeat them. The root path does not scale its constant.

The selected effective mass is compared with positive zero using an ordered
comparison at `0x82b61e1c`, and that result is retained while the common
install sequence executes. `0x82b61e30..0x82b61e54` stores the prepared volume
in `SkeletonBody::mVolumes[part]`, stores it again in
`PartDefinition::m_volume`, copies `SkeletonBody::mMaterialNormal.m_mud` into
both `PartDefinition::m_material.m_mus` and `.m_mud`, then copies
`mMaterialNormal.m_res`. Only then does `0x82b61e58` branch on the saved
comparison. The typed boundary therefore separates a prepared-volume identity
from the stable installed-part-definition identity. Both mass-property calls,
the optional hat identity write, and either debug helper receive that same
part-definition identity.

### Hat helper boundary

`CreateHatVolume` is kept as one opaque stateful callback, but its externally
observable contract is more specific than merely “make a volume”:

1. It sets `mHatPresent = true` before allocation at `0x82b4fb20`.
2. It takes a first live `kEnableHatCollision` sample at
   `0x82b4fb28..0x82b4fb34` and stores that result in
   `mLastEnableHatCollision +0x9d`.
3. It creates an inner `SimpleMappedArray` with exactly two children and stores
   that inner identity in `mHatAggregate +0x98` at `0x82b4fbcc`.
4. Child 0 is a cylinder, created at `0x82b4fc0c..0x82b4fc18`. A second,
   independent live `kEnableHatCollision` sample at
   `0x82b4fd54..0x82b4fd70` controls the cylinder's enabled bit. The two samples
   can differ; neither may be reused for the other.
5. The helper produces its unscaled out mass at `0x82b4fdd4`, writes that
   unscaled value to `SkeletonState::mPartMass` at `0x82b4fe0c`, then reads key
   `0x9f4a03c75637e7e9` and produces the scaled final out mass at
   `0x82b4fe48..0x82b4fe50`.
6. Child 1 is a sphere, created at `0x82b4fed4..0x82b4fed8`; its enabled bit is
   set at `0x82b4fefc..0x82b4ff00`.
7. The inner aggregate is updated only after both children at
   `0x82b4ff28..0x82b4ff38`.
8. Finally, `0x82b4ff3c..0x82b4ffb4` creates and returns an outer
   `AggregateVolume` wrapper. `PreparedShape::HatAggregate.volume` is this
   outer wrapper, never the inner `SimpleMappedArray`. The Rust observation
   carries a separately typed inner identity specifically to prohibit their
   conflation; the outer API does not otherwise expose or manipulate the inner
   pointer.

Cylinder/sphere dimensions, transforms, VMX arithmetic, allocation details,
and aggregate internals remain opaque. `HatPreparationObservation` records the
two snapshots, resulting child flags/order, inner identity, and raw mass values
without implementing those formulas.

For every valid part, the two passes are:

1. Load `RagdollMassFactor` at `0x82b61e98`, multiply it by effective mass,
   derive and write inverse mass using the earlier numerator, call
   `ComputeMassProperties` at `0x82b61ea8` with exact density `1.0`
   and null optional output, check its boolean result, and on success
   immediately write all four inverse-inertia lanes at
   `0x82b61ee4..0x82b61eec`.
2. Load `MassFactor` at `0x82b61ef0`, multiply it by effective mass, derive
   and write inverse mass from numerator `1.0`, and make the second
   mass-property call at `0x82b61f00`, overwriting the working inverse tensor.
   Its boolean failure is distinct from the first call's failure.
3. Part 1 reads the live hat-present byte after the second call; when true it
   writes `m_hasInverseBodyLTM = 1` before installing the four identity rows
   at `0x82b61f20..0x82b61f5c`. This receives the same installed part-definition
   identity as both mass-property calls.
4. Every part reads debug mode hash `0xbe03a6ea6ddd9384`. Neutral typed names
   `Mode1` and `Mode2` correspond only to raw values 1 and 2. Those values alone
   then read multiplier hash `0x54441f41425c5017` and call their respective
   helper. The final animated four-lane copy occurs only afterward at
   `0x82b62068..0x82b62074`.

The safe boundary has four deliberately explicit failure classes:

- A nonpositive or unordered ragdoll inverse-mass numerator aborts before the
  part loop, with no per-part callbacks or writes.
- A nonpositive or unordered effective mass aborts only after that part's
  volume, part-definition, and material install writes. This ordering follows
  the saved condition-register result described above.
- Either false `ComputeMassProperties` result becomes
  `MassPropertiesFailed { part, pass }`. The backend contract explicitly
  retains writes made inside the false-returning call, because the native
  routine can modify its `PartDefinition` before returning false. Earlier
  cache/state writes also remain: a second-pass failure includes the completed
  ragdoll inertia write.
- `PreparedShapeMismatch` is only a host-side collaborator-contract check. It
  rejects impossible successful-helper combinations such as a hat callback
  returning a box. It has no executable failure-path parity claim and does not
  undo path-dependent preparation effects. For the hat path, every helper
  effect listed above has already occurred before the mismatched result can be
  observed.

The first three safe aborts do not roll back. Callers must discard the partial
state and collaborators and must not retry them. Replacing the debug
executable's assert/failure continuation with these safe exits is a
`high`-confidence deliberate deviation, not a parity claim for invalid input.

Confidence: `exact` for valid-path branch/call/read/cache-copy ordering, index
ranges, hashes, boolean outcomes, and raw constants. The Rust multiplication
expresses the recovered single-precision data flow, but no cross-host
bit-parity claim is made without a reference trace.

## Floating-point selection facts

The sphere scalar selection at `0x82b61c08..0x82b61c0c` keeps the first
candidate for finite equal candidates and chooses the second candidate when
the subtraction is unordered. In particular, same-signed infinity subtraction
is unordered; this is not described as an ordinary tie. The capsule selection at
`0x82b61ae0..0x82b61ae4` keeps a positive remainder, selects the executable's
positive-zero operand for either zero sign or a negative remainder, and
bit-preserves the original remainder when unordered. These follow the
operand order and the documented PowerPC `fsel` rule: a NaN comparison chooses
the fourth operand, while either zero sign takes the greater-than-or-equal
arm. See IBM's
[`fsel` instruction definition](https://www.ibm.com/docs/en/aix/7.3.0?topic=set-fsel-floating-point-select-instruction).

Confidence: `exact` for selection polarity, ties, zero signs, and unordered
operand choice. The preceding VMX candidate construction remains outside the
module.

## Supplied data check

The mounted corpus copies independently hashed during this slice are:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `skater.vlt` | 851,568 | `36642cd06358d569632de24d02261286cec45a3fa7898a2edd8a95d7b62f6fff` |
| `skater.bin` | 812,520 | `458e8fa67489a612cb5239d566bea648980cb92a40b74af14fe929c18ffa5fd7` |

For the supplied data, the general-mass-scale hash resolves to raw
`0x43a00000` (`320.0`) and the ragdoll inverse-mass numerator resolves to raw
`0x3f800000` (`1.0`). Capsule-only dimension-scale key
`0x07204f10dd937907` resolves to raw `0x3f99999a`. That value is sampled once
before the native part loop: key construction is `0x82b6178c..0x82b6179c`,
lookup is at `0x82b617a4`, default handling is `0x82b617b0..0x82b617b4`, and
the load is at `0x82b617bc`. It is reused from `f25` on capsule paths, including
at `0x82b61aa0`. The typed function therefore accepts
one already-sampled value and forwards its raw bits unchanged to every
general-shape callback; it does not expose an in-loop lookup. These are
supplied-data observations, not hard-coded defaults: live controls and the
capsule snapshot remain caller-supplied.

## Explicit boundary and remaining evidence

The following are `hypothesis`/unknown for parity and are deliberately
collaborator operations:

- hat aggregate/cylinder/sphere geometry and its returned mass. The complete
  `CreateHatVolume` callback owns the proven `mHatPresent`, mapped-array,
  debug-object, volume-flag, `mPartMass`, internal mass-scale lookup, and
  aggregate-volume side effects at `0x82b4fae8..0x82b4ffb4`;
- general box/sphere/capsule VMX construction and mass integrals;
- `rw::physics::ComputeMassProperties` internals;
- the exact reciprocal/divide result on the reference VMX/PPC unit;
- the two debug inertia-helper internals;
- unrecovered side effects of non-hat volume construction and the surrounding
  parts of the full `SkeletonBody::Init` routine.

There is no host reciprocal fallback and no inferred mass, inertia, or shape
tuning in this slice. `PreparedShape` carries enough identity and operands to
make collaborator calls and tests observable. A deterministic reference trace
of the 24 input records, prepared shapes, two requests, and four output caches
is still required before claiming end-to-end runtime parity.

Native preconditions not representable by this typed 24-entry API remain out
of scope: the `PartIndex == -1` and `part >= 24` debug assertions cannot arise
from `SkeletonPartIndex`, and the surrounding initialized-state assertion is a
caller/lifecycle responsibility. They are not silently reclassified as any of
the implemented safe aborts.

## Focused validation

The isolated tests cover all 24 parts and both pass orderings; canonical root
keys; raw four-lane size preservation; late mutable hat-type observation;
hat-over-root priority; root-index/dimension/construction/zero-mass ordering;
duplicate root IDs; general shape/size/scalar and prepared-identity forwarding;
general unscaled-mass write before scale lookup; distinct volume and installed
part-definition identities; exact `m_mud`-to-both-frictions then restitution
install ordering; root flag clearing from initial `0`, `1`, and `u32::MAX`;
the hat observation's distinct inner/outer identities, two differing live
samples, child order/flags, and raw mass fields; impossible successful variant
combinations and retained path effects; same-part-definition forwarding to
both mass calls, hat identity, and debug helper; exact root, hat, and general request masses;
one raw-`0x3f99999a` capsule-scale snapshot remaining stable across callbacks;
every inverse-mass/inertia write point; late debug-adjustment copying;
conditional multiplier reads; each mass-property failure point and its
partial state, including backend internal writes on false; invalid volume
types; effective-mass `+0.0`, `-0.0`, negative-finite, and NaN rejection after
the full common install; and sphere/capsule
`fsel` finite ties, infinities, signed zeros, and NaNs. Standalone `rustc -D
warnings` and `clippy-driver -D warnings -W clippy::pedantic` harnesses include
the existing `ragdoll.rs` part mapping. The file is intentionally not exported
until root integration.
