# GeneralUpdate IK continuation: next source packet

2026-09-08, bounded read-only author audit following the grounded
hooks/reset implementation. No IK implementation or new parity checkpoint is
accepted by this packet. Raw caller/data/status/selected floating branches were
checked; the larger solve and contact dependencies below are identified, not
claimed fully recovered. No Rust changes or Cargo runs were made for this audit.

## Required next path

`GeneralUpdate` calls `Skeleton::UpdateIKData` at82b5bb64 and
`SkeletonIK::Update` atbb6c unless retained processed **current state+8e8** is
0x2be. It then passes the SAME mutable mapped volumes+3040 to Drives.Update at
bb7c. The current actual adapter is
`dynamic_assembly_skeleton_general_update.rs::try_update_grounded_before_ik`;
its explicit `UpdateIkData`/`UpdateDrives` continuation accurately marks this gap.
Re-running the controlled raw-pose mapping before Drives would discard IK edits.

The smallest complete active producer-to-consumer scope is the six-stage
pre-physics IK path through its actual mapped-row writes, followed by the same
owner's Drives continuation. Merely copying DataIn or updating blend scalars
does not close that path. Both contact and pre-physics solve gates default true:

- kEnableIKContactOffset reference8308e39c, default1 at82f7db50.
- kDOPrePhysIK reference8308e8dc, default1 at82f7de60.

Their values must be sampled at the actual late call sites. Setting either false
to bypass missing work is not the original normal path.

## Source identities

All spans are half-open; hashes are raw database bytes, SHA256.

| Routine | Span | SHA256 |
|---|---|---|
| Skeleton::UpdateIKData | 82b414d8..82b417c8 | 7a9f51b5e2b924d3cf4246976dc6297dd83d863d6a0f1d0992892c3eecd93fa5 |
| SkeletonIK::DataIn::operator= | 82b3b040..82b3b178 | b4d78ca070b41ca58d9753fd3ffb9e10604584cd2f7175fb8c6a581410e24bb3 |
| SkeletonIK::Update | 82b59c60..82b59cc8 | dcbb1723386b1fa87528dbad64ef40f3756edc24c4fd8eb93907785bf7e163e5 |
| UpdateStatus | 82b3db20..82b3dd44 | 9716312d32c2fb85392b03871c7b1ca9e54587397cf5ac1f07835c41a8cec1f0 |
| UpdateBlendValues | 82b51d38..82b520cc | 9d29836f958876c9d4fcf0ccf3aee51d1e644c91d771ad2a0883301256526ec4 |
| CalculateInitialPartTransforms | 82b51710..82b51d38 | bd946b97252de0bd250b99ecdeb685e5ebf4ede5eeaefdff8b0d2ba4867897b0 |
| BlendTransforms | 82b520d0..82b52e8c | 1504f1073f2b7f1bb9ee86c5621265f0f2e042ab01c269e8d2853934e50974a9 |
| UpdateContact | 82b55560..82b55bec | 8b738e8c3ba3e27f130b56814114c33a5588ac87fd79acfec9c9892674914f75 |
| UpdateDriveFrames | 82b52e90..82b5321c | 2bf748933dc6c0e003968cde852fe6baeec6799b17cc7b9644e3e0446197f29a |
| SkeletonIK constructor | 82b550f0..82b55398 | fe5ec937a3845bb4c3de546b194979c77a1702795ade9eb74be981315d5c85f6 |
| SkeletonIK::Reset | 82b51370..82b51710 | 138aafd84e1f4f04454f94a5af1e945fe7c0a160c63aa5943bb5316d29030e06 |

## DataIn: actual fields and copy timing

The 0x1b0-byte DataIn is at IK+20. UpdateIKData clears a LOCAL scratch first,
then fills it; the retained owner is not zeroed each frame. Its final assignment
at417bc copies fields in layout order, without callbacks:

| DataIn offset | Source |
|---|---|
| 000 AnimToWorldMatrix | Skeleton+2d80, current roots |
| 040 WorldToAnimTransform | Skeleton+2dc0, current inverse |
| 080 PhysicsSkateboardMtx | branch below |
| 0c0 InverseSkateboardMtx | branch below |
| 100 HipsTransformAnim | Skeleton+2d00, adjusted current hips |
| 140 SkateboardMtx | branch below |
| 180 MostVerticalToeContactNormal | actual SkeletonBody.collision+480 |
| 190 HipsOffset | Skeleton+3d70, NOT HookToHips+3d80 |
| 1a0 AnimVolumesPoseBuffer | actual mutable 24 rows at Skeleton+3040 |
| 1a4 AnimPoseBuffer | actual 24 PART-ORDER bone rows at Skeleton+3640 |
| 1a8 Island | actual SkeletonBody+24 IIsland capability |
| 1ac SkeletonAssembly | actual SkeletonBody+18 Assembly capability |

The two pose pointers are not interchangeable. Existing
SkeletonAnimationTargets.animation already retains the required part-order raw
bone matrices, although its row getter is currently test-only. It is not the
hierarchy-wide raw pose passed to current-animation target lookup.

Raw415a4 loads current state+8e8, compares exactly0x1f9 LANDINGONDECK at415a8:

- Ordinary branch copies roots.board+2f00, roots.board_inverse+2f40 and newly
  retained general.physics_skateboard+2fc0.
- Landing-on-deck computes `AnimToWorld OUTER * AnimationRecord.deck INNER`.
  It copies that result to BOTH DataIn skateboard and physics-skateboard, then
  builds its rigid inverse. The record is the existing same-owner animation
  history, not current mapped0 substituted by convenience.
- Composition executes X multiply / Y FMA / Z FMA, P starts with X FMA into
  outer P. Inverse uses arithmetic zero-P, transposed XYZ and Z/Y/X reduction.
  Raw VMX128 aliases at41664/416bc must be decoded, not trusted as printed.
- Typed AnimToWorld.P.W uncertainty flows into the two landing matrices;
  inverse reconstructs known lanes. The toe-normal W may independently be
  unknown from the actual collision XYZ producer.

All matrix work is local until assignment. Missing actual collision normal data
is a separate late read boundary at41798, before replacing retained DataIn.
Island/assembly fields merely copy their owner identities, without a null
assertion or line query. An unavailable line-query implementation must refuse
at its later contact call, not during DataIn construction. Pointer members
should retain actual borrowed owner capabilities, not cloned pose arrays or
invented island handles. This distinction was independently raw-audited.

HipsOffset+3d70 has a constructor-zero store through indexed register26 at57394;
its ctor offset load is57338. A scan of Skeleton-region direct3d70 references
found this constructor, UpdateIKData and SyncLog; no active writer was found in
that scan. This is not proof against indirect/aliased writers. It must remain a
distinct retained field, never be filled from post_physics_records.hook_to_hips.

## Update order and retained mode machine

Update calls, in order: Status59c74, BlendValues59c78, InitialTransforms59c80,
BlendTransforms59c88; then reads enable-contact59c94/98, optionally Contact59ca8;
finally DriveFrames59cb0. There is no top-level mDoIK early return.

PDB modes are Disabled0, OnDeck1, External2, ExternalLocal3. Entries run ascending
0 left toe Part15/parent16, 1 right toe Part19/parent20, 2 left hand Part3/no
parent, 3 right hand Part7/no parent. Existing target owner already has these
Init mappings and the four target matrices; do not create duplicate mappings.

Status prepares desired modes `[bool(mDoIK), bool(mDoIK), flags8c8.bit9,
flags8c8.bit10]`, then processes entries0..3:

- Disabled: external-set wins over local-set; entering either external mode
  copies TargetBlend to ExternalBlend and clears OnDeckBlend. Local entry also
  clears LocalDelta full4. Otherwise clear both blends and select desired mode.
- OnDeck: external/local transitions preserve existing OnDeckBlend. Entering
  ExternalLocal also clears full4 LocalDelta atdc78. Without
  either target, switch to desired mode only when OnDeckBlend is exactly0;
  clear ExternalBlend at that transition.
- External: losing external-set changes to ExternalLocal, with no blend clear.
- ExternalLocal: external-set first selects External and copies TargetBlend.
  A second independent test then checks !local-set && the NEW ExternalBlend==0, clears
  LocalDelta and selects desired mode. Do not turn those into exclusive else-if.
- Invalid mode values fall through without modification. Float equality to0 is
  ordered; NaN never qualifies as zero.

BlendValues fetches FOUR attributes before its first retained blend store:

| physics_skeletonik key (index0) | Payload/use |
|---|---|
| aa378f2ebf7aa313 | Vector4; add to [deckHalfWidth,0,deckTotalHalfLength,0] |
| 7ce328814f9309ae | Vector4 falloff range; add to preceding extent |
| 39fa8c07e8356284 | float OnDeck blend step |
| 094ace5de06851fc | float external-local blend step |

The attribute aliases in pseudocode are wrong ABI arguments; raw r5 is0 at all
four calls. Fallback sizes are16/16/4/4 through DefaultDataArea. Capture/read
timing must retain ordered service failures, and original payloads still need
corpus verification. Only copied XYZ extent/range scalars feed this stage's
geometry; the misleading decompiler out-pointer arguments are not native calls.

External mode asserts external-set, stores ExternalBlend=TargetBlend, then
decays OnDeckBlend with source rounded subtract/fneg/fsel. ExternalLocal stores
ExternalBlend1 if local-set, otherwise decays it, then decays OnDeckBlend.
OnDeck feet approach target0 when flags8d0.bit28, otherwise1. Hands use the PRIOR
retained PartPos: BlendValues runs before InitialTransforms updates PartPos.
Hands outside the expanded box with no positive old blend do not update.
The raw box reject branches51fc4/cc/d4 are BO4 !LT, so unordered coordinates
reject. Axis falloff uses scalar fdivs and orderedGT gates, then fsel bounds and
rounded step-limited approach. Generic min/max/clamp would change NaN/zero
selection. These selected raw gates and store order are audited; implementation
still needs the complete scalar register graph preserved.

InitialTransforms fetches Vector4 key121e67e2b4096a41 (index0/default16), then
visits entries ascending and clears each PartOnDeck before optional work.
External modes first call CalculateExternalIKOffset82b48d10. The common stage
runs for modeOnDeck OR OnDeckBlend>0 OR ExternalBlend<1. Raw518cc BO4 !LT skips
the last alternative for NaN. It uses inverse AnimationRecord.deck and retained
target transforms for deck-local targets, but AnimationRecord part pose for
initial animation/world matrices. These are different inputs. Optional parent
matrices follow each entry. Debug draw calls resolve to BLR825187c0, while real
singleton availability checks remain observable.

BlendTransforms mutates retained part/parent world matrices. It contains four
inline full-lane blend/normalization graphs plus SLerp calls5245c,52738,52a28,
52d0c, then external interpolation calls52d80/52dd0. Existing affine math is
reusable only after matching exact blend direction, inline operand graph and
typed unknown-W endpoints; this packet does not certify those whole graphs.

## Solve/contact dependencies and actual output

UpdateDriveFrames reads kDOPrePhysIK first. If enabled it reads max leg/arm
length cvars, then visits modes1/2/3 in four-entry order. For each it maps target
world transform through DataIn.WorldToAnim. A valid explicit parent (feet)
selects the parent target for the two-bone solve, while preserving the child
target separately. GetBoneMtxFromVolMtx82b3ae60 precedes reading the corresponding
PART-ORDER AnimPoseBuffer translation. No bone-id lookup belongs at this read.

DoTwoBoneIk_PrePhysics82b49950 validates selected Part then parent and uses
SkeletonData.parents to find parent/grandparent. It clamps desired endpoint
relative to the grandparent, fetches two global physics_animation attributes,
calls the recursive solver, builds two line-derived rotation deltas, and writes
mapped grandparent before parent. It then returns to DriveFrames, whose success
branch commits the selected target/optional parent mapped rows and retained
world-position adjustments. Earlier parent writes survive later failure.

Raw call53168 passes delta in r5 and output adjustment in r7; r6 occupies the
floating argument slot. Pseudocode incorrectly aliases delta/adjust. Output
scratch is initialized fullzero before the call. TwoBoneSolve is called with
booltrue and recursion/count2 (raw49cb8 and stack49cc0). Invalid result2 writes
zero adjustment and returnsfalse; non-invalid first writes the adjustment,
then asserts result0 before further matrix work. No generic IK library is an
equivalent substitute.

| Necessary dependency | Span / role |
|---|---|
| CalculateExternalIKOffset | 82b48d10..82b49534; external/local histories, ASin, SinCos, normal/position clamps |
| AddInContactOffset | 82b49538..82b49698; world target/parent offsets |
| ForceOffsetForPenetration | 82b49698..82b49760 |
| ContactLineTest | 82b49760..82b4994c; actual IIsland virtual line query |
| GetDistToContact | 82b53220..82b53398 |
| DoTwoBoneIk_PrePhysics | 82b49950..82b4a140 |
| ClampVectorAWithinRangeOfVectorB | 82e496d0..82e49854; assertions and inline numeric graph |
| TwoBoneIKSolve wrapper | 829b1e30..829b1e88 |
| _TwoBoneIKSolve | 829b1240..829b1e30; recursive core, SHA377269268ca1ea81cfd8e9a66d3b4bca4509bfa648bdb119cd827398c1988347 |
| CalculateMatrixFromLineNew | 82b3e1d8..82b3e384 |
| CreateMtxFromLineAndNormal | 82b3e030..82b3e1d8 |
| OrthoNormalizeRotation | 82b22c78..82b22d24, already accepted skateboard_spawn_math graph |

The solver core calls ACos, standalone Cos, combined SinCos and recursively
itself; exact recursive branches, degenerate cases and output-store graph remain
to recover. Existing reciprocal/rsqrt/dot boundaries, NormalizeSafe, ASin/ACos,
SLerp/interpolation and normalizer avoid inventing new generic math. Standalone
Cos must use its own existing recovered DAG, not combined SinCos's cosine.

Pre-physics solve fetches global physics_animation keysb76b5101768e856c then
a175875dec9d6db7, each float index0/default4, after the clamp/debug prefix.

Contact first clears TellAnimDontPushOrBrake, not SkateboardDontPush, then visits
only feet0/1. It consumes modes, ContactBoneID+910, flags8c4, previous contact
status/position, inverse skateboard and most-vertical-toe-normal. It updates
target/current vertical offsets and applies them before some late line queries.
Contact status is cleared immediately before each ContactLineTest call, so a
query refusal must preserve that clear and earlier transform/flag writes.

ContactLineTest uses processed EffectiveTransform+80, kSidewaysOffset, target
world P and optional debug availability. It projects out effective X then adds
signed sideways offset (+ for foot0, - for foot1), uses negative effective Y
for a segment from0.3 above the projected point to0.7 below it, and calls actual
IIsland.LineCollisionTest at498ec with the SkeletonIK label and final float0.
Hit stores contact position then VALID; miss stores NOT_VALID only. An absent
query capability must not be represented as a geometric miss. No matching
actual island line-query adapter was found in the current Rust owner path.

## Lifecycle and implementation split

The existing current_animation.ik owns only Init mappings and target transforms.
Extend one logical IK owner with DataIn snapshots, mode/blend/external/contact
history and calculated matrices. Keep the existing target arrays as the sole
target authority. Reset sets mDoIKtrue; all modes/blends/target flags/local
deltas/contact scalars reset0; matrices initialize I/J/K/P0. Full Reset requires
raw confirmation of every field store, not blanket struct-default inference.
Current teleport reset only calls reset_target_transforms and must eventually
invoke the expanded native Reset state graph at the same call boundary.

Ctor binds physics_skeletonik classc8be3349f2bd493e,
collectiond7edbd362d7d2152, clearsDataIn and callsReset before four sequential
global physicsdeck reads: d76d9db5ee1fd008 (width/2), c94d65091dcb828b
(boxlength/2),1d53bc53932190e7 (+boxhalflength),80203de9f9b732ca
(endangle). Last value is multiplied by degree-radian constant then fsel-clamped
between bits3c23d70a and3fc7c82d before XMVectorTan82581520..82581654.
That Tan helper is an additional unrecovered ctor dependency. Pseudocode
incorrectly clamps the literal degree factor; raw5534c stores the PRODUCT,
55354 reloads it before clamps. Preserve lazy global-instance/read prefixes.

External-target entry writers are Skeleton wrappers41100/411a8 (hands),
41250/41270 (world feet),41290/412b0 (animation feet),412f0/41348 (local feet),
and normal wrappers412d0/e0 to IK.SetTargetNormal3da78. Their caller provenance
still needs source audit before exposing production activation setters.
UpdatePostPhysics55398 clears normal-set320..323, external-set324..327 and
local-set328..32b only in its final55520..5554c suffix. Do not clear these at
the end of pre-physics Update just to obtain plausible frame behavior.

A manageable implementation split, with one root-owned aggregate interface:

1. Full ctor/reset + DataIn/status/blends/initial/external retained producer,
   using original Vault and current part-order/record/root inputs.
2. Full two-bone core + line/normal matrix construction + pre-physics caller,
   verified independently against raw VMX operands/branch bits.
3. Blend/contact/DriveFrames consumer with actual lazy island query, mutating
   existing animation_targets volume rows, then root's same-row Drives adapter.

Do not claim the active continuation complete until all three connect. Typed
errors may expose actual unresolved services at their source sites; they are
boundaries, not successful substitutes. Tests should cover different current
state8e8 versus8e4, landing-on-deck record selection, partial-W inverses, old
PartPos blend timing, four-mode transitions, prefix failures at each attribute/
cvar/query, two-bone invalid/degenerate/recursive results, grandparent-before-
parent row commits, and a final actual-drive consumer using the modified rows.

Confidence: high for the top-level order, DataIn provenance, audited status
machine, selected raw numeric gates, mapped-row ownership and identified
dependencies. Medium for unexpanded blend/contact/solver mathematical graphs;
no reference runtime trace or original-attribute corpus validation was performed
for this next-stage packet.
