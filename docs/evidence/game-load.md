# Game and world load sequence

Confidence is `exact` for the supplied debug build's compiled world seed,
OneOff location registration/selection, outer game-stream pump, and
initial-focus branch. Scheduling outside the recovered OneOff worker remains
`high`: no reference runtime trace yet establishes OS-thread interleavings or
wall-clock timing.

The reference inputs for this slice are the debug XEX (SHA-256
`3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`),
PDB (`0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`),
linker map (`51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`),
and supplied `worldbam.big`
(`D304BD10C08375AB09EDBCAEB0897897A8347ED7B310CF2D2EAABDE5C096FE66`).

## Entry into the game module

- `FrontEndManager::StartGameModule` — `0x82633da0`
  - removes the front-end letterbox when required;
  - enables progression only for a single-player session;
  - creates and sends `MESSAGE_EnterGame` to the flow manager.
- `GameModule::Load` — `0x8273a420`
  - mounts the `Load` BIG group;
  - resets the game-load state;
  - constructs pause, database, simulation, presentation, and menu systems.

## One-off load

`GameModule::OneOffLoad` at `0x8273add8` performs process-lifetime world setup:

1. Mount speech files and select offline audio behavior.
2. Load the ChallengeCore/progression-core vaults when applicable.
3. Mount `Game`, `Characters`, and `CreateACharacter` BIG groups.
4. Disable deferred collection loads.
5. Initialize `Simulation` with the asset and location managers.
6. Load one-off textures.
7. Register the non-streamed location `Z_TrainingStart_Fixed`; when
   `kbDebugFE` is false, select it unconditionally. Debug-FE mode preserves the
   location that was already selected.
8. Enter the audio world and reset `Physics::World`.
9. Optionally load and activate proxy world stream `PROXY` from
   `data\content\world\stream`.
10. Start loading `sGameInfo.worldFile` through `StartWorldStreamLoad` and pump
    until `UpdateGameStreamFileLoaded` succeeds. In the supplied debug build,
    the ordinary offline seed is exactly `BAM`.
11. Initialize the streamer's first focus/location.
12. Initialize LivingWorld recipes and pump until initial/required models load.
13. Wait for stream focus, create the presentation stream controller, and load
    AI skater assets.
14. Load front-end core/HUD components and their texture packages.

## Secondary main-DMO load gate

The main DMO bank is not a child of the BAM `tGameStream` record.
`GameModule::SecondaryLoad01` (`0x8272d480`) obtains the
`cDMOBankManager` owned by `Sk8ChallengeSystem`, calls
`LoadMainDMOBank` once at `0x8272d550`, and immediately calls
`IsMainDMOBankLoaded` at `0x8272d558`. When that result is false, its loop is
exactly:

1. `cDMOBankManager::Update` at `0x8272d570`;
2. `EA::Thread::ThreadSleep` with raw value one at `0x8272d574..0x8272d57c`;
3. `IsMainDMOBankLoaded` at `0x8272d584`;
4. branch back to the update while false at `0x8272d590`.

The sleep therefore still happens when the immediately preceding update made
the hull loaded. There is no initial sleep and no second load request.
The private subtitle-archive load and two diagnostic log writes surround this narrow
slice but do not change the DMO manager cadence.

`GameModule::GameLoad` calls `SecondaryLoad01` at `0x8273bf9c` only when its
secondary-load byte at `GameModule +0x5c` was zero; it stores one to that byte
first at `0x8273bf94`. After all secondary-load helpers and camera setup,
`GameLoad` independently checks the same DMO manager at `0x8273c074`. Its
false loop repeats Update, raw-one sleep, and recheck at
`0x8273c084..0x8273c0a8`, but never calls `LoadMainDMOBank`. In the ordinary
path the earlier blocking `SecondaryLoad01` means this later barrier performs
one successful check and no update. Keeping the barrier is still observable
for paths where the secondary-load byte was already set.

The BAM world-stream start/pump occurs later in `GameLoad`. Consequently the
truthful runtime composition is a `GameModule`-level sequence containing a
separately owned `cDMOBankManager` and game-stream manager, not a DMO lifecycle
nested inside [`WorldStreamLoads`](../../crates/skate2-core/src/session/game.rs).

Readable Rust now exposes `MainDmoBankLoadRuntime`,
`run_secondary_load_main_dmo_bank`, and `wait_for_main_dmo_bank`. The common
wait preserves the immediate check and update/sleep/recheck ordering; the
secondary helper alone issues the load request. Checked collaborator errors
are host policy because the three native manager methods provide no recoverable
error result. The mounted original-data probe now supplies an incremental
`MainDmoBankLoadRuntime`: one request enters resident loading, and each update
advances one deterministic resident, deferred-registration, callback, or hull
step. The recovered outer driver supplies raw value one after every such
pending update and observes completion at hull raw state 10. This proves the
owned composition and caller cadence against the corpus; its fine-grained host
subdivision is not evidence for native worker/postman interleaving. The probe
now constructs `DMO_Pres` only from `load_main_dmo_bank`; each later manager
update polls the mounted XSM/XST manifest once, and only `LOADED` permits the
single `cPres_Global` snapshot and resident enqueue. The application uses an
actual one-millisecond host sleep for raw value one. Host OS wake precision is
not a claim about Xbox scheduler timing, but no manifest I/O or resident work
occurs before the recovered one-shot request.

`run_game_load_main_dmo_barriers` owns the exact surrounding byte branch at the
narrow DMO-only seam. `run_game_load_through_main_dmo_barrier` now composes the
complete recovered secondary group and both unconditional state-graph calls
before that later barrier. It changes raw zero to one before the group, leaves
every nonzero byte unchanged while skipping the group, and always reaches the
later no-load wait. The mounted application supplies its complete DMO session
as the SecondaryLoad01 DMO slice; the ordinary corpus path therefore issues one
request and reaches raw state 10 before the later barrier performs one
successful loaded check with no update or sleep.

## Remaining secondary-load helpers

Fresh disassembly fixes the complete helper boundary around that DMO wait.
Before the `SecondaryLoad01` start log and main-bank request, the direct group
calls `Sk8::FE::SubtitleManager::LoadBigSubtitleFile` (`0x826aae08`) with
`data\fe\subtitles\subtitles.big` at `0x8272d4c0`. This synchronously loads the
complete nested BIG, parses it, and appends a wrapper to the subtitle manager's
private ordered list at `+0x20`. Duplicate loads append duplicate wrappers;
`SubtitleManager::Purge` deletes every wrapper and clears that private list.
It is not a global VFS registration. The shipped nested archive is 46,984 bytes
after its 12,557-byte outer entry expands and contains 288 named entries; the
mounted application exercises and checks that exact private ownership path.

`SecondaryLoad02` (`0x8272d5a8`) then performs this exact sequence:

1. obtain the resource allocator and call
   `cGlobalFileLocationManager::LoadGlobalLocations` (`0x82a8f3b0`);
2. immediately call inherited
   `cAssetActivationManager::IsPendingRequests` (`0x82d54870`);
3. while the returned low byte is **exactly one**, call
   `cAssetActivationManager::Update` (`0x82d5d8e0`), sleep with raw value one,
   and recheck;
4. invoke `Sk8::Audio::AudioSystem::SecondaryLoad` (`0x824fe178`).

The pending predicate is not a generic nonzero test: `clrlwi` followed by
`cmplwi 1` appears at both `0x8272d63c..0x8272d644` and
`0x8272d668..0x8272d670`. The sleep follows every raw-one update, including an
update that clears pending state before the recheck. The AudioSystem target is
also directly resolved rather than guessed: its constructor stores the
`IAudioInterface` vtable `0x82022ac0` at object offset zero, whose `+0x10` slot
contains `AudioSystem::SecondaryLoad`.

`LoadGlobalLocations` enumerates `data\content\global_locators` with pattern
`*.rx2`, hashes each complete path with case-sensitive `RwHash64String`, skips
an ID already in its manager map, queues `LoadArenaAsset`, and inserts
`{path, id, Loaded=false}`. Native activation owns separate queued and active
vectors, eight inline active records, forward admission, and same-ID active
suppression. The constructor's raw argument four is only initial queued-vector
capacity; eight is the actual active bound. `IsPendingRequests` returns one iff
either collection is nonempty. `Loaded` changes only in `AssetAdded`, after the
simulation registration completion callback advances the request to raw state
seven.

`SecondaryLoad03` (`0x8272d6b8`) calls `LivingWorld::RecipeMan::Init`
(`0x827289f8`). Any nonzero initialized byte at `RecipeMan +0x4` returns
immediately and preserves both that byte and frame-delay word `+0x8`. Raw zero
calls `InitRecipeMappings`, then `InitLoadingQueues`, then stores one and zero
to those fields. The operation is synchronous and consumes already loaded
`skater.vlt`/`skater.bin` attribute collections; it does not parse the later
version-seven `.recipe` payloads. The typed Rust owner preserves this exact
idempotence, call order, and terminal-store timing. A concrete vault backend is
still withheld because final queue order depends on native EASTL fixed-hash-map
bucket/chain traversal, which has not yet been reproduced.

SecondaryLoad04 has no substantive call in this build; it emits only start/end
diagnostics. `GameLoad` invokes helpers 01, 02, 03, and those diagnostics in
that order after storing one to `GameModule +0x5c`.

Readable Rust now exposes `SecondaryLoadGlobalLocationsRuntime` and
`run_secondary_load_global_locations`. Tests freeze immediate completion,
multiple Update/sleep/recheck iterations, the final AudioSystem call, and the
noncanonical raw-two case that exits because only raw one continues. These are
typed collaborator boundaries. A separate mounted corpus collaborator now
reads the 18 shipped named arenas in archive-table order and takes every one
through RW4 decode, Simulation projection, Location preparation, and live
Location registration. It proves 18 containers, 744 locations, 2,419
sublocations, 744 registered entries, and zero ordinary Simulation operations.
Archive-table order and `high_priority_raw=0` are explicit host probe policies,
not claims about multi-device directory enumeration or the live native cVar.
The asynchronous activation queue and AudioSystem's own secondary-load body
remain unwired rather than represented as completed backends.

After the optional group, both byte branches unconditionally call
`StateGraph::Cache::Load("script/camera/Default_cameragraph.xml", false)` and
then `LivingWorld::Manager::PreLoadStateGraphs` (`0x82724280`) before the later
DMO check. The first is a logical state-graph request: the cache may satisfy it
from a sibling compiled `.stategraph`, so it is not claimed as a direct XML
physical read. The LivingWorld call builds 18 state-graph paths, then ten
BehaviourGraph plugin descriptors and prop data. The complete call order is
frozen in core tests; real StateGraph/BehaviourGraph managers remain explicit
application backends.

The alternate thread entry `GameModule::backgroundSecondaryLoad`
(`0x8272d728`) reads the raw word at `GameModule +0x84` and accepts only four
sparse cases. It stores `6`, `9`, `12`, or `15` before respectively invoking
SecondaryLoad01, SecondaryLoad02, SecondaryLoad03, or the diagnostic-only
fourth phase from input states `5`, `8`, `11`, or `14`. Every other value
reaches the native assertion. `run_background_secondary_load` preserves these
store-before-call transitions, rejects other raw values without normalization,
and does not roll a stored transition back after a checked host error.

`StartWorldStreamLoad` at `0x8272d0c0` strips the selected world's extension,
takes its basename, combines an optional BIG prefix with
`data\content\world\stream`, and calls
`cGameAssetManager::LoadGameStreamFile(path, basename)`.

The string order is significant. It copies `worldFile`, truncates at the last
dot anywhere in that complete string, and only then searches for the last
backslash. Forward slash is not a separator. Thus `dir.name\BAM` produces
basename `dir`, while `foo/bar.xst` produces `foo/bar`. A non-null `bigFile`
uses exact format `%s|%s`; an empty prefix therefore produces
`|data\content\world\stream`. The reference uses unbounded operations on
256-byte C buffers. The native boundary accepts owned UTF-8 strings that fit a
255-byte payload and returns a validation error for interior NULs or overflow
risk; behavior for the reference's invalid/null/overlong inputs remains
`unknown` rather than being represented as parity.

`cGameAssetManager::LoadGameStreamFile` at `0x824dba00` allocates a `0x210`
byte `tGameStream`, assigns the next raw `u32` handle, and installs outer state
1. The constructor at `0x824dfb68` seeds the first handle to 1. The PDB places
`mNextGameStreamHandle` at `+0x203c0`, its fixed hash map at `+0x203c4`, and
the map mutex at `+0x20578`. The map has 16 inline nodes but uses
`fixed_pool_with_overflow`; exhausted inline storage allocates another node
from the heap at `0x824d6a98..0x824d6ab8`. A record contains atomic raw state at `+0x0`,
three `cStreamFile *` slots at `+0x4`, `mStreamPath[256]` at `+0x10`, and
`mFilename[256]` at `+0x110`.

The executable inserts the allocated pointer under the map lock before it
copies the strings and initializes state. That publication order permits a
race for unsupported concurrent callers. Native `WorldStreamLoads` explicitly
owns and fully initializes the record before insertion. This is a deliberate
safe ownership boundary, not a claim that the race itself was reproduced.
It preserves overflow beyond 16 records. Allocation failure and concurrent
start interleavings remain `unknown`.

Handle collision after raw-counter wrap is separately proven. Native
`DoInsertValue` keeps the existing map node, reports `inserted = false`, and
does not replace its pointer; `LoadGameStreamFile` ignores that result,
finishes initializing the now-unreachable allocation, returns the collided
handle, and has already advanced the counter. Native Rust deliberately rejects
that collision before advancing the counter, avoiding the leak while leaving
the old mapped record authoritative. This is an explicit safety deviation,
not parity or an unknown reference path.

### Outer game-stream state machine

`cGameAssetManager::UpdateGameStreamFileLoaded` at `0x824e1f48` locks only
long enough to resolve the handle, then advances at most one outer case per
call. The outer state field is `AtomicInt<unsigned int>` rather than the PDB's
separate `eGameStreamState`; the descriptive names below are native names, not
recovered source enumerators.

| Raw | One poll's exact action | Result/state |
| ---: | --- | --- |
| 0 | No work. | `Pending`, remains 0. |
| 1 | Begin presentation stream type 0 with stored path/basename. | Store slot 0, state 2, `Pending`. |
| 2 | Pump presentation stream file. | If loaded, begin its XML and set 3; always `Pending`. |
| 3 | Read presentation XML state. | Exact 2 begins simulation type 1 and sets 4; otherwise update presentation XML and remain 3. Always `Pending`. |
| 4 | Pump simulation stream file. | If loaded, begin its XML and set 5; always `Pending`. |
| 5 | Read simulation XML state. | Non-2 updates XML and remains 5. Exact 2 samples the live texture cVar: enabled begins type 2 and sets 6; disabled sets 7. Always `Pending`. |
| 6 | Pump texture stream file. | If loaded, set 7; always `Pending`. No texture XML is started. |
| 7 | Calculate the world bounding box. | Reset state to 0 and return `CompletedThisPoll`. |

Missing handles assert and then continue toward an invalid iterator
dereference in the reference; raw states above 7 assert and return false.
Native code represents both as invariant errors, never ordinary `Pending`.
Completion is a one-poll pulse, not sticky: polling the retained record again
observes state 0 and returns `Pending`. Texture-disabled loading still requires
the next state-7 poll to calculate bounds and complete.

The inner boundary is narrow and does not decode XMM or XSF. At `0x824e1ad0`,
`cAssetStreamSystem::UpdateStreamFileLoaded` first updates its `cStreamFile`
and returns true only when `cStreamFile::mState` at `+0x28` is exact PDB value
`eStreamFileState::kStreamFileState_LoadDone` (4). Raw 8 is not a named
`cStreamFile` failure state. `eGameStreamState::kGameStreamState_LoadFailed`
(8) belongs to the nested `cGameStream::mState` at `+0x24`; after a nested
XSM/XST failure, `cStreamFile` remains raw 1 because `IsStreamLoaded` accepts
only nested raw 4. The outer OneOff pump therefore remains pending and has no
separate failure branch. Arbitrary sampled `cStreamFile` values other than 4
also remain pending.
Presentation and simulation XML advance only on exact
`kStreamXmlState_LoadDone` (2); all other raw XML states call
`cStreamFile::Update` and retain their outer state.

The live gate is `kEnableTextureStream` at `0x8303dbec`. Its dynamic
initializer at `0x82f4e1b8` uses section `stream`, label
`kEnableTextureStream`, and compiled default true. The PDB places
`cVarBoolRef::mTarget` at `+0x20` and `tVarBool::mValue` at `+0xcd`. The outer
machine samples it only as state 5 leaves completed simulation XML; the native
default constant does not replace that live sample.

In the OneOff worker, `StartWorldStreamLoad` returns before the first poll and
the handle is stored in `GameModule::mGameStreamHandle` at `+0x54`. The first
poll is immediate. Each false result calls `EA::Thread::ThreadSleep` with raw
value 1, then retries; no initial sleep, timeout, cancellation, or explicit
failure exit exists. The true result proceeds immediately to
`InitStreamerLocation`. The native coordinator preserves this order through
separate store, retry-yield, and late initialization callbacks, so focus inputs
are not sampled during the earlier seed stage. It ends after that callback and
does not model the separate raw-value-one sleep at `0x8273b5a8..0x8273b5b0`
or the subsequent `OneOffLoad` work.

`InitStreamerLocation` at `0x8272e888` selects the initial focus. Its network
branch calls `Sk8::Net::GetStreamerPosition(-1, -1)` and bypasses the offline
game-mode/profile predicate. In the offline branch, focus begins as a zero
vector. Unless the game mode is exactly `GM_SinglePlayer` (numeric 1) and the
four-byte integer `ProfileBackEnd::tProfileData::mNewProfile` is nonzero, the
function copies the stored game-info location position. Both branches then
verify that a named location never has a zero focus vector and call
`cGameAssetManager::InitStreamFocus(position, false)`. Only the offline branch
subsequently enables deferred collection loading. The single-player/nonzero-
new-profile conjunction retains the initialized zero focus and must not be
folded into the normal path.

On the offline path it also sets embedded `cAssetStreamSystem::mLoading` to 1
(`+0xea78` from the complete asset-manager object) and clears
`mEnableMemoryLimiting` at `+0x20399` before the focus call. After
`InitStreamFocus`, `EnableCollectionLoads` at `0x8272e058` first updates the
asset stream system, changes `mLoading` to 2, clears `mCollectionDefer` at
embedded offset `+0x11920`, and passes false to the underlying stream manager's
collection-defer control.

## Supplied debug build's offline seed

PDB type `Sk8::BE::CommonBackEnd::GameInfo` is `0x2b0` bytes. Relevant fields
are `worldFile` at `+0x14`, `locationName` at `+0x128`, `locationPosition` at
`+0x1b0`, the startup-position flag at `+0x1c0`, and its matrix at `+0x1d0`.
`GameInfo::GameInfo` (`0x82559418`) clears the strings, position, and startup
flag, initializes the startup matrix to identity, and does not choose a world
by itself.

`CommonBackEnd::CommonBackEnd` (`0x825623c8`) then seeds offline mode, copies
literal `BAM` into `worldFile`, and calls
`SetLocationName("Z_TrainingStart", false)`. The constructor stores exact raw
game mode `GM_NoProfileFreeSkate` (`0`) at `0x825623f0`.
`OneOffLoad` always adds a runtime alias named `Z_TrainingStart_Fixed`, then
samples `kbDebugFE`. When false it overwrites the current name with the fixed
alias; when true it performs no selection and preserves the current value.

The exact 16 runtime matrix words passed to `AddNonStreamedLocation` are:

```text
3f800000 00000000 00000000 00000000
00000000 3f800000 00000000 00000000
00000000 00000000 3f800000 00000000
c5256ddb 4301b88c c2cb1ef8 00000000
```

The translation is `[-2646.865967, 129.720886, -101.560486]`. The runtime
matrix's final lane is positive zero, not the XML homogeneous `1.0` stored for
the authored `Z_TrainingStart` entry. `BAM_Sim.loc` contains that authored
location at the identical translation, but no `Z_TrainingStart_Fixed`; the
fixed name exists only through the OneOff registration.

Career and free-skate transitions remain distinct from this process-lifetime
seed. New-profile non-demo single-player chooses the sole NewCareer entry,
`J_Gamestart`, at `[1491.814453, 109.960312, -743.305054]`. Free-skate first
selects index zero of its 27-entry data list, `Z_DT02`, at
`[155.614197, -35.458317, 429.690186]`, before any user choice. Existing-career
resume can use saved progression orientation or another selected location, so
there is no universal resume coordinate.

`A_RobsPark` is valid authored BAM data, but it is absent from the executable's
strings and all 75 `fe_locations` collection IDs. It is neither the OneOff,
NewCareer, nor first FreeSkate selection. Its use in the native project is an
explicit collision/presentation probe, not a boot-default claim.

## Per-session game load

`GameModule::GameLoad` at `0x8273be50` performs the second phase:

- loads secondary resources and preloads state graphs;
- loads the dynamic-object audio bank and additional world state;
- starts `PlayerLoader`, pumping until the skater assets are ready;
- moves stream focus to the location spawn;
- loads challenge XML, boundaries, and challenge-specific assets;
- advances through explicit asynchronous wait states rather than blocking the
  main loop.

`GameModule::UpdateGameLoad` at `0x8273ddc8` advances those wait states.

## Native reconstruction consequence

The PC implementation should preserve these observable phases and readiness
boundaries, but express them as typed Rust state machines and native jobs. It
must not reproduce the PowerPC instruction stream or depend on an Xbox kernel
or emulator implementation.

The typed core slice owns raw-preserving handles, validated request derivation,
the exact one-case-per-poll outer state, live texture-gate sampling, and the
blocking OneOff world-stream-segment call-order boundary. Its backend trait is
result-bearing so the exported `world_stream_adapter` can own the checked inner
file/XML state machine and required local/global publication barrier. The outer
record latches the first shared backend cause and never retries partial effects;
this is a safe native ownership policy, not a claimed reference recovery path.
The concrete mount-backed asynchronous read boundary now preserves the 128-slot
pool, three-active FIFO admission, release-gated leases, and load-thread
registration split; see `async-stream-loader.md`. Application scheduler wiring,
Octree spatial queries, and world-bounds effect internals remain collaborator
boundaries. The concrete publisher now composes the parsed catalog, local
Octree, and global store while leaving those effects explicit. The
stable per-file identity now follows every registration callback, and the
shared parsed-registration catalog is concrete. A dedicated real-data
application probe now composes those pieces through the exact outer
`[1, 2, 3, 4, 5, 6, 7, 0]` load trace and reverse file unload, while explicitly
stopping before XSF collection loading and production `LoadController`
scheduling. The concrete global
publication/unpublication owner now exists in
`world-stream-store.md`, and local Octree resource planning, initialization,
sequential insertion, and the matching state-5 removal loop are concrete in
`octree.md`. The adapter's token contract now enforces local release before raw
state 6 and global unpublication afterward, including typed one-shot failure
latching. Once the exact case-seven finalization has calculated world bounds,
stored outer raw state zero, and emitted its completion pulse, the safe host API
may atomically move the whole dormant Presentation/Simulation/optional-Texture
record to a diagnostic owner. That extraction adds no backend call, unload
order, or automatic destruction policy; it exists so an application probe can
drive the already-reconstructed file unload contract without inventing
`LoadController` scheduling. See
`world-stream-formats.md` for the recovered adapter order and evidence.
The catalog's ownership and confidence boundary is recorded separately in
`world-stream-catalog.md`; concrete publication ownership is in
`world-stream-runtime.md`.

## Open evidence questions

- XMM manifest header and per-stream descriptors consumed by
  `LoadGameStreamFile`.
- Resource type dispatch within `cPres_*.xsf` and `cSim_*.xsf`.
- Collision island and clustered-mesh ownership during simulation-cell load.
- OS-thread scheduling/interleaving and wall-clock timing around the proven
  blocking OneOff world-stream pump segment.
- Failure propagation above nested `cGameStream` failure state 8; the recovered
  OneOff worker has no such branch and its enclosing `cStreamFile` remains 1.
- Mission/challenge-specific location overrides and arbitrary saved-session
  orientation values.
