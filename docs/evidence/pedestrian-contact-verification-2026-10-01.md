# Pedestrian physical-contact verification

The visible/moving run completed3600 ticks before this diagnostic was added.
That run alone did not prove physical contact.

Re-read IDA OnEntityCollision82e95158. It obtains the collision classification
through the existing entity callback, then for non-self type4/5 contacts uses
pBodyOther.mpOwner and IActor.IsLocalPlayer/GetPlayerIndex before recording the
local collision flags. The existing Rust contact owner path reads sealed Island
contacts and resolves actual body owners. It does not infer collisions from
presentation proximity. No collision logic was replaced during this check.

SKATE2_TRACE_PEDESTRIANS now optionally records actual local Actor contacts at
resolve_classified_collision. First20 and every120th later contact are printed;
normal play remains silent. An Ignore contact is not proof of response; a
Light/Full contact and successful continuation is the intended verification.

The opt-in --live-controls-demo pedestrian-approach diagnostic supplies ordinary
raw pad input: Y to dismount, then horizontal movement toward the nearest
published pedestrian on roughly the same level. Camera projection horizontal
right/forward axes determine the diagnostic stick. There is no forced spawn,
teleport, collision, state switch, or altered AI/physics setting. This driver is
only a test tool, not recovered original-game behavior or a gameplay feature.

Build/runtime results pending in artifacts/pedestrian-contact-build.log.


First diagnostic build PASS (6m36s). Approach run stopped at1579 on generated
WatchGreetTarget_102 828f7770: relocated payload at BIN606080. Re-read
ChannelGreetWantTarget.Update82707ef0 and PedStateOut.GetAudioCharacter82e78490.
The getter loads precisely the u32 at model layout+4 (82e784e4), not bytes0..8.
The Rust prefix read overlapped the preceding relocated string pointer. Changed
to existing try_cached_layout_field(4,4), preserving relocation guards and the
original return-greet states69/62. This is not a default or callback skip.
The approach driver initially got stuck against plaza geometry before contact;
it now follows the already successful straight walking route for3000 ticks
before steering toward a pedestrian. No gameplay navigation changes.
