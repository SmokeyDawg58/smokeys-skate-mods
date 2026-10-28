# Independent review: selected production input owner, 2026-09-23

**Approve the revised unregistered input owner as a bounded component; reject
production boot integration until the selected player constructor accepts its
shared map and Juice trick occurrence.** The first draft deferred the native
early `input.cfg` map and borrowed player fields in a way a retained session
could not own. The author revised those parts during this review. The owner
remains uncompiled and has no registered production caller. No registered file
was edited and no Cargo command was run for this review.

Follow-up independent review approved the narrow, still-unapplied
`artifacts/selected-production-input-shared-map-juice-20260923.patch` for that
constructor injection. It adds `try_new_with_input_owner` while preserving the
legacy wrapper, passes one preloaded Arc map and Juice occurrence into the
existing inner constructor, and skips the later map read/parse on the supplied
map branch. `git apply --check --ignore-space-change` passes. This does not
upgrade the component to a compiled or runnable production session.

Direct IDA check on `sk82_na_m.xex`: `InputManager::Init826e2788` constructs
and initializes `cInputDeviceMgr` at `826e27b4..d4`, calls
`LoadInputMap("config/input.cfg")` at `826e27ec`, constructs/initializes the
SIM and PRES caches at `826e2818..291c`, then resets vibration at `826e2a8c`.
`Sk8AppMan::InitBoot82740458` calls that Init at `82740db8`, publishes the
manager at `82740dbc`, calls `JuiceManager::Start` at `82740e38`, then installs
`JuiceBase::Instance` as external remap at `82740e54`. The revised draft
creates the device manager, reads and parses `input.cfg` once into an
`Arc<SelectedInputMap>`, creates both group caches, resets actuators, then
calls Juice construct/Start and installs remap. This restores the observed
InputManager/InitBoot order inside the bounded owner. The registered player
constructor still parses its own map later; that duplicate must be removed
through an explicit shared-map constructor input.

`TimerCallback82729770` calls `PollHardwareForInput` at `82729798` then loops
`CacheInput` from `kINPUT_SIM` through the two input groups at
`8272979c..a8`. The draft's `poll_and_cache` preserves that bounded order.
`JuiceNG::Start82a76e90` calls `SetupGamePad` at `82a76f7c` and writes
`sJuiceReady=1` only at `82a76fb0`; `SetupGamePad82a75c00` registers its
devices at `82a75ea8`. Existing `JuiceInputBoot::try_start` leaves a successful
socket at `HandshakePending`, and the draft retains that failure without
installing a fake ready remap. An explicitly disabled or failed first
connection uses the existing retained remap handle. These branches are honest
within their documented limits.

The remaining integration gates are concrete:

1. `selected_session_player_startup.rs::try_new` constructs a new
   `JuiceTrickInput::new_static_initialized()` inside `MountedSelectedOwners`.
   `SelectedInputBootstrap::trick_input()` shares its own occurrence, but the
   current player API offers no way to pass that occurrence to the Actor.
   Inject this exact clone into player startup before calling it a shared
   input path.
2. The revised `SelectedProductionInputOwner<C>` owns focus/pause clones,
   captured controller/player index, device/cache/map, queue and RNG; it no
   longer borrows the selected player. Its per-frame method takes the actual
   loader and rejects a different player index. This resolves the structural
   self-reference problem. The controller is captured once from the same
   backend settings and loader at bind.
3. The revised bootstrap retains the early map and exposes `shared_map()`
   only after full input initialization. `try_bind_player` requires
   `Arc::ptr_eq` with the selected player's map. The missing step is a
   registered player constructor parameter for that exact Arc, with the
   existing `GameInputFrameSession` reset using it instead of rereading
   `input.cfg`.

The draft's in-place `try_initialize` retains completed device/cache owners
and Juice boot state on actuator or Juice failure, then refuses retries. Its
`try_bind_player` returns bootstrap, queue and random on typed pre-move
errors. Fresh manager/cache constructors have no meaningful
intermediate failing allocation modeled here, so the stated partial retention
is appropriately limited to represented stages. Host numeric implementations
and explicit pad/INI policies are not test fixtures. Runtime ordering and
same-Actor command delivery still need original-data tests after shared
map/Juice injection and root registration.

The revised `ForeignPlayer` guard runs after SIM `last_input` drains, so a
caller mistake can consume that queued cache line before reporting identity
failure. This is a host misuse guard rather than a native scheduling claim;
move it before drain if the intended contract is nonmutating rejection.
