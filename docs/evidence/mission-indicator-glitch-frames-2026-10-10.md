# Mission indicator glitch frames (2026-10-10)

The indicators floating over the mission spots (the icons above the AI skaters)
distort for a moment: lines or vertices jumping away, then the icon recovers. All addresses are
`sk82_na_m.xex` (Jan 17 2009) VAs, read in IDA (read-only). The indicators are not skinned or
animated models: they are `VIPresentation` location ribbons (`career-visual-indicators-
2026-10-09.md`), B-spline control points jittered again every `RenderSettings` jitter period
(0.1 s for the sign-up icons) and drawn by the `spline` effect.

## Result

- The reported glitch is native. A ribbon slot keeps its control-point layers when the
  nearest-first ribbon order hands it a different icon, until its next jitter rebuild (at most
  5 frames at 60 Hz). The port does exactly that and is left as it is.
- Checking the other users of the code path found a port-only glitch in the follow trail
  (`DynamicIndicator`): the port ran `VIPresentation::Update` on every drawn frame, which broke
  the native 60 Hz timing that adds trail points and rebuilds the jitter on the same frame. The
  update now runs once per shown tick with whole ticks, as natively; the frames in between only
  turn the billboards to their camera.

## Reproduction

A temporary per-frame check (since removed) compared every drawn control-point layer with its
spline's own control points (both edges, the drawn range). Jitter and width keep a fresh layer
within about 1.3 m (m_1a_01's objective ribbons, not billboards, reach about 1.6 m).

Runs: Continue Career on a scratch copy of `perf/saves_nbs_fix3` (2 completed), with
`run_perf.sh` and the career demo. Three sign-up ribbons are within range:
- A, the bulb (tutorial, spline asset `0x00019af403e38707`, 29 segments) at
  (1644.8, 102.6, -545.8);
- B, the camera (photo, `0x00019af203e38707`, 35 segments) at (1671.6, 104.4, -521.0), the
  photographer below it;
- C, a bulb at (1631.5, 102.6, -497.7).

Findings:
- `ind_c1`, `ind_c2`: while the skater rolls, B and C swap places in the nearest-first list,
  so slots 1 and 2 swap icons. Each slot, once in view, drew the other icon's points: off by
  2.05 m and 2.20 m for 0.06 to 0.1 s, then rebuilt.
- `ind_c2`: standing 7 m from A for 25 s gave no stale frame. Captures every 0.1 s show only
  the jitter.
- `ind_c4`: travel to (1656.04, 101.5, -535.42), facing B (yaw 0.824), then pushes. When B
  became the nearest, slot 0 (A's bulb until then) drew B's spline over the bulb's points for 4
  frames (log 31.384..31.412, tick 615..617).
  - Captures of every frame: 331 to 334 show the bulb's outline where the camera icon stands,
    plus a long straight line off to the side.
  - The line is the camera spline's last 6 control points: that slot had never written them, so
    they are zero with W 0, which puts them at the world origin.
  - Frame 335 shows the camera icon again.

## Native

### A slot keeps its layers when its ribbon changes

- `VDWriter::_PrepareChallengeVisualIndicators` 0x827ff138 gathers the ribbons around the local
  skater's position (vtable +8, 0x827ff220), through `GetVisibleLocationIndicators` (0x827ff250).
  `AddLocationRibbon` 0x827ff050 appends them in that order.
- `cVisualIndicatorGroup::AddIndicatorsFromGroup<tLocationIndicator>` 0x82db0c30 sorts each group
  by distance to that point (`quick_sort` with `SortLocationIndicatorOnDistanceToPointOfInterest`,
  0x82db0dc0). `cChallengeSignUpManager::GameLoad` 0x82daec00 creates one group (0x82daec78) for
  every sign-up, so the sign-up icons trade places as the skater moves.
- `VIPresentation::UpdateLocationSplines` 0x82803528 gives record i to slot i. On a different
  spline (0x82803618) it only sets `mpSpline` and `mEnable` and calls
  `SplineIndicator::CalculateBounds` 0x828010c8 (0x82803628). Then come `SetLocalToWorldMatrix`,
  `Cull`, and `Update` when `Cull` passes.
- `SplineIndicator::Update` 0x828013f8 adds dT to `mAnimTime` (0x82801438). It rebuilds the
  layers (`mControlPoints[3][142]`, for the current spline's `mCPCount`) only once `mAnimTime`
  reaches the settings' jitter period (+0x38, compare at 0x8280157c), and then zeroes it
  (0x82801748).
- `mAnimTime` has no other writers: a scan of every `VisualIndicator`/`VIPresentation` function
  finds only `Init` 0x828023e4, those two, and `DynamicIndicator::Update` 0x828017b0/0x828019f8.
  `Init` 0x828023c0, `CalculateBounds` and `SetLocalToWorldMatrix` 0x82800548 leave the layers
  alone.
- `SplineIndicator::Render` 0x82802940 draws the current spline's vertex buffer (`mVtxCount`,
  0x828029d4) over `mControlPoints[0..default layers]`.
- So from the swap to the next rebuild, the new icon's vertices are evaluated on the old icon's
  jittered points. Past the old icon's `mCPCount` the slot holds whatever it held before.
  - `AddInstancedSplineInstance` 0x828050a8 allocates the 22 slots (0x82805214, 0x1bf0 bytes),
    and neither the constructor 0x82804cf0 nor `Init` clears the layers.
  - The port starts them at zero, hence the line to the world origin.
- `UpdateGateSplines` 0x828037e0 reuses the race gate slots the same way.

Not changed, for parity. Should a deliberate deviation be wanted, it is one line in
`ViPresentation::place`: when the spline changes, set the slot's `anim_time` to its jitter period
so that its next `update` rebuilds the layers.

### The follow trail's timers meet on the native frame

- `VIPresentation::Update` 0x828055e0 takes dT from the presentation tick count:
  - (ticks + delta change) × 0.016666668 (0x82805680);
  - its absolute value (`fsel`, 0x82805688);
  - zero from 15 ticks away (0x8280566c).
- `UpdateDynamicIndicator` 0x82803fa0:
  - `mDynamicIndicatorSampleTimer` += dT (0x82803fe4);
  - above `kDynamicIndicatorPointTime` (0.05, 0x82803ff8): `AddPoint` 0x82803df0 (0x82804050)
    and the timer less 0.05 (0x82804064);
  - then `DynamicIndicator::Update` 0x82801770 (0x82804090): `mAnimTime` += dT, and at the
    `key_trail` jitter period (0.05, 0x82801868) a rebuild and `mAnimTime` = 0.
- `AddPoint` writes the trail spline's control points (0x82803f30), not the layers.
  `DynamicIndicator::Render` 0x82802c00 draws 2 (points - 3) samples vertices, so the newest
  point's layer entry is drawn.
- At 60 Hz, dT is 1/60 a frame. In single precision three of them make 0x3d4cccce, above
  0x3d4ccccd, so both timers end on every third frame together: `AddPoint`, then the rebuild.
  A new point is jittered before it is ever drawn.
- The port called `Update` on every drawn frame with fractional dT. The point timer keeps its
  remainder and the jitter timer drops it, so at hundreds of frames a second they drift apart.
  - Points arrived frames before a rebuild. The newest drawn layer entry was still the jitter of
    the empty trail's zero, up to 1680 m from the point (the world origin).
  - At the end of the last segment it weighs t³/6 = 0.12, so the trail's head shot about 200 m
    toward the world origin for up to 0.05 s, on most points while the trail grows.
  - `ind_f1`: 633 such frames in 4 s of m_1a_07's heat.

## Changes

- `crates/skate2-core/src/challenge/vi_presentation.rs`:
  - `SplineIndicator::face_camera` is `Cull`'s billboarding (0x828012c4..0x828013b8), split out
    of `cull`.
  - New `ViPresentation::face_camera` turns the enabled, visible ribbons (and gates) to a
    camera. Time, jitter, the trail and visibility are left as the last update set them.
  - The `update` doc states the native timing.
- `apps/skate2-pc/src/visual_indicator_view.rs`:
  - `VIPresentation::Update` runs once per shown tick, with the whole tick and a zero fraction,
    so dT is 1/60 a tick as natively.
  - The other frames call `face_camera` with their own camera.
  - The location ribbons now rebuild every 6 ticks, as natively; visibility follows the tick's
    frustum.

## Also checked (matching, unchanged)

- `spline.fx` `defaultVS` against `visual_indicator.wgsl`:
  - t = frac(x) and the index = floor(x);
  - the uniform cubic B-spline weights, divided by 6, with P.w = 1;
  - the clip-depth fades.
- `Spline::InitFromSplineHeader` 0x82801d00:
  - the control points: the first segment's translation four times, then each segment's rows
    summed less `mBC`, then the last three more times, on both edges;
  - the vertex pairs (t, t, 0) and (t + 71, t, 1) at 0.1 steps.
- `Spline::Update` 0x82800f60: 2 × segments × min(samples, 10) vertices at 1/samples steps.
- `cParticleRandomManager::Build` 0x828009a8: four draws of the 0x10dcd/0x29 LCG, mantissa
  [1, 2) less 1, for W, Z, Y and X.
- `SplineIndicator::Update`'s rebuild loop, VMX decoded:
  - near = point + (random - variance / 2);
  - far = near - (right + up) × width, with the camera rows' W cleared.
- `Cull` 0x82801278's billboard, decoded: Z = -at with Y cleared and normalized, X = (z, y, -x).
- The PC side:
  - per-layer uniform records (2432 bytes at 256-byte offsets);
  - vertex buffers by count and step;
  - one strip per layer, darken then default.
- Race gates and the streamed world splines are not ported and draw nothing.

## Verification

- `cargo build --release -p skate2-pc` passes, with no warnings in the changed files.
- `parity.sh ind_parity`: PARITY IDENTICAL (6000 lines).
- `ind_f2` (the diagnostic build with the change): m_1a_07 (`SKATE2_DIAGNOSTIC_CHALLENGE` at
  tick 3000) gave 0 stale trail frames over the 7 s heat.
- `ind_c5` (the same build): the swaps above remain, as natively, for 1 to 3 ticks, and the
  camera icon draws normally over 160 consecutive captured frames.
- Final build (`ind_f3`): the m_1a_07 heat runs (the skater stays put, so the trail leaves
  the view); captures deleted.

Not run: the skate2-core tests (memory). The shipped-data test in `vi_presentation` already
calls `update` once per tick with a zero fraction.
