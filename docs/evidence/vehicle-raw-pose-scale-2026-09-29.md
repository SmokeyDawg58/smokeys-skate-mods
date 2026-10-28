# Vehicle RAW bind-pose scale rejection

Reported runtime failure: `UnsupportedRawPoseLayout` with masks
`[1, 0, 0, 0, 252, 0]`, selectors `[0, 1]`.

## Root cause

`VehicleRender::update` initializes `VehicleAnimation`, which resolves the
authored model pose in VehiclesDB and calls `AndalePose::sample_raw_sqt`.
The static `AndaleAnimationPart::sample_raw_pose_sqt` only supported rotations
and translations and rejected nonzero scale masks. The existing integer RAW
clip decoder already supported float3 scales. This was an incomplete static
pose decoder, not an invalid database or a gameplay intention failure.

## Source and original-data verification

Re-read IDA `Andale::Vector_ExtractPackedN` 0x82d6f838, including raw PPC:

- 82d6fc7c..98 loads the scale, rotation and translation mask pairs.
- 82d6fd20..60 selects the data using frame stride and frame index.
- Equal-frame path 82d70990 onward extracts channels in bone order;
  82d709ac..e0 selects/advances the three mask bits. The selected scale
  consumes12 bytes (82d709f0/82d70a04), before the rotation and translation
  payloads. Identity lanes and selector meanings agree with the existing
  clip decoder and `actor-face-playback.md` evidence.

Read only the archive header and VehiclesDB entry from Jan17 `data/big/anim.big`,
then decompressed that entry into temporary scratch outside the repository.
All31 pose parts have8 bones and selectors `[0,1]`. Layout counts:

| Masks | Pose parts |
| --- | ---: |
| `[1,0,0,0,252,0]` | 17 |
| `[0,0,0,0,252,0]` | 12 |
| `[1,0,12,0,252,0]` | 1 |
| `[1,0,220,0,252,0]` | 1 |

Every selected scale is on root bone0, XYZ approximately0.01 (f32).
Translations select bones2..7. Independent Python traversal consumed exactly
each declared payload/stride; all scale and translation lanes were finite.
The root scale must be preserved, not ignored to bypass validation.

## Implementation and limits

Moved the existing RAW clip channel extraction loop into a shared helper in
`andale_raw.rs`. Both static poses and integer clip frames now use that same
helper. Static pose validation includes scale bytes and retains selector and
payload rejection, with a mask/bone bound check for directly constructed parts.
Unselected channels still retain the existing SQT identity. No vehicle-specific
pose substitute or second animation evaluator was added.

Added an UNRUN synthetic regression for all four actual mask families, checking
root scale, wheel translation offsets, identity channels and truncated payload
rejection. Existing clip and full-skater bind-pose tests remain applicable.
Scoped rustfmt check passed. No Cargo/build/check/test/game commands run;
compilation and runtime validation remain pending. The read-only asset
audit verifies serialized layout, not compiled decoder execution or complete
vehicle animation parity.
