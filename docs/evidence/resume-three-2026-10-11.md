# Resuming the races, activities and sponsor handoffs (2026-10-11)

Implementation and scoped live verification completed, solo. Live Jan17 rich-symbol IDA is read-only. Existing
uncommitted work is preserved. Scratch runs and build logs live under
`.scratch/resume-three-20261011`; real `saves/SKATER.P` is not a diagnostic target.

## Implementation

- Betting poster: original challenge_info SetMode/UpdateBettingArrows/
  UpdatePositions, wallet, insufficient-money text, arrow visibility and refresh.
  The original fee-failure input block is retained; no parity exception was requested.
- GVR results: original endchallenge_gvr movie (asset 84, fingerprint 3bce2a31),
  screen 43, team totals and the shared results list. Right/center autosizing uses
  AptCIH::EnsureStringAllocated 0x82a12220 (0x82a12540..1260c), with measured bounds.
  Results_GetGoofyTeamScoreRegTeamScore 0x826b9a90 reads the first two final scores.
- NIS borrows dynamic AI/party characters as well as the local player:
  SceneCharacterManager::InstanceCharacter(int) 0x828769f0,
  GetSkaterPresEntityFromPlayerId 0x82876aac, ControlCharacterRenderable 0x82870fb0.
  Presentation publication, borrowed bounds/hips and normal-submit exclusion follow
  the borrowed player ID. Verified in both race-win7 cutscenes below.
- Start hub Blog select opens the existing FE61 flow:
  IStartHubState::OnMenuSelect 0x82674d08, PushState61 at 0x82674f28.
  HoM screen46 and Annals were already implemented in the prior task.
- Sweep sign-up waits for no main challenge, AI release completion and no open
  poster/overlay. It declines unaffordable posters. Native CanStartMainChallenge
  0x82cd8798 and AI release gate 0x825addf8.
- AI name trace added. Original Global language table does contain Sammy, Mike,
  Chris Cole; expanding localization categories without evidence would not fix it.

## Photo save and coverage images

Native references:

- PhotoChallenge::ReceiveInput 0x82660048: Y hides the selection controls (-3,-10),
  blocks further input and requests a career photo. Replaying challenges and slot0
  finish immediately. Complete 0x8266c180 and failure 0x8266c1d8 both call PhotoDone.
- GetUniquePhotoChallengeID 0x825d0328, GetPhotoArticleChallengeForChallenge
  0x825ca080: signpost ChallengeID and the first matching progression photo collection.
- ScreenshotBackEnd::GrabScreenshot 0x82562588: full post-effect image and 160x120
  thumbnail. ResizeJPEG 0x82837198 -> ApplyResizeFilter_Bilinear 0x8282ca40 ->
  ApplySampleModeBilinearFour8BitUInt 0x82b1f640. Coordinates begin at -0.5, step by
  f32 size ratios, channels truncate; GetPixel 0x82afe008 clamps each edge.
- SetScreenShotAndThumbData 0x8274ad18: JPEG quality75, decrement10, capacities
  153600/15360. The PC JPEG codec is a host adapter; compressed bytes are not claimed
  identical to the Xbox codec. The portable store keeps JPEGs under
  `saves/career_photos` using the original slot IDs, rather than Xbox I/H/B containers.
- Original articles AS 0x2419..2423 requests mode4 thumbnails at
  mArticles.mArticleNN.mPhotoN.mImage. covers AS 0x1619..1627 requests mode5 full
  images at mPanel.mImage. Blog thumbnails also request mode4.
- JPEG::WriteJPEG 0x8282aa18 uses jpeg_set_defaults; YCbCr at 0x82b9a940 has
  2x2 luma and 1x1 chroma (4:2:0). The host encoder explicitly selects that mode.
  Codec API references: https://docs.rs/jpeg-encoder/0.7.1/jpeg_encoder/struct.Encoder.html
  and https://docs.rs/jpeg-decoder/0.3.2/jpeg_decoder/struct.Decoder.html.

## Training facility exit report (report-only handoff item)

The exit is not only a position teleport. The authored
`m_08_01_teleport_trainingfacility_out` targets
`DIST_TrainingFacility_{0:SponsorWorld}`. It needs the additional-world lifecycle:

1. GameBackEnd::RequiresAdditionalWorldOperation 0x8255b898 checks a pending
   loadAdditionalWorldLocation or unloadAdditionalLocation.
2. StateLoading::BringUpAdditionalWorldLoadScreen 0x82775558 pushes TELEPORT pause,
   requests the Tips loading screen and waits for it before proceeding.
3. UnloadAdditionalWorldStream 0x82774428 purges the additional world, clears its
   flag and audio location, waits for streaming release, resets AdditionalLocation
   to -1, then transitions to LoadAdditionalWorldStream 0x82772680.
4. RequiresAdditionalWorldFile 0x827494f8 follows the challenge's location reference
   and checks fe_locations.WorldFile against BAM.

The port's teleport path does not implement that world replacement transaction.
Moving the actor alone cannot complete this exit correctly. No world-switch
implementation is included, following the explicit report-only handoff scope.

## Verification log

- cargo check3: core compiled; PC found the appended movie array needed 85 entries.
  Corrected before release build1.
- Release build1 passed, 12m31s (pre-photo-capture integration).
- Photo integration check4 found two PC compile errors (GpuScene inspection literal,
  conditional temporary slice); corrected. Check5 passed. Release build2 pending.
- race-win1/2/3: Global activities start (security, OTS discovery, HoM spots), and
  AI name trace resolves Shingo/Freeway/Sammy correctly. Runs ended at the harness's
  25,000 rendered-frame cap during the start NIS. The limit is independent of the
  career completion tick. cdb on run3 saw normal exit0 and no AV/stack overflow.
  Launcher cap raised to 1,000,000; run4 pending. These early exits do not verify
  the heat/results and are not treated as game failures.
- GVR InitAction verified directly in original .apt: at 0x1400, control8,
  character53, action pointer0x21e0 (big-endian).
- Initial real save SHA256:
  C2B071DA246EF664ED1353D8A1244C330F45A03296EA27ABC4F7D5C73118D297.

## Follow-up findings from live runs

- `race-win4` reached `run_heat`, crossed the real gates via the bounded diagnostic,
  won, and entered `RaceEnd_TiredSkaters`. It then stalled: the port logged and
  discarded `NisEvent::OpenResultsScreen`. Fixed FE state 79 -> 8 from
  FrontEndState_NIS::OnEnterState 0x826675a0/0x82667628 (win=true, no throwdown,
  offline, openedByNIS). FE8 OnLeave 0x82658b48/0x82658bbc..0x82658c08 forces
  SkipNIS(true), pops NIS, clears openedByNIS; OnOutroDone 0x826590c8 sends
  HudContinue after the results movie's outro. Awaiting rebuilt runtime check.
- The same run proved the blank-name cause: Shingo and Freeway were localized and
  written, then queued ambient Unspawn messages for slots1/2 removed their newly
  created backend records. Sammy in slot3 survived. AISkaterManager::UnspawnAISkater
  0x824cbc10 removes its backend row immediately at 0x824cbd08; only actor destruction
  waits for the message. Moved that backend removal to post(Unspawn), removing the
  second removal from message dispatch. This preserves all replacement metadata.
- Race time-bonus messages showed literal `{0:s}`. The session was dropping each
  CentralMessage's four arguments. They now survive into CentralMovie and the typed
  Localizer formatter, matching ShowCentralMessage 0x826bbf3c..0x826bbf60.
- `sponsors1` completed all four choices: Girl, Krux, Hubba, Fallen. Each overlay
  returned to gameplay. Captures043 and046 show shoe navigation and Yes confirmation.
  Exit0 at the intended frontend-tick bound. This verifies in-memory settings and
  outro behavior; this injected-message run did not request a profile save.
- `coverage1` unlocked the authored SBM cover and Thrasher article, reached FE61,
  then failed on `untranslated Localizer argument type PLAYER`. ProcessPlayerName
  0x8255a4d0 reads ProfileBackEnd +0x28 (tProfileData.mProfileName, confirmed UDT/raw
  PPC). Connected the profile name and Board/Truck/Wheel/Shoe custom tokens to the
  shared formatter. Sponsor token callbacks 0x82562e80..0x82563320 use
  GetSponsorName 0x825d13e8, selected sponsor message setting -> layout +8 HAL,
  missing -> ID_COMMON_UNKNOWN. Awaiting rebuilt runtime check.
- Release build2 and checks6/7/8 encountered Windows memory exhaustion, not a
  Rust type error. With other programs closed, build3 is running with one compiler
  job and no simultaneous game. Scratch launches now copy the executable into a
  unique bin directory, so they cannot lock the release linker output.
- Added opt-in named/coordinate diagnostic teleports to exercise normal global
  activity boxes, and manual-poster mode to inspect betting input without automatic
  A presses. These diagnostics do not run without their environment flags.

## Presentation admission follow-up

- Release build3 passed (12m43s). race-win5 then failed before the heat with
  Material(EntityReplacementPending); photo-team1 reached the same failure in
  the front end. This exposed a missing producer gate after the backend lifetime
  fix, not an unknown recipe substitution.
- Live IDA: SubmittableSkater::ShouldSubmit 0x825f0008 returns player8 OR
  PlayersBackEnd::GetPlayerType(player) != PT_None. DoSimSubmit 0x827dd3c0..3e8
  calls it and additionally excludes player8 from ordinary Actor submissions.
  The port previously submitted every retained Actor, including AI rows already
  removed before queued destruction. Added the original admission gate before
  WriteData, clearing unpublished output; callers accept an absent submission.
- UnspawnAISkater's SendMsg<cMsgUnspawnAi> 0x824c7998 queues on GameSim
  (Enqueue at0x824c79f8); backend removal remains immediate at0x824cbd08.
  No generic peer/recipe substitution or swallowed material error was added.
- Release build4 passed (13m14s), one compiler and no concurrent game.

## Build4 live verification

- race-win7: exit0 at frontend8500. All three AI render in start NIS (capture014)
  and the results NIS (030); Shingo/Freeway/Sammy appear in the live leaderboard
  (023) and screen41 layout3 (033). Time-bonus arguments render correctly (023).
  Continue3584 -> complete3593 -> unlock3672 -> A3735 -> autosave3736 and gameplay.
- race-win6 was stopped intentionally: one-second screenshot readbacks caused
  diagnostic capture backlog. Subsequent runs use two-second captures.
- The team photo challenge has no progression photo slot. It uses demo-sponsor
  competitors (see prerequisite findings below); m_2e_01_photo_tm_burnette
  (slot14/article5) checks JPEG save.

### Team-photo diagnostic prerequisites

- photo-team2 reproduces recipe None on the fresh starter profile. The original
  mission has UseDemoSponsorCompetitors=true, so its authored script calls
  ChallengeProgression.SelectDemoCompetitors rather than selecting its literal
  Chris Cole competitor. LoadSkaterModels82e0e990 deliberately skips this flag.
- Live IDA SelectDemoCompetitors82891b18 passes (false,noAssetLoad) to
  SelectDemoCharacters825c1740; GetDemoCompetitorCount825d15c0 returns zero
  without a Pro Beaten setting. The starter profile lacks that prerequisite.
  Original throwdown data: chris_cole -> [chris_cole]; mike_carroll ->
  [mike_carroll,eric_koston]. No character fallback belongs in production code.
- Rerun uses the sponsor-selection save to supply valid progression.

- sponsors-save1: all four choices completed again. Start hub Quit wrote the
  profile, reached the main menu and entered Free Skate without the prior
  pedestrian PRES teardown error. Read-only save parsing against original
  setting-definition keys confirms Girl/Krux/Hubba/Fallen and Pro Beaten=
  mike_carroll persisted with their exact GetHashValue32 values.

- photo-team3 reloads the sponsor save, selects Mike Carroll and spawns his
  requested recipe successfully (slot1,1541..1554), then enters challenge_running.
  Stopped deliberately after this check: the mission is outside BAM. Original
  RequiresAdditionalWorldFile827494f8 follows 24491bf00316aa45 then
  6c814db59f16ebd2 to fe_locations c202f670c737a38f, whose WorldFile is
  DIST_TrainingFacility_{0:SponsorWorld}. Full gameplay requires the report-only
  additional-world loader. No native character fallback was added.
- photo-save1 stopped deliberately after its one-shot diagnostic completed only
  the first of three objectives. Capture check changed to m_2d_01 (slot13, one
  objective); this is a diagnostic constraint, not a production scoring fix.
- coverage2 confirms PLAYER formatting on the Blog. Completing five unrelated
  missions at once queues overlapping reward screens; stopped that invalid UI
  setup. Rerun completes each photo separately and closes each reward screen.

- photo-save2: normal bounded exit0; the single photo objective was outside its
  scoring volume at the start locator, so the diagnostic only completed the
  points objective. No save attempted in this run.
- photo-save3: m_2e_01 with the existing force-objectives diagnostic completing
  its two remaining objectives after the highlighted first one. Original flow
  enters display_photo2441, photo selection3264 (8 images), Y saves slot14,
  results ready3392, Continue3420, complete3430, rewards3493, autosave3556.
  Exit0 at frontend6500. Full JPEG107546 bytes (1280x720), thumbnail3969 bytes
  (160x120); inspected both, skater/pool visible and no front-end controls.
  coverage3 reloads this profile plus its JPEGs for image/navigation checks.

- coverage3: exit0, sequential progression rewards and autosaves passed. Reloaded
  slot14 JPEG appears in the Blog thumbnail, Thrasher article, and full viewer
  (captures054/069/076). RT wraps4->1 and LT1->4; B returns through article/Blog
  to gameplay. SBM single-photo cover (slot12 intentionally absent in this
  diagnostic profile) renders authored cover geometry (123), then B returns
  and autosaves. Quit saves4170 and reaches menu/Free Skate without teardown
  failure. Pause-hub sequence selected Edit Skater: default selection is Map,
  so it needed two Down inputs, not three. Separate focused Blog check queued.

- betting1 visual checkpoints: $500/$2000 minimum with only right arrow (010),
  single-press $1000/$4000 with both arrows (071), held Right reaches native
  maximum $5000/$20000 with only left arrow (082); wallet remains $5470.
  Insufficient-funds configs use AddCash(-170) on the $470 starter, producing
  $300 rather than mistakenly adding $300. Pending captures slowed to4seconds
  to avoid the screenshot scheduler accumulating overdue GPU readbacks.

- betting1 completed normally (exit0); B unpins the player and unloads the
  challenge hulls. Rechecked native BetMore82626b98 (strict cash comparison),
  CantPayFeeCallback82641ef8 (only clears +0x48), DeductChallengeLosses825de1e0
  and IssueChallengeAwards825ddfc8 against the current source. Loss deducts
  the stake; win credits pot minus stake; both clear the active bet.

- betting-low1: exit0. Capture015 shows wallet$300, minimum$500 in red,
  insufficient-cash text, no pot/arrows or Accept help. A3000 does not start
  the challenge; B3400 unpins the player and unloads both challenge hulls.

- fee-low1: exit0 at frontend3200. GVR fee$5000 with wallet$300 opens popup
  at1801. After dismissal, A2200/B2600 cannot leave or accept the poster;
  no challenge starts and the player stays pinned, matching the retained
  native local-input block. This is a known native soft-lock, not a port fix.

- race-quit1: exit0 at frontend4200. run_heat2903, native abandon message3023,
  main challenge None3024, challenge hulls and racers released, continued
  gameplay with no queued Actor/PRES failure. This directly exercises the
  QuitChallenge message; the pause-menu confirmation itself was not driven.

- route-globals1: exit0, normal global start of security, OTS discovery and
  four HoM activities; named-location teleports trigger City Hall cap services,
  pink-motel drain and Matrix fountain service. Leaving areas terminates/resets
  their scripts. Quit8000 autosaves470, unloads activities, enters the menu and
  Free Skate without the pedestrian/PRES failure. Stadium/go-to did not start
  on this profile; their Global=2 full-global mode still requires progression
  availability (IgnorePrereqForGlobalStart absent/false), not just position.
- hub-blog1: Blog opens from the proper row, but capture007 exposed the hub
  remaining behind it; B also retained the old Blog movie while the paused
  host skipped its normal career update. Native UpdateOnLeave82d66300,
  StartHub::OnLeave8264c4d8 -> CloseScreen, ExecutePopState82d672f8. PC fix:
  hide/close hub while Slappys/Annals own the front end; feed the fresh career
  view to paused rendering so the Blog closes and hub reopens on return.
  Build/runtime follow-up pending; no new core or physics changes.

- contest1: reached Mountain Mayhem round results with all ranks/names/scores,
  but Next logged unported Page(13); stopped after capture027. Implemented
  OptionSelect82676da0 Next/Previous (82677060/826770cc), retained screen
  SetState6 from both original APT movies, once-only overall announcer
  PlayerOverallResultHandler825007b8 (leader default100 at8250e39c), and round
  intro sounds d8f50a95/735f373f from original fe collections. GVR speech is
  Cyrus36 with466/467: raw82657a18/24, message9703d2a4 matches
  cMsgPlayMissionSpeech::MessageDefinition8216b4d8, despite folded symbol.
- gvr1: five AI spawned, then PRES initialization hit DatabaseCapacity.
  Native DataBaseManager82d79260 has32 slots; retained that capacity. Native
  Presentation::Init827f10c0 constructs ONE SkaterPresentation animation owner
  (Init827e4098, InitAnimation827e1c08) with nine ACS streams. Port erroneously
  initialized two global PRES databases plus four temporary ones per AI.
  Later entity wrappers now borrow primary PRES managers/playbacks/databases,
  retain their per-entity pose buffers, and do not tear down global resources
  on actor removal. Awaiting build and live contest/GVR checks.
- parity1 on build4: exit0, all6000 selected state/board/offboard/manual lines
  match the prior PORT regression baseline. This is not native parity proof.

- Release build5 passed (12m16s); incremental confirmation exited0 (1s).
  Includes shared PRES ownership, Next/Previous and result audio, plus paused
  Blog movie lifetime. Follow-up batch hub-blog2/gvr2/contest2/parity2 running.

- hub-blog2 build5: exit0, no pause hub behind Blog (captures005/007), no
  stale Blog after exit, Quit->menu/Free Skate still succeeds. Found a second
  issue: the closing B fell through to the still-visible parent hub in the same
  input update, closing both. Source fix uses existing PauseMenu::reenter
  (ExecutePopState82d672f8/StartHub::OnEnter8265e4d0), consumes the key once
  (ReceiveInput82656af8), and retains the parent's FRONTEND pause. Awaiting
  next build; GVR/contest build5 checks continue independently.

- gvr2 build5 passed five-AI spawn, intro, gameplay, first/second round results
  and Continue. Capture029 revealed score columns and headings shifted right.
  Fixed missing numberbox/textbox SetValue's non-left `_width = fixedWidth`
  (AS numberbox0xdc0/0xe0c), which native TextMembersIndex wordlist83001918
  maps to20 -> objectMemberSet82a16b5c: changes text bounds, not glyph scale.
  EnsureStringAllocated82a12540 then shrinks it and preserves the right edge.
  Title autosize likewise must occur after CreateTitles1217..12c2's `_x`
  writes, as native allocation is lazy. Awaiting rebuild/visual verification.

- gvr2 completed all three rounds (results8017/14255/20460), final loss
  screen20655 and Retry20870 -> confirmation20931 -> run_round21691. Stopped
  the owned process deliberately after restart; exit-1 is not a game failure.
  Score alignment correction above is still unbuilt.
- contest2 build5: Mountain Mayhem rounds1/2 pass Next -> overall, Previous
  -> round, Next -> overall and Continue -> following round. Overall audio
  emits once per screen lifetime, including when revisiting overall. Captures
  025/026/027 show correct points/round scores and A/B help. Final pending.
- contest2 completed round3 results20775, all page transitions, final loss
  screen21510, Retry21727 -> Yes21788 -> run_round22409. Stopped deliberately
  after retry verification (owned PID242680). It did not exit from a failure.
  Final Quit uses the shared EndOptions path and is queued for gvr3.

- parity2 on build5: exit0, all6000 selected state/board/offboard/manual
  lines identical to the prior port baseline. Final release build6 started
  with one compiler job and no game process.

## Scope and evidence limits

- Confidence: high for reconstructed control flow and UI behavior, grounded in
  live rich-symbol IDA, raw instructions where folded symbols were misleading,
  and the original APT/Lua/Vault data. No instruction-level parity claim.
- The 6000-line comparisons are against an earlier port build, not an Xbox
  capture. They detect changes to that skating baseline only.
- Live diagnostics use copied profiles. Forced challenge completion, diagnostic
  race gate teleports and injected progression messages exercise the actual
  downstream game paths; they do not prove every objective or unlock condition.
- The team-photo starter-profile failure required its original Pro Beaten
  prerequisite. A valid sponsor profile successfully selected/spawned Mike
  Carroll. Full training-facility gameplay awaits the report-only additional
  world loader; no fabricated character or position-only world exit was added.
- Additional-world loading, the separate full Replay Editor, Free Skate's wider
  career activity host, and unrelated mission speech variants remain outside
  this continuation's implemented paths. Contest overall standings speech and
  round-result UI sounds are connected; not every contest audio event is ported.
- No unit test suite was added or run. The final release and named live runs
  are the validation reported here. JPEG encoding and storage are host adapters
  with the native dimensions, slot identities, quality/size policy and resize
  procedure; compressed files need not be byte-identical to the console codec.

## Final build and follow-ups

- Release build6 passed (exit0,10m31s), one compiler/no concurrent game.
  Includes fixed-width results cells/lazy title autosize and Blog input
  consumption/parent reentry. Final batch hub-blog3/gvr3/parity3 running.
- hub-blog3 build6 visual check passed: capture007 Blog without hub behind it;
  008/009 after B retain the pause hub and FRONTEND pause, reset selection to
  Challenge Map per StartHub::OnEnter; 010 after the second B returns to gameplay.
  No stale Blog movie and no double consumption of the closing key.
- hub-blog3 completed normally (exit0, frontend7000). Quit5000 saved the
  copied profile, returned to the main menu and entered Free Skate without
  the old pedestrian PRES failure. gvr3 started after this process exited.
- gvr3 build6 capture029 verifies fixed placement on the FIRST results screen:
  both Score headings, selected-row scores1483/1586, other scores, and totals
  2403/4173 align inside their native columns. Round1 Continue -> run_round8865.
  Five AI and shared PRES databases remain live without DatabaseCapacity.
- gvr3 round2 results14266/ready14274, Continue14454 -> round3 run15077.
  Capture056 confirms the selected/other rows and team totals remain aligned
  after a new round and changed rankings (3140/5662 totals).
- gvr3 completed round3 results20478/ready20486; Continue20666 -> final
  loss20676/ready20712. B Quit20892 directly closes this end screen; main
  challenge None20902 and gameplay resumes. Capture083 has aligned final
  totals6276/10368,084 loss screen45,087 normal HUD with racers released.
  The final end-screen Quit has no confirmation; Retry confirmation was
  exercised by gvr2 and contest2.

- gvr3 completed normally (exit0, frontend25500). Final Quit returned to
  gameplay, then StartHub Quit23500 autosaved and reached the main menu and
  Free Skate. No database-capacity, stale-AI, pedestrian/PRES or UI failure.
- The saved $25,470 after a lost GVR is native placement money, not a win
  misclassification: AwardPrizeCash825de2d0 pays the improvement of the
  place's economy value. Original challenge1106 economy rowee799ad6c6c7a034
  pays $25,000 for second, entry fee rowf1a4f9df4a3b34c1 is $5,000. The
  original complete-handlerf5b763e7a2a60330 calls UpdateCashAward(LocalRank)
  for loss as well as win. $470 + diagnostic$5,000 - fee$5,000 +$25,000.
- parity3 on final build6: exit0, all6000 selected state/board/offboard/manual
  lines identical to the prior PORT baseline. No native trace equivalence
  is claimed. No compiler or diagnostic game remains running.

## Completion

The three handoffs' implementation and verification follow-ups are complete
within the stated scope. The training-facility exit remains the requested
report-only item. Release executable: `target/release/skate2-pc.exe`.
Original handoffs, AGENTS, STATUS and RESEARCH_LOG point here. Real save
SHA256 remains C2B071DA246EF664ED1353D8A1244C330F45A03296EA27ABC4F7D5C73118D297.
