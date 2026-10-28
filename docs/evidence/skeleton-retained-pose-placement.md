# Initial skeleton Part placement on the actual owner

2026-09-07. Confidence **high** for source control flow and existing ordinary
finite host numerics; **exact** for retained owner selection, field offsets and
Part traversal. Extends [mixed collision ownership](skeleton-collision-and-mixed-pool.md).
Final strict validation is recorded below.

## Source and scope

`Sk8::Physics::Skeleton::Set` at `82b4c940..82b4cc00` first places its 24 Parts.
This implemented prefix ends at `82b4ca2c`, after the ascending loop branch at
`82b4ca28`. Each iteration calls `GetVolumeMtxFromPhysicsBuffer` at `82b4c9bc`,
copies all 16 result lanes, and calls actual Part::SetTransform at `82b4c9fc`.
It does not precompute every transform before issuing the setters.

The helper `82b413a0..82b414d8` reads the supplied physics-pose matrix for that
Part and the same skeleton's bone-to-volume matrix. PDB offsets independently
confirm SkeletonData at Skeleton+40, physics pose at data+70 (Skeleton+b0),
bone-to-volume at data+c70 (Skeleton+cb0), and the DIFFERENT inverse mapping
at data+670. Placement must not use that inverse mapping here.

Raw VMX128 decoding confirms axes use X multiply, then Y FMA, then Z FMA.
Translation uses X FMA into the pose translation, then Y and Z FMAs. All four
outer lanes are computed; inner W coefficients are not used. This is the exact
operand graph of existing `skateboard_spawn_compose`, despite its historical
board-specific name. The helper then calls `rw::math::vpu::OrthoNormalize3x3`
at `82b414c0`, targeting `82ae33b8`, with translation copied unchanged.

The actual Part setter separately applies inverse-body-frame metadata when
present and its own body normalization, quaternion and rotated-inertia updates.
Both normalization stages remain. Neither is replaced with the board's distinct
reverse-axis Gram-Schmidt routine or elided because it appears redundant.

Full-function SHA256 evidence:

```text
GetVolumeMtxFromPhysicsBuffer 82b413a0..82b414d8
b9e7b0a55ea3c4ab4a6970b6039a03288bbae2a4be0db0f9d1a65572ce858f54
Skeleton::Set 82b4c940..82b4cc00
125e55e0238fe327a275e186cf5a804f79b3b35ae965d7e924524f9c48a3503b
OrthoNormalize3x3 82ae33b8..82ae3750
b314d637e212a7165b004661622f7da199d1301717f4f6629597f73181e41430
```

## Connected owner path

`SkeletonNormalAssembly::try_place_parts_from_retained_physics_pose` retains
the SAME SkeletonData Arc and interleaves each transform calculation with the
existing in-place session setter. It accepts no caller-created replacement
pose, body observations or fake arena. Body allocation occurrences, constraints
and live mutable inertia owners survive; motion, forces and RF are not reset.
The setter's raw Part transform cache retains the actual full-lane request.

Detached or foreign owners refuse before writes. A checked error at a later
Part retains completed earlier Part writes and leaves later Parts untouched.
This is explicit checked host failure behavior, not a native exception model.

`SimulationSkaterAssemblyPool::try_place_skeleton_parts` checks actual membership,
kind and backend, then invalidates ALL publication freshness before the first
possible setter. Its manager wrapper preserves exclusive access. The same
publication refresh and shared ground consumer read those changed actual bodies.
No body is removed/re-added and no second skeleton or physics pool is allocated.

This operation uses the retained constructor physics pose. The mounted corpus
still uses original OnBoard phys_tpose24 and the explicitly controlled original
named RAW rig_tpose36 from its compatible hierarchy. This is not a recovered
mutable animation-pose producer or proof of full native pose provenance.
Existing host reciprocal-square-root estimates, reduction rounding and special
floating-point cases remain documented numerical boundaries.

## Validation

Final gate32085 passed: fmt check, 2512 unit tests (2284 core, 187 formats,
38 app, 3 symbols), six doc tests, all-target workspace Clippy with warnings
denied, all seven original core corpus checks and git diff check. All code
changes preceded this gate; only documentation changed afterward. No app
ignored probe was rerun. Independent source, production and test review passes.

Four new tests cover all-Part source routing with/without hat inverse frame;
detached/foreign no-write rejection; an actual foreign body injected at Part2
only in a test, proving earlier writes remain and later Parts are untouched;
and same-frame all-member publication invalidation. Actual integration supplies
nonzero motion to verify it survives placement. Tests compare complete body
snapshots with only the pose/quaternion/derived-tensor fields changed, unchanged
joint/drive ownership and packed inertia/RF, raw Part cache and live getter.
These route through separately source-audited host kernels, not independent
Xenon floating-point trace oracles.

Original hats0/4/1 all place the SAME actual owner and refresh its publication
without changing body occurrences or inline Volume words. The original mixed
board/skater manager path also places its skeleton before the shared ground
frame. It still deliberately uses zero solver iterations and controlled board
terrain translation/settings; this proves retained-pose placement and shared
consumer integration, not live spawn, humanoid terrain contact or freeroam.
Prior four Rob's Park board trajectories and all existing corpus checks remain
unchanged. The complete next drive/hook packet, including original Vault words,
is in [next source evidence](skeleton-drives-next-source.md).

## Required continuation

This is not full Skeleton::Set. Outer Init constructs SkeletonDrives at
`82b629c8` before Set at `82b62a64`. Set's later suffix places two separate
skeleton-drive hook Parts (Hips23 then Root0), then updates the proxy target.
The board's single hook is not either of those hooks. That construction,
admission, drive lifecycle and placement suffix must be connected next.
Actual Island/Body/State registration, live input/animation, complete collision
pair scheduling, camera and rendering remain required for freeroam.
