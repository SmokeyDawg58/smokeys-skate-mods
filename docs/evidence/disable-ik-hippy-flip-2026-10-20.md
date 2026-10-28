# DisableIK and the hippy jump's flipped board (2026-10-20)

`Skeleton::DisableIK` 0x82b462f0 does more than clear the IK flag. For each of the four IK entries
(left foot, right foot, left hand, right hand) it stores 0 to the on-deck blend (`SkeletonIK+0x1d4`),
the mode (`+0x1f4`) and the external blend (`+0x1e4`), then 0 to `mDoIK` (`+0x1d0`).
`Skeleton::EnableIK` 0x82b462e0 stores only `mDoIK = 1`. `Skeleton::Init` inlines both, DisableIK
first (0x82b62ad0..0x82b62b08), and `SkeletonIK::UpdateWipeoutAdjust` ends with DisableIK's stores
(0x82b59db8..0x82b59ddc).

The stores decide when a foot lets go of the deck:

- `SkeletonIK::UpdateStatus` 0x82b3db20 moves a foot out of mode 1 (on deck) only once its on-deck
  blend is 0; the mode it takes then is `mDoIK`.
- `SkeletonIK::UpdateBlendValues` 0x82b51d38 drives a mode 1 foot's blend toward 1, or toward 0
  while `DisableFeetTargetIK` (ProcessedPhysIn+0x8d0 bit 28) is set. The motion graph creates that
  attribute only in four `Dismount` states: `HandPlantAir/FS`, `HandPlantAir/BS`,
  `InAir/GrabsTweaks/FSGrab` and `InAir/GrabsTweaks/BSGrab`.

Without DisableIK's stores, a foot that was on the deck stays on it. `Skeleton::GeneralUpdate`
0x82b5ba88 runs `UpdateIKData` and `SkeletonIK::Update` every frame outside teleports, and
`Skeleton::ProcessData` takes the foot targets from the animated pose
(`SkeletonIK::CalculateTargetTransforms` 0x82b4a140: `LeftToeBase_Reparented` and
`RightToeBase_Reparented`, children of `skateboard_root`).

DisableIK's callers:

- `PhysState_WipeoutGround::Enter`;
- `PhysState_LandingOnDeck::Enter`, and its `Update` while the animated board's Z axis has a
  Z component smaller than `kLOD_MinDotFotFeetIK` (0.97) in size;
- `OffBoardFeetIK::ConsiderReset` 0x82dcd110, from the Enter of `OffBoard`, `OffBoard_Locomotion`,
  `OffBoard_Jump`, `OffBoard_Pushing` and `OffBoard_Air`, unless the last state was `OffBoard`,
  `OffBoard_Pushing`, `OffBoard_Air` or `OffBoard_Locomotion`.

## The flipped board

A hippy jump whose turn passes 90° lands with the board backwards. `SkaterAnim::FlipBoardBackwards`
0x82cb75a8 toggles `mIsAnimBoardBackwards`, and `AddBindPose` then adds two poses to each new
channel (`OffBoard.abin`):

| Pose | Bones | Rotation |
|---|---|---|
| `board_backwards` | `skateboard_root` | (0, -1, 0, 0): 180° about Y, in the bone's frame |
| `board_backwards_ik` | the four `_Reparented` bones | (0, 1, 0, 0): 180° about Y, applied before their translation |

Together the two leave the toe and hand targets where they were. The landing channel blends in from
the unflipped one over 0.2 s, and each bone blends on its own: the board passes 90°, and each
reparented bone's translation moves in a straight line to its mirror image across the board's up
axis. In `t_hippyjump_high_n_0_cyc` the toe targets sit about 0.2 m off that axis (0.20, 1.06, 0.00
and 0.18, 1.13, 0.18 in the board's frame), so at the middle of the blend both pass close to it,
one above the other.

In the hippy clips (`t_hippyjump_*_into`, `_cyc`) the reparented toes and hands sit within 0.4 to
2 cm of the toes and hands themselves.

## The port

`try_disable_wipeout_ik` cleared only `do_ik`. Through a hippy jump the feet kept the on-deck mode
and the blend of 1 they had while riding, so they followed the toe targets. With the targets on the
toes nothing showed until the flip: its blend drew both feet toward the board's up axis. The legs
crossed under a turned body, and the drift sometimes ended in a `SkeletonSquashed` wipeout: in a
scripted left hippy 180 at the Waterfront spawn, the wipeout came one frame after the board passed
90°.

`SkeletonIkState::disable` now makes DisableIK's stores. `enable` stays a flag store,
`finish_skeleton_init` is `disable` then `enable`, and `finish_wipeout_adjust` is `disable`.

The scripted hippy 180s (`hippy-spin` controls demo at `F_WF_Waterfront`, stick held from the
release, 360 frames), before and after:

| Spin | Before | After |
|---|---|---|
| Left | LandingOnBoard at frame 316, Flailing at 323 (wipeout) | LandingOnBoard at 316, rolling at 344 |
| Right | LandingOnBoard at 318, rolling at 344; feet drawn together at 323 | the same frames, feet apart at 323 |
