# Grounded Action condition 96

Unregistered implementation draft, 2026-09-12. No Cargo run and no claim that
the original grounded graph executes yet. This extends the proposal in
`generated-action-ground-continuation.md` only at the first necessary grounded
condition callback.

## Source and ownership

`DisablePushBrake::ActivationFunc82c9a8f0` resolves the actual Actor physical
interface. Its first GetPhysOut at 82c9a9cc reads Ground (+20 pointer), then
Ground+50.Y at 9e0/9e4. This is a const direct read, without the output write
latch. The proposed PhysOut child reads the existing retained GroundUp vector;
it neither constructs another normal nor requires W.

The condition captures anim_motion_pushing's slope attribute pointer at
82c9aa28 (key a40df063c0ac73df, element zero). GlobalAttributeCollection+13c
is a distinct actual selected allocation: allocation 828b2e54, generated
anim_motion constructor 828b2e78, member publication 828b2e84. The generated
class is 90c3decf0105631e, collection 9dc4354dbe428a7f, cached layout 0x5c0.
The draft retains that Vault instance once; a missing global remains an error.
Only a real missing attribute follows native DefaultDataArea(4), at aa38.

The pointer is saved at aa4c. XMVectorASin at aa6c precedes the float load
at aa78. The implementation reuses existing reckoning_numeric::try_asin
(source 825e8ce0), with its explicit estimate collaborator unchanged.
The result is multiplied by radians-to-degrees bits 42652ee1 (822f21d8),
then subtracted from 90, bits 42b40000 (82010398). No domain clamp,
normalization or host asin is introduced. Raw aac0 word 1b19d965 is the
ordered greater-than comparison; VMX128 VA is 57.

A SECOND GetPhysOut at aad8 executes even when the slope test is true.
Only then does aae4 short circuit the result. Otherwise aae8 loads the
Skeleton pointer (+14), and aaec reads Skeleton+242, PDB
mTellAnimDontPushOrBrake. The adapter uses the existing retained Skeleton
field, and does not prefetch it before that branch.

## Proposed registration

* actor_action_disable_push.rs: child of actor_action_motion_intent_bridge,
  exposing PushingMotionAttributes and Actor.action_runtime_disable_push.
* actor_action_disable_push_tests.rs: its five focused tests (late failures,
  pointer/value order, mandatory second output resolution, equality and NaN).
* physical_player_phys_out_action_conditions.rs: child of PhysOut root,
  exposing a direct const GroundUp.Y read without a synthetic write guard.
* Parent integration must retain the real selected +13c global and supply the
  numeric collaborator to condition dispatch. No shared registration changed.

## Immediately following original expressions

Actual loaded OnGround children are Grabbing45, Antic46, HippyJumping47,
Pushing48, Brake49, Kickturn50, IdleTurning51. Neutral input reaches
Pushing expression67: !c96 && (c44 RightPush || c45 LeftPush). It then
reaches Brake expression68: c97 Brake && !c96. Thus Brake presence is
sampled before the reused/cached condition96, and no Brake means no new
condition96 call there.

Next is Kickturn expression102. Its FIRST predicate is c98,
PhysicsSpeedAndSlopeCompare less0.1, even when KickTurn intent is absent.
Fresh native ActivationFunc82c99710 raw97f0/97f4 loads PhysOut+4 then
Motion+c0, and 97f8 invokes existing EvalCompare. PDB identifies this as
PhysOut_SkateboardMotion.mHeadingAdjustFactor, already retained as
MotionRemainder.heading_adjust_factor. It is not deck speed or a newly
computed slope. This is the next smallest necessary dispatch closure.

If c98 passes, c99 requires abs(KickTurn)>0.3; only then do the remaining
physical conditions execute. Source Pre102 is c98 && c99 &&
((c101 && c102 && !c100) || (c100 && c103 && c104)). c100 is
IsRidingFakie82c9a308; c101/102/103/104 are PhysicsSpeedCompare abs less
4.0/4.6/1.5/2.1 respectively. Sustain102 omits c98 and the tighter entry
thresholds. These are not needed on a neutral absent-KickTurn route after
the genuine c98 read. Finally HardTurn93 tests c105 abs(HardTurn)>0,
then !c106 Sliding; otherwise actual IdleTurning94 is selected.

An actual anticipation input can enter earlier: Pre58 tests c76 AnticMag>0.9
then c77 AnticMag presence; OllieAnticipation83 tests c78 abs(AnticAngle)
<=0.52. These HasAGIntent predicates reuse the existing actual intent map
and comparison semantics. They precede c96, rather than being a generic
flick-to-state shortcut. Their selected behaviour/update callbacks still
need the separately recorded original dispatch closure. Gesture-to-Trick
callbacks and non-Default Motion execution remain unfinished.
