# SkeletonController ground entry on the retained Skeleton

Read-only source/code audit, 2026-09-08. Original debug XEX/PDB, fresh raw instructions. No Rust or Cargo changes. This closes the evidence for the mandatory PhysicsGround.Enter collision stage82d99c7c; it does not establish a complete physics-state scheduler.

## Actual constructor and startup state

There is no separately emitted SkeletonController constructor function in this image. The actual constructor is inlined in PhysicalPlayerHiLOD construction:

| EA | Store/effect |
|---|---|
|82e0447c|Allocate0x14 bytes after earlier actual Skeleton.Init4228 and SkateboardController construction437c. Null allocation branches to null owner publication.|
|4488/4490|Read SAME processed pointer+2b0 and Skeleton pointer+2fc.|
|4494|mState+0 =0, SCS_NOT_YET_SET.|
|4498|mStoredState+4 =0.|
|449c|mStoredStateValid+10 =false.|
|44a0|Bind processed+0c.|
|44a4|Bind Skeleton+8.|
|44a8|mCollideWithWorldOnly+11 =false.|
|44ac|mNeedAirCollision+12 =false.|
|44b8|Publish actual pointer in HiLOD+2ec.|

All zero stores use r25, established by raw `li r25,0` at82e03c7c. Allocation padding is not initialized by these stores. A retained Rust owner should distinguish allocation from this explicit constructor stage, keep actual same-pair bindings, and never infer construction from a default enum.

PDB enum values: NOT_YET_SET0, WORLD_ONLY_OFFBOARD1, WORLD_ONLY_ONBOARD2, WORLD_ONLY_RAGDOLL3, OFFBOARD4, OFFBOARD_AIR5, STANDARD6, ONBOARD_AIR7, RAGDOLL8. Native initial Sleeping→PhysicsGround occurs after this constructor. Therefore original startup STANDARD6 sees actual worldOnly=false and oldState0, a legitimately source-established branch.

## SetSkeletonCollisionState and source failure prefixes

SetSkeletonCollisionState82dd9e70 first reads worldOnly+11 at9e7c.

- True: publish requested storedState+4 at9e8c, then storedValid=true9e94. Dispatch STANDARD6 to EnterState(WORLD_ONLY_ONBOARD2) at9f20. Both stores precede downstream collision effects and must survive any failure. OFFBOARD4 maps to1, OFFBOARD_AIR5 and ONBOARD_AIR7 map to2, RAGDOLL8 maps to3. Requests1/2 assert at9eec; other unsupported requests also assert after the stored-state prefix.
- False: read current mState9f78. Raw signed comparisons permit current<=0 or current>2 to call EnterState(request)9f08. Only old states1/2 assert at9f9c. Do not broaden this guard to 'all world-only states': current3 is permitted by the actual comparisons.

EnterState82dd4788 publishes mState=request at47a4 **before** its dispatch. STANDARD6 calls SAME Skeleton.SetGroundCollision at483c. WORLD_ONLY_ONBOARD2 calls SAME Skeleton.CollideWithWorldOnly at4894. A failure leaves the new raw state selected; an enclosing completion/replay guard must not undo that store or authorize ordinary continuation.

For later source producers of worldOnly:

- StartCollideWithWorldOnly82dd9db8 reads current state, stores storedValid=true9ddc then storedState9de4; chooses target1 only for old OFFBOARD4, otherwise2; EnterState9df0, then worldOnly=true9df4. If Enter fails, worldOnly remains its previous value and the stored-state prefix remains.
- StopCollideWithWorldOnly82dd9e10 captures storedValid9e2c, clears worldOnly9e30; true enters storedState9e40 then clears storedValid9e44, false enters STANDARD6 at9e50. A failed true branch retains storedValid=true after worldOnly was cleared.

These functions are actual callers in the later HiLOD constructor world-only setup (04d78/04da4). They are not needed to assume fresh worldOnly=false; that comes from the constructor itself. Supporting the world-only branch requires these real producer methods or an explicit externally controlled diagnostic boundary, not a setter that manufactures state.

## Reuse the existing NormalCollision mutation

`SkeletonRuntimeState::try_set_up_normal_collision` already ports **SkeletonBody.SetUpNormalCollision82b50400**. `SkeletonNormalAssembly::try_set_up_normal` calls it on the SAME retained runtime state when its actual normal-collision attribute admits. Do not duplicate this loop and do not invoke broad SetUpNormal from ground entry: that broader function also changes joint limits, culling, drag, material/inertia and compliance.

Raw NormalCollision sequence:

1. Capture actual Assembly, count and Part list; Assembly group=6 at50424; ascending every actual Part userdata=6 at5043c.
2. Folded helper50450, then Volume0.groupID=4 at50468; root flags&=~1 at50480; root disableCount=0 at50484.
3. For Parts1..23, read that actual signed disable counter5048c. If zero, folded helper5049c, Volume flags|=1 at504b4, groupID=0 at504bc. Nonzero preserves both fields.
4. After the optional volume writes, fresh Assembly/Part access and repeated Part userdata=6 at504d4; then source bound assertion and next part.

The existing state owns `volume_headers[24]`, `disable_counts[24]`, `assembly_group`, `part_user_data[24]` and the actual collision binding. It already preserves failure prefixes and tests zero/nonzero/unknown counters. Its explicitly bounded unknown-counter join is permitted only when both native branches already yield enabled/group0; it preserves None and disclaims an exact native conditional-read/store trace. Do not relabel that join as a known source counter. Actual selected TeleportReset populates all counters Some0 and active=false, so a completed constructor can provide genuine known counters without guessing.

## SetGroundCollision outer suffix and confirmed folded helper

Skeleton.SetGroundCollision82b543d0:

1. Capture actual SkeletonBody+1930 and call existing NormalCollision543ec.
2. Store Skeleton.mSkeletonProxyVolumeEnabled+3e43=false543f4.
3. Fresh SkeletonBody capture543f8, folded helper54400.
4. Read actual root Volume pointer Body+bb4; flags&=~1 at54410, then root disableCount Body+1e00=0 at54414. These repeat part of NormalCollision after the proxy store; do not remove them as redundant source effects.

**The call target at54400 is825187c0. Fresh raw word4e800020 is exactly `blr`.** The PDB-folded AIProcess::OnEnd name is not an unimplemented AI collaborator. The corresponding calls50450/5049c and world-only helpers resolve to this same folded empty body. They require no fake service and no side effect.

The existing `SkeletonPostPhysicsRecords::proxy_enabled` owns the SAME +3e43 field. Its constructor comment records source false574a8 and Init false62b08; actual post-physics reads it. Add a narrow source mutation there and call it from the existing Skeleton owner. Do not add another proxy flag in a new collision-controller or runtime-volume structure.

## World-only onboard branch

Skeleton.CollideWithWorldOnly82b54370 first calls actual Body.CollideWithWorldOnly5438c; then repeats the same proxy=false54394, fresh Body54398, empty folded helper543a0, root flags clear543b0, root counter0 store543b4.

Body.CollideWithWorldOnly82b4fa80 first calls **EnableSkeletonCollision82b476f0** at4fa94. That helper visits Parts1..23 ascending, reading each disableCount; zero enables flags bit1 at47740 and sets groupID0 at47748, nonzero leaves header unchanged. Unlike NormalCollision, this helper does not set Assembly group/userdata first and does not touch rootVolume/count. Only after it completes, Body captures Assembly/count/list and publishes Assembly group3 at4faac followed by all ascending Part userdata3 at4fac4.

Do not implement this by calling NormalCollision then assigning group3: that would incorrectly alter root state and all group/userdata fields before a potential late unknown-counter failure. The smallest reuse is extracting the existing per-Part admitted-header operation from NormalCollision into the SAME runtime state, then composing EnableSkeletonCollision's order without a second owner or counter array.

## Actual bridge and publication requirements

The retained normal Skeleton and its physical pair already exist. Add a small SkeletonController owner under SAME PhysicalPlayerAssemblyOwner, explicitly initialize it at the selected native constructor stage after actual Skeleton.Init, and bind the existing processed/Skeleton occurrence. It needs only current/stored state, storedValid/worldOnly/needAir fields with constructor provenance. Use source-specific ground/world-only methods on `SkeletonNormalAssembly`; their effects reach its existing runtime state and post-physics proxy field.

`SimulationSkaterAssemblyPool::try_set_skeleton_normal` and adjacent owner mutations demonstrate the real bridge: validate backend/member/kind, invalidate refresh **before possible writes**, then borrow actual candidate.owner. A new controller-ground bridge should follow that invalidation pattern and validate same physical pair. Merely cloning owner through `try_with_jump_pair` is insufficient for volume mutations because that helper does not invalidate publication.

`SkeletonCollisionPublication::try_update_volumes` already reads actual runtime top-level flags and current Part userdata, then Assembly group. It intentionally does not rewrite inline Volume/group/surface/Part ID records: native Create versus Update behavior differs. Preserve this existing source distinction. The pool's active-frame and publication bounds must become stale before the mutation and must be refreshed through existing actual update before collision consumers can use them; source mutation failure retains that stale status and its partial field changes.

Minimum concrete implementation is therefore: retained controller ctor + selected SetState dispatch → existing NormalCollision on same runtime state → existing proxy flag mutation → repeated root-volume/count stores → actual publication invalidation/refresh. World-only adds EnableSkeletonCollision's ordered conditional-header loop using shared internal logic, then actual group3/userdata writes. No new collision provider, fabricated volumes, cloned counter state or broad SetUpNormal rerun is required.

### Selected controller implementation boundary

New `skeleton_controller.rs` retains the five selected fields as explicit unknown versus constructor/transition values. Its one-shot `try_initialize_constructor` establishes only the recovered inline stores; actual same-pair associations and scheduling belong to the enclosing physical owner. `try_set_collision_state`, `try_start_world_only`, and `try_stop_world_only` preserve the source state/validity/flag publication order.

The required `SkeletonControllerCollision` interface has only concrete ground and world-only-onboard mutations, no default implementations. The real bridge must execute those mutations on the SAME existing `SkeletonNormalAssembly` and invalidate its publication. Unit tests use an explicitly labeled orchestration recorder; they do not establish runtime collision behavior. Unsupported valid collision modes return typed `PendingCollisionMode` after the native current-state store; invalid enum states return a distinct assertion error at the same point. World-only invalid requests preserve storedState/storedValid but do not reach the current-state store. Only successful selected transition calls authorize their completion; raw current-state access after a failure is not a completion capability.

Five tests cover source constructor origins/replay refusal, fresh STANDARD admission, collision failure prefixes, world-only stored-request publication/validation, Start/Stop flag ordering, unsupported mode refusal and the precise old-state1/2 guard (state3 remains permitted). Root owns real bridge/owner registration and Cargo validation. No full PhysicsGround.Enter or ordinary state scheduler claim follows from this selected leaf.

## Fresh fingerprints

SHA-256, end exclusive:

| Range | SHA-256 |
|---|---|
|82e04488..82e044bc constructor/publish block|f0a1981b795e4809e20a411fe035d346b2f88dba0af6410c8d5d4a37b6441422|
|82dd9e70..82dd9fb4|eeb0307e791144ae7b9bcdb0d2045bf7f768b047c021597660d2d73b340cf2e3|
|82dd4788..82dd48dc|ac9b9104f0220a3c8c905b3b294d18342c0f2dbb9416035c8298acbc26e7fcca|
|82b543d0..82b54430|30e252b529b0121140e072b21efaecb89fefd701b78eed92e62e565d58ef593f|
|82b50400..82b50508|4dc1b4933aec1c7bf249e8dd2d0d288f5fbf8e1412732c9df579a9fa782165e0|
|82b54370..82b543d0|1c81429e58b77087140b93938fb13ea84a9be344a7286155b9d7e4aef9616c84|
|82b4fa80..82b4fae8|22c4465f2662d3f7d8bb1c0283e6a48176ece6a9962576b414e2259bee7cf8c1|
|82b476f0..82b47780|c261b21b96dd2620a8fd38d025ebc4c447e270203145e9d59bde75670df20fc6|
|82dd9db8..82dd9e10|289bf21d9c04722146f7ace3b4048d87ed7c9ea77ae186e3535cf9e867f14997|
|82dd9e10..82dd9e6c|7f431ef80e111f6c4b2f9cd956d4c10c13075ecfbf34c6b36dd5c91b03412128|
