# Animation-to-physics handoff

Confidence is exact for layout, reset defaults, transfer order, fixed step,
trick flags, the 99 distinct scalar/state keys, and type-3 push/brake/hand
contact interpretation. Downstream board/body simulation is not yet
reconstructed.

## Reference ownership and order

| Address | Symbol | Established behavior |
| --- | --- | --- |
| `0x825ecbc0` | `Sk8::Physics::AnimOutPhysIn::Reset` | clears per-frame state, selects `1/60` seconds and compression `0.5`, identity-initializes both pose buffers |
| `0x82c96d58` | `Sk8::Skater::SkaterAnim::GetPhysUpdateData` | verifies bone count; transfers local/global poses, trajectory/COM/foot targets, stance, and locomotion state |
| `0x825f7e50` | `Sk8::Skater::SkaterMotionGraph::GetPhysUpdateData` | transfers trick/gesture/wipeout output and resets the source one-shot values |
| `0x825ff6c0` | `Sk8::Actor::SetUpPhysics` | resets, gathers animation and motion-graph data, merges attributes/settings, and submits the handoff |
| `0x82dfe688` | `Sk8::Physics::PhysicalPlayerHiLOD::SetPhysUpdateData` | retains the completed input for the physics update |
| `0x825ffd98` | `Sk8::SkaterManager::UpdatePhysics` | runs setup and update at fixed `0.016666668` seconds |
| `0x82b59de8` | `Sk8::Physics::Skeleton::ProcessAnimAttributes` | converts named animation attributes into semantic physics inputs |
| `0x82b39098` | `Sk8::Physics::ProcessedPhysIn::Reset` | resets per-step working state and its exact scalar/state defaults |
| `0x82e01f98` | `Sk8::Physics::PhysicalPlayerHiLOD::Update1_ProcessInput` | resets working state, gathers animation/external/current-physics inputs, and prepares state logic |
| `0x82e00030` | `Sk8::Physics::PhysicalPlayerHiLOD::ProcessAnimOut` | copies time, compression, stance, board settings, and external state into working input |

The exact `Actor::SetUpPhysics` order is:

1. reset animation output/physics input;
2. gather pose and stance/state from skater animation;
3. transfer motion-graph trick, gesture, wipeout, and intents;
4. clear the destination attribute list, then add motion-graph attributes and
   animation attributes;
5. add living-world impulse, AI/reset state, and player board settings;
6. submit the completed input to the physical player.

## PDB layout evidence

`Sk8::Physics::AnimOutPhysIn` is 10,928 bytes in the reference ABI. It begins
with pointers to paired global and local pose buffers, followed by motion-graph
intents and a fixed attribute collection. Later fields include stance and
locomotion flags, time delta, compression, trick/gesture/wipeout values,
external impulses and reset state, board settings, COM and foot targets, and
foot-IK values. The final packed word carries collide-world-only, running-out,
off-board-jump, wipeout, air-dismount, and locomotion flags.

`AnimOutTrickData` is 60 bytes. Its two `FastString<36>` values are trick and
gesture names; the packed flags name switching, landing, ride-idle, wipeout,
trick, grabbing, two manuals, two reverts, and two powerslides; the last two
floats are tweak X/Y. `Reset` invalidates the names and clears flags/tweaks.

The Rust runtime deliberately does not preserve these offsets. It owns
`AnimationPhysicsInput`, `AnimationTrickData`, and `MotionGraphOutput`, uses
vectors for pose ownership, and uses named enum sets for flags. Attributes are
also an ordered vector: the reference fixed vector permits duplicate names,
and later matches overwrite earlier semantic fields. This is the
source-reconstruction equivalent of the proven behavior, not a serialized
console-structure clone.

`ProcessedPhysIn` is the following 2,608-byte reference working state. Its
reset selects `1/60` seconds, gravity `-9.8`, compression zero, flipped-control
scalar one, invalid physics/category/filter states, no grind, 100 frames since
jump, and 1,000 frames since corrected jump. Native `ProcessedPhysicsInput`
captures these proven semantic defaults and the first `ProcessAnimOut`
transfers. The complete vector/contact/surface and state-machine payload is
still pending.

## Animation attribute interpretation

The physics skeleton iterates animation attributes by exact `FastString<30>`
name. `tools/ida/dump_physics_attribute_keys.py` follows the PDB-named dynamic
initializers and records 104 definitions in
`artifacts/physics-animation-attribute-keys.json`. Cross-checking every
comparison in `Skeleton::ProcessAnimAttributes` identifies 99 distinct
scalar/state keys after excluding the unrelated `GlobalAtt` reference. All 99
are now represented by named fields or flags in the native interpreter.

Representative mappings include:

| Key | Native semantic result |
| --- | --- |
| `Spin`, `BodySpin`, `Turn`, `OB_Turn`, `RawTurn`, `HardTurn`, `Slide`, `Balance` | scalar copied with reference sign inversion |
| `Brake` | foot-brake scalar copied directly |
| `ManualBrake`, `Wipeout`, `PrepareJump`, `FlipTrick`, `Grabbing` | presence sets a named intent flag |
| `TrickHeight` | retains the authored jump-height scalar |
| `SlideBoard`, `SlideNose`, `SlideTail`, `Grind5050`, `Grind5_O`, `GrindNose` | retains the exact grind animation identity |
| `OBL_SteerMagnitude` | sets steering only when the authored scalar is strictly greater than `0.2` |
| `RevertDir` and revert presence keys | retains authored direction; absent direction defaults to `1` |
| `jump` | exact lowercase spelling sets the jump intent |

The mapped set covers scalar controls and presence states for jump/revert,
flips, grabs, coffin, grind exits, hand plants, board transitions, off-board
movement and board handling, world grabs, object movement, and wipeout
controls. Exact sign inversion occurs only for the eight fields listed above;
other scalar axes retain authored sign. Source order and duplicates are
preserved, so grind identity, revert direction, and repeated scalar values use
the reference's last-match behavior.

Type 3 carries a 24-byte contact record: a packed `FastString<30>` bone name
plus a final scalar. The six exact initializer strings are `push_contact`,
`brake_contact`, `right_hand_grab`, `left_hand_grab`, `righthand`, and lowercase
`jump`. Andale now parses, collects, blends, and decodes these values, and the
native handoff forwards them as `ContactAnimationAttribute` rather than opaque
bytes.

The complete `OnBoard.abin` inventory contains 864 type-3 records. Push and
brake contacts use `lefttoebase`/`righttoebase` and values `-1`, `0`, or `1`;
hand grabs use `lefthand`/`righthand` and values `0` or `1`. Physics parameter
slot 19 is authoritatively `righttoebase`, closing the reference's push-foot
comparison. Processing preserves exact floating-point equality for the `1`
and `-1` transition bits, derives push velocity from trajectory translation
length divided by the animation step, and accumulates left/right hand state by
the payload bone exactly as the consumer does.

## Jump-strength postprocessing

The tail of `Skeleton::ProcessAnimAttributes` applies three settings from
`anim_motion.jumping` after the ordered attribute loop. Their exact keys are:

| Meaning | Attribute key |
| --- | --- |
| enable endpoint handling | `0xb52e6ee4f322a9f4` |
| lower endpoint | `0x982f499ec97735a9` |
| upper endpoint | `0xa1f65bb8c4d1ba8b` |

When lowercase `jump` is present and endpoint handling is disabled, jump
strength is the authored `TrickHeight`. When enabled, a height at or below the
lower endpoint selects `MinJump`, a height at or above the upper endpoint
selects `1.0`, and an interior height remains `TrickHeight`. Presence of
`HippyJumping` then overrides the result back to `TrickHeight`.

The supplied `anim_motion.jumping` collection resolves all three settings
through the reference null-value fallback, yielding `false / 0.0 / 0.0`.
`AnimationPhysicsTuning` and `apply_jump_strength` preserve the exact branch
and strictness without embedding attribute-vault access in the gameplay type.
The archive probe loads the real collection and exercises the native handoff.

## Current validation

The archive probe recursively evaluates the real
`b_antic_360shuvit_cyc` controller, overlays its 36 body transforms on the
148-bone skater hierarchy, creates both local and global matrices, and installs
them into a reset 148-bone `AnimationPhysicsInput`. A named one-shot trick is
then drained from `MotionGraphOutput`; the source becomes empty and the input
retains `r_antic_360shuvit_l_0_cyc` with the `Trick` flag. The same recursive
route collects its runtime attributes and forwards scalar values into the
physics interpreter. Unit tests cover reset identity/defaults, count rejection,
one-shot transfer, local matrix exposure, the complete 99-key scalar/state
mapping, duplicate ordering and thresholds, type-3 contact decoding and
trajectory-derived push speed, jump-strength endpoints and hippy override, and
processed-input reset plus handoff. The workspace checkpoint contains 135
passing tests.

This validates the native seam and jump-strength handoff only. It does not yet
claim that forces, contacts, the full state machine, or trick scoring have
reference parity. The first exact grounded selector slice is documented in
`docs/evidence/physics-state-machine.md`.
