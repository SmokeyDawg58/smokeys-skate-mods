# Hall of Meat effects and recorded replay audio (2026-10-10)

Solo implementation; Jan 17 rich-symbol IDA was used read-only. This follows
`prop-hud-hom-flow-2026-10-10.md`. Scope: missing HoM sounds and
effects, an ultrawide results background, an eligibility/slowdown audit and
skater sounds during automatic replays.

## Native evidence

| Reference | Recovered behaviour |
| --- | --- |
| `AudioPresentation::WriteData` 0x827fb2f8 | Records world/player audio; global flag 22 is the local player's HoM bail AND HoM effect enabled. |
| `AudioPresentation::DoSimRestore` 0x827f5c60 | Restores the selected audio frame, interpolates only the four wheel positions by player ID, takes the absolute restore speed and the separate rewind flag. |
| `AudioSystem::UpdatePresentationData` 0x82504848 | HoM requires the loaded HoM bank, global flag 22 and the optional no-HoM-in-replay gate. Entry requires speed at or below `HOM_MinGameSpeed`; an active HoM latch persists while eligible. |
| `HOM_MinGameSpeed` getter 0x824e3360; constructor 0x82504c0c | `aud_general` collection `34049ABCE23C10AC`, attribute `4BD3E0CCEA90BE26` (shipped value 1). It is NOT the default collection. |
| `GetHOMMaterial` 0x82520920; `GetImpactorMaterials` 0x82540328; `GetSweetenerLevel` 0x8251e128; `CanPlayCollisionSound` 0x825209e8 | Existing collision audio selects the body/cloth/bone-crack/bone-snap/HoM sweeteners using force, body part, material and the authored priority rules. HoM selection depends on the audio system's HoM latch. |
| `SFXObj_Contacts::PlayGrunt` 0x82520080 | Excludes the local skater; adding generic local grunts would change the reference. Living-world speech also has its existing replay exclusion. |
| `SFXObj_Pause::UpdateParams` 0x82532f48 | The ordinary simulation-pause mute excludes replay playback and active NIS scenes. The port already contains this rule; restored replay state now reaches it. |
| `SceneReplayTimeController::GetTickCount` 0x82872720 and `Update` 0x828727e8 | Replay rate uses the scene multiplier product. Rewind direction follows the last whole-frame movement and can remain set at zero speed (photo scrubbing). |
| `VDWriter::_PrepareBailData` 0x827fcd40; `VDReader::ReadData` 0x827fdd78 | The visual treatment consumes the recorded bail duration, score and effect-enabled flag. |
| `PostFX::VisualFX_Animation` 0x82847610 | HoM precedes challenge results/fades. Blend amount is max(effect vs score, effect vs time); overlay uses score; vignette is 0.25 + 0.5 * effect. The native 30 Hz mode selects VisualFX then Uber. |
| `PostFX` constructor 0x828397cc/0x8283984c/0x828398cc | HoM colour matrix `BBD0B8DAE3AF1522`, bloom `7E9942D39FB220E8`, functions `65D868CEE4BE7BA4`. PointNegGraph has a 16-byte editor header before its four x and four y values. |
| `PostFX::SetTexturePointers` 0x82828728; `ApplyUber` 0x828334d8 | Original `xray.Texture` is Uber texture 6; texture 7 is the scene stencil texture. |
| `visualFX_PS` 0x8212e570; `uberPostFX_PS` 0x8212dc28 | Decoded Xenos shader words confirm character stencil masking and x-ray modulation followed by stencil blending toward the pre-cube colour. |

The original assets remain external. Reference exports, shader words and tuning
dumps are under `.scratch/hom-audio-fx-20261010`. Vault attributes are read at
runtime; the tuning dump is evidence, not replacement constants.

## What changed

- `selected_session_replay.rs`: record an `AudioFrame` beside each presentation
  submission. Replay audio restores recorded skaters, pedestrians, vehicles,
  dynamic objects, environmental keys, challenge/player globals and bail
  treatment. The listener uses the replay scene camera. Live and recorded
  physics share the existing audio packing/material conversion.
- `nis/manager.rs`: exposes the replay controller's multiplier product.
  `ReplayClock` keeps the original direction independently from playback speed.
- `live_skating_client.rs` / `game_audio.rs`: feed recorded audio into the normal
  mixer while a replay controls the presentation; supply actual game speed and
  rewind direction. Previously this path kept reading the paused live actor.
- `skate2-audio/system.rs`: restore the HoM bank/speed/flag latch and its replay
  cvar gate. The existing body and bone collision sounds can now select the
  original HoM material variants. NIS/pause/speech gates remain in their owners.
- `render_postfx.rs`, `live_skating_view.rs`, GPU PostFX: original HoM grade,
  bloom, vignette and x-ray overlay, including recorded replay bail data and
  scene stencil masking. ICE grades/bloom override the HoM grades as natively;
  overlay and vignette persist. The PC frame-rate setting is retained.
- `main.rs` / `render_inspection.rs`: retain and upload the shipped x-ray
  texture through the existing boot texture owner/cache. The first runtime run
  exposed that `load_loose_presentation_collection` retained only the missing
  texture and vignette from `globaltextures.rx2`. It now also retains the
  original x-ray asset (GUID `2F278EAD77A92866`, 512x512, one mip,
  pixel format `1A200152`). No generated substitute.
- `marker_ui.rs`: adds only HoM results' background shape `(81, 1)` to the
  existing ultrawide stretch handling. The original APT's character 2 contains
  this full-screen background. The skeleton, text and foreground panels retain
  their proportions. This is a PC widescreen adaptation.

## Slowdown and replay eligibility audit

No additional minimum-bones or minimum-impact rule was found, so none was added.

- `IsHallOfMeatEffectEnabled` 0x827acbd0: the visual cvar/front-end/mode gates,
  force override, HoM visuals setting and single-player mission's **Hom
  Available** setting are already implemented.
- `VDWriter::UpdateHoMSlowMotion` 0x827fd448 / `WriteData` 0x827ff8c8: normal
  slowdown starts during an eligible bail, before a bone break. The original
  `visualtreatments_bail/default` normal curve falls from 1 to about 0.6 at
  0.1384365 seconds. The severe curve reaches 0.325 at 0.1140065 seconds; recovery
  raises the multiplier. The existing implementation uses these authored curves.
- `CanEnableHOM` 0x825ae488 and manager update 0x825b30a8: signing up, an active
  main challenge and a chase disable career HoM challenge evaluation. Re-enabling
  is deferred during a bail; disabling is immediate.
- `ReceiveWipeoutOver` 0x825b6110: local-player and single-player gates, enabled
  manager and physical effect flag, then a completed HoM challenge OR a new PB
  can make a replay ready.
- `StatsBackEnd::UpdateBailStats` 0x8257da60: the PB top list requires at least
  20,000 points and a strictly better stored score. **A completed HoM challenge
  can show results below 20,000.** Its recording/achievement gate is separate
  from this top-list branch and already exists in `track_hom_statistics`.

## Verification

Artifacts are under `.scratch/hom-audio-fx-20261010`.

- `cargo check -p skate2-pc` passed (`check1.log`). The final production source
  passed `cargo build --release -p skate2-pc -j 1 --timings` (`build3.log`, 3m02s).
  The preceding complete build passed too (`build2.log`); its first launch
  exposed the omitted boot asset described above. Build1 had read the old core
  schema before the final audio-global additions and failed on those three
  fields; the subsequent builds rebuilt both sides consistently.
- A standalone Naga probe parsed and validated both the single-sample and
  multisampled PostFX WGSL. The real game then ran the shader on Vulkan
  with four-sample MSAA, without GPU validation errors.
- `hom5`: controller pushes after the map-load transition, then both sticks and
  triggers, produced a real **13,625-point, seven-break** bail. It completed the
  5,000 body-damage challenge, was neither a top record nor a PB, and correctly
  started `highlight_generic_right` (recorder frames 951..1472). No score or
  broken-bone count was injected. The 460 live slowdown samples ranged from
  **0.3275764 to 1**, returning to normal speed.
- That replay restored **584 audio updates** (recorded audio frame numbers
  335..856). All **506 updates with nonzero local skeleton contact force**
  matched the corresponding recorded live force. During replay the original
  HoM bank 7 had six nonempty splice requests; body/collision bank 5 had 90.
  This compares the port's recorded and restored values, not an Xbox trace.
- `hom5_024.png` shows the live HoM treatment; `hom5_041.png` shows the original
  x-ray overlay during replay. `hom5_051.png` shows the seven broken bones and
  challenge reward at **1720x720**, with the backdrop reaching both display
  edges and the skeleton/text retaining their proportions. The seven body-part
  FE sounds and HoM ambience also appear in the trace. A advanced to points;
  the next A continued into `FMV - HOM` and stopped the results ambience. The
  configured diagnostic budget ended during that tutorial movie (`hom5.png`);
  this run does not claim a post-tutorial autosave check.
- `hom2`, `hom3`, `hom4`: the initial input recipes produced nonqualifying
  bails (including zero and 800 points), no broken bones and no completed HoM
  challenge; replay_ready stayed false. They were diagnostic staging attempts,
  not ordinary-ollie equivalence checks. Inputs held through a map load are
  ignored until released, so the successful recipe uses fresh push pulses.
- `film`: `sponsor_me_02`, with physical pushing and the existing diagnostic
  objective-completion driver, reached its automatic highlight. **180 replay
  audio updates** restored frames 863..979; nonempty original sample requests
  occurred in banks 16, 23, 21, 0, 8, 20 and 17 (foot catch, skating and collision
  sounds). The replay's pause inputs were 0/0/0 despite simulation pause.
  Continue, mission completion and scratch autosave all ran; the controlled run
  completed normally. No HoM flag was set during that mission replay.
- Both stereo 48 kHz mixer dumps contain finite, nonzero output: HoM peak
  0.9153025 / RMS 0.0649583; film peak 0.5947520 / RMS 0.0503448. Sound validation
  here is sample-request/mixer evidence, not a listening comparison against
  original hardware. Music contributes to those whole-dump levels.
- Photo scrubbing's zero-rate/reverse-direction cases were source-audited,
  but a separate photo-selection runtime run was not performed in this batch.
- Existing test fixtures were updated for new frame fields/bindings; no unit
  tests were added and no test suite was run. Unrelated working-tree changes
  were preserved.

## Scope and confidence

Confidence: **high** for the documented source/data reconstruction. These
checks do not establish instruction-level or sample-exact original-console
parity. Generic DOF/motion blur and the full Replay Editor FE12 remain separate
unported systems. This change connects audio to the existing automatic HoM,
film, photo and tutorial replay owner; it does not implement that editor.
Local grunts were not added against the original exclusion. Existing unported
NPC collision speech is outside the recorded contact/board audio path.

Real `saves/SKATER.P` is unchanged: before/after SHA256
`F14055A6C3C1A412B3A6C36CFDDE2298DF7D6B1AEA6915F7FB1411CAE531C952`.
All diagnostics used scratch profiles.
