# Same-owner ordered animation attributes

The continuing Skeleton ProcessData call at 82b5ed44 invokes
ProcessAnimAttributes82b59de8 with its retained ProcessedPhysIn and Physical
AnimOut pointers. `physical_player_skeleton_anim_attributes.rs` consumes the
actual Actor-published AnimOutPhysIn records in vector order. It does not use
the detached `physics.rs` ProcessedAnimationIntent summary.

Entry82b59e18..48 requires the actual DebugOverlay singleton before the input,
dt and vector range loads. Root independently reconstructed its concrete
one-byte allocation at Construct8272b300, called from InitApp827352b0; there
are no DebugOverlay member constructors. The requirement remains present for
an empty vector. This does not claim a renderer or invent enabled debug draws.

## Record layout and ordered behavior

Fresh PDB Andale::Attribute: payload +0[24], begin+18, end+1c, name+20[20],
flags+34, signed type+35, sequence+38, stride40. Raw82b5a464/46c/474/47c/484
loads the bone payload words at 0/4/8/c/10; transition is at14. GetBonea4b0
stores its result at Processed910/a4b4 before testing event names. Existing
raw helper and its deliberately poisoned event-name regression were already
correct. Serialized archive records have a different header and aligned
payload; Andale parsing and PhysicsAnimationAttribute publication preserve
that distinction. No helper or fixture payload offset was changed.

The accepted type3 flags mask is0c. First matching hierarchy name wins; absent
names write native -1 (the retained optional bone index). Push sets8c4 bits
08000000,04000000(right mapped physics part19),02000000(start transition1),
01000000(end transition-1), then unconditionally computes trajectory speed.
Brake sets10000000/40000000/00800000. Hand grabs set8c8 bit400 for payload
`righthand`, otherwise200, regardless of which hand event name matched.
All contact string initializer literals were checked at82f7c9f8..ca58.

Type0 dispatch ignores flags and does not sample payload for name-only flags.
The implementation preserves each record's store sequence (notably NoInput
writes PhysicalAnimOut before Processed8c8), last assignment wins, OR fields
accumulate, and release-board replaces only8cc mask6000 with4000. Unknown
attribute names and other types follow the original no-effect path. Each
referenced XAtt static initializer was independently checked to assign exact
literal X. All comparisons retain packed FastString case folding.

## Retained scalar destinations and Reset

Each new field below belongs to the SAME Processed owner. PDB names/offsets
were checked against actual stfs instructions; all listed Reset stores write
f31=positive zero. New/unconstructed fields remain unknown until Reset or a
real record writes them.

| Retained field | Native member/offset | Reset store |
| --- | --- | --- |
| push_speed | mAnimPushSpeed940 | 82b39408 |
| body_spin | mBodySpin960 | 82b394dc |
| turn | mTurn980 | 82b39468 |
| offboard_turn | mOB_Turn984 | 82b39470 |
| look_x | mOffBoardLookAtX990 | 82b39484 |
| look_y | mOffBoardLookAtY994 | 82b3948c |
| raw_turn | mRawTurn998 | 82b39494 |
| hard_turn | mHardTurn99c | 82b3949c |
| foot_brake | mFootBrake9a8 | 82b394e4 |
| move_object_z | mMoveObjectZ9dc | 82b39544 |
| move_object_x | mMoveObjectX9e0 | 82b3954c |
| move_object_rotation | mMoveObjectRot9e4 | 82b39554 |
| grind_translation | mGrindTranslation9e8 | 82b3955c |
| grind_nudge | mGrindStabilityNudge9ec | 82b39560 |
| grind_up_down | mGrindUpDown9f0 | 82b39564 |
| grind_loose_end_height | mGrindGrabLooseEndHeight9f4 | 82b39568 |
| physics_body_spin | mPhysBodySpin9f8 | 82b3956c |
| world_grab_y | mWorldGrabY9fc | 82b39570 |
| world_grab_z | mWorldGrabZa00 | 82b39574 |
| wipeout_x | mWipeoutControlXa04 | 82b39578 |
| wipeout_y | mWipeoutControlYa08 | 82b3957c |
| offboard_jump_x | mOB_JumpXa14 | 82b39588 |
| offboard_jump_z | mOB_JumpZa18 | 82b3958c |

Repository retained-owner search found no existing Processed fields for these
23 destinations. The older physics.rs summary has similar names but is not
the current retained owner. Actual existing destinations are reused instead:
balance9a0, slide9ac and startRevert9b8 in physics_state; spin97c, strength950
and adjustments954/958 in jump_events; Skeleton adjustment3e34/3e38 in the
same Normal assembly; PhysicalAnimOut grind_name and flags; Processed flags
8c4 in jump_events,8c8 in contact,8cc/8d0/8d4 in physics_state. HippyJumping
now writes/reads actual8d0 bit8000, removing the earlier duplicate bool. An
unknown whole8d0 word refuses its read-modify-write explicitly rather than
inventing the other31 bits; the real Reset establishes the word.

## Genuine callbacks and suffix

PrepareJump stores8c4 PREPARE before actual Actor.GetInput; it captures that
nullable gameplay owner once and reads74 then75. The post-loop jump endpoint
uses the Skeleton constructor's retained IActorQuery pointer, not a later
replacement of the enclosing physical owner's query slot. The endpoint
logic uses the same retained owner.jump_attributes Vault, acquired lazily at
each source capture/lookup. Cached Skeleton adjustment reads or fresh captured
input74/78 follow the actual retention attribute. The old caller-selected
Human/AiNull fixture is not used by this production adapter.

Revert records retain local presence and direction; after all records, the
presence gate stores9b8 at82b5b798. The existing typed jump finisher follows,
then living-world impulse sets8c4 Wipeout; actual input locomotion bits27/25/28
insert8d0 bits6/2 and8d4 bit24. Existing retrieve/drop bits clear8cc GrabWorld
only after these stores. Error returns preserve the executed source prefix.

DangerZone sets8c8 bit10000 before live kShowDangerZone. Initializer82f7c648,
table82fd4558, reference8308e27c: Animation/kShowDangerZone, titleShowDangerZone,
defaultfalse and null callback. Enabled call82b5af48 targets825187c0, whose
raw instruction is BLR. Both outcomes converge with no omitted draw call.

Push numeric raw82b5a5e4..620 was independently decoded by root: dot3 is
trajectoryP with itself, despite misleading IDA extended register aliases.
The explicit numeric collaborator supplies dot3 and reciprocal-sqrt estimate.
Two refinements use square=r*r, half=r*.5, residual=-fma(squared,square,-1),
r=fma(half,residual,r). Length=squared*r; squared==0 selects positive zero
AFTER unconditional estimate/refinement, then scalar(1/dt)*length. Even an
ended contact executes this graph before storing final positive-zero speed.
The shared host collaborator's dot reduction and estimate policies remain
explicit; this does not establish exact Xenon estimate/reduction parity.

## Validation scope

New tests distinguish payload/name and hand-event/payload side; type/status
gates; last assignment versus accumulating flags; NoInput partial failure;
matching-only scalar payload reads; negative zero; NaN steer threshold; actual
Reset fields; and unconditional zero-length estimate before zero selection.
The live cvar test checks unlinked/default/mutated target behavior. Root owns
Cargo and the original-data neutral attribute continuation gate. Those gate
results are pending at the time of this packet, not implied by unit drafts.

Independent review: skeleton_closure verified all23 scalar PDB offsets and
raw Reset stfs destinations, including f31's positive-zero load at82b3927c,
and found no duplicate retained Processed destinations. Root independently
checked all50 direct flag mappings, six PhysicalAnimOut cases and all24 scalar
assignment signs against the original, as well as the Push VMX register decode.
