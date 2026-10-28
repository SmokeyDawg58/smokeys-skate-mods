# Nine reports: hippy, nose/rocket grabs, late flips, HasTweak, HandsFace reuse, RB skitch (2026-09-26)

**Status:** rustfmt parse PASS on every touched file, and no touched file gained a rustfmt hunk. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`. **Item 9 (RB) is NOT fixed**; see section 9.

## 1. Motion PendingExpression(38)

- t317 HippyRunout16 -> WipeOut4.
- Pre 8290bf9c: cached `OBTimeToLand(lessEqual 0.05)` (c86), returned directly.
- Sustain: the native default.

## 2. Motion PendingExpression(179), plus the nose-grab and RocketAir families

**179.** The Cycle1149 (nose grab) guard.
- Pre 8290f644: cached `HasIntent(NoseGrab)` (c272).
- Sustain 8291fc98: the shared case 179/332, the same c272.

**Closure audit.** Run from Cycle1149 (roots Down251/Grab689); every entry is decoded, including cache-bit checks:

| Expr | Pre | Semantics | Sustain |
|---|---|---|---|
| 176 | 8290f50c | RocketAir Into992 = c269 `HasIntent(RocketAir)` | default |
| 177 | 8290f56c | Cycle993 = c270 `HasIntent(RocketAir)` | 8291fc38 = c270 |
| 178 | 8290f5cc | Out994 = !c269 | default |
| 183 | 8290f7dc | nose Out996 = !c271 `HasIntent(NoseGrab)` | default |
| 329 / 330 / 331 | 82914c40 / 82914d0c / 82914dd8 | c271 && c265 / c244 / c240 (NoFootAirWalk / Left / RightOneFootAir) | default |

The only remaining item in this family is GrabOut1246, the known FingerFlipOut frontier.

## 3. Motion PendingCondition(228)

- `HasTweak(greaterEqual 0.8)` was blocked by an index gate (`229 | 236`) on the HasTweak runtime arm.
- The constructor at 82cbabf0 builds the ComparisonCondition base and then assigns the literal "TweakX" (+28) and "TweakY" (+40).
- So the graph's four HasTweak instances (228/229/236/253) differ only in their comparison.
- `ActivationFunc` 82cc26f8 matches the existing arm: both HasIntent reads, the presence check, X^2 then fmadds Y^2, rsqrte with two refinements, then EvalCompare.
- The gate was removed.

## 4. PendingStateUpdate 0x828f4b10: ground BS/FS grab powerslides

- LeftSlide/RightSlide 1319/1320/1322/1323.
- The native thunk expansion (35 calls) matches every alias position by position.
- The InCandidateSlidingState, HandBusy and DisallowPumping updates are BLRs.

## 5. PendingStateUpdate 0x828f05e0: hippy jump

**Updates.** Carve279/281 at 828f05e0 (25 calls), plus the rest of the family found by the closure audit: KickTurn278/280 at 828f0568, Into93 at 828ef1d8, Cyc94 at 828ef260. All are thunk-verified.

**Guards.**
- **264** (82911868): Anticipation91 -> AnticCycle92 = c397 `InStateForTime(Anticipation > 0.49)`.
- **265** (829118c8): = c399 `ApexReached`.
- **266** (82911928): = c89 `PhysFilteredState(ground)`.
- **364** (829168f8): the Into93 guard = !c362 || !c363, i.e. not both RightPush and LeftPush; c362 false short-circuits true.
- **365** (829169c4): Into93 -> WipeOut4 = c19 `WillExpire(0.1)` && !c202 (ground) && !c398 (offboard). Its Sustain (829214d8) drops the WillExpire read.

**New condition: `ApexReached::ActivationFunc` (82c9b460).**
- It reads IPhysical PhysOut+24 `PhysOut_SystemReckoning`+10 `mCentreOfMassVelocity`, splats lane Y, and runs vcmpgtfp128 zero > Y.
- The raw word 1b7cf165 decodes VA to v60 (the zero splat), although IDA prints VA92.
- Port: `0.0 > com_velocity.y`, the same convention as Air Post.

## 6. PendingStateUpdate 0x828ee9b0: SpawnDismount31

- The thunk (17 calls) is verified.
- The closure audit also found Sustain 1 (SpawnDismount31's own guard) missing; it is the native default.

## 7. Late flips: PendingCallback {672, 0x82cb04d0}

**`SetDeckPitchAndYaw::Begin` (82cb04d0).**
- It asserts IPhysical, ISkaterAnim and IAnimatable.
- It then reads fresh PhysOut+14 `PhysOut_Skeleton` +218 `mSkeletonDeckYaw` and +21c `mSkeletonDeckPitch`.
- It writes each through IAnimatable vfunc+48 with (0, -1), using the behaviour's `skateyaw`/`skatepitch` names. This is the same call as native AttachIntent::Update's settable path, which the port models as `Animatable::AddSettableAtt82d7f5f0` (`try_add_gesture_attribute(name, value, false, -1)`).

**Other callbacks.** CreateInstance is the shared null (82bb3da8); Update and End are the shared BLR (825187c0).

**Producer.** The deck yaw/pitch stores are at 82b583f4/82b58474 in Skeleton FillPhysOut, which is already ported.

**Allowlist.** 253..258 at 828eff60 (28 calls) are thunk-verified.

## 8. HandsFace `ReuseFixedRootsPending` (vert-wall ollie)

**Native GetNextLoc (82a24c28) reuse path.**
- It enumerates graph roots (pid -1) after `mBeginCommandsNodeId` in node order.
- It gives consecutive slots only to non-fixed roots; opcode 12/15 with bit0 set is skipped.
- It then emits reuse command 15 for the popped stack top, carrying that top's fixed bit.

**Native DispatchReuseBufferCmd (82a2fc18).**
- Its destination cursor (r5, advanced at 82a2fe6c) advances for EVERY root. That includes fixed roots (fd90) and roots at or before `mStartBuffSegmentId` (fd58).
- It records copies (mSqts[cursor] <- mSqts[command output]) only for non-fixed roots after the boundary.
- As a result, a live fixed root ahead of dynamic roots shifts the copy destinations but not the reader slots. That is native behaviour and is reproduced exactly.

**Buffered `SetDataById` (82a24890).** It emits opcode12|bit0 and pushes a pid -1 graph node, so every fixed SQT is a counted fixed root.

**Port changes.**
- `ReuseState.fixed` tracks fixed roots in node order: `set_fixed_sqt` → `produced_fixed`, and consumption is LIFO in `produced`.
- `compact` adds earlier fixed roots to each copy destination.
- A fixed stack top becomes the fixed reuse root.
- Fixed outputs of any other producer (an in-place Mirror of a fixed pose) remain an explicit `ReuseFixedRootsPending`.

**Related fix.** `set_external_sqt` (native SetData 82a2c828: GetNextLoc, then opcode12 without bit0, plus a graph node) now registers its dynamic root through `produced`, as `push_fetch` does.

**Tests.**
- The existing consumed-fixed test now declares its fixed operand consumed.
- New: `live_fixed_root_advances_reuse_copy_cursor_but_not_reader_slots`, `fixed_stack_top_becomes_the_fixed_reuse_root`, `in_place_fixed_mirror_keeps_reuse_pending`.

## 9. RB: `FinishGround(GrabbableObjectSearchPending)`, NOT fixed

**What RB does.** RB is the skitch reach.
- With `mGrabWorld` set (8cc bit 24), Ground Update (82da1068) calls `PhysState_PhysicsGround::CheckForGrabbableObject` (82d9faa8).
- That function reads the `physics_state_skitching` box attributes, runs `InitializeGrabbableBoxTransform` and `ConsiderGrabSplineQueryResults` (0x5dc), and calls `PlayerGrabSpline::StartGrabSplineQuery(GSQ_VEHICLE)`.
- The query is served by `GrabSplineQueryManager::GrabSplineQuery` (827b1ed8). It calls `LW::Man::GetVehicleMan` → `LWVehicleMan::GrabSplineQuery` and `LWDynamicObjectMan::GrabSplineQuery`, then sorts by distance.

**Why it is not fixed.** A faithful fix needs the living-world traffic/vehicle and dynamic-object managers, which the port does not have. Returning an empty result set without them would fabricate world state, so the explicit pending error is kept.

## Proof and remaining frontier

**Proof.**
- `tools/compare_graph_expressions.py`: motion 598 matched / 0 mismatches (+29: 14 Pre and 15 Sustain); action 171 / 0.
- New test `hippy_nose_rocket_and_runout_guards_keep_native_reads`.
- No duplicate match arms or allowlist keys (542 entries).

**Remaining frontier (explicit pending).**
- FingerFlipOut GrabOut (2568/3e20) and its guards.
- FootPlantAbsorb (3bf8).
- InAirMount35 (828eea88), reached from Mount34.
- MVOBJ/step under Locomotion.
- The skitch/vehicle chain (section 9).
