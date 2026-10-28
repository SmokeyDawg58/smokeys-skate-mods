# Walking foot placement correction, September 25

Legs were seen stepping behind the skater. The previous five-control
milestone demonstrated completed input sequences, not visual walking parity.
Work is solo; diagnostic builds and launched-game replays are authorized.

## Source mismatch

IDA `OffBoardFeetIK::ApplyIK` at **0x82dcd8e0** branches on retained state 3
(locomotion) at 0x82dcd900..908. In that branch:

- Left ProcessedPhysIn influence +0x988 is checked at 0x82dcd91c..928.
  Only an ordered positive value calls `SetLeftFootIKTargetPosition` with
  retained left world target and blend **1.0**, at 0x82dcd938.
- Right influence +0x98c is checked at 0x82dcd940..948, then the corresponding
  right world setter is called at 0x82dcd958, also with blend **1.0**.
- Raw branch words 0x40990014 and 0x40990180 are BO4/BI25, skipping when
  GT is false (including unordered). No local setters, blend cvar read, or
  foot-normal setters occur in this branch.

`Skeleton::SetLeftFootIKTargetPosition` **0x82b41250** and right **0x82b41270**
copy the full position to SkeletonIK+0x220/+0x230, set the external target
byte and blend. They do not transform the position. The already implemented
Local setters **0x82b412f0/0x82b41348** instead transform world to animation
space and set a different target/flag, selecting a different IK mode.

Rust had reused the standing/air local setter path for locomotion. It
refreshed BOTH foot targets (including the inactive foot's deliberately
retained old target), used kOBIK_IKBlendVal, and refreshed normals. These
are real differences from the reference, not missing animation assets or
values that should be tuned.

## Correction

Both ground and locomotion/air callers now share one ApplyIK dispatch based
on retained feet state. State 3 uses the existing retained SkeletonIK owner
with the native world setter semantics and influence gates, left then right.
Other admitted states retain their local behavior, with air blend 1.0.
Fixed-foot states 0/1 remain explicit earlier CalculateIKTargets boundaries.
No target clearing or blend-history reset was added: existing UpdateStatus,
UpdateBlendValues, and end-of-postphysics flag consumption perform release.

The regression checks world target retention, full blend, inactive-foot
preservation, absence of local requests, and release to mode 3 when footfall
switches. Existing IK status/external transform tests cover downstream owners.

## Validation

Baseline captured with existing `offboard-jump` raw-input script stopped at
tick 208 (before X at 220):
`artifacts/walking-fidelity-before-208-20260925.png` and matching log.
Release build PASS: `artifacts/walking-ik-release-build-20260925.log`.
Corrected captures at 180, 195, 208 and 218 all complete successfully and show
alternating planted/swinging legs. The matched 208 before/after comparison
shows the swinging leg no longer pulled against the other leg's old target.
These captures are visual corroboration of the source fix, not an original-game
pixel comparison or proof of all walking states.

Full walk/jump/stop replay PASS 540 ticks, ends Motion752 / physics500:
`artifacts/walking-fidelity-after-540-20260925.log`. Nine seconds of simulation
completed in 8.988515 seconds with the first tick immediate. Captures/logs use
`walking-fidelity-after-{tick}-20260925` filenames.
Standing/turning regression (`offboard-right-left`) PASS 540 ticks:
`artifacts/walking-ik-standing-turn-regression-20260925.log`.

Focused tests PASS **131**: `skeleton_ik_state` 107 (including the new walking
plant/release regression), `feet_ik_prefix` 24. Logs:
`artifacts/walking-ik-state-tests-20260925.log` and
`artifacts/walking-ik-feet-tests-20260925.log`.
Test build log: `artifacts/walking-ik-test-build-20260925.log`.
IDA decompile retained in `artifacts/walking-applyik-ida-20260925.txt`.

The release executable is ready at `target/release/skate2-pc.exe`.
No renderer, animation asset, locomotion speed or camera tuning was changed.
