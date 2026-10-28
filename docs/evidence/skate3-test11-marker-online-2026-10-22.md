# Test 11: offboard marker movement and online diagnosis

## Scope and baseline

The supplied Test 10 executable has SHA-256
`0ef857bcec7c31d16f4e920985c6946b7afa78ff0cfe2f57d4ffd8b1eac8554f`.
The supplied full-source snapshot already includes upstream Skate 2 commit
`20ac75e7238655fe08a65c27f8a6d0642915ae33`, version 0.2.5. A fresh fetch
on 2026-10-22 found no later commit. Preserve its integrated mod features.

## Reproduced marker failure

`skate3_offboard_marker_return_allows_walking` uses the real Skate 2 world,
embedded Test 10 player assets, actual controller input and the native Actor.
It dismounts, captures the same `CalcRespawnTransform` used by PlayerUI while
offboard, walks away, returns through Actor teleport, and then holds walking
input for 150 frames. It uses a separate copy of the supplied character save.

Before the fix, the marker return completed and the physical state was 500,
but the ActionGraph remained in `OnBoardGeneral.OnBoard.Teleport`. The sampled
stick and camera-relative axes were nonzero, while biped movement input and
velocity stayed zero. Walk-away distance was exactly zero:

* start: `[786.14886, -59.4285, 882.5687]`
* end: `[786.14886, -59.43097, 882.5687]`

The previous reset regression checked destination and acknowledgement but
never required locomotion afterward. Its passing result did not cover this bug.

## Focused reset correction

The host's manual-target graph override previously jumped directly from filtered
Teleport (5) to Offboard (6). That skipped the Ground observation corresponding
to `enter_after_teleport`, which follows original reset ordering
`PhysicalPlayer::Teleport` 0x82DB8998 / 0x82DB8DD4..0x82DB8DE4.

Keep both targets on the Ground graph continuation (1) after the pending reset
transform is consumed. The requested offboard target remains retained; State77
and the authored `TeleportOffBoard` transition still own the eventual dismount.
Do not reset whole animation controllers, fabricate world contacts, clear user
input, or force the physical player into a different state.

Stock embedded ActionGraph evidence: Teleport leaf at file offset 90756;
`PhysFilteredState(teleport)` at 90808; `TeleportOffBoard` at 90874 with
`PhysicsRequestsDismount` at 91001. Confidence is **high** for the observed
host-integration failure. This is a PC mod integration correction, not a new
claim of console physics parity. Post-fix runtime validation is recorded below.

## Online findings

The configured server `172.237.109.212:32082` advertises protocol 11. The
supplied mod's existing network build ID `fadf63f8` completed a verified server
handshake. A bounded client using that same identity, version 0.2.5 and the
supplied networking source successfully signed in and received an empty lobby
list. Empty lobbies are not a version rejection. No lobby was joined or changed.

The existing installed game executable is byte-identical to the standard
download, rather than the supplied Test 10 executable. This is a possible source
of launching a different build, not proof of what the user ran when rejected.
No public-server authentication or wire protections are changed.

`--check-online` now checks the exact launched executable's build, configured
server, sign-in and lobby listing, with separate build/protocol/network failure
messages. It does not load the game world, write settings or join a lobby.

Network unit tests: 91 passed, including authentication, replay protection,
transport, lobby listing and challenge handling. The initial sandbox run failed
one temporary-file write; rerunning with a writable test temp directory passed
all 91. Existing source warnings remain outside this focused change.

## Post-fix actual-data results

The reproduced walk-away regression passes after the focused correction:
13.415833 metres of walking in the same 150-frame interval, versus zero before.
The ActionGraph returns to its offboard Default state and forwards movement.
The test still checks the return destination and teleport acknowledgement.

The following existing actual-data regressions also pass:

* Offboard marker return after a bail stays near its destination.
* Repeated host spawns retain the requested stance, board state and heading.
* Banked marker returns accept pushing held during the reset and can move away.

The source archive omitted root `changelog.txt`, required by `include_str!` in
the PC executable. Restore that file verbatim from the matching upstream
20ac75e checkout; no placeholder changelog or runtime fallback is introduced.

## Final executable and rendered checks

* Release build passed. The exact Test 11 EXE completed verified sign-in and lobby browsing; no public lobby was joined.
* Rendered PlayerUI marker return completed 740 ticks. The actual offboard return was observed at tick 414, followed by 8.9154 metres of walking after tick 560. A watchdog warning occurred only during the final screenshot readback after gameplay completed; the diagnostic exited successfully.

Build inputs are private and remain outside the source tree. The existing asset
pack was recovered unchanged from the supplied executable and all 22 records
were verified against their embedded SHA-256 hashes. Do not distribute network
identity sidecars or include private compiler build outputs in source packages.
