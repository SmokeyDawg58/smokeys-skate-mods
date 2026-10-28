# Off-board feet height (October 8)

Report: off board, the skater seems to stand slightly above the ground, as if the
feet were not quite touching it.

## Measured

Method: CPU-skinned shoe vertices with the shader decode
(`pos*2 + (0, 0.8, 0)`, fourth weight 1 − x − y − z). These were temporary
probes and have been removed.

| | Root | Lowest shoe vertex | vs collision triangles |
|---|---|---|---|
| Standing (IK state 2) | on the feet surface (height above ground 0) | +0.9 to +1.8 cm above that surface | about 4 cm above |
| Walking (IK state 3) | — | median −3.0 cm vs the feet surface | at the triangles |

- The feet surface sits 3.0 cm above the collision triangles: city street at
  (1300, 91.33, −468), two feet, every frame.
- While walking, the planted targets are the triangles + 1.5 cm.

## Native chain (the port matches each step)

1. **Line test start.** `PhysicalPlayerHiLOD::StartSkeletonLineTests`
   0x82dfe850 starts each foot test at the toe base + 0.33 and moves it
   1.83 m down. The sphere radius is `kFeetLineTestRadius`.
   - Its value is 0.03, and no data file overrides it; the name appears only
     in the map and pdb files.
2. **Sphere centre, not contact point.**
   - `LineTest<TrajectoryLineTestFunctor>` 0x827c01a0 calls
     `TriangleLineSegIntersect` 0x82adf658 with line fatness = radius and
     triangle fatness 0.
   - `FatTriangleLineSegIntersect` 0x82adeb48 stores lineStart + lineDelta·t.
     That is the sphere centre.
   - The post-adjust subtracts only triangle fatness × normal, which is 0 here.
   - `TrajectoryWalkFunctor` 0x827c1c78 copies that point into the result.
3. **Copy.** `EndSkeletonLineTests` 0x82dfeb70 copies the position unchanged.
4. **Root height.**
   - `OffBoardFeetIK::CalculateHeightAboveGround` 0x82dd0788 computes the
     Skeleton+0x2D80 root y − min(surface y + slope adjust).
   - The off-board root update moves the root by the clamped proportional
     height, so the root settles on the sphere centre.
5. **Foot IK.** `CalculateNewYPos` 0x82dc8b38 can only raise a foot.
6. **What is drawn.**
   - `Skeleton::FillPhysOut` 0x82b58230 copies Skeleton+0x2D80 into PhysOut
     (0x82b58580).
   - The presentation draws the root × the saved Hips.
7. **Walking.** `StepSelector::GetStepPos` 0x82d81278 projects onto the
   support triangle and adds `kShoeOffset` (0.015).

## Conclusion

- **Standing.** The height is about 3 cm from the line-test radius plus about
  1 cm in the standing pose. That is native behaviour, so the port keeps it.
- **Walking.** The shoes touch the collision ground.
- **Not checked against native:** the standing pose's own height above its
  root. Walking places the root by matching the centre of mass, which hides a
  constant pose offset, so only standing would show one.
