# Update5 Skeleton output entry

Status: unregistered draft, source-reviewed by implementation author; independent
review and Cargo/original-data gates pending. No full Update5 or conditioned state
publication is claimed.

`Sk8::Physics::PhysicalPlayerHiLOD::Update5_ProcessOutput` calls Board
`FillPhysOut` at 0x82e04ef8 (target 0x82b2f3e8), then the actual Skeleton virtual
getter at 0x82e04f10 and Skeleton `FillPhysOut` at 0x82e04f18 (target
0x82b58230). This draft is the contiguous Skeleton entry through 0x82b58614.
It stops before the raw Skeleton+B70 translation load at 0x82b58618 and
`GetPartTransform(19)` at 0x82b5861c, target 0x82b45e18.

## Actual owners and publications

Fresh PDB layout and raw loads establish these identities:

| Native field | Retained source | Output |
| --- | --- | --- |
| Skeleton+2ca4, input+2880 | constructor bindings.animation_input, actual AnimOutPhysIn flags.board_flipped | two independent reads, not a Processed flag |
| Skeleton+1970 | SkeletonState+1940, AnimationRecord+10, PoseBuffer[0].Z+20; current_animation.record().poses()[0] | yaw Skeleton+218, pitch+21c |
| Skeleton+2d80..2db0 | normal.root_transforms().animation_to_world() | Skeleton root matrix+170..1a0 |
| Skeleton+3d20 | normal.post_physics_records().centre_of_mass_velocity() | SystemReckoning+10 |
| Skeleton+3d00 | normal.post_physics_records().centre_of_mass() | SystemReckoning+30 |

The PDB nested type `Sk8::Physics::SkeletonState::AnimRecord` contains
mPoseBuffer at offset zero. Its first row is the animation record, not a fresh
physical Part query or a host canonical affine matrix. Root vectors preserve
unknown W; XYZ-only consumers never turn scratch W into published data.

The retained outer COM owner is constructed by Skeleton stores
0x82b5732c..0x82b5739c and updated by `UpdateRecords`0x82b54118. The older selected
`physical_player_com_session` adapter instead passes the PhysicsHistory record.
Its callers are collision corpus at lines38/425 and set-velocity corpus at51.
The next shared batch should change only its source type/getter to the actual
post_physics_records owner while retaining its explicitly selected history/frame
validation contract. The existing `try_publish_skeleton_com` is the single
publication implementation and is reused here unchanged.

## Ordered numeric and failure behavior

Capture input at58270, capture animation Z at58274, then dereference flip58284.
Null input capture is retained until that dereference. Flip XORs the sign of
X and Z. Reciprocal estimate is unconditional; raw VNMSUB words
582f0=15b8c971 and58370=15af8971 decode VA56/VA47, respectively. The local
residual is `-seed.mul_add(denominator,-1)`, followed by
`seed.mul_add(residual,seed)`. Ratio uses fused multiply/add with positive zero.
Reuse the existing source polynomial `try_atan`, target XMVectorATan82581658.
The signed PI (82261edc,40490fdb) adjustment occurs only for ordered negative
denominator; either signed denominator zero selects signed HALF_PI
(82261ee4,3fc90fdb). Unordered comparisons do not select either adjustment.
Output latch583b8 precedes the final selects; assertion precedes yaw store583f4.

Fresh input pointer583f8 and fresh flip583fc affect pitch using the *retained*
earlier animation Z. Latch58438 precedes dot3 with native J vector; assertion
precedes pitch58474. Root rows are then validated separately in order.
Raw5847c=14431af5 is a self-copy (VA35),58480=18411f55 inserts rotated X into W,
and58484=18221065 compares the resulting vector with itself (VA34). The same
pattern repeats for each row. Thus XYZ NaN fails, but unknown or NaN W is not
an extra source assertion. After all four validations, latch58554/assert precede
four fresh root loads and stores58580..585a8. A late host read error preserves
already published columns and angles.

COM reuses the existing leaf: sample latch585ac, early velocity585b0, assert,
late COM585d8 and its store, second latch/assert, retained early velocity store
58614. Neither a ContactSpy frame nor a synthetic completed-history requirement
exists in this native contiguous region.

Host reciprocal estimates, reduction rounding, denormals and NaN payloads remain
explicit numeric boundaries. The local exact negative-fused residual does not
change shared numerical helpers or claim whole-Xenon bit parity.

## Files and checks

New unregistered files: physical_player_phys_out_skeleton_fill_prefix.rs under
PhysOut construct; physical_player_skeleton_fill_prefix.rs under skater_pool;
the focused leaf tests child. Pool API:
`try_fill_paired_skeleton_output_prefix(backend, board, skeleton, expected_owner, numeric)`.
Actual pair validation and Arc identity precede access to the SAME constructor
and Normal owner. The parent caller retains its before-next-Part continuation.
No detached payload, alternate owner, early state category or duplicate COM
storage is introduced.

Six drafted tests discriminate two flip reads, complete read order, unknown root
W preservation, locked early-load prefix, late per-column copy failure, XYZ NaN
assertion, late COM failure and exact residual signed zero. Only rustfmt has run;
these drafts have not been registered or compiled.

## Remaining ordered callbacks

Skeleton remaining calls include Part transforms5861c..5894c, effective-root
queries58d0c/58d6c, Part594d8, SkeletonIK.Fill594e4, angle helpers59500/59564,
Part595f8/5960c, NormalizeSafe59638, planar angle59664, collision audio Fill59714,
live attribute lookup59780/default59790, and graph call597e0, ending59864.

After the complete Skeleton callback, Update5 continues without permission to
skip any selected callback:

| Call site | Target/owner |
| --- | --- |
| 82e04f24 | Pumping.Fill82dd4348, HiLOD+2c0 |
| 82e04f30 | GrindManager.Fill82dd3970, HiLOD+2f0 (parked dependency; not a no-op) |
| 82e04f3c | TrajectorySelector.Fill82db64d8, HiLOD+2c4 |
| 82e04f48 | Reckoning.Fill82dd4150, HiLOD+2bc |
| 82e04f54 | StateChanger.Fill82dd3bc8, HiLOD+300 |
| 82e04f60 | HandPlant.Fill82db8580, HiLOD+2cc |
| 82e04f6c | FootPlant.Fill82db6890, HiLOD+2d0 |
| 82e04f78 | SkateboardController.Fill82dc8370, HiLOD+2d8 |
| 82e04f84 | Wipeout.Fill82dd46b0, HiLOD+2f4 |

There is no separate Jump.Fill in this list. Later Update5 writes actual state
type at5434 (virtual GetType5400), samples wipeout/runout requests5438/5470,
then fresh type/category55f4/55fc and publishes category56b8. Remaining late work
includes conditional offboard transforms, trick-data assignment5c1c,
DoPhysicsSurfaceCheck61ec, ProcessIntentsForPhysOut61f4, AnimOut.Fill6200 and
AnimOut.Reset6208. None is implemented by this prefix. The already drafted first
three Physics conditioner children must await the real complete producer chain;
this prefix does not force Ground or publish a provisional category.

## Root registration and validation, September12

Root independently reviewed full prefix raw/control flow and both source/output
adapters. Registered the same-pair pool adapter and world continuation consuming
the actual BoardFill owner token, returning explicitly before Part19. Skeleton
agent independently reviewed root wiring and COM owner correction: PASS.
The earlier standalone COM diagnostic now reads post_physics_records3d00/3d20;
its existing history-frame check remains a host diagnostic admission only.
workspace38 PASS3,945units+6docs; original35 PASS17/17; Clippy56,
format30,diff30 PASS. The six prefix tests and real same-Actor continuation pass.
Full Skeleton.Fill has not returned at this checkpoint.
