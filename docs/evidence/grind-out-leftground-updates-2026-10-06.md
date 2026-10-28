# Grind-out ollie/nollie state updates (2026-10-06)

**Status:** `cargo build --release -p skate2-core` PASS. NOT run in game. Worked against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

## Report

`controlled tick after 1381 completed frames: Failed { stage: QueuedFrame, detail: "Tick(Actor(Motion(PendingStateUpdate(2190418128))))" }`. 2190418128 = 0x828f20d0.

## Graph

- `tools/graph_query.py motion update 0x828F20D0` gives four aliases:
  - Ollie: Takeoff515/FromAntic789 and GrindOutAssist514/LeftGround786.
  - Nollie: Takeoff560/FromAntic868 and GrindOutAssist559/LeftGround865.
- Only (789, 0x828f20d0) and (868, 0x828f20d0) were in the update allowlist. LeftGround786 and LeftGround865 are the leaves under the trick families' GrindOutAssist (expression 446), so the error means an ollie or nollie out of a grind.
- Neither GrindOutAssist514 nor GrindOutAssist559 contributes a behaviour slot.
- The LeftGround slots differ from FromAntic only in the leaf PlayAnimation and SetTrickHeight:
  - 786: 1464, 1465 (`B_OLLIE_A`, `transitionUnder`, `setToValue 1.0`);
  - 865: 1621, 1622 (`B_NOLLIE_A`).

## Native thunk

`StateUpdate_FromAntic_868` 0x828f20d0 runs three calls:
- `StateUpdate_Ollie_188` (the folded ancestor update for slots 0..27);
- `PlayAnimation::Update` on packed slot 28 (+0xE0);
- `AIProcess::OnEnd`, the merged no-op that is SetTrickHeight's Update, on slot 29 (+0xE8).

All four aliases have PlayAnimation and SetTrickHeight in those slots.

Existing handlers cover both parameter forms:
- `SetTrickHeight::Begin` (`actor_motion_set_trick_height.rs`) takes `setToValue`, which sets `set_manually`.
- PlayAnimation's `transitionUnder` is parsed at construction and used by many states already.

## Exits

LeftGround786/865 transitions:
- InAir21 and Land22: expression 88;
- Idle1025: expression 11;
- KickoutDismount13: expression 292.

All three expressions exist in both Pre (`generated_motion_runtime_selection.rs`) and Sustain.

## Frontier audit

There are 32 LeftGround leaves under GrindOutAssist families:
- 0x828f20d0: 2, added now.
- 0x828f2138: 24 flip families, all 24 present (with their FromAntic siblings, 48 entries).
- 0x828f32b0: 4, all present.
- 0x828f3110: 2, missing.

`StateUpdate_LeftGround_1065` 0x828f3110 also serves FromAntic1068 and FromAntic1073, so none of its four aliases were admitted. It runs the folded `StateUpdate_TrickInto_509` for slots 0..26, PlayAnimation on slot 27 (+0xD8), and the no-op on slot 28 (+0xE0). All four aliases have PlayAnimation and SetTrickHeight there.

These families are guarded by `IsProSkater(MikeCarroll)` (expression 86, c142) and `IsProSkater(MarkGonzales)` (expression 91, c165), so a created skater never reaches them. Their TrickCyc/TrickOut updates were not audited.

## Change

`generated_motion_runtime_lifecycle.rs` gains the ordered slot lists for:
- (786, 0x828f20d0) and (865, 0x828f20d0);
- (1065/1068/1070/1073, 0x828f3110).
