# Skeleton ProcessData prefix

Source symbol `?ProcessData@Skeleton@Physics@Sk8@@QAAXPAUAnimOutPhysIn@23@PAUProcessedPhysIn@23@PAUAnimOut@23@@Z`
is `82b5ecc8..82b5f348`, called by Update1 at `82e03718` after Toolkit and
old line-result publication. The earlier task address82b3ecc8 is inside
SkeletonCollision::InitSpecificBodyParts and is not this function.

Raw source SHA-256 fingerprints (half-open spans):

| Span | SHA-256 |
|---|---|
| 82b5ecc8..82b5efe0 | `3afd8a401e3e8ec386f9a423646f2709afd61cf9b283d8418e3a9ea70aa734e4` |
| 82b5ed48..82b5efd0 | `31dcf6f7c6cd67d0876a2e7131d852cbd30f90b618e50223adbaffa1c4772b18` |
| 82b244b8..82b244d4 | `c3d671de7f894bc16074419db47bfc99b794dd017c288a2fbf701e550635a517` |

The source stores the same passed input pointer at Skeleton+2ca4 (ece8), then
input dt+288c into Skeleton+2ca8(ecf0). IPhysical.GetSkateboard(vtable40),
GetBody(vtable4), and SkateboardBody.GetVelocity82b244b8 produce the full native
Part6 velocity vector, stored at Skeleton+3cf0(ed28). Existing body admission
and Part observation APIs provide actual XYZ, while W is an intrusive link,
not a fabricated homogeneous zero. Retained velocity therefore uses
SkeletonRootVector and preserves unknown W. Assembly.Activate82ae09b8 runs at
ed34 on the actual Skeleton assembly before ProcessAnimAttributes82b59de8 at
ed44. Its existing ascending Part activation/cooldown implementation is reused.

ProcessAnimAttributes remains an indispensable ordered collaborator. It reads
the actual input's attribute array, mutates the same processed words and
Skeleton adjustment state, and writes the separate retained PhysicalAnimOut.
The post-loop global jump/input suffix executes even for an empty array.
The graph agent owns completing that producer; no empty/no-op adapter is valid.

## Correction to the initial read-only report

The author's initial contact-helper defect report was incorrect. Independent
integrator raw review and subsequent author raw verification show the native
record layout is payload+0 (24 bytes), event name+20 (20 bytes), flags34/type35.
a420..a43c load the event name from +20,+24,+28,+2c,+30; a464..a484 load payload
bone-name words from +0,+4,+8,+c,+10, with transition float+14. GetBone at a4b0
receives that payload. Existing `apply_raw_attribute` correctly reads raw[..20].
No helper or fixture modification is warranted. The incorrect report assumed
a name-first record layout from decompiler prose instead of checking offsets.

## Trajectory graph and storage

The constructor's outer current trajectory+2e00 and inverse+2e40 remain distinct
from GeneralUpdate's sole retained NextTrajectory+2e80. ed60..ed98 copies Next
to current before reading input.poseBuffer[trajectory_node]. Init writes that
node index0. edb4..ee44 computes `compose(pose, current)` using X multiply,
Y FMA, Z FMA; translation begins with X FMA into pose translation. ee48..ee60
writes the resulting current matrix back into the same NextTrajectory.

Raw VMX128 decoding uses the previously independently established encoding:
VD=low|(bits3:2<<5), VA=low|(bit5<<5)|(bit10<<6), VB=low|(bits1:0<<5).
For example edd4=151600b0 is v8=v54*v0; edd8=14f500b0 is v7=v53*v0;
ede8=145300b0 is v2=v51*v0. IDA's displayed VA86/85/83 are wrong. These operands
confirm the existing skateboard_spawn_compose helper's order.

ee64 reads processed dt+93c, independently of the earlier input dt. ee90 obtains
the reciprocal seed, then ee9c=11e05caf and eea4=11a05c6f use native VNMSUB:
`residual = -(seed.mul_add(dt, -1))`, followed by
`seed = seed.mul_add(residual, seed)`, twice. The final reciprocal multiplies
current translation. AnimToWorld+2d80 rotates this vector with X multiply,
Y FMA, Z FMA into velocity+3d90 at eec8. Known XYZ remain publishable even if
the retained physical root's basis W lanes are unknown; the destination uses
SkeletonRootVector and marks W unknown unless all three source basis Ws are
known. No scratch zero is published as a native W value.
No zero-dt guard or normalization is introduced.

Only afterward, eecc..eed8 samples kZeroTrajectoryTransform. True writes
I/J/K/P0 into both current and Next, preserving already published velocity.
ef44..ef50 independently samples kSkeletonDisplayTrajectoryNode. Root review
identified that the enabled call at ef64 resolves to shared825187c0. Author
raw verification confirms its sole instruction4e800020 is BLR. The live cvar
read remains, but both branches continue with no presentation effect. The
earlier draft's missing-renderer error was unjustified and was removed.
ef84..efcc rigid-inverts current: transposed XYZ with basis W+0, arithmetic
`+0-P`, then Z multiply/Y FMA/X FMA. Existing rigid inverse helper matches this
corrected raw graph. The next mapped-pose loop begins at efe0.

Both cvars are live boolean references in Physics_Skeleton, default false,
null callback. Display: ref8308e0f8/init82f7c698/table82fd455c,
label kSkeletonDisplayTrajectoryNode, title Draw TrajectoryNode.
Zero: ref8308eac4/init82f7c848/table82fd4570,
label kZeroTrajectoryTransform, title Zero Trajectory.

Confidence **high** for source ownership and arithmetic graph; host reciprocal
seed, exceptional values and FP environment remain explicit numeric boundaries.
Three focused draft tests cover nonidentity accumulation/world rotation, velocity
publication before zero override, missing pose preserving the initial copy,
the enabled display branch reaching its proven shared BLR, unknown basis W,
and missing processed dt after current/Next publication. Both prefix children
are registered. The world wrapper `try_process_player_skeleton_data_prefix`
accepts the actual Update1AfterTeleportCheck continuation, validates its pair,
and forwards its same owner/input/processed pointers. The concrete ordered
attribute adapter is mounted at the native ed44 call. Root-owned Cargo and
original-corpus validation are pending; no complete ProcessData/frame claim.

The root integrator independently reviewed the trajectory composition,
current/Next/velocity ordering, reciprocal refinements and inverse graph.
