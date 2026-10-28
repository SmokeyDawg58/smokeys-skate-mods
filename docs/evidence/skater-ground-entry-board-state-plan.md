# Ground-entry board and Reckoning state

Read-only original debug XEX/PDB audit, 2026-09-08. No Rust/Cargo changes. Required order remains PhysicsGround.Enter: SetStandard9c5c → EnableIK9c64 → truckTilt=false9c74 → collision controller9c7c → state-local clears → ResetBodySpin9c9c → DisableBoardAnimation9ca4 → actual deck-omega projection → ResetSpeedModel9d0c → ResetSpeedWobble9d14. These are distinct source operations, not one broad reset.

## ResetBodySpin: existing Reckoning fields, not BodySpin collaborator

ResetBodySpin82dd4068 stores **mBodySpinSpeed+354=+0 at4070**, then **mBodySpinAngle+350=+0 at4074**. It leaves the separate BodySpin pointer+14, its internal filter/history, BodySpinTransform+160, BodyFlipSpeed+364, normals/heading/PID/animation state untouched.

The actual retained `PhysicalPlayerAssemblyOwner::reckoning()` already owns the SAME `Reckoning` used by board+104 and normal Skeleton. Its `spin_speed` and `spin_angle` map to these fields. Original Reckoning constructor stores angle then speed at82dd7e28/7e2c after its PID construction; this is a different order from ResetBodySpin. Existing `Reckoning::reset()` touches many unrelated fields and is not suitable.

Minimal API: add `Reckoning::reset_body_spin()` with speed then angle stores and narrow read access for validation; add `PhysicalPlayerReckoningOwner::try_reset_body_spin()` requiring actual constructed Reckoning and invalidating its current completion token before mutation. No new Reckoning or phase should be minted from this reset. Ground entry consumes the same owner even when no grounded-update completion exists yet; requiring a previous update token would impose a false native prerequisite.

Other actual callers are PhysicsAir.Exit82d841a0, KnownAir.Exit82d99800, SlideGround.Enter82d84c90 and Nonspecific.Enter82d86748. This is a reusable owner primitive, not ground-only state storage.

## DisableBoardAnimation: same outer flag and actual hook payload

PDB outer Skateboard fields: targetSpeed+e4, animated+102, Reckoning pointer+104, Pumping pointer+108, SpeedWobble pointer+10c. These are not SkateboardBody fields.

Original Skateboard ctor82b38918 establishes targetSpeed=+0 at38b10, animated=false38b14 (r30=0 at3893c). After actual Body creation/Part6 deck handle binding38c10 it repeats animated=false38c14. Preserve distinct selected constructor execution; raw allocation alone does not establish either value.

DisableBoardAnimation82b2a100:

1. Capture actual Body pointera104.
2. Outer animated=falsea110.
3. Captured Body hook-drive linear typeHARD2a118, spring+0a11c, damping+0a120, strength+0a124.
4. Fresh Body pointera128.
5. Hook angular spring+0a12c, typeHARD2a130, damping+0a134, strength+0a138.

No cvar or Vault reads. Do not use NO_DRIVE0. Do not change truck dynamics or the newly admitted controller hand drives. A host failure resolving/locking the first payload retains animated=false; failure on the fresh second Body/payload retains completed linear stores. Lock scopes should permit this source staging rather than prelocking all future payloads.

Existing `SkateboardHookConstruction` in skateboard_hook.rs owns the correct mutable `Arc<RwLock<SimulationDriveDynamics>>` initialized by native Body hook construction. The factory retains it in `PhysicalPlayerAssembly::hook_construction` and passes that same Arc at `try_connect_registered_hook`. Actual `DynamicAssemblySession::post_attachment_drives` retains only nullable Drive handle plus child guard; the pool retains payload Arcs for accepted drives, but a capacity-null result has no pool payload. Thus querying accepted Drive payloads is not sufficient ownership for this source Body mutation.

Minimal actual integration: retain/bind the SAME hook dynamics Arc on the actual board/physical owner at its selected construction stage, with occurrence/hand-off validation analogous to completed truck-drive handoff. Keep failed construction prefixes owned; do not reconstruct dynamics from definition/tuning, and do not create new payloads if admission was null. Then a narrow actual board DisableBoardAnimation method mutates that exact Arc and a retained outer animated field. No full hook or body rebuild is needed.

Other callers include controller LetGo82dd0400, LandingOnDeck.Enter82d9ed74, Grind.Enter82d850b8/Exit82d8fa50 and several ground/path/skitch states. It is a concrete shared source primitive.

## ResetSpeedModel: current actual getters, no stale processed snapshot

ResetSpeedModel82b2a1d0 is called only by PhysicsGround.Enter9d0c in this image:

1. Actual GetEffectiveTransform82b23778 at2a1f4.
2. Actual GetDeckVelocity82b25ff8 at2a200.
3. Read velocity and effective at/Z column, VMX dot3 at2a214, store temporary; scalar fabs and targetSpeed+e4 store2a224. Numeric is source dot then sign-clear, with no threshold, clamp, NaN rejection or guessed dt.

GetEffectiveTransform calls actual Body.GetPosition(Part6)2379c, then freshly reads SAME processed flags+8c4 bit20 at237b4. True XORs sign bits on all four lanes of matrix X and Z, preserving W bits and signed-zero/NaN payloads. The existing `SkeletonProcessedContact` toolkit producer already expresses this sign-flip math, and `DynamicAssemblySession::try_live_part_transform_host` provides the actual Part::GetTransform route; extract/reuse the expression if needed. Do not call the full toolkit sampler (it also samples mass and twice reads transforms), and do not replace this current getter with a retained previous processed effective matrix. Unknown source flag must remain a refusal at its read unless genuinely supplied by the actual producer.

GetDeckVelocity asserts actual Body pointer nonnull before resolving its Assembly/list/Part6/body. It loads actual Body+20 full packed velocity at26050 and returns it; current source XYZ can be read from the existing actual rigid-body getter, while W is retained unknown if the host does not represent it. W does not feed the surviving targetSpeed dot3, so no manufactured W is required to compute that scalar.

Minimal staged API: same board source `try_current_effective_transform` followed by `try_current_deck_velocity`, existing ReckoningNumeric.dot3, then store actual outer targetSpeed. A failure at velocity/dot preserves the previous targetSpeed; prior DisableBoardAnimation and omega projection remain committed. No separate speed physics implementation or generic movement controller is needed.

## SpeedWobble: real constructor owner and deliberately unknown amplitude

PDB SpeedWobble size30: own attribute instance+0; time+10, **amplitude+14**, frequency+18, lastTilt+1c, anglePosition+20, effect+24, wobbleTilt+28, active+2c.

Outer board constructor allocates this actual30-byte object38b8c; calls SpeedWobble ctor82b2eaf8 at38b98 and publishes pointer+10c at38ba4. Null allocation publishes null; no default wobble instance is silently substituted. This occurs before actual SkateboardBody creation, not during Ground entry.

SpeedWobble ctor first binds its OWN generated physics_speed_wobble.default instance at2eb24: class **5be46545de485c98**, default collection **d7edbd362d7d2152**, msgPort0. Generated wrapper82b25408 calls FindCollection25438 and Instance ctor25448, then if cached layout pointer absent uses **DefaultDataArea(0x100)** and writes that cache25460. Retain that actual immutable instance and fallback semantics for later wobble consumers; no tuning values are read by this constructor.

Then active=false2eb34; time+0eb3c, frequency+0eb40, lastTilt+0eb44, anglePosition+0eb48, effect+0eb4c, wobbleTilt+0eb50. **Amplitude+14 is not written.** Allocation, constructor and ResetSpeedWobble must not invent its value.

ResetSpeedWobble82b26620 reads existing owner pointer26624, then active=false26630, time+0/ frequency+0/ lastTilt+0/ anglePosition+0/ effect+0/ wobbleTilt+0 at26634..48. It leaves amplitude and the existing attribute instance untouched. No cvars or Vault reads occur during reset. The actual repo has no retained SpeedWobble owner yet; create a bounded owner with explicit construction and Option amplitude, not a new standalone reset calculation or fresh object on every entry.

## Existing Skeleton IK and truck-tilt owners

EnableIK82b462e0 loads actual SkeletonIK and stores mDoIK=true462e8. Existing `SkeletonIkState.do_ik` is the SAME field; its Reset already sets true but resets all contact histories. Add a narrow source enable operation on that state and route through actual retained SkeletonIk; never call its broad reset to enable IK. `SkeletonIk::state()` is currently readonly, so expose a narrow operation rather than arbitrary public mutable state.

Actual Skeleton truck-tilt field+3e41 ctor is **true**, not false: source r27=1 at82b57370 and store574a0. Ground Enter storesfalse9c74; GroundAnimation.Enter storestrue82d83e9c. No existing retained field was found in the normal Skeleton owner; add one there with explicit constructor provenance at574a0, and connect both later state setters when their source stages execute. It is distinct from controller truck-drive angles, processed tilt inputs, proxyEnabled+3e43 and post-record HookToHips.

## Concrete sequence to implement next

1. Narrow SAME-Reckoning reset and SAME-Skeleton IK enable; retained truck-tilt constructor/entry stores.
2. Retain exact existing hook dynamics on actual board owner, add actual outer animated/targetSpeed constructor fields, and implement DisableBoardAnimation with its two Body reads and partial-store behavior.
3. Retain actual SpeedWobble constructor owner/instance including unknown amplitude; implement its narrow reset without rebinding.
4. Add actual current effective-transform/deck-velocity service and reuse existing dot3 for ResetSpeedModel, preserving late processed flags and target-speed publication only after successful calculation.

The intervening actual deck angular projection in PhysicsGround.Enter remains necessary in its original position; the prior entry packet documents normal/deck-angular inputs and old packed-W preservation. These primitives are reusable prerequisites. They do not skip category/manual/landing/Pumping/Wipeout branches or complete PhysicsGround.Enter by themselves.

## Fresh function hashes

SHA-256, end exclusive:

| Range | Hash |
|---|---|
|82dd4068..82dd407c|8e12c248fa1f61a0e2a904db7b93d40198be2e59d212fbdae01a77c1078efd82|
|82b2a100..82b2a140|2612150bbde7fc7dac09655ce136a0e84e9729f24d14c43fda70fc2df514c9a9|
|82b2a1d0..82b2a240|8da26fd8184a2a4fa40f99865fae888547860fa0e2742198588e1e5ded09584a|
|82b26620..82b26650|2d24bc636083c167c2b2ef7a9da3fc35be21fffc889d212a2bdaa70068d221ac|
|82b2eaf8..82b2eb68|97624ddb8f6f7cd8d25a965ad67d80d4b1bef999417513614f44672643ed20e4|
|82b25408..82b25480|79dbee66d807c06ccb19d3d6f8f36ca0ef7b85873aa816c6ed540208e7067d01|
|82b23778..82b237f8|c798e843be385a3942929c594e5ac418f534651ea0e1c2c85d3864313e1787f7|
|82b25ff8..82b26070|0f0140ab7795becac137fad1dd563afd5aa92fde1f5387e665e46aa178682c9b|
