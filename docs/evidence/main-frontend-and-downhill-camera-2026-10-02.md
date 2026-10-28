# Main frontend and downhill camera audit ? 2026-10-02

Release build and final live checks passed; see final validation below.
The first runtime attempt exposed a repeated frontend-pause push
(StackCapacity). Entry now pushes once; returning from StartHub retains
its existing pause.

## Original evidence

Live IDA was available throughout this pass. Frontend sources:
- FENISManager::RunNIS 82615470; original sequence table82fdb240.
- MainFEState82778340 and UpdateFEState82639648 choose startup tracks0/1
  and return track6. Camera completion (Scene::Update82870bb8), rather
  than the longer scene audio duration, gates entry to state22.
- MainMenu OnEnter82659fd8, OnLoaded82648740, OnGainFocus8264a018,
  OnMenuSelectGame82681338, StartSinglePlayer82678c20.
- StartHub IStartHubState::OnMenuSelect82674d08 opens template153 when
  no career activity is active. gPopupTemplate82fdbc58, stride56, entry
  82fdddd0: warning style0, ID_COMMON_WARNING_TITLE,
  ID_QUIT_GAME_MESSAGE_NO_DATA_LOSS, NOT cancellable, No/Yes, default0.
  QuitPopupCallback82671678 quits only for index1.
- RenderToTexture8261dd30 renders APT layer1 separately. Render3D8262a550
  places the offscreen texture on a world quad with depth test/write and
  native RGB tint bits3f34fdf4. Render2D82631a98 excludes layer1.
- fe_3d_screen5453b391061ee548 supplies original wall position/rotation/scale.
- SetApt3DMatrix82601870 raw instructions preserve the APT affine matrix;
  menu placement comes from the original mainmenu movie, not screen guessing.

Original ICE and APT assets stay external; extracted research copies under
ignored artifacts/frontend-source. ICE camera decoder follows
LoadICECameras82a3e8a8, SetDataPointers82a366f0, RegisterElements82a4e8b8,
GetSlope82a390e8, UpdateLens82a36be0, UpdateShake82a39db0,
CreateWorldCameraMatrix82a371b8, ConvertMatrix4ToSkate826d2f68.
ATan table830022f8, Sin82a33330. Recorded shake retains unusual source
semantics: two square roots for quaternion W, and identical position key
index for both interpolation endpoints (verified raw PPC).

ICE track keys: BootToPressStart da792ba5, PressStartToMainMenu30a969ab,
GameToMainMenu3dcbc1ca. Frontend context80000006 and recorded-shake
context80000002 are decoded; unsupported populated channels fail explicitly.
Original mainmenu APT fingerprint4860d06f and its shared picker/fonts are
reused. Only Continue Career has a destination in this scope; other rows
remain navigation-only. This integration automatically advances past the
boot Press Start point; profile/online/career selection is not implemented.

## Camera audit

The airborne camera stays high on any downhill road, especially
between manuals; landing restores height even while maintaining a manual.
Freshly checked CameraMan::UpdateTrajectoryData82e44dd8,
Rig::UpdateReferenceHeight82e3fb80, UpdateAnchor82e3e6a8,
GameCameraManager::UpdateSubject82e38e80, KnownAir::InitTrajectoryInfo82d992a0,
KnownAir::FillPhysOut82d8c348, PredictionResults::GetCollisionPosition82d8a648.
Current formulas agree: airborne landing-height blending is gated by valid
trajectory, completed previous blend, onboard state and relative landing Y
strictly greater than -3.0. Collision can cancel the blend. Native grounded
tracking uses0.3 and grinding0.05. None of these values were retuned.
GetCollisionPosition really evaluates the trajectory at collision time,
not the stored contact point, despite the misleading Hex-Rays parameter name.

Added opt-in SKATE2_TRACE_AIR_CAMERA reporting source input values and blend
state, and raw-input downhill-manual sequence. No normal gameplay logging
is enabled by this addition. A runtime conclusion is still pending.

## Validation so far

- Original ICE decoder: all three frontend tracks, 3003 world-camera samples,
  finite output, FOV bounds and near-unit axes PASS (ice_probe.exe).
- Current-source standalone StartHub confirmation test PASS: No default,
  source non-cancellable policy, focus gating and one-shot Yes dispatch.
- First release build PASS, build2.log; first live run failed with the pause
  stack issue described above. A corrected build is in progress.
- Diagnostics use isolated artifacts/frontend-source/diagnostic-saves;
  normal SKATER.P is not modified.

## Confirmed camera defect and correction

The first downhill trace found camera second_flags=a0 on all811 frames.
Even during44 valid KnownAir frames, blend_duration stayed0. The preserved
teleport latch80 forced the collision/cancel branch in UpdateTrajectoryData.
This was not caused by the -3.0 threshold or a missing KnownAir landing.

Fresh full CameraMan::Update82e48870 plus raw82e489c0..82e48a74 proves a
missing post-UpdateRig suffix. Restored in the existing update owner:
- OR logical positioner hard-cut into CameraMan flag40.
- If second20, call existing CameraMan::Teleport; otherwise grounded clears80.
- Set existing ShotManager.instant_transition from heading-follows-model time.
- Read native kCameraNotReadyDuration, default30, range0..300, initializer
  82fa27a8. Unsigned compare marks ready40 after the threshold.
- Publish Frame hard-cut and parameter near-clipping-distance-min.

Cvar1245 inserted after kbEnableAirCameraCollision463 and before
kbShowReferencePoint331 in actual original initializer order (table82fd6648).
The teleport and camera owners already existed; no second camera introduced.

First main-menu capture shows original animated alley camera and projected
wall picker. Roundtrip returned to the wall after213 actual gameplay ticks.
Both runs then hit a diagnostic completion check that wrongly required600
SIM frames while paused. This was a harness error; frontend diagnostics now
use their menu/roundtrip assertions instead. Final rerun remains pending.
A separate downhill input exploration encountered pre-existing motion state
2190429384 at tick811. It is not suppressed or claimed fixed here.

## Final validation and remaining limits

- Release build PASS, build5.log, 7m26s, existing warnings only.
- Main-menu startup/navigation, original camera completion and selection
  restoration PASS, exit0, main-menu-final.log/.png. Original wall menu
  capture inspected. Up/down navigation returns to Continue Career.
- Main-menu roundtrip PASS, exit0, main-menu-roundtrip-final.log/.bmp:
  Continue Career resumes213 gameplay ticks, StartHub Quit confirmation Yes
  returns to settled original wall menu. Simulation remains paused there.
- Downhill ollie into manual PASS, exit0,260 ticks,
  downhill-first-landing.log/.png.44 valid airborne samples,43 with active
  landing-height blend. Predicted landing is0.27663m below launch; camera
  reference moves down0.12148m during flight. Teleport flags release after
  initial two updates; readiness40 appears after the native30 threshold.
  analyze_camera.py --require-blend PASS. Capture shows Ollie -> Manual.
- Longer downhill sequence again encounters the unrelated unimplemented
  motion state2190429384, now at472 ticks. Not hidden, bypassed or fixed.
- Normal SKATER.P untouched: all tests use isolated diagnostic-saves.

This is not a claim of complete frontend presentation parity. Original ICE
focus/bloom channels are decoded but not applied to missing postprocess
owners; ambient frontend actors are not instantiated. Scene visibility uses
existing streamed geometry at the final camera focus, not a complete native
frontend scene-lifetime implementation. Capture has some bright environment
artifacts in the distant yard that remain unaudited. Other menu destinations
are deliberately navigation-only. Startup automatically passes
the Press Start point; profile/service workflows are not implemented.

Run play-quiet.bat to enter the new frontend; Continue Career enters the
existing skating session. Pause -> Quit Game -> Yes returns to the wall.
No recompile is needed for the verified executable.
