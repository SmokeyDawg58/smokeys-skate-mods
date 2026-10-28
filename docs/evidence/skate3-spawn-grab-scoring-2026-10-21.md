# Skate 3 player: native spawns, grab scoring and HUD recovery

Confidence: **high** for the tested PC integration behavior. These are explicit
PC host adaptations; no console-wide parity or exhaustive online certification
is claimed.

## Evidence and ownership

- S2 Actor::Teleport825ed1c8 publishes the real external reset and generation.
  Actor::SpawnFootForwardStance825ed638 stores a requested leading foot;
  PlayerSettings.SetPlayerStance8255b570 independently stores the character's
  natural stance. `FootForward` RIGHT0/LEFT1 differs from S3 natural GOOFY1/
  REGULAR0. A switch marker must not edit the character profile.
- S3 checkpoint stance82592B68/82592C08 saves the leading foot for
  ResetToGivenStance82B97308. The host previously restored it only on first
  controller admission. Later native resets retained old relative stance and
  graph playback; real-road tests reproduced a requested onboard reset staying
  offboard and switch markers retaining the previous leading foot.
- Onboard Actor resets now enter the stock teleport graph. Offboard resets
  apply the same native GivenStance operation before their standing pose is
  published. Physical bodies remain the existing host-owned Skate2 assembly.
  Generation acknowledgement waits for the manual reset continuation and
  cannot acknowledge a superseding generation.
- The real source input callback (`input_phase::advance`, reset_player82DB8998)
  publishes a one-tick `teleported` fact. It drives S2 has_just_teleported so the
  native camera anchors initialize even before teleport completion. It also
  reaches the retained ScoreModule.Reset path from TeleportPlayerNow82e00988.
  Resetting unrelated S2 camera/trajectory owners invalidated source anchors
  in an intermediate test; that approach was rejected.
- S3 ScoringGrabs82BBEF60 publishes its authored continuous name/tweak separately
  from ScoringTrick82BBFA88/8258FA20. S2 Nursary.ProcessAnimData82dfc358 admits
  a descriptor only with 0x03000000. The source grab payload lacked bit25, so
  ordinary source grabs animated but contributed no native grab award. Source
  grabs now supply that existing continuous-descriptor bit; names, curves,
  penalties, holding time and arithmetic remain native S2 scoring inputs.

## Scoring and display reproduction

The human's retained Test4 log contains an Airwalk at tick325139..325330:
banked5804, then finished5804/completed_successfully=true. The following idle
HUD score is zero. This proves that at least this reported-looking episode
banked points; a zero live score alone is not evidence of a lost award.

The original-road grab probe uses ordinary raw pad pushing, ollie and trigger
holds. Before the admission-bit fix, all four source cases banked only the
ollie's two points and recorded no ended grab. Native S2 grabs produced three
points when released and six when held. After the fix all four source cases
recognize the same FS/BS grabs and bank the corresponding three/six points.
MissionFrame completed totals and banked changes agree with HUD finished totals.

TrickDisplay previously discarded the one-tick ReadyToScore event whenever
`allowed=false`; mbWipedOut could remain latched when a challenge showed the
movie again. Logical recovery now happens while hidden and emits no display
command. The real APT test covers hidden and unbound recovery.

ScoreMovie now keeps a positive completed score through the original name-outro
callback. Following idle packets can no longer erase it before a rendered frame.
EndSequence starts the original name outro; its authored callback ends the hold.
A new trick, bail or recovery clears the retained presentation value. This is a
PC presentation fix, not a change to banked points or transmitted mission facts.

## Validation

- `spawn106-fixed-corpus`: PASS72.53s. Twelve sequential native Actor reset
  cases: both natural stances, both requested feet, board/offboard/reboard,
  reversed authored headings. All requested feet, board states, profile stances,
  generation acknowledgements and scoring reactivations pass. Onboard deck
  headings match the authored target (horizontal cosine greater than0.998).
  Offboard raw/effective heading conventions remain logged rather than asserted
  as console parity; the corpus verifies their requested board and foot states.
- `grab106-fixed-corpus`: PASS99.18s. Eight native/source landing cases; all
  have a real ended grab, positive award, matching mission/HUD totals and no bail.
- `spawn106-ui-units`:9PASS; `spawn106-scoring-units`:11PASS.
- `spawn106-hud-final2`: real original trickdisplay2 APT/localization test
  PASS6.35s, including completed-score persistence across four idle packets,
  release through the actual authored callback, both hidden/unbound recoveries,
  trick text, multiplier/ring, timer and bail.
- Earlier failing harnesses and intermediate failures are retained in work/;
  they are not reported as successful verification.

The original ROM/save and frozen Test4 distribution were not modified. Network
identity/protocol/server checks are unchanged. Runtime checks use copied saves
and controlled loopback clients; exact optimized-build results belong in the
delivered package manifest. Stutters/FunTrack detail and full-career certification
are outside this bug-fix result and remain open.

## Optimized Test5 runtime

The final executable SHA-256 is
`a30ac30a01798607fb6d9798a4649ff4b2cac2c9916a4eac56726bca38b862a2`.
Its recorded source fingerprint
`f77effde9908f2e6250735453b2dfdf68a2731abdf11b2e04ae3039c990bc754`
matches the independent calculation made at build freeze over the4,897 build
inputs, including the raw-trigger online diagnostic. The optimized build
succeeds with existing warnings; the full workspace/clippy gate is not claimed.

`full-skate3-validation102-marker-normal-20261021-155123` and
`full-skate3-validation102-skate-easy_motorized-20261021-155208` both exit0,
advance900 actual source physics ticks, retain complete unique journals from
sequence0 through session_end, produce captures and leave the original save
unchanged. No fatal/tick/nonfinite/anomaly events occur. Visual inspection
confirms the original Skate2 character/world with an onboard board pose.
The timing journals still contain streaming spikes; this is not a stutter fix.

The first two-client test, `test106-online-pair-20261021-155246`, verifies
normal sign-in, joining, a real two-player challenge and network poses, but is
not a grab-scoring verification: MainMenuGameplay ends at900 gameplay ticks
even when --live-completions requests3000. Both clients exit normally before
the planned grab hold. The harness uses the existing MainMenu bounded clock
for the subsequent run; production behavior is unchanged.

`test106-online-pair-20261021-155407` reaches actual grab scoring on both
clients: FS/BS grab announcements and successful35-point banks agree with the
finished sequence scores. Thirty-three common rolling challenge-fact hashes
match with zero mismatches. Both exit1 because MainMenu's diagnostic asserts
that it ends at the front end, while this test intentionally remains inside
the online challenge. This failed diagnostic postcondition is retained, not
reported as a clean runtime pass. The final harness uses the existing
MainMenuFreeskate clock, whose bounded completion permits active gameplay.

`test106-online-pair-20261021-155626` passes in83.51s with the exact frozen
executable. Both clients sign in, join the same lobby and start a two-player
Own The Spot challenge. They advance3,283/3,281 source ticks and send3,278/3,276
network pose frames. Both announce a real grab and bank7 points, matching their
finished scores. Forty-one common challenge-fact hashes match, with zero
mismatches. Retained journals are complete/unique, include session_end and have
no fatal/tick/nonfinite/anomaly events. Both clients exit0, the server is stopped
and the original save is unchanged. This is a bounded compatible local-server
test, not certification of every public challenge or network condition.

## Follow-up: randomly buried wheel report

The human clarified that wrong board state, heading, stance and a buried wheel
occurred randomly across challenges/locations. The existing repeated reset
corpus was extended with test-only measurements of all four actual collider
wheels against the loaded collision triangles after settling. The query uses
each solved wheel center and the actual Vault wheel radius, and does not move
any body or change solver settings. The test has an explicit5mm regression
limit; this is a diagnostic limit, not a native collision constant.

`spawn107-wheel-corpus` passes62.73s for the same twelve reset cases. All eight
onboard cases have four wheel contacts. All32 measured wheel gaps are between
-0.02085mm and -0.01321mm, far smaller than a visually buried wheel. The stuck
wheel symptom is not reproduced in these fixed-path cases; other locations,
transient states and the player's next in-game test remain unverified.

The wheel measurements add two cfg(test) accessors and assertions in an
existing test-only corpus module after the executable was frozen. A byte-level
inverse of just those additions reproduces the frozen source fingerprint above;
production code is unchanged. The source archive retains the new tests and
supplies the three exact-build source overrides for fingerprint reproducibility.
