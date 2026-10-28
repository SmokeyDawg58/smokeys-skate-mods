# Retained body animation root producer

Confidence: high, pending independent root review and the original-corpus gate.
This closes native tree construction and base-pointer publication; it does not
claim mutable animation evaluation or a rendered skating frame.

`Andale::Animatable::PlayAnim` 82d7a7a0 belongs to the same retained SkaterAnim
owner used by the existing generated PlayAnimation lifecycle. The terminal
`AnimationDispatch::Play` request now reaches `try_play_body_animation` through
`try_dispatch_owned_animation`; Blend, ChannelBlend and Sequence requests report
their explicit unreconstructed native producer boundary.

The native order is TransitionStore::Clear 82d7a080, TreeStore::Clear 82d79e18,
TreeStore::NewTree 82d74070, virtual SkaterAnim::GetAnimTree 82c8b710, name copy,
root SetSpeed, optional root SetTime, virtual ChannelAnimatable::SetBaseAnim
82c8a4e8. Transition Clear resets lists and rebuilds ascending free lists while
preserving blend/sequence payloads. Tree Clear only rebuilds its free list;
NewTree removes its last element (initially body pool 3) and then calls actual
PoolStack::Clear 82c8ef98. Pool Clear destroys entries in allocation order and
resets count/location, preserving backing pool ownership.

The retained playback database constructs the actual requested record into that
pool using the actual construction map. SkaterAnim then adds actual initialized
bind-pose/board-pose/mirror owners using the already independently reviewed
AddBindPose implementation. An absent record causes the two list Clears again
and returns false. Native leaves the previous base pointer and name unchanged,
even though its selected pool was cleared. The checked Rust node lookup exposes
this invalidated pointer instead of inventing a usable pose.

Only a successful tree copies the exact six packed animation-name words, then
sets playback speed. At 82d7a8a4 raw instruction 4099001c has BO4 BI25: branch
when CR6.GT is false. Therefore only ordered positive time invokes SetTime;
NaN skips this call. Base publication happens before following the actual
channel root's base links and updating the bottom ChannelBlend's base reference.
Each root retains its original pool index and a pool-identity-checked node handle.

Wrapped time operations required by this path were recovered from the actual
vtables: AddSQT::SetSpeed 82588618 and SetTime 825885b8 visit A then B, preserving
aliases and repeated visits. Its GetLength 82588588 reads the motion operand.
Mirror forwards to its child at 82c92910/82c928f8/82c928c8. Pose vtable 82044b48
slots 7 and 8 point to the empty function 825187c0; Pose::GetLength 82587bd0
asserts and remains an explicit error. No substitute clip or evaluated pose is
created for these constant operands.

The dedicated initialized-Actor NewChannel corpus helper now continues with
actual Play, replay into the same native pool, bottom-channel retargeting, and
failed-name lookup. It uses the actual B_GSTR_SKY_BOTH record and checks retained
unevaluated root ownership and positive length. This is a source API integration
probe, not a claim that a graph selected that gesture as its ordinary body state.
Root owns compilation, corpus execution and independent verification.

Fresh external evidence: `%TEMP%/skate2-native-play-anim-source.json` contains
fourteen functions' decompilation and complete original bytes. Original assets,
symbols and IDA database remain external. Native animation AddTime, dynamic
Eval/ACS fetch-and-blend, and body current-pose production still require their
actual producer chain before BodyDispatch can admit a nonnull tree.
