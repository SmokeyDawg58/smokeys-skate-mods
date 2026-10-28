# Final Physics conditioner call

Confidence: high source reconstruction; root independent review/gates pending.
Author output_continuation, new unregistered files only, direct rustfmt passed.
No Cargo, shared edits, or native trajectory parity claim.

`Sk8::Physics::PhysOutConditioner_Physics::WatchForStairs`82e2cb70 is the
seventh and last call of Physics.Update82e2f970 at82e2f9b4. It follows actual
CalcLandingQuality. New `try_finish_physics_update` consumes PhysicsBeforeStairs,
validates its output and both retained Conditioners/Physics bindings, executes
WatchForStairs on that same output, then returns PhysicsUpdateComplete.
It changes no Physics history. This completes the seven-call Physics owner
only after all preceding actual tokens; it does not complete UpdateAll or Actor.
The earlier Physics callbacks retain their explicit unsupported-state errors.

## Source reads, branches and stores

1. Output capturecb88, guardcb8c/assertcba4. State.physics_state loadcbb0,
   compare100cbb4/branchcbb8. Other states return; state101..104 do not enter.
2. Fresh outputcbbc/guardcbc0/assertcbd8, state manual+33 bytecbe4. True returns.
3. Collision gettercbf4, current collision surface+10 readcbf8, compare8 Stair.
   Other surfaces return. No surface table lookup or grind owner involved.
4. Motion gettercc08 guard, deck_speed+a0 loadcc10 BEFORE MinSpeed targetcc18
   and live floatcc1c. Rawcc244099007c exits on !GT(speed,minimum), including unordered.
5. Board gettercc2c guard, actual transform.zAxis.y+24 loadcc34, fabscc38 BEFORE
   NoseUp targetcc40 and floatcc44. Rawcc4c40990054 exits on !GT(abs(y),maximum),
   including unordered. The source uses actual transform, not effective transform.
6. Animation gettercc54 guard, set should_disallow_tricks_due_to_stairs+a2=true
   atcc60. This flag is never cleared by WatchForStairs when an earlier test exits.
7. Mandatory actual DebugOverlay singleton Getcc84 after the flag store.
   Missing service produces a checked failure retaining the flag. Green and
   text `No Trick On Stairs!` plus450/450/1 are debug arguments; final callcc9c
   targets825187c0, whose fresh raw word4e800020 is BLR. No debug draw implementation
   is missing here and no fake service is inserted. Actual singleton still required.

Native decompiler emits `speed > threshold && abs(y) > threshold`, which loses
both unordered branch continuations. Implementation uses the raw BLE exits.
All output fields already exist, as do actual State/Collision/Motion/Board.Fill
producers. Existing cvar80 kGroundStairsFrames belongs to the earlier filtered
state callback; this final method neither reads80 nor alters its frame history.

## Two new live references220/221

Both section `Physics_Conditioners_Physics`, minimum0 and null callback. Fresh
raw initializer loads prove the exact defaults and r10=0 before each constructor.
Actual table dwords82fd647c and82fd6480 point to listed initializers.

|index|label|title|defaultbits|max|initializer|ctorcall|table|reference|
|---|---|---|---|---|---|---|---|---|
|220|kMaxStairTrickDeckNoseUp|Max Stair Trick Deck Nose Up|3e4ccccd|1|82fa07d0|82fa0810|82fd647c|8311e734|
|221|kMinStairTrickSpeed|Min Stair Trick Speed|3fc00000|5|82fa0830|82fa0870|82fd6480|8311e570|

Root appends these definitions at reserved220/221 using Value::Float(from_bits).
Insert220,221 after212(table6478) and before31(table6494) in selected initializer
order, respecting any newly registered intervening native table positions.
Runtime read order is221 then220. Values remain live via existing current-float
target capture/read path; metadata ranges are not invented clamps.

## Root registration and session continuation

In physical_player_phys_out_conditioners.rs:

```rust
#[path = "physical_player_physics_stairs.rs"]
pub(crate) mod stairs;
```

In physical_player_phys_out_construct.rs:

```rust
#[path = "physical_player_physics_stairs_output.rs"]
pub(crate) mod stairs;
```

In input_cvar_runtime.rs:

```rust
#[path = "physical_player_physics_stairs_cvars.rs"]
mod stairs;
```

After actual landing quality call:

```rust
conditioners.try_finish_physics_update(before, output, cvars, overlay)
```

Returns `Result<PhysicsUpdateComplete, construct::stairs::StairsError>`.
The token binds this actual output. Do not rerun earlier callbacks to synthesize
a prerequisite. Mount into the actual selected session and retain the existing
explicit parked-grind policy; outer UpdateAll remains separately incomplete.

Five tests authored: early return/lazy cvars/stale flag persistence; equality
at both thresholds; flag surviving missing overlay; NaN continuation with actual
overlay; complete actual seven-call token chain on synthetic constructed owners.
Tests remain unexecuted by author, and their fixtures do not establish reference
per-tick parity. Root independent source review and full gates are required.
