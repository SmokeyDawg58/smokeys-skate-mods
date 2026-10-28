# Actual BoardFill surface continuation

The registered `physical_player_board_fill_surface.rs` adapter continues the
same owner token after deck-line/acceleration publication, reusing the existing
`PhysicalPlayerPhysOut::try_publish_surface_motion` leaf for 82b2fdd8..82b3004c.
It stops before the first SpeedWobble read at 82b30050. This does not complete
BoardFill, Update5, or the intervening simulation frame.

## Retained reads and ordering

The initial output assertion fdd8..fdf4 precedes the native owner captures:
Reckoning fe00 and Processed fe0c. Ground normal is Reckoning+1f0 (fe1c),
published deck velocity is Motion+50 (fe24), and deck Z is Processed+60 (fe48).
The discarded NormalizeSafe result at fe60 does not replace the projected deck
Z. Direction selection is ordered dot > 0, with zero and unordered selecting
the sign-flipped projection. Reckoning is freshly captured at fed4, and its
ground transform+20 supplies the signed-angle axis. The caller uses actual
retained owners and typed vectors; unknown W remains unknown through retained
projection and copies. It does not require a ContactSpyFrame or a completed
whole Reckoning phase absent from these native direct reads.

The existing selected, stricter session wrapper remains separate. The new
adapter reads Processed.deck_transform directly, rather than requiring the
unrelated toolkit completion marker. It never replaces missing values.

## Local numeric review

Fresh raw surface speed instructions 82b30008 (15a30971, VA35) and 82b30018
(15820171, VA34) are VNMSUB using retained squared length. Their graph has
negative-fused residual `-squared.mul_add(inverse_square, -1.0)`, followed by
the corresponding Newton addition twice, with zero selection only afterward.
This matches the independently accepted local initial board-speed graph.
The surface leaf now reuses `board_speed_from_squared_and_estimate`, without
changing shared numeric policy. Parent independently accepted the correction
from these raw words and the preceding board-speed graph. The existing local
helper regression distinguishes the negative-zero residual at exact product one.
Registration and correction await the next parent-owned Cargo gate.

## Actual deck-line producer and frame placement

`Update3_State` 82dfe708 calls state.Update at e7d0, controller.Update at e7d8,
the board's OnEnd at e7f0, and Skateboard.UpdatePrePhysics at e808. That outer
call captures **Reckoning+1b0 mUpVector** at 82b37ce4, calls StartWheelLineTest
at 37cfc, then ApplyForces at 37d04. This is the existing typed `up()` field,
not ground_normal(), which is the distinct +1f0 field.

StartWheelLineTest82b24530 acquires and begins its actual batch before adding
four wheel lines. Only afterward it queries Part6.GetTransform at 24664,
stores deck.valid=false at 24670, and writes translation to Body+1f10.start
at 24680. It adds the fifth/deck query at 246b8, asserts successful admission,
and ends the batch at 246ec. Corrected raw VA decoding for 24694 (171fc835)
is VA63, the freshly loaded deck translation; IDA's displayed v95 alias is
misleading. Existing start calculation uses the correct translation.

`Update4_PostEngineAdjust` 82e01c78 first calls EndSkeletonLineTests at 1c94,
then board.UpdatePostPhysics at 1cac. The board outer call begins with
EndWheelLineTest at 82b37d48, before bad-data/strange-acceleration reset paths
and ordinary body.UpdatePostPhysics at 37e5c. Actual End owns synchronization,
result admission, and release. No constructor initializes deck.start or valid.

Existing `try_start_board_wheel_lines` and `try_end_board_wheel_lines` already
operate on the same actual constructor-retained line owner and real world
query services. A selected corpus schedule may exercise these source stages
with a late actual Reckoning.up read, but must explicitly report unfinished
remaining state/controller/force/solver and post-physics stages. Moving these
calls after a partial Ground prefix does not make that partial prefix a full
Update3. Neither unknown deck fields nor missing query results authorize zeros.
