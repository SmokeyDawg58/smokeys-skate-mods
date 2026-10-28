# Fuzzed input: crash fixes (October 8)

Deliberately erratic input in fakie broke the simulation:

```
diagnostic grounded root assertion AnimationToBoard: reckoning up=SkeletonRootVector { xyz: [NaN, NaN, NaN] ...
controlled skating test failed: controlled tick after 26366 completed frames:
Failed { stage: QueuedFrame, detail: "Tick(Before(Skateboard(Update(Update(Roots(Assertion(AnimationToBoard)))))))" }
```

## Fuzz driver

- `SKATE2_DIAGNOSTIC_FUZZ=<seed>` (apps/skate2-pc/src/input_fuzz.rs) replaces
  the controlled pad sample every tick.
- Each channel re-rolls on its own timer:
  - left stick: 8–90 ticks;
  - right stick: 1–14 ticks;
  - triggers: 10–80 ticks;
  - buttons (A, X, B, LB, RB, Y): 2–40 ticks.
- It never presses Start, Back or the D-pad.
- Long runs are not deterministic (streaming and threads). Seed 4 failed at
  frame 30801 once; two replays then passed frame 38000.

## AnimationToBoard NaN (the reported crash)

### Signature

- Both fuzz repros (city seed 3 at frame 37763, seed 4 at 30801) had the
  board at y ≈ −61.25 near (830, 935–996). That spot is flat ground and the
  skater had settled.
- `up_target` = `ground_normal` ≈ (0, 1, 0), with denormal or tiny x and z
  (4e-45, −1.55e-32).
- `up_speed` and `up` were NaN; every input was finite.

### Cause

1. The grounded up spring (`try_update_grounded_fixed_heading_read`) clamps
   its acceleration with `try_clamp` → `try_length` → `host_dot3`.
2. Once the spring settles, the acceleration's components are about 1e-20.
   Its squared length is about 1e-40, which is a denormal.
3. The host rsqrt seed is 1e20. Its square overflows to inf, so the two-step
   refinement gives inf − inf = NaN.
4. `squared != 0`, so the exact-zero select does not apply.
5. The NaN length fails `length < SMALL_FLOAT`, then multiplies the vector,
   so `up_speed += NaN`.

### Native

- `ClampVectorWithinMaxLength` 0x82e49430 takes the squared length from
  `vmsum3fp128`, then `vcmpeqfp128`/`vsel` select length 0 for a zero square.
- The title never writes VSCR (no `mtvscr`/`mfvscr` anywhere in the xex), so
  VMX runs in the console's non-Java mode. That flushes the denormal square
  to +0.
- The length is therefore 0 and the vector comes back unchanged.

### Fix

- `vmx_flush`, `host_dot3` and `host_dot4` (physics/reckoning/reckoning_numeric.rs)
  now treat denormal operands and results as signed zero.
- `HostSkeletonCollisionNumeric::dot3` now uses `host_dot3`. Its
  `try_clamp_xyz` ports the same native function.

### Tests

- `denormal_squared_length_is_zero_so_clamp_returns_the_settled_vector`
  (reckoning_numeric_tests.rs).
- The settled-vector rows added to
  `host_numeric_policy_is_explicit_left_association_and_unmodified_estimates`
  (skeleton_collision_numeric_tests.rs).

### Verification

- parity.sh: IDENTICAL (6000 lines), both after the flush and on the final
  build with the action fix.
- City seed 3 on the fixed build ran 60000 frames.
  - It logged the exact case five times and carried on with no NaN.
  - Example: acceleration (0, 0, −1.85e-20), squared length 3.4e-40, with
    `up_target` holding denormal lanes (4e-45).
  - Before the fix, the same seed died with this NaN at frame 37763.

### Scope

Private host dots elsewhere still keep denormals:

- skateboard spawn math;
- the teleport matrix;
- skeleton physics history.

## Action expression 52 (found while verifying)

- `Tick(Actor(Action(Update(PendingExpression(52)))))` at frame 19809, seed 4.
- Native Pre case 52 (`EvaluateExpression<1>` 0x828ff990 → 0x82901a68)
  returns NOT (HasAGIntent RightAirGrab, condition 64, AND HasAGIntent
  LeftAirGrab, condition 65). Condition 65 is read only while 64 holds.
- Native Sustain (0x82906048, table 0x82906080) sends 49 and 52–55 to the
  default at 0x8290a90c: true, with no reads.
- Fix: generated_action_runtime_selection.rs.
- Test: `not_both_air_grabs_reads_left_grab_only_while_right_is_held`.

## Earlier fuzz finds (same session)

- **FingerFlipOut, PendingCallback.**
  - CreateInstance 0x82712148 (0x0c, float +8 zeroed), Begin 0x82cbea20,
    Update 0x82cca470, End BLR 0x825187c0.
  - The timer winds down while the grab intent is held, otherwise up. It is
    clamped to `anim_motion_finger_flip` +0x140/+0x148 and fed through the
    eight-point graph (X +0x150, Y +0x170).
  - The result goes to `AddSettableAtt` (normalized false, sequence −1).
  - Code: callbacks/actor_motion_finger_flip_out.rs.
- **GrabOut, PendingStateUpdate.** Registry rows were added for:
  - 967/969/979/981/983/985 at 0x828f2568;
  - 1235/1246 at 0x828f3e20.
- **Unregistered states.** Registry rows were added for 34 states, among them
  TailManual Brake Stationary1330 (0x828f4cc8). All rows were expanded from
  the native StateUpdate slots.
- **PendingExpression.**
  - Pre evaluator cases 309/310, 313/314, 316/317, 320/321, 326/327 and
    332/333.
  - Cases 280–282.
  - Case 24, which shares 29's `HasIntent(OB_Mount)` (0x8290ba94).
- **Sustain.** 280/282/371/372 take the evaluator default (true, 0x829221c8).
- **HandsFace pose stream, FixedSqtId.**
  - `SetSQTById` 0x82a2c6a0 sets ACS error 4 for an id at or past
    `mFixedSQTs->mMax_Num`.
  - Each later command is skipped, and `EndCommands` 0x82a289e0 rolls the
    block back.
  - `StreamCommands` now does the same (hands_face/actor_animation_hand_face_commands.rs).
