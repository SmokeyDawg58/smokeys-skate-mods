# PC Replay Editor, 2026-10-13

## Follow-up: cinematic PostFX entry crash

The first run reached Replay Editor but stopped with
`cinematic PostFX frame without grade inputs`. The editor correctly selected
the cinematic pass and excluded HoM, but the GPU code only accepted grade
depth inputs from ICE, HoM or a stereo eye. ICELite alone activates none of
those treatments.

Live debug IDA confirms:

- `PostFX::Apply` (`8284c848`) selects the cinematic path for ICELite;
  `8284c914` excludes the HoM treatment while the editor is on.
- `PostFX::SetParameters` (`8284bbf0`, camera reads at `8284bd48/4c`)
  always samples the presentation camera's near/far planes.
- `PostFX::VisualFX_Animation` (`82847610`) writes both normal authored
  colour matrices and `distance_near`/`distance_far` when no special
  treatment applies. These are already loaded from the original normal
  collection `2d9c5e5c45ce93dc`; no substitute constants are needed.

The PC frame now carries the actual camera planes from `draw_retained`.
The cinematic pass accepts those with the authored normal grade distances,
and both normal colour matrices are published. Explicit ICE/HoM/stereo inputs
retain their precedence. Live and saved replay entry use the same draw path.
The editor passes no live restore clock to `view.update`, so its draw completes
before the client selects cinematic mode; these inputs cannot be lost to a
deferred live-frame draw. Shader code and gameplay physics are unchanged.

A Select/Back entry was also traced. Original
`LoadInputMap` (`826ddc18`) binds `AptSelect` to state 135 / FE keycode 1.
`StateManagerFrontEnd::CheckForOnlineHubMenu` (`82656428`) handles that key
by pushing state 44 (Online Hub); `CheckForOptions` (`82656718`) handles
Start/keycode 0. `GameModule::DoInputUpdate` (`82730668`) has a separate
debug shoulder/B replay shortcut, disabled by default. Select is not a native
direct Replay Editor shortcut in this reference. A PC convenience override
is possible; no button mapping has been changed in this fix.

Validation: `cargo check -p skate2-pc --bin skate2-pc -j 1 --offline` passed
in 42.75 seconds (437 core / 18 PC existing warnings). Scoped whitespace
review passed. Existing fixture initializers were updated for the added
camera field; no tests were added or run. Normal release build passed
(`cargo build --release -p skate2-pc -j 1 --timings`, exit 0, 202.889 seconds,
finished `2026-10-13T01:35:45Z`). Executable: 43,582,464 bytes, SHA256
`811D9BC1715574480E7B97ECC84CF6818220B7F6A3ED247030F472FDEB20BC85`.
Confidence: **high** for the recovered source behavior; in-game entry and
visual validation remain pending. Backups, build records and the full live
IDA packet: `.scratch/replay-entry-fixes-20261013/`.

## PC Film Pack override

A later PC change removes Film Pack gating. The central
`Unlockables::is_unlocked` query now returns true for REPLAY/`filmpack` and
REPLAY/`filmpackmovie`, independent of profile contents or DLC inclusion.
All advanced editor controls, saved Film Pack footage and the Extras advanced
tutorial are available. Existing native callers consume this shared policy;
no DLC slots are written into `skater.p`. This is an intentional parity change.

The editor query is confirmed by `ReplayEditorBackEnd::Start82567450`; live IDA
`FrontEndState_Extras::GetNumOptions82643090` also confirms category 11 and
`filmpackmovie` add the seventh Extras item. Source/EXE backups and the release
build log/result are in `.scratch/replay-filmer-unlock-20261013/`.

The requested normal release build succeeded (exit 0) on
2026-10-13 at 00:38:42 UTC, in 883.069 seconds:

```text
cargo build --release -p skate2-pc -j 1 --timings
target/release/skate2-pc.exe: 43,582,464 bytes
SHA256: 7DE99CFF4CF2DFC5A24EE9E78B5771FF7A7E7FCD8D97C4029BDAD51EAB4E2CE8
```

This executable includes the Replay Editor and unrestricted Film Pack policy.
Warnings: core 437 / PC 18. No tests or game launch were performed.

## Result and validation status

The previously disabled pause-menu Replay Editor is connected to the PC
presentation recorder, recovered ICELite authoring system and original Replay2
frontend assets. Local playback, editing, photo capture, replay saving, saved
replay reopening and the offline My Footage browser are implemented.

This is source/reference validation, **not a completed in-game parity gate**.
The final metadata compile check passed:

```text
cargo check -p skate2-pc --bin skate2-pc -j 1 --offline
typecheck-14.log: exit 0, 2.29s, core 437 warnings / PC 18 warnings
Previous full metadata check13: exit 0, 17.24s
```

No tests were added or run. The later release build is recorded above; no game
was launched. Controller, image, sound and save/reload behavior still require
runtime verification. Work was solo and restricted to this repository;
Android was not edited.

## Reference and confidence

IDA used the symbol-rich Xbox debug build. Original APT/BIG,
input-map and ICELite assets were decoded from the external game installation.
No original assets are added to the distributable source tree.

- **exact**: directly read addresses, debug declarations, enums, asset paths,
  script offsets and raw constants in the reference packets.
- **high**: Rust translations reviewed against those functions and callers,
  including PPC branches Hex-Rays failed to express completely.
- **medium**: integration with the existing PC recorder, renderer and streaming
  services; behavior has not been compared to a running original.
- No speculative gameplay/physics replacement is introduced.

Evidence and pre-edit backups are in `.scratch/replay-editor-20261013/`.
`scope-manifest.json` and `scoped-final.diff` compare against those backups,
since the repository already contained substantial untracked work.

Reference packets in `reference/`:

- `symbols.json`, `core-functions.json`, `ice-functions.json`: Replay2,
  ReplayEditorBackEnd, ICELite author/controller and replay flow functions.
- `editor-types.json`, `lite-types.json`, `final-layouts.json`: debug types,
  enums, ownership and field offsets.
- `lite-elements.json`, `lite-descriptor-raw.json`, `lite-data-summary.json`:
  editable descriptors, raw initializer branches and original shake data.
- `cvars.json`, `extra-camera-functions.json`, `camera-collision-final.json`:
  tweak defaults, movement and collision queries.
- `frontend-tables.json`, `host-hooks.json`, `timeline-colours.txt`,
  `postfx-editor.json`, `replay-sounds.json`: UI, audio and render integration.
- `media-symbols.json`, `media-functions.json`, `media-browser-functions.json`,
  `saved-replay-final.json`, `final-audit.json`, `final-review-20261013.json`:
  storage/browser paths, hull ownership, final branch and table checks.
- `data_fe_source_*.apt.txt`: original APT actions, hierarchy and imports.

## Native function map

Addresses are virtual addresses in this debug build, not retail addresses.

| Owner / symbol | Address | Responsibility |
| --- | --- | --- |
| `StartHub::OnEnter` | `8265e4d0` | Sample availability at 150 tracked frames |
| `StartHub::ProcessSelection` | `8265ea98` | Fade and enter frontend state 12 |
| `ReplayEditorBackEnd::Start` / `Stop` | `82567450` / `825675d0` | Freeze submissions, own pause 7 |
| `FrontEndState_Replay2::OnEnter` / `OnIntroDone` | `82653b88` / `82653d80` | Screen 15 and live/saved entry |
| `FrontEndState_Replay2::OnUpdate` / `ReceiveInput` | `826799f8` / `82682b68` | Input, transitions, delayed exit |
| `ReplayEditorBackEnd::SetCurrentEditorMode` | `8256e008` | Camera commit, scrub/play/effect modes |
| `ReplayEditorBackEnd::GetCameraEditIndex` | `82567910` | Selection side effect consumed by APT |
| `ProcessEditSelection` / `SetNewCamera` / `CheckOverwrite` | `8267ab70` / `82671ce8` / `8267a940` | Cameras and replacement prompts |
| `CanPlaceMarker` | `8255d968` | Ordered inclusive proximity windows |
| Encoded duration | `8255e6f0` | Trim, speed and frame rounding |
| `UpdateStall` / `SimUpdate` | `82567158` / `82570e60` | Streaming latch and playback |
| `Render` / `DrawHardCuts` / `DrawMarkers` | `82579988` / `82571018` / `825714b8` | Render gate, colors, marker textures |
| `CheckStreamStalled` / `FadeComplete` | `8265f590` / `8265f6d8` | Overlay and newest-frame restoration |
| `SceneLite::Init` / time controller | `8287df38` / `8287d590` | Camera channels and frame clock |
| `ICELITETake::Init` / `NewBuffer` | `82a55ab0` / `82a5b2c8` | Packed layout and capacity |
| Take / channel parameter updates | `82a5a150` / `82a55678` | Interval selection and ordering |
| `Insert` / `Harden` / `Delete` | `82a5bce0` / `82a5c128` / `82a5bf70` | Soft keys, hard boundaries, deletion |
| `ICELITEAuthor::SetState` / `InsertKeys` | `82a622a8` / `82a5ffb0` | Endpoint editing and key insertion |
| Camera setup / `EndSetup` | `82a638d0..82a640f0` / `82a63120` | Camera setup and commit |
| `ClearSoftKeys` / `DeleteMarker` | `82a63718` / `82a64ce0` | Replacement and endpoints |
| `SetSpeed` / `SetShake` | `82a646d8` / `82a64458` | Effect channels |
| Bubble conversion / camera update | `82a606f0` / `82a61a28` / `82a64818` | Coordinate spaces and binary angles |
| User ICELite world camera queries | `826dad28` / `826db510` | Distance, fat lines, slide, line of sight |
| Shake update | `82a58e68` | Absolute take-time sampling |
| `SerializeReplay` / `DeserializeHelper` / `PrepareLoadedData` | `82575728` / `8257a038` / `82567d98` | Saved edits and full recording |
| Dynamic hull pack / game load | `82da4aa8` / `82da48e8` | Version 3 lookup and game-type mask |
| `StateReplay::OnEnter` / `OnLeave` | `8275ca30` / `8275cb98` | Saved-world ownership/restoration |
| `LoadReplayData` / `LoadDMOS` | `82774c38` / `8276ec70` | Focus and DMO preparation |
| Slot exhaustion / slot numbering | `8256ff60` / `825761b0` | Limits and browser/delete flow |
| Online options entry / selection | `8264b658` / `82674b58` | Offline-accessible Skate.Reel route |
| `GoToSkateReel` / general entry / selection | `826613a0` / `826a0648` / `826ab510` | State 55 and tiles |
| My Footage selection / video / photo options | `826ae370` / `826a5ec0` / `826a0c80` | View/load/delete restrictions |
| Photo entry / local photo / input | `82697478` / `8268f378` / `826a6400` | State 58 and offline photo exit |

## Types, values and ownership

Frontend and backend retain separate readiness, visibility, saved-entry, edit
and render flags. Live debug layouts establish these byte offsets:

| Type | Size | Fields |
| --- | --- | --- |
| `Sk8::BE::ReplayEditorBackEnd` | 560 | mode 64; start/end 136/140; selection 424; DLC 433; watermark 436; render 528; stream stall 530 |
| `Sk8::FE::FrontEndState_Replay2` | 172 | saving 124; instant 125; visible 126; from-save 132; OnReady 151; loaded data 153; mode 156; editor ready 162 |
| `ICELITETake` | 5348 | parameter 4; subparameter 8; 34 channels at 556 |
| `ICELITEChannel` | 16 | u16 key/interval counts 0/2; parameter pointer 12 |

Offline mode IDs: Play 1, Trim 2, Instant 5, Timeline 6, Follow 7, Tripod 8,
Game 9, Free 10, Effects 11, debug FrameStep 13, Help 16. APT camera row ordering
differs from the backend enum: Game precedes Free in the UI. Its
GetCurrentCameraEditIndex callback also writes the backend selection.

The take uses ICELITE 2.96: 135 editable descriptors, 34 channels, allocated
capacity 199 keys / 198 intervals. Camera/lens/shake/speed use channels 0/1/4/5.
The frontend separately enforces its 100-marker gate. Existing cinematic files
retain their 146-element descriptor mapping.

Immutable recorded frames retain local-player identity across Party Play turns,
skater rig/pose/material references, audio, dynamic objects, vehicles, pedestrian
poses and traffic lights. Pedestrians own a playback palette per identity, so
scrubbing does not overwrite live animation palettes. Live physics does not
advance while editing.

## Controls and editing

Xbox names refer to the existing controller input path and keyboard pad mapping.
Expressions come from the original input map. PRES is sampled once per frontend
tick; high render FPS does not multiply camera/scrub deltas.

| Context | Control | Action |
| --- | --- | --- |
| Instant / timeline | LT / RT | Smoothed rewind / fast-forward |
| Playback | RB | Play / pause |
| Instant / timeline / playback | L3 | Hide/show editor UI |
| Instant | A | Enter timeline editing |
| Instant | D-pad left/right press | Jump to start/end |
| Timeline | D-pad left/right release | Previous/next marker |
| Timeline | D-pad down release | Insert clip boundary |
| Timeline | D-pad up release | Delete selected marker with confirmation |
| Timeline | Hold LB, move LS/RS horizontally | Move in/out heads; LS takes priority |
| Timeline | Release LB | End head editing |
| Timeline | A | Edit clip or selected camera key |
| Camera | RS | Rotate |
| Follow/tripod | LS vertical | Distance |
| Free camera | LS | Move |
| Camera | D-pad up/down, left/right | Height, zoom as allowed by type |
| Camera | A | Commit edit/key |
| Eligible editor modes | X | Photo with save confirmation |
| Editor | B | Finished menu: return, exit, save, eligible upload entry |
| Editor | Start | Context-specific original help |
| Help | A / B | Next page / return |

Basic editing exposes Follow, Tripod, Game and speed. In the original game,
the `filmpack` unlock in category REPLAY adds Free, advanced Follow/Tripod/Free,
shake and watermark selection. The PC policy above always makes
that unlock available, also bypassing the saved-footage missing-pack restriction.

Advanced cameras insert/edit soft keys; clip markers make hard cuts. Insertion
distance, ordered ties, endpoint copies, soft-key clearing/replacement,
cancellation and camera commit follow the author/controller functions. Editing
retains an unquantized cache; stored values use native packed ranges and binary
angles. Pitch retains the 85-degree limit. Camera collision uses BFDefault7
world queries without stepping physics.

Speed is 10..100 percent by 10; shake is Off/Low/Medium/High; watermark is
Off/Default/Custom. Shake uses original `data/scene/ICELite.bin` tracks and
absolute replay time. ReplayEditorTweaks remain live cvar reads (1259..1272).
Orbit-height input preserves the source literal 0.01 despite a registered height
cvar. Timeline/head operations use native 30 Hz quantization and 0.1 minimum
normalized trim span.

## UI, transitions and sound

Original fonts, art, marker textures, hierarchy, help pages, buttons and localized
strings are loaded through the existing APT renderer:

| Screen | Asset |
| --- | --- |
| Replay Editor 15 | `replay2/replay_editor2`, `9cb595f1` |
| My Footage 102 | original footage / media_browser / mediathumb |
| Online options 9 | `04e9e0b3`, original HOM picker export 27 |
| Skate.Reel general 101 | `afcf3ebc` |
| Photo 103 | `aecfc0a2` |

Translations dispatch exact asset/character/action offsets, including
InitActions; unknown actions fail explicitly. Layout is asset-led: custom
timeline uses native 817px span, original band colors/alpha and 13x51 marker
quads. Backend render enable is independent of APT alpha; Instant Replay and
its playback do not draw custom clip bands/markers.

ScreenManager's staged loading, eight-tick open hold, authored intro/outro,
depth and following-tick destruction are reused. Live input starts at root
intro. Saved replay also waits for APT InstantDone's UpdateReplayEditorReady,
state 9 and cursor refresh. Browser return resets readiness latches and retains
whether the editor opened from saved footage.

PostFX selects ICELite cinematic and excludes HoM/tazer treatments. Game
HUD/indicators are suppressed. Photo capture is before UI with original watermark
texture/placement. Intentional lens-flare, HoM-speed and PostFX-off preferences
are preserved.

Recorded audio is reused with editor camera, replay flag and playback/scrub
rate. The 20-byte `sFEStateInfo` table at `82fded80` gives editor 12 (music 9,
unpaused), online 23 (music 0, unpaused), general 55 / footage 56 / photo 58
(music 9, paused). UI sound keys and playhead-reset sound are connected.

## Saving and reopening

Main Menu -> Online -> Skate.Reel -> My Footage now opens offline saved media.
The original six-thumbnail browser supports replay/photo filtering, navigation,
photo display, replay loading, confirmation dialogs and deletion. The native
video-load restriction when entered from the editor is retained.

Native limits: 60 replays, 100 photos. Numbering follows the largest existing
slot. Full-slot popup opens the deletion browser. The PC adapter renames a
completed pending directory to commit a slot. Errors show a dismissible notice;
numeric slot validation and known file names bound deletion to selected media.

Media lives beside active `skater.p`, under `skate_reel/`:

```text
replays/<8-digit hex slot>/replay.sk2r
replays/<slot>/thumbnail.jpg
replays/<slot>/metadata.bin
photos/<slot>/photo.jpg
photos/<slot>/thumbnail.jpg
photos/<slot>/metadata.bin
```

Thumbnails are 160x120 / 15 KiB; full-photo budget is 150 KiB. Metadata contains
duration and the FE location at initial replay position.

`SK2PCR01` is an explicitly PC-specific versioned container. It stores the
original ICELite track, Film Pack requirement, version-3 packed hull lookup and
complete retained PC recording. Scene recipes reload original geometry from
mounted assets; assets are not embedded. Checksums, counts, hierarchy/remap/pose
bounds and frame ordering are validated. Native SerializeReplay does not save
trim heads or watermark, so this adapter does not persist them. Footage outside
the trim is retained.

Loaded footage owns its hull lookup independently of the retained frontend
world. Entry uses the original free-skate mask (2), streams recorded position
and waits for readiness. Exit restores the newest untrimmed frame, waits for
presentation/simulation focus, unloads owned hulls and returns to My Footage.
Live replay exit returns to the pause hub.

## Availability and edge cases

- At least 150 tracked frames in Single Player, its challenge hub, offline Free
  Skate and Party Play. Create-a-Spot/online restrictions and pause eligibility
  remain in effect.
- Startup/fade/loading consume input. Focus loss resets movement deltas.
- Help retains camera edits; other camera exits commit them.
- Boundary exclusions, marker limit, replace/delete prompts, playback end,
  trim limits, hidden UI and debug frame-step are handled.
- Offline profile existence is the PC signed-in-user bridge; EA authentication
  remains separate.
- Upload is omitted above 60 encoded seconds, accounting for clip speed.
  Local replay save remains available.
- Expensive scrub stall is a black-overlay/latch, not an input freeze:
  distance >=1 metre and estimated cost >=120 ticks. Cost counts the first
  eight unread collections at bytes/133333. Playback does not latch a stall.
  Exit waits for loaded focuses and a subsequent update.
- Party Play restores the recorded controlled actor, not the actor active at
  editor entry.
- Existing film/photo/HoM clocks and challenge frame pins share the recorder.
  No skating force, collision solver or physics timestep changes are made.

## Host boundaries and remaining validation

These prevent claiming complete numerical/visual parity today:

1. Recording retains the existing 5400-frame bound and shared presentations,
   not the console's compressed 12 MiB DataSystem eviction policy. Busy-scene
   duration/memory need separate comparison.
2. PC saves are not Xbox memcard-compatible. VP6 web encoding, EA upload/auth
   and remote footage are not implemented. Offline upload reports the sign-in
   requirement rather than claiming success. Create-a-Spot browser remains
   outside this task.
3. PC streaming publishes whole generations; native cost/eligibility rules are
   translated but per-asset asynchronous timing differs. Additional-world/DLC
   maps remain an existing port boundary.
4. Native camera collision calls are translated, but camera-versus-moving-DMO
   scrubbing needs a runtime comparison. Frozen simulation query ownership is
   retained.
5. UI assets/scripts were read and wired, but runtime visuals, transitions,
   controls, audio and save/reload have not been exercised.

Runtime validation should cover entry/exit in all four hubs;
every camera and help return; clip/key insert, replace/delete, trim and speed;
moving actors/audio; save/photo/reopen across restart; full-slot/error dialogs;
always-available Film Pack controls and display sizes. Record results here rather than treating
the type check as parity proof.
