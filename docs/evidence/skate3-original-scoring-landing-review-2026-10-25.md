# Local scoring and transition follow-up, 2026-10-25

The owner rejected accelerated menu transitions and requested the original Skate 3
scoring behavior. These changes are local review work; hosted update 5 remains held.
Only short diagnostics are authorized.

## Original evidence

Original installed Skate 3 memory pages and converted Vault data were inspected
outside the repository. No original executable or game data is committed.

- TU3 ScoreModule RewardAirSequence `82DA32D8..3308`: sketchy flag at holder+7245
  applies collector+0x620; clean flag at holder+7246 then applies collector+0x630.
- CheckForAirSequenceReward `82DA3310..33C8` captures landing type 0 into the
  separate clean flag. Landing type 2 is neither clean nor sketchy.
- CalcLandingQuality `82DE61D0` publishes valid-data167, type96, spin92 and
  sideways-speed84. The selected source solver already calculates this output.
- Original collector default field `5A2D1476F08690EC` is binary32 1.1. Air
  curves at 0x320/370/3c0/410 and this scalar are byte-identical between the
  installed original data and the embedded pack. All 269 shared catalog entries
  also agree on category and score type with Skate 3's table.

The host observer did not copy the source landing result. Its dormant Skate 2
landing producer therefore never updated the scoring consumer. The host ledger
also omitted the separate Skate 3 clean reward. The change copies the completed
source facts, retains clean classification across the existing two deferred
landing frames, and applies the original scalar after the sketchy branch. It
does not raise point tables or introduce a guessed big-air multiplier.

Confidence: **high** for this native branch/data match; not a certification of
every retail collector or every gameplay situation.

## Presentation

Remove the 0.35-second menu-camera acceleration and time clamp from preview 7.
The authored camera track again advances at the ordinary frontend delta.

## Bounded observations

A copied-save remount trace of preview 7 and this diagnostic revision showed
input reaching the action graph during boarding. After Y at frame150, source
GroundAnimation103 was observed at180; PhysicsGround100 and a nonzero steering
attribute were observed by185. Heading changed from -1.039 at185 to -0.951 at190.
This does not reproduce an indefinite steering lock and does not test a normal
main-menu launch. The opt-in startup-turn probe covers that separate handoff.

A standalone recovered-source scoring probe, using original data, checked a
controlled 60m-distance/12m-peak/1080 kickflip. Raw reward is 2180; clean landing
is 2398 at1x, 3597 at1.5x, 4796 at2x and 7194 at3x, with exactly one bank. These
are controlled accounting inputs, not a claim that a controller performed that
jump in the live game. Final executable checks and exact hashes belong to the
local signed package's VALIDATION.json.

Opt-in controller probes are finite, use copied saves and have an external
timeout. They do not run during ordinary play. No publisher, watcher, scheduled
task or friend update is started here.

## Marker-return steering clarification

The owner clarified that the failure is placing and recalling a marker while
remaining on board, then moving the left stick without pushing. A copied-save
controller recording reproduced this in the diagnostic build: Actor reset
generation 1 at tick394, PhysicsGround100 at396 and GroundAnimation103 at415.
Root yaw stayed exactly -2.8080 from420 through505 despite positive then negative
Turn/KickTurn inputs. The board heading also stayed fixed. The earlier Y-remount
observations did not cover the reported scenario.

GroundAnimation103 PreUpdate82D33DB8 calls generic Reckoning82D8C8F0 through
input-fill82D8E5C0 before SkeletonUpdateAnimated82BDDA10. The PC integration
instead retained a heading to avoid feeding absolute animated deck rotations
back into takeoffs. That also discarded KickTurnSteering's native signed Spin.
The narrow PC repair integrates that attribute about the current reckoning up
axis only while native KickTurn bit28 is active. Spin is still evaluated by the
original clip/curve service82BAE4A8; no new turn-rate constant or post-teleport
timer is introduced. Other animated takeoffs retain their existing heading.

Confidence: **medium** as a PC integration repair, not a complete recovered
implementation of generic Reckoning. The exact marker-place/recall sequence,
with both stick directions, is an opt-in bounded final-executable check.

## Final-executable follow-up

Revision52876f7 passed the exact marker-return check. Root yaw changed -0.3080
and +0.3700 radians in the two direction windows; solved physical-board yaw
changed -0.3114 and +0.3687. The initial copied-save live air probe banked257
against recovered-source257.8186 (integer HUD truncation), but observed no
retained landing-data latch. Inspection exposed an additional PC integration
error: publish_feedback cleared the persistent CalcLandingQuality fields each
tick. Remove that per-frame clear and reset at actual teleport instead, matching
Reset82DE3F38 and CalcLandingQuality's nonlanding early return.

TU3 CheckForAirSequenceReward82DA3338..3394 compares kind1 for sketchy and kind0
for clean; all other kinds set neither. The host's S2 error policy for kind3
must therefore be bypassed only in S3 mode. This avoids rejecting a valid native
landing classification when connecting the result to the host ledger.

The first startup steering probe started counting as soon as the menu object
disappeared, before the Actor/source was loaded. Its startup-source-absent error
is a diagnostic readiness failure; wait for a focused source before beginning
that measurement. These changes require a new executable and fresh checks.
