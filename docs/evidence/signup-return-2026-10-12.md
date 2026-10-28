# Challenge sign-up prompt return audit - 2026-10-12

## Request and result

The D-pad Up sign-up prompt failed to return after bailing,
leaving an AI skater's challenge area and walking back. Work is PC-only, solo.

Two confirmed source mismatches in the shared prompt/HUD path were corrected.
The exact reported sequence was not reproduced: the challenge/AI identity and
whether an invisible prompt still accepted D-pad Up were requested but have not
been supplied. Do not claim a demonstrated root cause for that particular run.

Confidence: **high** for the corrected ownership/dispatch and drained-event
handling. Applicability to the reported symptom remains a **hypothesis** until a
matched gameplay reproduction. No arbitrary distance, bail timeout or forced
challenge-state reset was added.

## Original reference

Live IDA Xbox debug reference, not inferred tuning:

- `cChallengeSignUpManager::Update` **82db44a0**: every update performs the box
  test. WaitingForDPadUp hides and returns to WaitingForSignUpVolume if sign-up
  becomes unavailable, or a changed collision set becomes empty. WaitingForSignUpVolume
  checks the current nonempty set and availability each update, without requiring
  another enter edge. This portion already agrees with the port.
- `PerformBBoxTests` **82db33e8**: an invalid/missing local tester leaves previous
  collisions alone; valid tests replace them and report set changes. Do not clear
  this cache arbitrarily during a teleport/bail.
- `IsAllowedToSignup` **82da2d60** calls the activity manager and the high/low
  priority sign-up subcontrollers. `ChallengeHOMManager::IsSignupAvailable`
  **825b2fc8** excludes BOTH Scoring2 bail-in-progress and bail-completed flags.
  Existing chase, main challenge, go-to, discovery and service gates were retained.
- `HUDManager2::FixupActiveComponents` **82cedff0** binds front-active data,
  makes it dirty, and calls `Show(false)` on components that become unbound.
- `HUDManager2::Update` **82cee4f0** synchronizes component visibility with the
  bound instance, then updates it. Unbound components do not receive Update.
- `SignUpHUD::Show` **826c0430**: show refreshes and sets the screen state, THEN
  stores 15 at +0x74. Hide stores zero to visible +0x14, delay +0x74 and held flag
  +0x78, then sets screen state 0. Raw stores at **826c0470..90** confirmed.
- `SignUpHUD::Update` **826bd3e8** refreshes dirty data first, then decrements
  delay if D-pad Up is not held. Raw **826bd424..38** confirms this order.
- `HUDManager2::UpdateComponentInput` **82cebc38** only dispatches input to
  active, visible components. `SignUpHUD::UpdateDPadUp` **826bd450** applies
  the delay for sign-ups; teleport and DMO actions take their separate branches.
- `SignUpHUD::UpdateInput` **826bd7c0**, release branch **826bd938..44**:
  0x4000 clears the delay and held flag without a second session-level latch.
- `FrontEndManager::Update` **82640130**, raw **82640220..40**: APT update,
  then HUDManager2 update whenever FE state is IN_GAME (5). There is no pause
  gate around this HUD update. Screen state calls still take effect when another
  front-end screen covers the HUD.

The original actionoverlay2 APT timeline was also inspected via the existing
`.scratch/prop-hom-20261010/actionoverlay.txt`: button_intro starts at frame 4,
fades in through 14; button_outro fades out through 24. No animation/tuning
changes were justified or made.

## Changes

- `challenge/signup_hud.rs` now owns the delay and held flag, resets them through
  the original Show path, and decrements in its bound-component Update. Input
  checks actual binding/visibility. Teleporters retain their immediate branch;
  DMO input remains with the prop adapter. Direct several-challenge popup hide
  and cancel/show calls use the same lifecycle.
- `challenge/signup.rs` retains the activity/box/state machine and diagnostic
  prompt output; its public input functions delegate to the HUD. Removed its
  duplicate input state and `prompt_update` from `challenge/system.rs`.
- `session/selected_session_career.rs` forwards the translated release to the
  HUD without an extra session latch. Its popup/poster/overlay input priority
  is retained.
- `apps/skate2-pc/src/marker_ui.rs`: one HUD screen-event consumer is used by
  gameplay, paused career overlays (including HoM results), and pause/map update
  paths. Previously `take_career_hud` drained signup/objective/tutorial changes
  that the latter paths never applied. A drained show could leave the movie in
  its old hidden state while the core already considered the component visible.
  The shared helper also retains central and turn HUD events. Drawing remains
  subject to existing HUD visibility rules; queued changes are applied once,
  after the APT advance, including while covered by another screen.

## Review and limits

- Scoped before/after diff and callers reviewed, including every PC
  `take_career_hud` consumer and sign-up input call site.
- Manual source-path review: ordinary bail/leave/re-enter; teleport invalid
  tester; held early press; HUD unbind/rebind; popup cancel; teleporter/DMO
  priority; pause/map and career overlay event delivery.
- Five Rust files parsed with rustfmt stdout mode, without writing formatting.
  This is syntax review, not Rust type checking or runtime verification.
- No tests added or run. No compilation or game launch. Existing EXE predates
  these changes. No physics, original sign-up bounds, challenge eligibility
  gates or Android files changed.
- Still needed to confirm this specific report: challenge/AI name, whether
  D-pad Up worked while invisible, and gameplay reproduction on the rebuilt PC
  executable. If it persists, capture availability gates, current box contacts,
  HUD binding/visibility and emitted screen states; do not bypass the gates.

Source backups, scoped diff and live IDA exports:
`.scratch/signup-return-20261012/`.
Earlier source-only lens-reflection removal and PostFX-off +10% brightness
changes are preserved and still also need a rebuild.
