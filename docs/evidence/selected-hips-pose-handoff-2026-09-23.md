# Selected Hips pose handoff, grounded controlled run (2026-09-23)

The original-data release run with `SKATE2_TRACE_ADJUSTED_POSE=1` completed
30 selected frames with zero exit. The current capture is
`artifacts/controlled-skating-release30-hips-stages-2026-09-23.log` and `.png`.
The skater still renders reclined, while frame 30 physical state is Ground,
four wheels are processed, and board Part 6 is upright. This is a controlled
Rust trace, **not** a matched native per-tick measurement.

The body hierarchy maps named `hips` bone 1 to Skeleton Part 23. At frame 30,
the completed PhysRecord Part 23 position is approximately
`[588.7606,-53.4982,996.5284]`; its local Y basis is
`[0.4646,0.2960,-0.8346]`. `UpdatePoseBuffer` publishes AnimOutPhysIn local
Hips with Y basis `[0.8332,0.2960,0.4671]`, position
`[-0.02375,0.93438,-0.10110]` in animation space. That change in basis is
expected from the retained `world_to_animation * PhysRecord * volume_to_bone`
chain; comparing raw Part axes directly to animation axes is invalid.
The submitted 208-byte presentation header's Hips matrix matches that exact
AnimOut local Hips at frame 30. Frame 1 has different Hips rotation and
translation, so the physical-to-header channel is live.

At the `Decoded` boundary replay Hips SQT follows the changing header:
frame 1 quaternion approximately `[.6059,.7203,.1831,.2838]`, translation
`[.02591,1.00237,-.08338]`; frame 30 quaternion
`[.5775,.7736,.1358,.2226]`, translation
`[-.02375,.93438,-.10110]`. The `Corrected` boundary saves this changing
rigid Hips matrix and resets replay Hips to identity, matching native
`MoveRootToHips_Individual` at `0x827d35d8`. The finalized hierarchy's named
Hips slot 1 is identical in frames 1 and 30; the first published mesh rows
also retain a large, nearly constant Y/Z rotation. This locates an apparent
visual disconnect **after** header/decode, but does not prove a bug by itself.

Independent IDA review of native `UpdateBodies` `0x827deb48..0x827dec24`
resolves the saved-Hips destination: it writes the four matrix vectors at
offsets `0,+0x10,+0x20,+0x30` of each `mInterpolatedData[i].mPoseBuffer`
base, i.e. final matrix slot **0**, not named Hips slot 1. Rust
`publish_saved_hips` writes `interpolated[row]` slot 0 and is correct on this
point; changing it to slot 1 would break source parity. Native
`GetHipsMatrix` `0x826c70d8` composes the presented root with the saved Hips
matrix. The next source question is which local-to-world matrix the skater
model supplies to `PackAndMultiplyMatricesForUpload`: root only or root
combined with saved Hips/slot 0. Rust selected palette currently uses the
submitted same-frame root only. Inspect the native callsite and actual model
bone remaps before changing that product.

The native callsite resolved that question. `SkaterPresentation::DoSimRestore`
`0x827e7160..7248` loads row root at `+0x30` and saved Hips at `+0x70`,
computes **root × saved Hips** with four-term column multiply, then calls
`PresentationEntity::SetLocalToWorldMatrix` `0x827efd08`, which stores
`m_MatLtoW` at entity `+0x160`. `SkaterPresEntity::EndJobs` `0x827dbe24`
passes that exact field to `cModelInstance::PackAndMultiplyMatricesForUpload`
`0x828221e0`. `UpdateBoneTransforms` only gathers hierarchy matrices through
the model palette remap. The earlier Rust selected palette supplied root
alone, omitting physical Hips placement from the model upload. Two independent
read-only source audits confirmed the call order and destination.

`SelectedPresentationRuntime::try_update` now gives the existing packer the
retained submitted root multiplied by the saved Hips, using the existing
ordered `move_root::multiply` expression. It leaves the native slot-zero
overwrite intact. The post-fix release captures
`artifacts/controlled-skating-release30-matltow-hips-2026-09-23.png` and
`artifacts/controlled-skating-release120-matltow-hips-2026-09-23.png` both
exit zero and visibly show the skater upright on the board instead of
reclined. The frame-30 camera clips the head; frame 120 shows the full
upright skater from the side. This visual result does not establish matched
native pose ticks, full input behavior or correct camera framing.

Read-only diagnostics were added under `SKATE2_TRACE_ADJUSTED_POSE` in the
selected public facade. They record live Skeleton Parts 1/23 after solver,
completed PhysRecord/mapping, AnimOut global/local Hips, submitted header
Hips/board, decoded and corrected replay Hips, saved Hips, and final Hips.
The default runtime path does not perform these reads. `cargo check --workspace`
passed after the behavioral correction in
`artifacts/check-skater-matltow-hips-2026-09-23.log`; full workspace tests
passed in `artifacts/workspace-skater-matltow-hips-2026-09-23.log`, and both
mounted original-data facade/cross-owner frame tests passed. Confidence is
**high** for the native local-to-world source path and controlled visual
correction, **exact** for the native slot-zero write address, and still
unproven for reference per-tick pose parity.

`cargo fmt --all -- --check` passed. Strict workspace Clippy with
`-D warnings` remains red across unfinished repository paths (152 lib-test
errors in this run); its full output is
`artifacts/clippy-skater-matltow-hips-2026-09-23.log`. The changed selected
runtime matrix composition produced no direct Clippy diagnostic. A small
redundant-closure diagnostic in the optional trace was corrected afterward.
