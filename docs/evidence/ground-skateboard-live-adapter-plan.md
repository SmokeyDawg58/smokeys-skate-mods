# GroundSkateboard live owner adapter plan

Read-only source/repository review after the typed Reckoning correction. No
registered adapter or complete Ground.Update claim accompanies this plan.

## Native sequence and existing algorithm

Ground.Update calls Skeleton.UpdateGroundSkateboard at82da0ea8, before the
Pumping caller math owned by the parent integration task. Fresh IDA review of
82b5e468 and82b5ba88 confirms:

1. Set Skeleton.mUpdateHeading; read actual Skeleton.mTimeDelta, then run
   UpdateRootTransforms. This timestep is constructor.outer.time_delta, copied
   from actual animation input by ProcessData. It is not processed93c.
2. Assert predicted position/current root XYZ, compose current mapped volume0
   through the existing root graph, assert resulting orientation XYZ.
3. Publish PASSED processed orientation+0, then clear only8c4/00080000.
4. Obtain actual owning skateboard Body and freshly read Part6. Store physics
   skateboard then skate-root matrices. These copies preserve unknown W.
5. GeneralUpdate82b5ba88 reads RETAINED processed93c at velocity computation,
   updates retained animation velocity/last position, then actual hook owners.
6. Failed hook acceptance invokes actual ResetSkeleton, then reads8e8 to decide
   want-respawn. Read8e8 again for the non-Teleporting IK branch. Do not skip IK
   by replacing actual state with2be.
7. UpdateIKData/IK.Update, actual drives update, then late gravity964 read and
   existing24-part displacement algorithm. Only after complete GeneralUpdate
   reset NextTrajectory in source X/P/Y/Z store order.

The numerical/control bodies already exist in skeleton_root_transforms.rs,
skeleton_general_update.rs, dynamic_assembly_skeleton_general_update.rs and
its existing IK/drives children. Existing physical_player_grounded.rs and
skater_pool_grounded.rs only accept controlled/eager projections today.

## Required shared boundaries; no duplicate numerical body

- SkeletonRootBoard: add typed transform service with explicit known-array
  default for controlled callers; actual LiveBoard overrides with the same
  session.try_live_part_transform_typed(backend,6). Both root initial read and
  grounded suffix fresh read use the typed method. Current live raw host method
  must not be promoted to original full-lane data.
- Existing try_grounded_suffix: split one generic fallible output-sink body
  from its controlled mutable orientation/flags wrapper. Actual sink writes
  same processed orientation then clears selected known8c4 mask before the
  fresh Part6 read. A later failure preserves both completed writes.
- Existing GeneralInput: add fallible late reader for DT, current state,
  selected8c8/8d0 bits, and gravity. Controlled struct delegates to the SAME
  body. Actual reader uses retained processed owner. No all-fields snapshot
  before roots/hooks and no gravity lookup before drives finish.
- Existing IK contact processed trait: introduce fallible selected flag,
  optional contact bone, and copied typed effective-transform reads at their
  existing call positions. Keep controlled Ready compatibility. This avoids
  ready() requiring unrelated effective data before the native request/foot
  branches and avoids fabricating missing flags. A fixed processed-read error
  variant can extend existing error enums without adding a numeric algorithm.

A new physical_player_ground_skateboard.rs session adapter checks actual pair,
selected Ground state, same owner Reckoning and processed identity; the pool
provides its existing Normal owner plus actual constructor timestep. A single
borrowed processed adapter supports sink and late-reader operations. The
existing world IK query service must keep the same world/static-ground/DMO
owners. No new query world, ContactSpyFrame, or completed-Reckoning token is a
substitute for the source fields.

## Settings and remaining admission work

A live cvar adapter is required for existing General/Drives/IK/PrePhysics trait
methods: feet-on-deck debug, external-normal angle/blend/local/world limits,
offboard drive scalar/soft damping, contact debug/penetration/sideways debug and
offset/max offset/toe threshold, prephysics debug/enable/max leg/max arm, and
contact-offset enable. Reuse existing registered references where present;
recover missing initializer identities before reserving new slots. Sampling
must remain inside each existing method; source defaults are not values to
hardcode into the adapter. Debug singleton comes from the actual shared slot.

The source contact packet already documents six exact registrations at refs
8308e610,8308e1d4,8308e314,8308e2f0,8308ec74,8308ea98. Other initializer identities
remain to inventory against the current live registry before implementation.

Focused tests should prove orientation/flag writes survive a subsequent Part6
failure, no gravity read before drives completion, repeated current-state
reads at their separate source positions, and unknown attached W propagation.
The same original pair corpus must then run through this stage immediately
following actual inactive Handplants and before Pumping. This plan does not
claim that such a call has passed yet.


## Registered reference inventory

Slots93..110 were reserved after checking all existing93 definitions; none of
these18 labels were previously registered. Raw packets are retained in
artifacts/ground-skateboard-cvar-initializers(-raw).txt. All float callback
registers r10 and boolean callback registers r8 are zero. Table462c existing
UseXYZCentreOfMass remains between new OffboardDriveScalar and SoftDamping.

| Slot | Label | Ref | Init | Table | Default |
|---|---|---|---|---|---|
|93|kOffboardDriveScalar|0x8308e538|0x82f7d648|82fd4628|Value::Float(f32::from_bits(0x4000_0000))|
|94|kSoftDamping|0x8308e474|0x82f7d6f8|82fd4630|Value::Float(f32::from_bits(0x4348_0000))|
|95|kDrawPreFeetIK|0x8308e378|0x82f7d878|82fd4640|Value::Boolean(false)|
|96|kDrawFeetOnDeckIK|0x8308ed90|0x82f7d918|82fd4648|Value::Boolean(false)|
|97|kToeContactThreshold|0x8308ea98|0x82f7da08|82fd4654|Value::Float(f32::from_bits(0x3f33_3333))|
|98|kTargetNormalBlendSpeed|0x8308e200|0x82f7da68|82fd4658|Value::Float(f32::from_bits(0x3e4c_cccd))|
|99|kEnableIKContactOffset|0x8308e39c|0x82f7db28|82fd4660|Value::Boolean(true)|
|100|kMaxIKOffsetLength|0x8308e674|0x82f7dc68|82fd4670|Value::Float(f32::from_bits(0x3f33_3333))|
|101|kMaxIKLocalDelta|0x8308ea24|0x82f7dcc8|82fd4674|Value::Float(f32::from_bits(0x3ccc_cccd))|
|102|kEXIK_MaxAngle|0x8308e0a0|0x82f7dd28|82fd4678|Value::Float(f32::from_bits(0x3f35_c28f))|
|103|kDrawIKContactDebug|0x8308e610|0x82f7dd88|82fd467c|Value::Boolean(false)|
|104|kMaxIKContactOffset|0x8308ec74|0x82f7ddd8|82fd4680|Value::Float(f32::from_bits(0x3e99_999a))|
|105|kDOPrePhysIK|0x8308e8dc|0x82f7de38|82fd4684|Value::Boolean(true)|
|106|kMaxLengthTestLeg|0x8308e2c4|0x82f7de88|82fd4688|Value::Float(f32::from_bits(0x3f4c_cccd))|
|107|kMaxLengthTestArm|0x8308e3e4|0x82f7dee8|82fd468c|Value::Float(f32::from_bits(0x3f80_0000))|
|108|kMaxIKContactPenetrationOffset|0x8308e1d4|0x82f7df48|82fd4690|Value::Float(f32::from_bits(0x3e99_999a))|
|109|kSidewaysOffset|0x8308e314|0x82f7dfa8|82fd4694|Value::Float(f32::from_bits(0x3e8f_5c29))|
|110|kSidewaysOffsetDraw|0x8308e2f0|0x82f7e008|82fd4698|Value::Boolean(false)|


## Implementation checkpoint (awaiting integration gate)

The new live child physical_player_ground_skateboard.rs is registered below
physical_grounded, so it reuses the existing private world-query service
without widening session/query fields. Its paired child verifies constructor
processed/physical bindings, borrows actual constructor.outer.time_delta, and
uses one mutable processed guard with a RefCell view shared by sinks and late
readers. It invokes the existing complete General algorithm through its new
read/write interfaces; the controlled wrappers delegate to those same bodies.

The only new retained field is Processed.mSkateboardOrientation+0. PDB confirms
that offset and separates deck+40. Reset raw390f0/390fc/39100/39108 stores
Y/X/Z/Pzero; generic unconstructed new remains None. Source orientation and
selected8c4/00080000 writes precede the fresh typed Part6 call. The actual
Reckoning reader now consults the complete known word, not the jump-only
00600000 diagnostic projection, for its selected board-flip00100000 read.

IK status reads selected8c8/600 after DataIn publication. The existing blend
body now reads selected8d0/10000000 separately inside each mode1 foot branch,
after four attributes. Contact uses a fallible live-read trait; old controlled
providers adapt through a blanket implementation. Request42000000 and later
valid-contact01800000 reads remain separate. Effective transform is copied only
at ContactLineTest. No incomplete field is replaced by a fake value or miss.

ContactLineTest's sideways-debug target is captured before dot and the SAME
Target Arc is loaded after dot. The actual outer World adapter forwards the
capture hook and intercepts query dispatch; invoking the inner settings-only
adapter as a world query returns explicit WorldQueryRequired.

Separately approved cross correction: raw82ddd670 word116c682f is standard
VNMSUB with VD11/VA12/VB13/VC0. Inputs are J.YZXW, rounded J*ground.YZXW and
ground, then output is permutedYZXW. Therefore each lane is
-(a[k].mul_add(b[j], -rounded(a[j]*b[k]))). This final-negation graph replaces
the algebraically similar negated-first-input FMA in the single shared helper.
It preserves the native negative exact-zero result; the previous signed-zero
audit item is resolved for this cross helper only. Other VMX boundaries are not
implicitly certified. Existing W residual oracles remain; a new all-zero cross
oracle explicitly expects80000000 in all four lanes.

New focused tests exercise actual processed orientation/flags surviving a
failing fresh Part6 call, typed unknown Part W, selective contact admission,
per-foot late blend read failure retaining the first foot store, exact18 cvar
defaults/live target changes/relink capture, and cross signed zero. No Cargo run
was performed by this implementation agent; root gates determine acceptance.
