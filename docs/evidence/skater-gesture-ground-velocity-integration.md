# Original gestures, wheel ground position and direct velocity writes

2026-09-08. Root integration; independent code/source reviews.
The selected paths below are source-backed. They do not constitute a playable
Actor or a deterministic comparison with the original game's per-tick trace.

## Command-owned recognition

`game_input_player::recognizer` now retains two histories and seven matchers on
the SAME command-updated gameplay owner. It reads map78 then77, completes the
right-stick update, then reads75 then74 and completes the left. Each history
update samples its deadzone target/value once, preserves ordered comparisons,
then runs all held checks before all new-match checks for that stick.

The original matcher is reused. Successful matches go through the source's
shared display-reference service and conditional local-player/debug attribute
lookups before message insertion. Both optional input-display stages follow
both stick updates. These enabled service branches are explicit collaborators,
not silently disabled native behavior. Histories/matchers are a selected late
construction stage; full native PlayerInput construction chronology is pending.
See [recognizer source proof](game-input-recognizer-runtime-plan.md).

`game_input_recognizer_corpus.rs` mounts input.cfg and all seven original pattern
files and consumes the actual supplied Vault (class `7f086875d621c88c`). It feeds
the first original Ollie's authored endpoints through the actual command codec
and registered player. Start/start/centre/end yields the original pattern
occurrence with time3, another endpoint produces held, and centre releases it.
The actual cached right-stick waiting byte is **10**. Cvars, port and sample
schedule remain explicit diagnostics. This proves message production, not
Actor event delivery or a physical jump; Ollie held-message consumption must
follow the actual listener, not an invented direct jump mapping.

## Ground position

`skateboard_calculate_ground_position.rs` implements `82b2f1b0..82b2f3e8`:
same retained Reckoning ground basis, zero output, ascending raw body positions
of Parts0..3, transpose/transform, left-associated four-point average,
asymmetric minimum-height selection and final basis transform. Native full
basis W survives, with no body-W, terrain-query, radius or translation substitute.
The same-board bridge requires the actual completed Reckoning occurrence.
See [ground source proof](skater-calculate-ground-position-runtime-plan.md).

The original moving board produces `[588.6685, -54.45357, 996.48944]`, W=0,
agreeing with an independent f64 projection/height/rotation oracle. This is
the selected calculation. HiLOD +d0/+e0 still need actual category, retained
jump-counter and intervening effective-transform stages.

## Direct velocity replacement

`skateboard_set_velocity.rs` implements Body `82b244d8..82b24528`, reached by
Skateboard `82b29ee0`. Every actual Part reloads the borrowed source velocity
before body resolution and commits immediately. Count is checked again after
each write. The narrow pool setter changes only XYZ; native velocity W is an
intrusive link stored separately in the host and remains intact. No force,
omega, activation, cooldown, drag or native timing side effect is added.
See [SetVelocity raw proof and host policy](skateboard-set-velocity-runtime-plan.md).

After the first successful write the host pool's frame identity becomes
unavailable until the next actual frame begins. This invalidates stale solved
certificates without changing timestep/frequency or historical output numbers.
The original corpus checks all seven XYZ writes, stale spies/motion/COM refusal,
preserved prior output, then a real ground solve and recovered motion sampling.
Its supplied velocity is the prior actual deck velocity, explicitly a diagnostic
setter input, not a generated active jump.

A private cfg(test) helper temporarily hides Part2's body reference, invokes
the actual setter, restores the same reference, and checks that only Parts0/1
committed. It never removes/replaces bodies or adds a production fault API.

## Validation and remaining work

Core **56319/049271**: 3,202 units pass. Original **34962/3be28d**: all twelve
checks pass, including the original Ollie and actual next-solve recovery.
Workspace **2180/4f87e8** passes 3,445 units and six docs. These runs precede
the final private late-Part fault helper integration. Current strict Clippy,
its original-data rerun and fmt/diff checkpoint were pending at that intermediate
point. Final strict Clippy **16520/cdca2b** and all twelve original-data checks
**68907/2b18ba** now pass, including the restored Part2 failure fixture.
Subsequent controller ownership/helper edits are a separate unaccepted batch.

Next work is actual listener/Actor delivery and the genuine retained physics
state/startup path. The source starts Sleeping and transitions into PhysicsGround;
controller hand-drive admission/Stop, state Enter/Exit, active jump trajectory
and final animation/physics scheduling remain real dependencies. Broad DMO
work stays parked and the offline playable goal stays active.
