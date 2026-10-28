# Motion PowerSliding29 live draft

2026-09-19; connected IDA sk82_na_m.xex, fresh pseudocode and raw control flow.
Confidence **high** for typed callback/order/owner binding; numerical host
estimate and dot reduction remain the repository's explicit host boundary,
not Xenon bit parity. Unregistered, no Cargo/corpus execution by author.

## Files and mounting

* actor_motion_power_sliding.rs: proposed Actor motion_runtime_session child
  `power_sliding`, with actual IPhysical, animation/UCOM, Motion intents and
  sliding-state writes. Its child motion_power_sliding.rs is the kernel;
  motion_power_sliding_tests.rs supplies four focused production-kernel tests.
* actor_motion_power_sliding_attributes.rs: proposed input_cvar_runtime child
  `power_sliding`, with retained generated global and live cvar230 read.
* physical_player_phys_out_motion_heading.rs: proposed
  physical_player_phys_out::construct child `motion_heading`, containing
  three component-only const getters used by the original heading callback.

Add shared ActionRuntimeServices.power_sliding: PowerSlidingAttributes,
try_bind on its existing runtime Vault. Pass borrowed optional attributes,
actual controller cvars, borrowed existing application DebugOverlaySlot and
actual retained controller dt to motion_runtime_power_sliding. Do not bind
globals or construct a debug overlay per Actor/update. Missing services fail
at first native use, leaving prior callback stores intact.

Root controller Instance gets a PowerSliding variant with
kernel::SlidingInstance. Create82ca3088 requests size18/alignment10 hex;
allocation failure leaves None. Source floats+8/+c/+10 initialize0, three
high bits of word+14 initializefalse; low29 bits remain unknown and are not
invented typed fields. Vtable823006d8 slots2c/30/34/38 are82ca3088,
825187c0,82cc7478,825187c0. Begin/End are verified raw BLR. Update must still
reject absent instance after interface/intent/filtered reads.

## Actual neutral callback semantics and order

Native Update82cc7478 resolves IPhysical, ISkaterAnim, IMotionGraph,
ISkaterMotionGraph in that order, asserting cached/UCOM identity. Virtual+40
at770c returns graph SlidingState+0c84 (GetSlidingStateReadOnly825fdfc8).
Offsets0/4 are left time/flags,8/c right time/flags. Draft uses existing actual
ActorSkaterGraphState.slide_time[0/1] and slide_bits[0/1]; preserves unrelated
bits and updates reconstruction known masks only for written flags.

1. Read RightSlide presence/value773c and LeftSlide775c/7760, absent values0.
2. Read actual filtered Ground7770. On ground entry reset instance elapsed,
   capture both current values, clear start-history flags. Always store
   was_ground. Non-ground returns here and does not touch tuning/cvars.
3. Read actual Motion+a4 deck speed77f8, live speed threshold7838. Compute
   ordered `fast = speed > threshold` once and retain it.
4. Read RightSlideStart786c, publish right start80000000 iff fast, present,
   not previously present, and prior-right < current-right. Store prior-right
   and start-presence. Then analogous LeftSlideStart78ec with prior-left >
   current-left; publish left start, store left value/presence. Ground entry
   cannot spuriously produce start pulses because prior values just captured.
5. Read debug ref7968 and, when true, resolve real overlay twice; both actual
   draw targets7a20/7aa8 are825187c0 raw BLR.
6. Call actual GroundHeadingCosine7ac8. Evaluate speed graph7b14, form
   `(speedGraph * (1 - abs(heading)))` using separate fsubs7b28/fmuls7b2c.
   Evaluate heading threshold graph7b68 then duration graph7bb0.
7. If computed threshold < heading threshold, add dt to instance elapsed;
   otherwise store+0. Compare elapsed > duration, retaining result.
8. Read live minimum duration7c2c; add actual dt to right graph timer7c5c,
   reread actual dt and add to left7c74.
9. Publish right then left end40000000 iff corresponding timer > minimum AND
   (slide intent absent OR !fast OR heading expiration). Read debug ref7cf8
   again; if true resolve overlay three times before BLR draw calls7d54,
   7da4,7df8.

Absent slide intents do not skip steps3..9: neutral grounding consumes live
globals/heading and advances both timers. The typed kernel preserves these
stores even if a later graph/cvar/overlay operation fails.

Raw CR proof: speed7858=41990008 BGT, right-increase7880=41980008 BLT,
left-decrease78fc=41990008 BGT. Heading timer7bbc=40980018 BO4/BI24 skips
on !LT (including UN). Heading-expired7be4 uses ordered BGT. End gates
7c7c=4099002c and7cbc=4099002c skip on !GT, including UN; thus NaN timers
do not set end flags. `!fast` deliberately includes unordered speed.

## Global and cvar evidence

Global+14c allocation828b2f74 calls generated anim_motion ctor2f98 with
collection **d124e6721b8efc67**, member store2fa4. Generated ctor828b1590
binds class **90c3decf0105631e** and cached layout fallback DefaultDataArea5c0.
Provider validates binding once, rereads retained cached graph bytes each
call and runs dynamic node/inheritance lookup separately for each scalar.
Missing scalar fields use actual DefaultDataArea4 zero; missing owner errors.

|Consumer|Source|Key/layout|
|---|---|---|
|Minimum deck speed|82cc7838|1cb95aadd7b65137|
|Speed graph|82cc7b14|layout+4a0, four x+10/y+20|
|Heading threshold graph|82cc7b68|layout+500, four x+10/y+20|
|Heading duration graph|82cc7bb0|layout+4d0, four x+10/y+20|
|Minimum slide duration|82cc7c2c|09ef22b12364eb7f|

Cvar230 **kbSlideDebugDisplay**, section **Animation**, title **Slide Debug
Display**, boolfalse, no update hook. Reference830a43c4; ctor82f85b70,
registration82fd4eac, after BodyTilt227..229 and before BodySpin231's
82fd4ecc. Among previously mounted cvars insert between
kDebugWipeoutConditions82fd4d10 and kMOAngleMaxDelta82fd4f38. Runtime reads
the actual current linked target at both gates, never assumes false.

## GroundHeadingCosine825f4ea8

The source repeats actual IPhysical cached/UCOM resolution, captures actual
Motion+50 deck velocity, vmsum3fp squared length4f90 and compares against
immediate float bits3a83126f at4fa0. Raw4fa8=40990124 BO4/BI25 branches to1
on !GT, so degenerate and unordered squared lengths return1 before reading
Ground transform/flip. Positive branch captures PhysOut+20 Ground transform,
reads Motion+111 skateboard_flipped, selects transform Z column and toggles
its sign bits if flipped. This is not animation riding-fakie or a physical
deck-speed-scalar test.

Raw5058..50bc computes original length using reciprocal-sqrt seed and two
refinements, squared-times-inverse with exact-zero select, then reciprocal
seed/two refinements, velocity scaling, final XYZ dot with selected forward.
Draft reuses length_from_squared_and_estimate/refine_drive_reciprocal and
host_dot3. Ordinary host seeds1/sqrt and1/x match the repository's existing
HostReckoningNumeric policy; native estimate bits, VMX reduction exceptional
rounding and denormals still require reference traces. It deliberately does
not simplify this to direct normalization or clamp the returned cosine.

The three proposed getters do not sample the write latch; they check actual
component construction and read retained Motion vector/Ground transform/
MotionRemainder.skateboard_flipped. Existing generic projected vector getters
sample a latch and are inappropriate for these direct const source reads.
Deck scalar getter already exists from grounded PhysicsSpeedCompare.

## Validation and limits

Four kernel tests: neutral live-read order/timers/end flags; first-ground
start suppression and history preserved on late graph failure; unordered
heading/time comparisons; non-ground history with lazy tuning and allocation
failure. All five Rust files formatted and rustfmt-checked, not compiled by
author. Root must register, independently review, compile and run actual
actor corpus. This is complete required neutral PowerSliding behavior, not
proof of an executable neutral Motion frame: BodySpin30 and later callbacks
remain their own explicit pending work.
