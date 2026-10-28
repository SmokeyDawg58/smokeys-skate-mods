# PC replay entry, first profile name, Trick Guide, and Career SKATE turns

2026-10-13. PC only. All implementation came before
compiling, then an end-to-end Career -> Replay Editor script repeated until
successful. Automated gameplay tests are limited to replay, as requested.

## Replay texture ownership

The reported `unknown UI texture owner` came from the presentation switch in
`marker_ui::flatten_projected`. It stopped at movie 92, although the editor and
its supporting movies occupy 93..99. Each loaded movie now retains its original
asset basename and resolves `<basename>_textures/<id>.Texture` through the
existing texture archive. Imported controls use their own owning movie.
Unknown textures still report an error; no placeholder texture or skipped draw.

`tools/test-career-replay.ps1` launches the normal release executable with an
isolated copy of the existing profile. Its opt-in controller route enters
Career from the main menu, records real gameplay frames, opens StartHub and
selects Replay Editor. It requires the real intro and fade completion, checks
LT rewinding after RT scrubbing, exercises RB playback, and waits for rendered
and presented editor frames. The expanded route also enters Edit Timeline,
commits basic and advanced Freecam, checks the retained camera key, exits via
the original confirmation, requires a visible pause return, and resumes 120
simulation ticks. Periodic images show the intermediate UI; the final image
shows resumed gameplay. It checks
that the source profile's hash is unchanged. There are no injected recorder
frames or fabricated readiness callbacks.

### Freecam and exit follow-ups

The Edit Timeline -> Freecam crash was a translated ActionScript
property-access mismatch. The original effects timeline removes mcDropMarker
and mcRemoveMarker (depths 53/59, frame 29); camera timelines inherit that
absence. Native RefreshCursor 0x8264e2c0 still updates their alpha, and
AptActionInterpreter GetMember/SetMember 0x82a0ef30 / 0x82a0f160 tolerates the
undefined target. These two optional marker writes and dynamic bottom-menu
EnableButton writes now do the same. Required authored children still fail
when missing. Reference: replay-freecam-native.json and original APT dump.

The black screen on leaving Replay was a missing renderer acknowledgement:
update_replay_editor set frontend_reveal_pending, but the paused draw path never
prepared the game camera/PostFX and acknowledged the scene. That path now draws
the retained live world and acknowledges readiness without needing a simulation
tick. Native Replay2 OnUpdate 0x826799f8 waits for the live focus then stops and
pops; StartHub OnEnter 0x8265e4d0 opens the pause screen. The extended scripted
route checks the visible return and resumed simulation, not merely editor entry.
The branch that retains a Career challenge/NIS pause also acknowledges its
prepared full world frame, since it precedes the ordinary StartHub draw branch.

## Mandatory first-use name

Intentional requested PC behavior: when SKATER.P does not exist, the startup
wall's completed intro presents a mandatory typed-name popup. Empty/space-only
input is rejected, Escape/B cannot cancel, and Enter or A submits. This reuses
the Extras host text-input path and original popup artwork. The name is saved
through the existing ProfileData v3 memcard owner; other savables and the new
Career flag are preserved. Startup loads the saved name into the profile and
offline player-name owners.

Native references:

- `SetProfileName` 0x825647c0: name buffer, StringnCopy limit 31, and player 0
  name update; nickname is separate.
- `DoRequestProfileName` 0x82759248: `ID_KEYBOARD_PROFILE_NAME_HEADER`, eGamertag,
  keyboard limit 10. The host popup retains the ten-character input limit.
- Accepted-name callback 0x82743300.

The new popup timing/no-cancel rule is a deliberate user override, not a claim
that the original console used this exact first-launch flow. Host input uses
the existing printable ASCII textbox alphabet. No profile tests were run.

## Trick Guide

The StartHub row opens original FE state 18 / screen 32, with its fade/pause,
audio state, authored APT intro/outro and letterbox. It uses the original
`tricks/trickguide` movie (FNV fingerprint f3067239), imported picker and
gesture/button controls, descriptions and original localized strings.

Runtime Vault categories/subcategories supply 205 tricks and 27 Actions /
Terminology entries: Flips 62, Grabs 83, Grinds 32, Manuals 2, Plants 26,
Actions 14, Terminology 13. No fabricated move list. Class table iteration and
EASTL quick_sort's partition, insertion and tie behavior determine trick order.
Original APT wrapping, submenus, one-subcategory skip, back navigation, four
diagram slots, button kerning and connectors are retained. The native
`sMethod_IsGoofy` query returns false in this build; button swapping separately
uses actual stance as the authored script does.

The preview uses original ViewScene, camera takes, animations, character
recipe, room geometry/textures and lighting. Native scene-character animation
swapping resets time and takes the animation's length; ViewScene loops. The
gameplay scene is retained while the separate preview renders and restored on
exit. The preview updates on presentation time while gameplay is paused.
The selection timer follows FrontEndManager::Update 0x82640130's fixed
16-unit state update: decrement only in Idle while positive, reset to 60 on
an animation change. It is not a one-second wall-clock delay.

Native references:

- `FrontEndState_StartHub::ProcessSelection` 0x8265ea98.
- Trick Guide SetState/Setup/IsReady 0x82644be8 / 0x82644bf0 / 0x82644c60.
- OnEnter/OnLeave 0x8264e920 / 0x8264e9e8.
- OnUpdate 0x826605b0, SetNewTrickName 0x82660838.
- OnMenuHilight/ReceiveInput 0x8266c238 / 0x8266c300.
- PopulateMenuLevel 0x826834c8, OnLoaded 0x82687c08,
  OnMenuSelect 0x82687ca0; IsGoofy 0x82623fe8.
- NIS EnableTrickGuide/SetTrickGuideAnim 0x828a3110 / 0x828a3198.
- Scene view-mode start/update/request 0x8286e548 / 0x8286e788 / 0x82863108.
- AnimPool make path/load/request/viewing-name 0x828703b8 / 0x828704c8 /
  0x828705a0 / 0x82870868; character SwapAnimation 0x828754d0.
- Scene::Update 0x82870bb8 -> ICESceneBridge::SetTime 0x826cbd48 ->
  ScenePlayer::SetTimeElapsed for ViewScene looping.
- LooseWorld load 0x827e3510; render location 0x827d0d90,
  full Vault key 6568f34643066684. Original DIST_trickguide assets.
- gFEStateInfo 0x82fded80, 20-byte row 18: paused, music state 1.

Confidence: high for the recovered branches, catalogue and authored UI
translation; runtime visual/animation equality has not been demonstrated.
The host loads requested preview animation data synchronously after the native
selection debounce; the original asynchronous AnimPool lifetime is not copied.
Trick Guide and first-name runtime testing were not requested for this pass.

### Reported Trick Guide camera, exit and panel widths

Fixed the camera reference decoding: this attribute is a
ClassRefSpec_trickbook_camera_tracks (collection key then cached pointer), not
a generic Attrib::RefSpec (class key then collection key). The old read looked
up a nonexistent class/collection and obtained default track 0. Constructor
0x82653e50 supplies class 7cce51f922c0591d; SetNewTrickName's lha at 0x82660994
and 0x82660a80 reads the two-byte track layout. The corrected reader uses that
class and signed short. FS Grab's collection 57362217d1cd1ba9 selects track 3;
FS Noseblunt's a9d6dfeab8cb25bb selects track 1.

Fixed return-to-pause: Trick Guide hid StartHub on entry but called reenter
without clearing screen_hidden. It now uses return_from_replay, which clears
that flag and starts the normal intro/input gate. The retained gameplay scene
is restored by the existing loose-world close path.

The reported wider bottom panel is authored, so no arbitrary width correction
was made. The Xbox and PS3 files have byte-identical APT, CONST and GEO.
The full decoded Xbox BC3 payloads for all four panel textures are also present
byte-for-byte in PS3 trickguide.rps3. Upper artwork alpha spans x74..438 in the
512px texture; lower artwork x64..447. Their root/inner placements put the
lower panel farther left and slightly farther right, as in the reported screenshot.
The only constructor adjustment subtracts the same Screen_EdgeOffset from both
panels (zero here), with no panel scale operation. Exact hashes, bounds and
PS3 payload offsets: reference/trickguide-camera-layout-proof.json.

The reported screenshot is trickguide.png. New camera and
exit fixes still need a hands-on comparison; no additional Trick Guide
runtime test was added; the test scope was replay only.

## Career AI S.K.A.T.E. turn rules

The reported Shingo behavior is original. Alphabet challenge
`t_1b_01_skate_alphabet_shingo` (63c1d01b40e7ee1e) uses original scripts:

- follower_copy / TimerExpired, EA67BBE4F9368F09: increments FollowerIndex and
  enters prepare_follower, without changing LeaderID.
- process_round / StateEnter, 9736EDF1553348FC: the OnlineChallenge +
  AllFollowersHaveCopied special branch enters leader_get_letter. Normal
  Career updates elimination state and returns to round_setup with the same
  LeaderID when the match continues.
- round_setup / StateEnter, 93282A6A9A0A10BC: builds the next round around that
  existing leader; it does not transfer leadership to the successful copier.

No Career turn-rule change was warranted. This conclusion concerns the Career
challenge scripts; it does not assert that every Party Play rule is identical.

## Build and replay validation

First release compile exposed three new integration type errors;
the module path, sound-request conversion and character recipe ownership were
corrected. Release build 2 succeeded (12m56s). The first scripted run stopped
during startup on missing Trick Guide InitAction 100:32/0x45fc. The original
APT's raw root controls confirm that exact class registration; its translated
methods already exist. Added its precise initialization dispatch and rebuilt,
also incorporating the frontend timer correction. Both the copied and source
profile hashes after that failed run were
27320921DEC653166759E99232BC1DE525E072B08111A2137910C74F21522F09.
Release build 3 passed (7m32s). The entry-only run
`.scratch/replay-entry-test-20261013-115111-474/` passed Career entry, RT/LT,
RB playback and 1687 rendered / 1425 presented editor frames. The real profile
was unchanged. This predates the extended Freecam/exit route.

Builds 4/5 reached the final executable write but Windows denied replacement
while the game was running. After it closed, build 6 passed
in 10m46s. Its SHA256 is
E664C8EF5A50DFEC3A820212C217285D45E810B69298848FD14035A391560797.
Extended run `.scratch/replay-entry-test-20261013-123045-695/` reached the
Effects menu, then failed its selection check (1 instead of 2). This was a
probe error: the older neutral_demo_sample holds button bit0, D-pad Up. The
test now constructs a genuinely neutral sample between presses, preserving
the production camera/menu behavior and the strict selection assertion.
The real and copied profile hashes still match the value above.

Release build 7 **passed**, exit0, 3m26s. Normal command:
`cargo build --release -p skate2-pc -j 1 --timings`. Existing warnings:
core437 / PC18. Executable `target/release/skate2-pc.exe`, 43,705,344 bytes,
modified UTC 2026-10-13T09:41:31.7753313Z, SHA256:
446B21C7920DADDDD6D6471E259AA7E8315F32997A606A7D3F618BE020810503.

The corrected extended replay test **passed**, exit0:
`.scratch/replay-entry-test-20261013-124150-068/`. It entered Career replay
with 183 actual recorded frames, scrubbed RT to parameter1 and LT to
0.72968745, exercised RB playback, entered Timeline, opened/moved/committed
basic Freecam, opened/moved/committed Advanced Freecam, and retained its key
(two camera intervals). Exit confirmation returned to the visible pause menu;
the test waited for at least120 presented pause frames and then resumed at
least120 simulation ticks. It logged 7042 rendered / 6517 presented editor
frames, 493 total completed simulation ticks, 79.764s wall time. No game error.

Inspected captures: replay-roundtrip_017.png shows Free Camera/Commit;
replay-roundtrip_033.png shows the restored StartHub over the live skater/world;
replay-roundtrip.png shows resumed gameplay/HUD. This is a functional replay
regression, not a performance benchmark or a complete original-game visual
comparison. The source and copied SKATER.P hashes both remain
27320921DEC653166759E99232BC1DE525E072B08111A2137910C74F21522F09.

Scoped whitespace review passed. No Trick Guide or first-name gameplay tests
were run; the test scope was replay only. Those camera/exit fixes were
checked against IDA and original assets and are included in this release;
hands-on comparison remains pending. Saved-footage, every camera/effect, and
all possible challenge contexts were not exercised by this route.

Backups, decoded original APT/Vault/Lua, native packets and build logs:
`.scratch/replay-name-trickguide-20261013/`. Original game files remain external.
No Android changes; prior PC overrides are retained.
