# Native physics state selection

Confidence is exact for the `PHYSICS_GROUND` branch of the state selector,
its grind gate, and its ground-clearance predicate. State-specific force and
constraint updates are still incomplete.

## Reference evidence

| Address | Symbol | Established behavior |
| --- | --- | --- |
| `0x82ddbd58` | `PhysicalPlayerStateChanger::CalcSuggestedState` | updates state counters and selects the next state in a strict priority order |
| `0x82dd3ac8` | `PhysicalPlayerStateChanger::CheckForGrind` | rejects unavailable grinds, dismounts, and one-foot airs, then maps the five grind types to states |
| `0x82dd78d8` | `PhysicalPlayerStateChanger::ConditionIsOffGround` | applies the two-height, elapsed-wheel-contact test |
| `0x82da0de8` | `PhysState_PhysicsGround::Update` | performs the grounded reckoning, handplant, skeleton, pumping, board, and predicted-position updates |
| `0x82d99c30` | `PhysState_PhysicsGround::Enter` | restores standard board/collision/IK state and resets grounded timers |
| `0x82d98d68` | `PhysState_GroundAnimation::Update` | updates fixed-heading reckoning and the animated skateboard before its board step |
| `0x82d8a7b0` | `PhysState_GroundAnimation::UpdateSkateboard` | calculates a ground jump, launches its trajectory when requested, or applies grounded forces |
| `0x82b315a8` | `Skateboard::Toolkit_CalcGroundJump` | constructs the new velocity and COM-height jump result from authored curves and live physics inputs |
| `0x824e2c40` | `PointGraphEval::Evaluate` | clamps outside endpoints and linearly evaluates the selected authored segment |

The PDB definition of `ProcessedPhysIn` names every packed input consumed by
the selector. The native implementation converts those inputs to
`GroundStateFlag`, `GrindCandidate`, and ordinary scalar fields. Packed PPC
words and guest offsets are not part of the PC runtime.

## Grounded-state priority

For a current state of `PHYSICS_GROUND`, the reference checks the following in
order and returns at the first match:

1. A hippy jump with either foot absent from the deck enters
   `LANDINGONDECK`.
2. A found handplant position enters `HANDPLANT` when no wheel is touching, or
   when a physical handplant was requested with at most two wheels touching.
3. Off-board trajectory, COM-motion, and air flags select `OFFBOARD`,
   `OFFBOARD_LOCOMOTION`, and `OFFBOARD_AIR` respectively.
4. A wipeout request selects `WIPEOUT_GROUND`.
5. Exactly one frame since jump selects ordinary air, or known air when the
   trajectory selector is ready.
6. Nonzero revert input selects `REVERT_GROUND`.
7. A valid grind investigation selects its exact boardslide, 50-50, tipslide,
   5-0, or backslash state. Dismount and one-foot-air inputs suppress it.
8. Loss of all wheel/deck/truck contact plus the ground-clearance predicate
   selects `PHYSICS_AIR`, unless the teleport countdown is active.
9. An animated skateboard selects `GROUND_ANIMATION`, also gated by the
   teleport countdown.
10. An object grab with a valid grab spline selects `SKITCHING` unless the
    global disable switch is set.
11. Nonzero slide input selects `SLIDE_GROUND`; otherwise the player remains
    in `PHYSICS_GROUND`.

`GroundStateInput::suggested_state` preserves this ordering directly. Three
tests exercise priority, two-stage ground clearance/contact suppression, and
grind mapping/suppression.

## Ground-clearance tuning

`ConditionIsOffGround` reads three dynamic definitions from
`physics_airstates.default`:

| Native meaning | Exact attribute key |
| --- | --- |
| unconditional clear-ground height | `0x0fc22cb443a763ac` |
| lower height after recent contact | `0x8e8b8588435b6fe8` |
| wheel-contact grace time | `0xc6e6e6bd74dffb65` |

The predicate is:

```text
wheel_height > clear_height
OR (wheel_height > recent_contact_height
    AND time_since_wheels_touching > contact_grace_seconds)
```

All comparisons are strict. The state selector calls it only when wheel count
is zero and neither the deck nor a truck is in contact.

The supplied `skater.vlt/bin` stores all three values explicitly in the
`default` collection. Each node has flag `0x40`, so its four-byte pointer word
is inline value storage rather than a pointer fixup. The serialized words and
decoded `f32` values are:

| Attribute key | Serialized word | Value |
| --- | --- | --- |
| `0x0fc22cb443a763ac` | `0x3df5c28f` | `0.12` |
| `0x8e8b8588435b6fe8` | `0x3d75c28f` | `0.06` |
| `0xc6e6e6bd74dffb65` | `0x3e4ccccd` | `0.2` |

`AttributeCollectionEntry::inline_value_word` and the application attribute
resolver preserve the Vault's big-endian byte order. The release archive probe
loads these exact bit patterns and reports their corresponding decimal values
through the same inheritance/fixup resolver used by material and animation
settings. A focused application test guards the inline-word-to-tuning decode
independently of the external corpus.

## Remaining boundary

The ordinary ollie path enters `GROUND_ANIMATION` while the board is authored
by animation. That state's board update calls `Toolkit_CalcGroundJump`; a
successful result sets board velocity, clears linear drag, and launches and
immediately updates the trajectory selector. It otherwise applies front/rear
foot, braking, slide-friction, collision-response, and drag results.

`PointGraph<N>::evaluate` now reproduces the shared evaluator exactly, and the
archive probe resolves all five embedded `physics_jump.default` graphs through
the native VLT layout:

- 16-point `VerticalResponse`;
- 8-point `JumpYScalarVsGroundNormalY`;
- 8-point `JumpSpeedScalarVsAngle`;
- 8-point `MinHeightVsSpeed`;
- 8-point `MaxHeightVsSpeed`.

The real curves are retained as `JumpPhysicsCurves`, not copied into a source
lookup table. For example, minimum-height Y is identically `1.0`; maximum
height falls from `1.0` to `0.46`; and the angle speed scalar falls from `1.0`
to approximately `0.1357143` across its authored domain. Two unit tests cover
endpoint clamping, interpolation, and repeated coordinates.

Nine additional lookups in `Toolkit_CalcGroundJump` are dynamic class
definitions. Their original field spellings are not retained in plaintext, so
the native names below describe their proven consumer semantics rather than
inventing source names:

| Native meaning | Exact key | `physics_jump.default` value |
| --- | --- | ---: |
| height-curve reference speed | `0x97cdb6c313cd0a1b` | `27.8` |
| shared low-speed jump height | `0xa35ee33195004033` | `0.95` |
| minimum jump-height endpoint | `0xbb9e9bafba1b29e6` | `1.33` |
| maximum jump-height endpoint | `0x0b2454c666410860` | `1.71` |
| use vertical-response curve | `0xc1a016ba4ca88480` | `true` |
| minimum vertical response | `0x8d134b7861d88c9a` | `0.1` |
| horizontal compensation cap | `0xbe6c6a3ed7c6d148` | `1.5` |
| sideways (local X) adjustment scale | `0x09a6c205c73dd180` | `0.2` |
| forward (local Z) adjustment scale | `0x4eddbd6972ee9c56` | `0.18` |

These values initially appeared as nulls because the collection resolver only
followed pointer fixups. Inspection of the real collection nodes proved flag
`0x40` means a value of up to four bytes is stored directly in the serialized
pointer word. Reading that word in big-endian order reproduces all nine
settings and matches the generated runtime instance used at global collection
offset `0xd0`.

`calculate_ground_jump_vertical` now implements the fully established scalar
and vertical-vector stage. It clamps `surface_speed / 27.8`, evaluates the two
height curves, interpolates each from the `0.95` base, blends the resulting
minimum and maximum by animation jump strength, and measures current COM
height by dotting `(centre_of_mass - ground_position)` with the reckoning up
vector. The required normal speed is
`sqrt(-2 * gravity * max(target_height - current_height, 0))`. When enabled,
the ground slope is normalized as `acos(clamp(jump_normal.y, -1, 1)) * 2/pi`,
fed through `VerticalResponse`, and clamped to the `0.1` response floor. Two
unit tests cover the ballistic result, speed clamp, and response floor.

The preceding tangential stage is now recovered as
`calculate_ground_jump_tangential_velocity`. VMX128 load and arithmetic
operands are rendered 32 register names apart by IDA in this sequence; tracing
that alias resolves the formerly ambiguous vectors. The exact behavior is:

1. Reject current velocity and animation `mPrepareJumpVel` from
   `mJumpNormal`.
2. Compare their squared difference directly with `kMaxJumpError`.
3. Retain the prepared tangent when the comparison is less than or equal.
4. Otherwise normalize both safely, evaluate their angle as
   `acos(clamp(dot, -1, 1)) * 2/pi`, and feed it to
   `JumpSpeedScalarVsAngle`.
5. Replace the prepared tangent with the current tangent clamped to the
   evaluated maximum length.

The `kMaxJumpError` initializer at `0x82f7b3d8` proves a default/min/max of
`7.0 / 0.0 / 10.0`; the routine compares the default directly with squared
error. `PhysicsUtility::ClampVectorWithinMaxLength` at `0x82e49430` retains
vectors shorter than `1/65536`, otherwise scales by
`min(length, maximum_length) / length`. Two focused tests cover the retained
and corrected branches.

The final adjustment block is now recovered as source-level vector math. The
VMX word-permute immediate `0x63` selects `[y, z, x, w]`; applying the ISA's
negative multiply-subtract semantics to the two optimized cross-product
sequences proves these axes rather than merely suggesting them:

1. `body_right = normalize_safe(world_up x mBodyZAxis)`, with a zero-vector
   fallback for the degenerate case.
2. `body_forward = body_right x world_up`.
3. Evaluate `JumpYScalarVsGroundNormalY(mJumpNormal.y)`, multiply the excess
   above one by the launch velocity's world-Y component, clamp it to
   `[0, 1.5]`, and add it only to world Y.
4. Dot that compensated velocity with `mJumpNormal` to obtain the adjustment
   speed.
5. Add `body_right * -(mJumpAdjustX * 0.2 * speed)` and
   `body_forward * (mJumpAdjustZ * 0.18 * speed)`.
6. Store the new velocity minus current velocity as `mJumpDV` and return the
   normal-path `do_jump` flag.

`calculate_ground_jump` composes this with the recovered tangential and
vertical stages and retains named intermediates for tests. A flat-ground test
checks both basis signs, the capped Y compensation, the `0.2/0.18` X/Z
scales, final velocity, and delta; a degenerate-forward test verifies that
safe normalization removes only the authored X/Z adjustment. The entire
normal path of `Toolkit_CalcGroundJump` is therefore represented as readable
native behavior. Its successful `PhysState_GroundAnimation` installation is
the following boundary; the corresponding air-state update remains open.

That successful installation branch is now represented by
`GroundAnimationState`, `TrajectoryLaunchInfo`, and
`prepare_ground_animation_jump`. The PDB proves `SkateboardJumpInfo` is the
new velocity, centre-of-mass height, and `do_jump`; it also supplies all named
fields of the 272-byte launch record. At `0x82d8a7f0..0x82d8a890`, the state:

1. Copies `SkateboardJumpInfo` into `mJumpInfo`.
2. On `do_jump`, calls `SetVelocity(mNewVelocity)`, sets
   `mJumpedThisFrame`, and calls `SetLinearDrag(0)`.
3. Obtains the skeleton's `TrajectoryLaunchInfo`, overwrites `startVel` with
   the computed velocity, and sets `playerJumped = true`.
4. Calls `TrajectorySelector::Launch` and then `Update` synchronously.
5. Copies the selector's possibly revised `mLaunchInfo.startVel` into the
   state's `mJumpVel`.

The native handoff preserves that order while leaving skeleton data gathering
and trajectory selection as typed collaborators. One test verifies launch
record preservation and both state transitions; a second verifies that a
false `do_jump` retains the separate force path. `Skeleton::GetLaunchInfo` at
`0x82b46098` and the selector entry points at `0x82dc4098` / `0x82dc6528` are
captured as trajectory-recovery evidence.

### Ground-animation non-jump skateboard forces

Confidence is `exact` for the wrapper control flow at
`PhysState_GroundAnimation::UpdateSkateboard` `0x82d8a8a0..0x82d8a9cc`.
After the jump branch has been ruled out, it calculates every ordinary result
before inspecting the collision result, in this order:

| Order | Address | Exact map/PDB symbol | Caller argument |
| ---: | --- | --- | --- |
| 1 | `0x82b28b70` | `Skateboard::Toolkit_CalcFrontFootForce` | `timeSinceLanding = 1.0`, `absorption = 0.0` |
| 2 | `0x82b28d98` | `Skateboard::Toolkit_CalcRearFootForce` | `timeSinceLanding = 1.0`, `absorption = 0.0` |
| 3 | `0x82b26df0` | `Skateboard::Toolkit_CalcBraking` | none |
| 4 | `0x82b28260` | `Skateboard::ToolKit_CalcSlideFriction` | `ProcessedPhysIn::mStateTimer`, forwarded unchanged |
| 5 | `0x82b27078` | `Skateboard::Toolkit_CalcLinearDrag` | none |
| 6 | `0x82b27530` | `Skateboard::Toolkit_CalcCollisionForce` | zeroed force output, separate writable torque output, null new-velocity output |

When collision calculation succeeds, the state calls `AddSkateboardForce`
only once with force type `15` and the collision force. It neither queues the
four already-calculated ordinary forces nor calls `SetLinearDrag`. When it
fails, the state queues front foot type `4`, rear foot type `5`, slide friction
type `1`, and braking type `2`, in that order, then calls `SetLinearDrag` with
the previously calculated value.

`Skateboard::AddSkateboardForce` at `0x82b38ce0` tests the current queue count
against `0x15` independently on every insertion and silently skips that
insertion when the count is already 21. Consequently, an ordinary miss branch
starting at lengths 18, 19, or 20 retains respectively its first three, two,
or one new forces; a queue already at 21 retains none. Earlier insertions are
not rolled back.

The native implementation represents this orchestration with typed force and
force-kind values plus a collaborator boundary for the six helpers. Their
inner force, collision, and drag formulas remain unrecovered and are not
replaced with plausible skating physics. The jump path remains a separate API
and does not invoke this wrapper.

### Physics-air skateboard wrapper

Confidence is `exact` for `PhysState_PhysicsAir::UpdateSkateboard` at
`0x82d990b8`, specifically its body at `0x82d990dc..0x82d9919c`. It performs:

1. `Skateboard::UpdateSteeringTilt` (`0x82b29de0`) with exact positive zero.
2. `Skateboard::Toolkit_CalcCollisionForce` (`0x82b27530`) with a zeroed
   `SkateboardForce`, a separate uninitialized writable torque output, and a
   null new-velocity output.
3. On collision, one type-15 `AddSkateboardForce` attempt and an immediate
   return. Jump correction remains suppressed when the 21-entry queue cap
   silently drops this insertion.
4. On a miss, a signed strict comparison of
   `mFramesSinceJumpCorrect < kPhysicsAirJumpFixFrames`. A passing value is
   forwarded unchanged to `PhysState_PhysicsAir::CalcVelFromJump(int)` at
   `0x82d8af08`, and its vector result is immediately passed to
   `Skateboard::SetVelocity` (`0x82b29ee0`). Equality does not correct;
   negative frame values can pass.

The map decoration for `CalcVelFromJump` ends in `H@Z`, proving its explicit
`int` parameter. The caller loads that argument from processed input offset
`0x920`, performs the gate, and retains it in the argument register. The
callee converts the explicit value to `f32`; it does not reload offset
`0x920`. Tests therefore verify both the signed gate and exact argument
forwarding, in addition to call order, positive-zero steering, zero collision
input, the collision-only force path, and full-queue suppression.

The active CVar initializer gives `kPhysicsAirJumpFixFrames` default 12,
minimum 0, and maximum 30. The native wrapper accepts the live value rather
than embedding 12. `CalcVelFromJump`, steering, and collision calculations
remain typed collaborators. In particular, its unresolved VMX X/Z correction
is not replaced with generic air-control math.

The same evidence audit corrected three nearby processed-input masks in the
word beginning at byte `0x8c4`: skateboard contact is `0x00004000`, skeleton
contact is `0x00008000`, and skateboard-orientation-valid is `0x00080000`.
None gates the collision-helper call inside this wrapper. They belong to the
outer air update, which owns trajectory relaunch, landing-normal blend,
COM/animated-skeleton selection, and reckoning updates described next.

### Physics-air trajectory launch

Confidence is `exact` for the outer control flow of
`PhysState_PhysicsAir::LaunchTrajectorySelector` at
`0x82d99010..0x82d990b4` (`physstates:states_lib.obj`). Its locally
constructed `TrajectoryLaunchInfo` is the PDB-proven `0x110` bytes: two
`0x40` matrices at `+0x00` / `+0x40`, then eight consecutive 16-byte
`rw::math::vpu::Vector3` values at `+0x80..+0xff`, followed by `coneX`
`+0x100`, `coneZ` `+0x104`, `timeDelta` `+0x108`, `playerJumped` `+0x10c`,
`useOverride` `+0x10d`, and the `uint16_t` trajectory count at `+0x10e`.
The constructor at `0x82d83cf8..0x82d83dcc` installs the two affine identity
orientations with zero translation, zeros all four lanes of all eight vectors
and the three scalar floats, clears both booleans, and stores a zero count.
The native Rust record preserves every 16-byte vector lane rather than
collapsing the PDB type to three floats.

Two older source-level solvers still expose only XYZ. The grounded-jump
handoff and generated trajectory-fan candidates therefore use the explicitly
named `from_xyz_with_positive_zero_w` compatibility boundary. Its positive-zero
fourth lane is not recovered behavior. Candidate zero now copies the complete
launch `startVel`, and a selected candidate's complete stored vector is copied
back for a second pass, so those paths preserve lane four when it is available.

The exact call/read order is:

1. Construct the record, then call `Skeleton::GetLaunchInfo` at `0x82b46098`.
2. Read `ProcessedPhysIn::mFramesSinceJumpCorrect` (`int`, `+0x920`), then
   the live `kPhysicsAirJumpFixFrames` value. The comparison is signed and
   strict. The registered CVar initializer at `0x82f8ea18` supplies default
   12, minimum 0, and maximum 30, but the function does not embed 12.
3. When `frames < limit`, pass the exact signed frame to
   `PhysState_PhysicsAir::CalcVelFromJump(int)` (`0x82d8af08`) and replace all
   16 bytes of `startVel`. This branch does not read
   `mUseCentreOfMassVel`.
4. Otherwise, read `mUseCentreOfMassVel` at `+0xad`; only a true byte calls
   `PhysState_PhysicsAir::GetComTrajectoryLaunchInfo` at
   `0x82d84050..0x82d8418c`. That helper begins with a second
   `Skeleton::GetLaunchInfo`, so its later snapshot can replace fields from
   the first.
5. Unconditionally clear `playerJumped`, then call the public
   `TrajectorySelector::Launch` at `0x82dc4098`.

The first `Skeleton::GetLaunchInfo` population order is also exact from
`0x82b460a4..0x82b46214`:

| Order | Launch destination | Native source |
| ---: | --- | --- |
| 1 | `boardPos` `+0xc0` | processed input `+0x70` |
| 2 | `comPos` `+0xd0` | processed input `+0x250` |
| 3 | `comPosAnim` `+0xa0` | skeleton `+0x3d40` |
| 4 | `boardPosAnim` `+0xb0` | skeleton `+0x3d60` |
| 5 | `reckoningOrient` `+0x00` | reckoning transform at `+0x60` |
| 6 | `reckoningOrientInverse` `+0x40` | reckoning inverse at `+0xe0` |
| 7 | `startVel` `+0x80` | processed input `+0x190` |
| 8 | `velBeforeJump` `+0x90` | processed input `+0x2c0` |
| 9 | `timeDelta` `+0x108` | processed input `+0x93c` |

It then maps raw mask `0x00002000` in the processed-input word at `+0x8c4`
to `numTrajectories`: clear produces 1 and set produces 7. No semantic name is
assigned to that mask without stronger type evidence. Finally, in lookup
order, `physics_trajectory.default` supplies `ConeAngleX` at layout `+0x248`
from key `ff502a768f34b37c` (active raw word `41200000`, 10.0) and
`ConeAngleZ` at `+0x240` from key `5ac9f1ba8d872e1d` (active raw word
`42200000`, 40.0).

`GetComTrajectoryLaunchInfo` has a further exact orchestration boundary. It
first takes the second complete skeleton snapshot, uses fixed-step word
`3c888889`, performs still-unimplemented VMX trajectory/override work, sets
`playerJumped = true`, then samples these live CVars in order:

| Read order | Exact map symbol / storage | Registered default and range |
| ---: | --- | --- |
| 1 | `kOBA_BoardPosDelta` / `0x830d7628` | `-0.8` (`bf4ccccd`), `[-2, 2]` |
| 2 | `kOBA_TrajPosDelta` / `0x830d9960` | `-0.65` (`bf266666`), `[-2, 2]` |
| 3 | `kTrajectoryConeAngleScalar` / `0x830d9c08` | `0.4` (`3ecccccd`), `[0, 1]` |
| 4 | `kOBA_NumTrajForMount` / `0x830d6f1c` | 5, `[1, 8]` |

The first two values feed the opaque override-vector SIMD sequence. The helper
sets `useOverride = true`, multiplies the already-populated X and Z cone angles
by the third value in that order, and stores the fourth value as the trajectory
count (`0x82d84110..0x82d84180`). These direct reads/writes are `exact`; the
vector formulas between them remain unimplemented rather than inferred.

The public selector has a separate exact busy boundary at
`0x82dc40b0..0x82dc4124`: nonzero `mJustStartedQuery` (`+0x2392`) returns
without copying the supplied record or starting a query. A zero byte enters
the selector's existing reset/copy/adjust/launch sequence. The new outer
wrapper deliberately leaves both outcomes owned by the selector collaborator.

`CalcVelFromJump`, the COM helper's SIMD construction,
`Skeleton::GetLaunchInfo`, and selector internals remain explicit collaborator
boundaries: their inner math is not inferred here. Tests lock the `0x110`
layout, constructor-visible initial state, signed boundary cases, conditional
read suppression, exact four-lane/NaN bit copies, the second COM snapshot,
forced-false `playerJumped`, and final call order.

### Physics-air outer update

Confidence is `exact` for the orchestration and state decisions in
`PhysState_PhysicsAir::Update` at `0x82d9f4b0..0x82d9f824`. Vector-angle VMX
rounding, trajectory-launch internals, COM correction, COM-trajectory
multiply/add math, and skeleton/board transform internals remain collaborator
boundaries rather than inferred implementations.

The PDB type `PhysState_PhysicsAir` is `0xc0` bytes. This slice owns
`mLandingNormal` (`+0x90`, 16-byte `Vector3`), preserving all four raw
`__vector4` lanes even though the geometric meaning of lane 3 is not yet
established; `mTimeInState` (`+0xa0`,
`float`), `mMaxY` (`+0xa8`, `float`), `mUseCentreOfMassVel` (`+0xad`, `bool`),
and `mTrajectoryTimer` (`+0xb0`, `int`). It uses the PDB-proven processed-input
fields `mPhysicsState` at `+0x8e4`, `mTimeDelta` at `+0x93c`, and `mStateTimer`
at `+0x974`. In the bitfield word starting at `+0x8c4`, skeleton contact is
mask `0x00008000` and skateboard contact is mask `0x00004000`.

The exact outer order is:

1. Call `SkeletonController::UpdateAirCollision` (`0x82d9f4cc`). Load
   `mTrajectoryTimer` with zero-extending `lwz`, apply `addic. -1`, and store
   the low word. **Corrected 2026-09-26:** title code runs in 32-bit mode, so
   the record form compares the low word as signed and the gate runs whenever
   the decremented `int` is <= 0. A relaunch blocked by skeleton contact
   therefore retries every frame until contact clears. The earlier 64-bit
   reading (never retry after underflow) was wrong; see
   `air-relaunch-timer-cr0-2026-09-26.md`. Only when
   skeleton contact is clear does it read `mPhysicsState`; only when that value
   is nonzero does it read `TrajectorySelector::mJustStartedQuery` (`+0x2392`).
   If that final byte is false, call `LaunchTrajectorySelector`, whose callee
   reads the current `mUseCentreOfMassVel` byte at `0x82d99074`, and after it
   returns write timer value `7` (`0x82d9f4d0..0x82d9f51c`).
2. Always call `TrajectorySelector::Update` (`0x82d9f520..0x82d9f524`). Then
   snapshot `Reckoning::mUpVector` (`+0x1b0`), pass that snapshot as the
   fallback to `GetSuggestedLandingNormal`, store the result in
   `mLandingNormal`, and calculate the normalized angle between the selected
   normal and the same fallback snapshot (`0x82d9f528..0x82d9f5d4`).
3. Read `physics_airstates.default` key `c052d34b6b052f44`. Its supplied-corpus
   word is `3fce147b` (`1.61`). Only when the normalized angle is not ordered
   greater does the code read key `724c206c94a73043`, whose active word is
   `3dcccccd` (`0.1`); an ordered-greater angle instead selects positive zero.
   The raw branch word is `4099000c`, or `bc 4,25`: it branches when CR6's GT
   bit is clear. Because `fcmpu` clears GT for an unordered comparison, NaN in
   either operand takes the blend-lookup path.
4. Read `mStateTimer`. A value which is not ordered greater than positive zero
   forces blend to positive zero; that includes negative values, both zero
   signs, and NaN. Only an ordered-positive timer reads skateboard contact,
   and contact clears `mUseCentreOfMassVel` (`0x82d9f640..0x82d9f66c`). Its
   `40990020` branch is the same `bc 4,25`/not-GT predicate.
5. Read `kbDrawPhysicsAirDebugInfo`. Only a true value reads the debug-overlay
   singleton and reports its missing-singleton invariant. Then calculate body
   spin and call `Reckoning::UpdateAirStates(landingNormal, normalBlend,
   bodySpin, +0.0)` (`0x82d9f670..0x82d9f6d0`).
6. Read `physics_airstates.default` byte key `fd9ce5e1f36ccf2e`; its active
   supplied-corpus value is false. A nonzero live value calls
   `AddCOMCorrection`. The caller reads `mUseCentreOfMassVel` only after that
   optional call returns (`0x82d9f6d4..0x82d9f720`).
7. When the post-correction flag is true, advance the opaque state-owned COM
   trajectory with exact fixed-step word `3c888889` and pass its resulting
   position to `Skeleton::UpdateKnownAir`. Otherwise call
   `Skeleton::UpdateAnimatedSkateboard(processedInput, false)`
   (`0x82d9f724..0x82d9f7c8`).
8. Delegate to the separately recovered `UpdateSkateboard` wrapper, set
   `Skeleton::mAddSkateboardErrorToSkeleton = true`, then call
   `Skateboard::GetTransform` and sample `wAxis.y` (`0x82d9f7cc..0x82d9f7ec`).
   The maximum retains the old Y only for an ordered strict `old > new`;
   equality and unordered comparisons select the new operand, preserving its
   signed-zero or NaN bits; branch word `40990008` is again `bc 4,25`.
   Finally it stores that Y and performs the late
   `mTimeDelta + old mTimeInState` update in that operand order
   (`0x82d9f7f0..0x82d9f814`).

Typed tests cover full call order, zero-extended timer wrap and every relaunch
short circuit, the launch helper's no-argument ownership boundary, preservation of
unusual fourth-lane vector words, conditional tuning and debug reads, NaN and
signed-zero branches, contact
clearing followed by a post-correction flag reread, both skeleton paths, exact
fixed-step and flip-speed words, skateboard/error/transform ordering, and
strict/tie/unordered maximum-Y selection.

## Trajectory selector control and pass scheduling

Confidence is `high` for the selector control state, candidate-velocity
construction, ballistic helpers, collision-walk scheduling, collision-time
refinement, ground-normal construction, score orchestration, authored-grind
spatial query, dynamic-provider ordering, living-world grind-body ownership,
and single-primitive grind intersection now represented in Rust. Trajectory
adjustment and general physics-island query providers remain separate typed
boundaries. The
clustered-mesh swept-sphere backend is behaviorally complete at the geometric
contract level, but its floating-point edge cases still need differential
testing against a reference execution trace before it can be called bit-exact.

The PDB proves a seven-element prediction-future array, seven 64-byte
`TrajectoryData` records, seven scores and wall-ride flags, a copied
`PredictionResults` winner, the 272-byte launch record, and the named pass,
ready, query, targeting, and validity fields. `TrajectoryResult` is 80 bytes:
collision position, collision normal, ground normal, time, frame, and surface.
`PredictionResults` adds the 64-byte ballistic trajectory, radius, two error
values, and a draw flag. PDB layouts also prove the 48-byte `VolumeLineResult`,
64-byte `Triangle`, and 192-byte `ClusteredMeshData` records used by the
predictor's query callbacks.

### Ballistic collision predictor

The collision path through `LocalClusteredMeshTrajectoryTest` (`0x827c2270`),
`tTrajectory::WalkTrajectory<TrajectoryWalkFunctor>` (`0x827c1e58`), and
`TrajectoryWalkFunctor::operator()` (`0x827c1c78`) is now readable Rust.
`tTrajectory::Get_dTWithErrorR` (`0x827bf868`) proves the adaptive chord step
directly:

`dT = sqrt((-8 * errorRadius) / gravityY)`

This is the exact maximum deviation of a vertical-gravity parabola from its
chord. The walk uses `MaxErrorStart * radius` and `MaxErrorEnd * radius`, then
linearly interpolates those two step lengths by `time / duration`. With zero
gravity both steps equal the full duration. Successive short chords are
accumulated and no collision query is made until their squared displacement is
strictly greater than `radius^2`; the first swept-sphere hit terminates the
walk.

`tTrajectory::CalculateTimeOfCollision` (`0x827bce08`) refines the chord hit
against the parabola. Under normal gravity it starts at the previous chord
time, samples in exact `1/60`-second steps until squared distance to the hit
point begins increasing, backs up one step, and reports
`ceil(time / (1/60))`. If no increase occurs before the authored duration, the
duration is selected. The effectively zero-gravity branch uses the executable's
unusual whole-unit `floor(distance / speed)` before converting that integer to
seconds; the native implementation deliberately preserves it.

`tTrajectory::CalculateGroundNormal<Triangle,64>` (`0x827bf5a0`) gathers at
most 64 AABB-overlapping triangles. A face is eligible when `normal.y > 0.7`
or `dot(normal, landingVelocity) <= 0`. The most vertical eligible face is
selected, all eligible normals with a strict dot product above `0.5` against
that face are accumulated, and the result is normalized. An empty candidate
set returns world up.

The real defaults are loaded rather than duplicated as tuning guesses:
`TrajectoryRadius=0.2` from `physics_reckoning.default`, plus
`TrajectoryMaxTime=4`, `TrajectoryMaxErrorStart=0.25`, and
`TrajectoryMaxErrorEnd=1.2` from `physics_trajectory.default`. A native probe
over the four streamed Rob's Park simulation cells starts at
`[588.8617, -4.231647, 996.5048]` with velocity `[0,-10,0]` and gravity
`[0,-9.8,0]`; it reaches surface ID 133 at `2.3396957` seconds/frame 141 with
an up collision and ground normal. Moving the stream focus out and back still
restores the identical nearest thin-line hit.

The world adapter currently covers resident static clustered meshes plus all
three mapped primitive families. Its KD broad phase, triangle AABB cache,
continuous sphere-versus-triangle feature regions, and primitive Minkowski
expansion are implemented and unit tested. `SphereVolume` and
`BoxVolume` at `0x82ad9500` and `0x82ad7170` prove that the query radius adds to
the serialized shape radius. The rounded-cylinder path at
`0x82addf48` uses `rwcTorusLineSegIntersect` (`0x82ad8ac8`) and
`SolveQuarticRoots` (`0x82acd450`). The latter is a 50-iteration damped Newton
search from zero: after each evaluation the step is scaled by
`(50-iteration)*0.02`; iteration stops below residual `1e-6` and accepts below
`0.001`. Native cylinders now retain distinct side, cap, and toroidal-rim
regions instead of using a capsule approximation. A corpus probe casts both a
thin line and a radius-0.2 fat line through all 14,932 mapped primitives and
gets 14,932 hits in both modes with no misses or invalid parameters. Dynamic
collision islands and randomized reference differential tests remain follow-up
work, preventing the successful static-world probe from being mislabeled as
complete aggregate or bitwise query parity. The active high-LOD collision
instances sampled in Rob's Park have zero translation and already-world-space
bounds, consistent with the fused cell geometry queried here.

The public launch at `0x82dc4098` ignores a new request while
`mJustStartedQuery` is true. Otherwise it resets the pass to zero, invalidates
the best index, resets the most-vertical grind normal to world up, copies the
launch record, clears ready and grind-bonus state, adjusts the start velocity
on verts, and enters the private launch at `0x82dc2800`.
`InitTrajectoryInfo` caps the authored `numTrajectories` at seven. The private
launch creates each predictor query and sets `mJustStartedQuery`.

The optimized SIMD in `InitTrajectoryInfo` was resolved at the operation
boundary rather than copied as instruction-shaped source. Raw operand decoding
shows that IDA renders the VMX128 `VA128H` field 32 registers too high in this
sequence. Correcting that field proves the orthonormal fan basis as
`normalize(world_up x start_velocity)` followed by
`normalize(right x world_up)`. Candidate zero is the unmodified start velocity.
Candidates one through N-1 use angles starting at zero in steps of
`2*pi/(N-1)`, adding the X spread with `sin(angle)` and the forward Z spread
with `cos(angle)`.

Both spreads use `sin(cone_degrees * pi/180)`. The Z angle is first multiplied
by `ConeAngleZVsSpeed(speed * 0.05)`. The fan speed is start speed normally and
`velBeforeJump` speed after a player jump, then clamps through
`SpeedFactorMin/Max`. Each perturbed vector is normalized and rescaled by
`min(length(startVel), length(perturbed))`, proving that a candidate can lose
speed but cannot gain it. `TrajectorySelectorState` stores this seven-entry
native candidate batch before dispatch to the reconstructed predictor.

`TrajectorySelector::Update` (`0x82dc6528`) preserves the last ready result
until the outstanding future is complete. On completion it increments the
pass and chooses the first candidate whose score is strictly greater than the
running maximum, initialized to `-100000000`. This proves stable first-wins
tie behavior. The winning prediction and its `TrajectoryData` are copied into
owned buffers. If the collision time is below the authored minimum, its
collision normal becomes the suggested landing normal, ready remains false,
and the pass stops.

For an acceptable collision, the winner becomes ready immediately. A second
pass is launched only after pass one, with at least two candidates, when the
angle between the winning landing normal and current ground normal reaches the
authored threshold. AI path following and a grind-bonus winner at index zero
suppress it. `LaunchSecondPass` (`0x82dc3fe8`) replaces `startVel` with the
winning trajectory velocity, installs the authored X/Z cone angles, launches
another prediction batch, and sets pass two. The usable first-pass result
stays ready while that batch runs; when pass-two results complete,
`mHasJustChanged` is asserted for that update.

`CalculateCOMTrajectory` (`0x82dc02f8`) copies the 64-byte `tTrajectory` from
the winning prediction, then—only for a positive collision frame—moves its
initial position to the launch record's live centre of mass. It measures the
absolute separation between `mStartTrajectoryPos` and that centre of mass
along `ProcessedPhysIn::mUpVector`, evaluates
`LandingComScalarVsSlope(winningLandingNormal.y)`, and places the desired
offset along the landing normal. The difference from the trajectory's original
COM offset becomes a velocity correction.

The separately recovered `tTrajectory::AdjustTrajectory` (`0x82db6e78`)
divides that positional correction by `collisionFrame * 1/60`, clamps its
length to `MaxTrajectoryAdjust`, and adds it to the trajectory's initial
velocity. This is represented by the native `BallisticTrajectory` record and
`calculate_centre_of_mass_trajectory`; it preserves gravity and duration and
does nothing for a non-positive collision frame.

The direct portion of `CalculateTrajectoryScores` (`0x82dc5e00`) is also
recovered. A negative collision time leaves the candidate score at zero. For
a valid collision it evaluates `LandingForceScalarVsDPVelNorm` with the
absolute dot product of landing velocity and landing normal. Landing-force
score is the signed dot product times that curve and `ScoreLandingForce`;
landing-direction score is the magnitude of
`landingNormal x normalize(landingVelocity)` times the same curve and
`ScoreLandingDirection`. Candidate zero receives `ScoreMiddleBonus` only on
pass one while off vert; the debug central-trajectory switch replaces that
term with exactly `1000000`.

The final scalar order is
`landingDirection + 1 + landingForce + middle + grindOrPenalty + transition
+ LandingTimeBonus(collisionTime) + surface`. The native
`calculate_trajectory_score` preserves that addition order and exposes every
term.

The specialized terms are now separately recovered:

- `CalculateSurfaceScore` (`0x82db9dd8`) decodes physics surface type from
  packed bits 7..11. Unrideable returns `ScoreSurface_Unrideable` and replaces
  the collision normal with `ProcessedPhysIn::mUpVector`; do-not-align does
  the same with `ScoreSurface_DontAlign`; stair receives the do-not-align score
  without replacing its normal. Every other value returns zero unchanged.
- `CalculateGrindPenalty` (`0x82db9ee0`) starts at squared distance `1000000`
  and scans each 48-byte nearby-grind result. For each result it normalizes
  `end-start`, computes the collision point's axial rejection plus distances
  to both endpoints, and retains the smallest squared value before taking the
  square root. A negative collision time substitutes the zero vector; an
  invalid or empty query remains at distance `1000`. The result evaluates
  `GrindPenaltyVsDistToGrind` and is asserted non-positive by the reference.
- `PredictionResults::GetCollisionPosition` (`0x82d8a648`) is the standard
  ballistic equation `p0 + v0*t + 0.5*g*t^2`. `CalculateGrindScore`
  (`0x82dc5c38`) centers a `GrindDetectionRadius` AABB there, runs
  `GrindData::TrajectoryBoxTest` with the deck takeoff position, marks a
  nonempty result valid, and awards `ScoreGrind` only when
  `AnalyzeAndAdjustTrajectory` accepts it. The static portion of that query is
  now a concrete resident-world provider. The trajectory-specific static pair
  filter, five-line static-world geometry query, landing orientation, and every
  non-island analyzer gate are native. `DoGeometryTest` requests
  `BF_Default = Ground | Dmo | Vehicle`; vehicles are present only for a
  `FLAG_Both` island, and the aggregate keeps strictly nearer results in
  ground/DMO/vehicle order, retaining the earlier source on a tie. The
  analyzer-backed selector retries
  rejected candidates in reference order and retains the complete accepted
  geometry result. Living-world grind bodies now have concrete manager-owned
  query state; dynamic collision-island providers remain typed boundaries
  rather than guessed logic.
- `CalculateWallRideScore` (`0x82dc0558`) admits only sufficiently vertical
  contact normals, offsets the collision point along that normal by
  `WallRideTestDist`, and probes ten units downward. It rejects a related
  ground/wall-normal pair, then applies three distinct penalties for a nearby
  grind, a positive landing slope, or insufficient height. A valid wallride
  scores height times `ScoreWallrideHeight`, evaluates `WallRideBoost`, updates
  the highest wallride collision Y, and adjusts the ballistic trajectory using
  the shared `MaxTrajectoryAdjust` clamp. The physics-island line test is a
  typed callback.
- The inline vert-transition path in `CalculateTrajectoryScores` selects sign
  `+1/-1` at transition input `0.5`. A reverse direction cancels a positive
  grind bonus and substitutes `GrindPenaltyVsDistToGrind(0)`. Transition score
  is `dot(collisionNormal, normalize(horizontalGroundVelocity)) *
  (1-abs(groundNormal.y)) * sign * ScoreTransition`, plus `-500` when the
  collision is less than `MinTimeAfterApexForTransition` from the ballistic
  apex. `_GetTimeAtHighestPos` (`0x82db7198`) supplies `max(0, v0.y/-g.y)`.

`TrajectorySelectorState` expresses these rules with named flags and
`TrajectoryCandidate` values. Focused tests now cover launch guarding, fan
ordering and speed, COM adjustment, first-wins selection, direct score order,
surface mutation, grind penalty/query decisions, every wallride decision, vert
transition direction/apex behavior, and the complete two-pass readiness/change
sequence. The clustered-mesh collision predictor and static authored-grind box
query are now concrete native providers. The single-primitive candidate path
also preserves the recovered plane crossing, strict expanded-endpoint test,
closest point, direction, distance, projected velocity, landing angle, and
frame outputs. A full 120,244-segment corpus probe accepts 119,282 nonvertical
candidates, classifies 959 vertical rails, and isolates three one-ULP-scale
degenerate strict-boundary misses. `FindBestGrind` now applies the reference's
strict nearest/first-wins rule outside the centre lock and its exact unnamed
Vault graph to landing-angle improvement versus selected-height delta inside
the lock, then removes the winner for retry order. The static trajectory grind
query also applies the recovered nearly-parallel line-distance, midpoint,
vertical-preference, and takeoff-distance pair filter.

`AnalyzeAndAdjustTrajectory` now removes/retries candidates through its full
non-island acceptance tail. It strips rail-parallel landing velocity, admits
absolute vertical speed through `7`, and admits squared horizontal approach
through `36` for ledges or `64` for rails. Thin/fat lock range is centre plus
tip distance. Ledge range selects `.35/.5/.65/1` from the strict signs of the
board-centre/high-side and selected-up/high-side dot products; the low-side
branches additionally include centre distance. Every range comparison is
strict. The collision-to-rail displacement aims at the centre inside `.9` and
otherwise retains `.7` times the real deck footprint radius `.3775`. That
radius is half the sum of `physicsdeck.default` values `.59` and `.165`, loaded
by executable-proven keys `c94d65091dcb828b` and `49f429ae2e66f063` because
their source schema names are absent.

The last retry gate compares initial velocity with
`initialVelocity + displacement / continuousPenetrationTime` and rejects only
above the authored `11` degrees. Accepted displacement is then applied at the
discrete penetration frame by the already-native trajectory adjustment, so
the source intentionally exposes both time bases. The result also computes
the PDB-named landing and most-vertical grind normals: approach velocity leans
world up by `.1`, a decompiled shared clamp limits it to `20` degrees around
the geometry normal, and the vertical normal projects world up perpendicular
to the rail. Dynamic grind participation remains an explicit input.

The geometry collaborator is now narrowed substantially. PDB type data proves
`GrindGeometry` is 112 bytes: two line positions at offsets 0/16, upmost normal
at 32, primitive direction at 48, high-side direction at 64, low/high normal
limit angles at 80/84, type at 88, surface info at 92, and the stair/apex/curb/
uncertain bits at 100. `PossibleGrindOrientations` is 80 bytes with type at 0
and boardslide, tipslide, backslash, and high-side vectors at 16/32/48/64.
`CollResultsArray` is exactly five 64-byte `PhysicsUtility::CollTestResult`
records, indexed close-one, close-two, far-one, far-two, and centre; each record
contains hit, line parameter, position, normal, and surface ID at offsets
0/4/16/32/48.

`CalcGrindGeometry` (`0x82cfc2d0`) classifies no close hits as thin. Both close
hits make the geometry impossible and set apex. One close hit is a ledge only
when its corresponding far hit exists and `normal.y` is strictly below the
compiled coarse-curb parameter `.65`; equality is fat. The second-close branch
also reverses high side. `IsStair` (`0x82cf93d0`) recognizes either close hit
whose packed surface satisfies `(surface & 0xf80) == 0x400`. `IsCurb`
(`0x82cf9258`) accepts only ledges with both far hits, both normal likenesses
strictly above `.99`, a high-side projected height strictly inside `-.035..035`,
and a lower height strictly inside `-.24..-.20`. These decisions are readable
pure Rust with boundary tests.

`CalcTiltedUpmostNormal` (`0x82cf9440`) retains the upmost normal when its
angular interval contains pi, averages a wrapping interval, and otherwise
chooses the endpoint nearest pi before rotating about the rail. Landing
orientation reconstruction preserves the reference uncertain fallback and the
thin/fat/ledge direction branches. Thin rails choose the takeoff side; fat and
ledge geometry split tipslide/backslash across high side, while ledges use the
primitive/up cross for boardslide. Compiled cvar initializers prove tipslide
and backslash tilts of `20` and `42` degrees. `GetGeometryCollisionOffset`
(`0x82cfabd8`) constructs key `cbfeffea12f4cc27`; resolving it through the real
`physics_grinds.default` Vault yields `.243`. Other compiled defaults are test
height `1.06`, fine distance/depth `.09/.07`, coarse curb `.65`, wheel-clear
angle `16`, normal likeness `.99`, average curb height/range `.22/.02`, and
high-side range `.035`.

`DoGeometryTest` (`0x82cfac60`) is now source-shaped as one typed batch. It
asserts normal and rail-direction magnitudes strictly inside `.98..1.02`, then
forms high side as `railDirection x up`. Lines run high-to-low in exact result
order: close high side, close low side, far high side, far low side, and centre.
The two far outputs use their low endpoints when no hit exists and collision
positions otherwise. `CalcGrindGeometry` projects the requested grind point
onto the rail, derives up by rejecting rail direction from world up, and uses a
`1e-5` projected-up threshold for its near-vertical uncertain escape. That
escape writes `grindPosition +/- X * .243`, chooses world Z as upmost, and
forces polished-concrete/smooth surface defaults.

After the batch, both far-low vectors are normalized from the projected rail
point. Low limit is `acos(dot(-up, farOne)) + pi/2 + wheelClear`; high limit is
`2*pi - acos(dot(-up, farTwo)) - pi/2 - wheelClear`. The implementation clamps
each cosine to `[-1,1]` and preserves the reference's asserted nonzero vector
requirement. `ResolveSurfaceInfo` (`0x82cfb380`) is complete: curb forces raw
audio/physics `4/2` (concrete rough/rough); thin forces slippery physics `4`
and chooses centre-hit audio or metal-solid-round-one `11`; other geometry
prefers the higher close hit, then higher far hit, then centre, with ties going
to the second side, and falls back to polished concrete/smooth `3/1`.

The resident static collision scene now implements the batch provider by
retaining the nearest hit for each line. An optimized Rob's Park corpus run has
zero simulation failures and classifies a real resident authored rail as fat,
with rough-concrete surface and normal interval `3.4271252..3.5946743`. Dynamic
rail queries now transform authored segment AABBs and endpoints through each
live body's affine columns. `Dmo::SimManager` recomputes the vertical flag in
world space; the two living-world managers instead copy that high bit from an
uninitialized stack word. The native scene makes this retail defect a
trace-fed compatibility bit with a deterministic false default. It preserves
manager hash/map order as supplied, the enabled-by-default DMO gate, both
15-unit size cvars, strict radius-boundary rejection, DMO-before-vehicle order,
and provider-local shared-cap exits. Populating those bodies from streamed
entities is no longer an opaque asset problem: `Vehicle::InitSplines` and
`DynamicObject::InitSplines` read the model instance's embedded grind/grab
arrays into a 64-entry per-body map. The decoded `livingworld.big` bank proves
32 grind-bearing vehicle component arenas across 16 named schemas. All 194
embedded header references resolve and build 564 native primitives, with only
2 through 9 headers on any body. Runtime recipe/LOD selection and attaching
those vehicle templates to live transforms remain open. Streamed DMO records prove a
separate registry-GUID-to-local-model-ID layer. The dedicated DMO bank is now
decoded: 93 `Instance_DMO` models, 77 embedded grind bodies, 470 exact headers,
and 700 native primitives. Eighty-seven of BAM's 99 locator model IDs resolve;
the other 12 are named missing assets in this debug corpus. In the four-cell
Rob's Park focus all 249 resident locators resolve; 168 grind-bearing catalog
entries contain 1,430 assets / 2,361 transformed primitives. The collision
ownership chain resolves all 93 bank models through `tCModelData -> Assembly
(0x00040006) -> AssemblyDefinition (0x00040007) ->
PartDefinition.volume`, yielding 14,451 template primitives. All 249 catalog
locators can instantiate collision bodies/parts and 35,626 primitives.

They are no longer conflated with live bodies. Offline `Island::InitWorld`
creates one global 50-assembly DMO pool with its own octree-backed line batch;
only spawned collision groups 14 through 16 enter it through `AddDMOBody`.
The real VLT dynamic-object census endpoints are both no-spawn/spawn/cull/
shift/speed `0/60/75/0/0`, and no Rob's Park resident locator is inside the
60-unit spawn circle. The active DMO pool at that focus is therefore exactly
empty. A separately isolated, owner-strict catalog probe finds one authored
grind whose same locator/model collision body supplies three five-line hits
and surface values `30/1`; it cannot be satisfied by an unrelated resident
body. `BF_Default` selects ground, DMO, and conditionally vehicle pools, then
strictly-nearest line parameters are merged in that order. The model-specific
priority values, global priority weights, five-field score construction, and
strict live-queue comparison are now reconstructed. The exact primitive-overlap
pipeline, DMO priority rejection, and ordered DMO/vehicle/lowest-pool cull
policy are also reconstructed. Application-owned live collision bodies now
feed the exact DMO and vehicle owner lists into that policy. Runtime owner-ID
packing/allocation is now exact: one shared living-world counter initialized to
1 supplies the low 24 bits, while a DMO installs type byte 5. Cull requests
immediately detach once and defer destruction to the next census update.
Integrating that allocator and lifecycle in the original cross-entity
construction order, supplying the reference DMO-store base for pointer-hash
destruction ordering, and live vehicle attachment remain open. The typed
application census already composes physical admission, ordered DMO/vehicle/
lowest-pool cull requests, deferred destruction, and exact fixed-slot reuse.
VMX reciprocal/trigonometric rounding still needs reference differentials
before any bit-exact numeric claim.

The PDB layout for `Attrib::Gen::physics_trajectory::_LayoutStruct` supplies
the original authored names for every graph and scalar. The native VLT loaders
now also resolve the complete 14-scalar grind block (tip/offset, speed gates,
lock distances/scalars, landing velocity, and landing/adjustment angles), in
addition to the 26 named values used by initialization, COM, direct score,
specialized score, wallride, and completion paths from
`physics_trajectory.default`, plus two behavior-identified 0.5 gates from
`physics_feet.default`. Their generated schema names are absent from the PDB,
so their exact keys (`baba91606102af9a` and `90c4e79cbfbaed5e`) are retained
with semantic field names rather than invented original names.
`ConeAngleZVsSpeed` has X samples
`[0, .228013, .3859935, .4739414, .5895765, .7557003, .8583062, .9967427]`
and Y samples
`[.3513514, .3614865, .3783784, .4087838, .4560811, .527027, .5608108,
.5608108]`; `SpeedFactorMin/Max = 2/4`,
`LandingComScalarVsSlope` has X samples
`[-1, -.4299674, -.257329, 0, .0586319, .1140065, .247557, 1]` and Y samples
`[.9428571, .7928572, .7035714, .4535714, .8678572, .95, 1, 1]`,
`MaxTrajectoryAdjust = 1`,
`LandingTimeBonus` rises from `0` to `1000` across its eight samples,
`LandingForceScalarVsDPVelNorm` rises from `0` to `1`, and
`ScoreMiddleBonus/ScoreLandingForce/ScoreLandingDirection = 40/20/200`,
`ScoreGrind = 1000000`, `GrindDetectionRadius = 2`,
`ScoreSurface_Unrideable/ScoreSurface_DontAlign = -200000/-100000`,
`ScoreTransition = 500`, `MinTimeAfterApexForTransition = 0.15`,
`MinTimeForTrajectoryToBeValid = 0.25`,
`MinNormalDeltaForSecondPass = 0.1`, and the second-pass X/Z cone angles are
`5/15` degrees. Cone angles are converted inside trajectory initialization;
the normal-delta threshold is compared with the radian result of
`AngleBetweenVectors`.

`GrindPenaltyVsDistToGrind` has X samples
`[0, .03298048, .06376221, .1150652, .1876222, .2902279, .3818404, .45]`
and Y samples
`[-1000, -764.286, -610.714, -428.5714, -278.5714, -142.8571, -50.00001, 0]`.
Wallride data is `WallRideTestDist = .05`,
`WallRideMinHeightFromGround = .6`, `WallRideAngleToAllowLanding = 0`, and
`ScoreWallrideHeight = 500`. `WallRideBoost` has X samples
`[.6, .6319218, .6983714, 1]` and Y samples
`[.4960938, .3359375, .1484375, .00390625]`.
