# Retained transition time and shared blend coefficient

Fresh IDA decompile/raw review 2026-09-19. Draft `actor_animation_transition_time.rs`
is intended child `actor_animation_construction::transition_time`. No Cargo/shared edits.

Mount contract: channel_time calls `try_update_body_transitions()` after its native
channel prepass and BEFORE Animatable property resets. Replace Body match arm with
`add_body_reference_time(reference, dt, properties)`. Body/hands consumers share
`try_transition_blend_weight(index)`, retaining their different assertion/child order.
Existing GestureChannelError mapping is explicit: tree errors retain payload,
BindPose preserves underlying channel error, InvalidStore becomes InvalidChannel,
missing/unsupported owner retains label, lock remains lock.

Animatable::AddTime82592b58 calls UpdateTransitions at2b78, resets properties at
2b88/90/98/9c, fetches current animation2bac, calls current AddTime2bcc, then sets
playing true2bd4. Existing channel_time owns these resets/result publication.
UpdateTransitions82d7aef0 checks count af08, repeatedly calls GetAnExpiredTransition
af24 then ForceTransitionRelease af30, rescanning after each release. The getter
82d71fc8 traverses oldest first; it does not stop when the oldest is incomplete.
TransitionComplete82d6e070 raw e080=41990008 is trueGT(time,length): equality and
unordered remain incomplete. Completed transitions are removed before this frame's
clock increment, not immediately after reaching/exceeding their length.

TransitionBlend::AddTime82d6df98 assertion dfc8=4098001c tests !LT(dt,0), so NaN
passes. Without flag20000000, FROM AddTime e028 uses a local default property owner:
false,false,-1,-1. TO e044 receives caller properties. Clock load e048/add/storee050
occur only after both child calls. Current-frame=true skips FROM entirely. Draft
recurses through actual retained pool handles and transition objects; no new tree owner.

GetBlend82d6e090 exact arithmetic: fdivs(time,length), fsel(-ratio,0,ratio), then
fsel(1-lower,lower,1). e0c8 trueLT length<0x3d4ccccd returns linear; NaN length
continues. Square rounded at e0cc; trueGT value>1 e0d4 or trueLT value<0 e0dc returns;
cube rounded e0e0, cube*2 rounded e0f4, fused square*3-minus-product e0f8. Draft uses
mul_add(3,-product), not separately rounded multiplication/subtraction. No positive-length
assertion belongs inside GetBlend: body and hands assert at different native points.
Tests cover adjacent threshold, FSEL unordered-to-one, signed zero, and clamping.

Rustfmt passed. Root must run mounted unit/strict checks and actual original-asset
neutral continuation; this author did not run Cargo. Host floating-point estimate,
denormal and NaN-payload limitations remain the project's existing boundary.
