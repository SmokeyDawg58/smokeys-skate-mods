# Owner-bound SerialisedAssembly source and Create/Update volume topology

Date: 2026-09-06 (Europe/Helsinki)

This evidence chain now reaches a private, staged CPU
`EA::Physics::SerialisedAssembly` Create/Update owner. It joins immutable arena
topology to occurrence-checked live body snapshots, full four-lane Part
transforms, swept primitive bounds, Header/Params values, owner-qualified
culling identity, and the Part-order body array. It deliberately stops before
claiming a production DMO-session bridge or a native memory-layout object.

## Authority

- Xbox 360 executable SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`
- full PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`
- linker MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`
- supporting Create/Update and body-binding raw packet SHA-256:
  `8BA5D8692B87B688693B0E9E0E19A8354DA7C4012416CE654A5A5458118F4E8D`
- supporting `VolumeData::Initialize` raw layout packet SHA-256:
  `7E1BE886C77F7D6B441DB5912C50463FFDFD4F498EC53A54BD68FC46B8178A9C`
- supporting swept-bbox audit SHA-256:
  `2502B37462901CD7010DFF9B795C2A952A05649083D9674FC9398D95AF981731`
- supporting CPU `rw::BitTable` extraction SHA-256:
  `A75ABFE97EFFEAD55A37444D2E673F584A3F919B6C549C0AD7F237BC053EEAD1`
- audited private template source SHA-256:
  `CE6FC16581A0C77110B18ABFC0F8E6BE9BF9ED89927E68622879B356BF9C1511`
- audited staged Create/Update owner source SHA-256:
  `DCEBB7FAA76B60E467D617BA17A1867B60F8935A67720674DC5B70FEDF81B6B7`
- audited swept/primitive source SHA-256:
  `DF44098B9A3388349315E698E25A2DF816F0DFB56A58A733E01E37E8A15941FD`
- audited rigid-body occurrence source SHA-256:
  `D88C8D052F6FB98D1EC7F3D2FCF88239602B59A9915B91ED0ACF112D63FA8577`
- audited collision/BitTable parser source SHA-256:
  `B3F07B360DB309B9F1B6E49B3304987C32643EACC28F9EC8B2038B85FA96E138`
- audited DMO owner source SHA-256:
  `5C2E5F8CD2CD2D6EA84CADFE279FCF4A91097F57B1929A1212B36F5B25629C4E`
- audited real-data gate source SHA-256:
  `E8CDE09E2E0AD1479DC59A81E964FD9C8B59A20674518A240EA2E44519D86788`

## Recovered boundary

`EA::Physics::SerialisedAssembly::Create` at
`0x82e0c850..0x82e0cf2c` takes one fixed-up `rw::physics::Assembly`, follows its
`AssemblyDefinition`, and visits the Assembly Parts in index order. The PDB
fixes `SerialisedAssembly` at 0x1780 bytes with 26 inline `VolumeData` records
and 26 rigid-body pointers. Create bounds-checks individual emitted volume
records, but its earlier Part/body-pointer loop has no corresponding capacity
guard. Therefore `Assembly::numParts <= 26` is a native external prerequisite,
not a volume-truncation rule.

The private Rust template now:

1. resolves a local dictionary object of exact type `0x00040006` (`Assembly`);
2. follows its local `mDefinition` object of exact type `0x00040007`;
3. parses the base-relative 0xa0-byte `PartDefinition` array;
4. requires runtime, definition, and parsed Part counts to agree;
5. refuses more than 26 Parts before publication;
6. resolves each Part's local type-`0x00080001` root Volume in definition order;
7. retains the complete `SimulationNormalArenaPlan` in an `Arc`, so root
   Volumes and serialized source bytes cannot outlive their arena.

The DMO-bank processed-asset builder constructs every Assembly template in
dictionary order before returning an asset. A malformed link, type, count,
allocation, or capacity condition therefore cannot publish a partial arena.
The template remains crate-private. Public callers receive only aggregate
validation metrics, preventing this pre-Create structure from being treated
as a live `SerialisedAssembly`.

## Create volume selection topology

The same recovered Create body proves a deliberately shallow topology walk:

- an ordinary Part root emits one record;
- a top-level ClusteredMesh aggregate emits its aggregate wrapper;
- a top-level SimpleMappedArray visits children in stored u16 index order;
- an ordinary child emits, while a ClusteredMesh child emits its wrapper;
- a child SimpleMappedArray visits its grandchildren in stored u16 order but
  emits only nonaggregate grandchildren;
- aggregate grandchildren and other aggregate implementations are skipped.

Each eligible emission independently checks `numVolumes < 26`. Once full,
later records are silently omitted while traversal continues. The retained
source records contain the Part ordinal/body index, cached Volume type, exact
inline Volume copy, explicit surface ID, and literal padding `0.05f`
(`0x3d4ccccd`). They intentionally omit the values Create reads from live Part
and body state.

## Update selection is deliberately distinct

`SerialisedAssembly::Update` at `0x82e0cf30..0x82e0d35c` independently starts
its destination index at zero and replays a shallow source walk. Ordinary
roots and the SimpleMappedArray child/grandchild cases match Create, including
child ClusteredMesh wrappers and the per-emission 26-record cap. The aggregate
root branch differs: it admits only a SimpleMappedArray. A top-level
ClusteredMesh wrapper emitted by Create is skipped by Update. This is retained
as a separate typed source sequence rather than normalizing both functions to
one convenient traversal.

The Update source keeps the full selected Volume only as host-side occurrence
identity for mutation-resistant corpus validation. Native Update reads only
that occurrence's enabled bit before copying live Part material, transform,
user-data group, and body-frozen state.

## VolumeData live-field contract

The PDB fixes each native `VolumeData` record at 0xe0 bytes. CPU Create's
inlined stores and `VolumeData::Initialize` at `0x82e0b690..0x82e0b75c`
independently agree on the semantic fields: body index at `+0x00`, cached type
at `+0x04`, inline 0x60-byte Volume at `+0x10`, hardcoded `0.05f` padding at
`+0x70`, three-float Material at `+0x74`, explicit surface at `+0x80`, Part
transform at `+0x90`, enabled byte at `+0xd0`, Part user-data group at `+0xd4`,
and frozen byte at `+0xd8`. The formal padding argument to Initialize is not
consumed; the constant-pool literal is always stored.

`Part::GetTransform` at `0x826c3480..0x826c35a4` has three pointer-selected
paths. A detached Part copies `m_partTM` bit-for-bit. With a body but no
`m_inverseBodyLTM`, it copies the body pose bit-for-bit. With both, it computes
`body pose * inverse-body LTM` in the recovered multiply/FMA order. The native
assembly pool invokes Create only after adding Parts to Simulation, so the
live-record constructor requires a body pose/state while the independently
tested helper still preserves the detached branch.

The crate-private owner-qualified record set now performs the Create snapshot
and exact Update write set. Update changes only enabled, Material, transform,
group, and frozen in independently selected destination order; body index,
cached type, inline Volume, padding, and surface remain unchanged. Current
Volume flag words are supplied separately because native flags are live even
though selection topology must remain stable. Safe-host count, storage, and
owner checks precede every write and are not claimed as native exception
paths. Native intermediate-store visibility under concurrent mutation is not
yet claimed.

## Mutation-resistant validation

Synthetic tests preserve raw float words through inertia, inverse-body matrix,
material, and root Volume fields, including signed zero and signed NaNs. They
also prove that the retained `Arc` survives loss of the caller's owner, that
dictionary discovery is ordered, that the 26-Part capacity is independent of
the later 26-volume emission cap, and that count, nonlocal-section, and
wrong-type mutations produce distinct refusals. Additional topology tests
cover disabled records (Create still selects them), top-level and child mesh
wrappers, ordinary children, ordinary grandchildren, skipped deeper
aggregates, and traversal after the 26th emission. A discriminator additionally
proves that Update omits a top-level mesh wrapper while retaining the same
child mesh and one-level SimpleMappedArray cases. Both walks continue far
enough after saturation to report a malformed later aggregate target.
The live-record test preserves signed zero and both NaN signs in copied
Material words, covers the attached inverse-body transform path, toggles every
Update-owned field, and proves every Create-only field remains bit-identical.
Wrong flag count and a byte-identical but separately owned template both fail
before mutation. A separate `Part::GetTransform` test proves the two bitwise
copy branches and noncommuting composed path.

The optimized reference-data gate rebuilt all 125 dedicated DMO arenas through
the checked resident loader. It resolved 93 templates with 93 Parts and 679
Create-selected Volume records. The latter decomposes consistently with the
independent corpus inventory into 586 primitive volumes plus 93 clustered-mesh
wrappers. A nested,
order-sensitive FNV-1a fingerprint covers physical arena order, Assembly and
Definition IDs/header words, every Part payload, and every resolved root
and selected Volume word. The frozen V2 corpus signature is
`2de19d3348a1226b`. The independent
parser payload signature remains `6e38d215339bf897`; both passed together.

The same optimized gate now also walks Update independently. It observes 679
Update-selected occurrences and zero positional differences from Create in
the supplied 93-Assembly DMO corpus, so the real bank does not exercise the
top-level-wrapper divergence even though the recovered function does. The
separate frozen V1 Update-selection signature is `eac3e52c13270389`.

## Staged CPU Create/Update publication

The private staged owner now reconstructs CPU Create at
`0x82e0c850..0x82e0cf2c` and Update at `0x82e0cf30..0x82e0d35c` without
publishing partial host state. Create retains the arena owner, definition and
Assembly identities, exact Params bits, `VolumeData`, the Part-order body
occurrence array (including repetitions), and the Header semantic values.
Update requires the same owner/template/body occurrence sequence and packed
address-domain authority, then rewrites only the native Update field set.

Every Part's portable semantic fields are occurrence-validated and sampled
once as an explicit atomic host policy. Packed topology and Add-outcome inputs
are revalidated for each requested transform or motion callback under the same
immutable pool borrow. Packed transform words are requested only for Parts that have emitted
Create/Update records; packed motion-link words are requested only for Parts
with bbox-producing primitive records. Aggregate wrappers therefore consume a
transform but perform no bbox numeric work. Parts with no admitted record do
not require either packed-word callback. Native can expose partial stores and
observe concurrent mutation between repeated reads; those racy intermediate
states are not reproduced.

The five pointer-adjacent fourth words are never synthesized as zero or from a
host pointer. Zero is nevertheless the exact native `mInertia` word for
Static and is also a permitted generic non-Static Add argument. The
Definition-backed Part/DMO route supplies its exact Add-recorded arena
address. A provider supplies virtual-Xenon `mStasis`, `mInertia`, solver-row,
right-link, and left-link word images under a retained authority identity.
`mTag`, `mKine`, and `mCool` come from the occurrence-checked body snapshot.
The attached/inverse-body transform computes and stores all four lanes of each
column. Its ordinary-finite operation association follows the recovered VMX
graph; exceptional Xenon VMX/FPSCR behavior remains a hardware boundary.

## Header and BitTable contract

The PDB fixes the 0x40-byte Header: `numVolumes/+0x00`,
`numRigidBodies/+0x04`, `numStatic/+0x08`, `group/+0x0c`, culling pointer and
byte size at `+0x10/+0x14`, `allPartsFrozen/+0x18`, Assembly pointer at `+0x1c`,
and the 0x20-byte AABB at `+0x20`. Create writes every semantic field. Update
rewrites only `numStatic`, group, culling pointer/size, and all-frozen; counts,
Assembly identity, and AABB remain Create snapshots.

The all-frozen reduction starts at one and ANDs body-state bit one in Part
order, so an empty Assembly remains true. Culling bytes are zero for a null
table; otherwise the exact wrapping-u32 graph is
`4 * ((((columns * rows) + 31) >> 5) + 3)`. This includes the three-word
BitTable header preceding its packed words. `Params::Params` at
`0x82e0b680..0x82e0b690` stores the exact threshold bits `0x3f7fbe77`.

Create computes the Header AABB through the concrete primitive and swept
graphs below. It skips Aggregate records, copies the first primitive box,
and applies ordered accumulator/current min/max to later producers. Records
beyond the 26-record emission cap never participate. A zero-producer CPU
Create is rejected because native would publish indeterminate stack lanes; it
is not normalized to the SPU/Union zero result. Update leaves this AABB
unchanged.

Only the Aggregate bypass is proven in native Create. A selected Null would
enter the non-Aggregate vtable path without a deterministic callable target,
so the staged host refuses Null before callbacks as an explicit malformed-
input policy.

PDB layout and `rw::BitTable::Initialize` at `0x82bc3308..0x82bc336c`
establish rows `+0`, columns `+4`, stored `arraySize` `+8`, and packed words at
`+0xc`; the registered type is `0x0001000f`. The parser retains the stored
word count even when it disagrees with dimensions. Native Create/Update borrow
the shared allocation through a const pointer and derive Header bytes from
rows/columns only; mutations by another owner/alias would remain visible. They
do not read `arraySize`, validate ownership, or copy/refcount the table.

Rust retains an owner-qualified, `ObjectId`-keyed immutable parsed snapshot in
an `Arc`; later native alias mutation is not modeled. Before publication it
rejects foreign owners and a stored allocation smaller than the wrapping
derived word count. These are explicit safe-host policies. Oversized stored
tables remain accepted, null maps to zero bytes, and a non-null zero-dimension
table retains identity and produces 12 bytes.

Local object/type resolution, Part ordering, count agreement, fixed capacity,
owner lifetime, Header field ownership, and the observed DMO topology are
`exact` for the supplied reference build and recovered boundary. This remains
a crate-private semantic owner, not a native-layout object or production live
DMO session. A concrete five-word virtual-Xenon provider is available only
through inseparable Create/Update wrappers. The actual session/body-pool
lifetime bridge, allocator leases and base provenance, multi-arena routing,
live mutable Volume aliasing before Create, and exceptional numeric
differentials remain open. Joint/Drive arrays are outside this milestone.

## Swept Part AABB value graph

The namespaced helper is now recovered directly rather than replaced by a
generic transformed shape box. `EA::Physics::CalculatePartBBox` spans
`0x82e0b760..0x82e0b96c`. It calls the concrete Volume vtable's `GetBBox`
slot with `(volume, transform, 1, temporaryBBox)`, ignores that method's
return, and unconditionally consumes a non-null `RigidBody`. Four 16-byte
loads preserve both the three-float PDB member and its adjacent fourth word:
velocity/`mRight` at `+0x20`, omega/`mLeft` at `+0x30`, force/`mKine` at
`+0x90`, and torque/`mCool` at `+0xa0`.

The exact post-shape instruction graph uses timestep bits `0x3c888889`. It
forms the maximum absolute pairwise difference between the shape's X/Y/Z
extents, predicts angular motion from omega and torque with a clamped dot
product, evaluates `vrsqrtefp128` even for exact zero, performs two fused
Newton refinements, and caps the resulting angular magnitude at one. That
amount expands both shape bounds. A second clamped dot product predicts the
one-step linear displacement from velocity and force, after which ordered
VMX min/max operations union the box with its translated copy. Finally the
helper evaluates `center=(high+low)*0.5`, `radius=high-center`, and
`center +/- radius*bits(0x3f866666)` without reassociation.

The crate-private Rust stage takes the concrete shape bounds as an explicit
input and reconstructs this complete post-`GetBBox` graph. Its numeric
interface exposes all native `vmsum3fp128`, `vrsqrtefp128`, `vminfp128`, and
`vmaxfp128` sites in source order, so an eventual Xenon differential backend
can supply estimate, reduction, NaN, signed-zero, and denormal semantics
without changing the typed graph. Individual add/multiply/FMA operations are
currently an ordinary-finite host transcription; exceptional arithmetic and
the semantic meaning of the adjacent fourth words remain explicitly open.

`EA::Physics::CalculatePartBBoxUnion` at `0x82e0b970..0x82e0b9fc` first
writes positive-zero min/max vectors, scans one validity byte per 0x20-byte
box, accepts any nonzero byte, copies the first valid box bit-for-bit, and
then performs ordered accumulator/current min/max operations. Count zero or
all-invalid input therefore returns exact zero. The typed helper preserves
that disjoint-input behavior; native output/list pointer aliasing remains
outside the safe slice contract.

CPU `SerialisedAssembly::Create` does not call Union. It copies the first
bbox-producing record directly, reduces later ordinary records in emission
order, and excludes every emitted Aggregate/ClusteredMesh wrapper. Every
emission gate precedes bbox work, so records omitted after the 26-volume cap
also cannot affect the header AABB. If no ordinary record is emitted, the CPU
path stores uninitialized stack lanes; this has no deterministic zero oracle.
By contrast, `CreateOnSpu` and `AssemblyPoolUpdateJob` call Union and therefore
produce zero for an all-invalid list. The host must reject the CPU zero-valid
case or label a policy rather than merging these distinct behaviors. Update
never changes the Header AABB, and the pool job writes its recomputation to a
separate external bbox array.

Focused tests prove the exact constants, three reduction calls, estimate-on-
zero chronology, four-lane motion forwarding, clamp operand order, exact first
valid copy, nonzero validity-byte handling, all-invalid zero, and ordered
later union. The concrete primitive dispatch is connected to the staged Header
owner. A Xenon numeric oracle remains necessary for exceptional-value
differentials; ordinary-finite association is already retained.

New evidence artifacts:

- complete normalized proof packet SHA-256:
  `BDC953909939DDC1411634B5FA2C1BD4D31D31D3BD7AB77F63742B09EF5089DC`
- raw helper extraction SHA-256:
  `4BB0F32B13512F9E147DF9E120615B18BB680BEEA028D8E19ADAC501C84F5A90`
- normalized VX128 decode SHA-256:
  `930E86139F96A19C7FEB1BC1ECDC75D30F0264890B85ABCD8A6C46CA6C942453`
- independent broader dump SHA-256:
  `7A92A51E440D628302B032A437F19B109979BF35709E7F13EFA0DCAB0F44E523`
- reconstructed swept-stage Rust source at commit `d3f7d78` SHA-256:
  `B093893487264789CBCECB06E77BDEA8AF4061CD2287B537A17FABEED8704727`
- reproducible IDA dump script at commit `d3f7d78` SHA-256:
  `4F30C65BC2F3C9C57A74BAB0547B2C8D06345CA629591191C7959C9EF654FFDF`

## Concrete primitive Volume bounds

All five primitive vtable implementations reached by the assembly path are
now recovered from raw PPC/VX128 instructions: Sphere
`0x82acdd28..0x82acdd98`, Triangle `0x82acdfe8..0x82ace0c4`, Box
`0x82ad7e78..0x82ad7fa0`, Capsule `0x82ad8038..0x82ad80f0`, and Cylinder
`0x82ad8450..0x82ad87f8`. Each receives `this` in `r3`, an optional parent
matrix in `r4`, `tight` in `r5`, and the 32-byte result in `r6`; every path
returns integer one without validating dimensions or exceptional values.
Sphere, Triangle, Box, and Capsule ignore `tight`; any nonzero value selects
Cylinder's component-wise branch.

When the parent pointer is null, each implementation loads its required
stored vectors bit-for-bit. Otherwise it applies `parent * stored`: directions
start with a separately rounded X multiply and then Y/Z fused adds, while
points start at the parent translation and use X/Y/Z fused adds. Stored vector
W is never a coefficient, but all parent column W lanes participate and every
result vector publishes W. Sphere expands the stored center at `+0x30` by the
radius at `+0x50`. Triangle transforms the three vectors at `+0/+0x10/+0x20`,
reduces point one against point two before point zero with ordered VMX min/max,
then expands by `+0x50`. The owned host form excludes native overlapping
`this`/output behavior, including Triangle's preliminary store/reload.

Box clears the sign bit of all three transformed axes and evaluates the exact
extent schedule `abs(Y)*hy`, fused `abs(X)*hx + prior`, fused
`abs(Z)*hz + prior`, then a separate radius add. Dimensions are the raw words
at `+0x44/+0x48/+0x4c`. Capsule reads only stored Z-axis and center, then uses
one fused `abs(axis)*hh + radius` with `hh` at `+0x44`. Neither path
canonicalizes W, dimensions, radius, signed zero, or NaN payloads.

Cylinder reads Z-axis, center, `hh` at `+0x44`, inner radius at `+0x48`, and
outer radius at `+0x50`. Loose mode separately rounds both radius sums, forms
one fused diagonal square, performs one estimate plus two fused Newton rounds,
and emits `[d,d,d,+0]`. Tight mode retains the native interleaved X/Y/Z
pipeline: it uses bitwise `abs(1-axis_component^2)`, performs three estimates
even for zero radicands, records each zero comparison before refinement but
selects only after both rounds, and fuses perpendicular-radius plus axial
extent. Its final W extent is the original transformed X-axis square, a native
quirk preserved rather than replaced by zero. There is no final sort, so
negative dimensions can deliberately produce inverted XYZ bounds.

The complete typed primitive-to-swept path now passes the non-null transform
and literal tight one used by `EA::Physics::CalculatePartBBox`, then feeds the
shape result into the already recovered motion expansion. The real DMO census
exercises all 586 bbox producers: 24 Sphere, 85 Capsule, 448 Box, and 29
Cylinder; it contains no TriangleVolume. Aggregate wrappers remain excluded
by the proven Create selection. Twenty-one focused tests freeze dispatch,
matrix/W conventions, fused-versus-split ULP cases, reduction and estimate
order, zero timing, Cylinder loose/tight behavior, negative dimensions, and
the combined shape-plus-sweep path.

Arithmetic source association and callback chronology are reconstructed.
Exact Xenon estimate seeds, min/max exceptional selection, denormals, NaN
payloads, signed-zero details outside bitwise sign clearing, and FPSCR effects
remain behind the numeric boundary rather than being falsely claimed from
host `f32` behavior.

Additional evidence artifacts:

- full primitive raw/pseudocode dump SHA-256:
  `FD8417F6CBF947A545ADEFA897F1BD81DA6ED21D7EAF5BEC606FF45CD26E603E`
- independently normalized Sphere/Triangle decode SHA-256:
  `C0A886029DC9FF0F1FFBCC734F7D403FCB032BE42F538BCC1AD60B7A7D608F46`
- Sphere/Triangle proof packet SHA-256:
  `9E7D64ADD4B198DC996B210F4AC1343016B0C3B36DA8192A5B8421A053EECE3B`
- complete primitive/swept Rust source SHA-256:
  `8C857FB60E40BF4C503BFEFA9D7EF58BB73EF6321C08083B7266EE00E943BE22`
- expanded reproducible IDA dump script SHA-256:
  `CA8C735A5F151676EC491190A89628C7B6A4D7BE65923D2E2E79813251DC619D`
