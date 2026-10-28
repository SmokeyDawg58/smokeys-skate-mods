# PhysicsGround Enter: actual startup dependencies

Read-only audit, 2026-09-08, original debug XEX/PDB and fresh raw instructions. Scope is `PhysState_PhysicsGround::Enter` **82d99c30..82d9a014**, after the real retained controller's inactive Stop. No Rust implementation or Cargo run in this audit. Broader DMO remains parked.

The actual HiLOD constructor selects Sleeping then calls SetPhysicsState(100); SetPhysicsState selects its already constructed PhysicsGround object before calling Enter. A failure here must retain that selected object and all preceding transition stores, without authorizing Update or category publication as a completed startup. See physical-player-state-owner-runtime-plan.md for that transition.

## Source entry order and actual owners

| Source stage | Effects and required owner |
|---|---|
|9c5c|SAME board.SetStandard, detailed below. This is the first missing production entry effect.|
|9c64|Skeleton.EnableIK82b462e0: load actual SkeletonIK and store mDoIK=true at462e8. Existing retained SkeletonIkState owns this field; add a narrow setter there, not another enabled flag.|
|9c74|Actual Skeleton.mUsePhysicsTruckTilt=false. Retain on the actual normal Skeleton owner.|
|9c7c|Actual SkeletonController.SetSkeletonCollisionState(SCS_STANDARD), including its existing-state/collide-world-only branches. This cannot be replaced by setting one mode enum.|
|9c80..94|PhysicsGround mGrabSplineDesc.type=INVALID, objectID=0; JumpInfo.doJump=false; playerProvidedControls=false; spawnCoordinatesSet=false; wasPinning=false, in that order. Other JumpInfo/vector fields are untouched.|
|9c9c|SAME Reckoning.ResetBodySpin82dd4068: speed+0 at4070, angle+0 at4074. Existing Reckoning owns spin_speed/spin_angle, but its broad reset also changes normals/heading/lean; do not use it.|
|9ca4|SAME board.DisableBoardAnimation82b2a100, including actual hook-drive dynamics, detailed below.|
|9cc0..9d04|Capture processed ground normal+1d0, then processed deck angular velocity+2d0. Two actual GetSkateboardBody virtual calls at9ccc and9ce0 precede dot9ce4. Resolve second body's Assembly Part6, calculate normal*dot3(angular,normal), read current packed body omega, preserve its W using vrlimi mask1, store full packed omega9d04. This is an XYZ projection with surviving old body W, not an XYZ-plus-invented-zero write.|
|9d0c|ResetSpeedModel82b2a1d0: actual GetEffectiveTransform first, actual GetDeckVelocity second, dot3(velocity,effective.at), fabs, targetSpeed store. Existing effective transform/body velocity and ReckoningNumeric dot are reusable; retained board target-speed field and this ordered producer are missing.|
|9d14|ResetSpeedWobble82b26620: actual SpeedWobble owner active=false, then time/frequency/lastTilt/anglePos/effect/wobbleTilt=+0. No retained actual SpeedWobble owner found.|
|9d28..9da4|Fresh processed physicsCategory+8ec ==100 chooses global manual force scaling; otherwise ResetManual then RemoveVelIntoGroundForManual. Details below.|
|9db0..9dc8|Fresh lastCategory+8f0 ==200 invokes actual Skeleton.StartDeckLandingWobble(1). Skeleton wobble construction was explicitly left outside the existing normal constructor projection; no fabricated inactive wobble is sufficient.|
|9dd4..9f94|Fresh lastState+8e0 ==505 selects landing-on-deck force. This is an actual conditional force producer, not implied by current Ground category.|
|9f9c|SAME Pumping.Reset.|
|9fa8|SAME Wipeout.SetState(GROUND), conditional on retained previous Wipeout state.|
|9fb4..9ff0|Only after those collaborators: pushScalar=1, dampedTurn/timeInState/collisionTimer/timeWallRiding=+0; full collisionTorque+70, hangupForce+80, antiFlipTorque+90 zero; hangTimer/hangPulseCountdown/hungUpWipeoutCounter/antiFlipFrames=0; isPushAccelerating=false; antiFlipNudgePresent=false.|

PDB PhysicsGround size/fields include JumpInfo+d0, exitingFromWallRide+f0, isHuman+f4, and several fields not written by Enter. Preserve constructor origins and unknown fields. Enter does not reset the whole object. The existing generic PhysicsState enum and suggested-state helpers in physics.rs do not own these native fields.

## Smallest concrete first closure: SetStandard

Outer board.SetStandard82b355a0 calls Body.SetStandard355b8 then fresh Body.EnableCollisionVolumes355c0. Body.SetStandard82b32d70:

1. Clear byte Body+2098 bit80 at32d8c, preserving all other bits and any aliases in the retained Body state.
2. SetStandardAngularDamping82b2b818. Capture actual global attribute collection and its physicsdeck.default instance at global+94. Lookup key **8d3065ced858a7df**, index0, atb878; missing pointer uses DefaultDataArea(4). Source resolves Assembly pointerb88c before scalar readb894, Part listb898 before literal readb89c; rounded `scalar * f32::from_bits(0x426fffff)` b8a0; then Part6 body and its actual inertia, store angularDrag+24 atb8ac. No zero/nonfinite clamp. This is the GLOBAL default instance, not the own physicsdeck collection used by queued forces.
3. Assign actual Body-owned material identities to Parts in order **6 deck, 0 wheel, 1 wheel, 2 wheel, 3 wheel, 4 truck, 5 truck**, stores32da8..32df0. These are constructor-retained standard material records, not currently selected SurfacePhysics wheel materials.
4. EnableCollisionVolumes82b241a8 sets bit1 in actual mVolumes[0], [1], [2] at241b8/c8/d8. Then if actual numDeckVolumes>0, visit each aggregate deck child volume ascending, set its flag bit1 at24210, re-read count for loop comparison24218. Native aggregate volume stride60 hex; decompiler's 32*(i+2i) is address arithmetic, not three children per iteration.

Reuse `SkateboardBodyConstruction::materials`, `DynamicAssemblySession::try_set_part_material`, the actual publication's material resolution, and backend/rigid-pool `try_set_part_angular_drag`. Current construction materials are value records; give those SAME retained records stable material ownership for pointer binding rather than creating unrelated tuning copies on each call. Existing SurfacePhysics material identity mechanism is a useful pattern, not the owner to select here.

The actual serialized/runtime collision-volume owners and publication enabled flags must both observe the mutations. Body activation/contact-query code is not a substitute for volume bit writes. `SkateboardCollisionPublication` already derives enabled from volume flags during publication/refresh; identify the authoritative retained volume mutation API before porting. Do not re-create a collision publication that loses retained pointers, or claim SetStandard complete after only materials/damping.

## Skeleton collision controller and hook drive

SkeletonController.SetSkeletonCollisionState82dd9e70 reads actual collideWithWorldOnly+11 first. True stores requestedState+4, storedValid+10=true before dispatch; STANDARD is converted to COLLIDE_WITH_WORLD_ONLY_ONBOARD. False validates current mState range then EnterState(new). EnterState82dd4788 publishes actual state+0 at47a4 **before** calling Skeleton's selected collision mutation. STANDARD calls SetGroundCollision82b543d0 at483c.

SetGroundCollision calls actual SkeletonBody.SetUpNormalCollision543ec, then Skeleton.proxyVolumeEnabled=false543f4; fresh SkeletonBody, folded helper54400 (must independently establish it is empty), Body volume0 flags&=~1 at54410, disableVolumeCount0=0 at54414. Existing normal Skeleton construction/volume publication is reusable, but a retained SkeletonController and these runtime collision changes are not yet implemented. Do not assume collideWithWorldOnly=false from unrelated corpus setup: recover actual constructor stores before selecting the startup branch.

DisableBoardAnimation82b2a100 captures actual Body pointera104, writes outer board.animated=falsea110, then SAME hook linear typeHARD2a118/spring0a11c/damping0a120/strength0a124. It freshly reads Body againa128, writes angular spring0a12c/typeHARD2a130/damping0a134/strength0a138. Existing `SkateboardHookConstruction::dynamics()` is the correct actual shared drive payload; truck and newly admitted controller hand drives are different owners. Actual outer animated state must be retained alongside its source consumers. No full hook rebuild is required.

## Later branches and missing retained state

Manual category branch compares integer category to100, not current state ID. Ground uses actual global physics_manual.default at+C8, lookup **076582f2a4a7cd27** index0 at9d88, missing scalar DefaultDataArea(4), late read9d9c then ScaleManualForce. That leaf82b23ed0 rounds scalar*errorIntegral before scalar*torque; torque store3ee0 precedes integral store3ee4. Non-ground ResetManual82b23ef0 clears five scalars in order errorIntegral, lastTargetAngle, lastActualAngle, torque, updateTimeSinceReset.

Then RemoveVelIntoGroundForManual82d8d090 reads actual balance. Signed zero skips; NaN follows nonzero path (raw compare equality branch must remain explicit). It captures groundNormal+1d0 then removes deck6 velocity along normal. Only afterward reads flags8c8 bits27/28 and flags8c4 bit20 to choose front/rear ordering. Selected front removes Parts0,1,4; selected rear2,3,5. Each triplet has its own late ShowVelRemoval cvar/debug and real Part.GetTransform branch. Reuse existing body velocity projection/numeric operations after auditing RemovePartVelAlongNormalVector's exact dot/FMA/store graph; private physics.rs `project_on_plane` is an XYZ compatibility helper, not proof of full native operation order. Missing flags/producers must refuse rather than force balance0.

Landing-on-deck505 branch uses processed velocity+190, COM velocity+260 and upVector+220; source computes the tangential relative velocity, then global physics_feet.default key **d5e239291aa44155** at9e4c, runtime skateboard mass, fixed timestep reciprocal estimate/two refinements, then independently captures global feet and key **62a60baeae01ed89** at9f44. Both missing keys use DefaultDataArea(4). It publishes an actual SkeletonLandingForce record to the existing board force queue at9f94. The decompiler's VMX aliases are unreliable; this branch's full lane/reciprocal graph still needs separate raw review before implementation. Existing `FootForce` record, actual queued-force owner and ReckoningNumeric reciprocal graph should be reused. It must not become a caller-provided successful 'landing force' callback with no queue mutation.

Pumping.Reset82dd8a60 sets recordValid=false8a74; clears +20..48 scalar fields ascending8aa4..8acc; then full lastPosition+0 and lastNormal+10 zero8ad8/8adc, including W. Existing repo has no actual retained Pumping state owner; constructor/reset projection is a small concrete addition. Do not zero its unrelated debug signal storage.

Wipeout.SetState82dd8c50 compares requested against current. Equal does nothing. Different first publishes new state8c5c; Ground clears offBalanceTotal8c84, Air clears deckContactCounter8c74. Existing wipeout.rs supplies request ledger, enable controls and state suggestions; it does not implement this retained owner transition. A new state must preserve current-state provenance, so Ground cannot unconditionally zero offBalance on every Enter.

## Recommended implementation sequence

1. Close actual board SetStandard over same construction materials/inertia/volumes and publication; add only the missing flag and stable material ownership, with failure tests between damping, each Part store and volume writes. Runtime global deck provider must preserve source lookup/read timing.
2. Recover/retain SkeletonController constructor and implement selected STANDARD/world-only entry on SAME normal Skeleton, plus narrow IK/truck-tilt mutations. This is the next genuine mandatory entry blocker, not optional debug work.
3. Add narrow ResetBodySpin on existing Reckoning and DisableBoardAnimation on existing hook owner; implement deck angular projection with preserved packed W and source staged input reads. Add actual board speed/manual state and actual SpeedWobble/Pumping/Wipeout selected constructors/resets.
4. Only then coordinate the entry sequence on retained PhysicsGround, with category/last-category/last-state sampled from SAME processed owner at the source stages. Recover missing motion/control fields and optional branch math before enabling those branches. Initial Reset-produced values can be consumed when genuinely source-established, but must not be asserted to avoid a branch.

No successful Enter token should be returned before its selected branches and final stores finish. A failed entry must preserve its raw selected state object and prefixes, while refusing ordinary state Update/category completion. This plan does not replace Actor/control producers, complete SetPhysicsState, or claim playable freeroam.

### SetStandard selected leaf and concrete integration boundary

New `skateboard_set_standard.rs` stages this first closure using the existing `SkateboardAttributeRead` and retained global `SkateboardAttributes`; it does not bind a duplicate provider. The actual bridge must supply the same constructed board's global Deck instance. `SkateboardSetStandardServices` separates Body, Assembly, Part list, Part body, inertia and standard material captures. Every resulting operation must mutate the same retained owner and report actual errors. Unit mocks are explicitly component probes, not evidence that World execution is already complete.

Raw precision additions for implementation:

- Body constructor387bc reads byte2098,387c0 masks to low6 bits,387c4 stores: bits80 and40 become known false while lower bits are preserved. Existing actual `post_fields.free_wheel_enabled` represents bit80; reuse it. Do not introduce a second raw-byte owner or clear unknown lower bits.
- After damping, Assembly pointer32d94 is captured BEFORE embedded material addresses are derived: deck203c at32d98, wheel2030 at32d9c, truck2048 at32da0. Then Part list32da4 and first Deck6 store32da8. Retain all three actual material identities once; subsequent Part stores reload Assembly/list separately. No material value decode belongs in this stage.
- PDB mVolumes[3] is Body+1e04. Original creation stores establish top0=deck wrapper at33ab8, top1=shared truck Volume at33ff8, top2=shared wheel Volume at347a0. Thus the first three enables are not Part0/1/2. Shared truck/wheel geometry Arcs must retain identity across runtime flag changes.
- Body aggregate+3ec is freshly read for each child at241e8. Native child addressing masks the loop index to16 bits at241ec/f0, multiplies by stride60 hex, but increment and count comparison use the full signed index. The leaf passes this masked address index to its explicit child-volume RMW callback. Count is first read241dc and re-read24214 after every child write24210. Negative/zero initial counts skip children after all three top-volume writes.

Root's actual integration needs stable material records owned by the same construction; an actual mutable flag projection bound once to the same immutable CollisionDefinition shape identities; and revision-aware publication/bounds invalidation. Publication must read current runtime flags for shared wheel/truck and deck child records. It must preserve original Volume data as construction evidence rather than modifying unrelated copied query records. Source top deck wrapper also needs its retained runtime flag even though primitive publication iterates deck children.

Four focused leaf tests cover ordered pointer/scalar/material callbacks, distinct failure prefixes, changed late child counts/nonpositive count behavior, and negative-zero/nonfinite damping. Cargo and actual-board integration are parent-owned. This selected leaf does not by itself complete SetStandard on the physical board or PhysicsGround.Enter.

## Fresh raw fingerprints

SHA-256, function end exclusive:

| Range | SHA-256 |
|---|---|
|82d99c30..82d9a014|875fb4084f0571ae83e9c87e6bd1280f63000396bd964c37877aaa72f0b4fe73|
|82b32d70..82b32e08|e5d28d7542eac93e799b72f6ff6d4369c3cc55840fca9005b3eababc85432aa1|
|82b2b818..82b2b8c8|5c5bf09be6111285166134fbddd5983489f245f0f85675da21b02db065be62f6|
|82b241a8..82b24224|d0c7e7651fa71ad2bcd1fb2b80f53d57c7d1420a4311f9aec318202a3f7f34a8|
|82dd4068..82dd407c|8e12c248fa1f61a0e2a904db7b93d40198be2e59d212fbdae01a77c1078efd82|
|82b2a100..82b2a140|2612150bbde7fc7dac09655ce136a0e84e9729f24d14c43fda70fc2df514c9a9|
|82b2a1d0..82b2a240|8da26fd8184a2a4fa40f99865fae888547860fa0e2742198588e1e5ded09584a|
|82b26620..82b26650|2d24bc636083c167c2b2ef7a9da3fc35be21fffc889d212a2bdaa70068d221ac|
|82dd8a60..82dd8ae4|b2c176a211eb73e2d48a0f941629e06221fdf6554816a2cea4b0baf825333578|
|82dd8c50..82dd8c8c|8a0fe145c96426f0aedda42f99f798f74f7299ec4c79387a43e1050681a095cf|
|82dd9e70..82dd9fb4|eeb0307e791144ae7b9bcdb0d2045bf7f768b047c021597660d2d73b340cf2e3|
|82d8d090..82d8d258|600dcd936b687e3b10363af77ac452e1f5f3babb05a706e4811e3aa06e7e5a95|
