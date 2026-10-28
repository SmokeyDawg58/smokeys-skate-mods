# Retained physical-player state owner and runtime dispatch plan

Read-only audit, 2026-09-08. Confidence **exact** for cited raw callsites/vtables and **high** for decompiled entry/update dependency graphs; numerical subroutines identified below are not all implementation-ready. No code/Cargo. This follows skater-ground-position-state-runtime-plan.md and prioritizes a genuine retained state owner rather than a category fixture.

## Source owner, construction and identity

Native HiLOD owns one pointer to the current **PhysicsState at+2a8**, plus separately retained state objects. A transition selects an existing object; it does not construct/reset all state objects on each frame. The PDB base `PhysicsState` size20 holds board+4, Skeleton+8, Wipeout+c, SAME processed+10, SAME Reckoning+14, TrajectorySelector+18 and SkeletonController+1c. These references are copied from the actual constructor arguments and PhysicsSystems snapshot.

PhysicsSystems size40 contains Reckoning+0,Wipeout+4,TrajectorySelector+8,GrindAirAdjust+c,FootPlant+10,HandPlant+14,PlayerGrabSpline+18,OffboardTrajectorySelector+1c,SkateboardController+20,OffBoardFeetIK+24,LandingOnDeck+28,Pumping+2c,OffboardLocationTest+30,OffboardJumpingTest+34,SkeletonController+38,GrindJumper+3c. Existing pair-owned board/Skeleton/processed/Reckoning are four real reusable owners; absence of the remaining objects is not equivalent to empty/default instances.

Selected actual constructor order, with native null-allocation branches retained:

| HiLOD field | Object | allocation/constructor/publication |
|---|---|---|
| +250 | GroundAnimation, size60 | alloc82e04630; ctor82d83dd0 call0464c; publish04658 |
| +254 | PhysicsAir, sizec0 | ctor82d8a9e0 call04688; publish04694 |
| +258 | KnownAir | ctor82d8b578 call046c4; publish046d0 |
| +244 | PhysicsGround | later ctor82d8cef0 call04b98, explicit original isHuman argument; publish04ba4 |

They occur among additional source state allocations, not as an exclusive contiguous four-object constructor. At each call, r4 is same HiLOD processed+2b0, r5 board+2f8, r6 Skeleton+2fc, r7 stack PhysicsSystems; PhysicsGround additionally receives actual caller isHuman r8. Do not infer Human merely from selected map/pad input.

Current pointer starts null at03e94. Constructor completion loads already retained **Sleeping+240 at04d00**, publishes current+2a8 at04d0c, then calls **SetPhysicsState(100)04d10**. Thus native startup is Sleeping?PhysicsGround. GroundAnimation103 is a later source-selected state, not the constructor default. Failure during PhysicsGround.Enter leaves its already selected pointer and earlier transition mutations retained; no completion capability should be minted.

## Virtual dispatch table, including folded functions

Vtable slots: destructor0, Update4, Enter8, Exitc, PredictFutureOfDeck10, GetType14, GetTypeString18, FillPhysOut1c, UpdatePostPhysics20, GetSyncLog24, additional28. Meaning comes from the vtable and PDB, not a misleading folded symbol name.

| State ID | Vtable | Update | Enter | Exit | GetType |
|---|---|---|---|---|---|
| Sleeping700 |823715ec|825187c0|825187c0|825187c0|82dfe588 returns700|
| PhysicsGround100 |8234a338|82da0de8|82d99c30|82d84a80|8276cb50 returns100|
| GroundAnimation103 |8234a1b8|82d98d68|82d83e68|82d83ed0|82d83e50 returns103|
| PhysicsAir200 |8234a2a8|82d9f4b0|82d98dd0|82d84190|82db40e0 returns200|
| KnownAir201 |8234a2f0|82d99820|82d9f828|82d99728|82d8b730 returns201|

825187c0 is actual `blr`; IDA names it AIProcess::OnEnd due folding. It legitimately supplies Sleeping no-op Enter/Exit/Update, not permission to no-op the other states. GetType100/200 similarly have unrelated indicator-manager names but raw immediate returns prove their values.

## SetPhysicsState is source-ordered, not an enum assignment

SetPhysicsState82dff158:

1. Read actual current GetType and assert new differs atf190..f1a4.
2. Actual SkateboardController(+2d8)::EnterNewPhysicsState(new) **f1b0**, before timer/state writes.
3. Capture processed pointerf1b8; HiLOD stateCount+130=0f1c0; HiLOD stateTimer+14c=+0f1c8; captured processed stateTimer+974=+0f1cc.
4. Independently call current GetType againf1e0; fresh processed pointer, store lastPhysicsState+8e0f1e8; GetPhysicsCategoryf1ec; fresh processed store lastCategory+8f0f1f4; fresh nextState+8dcf1fc; fresh currentState+8e8f204. These are not processed.physicsCategory, which follows output/input publication elsewhere.
5. Call old state Exit **f218**. Failure retains all preceding fields and still selects old current object.
6. Actual IActorQuery.GetPlayerID and offboard telemetry/JPhy-state tracking followf250..f320, with a potentially second GetPlayerID callf2d0. Source-selected player ID and processed category cannot be replaced by guessed local-player0 or an opaque successful no-op. Telemetry implementation may be scoped as a supplied actual service, but preserve reads and JPhy storef318.
7. Select preconstructed object, e.g PhysicsGround current storef370, GroundAnimationf394, PhysicsAirf3b8, KnownAirf724, Sleepingffc90. Unsupported state hits source assertion after earlier effects. A safe-host preflight for absent supported owner must be explicitly distinguished from native prefix behavior; preferred continuation preserves prefixes and refuses at the source selection site.
8. Call newly selected object's Enter **ffd1c**. Only successful Enter can authorize ordinary subsequent update/publication. A host failed-attempt phase guard prevents silent replay; it is not a claimed native recovery algorithm.

Controller prerequisite is concrete: EnterNewPhysicsState82dd1d50 selects StartController only for state500..504 inclusive; all selected ground/air IDs call StopController82dd1710. Stop reads actual systemOn; if on and controller state nonzero, LetGoOfSkateboard1744 then state=controlled0 at1748; clear systemOn174c; then always validate relationship between systemOn and handHoldingSkateboard sentinel1768 (assert177c). Merely setting an `enabled=false` field would omit owned hand-drive release/invariant behavior. No production retained SkateboardController owner implementing this graph was found.

CheckForStateChange82e005a8: current GetType005d0?actual StateChanger.CalcSuggestedState005e0?compare; if different **framesSinceJump+128=100005f8 before SetPhysicsState005fc**. Called from Update2_SystemLogic82e01c40 at01c54; Wipeout.ClearWipeoutRequests follows01c5c. This counter write is a second producer beyond the late Update1 jump counter.

## GroundAnimation: nearest reusable state path

Constructor82d83dd0 copies base references; initializes JumpInfo mCentreOfMassHeight0 at3e40, mDoJumpfalse3e44, full vector3e48. PDB object fields: JumpInfo+20,jumpedThisFrame+40,jumpVel+50. Do not equate a `Default` helper's initialization with all native constructor fields; jumped flag and jumpVel are explicitly established by Enter.

Enter82d83e68 source effects:

- jumpedThisFrame=false3e90, full jumpVel+50 zero3e94;
- SAME Skeleton.mUsePhysicsTruckTilt=true3e9c;
- SAME board.EnableAngularDriveOnly(1)3ea8;
- SAME Wipeout.SetState(WIPEOUT_GROUND)3eb4.

EnableAngularDriveOnly82b2aca8 first executes HookEnableAngularDriveSoft82b24ad8, then SAME board body hook-drive linear parameters: type HARD raw2 (li r9,2 at82b2accc; store3cc at82b2acd4), spring0acd8,damping0acdc,strength0ace0. The hook drive is distinct from the two truck drives. Existing SkateboardHookConstruction retains the actual shared dynamics Arc and real pool linkage; modify that SAME owner, not cloned dynamics or a new drive.

HookEnableAngularDriveSoft has two actual cvar targets and the existing reciprocal-square-root estimate/refinement graph: capture Disp target24b08, read24b2c, rounded product with supplied strength24b34; rsqrt estimate24b44; later Str value24b60; stores angular type SOFT24b64,strength=Str*literalbits4560fffe24b6c,spring=product24b70, then completes zero-select/two refinements and damping24bb0. This is a concrete missing reusable entry primitive, not a hardcoded drive tuning preset. Full raw operation/NaN/default audit should precede implementation.

Update82d98d68 clears jumped flag8d80; SAME Reckoning.UpdateGroundStatesFixedHeading8d94 using actual processed ground/dynamic normals; SAME Skeleton.UpdateAnimatedSkateboard(input,true)8da4; Skeleton.mAddSkateboardErrorToSkeleton=true8db4; state.UpdateSkateboard8db8. Existing retained Reckoning and Skeleton/IK owner paths plus actual six-force bridge can service much of this, but source UpdateSkateboard steering/material/jump decision must select the existing leaves and preserve its retained JumpInfo.

Exit82d83ed0: board.SetLinearDrag(0)3ef8; RestoreWheelMaterial3f00; late JumpInfo.doJump3f04. True builds gravity/timeDelta adjustment from actual processed fields and retained jump vector, Toolkit_ClampJumpVelocity3f80, SetVelocity3f98. Existing material/linear drag/velocity owners should be reused; the VMX expression and clamp must be independently raw-reconciled before claiming jump exit parity. False branch is a real source result, never an imposed no-jump default.

Fill82d83fc0 checks actual jumpedThisFrame, then separately guarded Air.stateTriggeredJump=true84010 and full retained jumpVel?Air+8084044. Existing GroundAnimationState/prepare_ground_animation_jump has useful semantics but stores XYZ/positive-zero compatibility values and explicit host numeric limitations. Adapt/refactor that existing helper for typed full lanes and retained state instead of creating a competing jump solver or using it as a hidden parity shortcut.

## Mandatory PhysicsGround startup and air dependencies

PhysicsGround.Enter82d99c30 begins actual SetStandard9c5c, Skeleton.EnableIK9c64, Skeleton truck-tilt=false9c74, SkeletonController.SetSkeletonCollisionState(Standard)9c7c; local flags/description clears; Reckoning.ResetBodySpin9c9c; DisableBoardAnimation9ca4; actual processed basis/velocity reads then deck6 angular velocity projection with retained packed W; ResetSpeedModel9d0c/ResetSpeedWobble9d14; source category branches for manual reset/force scaling; optional landing wobble and landing-on-deck force; Pumping.Reset9f9c,Wipeout.SetStateGround9fa8; later local scalar/vector/counter clears. The selected source constructor already sets mExitingFromWallRide=false at82d8d040, but Enter/Update cannot be inferred completed from that byte.

SetStandard82b355a0 calls body.SetStandard then EnableCollisionVolumes. Body.SetStandard82b32d70 clears source flag, SetStandardAngularDamping, and assigns actual retained material identities in order deck6,wheel0,1,2,3,truck4,5. Reuse actual material owners/pool setters; a successful unrelated wheel material setup does not satisfy the rest of SetStandard.

PhysicsGround.Update82da0de8 uses ordinary Reckoning.UpdateGroundStates0e94 (with actual deck heading), then UpdateHandplants0e9c, Skeleton.UpdateGroundSkateboard0ea8, angle/pumping inputs and Pumping.Update1014, PhysicsGround.UpdateSkateboard101c, predicted-position update1024, then push-exit and grabbable-object branches. It is not the existing GroundAnimation six-force tail under another name. Initial PhysicsGround startup cannot be skipped just because GroundAnimation is smaller.

PhysicsAir constructor82d8a9e0 retains separate physics_airstates and physics_reckoning instances, FootPlantManager, a real COM trajectory with full-zero position/velocity/acceleration and -1 terminal field, landing UP, scalar flags/timers. Raw global/class/collection binding needs the usual independent provider audit; decompiler ABI is unreliable there. Enter82d98dd0 performs SkeletonController.InitAirCollisionUpdate, IK/collision mode, angular-only drive, FootPlant.FullReset, source local flags, actual board transform/processed groundPosAllStates and last state/category reads; conditionally computes/init COM or mount trajectory and heading updates; late debug singleton requirement. Existing `physics.rs::PhysicsAirUpdateState`, update_physics_air and trajectory helpers provide recovered algorithms, but do not establish this owner or all real collaborators.

KnownAir is a distinct object201 with its own retained attributes, GrindAirAdjust/FootPlant and trajectory/heading/flip fields. Do not alias it to PhysicsAir200 merely because both map to categoryAir.

## Actual scheduling and category output

Update3_State82dfe708 increments stateCount, virtual PredictFutureOfDecke740 and Skeleton predicted-transform stores, ResetPushAbort, board/state flags and optional PlayerGrabSpline.Sync, then actual selected state.Update at**e7d0**. SkateboardController.Updatee7d8 follows; then board.UpdatePrePhysicse808 and StoreForceDisplacemente820. Existing controlled suffix calls do not establish this full order.

Update5 later reads SAME current object's GetType at82e055f4 and maps/publishes category through guarded State+0c store56b8. This produces the next Update1 early snapshot2008. Category must come from that owner and transition completion, not an independently supplied enum. Native raw current pointer is already selected before Enter completes; a safe-host API should distinguish raw inspection from completed-state dispatch.

## Concrete bounded implementation recommendation

1. Add one retained physical-state owner under the SAME PhysicalPlayerAssemblyOwner, with separately retained typed selected state instances, explicit absent/constructed state, same processed/board/Skeleton/Reckoning identity and phase/progress. Reuse existing GroundAnimationState and PhysicsAirUpdateState where their field fidelity permits; extend them instead of duplicating their calculations. Preserve uninitialized constructor gaps and full-vector provenance.
2. Implement actual small entry primitives first alongside owner storage: SAME hook-dynamics angular-soft/angular-only mutation with cvar/numeric ports, SAME Skeleton IK/truck-tilt/error flags, and actual Wipeout state storage where source ctor is recovered. These create concrete effects for GroundAnimation.Enter. Do not mark startup complete until required PhysicsGround.Enter and controller Stop collaborators run.
3. Implement source transition coordinator against actual retained state objects and typed staged services, including repeated GetType/processed reads, Exit, telemetry and pointer-before-Enter publication. Missing collaborators must refuse at exact stages and retain prefixes. Avoid a broad callback `enter_state(id)` that merely allows callers to claim opaque success without actual owner mutation; its implementations should be the source-specific state entries.
4. Close genuine Sleeping?PhysicsGround constructor transition next: recover/port controller Stop + constructor invariant, standard board modes/collision enable, Skeleton collision mode, manual/speed/pumping/Wipeout entry dependencies on existing owners. This is the minimal honest startup, though larger than setting a state ID. Null allocation and unavailable states remain explicit failures.
5. Connect actual StateChanger selected-ground request producer using the existing suggested_state leaf only after proving all preceding global gates, flags/contacts/animation-animated query and late tuning reads. PhysicsGround?GroundAnimation branch occurs after wipeout/jump/revert/grind/offground priority and actual Skeleton.IsSkateboardAnimated, not a user-selected category. GroundAnimation returns to PhysicsGround when actual animation ends; jump count1 selects PhysicsAir200 or KnownAir201 via actual trajectory-ready bit.
6. Route owner.Update for the successfully entered state through existing kernels in native order, then owner.GetType/category output, and finally next input snapshot. Keep missing Air trajectory/managers and full Actor graph production visible. Generated Action/Motion state_graph modules model a different subsystem; they are not a replacement PhysicsState implementation.

This plan intentionally identifies concrete prerequisite effects rather than proposing another Reset-backed category publication wrapper. The next implementation can meaningfully establish retained identities and real GroundAnimation entry mutations with source failure prefixes, while the native startup transition remains explicitly incomplete until its genuine dependencies are supplied.

Fresh raw SHA-256 (end exclusive):

- GroundAnimation ctor82d83dd0..82d83e50: e6786b9d342be3c9d61032dcf66c38aa0d06bb642e974641bc389f7721da3b34
- GroundAnimation Enter82d83e68..82d83ed0: 4c18198bf6d586d66da398c919de5596406a3fa528946a5454ccb2afea9024c0
- SetPhysicsState82dff158..82dffd30: 5d4e7058377addcd36734c2a08eb12a5c54362611d8dcf8ba7fbb4e6d32e1eb5
- selected construction82e04620..82e046d4: 2a35bb27d32766736469b7db4198d2bce6e78e278dc0abea01dcf5a67e278de6


## Selected next dependency: actual controller constructor to inactive Stop

Fresh recovery after the initial plan: SkateboardController ctor82dd02a0 is size1d0. It resets retrieval data02c0 (three full I/J/K/zero transforms and four +zero time scalars, helper82dc73e8), binds **physics_skatecontroller.default** into instance+d0 (raw class7dea05dc70a1ed7d, collectiond7edbd362d7d2152 at02e4/02e8/02ec), initializes two actual DriveFrames+e0/+120 to identity quaternions gL and zero positions, and two DriveDynamics+160/+180 to raw NO_DRIVE0 with zero scalar fields. Source later initializes handHolding+1a8=2, binds actual board/Skeleton/processed/Island references+1ac/+1b0/+1b4/+1b8, state+1bc=Controlled0, systemOn+1c0=false, drive pointers+1a0/+1a4=null. **It then calls AddDrives03d8**, so those nulls are not the completed original constructor outcome.

AddDrives82dcc370 captures/reads kSKC_UseHandDrives, true asserts both prior drive pointers null and calls InitHandDrivesc3ec. Original cvar default is **true**, ctor82f9856c, section Physics_Skateboard_Controller, target reference830db1e4. InitHandDrives82dc7508 independently re-reads the cvar7524. On true calls the actual retained Island.GetRWSimulation7550; writes both retained dynamics slots to raw HARD2 but zero strengths/springs/damping, resets both pointer slotsnull; then:

- Skeleton.GetPartRigidBody(**3/left hand**)75ac; actual board.GetDeckRigidBody75b8; Simulation.AddDrive(deck,parent;hand,child;SAME slot0frames/dynamics)75d0; store nullable result75d4.
- Skeleton.GetPartRigidBody(**7/right hand**)75e8; fresh board.GetDeckRigidBody75f4; AddDrive slot1 at760c; store nullable result7610.

Native capacity-null results remain valid individual outcomes; a host admission error preserves prior slot. Original default must not be overridden to skip this real construction effect. These hand drives are distinct from hook/truck/skeleton drives already retained by the pair.

With actual constructor complete, StopController82dd1710 legitimately reads systemOn=false and skips LetGo. It still reads handHolding2 then systemOnfalse, tests `systemOn != (handHolding == 2)`, and passes (false != true). This gives a concrete usable dependency for Sleeping?PhysicsGround without inventing a false controller input. Later active Stop must actually execute LetGo before resetting state/system, or explicitly refuse there.

LetGo82dd03e8 is not a generic hand flag clear: DisableBoardAnimation0400, SetFree0408, real Body.EnableCollisionVolumes0420, fresh Body flag bit clear0444, optionally DisableHandDrive0458; returning state optionally SetVelocity(fullzero)04a0; processed throw flag optionally ComputeSkateboardThrowParameters0508, righting forces0594, actual force queue Add0594/05a4 and three TargetAngularDisplacementExact calls05b0/05bc/05c8. Those later enabled/offboard branches can stay explicit continuations while the original inactive constructor path is completed.

### Controller implementation, admission, and teardown boundary

`skateboard_controller.rs` retains the selected native fields and both actual mutable payload pairs. Allocation has unknown retrieval/flags/state/payload fields; explicit construction establishes them. Its generic drive type is concretely `SimulationDriveHandle` in the actual bridge. `SkateboardControllerServices` preserves separate cvar captures/reads, actual Island simulation capture, left/right Skeleton body reads and repeated deck-body reads. The enclosing pair must bind actual board/Skeleton/processed/Island identity before invoking this selected constructor. No separate cloned physical owners are created by this component.

Use `SimulationDrivePool::try_add_drive(&actual_body_pool, actual_deck, actual_hand, &same_frames, &same_dynamics)` (simulation_drive_pool.rs). This retains source capacity-null behavior before body validation and distinguishes host failure. The infallible Assembly backend adapter folds errors into a recorded nullable result and is not the suitable direct fallible bridge here. Admission success stores each nullable handle immediately; later errors retain the earlier handle and both payloads. One-shot constructor phases are host replay guards, not a native recovery algorithm.

The native HiLOD constructor establishes Skateboard at82e0413c, Skeleton at41c8, executes **Skeleton.Init82e04228**, and only later calls controller constructor **82e0437c** then stores its pointer+2d8 at438c. The controller admission belongs after actual Skeleton body construction, rather than at raw physical-owner allocation. It precedes physics-state construction/initial SetPhysicsState.

Controller destructor82dcc338 first calls **RemoveDrives82dcc34c**, then destroys its retained attribute instance atc354. RemoveDrives82dc7490 freshly captures cvar target74a8/read74ac. False returns without simulation lookup or removals, even if handles exist. True loads actual Island+1b8 and calls virtual+3c **74c8** (74cc is the result move), then visits slot0 followed by slot1. Each nonnull handle calls actual Simulation.RemoveDrive74ec, then clears that pointer74f0. Null slots skip only RemoveDrive. A host failure must retain the failing handle and all later handles; earlier successful removals remain cleared.

The selected Rust `try_remove_drives` is explicit and can clean admitted prefixes after a failed constructor. It is not automatic rollback or a Rust Drop implementation. The enclosing teardown must inspect remaining handles before destroying bodies/owner, including a late source cvar=false result; it must not silently force the cvar true. Attribute destruction follows actual removal stage. Active LetGo and offboard Start remain typed pending continuations at their actual first use. The selected constructor/inactive Stop implementation does not establish a completed overall HiLOD startup or PhysicsGround.Enter.

Six focused tests cover shared payload identity and native ordering; independent first/second cvar gates and actual capacity-null outcomes; late body refusal preserving the first drive and rejecting constructor replay; active release refusal before state/system writes; post-clear invariant failure; retained original attribute binding; and source-gated ascending teardown with partial failure. Root owns Cargo execution and actual-pair corpus integration.

Fresh raw function SHA-256, end exclusive:

| Function range | SHA-256 |
|---|---|
|82dd02a0..82dd03e8|9b6c085e7c1fbfb393ecfcff03d33f069f676fd7dce75a150c0b00879703393d|
|82dc73e8..82dc7490|8723d44a03d8846341d34fcfb98df94a6f938c36c1b0899c4e09e74f6f2abd22|
|82dc7490..82dc7508|973186198dbe0d924f80ed543d6e90adfab12fbbadbec678cf73bf2bc4fef467|
|82dc7508..82dc761c|eb7c045a34abc12dc5ceb18cac2de539afe3efa2a2e29eca7a89d7cf84fdf647|
|82dcc338..82dcc36c|22ac5b6b06cff69237c121741c791bb44f3d63af0dc3d5f827ad48f5e6c49633|
|82dcc370..82dcc408|3e590f9ebd6b3e4b26abb012fcd11a55f41804dfa325aefab53f75433baf289a|
|82dd1710..82dd179c|4a18a4dd9e8fe39c89f18feb3cbb140775f77a17935b9ca5ffdc2ee633ec7ff6|
|82dd1d50..82dd1d68|4402632967c7df114198ee63c08fd8291cb9410b247e8492609e6c862276a694|
