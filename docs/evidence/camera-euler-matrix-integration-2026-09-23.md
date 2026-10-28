# Matrix-to-Euler helper handoff (2026-09-23)

Integration update: the patch below was independently reviewed and applied.
Five focused tests pass in `artifacts/board-adjust-euler-focused-2026-09-23.log`.
The separately reviewed angular-publication continuation now calls this helper
old-basis first, then new-basis, and stops before subject-speed work. References
below to an unapplied patch or a pending integrator Cargo run describe the
author's earlier handoff state.

Source proof: `camera-euler-matrix-source-2026-09-23.md`. The isolated registered-file patch `camera-euler-matrix-2026-09-23.patch` is unapplied. It registers the new `camera_frame_euler.rs` module and extracts the existing camera heading one-refinement ratio/`XMVectorATan` graph into a pending result, allowing the Euler helper to issue all atan producers before its quadrant additions. No other registered file was edited here.

The new `camera_frame_euler_tests.rs` distinguishes the normal and near-vertical/NaN branches, two versus three atan calls, source ratio-denominator order, signed-π quadrant correction, +0 low-branch Z and W=X output. Rustfmt passed on the two new files and `git apply --check -- docs/evidence/camera-euler-matrix-2026-09-23.patch` passed on the current shared tree. No Cargo command was run by this author; root should compile and run focused tests only after independent source and patch review.

This is a source-proven **helper**, not a live caller continuation. `CameraMan::UpdateRig` must still call it old-basis first and new-basis second, publish the new-minus-old Euler difference and three ordered `Math::Angle::Normalize` results, multiply by inverse dT, then complete subject speed, ShakeEffect and frame-matrix publication. The current pending stop remains correct until that caller segment is implemented. Numeric service estimates and NaN payloads await a PPC trace for bit parity.
