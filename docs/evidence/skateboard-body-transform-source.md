# Body-frame versus Part-frame getters

Original debug XEX/PDB audit, 2026-09-08. No code or Cargo changes. This is the
getter prerequisite for the compression and Group4 stages in
`skateboard-body-postphysics-runtime-plan.md`.

**Compression and Group4 require GetBodyTransform. The existing
try_live_part_transform_host is GetTransform and is not an interchangeable
getter when the original Part has an inverse-body mapping.**

## Exact calls and ABI

| Native getter | Range | SHA-256 |
| --- | --- | --- |
| `rw::physics::Part::GetBodyTransform` | `827ab5c0..827ab710` | `f283679c65b6c498b1c7ec0c548ad5f1094134c46c82cb6bfbb770b05d73202d` |
| `rw::physics::Part::GetTransform` | `826c3480..826c35a8` | `ad60181e8e2c92d1f75b60b092d975aeb92f3166f64ea6627d7633fc827edf32` |

Both have hidden returned Matrix44Affine storage in r3, and the actual Part
pointer in r4. Decompiler argument names reverse these identities. PDB Part
layout: cached m_partTM+0, m_inverseBodyLTM+44, m_body+4c; stride60.

CalculateAverageWheelCompressions `82b2b538` reads actual Assembly+18 from
SkateboardBody, then part-list+8, then passes list+240 (Part6) to
**GetBodyTransform at b564**. Subsequent **b658, b674, b690, b6ac** obtain
Parts0,1,2,3 through the same getter, in that order. The deck-body inverse is
computed between the first and second calls. Do not gather all five frames
before executing that inverse's fallible numerical work.

BodyPostPhysics Group4 takes the same actual list+240 at **82b36cc4** and
calls **GetBodyTransform at 36cc8**. The native group branch chooses this
getter only after identifying the actual admitted contact's group as 4.
Do not fetch the frame eagerly for contacts which never enter that branch.

StartWheelLineTest `82b24530` instead calls **GetTransform at 245c4** inside
the four-wheel loop and **GetTransform at 24664** for Part6. The existing
wheel bridge's use of try_live_part_transform_host therefore selects the
correct getter for those rays. Do not change that existing call to body pose
when adding the new compression getter.

## Pointer-selected behavior

| Condition | GetBodyTransform | GetTransform |
| --- | --- | --- |
| body nonnull; inverse pointer arbitrary | copy live body frame directly; never inspect inverse pointer or cached frame | inspect inverse pointer; compose bodyFrame * inverseBodyLTM when nonnull, otherwise copy body frame |
| body null; inverse nonnull | cachedPartFrame * rigidInverse(inverseBodyLTM) | copy cachedPartFrame without inspecting inverse pointer |
| body null; inverse null | copy cachedPartFrame | copy cachedPartFrame |

GetBodyTransform's attached branch is precise raw copying:

1. body=Part+4c (`b5c0`), branch on null (`b5cc`).
2. Load body+10 full vector first (`b5d4`) and retain it for translation.
3. Load body+40 (`b5e8`), store result+0 (`b5ec`).
4. Load body+50 (`b5f0`), store result+10 (`b5f4`).
5. Load body+60 (`b5f8`), store result+20 (`b5fc`).
6. Store the already captured translation at result+30 (`b600`), return.

This branch has no normalizing, quaternion rebuilding, general inverse,
inverse-body mapping read, or cached Part read. A host getter can validate the
actual session/backend/body occurrence and return the retained body XYZ pose;
it must not add those geometric operations. If exposed as staged fallible
column callbacks in future, retain the translation-first read and subsequent
column/store order. Current actual pool reads are noninterleaved under the
borrowed world and do not expose per-column mutation callbacks.

Only detached GetBodyTransform reads inverseBodyLTM+44 (`b608`). A nonnull
mapping takes `b61c..b6e8`: transpose the mapping's XYZ rotation, build inverse
translation from +0-minus-translation, using Z multiply then Y/X FMAs; compose
cachedPartFrame * that rigid inverse. This is not a general matrix inverse.
The source composition keeps full vector lanes and its multiply/FMA graph.
With no mapping it copies the four cached columns (`b6ec..b708`) in order.

GetTransform instead tests body at `6c3480..348c`, checks mapping only in
the attached branch (`3490..3498`), and applies bodyFrame * inverseBodyLTM
without first inverting the mapping. Its detached branch at `357c` directly
copies cached columns. These are two different coordinate frames, not merely
different cache freshness policies.

## Existing code reuse and the narrow runtime addition

`dynamic_assembly_part_transform.rs::try_live_part_transform_host` validates
session/backend/index, chooses attached body before cached readiness, obtains
`SimulationAssemblyBackend::try_part_body_pose`, then calls
`world::part_get_transform` with the actual metadata's optional inverse-body
mapping. It correctly implements GetTransform's ordinary finite XYZ path.
Calling it for compression would apply the unwanted Part-frame mapping.

Reuse its existing owner/index validation and the same retained Part.body
handle lookup, but give GetBodyTransform a distinct named API. The attached
arm can directly call `SimulationAssemblyBackend::try_part_body_pose`, which
delegates to `SimulationRigidBodyPool::try_rigid_body_pose`. That pool getter
reads the actual retained right/up/at/COM fields after occurrence/ring
validation; it does not rebuild orientation from quaternion.

For the current actual constructed-board caller every Part has its retained
body. An explicitly attached-only getter may refuse a missing body before
claiming detached support. For a complete getter, the detached arm can reuse
`world::part_attachment_transform(cached, inverse)` after preserving source
branch order and checking actual cached-lane readiness. That helper already
implements the same transpose-only inverse plus composition graph for
Part::AddToSimulation. This helper is distinct from the **general cofactor
inverse used later by CalculateAverageWheelCompressions on the returned deck
body frame**; those two inverse operations must not be conflated.

## Full-vector knowledge boundary

PDB RigidBody fields explain the fourth words copied by the native getter:

| Copy address | Semantic XYZ | Adjacent native W word |
| --- | --- | --- |
| body+10 | mCom | mId+1c |
| body+40 | mRi | mStasis pointer+4c |
| body+50 | mUp | mInertia pointer+5c |
| body+60 | mAt | mTag+6c |

The current pool's host affine getter explicitly inserts axis W=+0 and
translation W=1 for its public matrix interface. Those values are not native
packed words. A new source-vector getter must expose these four W lanes as
unknown, or use an actual independently proven packed representation; do not
wrap that host array in `SkeletonRootMatrix::known`. The compression/group4
consumer may retain known XYZ while carrying unknown W through its existing
numeric/vector boundary. A need for full lanes downstream is an explicit
readiness issue, never permission to turn host affine conventions into source
data. Detached known cached matrices retain their actual authored W bits.

Minimum verification: an attached Part with a nonidentity inverse-body map
must distinguish body getter from current Part getter; attached incomplete
cache must still succeed; foreign/stale body authority must fail; and the
actual original-board compression/Group4 caller must retain unknown W while
consuming the exact same body occurrence. Existing wheel-line geometry tests
continue to exercise GetTransform and should remain unchanged.
