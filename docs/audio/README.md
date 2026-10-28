# Skate 2 audio — reconstruction plan

Status: **research and planning only (2026-10-02). No audio source code exists in the port yet.**
Every fact in this folder was taken from the debug executable `sk82_na_m.xex` in IDA (read-only),
from `sk82_na_m.pdb` (dumped with `llvm-pdbutil`), from `sk82_na_m.map`, or by parsing the
Jan-17-2009 `data/audio` tree. Addresses are debug-xex virtual addresses. Anything not yet
confirmed is marked **UNVERIFIED** or listed in [open-research.md](open-research.md).

| Document | Contents |
| --- | --- |
| [README.md](README.md) | Architecture, frame placement, lifecycle, implementation plan |
| [formats.md](formats.md) | Every audio file format: BIG4/Viv4, SNR/SNS, EA-XMA, ABKC, S10A, MOIR, EMS, MXB, SPLC, grain, MPF/MUS, speech |
| [engine.md](engine.md) | RenderWare Audio Core (voices, plugins, buses), Skate's sample player, AEMS interpreter, Csis |
| [game-layer.md](game-layer.md) | `Sk8::Audio::AudioSystem`, state managers / states / SFX objects / controllers, Nicotine mixer, presentation data, messages |
| [nicotine.md](nicotine.md) | Nicotine dynamic mixer: data model, `MixMapSK8.mxb` layout, per-state counts, constant tables |
| [cvars.md](cvars.md) | All 146 audio cvars with categories, defaults and ranges |
| [open-research.md](open-research.md) | What still has to be reverse engineered, in implementation order |

## 1. What the original audio stack is

Skate 2's audio is about 7,000 functions in four layers, plus a data layer that drives almost all
behaviour. Counts are map symbols per library.

```
 Game code (physics, living world, FE, challenges, NIS)
   │  AudioPresentation (sim→pres data), Hermes messages, IAudioInterface calls
   ▼
 Sk8::Audio game layer            audio_lib (2371), audioobjs_lib (1460), audiostates_lib (341)
   AudioSystem ─ 13 state managers ─ states per entity ─ 42 SFX object types + 19 controllers
   MusicSystem, Sk8SpeechSystem, Sk8EmitterSystem, GlobalEffects, MicrophoneMan
   Nicotine dynamic mixer (MixMapSK8.mxb): states → mix controls → channels → SFX inputs
   ▼
 EA middleware                    audiocore (115), audioassetmanager (45), csiscore (49),
   Csis (class/function/global API)  musiccore (68), speechsystem (45), spch (109),
   AEMS bytecode interpreter         contextsystem (225), emittersystem (63),
   Pathfinder music, speech, Splicer, splicer (48), GrainPlayer (16)
   GrainPlayer, Contexts (XML rules)
   ▼
 RenderWare Audio Core            rwaudiocore (1885): plugins 933, Xenon 306, common 256,
   System, Voice, PlugIn graph,     decoders 242, AEMS runtime 94, encoders 54
   SndPlayer1, decoders, DSP
   ▼
 Xbox 360 XAudio + XMA hardware   xaudio (801) — replaced by a PC backend
```

Data drives behaviour at every layer:

* **AEMS banks (`.abk`, 257 files)** hold compiled sound "patches": bytecode modules plus an embedded
  sample bank. They implement the sound classes the game creates through Csis.
* **Csis interfaces (`.csi`, 23 files)** declare the classes, functions and global variables that game
  code and banks agree on.
* **The Nicotine mix map (`MixMapSK8.mxb`)** decides which SFX objects and controllers exist per state
  and how controller inputs become volume, pitch, azimuth and filter outputs.
* **Vault classes `aud_*`** (`aud_material`, `aud_music`, `aud_general`, …) hold tuning.
* **Speech context XML** (362 files) holds announcer, cameraman and main-cast rules.

**Two codecs are used, and both must be supported:**

* **EA-XMA (EAAC codec 3)** covers all 4,936 AEMS bank samples and `.snr` headers, the NIS, post and
  ambience `.sns` streams (chunk/pair structure checked), and the announcer (EN/DE 41 kHz),
  cameraman (stereo 44.1 kHz) and living-world (44.1 kHz) speech. The 360 decoded XMA in
  **hardware**, so the binary contains no decoder to port. A PC XMA2 decoder is a hard prerequisite
  (§6, D1).
* **EALayer3 v1 (EAAC codec 5)** covers **all main-cast speech** (EN/FR/DE, mono 44.1 kHz, about
  21,000 clips) and the **French announcer** (22.05 kHz). It was decoded in **software**
  (`rw::audio::core::EaLayer31Dec`, `EALayer3Core`, `Layer3Dec`), so it can be ported 1:1 from IDA
  (R-F5).

## 2. Boot, lifecycle and frame placement (verified)

### 2.1 Boot — `Sk8::Sk8AppMan::InitApp` 0x827349c8

1. `Flow::Manager` constructed.
2. **`AudioCore` constructed and `AudioCore::InitializeAudioCore` 0x82a7f008** — before the data system.
   This builds the RWAC system, plugin registry, decoders, the master/music/speech bus voices, the
   SpliceManager and the AEMS sample-player factory, and sets the AEMS minimum update rate to 30 Hz.
3. DataSysMan, TrickDatabase, RenderMan, characters, language, memcard, backend, FE, RecipeMan,
   textures, paths, scenes, subtitles, debug overlay, LoadingMan.
4. **`Sk8AppMan::CreateAudioSingletons` 0x8272e4c0**, in order: GlobalEffects, Sk8SpeechSystem,
   AudioAssetManager (FileLoader, AudioCore allocator, SpliceManager), MusicSystem (+`Initialize`),
   Sk8ContextSysMan, AudioSystem (ctor 0x8250dda0), MicrophoneMan.
5. FrontEndManager::Initialize, net boot.

### 2.2 World entry and loading

| Caller | Audio call | Notes |
| --- | --- | --- |
| `GameModule::OneOffLoad` | `IAudioInterface+0x08 EnterWorld` 0x82504450 | `Initialize` → `LoadInGameData` → `InitializeMixer` |
| `GameModule::SecondaryLoad02` | `+0x10 SecondaryLoad` 0x824fe178 | language speech banks, resident speech, vehicles, moments |
| `GameModule::GameLoad` | `ResetAdditionalLocationKey`, `SetAdditionalLocationKey(u64)`, `AdditionalDataLoaded` | per-location emitter (DLC) data |
| `GameModule::GameUnload` | `+0x1C ExitGamePlay`, `ResetAdditionalLocationKey` | |
| FE `StateManagerFrontEnd::ChangeFEAudioState` / `ChangeFEMusicState` | `cMsgFEMusicStateChange` / `cMsgFEAudioStateChange` | message based |

`AudioSystem::Initialize` 0x82509c18 opens 16 BIG files, loads the 8 Csis event systems, registers
the SFX/state/state-manager classes, initialises speech, and creates the three speech context systems
(Sk8CameraMan 225, Sk8Announcer 200, Sk8MainCast 200). It then sync-loads `emitter_utility.abk` and
starts the emitter system. `LoadInGameData` 0x824fdfe8 and `SecondaryLoad` are itemised in
[game-layer.md](game-layer.md#lifecycle).

### 2.3 Per-frame — `Sk8::GameModule::Render` 0x827320f0 (presentation thread)

```
UpdatePresCollectionDefer → PresThreadMsgMan dispatch → Presentation::Input
SceneManager::UpdateFrameBegin
DataSysMan::StepTime(dT) + RestoreFrame          (sim→pres interpolation)
streamer / textures / asset release / load-screen-on-stall
Presentation::Update(dT)
SceneManager::UpdateFrameEnd
AudioSystem->Update(dT, RenderMan.m_PostFX.m_b30HzMode)   ← vtable +0x94 at 0x827324d8
RenderMan::Render(dT)
```

Audio therefore runs **after** presentation and scene updates and **before** render, on the
presentation thread, using the interpolated presentation data. `AudioSystem::Update` 0x82507b88 is a
**two-phase, half-rate scheduler**: phase A (assets, events, music, speech, contexts, splices, state
`UpdateParams`, emitters, presentation data) and phase B (Nicotine `ProcessMixMap`, state
`ProcessUpdate`) run on alternate frames with accumulated dt. Both run in the same frame when
`b30HzMode`, `kForceSingleAudioUpdate`, or `kCompensateForSlowFrameRate && dt > 0.02` holds. The full
algorithm is in [game-layer.md](game-layer.md#update).

Below that, the RWAC mixer runs on its own thread (`Dac::XenonThread`). AEMS modules run from an RWAC
timer (`SNDAEMSI_timerupdate`, stage `TIMERSTAGE_BEFORECOMMANDS`, "Aems Modules"). The game talks to it
through the locked command buffer (`System::Lock/Unlock`, 256000 bytes, high-water 100).

## 3. Output format

The mastering voice is `SubMix("Master", 6 ch) → SampleCapture → Dac`, and the Dac sample rate is set
to **48000 Hz** (Event 2). Output is **6 channels (5.1) at 48 kHz**. "Music Master" and "Speech Master"
are `SubMix(6) → VuMeter → Send` voices feeding Master. A PC backend must accept a 5.1 float mix and
downmix it for stereo devices (§6, decision D3).

## 4. What already exists in the port

* Physics already publishes the audio-relevant PhysOut fields: wheel/truck/deck audio surfaces
  (`physical_player_phys_out_board_fill_contacts.rs`), grind audio surface, and
  `SkeletonCollision::FillPhysOut_AudioCollisionInfo` (`skeleton_collision_fill_audio.rs`,
  `docs/evidence/skeleton-audio-collision-fill.md`).
* `AudioPresentation::FillSkaterData` 0x827f4c48 is partly ported for the minimap only
  (`session/selected_session_minimap.rs`).
* APT UI clips already raise `FE_SOUND_*` callbacks; they are currently ignored
  (`marker_ui/*.rs`).
* There is **no** audio output backend, decoder, bank loader, mixer or sound logic yet, and no audio
  crates in `Cargo.toml`.

## 5. Implementation plan (phased; each phase IDA-verified before it lands)

Each phase ends with something audible or testable and keeps the existing presentation-thread order.

| Phase | Deliverable | Depends on |
| --- | --- | --- |
| **A0 Formats** | Rust readers: BIG4/Viv4, SNR/SNS chunks, S10A, ABKC header + fixup tables, MOIR, EMS, `.sek`. Corpus tests over all of `data/audio` (counts must match the census in formats.md). | — |
| **A1 Decoders** | EA-XMA → s16 → f32 decoder (EAAC block/pair splitting, 2048-byte packets, 512-sample frames, 384-sample priming, padding rules verified in formats.md), and an EALayer3 v1 decoder ported from the game's own software decoder. Offline test that decodes every bank sample, stream and speech clip without error. | D1 |
| **A2 Output + core mixer** | PC output device (5.1 float @ 48 kHz), RWAC-equivalent voice graph: SndPlayer1 → Rechannel → Resample → HPF → LPF → Send → Gain → Pan2D → Send; SubMix buses Master/Music/Speech; RWAC timer stages. | A1, D2, D3 |
| **A3 AEMS + Csis** | Csis System/Class/Function/GlobalVariable API; AEMS bank resolve (function/static/interface fixups), module instances, the 40-opcode interpreter with exact integer/rounding semantics, PLAYER → SkateSamplePlayer. Test: instantiate every class in every bank. | A0, A2 |
| **A4 Game skeleton** | `AudioSystem` lifecycle (Initialize/EnterWorld/LoadInGameData/SecondaryLoad/ExitGamePlay), two-phase `Update` at the exact frame slot, AudioAssetManager (sync/async loads, swap banks), CSTATEMGR/CSTATE/SndBase framework, cvars. | A3 |
| **A5 Nicotine mixer** | `MixMapSK8.mxb` loader and `NFSMixMap::ProcessMixMap` (states, master/sub channels, event ADSR/ASR/AR, 3D mix controls, curves/shapes, snapshots); DMixIO wiring via `ConnectDMixIO`. | A4 |
| **A6 Presentation data** | Full `AudioPresentation` (SkaterData 512 B, GlobalData, Vehicle/Pedestrian/Dynamic/SpacialInstance) filled from the port's existing PhysOut/Actor data with sim-submit / viz-restore / blend. | A4 |
| **A7 Skater SFX** | CSTATE_Player + SFXObj_SkateBoard/Wheels/Contacts/Rail/Tricks/Cracks/Clothing/SenseOfSpeed/OffBoard/HandGrabs/Whoosh/Treatments + SFXCTL_PlayerPhysics/3DPlayerPos/3DBoardPos; rolling grains (GrainPlayer) and Splicer collisions. **First milestone the player hears: rolling, pops, landings, grinds.** | A5, A6 |
| **A8 World** | Ambience + Main_Ambience_Crossfade, emitters (`*.ems` + emitter banks + octree), reverb zones + GlobalEffects, crowds, speakers, DMO sounds, traffic engines/horns/skids/whooshes, pedestrian SFX. | A7 |
| **A9 Music** | MusicSystem + MusicCore + Pathfinder (`*.mpf/.mus`): FE/world/game states, playlists (`aud_playlists`), chase/race/replay/challenge triggers, DSP voices, game-speed effects. | A4 |
| **A10 Speech** | Sk8SpeechSystem, SpeechInstance banks per language, spch/VOX events, the three context systems (XML rules), living-world, announcer, cameraman and main-cast speech, mission speech. | A4 |
| **A11 FE / NIS / HUD** | Menu sounds (`PlayMenuSound(u64)` from APT `FE_SOUND_*`), menu flange voice, FE HUD splices, load screens, NIS sfx/speech/post, Hall of Meat bank, pause FX, movie volume. | A4 |

## 6. Design decisions

* **D1 XMA decoder.** Almost all content is EA-XMA (XMA2, WMA-Pro based), which the 360 decoded in
  hardware. (EALayer3 is ported from the binary, so it needs no decision.) The options:
  1. Port an XMA2 decoder to pure Rust from a reference implementation, verified against
     vgmstream/FFmpeg output as a test oracle. **Recommended:** native, no C dependency, and in line
     with the project's "purpose-built native PC backend" rule.
  2. Link FFmpeg's `xma2` decoder through a C binding. Fastest, but a large C dependency.
  3. Pre-convert assets offline. This breaks the "original assets loaded through reconstructed
     formats" rule, so it is **not recommended**.
* **D2 Output API.** `cpal` (WASAPI) as a thin device layer, with all mixing in our own RWAC
  reconstruction. **Recommended** over a high-level engine (kira/rodio), whose own mixing would replace
  the original DSP.
* **D3 Channel layout.** Keep the original 5.1 internal mix and downmix at the device, or render
  stereo directly. **Recommended:** keep 5.1 internally (the original Pan2D and music channel mapping
  are 6-channel), and downmix to stereo when the device is stereo. The downmix law is a host decision
  and must be documented as such.
* **D4 Threading.** The original mixes on a dedicated Dac thread with a command buffer. **Recommended:**
  an audio-callback thread for DSP, with the game layer on the presentation thread exactly as in §2.3,
  communicating through a lock-protected command queue (mirrors `System::Lock/Unlock`).
