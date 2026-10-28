# Fresh skateboard wheel definitions

Date: 2026-09-07. Implementation:
`crates/skate2-core/src/physics/board/runtime/skateboard_wheels.rs`. Confidence is `high` for the
typed reconstruction; addresses, layouts, branch encodings, scalar constants
and quoted original attribute words are `exact` source facts. Host arithmetic
tests are not reference-runtime traces.

## What this implements

The bounded `SkateboardBody::CreateWheels` slice creates four ordered authored
Part definitions sharing one initialized sphere/capsule collision volume. Each
part retains its own computed `SimulationInertia`, material, inverse-body-frame
presence, inverse-tensor W lane, and debug-sphere index. The owner retains the
immutable tuning snapshot. Private construction/accessors preserve that source
relationship without supplying fabricated serialized arena objects or body
handles. It can become part of a later seven-part owner.

This is not a complete skateboard, actual player spawn, registered simulation
assembly, wheel placement, joint/drive graph, or skating controller. Current
production callers do not yet consume it. The serialized assembly template
requires an arena-backed definition; that is not valid provenance for newly
authored parts. Integration must add a real runtime-definition path, not invent
BAM bytes to satisfy the parser. Existing body pools, collision GP construction,
ground-contact and Horus/integration calculations remain the downstream reuse
targets. `ragdoll_dynamics.rs` still has a mass-properties collaborator; this
change does not claim that collaborator is integrated.

## Source and ownership

| Reference | Address/span | Role |
|---|---|---|
| `?CreateWheels@SkateboardBody@Physics@Sk8@@AAAXXZ` | `82b344f0..82b34a8c` | creation order and attributes |
| `SkateboardBody::Init` | call `82b37690` | sole direct caller; wheels precede trucks/deck/joints |
| `AssemblyDefinition::Initialize` | `82ae13f0` | initializes all seven fresh definitions first |
| `PartDefinition::Initialize` | `82ae9058` | inertia/material defaults |
| `SphereVolume::Initialize` | `82ad9438..82ad94c8` | sphere state |
| `CapsuleVolume::Initialize` | `82ad7fa0..82ad8034` | capsule state |
| `ComputeSimpleInertia` | `82ae93c0..82ae9560` | supported primitive dispatch |
| `ComputeCapsuleInertia` | `82ae8100..82ae81d0` | capsule diagonal/volume |
| `ComputeMassProperties` | `82ae9a50..82ae9df4` | mass application and inverse frame |
| `Attrib::DefaultDataArea` | `828b5340` | returns zero-filled area at `83059d30` |

Live byte-span SHA-256:

- CreateWheels: `C9E38A4EBBE5AF418C2C5A132B755E80AAA74BB182D70B3AAADE3CC98E8B8AA2`.
- ComputeSimpleInertia: `55E117E92491F05866FE79C00690346171834B5E862FDD8EFA57831AF53B35FF`.
- ComputeCapsuleInertia: `CF4AB01148896F00B4642E4AA3BF38F249C4C3DB6BD374C1D750DE368B40B007`.
- ComputeMassProperties: `89453978A60750FC1440543803D5B2A1125FAA28B7ADF7F7DBDEDE87601F53BD`.

PDB `PartDefinition` is `0xa0`: volume `+0`, Inertia `+10` (size `30`),
inverse-body transform `+40`, material `+80`, inverse-present `+8c`, static word
`+90`. `Inertia` offsets are inverse tensor `+0`, inverse mass `+10`, spherical
`+14`, max velocity `+18`, max omega `+1c`, linear/angular drag `+20/+24`.

Fresh part defaults reuse `SimulationInertia::native_default`: inverse tensor,
inverse mass and spherical 1; both maxima `7f7fffff`; both drag values +0.
The inverse-present and static words are zero. Inactive inverse-matrix storage
is uninitialized natively and represented by `None`, not an invented matrix.
CreateWheels sets max omega to `476a5fff` before mass calculation, which does
not overwrite either maximum or either drag control.

The native body debug array is PDB `SkateboardBody` ordinal 18765:
count `+410`, 32 objects of stride `b0` at `+420`, start/end arrays `[7]` at
`+1a20/+1a3c`. Host construction refuses an initial count above 28 before any
numeric calls. This is an explicit safe-host error; native indexing is unchecked.

## Original tuning, not invented defaults

The existing `inspect_vault.exe` read extracted original `skater.vlt/bin` from
the supplied game's `data/db/unpcaked#/data/db` directory. These extracted
copies have direct pointer offsets pre-applied; they are NOT byte-identical to
raw archive payloads (see mounted verification below). Extracted-file hashes:

- VLT: `36642CD06358D569632DE24D02261286CEC45A3FA7898A2EDD8A95D7B62F6FFF`.
- BIN: `458E8FA67489A612CB5239D566BEA648980CB92A40B74AF14FE929C18FFA5FD7`.

`physicswheels` class `46add2eea90cc062` has one `default` collection
`d7edbd362d7d2152`, no parent/layout, and 13 explicit entries. `physics_world`
class `485a76b68ca65d6f` has the same default key and one explicit entry. These
inputs are four-byte scalars, type ID `16294631463297616344`, node flags `40`,
inline data words; there is no pointer to a separately stored value.

| Attribute | Key | Original word |
|---|---|---|
| WheelCapsuleLength | `20a54376b6f2c399` | `00000000` |
| WheelRadius | `12646f5e842c4db3` | `3cfdf3b6` |
| WheelDynamicFriction | `2a1082ddfeef3ea7` | `3f333333` |
| WheelStaticFriction | `7d777041dbfa9234` | `3f4ccccd` |
| WheelRestitution | `5d095e6365260b84` | `00000000` |
| WheelMass | `a356746a6fe7dc00` | `3da9fbe7` |
| World mass multiplier, original spelling unresolved | `46507e40c85372fc` | `40a00000` |

The six wheel spellings independently match the recovered StringHash64
algorithm. These are hash-verified preimages, not an original spelling table.
The last row's descriptive label is not claimed as an original attribute name.
All source reads use element index 0 and call `DefaultDataArea(4)` only after
a missing attribute pointer. Snapshot `None` therefore supplies bits zero and
retains that origin; supplied NaNs and signed zero are retained bit-for-bit.
The shipped numbers appear in tests, not as silent runtime fallback tuning.

The snapshot is not itself a VLT loader/provenance capability: its caller must
resolve the correct collections, keys, index and inheritance. Snapshotting is
explicit immutable host ownership policy. Native loads retained length only
after radius lookup, and retained mass only after world multiplier lookup;
dynamic friction is loaded before static lookup, and radius is reread for each
debug sphere. Equivalence only covers immutable tuning during construction,
not arbitrarily mutating callbacks or concurrent attribute reloads.

## Control flow and source ordering

One allocation creates the shared volume. Source allocation labels are
WheelSphere/WheelCapsule; size `60`, alignment 16. Flags are 1, surface zero,
group initially zero then set to 5; the body retains it in `mVolumes[2]`.
Its transform is I/J/K with a zero translation vector, including element 15=0.
Capsule half-height is one single-precision `length * 0.5` operation; its axis
is local Z. The existing semantic `Volume` adapter uses type discriminants 1/2,
not a made-up guest pointer. Unused union words are canonical zero host storage,
not a claim about native uninitialized bytes.

Three alias-reading errors in the preliminary external packet were corrected
by independent review of raw PPC BO/BI fields before implementation:

| Branch | Raw instruction | Correct predicate |
|---|---|---|
| `82b345c8` | `40990120`, BO4 BI25 = !GT | capsule only when length > 0; NaN selects sphere |
| `82b34858` | `40980008`, BO4 BI24 = !LT | replace static only when static < dynamic; NaN/tie retain static |
| `82ae9cd8` | `40980010`, BO4 BI24 = !LT | density fallback only when mass < smallest normal; NaN remains mass |

After shared geometry/material setup, visit parts 0,1,2,3. Per part: assign
volume, static/dynamic friction, max omega; read/store restitution; obtain
mass/world multiplier, multiply with `fmuls`, call mass properties with density
1 and null volume output; append one debug SPHERE=3 of the configured radius.
The debug primitive remains a sphere for the capsule collision branch.
No per-wheel offsets are applied here. `InitializeTransforms` is a later call
at `82b37708`; inventing placement in this owner would change source semantics.

## Fresh primitive mass numerics

Sphere uses `r = fsel(radius - eps, radius, eps)` with eps `33d6bf95`.
The subtraction is rounded first; unordered selects eps, nonnegative including
-0 selects radius. Diagonal is `(r*r)*3ecccccd` on all three axes; volume is
`((r*r)*r)*40860a92`. Do not replace source fsel with generic min/max.

Capsule uses positive half-height `h` from the shape branch. Each plain
operation below is f32; `fma(a,b,c)` is one `fmadds`:

```text
h2=h*h; minimum=h*3d4ccccd; h3=h2*h
r0=fsel(radius-eps,radius,eps)
r=fsel(r0-minimum,r0,minimum)
t=fma(r,3fcccccd,h*0.75)
den=fma(r,4,h*3)
length=fma(r,3faaaaab,h*2)
partial=fma(t,r,h2*4)
numerator=fma(partial,r,h3)
diagX=diagY=numerator/den
diagZ=(r*r)*3ecccccd
volume=((length*r)*r)*40490fdb
```

This is the source formula, including its axial diagonal, not an idealized
uniform-capsule replacement. PI at `8225aa88` has bits `40490fdb`.

The mass routine's fresh volume frame is known identity/zero. Its threshold is
`((diagY+diagZ)+diagX)*358637be`. Raw branch `82ae9be0=4098009c` (!LT) takes
inverse construction when NOT(`0 < threshold`), including NaN. In that arm,
the source transpose and Z/Y/X translation products yield the same I/J/K/zero
matrix with W15=0. Otherwise the exact fresh axes pass the identity check and
inverse-present is false. General arbitrary-frame/volume-integral behavior is
not implemented or approximated here.

Mass below `00800000` becomes `volume*1`; otherwise supplied mass is kept.
Inverse mass is scalar `1/mass`. Scale the diagonal by mass with separate
products. Visit reciprocal estimates in Y,X,Z order. All three residuals
precede all three corrections, repeated exactly twice:

```text
residual = fma(-denominator, seed, 1)
seed = fma(seed, residual, seed)
```

Pack inverse tensor X,Y,Z; its W lane retains X. Spherical is the scalar
reciprocal of the selected minimum inverse tensor: choose Y unless X<Y, then
choose Z unless selected<Z. Ties/unordered therefore select the later operand.
Maxima/drag remain untouched. The executable host seed uses `f32::recip`,
followed by these source refinements. Xenon estimate bits, exceptional VMX
arithmetic, NaN payload propagation, denormal/FPSCR behavior are not claimed
bit-identical. Branch selections and stored tuning bits remain explicit.

## Validation

Focused tests cover exact hashed keys/default origin; four ordered parts sharing
one volume; all material/control fields; W15=0 and inverse-tensor W=X; debug
ranges/capacity; native unordered and signed-zero branch distinctions; scalar
sphere and capsule packets; strict mass fallback and NaN retention; exceptional
COM threshold; and unequal injected Y/X/Z seeds distinguishing two refinements
and reciprocal-of-minimum spherical selection.

Independent source scalar evaluation for the shipped sphere predicts mass
`3ed47ae1`, unit diagonal `39c9894b`, volume `3902d997`, mass-scaled diagonal
`3927466f`, inverse mass `401a3785`. For radius `3cfdf3b6`, capsule half-height
0.25, independent rational-intermediate and CRT `fmaf` evaluators agree on
diagonal `[3cdd3abf,3cdd3abf,39c9894b]`, volume `3ad636cc`. These are explicitly
host/source-derived predictions, not captured runtime observations.

Workspace validation and eventual mounted-data/runtime integration remain the
root integration gate. A full physical skateboard still requires recovered
trucks, deck, transforms, joint/drive setup, culling, and native island admission,
followed by actual per-tick reference contact and pose comparisons.

### Mounted original attribute corpus

`skateboard_wheels_corpus.rs` adds an ignored regression reading the actual
`data/db/db.big` through the existing original mount table and `MountSet`.
Both `data/db/skater.vlt` and `data/db/skater.bin` are loaded from that archive;
no extracted-file fallback or generated tuning supplies this test. The mounted
bytes must match independently inspected raw BIG4 payloads: VLT offset 256,
length 851568, FNV-1a64 `f7db0ed527db57b4`; BIN offset 851840, length 812520,
FNV-1a64 `6b4fc086777fe4df`. Raw archive SHA-256 identities:

- VLT: `496215B210C3CE8BDA3F7A64B3CC3D59C56546E8D3026DBF1DA4A72F93FA7DDE`.
- BIN: `0D20A6CB05A21CF1F523478132319EA4B6BDFECA7FB29C7959A8BA069DCFE7AA`.

The first mounted run correctly failed against extracted-file fingerprints
`d7fc097bf5e37f2f` and `b9bd60975926f6b9`. An independent raw BIG4 reader and
root direct-region SHA checks confirmed distinct representations. Every one of
22970 VLT / 10968 BIN differing bytes belongs to a `PtrN` Direct slot: 7887 VLT
and 5456 BIN changed words are zero in the archive and exactly the relocation
argument in the extracted copy. There are no differences outside those slots.
All seven wheel tuning words remain identical. Correcting the archive oracle
does not change or regenerate the scalar/numeric expected values. The test
uses existing pointer resolution over raw mounted data and does not depend on
the extracted copies.

Existing `AttributeVaultMetadata` pointer resolution and class-definition
decoding retain both dependency buffers. Collection decoding selects the two
exact class/default tuples, rejects duplicate keys, and verifies the shipped
no-parent/no-layout, single-element, four-byte inline scalar representation and
type metadata. The sole dependency is checked as asset `skater.bin`, ID
`16762145128419181554`, before assigning its buffer index; a collection layout
slot may have no fixup or an explicit null fixup, never a non-null relocation.
Each word is decoded from the actual collection entry; frozen
expected words are only comparison oracles, never construction inputs. The
resulting immutable snapshot retains seven `ResolvedAttribute` origins and
constructs four ordered parts sharing the shipped sphere with the observed
mass/material/control values. This narrow corpus resolver is test-only and
does not pretend to implement general runtime Attrib inheritance, arrays,
layout fields, mutable reloads, or missing-pointer service behavior.

Initial debug count zero and `HostSkateboardWheelMassNumeric` are explicit
harness choices. The test proves mounted tuning reaches the real definition
constructor, not that a seven-part skateboard is spawned or native reciprocal
estimate bits have been observed. Run with the external game root set:

```powershell
cargo test -p skate2-core mounted_skater_attributes_create_four_source_wheel_definitions -- --ignored --nocapture
```

Root execution now passes the mounted test, including all seven actual scalar
words and the shared four-part owner. Both core ignored corpus tests pass
together. Final workspace gate passes formatting, 2208 unit tests, six
doctests, and all-targets Clippy with warnings denied. Three corpus tests are
ignored by the ordinary workspace command (two core explicitly rerun here,
one pre-existing app test). Independent source and corpus reviews accept the
bounded path; captured Xenon per-tick numeric parity remains unclaimed.

### Shared retained Vault reader integration (2026-09-07 follow-up)

Wheel owner inputs now come through the shared immutable read graph in
attribute_runtime.rs, not the older inline-only test decoder. Production
SkateboardWheelAttributes binds world then wheels in the source's retained
GlobalAttributeCollection order and performs scalar FETCH when sampling.
Unsupported representations propagate errors; only missing/null pointers get
the existing caller-specific zero defaults. The original inline decoder is
still an independent mounted assertion oracle. All seven words and four-part
results are unchanged. Snapshot equivalence remains explicitly immutable, not
full native repeated-read/editor timing; no board replacement is implemented.

The [reader evidence](vault-runtime-fetch.md) records explicit-parent/child-layout
precedence, retained identities, generated gravity's separate direct-layout
path, pointer/type guards and unsupported class-static registry storage. Final
root gate passes2240unit tests, six doctests, formatting, warnings-denied Clippy,
and all three ignored core original-data checks. The actual next board owner
requires the [authored-body path](skateboard-authored-body-path.md), including
six joints, two truck drives and the separate static hook assembly/drive.
