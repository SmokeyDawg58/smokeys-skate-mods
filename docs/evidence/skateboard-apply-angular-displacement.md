# ApplyAngularDisplacementToDeck82b36798 raw closure

New unregistered files: simulation_rigid_body_apply_angular.rs and tests,
simulation_assembly_apply_angular.rs, dynamic_assembly_skater_apply_angular.rs.
Existing unregistered physical_player_ground_queue_torque_prefix.rs now extends
through a980 and returns GroundBeforeManageHangUps. No registered/shared edits,
Cargo, or claim that HangUps/complete state Update is implemented.

## Actual ownership and distinct method

Outer Skateboard.ApplyAngularDisplacementToDeck82b37e90 loads SAME +c Body and
dispatches to SkateboardBody method82b36798..36980. Body+18 assembly -> partList
-> Part6+28c cached body. The draft pool adapter resolves exactly that existing
Part6 observation, then the backend's SAME rigid-body pool. No new body, source
inertia, force-queue entry or generic angular policy is substituted.

Compared with TargetAngularDisplacement82b361e0, ApplyAngular does NOT read omega,
normalize requested displacement, subtract projected angular motion or clamp
correction. It performs the retained inverse-tensor/DT/forward-tensor graph
directly on requested XYZ. It does not use ApplyLocalTorque's board orientation.

The implementation is a new child under existing target_angular so it reuses
private source-reviewed cross_xyz and reciprocal_second helpers. It does NOT
call TargetAngular or run its prefix. Distinct instruction/read ordering stays
in the new method; no new common matrix inverse implementation is introduced.

## Register mapping and numeric graph

VMX128 raw fields decoded using VD=(word>>21)&31 | (((word>>2)&3)<<5),
VA=(word>>16)&31 | (((word>>5)&1)<<5) | (((word>>10)&1)<<6),
VB=(word>>11)&31 | ((word&3)<<5). Standard VMX VNMSUB uses ordinary registers.
IDA's VA display often adds32, so printed operands alone are insufficient.

Actual packed body cache is +70 [xx,xy,xz,inverseMass], +80 [zz,yy,yz,state].
Only XYZ reaches retained torque stores. The permute tables produce column
Z=[xz,yz,zz] and Y=[xy,yy,yz]. Input W/packed adjacent fields remain in scratch
W only; they cannot affect these XYZ results. Unknown request W is allowed
without canonicalization or publication of a replacement lane.

|Address|Dependency / retained access|
|---|---|
|367ac|load requested full vector|
|367c4|fixed DT bits3c888889 from8225c488, stored twice in distinct locals|
|36800|capture actual inverse_inertia_split|
|3680c|capture old torque X|
|36810|capture inverse_inertia_full|
|3681c|capture old torque Y|
|36860|cofactorX=cross(Y,Z), final-negated rounded VNMSUB|
|36868|first reciprocal estimate(DT)|
|36870/74|cofactorY=cross(Z,X), cofactorZ=cross(X,Y)|
|3687c|first DT residual=-(seed.mul_add(DT,-1))|
|36888|dot3(X,cofactorX) determinant|
|3688c|retain original captured split tensor for final multiply|
|36890..ac|cofactor transpose; first DT refinement at a8|
|368b4|reciprocal estimate(determinant)|
|368bc|second DT residual|
|368c4|first determinant residual|
|368c8/cc|second DT refinement, first determinant refinement|
|368d0/d8|second determinant residual/refinement|
|368e0|angularRate=inverseDT*requested XYZ|
|368e4|fresh inverse_inertia_full read for final multiply|
|368f0|capture old torque Z|
|368f8|SECOND fresh reciprocal estimate(DT)|
|36904/0c/14|multiply transposed cofactors by inverse determinant|
|3691c/24|first residual/refinement for second DT reciprocal|
|36920/28/30|inverseTensor.X*rateX -> Y FMA -> Z FMA|
|3692c/34|second residual/refinement for second DT reciprocal|
|36938|multiply transformed result by second inverseDT|
|36948/4c/50|cachedTensor.X*torqueX -> Y FMA -> Z FMA|
|36964/68|newZ=oldZ+accZ, newY=oldY+accY|
|3696c|store torque Y|
|36970/74|newX=accX+oldX; store X|
|36978/7c|store Z; cooldown=0|

Both reciprocal iterations use final-negated FMA residuals. Do not replace with
negative-multiplicand FMA, generic reciprocal/inverse, cancellation of the two
tensor passes, or one DT division. The code preserves separate estimate calls
and their source order. No body torque/cooldown mutation occurs until all
fallible numeric services have completed. Host estimates, dot3 reduction,
denormals and NaN payloads remain explicit existing policy boundaries.

Fresh raw shows final scalar additions differ from TargetAngular's operand
order: Apply uses oldZ+accZ, oldY+accY, accX+oldX. It also differs from
ApplyLocalTorque's final X order. These are preserved rather than commuted.

## Extended Ground consumer

After the earlier queue/Target prefix, actual ApplyAngular is called separately
for retained AntiFlip a8c0 then Manual angular output a8cc. Only then read the
same Processed6b0 high flag. Unknown high-bit provenance errors at this point,
preserving the already committed torques; it does not silently include/omit
the conservation force. High clear appends6; then unconditional7manual,1slide,
8pump,16wall, retaining each original full force/local-position record.

Set captured LinearDrag at a93c using existing all-Parts Body helper. Then
compute stored combined(push+brake)+pump with a separate rounded addition,
dot3 with itself, ordered GT bits37800000. Success sets existing board530 high
flag via SAME Entry mutation (currently named mark_collision_response); false
does not clear. Return before ManageHangUpsa984. A failure preserves all
preceding queue, torque, drag and flag writes in source order.

## Registration and validation requirements

Root adds child target_angular::apply_angular, backend::apply_angular, and
skater_pool::apply_angular. Existing error envelopes are reused because the
operation has the same body-admission/numeric failure types, not because the
algorithms are identical. Queue prefix's final token is now
GroundBeforeManageHangUps rather than GroundBeforeApplyAngular.

Four new tests use actual retained body slots and cover direct displacement
despite large omega, nonsymmetric/off-diagonal cache values, distinct repeated
DT estimate and determinant service order, failure before all stores, and
request W independence. They are not run. Compilation, independent raw review,
same-pair original corpus and queue/torque accumulation integration gates are
still required before accepting this draft as production-complete.

## Root and independent review, pending integration gate

Root freshly checked full raw82b36798..36980 and82d9a7e8..a984 against the implementation, including distinct reciprocal calls, split/full reads, ordered Y/X/Z torque stores, late AI bit, queue record order, and all-Parts drag. Independent graph_execution review found no concrete discrepancy. Root directly read SMALL_FLOAT_163 at82341f10 as37800000. Registered modules and original same-pair queue-order assertion are now mounted. Clippy90/fmt42 PASS; workspace57 is running. diff42 failed only a root-added blank EOF in RESEARCH_LOG; diff43 passes after trimming it. This is not yet an accepted physics checkpoint.

Accepted: workspace57=4059units+6docs, original49=17/17 8.43s with actual queue order and successful same-Part6 torques/drag; Clippy90/fmt42/diff43 PASS. Four new actual-slot tests pass. Original fixture remains neutral and does not yet run complete Ground/HangUps/force-consumption/solver sequence.
