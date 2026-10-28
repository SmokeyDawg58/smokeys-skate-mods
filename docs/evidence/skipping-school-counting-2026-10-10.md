# Skipping School: successive grab objectives

Solo investigation of the reported inconsistent grab credit. The live rich-symbol IDA reference was read-only. The source fix restores an omitted native call in shared objective activation.

## Original mission requirements

The English and Labels categories in the January 17 build's `data/big/miscboot.big` identify `ID_CHALLENGE_M_1B_03_SPOTFILM_SPONSOR_ME_3_TITLE` as **Skipping School** (localization hash `293431b2`). Its internal challenge is `sponsor_me_03`; the displayed objective is **Do 4 Grabs off the Banks** (`28d5b85f`).

`ChallengeCore` contains four hidden objectives, `sponsor_me_03_0` through `_3`, with identical completion scripts: enter any of nine specified bank volumes and complete a trick in the `grabs` group. They evaluate on successful sequence completion. A tweak is not a separate requirement. Objective `_4` displays the completed-objective fraction and succeeds at four.

All four grab objectives use activation mode 1 and the same scoring boundary, `0005751903e38706`. The challenge's flow type is 1, maximum active objectives 2: the displayed fraction objective and one grab objective at a time. Completing a grab enables the next hidden objective.

The research database's six VLT/BIN files were compared byte for byte with the original `data/db/db.big` payloads; all match. Decoded scripts/settings and provenance hashes are under `.scratch/skipping-school-20261010/`.

## Confirmed mismatch and fix

`cChallengeObjective::ActivateObjective` (`82d4c170`) handles a disabled objective with nonzero activation mode by:

1. Setting status to Enabled.
2. Clearing `InScoringBoundary`.
3. Calling `cTriggerVolumeManager::ResetTriggerVolumeCollisionState` on its authored scoring boundary.

The port implemented the first two steps but omitted the third. Thus a new grab objective could remain Enabled while the skater was already inside the shared scoring boundary: the existing collision history suppressed the enter event that would make it Active. Leaving and re-entering the boundary could restore counting, making the result depend on the route between attempts.

`crates/skate2-core/src/challenge/objectives.rs` now emits the existing `ResetTriggerVolumeState` output at that transition. The session already applies this output to collision history, so continued overlap generates a fresh enter on the next collision update. This restores the shared activation path for other objectives using the same mechanism.

The new call is confined to Disabled -> Enabled with nonzero activation mode. Repeated activation of an already Enabled/Active objective does not reset its boundary. Mode 0 retains its immediate activation behavior. Original grab timing, bank volumes, scripts, successful-landing requirements and mission thresholds are unchanged.

## Reference audit

| Native address | Evidence |
| --- | --- |
| `82d4c170` | ActivateObjective; raw PPC `82d4c230..82d4c248` confirms the scoring-boundary load and reset call, including the by-value 16-byte argument that Hex-Rays misrepresents. |
| `82d4c508` | ActivateObjectives: authored flow limit, skipping completed objectives, activating the next. |
| `82d4c918` | Completion follow-up invokes ActivateObjectives. |
| `82e22688` | Boundary reset resolves the volume using `(float)_u64tod(VolumeID)`; the fix uses the same conversion and existing output path. |
| `82e20e10` | Reset delegates to the trigger groups. |
| `82d4de10`, `82d4bd58` | Boundary enter activates Enabled objectives; boundary exit re-enables an Active mode-1 objective. |
| `82d4dfd0`, `82d51cf8`, `82d4d038` | Completed tricks are retained while Active; successful sequence completion evaluates objectives; the special sequence-end reset applies to mode 4, not this mission's mode 1. |
| `82daf620` | Grab membership uses the authored group and its descendants. |

## Validation and delivery

- `cargo check -p skate2-pc` passed, exit 0, in 55.24 seconds; existing warnings remain.
- Reviewed the exact change against a byte-preserved pre-edit snapshot. Only the activation key capture and missing reset were added to implementation code.
- No tests were added or run. No gameplay reproduction or original-runtime comparison is claimed.
- The release executable has not been rebuilt; run `build-release.bat` to play the fix.
- Real saves were not written. Existing unrelated work was preserved.

Scratch evidence: `original-objectives.txt`, `database-provenance.txt`, `ida-reference.json`, `objectives.before.rs`, `change.patch`, `check.log`.
