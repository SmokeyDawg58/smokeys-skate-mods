# Camera frame 0 speed producer and publication boundary (2026-09-23)

Read-only IDA and current-Rust audit. Exact `Test-Path` was false before this
file was created. No registered Rust, Cargo command, or IDB state was changed.
This adds the live velocity-producer link to the existing
`camera-shake-subject-inputs-source-2026-09-23.md` and
`camera-shake-apply-publication-audit-2026-09-23.md`; it does not reimplement
their already drafted input helpers.

## Live ownership and order

`CameraMan::UpdateRig82e47e68` saves `shotEditor` entry `r6` in `r20`
(`82e47e94`), narrows it into `r21` (`82e47f84`), and calls
`CameraMan::UpdateRigSubjectMatrix82e41208` at `82e47f9c` only when that flag
is false (`82e47f8c..90`). The mounted first-skater caller passes false, as
mapped in `camera-update-argument-map-2026-09-23.md`. This is the normal
frame-0 producer before framing and shake.

In `UpdateRigSubjectMatrix`, `82e41260..68` calls `UpdateAnchor82e3e6a8`
with frame dT. Its `82e3e71c..40` reads the selected Subject anchor by the
retained `CameraMan+0x310` anchor index and stores the full vector at
`CameraMan+0x270` (anchor position). When the source `fcmpu dT,+0; ble`
and teleport/reset checks allow a velocity update, `82e3e758..790` subtracts
the old anchor vector from the new one, divides 1 by dT, splats that scalar,
and stores the full product at `CameraMan+0x290` (anchor velocity). A skipped
update leaves the old velocity in place. The source calls `UpdateAnchor(0)`
first on anchor-index change (`82e41230..4c`) and sets `Rig+0x1d9` bit
`0x40` (`82e41250..5c`), then runs the ordinary dT call.

`82e4126c..a0` reads that `CameraMan+0x290` vector, subtracts the prior
`Rig+0xe0` subject velocity and stores the difference to `Rig+0xf0`
subject acceleration, then copies the **full** anchor-velocity vector to
`Rig+0xe0` subject velocity and zeroes `Rig+0x100` subject angular velocity.
The call then builds the anchor matrix from Subject's effective board basis
and `CameraMan+0x270` translation (`82e412a4..41300`). Current Rust has this
same owner chain in `camera_rig_subject_matrix.rs`: `try_update_anchor`,
`self.anchor_velocity`, `self.rig.subject_velocity`, and
`self.rig.anchor_matrix`. Subject `board_velocity` is a separate field and
is not the source of this shake-speed operand. Source confidence: **exact**
for these reads, writes, branches, and order; host numeric bit parity remains
**medium** without a PPC trace.

## Scratch to output

After `Rig::DoFramingUpdate` and frame angular-velocity publication, the
same `UpdateRig` reads `Rig+0xe0` at `82e48678`. Its length operand contains
only the velocity X lane; Y/Z/W are +0, so ordinary finite speed is
`abs(anchor_velocity.x)` (`82e4868c..e0`). This value is stack scratch until
loaded as Apply argument f2 at `82e48744`; it does not write a `CameraMan`
speed field or `Frame` member. The distinct
`CameraRig::try_subject_speed` in `camera_rig_speed.rs` computes signed speed
for shot/condition consumers and cannot replace it. The unregistered
`camera_shake_subject_inputs.rs` already implements this X-only helper,
wipeout/collision activation, and anchor-minus-camera distance.

The source forms activation with `GetIsWipingOut` (Subject vtable +0x17c)
and positioner collision, then tests `shotEditor` at `82e48714`. The true
branch writes four direct frame-matrix columns and joins previous-matrix
selection; the live false branch computes subject distance and calls
`ShakeEffect::Apply82e43700` at `82e487a8` with f1=dT, f2=speed,
f3=distance, r8=staged matrix, r9=activation. Apply's returned columns
become `Frame.matrix` (`82e487b4..d0`); its shake offset is passed to
`LogicalCameraPositioner::SetShakeOffset` (`82e487d4..e8`). Only then does
`82e487ec..48854` select the previous matrix from either the newly
published matrix (hard cut/teleport) or the saved old matrix. Source
confidence: **exact** for caller ABI, order, and branch; the returned Apply
matrix calculation is unresolved.

## Current Rust and decision

`CameraMan::try_compose_frame` currently publishes the reference/framing,
camera velocity, and angular velocity, then returns
`PendingCameraSubjectSpeed`. It does not execute the available unregistered
scratch helper or publish a matrix. Moving the diagnostic after the scratch
helper alone would still leave the normal frame at the unrecovered Apply call.
The earlier Apply audit identifies missing BezierLite evaluation, fractional
phase and keyframe selection, three SinCos operations, and VMX matrix/offset
lanes. Native Apply writes multiplier and time before matrix output, so a
partial write implementation would also need exact failure-prefix behavior.

No new implementation patch is proposed: the unique source-proven input
helper already exists, and the smallest complete live frame continuation is
Apply followed by frame matrix, positioner shake offset, and previous-matrix
publication. Next audit should prove Apply's remaining phase/rotation/output
lanes against raw instructions and the mounted `.shk` count/array contract;
then independently review a same-owner Rust continuation and run the focused
camera test plus original frame-0 baseline. Neither test was run here.
