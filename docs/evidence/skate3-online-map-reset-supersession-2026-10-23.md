# Online map reset supersession (2026-10-23)

## Captured failure

With all Skate 3 physics enabled, entering Fun Track could leave a fully black
screen while sound and simulation continued. The failing session recorded fade
alpha 1, map travel dispatched, expected generation 4 equal to the Actor's
current generation, and no ready acknowledgement. The prior reset (generation
3) had been applied with on_board=false. The physical player then remained in
category 200, falling, while teleport_pending stayed true. Generation 4 never
reached the Skate 3 spawn-apply bridge.

## Host integration correction

`Skate3Player::prepare_animation_tick` previously waited for both a pending
physical input transform and the earlier manual board-family continuation.
That continuation can remain unfinished after the physical transform was
consumed. A newer explicit Actor reset then waited behind an older request
that could no longer complete, blocking the map reveal.

Admit the newer queued reset once `player_input.pending_teleport()` is empty.
`Runtime::request_manual` already replaces the prior manual target. Retain the
physical-input guard because request_teleport_ex rejects a second transform
while one remains unconsumed. Retain manual board-family continuation for
ordinary marker returns, including walking away from an offboard marker.

Confidence: high for the captured host-bridge deadlock. This is a PC mod
integration correction, not a claim of additional console behavior parity.

## Actual-data regression

`skate3_online_map_reset_supersedes_unfinished_offboard_continuation` runs the
embedded player with original Skate 2 world/rig owners. It first publishes an
offboard reset while pinned, confirms the prior request is still pending,
then publishes a newer onboard reset. With the repair the newer destination
is acknowledged on step 24, one tick after publication. The original-data test
passed. The seven existing physics preservation checks and actual-game online
map tests must also pass before delivery. User Fun Track validation is pending.
