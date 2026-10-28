# Selected player/Actor startup extraction draft — 2026-09-23

Integration result: independently IDA-reviewed and registered as
`selected_session_player_startup.rs`. The first patch missed the corpus-only
`player_index` assertion local; root restored that lookup through the same
retained `services.skaters` owner before validation. The selected original-data
baseline passed 23/23 in
`artifacts/original-baseline-selected-player-startup-2026-09-23.log` and the
workspace/format checks passed in
`artifacts/workspace-selected-player-startup-2026-09-23.log` and
`artifacts/fmt-selected-player-startup-2026-09-23.log`. Independent IDA review
confirmed this preserves the selected Rust mounted constructor prefix only:
native `Simulation::InitAnimation` precedes the later SpawnPlayer command,
whose roster/graph/observer sequence remains a separate integration gate.

Unregistered draft: `crates/skate2-core/src/session/startup/selected_session_player_startup_draft_20260923.rs`. Review patch: `artifacts/selected-session-player-startup-2026-09-23-draft.patch`. The patch registers the same source as `selected_session_player_startup.rs` and replaces only the mounted `actor_player_startup_corpus::construct` constructor prefix. `rustfmt --check` on the draft and `git apply --check` pass. No registered file or Cargo run was made by this author; root must compile and run the mounted baseline after independent review.

## Source-backed movement

`actor_player_startup_corpus.rs::construct` currently opens a test corpus mount, obtains `world.local_skater_identity`, derives its local index, initializes `AnimationJobManager`, creates `MountedAnimationSource`, fills `MountedSelectedOwners`, then calls `SelectedPlayerStartup::try_initialize_mounted` using the caller's world, Vault, cvars, marquee, spawn and flags. The new fallible `SelectedSessionPlayerStartup::try_new` moves exactly that production-capable composition into one owner. It takes a supplied `Arc<MountSet>` instead of opening test archives, and a supplied `FnMut() -> Result<u64, E>` thread answer instead of selecting fixture thread 1. The existing `MountedAnimationSource` invokes that callback at its native sampling point. The test still supplies thread 1 explicitly, as test data. It still selects `DEFAULT_MARQUEE_ID`, `actor_flags: 1`, and its actual mounted spawn explicitly; this is not a production spawn-manager claim.

The existing `SelectedPlayerStartup::try_initialize_mounted` remains the only implementation of offline player creation, marquee attribute lookup, recipe load/registration phase, global animation init and the `try_initialize` input.cfg/Actor/animation sequence. The new constructor returns its retained `SelectedPlayerStartup` **and the same mounted animation source**. The corpus continues to inspect the same loader, recipe, input, Actor listener and animation state. It also keeps its `!native_roster` component-only graph load branch; the constructor does not load graphs or fabricate a registration callback. `try_initialize_mounted_presentation` remains a separate subsequent call with that same source, preserving the native registration gap. Its current test call follows the corpus assertions; a live host must supply any real intervening registration work when reconstructed.

The typed new-prefix failures are `WorldLock`, `MissingLocalIdentity`, `LocalIdentity`, `InvalidLocalIndex`, `AnimationJobs`, and `Mounted(MountedSelectedError<MountedAnimationSourceError<E>, ReckoningHostNumericError>)`. The latter retains the existing foreign-world/index, player creation, attribute, recipe, animation, input, Actor and numeric error path. Failure ends this boot attempt; this wrapper does not claim a retryable transaction after partial owner writes. It does not construct the physical pair: `SelectedPlayerStartupOwners::try_physical_inputs` and `PhysicalAssemblyWorld`/Actor allocation and `try_complete_selected_constructor` remain in `physical_player_ground_startup_corpus.rs` and are the next extraction gate. The `native_roster` branch's camera capture, graph registration/admission and streamed geometry remain later gates. No renderer, PROXY activation or playable loop follows from this patch.

## Mounted test rewrite and gate

The patch changes the common `repeated_frames` player constructor prefix, so both its native-roster and historical component-fixture modes exercise the same production constructor. It destructures `SelectedSessionPlayerStartup { selected: mut retained, source: mut startup_source }` and leaves all assertions and the explicit `!native_roster` graph fixture in place. At integration, root should run formatting, the selected original-data mounted roster including three-frame camera and scripted Kickflip, and the workspace suite. The comparison is meaningful because the corpus still checks mount identity, registering loader phase, recipe registry/scene identity, loaded-graph state, same Actor listener, and subsequent physical/camera tick behavior.
