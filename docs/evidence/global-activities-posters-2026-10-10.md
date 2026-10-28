# Global activities, betting and entry-fee posters (2026-10-10)

## Continuation on 2026-10-11

Betting visuals, safe sweep sequencing, global startup/location services and
valid-prerequisite team-photo selection are verified. The native fee-failure
input block is retained. The training-facility additional-world lifecycle is
documented as the requested report-only item.
See [continuation evidence](resume-three-2026-10-11.md) for changes, native
references, run results and remaining scope limits. The original handoff below
is historical.

## Resume here

State: `cargo build --release -p skate2-pc` succeeds with every change below. Nothing was run in
the game and parity was not run; all of it is written from IDA but **unverified live**.

### Done (compiles, not run)

- Global activities (`challenge/activity.rs`): `ActivityTree`, globals classified in
  `ActivityManager::new` (CreateActivity `0x82cdbca8`, debug-only excluded),
  `reevaluate_prereq_activities` rewritten (`0x82cdbaa8`: flags 4 sub-controllers + globals, 2|8
  globals only, else nothing), `reevaluate_global_activity_bboxes` (`0x82cdb260`),
  `perform_bbox_tests` (`0x82cdaf00`), `suspend_all_global_challenges` (`0x82cd9c40`) in
  `start_main_challenge`, `resume_suspended_global_challenges` (`0x82cd9080`) in
  `complete_challenge`, `can_start_main_challenge` (`0x82cd8798`), `terminate_mission`
  (`0x82ce4f80`, deferred queue), `game_unload` (`0x82cda420`), `run_global_activity`
  (Run + asset `0x82e12838` + DMO `0x825c3aa8`), `ChallengeSystem::deliver_terminations`.
- `system.rs`: World fields `ots_discovery`, `world_painter`; update(): deliver terminations,
  pending resets, `perform_bbox_tests` while game loaded, `ots_discovery::update`, and the two
  `ReevaluatePrereqActivities` calls of `Sk8ChallengeSystem::Update` `0x825a3f10` (progression
  toggle and StartGameSession); flush() delivers terminations before each event; OTS discovery
  handler after callouts; `game_unload_activities`; shutdown now calls `assets::shutdown_activity`
  (native ShutdownActivity `0x82e10c98` also unloads the activity's challenge assets);
  `diagnostic_run_global` now runs RunGlobalActivity too.
- New `challenge/ots_discovery.rs` (cChallengeOtsDiscoveryManager: AddActivity `0x825c0600`,
  GameLoad `0x825b6a30`, Update/PerformBBoxTest `0x825c3990`/`0x825c2c78`,
  TriggerEnterCollision `0x825c0720`, OnTimerExpired `0x825b49f8`, IsSignupAvailable
  `0x825af878`, CollidingSignupChallenges `0x825c09a8`).
- New `challenge/world_painter.rs`: layer 6 → film manager `0x82da7838`
  (`cMsgSecurityEnterNoSkate`/`ExitNoSkate`, IsInNoSkateZone), layer 13 → service manager
  `0x8289d390` (`cMsgWorldPainterServiceNotification`/`LeftServiceNotification`); session feeds
  keys each career update (`career_challenge_layer_keys`, LayerMan::Update `0x82ceb108` order:
  before the challenge update).
- `services.rs`: IsLocalPlayerInNoSkateZone reads the film flag; IsSignupAvailable `0x8289c660`;
  SuspendGlobalActivity `0x8289da38`. `callouts.rs`: Suspend/Resume `0x82da5220`/`0x82da5278`.
  `assets.rs`: RunGlobalActivity, `shutdown_activity`. `dmo.rs`: RunGlobalActivity.
- `signup.rs`: IsAllowedToSignup gates (BE `0x825ada38` goto flag, HOM `0x825b2fc8`
  PhysOut_Scoring2 +0x4640/+0x4641 = bail completed/in progress, OTS, service); WaitingForDPadUp
  tail calls OTS CollidingSignupChallenges (`0x82db44a0` tail).
- `natives.rs`/`attrib_lua.rs`: `ObjectKind::Activity`; Get/SetOtherChallengeSaveData accept the
  light-userdata activity the discovery messages carry.
- Betting core: `signpost_progress::{Betting, set_players_bet (0x825cfaf8),
  deduct_challenge_losses (0x825de1e0), issue_challenge_awards payout (0x825ddfc8)}`;
  `SignPost.betting`; `end_challenge_unwon` deducts; `DidPlayerBetMax` reads the flag.
  `signup_poster.rs`: `BetView`, `bet_view`, `bet_more` (`0x82626b98`), `bet_less`
  (`0x82626db8`), ChallengeSystem `sign_up_bet_more/less/view`; PosterView gains `bet`.
- Input: `.released` modifier (LookupModifierName `0x829868f8`), frontend map ids 164/165
  (AptLeftReleased/AptRightReleased), `PauseMenuInput.left_released/right_released`.
- Session (`selected_session_career.rs`): poster opens run challenge_info's `BetLess`; betting
  arrows (`poster_betting_input`, challenge_info UpdateInput `0x3960`, reps 5); fee-failed popup
  (`DialogOwner::FeeFailed`, template 203) with the poster's input left blocked (native, see
  below); `unload_career_activities` + `try_update_unowned_challenge_hulls`
  (`selected_session_freeskate.rs` game_unload calls it; terrain `hulls_unloaded`).

### Not done

1. PC poster betting panel (`apps/skate2-pc/src/marker_ui/signup_poster_movie.rs`
   `set_mode_signup`, still prints "betting panel is not ported"): EnableBetting(true),
   `mBetting.mcBetArrows.amountText` = currency(bet), `totalAmountText` = currency(pot),
   `mArrowRight/Left._visible` = can_bet_more/less; EnableCash(true) with
   `mCash.totalbetText` = Cash_GetFormattedCashString (reuse `pause_movie::set_wallet`, make it
   pub(super)); without the minimum bet: amountText red 16711680, totalAmountText hidden,
   `mBetting.totalbetText` = `ID_SIGNUP_NOT_ENOUGH_CASH_FOR_BETTING` red; UpdatePositions order
   …mKilledIt, **mBetting** (y = mYOffset; += height+10), mMenuPicker, mEntryFee, mActivity,
   mView (577), **mCash** (y = mView.y − height − 10), mNISOption (525). Refresh the texts when
   `view.bet` changes (store the last BetView in `SignUpPosterMovie`).
2. Sweep diagnostic (`apps/skate2-pc/src/live_skating_client.rs` ~1266-1301): decline (B) a
   betting poster without the minimum bet and a fee poster without the cash; never sign up while a
   poster is open (the PC movie does not restart its intro for a replaced poster: the cause of
   "later posters never accept"); wait for `ai::can_start_main_challenge` before diagnostic
   sign-ups.
3. m_08_01_photo_team_shot_1 "challenge recipe None": run it first in a fresh sweep. Evidence so
   far: its Competitors entry is `chris_cole` (exists); native SelectCharacters `0x825c1420`
   returns early when AISkaterManager::HasChallengeAssetRequestOccured, so the sweep's bypass of
   IsAllowedToSignup/CanStartMainChallenge is the likely cause.
4. Training facility exit (task 4, report only): `m_08_01_teleport_trainingfacility_out` →
   `DIST_TrainingFacility_{0:SponsorWorld}`; needs GameBackEnd::RequiresAdditionalWorldOperation
   `0x8255b898` and a world switch behind the load screen; not ported.
5. Live verification, then `NO_SAMPLER=1 sh <scratch>/perf/parity.sh glob_parity` (expect
   "PARITY IDENTICAL (6000 lines)"; the new reevaluation calls and global runs only touch the
   career challenge system).

### Verify with (local perf scripts, not in the repository)

- Career start globals: `sh career_run.sh glob_start 1 90 SKATE2_TRACE_CAREER=1` and grep the
  log for `career graph service_security: enter`, `ots_discovery_sp_flow_manager`, HOM spots,
  teleporters (`c_3b_*`), go-tos, and location globals entering as you skate into a cap/drain box.
- Security guard: skate into a no-skate zone → `cMsgSecurityEnterNoSkate`, security HUD shown.
- OTS discovery: approach an OTS location → assets load, entering the boundary → 
  `cMsgEnterOTSChallengeBoundary`, sign-ups held 4 s.
- Betting: `SKATE2_DIAGNOSTIC_CASH=5000` sweep on `t_1a_01_1up_location`, `r_2_1_dr_ur_01`;
  fee popup on `c_3a_01_street_gvr` with $300.
- Quit to menu after a cap/teleporter loaded: the challenge hulls must unload.

### Known risks / errors

- Native soft-lock reproduced on purpose: after the fee-failed popup the poster takes no input
  (`ReceiveInput` sets mLocalInputState = Block at `0x82657da0`; CantPayFeeCallback `0x82641ef8`
  only clears mActionsBlocked (+0x48); only ExecutePushState/ExecutePopState `Show(true)` allow it
  again, and CheckForOptions refuses the pause menu while state 32 is stacked). Open question.
- Terminations are delivered when the challenge Lua state is free (start of update, after the box
  tests, before each flushed event, end of update), not synchronously inside the caller.
- Free skate (flags 2) natively runs the teleporter globals (GameType 2 flag); the port's free
  skate runs only the dynamic hull manager, so they still do not run there.
- Not ported: Sk8ChallengeSystem::IsSignupAvailable (`0x825a4488`, memcard/NIS), BE STATE_Loading
  check, `cMsgReevaluatePrereqActivities` to SimUnlockableManager, `ChallengeProgression.SetPlayersBet`
  (`0x82890c60`, no script calls it).
- Unit tests: none added or changed (the input map test `Pad` gained a `released` method only).

## Native evidence

- ReevaluatePrereqActivities `0x82cdbaa8`; ReevaluateGlobalActivtyBBoxes `0x82cdb260`
  (EvaluateActivitiesForPrereqs `0x82cd9e88` with `IgnorePrereqForGlobalStart`; flag 0x10 or
  online without GameType8 `0xf9a9a1dded84a643` drop newly passed; Global 1 → box at `Location`
  `0xf1c6d5fa4a23951e` ± `challenges` radius `0x8022ba64cb8e431d` (default 40, GenerateLocation
  ActivityBBox `0x82cd89c0`); Global 2 → Run + RunGlobalActivity, or the suspended list).
- PerformBBoxTests `0x82cdaf00` (tester valid; exits of running → TerminateMission + pending
  reset; all colliding → Run unless suspended/running, kEnableGlobalTriggers).
- cActivityManager::Update `0x82cdba20`: ProcessPendingResetList, PerformBBoxTests (GameLoaded),
  sub-controllers. GameLoad `0x82cd92c0`, GameUnload `0x82cda420` (ActivityMapping is
  `xfixed_map<u64, cActivity*>`, key order).
- StartMainChallenge `0x82cda588`, CompleteChallenge `0x82cd9d80`, TerminateMission
  `0x82ce4f80` (local `cMsgTerminateChallenge` 0x112AD959, empty table).
- Sk8ChallengeSystem::Update `0x825a3f10` tail (two ReevaluatePrereqActivities).
- SimUpdate1 `0x8289f410`: LayerMan::Update `0x8289f858`, cTriggerVolumeManager::Update, then the
  challenge system `0x8289f8b8`.
- Sign-up availability: `0x825a4488`, `0x825ad858`, `0x825ada38`, `0x825af878`, `0x825b2fc8`,
  `0x8289c660`; IsAllowedToSignup `0x82da2d60`; sign-up Update `0x82db44a0`.
- Betting sounds (fe keys): core_nav_right `0xD8EE38A61C45EA28`, core_error
  `0x47F720222F2CAED4`, core_nav_left `0xAD8BF9A0B4997B49`. Settings MinimumBet 500, MaximumBet
  5000, BettingIncrement 500; Competitors `0xFA185F4F092802AA`.
- Poster input: StateManagerFrontEnd::ReceiveInput `0x82656af8` (state input only while
  mLocalInputState Allow; then SetScreenInput when gFEStateInfo[32].mbInputToActionscript = 1);
  challenge_info UpdateInput `0x3960`, `mBettingRepsBeforeAutoSet` 5 (`0x55e4`).
- Fee popup template 203 (gPopupTemplate `0x82fdbc58`): ID_COMMON_WARNING_TITLE,
  ID_CHALLENGEINFO_FEE_FAILED, one option ID_COMMON_OK.
