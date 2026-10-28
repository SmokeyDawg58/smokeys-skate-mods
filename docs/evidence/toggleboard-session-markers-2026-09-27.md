# LT/RT board drop/throw/fetch (ToggleBoard) and session markers (2026-09-27)

**Status:** rustfmt is clean on every touched hunk, and every new file is rustfmt-clean. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

## Part A: LT/RT off the board

### Why LT/RT did nothing

The input side already worked:

- The action fill sends a fresh full LT as `OB_DropBoard`, a fresh full RT as `OB_ThrowBoard`, and either trigger as `OB_RetrieveBoard`.
- OffBoard2's `CreateMGIntentFromAGIntent` forwards each of them unchanged to the Motion graph.

Two breaks sat in the Motion `ToggleBoard` node:

1. `ToggleBoard::Update` 82cc2c08 was still `PendingCallback 0x82cc2c08` in the generated lifecycle.
2. The old adapter compared intents against `OB_DropBoardIntent`, `OB_ThrowBoardIntent` and `OB_RetrieveBoardIntent`. Those are the C++ global names. Their `FastString<36>` values (dynamic initializers 82f81f70/88/a0) are `"OB_DropBoard"`, `"OB_ThrowBoard"` and `"OB_RetrieveBoard"`, so `HasIntent` could never match.

### ToggleBoard::Update 82cc2c08

- While grabbing (OffBoard +0x38), no command is read.
- Every command requires the idle state:
  - Drop needs the board held (+0x3f): state 5, clear bit 0x40.
  - Throw needs the board held: state 5, set bit 0x40.
  - Retrieve needs the board not held: state 1.
- Dispatch: state 0 runs ResetYawPitch; states 1-4 run UpdateRetrievalState; states 5-6 run UpdateDropState.

### UpdateDropState 82cbbc38

- It publishes `AddMgAttribute` and `AddSettableAtt("OB_DroppingBoard", 1)`.
- State 5 moves to 6 and calls `TransitionTo("RetrieveBoard", throw ? OFB_THROW_0 : OFB_THROW_90, b, {0,F,F,1,b,F,b,F}, T,T,T)`, where `b = kMaxRetrieveBoardBlendFrames * 1/60`.
- State 6 ends the channel prematurely, with `kMaxRetrieveBoardBlendOutFrames * 1/60`, and returns to state 0 when any of these holds:
  - the channel is gone;
  - `TimeLeft` is not greater than 0;
  - `TimeElapsed` exceeds `kMaxDropBoardPlayOutFrames * 1/60`.

### UpdateRetrievalState 82cbb240

`"OB_RetrievingBoard"` is always published.

- **State 1:** YawPitch, then state 2 and `TransitionTo(retrieve[0], b, ..., T,T,T)`.
- **States 2/3:** YawPitch.
  - A missing channel, or a non-positive `TimeLeft`, returns to state 0.
  - State 3 while holding moves to state 4 with `TransitionTo(retrieve[2], b, ..., T,F,T)`.
  - Otherwise, when `TimeLeft` is not greater than `b`, it moves to state 3, calls `SequenceTo(retrieve[1], ..., T,F,T)` and publishes `"OB_RetrieveBoard"`.
- **State 4:** the same end-channel conditions with `kMaxRetrieveBoardPlayOutFrames` return to state 0.
- **Tail:** `yaw` (mirror-corrected), then `pitch`.

The retrieve clips are `OFB_RETRIEVE_HIGH_FRONT/BACK_INTO/CYC/OUT`.

### UpdateYawPitch 82caae98

1. `yaw = yawToFreeBoard * -57.29578`, negated when mirrored.
2. Below -90 it snaps: with smoothing, `FSEL(prevSigned, 180, -90)`; without smoothing, `yaw < -135 ? 180 : -90`.
3. It is negated again when mirrored. With smoothing, the change is clamped to `kMaxRetrieveBoardYawChange`.
4. The pitch is `pitchToFreeBoard * 57.29578`.
5. The back clip set is used when `|signed yaw| > 90`, otherwise the front set.

### Channel runtime

StartTransition 82d7ee68 and NewChannel 82d7eb98 are generalized to the by-value `tChannelParams`:

- speed, mirror, first/last frame, cycle, blending;
- the `use attributes` flags;
- signed priority insertion.

It also adds:

- `Sequence` (+0x48): plays FROM until it expires, then TO, carrying the overshoot.
- `TransitionBlend` (+0x64).
- Resurrect.
- `TimeElapsedInChannel` and `TimeLeftInChannel`, both FSEL clamps.

Get/SetAttributes follow the Sequence (FROM until `PlayingNewAnim`) and Blend (TO) targets.

### Cvars and physics

Cvars 584-588 are in section `Animation_RetrieveBoard`:

| Cvar | Value |
|------|-------|
| `kMaxRetrieveBoardYawChange` | 5 |
| `kMaxRetrieveBoardBlendFrames` | 15 |
| `kMaxRetrieveBoardBlendOutFrames` | 10 |
| `kMaxRetrieveBoardPlayOutFrames` | 10 |
| `kMaxDropBoardPlayOutFrames` | 30 |

The physics consumers of the published attributes and flags were already ported: SkateboardController let-go, throw, hide, retrieve and hold.

## Part B: session markers

### Input

`input.cfg` (miscboot.big) defines the two states:

- `SessionMarkerSet` (52) = `LBumper.held && DPadD.pressed`
- `SessionMarkerUse` (53) = `LBumper.held && DPadU.held`

The buttons are DPadU = Button0, DPadD = Button1 and LBumper = Button8. Execute opcode 5 is a short-circuit AND. `cInputMap::GetState` 8298c8c8 is `Execute != 0`.

The production map now parses these two states (`parse_with_player_ui_inputs`), and `.held`, `.pressed` and `&&` are supported. GetState is read on this controller's drained SIM pad.

### Native frame order and the port

**`Sk8AppMan::PreUpdate` 8272c478**

- If `LastInput(SIM)` is false, it returns before anything else.
- Otherwise it runs the stream managers, then `Flow::Manager::Update` (StartApp flag +0x85), then UpdateVibration.

**`GameModule::DoGameUpdate` 82730418**

Only when `mInputUpdate` is set, it runs in order:

1. the network vcall;
2. `SessionMarker::Update` (GameModule+0x50, vtable +4);
3. `GameUI::Update`, which calls `PlayerUI::Update(!paused && lastSync, feFocus)`;
4. `GenerateSyncCommand`.

**`PlayerUI::Update` 828af9f8**

1. Without canSimulate, it returns.
2. A focus change sends the GainFocus command.
3. With a marker (+0x14) and focus, it runs `UpdateSessionMarker`.
4. +0x18 is `INetworkGameCollisionQuery`, which is only present in network games.
5. Sampling and DoCommand follow.

**`SimController::Update1`** flushes eEQGameSim before NIS, the queued commands and SimUpdate1.

**Port (`try_tick_once`)** does the same in this order:

1. `try_last_input`;
2. Flow pump, then `SessionMarker::Update`;
3. `try_player_ui_prefix`;
4. `UpdateSessionMarker`;
5. `try_player_ui_sample_and_sync`;
6. eEQGameSim flush into PlayerUI;
7. the tick: queued commands, then `SkaterManager::OnCommand`, then the Actor frame.

### SessionMarker (0x80 bytes)

The constructor 828adab8 sets identity axes with a zero translation, RIGHT, on-board 1, and every other byte clear.

`Update` 828acc90 does the following:

- **Front end not in game:** both permissions become false.
- **canSet:** true when any of these holds:
  - network game;
  - Create-A-Spot;
  - `kDisableSessionMarkerPreConditions`;
  - not `mInNoSkateZone`;
  - the security service;
  - `!(gameMode != 3 || securitySetting)`.
- **Set verifies, in order:**
  1. LocalPlayerOkToSet: the skating rules, plus a respawn check in network games.
  2. ChallengeOkToSet.
  3. Progression `"Set Session Marker Visible"` (single player, unless Create-A-Spot).
  4. SignUp.
- **canUse:** starts as `valid`, then the Use verifies run (the chase check applies to use only, and so does the HOM allow).
- **Stairs:** a local Actor on stair surface 8 clears canSet.

The skating rules:

- **Set, ground category:** at least 2 wheels and not skitching (104).
- **Set, offboard category:** state other than 501.
- **Use:** blocked while wiping out without being able to leave, and in states 104 and 501.

`SetSessionMarker` 828acf30 accepts when any of these holds:

- force;
- canSet with a verified position;
- canSet with the security service running.

`VerifyPosition` 828ab328 reads WorldPainter layer 6 (`eSecurityGuardAreas`) at (x, z). `HighAlert_Area` and `NoSkateZone_Area` reject an offline set. Otherwise the local breadcrumbs decide through `IsPostionOk` 82ce1fc0, which is `_TestForCollisionConstraints(marker, false)`.

### PlayerUI::UpdateSessionMarker 828af3f0

The order is: `GetSimInput`, then `GetLocalActor(playerIndex)` (ghost, else FindByIndex), then `GetState(52)`, then `IsSetAllowed`, then the Actor.

**Set path.** It reads `IsOffboard`, `GetFootForwardWithFakie` and `CalcRespawnTransform`, then calls `SetSessionMarker(force false, onBoard = !offboard)`.

**Use path (53 held).**

- Releasing the input clears the latch, the notice and the clock.
- When not yet used, and (`kDebugSessionTeleport` or IsUseAllowed) with an Actor present, the clock advances by exactly 1/60 per update.
- `distance = |PhysOut Skeleton+0x1a0 - marker.w|`: vmsum3fp, then rsqrte, two fused Newton steps, `x*y`, and vsel to 0 for a zero square.
- Nothing more happens unless the distance is strictly greater than `kMinDistanceForTeleport` (0.5).
- The hold time ramps from 0.2 s at 100 m or less to 1.0 s at 1000 m or more (fnmsubs/fmadds).
- It teleports when `IsSafeToUseSessionMarker` 828aa130 holds and (debug or `held > hold`). That check is: no requested teleport (+0x3b), not (wiping out (+0x32) and Skeleton+0x248 pending), and state not 702.
- The teleport sets the latch and snow = 3, then sends `StateMessage_TeleportToPosition(transform, foot, onBoard, marker = 1, largeRadius = 0)`.
- A refused use latches the front-end notice once. Snow counts down every update.

### Teleport chain

**`DoTeleport` 827767e8, no-load-screen branch.** It calls `MoveStreamFocus(transform.w, "StateLoading::DoTeleport", false, 0xff)`, which is the port's terrain `try_update_focus`, then `SendTeleportMsg` 82758388. Key -1 sends two messages on Simulation+0x38:

- `cMsgSetSpawnStance` 0x8bac4885 (+0x10 foot);
- `cMsgTeleport` 0x8fcdc5e2 (four rows, +0x50 onBoard, +0x51 marker).

**`ReceiveSetSpawnStanceMsg` 828aa418** stores PlayerUI +0xa0.

**`ReceiveTeleportMsg` 828ad120**

- It skips an all-zero translation.
- Otherwise it sends Command type 5 to receiver 1 through the proxy, calls `PauseMan::Pull(kPAUSE_STATE_MANUALCAM = 6)`, and sends `cMsgSetManualCamState` to Presentation.

**`SkaterManager::OnCommand` 82600920** switches on `mType - 3`; case 2 is type 5.

1. Defaults: index -1, RIGHT, identity axes with zero w, onBoard 1, marker 0.
2. GetData.
3. `FindByIndex` 825f19c0 (ISkaterManager vtable +0x18). A null result goes straight to UpdateLocalPlayer.
4. `AdjustSpawnPosition` 825f1a20: +0.75 to -500, hit +0.15. It runs in network games or when `kbEnableAdjustSpawnPosition` is set.
5. `CensusMan::ClearPoint`.
6. `Actor::SpawnFootForwardStance` 825ed638.
7. Actor vtable +0x3c `Teleport`.
8. If the marker byte is 1, `cMsgUseMarker`.
9. UpdateLocalPlayer.

**Timing:** a request in frame N is pumped by Flow and eEQGameSim in frame N+1, and the command runs in frame N+2, as in native.

### SpawnFootForwardStance, and a bail-respawn fix

The SkaterAnim constructor 82cc0a90 stores three vtables:

| Offset | Vtable | Slot +0xc0 |
|--------|--------|------------|
| +0 | Animatable-base, 0x82302d28 | `SetSpawnFootForwardStance` 82cb7868 |
| +0x52b0 | IChannelAnimatable | |
| +0x52e0 | primary / ISkaterAnim, 0x82302c08 | `IsTryingToSprint` |

- Actor+0x1fc holds the ISkaterAnim pointer.
- `SpawnFootForwardStance` downcasts it by -0x52e0 and calls `SetSpawnFootForwardStance`: `+0x6144 = foot == 0 ? (natural == 1 ? 0 : 1) : (natural == 1 ? 1 : 0)`. Actor+0x240 = foot follows.
- `TeleportSomewhereAppropriate` 825f0ecc..ef0 (the bail respawn) inlines exactly this with the breadcrumb's stance.
- The port had read that slot through the ISkaterAnim vtable (`IsTryingToSprint`) and discarded the result, so a bail respawn never wrote the spawn stance.
- Both paths now share `Actor::try_spawn_foot_forward_stance`. `ResetToGivenStance::Begin`'s `ResetToSpawnStance` 82cb77e0 consumes it.

### Answers from game-layer owners the port does not run

These are the native constructor or idle states:

- The front end is in game.
- The Create-A-Spot service bytes +0x34/+0x35 (Simulation+0xe8, then +0x12c) are never set.
- `IsSecurityServiceRunning` 828ab278 resolves to `LivingWorld::Manager::IsBigBlackServiceActive` 8271d3a8. That reads +0x1b5b4, which constructor 82721fd0 and Reset 8271d504 clear.
- There is no main activity.
- The teleport overrides are empty.
- No chase runs.
- The HOM constructor allows use.
- The sign-post message table is empty, so the default (true) applies.
- The sign-up state is 0.

These paths are explicit error boundaries that offline game mode 1 never reaches:

- `securitySetting` (ProfileFreeSkate only);
- the network respawn check;
- `TeleportRequiresLoadScreen` (it needs the security service);
- the security popup;
- the ghost;
- `CreateSessionMarker` for OnlineChallenge.

`CreateSessionMarker` 82729c28 returns null for PartyPlay. The port creates the owner once the controlled frame is ready, as `ResetGame` does.

### Deviations

1. **Load screen.** DoTeleport takes the no-load-screen branch without evaluating `TeleportRequiresLoadScreen` (`IsAreaLoaded` is unported). The selected terrain never unloads admitted collections, and a marker is only set where the skater stood. The load-screen state (`LoadingMan::Push(6)`) is unported.
2. **Popup order.** The popup condition is evaluated network, then security, then load screen. Native evaluates network, then load screen, then security. All three are free of side effects, and the security service never runs in the port.
3. **eEQGameSim queue.** Teleport messages use a FIFO beside `SimulationSimRate`'s SetSimRate FIFO; native has one FIFO. The handlers touch disjoint state, and the port flushes the teleport FIFO immediately before the tick, whose first step is the SetSimRate flush.
4. **OnCommand timing.** `SkaterManager::OnCommand` runs after the batch rather than inline. Its writes (the Actor reset and the SkaterAnim spawn stance) are disjoint from the input receivers' writes, and both finish before the Actor frame.
5. **Marker UI.** The marker UI is skipped: `cMsgSetMarker`, OnTrySet/OnTryUse, the crackle amount, the snow message, `cMsgUseMarker` and ManualCam.
6. **Unported producers.**
   - The DMO locator table is empty (no `cMsgAddDMOToSessionMarker` sender).
   - ClearPoint has no census population.
   - `mInNoSkateZone` stays false because its WorldPainter key-change producer is unported. VerifyPosition still rejects no-skate and high-alert areas when setting.
   - `IsSessionMarkerValid` and `ClearSessionMarker` have no port callers.
   - SimController `mStreamPauseTick` is not modeled.
7. **AdjustSpawnPosition.** It is now one shared helper for the boot spawn and OnCommand. The boot path fetches the ground pool before the network and cvar reads; that ordering is host-only.

Cvars 589-595:

| Cvar | Value |
|------|-------|
| `kMinTimeUseSessionMarkerHeld` | 0.2 |
| `kMaxTimeUseSessionMarkerHeld` | 1.0 |
| `kMinDistanceForMinTimeUseSessionMarkerHeld` | 100 |
| `kMaxDistanceForMaxTimeUseSessionMarkerHeld` | 1000 |
| `kMinDistanceForTeleport` | 0.5 |
| `kDebugSessionTeleport` | false |
| `kDisableSessionMarkerPreConditions` | false |

## Tests

- `motion_toggle_board_tests.rs`
- `session_marker_tests.rs` (10)
- `session_marker_player_ui_tests.rs` (9)
- `session_marker_teleport_tests.rs` (7)
- the `input_map` session-state test

## Playtest

**Off the board:**

- Holding the board: LT drops it (OFB_THROW_90) and RT throws it (OFB_THROW_0).
- Without the board: either trigger fetches it with the retrieve clips.

**Markers:**

- **LB + D-pad Down** sets a marker. You must be skating on at least two wheels and not skitching, or be off the board in any state except OffBoardPushing (501). Stairs and no-skate areas refuse it.
- **LB + D-pad Up**, held, teleports you. It takes 0.2 s near the marker and up to 1 s at 1000 m, and the teleport lands two frames after the hold completes. You arrive with the stance and board state you had when you set the marker.
