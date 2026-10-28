# BodySpin30: native neutral common path and recovered active kernel

2026-09-19, fresh IDA/raw review. Existing `reckoning_body_spin.rs` is a different
physics operation; no duplicate Motion BodySpin implementation was found.
New unregistered files checked absent before creation:

* actor_motion_body_spin.rs: same Actor/interface adapter, intended child
  actor_construction::motion_runtime_session::body_spin.
* motion_body_spin.rs: retained instance and full recovered Update kernel.
* actor_motion_body_spin_attributes.rs: intended input_cvar_runtime::body_spin_motion,
  shared Global+130 generated wrapper and late live cvar provider.
* actor_animation_body_spin_channels.rs: intended actor_animation_construction
  child; actual EndChannel overload and SetChannelInfluence implementations.

## Lifecycle and native mounting

Occurrence30/BodySpin vtable8230001c: Create+2c82c9eb10, Begin+30BLR825187c0,
Update+3482cc4600, End+38BLR825187c0. Both no-op entries were reread as4e800020.
Create calls StateGraphAlloc20 hex, nameunknown, alignment10 hex at82c9eb34.
NULL remains absent. Success sets state+8=0 and floats+c,+10,+14,+18,+1c=0.
The merged UpdateRidingFakie::Data vtable symbol does not change these semantics.
Root should add the typed BodySpinInstance to the existing lifecycle occurrence
storage and route Update only; Begin/End need no extra behavior.

The actual adapter resolves IPhysical, IAnimatable, IChannelAnimatable,
ISkaterAnim, ISkaterMotionGraph, IMotionGraph, IActor in native order, validating
existing published allocations/registry identities. It retains the actual
physical and animation Arc allocations; Motion reads check the captured graph
occurrence. It never creates animation/channel/physical state.

## Required common path, including neutral input

After interfaces, read live enable-landing-on-board. Only true reads actual
PhysOut.State.physics_state and tests505. Independently read filtered state
three times: Air2 (or landing505), Wipeout3, Ground1. Only when not airborne
read original IsDoingTrick825fe0b8: retained Motion flag08000000. Read actual
Motion BodySpin intent (absent means source-initialized0), then actual SkaterAnim
mirror flag40000000 and negate when set. Evaluate live abs-input graph and
reapply sign with FSEL. Only airborne invokes IsTimeToPrepareLand825f4b60.
Read actual Motion GrabState two busy integers. Only then dereference instance.

State transition assignments are sequential, not mutually exclusive priority
guesses. E.g. state1 first clears to0 if disallowed, then4 if grabbing, then2
if abs(input)>.3; state2 ends3 whenever airborne even after earlier assignments.
All four tuning reads occur after state selection, including states0/4.

Neutral1170 state0 therefore performs all preceding reads, calls actual
EndChannelPrematurely(FS,.2,true), then(BS,.2,true), then commits instance state+8
and signed input+c. Failure in second channel operation preserves the first
operation but not these final instance stores. It is not an early no-op.

Original packed names initialized82f84600/618 are
IA_BODYSPIN_OLLIE_BS_0_N and IA_BODYSPIN_OLLIE_FS_0_N. These are both channel
and animation names when NewChannel is called.

## Active kernel and explicit remaining services

State1 zeroes Front weight/delta(+18/+1c) then Back(+10/+14). State2 tests
input>.3 then separately input<-.3; conditionally starts named channel if
absent and not preparing; ends opposite with.15/current-frame=true. States2/3
independently query FS then BS, integrate each present channel's weight and
publish influence after its two history stores. FS selects alternate blend
when target<old; BS deliberately selects alternate when target>old. Preserve
this source asymmetry. Raw weight code uses multiply plus FMA and two FSEL
rate limits, then unit clamp. Ordered threshold branches skip for NaN;
FSEL unit clamp selects1 for NaN and +0 for either sign of zero.

Actual owner adapter currently exposes two precise active Pending boundaries:

1. Air/landing505 IsTimeToPrepareLand825f4b60. This is not called by native
   neutral grounded route. Existing implementation absent. Source first calls
   its virtual override predicate, resolves actual IPhysical, checks actual
   SystemReckoning COMvelocity.Y<0; then global anim_motion_inair_disttocog
   scalars and OffBoard/Air/Animation landing fields, optionally original
   global animation LandingDistanceScalarVsNormalY graph and offboard preland
   cvar. Do not replace with an unconditional false or generic time threshold.
2. NewChannel with source params: priority0, cycle/mirror false, speed1,
   blend-in.2, blend-out.3, endpoint flagsfalse, use-channel-attributesfalse.
   Existing try_new_gesture_channel has identical owned tree/pool producer but
   hardcodes BOTH lengths.3. Root should generalize its existing initialization
   parameter at the original store (retain gesture callers' .3), then route
   this callback to it. Do not clone the producer or post-edit a published
   channel, which changes partial-failure behavior. Status remains ignored by
   original BodySpin caller. This branch is explicitly Pending until that reuse.

EndChannelPrematurely82d7d648 actual dictionary lookup: missing channel succeeds;
existing channel only changes when blend_out_time equals0 (either sign). It
loads actual flags, writes supplied length, then flag
`old & bfffffff | (true<<30) | 28000000`, equivalent OR68000000. Runtime named
flags blend_out_last_frame/blending_out/blend_out_early mirror these source bits.
SetChannelInfluence82d7d760 returnsfalse on missing, otherwise writes influence
without clamp/NaN assertion; original caller ignores returnedbool.

## Shared attributes and cvar packet

Global ctor alloc828b2d7c, generated anim_motion ctor2da0 with collection
dc bade97 ca665643 (full key **dcbade97ca665643**), stores+130 at2dac.
Same original generated class90c3decf0105631e and cached layout5c0 as existing
BodyTilt global wrapper, but a distinct collection. Generated ctor828b1590
supplies DefaultDataArea5c0 when no cached layout, so zero layout fallback is
source-supported HERE (unlike physical conditioner raw physics_animation).
Retain one shared wrapper on ActionRuntimeServices and validate the same Vault
Arc as the actual physical owner at first attribute use, not eagerly.

Graph reads x360..37c/y380..39c at82cc4b6c..4b84; reuse PointGraph8.
Four independent generic/inherited scalar lookups, index0, missing field zero:

|Use|Key|Call|
|---|---|---|
|Maximum step|d2cf959d06792567|82cc4d5c|
|Maximum delta change|c215e76743ae5d4a|82cc4db4|
|Blend|803b5ae19b41b6c8|82cc4e0c|
|Alternate blend|4ad662ad587df578|82cc4e64|

Reserved runtime cvar231: booltrue, null callback.
Reference830a5d20; initializer82f85e40; bool constructor82f85e6c;
table82fd4ecc. Section Animation_Skater_InAir@822faf8c,
label kEnableLandingOnBoardBodySpin@822faf6c,
title Enable Landing On Board Body Spin@822faf48. Read current target each Update.

## Validation and remaining mounting

Three kernel tests authored: full neutral common-path/read ordering and nullable
constructor; second-channel failure preserving final instance stores; state1
threshold override/channel weight update and exceptional FSEL semantics.
Standalone rustfmt only; no Cargo or registered edits. Independent review
requested. Root must mount typed instance/lifecycle, shared attributes/cvar,
Actor adapter and actual animation methods before claiming runtime execution.
These files enable the complete neutral common path, not all active BodySpin
dependencies or sustained air skating.
