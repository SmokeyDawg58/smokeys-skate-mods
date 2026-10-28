# Authored skateboard initial placement and self-culling

2026-09-07 live IDA/PDB recovery. Confidence: `exact` checked stores, keys,
bit layout and call/branch order; `high` readable arithmetic and ownership.
Implementation and eight focused tests are written in
`skateboard_initial_transforms.rs` and `skateboard_initial_transforms_tests.rs`,
with independent source/implementation audit and execution pending. Host trig is not original
XMVector numeric parity; there is no captured runtime trace.

## Initial seven-part transforms

`Sk8::Physics::SkateboardBody::InitializeTransforms`,
`?InitializeTransforms@SkateboardBody@Physics@Sk8@@AAAXXZ`,
`0x82b2bc10..0x82b2c38c`, writes Parts in order **6,0,1,2,3,4,5** before body
admission. This is local placement, not the later input spawn transform.

All lookups use index zero and missing-pointer `DefaultDataArea(4)`. There are
14 lookups, not one shared placement snapshot:

| Order | Address | Class | Key / meaning |
|---|---|---|---|
| 1 | 82b2bcdc | deck | c94d65091dcb828b length pointer |
| 2 | 82b2bd34 | trucks | e02dcbd07830d3a5 front offset |
| 3 | 82b2bd9c | trucks | a69cca4e15dba369 rear-offset pointer |
| 4 | 82b2bdf4 | deck | c94d65091dcb828b length again |
| 5-6 | 82b2be80,82b2bed8 | trucks,wheels | vertical,lateral for wheel 0 |
| 7-8 | 82b2bf60,82b2bfb8 | trucks,wheels | vertical,lateral for wheel 1 |
| 9-10 | 82b2c03c,82b2c094 | trucks,wheels | vertical,lateral for wheel 2 |
| 11-12 | 82b2c11c,82b2c174 | trucks,wheels | vertical,lateral for wheel 3 |
| 13 | 82b2c21c | trucks | vertical for truck 4 |
| 14 | 82b2c320 | trucks | vertical for truck 5 |

Vertical key `2be08dd7a9207622`; lateral key `5a84c435dc340af8`.
Classes/default: deck `9ed291a274453fc0`, trucks `af781d7b614dab50`, wheels
`46add2eea90cc062`; collection `d7edbd362d7d2152`.

Exact delayed loads:

- Front: retain length pointer, fetch front pointer, load length then front at
  `82b2bd50/54`; `fmadds` at `82b2bd60` computes
  `front_z=length.mul_add(0.5,front_offset)`.
- Rear: retain rear pointer, fetch length, load rear at `82b2be08`, negate at
  `be10`, load length at `be20`; `fnmsubs` at `be40` computes
  `rear_z=-fms(length,0.5,-rear_offset)`. Preserve fused product/subtraction
  and source negations. Host expression is
  `-(length.mul_add(0.5,-(-rear_offset)))`, not separate multiply/add.
- Every wheel retains vertical pointer across lateral lookup, then loads
  lateral before vertical. Do not reuse either input across wheel calls.
- Each truck independently reads vertical.

`I0` below denotes I/J/K axes with zero axis-W and translation-W:

| Part | SetTransform call | Axes | Translation |
|---|---|---|---|
| 6 deck | 82b2bc8c | I0 | [0,0,0,0] |
| 0 wheel | 82b2bf2c | I0 | [-lateral0,vertical0,front_z,0] |
| 1 wheel | 82b2c008 | I0 | [+lateral1,vertical1,front_z,0] |
| 2 wheel | 82b2c0e8 | I0 | [-lateral2,vertical2,rear_z,0] |
| 3 wheel | 82b2c1c4 | I0 | [+lateral3,vertical3,rear_z,0] |
| 4 truck | 82b2c270 | I0 | [0,vertical4,front_z,0] |
| 5 truck | 82b2c36c | rotation below | [0,vertical5,rear_z,0] |

Only after setting truck 4 does native call `XMVectorSinCos` at `82b2c28c`,
using positive half-pi `3fc90fdb` at `8225c464`. With returned sine s and
cosine c, truck 5 axes are:

```text
X=[c,0,-s,c]
Y=[0,1,0,0]
Z=[s,0,c,s]
```

The nonzero W lanes are actual stores, not conventional affine-matrix
substitution. Live permute bytes at `82fd6e90` are
`00010203 14151617 00010203 00010203`: `<0,5,0,0>`; subsequent mask-2 insertions
replace Z only. Raw VMX operands were checked, accounting for IDA's
high-register display error. The same Y construction appears in the
independently recovered joints packet.

Original extracted Vault words reread with existing `inspect_vault.exe`:

```text
deck length  3f170a3d
front offset bd54fdf4
rear offset  bd54fdf4
vertical     bd676c8b
lateral      3dc28f5c
joint enable 01000000
```

Asset path: external supplied build's `data/db/unpcaked#/data/db/skater.vlt`
and declared `skater.bin` dependency. These are pre-relocated extracted-corpus
observations, not a newly executed raw mounted archive test. All listed truck
values are explicit inline entries; truck default has no parent/layout.

## Part initialization and GetTransform

`Part::Initialize(resource,definition)` at `82ae9820` delegates to `82ae8fc8`,
then retains `&definition.inverseBodyLTM` iff the presence word is nonzero.

PDB Part size 0x60:

```text
+00 stored part transform
+40 volume pointer
+44 inverse-body transform pointer
+48 inertia pointer
+4c rigid-body pointer
+50 material pointer
+54 world index
+58 internal flags:u16
+5a user flags:u16
+5c user data
```

Base initialization sets stored transform I/J/K/zero, body null, inverse pointer
null, world index zero, user flags/data zero; static truthiness becomes internal
bit zero. Definition lifetime must exceed all retained Part references.

`Part::SetTransform` is `82b25480..82b25ba0`. During initial placement, its body
test at `82b254a8..b0` selects detached handling. It copies all 16 input lanes
to stored Part transform at `82b25b70..8c`. No inverse application,
orthonormalization or body update runs yet.

`Part::GetTransform`, `826c3480..826c35a8`:

1. Test body pointer first.
2. Null body: copy stored transform, all 16 lanes.
3. Body present, inverse pointer null: copy body axes +40/+50/+60 and COM +10,
   including adjacent fourth words.
4. Both present: calculate `body_transform*inverse_body_ltm`. Basis columns
   use X multiply then Y/Z FMA; translation uses X/Y/Z FMA beginning with body
   COM. Native computes all four output lanes.

Reuse `world::part_get_transform` and `part_attachment_transform` within their
documented boundaries. `compose_collision_affine` matches finite XYZ but
canonicalizes unused W lanes; it is not raw four-lane GetTransform parity.
Initial detached copy preserves raw lanes. Do not normalize initial truck 5
before admission; existing body Add/orthonormalization is the real consumer.

## Exact culling table and direct consumer

`InitializeCullingTable`, `82b34d98..82b34fa8`, allocates 20 bytes, alignment 4,
then calls `rw::BitTable::Initialize(resource,7,7)` at `82b34e54`.
`BitTable::Initialize`, `82bc3308..82bc336c`, sets:

```text
rows      +00=7
columns   +04=7
arraySize +08=(7*7+31)>>5=2
words     +0c=initially[0,0]
```

Outer loop visits rows 0..6; each row sets columns 0..6 in order:

```text
index=columns*row+column
words[index>>5]|=1<<(index&31)
```

Final words are `[ffffffff,0001ffff]`, not two full ffffffff words. Pointer is
published into `AssemblyDefinition+28` at `82b34f9c`.

`RunIntersectionTests`, `827cbdc8..827cc410`, self-collision path reads both
volume occurrences' rigidBodyIndex. Equal indices are independently suppressed.
Different bodies consult bit `columns*first_body_index+second_body_index`; one
suppresses. Raw bit test is `827cbeec..827cbf34`.

All 49 bits suppress board self-contacts. They do NOT permit dropping the self
assembly-pair producer. `AssemblyAssemblyCollisionJob` tests nonnull self-table
at `827cc9f8..ca00`, appends self pair at `827cca04..5c`, then calls intersection
at `827cca98`. Self pair can affect island partition with zero emitted contacts.

## Init, definition and outer-constructor chronology

This context is retained for integration; the bounded initial-placement module
does NOT implement full Init, material caches or final spawn placement.

1. Init reads truck joint-enable byte at `82b37580`: zero gives 0 joints, any
   nonzero gives 6. Allocate/initialize 7 parts, selected joints, 0 drives.
2. Find temporary class `babb3ec110849e08`, collection `d7edbd362d7d2152`,
   construct local Instance at `82b3761c/2c`. No geometry reads use it; it is
   destroyed at Init end. Its binding lifetime is not implemented here.
3. Independently read vertical at `82b37670`; store wheel-distance-from-deck
   +2094 at `82b3768c`.
4. CreateWheels, CreateTrucks, CreateTriangleDeck; CreateJoints only if count>0.
5. InitializeCullingTable; Assembly::Initialize; InitializeTransforms.
6. Set Assembly group4 and each Part userdata4.
7. Populate board standard/wipeout/grind material caches. These are board
   fields, not replacement of authored per-Part materials by universal tuning.

`Assembly::Initialize` at `82ae1338` retains definition and constructs Parts
ascending, then JointedPairs ascending, then DrivenPairs ascending, and copies
culling pointer. PDB Assembly:

```text
+00 culling +04 definition +08 parts +0c joints +10 drives
+14 worldIndex +18 group +1c partCount +20 jointCount +24 driveCount
+28 userData +2c flags:u16 +2e userFlags:u16
```

`JointedPair::Initialize`, `82ae8f78`, retains parent+00, child+04, frames+08,
limits+0c and initially null live joint+10. No COM/frame conversion occurs.

Outer constructor:

```text
Init -> IIsland::AddSkaterBody(this,ACTIVE_BODY=4,true) -> SyncCheckBody
-> CalcTruckTransforms -> CreateDrives -> optional CreateWheelDrives
-> mandatory CreateHookDrive -> SetTransform(input spawn transform)
```

Call sites are `82b387e8`, `82b38808`, `82b38820`, `82b38828`, `82b38830`,
`82b388a0`, `82b388a8`, `82b388b4`. CalcTruckTransforms is after admission,
not part of CreateJoints. Its recovered equations are in the separate joints
packet and are not duplicated here.

## Authored owner and current reuse boundaries

Retain four wheel definitions sharing actual volume ownership, two trucks
sharing their capsule, deck owning its actual mapped array, seven ordered
inertia/material/inverse-presence records, six ordered joint frame/limit owners
and pairs, real 7x7 table, source initial transforms, and the independently
sampled wheel-distance field. Definition ownership must outlive Parts and
collision records.

Existing flattened selection implies 19 shipped collision occurrences: four
wheels, two trucks, then all 13 deck children associated with body index6.
Repeated shared volume pointers remain distinct Part occurrences. The deck
wrapper is not a fake clustered-mesh record.

`DynamicAssemblyPartAttachmentTemplate::try_from_parts` seals positional input.
`DynamicAssemblySession` owns Part/Joint/Drive attach and rollback chronology.
`SimulationAssemblyBackend<AssemblyPartMaterial,SimulationJointFrames,
SimulationJointLimits>` already accepts semantic inertia Arcs and real joint
Arcs with body-occurrence checks. `world::part_attachment_transform` is reused
before Add. Primitive bbox/volume-data and mass owners remain reusable.

At source investigation, runtime-template construction required a clustered
mesh owner, while serialization/culling/inertia sources required arena IDs and
address bindings; the arena-routed backend rejected nonzero joint/drive counts.
Root owns a new explicit authored source bridge. Do not fabricate BAM, arena
bases, mesh capabilities or asset IDs to bypass those restrictions. Seven
bodies/six joints is an intermediate admission checkpoint, not complete board
physics: truck drives, mandatory hook, actual collision publication and the
post-admission spawn transform remain separate work.

## Bounded implementation and validation policy

The constructor returns ordered updates and a definition-order projection.
It stages deck6 before any lookup and preserves later local store/call order.
Publishing a completed owner is host transactional policy; this API does not
expose native partial Part writes to callbacks if a lookup/numeric service
fails midway. It is not an attached Part::SetTransform implementation.

Focused tests discriminate all14 lookups/delayed loads, independent wheel
samples, both fused longitudinal formulas, one SinCos between truck4 and final
vertical read, all raw W lanes, update order6/0/1/2/3/4/5, and all49 culling
bits with zero padding. These scripted-input tests reuse the provider boundary;
missing/default semantics are tested separately in skateboard_attributes and
the shared Vault runtime, not by these eight placement tests.
Independent live PPC/code/test review and mounted input/owner admission pass.
The final root gate passes2294 unit tests,6 doctests, fmt and warnings-denied
Clippy, plus all four explicitly enabled core original-data tests.

## Live code fingerprints

```text
InitializeTransforms 82b2bc10..82b2c38c
90283702c0830dbefede4f2832928f6d7a02bc6e273f10e0cebf6d81e4d485aa
InitializeCullingTable 82b34d98..82b34fa8
47e376c7f7ab68eaf87a030ca4c5259c52a7485f81e9d9ab1bfdf4e98faa881b
Init 82b374e8..82b37b84
bf4f1b2daafff14bfb7536615e13b17a7f8b0a35c8ae8590c37949351e998657
BitTableInitialize 82bc3308..82bc336c
df46e75b896b6161ce14a1caa7c09e826d457155b625eeee8f2afadd37db5f4f
PartGetTransform 826c3480..826c35a8
ad60181e8e2c92d1f75b60b092d975aeb92f3166f64ea6627d7633fc827edf32
PartSetTransform 82b25480..82b25ba0
19c1db6470002ab49c7d8d10fe507958d8153899f7f6ff1cc66c5959fb95e8af
PartInitialize 82ae8fc8..82ae9058
32096d0f7dbc1bb379cd9f18c10767366ae824495516c9366690877d48371c6f
PartInitializeDefinition 82ae9820..82ae9874
e713562c048f22ec1969fd3da02a584ed230ccd3873c7a0593eb12c11d6d8e33
AssemblyInitializeComponents 82ae0bf8..82ae0df0
007c0a98a60d5a7c3535621ca32e78e94a46087b292a1a4b77a6c477307cba51
AssemblyInitialize 82ae1338..82ae13ec
0aaa886f0912d51660720587470c3f9b12fd9c169d7d6f66391073c88e3ce3fc
JointedPairInitialize 82ae8f78..82ae8fc8
19d4a6c521686622018cd2d6bfd561bc54ba6d49f3147cb08a6edd4acffe7608
RunIntersectionTests 827cbdc8..827cc410
27374f8e970c29578d0d8615a24e591c77508cec3fb9bb32d4c357fbf54be0c3
```
