# Grind Transparency and the skater's fades — October 19, 2026

When the original makes the skater see-through, checked step by step against the port. Native
addresses are the debug `sk82_na_m.xex` (Jan-17-2009).

## Native

1. **The option.**
   - `FrontEndState_Options::ToggleGrindTransparency` (0x8265c8b8) flips the profile's
     `mGrindTransparency`, which `tSettings::Default` (0x825644d8) leaves off.
   - `CameraManager::Update` (0x82584448) copies it into `GrindDetector.mIsActive` every update.
2. **When the detector runs.** `CameraMan::PreUpdate` (0x82e453f8) calls `UpdateGrindDetection`
   (0x82e3e8b0) for camera 0 while `kbEnableGrindDetector` (default 1) is set:
   - It runs only when the horizontal `Rig::mSubjectVelocity` (Rig+0xE0) is at least 0.01. A
     standing skater keeps the last anticipation.
   - The frustum's rows are `[cross(up, forward), up, forward]`, turned about the first by
     `mGrindFrustumPitchOffset` × −π/180 (`RotateAroundArbitraryAxis`).
   - The frustum sits at the board's effective position plus forward × `mGrindSkaterOffset`.
   - `Update` gets the speed and `PhysOut.mGrinds->mIsGrinding`. `UpdateGrindAnticipation` gets
     the camera frame's position and the skeleton hips.
3. **`GrindDetector::Update`** (0x82e385e8):
   - It forgets the last rail, and stops there while grinding or inactive.
   - The lookahead is `mGrindLookaheadTime` × speed, clamped to `mGrindMin/MaxLookaheadDistance`.
   - The frustum is raised by `mGrindSkateboardOffset`.
   - **`FindBestGrindCandidate`** (0x82e37430):
     - The box spans the far point and four rays (pitch 0 or −vertical, yaw ± half the
       horizontal angle). Each ray ends at `position + radius·(sin yaw·cos pitch·side −
       sin pitch·up + cos yaw·cos pitch·forward)` (`AddFrustumRayToBox` 0x82e36900), so the box
       reaches up from the board, capped at `mGrindMaxRelativeHeight`.
     - `GrindData::BoxTest` keeps vertical primitives and returns at most 40.
     - Of these, the nearest qualifying nearer end is kept.
4. **`UpdateGrindAnticipation`** (0x82e37338):
   - The anticipation rises at `mGrindFadeInSpeed` while `GetCameraToGrindIsObscured`
     (0x82e36b60) holds, otherwise it falls at `mGrindFadeOutSpeed`.
   - `GetCameraToGrindIsObscured` holds when the hips are within 0.3 of the camera's line
     through the rail's end, on the ground plane. The line is not clamped to the segment.
   - The value is clamped to 0..1 and eased by `sin(a·π − π/2)·0.5 + 0.5`.
5. **`CameraMan::UpdateRig`** (0x82e482e4..0x82e4833c): `mOpacity` is
   `min(shot opacity, (1 − mGrindOpacity)(1 − eased) + mGrindOpacity)`.
6. **`Actor::UpdateSkaterOpacity`** (0x825f1240): the camera's skater takes
   `GetCameraSkaterOpacity`, for the main camera only. Other skaters take 1.
7. **`SkaterPresentation::DoSimRestore`** (0x827e7298..0x827e73e4):
   - The target is the frame's opacity byte.
   - The current opacity approaches `EvaluateProximityOpacity` (0x827d3458, the camera within
     the near/far distance of the skater's body). It falls at the down rate and rises at the
     up rate, each times `kUpdateTime`.
   - The result is clamped and capped by the target. Below 1 the skater draws ghosted.

## Data

`camera_grinddetector` default:

| Field | Value |
| --- | --- |
| SkaterOffset | −1.5 |
| SkateboardOffset | 0.1 |
| Opacity | 0.4 |
| MinLookaheadDistance | 2 |
| MaxLookaheadDistance | 20 |
| LookaheadTime | 3 |
| FrustumAngleHorizontal | 15 |
| FrustumAngleVertical | 10 |
| MaxRelativeAngle | 30 |
| MaxRelativeHeight | 1.1 |
| GrindFadeInSpeed | 1 |
| GrindFadeOutSpeed | 1.5 |
| FrustumPitchOffset | 0 |

`rendering` skaters (proximity):

| Field | Value |
| --- | --- |
| Near opacity | 0.15 |
| Near | 0.5 |
| Far opacity | 1 |
| Far | 1 |
| Half length | 0.61 |
| Down rate | 5 |
| Up rate | 15 |

### What this means in play

- **Trigger.** The fade starts only while the skater heads at the end of a rail:
  - it lies 2 m out to 3 s of travel ahead (at most 20 m), measured from 1.5 m behind the board;
  - it is within 7.5° either side of the travel and no more than 1.1 m above;
  - the rail runs within 30° of the travel;
  - the hips cover the camera's view of that end.
- **Fade in.** The skater goes to 40% over about a second.
- **Fade out.** The skater comes back at 1.5 a second once the end is no longer covered. That
  includes the start of a grind, because the detector forgets rails while grinding.

## Port

Every step matches the original:
- the option, the gate and camera index, the inputs, and the frustum and box;
- the candidate filter, the obscured test, the anticipation, the opacity, and the actor's and the
  restore's use of it.

The fields come from the generated `camera_grinddetector::_LayoutStruct` and
`camera_droppredictor::_LayoutStruct`. `ReadAttribs` (0x825835a8) writes them through the
attribute manager, whose parameters begin 0x10 in.

A live run with the option on rides past the lighthouse rail, spawned at
`ol_fa_lighthouse_simulgrind` with `perf-skate` and a profile with the option set. The skater
is opaque beside the rail. Heading at the curved rail ahead, he turns see-through: the rail
shows through his legs.

## The difference fixed

The in-between restore (`PresentationRestorer`) ran `DoSimRestore`'s approach once per drawn
frame, at `kUpdateTime` each. The original presents, and so restores, at most 60 times a second.
At 165 frames a second the proximity fade therefore ran 2.75 times as fast.

`render_opacity::try_restore` now takes the time the restore stands for:
- the tick path, online skaters' `advance` and the replay view pass `kUpdateTime`;
- the in-between and replay-editor restores pass their frame's time.

The tick path is unchanged bit for bit. The grind fade's timing comes from the camera's target,
which the approach always outpaces (up 15 a second against at most 1.4: 0.6 × π/2 × 1.5), so it
was unaffected.
`a_fade_drawn_at_any_rate_keeps_the_original_speed` covers the approach.
