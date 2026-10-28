# Selected production input owner draft, 2026-09-23

Unregistered draft: `crates/skate2-core/src/selected_production_input_owner_draft_20260923.rs`.
No registered source or Cargo files were edited and no Cargo check was run. The
root integrator must independently source-review, register, compile and connect
the draft before claiming a runnable PC path. `rustfmt --check` passes for the
draft file. Confidence: high for the cited native ordering; medium for host
composition until compiled and exercised with a connected XInput pad.

## Source order and actual owners

Fresh IDA decompilation confirms `InputManager::Init826e2788`: it constructs
`cInputDeviceMgr` at `826e27b4..d4`, loads `config/input.cfg` at `826e27ec`,
constructs and initializes two `cPadCache` owners at `826e2818..2848`, and
creates four group pads for each. It constructs vibration owners and calls
`Vibration::Reset` at `826e2a8c`. `Sk8AppMan::InitBoot82740458` calls input
manager Init at `82740db8`, publishes it at `82740dbc`, starts Juice at
`82740e38`, then installs `JuiceBase::Instance` as the external remap at
`82740e54`. Thus an already-mounted selected player must not be a prerequisite
to Juice startup. The draft separates an early `SelectedInputBootstrap` from
the later player bind. The bootstrap exposes its one shared `JuiceTrickInput`
clone for `MountedSelectedOwners::juice`.

The revised bootstrap reads and parses `input.cfg` once after device-manager
initialization and before its two caches/Juice. It retains an
`Arc<SelectedInputMap>`, exposes that Arc after successful startup, and rejects
a different Arc at player bind. The registered selected player startup still
parses its own map and creates a second Juice occurrence. The narrow unapplied
patch `artifacts/selected-production-input-shared-map-juice-20260923.patch`
adds `SelectedSessionPlayerStartup::try_new_with_input_owner` for both shared
owners and preserves its existing `try_new` wrapper. The underlying mounted
initializer uses a supplied map when present and skips the mount read/parse;
the existing None path stays for legacy corpus callers. That patch requires
separate review and registration before this owner can bind to a selected
session. No duplicate parser or mapper is introduced in the intended path.

Fresh IDA `TimerCallback82729770` polls hardware at `82729798` then caches
group 0 SIM and group 1 PRES at `8272979c..b4`. The draft's explicit
`poll_and_cache` preserves that order using `SelectedDeviceInputRuntime` and
the supplied `PadPlatform`; the application must pass `WindowsPadPlatform` and
own the timer cadence. `try_preupdate_enqueue` advances the existing SIM cache,
focus prefix, current device cache line, retained map/command codec and
`SelectedSimCommandQueue`; the same queue and `SimulationCommandRandom` are
returned to `SelectedQueuedTickInput`. `PhysicalGameInfo`, can-simulate and
focus remain caller-supplied after the preceding application effects.

## Juice readiness boundary

Fresh IDA `JuiceNG::Start82a76e90` invokes `SetupGamePad` at `82a76f7c` and
only then writes `sJuiceReady=1` at `82a76fb0`. `SetupGamePad82a75c00`
constructs 24 button channels and registers controllers 0 and 1 at
`82a75ea8`. `JuiceBase::UpdateGamePad82a74c70` exits when readiness is false;
`GamePadPlugin::Update82ed8768` additionally requires `index<mDeviceNum`
before updating a device. Existing `JuiceInputBoot::try_start` stops a connected
socket at typed `HandshakePending`; it does not perform protocol handshake or
registration. The draft therefore keeps partial bootstrap state and refuses
player binding on that branch. Explicit `DisableBeforeStart` or a genuine
failed first connection can install the retained Juice remap. The original
`Juice_m.ini` has `enable=1`; disable must be an explicit host choice, never a
silent default. The host supplies actual file bytes or a proven absent-file
result and retains `WindowsJuicePlatform` for its Winsock lifetime.

The next enabled-Juice route is to implement the existing boot owner's
`InitialConnectionHandshake82969eb0`, update-thread and `SetupGamePad`
registration path, then publish readiness only after those stages. A generic
pass-through remap, successful-socket shortcut, or constructor device-count 0
would prevent faithful connected input. Independently, XInput capabilities
queried during vibration Reset can be unknown for a slot that connects later;
the existing `XenonPadDevice` then returns `UnknownGuitar` on connected sample.
Native hotplug/capability refresh ordering needs a separate source check before
claiming late-connect support. A pad connected at startup with successful
capability output can take the disabled-Juice path.

## Typed partial state and remaining integration

`try_initialize` is one shot. Failed manager/map/cache/actuator/Juice stages retain
already-created fields in `SelectedInputBootstrap`; another call returns
`AlreadyAttempted`. `try_bind_player` returns the bootstrap plus caller-supplied
queue/RNG on missing/foreign map, missing completed boot, or controller-selection
error. The bound owner keeps focus/pause Arcs, copied selected controller and
player indices, the actual device runtime, queue and RNG. It does not borrow
the player startup across session storage. Each frame borrows the actual loader
briefly and checks its player index before draining SIM; sampling and command
packing reuse the existing helpers. `rustfmt --check` passes for the draft;
`git apply --check --ignore-space-change` passes for the narrow patch on this
CRLF worktree. No Cargo check was run. The root still needs independent patch
review, registered integration, real Windows adapters, timer/PreUpdate
scheduling, queued tick connection and focused tests before a skating claim.
