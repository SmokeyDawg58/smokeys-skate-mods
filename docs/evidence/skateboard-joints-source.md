# Authored skateboard joints and level truck transforms

Source-only recovery, 2026-09-07. No joint constructor or truck-transform
implementation was added by this investigation. Confidence: **high** for the
control flow, typed payloads, lookup chronology, and arithmetic below; native
Xenon numeric/runtime traces remain unvalidated. Live IDA PPC instructions and
PDB types, not the decompiler's scalar arithmetic rendering, are authority.

## Identity and placement

- `Sk8::Physics::SkateboardBody::CreateJoints`,
  `?CreateJoints@SkateboardBody@Physics@Sk8@@AAAXXZ`,
  `0x82b2d0e8..0x82b2e080`, SHA-256
  `8b5671cca5ac70f596bb9508fb6880ae926f348fce0d8b83e98cbdd09963565d`.
- `Sk8::Physics::SkateboardBody::CalcTruckTransforms`,
  `?CalcTruckTransforms@SkateboardBody@Physics@Sk8@@AAAXXZ`,
  `0x82b2c390..0x82b2c998`, SHA-256
  `3686237ab7c902b6ab7e569466955fb9d440b083bebb49f1ff17f56618930ab2`.
- `SkateboardBody::Init` calls CreateJoints at `0x82b376b0`, after wheels,
  trucks, and deck construction and before culling-table initialization.
- `SkateboardBody` constructor calls CalcTruckTransforms at `0x82b38828`,
  after Init, island insertion, and SyncCheckBody, and before CreateDrives.
  CreateJoints does **not** call CalcTruckTransforms. They repeat related
  matrix calculations with different attribute lookup/load schedules.

Init's lookup at `0x82b37580` reads physicstrucks/default key
`b1cbe3e929afc8a0`, index zero. Missing pointer uses DefaultDataArea(1), then
`lbz` at `0x82b37594`: zero gives zero joints; any nonzero byte gives six.
Definition allocation uses seven parts, this joint count, and zero drives.
`cmpwi`/`ble` at `0x82b376a4/6a8` skips CreateJoints for zero. CreateJoints
itself has no enable lookup or alternate joint count.

## Existing reusable owners

`simulation_joint_pool.rs` already defines the exact aligned value payloads:

- `SimulationJointFrames`, size 0x50: quaternion_a +00, position_a +10,
  quaternion_b +20, position_b +30, quaternion_l +40. A is child; B is parent.
- `SimulationJointLimits`, size 0x40: p_prism +00, v_prism +10, v_twist +20,
  v_swing +24, swing_a +28, twist_a +2c, swing_c +30, twist_c +34,
  raw_swing_type +38, raw_twist_type +3c.

Use these concrete types, not a second joint representation. An authored
definition can own six `DynamicAssemblyJointDefinition<Arc<SimulationJointFrames>,
Arc<SimulationJointLimits>>` values, whose parent_part/child_part are signed
indices. `SimulationJointPool::try_add_joint(parent, child, &frames, &limits)`
retains the Arcs and stores parent into body_b, child into body_a.
`SimulationAssemblyBackend<M, SimulationJointFrames, SimulationJointLimits>`
already implements the corresponding body-occurrence-checked add port.

Native `Assembly::Initialize` `0x82ae1338..0x82ae13ec` calls
`InitializeComponentsFromDefinition` `0x82ae0bf8..0x82ae0df0`. Its joint loop
retains each ordinal's frames/limits and resolves the ordered parent/child
part indices. `JointedPair::Initialize` `0x82ae8f78..0x82ae8fc8` simply stores
these body-space frame and limit pointers; it does not adjust them by COM or
recompute angular frames. SHA-256 respectively:

- `0aaa886f0912d51660720587470c3f9b12fd9c169d7d6f66391073c88e3ce3fc`
- `007c0a98a60d5a7c3535621ca32e78e94a46087b292a1a4b77a6c477307cba51`
- `19d4a6c521686622018cd2d6bfd561bc54ba6d49f3147cb08a6edd4acffe7608`

This does not make the existing clustered-mesh runtime-template producer an
authored primitive-board producer: its collision owner still specifically
requires `DynamicClusteredMeshTemplate`. Do not fabricate that capability to
admit authored sphere/capsule/aggregate parts. Authored seven-part ownership,
initial transforms, and actual admission must be connected honestly.

## CreateJoints lookup and load sequence

All lookups use index zero. Each missing pointer selects DefaultDataArea(4).
Class singleton offsets: physicsdeck +94, physicswheels +9c, physicstrucks +dc.
The existing `SkateboardAttributeRead` lookup/float port preserves retained
pointers and delayed loads. Semantic descriptions below are roles, not
assertions of original plaintext attribute spellings.

| Order | Call address | Class | Full key | Role |
| --- | --- | --- | --- | --- |
| 1 | 82b2d238 | trucks | 7ca201065c78f1cb | first truck angle |
| 2 | 82b2d3ac | deck | c94d65091dcb828b | first deck length pointer |
| 3 | 82b2d404 | trucks | e02dcbd07830d3a5 | positive-Z offset |
| 4 | 82b2d4ac | trucks | 2be08dd7a9207622 | first vertical offset |
| 5 | 82b2d57c | trucks | 7ca201065c78f1cb | second truck angle, reread |
| 6 | 82b2d6c4 | trucks | a69cca4e15dba369 | negative-Z offset pointer |
| 7 | 82b2d71c | deck | c94d65091dcb828b | second deck length |
| 8 | 82b2d7b8 | trucks | 2be08dd7a9207622 | second vertical offset |
| 9 | 82b2d8a4 | trucks | e8b34c12d753b164 | twist arc angle |
| 10 | 82b2d950 | trucks | 870ba43ed43377d6 | twist velocity input |
| 11 | 82b2dc50 | wheels | 5a84c435dc340af8 | positive axle offset |
| 12 | 82b2dcf4 | wheels | 5a84c435dc340af8 | negative axle offset, reread |
| 13 | 82b2dde4 | wheels | 1a90ba4d11957560 | axle swing velocity input |

At d41c the positive offset is loaded, then at d420 the retained deck length
is loaded. `fmadds` d430 computes length.mul_add(0.5, offset), **not** separate
multiply/add. At d730 the retained rear offset is loaded, d738 negates it,
d73c loads deck length, and `fnmsubs` d744 computes the negated fused
product-minus-negated-offset: `-(length.mul_add(0.5, -(-offset)))` with the
two source negations retained (do not simplify signed-zero/NaN handling).
The output positions are `[+0, vertical, longitudinal, +0]`.

The first angle is multiplied by DTR and its sign bit flipped before SinCos;
the second is multiplied by DTR without that flip. The two vertical values
and two wheel offsets are independently sampled. The truck arc/cosine and
velocity are sampled once and copied to both truck limits; wheel velocity is
sampled once and copied to all four axle limits. Do not cache across the
explicit repeated lookups.

## Matrix and quaternion calculations

Source constants: DTR bits `3c8efa35` at `0x8225c454`; positive half-pi bits
`3fc90fdb` at `0x8225c464`; negative half-pi bits `bfc90fdb`; timestep bits
`3c888889`; angular velocity multiplier bits `426fffff` (not exact 60).

For SinCos(Y angle) = (sY,cY), the constructed four-lane matrix columns are
`Yx=[cY,0,-sY,cY]`, `Yy=[0,1,0,0]`, `Yz=[sY,0,cY,sY]`.
For SinCos(X angle) = (sX,cX), the constructed columns are
`Xx=[1,0,0,1]`, `Xy=[0,cX,sX,0]`, `Xz=[0,-sX,cX,0]`.
These raw W lanes follow `VectorPermuteConstant<0,5,0,0>` plus Z-lane
insertion; do not substitute conventional W-zero Matrix33 columns.

Compose each Y column a into `R(a)` lane-wise in exactly this order:
`partial=Xx[lane]*a.x`, then `partial=Xy[lane].mul_add(a.y,partial)`, then
`result=Xz[lane].mul_add(a.z,partial)`. This is X*Y, with initial multiply
followed by Y then Z fused additions. The first truck uses Y=-half-pi and
X=-(first_angle*DTR); the second uses Y=+half-pi and X=second_angle*DTR.

First frame setters: child identity d434, parent R d440, linear R d44c.
Second: child identity d748, parent R d754, linear R d760. Wheel frames use
only the constructed X matrix at +half-pi: child/parent/linear setters at
dbe0/dbec/dbf8, then dc84/dc90/dc9c. Both wheel frame sets are calculated,
not one quaternion copied from a hardcoded quarter-turn constant.

`JointFrames::SetChildAngularFrame` `0x82b22f58..0x82b23108` stores at +00;
`SetParentAngularFrame` `0x82b23108..0x82b232b8` stores at +20;
`SetParentLinearFrame` `0x82b232b8..0x82b2346c` stores at +40. Their matrix
conversion constructs the four quaternion candidates, uses a vector rsqrt
estimate plus two refinements, and selects by strict comparisons:
trace>0, otherwise xx>yy && xx>zz, otherwise yy>zz, otherwise Z candidate.
Thus ties/unordered comparisons are not normalized into a preferred axis.
Candidate magnitudes group `(signed_x + signed_y) + (signed_z + 1)`; trace
groups `(xx+yy)+zz`. There is **no final quaternion normalization**.

The existing private `world::simulation_quaternion_candidate_host` expresses
this candidate graph and is the smallest math reuse opportunity after a
source-reviewed visibility/numeric-seam refactor. Its public wrapper
`simulation_quaternion_from_orthonormalized_matrix_host` adds a final dot4 /
rsqrt normalization that is absent here and must not be used unchanged.
Preserve all twelve setter evaluations, including identity-matrix ones.
The helper SHA-256 values in order above are:

- `87ab1e713082262ad5ff55af873e86e9e89877a78fdb8a5f00716a4224c93a5c`
- `24162358d7515c009e158ce50419c1cc7af4afd7b8ab877191510f725168e12f`
- `7d25c69d6286fb83aa3cf9d4de5f85632f313db1166fefa23649d1232384d817`

SinCos is `0x82584f60..0x825850e8`, SHA-256
`418cfa6763a323a517a3b4ab4da4ce161306cab52016b9f536e1d5714c67bae0`;
Cos is `0x824b0130..0x824b0208`, SHA-256
`6fa06e3a5c04ff91ae16eede7f791f1ad5cd931b8e3fcc34e3554aafb16253a5`.
This packet preserves their calls, not a claim that host libm equals Xenon.
An explicit numeric provider should expose SinCos, Cos, rsqrt estimate and
reciprocal estimate, while the recovered fused refinements remain authored
math. The deck provider has related seams but should not force joint code to
depend on deck-only mass/integral behavior.

## Six outputs in source order

All position_a values are four positive zero lanes. Let Iq mean the result
of the source identity-matrix setter, Q0/Q1 each truck's converted R, and Qw
the converted +half-pi X matrix. Identical matrix arguments still undergo
the respective setters. The table describes values, not coalesced calls.

| Joint | Parent | Child | quaternion_a | quaternion_b / quaternion_l | position_b |
| --- | --- | --- | --- | --- | --- |
| 0 | deck 6 | truck 4 | Iq | Q0 | [0, first vertical, positive Z, 0] |
| 1 | deck 6 | truck 5 | Iq | Q1 | [0, second vertical, negative Z, 0] |
| 2 | truck 4 | wheel 0 | Qw | Qw | [0,0,+first width,0] |
| 3 | truck 4 | wheel 1 | Qw | Qw | [0,0,-second width,0] |
| 4 | truck 5 | wheel 2 | Qw | Qw | [0,0,+first width,0] |
| 5 | truck 5 | wheel 3 | Qw | Qw | [0,0,-second width,0] |

Truck limits (writes 82b2da7c..82b2daa0 and the subsequent second copy):
p_prism=zero; v_prism=refined_reciprocal(timestep)*zero; v_twist=
`(input*DTR)*f32::from_bits(0x426fffff)` with two separate products;
v_swing=0; swing_a=0; twist_a=arc_input*DTR; swing_c=1;
twist_c=Cos(twist_a); raw_swing_type=0 (locked), raw_twist_type=1 (arc).

Axle limits (82b2deac..82b2e064): p_prism=zero;
v_prism=second_refined_reciprocal(timestep)*zero; v_twist=0;
v_swing=`(input*DTR)*f32::from_bits(0x426fffff)`; swing_a=twist_a=0;
swing_c=twist_c=1; raw_swing_type=3 (axle), raw_twist_type=0 (locked).

September24 correction: the earlier value4 recorded here was incorrect.
Direct IDA disassembly loads `li r10,3` at82b2de84 and writes that register
to JointLimits+38 at82b2df0c,82b2df78,82b2dfdc and82b2e048. The twist stores
use r31, initialized to0 at82b2d25c. JointJacobian::Build reads +38 at82ae5d44
and compares unsigned with3 at82ae5d50: value4 selects free/default, whereas
3 selects the axle route. This discrepancy was found while investigating
weak moving steering. See artifacts/push-feedback-investigation-20260924.md
for the current implementation and replay gate; no native-feel parity claim.

The timestep reciprocal operations occur separately at d844 and dd84.
Independent raw-word review confirms multiplication operand order: 178bf09d
at82b2d868 and17ebc09d at82b2dda8 use refined estimate as VA and zero as VB.
Each starts with an estimate and performs twice
`residual=(-estimate).mul_add(timestep,1)` then
`estimate=estimate.mul_add(residual,estimate)` before multiplying the zero
vector. Do not reuse the first estimate for the wheel group, replace this
path by the fixed angular multiplier, or discard the operations because
ordinary finite outputs happen to be zero. No angular-value clamp or
finite-value guard occurs in CreateJoints.

## CalcTruckTransforms differences and output ownership

Its eight lookups, all index zero/default-four, are:
deck length c400; trucks positive offset c458; trucks rear offset c4c0;
deck length c518; trucks angle c5ec; trucks vertical c718; trucks angle
c7d8; trucks vertical c90c. The full keys are those in the preceding table.
At c474 it loads the retained first length, c478 loads the positive offset,
then `fmadds` c484 computes positive Z. This load order is opposite to
CreateJoints' offset-then-length loads. At c52c it loads retained rear
offset, c534 negates it, c53c loads length, `fnmsubs` c558 computes negative
Z. Do not merge the two functions' callbacks into a shared tuning snapshot.

SinCos call sequence: c55c(-half-pi), c64c(-(first_angle*DTR)),
c760(+half-pi), c840(second_angle*DTR). It uses the same four-lane matrix
construction/composition described above, but stores matrices directly,
without quaternion conversion. At c958..c964 it stores first rotation and
positive-Z position to **mBackTruckTransformLevel**, PDB offset +1e50.
At c968..c978 it stores second rotation and negative-Z position to
**mFrontTruckTransformLevel**, PDB offset +1e10. Preserve these apparently
counterintuitive names; do not relabel from the longitudinal sign.
Translation W is zero. The rotation columns retain the constructed W lanes.

## Corpus check, remaining seams, and discriminators

The existing read-only inspect_vault executable was run on external extracted
`data/db/unpcaked#/data/db/skater.vlt` with its skater.bin dependency, selecting
physicstrucks/default and physicswheels/default. This is the previously
documented relocated-pointer-word extraction, **not a new raw-mounted test**.
Selected entries were inline scalars (node 40, no PtrN at their scalar word):

- trucks angle `7ca201065c78f1cb` = `41f80000` (31);
- trucks arc `e8b34c12d753b164` = `41600000` (14);
- trucks velocity `870ba43ed43377d6` = `42ac3333`;
- trucks vertical `2be08dd7a9207622` = `bd676c8b`;
- both longitudinal offsets = `bd54fdf4`;
- wheels width `5a84c435dc340af8` = `3dc28f5c`;
- wheels velocity `1a90ba4d11957560` = `49742400` (1,000,000);
- enable key `b1cbe3e929afc8a0` = byte 01 (raw word 01000000).

These are evidence oracles, not runtime constants. The live retained
AttributeRuntime graph plus SkateboardAttributes already supplies these
classes and supports typed source reads. Original plaintext names for the
new keys were not recovered; use the full 64-bit keys and semantic roles.

Smallest useful implementation slice: authored six-record joint definition
builder over SkateboardAttributeRead plus explicit numeric seams; source
candidate-quaternion helper reuse; independent CalcTruckTransforms builder
with its own callback chronology. This produces actual frames/limits for
existing joint pools, but does not itself admit the seven authored parts,
initialize their source transforms, create the two truck drives or mandatory
hook, or constitute a skating body/tick. Those remain connected-owner work.

Focused tests should discriminate: byte enable versus word truthiness;
13 versus 8 exact lookup orders with mutation at delayed loads; both fused
longitudinal calculations; signed-zero/NaN retention; raw matrix W lanes;
matrix candidate versus normalized quaternion; strict trace/axis ties;
twelve setter evaluations; two separate reciprocal groups; exact six endpoint
pairs; independent width/vertical/angle rereads; no angle/velocity clamps;
actual mounted lookup through production provider into all six payloads;
and eventual admitted seven bodies -> six live joint occurrences -> existing
joint Jacobian/Horus consumer, with no fabricated clustered-mesh owner.

## Implemented and reviewed checkpoint

2026-09-07: skateboard_joints.rs and its ten focused tests now implement the
bounded six-joint builder and independently sampled CalcTruckTransforms.
Independent live PPC/code/test review passed, including the operand correction
above. The shared unnormalized candidate has two additional focused tests.
The raw-mounted board corpus now admits seven source-placed bodies and all six
joint records into the actual pools, verifies ordered endpoints and teardown.
It does not yet compile these authored joints into a live board solver tick.
Full gate:2294 unit tests,6 doctests, fmt and warnings-denied Clippy; all four
core original-data tests pass. See skateboard-assembly-admission.md.
