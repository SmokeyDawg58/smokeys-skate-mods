# Selected PhysicalPlayer Sleeping → Ground startup

2026-09-11. Native reconstruction against live `sk82_na_m.xex.i64` PDB,
decompiler and raw PPC instructions. Author: ground_path; root independently
reviewed allocator, speed/flip operations and same-owner mounts. Confidence:
**high** for the selected source control/data path; **exact** for the listed
instruction stores, bit insertion and allocation policy. This is not a claim
of deterministic per-tick physics parity or a complete playable skater.

## Why the local player's initial JPhy state is known

`Actor` allocates `PhysicalPlayerHiLOD` at `825ff2d8`: category `Sim` loaded at
`825ff2c8`, requested flags zero at `825ff2d0`, size `0x340` at `825ff2d4`.
The call reaches `XCore_EASTL_Category_Alloc` (`828b4bf0`), then the actual
category allocator obtained through `XAllocator::GetCoreAllocator`
(`8296f4e0`). `CategoryAllocator::Alloc` (`82973570`) combines category mask
and OR flags: mask load `82973580`, OR load `82973588`, AND `82973594`, OR
`829735a4`, `Internal::Alloc` call `829735ac`.

PDB `xcore::tCategorySetup` is 24 bytes: name `+0`, limit `+4`, two heap IDs
`+8/+c`, mask `+10`, OR `+14`. `MemCategoryMap::memCategorySetups` is the
61-entry static table at `82ff0988`. Its `Sim` entry at `82ff0eb0` contains
name pointer `821678a0` ("Sim"), limit `80000`, heap IDs `7/0`, mask
`ffffffff`, and OR `00000100` at `82ff0ec4`. `GetNextCategorySetup`
(`828b52a0`) supplies these actual entries to allocator construction;
`Internal::AddCategory` (`82975158`) installs mask/OR on each new category.

`Internal::AllocHeapMem` (`82972240`) tests **bit 0x100**, not bit 1, on both
the small-block and heap paths (`8297232c`, `829723a8`) and calls
`rw::core::stdc::MemClear` on the entire requested payload at `82972340` or
`829723bc`. Therefore the original Actor's successful allocation clears
HiLOD `+174` before its constructor. `IPhysical` base constructor
`82dfe5d0` only installs interface/vtable ownership; it is not the source of
this zero. The direct HiLOD `mJPhyState` zero store at `82e04dc4` occurs
**after** initial `SetPhysicsState(100)` at `82e04d10` and cannot justify the
earlier read by itself.

`PhysicalPlayerPhysicsStateOwner::from_sim_zeroed_allocation` records the
actual allocation-origin `jphy_state = Some(0)`. Generic `new()` retains an
unknown value. This allows the genuine local-player query branch without
using a nonlocal ID, skipping telemetry, or moving the late constructor store.

The separately allocated Ground object does **not** inherit this policy:
`82e04b74` loads `PhysicsSkater` for allocation `82e04b78`, flags 1, size
`0x120`. Table entry `82ff0d48` has mask `ffffffff`, **OR zero**. Thus its
constructor-unwritten fields remain explicitly unknown until entry writes
them. `PhysicsSystems` and the other inspected physics categories also have
OR zero; no general assumption of zeroed game allocations is valid.

## Retained objects and construction

`PhysicalPlayerAssemblyOwner` retains the selected state owner on the same
physical board/Skeleton pair. `PhysicalStateBindings` retains actual assembly
handles and a weak reference to that same outer owner; it creates no duplicate
Reckoning, Processed input, board, Skeleton or world. Sleeping and Ground
construction/publication remain explicit. Null allocation and repeated calls
are checked without turning absent objects into successful collaborators.

Ground constructor `82d8cef0..82d8d080` (call `82e04b98`, publication
`82e04ba4`) initializes its proven force vectors, collision/grab fields,
scalars, jump fields and flags. The `mIsHuman` byte at `+f4` preserves the
input byte, including noncanonical values. Hangup force `+80`, anti-flip
torque `+90`, hang timer `+100` and countdown `+104` are not written by this
constructor; they remain `None` before the final entry stores. PDB identifies
both timers as integers. Other `PhysicsSystems` collaborators, including
trajectory/handplant/grab-spline construction, remain outside this selected
field projection and are not represented by fabricated successful services.

Pumping is the actual shared HiLOD `+2c0` / board `+108` occurrence. Source
allocation `82e040bc`, constructor/reset call `82e040c8 → 82dd8a60`, and
publication `82e040d8` precede board construction. Reset clears valid byte
`+4c` at `82dd8a74`, eleven scalars `+20..48` at `82dd8aa4..acc`, then full
position/normal vectors at `82dd8ad8/adc`. Debug signal bytes are unrepresented.

Wipeout construction follows successful Skeleton initialization: inline
stores `82e04260/78` establish invincibility time/deck count zero, and Reset
`82dd4480` writes state `+bc` and request counter `+b4` only. Off-balance
`+b8` remains unknown initially. `SetState` (`82dd8c50`) returns immediately
on equality; otherwise it writes the new state first, Ground 1 clears
off-balance at `82dd8c84`, and Air 2 clears deck count at `82dd8c74`.

## Actual transition ordering

Selected coordinator `try_start_pair_ground_state` validates the actual
attached pair and borrows its retained state. It composes production operations
rather than accepting successful board/Skeleton callbacks.

`PhysicalPlayerHiLOD::SetPhysicsState` (`82dff180`) first calls the retained
Skateboard controller at `82dff1b0`, clears own count/timer at `f1c0/f1c8`,
then the same Processed timer at `f1cc`. Separate old-state GetType/category
captures establish previous Sleeping 700 (`f1e8`) and category 700 (`f1f4`);
next/current become Ground 100 (`f1fc/f204`). It does **not** publish current
physics/category at this point. Sleeping Exit is the verified folded BLR
`825187c0`.

Actual Actor player queries at `82dff22c` and `82dff2ac` remain separate.
The first local query reads JPhy; a non-offboard JPhy with current category
500 requests offboard telemetry. Otherwise the second actual player query
can request onboard telemetry when JPhy is 500 and current category is not.
Telemetry failure retains the old JPhy and all prior state stores. Successful
telemetry is followed by a fresh Processed category capture for JPhy store
`82dff320`. A genuine Sim-cleared initial local state and reset category zero
perform two local queries and request no telemetry. The selected current
Ground pointer is published at `82dff370` **before** Enter dispatch `82dffd1c`.
The host one-attempt guard prevents accidental replays of partial mutations.

Same Processed fields are owned in `skeleton_processed_physics_state.rs`:
Reset timer `82b39450`; next/last/physics/current at `82b39488/90/98/a0`;
balance `82b394a4`; category/last-category `82b394a8/b0`. Their unknown state
before actual Reset is retained.

The actual initial Reset producer is now recovered: HiLOD allocates Processed
size `0xa30`, flags 1, **Sim** category at `82e03ee4`, calls its constructor
`82e00de0` at `82e03ef0`, then publishes that pointer at `82e03efc` (`+2b0`).
The constructor initializes its foot surfaces, Grind investigation, AI input,
two GrabSpline members and interactable ID, then explicitly calls
`ProcessedPhysIn::Reset` (`82b39098`) at **`82e00f78`**, immediately before
return. `try_initialize_constructor_fields` models that selected constructor
Reset on the SAME retained Processed occurrence after the outer PrepareHistory scalar stores and before
Reckoning construction; a second initialization attempt fails before stores.
It does not claim construction of the omitted Grind/GrabSpline members.
Initial Ground's reset values therefore come from the real constructor site,
not a diagnostic pre-entry reset. The original `new()` remains unknown until
this explicit constructor stage is performed.

The real Actor query adapter is independently verified against raw
`825f9d98` (IActorQuery thunk subtracts `0x34`), folded `82485d18` (load
Actor `+1d8`, return), and Actor constructor store `825fe91c`. It reads the
same retained player-index field with bit-preserving signed interpretation;
selected telemetry branches return `TelemetryUnavailable` until implemented.

## Ground entry producer-to-consumer path

`PhysState_PhysicsGround::Enter` (`82d99c30`) is composed in source order:

1. Same board `SetStandard`; same Skeleton IK enable and physics truck tilt
   false (`82d99c64..74`); same Skeleton collision transition to 6.
2. Local prefix `82d99c80..94` clears only grab descriptor, do-jump,
   player-provided-controls, spawn-coordinates and was-pinning fields.
3. Same Reckoning spin reset; actual board animation disable.
4. Read same Processed normal/angular fields, capture board twice, dot normal
   with angular velocity, resolve actual Part 6, multiply and store omega XYZ.
   Raw sequence `82d99cc0..82d99d04` preserves packed adjacent metadata.
   The backend narrow setter invalidates the stale solve-frame identity and
   leaves velocity, force, torque, topology and other body state untouched.
5. `ResetSpeedModel` (`82b2a1f0..224`): effective transform before fresh deck
   velocity, numeric dot3 at `214`, absolute value at `220`, target-speed store
   at `224`. No unknown W lane is promoted into an observed value.
6. Reset the same retained speed-wobble owner, then source Manual branch.
   Initial reset category zero selects five-scalar `ResetManual`
   (`82b23ef8..3f08`); category 100 instead reads the retained global Manual
   attribute and scales its torque/integral, skipping velocity removal entirely.
   The other branch removes normal velocity when balance is nonzero/NaN;
   signed zero skips it as source does. Previous category 200 and previous state 505
   retain checked landing-wobble/landing-force dependencies; initial previous
   Sleeping 700 skips both.
7. Reset the same Pumping owner; set the same Wipeout owner state to Ground 1;
   only then perform final Ground stores `82d99fb4..9ff0`, including push
   scalar 1 and establishing the previously unknown hangup/anti-flip fields.

The same Processed flags word is fully known after actual Reset (`82b39428`
stores `0x2000`). Actual board-flip producer is HiLOD **ProcessAnimOut**
`82e00030`, not Update1 itself: raw `lbz` from AnimOutPhysIn `+2880` at
`82e00050`, low-byte-bit insertion into processed bit 20 at `82e00054`, store
`82e00058`. The selected setter preserves every other flag bit and knownness.
Full animation output scheduling remains separate; initial Reset establishes
the actual false flag used by the initial effective-transform speed reset.

### Manual continuation and actual tuning

Global `physics_manual_default` is the retained global slice at `+c8`, class
`a0d7351efa176078`: raw wrapper `828b0e88` assembles low `fa176078` and high
`a0d7351e` at `828b0e9c..0eb4`. Default collection is
`d7edbd362d7d2152`. Ground category 100 performs runtime
`GetAttributePointer` at `82d99d88` with **full 64-bit key
`076582f2a4a7cd27`**; a missing attribute selects `DefaultDataArea(4)` at
`82d99d98`. The actual board's retained `SkateboardAttributes` supplies this
read through its added Manual instance, followed by the same board entry
state's `scale_manual_force`. No test scalar replaces the runtime lookup.

Raw `ScaleManualForce` (`82b23ed0`) loads torque `+f4` then integral `+e8`,
multiplies torque at `3ed8` then integral at `3edc`, stores torque at `3ee0`
then integral at `3ee4`. Last target/actual angles, time, speed and animation
are preserved. The category-100 branch jumps past `RemoveVelIntoGroundForManual`.

Else, `RemoveVelIntoGroundForManual` (`82d8d090`) reads balance `+9a0` and
returns on equality to zero at `d0b0`. It captures the same Processed normal
`+1d0` and updates **deck Part 6 first** (`d0d4`). Only afterward does it
capture flags `+8c4/+8c8` (`d0d8/d0dc`). PDB names second-word logical bit
27 `mIsFrontWheelInContact`, bit 28 `mIsBackWheelInContact`; flip remains
first-word bit 20. Flipped chooses front for `[0,1,4]` and back for `[2,3,5]`;
unflipped swaps those conditions. The same Processed owner now retains the
second word: unknown before construction/reset, known zero from Reset store
`82b39430`. This is not an implementation of ordinary wheel-contact updates.

Each `RemovePartVelAlongNormalVector` (`82b266c0`) performs one normal dot3,
then source length assertions `0.99 < length < 1.01` using one reciprocal-sqrt
estimate/refinement for the lower test and a **second fresh estimate** only
when the lower comparison succeeds. It does not normalize the supplied vector.
Checked assertion failure occurs before body writes. Then actual current Part
velocity is projected as separate multiply and subtract operations. Source
`SetVelocityOfPart` (`82b24a00`) recaptures the Part and merges packed W;
the existing native body setter preserves adjacent links and invalidates a
stale solved-frame identity. The source performs full normal projection, not
only a clamp of velocity pointing into the ground.

After each selected three-Part group, source reads `kShowVelRemoval` through
ref `830d7584` and target `+cd`. Static initializer `82f8efd8` registers
section `Physics_Manual`, label `kShowVelRemoval`, title `Show Vel Removal`,
default false and null update callback (`82f8efec..82f8f004`). The same retained
`InputCvarReferences` registry supplies fresh actual reads, honoring loaded
startup.ini values. A true value stops at typed pending debug-overlay service
before source's first missing overlay acquisition, retaining the deck and group
velocity writes already performed. No constant false or fake overlay is used.

## Validation and remaining scope

### StateChanger and Update2 continuation

Live IDA/raw review: `PhysicalPlayerStateChanger` constructor `82dd39f0`
links the same PhysOut/Processed/board/Skeleton at offsets 0/4/8/c, then
zeros counters14/18/1c/20/24/28/2c/30 and flags34/35. Current state10 is
unwritten. HiLOD allocates size38 with PhysicsSkater flags1 at82e041e8,
calls ctor04204 and publishes+300 at04214, between Skeleton ctor publication
041d4 and Skeleton.Init04220. PhysicsSkater has no Sim clearing policy.

The host selected construction retains a Weak reference to the unique actual
physical owner before SkeletonData.Init, rather than demanding a Body handle
which cannot exist until later admission. The existing outer factory permits
only one Skeleton construction occurrence. Update2 validates the actual pair
through the pool and checks this retained owner identity. This is explicitly
a selected constructor projection: the existing assembly factory's full
Skeleton constructor/IK versus Init ordering remains incomplete.

`Update2_SystemLogic82e01c40` calls CheckForStateChange01c54 THEN actual
Wipeout.ClearWipeoutRequests01c5c. CheckForStateChange82e005a8 obtains the
current virtual state type005d0, calls CalcSuggestedState005e0 and, only if
different, commits HiLOD.framesSinceJump+128=100 at005f8 before calling
SetPhysicsState005fc. Unsupported transitions stop at that call boundary;
they never clear Wipeout requests or claim successful dispatch.

CalcSuggestedState82ddbd58 decrements a positive teleport countdown, stores
current state and clears two request flags before capturing the actual board
Body. Body+361 wheel count is distinct from Processed+90c count. Raw bdb4,
bdb8,bdbc read Processed count, flags8c8 and category in that order. The
source counters commit atbe5c/be84/bea0/bebc/bedc/befc/bf18 with wrapping
integer additions. Teleport flags precede the live fall-reset cvar, which
precedes AI follow-path priority and then the current-state case.

The retained Processed projection now includes genuine Reset stores:
AI6b0 clears only upper7 bits at393e8..f0; flags8d0 clears39478;
slide9ac clears394b4; GrindInvestigation.isPossible590 clears394f0;
wheelCount90c clears394f8; startRevert9b8 clears394fc; framesSinceJump91c
becomes100 at39518; flags8cc clears39558. Untouched AI bits remain unknown.
These are constructor/reset origins, not a fabricated completed Update1.

The selected ordinary Ground priority reaches real contact, animation and
state-transition dependencies. It does not treat board.entry.animated as
Skeleton.IsSkateboardAnimated. The latter82b4cd08 first checks forced animation,
then calls IsSkateboardAnimating82b45b60 even when forced physics is selected.
The actual owner query now reads animation-volume buffer+20 Y absolute value and
the retained physics_anim attribute keyb99772c107b9a8a6, assembled raw5b88,
5b90,5b9c, before its comparison. ConditionIsOffGround82dd78d8 reads Body
CollisionInfo+318 (Body+358) wheelHeightToGround and three runtime air-state
attributes. CollisionInfo.Reset23630 writes+314 at236b8 and+321 at236bc,
not+318. An unproduced wheel-height field cannot be replaced by zero.
The existing SAME Body-construction `SkateboardWheelLines.height` is that
field: `new()` leaves it unknown and genuine EndWheelLineTest82b24700 writes
0.2 only after batch Sync, then the four ordered hit-distance minima at
247d8/247dc. The system-logic continuation now consumes that retained field
directly; no duplicate contact field or fabricated first-query result is added.

Global air-state class28abb872cb8cb9ed is assembled in wrapper828b0c30.
ConditionIsOffGround reads key0fc22cb443a763ac, then key8e8b8588435b6fe8,
then recaptures actual Body and timer+1e00, then keyc6e6e6bd74dffb65.
All three attribute reads occur before the final boolean combination, even
when the first comparison is true. Runtime reads use the actual retained
Vault default collection and null-pointer DefaultDataArea4 behavior.

Skeleton mPhysAnimState+3df0 gets its actual selected ctor zero at573e8;
enum0 follows animation,1 forces physics,2 forces animation. The factory
mounts that selected store before IK. Forced animation bypasses pose/attribute
reads; forced physics still executes both before returning false. The mapped
board row belongs to the existing SkeletonAnimationTargets owner. Missing
mode, mapped row or attribute-instance construction remain explicit failures.
Only after this query does Ground reread actual grab flags; source byte8d0
low-bit test corresponds to full-word mask01000000, not mask1. Active skitch
selection requires a fresh linked kDisableSkitching target read.

Wipeout request arrays remain unknown after its constructor. Only the genuine
Update2 suffix instantiates/clears the same 30-entry retained request ledger
atClear82dd4570: request counter first, then each flag/value pair in order.
Selection errors leave existing requests untouched. Complete ordinary
Update2, actual animation/physics request producers and WipeoutGround entry
remain pending; no new skating-tick completion is claimed by this continuation.

Focused tests cover flags knownness/low-byte insertion, reset field isolation,
speed capture/failure order, omega packed-state preservation and invalidation,
Manual five-scalar reset, Pumping/Wipeout constructor and repeated-state
semantics, Ground unknown-field origins and entry tail, actual Sim origin,
two actual local query captures, and telemetry failure/fresh-category timing.
Root owns Cargo and original-data corpus integration. The new state batch is
not strictly validated until the root records the next actual gate results;
test presence is not a claim that those gates passed.

The original-data pair remains a diagnostic physical admission until actually
owned by the retained Actor admission path. A same-world Actor used for player
queries does not itself establish that ownership. Complete HiLOD construction,
ordinary Ground.Update, other state transitions, per-frame Actor/animation/
physics ordering and deterministic reference traces remain incomplete. DMO
and living-world expansion remain parked.

## Measured original Update2 and selected ragdoll continuation (2026-09-11)

The controlled original Ground-pair query produced five genuine misses, height
0.2, Body contact timer0, and live air thresholds0.12/0.06/0.2. The mapped board
was not animated. Therefore the source chooses Air200, not Ground100. The
fixture now checks PendingTransition200, committed HiLOD+128=100, unchanged
current Ground owner, and uncleared/uninitialized Wipeout requests. Root
reported original gate39095 passing15/15 after this correction. The later
supplied pose now comes from the same Actor's actual queued GetBindpose; that
change and the new ragdoll work require their own subsequent gates.

Selected SkeletonController8 now has an actual same-pair route rather than a
detached ragdoll-state mirror. EnterState82dd4788 stores state first, then case8
calls Skeleton.SetUpRagdoll82b54218, which forwards directly to
SkeletonBody.SetUpRagdoll82b4ffd8..82b503fc. World-only state3 remains an explicit
unsupported branch in this selected route. The earlier collision adapter,
which lacks backend/readers, still refuses state8; only the new qualified
`try_pair_skeleton_ragdoll_collision` supplies the retained actual dependencies.

The new adapter consumes the same SkeletonAssemblyConstruction body/joint
readers, original ragdoll culling inventory resource and bouncy ctor input.
SkeletonBody ctor82b62200 stores its makeBouncyRagdoll byte at+1e62/82b62594.
It reads UseRagdollJointLimits first; enabled mode calls actual dense joint
limits before reading bouncy state. Joint routine82b3bd08 reads local-joints
keys a57cf13fd8f95cf1 and4eee4c25b561406c for each valid parent row, with two
rounded swing multiplies and ordered twist multiplies, source fsel minimum,
then swing-angle/cosine/twist-angle/cosine stores on actual pooled limits.

Ragdoll setup preserves bouncy short circuit and repeated restitution cvar
reads. Otherwise it reads local Skeleton keys5e3ef6cc89228e59 then
bac7c1be38090b31 and writes the same embedded material records. It then selects
the actual ragdoll culling resource, group/userdata7, reads global Wipeout
angular dc1594a65b486273 and linear8a352071b8943245 before either drag helper,
and writes all24 actual mutable inertia owners in native groups of four.
Part1..23 stores are disable-count0, enabled bit, activefalse, ragdoll material,
fresh optional mass keyf1899bbc16050b02, then independent fresh tensor
keyd6de526735151b3f. Prepared mass/tensor come from the same admitted body's
constructor caches. Tensor+packedW, intermediate XY minimum and reciprocal
minimum remain separate stores. Final material overrides are Parts1,3,7;
isRagdoll=true follows them. The root Part's material/count/header are preserved.

The new original helper deliberately calls this selected controller route,
first with a failing cosine to verify controller8 and joint prefix before
body/material effects, then with the actual live readers to check all24 drag
owners and23 conditional ragdoll mass/tensor owners. This is not a full physical
bail: WipeoutGround ctor/Enter, board.SetWipeout, soft hip hooks, request6
velocity scaling, mandatory Part velocity clamp and full transition dispatch
remain pending. New code is authored/source-checked but not yet independently
reviewed or gated at this entry's writing. Confidence: high on recovered
selected source order; runtime closure awaits the new gate and review.

Subsequent root review approved the raw ragdoll and joint-limit ordering.
Workspace42928 passed3380 core tests plus245 other workspace tests and6 docs;
its original binary passed15/15, including same Actor queued pose through
factory, real Update2 Air prefix and actual24-Part ragdoll controller entry.
Strict88474 found a documentation-backtick lint in the new helper, now fixed;
the later registered soft-hook continuation is not covered by those results.

The soft-hook continuation uses SAME prepared hip hook0 dynamics and the SAME
construction body reader's GlobalAnimation instance. Angular82b47e70 reads
keys7cda0800cbb42d9b,0121f19dba38eb13,bc2bb9a096335327, then stores spring,
damping, rawType1, strength. Both spring and strength use the source two-fsel
clamped argument; strength then multiplies4560fffe. Linear82b48000 reads the
first two keys, estimates reciprocal sqrt of the UNSCALED spring at48144,
then writes spring48180, rawType1 at48188, strength4819c. Two explicit refined
sqrt steps and zero selection produce damping2*sqrt(spring)-spring*3c83126f
at481e0. Damping therefore remains active when the Enter argument iszero.
Numeric failure before48180 preserves all hook fields. The new original helper
checks that failure, actual hook Arc identity, other-hook/body preservation,
and identical damping when the actual same spring is used with strength1/0.
Root independent review caught VNMSUB's post-rounding negation: both residuals
now use `-spring.mul_add(square, -1.0)`. The exact-cancellation discriminator
checks the negative-zero residual; the final positive-estimate refinement can
hide that intermediate sign, so no different final-damping claim is made.
