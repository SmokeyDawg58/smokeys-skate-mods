# Skeleton constructor and Init closure, 2026-09-12

Confidence: **high** for the typed constructor values/order below, **exact**
for the cited PPC stores, constant reconstruction and PDB member offsets.
Native per-tick numerical parity is not established by construction tests.

## Fresh source proof

`Sk8::Physics::Skeleton::Skeleton`82b56d70 returns before the HiLOD
StateChanger constructor82e04204 and subsequent Skeleton.Init82e04228.
PDB Skeleton size16000, State at1940, Body1930, Drives1934, HeadTracking3df8,
IK3dfc, distinct SkateboardWobble3e00, ActorQuery3e70.

The constructor binds three OUTER instances before SkeletonState82b56e40:

| Stage | Class | Collection | Layout |
| --- | --- | --- | --- |
| plain animation Instance82b56de0 | babb3ec110849e08 | d7edbd362d7d2152 | plain Instance |
| physics_animation82b56e00, wrapper828b0f00 | 74bc9c93affe6aac | d7edbd362d7d2152 | cached or zero fallback220 |
| physics_skeleton82b56e20, wrapper828b10e0 | 9f862c2eaa016ce5 | d7edbd362d7d2152 | cached or zero fallback70 |

The decompiler miscombines 64-bit ABI arguments. Raw LIS/ORI/INSRDI at
56db8..dcc and wrappers0f14..2c/10f4..110c establish the actual keys above.
Earlier retained instances survive later checked-host layout failure.

Processed is stored at0 before those binds. Physical2ca0, NULL AnimOut2ca4,
timeDelta0 at2ca8 follow State construction. GetPhysicsIsland82b56e64 is a
SEPARATE live Entity-flags query from later Body island routing. The selected
native simulation path uses the actual enclosing physical manager; visual
island selection remains explicitly unsupported by this assembly path.

Matrices2cc0..3000,3c40,3c80 are I/J/K/zero P with ALL W lanes zero.
Vectors3cc0..3db0 are zero. Hex-Rays hides earlier stores for its reused
stack vectors: raw57128..94 initializes the first four buffers BEFORE loads
57214/218/21c/248 and stores572d4/dc/e4/5731c. Treating those four loads as
uninitialized would be incorrect. Quaternion3dc0, max-displacement3dd0 and
print-bone-cursor3e6c have no observed constructor store; existing max-displacement
and new quaternion/cursor projections preserve unknown values.

Heading3ded=true; offset counters/flags, wobble tilt/squish and prediction-valid
are zero; PhysicsAnimationState3df0=FOLLOW_ANIMATION_DATA0. HeadTracking B0
PhysicsSkater allocation573f0/ctor573fc precedes IK C80 Sim allocation57420/
ctor5743c, then distinct Wobble C0 PhysicsSkater allocation5745c/ctor57468.
Suffix57478..dc initializes jump offsets/deckZY/air-collision and landing
fields, assigns bouncy byte, Actor query and hat type. Big-endian halfword1
at3e40 is teleportReset=false, usePhysicsTruckTilt=true; halfword bouncy at
3e44 is teleporting=false, bouncyRagdoll=passed byte.

The early root/general/post/current/history projections move into the actual
normal assembly, rather than becoming separate continuing histories. Remaining
outer fields retain their named owner; unwritten fields stay Option::None.
Weak physical ownership resolves SAME Reckoning/GrindAir siblings; Processed
retains its actual Arc. Actor query is copied only at the ctor suffix.

## Ordered Init

Init82b62710 calls Data.Init62730, IK.Init6273c, then writes animation count
and trajectory index0. Seven outer GetBone calls run in this order:
Truck_Back62784, Truck_Front627b4, Left_WheelFront627e4,
Right_WheelFront62814, Left_WheelBack62844, Right_WheelBack62874, Head628a0.
Missing trucks/wheels retain signed-1. Head parent loads hierarchy.parents[head]
628b8, stores628bc, asserts count/-1, maps through Data.part_from_bone62904,
asserts valid Part, then stores that Part's parent at62934 (which may be invalid).
Safe-host errors preserve all preceding writes and do not validate the whole
hierarchy before the named lookup prefix.

Body allocation/ctor6296c/990 and Drives allocation/ctor629b0/9c8 precede the
24-row loop62a18..58. The loop initializes BOTH animation3040 and raw3640
buffers with I/J/K/zero P. This is initialized storage, not a completed
ProcessData frame. Set62a64 then InitializeMasses62a70 precedes TWO separate
root-Body.GetPosition/Processed-up/UpdatePhysicsRecord calls62a84/9c and
62ab0/c8. The same early State history receives Body's unscaled part masses,
then those masses normalize at InitializeMasses; no second continuing mass owner.

Raw Init suffix62ae0..f4 is **not twelve float blend stores**, despite its
decompilation. PDB IK+1d4 is on-deck blends, +1e4 external blends, +1f4 modes.
For each entry0..3: on_deck=0 (stfs), modes=0 (stw), external=0 (stfs).
TargetBlend32c and ExternalTargetNormalBlend33c survive. Then doIK1d0=false
62af8, true62b04; proxy3e43=false62b08. Folded callee62b14 targets825187c0
and is BLR. Body.volumes[0].flags clears bit1 at62b24, then disable-count[0]
at Body1e00 becomes0 at62b28. Full IK.Reset is not a substitute.

## Implementation and review boundary

Agent owns skeleton_constructor_runtime.rs, skeleton_init_state.rs/tests and
skeleton_animation_targets.rs. Root owns world/pool/session mounting and independent
review. Six focused tests discriminate name order, missing-wheel retention,
missing-head and invalid-parent partial writes, and all-W-zero initial buffers
without processed-frame publication, and transferred Instance retention against
a discriminating default/selected-collection fixture. The actual Skeleton+14
physics_animation Instance moves into pending normal-assembly storage, then into
the existing animation consumer on activation; activation no longer creates a
replacement Skeleton instance. Its separate global landing Instance remains
separate as required by source. Root performs Cargo and original corpus
validation to avoid shared-target Cargo overlap.

Root mounting independently reviewed: outer names after IK names and before
Body; buffers after completed Drives before Set; two history calls precede narrow
IK/proxy suffix; incoming already-initialized jump owner is moved into an
uninitialized destination, preserving the actual early constructor writes.
Completion publication must additionally validate the actual same owner/processed/
island/query bindings, all required transferred projections, completed Init and
constructor suffix. An assembly registration or children_complete alone is
insufficient to publish Actor's cached physical pointer.
