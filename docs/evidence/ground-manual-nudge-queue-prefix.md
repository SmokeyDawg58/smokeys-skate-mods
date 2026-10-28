# Ground manual/drag/nudge through first queue append

New-only drafts: `physical_player_ground_manual_nudge.rs`,
`physical_player_ground_antiflip_nudge.rs`, its focused tests, and
`input_cvar_antiflip_nudge.rs`. No Cargo, registrations or shared edits.

## Consecutive caller and real owners

`try_update_pair_ground_manual_nudge` consumes root's actual
GroundBeforeManualEffect token (through82d9a770). It verifies the same retained
player/board/skeleton and actual current Ground state, then borrows the actual
board force runtime and constructor attributes using existing
try_with_ground_force_board. It calls the complete ManualEffect draft at a774.
The same Entry e8/ec/f0/f4/f8 history is used; no per-call substitute owner.

At a77c it calls the existing SkateboardBrakeDragServices.try_linear_drag.
GroundForceInput uses the same Processed Arc. Its Reckoning guard exists only
for this leaf, after Manual's own late Reckoning read and before Nudge. This
avoids holding an outer guard across any operation that reenters the owner.
The returned drag is retained for source a93c, not applied early.

At a7b0: dot3(push.world,push.world). At a7b8 retain full typed push.world plus
brake.world (including unknown W), then a7c4 compares the squared magnitude
against1. Only ordered GT sets existing Ground+f1 is_push_accelerating true
at a7dc. False/unordered does not clear it. The existing Ground.Update reset
owns its preceding value. Nudge follows immediately at a7e4.

Successful GroundBeforeForceQueues retains ALL preceding force/torque locals,
ManualEffect, drag and combined push/brake. It stops before caller a7e8's
wallride-time gate/front/rear appends. Nudge may already have appended its
source force; no ordinary force is queued early by this wrapper.

## Actual LinearDrag9a4 producer closure

Fresh raw ProcessedPhysIn.Reset82b394a4/4ac stores f31+0 to balance9a0 and
push9a4 respectively; source f31 established+0 at3927c. The latter actual
retained field is currently missing. ProcessAnimAttributes82b59de8 contains
no direct9a4 store. An aligned .text scan for displacement09a4 finds only
Processed.Reset's stfs and Processed.GetSyncLog's address among physics uses;
other matching stores belong to Score.AirPeriodData. This scan does not claim
to rule out every possible indirect future write.

Required narrow integration amendment, parent approved: add push:Option<f32>
to existing ProcessedPhysicsStateFields, generic new=None, actual Reset=Some(+0)
at the native position. Replace GroundForceInput.push's unconditional
Missing("push9a4 producer") with its normal late self.read of that field.
Do NOT alias animation push_speed940, frame input, or inferred pushing flags.
The live caller intentionally uses the existing drag leaf and will retain its
explicit missing boundary until this actual producer amendment is integrated.

## UpdateAntiFlipNudge82d8d508

The new kernel receives the real mutable PhysicsGroundFields and actual
SkateboardForceQueue. It does not reset either owner.

1. d520..540 read retained Ground+90 anti-flip torque and dot3 with itself.
   d544 word4099 is NOTGT: unordered or <=0 sets counter0. Otherwise increment
   existing Ground+10c with native 32-bit wrapping addition. Store d55c BEFORE
   live kMinAntiFlipFrames target/value read d564/568.
2. Signed counter>minimum gates fresh Processed968 deck speed d580, then live
   maximum-speed target/value d584/588. d590 NOTLT includes unordered and exits.
3. d5b4 read actual Processed deck.Z60. If its Y is ordered>0, XOR-negate full
   vector. d614 overwrites Y with +0. Other XYZ and known/unknown W survive.
4. d61c dot3 planar vector with itself; d634 NOTGT against3a83126f (.001)
   includes unordered and exits without reading strength or queue.
5. Capture strength target d670 BEFORE reciprocal-square-root estimate d67c.
   Load captured value d690 AFTER estimate. Existing BoardFill helper
   board_speed_from_squared_and_estimate has exactly this following graph:
   seed square; half-seed; final-negated FMA residual (standard VNMSUB d6a0);
   refinement FMA; repeat at d6b0/b4; squared*inverse; zero selection. Reuse
   that one evaluator instead of the different drive refinement helper.
6. d6c8 scalar=strength/length; force is full planar vector*scalar, local
   position full+0. d6e0 AddSkateboardForce(type0x11=kAntiTorqueNudge) executes
   on the SAME queue, retaining existing capacity21/silent-full behavior.
7. d6f0 sets Ground+f3 anti_flip_nudge=true even if a full queue ignored append.
   d6f4/f8 then reads debug target/value; enabled requires the actual overlay
   at d708. Missing overlay preserves counter, queue append and flag.

Inactive paths never clear f3 here: the earlier real Ground.Update prefix does
that. No presumed inactive shortcut is used, and unknown W is never fabricated.

## Exact four references

All native callbacks are null. Host slot reservations146..149 follow the native
initializer-table order, independently of lexicographic title ordering.

|Slot|Reference|Initializer/table|Section; label/title; default and GUI|
|---|---|---|---|
|146|830d809c|82f8f078 /82fd571c|Physics_Skateboard; kbDrawNudgeDebugInfo / kbDrawNudgeDebugInfo; false|
|147|830d8c08|82f8f1d0 /82fd572c|Physics_States_PhysicsGround; kAntiFlipNudgeMaxDeckSpeed / Anti Flip Nudge Max Deck Speed;3f8ccccd (1.1),[0,2]|
|148|830d8958|82f8f230 /82fd5730|Physics_States_PhysicsGround; kAntiFlipNudge / Anti Flip Nudge;42480000 (50),[0,200]|
|149|830d8f8c|82f8f290 /82fd5734|Physics_States_PhysicsGround; kMinAntiFlipFrames / Min Anti Flip Frames;12,[0,100]|

Strings:146 title82342890/label823428a8/section823428c0;
147 title823429e4/label82342a04/section82342a20;
148 title82342a40/label82342a50/section82342a60;
149 title82342a80/label82342a98/section82342aac.

## Registration and verification boundaries

Root registrations required: constraint_session::ground_manual_nudge and
::antiflip_nudge; input_cvar_runtime::antiflip_nudge plus definitions/ordering;
cfg(test) child `physical_player_ground_antiflip_nudge_tests.rs` under existing
physical_player_physics_state (allows its actual private Ground constructor).
Manual's earlier five drafts need their separate registrations first.
Processed push reset/reader amendment above is the only new state requirement.

Four focused tests cover active append-before-debug failure, full real queue,
NaN torque counter reset preserving prior flag, integer wrap and NaN speed lazy
exit. They use the actual GroundFields constructor and retained queue. Not run.
Original same-pair corpus admission remains required; no complete Ground.Fill,
full Ground.Update or force solver execution is claimed by this draft.

## Root integration acceptance

Registered with ManualEffect and actual push+9a4 Reset/read. Root independently reviewed raw a778..a7e8, full Nudge d508..d71c, Reset394ac, and all six cvar initializers/order including null callbacks. workspace54 failed an error Display call;55 passed all nine new tests but failed the manually updated title-order oracle. Fixed the oracle using source section/title order (uppercase Draw/Show/Wrong precede lowercase max). workspace56=4055 units+6docs PASS; original48=17/17 8.59s including neutral same-owner Manual/Drag/Nudge; Clippy88/fmt41/diff41 PASS. Clippy87 style fixes preserve NOTLT/NOTGT NaN behavior and DT-first addition.
