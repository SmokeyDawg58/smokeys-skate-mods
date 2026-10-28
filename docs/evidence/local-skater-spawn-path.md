# Local skater registration, spawn, and physical ownership

Date: 2026-09-07. Read-only recovery from the live matched debug IDB, with PDB
types, decompiled control flow, selected PPC checks, direct-call references,
and comparison to the existing Rust application. Confidence is `high` for the
route and dependency findings. The quoted type sizes/offsets, direct-call
addresses, and explicitly identified PPC constants are `exact` source facts.
This is a dependency proof, not an implemented or runtime-validated spawn.

Matched executable SHA-256:
`3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`.
Matched PDB SHA-256:
`0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`.
These identities are the existing matched-image baseline; this audit used the
live database and did not re-hash the external files.

## Critical separation

`PlayerLoaderImpl::OnSimCharacterLoaded` does not construct an Actor. Its
registration request belongs to the character/presentation recipe pipeline.
`SkaterManager::SpawnPlayer` is reached separately through player command 3.
The existing app's `probe_default_player_load` loads real assets but directly
supplies synthetic character ID `0xFFFF000000000001` to the protocol callbacks.
That probe establishes neither actual presentation registration nor spawn.

The mounted world and character data already exist as owned Rust structures.
`load_runtime_collision_scene` in `apps/skate2-pc/src/main.rs` assembles the
selected BAM collision collections; `robs_park_resident_probe.rs` additionally
drives resident collection loading and explicit teardown. The default-player
probe loads the real `tutorial_male` recipe and its 42 distinct RX2 arenas,
and exercises skinning. These owners are useful implementation inputs but do
not supply the missing Actor, physical player, or persistent game loop.

## Registration route

| Address | Symbol | Source behavior |
|---|---|---|
| `0x82817F28` | `Character::PlayerLoaderImpl::OnSimCharacterLoaded` | stores the character ID, applies player attributes, requests registration |
| `0x828158F0` | `Character::PlayerLoaderImpl::ApplyPlayerAttributes` | applies selected player character/stance/hat data; CAS has additional settings; may reload an already-existing local Actor |
| `0x82817A08` | `Character::SimCharacterManager::Register` | reserves a request slot and sends a presentation registration message |
| `0x8281BAB8` | `Character::PresCharacterManager::ReceiveSimRegisterCharacterRequest` | finds the managed loader by character ID, requests recipe registration, queues its completion check |
| `0x82CFD418` | `Character::CharacterRecipeLoader::Register` | requires a loaded valid asset, records peer identity and registration state, calls the asset notification |
| `0x828173C0` | `Character::CAssetNotification::NotifyRegisterSchema` | sends the recipe/peer schema-registration notification |
| `0x8277C318` | `Sk8::PresThreadMsgMan::ReceiveRegSchemaMsg` | calls `SkaterPresentation::RegRecipe` |
| `0x827E6198` | `Sk8::SkaterPresentation::RegRecipe` | removes the prior skater association and registers the recipe mapping |
| `0x827E4DA8` | `Sk8::detail::SkaterRecipeMap::AddRecipe` | resolves the real CharacterAssetManager recipe and installs the peer-to-recipe mapping |
| `0x82818D50` | `Character::PresCharacterManager::Update` | waits for the presentation entity/peer match before emitting Registration status |
| `0x82816C10` | `Character::SimCharacterManager::ReceiveLoadStatusMessage` | locates the request and dispatches status-specific completion |
| `0x82815528` | `Character::SimCharacterRequest::OnRegistered` | registration callback dispatch |
| `0x82818098` | `Character::PlayerLoaderImpl::OnSimCharacterRegistered` | completes the matching request; it creates no Actor |

Representative decorated symbols are:

- `?OnSimCharacterLoaded@PlayerLoaderImpl@Character@@UAAXIABVcAssetID@assman@@@Z`
- `?Register@SimCharacterManager@Character@@QAAI_JVcAssetID@assman@@_N2AAVISimCharacterLoaderCallback@2@@Z`
- `?ReceiveSimRegisterCharacterRequest@PresCharacterManager@Character@@AAAXABVcMsgPresCharacterRegister@@@Z`
- `?Register@CharacterRecipeLoader@Character@@QAAX_J@Z`

The 64-bit arguments are frequently misrepresented by the decompiler. PPC
`0x82817FEC..0x82818010` establishes the actual registration call inputs:
manager in r3, peer ID loaded from loader `+0x40` into r4, character ID loaded
from `+0x08` into r5, ghost-enabled byte in r6, `!isNetworkGame` in r7, and
the callback owner in r8. The result replaces loader `+0x48`. Do not copy the
decompiler's overlapping `__int128` values or incorrect argument annotations.

`OnSimCharacterLoaded` applies the normal player index first. When game mode
is not PartyPlay and the ghost system is enabled, it also applies index 8
before registering. The CAS and ghost paths have not been implemented by
this audit or declared dispensable to native behavior.

`SimCharacterManager::Register` selects the first available pending request,
sets request type Register/status Registration, then issues `mNextRequestID`.
The increment skips `0xFFFFFFFF` by wrapping to zero. Exhaustion asserts.
Native request allocation, thread queues, and completion timing remain open.

The presentation Register branch at `0x82818F2C..0x82818F94` obtains a
`SkaterPresEntity` from the loader's asset and waits for its peer identity to
match `mRegisteredId`. Only then does it send Registration status and clear
the pending type/request. Installing a recipe map entry alone is not this
completion condition. The full scheduling relationship to presentation
entity creation still requires recovery.

| PDB type/ordinal | Size | Relevant fields |
|---|---|---|
| `cMsgPresCharacterRegister`, 20644 | `0x30` | request `+10`, character ID `+18`, ghost byte `+20`, apply-attributes byte `+21`, peer ID `+28` |
| `Character::CharacterRecipeLoader`, 23376 | `0x40` | manager `+4`, notification `+c`, state `+10`, asset `+18`, recipe `+20`, registered peer `+28`, registered byte `+30` |
| `Character::ManagedLoader`, 26448 | `0x18` | type `+0`, request `+4`, category `+8`, loader `+c`, ghost/attribute/morph/retain/pending flags `+10..15` |
| `Sk8::detail::SkaterRecipeMap`, 26182 | `0x174` | fixed map of signed 64-bit peer identity to `CharacterEntity*`, inline capacity 9 with overflow enabled |

## Independent spawn command

`PlayerUI::ReceiveSpawnMsg` at `0x828AC8B0` requires the spawner to be ready,
gets the transform for the message location key and player index, then emits
receiver 1 / command 3 with `PlayerSpawnData`. `PlayerUI::ResetPlayer` at
`0x828AD280` emits the same command, using the previous transform when its
location key is invalid. Reset also has attribute/session-marker work.

`SkaterManager::OnCommand` at `0x82600920`, command-3 branch
`0x82600984..0x826009BC`, performs:

1. decode player index and transform;
2. `AdjustSpawnPosition` (`0x825F1A20`);
3. destroy the prior player for that index;
4. `SpawnPlayer` (`0x82600630`);
5. `UpdateLocalPlayer` (`0x825FFEF8`).

`?SpawnPlayer@SkaterManager@Sk8@@AAAXHABVMatrix44Affine@vpu@math@rw@@@Z`
allocates an Actor of `0x260` bytes, constructs it with the requested transform,
player index, simulation flag and PhysicsWorld, appends the Actor to both the
skater and Entity-manager lists, sorts the skater list by player index, and
calls `InitPlayer` at `0x826000B0`.

`InitPlayer` selects the generated-graph settings, loads ActionGraph then
MotionGraph through their controllers (`0x826002A4`, `0x82600350`), notifies
skater observers, and updates the local-player association. This is the
onboard actor path; the pedestrian test graph cannot stand in for it.

PDB `Sk8::PlayerSpawnData`, ordinal 28386, is `0x50` bytes: signed player
index at `+0`, affine matrix at `+10`. Native padding is not a host ABI goal.

Spawn adjustment preserves orientation and can replace translation with a
world-line hit plus a vertical offset. Its observed constants are start
offset `0.75`, downward extent `500.0`, and post-hit offset `0.15`. The
network/debug-enable gate and VMX arithmetic still require a dedicated
numeric proof before implementing this adjustment as parity.

## Actor and physical dependencies

`??0Actor@Sk8@@...`, constructor `0x825FE850`, owns the Action input listener,
ActionGraph, SkaterMotionGraph, SkaterAnim, physical player, animation-to-
physics buffer, breadcrumbs, external/AI inputs, and animation jobs. It
requires the appropriate playback databases/managers, player attributes,
PhysicsWorld, and Simulation query services. It asserts player index <= 8.

PDB `Sk8::Actor`, ordinal 18608, is `0x260` bytes. Its key fields are player
index `+1d8`, simulation flags `+1dc`, input listener `+1f0`, ActionGraph
`+1f4`, MotionGraph `+1f8`, SkaterAnim `+1fc`, IPhysical `+200`, handoff
buffer `+204`, and SubmittableSkater `+208`. The contained Entity begins at
zero; these are evidence offsets, not proposed Rust storage.

For physical initialization Actor obtains its animation hierarchy, asks for
`phys_Tpose` through `Andale::PlaybackData::GetPhysicsParam` (`0x82D7CB30`),
and gathers the bind pose. It passes those with hat/bouncy/human state and
world/query services to `PhysicalPlayerHiLOD::PhysicalPlayerHiLOD`
(`0x82E03C50`, call `0x825FF2F4`).

The existing `wipeout-offboard.md` proves identical 2,752-byte type-5 records
in `SkaterBodyPres.abin` at `0x950`, `OffBoard.abin` at `0x8b80`, and
`OnBoard.abin` at `0x2e00`, SHA-256
`136D6DCAF907C96F51E6E5184F5C7CA46357EF1EF21CE0ADF755162AF7ED0EF5`.
`Skeleton::Init` at `0x82B62710` requires exactly 24 physics bones. The
selection of all possible additional playback overlays is still unproven.

PDB `Init_IPhysical`, ordinal 33857, is `0x30` bytes:

| Offset | Semantic input |
|---|---|
| `+0` | animation hierarchy pointer |
| `+4` | PhysicsParam pointer |
| `+8` | 8-byte affine pose-buffer owner |
| `+10` | bouncy-ragdoll flag |
| `+14` | grab-spline query manager |
| `+18` | network-game query |
| `+1c` | physical-world services |
| `+20` | living-world manager |
| `+24` | actor query |
| `+28` | physics world |
| `+2c` | human flag |

HiLOD is `0x340` bytes, PDB ordinal 33851. Its constructor creates the
processed-input state, physical output, conditioners, surface/reckoning/
trajectory/pumping services, Skateboard, Skeleton and drives, state changer,
and state-specific owners. It initially assigns Sleeping then calls
`SetPhysicsState(PHYSICS_GROUND)` at `0x82E04D10`; merely setting an enum
would omit the transition's real effects. It writes wipeout invincibility
bits `0x3D23D70A` at `0x82E04D20..24`.

For a present network query reporting offline, PPC
`0x82E04D2C..0x82E04DA4` sets the world-only flag if needed, calls
`Skateboard::CollideWithWorldOnly`, then
`SkeletonController::StartCollideWithWorldOnly`. A missing/network query
takes the converse branch. These are precise source branches, not permission
to replace absent services with an arbitrary offline stub.

At `0x82E04DA8..0x82E04DBC` the constructor invokes
`IPhysicalWorldServices::Add(this)`. Its concrete `Simulation::Add` is
`0x828A7C98`. PPC adjusts interface `this` by -4, loads Simulation's
PhysicsWorld at `+c8`, then the world-owned physical list at `+2c`, and
appends the pointer. The decompiler incorrectly labels the adjusted field
`mInputMan` and the pointer as `LineBatch`; the decorated signature and PPC
establish `IPhysical*` registration.

## Next concrete physical effect

The useful next ownership boundary is an actual seven-part skateboard body
admitted to the native island, not another load-status adapter.

`Skateboard::Skateboard` at `0x82B38918` creates `SkateboardBody` at
`0x82B38270`, retains it across reset, and selects assembly part 6 as the
deck rigid body. Body construction calls `Init` at `0x82B374E8`, then:

1. `IIsland::AddSkaterBody(this, ACTIVE_BODY=4, true)` at `0x82B38808`;
2. `SyncCheckBody` at `0x82B38820`;
3. truck transform and drive construction, optional wheel drives, hook drive;
4. `SetTransform` at `0x82B388B4`;
5. spy-target/command registration.

Body Init creates an AssemblyDefinition with 7 parts, no initial drives,
and 0 or 6 joints. PPC `0x82B37564..0x82B375B4` obtains the joint switch
from `physicstrucks.default` key `0xB1CBE3E929AFC8A0`; a nonzero byte
selects six joints. It then invokes:

| Order | Function |
|---|---|
| 1 | `CreateWheels`, `0x82B344F0` |
| 2 | `CreateTrucks`, `0x82B33DA8` |
| 3 | `CreateTriangleDeck`, `0x82B32E08` |
| 4, when joint count > 0 | `CreateJoints`, `0x82B2D0E8` |
| 5 | `InitializeCullingTable`, `0x82B34D98` |
| 6 | `Assembly::Initialize`, `0x82AE1338` |
| 7 | `InitializeTransforms`, `0x82B2BC10` |

The assembly group and every part's userdata become 4. Wheel/truck/deck
geometry, mass, and material values depend on real global attribute
collections (`physicswheels`, `physicstrucks`, `physicsdeck`, and
`physics_wipeout`). Numeric helper reconstruction and mounted value checks
are still necessary; their names are not sufficient evidence to fabricate
shape or force behavior.

## Validation and remaining work

This audit inspected live function bodies, direct cross-references, PDB
layouts, and the cited PPC slices. It created no runtime implementation,
changed no database type, and ran no native spawn trace. Existing documents
provide the independent real-asset checks for the world, default recipe,
skinning, and PhysicsParam record; those checks were not rerun here.

Before accepting an implemented spawn, validate actual recipe/peer readiness,
owner creation and registration order, body/part/joint/drive counts and
initial transforms against the matched image and mounted attributes. Then
capture initial and repeated fixed-step state/contact traces. Native input,
onboard Action/Motion behavior, physical control, presentation submission,
and the persistent application frame remain separate required integrations.
