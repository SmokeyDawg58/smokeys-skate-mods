# Retained SkeletonBody geometry and mass definition

2026-09-07. Confidence **high** source reconstruction; original payload words
**exact**. This is a concrete24-Part definition producer ending before
InitJoints, not a live humanoid, pool admission, or native floating-point trace.

## Source and ownership

The root independently reviewed source/implementation and integrated the body
producer. Separate authors recovered hat geometry and the mass-state
helper; an independent reviewer checked all production components.

| Symbol | Address | Ownership/call |
|---|---|---|
| `?Init@SkeletonBody@Physics@Sk8@@AAAXAAVSkeletonState@23@@Z` | 82b61738 |24Part loop, shapes, materials, ragdoll then animated caches |
| `?CreateHatVolume@SkeletonBody@Physics@Sk8@@AAAPAVVolume@collision@rw@@AAVSkeletonState@23@W4PartIndex@SkeletonDataBase@23@AAM@Z` |82b4fae8 | Neck1 nonzero-hat branch, two-child aggregate |
| `?ComputeMassProperties@physics@rw@@YAIAAVPartDefinition@12@MMPAM@Z` |82ae9a50 | Called twice per Part, density1 |
| `?ComputeCylinderInertia@physics@rw@@YAMMMMAAVVector3@vpu@math@2@@Z` |82ae81d0 | Simple Cylinder branch at82ae9518 |
| `?DebugMultiplyInertiaTensor@PhysicsUtility@Physics@Sk8@@YAXAAVPartDefinition@physics@rw@@M@Z` |82e4b0e0 | Generic physics mode1 |
| `?DebugMultiplyInertiaTensor2@PhysicsUtility@Physics@Sk8@@YAXAAVPartDefinition@physics@rw@@M@Z` |82e4b188 | Mode2 |
| `??0physics_hat@Gen@Attrib@@QAA@_KI@Z` |82b3e6a8 | Cached48-byte generated layout |

Source function SHA256s:

```text
BodyInit: 41afa55915339a9393fe199df6c138c243d4a18e2f08140867f444b15b1d877e
Hat82b4fae8..ffd4: fd6a093fda92509d7a1c09d8f9e8e134edcdbe70e5e103a48436cd48945831cb
Mass82ae9a50..9df4: 89453978a60750fc1440543803d5b2a1125faa28b7adf7f7dbdede87601f53bd
Cylinder82ae81d0..8270: 166b7674d4394cbb1a1688900f12473440bc01154f57a8e608d399e37bde727f
Debug1: 6fb6c7572cfdb819727151fcd0f806a5cd6fc58bb64aac9e7860f0baba60429d
Debug2: 1967743541f1dd731f21a91d7bf7f4cccf6549e5dfa3945617f2b97797f40aed
HatCtor: 3ac18dfdcb3899fbb0da385a6363c6804cdc9469910f710f9bc3db5e1ad80bda
```

New `skeleton_body_definition.rs` retains Arc<SkeletonData>, the actual
InitAttribData snapshot,24Part geometry owners, both packed4 caches, material,
unscaled state mass, effective mass, debug shape, and source joint allocation
choice. Primitive Volumes contain actual geometry. Hat retains a real
two-child array, bounds, and separate outer metadata: no fabricated aggregate
ID/pointer in a serialized shape union. The constructor has no admission caller
yet. Existing board/session/solver arithmetic was not replaced.

## Source chronology and shapes

Preloop: localBones capsule factor; globalAnimation byte chooses22/0 joints;
globalWipeout inverse-mass numerator must compare strictly greater than0
(NaN fails). Then Part0..23. HatPart1/nonzero branch precedes root comparison.
Root is matching saved Part0 ANIMATION BONE ID, not ordinal0 only; duplicate
matches overwrite skateboardPart each time. Root is a disabled capsule with
effective mass bits3a83126f, state PartMass+0; half-length lookup precedes radius.

All other ordinary PartMass values are `(sizeX*sizeY)*sizeZ`, never collision
volume or volume-scaled dimensions. Full size W survives Box debug storage but
is excluded from mass. Local Skeleton mass scale multiplies this raw mass.

* Box: half4=size4*(VolumeScalar*.5); actual radius0.
* Sphere: half4=size4*.5; fsel(halfX-halfY,halfX,halfY)*VolumeScalar.
  Z excluded, unordered selectsY; subtraction tie/signed-zero preserved.
* Capsule: baseRadius=(sizeX*preloopScale)*.5, radius=baseRadius*earlyScalar.
  remainder=.5*sizeZ-baseRadius; fsel(-remainder,+0,remainder). Then fresh
  GLOBAL Skeleton length-factor lookup, multiply by remainder, reread local
  bone VolumeScalar, multiply again. This differs from clamping scaled radius.
* Unknown ordinary shape refuses with a typed host error, no substitute sphere.

Fresh Volumes have I/J/K axes W0, translation FOUR zero words, group0,
surface0, enabled1 except disabled root. Inactive union storage is canonical
host storage, not an assertion about unwritten native bytes. Bone.has_collision
does not gate this construction stage.

Common material installs normal DYNAMIC friction into BOTH coefficients and
normal restitution. Effective mass strict-positive assertion follows those
stores. Ragdoll supplied mass=ragdollFactor*effective; cached inverse mass is
global numerator divided by this value, before mass calculation. Store full4
inverse tensor. Then animated supplied mass=animatedFactor*effective and cache
1/mass before a SECOND mass calculation. Aggregate principal calculation also
runs twice, not reused. Hat then forces inverse frameI/J/K/P0 with flag1.

Create a FRESH generic physics.default Instance per Part. Mode is u32; fetch
factor only for1/2, apply debug modifier, store animated packed4, destroy
Instance. No per-body cached mode or default-off tuning assumption.

## Mass and numerical boundaries

Mass frame uses actual primitive transform or diagonalized aggregate frame.
Threshold=((diagY+diagZ)+diagX)*bits358637be; dot3(P,P) NOT_LT threshold
(equality/unordered included) constructs inverse. Otherwise any absolute XYZ
axis difference from identity >bits3a83126f constructs it. W ignored in this
comparison. False flag preserves previously written inverse storage.

Inverse is rigid transpose XYZ/W0, not a general matrix inverse, followed by
arithmetic +0-P and Z multiply -> Y FMA -> X FMA, INCLUDING translationW.
For positive P the translationW is -0; basisW remains+0. Test initially expected
all+0, failed, and was corrected after independent raw-source confirmation.

Mass suffix reuses audited unit kernels and density-one application: only
mass<MIN_POSITIVE selects volume; Y/X/Z reciprocal seeds, two interleaved fused
refinements, inverse tensorW=X, source later-on-tie/unordered spherical minimum.
Raw inverseW is retained separately from existing solver's XYZ inertia.

Debug1 uses a distinct factor reciprocal estimate and two fused refinements,
then reciprocal*oldPacked4; Debug2 value=(1/(factor*3))*((X+Y)+Z), XYZ=value,
W explicitly+0. Both recompute spherical; neither changes inverse mass.

Cylinder simple dispatch succeeds (IDA-rendered jump-table annotation was
misleading). Native fields: +44 halfHeight=shape_words[0]; +48 innerRadius=
shape_words[1]; +50 outerRadius=Volume.radius. Initial hat author reversed the
two union words; root/raw independent review caught and corrected it before
accepted corpus. Let R=inner+outer,H=outer+halfHeight. RadiusSquared=R*R,
heightSquared=H*H, product=H*R, transverse=(heightSquared FMA4,
radiusSquared*3)*bits3daaaaab, axial=radiusSquared*.5,
volume=(product*R)*bits40c90fdb. No clamp/bbox fallback. Construct/transform
integral exactly once per child even if disabled.

Host sin_cos, reciprocal estimates, dot reduction, scalar division, denormals
and exceptional NaN payload behavior remain explicit numeric boundaries, not
Xenon bit-parity claims. Native unsafe allocation/assert failure prefixes are
typed host refusals without publishing partially constructed bodies.

## Hat data and selection

Hat enum None0,Helmet1,Beenie2,Crochet3,FrontBrimOnly4,FullBrim5,MAX6.
GetSkaterHatType82cfe9e8 fetches34418cad1c23bc94. Original tutorial_male
2275b90050166969 / characters_marquee91bca6693efc9aa7 VLT35940 has no own
nodes; parentdefault35330 node353c0 inline+8 is0. Actual ApplyPlayerAttributes
828158f0 calls getter828159d4 and SetHatType82815a08. Presentation Hat component
does not imply physics presence; later CAS/profile choices remain separate.

Body ctor82b622fc..62334 selects physics_hat class03df76a47692efff collection
3e30b6a373a58559 for4; otherwise2d6431310b6e1662. Cached48 bytes:
PosOffset full4@0,OrientationOffset full4@10,Thickness@20,Radius@24,padding8@28.
Missing cache uses independently verified zero DefaultDataArea48.

```text
Other BIN9ccb0:
3CF5C28F00000000BCA3D70A00000000
3E99999A3FCCCCCD3E80000000000000
3CA3D70A3E3851EC0000000000000000
Front BIN9cce0:
3CF5C28F3DE147AE0000000000000000
3CF5C28F3FCCCCCD3E80000000000000
3CA3D70A3DB851EC0FCFC1285FF9B2EB
```

First live enable sample precedes cylinder/trig; second independent sample
changes only its enabled bit after debug stores. Caller computes raw/effective
mass, then sphere radius from the actual bone; finish creates Sphere child1,
updates bounds including disabled Cylinder child0, and constructs outer owner.
Native cvar8308e96c compiled defaulttrue (initializer82f7d2e8,constructor82f7d314);
runtime must supply actual samples. Tests deliberately change true->false.

Hat Euler has unnormalized axisW=X, not zero. PosOffset full4 is stored before
X/Z/Y. Three ordered sin_cos calls; positive mixed terms are fused, negative
terms separate products/subtractions. The exact graph lives in skeleton_hat.rs;
a 2^-46 discriminator distinguishes fused from rounded multiply-plus-add.

## Original-data validation

New ignored corpus composes the same fingerprinted original OnBoard
PhysicsParam + controlled SkaterBodyPres RAW pose described in
skeleton-data-and-attributes.md, actual retained Vault reads, and all24 parts.
It tests selected None0 and both real hat layout variants4/1. This is NOT a
live caller pose, full SkeletonBody lifecycle, or rendered simulation frame.

Independent raw VLT node+8 words (default collections):

| Route/key | Node | Bits |
|---|---|---|
| Bones07204f10dd937907 |5e050|3f99999a|
| Skeleton5fe9f585a20162a2 |5dd20|3f99999a|
| Skeleton9f4a03c75637e7e9 |5ddc0|43a00000|
| Skeleton5e43a9ebd2bebbde |5de40|3ecccccd|
| Skeleton7e90113b05bacda6 |5de50|3eae147b|
| Wipeout3689013d7adbdd2a |5f520|3f800000|
| Animationdff972080e90f82f |5bdc0|byte01|
| Physics6ddd9384be03a6ea |88a0|00000001|
| Physics425c501754441f41 |8890|40a00000|

Thus this supplied debug build really enables mode1/factor5. Physics debug
classbabb3ec110849e08 is distinct from generated gravity class04293d0f2adcfbb4.
No hardcoded runtime tuning was added.

Independent host-rounded raw/effective/inverse-mass oracle checks parts0,1,2,3,23.
Selected radii3eae147b,3e0c63f1,3d892253,3d70d842,3e2fb00a; raw masses
0,3babf89e,3a4d9528,39b0563b,3c1fc9f9. All ordinary selected parts except root
are spheres. Fixture sourceW ca03126e must not enter mass.

The independent oracle's raw inputs are retained here for reproduction; no
printed production inverse-tensor output was promoted into an expected value.
OnBoard bone records begin2e40, stride112, size at+32:

```text
Part0 size@2e60: 3e79096c 3ca3d70a 3f608312 ca03126e
Part1 size@2ed0: 3e3ac70f 3e3b2fec 3e212d77 ca03126e
Part2 size@2f40: 3d6bedfb 3de48e8a 3df9db23 ca03126e
Part3 size@2fb0: 3e168729 3ca3d709 3dea4a89 ca03126e
Part23 size@3870: 3e305531 3e38ef33 3ea0902d ca03126e
```

Mapped bone rows0/1/2/3/23 are4/13/14/19/23; animated factors100/4/4/4/32,
ragdoll factors1/1/1/4/1, volume scalars1/1.5/1.2/.8/1.9, with original words
3f800000/3fc00000/3f99999a/3f4ccccd/3ff33333. Source scalar grouping above
with f32 rounding after each ordinary product derives the test's expected
radius/raw/effective/inverse-mass words independently of its implementation.

25 new ordinary tests:7body,10mass,6hat,2Cylinder. New seventh ignored core
test covers original body/hat producer. Full strict gate recorded in RESEARCH_LOG.

## Next required producer, not an admission shortcut

InitJoints82b464f8, normal/ragdoll culling, then source assembly creation,
group/userdata1, active4 insertion, and SetUpNormal82b50508 changing state1->6.
Normal setup mutates actual live inertia/material/joint authorities AFTER
insertion/Create. Current immutable board pool binding is not a substitute.
Do not preinstall final state before insertion or add a fake second member.

InitJoints angle floor was corrected in wipeout-offboard.md: compare
product-epsilon, select originalproduct or epsilon0.01; NOT max(product-.01,0).
Global Animation emission gate must remain separate from preallocated22/0.
Then actual two-Part skeleton hook/drives, player/Actor, input histories and
original controls, pairs/World scheduling, camera/rendering remain open.
