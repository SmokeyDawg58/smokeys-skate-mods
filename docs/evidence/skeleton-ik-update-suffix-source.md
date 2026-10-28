# Skeleton IK Update suffix and processed-input ownership

2026-09-08. Read-only recovery from original IDA bytes, decoded
VMX128 operand extensions, PDB names/layouts and actual callers. No Rust edits,
Cargo execution, world-query implementation or completed native scheduling is
claimed. Independent source review is a separate gate.

## Source identities

Ranges are half-open; SHA256 covers the original bytes.

| Function | Range | SHA256 |
|---|---|---|
| SkeletonIK::Update | 82b59c60..82b59cc8 | dcbb1723386b1fa87528dbad64ef40f3756edc24c4fd8eb93907785bf7e163e5 |
| enable-contact cvar initializer | 82f7db28..82f7db74 | 0fad4fc8f92ab0e18cee33f7f9125d41bf6ac3ff5ee3e49530ce773607fa0e5e |
| Skeleton::UpdateIKData | 82b414d8..82b417c8 | 7a9f51b5e2b924d3cf4246976dc6297dd83d863d6a0f1d0992892c3eecd93fa5 |
| Skeleton::GeneralUpdate | 82b5ba88..82b5bc58 | 6f9035906e5fc80631410e91625af3b7369602c7bb4c51e6b110cfdb0e8f8693 |
| SkeletonIK::FillPhysOut | 82b56590..82b5661c | 56fbbde015d1763a38d9c685a0b7973860a94dc03e228b54e679a510e02be25a |
| Skeleton::RequestPushExit | 82b3bb70..82b3bb80 | 9a988afef3c4cbec7e1c3064ee3559f8a4b6830587b430b2b3bcc8743e7ff4b5 |
| Skeleton::ResetPushAbort | 82b3bb80..82b3bb90 | d0e0f1b86347037a702a109ee364d7745c4af7316eaed3d3c5528d9fbee00a30 |
| ProcessedPhysIn::Reset | 82b39098..82b395a4 | 6f8a1ad35ad9291fbebeb71065b46f74559a2818923f73a9f4e61dccbcbce20b |
| ProcessAnimAttributes contact prefix | 82b5a418..82b5a4b8 | a8eb72db2e8bba88d557ae25c7ac1f4f1ebadf40ab30e014d76c5cc53d6acba2 |
| Skateboard::PrepareToolkitInput | 82b26910..82b26df0 | 7e2bf85211c060935bcde2c34727eaa2d406658304debc8a7ef354659f6b2003 |
| Skateboard::GetEffectiveTransform | 82b23778..82b237f8 | c798e843be385a3942929c594e5ac418f534651ea0e1c2c85d3864313e1787f7 |
| Skeleton::GetEffectiveAnimToWorldTransform | 82b3ba48..82b3baf8 | 718314771b6e61dc1bdd468856c8c0761efa42eeca5eda715eb26666f8472973 |
| Conditions::DisablePushBrake::ActivationFunc | 82c9a8f0..82c9ab24 | 07449092f033096a3c0bc7d6ed440f8d8a5304e0cbad0b04a694a14c1bf160e4 |

Symbols are the named `Sk8::Physics` members except `Sk8::Conditions` and the
dynamic cvar initializer. Existing detailed packets cover
[contact](skeleton-ik-contact-source.md),
[the line-query boundary](skeleton-ik-line-query-source.md), and
[General drives/force](skeleton-general-drives-next-source.md).

## Complete top-level ordering

`SkeletonIK::Update` calls, in order:

1. UpdateStatus at 59c74.
2. UpdateBlendValues at 59c78.
3. CalculateInitialPartTransforms at 59c80.
4. BlendTransforms at 59c88.
5. Load `kEnableIKContactOffset` ref at 8308e39c: target pointer +20 at
   59c94, then actual bool target+cd at 59c98. If nonzero, UpdateContact at
   59ca8; otherwise skip that entire call.
6. UpdateDriveFrames at 59cb0, unconditionally after the optional contact.
7. Epilogue 59cb4..59cc4; return. There is no further IK stage.

There is no top-level DoIK early return. A disabled contact value does not
clear its histories or either cached feedback flag. An adapter must read the
cvar at this late site; failure retains completed status/blends/initial/blend
prefixes and prevents contact and DriveFrames.

The initializer uses section `Physics_Skeleton` at 82266148, label
`kEnableIKContactOffset` at 82266130 and title `Enable IK Contact Offset` at
82266114. It passes default true at db50 and null update callback at db4c to
the cVarBoolRef constructor db54. The default is evidence, not permission to
replace the runtime read with a constant.

GeneralUpdate freshly reads processed current state +8e8 at bb54. State 0x2be
skips both UpdateIKData and IK.Update. Every other state calls UpdateIKData at
bb64, then Update on the same retained Skeleton IK pointer +3dfc at bb6c.
The immediate next operations are mapped-row pointer Skeleton+3040 at bb70,
retained proxy-enabled byte +3e43 at bb74, same Drives owner +1934 at bb78,
and Drives.Update at bb7c. There is no feedback getter, remap, pose rebuild or
final IK operation between these calls. The existing gravity/all-Parts-force
suffix then runs on actual Body +1930. Grounded NextTrajectory reset belongs
to the already recovered outer continuation after GeneralUpdate.

Full raw UpdateIKData likewise contains no reads of IK+c70/c71. It prepares
its local DataIn and commits it at 417bc. Island/assembly pointers are copied
from actual Body at 417ac..b8; contact availability must remain a later query
boundary. The collision normal read at 41798 is a distinct actual input read.

## Cached feedback belongs to later publication

| Retained IK field | Native offset | Actual Rust authority |
|---|---|---|
| mSkateboardDontPush | +c70 | SkeletonIkState::skateboard_dont_push |
| mTellAnimDontPushOrBrake | +c71 | SkeletonIkState::tell_anim_dont_push_or_brake |

Reset initializes both false. Contact clears only TellAnim on entry and can
set both true. The existing readonly Rust getters observe this retained state;
they do not imply an extra native UpdateIKData or GeneralUpdate operation.

`SkeletonIK::FillPhysOut` first calls FillPhysOut_HandGrabInfo at 565a4 and
FillPhysOut_FeetDeckInteraction at 565b0. It checks PhysOut+4c is not locked for
write, asserting at 565d4/DebugBreak d8 if locked. It then loads IK+c70 at
565dc and stores it to PhysOut's Skeleton object (+14 pointer), member +241,
at 565e4. It rechecks the lock at 565e8, with assertion 56600/04, and only then
loads IK+c71 at 5660c and stores SkeletonOut+242 at 56610. A second lock
failure retains the first flag store and previous hand/feet output. No source
flag is reset by publication. `Skeleton::FillPhysOut` calls this at 82b594e4,
after its Hips Part transform read and before its later angle calculation.

The PDB distinguishes SkeletonOut+240 `mAbortPush`, +241
`mSkateboardDontPush`, and +242 `mTellAnimDontPushOrBrake`; these are not
interchangeable flags.

Two separate state-schedule producers also exist:

- RequestPushExit writes only IK+c71=true. PhysState_PhysicsGround::Update
  calls it at 82da1038 when that state's mRequestPushStop+f2 is true, after
  UpdateSkateboard and UpdatePredictedSkateboardPos.
- ResetPushAbort writes only IK+c70=false. PhysicalPlayerHiLOD::Update3_State
  calls it at 82dfe78c, after predicted-deck preparation and before actual
  state Update. It is not a Contact/General entry reset.

The next PhysicalPlayerHiLOD::Update1_ProcessInput reads the existing
PhysOut Skeleton+241 at 82e02e8c after its output lock check and inserts it
into processed flags+8c8 mask 0x800 at 2e94/98. It reads the published output,
not the current IK object. Animation's `DisablePushBrake::ActivationFunc`
reads SkeletonOut+242 at 82c9aaec after the unconditional PhysOut getter at
82c9aad8, unless the saved angle condition succeeds ataae4. A nonzero value
makes the condition true. That
function obtains actual IPhysical and its PhysOut through the object interface;
its earlier Vault/ASin angle calculation remains outside this integration.
No early direct flag handoff should replace either retained output consumer.

## ContactBoneID is produced from the animation hierarchy

Native processed+910 is a signed hierarchy bone ID. ProcessedPhysIn::Reset
stores zero at 82b39500 using r30, which was set to zero at 82b391ac. In an
Option representation this reset is Some(0), not None. A later failed
hierarchy lookup can produce -1/None.

Skeleton::ProcessData calls ProcessAnimAttributes at 82b5ed44. The relevant
ordered attribute loop reads the signed type byte at attribute+35 and accepts
type 3. It reads flags byte +34 and requires `(flags & 0x0c) != 0`. Payload
bone name is the five packed words +0..10; transition scalar is +14. The
attribute name is a different five-word value at +20..30.

For every accepted type-3 record, 82b5a4ac loads the SAME Skeleton hierarchy
pointer +44, 82b5a4b0 calls HierarchyData::GetBone(82590fc0), and 82b5a4b4
stores the returned signed index directly to processed+910. This store occurs
BEFORE attribute-name tests, including push_contact. GetBone compares the
packed five-word name in ascending hierarchy order, returns the first match,
and returns -1 on absence; there is no new assertion at this caller. The loop
advances by 0x40 at 82b5b77c and branches back at b784, so the last accepted
type-3 record wins even if its attribute name is unrecognized.

Push/brake processing then populates the flags used by Contact: push present
0x08000000, push started 0x02000000, push ended 0x01000000; brake present
0x10000000, brake started 0x40000000, brake ended 0x00800000. The Contact
packet gives the exact initial versus late read sites. The matched IK IDs are
the existing `SkeletonIkTargets::bone_ids()` (+204), not target Part IDs
(+460), collision contact Part enums, or an assumed foot ordinal.

Existing `physics.rs::ProcessedAnimationIntent::apply_contact_attributes`
retains the last contact bone NAME and push/brake flags from an ordered
semantic ContactAnimationAttribute list. It does not yet perform this actual
hierarchy-ID store; its input type has no raw +34 active flags. Reuse the
actual animation handoff/hierarchy instead of introducing a second contact
table or treating semantic unfiltered records as proof of raw acceptance.

## EffectiveTransform is a physical-board processed snapshot

PDB ProcessedPhysIn distinguishes +40 DeckTransform, +80 EffectiveTransform,
+c0 EffectiveAnimTransform and +100 InverseEffectiveTransform. Contact reads
+80. The existing Skeleton root/animation matrix is not this field.

Skateboard::PrepareToolkitInput retains its actual processed pointer +110.
It first reads actual Body mass and stores processed+970. It retrieves actual
assembly Part 6 through Part::GetTransform at 26944, and commits the four
DeckTransform columns to processed+40 at 26964/70/78/80. It then makes a
SECOND actual body-transform read by GetEffectiveTransform at 26984. The
four returned columns commit to processed+80 at 269b4/c0/d0/e0. A later
failure must not erase earlier mass/DeckTransform stores. The remaining
PrepareToolkitInput also prepares inverse/axes/other data; this packet does
not claim its full implementation.

GetEffectiveTransform calls Body::GetPosition(6) at 2379c, then reads the
current processed flags+8c4 at 237a8. If board-flipped mask 0x00100000 is set,
it XORs sign bit 0x80000000 into ALL FOUR lanes of X and Z at 237cc/d0 and
237d8/dc. Y and P are retained. It does not normalize, subtract a root or use
processed mirrored flag +8cc. This preserves signed-zero/NaN payload sign
behavior and W knowledge of the actual source transform.

The source schedule is Update1_ProcessInput: processed Reset at 82e0200c;
ProcessAnimOut at 82e02134 (including board-flipped input); PrepareToolkitInput
on actual board at 82e036cc; FillPhysIn_SkeletonLineTestResults at 36d4;
Skeleton::ProcessData at 3718. Thus the physical effective transform is
captured before ProcessAnimAttributes/current animation and remains a
processed input for later General/Contact. A fresh late board query inside
Contact changes that sampling boundary.

By contrast GetEffectiveAnimToWorldTransform copies retained Skeleton
AnimToWorld+2d80 and, if processed mirrored+8cc mask8 is set, multiplies its
full X/Z by -1. Update1 calls it at 82e0359c and stores to processed+c0 at
35b0/b8/c0/c8. This is a separate producer, field and flag.

## Smallest coherent runtime connection and remaining limits

The actual physical Part6 getter already exists through
`dynamic_assembly_skater_pool_roots.rs::LiveBoard::transform` and
`SkateboardAssemblySession::try_live_part_transform_host`. Its attached-body
canonical-W host boundary remains explicit; this packet does not recover new
packed-native W. `ProcessedPhysicsInput` already retains board_flipped, but
does not retain this complete physical EffectiveTransform or resolved contact
bone ID. `ReckoningProcessed::effective_z` is only a projection. None should
silently substitute for an absent processed owner field.

Root can connect the same retained SkeletonIk state through the existing
DataIn, Initial, Blend, Contact and DriveFrames stages, with a late enable
callback. Borrow actual mapped rows and existing targets; on success continue
existing Drives and force. Contact's borrowed processed interface should read
the actual producer's flags, signed hierarchy ID and retained physical
effective transform. The query service must still resolve the real retained
island/World at the actual line-query site and distinguish hit, miss and error.
No unavailable-world-to-miss conversion or fabricated query result follows
from this packet.

Full PhysOut publication and Update1/Update3/animation feedback scheduling are
separate outstanding paths. Preserve their retained flags now; do not insert
their effects into the middle of General. Controlled component tests may
prepare explicit source inputs, but must identify that schedule boundary.

Confidence: exact for the raw call/branch/store sites, cvar identity and
literal default, index namespaces and physical-versus-animation transform
distinction. High for the proposed mapping onto existing Rust authorities.
Full native phase timing, live line traversal and deterministic runtime parity
are not established by this source packet. No Cargo or original corpus run
was performed for this read-only task.
