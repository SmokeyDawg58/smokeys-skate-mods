# Motion 324/138: tail-grab one-foot, Superman and one-foot grab families (2026-09-26)

**Status:** rustfmt parse PASS on every touched file; the new hunks are rustfmt-neutral, and the appended test follows the test file's existing compact style. NOT compiled or tested. Worked solo against IDA for the Jan-17-2009 debug `sk82_na_m.xex`.

Playtest report (controlled skating test):
- `Tick(Actor(Motion(PendingExpression(324))))` after 749 frames
- `Tick(Actor(Motion(PendingExpression(138))))` after 50 frames

## 1. The two reported guards

- **324** is the state guard of `Into1296`: TailGrab one-foot air, left foot. Ancestry: Grabbing989 -> OneFootAir1145 -> TailLeftFoot1232.
  - Cycle1143's unguarded t1341 evaluates OneFootAir1145, so this runs in every tail-grab frame.
  - Pre 8291484c: cached `HasIntent(TailGrab)` (c263), then `HasIntent(LeftOneFootAir)` (c244), short-circuit AND.
- **138** is the state guard of `SupermanInto953`, reached from DBLGrab `Cyc643` via the unguarded t1060 -> Superman645.
  - Pre 8290e598: cached `HasIntent(Superman)` (c226), returned directly.
- Both Sustain cases land on the native default 829221c8, which returns true.

## 2. Frontier closed in the same families

`motion_reach_audit.py` was run over the transition closure of both leaves, bounded to their family roots. The InAir21 t1048 over-approximation and OnBoard3 re-entry are excluded; every flip's TrickOut/Land already exercises those. The audit showed the next guards these leaves reach, so they are ported too. Otherwise the very next frame would fail:

| Expr | Pre | Native semantics | Reached from |
|---|---|---|---|
| 139 | 8290e5f8 | `!HasIntent(Superman)` (c226) | SupermanCyc954 t1071 -> SupermanOut955 |
| 145 | 8290e868 | `HasAnimAttribute(OneFootEarlyOut)` (c241) | one-foot Into -> EarlyOut (t1351, and t1106/1118/... in the FS/BS families) |
| 146 | 8290e8c8 | `!HasIntent(RightOneFootAir)` (c240) | right-foot Out guard (Out1301, Out1110/1118/1128/1136, ...) |
| 147 | 8290e940 | `ShouldPrepareOneFootAirForFootplant` (c242) | Footplant guard (FS/BS one-foot, unguarded t1107 etc.) |
| 148 | 8290e9a0 | `IsFootPlanting` (c243) | Footplant Into -> Absorb |
| 149 | 8290ea00 | `!HasIntent(LeftOneFootAir)` (c244) | left-foot Out guard (Out1298, Out1114/1122/1132/1140, ...) |
| 170 | 8290f284 | `!HasIntent(NoFootAirWalk)` (c265) | NoFootAirWalk Out1231 / Out1242 |
| 174 | 8290f434 | `!HasIntent(TailGrab)` (c263) | tail-grab Out990 (t1355) |
| 323 | 82914780 | `TailGrab && NoFootAirWalk` (c263, c265) | Into1296 t1350 -> NoFootAirWalk1144 -> Into1229 |
| 325 | 82914918 | `TailGrab && RightOneFootAir` (c263, c240) | t1341: evaluated right after 324 when the left intent is absent |

- The condition index is the sConditions offset / 4. Each one was cross-checked against its cache word and bit (for example 226 = word 0x1c bit 2, sC+0x388).
- Every Sustain case above is the native default 829221c8, so the port returns true with no reads.

## 3. New conditions and cvar (FS/BS one-foot Footplant path)

Porting 145 moves FS/BS one-foot grabs from failing at 145 to failing at 147. To avoid leaving that half done, both Footplant conditions were ported 1:1.

**`ShouldPrepareOneFootAirForFootplant::ActivationFunc` (82ca7700).**
- Reads IPhysical vfunc+0x28 -> PhysOut+8 `PhysOut_Air` +c0 `mTimeToLandFootPlant`.
  - This is the same accessor `TimeToLand` 82c9cc40 uses, which the port reads as `phys_out().try_air_output()`.
- A negative time returns false before the cvar is read. Otherwise the result is `time <= kFootPlantTimeToLandThreshold`; unordered values fail.

**`IsFootPlanting::ActivationFunc` (82ca7830).**
- `PhysOut_Air` +1b0 `mIsFootPlanting`, then a fresh +c4 `mFootPlantTotalAbsorbTime >= 0` (ordered).
- Motion builds it as `MotionCondition::Shared` because it is an Action `ConditionClass`, so it lives in the Shared arm.

**Producers, verified by native writer.**
- `FootPlantManager::FillPhysOut` (82db6890) is gated on +1d5 and writes:
  - Air +c0 <- +1b8, +c4 <- +1bc, +1af <- landing, +1b0 <- landing || planting;
  - surface;
  - left/right = planting && part 15/19.
- It is identical to `try_fill_footplant`, which the live `try_continue_update5_plants` calls from post-solve conditioning.
- `PhysOut_Air::SetResetValues` (82e24ce0) resets +c0 and +c4 to -1.0 and the flags to 0, matching the port's defaults. With no predicted footplant, both conditions are false and the one-foot grab proceeds normally.

**Cvar 560 `kFootPlantTimeToLandThreshold`.**
- Section `Animation_Tricks_FootPlants`, title "Time to land threshold", default 0.5, GUI range 0..1, null callback.
- Initializer 82f88940, table slot 82fd512c, reference 830a4078.
- Registration order: inserted between 546 (table 82fd4f70) and 39 `kAnimUpGesture` (82fd5168), the port's nearest registered neighbours in the table.
- Accessor: `try_foot_plant_time_to_land_threshold`.

## 4. Lifecycle: DBLGrab266 at 828f0068

- `Cyc643` has an unguarded t1056 -> DBLGrab266 (Land22 -> GrabLanding83), gated only by Land22's guard. Landing while holding the double grab would fail with PendingStateUpdate(0x828f0068).
- The thunk expansion of 828f0068 (29 calls) matches 266's authored slots position by position, as it does for siblings 264/265 (leaf PlayAnimation 689, `L_DBLGRAB`).
- As current, it produces an audit issue set identical to the shipped FSGrab265.

## 5. Hooks and behaviours

- No transition hooks exist in the tail/Superman closure.
- The FS/BS Cyc -> Footplant transitions carry blend Override hook 9 on transitions 1110/1122/1134/1146/1209/1221/1233/1245, which are already in the audited Override gate.
- Every behaviour class in both closures is already handled by shipped grab and landing states.

## 6. Proof and tests

- `tools/compare_graph_expressions.py`: motion 569 matched / 0 mismatches (+24 = Pre and Sustain for 12 expressions); action 171 / 0.
- New test `superman_and_one_foot_tail_guards_keep_native_reads` pins every read order and the no-read Sustain defaults.

## 7. Remaining frontier (explicit pending, unchanged)

- **FingerFlipOut:** GrabOut 828f2568/828f3e20, plus their transition guards 309/310/313/314/326/327. These are evaluated only once a GrabOut is current.
- **FootPlantAbsorb:** Absorb 828f3bf8, reached only when `IsFootPlanting` is true (an actual footplant).
- **Late flips** 253..258 (828eff60).
