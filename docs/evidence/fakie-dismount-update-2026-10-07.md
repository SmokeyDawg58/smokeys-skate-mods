# Fakie dismount state update (2026-10-07)

**Status:** `cargo build --release -p skate2-pc` PASS. Not run in game, because no scripted demo rides fakie and dismounts. Worked against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

## Report

`controlled tick after 1927 completed frames: Failed { stage: QueuedFrame, detail: "Tick(Actor(Motion(PendingStateUpdate(2190406600))))" }`. 2190406600 = 0x828ef3c8.

## Graph

- `tools/graph_query.py motion update 0x828ef3c8` gives one state: DismountFromFakie100 (Motion0 / OffBoard1 / OBGround8 / Dismount32 / DismountFromFakie100, expression 3). That is stepping off the board while riding fakie.
- Its ordered behaviour slots are `[0..13, 193, 194]`:
  - Motion0 and OffBoard1's fourteen slots;
  - SetTurning 193 (`angle`, `dir`, `holding`, `quickness`);
  - PlayAnimation 194 (`B_SWITCH`, `playBackSpeed 1.25`, `time 0.15`).
- OBGround8 and Dismount32 add no slots.
- The siblings IdleDismount101 (0x828ef428), RunDismount102 (0x828ef4b0) and RunDismountWaitForLanding103 (0x828ef550) were already in the update allowlist. (100, 0x828ef3c8) was missing.

## Native thunk

`StateUpdate_DismountFromFakie_100` 0x828ef3c8 runs three calls:
- `StateUpdate_OffBoard_1` (slots 0..13);
- `StateBehaviourDirect_SetTurning_Update` on packed slot 14 (+0x70/+0x74);
- `PlayAnimation::Update` on packed slot 15 (+0x78/+0x7C).

So the allowlist entry is `(100, 0x828e_f3c8) => [0..13, 193, 194]`. SetTurning and PlayAnimation run through the existing payload handlers in `generated_motion_runtime_lifecycle.rs`.

## Exits

DismountFromFakie100's transitions 5 and 6 go to IdleDismount101 and RunDismount102 under expression 4 (c15 WillExpire). Those targets' guards are 274 and 275. Expressions 3, 4, 274 and 275 all exist in Pre (`generated_motion_runtime_selection.rs`) and in Sustain.
