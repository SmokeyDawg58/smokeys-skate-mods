# First camera frame: retained producers and dependencies

Read-only source and original-data investigation, 2026-09-12. Confidence is
high for the owner/caller map below; selected startup execution and camera
numeric parity have not been validated. No camera runtime implementation or
Cargo validation is implied by this packet.

## Original Default graph

Decoded the externally retained original payload
`stategraph_payloads/00041_a11_h00614.stategraph`, whose manifest identifies
`data/script/camera/Default_cameragraph.stategraph`. The existing formats wire
reader describes its NUL strings, big-endian counts/scalars, byte booleans and
recursive children. Independent Python decoding consumed exactly 21,341
bytes. Existing original corpus oracle records 357 nodes and 687 attributes.

The grounded forward low-camera candidate path is
`Camera/CameraLow/OnBoard/Ground/Forwards`, with
`CameraChooseShot(shot1="bl_chase")`. The low-camera offboard fallback is
`Camera/CameraLow/OffBoard/Default`, choosing `offboard_ground`. These are
authored candidate paths, not a claim that an unexecuted initial PhysOut has
selected them. `IsOffboard` at `82e345d0` accepts four offboard-related physics
states or the actual animation transitioning-on/offboard flag.

Priority conditions before grounded Forwards include wipeout, running-out,
offboard air/hippy jump/offboard, skitching, moving object, NPC threat,
grind-exit and air, hand/foot plants and boneless, race regions, grinding,
road, named volume, manual, jump preparation and turning. Their real inputs
must select the path; supplying false to bypass unreconstructed owners is
not supported.

Three easy-to-miss world dependencies are reached before the forward shot:

- `SkaterIsUnderThreat82e34788`: sample live `kbNPCThreatTest`, otherwise
  obtain actual `LW::Man::GetChaseMan()` and call `ThreatIsOn()`.
- `IsInWorldPainterRegion82e37010` calls
  `GlobalConditionData::SetWorldPainterRegionBlendValue82e36678` for layer
  `Cameras_Race`, type `svm_race`. An actually null owner returns false, but
  CameraManager initialization publishes the session's actual LayerMan. It
  reads `GetRegionData` (`GetTOCEntry`) at the hips (`mSkeletonHipsPos` X/Z):
  the painted region's palette `m_pRegionData` holds a type (+8) and a blend
  value (+12), and a type starting with the condition's (`strncmp` over its
  length) sets `mPaintedRegionBlendValue` and passes. Only the five `Race_*`
  regions (all `svm_race`; blends -1, 1, 0, 0.5, -0.5) carry that data, and
  the 33 shipped camera layer tiles (the mountain: x 128..1024, z -1920..128)
  paint nothing else (`world_painter_palette::CAMERA_REGION_DATA`).
- `IsInVolume82e38e48` calls `IsSubjectInVolume82e38460` for `MyVolumeName`.
  It unconditionally obtains the camera trigger group, queries an AABB
  centered on subject skeleton-root position with half extents 0.2, then
  checks returned volume names using source case-insensitive prefix matching.
  No shipped trigger joins the camera group (type 2): the 10 triggers of the
  BAM sim collections and the 2449 of the 557 mission hulls are all type 0.

The minimal chase and trigger services must retain real session construction,
state and loaded geometry. This does not authorize broader living-world or
DMO expansion. Neither a fabricated empty owner nor unconditional query miss
closes these dependencies.

## Actual owner and scheduling chain

`CommonBackEnd825623c8` selects camera set `Default`.
`CameraManager825849c8` constructs CameraAttribManager and GameCameraManager
(`82e37c08`). Camera manager Init `82584c60` registers its local-skater
observer and publishes actual graph cache, island, predictor, trigger,
WorldPainter, RAIN and activity collaborators.

Local-skater notification `82581c18 -> 82e3a758` calls
`CreateCameraManInstances82e3a478` before `SetLocalSkater82e34990`.
The former constructs the camera collision server, shot database, drop
predictor and CameraMan, then loads the selected camera graph through
`CameraGraph82e32130`. The latter retains the actual Actor, its PhysOut and
updates collision-server Actor identity before teleporting each camera.

`SimUpdate2` calls camera manager Update at `8289fc88`, then
WaitForUpdateToFinish at `8289fcb0`. CameraManager Update `82584448` refreshes
changed attributes, samples real backend camera settings, updates game
camera, and publishes its ready frame into BasicCamera. The next Actor
Phase1 reads the retained camera Actor and opacity.

## First update producer chain

1. `GameCameraManager::UpdateSubject82e38e80` obtains actual PhysOut, updates
   subject transform and anchor points, publishes board/skeleton/ground/air/
   animation/system fields, then updates reference points and compass signals.
2. `CameraMan::PreUpdate82e453f8` updates air elapsed/teleport state, turning,
   centered turning, mirrored stance, live-enabled grind detection, trajectory
   data and incline angle before graph evaluation.
3. `UpdateCameraManInstances82e36270` reinstantiates actual Actor collision
   parts, performs PreUpdate, then runs the graph controller using source
   fixed timestep 0.016666668, followed by CameraMan Update with caller dt.
4. `CameraChooseShot::Begin82e3b008` samples the live lock-shot cvar, clears
   and adds actual choices, then calls `MakeShotChoice82e46ee0`.
5. `SetShot82e46ca8` preserves prior interpolated shot, loads the actual new
   shot, sets transition timing and initializes recursive blend shots when
   selected by data. It already reads actual rig anchor/logical-camera
   position to calculate shot distance; shot selection is not independent
   of rig construction.
6. `ShotManager2::Update82e472c8` refreshes changed database content, updates
   normal or blended shot, then interpolates. `UpdateRig82e47e68` performs
   actual `Rig::DoUpdate` at `82e482e0` before storing opacity and completing
   the camera frame.

## Opacity source, not a constant substitute

`GetCameraSkaterOpacity82e34960` returns zero for the actually empty camera
list; raw `82e34988` otherwise reads first CameraMan `+0x36c`.
CameraMan Reset `82e3b380` initializes this value to 1.0. Continuous update
replaces it from the selected/interpolated Shot `+0xa0`.

Raw `82e48314..82e48330` confirms, with live grind detection enabled:

```text
g = GameCameraParameters.mGrindOpacity (+0x2bc)
anticipated = (1-g).mul_add(1-actual_grind_anticipation, g)
opacity = FSEL(shot_opacity-anticipated, anticipated, shot_opacity)
```

FSEL preserves source unordered behavior. When the live cvar is disabled,
the source copies shot opacity directly. Reset opacity is not a valid
substitute for these ongoing producers.

## Implementation scope and remaining original-data boundary

There is no existing live camera owner or renderer camera consumer in core/
app. `render.rs` queues mesh submissions; existing camera graph corpus code
only parses documents. Proposed bounded modules are `camera_session.rs`,
`camera_subject_runtime.rs`, `camera_graph_runtime.rs` and
`camera_shot_runtime.rs`, with actual collision/rig/compass/grind producers
required before successful frame publication. Reuse existing graph, Vault,
PhysOut and world query infrastructure; implement reached callback families
only, and retain explicit missing collaborators.

The data lookup to resolve was actual `camera_shots/bl_chase` and recursive
references. `AttribShotDatabase::WriteToShot82d617a0` lowercases the name,
uses the runtime Vault collection, reads shot type at layout `+0x6c`, opacity
at `+0xb8`, and blend references at `+0x30`, `+0x18`, `+0x00`. Referenced
collections expose shot names at layout `+0x48`. The `bl_` prefix alone is
not evidence of shot type or selected child families. Missing collection
leaves the existing shot unchanged in native WriteToShot; it must not be
replaced with invented camera values.

## Original Vault lookup completed

Used the already-built, read-only `target/debug/examples/inspect_vault.exe`
against extracted external `skater.vlt` with `camera_shots` and
each requested collection name. No Cargo or new diagnostic binary was
needed. Its existing `AttributeVaultMetadata::resolve_pointers` resolved
cached layouts into dependency 1 (`skater.bin`). Python read those exact
layout ranges and followed class/collection RefSpec IDs. Reference names
were matched using existing `tools/match_attribute_hashes.py::string_hash64`
against original BIN strings, then each named collection was independently
looked up by the existing inspector. This did not treat raw pointer words
as host addresses.

Provenance check caught that the preexisting `unpcaked#` files did not match
the mounted corpus signatures. Those files were not accepted as the final
oracle. A read-only Python transcription of existing `big.rs` named-entry
and `refpack.rs` decoding extracted the two entries directly from original
`data/db/db.big` into external `world_probe_temp/camera_original_vault`.
These bytes match both mounted corpus signatures: VLT 851,568 bytes,
FNV64 `f7db0ed527db57b4`; BIN 812,520 bytes, FNV64 `6b4fc086777fe4df`.
The existing inspector was rerun recursively on these verified bytes for
all 26 collections below. Every listed type, layout offset, blend point,
opacity and reference was reproduced. No original file was changed.
Raw generated constructor `82d61724..82d61740` confirms camera_shots class
`f27dd93e059ef6cb`; default layout size is `0xe8` at `82d61760`.

`bl_chase` is collection `2b9cb09571af0e5b`, parent
`6f1af62984b8e677`, with zero explicit nodes and cached BIN layout offset
425784. Its type is **1 (blended)**, blend type 6, transition 1.5 seconds,
opacity bits `3f800000`, and ordered blend points -40, 0, 70. Actual ordered
references are:

| Shot | BIN layout | Blend type | Ordered children |
| --- | ---: | ---: | --- |
| bl_chase | 425784 | 6 | bl_chase_down, bl_chase_flat, bl_chase_up |
| bl_chase_down | 431584 | 1 | bl_chase_down_slow, bl_chase_down_fast |
| bl_chase_flat | 432744 | 1 | bl_chase_flat_slow, bl_chase_flat_fast |
| bl_chase_up | 441792 | 1 | bl_chase_up_slow, bl_chase_up_fast |
| bl_chase_down_slow | 452000 | 13 | chase_down_slow_right, chase_down_slow, chase_down_slow_left |
| bl_chase_down_fast | 452928 | 13 | chase_down_fast_right, chase_down_fast_left |
| bl_chase_flat_slow | 447824 | 13 | chase_flat_slow_right, chase_flat_slow, chase_flat_slow_left |
| bl_chase_flat_fast | 448520 | 13 | chase_flat_fast_right, chase_flat_fast, chase_flat_fast_left |
| bl_chase_up_slow | 450144 | 13 | chase_up_slow_right, chase_up_slow, chase_up_slow_left |
| bl_chase_up_fast | 447592 | 13 | chase_up_fast_right, chase_up_fast_left |

All 16 referenced leaves were independently found as type 0, with null
blend references and opacity bits `3f800000`. This proves a family of ten
blended nodes and sixteen normal leaves; it does not prove which leaves a
particular initial PhysOut selects. All blended nodes have transition 1.5
seconds; normal leaves have 0.5 seconds. These are observed corpus values,
not runtime constants.

Speed nodes have ordered points 10,20 and no third child. Input-lean nodes
usually use -0.8,0,0.8; down-fast and up-fast use two children at -0.8,+0.8.
The authored up-slow node is unusual: three children but points
-0.7,+0.7,70. It must not be normalized to match sibling nodes.

`GetBlendRawValue82e44898` maps reached type 1 to absolute actual
`Rig::GetSubjectSpeed`, type 6 to actual CameraMan incline converted to
degrees, and type 13 to `GetAnimationInputLean` for nonzero dt (zero for
zero dt). These are source-decompiled producer identifications; full VMX
operand/numeric audit remains before implementation. `UpdateBlendValue`
at `82e45b00` retains smoothed/clamped per-shot blend values;
`UpdateBlendedShot82e45fd0` recursively updates the selected children and
uses sine-shaped interpolation. `SetupBlendShotsRecursive82e459e8` constructs
the referenced family when SetShot selects a blended shot.

Consequently a normal first-frame executor cannot be a single-shot copy.
The bounded chase implementation must support the three reached blend-value
producers, source smoothing/selection, recursive normal-shot evaluation,
shot interpolation and the actual subject/rig inputs already listed.
Other camera-shot families remain outside this packet. Even though all
these original leaf opacity values are one, live Vault updates and grind
anticipation still prevent replacing the producer with fixed opacity.
