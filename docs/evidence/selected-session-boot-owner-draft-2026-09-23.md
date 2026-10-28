# Selected session boot owner draft — 2026-09-23

The independently reviewed bounded draft was registered as
`crates/skate2-core/src/session/startup/selected_session_boot_owner.rs`; its historical
review patch is `artifacts/selected-session-boot-owner-draft-2026-09-23.patch`.
The author ran standalone rustfmt and reverse patch checks before integration;
root ran `cargo check --workspace`. Registration and type checking do not prove
a runnable frame or native whole-boot parity.

## Bounded behavior and ownership

`SelectedSession<T, C>::try_boot_prefix` calls the existing original-asset
bootstrap, the registered selected input bootstrap, and the registered
selected world/local constructor. Input bootstrap reads `input.cfg` once
between device-manager Init and the SIM/PRES caches, then starts Juice using
the supplied pad/transport, actual INI outcome and explicit host policy. A
connected Juice socket stops at `HandshakePending`. The selected world is not
constructed after that failure. `next_gate()` reports
`NativeSpawnScheduleAndRegistrationRequest` after a successful prefix. This
is the largest currently source-ordered session prefix. The selected player
constructor issues a Register request before Actor; its now-integrated owner
retains that exact request on success. Native command-3 sender timing remains
unproven.

For a controlled first-test sequence only,
`try_continue_controlled_player(HostSpawnWhileRegistrationPending,
...)` calls the registered
`SelectedSessionPlayerStartup::try_new_with_input_owner` with the input
bootstrap's exact shared `Arc<SelectedInputMap>` and `JuiceTrickInput`. The
host supplies exact spawn, marquee, Actor flags and thread answer. The asset
bootstrap's authored `A_RobsPark` transform is never silently substituted.
The current selected player constructor builds Actor while its loader remains
`Registering`. IDA's ordinary presentation path needs Actor submission to
create the entity that can complete registration, so completion-before-Actor
is not a valid gate. This controlled path still lacks native command-3 timing
and roster admission. `next_gate()` keeps reporting
`NativeSpawnScheduleAndRegistrationRequest` after that diagnostic continuation
because native spawn scheduling remains unresolved, even though the player
owner now holds the exact request;
`controlled_next_gate()` reports its separate pair/presentation progress.

The controlled pair continuation captures inputs from the same Actor's
evaluated animation **before** binding Action services. It binds Action
services from the retained selected Vault and same world's debug overlay,
retains them, initializes the existing gesture mapping with an explicit host
allocation callback, then installs the services and calls the registered pair
constructor with host numeric, capsule, assertion and hat answers. A failed
mapping or physical factory is retained. Only after the pair can the
controlled presentation method initialize the existing retained presentation
resource; it is not a registration receipt or native presentation schedule.
Native roster/graph/observer admission follows physical construction and is
not yet lifted into this owner.

The owner is mutated in place. Completed stages survive later errors and
attempt guards prohibit replay. Existing world and player constructors return
only an error; failures *inside* them still discard their local partial owner.
`ActionRuntimeServices::try_bind` likewise returns only an error before a
service value exists. The boot draft does not claim retention across those
inner API boundaries. It never exposes a runnable frame; terrain/query
residency, camera capture, actual input queue/RNG binding, tick scheduling and
same-frame palette/material publication remain separate gates.

## Direct source check

IDA `sk82_na_m.xex`: `SkaterManager::SpawnPlayer` at `0x82600630` allocates
one `Actor`, appends it to the skater list and simulation entity manager,
sorts the skater list, then calls `SkaterManager::InitPlayer` at `0x8260078c`.
`InitPlayer` is `0x826000b0` and loads Action/Motion graphs after the Actor
exists. `Simulation::InitAnimation` is `0x828a5640` and is a separate earlier
animation initialization stage. The Actor constructor at `0x825ff2b4`
reaches physical allocation around `0x825ff2d8`; the
`PhysicalPlayerHiLOD` constructor at `0x82e03c50` owns the subsequent
physical component chain. The Rust draft makes no claim that its explicit
controlled path recreates the intervening native manager scheduling.

The actual registration producer has a longer path. After
`PlayerLoaderImpl::OnSimCharacterLoaded 0x82817f28` applies attributes,
`SimCharacterManager::Register 0x82817a08` reserves a request and sends the
presentation request. `PresCharacterManager::ReceiveSimRegisterCharacterRequest
0x8281bab8` invokes `CharacterRecipeLoader::Register 0x82cfd418`, which stores
the peer and publishes asset notification. `PresThreadMsgMan::ReceiveRegSchemaMsg
0x8277c318` invokes `SkaterPresentation::RegRecipe 0x827e6198`, whose recipe
map resolves the real recipe. Crucially,
`PresCharacterManager::Update 0x82818f2c..82818f94` waits for
`GetSkaterPresEntity(character_id)` to return an entity with peer equal to
the loader's registered peer. Only that condition publishes
`LoadStatus_Registration` at `0x82818f8c`. The simulation receiver at
`0x82816c10` dispatches `SimCharacterRequest::OnRegistered 0x82815528`,
which calls `PlayerLoaderImpl::OnSimCharacterRegistered 0x82818098` and clears
the matching current request. That callback creates no Actor. The separate
PlayerUI command-3 path reaches `SpawnPlayer`; its message timing relative
to the Register request remains untraced.

IDA also shows the ordinary `SkaterPresEntity` is created by
`SkaterPresentation::UpdatePresEntity 0x827e62cc..827e6320`, called by
`DoSimRestore 0x827e69f8` using skater data submitted through
`DoSimSubmit 0x827dd2e8` from `SkaterManager::GetActor` and
`ISubmittableSkater`. Requiring the matching presentation entity and status
before first Actor would cycle. `PlayerUI::ReceiveSpawnMsg 0x828ac8b0`
checks the supplied location spawner, gets the transform and dispatches
command 3. `GameModule::ResetGame 0x8273ab5c` passes `mLocationMan`; its
vtable slot `+0xC` points to a constant-true body at `0x826c6d80`.
The receiver itself does not wait for loader completion. The sender timing
is still unknown.

The current `MountedPlayerRecipeLoad::try_load_selected` ends at
`PlayerLoadAction::Register` and `MountedRecipePhase::CallbackDelivered`.
`PlayerLoaderSlot::character_registered` can check request/character identity
and mark `Loaded`, but has only test callers. There is no production recipe
registration queue, `SkaterPresEntity`/peer association, presentation Update
condition, status delivery or typed completion receipt. Calling
`character_registered` directly from the boot owner would fabricate that
producer and is rejected.

The independently reviewed narrow extraction was integrated; its review
patch remains in
`artifacts/selected-player-registration-before-actor-split-2026-09-23.patch`,
with the corrected source trace and producer plan in
`docs/evidence/selected-player-registration-before-actor-split-2026-09-23.md`.
It returns the actual Register request before Actor and keeps the existing
wrapper. It does not supply a completion status. The current selected
presentation initializer requires Actor's physical animation and does not
publish a live `SkaterPresEntity` or peer-matched status.

## Review gates

1. Retain the now-extracted Register request in the session's selected player
   owner. Establish the command-3 sender timing; the receiver's spawner check
   is constant true and cannot substitute for that trace. After Actor/pair
   construction, add the native roster, presentation submit/restore entity
   creation, peer-matched status and simulation completion service. Do not
   treat the Register request as a completion receipt.
2. The boot module and shared-map/Juice player injection are now registered
   and `cargo check --workspace` passes. Compare the selected mounted roster
   and inject failures at each stage; type checking does not establish
   original-data execution or partial-owner retention.
3. Address inner world/player failure ownership before advertising full
   partial-owner retention. Roster/camera/input frame services still follow
   the pair and need their own source-ordered session continuation.
