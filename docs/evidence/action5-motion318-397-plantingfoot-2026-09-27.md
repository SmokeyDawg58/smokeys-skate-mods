# Action 5, Motion 318/397: switch late flips, Mute/Stale fingerflip shuvs, PlantingFoot push tricks (2026-09-27)

**Status:** rustfmt parse PASS; every new hunk is rustfmt-neutral and pre-existing diffs were left alone. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

Playtest report (controlled skating test):
- `Tick(Actor(Action(Update(PendingExpression(5)))))` after 259 frames
- `Tick(Actor(Motion(PendingExpression(318))))` after 113 frames
- `Tick(Actor(Motion(PendingExpression(397))))` after 30 frames

## 1. Action expression 5 (mirrored "Switch" leaves)

**Where it is used.** Expression 5 is the state guard of every mirrored leaf:
- the late-flip Switch96/98/100/102/104/106;
- the grind-grab Switch128/130/181..195;
- the ground-grab Regular161/163.

The late-flip Regular95..105 siblings use 70 (`DontMirrorTrick || !IsMirrored`). So any mirrored late flip or grab evaluated 5.

**Native.** Pre 828ffd5c and Sustain 82906300 are the same code: cached `IsMirrored` (sConditions+0x44 = c17), returned directly. Expression 4 (828ffce8) is its negation.

**Update lists.**
- Already admitted: Switch104, the grind grabs (828edc40: 127..130, 180..195) and the ground grabs (828edb50: 160..163).
- Not admitted: the late-flip Switch96/98/100/102/106.
- `StateUpdate_Switch_104` 828ed980 (IDA) runs `StateUpdate_OnBoard_6` (packed 0..18), then four `StateBehaviourDirect_CreateMGIntentFromAGIntent_Update` calls at packed 19..22 (+0x98..+0xb0).
- Each new leaf's slots are [0..10, 42..49, 87, first, first+1, first+2], with first = 165/171/177/183/195.
  - All four are CreateMGIntentFromAGIntent. For example, Switch96 maps AG `L_F_Kickflip` to MG `L_F_Heelflip`.
- The existing 828ed980 arm now admits 95..=106.

## 2. Motion 318 and the Mute/Stale grab frontier

**Evaluation order.** Into669/676 (guard 41 = WillExpire 0.01) and the unguarded Cyc670/677 transitions evaluate these targets in order:
1. FingerFlip671/678 (150);
2. MuteFingerFlipShuv672/679 (318);
3. StaleFingerFlipShuv673/680 (319);
4. Out674/681 (t1280/1303 guard 41, or t1286/1309 guard 162, then target guard 163/165).

318 was the first unported guard in that order; the rest sat right behind it.

| Expr | Pre | Native semantics | Sustain |
|---|---|---|---|
| 318 | 8291438c | `HasIntent(FingerFlipShuv)` c234, then `HasIntent(MuteGrab)` c233 | default 829221c8 |
| 319 | 82914458 | c234, then `HasIntent(StaleGrab)` c256 | default |
| 162 | 8290ef54 | `HasFilteredMotionGraphIntent(TweakY, abs, less 0.2)` c254, direct | transition only |
| 163 | 8290efb4 | `!HasIntent(MuteGrab)` c233, cntlzw on both paths | default |
| 165 | 8290f08c | `!HasIntent(StaleGrab)` c256, cntlzw on both paths | default |

**Already admitted:**
- update lists 672/673/679/680 (828f2508) and 674/681 (828f16f0);
- the outgoing targets Cyc659/Cyc648 (828f1448) and InAir21.

None of these transitions has a hook.

## 3. Motion 397 and the PlantingFoot push-trick families

**Structure.**
- PushTrick74 (a Trick19 child, guard 122) evaluates five families in order: FSBoneless218, BSBoneless219, FSFastPlant220, BSFastPlant221 and NoComply222.
- Each family's only active child is PlantingFoot604/608/612/616/620, guarded by 397/399/400/401/402.
- No port code or evidence referenced PlantingFoot or these families before, so this subtree had never been reached.
- Porting only 397 would fail at 399 as soon as the FS boneless intent is absent.

**Guards.**

| Expr | Pre | Native semantics |
|---|---|---|
| 397 | 82919378 | `PhysFilteredState(ground)` c202, `HasIntent(FSBoneless)` c203, `HasIntent(NewFootPlant)` c204 |
| 399 | 829195bc | c202, `HasIntent(BSBoneless)` c208, c204 |
| 400 | 829196e0 | c202, `HasIntent(FSFastPlant)` c209, c204 |
| 401 | 82919804 | c202, `HasIntent(BSFastPlant)` c210, c204 |
| 402 | 82919928 | c202, `HasIntent(NoComply)` c211, c204 |
| 123 | 8290dfcc | `HasIntent(LeftFootPlant)` c205, direct (Left943..951) |
| 124 | 8290e02c | `HasIntent(RightFootPlant)` c206, direct (Right944..952) |

- Sustain for 397 and 399..402 is the shared case 8291fe20 (cases 200, 397, 399-402): c202 only.
- Sustain for 123 and 124 is the native default.

**Update 828f21e8** (`StateUpdate_Right_948`, shared by all ten Left/Right leaves):
1. `StateUpdate_FSFastPlant_220`: `StateUpdate_Trick_19` for packed 0..22, then packed 23..29 = SetTrickAttr (BLR), CreateAttribute x2, HandBusy (BLR), IsDoingTrick (BLR), CreateAttribute x2.
2. Packed 30 PlayAnimation, 31 SetTrickHeight (BLR), 32 InitPush (BLR).
3. Packed 33 `ComputeTargetCoefsFromSpeedAndStrength`, 34 `SetPushCoefs`.

The merged `AIProcess::OnEnd` is the shared no-op Update.

**Admission.**
- All ten aliases have identical class layouts at packed 23..34, checked from the generated data.
- (943..952, 0x828f_21e8) were added with each alias's exact ordered slots.

**Already in place:**
- FootPlant/Lift/LeftGround 605..623 (828f0e70); 828f0e70 calls the same FSFastPlant_220 prefix.
- PlantingFoot exits t944/956/968/980/992 (guard 88) and the WipeOut4 exits (39/300) have Pre and no hooks.
- Every behaviour class involved already has Begin/Update/End arms in `generated_motion_runtime_lifecycle.rs`.

## 4. Reach audit

Scratch `reach_audit.py` mirrors `generated_motion_runtime_graph.rs`: search, valid_target, evaluate over active children, and Sustain on the current ancestry.
- Bounded to the new leaves (672/673/679/680, 674/681, 943..952, 605..623), the only gaps left are the known InAir21 t1048 grind over-approximation.
- Those grind guards (44..85, 289/290, 382..386) stay parked behind the grind policy.

## 5. Changes

- `generated_action_runtime_selection.rs`: `(_, 5) => c(17)?`.
- `generated_action_runtime.rs`: the 828ed980 late-flip arm admits 95..=106, and the tail table gains 96/98/100/102/106.
- `generated_motion_runtime_selection.rs`: Pre 123/124/162/163/165/318/319/397/399..402.
- `generated_motion_runtime_sustain.rs`: defaults 123/124/163/165/318/319, plus a separate `397 | 399..=402 => condition(202)` arm. Widening the aligned 200 arm would have made rustfmt realign that comment block.
- `generated_motion_runtime_lifecycle.rs`: (943..952, 0x828f_21e8).
- Tests, not run:
  - `late_flip_and_grab_switch_leaves_read_condition17_in_both_masks` (Action);
  - `mute_stale_fingerflip_and_planting_foot_guards_keep_native_reads` (Motion).

## 6. Checks and unknowns

- `rustfmt --edition 2024 --check` shows no diff in any touched hunk. Pre-existing diffs remain in `generated_action_runtime.rs` (`additional_shared_update_slots` and its test) and in `generated_motion_runtime_selection_tests.rs` (older compact tests).
- NOT compiled or tested.
- Once these states are selected, the push-trick animations and physics are not verified here. That is left to a playtest.
