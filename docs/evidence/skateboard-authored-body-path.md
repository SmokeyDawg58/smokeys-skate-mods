# Authored skateboard definition and live admission path

2026-09-07 read-only live IDA/PDB and original extracted Vault inspection.
Confidence **exact** for cited call order, checked keys/words and part pairs;
**high** for typed ownership interpretation. Full geometry, trig, integral and
fresh-board runtime parity is not established. No board integration is claimed.

## Actual constructor sequence

SkateboardBody constructor82b38270 retains base Body, Skateboard, owner and
owner's island. After drive/collision/debug initialization:

1. Init at82b387e8.
2. IIsland::AddSkaterBody(this,ACTIVE_BODY=4,true) at82b38808.
3. SyncCheckBody at82b38820; CalcTruckTransforms at82b38828.
4. CreateDrives at82b38830; optional CreateWheelDrives at82b388a0.
5. Mandatory CreateHookDrive at82b388a8.
6. SetTransform(input spawn transform) at82b388b4.
7. Register spy targets parts0..6 and command receiver.

Init82b374e8 initializes AssemblyDefinition with7parts, shipped6joints,0drives;
stores wheel distance from deck; calls CreateWheels82b344f0,
CreateTrucks82b33da8, CreateTriangleDeck82b32e08, CreateJoints82b2d0e8,
InitializeCullingTable82b34d98, Assembly::Initialize82ae1338 and
InitializeTransforms82b2bc10, in that order. It sets Assembly group4 and every
part userdata4, then standard/wipeout materials and zero grind materials.

Part ownership: wheels0..3 share mVolumes[2]; trucks4..5 share mVolumes[1];
deck6 owns mVolumes[0], an AggregateVolume wrapping a SimpleMappedArray.
This is not seven independently registered spheres or an arena-backed BAM asset.

## Source-ready truck extension

CreateTrucks82b33da8..82b344f0 creates one fresh capsule for parts4,5.
Radius=(wheelRadius*radiusMultiplier)*0.5 and half-height=
(widthMultiplier*wheelHalfWidth)*0.5, with separate f32 products. Attribute byte
controls enabled bit0 without replacing other flags. Per-part material reads
dynamic then static, delays dynamic dereference, stores both without the WHEEL
static/dynamic clamp, then reads restitution. Each part rereads truck mass and
world multiplier, multiplies and calls ComputeMassProperties(mass,1,null).
Debug capsule ranges after four wheels are[4,5),[5,6).

All lookups index0; missing pointer gets the correctly sized zero default area.
Labels below describe semantics, not proven original spellings.

| Collection / field | Key | Extracted original word |
| --- | --- | --- |
| physicstrucks / radius multiplier | b803dfbe2e4b56fb | 3f09999a |
| physicstrucks / width multiplier | 03edf4ad8e1741f6 | 3faccccd |
| physicswheels / radius | 12646f5e842c4db3 | 3cfdf3b6 |
| physicswheels / half-width | 5a84c435dc340af8 | 3dc28f5c |
| physicstrucks / collision enabled | ddcaedd8fa8f5529 | 01000000 (byte1) |
| physicstrucks / dynamic friction | 7e135b944834707e | 00000000 |
| physicstrucks / static friction | b34a64870f3f5944 | 3f666666 |
| physicstrucks / restitution | 329805a82950ab54 | 00000000 |
| physicstrucks / mass | 8e3a799a07260fd8 | 3ec7ae14 |
| physics_world / mass multiplier | 46507e40c85372fc | 40a00000 |

## Deck geometry and unrecovered mass dependency

CreateTriangleDeck82b32e08..82b33da4 uses signed subdivision count5 from
4bccb0af113e6953. It creates13ordered primitives: roundedbox0, longitudinal
capsules1,2, then10triangles3..12 as interleaved pairs in5iterations. Helpers:
AddCapsule82b2ba10, AddTriangle82b2b8c8, SimpleMappedArray::Initialize82acf260,
AggregateVolume::Initialize82ad6ea0. Category num=[5,5,3], start=[4,3,0] is not
three simple contiguous shape-type ranges.

Geometry keys: width d76d9db5ee1fd008; length c94d65091dcb828b; thickness
ceda721a4f0255b6; end extent49f429ae2e66f063; box enable3273d82db269c8ca;
end angles80203de9f9b732ca/00e949f6d567bb10. The routine updates/wraps aggregate,
writes material/drag/mass and calculates mass properties. It then FORCES inverse
body transform identity and present at82b33d70..84, overriding the general result.

Required general path is ComputeMassProperties82ae9a50→ComputeVolumeIntegral
82ae9878→ComputeMappedArrayIntegral82ae8028→DiagonaliseVolumeIntegral82ae9560.
Current fresh wheel sphere/capsule numeric implementation does not cover this.
Roundedbox/integral transforms and triangle bounding-box fallback must be
recovered, not replaced with plausible deck inertia.

## Transforms, joints and self-culling

InitializeTransforms writes parts6,0,1,2,3,4,5 before admission. Positions use
deck length plus front offsete02dcbd07830d3a5, rear offseta69cca4e15dba369,
vertical2be08dd7a9207622 and lateral5a84c435dc340af8. Ordinary finite semantic
positions: deck atorigin; wheels at±lateral/vertical/front-or-rear; trucks at
0/vertical/front-or-rear. Do NOT assume all rotations identity: final truck
path calls XMVectorSinCos with HALF_PI bits3fc90fdb at8225c464 and builds VMX
rotation. Exact arithmetic/lane recovery remains necessary.

physicstrucks joint switchb1cbe3e929afc8a0 is byte1. Six definition-order pairs:
deck6→truck4, deck6→truck5, truck4→wheel0, truck4→wheel1, truck5→wheel2,
truck5→wheel3. First two are locked-swing/arc-twist; wheel joints are
axle-swing/locked-twist. Actual frames/limits/trig are not generic hinge defaults.
InitializeCullingTable allocates7x7bits and sets ALL49: disables board
self-collision through the source consumer, not a null table.

## Runtime ownership and admission

Assembly::InitializeComponentsFromDefinition82ae0bf8 initializes Parts, then
JointedPairs, then DrivenPairs in their respective definition order and copies
culling-table pointer. Part::Initialize82ae9820 retains definition-backed
inertia/material/volume and optional inverse-transform pointers. Authored
definition ownership must outlive parts.

Island::AddSkaterBody827b66d0 selects contact-normal bending true→0.99900001,
inserts SkaterPool, assigns island, next dynamic-body ID, and dynamic-list entry.
AssemblyPool::Insert82e0dbb0 increments manager object ID/world index, calls
Assembly::AddToSimulation, publishes assembly, creates serialized collision
storage if pool supports it, copies its bounds (else zero), increments count.
Native ignores AddToSimulation failure here; checked host transactional refusal
must not be presented as native failure-path equivalence.

Current SerialisedAssemblyTemplate requires arena ownership. Backend
try_bind_part_attachment_template rejects nonzero joints/drives and requires
arena-routed inertia; general dynamic templates require clustered-mesh owners.
Needed integration is an actual runtime-authored definition/volume/inertia route,
reusing Part/body/joint/drive pools and collision preparation. Fabricated BAM,
asset IDs or just widening an app caller cannot establish that provenance.

## Drives and mandatory hook

CreateDrivesWithOffset82b37b88 creates deck6→truck4, then deck6→truck5. Setup:
CalcTruckTransforms82b2c390, SetTruckDriveFrames82b34fa8,
SetTruckDriveDynamics82b2cb08, CreateDrives82b37ea8. Additional attribute class
physicstrucks_drives6599909c97c57a89/default; fresh mForceTurn=false and shipped
physicsdeck103fdc19514d4246 istrue. Coefficients remain source work.
Optional four wheel drives SKIP for physicswheels69cef3ca3f1480b0 byte0.

CreateHookDrive82b2e4b8 ALWAYS creates separate one-part assembly: collision
box dimensions0.1/0.1/0.1, collision disabled, mass100/density1, group/userdata1,
HookPool insertion as STATIC_BODY, drive deck6→hook0. This is the real producer
for the previously recovered HookPool island exclusion. Minimum shipped fresh
graph:7board bodies+6joints+2truckdrives PLUS1static hookbody+1hookdrive.

## Retained evidence and next step

SHA-256: CreateTrucks6fe738fe6f8b28f85be7ea0427399c20e45a97c5eb536574a38782c125d38483;
CreateTriangleDeck5b20bc1dff54a2ae16b7be8042bbb0cc0b0590a2d67ffa0c0d268233952adb62;
CreateJoints8b5671cca5ac70f596bb9508fb6880ae926f348fce0d8b83e98cbdd09963565d;
InitializeTransforms90283702c0830dbefede4f2832928f6d7a02bc6e273f10e0cebf6d81e4d485aa.

Original words above were read using existing inspector over the documented
pre-relocated extracted VLT/BIN. Raw mounted truck/deck equivalence and fresh
board runtime traces were NOT rerun. Next bounded source work: the exact deck
integral/mass path, then geometry/joint/drive arithmetic and authored admission.
The truck extension is reusable but must feed this connected owner target.
