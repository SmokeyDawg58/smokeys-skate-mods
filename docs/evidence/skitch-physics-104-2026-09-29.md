# Skitching physics104 — implementation connected; build/playtest pending

## Latest follow-up (supersedes the historical checkpoint below)

This completes 104, which the earlier turn left
unimplemented. The missing live physics flow is now implemented and admitted.
This is a source/static-validation result, **not a compiled or runtime result**.
No Cargo/rustc/game commands ran in this follow-up.

### Implemented on the existing owners

- Shared SetPhysicsState now enters the retained Skitching allocation, calls its
  existing Reset and source Enter, and publishes104. Exit and post-physics hooks
  are the native shared empty function; deck prediction reuses the ground method.
- Update82da10a0 follows the native service order: disable hand collisions;
  primary spline readiness/copy; local physical/control inputs; ground normal;
  spline frames; internal state change; follow point; hands/state clocks; hand IK;
  board tools; deck animation offset; existing spline requests/notifications;
  release; ground Reckoning; existing Skeleton.UpdateGroundSkateboard; first flag.
- Added moving current/previous/future spline frames, curved chord-projection
  lookup, centrifugal/grip/turn/recentering controller, edge/exit states and timers.
- Speed and release forces enter the SAME board force queue (speed-conservation
  kind6); slide friction uses kind1. Existing steering, truck damping, wobble,
  heading/manual torque, anti-flip and exact angular target services are reused.
- Hand positions come from actual raw animation bones, with stance handling,
  authored-spline coordinates, extension checks, crouch and asymmetric blend
  rates. Targets use the existing Skeleton world-hand IK setters.
- FillPhysOut publishes native grabbing/crouch/grip/hand speed/spline height/
  contact speed/parametric position/reskitch delay/descriptor into existing
  components. Existing processed feedback consumes those values.
- Lateral SkitchShimmyingBehaviour reuses its already-existing Vault constructor
  payload and channels; added its0xc instance, Begin/Update and existing EndShimmy
  channel-end helper. Native ShimmySpeed normalization and channel switching are
  retained, including the unusual current-speed-relative clamp.

### Source details that must not be simplified

- Selector table82ddbff4 entry104 targets82ddc114 (105 is a different case).
  Loss of output grabbing8cc bit00800000 returns100 before wipeout300. Otherwise
  ConditionIsOffGroundSkitching82dd76f8 may return100 when teleport countdown is
  zero. Common teleport702 retains priority. Skitch does not directly select200.
- CheckForExit tests first-byte8cc bit1, i.e. host word01000000, distinct from
  the output grabbing bit. Impulse occurs at exitCount==1 BEFORE increment;
  forceExit follows count greater than the configured frame threshold.
- Off-ground selection uses the GLOBAL physics_state_skitching.default instance
  with keys0fc22cb443a763ac/8e8b8588435b6fe8/c6e6e6bd74dffb65. Local Update uses
  different keys40aad3fd99b464f4/f178f963558d24a0/03a59cdfa0b0b967.
- VMX128 disassembly/decompiler VA register labels can be wrong. Raw operands
  were decoded; standard VMADD operand order and scalar FMA boundaries matter.
  Full64 attribute keys were recovered from PPC construction, not low32 guesses.
- GrabSplineVelocity uses last-current at82d950a0; speed force precedes heading
  target recomputation. Do not change either ordering to an intuitive substitute.
- Torque82d96d38 calls XMVectorSin, not XMVectorSinCos. Deck animation rotation
  uses the latter. Reused the corresponding existing numeric helpers.
- GetCOMToDeckWorld82b3b468 copies Skeleton+2a30, the retained world-space animation
  vector. No additional world transform is appropriate.
- Live cvars use the existing registry,22 additional references at960..981 with
  defaults checked from static initializers. Existing offset-blend ref60 reused.
  The authored-spline switch, board settings and attributes remain live reads.

### Locations and verification

Physics kernels: physics/player/lifecycle/physical_player_skitch_*.rs.
Session integration: physical_player_skitch_session.rs; board integration:
physics/player/board/physical_player_skitch_board.rs. Output:
physics/player/output/physical_player_phys_out_skitch.rs. Shimmy callback:
animation/graphs/motion/callbacks/actor_motion_skitch_shimmy.rs.
All preserve the organized module layout and retained constructor payloads.

Source captures: skitch-physics-104-ida-2026-09-29.txt,
skitch-physics-104-raw-2026-09-29.txt,
skitch-physics-104-helpers-2026-09-29.txt,
skitch-physics-104-additional-2026-09-29.txt (selector, shimmy and cvar sources).

Scoped rustfmt syntax parsing/formatting passed. Module/include path audit passed
(1995 Rust files,1989 edges). Cvar definition count982 and complete unique
registration indices checked; all22 new literal labels unique. New regression
checks cover chord-projected/reversed spline points, release-count timing/lost
hands using the original-assets corpus, turn/spin sign handling, flag byte order,
selector grabbing-bit distinction and native transition admission. **UNRUN**.

Next check: build with the existing release batch, approach/grab a car,
stay attached while it moves/turns, shimmy both ways, release and bail. Exact
runtime feel and compiler correctness are not established by source inspection.
Native optional physics-debug drawing and disabled telemetry are not recreated;
no gameplay fallback or error suppression is used to hide missing behavior.

## Historical incomplete checkpoint (before this follow-up)

Reported failure: `Before(SystemLogic(Selection(PendingTransition(104))))`.
**This runtime failure is NOT fixed. Do not present this as a playable skitch build.**

## Proven cause

The selection already correctly returns104 when the grab request and primary
grab-spline readiness flags are present and skitching is enabled. See
`physical_player_system_logic_session.rs::finish_ground_selection`.
`physical_player_ground_to_air.rs::selected_transition` intentionally rejects104:
there is no CurrentPhysicalState::Skitching or admitted destination. The retained
Skitching payload has constructor/reset fields but no complete live update.

The earlier Motion callback175 change reaches this separate physics boundary.
It did not implement the attachment physics. Accepting104, changing it to100,
or swallowing PendingTransition would produce incorrect gameplay.

## Work completed in this follow-up

- Inspected the existing retained state, processed primary spline, shared board
  animation-disable path, transition publication, and pre-solve dispatch.
- IDA vtable8234a6a8 verified: Update82da10a0; Enter82d94678;
  Exit825187c0 (empty); PredictFutureOfDeck82d869c0 (existing ground method);
  GetType82d9d9e8; FillPhysOut82d97088; remaining hooks empty.
- Extracted reusable `Skitching::reset` from its EXISTING constructor code,
  retaining a single implementation. Native Reset82d887f0 preserves spline,
  descriptor, attributes/owner links, ApplySideForce, centrifugal sign, and
  ToIKHandSpaceTransform. It reads the hands-off delay again on each entry.
- Added `Skitching::try_enter` in physical_player_skitch_entry.rs, following
  Enter82d94678: Reset, DisableBoardAnimation, assert processed8d0 bit01000000,
  copy primary GrabSpline, clear only ForceExit, copy descriptor. Raw PPC
  82d946e8..473c corrects a decompiler error that otherwise appears to clear
  all flags. This entry is deliberately not dispatched into live play yet.
- Added a regression for the exact big-endian spline-ready bit. Syntax parsing
  passed. No Cargo compile/test or game run.
- Captured15 IDA functions in skitch-physics-104-ida-2026-09-29.txt. Capture is
  evidence, NOT a parity guarantee: VMX128 aliases and full64 attribute keys
  still require raw-disassembly verification before porting.

## Required before enabling transition104

1. Complete Update82da10a0 in original order: disable hand collisions; copy
   ready spline; local physical/control parameters; source-selected normal;
   UpdateGrabSpline; CheckStateChange; UpdateFollowPoint; UpdateSkitchState
   (including UpdateHands); UpdateHandIK; UpdateSkateboard; UpdateDeckAnimOffset;
   request/notify the retained PlayerGrabSpline; CheckForExit; existing ground
   reckoning and skeleton update; clear first-evaluation flag.
2. Port underlying speed/steering and release-force helpers, using existing
   board toolkit and force accumulators. Existing geometry supports spline
   length traversal; source GetPosAlongCurveByStraightLength needs checking too.
3. FillPhysOut82d97088 must publish actual crouch/grip/hand speed, spline height,
   contact speed/parametric position/reskitch delay, descriptor and grabbing flag.
   Extend processed feedback and state104's own selection/release conditions.
4. Wire complete lifecycle through CurrentPhysicalState, GroundDestination,
   SetPhysicsState, Update1/2/3/5, pre/post physics and output category paths.
   Audit exhaustive matches; use shared BLR Exit rather than Ground.Exit.
5. Preserve existing animation entry/holding work. Lateral shimmy animation
   callbacks remain another known gap (see callback175 evidence).

Relevant helper addresses: UpdateLocalPhysicalParameters82d88c58,
UpdateLocalControlParameters82d88dc8, UpdateGrabSpline82d94760,
UpdateFollowPoint82d95468, CheckStateChange82d95ce0, UpdateHands82d960e0,
UpdateDeckAnimOffset82d96430, CalcExitPushBackForce82d966a8,
CalcExitPushForwardForce82d96848, CalcSpeedForce82d969c8,
CalcSkitchTorqueVectorTowardsTarget82d96d38, UpdateSkitchState82d9da00,
UpdateHandIK82d9dc58, CheckForExit82d9e6d0,
CalcSkitchHeadingDisplacement82d9e8d8, UpdateSkateboard82da0960.
