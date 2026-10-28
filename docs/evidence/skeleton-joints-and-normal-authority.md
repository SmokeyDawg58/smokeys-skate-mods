# Skeleton joint producer and normal-body mutation authority

Update: `skeleton-normal-limits-and-collision-data.md` records the subsequent
mutable joint-pool/normal-limit consumer and collision-data implementation.
The later source packet below is now partially implemented, not full normal
setup or humanoid admission.

2026-09-07. Confidence: **high** for recovered source control flow, ownership
requirements and ordinary host arithmetic; **exact** for quoted instruction/data
words and culling bits. No Xenon per-tick numeric trace or playable-skater claim.
Root authored the joint producer; separate authors supplied mutable pool
inertia and culling resources; another reviewer independently audited source/code.

## Implemented boundary

`skeleton_body_joints.rs` produces real frame/limit records from the retained
`SkeletonData`, remapped joint attributes and fresh reader calls. It preserves
the earlier allocation count separately from the actually initialized prefix.
`skeleton_joint_attributes.rs` reads the real retained Vault graph.
`skeleton_culling.rs` owns and shares the two original packed culling tables.
`simulation_rigid_body_pool.rs` now supports explicitly authorized shared mutable
inertia alongside existing immutable callers. These are prerequisites for the
actual normal SkeletonBody, not completed assembly admission or normal setup.

Original body geometry/mass construction is recorded in
`skeleton-body-shapes-and-mass.md`. Do not replace post-admission mutation with
an immutable precomputed final body: that changes the source initialization
sequence and shared-pointer visibility.

## Symbols and source identity

Root rechecked these raw-function SHA-256 identities against the live symbolized
`sk82_na_m.xex.i64`. End addresses are exclusive.

| Symbol | Range | SHA-256 |
|---|---|---|
| `?InitJoints@SkeletonBody@Physics@Sk8@@AAAXXZ` | `82b464f8..82b472e4` | `1efe01dfc1d8091e6f78daeae25296a31f49a3794a87a9bf68eefae621e4ba54` |
| `?SetParentLinearFrame@JointFrames@physics@rw@@QAAXABVMatrix33@vpu@math@3@@Z` | `82b232b8..82b2346c` | `7d25c69d6286fb83aa3cf9d4de5f85632f313db1166fefa23649d1232384d817` |
| `?CreateNormalCullingTable@SkeletonBody@Physics@Sk8@@AAAXXZ` | `82b5f348..82b5f7cc` | `952cf7182416057d4854aa1dfc71c80d4fb1ee3a5bc1f05d38c1a65b99cdbb53` |
| `?CreateRagdollCullingTable@SkeletonBody@Physics@Sk8@@AAAXXZ` | `82b5f7d0..82b61734` | `c3d75b9454616a1067fc275e289fe6156365a5817f5108e8ab7c5660197c396e` |
| `?Initialize@BitTable@rw@@SAPAU12@ABVResource@2@II@Z` | `82bc3308..82bc336c` | `df46e75b896b6161ce14a1caa7c09e826d457155b625eeee8f2afadd37db5f4f` |

Ownership is `Sk8::Physics::SkeletonBody`, using `rw::physics::JointFrames`,
`JointLimits`, `AssemblyDefinition` and `rw::BitTable`. Body Init calls joints
at `82b620ac`, normal culling at `620b4`, ragdoll at `620bc`, installs normal
culling at `620d0`, then creates the assembly at `620d8`.

### Joint frames: authored orientation is not relative pose orientation

For child C and parent P, let Vc/Vp be authored orientation `+0x30` and position
`+0x40`, O be the child's authored parent orientation `+0`, J its joint
orientation `+0x10`, and Pc/Pp the actual supplied physics poses. Source builds:

- Relative pose = rigid inverse(Pp) * Pc.
- Child angular axes = transpose(Vc.rotation) * J.
- Parent angular axes = (transpose(Vp.rotation) * transpose(O)) * J.
- Child position = transform explicit zero point by rigid inverse(Vc).
- Parent position = transform explicit zero point by inverse(Vp) * relative pose.

The explicit zero products must not be optimized into simply returning a
translation: nonfinite axes still multiply zero. Quaternion expansion retains
the source's noncanonical fourth lanes. Reused rigid inverse is the audited
transpose/Z-product/Y-FMA/X-FMA path, not SkeletonData's general affine inverse.
Composition keeps X-product/Y-FMA/Z-FMA axes and X/Y/Z translation FMAs.
Parent angular quaternion, child angular quaternion, then parent linear identity
each use the actual candidate selection and two reciprocal-square-root
refinements. The linear identity is not assigned as a literal quaternion.

Root and independent reviewer read the raw graph, correcting IDA's misleading
extended VMX register aliases. Raw VMX128 VA decoding is
`((word >> 16) & 31) | (((word >> 5) & 1) << 5) | (((word >> 10) & 1) << 6)`.
Parent/child seed sites are `82b46cd4` and `82b46d50`; linear setter call is
`82b46ef0`. Source interleaves pure parent/child refinements; host helper calls
preserve results and intrinsic-call order, not native intermediate stores.

### Limits, live sampling and actual emission

Iterate all 24 Parts in order, skipping absent physics parents. Source native
parent bone ID -1 is not representable by validated SkeletonData. Retain this
input boundary; do not invent invalid negative bone IDs.

Each eligible Part performs all calculations and reads even if not emitted:

1. All three quaternion services above.
2. Full four-lane cached Skeleton layout `+0x30`, multiplied by a twice-refined
   reciprocal of update-time word `3c888889`. Prism position is zero.
3. Fresh LOCAL Joints byte `dff972080e90f82f`. Only if nonzero, inspect the
   current dense remapped row. Nonzero BallJoint selects default free limits.
4. For an enabled non-ball row: swing = bone.swing * row.swing_angle;
   twist = row.twist_angle * bone.twist. Both products precede swing cosine.
   Select original product when product-minus-epsilon >= 0, otherwise exact
   epsilon `3c23d70a`; unordered selects epsilon. This is NOT subtracting .01
   from the angle. Swing cosine precedes twist cosine. Both types become1.
5. Fresh local bytes `9cf0c7c389179e4c` and `265499faea27836a` can force swing4
   and twist2 without erasing already computed angles/cosines. Defaults are
   swing4/twist2, angles0, cosines1.
6. Fresh local Skeleton floats `71cc8102bc85277b` and `1b84fd9255412c0f`, each
   multiplied by exact `426fffff`, supply swing/twist velocity.
7. A separate fresh GLOBAL Animation byte `dff972080e90f82f` gates emission.

Only emission advances pair8/frame80/limit64/remapped-row20 storage. Allocation
was sampled earlier as22/0 and is never rewritten here. Parent frame selection
uses the stored nearest ancestor, but emitted parent index re-reads the
last-write bone-to-Part reverse map. Duplicate mapped bones therefore matter.
Host capacity/allocation/attribute/numeric errors publish no partial definition;
native unchecked writes/assertion behavior is not claimed as recoverable Rust.

Concrete Vault provider retains local Skeleton/Joints and global default
Animation instances. Cached112-byte layout missing means generated zero default;
generic byte/float missing means zero, with malformed scalar widths refused.
No hardcoded original tuning, editor reload, or mutable Vault notification claim.

## Real culling resources

The session inventory has two distinct original names, `cullTableNormal` and
`cullTableRagdoll`; find returns the actual existing Arc. Missing construction
initializes 24 rows,24 columns,18 zero words, sets all576 row-major bits, then
performs each source clear in forward/reverse order. Normal unordered pairs:
`(14,4),(14,8),(23,4),(23,8),(4,18),(8,22)`. Ragdoll uses the separately recovered
68-pair list already in `ragdoll.rs`. Diagonals stay set; directed clear counts
are12 and136. Both match the independent frozen18-word tables.

Native adds the allocated84-byte/alignment4 resource to inventory before filling
the bits. The exclusive host owner publishes only a completed immutable table;
reentrant partially filled resource observations and general inventory services
are outside scope. Vector reserve is fallible; Arc allocation uses Rust's
standard OOM behavior. Independent session inventories intentionally do not
fabricate process-global shared identity.

## Shared mutable inertia in the actual pool

Native Part `+0x4c` points to RigidBody; its `+0x5c` points to source inertia.
Pool Add `82ae6e00` caches mass/tensor data; SetUpNormal later writes the same
inertia object. A cloned immutable value cannot model subsequent reads.

`SimulationMutableInertia` holds private shared Copy-value storage. First
non-Static admission permanently binds it to one pool authority. Writes require
exclusive pool access, a live occurrence, and exact retained-owner identity.
No external mutable guard/callback escapes. Cross-pool, stale, free and wrong
owner writes are refused before effects. Permanent binding prevents another
pool from changing an already prepared update through an alias.

Mutable admission uses the existing reservation/reset/link/cache graph. Empty
capacity and Static skip payload reads. A source write changes storage ONLY:
it does not refresh cached inverse mass/tensors, RF rows, motion, tags or lists.
Actual Part transform refresh samples current storage at its call. Actual
DynamicUpdate preparation samples current drag/tensor values; its existing
body inverse-mass cache remains unchanged by the earlier owner write.

Immutable Arc APIs retain their signatures and identity. Borrowed immutable
`try_inertia` explicitly refuses mutable storage instead of returning fake None.
New `try_snapshot` reports lock failure; legacy `snapshot` documents a panic
on poison rather than silently substituting data. Production writes are private
Copy stores without external callbacks; poison tests deliberately inject failure.
Existing pool payload is XYZ tensor; native packed tensor W remains in the
SkeletonBody mass sidecar. Full normal-body mutable packed ownership is still
an integration requirement, not completed by this pool seam.

## Next dependency: source normal setup packet (not implemented)

Separate raw-source audit recovered the following; root rechecked all
function identities. Re-read relevant raw operations when implementing.

| Symbol (all `Sk8::Physics`) | Range | SHA-256 |
|---|---|---|
| `SkeletonBody::SetUpNormal` | `82b50508..82b50720` | `6877eab50f42157bdf44711a1d3fdc349ac726eed9cc8d448def066c71650557` |
| `SkeletonBody::SetNormalJointLimits` | `82b3bb90..82b3bd04` | `64e358ab6993f1b4b69e20964ad98c5b107008689f7238f318e39167b7d4adc3` |
| `SkeletonBody::SetUpNormalCollision` | `82b50400..82b50508` | `4dc1b4933aec1c7bf249e8dd2d0d288f5fbf8e1412732c9df579a9fa782165e0` |
| `SkeletonBody::SetUpNoCollision` | `82b47640..82b476ec` | `29a965fa5ebb9196db29a502ed6c06b730fcdbe164b3b5f1914121f8564c7da8` |
| `SkeletonCollision::Reset` | `82b3acc0..82b3ae5c` | `929bece7c094c7f8333ec8268d3bb601adcabb4e556f65f3f8dbf7309ace9fb9` |
| `SkeletonBody::TeleportReset` | `82b3bf88..82b3bffc` | `7d402450fbac36c254b3627f6c16d78f3723e54478077066305c8095c89b6ab8` |

Body Init creates assembly, assigns group1/all Part userdata1, inserts Active4,
publishes sync/Part spies and attached Joint spy1 (`m_spy/+0x1c`, NOT tag), writes zero material fields
and root material pointer, THEN calls SetUpNormal at `82b621ec`.

SetUpNormal ordering:

1. SetNormalJointLimits through `Body+1c -> AssemblyDefinition+c -> dense64`
   shared limit storage. No enable/ball/count gate. Loop24 skipping absent
   parents; swing product is row*bone and twist is bone*row (opposite operand
   order from InitJoints). Store swing angle, cosine swing; store swing cosine
   and twist angle, cosine twist; store twist cosine. Keep types/other fields.
2. Install normal culling, group6 and every actual Part userdata6.
3. SkeletonCollision::Reset.
4. SetAngularDrag(0,-1), then SetLinearDrag(0,-1), all Parts. Helpers
   `82de7f38..8090` SHA `acdb4602e11b58de08904a2706193cc642f78a733699b31067bbfc24f4fb7b5c`
   and `82de7de0..7f38` SHA `900ccf4b68c98689631e9e91d41c196e4b480bde7c8beada594432b51e67deac`
   retain grouped-four ascending pointer/store order and even zero*`426fffff`.
5. Literal Parts1..23: point material to Normal, write animated inverse mass
   and full4 tensor into live inertia, recompute spherical inertia using minXY
   then reciprocal minZ (source right operand on ties/NaN). Write collision
   compliance and priority from drive data. Root compliance becomes1.
6. Fresh LOCAL Skeleton byte `043f9227035de245`, default byte0 if missing,
   chooses NormalCollision or NoCollision(true). Original node5dd50 is01.
7. IsRagdoll=false. Folded AI call `825187c0` is exactly BLR, not missing work.

NormalCollision repeats group6/userdata6; root volume group4, clear enablebit1,
root disable counter0. Parts1..23 with CURRENT signed counter==0 become enabled
and group0; nonzero, including negative, preserves their flags/group. Counters
are not reset. Hat modifies outer aggregate, not independent cylinder child.
NoCollision sets root group4/disabled/counter0; clears enable on1..23 and resets
those counters only if resetTimers=true; it does not change other groups,
materials/compliance or VolumeDisableCountActive.

Collision Reset is selective, NOT blanket zeroing: hitVehicle/hitWorld/opposing
contacts;24 times/priorities/justCollided;5 BoneData contact/subcontact vectors,
surface/force/hit fields;8 contact channels' forces/friction/slide, normals,
contactBone=-1/audioSurfaceType0; totals/timer/drive/priority status and numNormals0.
MostVerticalNormal and toeNormal become J=[0,1,0,0]. Compliance24, NormalInfo20,
SpecificBodyParts, retained body/state pointers and padding remain untouched.

Constructor calls TeleportReset AFTER Body Init at `82b625cc`: ragdoll=false,
CollisionReset AGAIN, all24 disable counters0, VolumeDisableCountActive=false.
Do not claim first SetUpNormal sees initialized zero counters1..23: no earlier
store was found. Fresh volumes are already enabled/group0, making that branch
idempotent for constructor defaults; completed constructor counter zeros are
proven by TeleportReset. Its second reset also clears the priority values just
written by SetUpNormal, while preserving compliance.

Still required: mutable limits in the actual joint pool/Build consumer, shared
material/collision state, source-ordered24-Part admission and normal setup,
then the genuine separate two-Part Skeleton hook/drives and player/input path.
No fabricated second SkaterPool member, no early hands-on skating handoff.

## Validation

Focused tests cover joint read/callback order, skipped emission row reuse,
allocation mismatch, type overrides, exact angle floor, explicit zero products,
authored-vs-pose frames and refined linear identity. Eight actual pool tests
cover shared storage, refresh timing, DynamicUpdate use and authority/error
boundaries. Four culling tests cover frozen bits and real owner lifetimes.
The original-data body corpus is extended through these joint/culling producers.
Original db.big VLT851568/FNVf7db0ed527db57b4 and BIN812520/FNV6b4fc086777fe4df
are the same retained inputs as the preceding checkpoint. The pose remains the
controlled original SkaterBodyPres RAW fixture, not proven live caller selection.

Independent raw oracle: PtrN record `b2518` under target dependency0 resolves
VLT slot `5dcf8` to dependency1 BIN `9eed0`; cached+30 is
`[3c23d70a,3c23d70a,3c23d70a,00000000]`. Do not confuse another PtrN record
`ac418` with the same slot offset but different target dependency. Local Joints
nodes `5e220/5e1e0/5e1f0` contain enable1/freeSwing0/freeTwist0; global Animation
`5bdc0` contains enable1. Local Skeleton velocity nodes `5de90/5def0` both hold
`41200000`. All22 mapped BallJoint bytes in BIN `9fd68` are0. Independent raw
input calculations give host prism XYZ `3f199999`, W0, angular speeds `4415ffff`.
These are test oracles only, never runtime tuning. Reviewer independently read
the original archive again and recomputed these expected words, not producer
printouts. All22 explicit endpoints and emission ordinals match for hat0/4/1.

Final strict gate shell61348 EXIT0: `cargo fmt --all -- --check`,
`cargo test --workspace --quiet` (2228core+187formats+38app+3symbols=2456 unit
tests and6doctests), all-target warnings-denied Clippy, all SEVEN ignored core
original-data tests, and `git diff --check`. App ignored corpus not rerun.
21 new unit tests:9joints,8mutable inertia,4culling. Earlier gate75623 passed
all tests but reported four doc/literal Clippy issues; only those formatting
issues and one now-unused dead-code expectation were corrected, not arithmetic.

Existing board ground counts6/6/6/4 and all four centers remain unchanged;
separate8-frame DMO contact hash remains `0e7c704e6a10f08a`, final COM
`[588.8619,-54.40114,996.5048]`. No new humanoid simulation frames. Independent
review gives final PASS for production, all21 focused tests and corpus/evidence.
