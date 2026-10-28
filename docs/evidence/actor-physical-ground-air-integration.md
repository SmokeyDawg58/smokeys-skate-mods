# Actor physical registration and Ground-to-Air integration

2026-09-12, root implementation and independent source review. Confidence
**high** for this selected source path; no complete frame or reference-trace
parity claim.

## Actual retained physical allocation

Actor allocation825ff2d8 calls HiLOD825ff2f4; cached member+200 is only stored
after return at825ff300. IPhysical82dfe5d0 registers the same object before
child construction: owner+4 at82dfe5f8, duplicate Find82dfe628 and Add82dfe658.
The registry key is the interface-name function address825ebe48 itself.
`actor_physical_mount.rs` now retains this same allocation and registration
before the factory builds its board/skeleton. It verifies actual manager and
entity issuance, and retains allocation if later registration fails.

The mount intentionally does not publish Actor+200. HiLOD constructor closure
still includes the missing owners listed in
[the remaining constructor map](hilod-constructor-remaining-closure.md).
The selected factory completion phase does not prove native constructor return.
Independent review by input/PhysOut agent confirmed source registration timing.

## Selected transition through Update2

`physical_player_ground_to_air.rs` connects the original selector to the
original Ground100 -> Air200 transition. It rejects an unrelated Actor before
selection, using the registered physical allocation's actual Arc identity.
CheckForStateChange82e005f8 runs once. Its existing pending200 continuation
retains framesSinceJump=100. It then performs SetPhysicsState82dff180:

1. Controller EnterPhysicsState200; zero state count/timer.
2. Separate Processed stores: timer0, last100, lastCategory100, next200,
   current200 (82dff1cc..204), before old Ground.Exit.
3. Actual Ground.Exit, then source Actor queries.
4. Publish destination Air object (82dff3b8) before actual Air.Enter.
5. Only on success, run the sole ClearWipeoutRequests suffix, matching
   Update2_SystemLogic82e01c40.

Mutex scopes allow Ground.Exit and Air.Enter to access the same retained state.
Failures retain preceding mutations. Air state and entry fields are not inferred
from the Processed state number. Input/PhysOut and ground agents independently
reviewed the transition order. Root freshly inspected SetPhysicsState and
GetPhysicsCategory82d83b90; grind category is400, wipeout category300.

## Independent Air collaborator review

Root freshly inspected raw HookEnableAngularDriveSoft82b24ad8 and caller
EnableAngularDriveOnly82b2aca8: fresh Disp read and multiply, rsqrt estimate,
fresh Str read, angular type1/strength/spring/damping, then linear type2 and
zero spring/damping/strength. The two VNMSUB refinements retain final negation;
zero spring still requests the estimate. Constant4560fffe and live cvar
initializers were verified. Host estimate/VMX floating-point differences remain.

Raw FootPlant Reset82db6760 establishes all four lanes of six zero vectors,
stored in110/120/150/130/140/160 order. Other vectors and surface are preserved.
FullReset82dbbae0 clears readiness before Reset. Raw SetUpFullCollision82b47780
confirms root group4/disabled/count0 precede group and all Part userdata6,
then conditional enables for Parts1..23. This differs from normal collision.
See [Air entry evidence](physical-player-air-entry.md) for layouts and branches.

## Original-data validation

`physical_player_ground_startup_corpus` now runs two independently constructed
world/Actor/physical pairs. The first preserves grounded/bail probes and the
selector's explicit pending200 check. The second executes actual transition
and request clearing. It uses actual five terrain misses, measured height0.2,
and the original Vault thresholds. Assertions verify Air entry complete,
last/current state100/200, source request clearing, no COM branch from Ground,
and exactly one selector counter update. No state is forced to skip entry.

All16 original-data tests passed in `%TEMP%\skate2-air-original.log`. The workspace
then passed3,642 units (3,397 core +245 other) and6 docs in
`%TEMP%\skate2-air-workspace2.log`. Fmt and strict Clippy passed (session50556,
`%TEMP%\skate2-air-clippy2.log`), as did `git diff --check`. This is a controlled
schedule, not a playable loop. Later constructor/bail/Motion edits need a new gate.
