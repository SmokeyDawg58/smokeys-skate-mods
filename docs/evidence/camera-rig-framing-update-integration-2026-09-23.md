# Rig framing update integration handoff (2026-09-23)

This is an isolated, unapplied continuation of `Iris::Rig::DoFramingUpdate82e42ee0`. The independently reviewed source proof is in `camera-rig-framing-update-next-source-2026-09-23.md` and `camera-rig-framing-update-raw-2026-09-23.md`. The ordered `FCMPU/BLT` umbra correction is present in the Rust file and covered for NaN.

## Artifacts

- `camera-rig-framing-update-integrable-2026-09-23.patch` changes seven registered files. It registers six live camera framing cvars in native initializer order, exposes the existing framing basis owner, and threads the separate `shot_editor` argument through CameraMan to Rig. It replaces the stop at `PendingRigFramingUpdate` with the Rig call and a new stop at `PendingCameraFramePublication`. The patch was made from temporary copies and has **not** been applied here.
- `../../crates/skate2-core/src/input/tuning/input_cvar_camera_framing.rs` is a new unregistered Rust module for live cvar references 336–341 and defaults/override tests.
- `../../crates/skate2-core/src/camera/rig/camera_rig_framing_update.rs` is a new unregistered Rust module for the Rig continuation. It reuses the retained trackers, target quaternion basis, Euler owner, numeric service, and existing framing basis. Its cvar reads remain at each source use site.
- `../../crates/skate2-core/src/camera/rig/camera_rig_framing_update_tests.rs` is a new unregistered test module. Tests cover ordered NaN umbra behavior, source cvar read order including duplicate reads, shot-editor matrix-angle selection, roll matrix W lanes, and the partial write prefix when a late pitch parameter is missing.

## Validation and boundary

`rustfmt --edition 2024` completed for the three new Rust files. `git apply --check -- docs/evidence/camera-rig-framing-update-integrable-2026-09-23.patch` passed against the current shared tree. Per integration ownership, this agent did not edit any registered source file or run Cargo. The root integrator should apply the patch, then format, compile, run the focused tests, and test with real assets.

The final camera frame publication after caller `82e48490` remains unimplemented. Host numeric exceptional-bit parity, including PPC VMX and FPSCR behavior, remains a separate runtime-trace validation item.
