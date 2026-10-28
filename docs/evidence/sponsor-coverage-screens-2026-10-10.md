# Sponsor overlay, magazine coverage screens, several-challenges popup (2026-10-10)

## Continuation on 2026-10-11

All sponsor choices and persistence, photo JPEG capture/reload, magazine
coverage navigation and the pause-hub Blog return are verified. Annals was
completed in the earlier HoM task. Release build6 passed.
See [continuation evidence](resume-three-2026-10-11.md) for changes, native
references, run results and remaining scope limits. The original handoff below
is historical.

## Resume here

**Done and verified live**
- Several-challenges popup on the real `sk8popup` frame (rows, `>>` highlight, wrap, sounds, "A Select
  B Cancel", silent B): `nbs_pop7` captures 095/098, `nbs_pop9` 038–044.
- Sponsor overlay (FE 11) screen, shoe state: title, description, 3×3 logos, highlighter, slider,
  "A Proceed", spin, window scroll with Down, the confirmation popup 177: `nbs_spon2` captures 034–050.

**Written, built, not verified live**
- Sponsor board/truck/wheel states, `NextSelected` (settings `Board/Truck/Wheel/Shoe Sponsor`,
  `Board Sponsor Accepted`, `Sponsorships Earned`), outro → `cMsgHUDContinue` (`nbs_spon2` stalled on
  the confirmation popup because `SKATE2_DIAGNOSTIC_CAREER_YES` pressed Up/A into it).
- Everything of section 3 (coverage screens) and the career-relative diagnostics
  (`SKATE2_DIAGNOSTIC_COMPLETE_CHALLENGES`, `_SIGNPOST_MESSAGE`, `_CAREER_TRIGGERS`,
  `_CAREER_PAD_AFTER`): the release build of 11:53 compiles all of it; no run yet.

**Half-done / open**
- Photos: `LoadMemcardTexture` (blog thumbnails, article photos, covers) has no photo-slot store in
  the port; the image renderers keep their authored look.
- `FrontEndState_AnnalsOfMeat` (77) from the blog panel's third row is not ported (prints a line).
- The start hub's Slappy's Blog row (pause hub agent's files) does not open FE 61 yet; call
  `CareerSession::open_slappys_blog(false, male, profile)` from the hub's select.
- Unrun tests: `ui/sponsor_browser.rs` (2), `ui/slappys_blog.rs` (2), `challenge/tests.rs`
  `sign_up_at_indicator` assertion, `selected_pause_manager_tests.rs` `is_paused_reads_the_top_state_entry`.

**Next steps, in order**
1. Rebuild only if the tree changed since 11:53 (`cargo build --release -p skate2-pc`, gated).
2. Sponsor run (career-relative ticks; the "New to skate?" popup is answered by CAREER_YES at about
   +600, so start after +1500):
   `sh <scratch>/run_gated.sh nbs_spon3 30000 260 SKATE2_DIAGNOSTIC_CAREER_YES=1
   "SKATE2_DIAGNOSTIC_SIGNPOST_SETTING=Pro Beaten=mike_carroll"
   "SKATE2_DIAGNOSTIC_SIGNPOST_MESSAGE=2000:Show Sponsorship Screen - Board;3000:Show Sponsorship Screen - Truck;4000:Show Sponsorship Screen - Shoe"
   "SKATE2_DIAGNOSTIC_CAREER_PAD_AFTER=2300-2302:1000;3250-3252:0008;3300-3302:0002;3400-3402:1000;3460-3462:0002;3520-3522:1000;4250-4252:0002;4300-4302:0002;4350-4352:0002;4400-4402:0008;4500-4502:1000;4560-4562:0001;4620-4622:1000"
   SKATE2_DIAGNOSTIC_CAPTURE_FROM=40 SKATE2_DIAGNOSTIC_CAPTURE_EVERY=1.5 SKATE2_DIAGNOSTIC_CAPTURE_COUNT=150`.
   Check `career: sponsor overlay state N sets X Sponsor to …`, `career output: … Board Sponsor
   Accepted`, the board state's logo and description, `HudContinue` after each outro.
3. Coverage run: `SKATE2_DIAGNOSTIC_COMPLETE_CHALLENGES=2000:m_7c_03_photo_cover_sm_3,m_2e_01_photo_tm_burnette,m_2e_02_photo_tm_burnett,m_2e_03_photo_tm_burnett,m_2e_04_photo_tm_burnett`,
   `SKATE2_DIAGNOSTIC_SIGNPOST_MESSAGE=2100:Show Unlocks Screen`, A at +2900 (unlock overlay →
   `SaveAndClose` → FE 61), then pads per the `career: Slappy's blog mode …` trace (Down/Up/A on a
   photo row → article or cover; Right/Down/A in the article; `SKATE2_DIAGNOSTIC_CAREER_TRIGGERS=…:r`
   and `:l` for the cover's photos; B back; Left, Down, A on the panel; B out → `HudContinue`).
4. Parity: `NO_SAMPLER=1 sh <scratch>/perf/parity.sh <name>` gated (must print `PARITY IDENTICAL`).

**Known errors fixed on the way**: "unknown UI texture owner" (owner table lacked 75–79);
`apply_career_fe_background` visibility.

## 1. Several-challenges popup

Native: `MultipleSignupsCallback` 0x826b5430; FE 34 `FrontEndState_Popup::OnEnter` 0x826583d0;
`sk8popup::Refresh` (description `#` → no body, `mMenuPickerOnlySpacing` 15); `sk8popup::UpdateInput`
(B cancellable → `OnMenuSelect(0,-1,"CANCEL!")` without a sound; `menu_picker` wraps, sounds
`FE_SOUND_CORE_NAV_UP/DOWN` on a change; A `FE_SOUND_CORE_A_BUTTON`).
Port: `marker_ui/progression_popup.rs` (`update_sign_up_choice`), `edit_skater_movie.rs`
`configure_popup` (`#` layout), `marker_ui.rs` (text list removed), `selected_session_career.rs`
(wrap, sounds, silent cancel).

## 2. Sponsor overlay (FE state 11)

- Open: `cSignPostUtil::ShowUnlocksScreen` 0x825cca80 → `cMsgOpenSponsorOverlay` (SponsorID > 0:
  Board 1, Truck 2, Wheel 3, Shoe 4) → `ReceiveOpenSponsorOverlayMsg` 0x82614450 (`mActiveState`,
  `PushState(11)`). `gFEStateInfo[11]` 0x82fdee5c: pause, HUD hidden, music 4, input to AS.
- State: constructor 0x82642468 (sponsor lists by type, collection walk, no `default`, shoes skip
  `shoebigblack/shoenike/shoenixon`), `OnEnter` 0x826427d0 (screen 21, placement 4 spin),
  `ReceiveInput` 0x82674800 (A: `NextSelected` for tape/board, else popup 177 No/Yes),
  `SponsorSelectedCallback` 0x8266ff20, `NextSelected` 0x82667fa0, `OnLeave` 0x826427e8,
  `OnOutroDone` 0x82659920 (`cMsgHUDContinue`). Natives: `Sponsor_GetInitialState` 0x82624208,
  `Sponsor_GetBoardSponsorName` 0x8260d760 (`GetBoardSponsorHALID` 0x825d1358),
  `Sponsor_GetBoardSponsorIcon` 0x82624070 (`SetBoardSponsorIndex` 0x82642a68,
  `GetBoardSponsorSwf` 0x82642808), `Sponsor_GetTruckSponsorSwfs` 0x826242f0, `SetIndexPicked`
  0x82624518.
- Screen: sponsoroverlay.apt (InitAction char 40 0x1f7c; root 0x1f60; IntroComplete 0x2470,
  OutroComplete 0x249c; `FE_SOUND_SPONSOR_OVERLAY_INTRO` 0x2558/0x25a4/0x25e8) and
  sponsor_browser.apt (3×3, `MAX_SPONSORS` 9, `SliderLength` 380, `FE_SOUND_SPONSOR_NAV`,
  highlighter char 36 `FE_SOUND_SCRATCH_BOX_FX` 0x2f6c, thumbnails `_alpha` 100/40).
- Port files: core `challenge/sponsor_overlay.rs`, `ui/sponsor_browser.rs`,
  `session/selected_session_career_sponsor.rs`; PC `marker_ui/sponsor_overlay_movie.rs`, assets
  75/76 (139c3d55, 4f6cce94) and `Assets::ensure_rx2_image` (logos
  `fe\source\images\sponsor_logos\<name>`). Before: the output was printed and dropped.

## 3. Magazine coverage screens (FE states 61, 62, 63)

- Unlock: `ChallengeComplete` 0x825e00f8 → `UpdatePhotoArticlesBLOGS` 0x825d91a0
  (`GetPhotoArticleCountForChallenge` 0x825c9e48, `GetPhotoArticleForChallenge` 0x825c9f40,
  `GetPhotoArticleKeyFromID` 0x825c9980, `AllPhotosComplete` 0x825d01c0): a new photo-article BLOG
  entry sets `CoverageUnlocked`, `PickBLOGPhotoCaptions` 0x825c8168 (Simulation RNG), and
  `photoArticle`. `FrontEndState_Unlock::SaveAndClose` 0x82659468 → FE 61 with
  `mUseHudContinueMessage`. Before: never unlocked; `OpenCoverageScreen` sent `HudContinue` at once.
- FE 61 `SlappysBlog`: `OnEnter` 0x8268fc80 (screen 125), `OnLoaded` 0x826b1308, `PushScreenState`
  0x826b0940, `PopScreenState` 0x826b09c8, `UpdateBlogList` 0x826ac3e8 (modes 0 all, 1 Thrasher,
  2 SBM; newest first), `ReceiveInput` 0x826b1318 (B: pop mode, or HUDContinue + PopState),
  `OnMenuSelect` 0x826b1428 (panel: modes, Annals of Meat 77 or `ID_ANNALSOFMEAT_NOTYETUNLOCKED`;
  list: one photo → covers, else articles), `OnLeave` 0x82693480 (`ResetNewBLOGCount` 0x825c7d70),
  `GetItemText` 0x826a66d0, `GetGenderizedHALID` 0x826a1910, `GetPhotoSlot` 0x82693630.
- FE 62 `SlappysArticles`: `OnEnter` 0x8268fcd8, `ReceiveInput` 0x8269d298, `OnMenuSelect`
  0x8269d2f8, `GetNumItems` 0x826937f8 (`GetArticlePhotoCount` 0x825c5a30),
  `GetPhotoSlot` 0x826938e0 (`GetArticlePhotoChallenge` 0x825cfdc0), `GetArticleText` 0x826a1c80.
- FE 63 `SlappysCovers`: `OnEnter` 0x8269d350 (`EnableBackground` 0x826066c8 snap), `OnUpdate`
  0x8269d3b0, `UpdateCover` 0x8269d428, `NextPhoto` 0x826a1d58 / `PrevPhoto` 0x826a1de0 (triggers),
  `GetButtonHelpTextTrigger` 0x8268fd60, `GetBLOGPhotoCaption` 0x825c8498.
- Rows 61–63 (0x82fdf244): pause, dimmer, HUD hidden, music 9, input to AS. PushState leaves the
  state below (`ExecutePushState` 0x82d694e8 → `UpdateOnLeave` 0x82d66300), pop re-enters
  (`ExecutePopState` 0x82d672f8), so the blog reopens on Slappy's page.
- Port files: core `challenge/photo_articles.rs`, `challenge/slappys_blog.rs`, `ui/slappys_blog.rs`,
  `session/selected_session_career_blog.rs`; edits in `signpost.rs`, `signpost_progress.rs`
  (`add_photo_article_blog`), `system.rs`, `ui/frontend_fade.rs` (`fading`), facade (fade hook),
  `selected_session_career.rs`; PC `marker_ui/slappys_movie.rs`, assets 77–79 (632b87a9,
  d6632366, ad9d0792), dimmer and HUD hiding, acknowledgement.
