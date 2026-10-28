# GeneralUpdate drives and force continuation

Read-only author recovery, 2026-09-08. Rust implementation was not changed by
this task. Raw bytes and PDB types were read through IDA; no database mutation
or Cargo command was performed. Independent review and connected original-data
validation remain required for any new continuation.

## Implementation progress after the source audit

The source packet, force kernel/eight tests, narrow Part/backend/body writes,
retained-row drive adapter and state-0x2be grounded continuation have now passed
separate review. Root added two whole-pool write/failure-prefix tests,
one missing/invalid/foreign Part test and one nonidentity trajectory-tail test.
Accepted **48832/143430 EXIT0**: 2,885 units (2,657 core, 187 formats,
38 app, three symbols), six docs, all nine original core corpus checks, fmt,
all-target Clippy with warnings denied and diff pass. Initial gate76961 passed
units but found doc/type-complexity lint and an unused test lint expectation;
those were fixed without changing source arithmetic. Original55765 also passed.

The extended original helper passed separate independent review. Second
unchanged placement with state0x2be checks 44 same admitted bone-drive payloads,
original Vault scales/tuning, independent endpoint positions and dynamics,
all 24 force additions with distinct processed gravity, and completed trajectory
identity. Mapped rows and current-animation readiness survive the drive call.
A third call uses different damping and fails at force Part3: all 44 drive
updates and the first three force writes remain, with exact nested error and
whole-body/history preservation checked. Existing quaternion tests provide
separate numeric coverage; this helper independently checks endpoint positions
and dynamics, not a new quaternion graph. It performs no additional solve.

The existing normal owner now calls drives on its actual current mapped rows
and sole post-records proxy-enabled field. Shared drive admission validation is
reused; the old controlled-pose method retains its own frame guards and remap.
The new path does not invalidate current animation or re-map its rows. It visits
all 24 actual Parts for force accumulation only after drives succeed. Each pool
operation captures cached body inverse mass and old force before its numeric
service, then writes force XYZ and clears cooldown; other fields remain intact.
The native 0x2be branch completes these stages, while other states explicitly
return the pending UpdateIKData continuation.

NextTrajectory is a distinct private retained matrix at Skeleton+2e80. PDB
distinguishes current trajectory+2e00 and inverse+2e40. Constructor base r27 is
formed at56f10; stores X57048, Y57058, P57068, Z57070 create I/J/K/P0 with W0.
The half-open span5703c..57074 hashes
`17353c58686841570a7257456361b247dc771e63b8f129267745a8350bf7382b`.
Source constants use addresses8227e8f0/e900/e910 after signed low-displacement
calculation, not8228 addresses. ResetSkeleton and TeleportReset preserve this
matrix. The grounded identity tail runs only after all force calls succeed.
The next native ProcessData copies Next into current trajectory ated60..ed98;
that producer/consumer chronology is still a separate missing live integration.

## Reference spans

| Symbol / span | Address | SHA-256 |
|---|---|---|
| `SkeletonDrives::Update(Matrix44Affine*,bool)` | `82b54b90..82b550f0` | `fc6ee235c01e21c392d2a1800f24f0a92b621a668dde2d43eca3c5495d5a6d2a` |
| `GeneralUpdate` drives call through return | `82b5bb70..82b5bc58` | `d0367642f5d0a7695fd6af360932e1848e4aee943e5b6e47475de3af73487ab4` |
| `SkeletonBody::AddDisplacement(Vector3)` | `82b59a98..82b59b04` | `0453612090c60e2167a20cfdcfb4defe9019119df61d31647f35ea60ac03a3cd` |
| `SkeletonBody::ApplyDisplacementToPart(PartIndex,Vector3)` | `82b545e0..82b546c0` | `217d48c728988dd700dc271a4a53139fce501d099b2f0cc603b69c36df338d71` |

Full existing drives kernel evidence, subordinate frame/dynamics setter graphs,
attribute keys and original-data validation are in
`skeleton-drives-next-source.md`. The raw Update span was reread here and agrees
with that packet. This task identifies the missing actual caller integration
and the remaining GeneralUpdate force suffix.

## Caller ownership and order

GeneralUpdate calls UpdateHookPositions at `82b5bb18`, possibly performs full
ResetSkeleton at `bb2c`, and only after successful reset may set sticky
WantRespawn. It reloads retained processed current state `+8e8` at `bb54`.
State `0x2be` skips BOTH UpdateIKData `bb64` and SkeletonIK::Update `bb6c`;
other states execute those calls in order. The separate IK author owns their
source recovery. All states then reach:

- `bb70`: r4 = Skeleton `+3040`, PDB `mAnimVolumesPoseBuffer[24]`.
- `bb74`: r5 = byte Skeleton `+3e43`, `mSkeletonProxyVolumeEnabled`.
- `bb78`: r3 = Skeleton `+1934`, `mSkeletonDrives`.
- `bb7c`: retained `SkeletonDrives::Update` call.

These are the SAME current mapped rows already changed by ProcessData's
adjustments and subsequently by IK. Neither original raw bone poses nor physics
history matrices can replace this buffer. The byte argument is not an external
gameplay mask and is not derived from ragdoll state or current volume flags.

The current native byte has one existing Rust projection:
`SkeletonNormalAssembly.post_physics_records.proxy_enabled`, private with a
read-only getter. Constructor stores false at `82b574a8` (r30 is zero);
Skeleton Init repeats false at `82b62b08` (r31 zero). Implemented Set/Init do
not call a true writer. Raw verified true writers are:

- `Skeleton::SetOffBoardCollision [82b4f038,82b4f298)`: writes byte1 at
  `82b4f238`, after its earlier per-Part collision changes.
- `Skeleton::CollideWithWorldOnlyOffboard [82b542c0,82b5436c)`: calls
  SetUpNoCollision(true) at `82b54314`, then writes byte1 at `82b5431c`.

Those complete collision-mode producers remain unconnected. The continuation
must read the existing field; no caller-supplied substitute or second bool is
needed. Full ResetSkeleton preserves this field.

## Existing drive implementation and missing bridge

`skeleton_drives_update.rs::SkeletonDrives::try_update` already implements:

1. Scratch inverses for targets1..23, leaving scratch0 unwritten. No normalize.
2. Lazy global animation_default local scale, root scale, normal-follow byte
   lookup/load pairs, with genuine missing Default4/4/1 handling.
3. Actual Hook0 linear dynamics stores before any offboard cvar read:
   damping0 `54db8`, type2 `54de8`, spring `4415ffff` at `54dec`, strength
   `470c9fff` at `54df4`. Hook0 angular and Hook1 dynamics persist.
4. If proxy byte is nonzero, sample CURRENT `kOffboardDriveScalar` from
   cvar `8308e538` target+28/value+d8 at `54ecc`. This read occurs even when
   the later collision strength selects normal behavior. Otherwise scalar1.
5. Sample retained collision drive_strength once at `54ee0`. Ordered `<1`
   selects soft; raw BO4 !LT at `54eec` includes NaN in normal branch.
6. Update Parts1..22 ascending, local drive then root drive. Proxy mode chooses
   the source partial mask `[1,1,1,1,1,1,0,1,1,1,0,...,0]`; ordinary mode uses
   all ones. Flags write before parent checks and per-bone strength/frame work.
7. Lazy soft damping per active soft drive, cvar `8308e474` target+28/value+d8
   at `82b48a88`, after frame stores and before dynamics. Normal follow instead
   captures the retained cached animation layout and reads offsets8,12,4,0.

`SkeletonDriveRuntime.prepared` owns these original mutable frames/dynamics
payloads. Actual admitted drive occurrences refer to those same payloads.
`runtime.update_attributes` is the provider moved from constructor selection;
do not rebind or clone substitute selections. Its global update instance is
animation_default class `babb3ec110849e08`, collection
`d7edbd362d7d2152`, distinct from constructor skeleton-drive tuning and the
global physics_animation provider used by landing. Existing cvar trait
`SkeletonDrivesUpdateCvars` provides the two lazy reads. Existing
`SkeletonDriveNumeric` preserves per-quaternion estimate boundaries.

The existing bridge
`dynamic_assembly_skeleton_drive_update.rs::try_update_drives_from_controlled_pose`
cannot be used directly after IK. It invalidates current-animation readiness,
remaps a supplied raw pose into these rows, and requires a matching completed
collision frame with a once-per-frame attempt guard. These guards describe the
existing controlled feedback experiment, not the native GeneralUpdate call.
Grounded hook/body movement already invalidates the world collision publication;
GeneralUpdate consumes retained collision response rather than requesting a new
collision calculation between IK and drives.

Smallest coherent bridge: reuse/extract the existing actual-backend,
retained-data/provider and admitted-drive occurrence validation; call
`runtime.prepared.try_update` on the current volume rows, existing collision
data, original per-Part drive coefficients, and `u8::from(existing proxy flag)`.
Do not remap or reset animation/history. Preserve lazy source failure prefixes
and the actual GeneralUpdate stage if a drive service fails. If progression
needs a host capability, tie it to this GeneralUpdate attempt, not a new contact
frame. State `0x2be` can reach this drive stage without IK; other states must
complete the actual IK stage first.

## Remaining GeneralUpdate force suffix

After drives succeeds, `82b5bb80..82b5bc3c` builds a displacement. PDB confirms
retained `Sk8::Physics::ProcessedPhysIn +964` is `mGravity` (load `bc00`).
This is a scalar distinct from retained processed timestep+93c, root-input
timestep, and Simulation pool gravity. No existing caller may silently replace
it with one of those values.

Let `t = f32::from_bits(0x3c888889)` from `kUpdateTime` at `82261f04` and
`half = f32::from_bits(0x3f000000)` at `8200fd30`. Source constructs known
full4 `[+0, -mGravity, +0, +0]`, then performs three separate f32 vector products:

```
a = displacement * half; // bc24
b = a * t;               // bc30
c = b * t;               // bc34
```

No factor combination or dt substitution preserves this rounding graph.
Call `SkeletonBody::AddDisplacement` at `bc3c`, then GeneralUpdate returns.

AddDisplacement visits actual Parts0..23 ascending, calling
ApplyDisplacementToPart for each at `59ad0`. Fixed bounds make its index
assertions provable. The helper looks up the actual Part body through the
existing Assembly; native has no null-body branch. Per-Part helper:

1. Capture body `mInvm +7c`, then current force X/Y/Z (`5463c..54658`).
2. Estimate reciprocal of the fixed t at `54664`; two fused residual/refinement
   steps at `54678..54684`: `r = r.mul_add((-r).mul_add(t,1),r)` twice.
3. Full4 `scaled = reciprocal * supplied_displacement` (`54688`), then
   `delta = scaled * captured_inverse_mass` (`5468c`). PDB confirms +7c is
   inverse mass, not mass. Do not algebraically cancel either dt.
4. Add to captured force with separate scalar additions. Store force X at
   `546a8`, Y at `546b0`, Z at `546b4`, then `mCool +ac = 0` at `546b8`.

All 24 reciprocal operations execute, even though t is fixed. A fallible numeric
boundary must preserve completed prior Parts and leave the failing Part's force
and cooldown unchanged. No per-Part body mutation precedes its estimate; force
and inverse mass are captured before it. Force W is the separately owned
`mKine +9c` and survives. No velocity, omega, pose, history, mass, inertia,
solver delta/RF, ring, allocation, or hook write occurs here. This is force
accumulation despite the source function name, not position relocation.

Smallest new seam is an in-place operation on the same rigid-body pool,
forwarded through its backend and actual Part owner. Accept displacement plus
the existing reciprocal-estimate numeric service; use live stored inverse mass
and force. Do not use a body snapshot replacement, impulse abstraction, fresh
inertia read, pool gravity, or full body reset. Numeric policy remains the
existing explicit host-versus-Xenon estimate boundary.

## Grounded return tail and completion boundary

`UpdateGroundSkateboard [82b5e468,82b5ea24)` calls full GeneralUpdate at
`82b5e9d8`. ONLY after return, it resets Skeleton `+2e80`
NextTrajectoryTransform. Raw store order is X at `ea04`, P at `ea08`, Y at
`ea0c`, Z at `ea10` (r31=30, r29=10, r30=20). X/Y/Z come from source constants
`8227e8f0/8227e900/8227e910`, unit axes with known W0; P is full zero v127
created at `e6b0`. The resulting identity has P.W=0, not homogeneous1.

Thus a coherent complete grounded continuation is: successful hooks/reset;
state-dependent UpdateIKData and IK.Update; retained current-row drives update;
gravity-derived per-Part force accumulation; then X/P/Y/Z trajectory reset.
Any earlier error must prevent the later stages. The currently accepted
`...grounded_before_ik` entry correctly stops before these stages and must not
be labeled complete GeneralUpdate.

## Confidence and validation proposal

Exact: listed addresses/raw words, PDB field identities, integer loops,
store/service order, literal constants and retained-authority mapping.
High: proposed native Rust bridge and complete finite operation graph, subject
to independent review. Host estimate/reduction/NaN/denormal behavior is not an
exact Xenon numerical claim. Native mutable-alias interference is excluded by
the existing exclusive owners and immutable retained Vault graph.

Focused validation: prove final IK-mutated rows reach existing drive payloads
without remapping; verify proxy mask comes from its sole retained owner; inject
late cvar/quaternion failure and retain completed prefixes. Verify force suffix
with nonzero force, varying live inverse masses, nonzero cooldown and preserved
kinematic bits/RF/rings; fail a later reciprocal call. Extend original-owner
grounded corpus through the same admitted drives and bodies, independently
check source-form force arithmetic, and execute the existing solver afterward
to demonstrate consumption. Preserve a clear component-input scope until actual
processed gravity and full gameplay scheduling producers are connected.
