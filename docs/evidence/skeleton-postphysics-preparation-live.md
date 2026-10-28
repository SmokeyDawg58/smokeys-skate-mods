# Actual Skeleton post-physics preparation draft

2026-09-12. Exclusive new files only; no registrations, shared edits or Cargo.
Every new path was checked absent with Test-Path before creation. Parent owns
independent source review, integration and validation. Confidence: exact source
control flow/field routes; high draft mapping, not compiled or runtime verified.

## Draft files and actual owners

* `skeleton_postphysics_preparation_live.rs`: child of existing Normal assembly.
  Actual attached Part getters, retained body-local SkeletonBodyAttributes,
  existing ResetSkeleton and IK-init-reset methods, SAME general respawn flag,
  SAME constructor outer fields. Includes the mandatory actual IsBadData leaf.
* `skeleton_postphysics_collision_inputs_live.rs`: child of existing processed
  prior_output. Captures source system velocity separately before contact pull,
  then assembles actual collision inputs after the query. No caller-invented
  flags or ground vectors. Source branches read existing next-step storage.
* `skeleton_postphysics_history_input_live.rs`: input-cvar child reuses actual
  registered kUseXYZCentreOfMass target66 and SAME captured Processed Arc. The
  up payload is read only on the source projected-COM branch.
* Two test children: attached actual angular-velocity bad-data check, unknown-W
  affine propagation, full-W affine result, FMA cancellation; actual processed
  constructor vs reset, ordinary and special-ground routes, offboard lazy
  reads/override, and status-bit-set with an actual null grabbed-body target.

`skeleton-postphysics-shared-amendments.patch` is a review artifact, NOT applied.
It includes exact narrow field/method edits and source-ordered pool integration
requirements. No second contact producer, full-history owner or output buffer
is introduced.

## IsBadData and preparation

Skeleton.UpdatePostPhysics `82b575d0` calls Body.IsBadData `82b472e8` first.
Raw47300/04/0c constructs key `d2d74e3fdaf8ec6d`, read through actual Body+34
`Attrib::Gen::physics_skeleton` at47314, index0; absent data uses DefaultDataArea4.
Retained construction.body_reader LocalSkeleton is that existing occurrence.

Raw47354 selects Part23 body,47358 loads linear XYZ/packed W,4735c dot3,47368
ordered greater-than threshold. Equality and NaN do not take this early return.
Next Parts0..23 each load current linear XYZ, fresh body pointer/angular XYZ,
then actual GetTransform. XYZ self-equality checks reject NaNs only. Infinity
and unknown packed W are not invalid by this source predicate.

Taken bad-data branch57614 calls actual ResetSkeleton.5762c/30/34 clears IK
1d4 on-deck blends,1f4 modes and1e4 external blends (four iterations);57644
disables IK,5764c enables it,57650 unconditionally sets want-respawn. Existing
try_finish_skeleton_init matches these exact narrow writes. The failed-hook
respawn helper is state-gated and MUST NOT substitute for this unconditional
store; patch adds a separate method on the same general owner.

Fresh board Part6 position then subtracts retained predicted board position
Skeleton+3ce0; full-vector store is existing skateboard_error+3d50. Proposed
RootVector type retains unknown packed W. No homogeneous normalization.

kDrawSkateboardError: ref8308e4f0, initializer82f7c3c8, table82fd4538, section
Physics_Skeleton, label kDrawSkateboardError, title Draw SkateboardError,
defaultfalse/nullcallback. Live read remains mandatory; enabled call576d0
targets shared825187c0 BLR. No DebugOverlay or renderer callback is required.
New cvar index awaits root reservation; no static production bool substituted.

## History suffix and late collision input

UpdatePhysicsRecords cc4c captures actual board Part6 matrix BEFORE processed
up-pointer capture and existing history kernel. Source keeps that board P in
v127. After kernel, cc70 suffix reads OLD retained Skeleton COM+3d00 and SAME
world-to-animation. COM-local+3d40 store precedes reloaded matrix columns and
deck-local+3d60. Both use standard VMADD I*x+P, J*y+previous, K*z+previous,
not a transpose/rigid inverse and not a new pose query. Only XYZ are position
coefficients; all output W lanes depend on the full matrix W values. The same
outer fields are already consumed by frame agent's GetLaunchInfo draft; both
agents agreed the typed field amendment.

576dc reads Processed.physics+8e4; state201 selects PhysRecord COMvelocity
Skeleton+2840, otherwise outer skateboard_velocity+3cf0. Capture before
actual Island.GetCollisions bctrl5770c (57714 saves returned pointer). Actual
GetRWSimulation bctrl57728 follows, then info constructor57734.

After these calls,57744 loads primary GrabSpline status Processed+778. Bit40
guards body-pointer+788; an actual null pointer still leaves ignore-body null.
Processed embedded primary is distinct from PlayerGrabSpline's query result.
Its actual ctor is82e00f64 (alternatef6c), before Processed.Resetf78. Reset also
calls embedded GrabSpline.Reset; patch must reset an existing occurrence, not
manufacture one for generic new+reset. Reuse existing GrabSpline type, retaining
its source-preserved low5 status bits and length-unit value. Future nonnull body
targets remain explicitly unadmitted in that existing target representation.

Ground position: physics states601/602 select Processed+1e0, all others+1f0.
Raw577a4 captures flags8c8; extrwi577bc extracts bit0x00010000 danger,577c8 bit
0x40000000 AI. Category8ec is read577b8. Reckoning+1f0 supplies ground normal,
and OLD post-record COMvelocity+3d20 is read577e0. Offboard is category500 except
states502/505; those two states set offboard-air even under another category.
Category600 sets ground-trick. Only offboard true reads flags8d0; bit8 chooses
existing prior-input next-step position370/normal380. These are actual retained
fields from Update1; no new duplicate vectors are required.

Then use existing SAME pool collision->volume-states->post-record chain. The
old world helper accepts collision_info before history and cannot simply be
called with this late reader's result without changing source order. Do not
run history twice. Preserve existing completed-solve/frame/owner checks.

## Explicit remaining integration boundaries

The current history kernel stores full arrays and uses its existing host
canonical-W Part projection. The new affine suffix does not promote that host
policy into source evidence: its captured attached board P has W unknown, and
only XYZ feed this transform. Projected COM requires full actual Up W; the live
adapter refuses unknown W on that branch instead of injecting scratch zero.
Default true target66 does not read Up. General numeric estimates/dot reductions
remain the repository's explicit host policy, not Xenon bit parity.

Live collision cvar and post-record service adapters are not added by this
bounded preparation patch; their existing corpus fixtures remain controlled.
The new code is not a completed Update4 scheduler or proof that earlier solver
and board post-physics stages executed. First validate the connection on the
actual completed-frame collision corpus, then mount the real source caller.
