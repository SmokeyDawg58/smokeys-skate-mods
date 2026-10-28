# Retained processed surface selection

2026-09-08. Bounded read-only debug XEX/PDB and original
Vault audit. No implementation/Cargo. This refines the earlier
[surface packet](surface-physics.md) for actual grounded force integration.

## Actual producer chain

The selected surface is **previous published PhysOut collision state**, not
the current IK contact line or below-deck line's packed surface ID:

`Island body contacts -> SkateboardBody::UpdatePostPhysics -> wheel vote ->
Skateboard::FillPhysOut Collision.physicsSurfaceType -> next HiLOD
Update1_ProcessInput -> SurfacePhysics::Update/Dispatch -> ProcessedPhysIn+904`.

HiLOD `82e01f98` stores timeOnGround at `82e02260`, loads retained
SurfacePhysics owner `this+308` at `02264`, calls Update at `02268`.
Update `82dda0b0..a0c0` loads SurfacePhysics.mPhysOut+0, PhysOut.mCollision+18,
Collision.mPhysicsSurfaceType+10, then tailcalls Dispatch `82dd63e8`.
This does not perform a query, decode a packed ID or fetch Vault data.

SurfacePhysics owns PhysOut+0, mutable ProcessedPhysIn+4, six generated
physics_surfaces instances at +8(default), +18(smooth), +28(rough), +38(slow),
+48(slippery), +58(veryslow), and wheel materials +68/+74/+80/+8c/+98.
These are retained owners, not global physics_surfaces singleton pointers.

## Contact surface and wheel vote provenance

SkateboardBody::UpdatePostPhysics `82b36a58` calls actual island virtual
GetCollisions at `36ad0` with this body. Island::GetCollisions `827ba6b0`
locks its contact-map mutex, selects `mRigidBodyContacts[body]` (`a6e4`),
unlocks and returns the actual retained vector. It does not run a ground line.
ContactInfo is stride60: normal+10, position+20, relativeVelocity+30,
part tag+40, packed surfaceID+44, bodyThis+48/bodyOther+4c, flags+50/+51.

The body clears seven inContact bytes before traversing current contacts.
Admission precedes the selection described below: if pBodyOther is nonnull,
`36c9c/36ca0` load other/this Body.mpOwner (+20, PDB Entity pointer).
Equality branches at `36cac` to `36eb4`, skipping the per-part winner and
inContact updates. Thus same Entity-owner contacts (including the player's
own skeleton) do not participate. A null other body proceeds. The highest-Y
rule applies only to admitted contacts, not every raw contact in the vector.
At `36be8/bec` it loads actual part tag and surfaceID. At `36d98..36dcc`,
first contact for a part wins; subsequent contact replaces only when its
normal.Y is ordered strictly greater than the retained normal.Y. Equal or
unordered later Y keeps the earlier selected contact. Raw VMX comparison's
VA is v43 (the current Y splat), not IDA's printed v75. Selected normal is
stored at `36dd4`, position `36e00`, relative velocity `36e08`, pattern
`36e10`, physics type `(surfaceID>>7)&31` at `36e14`, audio `surfaceID&127`
at `36e18`; inContact is set later at `36e68`. The collision-info base is
body+40; its per-part physics types+2b4, inContact+30c, overall type+308.
The earlier contact-spy-to-ContactInfo publisher remains a separate retained
contact producer boundary; this packet does not invent one from line hits.

Body calls ChoosePhysicsSurfaceType at `82b3709c`. Leaf `82b24878..24a00`
zeroes twelve vote counters, examines **only wheel parts 0..3**, in order,
and only reads/increments that wheel's type when inContact is true. Native
indexes its local count array directly with the type; a checked host must
refuse out-of-range indices rather than silently smooth them before voting.
It scans counts for types1..9 in ascending order, replacing winner only on
strictly greater count. Initial winner1/count0 means no wheel contact yields
smooth, and equal counts favor the lower numeric type. Types0/10/11 receive
counts but are not candidate winners. Trucks/deck do not vote.

After the vote, it loads `kForcedWorldSurface` ref8308d858 target at `249e0`,
value target+d0 at `249e4`. Nonzero overrides winner (`249f0`); zero stores
winner (`249f8`), both to body+348. Initializer82f7b7a0 passes default0 at
`b7cc`, constructor `b7d0`, section Physics_Surface. This is a late runtime
cvar read, not a constant preflight. Skateboard::FillPhysOut `82b2f3e8`
later writes body collision overall type to PhysOut Collision+10 at `2fc20`,
after the source PhysOut write-lock assertion. This publication is what the
next Update1 SurfacePhysics reads.

## Dispatch writes and collection provenance

Dispatch first writes incoming enum to processed+900 at `63f0`, reloads the
processed owner and enum, then switches on enum-1 with unsigned range check.
Types1..5 select corresponding retained collection and material. Type6
(unrideable) rewrites processed type to3 and dispatches again. All remaining
values rewrite processed type to1 and dispatch again. The result is therefore
normalized processed type, while original PhysOut collision classification
remains available for its separate semantic consequences.

For smooth, collection pointer store is `6458` to processed+904, then
processed owner is reloaded before wheel-material pointer store `6460` to+908.
Other pairs preserve this order at `6474/647c`, `6490/6498`, `64ac/64b4`,
`64c8/64d0`. No selected attribute value is read by Dispatch. A host should
not preflight BrakingScalar and erase this publication prefix on later failure.

Constructor `82dd6060` constructs default first, then the five surface owners,
with these raw r4 collection keys and msgPort0:

| Role/type | Collection key | Constructor call | Own member |
|---|---|---|---|
| default, never selected by Dispatch | `d7edbd362d7d2152` | `6094` | +8 |
| smooth1 | `8e61e92e89225fbe` | `60b4` | +18 |
| rough2 | `a28a048ada290408` | `60d4` | +28 |
| slow3 (also incoming6) | `cb955af8908f2d65` | `60f4` | +38 |
| slippery4 | `960909a8602b4070` | `6114` | +48 |
| veryslow5 | `4ad0cd3497683aec` | `6134` | +58 |

Generated constructor `82dd5fe8` assembles class **f59fe715b7ae3426**,
FindCollection `82dd6018`, Instance constructor `82dd6028`. It then checks
Instance layout+4; null layout selects
DefaultDataArea(**10c**) at `603c` and stores it at `6040`. This generated
whole-layout default is distinct from per-key DefaultDataArea4. Do not fetch
BrakingScalar using an invented global scalar key.

Original inspect_vault.exe read external skater.vlt/bin: physics_surfaces
class export389256; default parent0, each named surface parent=default.
All six have zero explicit nodes and an explicit BIN layout of size10c.
PDB generated layout BrakingScalar+108 is the late read used by braking;
WheelStaticFriction+e0 and WheelDynamicFriction+e4 initialize materials.

Material constructor suffix first initializes all five triples to source
static .5/dynamic .3/restitution .4 (`6164..61b0`). It then performs five
separate global physicswheels+9c lookups of key **5d095e6365260b84**, index0,
DefaultDataArea4 if absent, each followed by its restitution read/store.
Lookup sites are `61ec/6248/62a4/6300/635c`; restitution stores are
`6204/6260/62bc/6318/6378`. After those stores, each smooth/rough/slow/
slippery/veryslow friction pair reads dynamic+e4 before static+e0, then
stores static before dynamic (`6380..63dc`). One pointer-stage detail:
the smooth layout pointer capture at6370 precedes fifth restitution scalar
read6374/store6378; its dynamic scalar read occurs6380. Retained immutable
layout ownership and host checked decoding must state that timing boundary.

| Collection | Original BIN layout offset | BrakingScalar raw +108 | Static +e0 | Dynamic +e4 |
|---|---|---|---|---|
| default | `a1b68` | `00000000` | `00000000` | `00000000` |
| smooth | `a1c74` | `3f800000` | `3f4ccccd` | `3f333333` |
| rough | `a1f98` | `3f99999a` | `3f59999a` | `3f400000` |
| slow | `a1e8c` | `40000000` | `3f666666` | `3f4ccccd` |
| slippery | `a20a4` | `3f000000` | `3f333333` | `3f000000` |
| veryslow | `a1d80` | `40000000` | `3f800000` | `3f7d70a4` |

Independent direct BIN byte reads verified all words after existing inspector
resolved pointers. These are corpus assertions only. Original smooth1 may
legitimately arise from the source **empty wheel vote**, but a corpus must
establish the actual contact owner/cycle; selecting smooth from a conveniently
nearby terrain line is not the same producer.

## Adjacent six-force input dependencies

Update1 copies prior PhysOut_SkateboardMotion velocity+50 to processed+190 at
`82e02638/63c`, surfaceVelocity+60 to processed+1a0 at `02674/678`,
zSpeed+a8 to processed+944 at `026dc/6e0`, surfaceSpeed+a4 to processed+96c
at `02710/714`. These reads occur after SurfacePhysics::Update and before
PrepareToolkitInput `82e036cc`. They are not new samples from the board body.
Prior Skateboard::FillPhysOut writes deck body velocity to motion+50 at
`82b2f788`, computes surface velocity/speed in its own later sequence, and
zSpeed from effective deck Z dot published velocity at `82b3016c..301a0`.

PrepareToolkitInput `82b26910` reads actual board mass first, stores+970 at
`2692c`, then deck/effective transforms. Body X/Y/Z fields+140/+150/+160
come from physical deck bases, with source flip treatment on X/Z. Its
bodyZDirection+170 is flattened Y and source-normalized (`26b20..26ba0`);
bodyYDirection+180 is orthogonalized/normalized (`26bb4..26c3c`); absBodySpeed
is source sign multiply of retained zSpeed at `26c68..26c9c`. Existing bounded
mass/deck/effective production does not imply these remaining fields or
PhysOut publication already execute. See brake/drag and foot packets for
consumer read timing. Full numerical producer audit/implementation remains
separate from this selected-surface proof.

## Fingerprints and integration limit

| Original span | SHA256 |
|---|---|
| `82dd6060..82dd63e8` | `0b9c7e6f68a3c7b908a388275281f112f6e1b1a5b82e942256e7df31126ffac8` |
| `82dd5fe8..82dd6060` | `c0325f7a4961697a9e59c075def7273795a01f2eba6599d7ebb5507e314341dd` |
| `82dd63e8..82dd64d8` | `991b315cfbb3b9e52825159aa61e4dcd9738f20971e16a1a34b312b79cb810a2` |
| `82dda0b0..82dda0c0` | `9cb6bb782d74c817ae893e5cc70a6c5f9b34b7b8ab8067803916e8a153aed9c5` |
| `82b24878..82b24a00` | `d3d74df2a015543067cc404543fe4cc8e6a8ac5be7d408f2a129a2237127f4fa` |

Confidence exact for these dispatch/vote/raw binding sites and PDB layout;
high for original layout values via pointer parser plus independent byte reads.
Recommended next owner: retained five selected generated Vault instances with
explicit layout-default semantics; selected token derives from published
collision enum and normalization, retains collection authority, and defers
BrakingScalar until the existing brake service asks after zSpeed. Missing
actual contact/PhysOut scheduling must stay explicit. No live frame trace or
complete six-force runtime claim follows from this read-only packet.

## Assigned retained owner implementation

Root subsequently assigned `skateboard_processed_surface.rs/tests`. The owner
binds the six source collection instances once through the existing shared
slide/collision provider; selected clones retain that same Vault instance.
Its one-shot material constructor retains partial triples on failure and
rejects replay after either success or failure. The provider's opaque captured
layout token preserves `6370` before fifth restitution read/store, without
preflighting layout bytes; dynamic/static decode remains deferred to the
source scalar-read stages. Later surface layouts are captured only after
the preceding material's two friction stores.

Dispatch publishes incoming type, normalized type when needed, selected
collection, then actual AssemblyPartMaterial. If material construction has
not completed, it explicitly returns MaterialsNotReady after selected-token
publication. It does not publish fabricated material identities or coefficients.
The concrete projection starts with Option readiness. Four focused tests
cover the constructor stage/failure graph and one-shot lifetime, dispatch
publication prefixes, and distinct retained source collections/foreign token
rejection. A helper permits root's existing original corpus to check selected
BrakingScalar values. Root owns registration, Cargo and actual grounded owner
integration; no author Cargo run is asserted. Full contact/PhysOut producers
remain separate from this constructor and dispatch projection.
