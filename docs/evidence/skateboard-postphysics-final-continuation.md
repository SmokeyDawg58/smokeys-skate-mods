# Final board PostPhysics continuation

2026-09-08 read-only source/runtime audit. No implementation or Cargo.

## Exact remaining Body stages

After the average-normal stores through82b37498, raw3749c loads the SAME
CollisionInfo.numWheelsInContact byte Body+361. If nonzero, fmr374b4 chooses
the retained source +0 constant. If zero, lfs374a8 reads Body+1e00 and
fadds374ac computes oldTimer+the original passed dt (f25). Store374b8 commits
Body+1e00. Then call SetTruckDriveDynamics82b2cb08 at374c0, followed by
SetTruckDriveFrames82b34fa8 at374c8. The remaining instructions restore registers
and return. There is no fresh input/cvar/timestep read in this suffix.

The timer is not a contact-turn blend timer. Native constructor store386d8
initializes it with source zero; it needs a distinct retained body field.
The sole f0 definition before386d8 is lfs3830c of __real_00000000. Neither
CollisionInfo::Reset nor Body::TeleportReset82b2e7f0 writes this timer. The
latter zeroes velocities/forces, sets forceTurn and clears turn angles, resets
CollisionInfo and clears grip state. Do not add timer reset to that path.
The earlier entry dt assertion still applies. Do not add a clamp, finite guard,
epsilon comparison, or scalar saturation: the zero-contact arm is a rounded
f32 addition (including exceptional results); the contact arm replaces old
NaN/infinity/negative zero with +0 without reading the old timer. Timer store
survives dynamics failure; completed dynamics stores survive frame failure.

`skateboard_pool_session.rs::try_with_constructed_board_truck_drives` already
borrows the actual retained truck owner and its same attributes. Inside that
borrow call `try_update_dynamics(attributes)` then `try_update_frames(numeric)`.
These methods preserve the existing published frame/dynamics Arcs and source
partial stores. Do not create new drives, call the GroundAnimation steering
setter again, resample contact flags, or advance forced-turn timers here.

The separate `try_update_constructed_board_steering` bridge sets retained tilt
and force-turn state at GroundAnimation's earlier source stage. Its current
original corpus executes controlled scheduling, not an Actor scheduler. The
Body suffix consumes that retained state with both deck profiles already
supported by the dynamics leaf.

## Outer wrapper boundary

Skateboard::PostPhysics82b37d20..37e90 performs:

1. Actual Body getter, EndWheelLineTest37d48.
2. IsBadData37d50. True: store outer bad-data byte+101=1, InitializeTransforms,
   TeleportReset using current processed SkateTransformAnim+40; result code2;
   skip Body PostPhysics and UpdateUpVector.
3. Otherwise HasAcceleratedStrangely37d8c, then outer byte+102. If accelerated
   and that byte is zero: InitializeTransforms, TeleportReset using processed
   last SkateTransformAnim+80, actual SetVelocity(full zero), clear deck angular
   velocity XYZ while preserving packed W37e10..18; result code1; skip both
   ordinary stages.
4. Ordinary: read actual Reckoning owner outer+104, capture its full up+1b0
   at37e3c, then current ProcessedPhysIn dt+93c at37e40; Body getter and Body
   UpdatePostPhysics37e5c; UpdateUpVector37e64. Result remains0.
5. Common path resets force queue pointers37e78/7c/80 and returns result.

`SkateboardForceQueue::clear_post_physics` already implements the logical
occupied-range clear. It is the last source stage, not part of ApplyForces.
Its corpus calls are explicitly controlled resets. Do not move it before a
failed Body/update-up continuation. The bad-data/acceleration recovery branches
are real dependencies of a claimed complete outer wrapper, not permissible
hardcoded false gates. The Body-normal checkpoint alone need not claim them.

## UpdateUpVector is a distinct remaining consumer

UpdateUpVector82b26158..2661c is not present as an actual retained runtime method.
It uses outer dynamic-up+70, dynamic-up-speed+80, contact-acceleration+90,
and average-ground-normal+a0 storage (fresh raw/PDB correction);
the current `SkateboardForceRuntime.jump_normal` is another source field and
must not be reused as this state.

At26190 it rereads the actual current Body wheel count. Zero count stores full
zero only to contact-acceleration+90 at261c8 and returns; it preserves +70,+80
and+a0 and reads no
Vault tuning. Nonzero count has the following concrete dependency graph:

- Capture actual Body CollisionInfo and global physics_default+e0 cached
  layout, load its first full vector26244. The source then tests contact bytes
  for wheels **0,1,2 only**, in that order. For each true byte, sum the full
  retained acceleration (Body+1b0,+1c0,+1d0 respectively), then subtract the
  captured layout vector. There is no wheel3 contribution in this function.
  NormalizeSafe with full-zero fallback262c4.
- Global physics_reckoning_default+e4 lookup
  `ab8f93ba080e0cb3`26308, missing DefaultDataArea(4), scalar read26328;
  multiply old full dynamic-up-speed by scalar, store26358. Subsequent failures
  retain that damped speed.
- Compute normalized target minus retained dynamic-up. Lookup
  `9f15310dbbe96b8c`263a4, scalar263c4; multiply correction full4.
  Read CURRENT ProcessedPhysIn surfaceSpeed, take fabs263e8. Lookup
  `20f1eca637708494`26434, scalar2644c, then rounded division26450.
- Lookup `30bf4da4bd7ae00a`26490 and retain its pointer. Separately reacquire
  the global reckoning layout, evaluate its eight-point X/Y arrays+460/+480
  with that ratio at264e4. Only afterward read the retained scalar pointer
  and multiply by curve result26504. This requires staged pointer/value reads,
  not one eager tuning snapshot falsely described as native timing.
- Correction length uses the existing estimate/two-refinement/zero-select
  graph. fneg2655c then fsel26560 selects zero for nonpositive length and the
  length otherwise; NormalizeSafe correction26564; rounded cap-minus-length
 26568 and fsel26594 choose the smaller in source exceptional order. NaN follows
  fsel's negative arm; ordinary min/max helpers are not a justified substitute.
- Full correction-direction*selected scalar FMA into the damped speed,
  store265b8; full add speed into dynamic-up, store265c4; NormalizeSafe that
  retained vector with full-zero fallback265c8, then normalized full store265d8.
  Finally reread actual Body, copy its average normal Body+40 to outer+a0
  at265f4/265f8. No dt multiplication appears in this function.

Existing `reckoning_numeric` supplies NormalizeSafe and length refinement;
`physics.rs` PointGraph evaluation is already recovered. Reuse these rather
than adding new smooth-up mathematics. Before implementing this consumer,
independently audit exact VMX aliases/operand ordering for the full4 update and
recover original +80/+90/+a0 constructor/reset writes and cached vector layout
identity. Current contact acceleration has known XYZ and potentially unknown W;
retained-vector readiness must survive this consumer too.

## Proposed minimum integration

First add source-initialized timer to the same actual body post fields, and a
completed-normal-token continuation that reads current wheel count, stores
timer, then borrows SAME truck owner for dynamics and frames. This completes
the remaining ordinary Body suffix without changing the earlier steering API.
Keep failure phase/one-shot ownership around the aggregate so a late frame
failure cannot repeat timer addition and dynamics writes implicitly.
Refuse an absent/stale completed-normal endpoint before the timer read/store;
this is a host source-prefix certificate, not a new native assertion or math gate.

Next add actual outer up-state and staged original Vault provider, fed by the
same collision acceleration/contact arrays and current processed surfaceSpeed.
The existing controlled force-input snapshots are not a recovered live producer;
the PhysOut ground-normal publication is not the processed speed producer.
Only after up-state completion can the ordinary outer wrapper clear its actual
queue. General Actor/state scheduling and exceptional recovery remain explicit.

## Debug source hashes

| Function | SHA256 |
|---|---|
| Body UpdatePostPhysics82b36a58..374e4 | a696abb4831797ccbd2cc4add314115011b0fa0636a67012c4b4855c6b7e9aba |
| SetTruckDriveDynamics82b2cb08..2d0e8 | 8146f9dc1f0e3007db282fc44c2c18b4d4ed25817cc947c4f72746d4ddb8533e |
| SetTruckDriveFrames82b34fa8..35270 | e94bd7528124e308c8f02a869ddc828d210826b54fcd8cd8de7235c7ac52a05f |
| Outer PostPhysics82b37d20..37e90 | b3c9a8114922683a336f64bb58aafa3a61b727dd9be163cc32efb5ae37042e49 |
| UpdateUpVector82b26158..2661c | c8ee0b5f750ea86ef743fc640dac985195a0db1645d619f3c3a90e9513ac236b |
