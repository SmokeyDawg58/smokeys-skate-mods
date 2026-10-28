# Photo mission intro NIS black: the challenge fade (2026-10-10)

The first photo mission's cutscene (`m_1a_05_photo_rail`, Atiba explaining
the mission) was completely black while the frame counter ran. All addresses are `sk82_na_m.xex`
(Jan 17 2009) VAs, read in IDA (read-only).

## Reproduction

`career_run.sh` with the photo recipe (sign-up of `m_1a_05_photo_rail` at tick 11100) and captures
every 0.5 s from 180 s (run `nb_r2`): the poster, the world dimming with the sign-up, then 12.5 s
of pure black (every pixel 0) from the poster's close, through the whole
`tutorial_m_1a_05_photo_rail` NIS and the challenge's setup, until the fade-in at the start
locator. `SKATE2_TRACE_FRONTEND_FADE` printed nothing in that window (the front-end fade was 0),
and a tutorial-intro scene takes no breadcrumb teleport (`TeleportToLastBreadCrumb` 0x828a0db8 only
for story and go-to scenes), so none of the 2026-10-09 changes (breadcrumb teleport, area loads,
front-end fade and travel, screen manager and dimmer, movie layering) was involved.

## Cause

The port drew the challenge effects' fade (`cMsgFadeAmount`) as a black quad over the whole
screen (`marker_ui.rs` `fade_batch`, from `CareerHud::fade`), NIS or not.
`cChallengeEffectsManager::OnChallengeAccept` 0x82da83e8 starts a 0.5 s `FadeOut` for every
challenge but Own the Spot (type 9) and hides the HUD (`eHUDVisibilityFlag_ChallengeEffects`);
`Update` 0x82daa1a0 sends the fade amount every update while an effect runs, and a finished
`FadeOut` stays persistent, sending 1.0, until a `FadeIn` replaces it (here the countdown's). The
photo template's `nis_overview` state plays `tutorial_m_1a_05_photo_rail` inside that window, so
the whole scene sat under a fully black quad. `m_1a_05` is the first challenge whose sign-up plays
an intro NIS (`m_1a_03`'s goes straight to setup), which is why nothing earlier showed it.

## Native

### The fade's path

- `VDWriter::ReceiveFadeAmountMsg` 0x827fc758 keeps the message's amount; `VDWriter::WriteData`
  0x827ff8c8 writes it into the simulation frame when it is not negative (non-PB location 12) and
  resets it to -1, so a frame carries fade info only while a fade runs.
- `VDReader::ReadData` 0x827fdd78 restores it into `VDReaderData::bFadeInfo` (+0x31a0) and
  `sFadeInfo.mNormalizedFadeValue` (+0x31a4); `VDReaderData::Reset` 0x827fcac0 clears the flag.
  `kPAUSE_STATE_NIS` sets `kPAUSE_FLAG_DATASYSRESTORE`, so `GameModule::Render` 0x827320f0 skips
  `RestoreFrame` and a NIS keeps the last restored values (here 1.0).
- Readers (a scan of every `.text` load at those offsets and at +0x31b0/+0x31b4 from the reader's
  owner): `PostFX::VisualFX_Animation` 0x82847610 (0x828485dc, 0x828485e8),
  `AptRenderingIntegration::SetColourTransform` 0x82609488 and the audio's
  `SFXObj_Bloom::CheckForBloom` 0x8253b698 (ported as the audio bloom). Nothing draws a black
  quad.

### `PostFX::VisualFX_Animation` 0x82847610

Each render, from `PostFX::SetParameters` 0x8284bbf0 (call 0x8284bd28):

1. `m_bloomMultiply` = (1, 1, 0, 0), `m_VignetteUVOffset` = (0, 0.25, 0, 0), `m_OverlayAdd` = 0
   (0x82847654..0x8284768c); threshold and saturation from `m_NEWbloom` (+0, +4); the near and far
   grades of `m_NEWcolour_matrix` through `ApplyFX_NEW` into locals.
2. The first of: the tazer effect; Hall of Meat (`m_bHOMEnabled`, `bIsBailForRender`,
   `mIsHoMEffectEnabled`); challenge success; challenge failure; then the fade
   (0x828485dc..0x82848650): threshold -= the fade amount; when the result is below zero,
   `m_bloomMultiply.x` += min(-threshold * 0.5, 0.5). Neither grade is written. Only the branch with
   no effect writes the grades (`m_FXMatrixR/G/B_A`, `_B`), `m_NISNearDist`/`m_NISFarDist` and
   `m_bloomMultiply` = (fine, medium, coarse, 0) from `m_NEWbloom` +0x10, +0xc, +0x8.
3. Tail: threshold -= 2 * `bloomToWhiteValue` (the ICE general block while `mICEPostfxActive`,
   else the game's), `SetParameters_MotionBlur`, `SetBloomCoeefs(threshold, saturation)`.

`GenerateBloomBuffers` 0x82832c68 multiplies the medium blur by `m_bloomMultiply.y` (+0x184), the
coarse ones by `.z` (+0x188) and hands `.x` to `Blur9_combined3`. A threshold below zero makes the
first bloom coefficient positive: the extract adds it to every pixel, `PostFX::Apply`
0x8284c92c..0x8284cafc moves the member grades toward the identity by min(coefficient, 1) in
place, and the vignette lifts. With the fade at 1 the screen blooms to white: the challenge fade
is a bloom, not a black fade.

### The NIS shows none of it

With `mICEPostfxActive` (Presentation +0x7c30, from `Scene::Init` to `Scene::Purge`)
`SetParameters` takes its ICE path after `VisualFX_Animation` (0x8284bff8): `ApplyFX_NEW` writes
both grades from the ICE data (0x8284c180, 0x8284c1c8), `SetBloomCoeefs` gets half the ICE
threshold minus twice `bloomToWhiteValue` and saturation 1 (0x8284c264), and `m_bloomMultiply`
is rewritten from the ICE intensity (0x8284c3e4). Everything the fade branch did is replaced, so a
NIS that holds the ICE post-fx shows no challenge fade.

### What `tutorial_m_1a_05_photo_rail` shows

- Scene record (`scene_000.bin`): type 19 (`TUTORIAL_INTRO`), bit field 0 (no ground snap, DMOs
  off), not replay controlled, 9.57 s, two records: `mike_carroll` and `eric_koston` (category
  marquee, clips `tutorial_m_1a_05_photo_rail_A01_01.abin` and `_A02_01.abin`).
- Camera (`ice_000.bin`, context `NIS_Tutorial`): one take, world space for eye and look, a fixed
  eye at (1666.76, 104.46, -516.86) panning over nine look keys around the photo rail; lens and
  fisheye overrides, shake type 6. Speech channel: command 2, id 5000, character 38 (Atiba); post
  5013.
- Visible types: `RenderMan::UpdateVisibleTypes` 0x82839be0 gives a character NIS 0x282f (0x42
  only online, with DMOs on, or under `kRenderDynamicObjectInCharacterNISs`);
  `DetermineVisibilityFromGameState` 0x82829698 draws an entity only when `mask & type == type`.
  `cPresPedestrian`'s constructor sets 0x222 (0x828122d4), and the challenge characters are living
  world spawns drawn as `cPresPedestrian`s (`cChallengeCharacterManager`), so Atiba, who stands at
  the rail during the sign-up, is not drawn in the scene; nor are the player's skater (0x122),
  traffic or dynamic objects.

So the native cutscene shows Mike Carroll and Eric Koston skating the ledge and the rail at
Slappy's, with Atiba's voice-over (speech 5000) explaining the mission.

## Port

| File | Change |
| --- | --- |
| `crates/skate2-core/src/session/selected_session_career.rs` | `CareerHud::fade` is the update's fade info (`Option<f32>`: the last `cMsgFadeAmount`, none while no fade runs), not a black amount. |
| `apps/skate2-pc/src/marker_ui.rs` | The black `fade_batch` quad is gone. |
| `apps/skate2-pc/src/live_skating_view.rs` | `challenge_fade`: `VisualFX_Animation`'s fade branch on the game post-fx frame (threshold, `m_bloomMultiply` as fine 1 + overshoot, medium 1, coarse 0, the grades kept), before `nis_postfx`, whose ICE path replaces it during a NIS. The retained frame carries the tick's fade. Test `a_challenge_fade_blooms_the_game_post_fx_and_a_nis_takes_none_of_it`. |
| `apps/skate2-pc/src/render_postfx.rs`, `render_world_painter.rs` | `Frame::keep_matrices`: the frame's `VisualFX_Animation` wrote no grades. |
| `apps/skate2-pc/src/gpu_renderer/postfx.rs` (+ `postfx_tests.rs` literals) | The grades are kept as the members they are natively: rewritten by the game's grade or the ICE grades, kept by a fade frame, and `Apply`'s move toward the identity is applied in place, once over fresh grades and once per 60 Hz presentation step over kept ones (frame-rate independent). |

The challenge fades the port drew black (sign-up accepts, challenge teleports with a transition,
the end of a photo challenge before its rewards) now bloom to white as in the original.

## Verification

- `cargo build --release -p skate2-pc`: builds, no new warnings.
- `nb_r3` (same recipe, after the fix): poster, white bloom fade, then the NIS: the two pros on the
  ledge and the rail, panning shots, an empty shot of the rail at the end, the NIS's bloom-to-white
  out, white until the countdown's fade-in, play at the start locator.
- `nb_r4` (the whole photo recipe, captures every second, `SKATE2_TRACE_SPEECH`): the go-to NIS
  at the career start (alley shots, "New to skate?" popup, the gesture tip NIS),
  `Goto_m_1a_01_goto_slappys_outro_2`/`_3`, the `highlight_generic_left`/`_front` replays,
  `01_Welcome_Back` (Slappy), `02_Pros_Arrival` (the pros), `tutorial_m_1a_05_photo_rail` (`speech:
  queue NIS speech 38 5000`, `play NIS speech 38 5000`), and the photo outro
  (`photo_killed_m_1a_05_photo_rail`: killed it, the fixed-frame shots at Slappy's sign, the photo
  pick, the rewards over the white fade, autosave, play) all render.
- `parity.sh` (`nb_parity`): PARITY IDENTICAL (6000 lines).
- The PC unit tests (the new one and the view and post-fx tests) were not run: the app's test
  target does not compile for an unrelated reason (`marker_ui/tests.rs` still passes seven
  arguments to `map_movie::update`, which takes six since 2026-10-09 23:34).
- `Apply`'s move, checked on the raw word at 0x8284ca30 (`0x11606b2e`: vD 11, vA 0, vB 13, vC 12,
  so row R is M_R * (1 - w) + (w, 0, 0, 0); IDA prints `vmaddfp` operands in encoding order).

## Open

- `AptRenderingIntegration::SetColourTransform` 0x82609488: while the fade info is above zero,
  every Apt level but 0, 4, 8 and 9 renders with the colour scale's alpha (its fourth lane,
  0x82609580..0x8260958c) at 0. Not ported; the HUD visibility flag `OnChallengeAccept` and
  `StartEffect` set already hides the HUD for the same window, and the end-of-challenge and unlock
  screens (level 4, the `g3dSpinContainer`) stay visible.
- `VisualFX_Animation`'s tazer, Hall of Meat, challenge success and failure branches (which take
  precedence over the fade) are not ported.
