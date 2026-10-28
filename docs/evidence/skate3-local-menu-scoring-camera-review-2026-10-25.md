# Local menu, scoring and camera review

Owner request: block movement through menus, use Skate 3 air scoring, and
address slow replay playback/transitions and the camera after spawning.
This is an owner-only preview. Publishing is held until the owner approves it.
The on-foot marker, authored prop mesh and marker-during-bail patches remain.

## Implementation evidence

- The source raw pad bypassed the PlayerUI focus condition in
  `selected_session_public_facade.rs` (GameModule DoGameUpdate 82730418).
  The same focus decision now supplies a neutral pad and pinned/ForceBrake
  policy to the source controller. Menu inputs cannot drive source actions.
  The frontend ambient-world path explicitly supplies that unfocused policy.
- The retained score database previously used Skate 2 values on shared trick
  names; only extra Skate 3 names received TU3 definitions. The source player
  now selects TU3 points, delays and variants, while preserving host trick IDs,
  labels, tutorial fields, HUD events and challenge consumers.
- Per-module scoring attribute wrappers select the embedded TU3 state/general
  data (classes 546C36B656038E04 and 349215E2E817703C). Cloned time/distance
  graph addresses share the module's selector. Air distance, peak height,
  normal-relative height gain, spin, flip, grab rates, stance/stationary
  factors, repetition and multiplier/line timing use those authored values.
  The 79 scalar words are cached once; graph samples already live in the
  retained typed ScoringData. No per-tick JSON/name scanning is introduced.
  `Scorable82DA2340`, `ScoreHolder82DA5D18` and the existing TU3 field schema
  supply the identities. This does not certify all collector code as retail parity.
- Frontend/replay controls consume fixed 1/60 deltas. Their host wakeup now uses
  60 Hz while an offline paused world can retain its own slow-motion request.
  Online menus that continue simulation retain the world timer.
- The PC host supplies elapsed wall time to replay-editor updates; live
  skating still receives 1/60. Expensive restoration therefore skips recorded
  frames as necessary instead of extending the recording's playback duration.
  Existing diagnostic completion/control runs retain their fixed clock.
- Initial source-player construction is deferred behind the frontend.
  A completed source teleport requests the camera's native reset and discards
  stale subject/query history before publishing the destination camera.

## Verification scope and confidence

The embedded default rows contain 39 general and 62 state fields. Static
inspection found all 31 literal scalar/word bindings used by these collectors.
The build is compiled with the existing v0.2.7 online input. No gameplay or
regression tests are run, following the owner's earlier explicit instruction.
Saves/settings are checked for byte preservation during signed local installation.

Confidence: high for the observed bypass, selected data identities and clock
coupling; medium for the proposed fixes' effect on the reported camera/replay
symptoms. Exact retail Skate 3 scoring and the owner's perceived responsiveness
remain unverified until gameplay review. No public host upload is authorized yet.
