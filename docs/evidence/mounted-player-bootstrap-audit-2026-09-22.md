# Mounted player bootstrap extraction audit

Read-only code audit, with this evidence file explicitly authorized. Exact
Test-Path was false before creation. No production implementation or Cargo run.
Confidence high for the present call/ownership inventory. Proposed extraction
is a selected offline session, not proof of the complete native application.

## Existing chain to reuse

`physical_player_assembly_corpus::mounted_paired_physical_construction` builds
the original Vault class set and selected physics inputs, then calls
`physical_player_ground_startup_corpus::check_schedule_mode`. The latter creates
the World and predictor/query/debug/grind/local-identity services, constructs
the same Actor/animation/input chain, allocates its physical owner, builds
board+skeleton, finishes HiLOD, publishes the actual Actor cached allocation,
and enters the existing repeated-frame composition.

The reusable production pieces already exist:

| Responsibility | Existing production entry |
|---|---|
| Original asset reads | `skate2_formats::mount::{MountSet, MountedBig}` |
| Linked startup cvars | `InputCvarManager::{try_link, try_deserialize_startup}` and retained references |
| Animation global startup | `SimulationAnimationStartup::try_initialize` / `AnimationStartupSource` |
| Command/input ownership | `GameplayInputUpdateManager::{try_construct, try_reset}`, `GameInputCommandManager`, `GameInputFrameSession::try_step` |
| Actor ownership | `ActorConstruction::{new, try_construct_prefix, try_construct_listener, try_construct_graphs}` |
| Animation mount | `try_construct_animation_prefix`, `try_construct_vault_suffix`, `try_finish_loaded_face_with_source`, `try_publish_animation_constructor` |
| Animation Initialize | `ActorConstruction::try_initialize_animation`, `try_construct_animation_physics_input`, `try_get_bind_pose` |
| Runtime animation/input services | `ActorBodyAnimationServices::try_from_startup`, `ActorInputRuntimeServices`, `ActionRuntimeServices::try_bind`, actual gesture-mapping runtime |
| Graph loading | `ActorGraphStartupOwners::{try_new_with_motion, try_enable_player}`, Actor `try_load_action_graph` / `try_load_motion_graph` |
| Same-pair physical startup | `try_allocate_physical`, factory board/skeleton construction and `try_finish_skeleton`, pair controller/state constructors, `try_finish_pair_hilod_constructor`, `try_publish_physical_constructor_return` |
| Selected tick | input `try_step` -> `try_selected_actor_frame_prefix` -> `try_prepare_ground_pre_solve` -> `try_step_authored_ground` -> `try_post_solve_selected_ground` |

`LoadedActionInputSession` is already only an alias of production
`GameInputFrameSession<ActorIdentity>`; do not create another input owner.
The `OriginalPhysicalAnimationInputs` carrier can become a fallible production
constructor-handoff helper: preserve the same hierarchy/parameter/pose owners,
then release its temporary bind-pose buffer at the recovered constructor end.

## Fixture-only collaborators that cannot be copied into bootstrap

- `game_input_recognizer_corpus::check` performs multiple authored Ollie sample
  deliveries, Fill calls with DiagnosticPhysics(state0), history/latch tests,
  and only later constructs/returns animation and graphs. Calling this helper
  would start gameplay with test-mutated recognizer/listener state. Extract
  construction only; first real input must arrive through the host tick.
- `simulation_animation_startup_corpus::MountedSource` is a real archive reader
  but reports controlled thread ID1 and owns corpus counters/environment-root
  lookup. A production adapter should accept retained mounts/cvars/job manager
  and actual host thread identity; `try_initialize_local_backend` is already a
  production job-manager initialization, not a fake job callback.
- `game_input_recognizer_corpus::AnimationSelection` hardcodes local0,
  network=false, ghost=false and DEFAULT_MARQUEE_ID. Its manager Arc returns are
  real, but selection responses are not a mounted PlayersBackEnd.
- `actor_animation_mount_corpus::InitializeSource` reads actual job cvar and
  managers but hardcodes regular stance and skaterID0; custom gesture/style
  methods panic. Existing `PlayersBackendSettings` only retains truck/wheel
  settings, not marquee/stance/skater/CAC selection. Source-backed retained
  player selection is a real remaining dependency.
- `load_tutorial_face` uses a literal tutorial_male and request/callback IDs1.
  Reuse `CharacterFaceManager` loading/dispatch, but derive face identity from
  the selected original marquee layout/recipe and retain actual request owners.
- `actor_animation_mount_corpus::initialize` calls body-session `corpus::check`,
  which dispatches/synchronizes body frames twice and exercises failure probes.
  Remove those diagnostics from startup; retain only source-required constructor
  dispatches/pose evaluation, using the existing production body services.
- Assembly `Services` returns tolerance0.01 and entity flags1. Read actual
  Actor fields at source query points and actual linked `kOrthonalTolerance`.
  Current SpawnServices is infallible, so a production adapter needs an explicit
  fallible boundary for missing/unlinked cvars rather than default substitution.
- Assembly `DriveCvars` hardcodes1/10. `kALC_HardDriveDisp` and
  `kALC_HardDriveStr` must be registered/read live. Existing evidence records
  initializer82f7d588/82f7d5e8 and source Disp/Str/Disp/Str sampling order.
  Spawn tolerance initializer82f7ba20, label spelling kOrthonalTolerance, is
  documented in skateboard-spawn-transform-source.md. These three cvars are
  currently absent from InputCvarRuntime definitions.
- Constructor `HistoryInput` hardcodes Up=[0,1,0,-0] and XYZ-COM mode0. Reuse
  `input_cvar_runtime::postphysics_history_input::PostPhysicsHistoryInput` with
  the actual owner's Processed Arc and cvars. This carrier already supports
  typed unknown W and deferred Up reads. The factory's board construction calls
  `try_initialize_constructor_fields_in_sim_allocation`, whose Processed
  constructor calls Reset82e00f78; Up220 is therefore already source-initialized
  before `try_finish_skeleton`. Do not insert an extra Reset.
- Initial physical inputs still declare bouncy_ragdoll0 and is_human1 as
  controlled selections; production must obtain those from selected character/
  Actor inputs. Hat type and face name already have actual Vault lookup examples.
- Tick `OfflineSync` returns2, raw input is a neutral24-value array, hands clamp
  panics, and high LOD/wait/flags/pool capacities are explicit fixture policies.
  The facade must retain an explicit supported offline schedule and real input
  mapping; it must not describe these constants as recovered backend owners.

## Narrow implementation proposal

First implement live **constructor services**: register the three missing
source cvars, adapt SkeletonDriveCvars, reuse existing Processed history adapter,
and read actual Actor flags. Resolve the spawn service's error contract at its
existing call chain. This removes behavior-changing fixture values before a
larger bootstrap starts consuming them.

Then extract one retained mounted-session constructor taking owned mounts,
Vault/cvar managers, actual player-selection source, selected authored spawn,
and explicit supported host schedule/numerical policy. It should own the same
shared World, Actor, command/gameplay session, global animation startup/managers,
face manager, body/input/action services, physical handles, ground and asset
residency. Use fallible original methods above; do not call any corpus `check`.
Existing local identity supplies local/network/ghost ownership where exposed;
extend the retained selection owner only from actual source initialization/
recipe data, rather than implementing the traits with former fixture answers.

Its tick accepts host-mapped PlayerInputData (later raw XboxPadSample conversion
uses existing input_pad), runs the existing five-stage selected composition,
and returns current physical/animation publication for rendering. Preserve
grind policy explicitly and stop at existing unsupported graph/state branches.
Expose a small application-facing API only after this internal extraction;
do not make the entire private simulation surface public or create another
physics/animation/input scheduler.
