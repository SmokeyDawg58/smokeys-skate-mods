# Wipeout requests and OffBoard state selection

Confidence is `exact` for the 30-entry request identity and ordering, request
ledger operations, enable gating, `IsWipeoutRequested` priority, and the four
implemented OffBoard-family selector cases. The individual producer/consumer
edges in the physics-animation wipeout handshake are also exact; the composed
cross-frame latency is `high` until it is checked against a per-tick reference
trace. Ragdoll construction is documented here only where its structure and
control flow are proven. Runtime tuning that has not been resolved from the
active Vault or authored physics database remains outside the implementation.

## Reference ownership and symbols

| Address | Symbol | Object/library | Established behavior |
| --- | --- | --- | --- |
| `0x82dd9988` | `Sk8::Physics::Wipeout::DoesPlayerRequestWipeout` | `physsystems:systems3_lib.obj` | gates the 30 request flags with a global disable and 30 per-type enable controls |
| `0x82dd4570` | `Sk8::Physics::Wipeout::ClearWipeoutRequests` | `physsystems:systems3_lib.obj` | clears the request count, 30 flags, and 30 values |
| `0x82ddec70` | `Sk8::Physics::Wipeout::CheckForOffBoardWipeout` | `physsystems:systems3_lib.obj` | contains representative request flag/value/count writes for OffBoard collisions |
| `0x82e00a50` | `Sk8::Physics::PhysicalPlayerHiLOD::IsWipeoutRequested` | `physskater:physskater_lib.obj` | combines runout, enabled physics requests, and the processed animation request |
| `0x82ddbd58` | `Sk8::Physics::PhysicalPlayerStateChanger::CalcSuggestedState` | `physsystems:systems3_lib.obj` | selects the next physics state in strict priority order |
| `0x82e01c40` | `Sk8::Physics::PhysicalPlayerHiLOD::Update2_SystemLogic` | `physskater:physskater_lib.obj` | changes state, then clears the request ledger |
| `0x82dfe708` | `Sk8::Physics::PhysicalPlayerHiLOD::Update3_State` | `physskater:physskater_lib.obj` | runs state and board pre-physics work |
| `0x82e01c78` | `Sk8::Physics::PhysicalPlayerHiLOD::Update4_PostEngineAdjust` | `physskater:physskater_lib.obj` | runs skeleton/state post-physics work and checks wipeout requests |
| `0x82e04ea8` | `Sk8::Physics::PhysicalPlayerHiLOD::Update5_ProcessOutput` | `physskater:physskater_lib.obj` | publishes the wipeout request in `PhysOut` |
| `0x82c99d58` | `Sk8::Skater::Conditions::PhysicsWantsWipeOut::ActivationFunc` | map ownership not yet recorded | consumes the two `PhysOut` wipeout booleans |
| `0x828ee0b8` | `Sk8::Skater::StateUpdate_WipeOut_4` | map ownership not yet recorded | attaches wipeout intent during the wipeout animation update |
| `0x82b59de8` | `Sk8::Physics::Skeleton::ProcessAnimAttributes` | `physskeleton:skeleton_lib.obj` | converts the `Wipeout` animation attribute to processed physics state |
| `0x82d8eb18` | `Sk8::Physics::PhysState_WipeoutGround::Enter` | `physstates:states_lib.obj` | switches board/skeleton systems into wipeout operation |
| `0x82b54218` | `Sk8::Physics::Skeleton::SetUpRagdoll` | `physskeleton:skeleton_lib.obj` | skeleton wrapper for body-level ragdoll setup |
| `0x82b4ffd8` | `Sk8::Physics::SkeletonBody::SetUpRagdoll` | `physskeleton:skeleton_lib.obj` | installs proven ragdoll collision, material, mass, inertia, and drag state |
| `0x82b5f348` | `Sk8::Physics::SkeletonBody::CreateNormalCullingTable` | `physskeleton:skeleton_lib.obj` | constructs the separate normal 24-by-24 skeleton culling table |
| `0x82b5f7d0` | `Sk8::Physics::SkeletonBody::CreateRagdollCullingTable` | `physskeleton:skeleton_lib.obj` | constructs the exact ragdoll 24-by-24 skeleton culling table |
| `0x82b61738` | `Sk8::Physics::SkeletonBody::Init` | `physskeleton:skeleton_lib.obj` | builds the 24-part, 22-joint skeleton body from authored data |
| `0x82bc3308` | `rw::BitTable::Initialize` | `rwbittable:BB_runtime.rwbittable.cpp.abKicTLlAphqL90iNc8uw.obj` | writes dimensions, computes packed word count, and zeroes all words |
| `0x827cbdc8` | `RunIntersectionTests` | `aaCollision_job.obj` | proves that a clear table bit permits testing and a set bit suppresses it |

The state implementation belongs to `physstates:states_lib.obj`; the request,
state-changer, and skeleton-controller systems belong to
`physsystems:systems3_lib.obj`; skeleton body construction belongs to
`physskeleton:skeleton_lib.obj`; and the staged high-LOD player update belongs
to `physskater:physskater_lib.obj`.

## Wipeout type identity and ordering

The PDB enum and the explicit enable-table construction in
`DoesPlayerRequestWipeout` prove a fixed 30-entry order:

| Index | Reference identity | Rust identity |
| ---: | --- | --- |
| 0 | `SkeletonContact` | `SkeletonContact` |
| 1 | `SkeletonDisplacement` | `SkeletonDisplacement` |
| 2 | `DeckAcceleration` | `DeckAcceleration` |
| 3 | `DeckFlipped` | `DeckFlipped` |
| 4 | `BodyFlip` | `BodyFlip` |
| 5 | `BadAngleLanding` | `BadAngleLanding` |
| 6 | `HeavyLanding` | `HeavyLanding` |
| 7 | `VehicleCollision` | `VehicleCollision` |
| 8 | `AirToHardGrind` | `AirToHardGrind` |
| 9 | `HardGrindEngagement` | `HardGrindEngagement` |
| 10 | `BoardslideStopOnRail` | `BoardslideStopOnRail` |
| 11 | `OffBalance` | `OffBalance` |
| 12 | `HungUpOnRail` | `HungUpOnRail` |
| 13 | `OffBalanceGrind` | `OffBalanceGrind` |
| 14 | `HardHorizontalGrind` | `HardHorizontalGrind` |
| 15 | `UpsideDownFalling` | `UpsideDownFalling` |
| 16 | `OpposingContacts` | `OpposingContacts` |
| 17 | `FailureToInvExit` | `FailureToInvExit` |
| 18 | `SkeletonSquashed` | `SkeletonSquashed` |
| 19 | `LeaningContact` | `LeaningContact` |
| 20 | `DeckOpposingContacts` | `DeckOpposingContacts` |
| 21 | `SensitiveSkeletonColl` | `SensitiveSkeletonColl` |
| 22 | `BadSkateboard` | `BadSkateboard` |
| 23 | `TooFastOffBoard` | `TooFastOffBoard` |
| 24 | `FailureToGoOffboard` | `FailureToGoOffboard` |
| 25 | `OB_BadGround` | `OffBoardBadGround` |
| 26 | `OB_DeathLeap` | `OffBoardDeathLeap` |
| 27 | `OB_LandingTooFast` | `OffBoardLandingTooFast` |
| 28 | `OB_NotUpright` | `OffBoardNotUpright` |
| 29 | `OB_LandedOnDeck` | `OffBoardLandedOnDeck` |

`WipeoutType` uses `repr(u8)`, retains an ordered `ALL` array, and provides
checked index conversion. An out-of-range index is rejected rather than being
folded or interpreted as another request.

## Request ledger semantics

The reference `Wipeout` object owns three related pieces of transient state:

- 30 request booleans;
- 30 associated `f32` values;
- one request-event counter.

Request producers set the indexed boolean, replace the indexed value, and
increment the counter. They do not first test whether that request type is
already set. Therefore two writes to the same request type leave one active
flag and the second value, but increase the event counter twice.

`CheckForOffBoardWipeout` provides direct instruction-address examples:

| Request | Set flag | Store value | Increment counter |
| --- | ---: | ---: | ---: |
| `SkeletonSquashed` (18) | `0x82ddeedc` | `0x82ddeee4` | `0x82ddeef0` |
| `SkeletonDisplacement` (1) | `0x82ddef3c` | `0x82ddef44` | `0x82ddef50` |
| `SkeletonContact` (0) | `0x82ddefb8` | `0x82ddefc8` | `0x82ddefd4` |

`ClearWipeoutRequests` at `0x82dd4570` first sets the counter to zero, then
iterates exactly 30 entries and writes `false` and `0.0` to each flag/value
pair. `WipeoutRequestLedger::new`, `request`, and `clear` reproduce these
semantics. Counter addition is explicit wrapping 32-bit addition, matching the
reference integer operation rather than introducing a debug-only overflow
failure.

## Enable controls and `DoesPlayerRequestWipeout`

At `0x82dd99a0`, `DoesPlayerRequestWipeout` returns false immediately when
`kWipeoutDisableAllWipeouts` is active. Otherwise it constructs the 30-entry
enable array in enum order and visits all request/enable pairs. Its boolean
result is the OR of every `requested[index] && enabled[index]` pair.

The iteration does not stop after the first enabled request because the
reference also allows optional debug reporting for every requested index. The
Rust boolean path likewise visits all 30 entries. Debug-overlay output is not
part of the native gameplay contract implemented by this slice.

`WipeoutEnableControls` deliberately has no generic `Default` implementation.
Its values remain caller-owned snapshots because debug configuration and the
generic cVar UI can mutate them after boot. The supplied debug reference
build's compiled seeds and normal startup overlay are now resolved separately.

### Debug-reference cVar bootstrap

The 31 gates are `cVarBoolRef` globals, not attributes in
`physics_wipeout.default`. The initializer at `0x82f9c320` registers
`kWipeoutDisableAllWipeouts` in `Physics_Cheats` with compiled seed `false`.
The 30 indexed initializers at `0x82f9c550..0x82f9ceac` register their exact
`kWipeout*Enabled` labels in `Physics_Wipeouts`, each with seed `true`.
`WipeoutType::enable_cvar_label` preserves their enum/table order.

`cVarBoolRef::cVarBoolRef` (`0x82a8adc0`) links every registration into the
reference list. During boot, `cVarMan::Init` (`0x82a8ac38`) walks that list and
`cVarBoolRef::Link` (`0x82a89ca0`) either finds an existing target or creates
one with both default and value seeded from the compiled boolean. Boot then
calls `cVarSerializer::Deserialize("data\\startup.ini")` at `0x8272e490`.
Every lookup uses INI section `Config`, regardless of the cVar registration
section. `cIniFile::GetBool` (`0x82a89620`) leaves the target unchanged for a
missing label; a present integer becomes false exactly when zero and true for
every nonzero value.

For the supplied debug XEX (SHA-256
`3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`), the
supplied normal `data\\startup.ini` (SHA-256
`9605E76530C933644B50965F80BA5D4DB25164A72990DD736BEFC36853183E5B`) contains
none of the 31 labels. Consequently, immediately after the ordinary boot
deserialize, the effective snapshot is exactly global-disable `false` and all
30 per-type enables `true`.

The active `db.big` (SHA-256
`AAD0AE6B585E1FC28A3BD1B56D53BA14D04A6D86FB5E9C0F1748D2F9F364D8AA`) and
its embedded `skater.vlt` segment (SHA-256
`496215B210C3CE8BDA3F7A64B3CC3D59C56546E8D3026DBF1DA4A72F93FA7DDE`) were
also checked as negative evidence: none of the 31 label hashes occurs in the
84 `physics_wipeout` definitions or 83 explicit `default` entries, whose
parent is zero. Three unrelated bool attributes in that collection do not
participate in request enablement. There is therefore no VLT inheritance path
for these gates.

`debug_reference_compiled_defaults` exposes only the build-scoped compiled
seeds. `WipeoutEnableIntegerOverlay` models already-resolved optional integer
values: missing entries preserve the prior snapshot, zero clears, and nonzero
sets. It intentionally does not parse INI syntax. A later debug-menu
`Deserialize("data\\%s")` call or generic cVar mutation can change the live
values, so the boot snapshot is not treated as permanently frozen.

## Final request orchestration

`PhysicalPlayerHiLOD::IsWipeoutRequested` at `0x82e00a50` has three strict
steps:

1. At `0x82e00a64`, a runout request returns false immediately.
2. At `0x82e00a78`, any enabled physics-ledger request returns true.
3. At `0x82e00a9c`, processed word 564 bit 6, `AnimRequestsWipeout`, supplies
   the final result.

Runout therefore suppresses both physics and animation sources. A globally or
individually disabled physics request does not suppress a separate animation
request. `is_wipeout_requested` and `WipeoutRequestSources` preserve these
ties without embedding update scheduling or animation state-graph ownership
inside the ledger.

## OffBoard-family selector inputs

The implemented selector uses named flags rather than exposing packed words at
runtime. Their reference offsets remain traceability evidence:

| Rust flag | Reference location |
| --- | --- |
| `WipingOut` | word 561, byte `0x8c4`, bit 18 |
| `AnimationRequestsWipeout` | word 564, byte `0x8d0`, bit 6 |
| `GrabbingObject` | word 563, byte `0x8cc`, bit 23 |
| `OffBoardComMotion` | word 563, byte `0x8cc`, bit 16 |
| `OffBoardTrajectory` | word 563, byte `0x8cc`, bit 17 |
| `OffBoardAir` | word 563, byte `0x8cc`, bit 8 |
| `OffBoardStandingJump` | word 563, byte `0x8cc`, bit 20 |
| `OffBoardJumpInfoReady` | word 565, byte `0x8d4`, bit 29 |
| `LandOnBoard` | word 564, byte `0x8d0`, bit 7 |
| `BoardMountInvalid` | word 565, byte `0x8d4`, bit 23 |

Word 564 bit 18, byte offset `0x8d0` and mask `0x0004_0000`, is intentionally
represented as `OpaqueProcessedWord564Bit18::{Unset, Set}`. Its branch polarity
is proven, but its original field name, producer, and gameplay meaning are
not.

## Exact selector branches

`PhysicalPlayerStateChanger::CalcSuggestedState` at `0x82ddbd58` returns on
the first matching condition. The implemented OffBoard subset covers only
current states `OffBoard`, `OffBoardPushing`, `OffBoardAir`, and
`OffBoardLocomotion`. Other current states, including `OffBoardJump`, remain
an explicit unsupported result in this slice.

### Current state `OffBoard`

The priority is:

1. `WipingOut` -> `WipeoutGround`.
2. `GrabbingObject` -> `OffBoardPushing`.
3. `OffBoardComMotion` -> `OffBoardLocomotion`.
4. `OffBoardJumpInfoReady` and either `OffBoardStandingJump` or `OffBoardAir`
   -> `OffBoardJump`.
5. `OffBoardTrajectory` -> remain `OffBoard`.
6. Opaque word 564 bit 18 set and `BoardMountInvalid` unset -> `PhysicsAir`.
7. Otherwise -> `WipeoutGround`.

The COM branch encloses the later no-COM decisions in the reference, so COM
motion outranks simultaneous jump and trajectory flags.

### Current state `OffBoardPushing`

The priority is:

1. `WipingOut` -> `WipeoutGround`.
2. `GrabbingObject` -> remain `OffBoardPushing`.
3. Otherwise -> `OffBoard`.

### Current state `OffBoardAir`

The priority is:

1. `WipingOut` -> `WipeoutGround`.
2. `OffBoardComMotion` -> `OffBoardLocomotion`.
3. `OffBoardTrajectory` -> `OffBoard`.
4. `LandOnBoard` -> `LandingOnDeck`.
5. If `OffBoardAir` is unset, return `PhysicsAir` when the board mount is
   valid, otherwise `WipeoutGround`.
6. Otherwise remain `OffBoardAir`.

Because the trajectory branch precedes `LandOnBoard`, simultaneous trajectory
and landing flags return `OffBoard`.

### Current state `OffBoardLocomotion`

The corrected priority is:

1. `WipingOut` -> `WipeoutGround`.
2. `AnimationRequestsWipeout` -> `WipeoutGround`.
3. `GrabbingObject` -> `OffBoardPushing`.
4. `OffBoardTrajectory` -> `OffBoard`.
5. `OffBoardAir` -> `OffBoardAir`.
6. `LandOnBoard` -> `LandingOnDeck`.
7. When `OffBoardComMotion` is unset, opaque word 564 bit 18 set returns
   `PhysicsGround`; an unset opaque bit returns `WipeoutGround`.
8. Otherwise remain `OffBoardLocomotion`.

The landing/no-COM tie was rechecked at instruction level. `0x82ddcce4` through
`0x82ddccec` test word `0x8d0` bit 7 and branch to `0x82ddc1d0`, which loads
state value `0x1f9` (`LandingOnDeck`). Only the unset fallthrough reaches the
COM test at `0x82ddccf0` through `0x82ddccf8`, followed by the opaque-bit
selection at `0x82ddccfc` through `0x82ddcd10`. Therefore `LandOnBoard=true`
and `OffBoardComMotion=false` must select `LandingOnDeck`.

## Cross-frame wipeout handshake

The request ledger is transient physics state, while the state changer consumes
animation feedback. The established producer/consumer chain is:

1. `Update2_SystemLogic` at `0x82e01c40` calls `CheckForStateChange` at
   `0x82e005a8`. That calls `CalcSuggestedState`, and `SetPhysicsState` at
   `0x82dff158` performs an old-state `Exit` followed by new-state `Enter` when
   the suggestion changes.
2. Still in Update2, `Wipeout::ClearWipeoutRequests` clears the prior request
   ledger.
3. `Update3_State` at `0x82dfe708` runs prediction preparation, the current
   state's `Update`, board-controller and board pre-physics work, then advances
   the state timer by the processed delta.
4. The physics-engine simulation occurs between the responsibilities exposed
   by Update3 and Update4. This outer boundary is `high`; a complete manager
   call trace is still required for an exact scheduling claim.
5. `Update4_PostEngineAdjust` at `0x82e01c78` ends skeleton line tests, performs
   board post-physics work, calls `Skeleton::UpdatePostPhysics`, calls the
   current state's `UpdatePostPhysics`, and then calls `IsWipeoutRequested`.
   OffBoard collision and landing checks create request-ledger entries in this
   post-physics phase.
6. `Update5_ProcessOutput` at `0x82e04ea8` fills output components, calls
   `IsWipeoutRequested` again, and stores
   `PhysOut::State::mPhysicsRequestsWipeout` at `0x82e0546c`.
7. `PhysicsWantsWipeOut::ActivationFunc` at `0x82c99d58` consumes the relevant
   `PhysOut` state booleans. `StateUpdate_WipeOut_4` at `0x828ee0b8` updates the
   motion state and attaches wipeout intent.
8. `Skeleton::ProcessAnimAttributes` at `0x82b59de8` recognizes presence of
   the exact `Wipeout` attribute and sets processed word 561 bit 18,
   `IsWipingOut`.
9. A later Update2 state selection sees `IsWipingOut`; the OffBoard-family
   branches then select `WipeoutGround`.

Every named call, bit consumer, and ordering edge above is exact. Treating the
whole chain as a particular number of input/output frames is `high`, because a
reference per-tick trace has not yet established the precise externally
visible latency.

## Proven ragdoll and `SkeletonBody` facts

These facts constrain future implementation but are not yet represented as a
complete native ragdoll simulation.

### Wipeout entry

`PhysState_WipeoutGround::Enter` at `0x82d8eb18`:

- resets its timers, drive weights, reset flags, impulse-frame count, and
  prediction readiness;
- initializes estimated time to collision to `FLT_MAX`;
- switches the skateboard to wipeout mode and zeros forced turn;
- disables board angular and linear hook drives and skeleton IK;
- sets angular-hook softness to `0.5` and linear-hook softness to `0.0`;
- handles `HeavyLanding` request index 6, then clamps skeleton-part velocity
  to the compiled maximum `20.0`;
- when the living-world impulse flag is present, scales the supplied impulse
  through runtime attribute hash `0x58fc35c49844acc6` and calls
  `Skeleton::PushRagdoll` at `0x82b59868`; that call applies displacement to
  `PART_SPINE2` only;
- selects skeleton-controller state `SCS_RAGDOLL`, numeric value 8;
- clears skeleton linear drag and copies the wipeout impulse and its mode bit.

The heavy-landing scale inputs and living-world multiplier are runtime tuning
dependencies. Their control flow is exact, but unresolved effective values
must not be replaced by generic constants.

### Body construction

`SkeletonBody` is 7,792 bytes (`0x1e70`). Relevant proven fields include:

| Offset | Field |
| ---: | --- |
| `0x00ac` | 24 `SkeletonBoneData` entries |
| `0x02ec` | 24 `SkeletonDriveData` entries |
| `0x05ec` | 22 `SkeletonJointData` entries |
| `0x07a4` | 24 animated inverse masses |
| `0x0804` | 24 ragdoll inverse masses |
| `0x0870` | 24 animated inverse-inertia records |
| `0x09f0` | 24 ragdoll inverse-inertia records |
| `0x0bb4` | 24 physics volumes |
| `0x1e00` | 24 disable counts |
| `0x1e60` | active flag |
| `0x1e61` | ragdoll flag |
| `0x1e62` | bouncy flag |

`SkeletonBody::Init` at `0x82b61738` creates one assembly part for each of the
24 `SkeletonDataBase::PartIndex` values and up to 22 joints. Part zero is a
special disabled root capsule with state mass zero. Each non-root authored
`SkeletonBoneData::VolumeType` selects box 0, sphere 1, or capsule 2; values 3
and above assert. Dimensions combine the physics-parameter bone size with the
authored volume scalar. Separate animated and ragdoll inverse mass/inertia
sets are computed from `MassFactor` and `RagdollMassFactor` rather than sharing
one generic mass model.

`SkeletonBody::InitJoints` at `0x82b464f8` derives parent-child mappings and
joint frames from the skeleton hierarchy and authored physics transforms. The
normal swing/twist limit path multiplies authored limits by the corresponding
normal joint scalar, compares the product minus exact epsilon `0.01`, then
selects the ORIGINAL product when that difference is nonnegative, otherwise
epsilon (including unordered). It stores this angle and its cosine, not a
zero-clamped difference. Raw source `0x82b46fd0..0x82b47020` independently
rechecked 2026-09-07 corrects the earlier subtraction/clamp description.
`SetRagdollJointLimits` at `0x82b3bd08` uses
the ragdoll swing/twist scalars plus runtime attributes
`0xa57cf13fd8f95cf1` and `0x4eee4c25b561406c`, then the same minimum-angle
selection and cosine storage. Raw `0x82b3be1c..0x82b3be20` and
`0x82b3be74..0x82b3be84` confirm the selected value is product or epsilon.

### Ragdoll setup and collision modes

`Skeleton::SetUpRagdoll` at `0x82b54218` delegates to
`SkeletonBody::SetUpRagdoll` at `0x82b4ffd8`. Controller state
`SCS_RAGDOLL` (8) calls that wrapper at `0x82dd4874` and returns. State
`SCS_COLLIDE_WITH_WORLD_ONLY_RAGDOLL` (3) calls it at `0x82dd4890`, then calls
`Skeleton::CollideWithWorldOnly` at `0x82dd4898`. The two operations therefore
remain separate in the native implementation.

The exact body-level setup order is:

1. `0x82b4ffec..0x82b50008` reads live
   `kSB_UseRagdollJointLimits`; when true it calls
   `SkeletonBody::SetRagdollJointLimits` at `0x82b3bd08`.
2. `0x82b5000c..0x82b500c4` selects restitution. The bouncy path is active
   when either `SkeletonBody + 0x1e62` or live `kBouncyRagdollCheat` is true;
   the local byte is sampled first and suppresses the cheat-cVar read when
   already true. Otherwise the cheat cVar is read exactly once. The bouncy
   branch performs separate live reads of `kBouncyRagdollCheatRestitution`,
   writing head first and body second. The normal path resolves and writes body hash
   `0x5e3ef6cc89228e59` first, then head hash
   `0xbac7c1be38090b31`; a missing node uses `DefaultDataArea(4)`.
3. `0x82b500c8..0x82b500d8` copies the resource behind
   `mCullingTableRagDoll + 0xb7c` into the assembly culling-table pointer.
4. `0x82b500dc..0x82b50110` sets assembly group ID 7 and sets every current
   part's `+0x5c` user-data word to 7.
5. `0x82b50114..0x82b501ec` resolves angular and then linear drag once each;
   only after both reads does it call `SetAngularDrag(value, -1)`, followed by
   `SetLinearDrag(value, -1)`.
6. `0x82b50218..0x82b503b4` visits exactly parts 1 through 23. For each part it
   zeros the volume disable count, calls folded empty function `0x825187c0`,
   sets volume flag bit zero, clears `mVolumeDisableCountActive`, assigns the
   ragdoll-body material, then independently re-reads the inverse-mass hash
   `0xf1899bbc16050b02` and inverse-inertia hash `0xd6de526735151b3f` for that
   part. Each hash is therefore read exactly 23 times, even if its value
   changes during the loop. When inertia is enabled, the first comparison
   selects `x` only for `x < y` and otherwise selects `y`; the second keeps
   that value only when it is `< z` and otherwise selects `z`. It stores
   `1.0 / selected` in the spherical lane. This preserves equality,
   signed-zero, and unordered-NaN branch behavior rather than using a generic
   float minimum. The first selection is transiently stored before the final
   reciprocal overwrites it; no call or read intervenes, so the typed boundary
   preserves final state rather than the unobservable intermediate write.
7. `0x82b503b8..0x82b503e4` changes parts 1, 3, and 7, in that order, to the
   ragdoll-head material.
8. `0x82b503e8` sets `SkeletonBody + 0x1e61`; `0x82b503ec` calls the same
   folded empty function once more.

The folded function is a shared `blr` COMDAT at `0x825187c0`. Its machine
effect is empty; an intended friendly semantic name is not proven.

PDB `rw::collision::Inertia` places inverse inertia at `+0x00`, inverse mass
at `+0x10`, spherical at `+0x14`, linear drag at `+0x20`, and angular drag at
`+0x24`. The drag helpers multiply by raw executable float `0x426fffff`
(`59.9999961853`) from `0x8225fd98`. Supplied VLT inputs `0.5` angular and
positive-zero linear therefore produce raw stored values `0x41efffff` and
`0x00000000` for every part, including part zero.

The constructor at `0x82b62424..0x82b624bc` initializes normal, ragdoll-body,
ragdoll-head, and zero material slots from executable constants: static
friction `0x3f000000` (`0.5`), dynamic friction `0x3e99999a` (approximately
`0.3`), and restitution `0x3ecccccd` (approximately `0.4`). Setup changes only
the body/head restitution lanes. Supplied `physics_skeleton.default` contains
raw `0x3c23d70a` for both normal ragdoll restitution hashes. The bouncy cVar
constructor at `0x82f7d418..0x82f7d458` seeds raw `0x3f7d70a4`
(approximately `0.99`), but setup reads its live value.

Plain setup does not change `Volume::m_groupID + 0x54`, the skeleton proxy
enable byte at `+0x3e43`, root volume flags, or root disable count. It assigns
user data and drag to part zero, but does not assign its material or prepared
mass/inertia. The world-only follow-up at `0x82b4fa80` conditionally enables
parts 1 through 23 and sets their volume group to 0, then changes assembly and
all-part user data to 3. Its wrapper at `0x82b54370` disables the proxy, calls
the folded empty function for the root edge, clears root flag bit zero, and
zeros the root disable count.

The collision modes are distinct:

- `SkeletonBody::SetUpRagdollCollision` at `0x82b47870` uses group/user data
  7 and enables eligible parts 1 through 23.
- `SkeletonBody::CollideWithWorldOnly` at `0x82b4fa80` uses group/user data 3;
  the skeleton wrapper also disables proxy/root collision.
- `Skeleton::SetOffBoardCollision` (symbol entry `0x82b4f038`; semantic body
  from `0x82b4f04c`) selectively enables
  authored limb/core collision and keeps a proxy volume with root user data 6.
- `Skeleton::SetOffBoardAirCollision` (symbol entry `0x82b4efd8`; semantic body
  from `0x82b4eff0`) sets up ragdoll-style
  collision filtering but does not itself install ragdoll material, mass, or
  inertia; it disables proxy/root collision.

### Supplied setup data and prepared-dynamics boundary

The supplied `skater.vlt` has SHA-256
`36642cd06358d569632de24d02261286cec45a3fa7898a2edd8a95d7b62f6fff`;
the sibling BIN has SHA-256
`458e8fa67489a612cb5239d566bea648980cb92a40b74af14fe929c18ffa5fd7`.
`physics_wipeout.default` is an explicit no-parent collection. Its relevant
inline values are:

| Hash | VLT value slot | Raw value | Executable use |
| --- | ---: | ---: | --- |
| `0xdc1594a65b486273` | `0x5f3b8` | `0x3f000000` | angular drag `0.5` |
| `0x8a352071b8943245` | `0x5f488` | `0x00000000` | linear drag positive zero |
| `0xf1899bbc16050b02` | `0x5f438` | `0x01` | install prepared inverse mass |
| `0xd6de526735151b3f` | `0x5f428` | `0x01` | install prepared inverse inertia |
| `0x3689013d7adbdd2a` | `0x5f528` | `0x3f800000` | inverse-mass numerator `1.0` |

The `physics_skeleton_bones.default` 576-byte layout is at BIN `0x9f2a8`
and has SHA-256
`156c7bce6b57cc7957bfa43333b5b0cc220f3c696db3c30705526d4cb04bfb06`.
Its exact ragdoll mass factors in `PartIndex` order are:

```text
[1, 1, 1, 4, 2, 2, 2, 4, 2, 2, 2, 1,
 1, 1, 1, 4, 4, 1, 1, 4, 4, 1, 1, 1]
```

`SkeletonBody::Init` loads the numerator at `0x82b618a4..0x82b618d8`,
combines the selected physics-record shape with the bone data and
`physics_skeleton.default` hash `0x9f4a03c75637e7e9` (supplied raw
`0x43a00000`, `320.0`), loads `RagdollMassFactor` at `0x82b61e98`, and stores
cached inverse mass at `0x82b61ea0..0x82b61ea4`. It calls
`ComputeMassProperties` at `0x82b61ea8` and copies the resulting 16-byte lane
set at `0x82b61ee4..0x82b61eec`. The setup implementation accepts these
prepared arrays from its caller; it does not invent or duplicate the still
incomplete shape/mass-property calculation.

The player path asks `Andale::PlaybackData::GetPhysicsParam` at `0x82d7cb30`
for packed name `phys_Tpose`. The one type-5 record in each scoped database is
byte-identical, 2,752 bytes, with SHA-256
`136d6dcaf907c96f51e6e5184f5c7ca46357ef1ef21ce0adf755162af7ed0ef5`:

| Database | Record offset |
| --- | ---: |
| `SkaterBodyPres.abin` | `0x950` |
| `OffBoard.abin` | `0x8b80` |
| `OnBoard.abin` | `0x2e00` |

Selection among those three therefore cannot change the physics-parameter
input. Excluding every possible additional mounted playback-data overlay has
not yet been proven. The serialized record mass lane is `1.0`, but the
inspected initialization path does not directly load that lane; it must not
be substituted for the prepared inverse mass.

### Exact ragdoll culling table

Confidence is `exact` for this supplied debug build. PDB type `rw::BitTable`
is 16 bytes: `m_rows` at `+0x00`, `m_columns` at `+0x04`, `m_arraySize` at
`+0x08`, and the first flexible-array word at `+0x0c`.
`rw::BitTable::Initialize` at `0x82bc3308` computes
`(rows * columns + 31) >> 5` words and initially zeroes them. The ragdoll
constructor passes 24 rows and 24 columns at `0x82b5f8ac..0x82b5f8b8`, giving
18 words with no tail padding. Its nested loop at
`0x82b5f93c..0x82b5f9c0` then sets all 576 bits, including all self/diagonal
bits.

Packed addressing is continuous row-major storage, not a word-aligned row:

```text
flat = row * 24 + column
word = flat >> 5
bit  = flat & 31
```

The 24 exact PDB part indices are:

```text
 0 PART_SKATEBOARD_ROOT   12 PART_SPINE2
 1 PART_NECK1             13 PART_SPINE1
 2 PART_NECK              14 PART_SPINE
 3 PART_LEFTHAND          15 PART_LEFTTOEBASE
 4 PART_LEFTFOREARM       16 PART_LEFTFOOT
 5 PART_LEFTARM           17 PART_LEFTLEG
 6 PART_LEFTSHOULDER      18 PART_LEFTUPLEG
 7 PART_RIGHTHAND         19 PART_RIGHTTOEBASE
 8 PART_RIGHTFOREARM      20 PART_RIGHTFOOT
 9 PART_RIGHTARM          21 PART_RIGHTLEG
10 PART_RIGHTSHOULDER     22 PART_RIGHTUPLEG
11 PART_SPINE3            23 PART_HIPS
```

The clear span contains 136 paired clear sequences: 136 `andc` instructions
(first at `0x82b5f9ec`) and 136 `stwx` instructions (stores at
`0x82b5f9f0..0x82b61728`), or 272 instructions total. Symbolic evaluation of
every store found 136 unique in-range
directed coordinates: 68 explicit symmetric pairs, no duplicate directed
write, and no self clear. The following list is in native execution order;
each `(a,b)` is immediately followed by the explicit `(b,a)` write:

```text
(1,11),
(16,20), (16,21), (16,22),
(20,18), (20,17), (17,21), (17,22), (21,18),
(17,7), (21,3), (17,9), (21,5),
(18,7), (22,3), (18,9), (22,5), (18,8), (22,4),
(18,3), (22,7), (22,8), (18,4),
(3,7), (3,9), (7,5), (3,8), (7,4),
(3,1), (7,1), (3,2), (7,2),
(3,14), (7,14), (3,13), (7,13),
(3,12), (7,12), (3,11), (7,11),
(8,4), (8,5), (4,9),
(4,1), (8,1), (4,2), (8,2),
(4,14), (8,14), (4,13), (8,13),
(4,12), (8,12), (4,11), (8,11),
(5,9), (5,1), (9,1), (5,2), (9,2),
(5,14), (9,14), (5,13), (9,13),
(5,12), (9,12), (5,11), (9,11)
```

The exact final 18-word snapshot is:

```text
47ffffff fc47fff4 9b8479ff 79bb8479
ffff9f84 b987c1ff c1bb87c1 fffff987
fffc45ff 47fffc47 fc47fffc ffffffff
7f8fffff fc678ffd ffffffcf d7f8ffff
fe47f8ff fffffffc
```

`RunIntersectionTests` checks the packed word at `0x827cbf30` and
`0x827cbf80`. Its final `beq` gate at `0x827cbf94` branches to `0x827cc3d8`
for a set bit; a clear bit falls through to the intersection path at
`0x827cbf98`. Therefore the final ragdoll matrix contains exactly 136 clear
bits eligible for testing and 440 set bits that suppress their directed pair.
Eligibility remains subject to the other volume, body, and collision gates;
it does not guarantee a contact.

Normal and ragdoll table selection is separate from the collision-mode helper
names. `SkeletonBody` contains `mCullingTableNormal` at `+0xb78` and
`mCullingTableRagDoll` at `+0xb7c`. `SetUpNormal` installs the former at
`0x82b50530`; `SetUpRagdoll` installs the latter at `0x82b500d8`. The normal
constructor clears only six symmetric pairs: `(14,4)`, `(14,8)`, `(23,4)`,
`(23,8)`, `(4,18)`, and `(8,22)`. `SetOffBoardCollision` and
`SetOffBoardAirCollision` alter groups and volume filtering but do not assign
the assembly culling-table pointer, so there is no third OffBoard culling
table in this object path.

The ragdoll constructor first looks up inventory key `cullTableRagdoll`. When
that resource already exists, `0x82b5f7fc..0x82b5f808` skips allocation,
initialization, and every clear. The native Rust value reconstructs the exact
missing-resource construction result only; inventory identity, reload, and
preexisting-resource lifetime remain caller responsibilities rather than a
guessed ownership layer.

### Wipeout update facts

`PhysState_WipeoutGround::Update` at `0x82d9aff8` calls
`Skeleton::UpdateWipeout` at `0x82b5ea28`, updates drive weights, manages
recovery, consumes the prior collision prediction, and launches the next
fake-body prediction. `SkeletonDrives::UpdateRagdoll` at `0x82b546c0` blends
authored start, normal, controlled, and end drive strengths for parts 1 through
22. The actual strengths remain runtime data.

The wipeout impulse path at `0x82d84da8` applies the copied world-space force
to the deck while the impulse flag is set and the frame counter is below 21.
The counter increments after each application, proving a maximum of exactly 21
state-update applications.

## Native implementation and validation

`crates/skate2-core/src/physics/wipeout.rs` contains the accepted native slice:

- stable `WipeoutType` identity and conversion;
- `WipeoutRequestLedger` write, query, count, and clear operations;
- caller-owned `WipeoutEnableControls`, exact debug-build compiled seeds,
  indexed cVar labels, and resolved-integer overlay behavior;
- `is_wipeout_requested` source priority;
- typed `OffBoardStateFlag` input and opaque word-564-bit-18 wrapper;
- the four exact OffBoard-family selector cases.

`crates/skate2-core/src/physics/ragdoll.rs` contains the independently bounded
skeleton-culling and setup slice:

- exact `SkeletonDataBase::PartIndex` identity and conversion;
- continuous 24-bit row-major addressing across 18 words;
- the 68-pair ragdoll clear sequence in native order, with both directed bits
  cleared explicitly;
- exact final words and typed clear/set polarity;
- the separate six-pair normal-table evidence without inventing OffBoard setup
  behavior or inventory ownership;
- caller-supplied object-local bouncy state and prepared inverse
  mass/four-lane inertia, with live cVar/VLT reads kept behind an ordered
  collaborator and no generic defaults or mass-generation approximation;
- exact restitution lookup/write order, drag scaling/order, culling and group
  changes, parts 1-through-23 setup loop, head overrides, ragdoll flag, and
  folded empty call edges;
- exact comparison/selection branches for covered ordinary finite, equality,
  signed-zero, and unordered inputs, with exact covered signed-zero reciprocal
  bits;
- plain ragdoll and its separate collide-with-world-only composition, including
  the proxy/root and volume-group distinction.

Seventeen focused unit tests cover all enum indices and conversions, exact cVar
label mapping, build-scoped compiled seeds, missing/zero/nonzero overlays,
duplicate request writes, per-type/global enable gates, clear behavior, runout
suppression, animation fallback, selector priority ties, both opaque-bit
polarities, the landing/no-COM Locomotion tie, and rejection of unsupported
current states. Sixteen ragdoll tests cover the prior culling-table proof plus
normal and bouncy restitution order, separate live bouncy reads, joint/folded
call ordering, local-bouncy cheat-read suppression, index ranges and part-zero
exclusions, exact drag bits, 23 independent live reads of each mass/inertia
gate, conditional mass/inertia preservation, PPC tie/NaN selection, and
separate world-only composition and disable-count behavior. The focused
validation commands are:

```powershell
cargo test -p skate2-core wipeout
cargo test -p skate2-core ragdoll
cargo clippy -p skate2-core --all-targets -- -D warnings
```

Both pass on the integrated source tree.

## Touch bails from corner contacts (2026-10-19)

Running off-board past a vertical corner (a box prop, bench, planter or tree trunk) requested
`SkeletonDisplacement` although no check had changed: `CalculateHookToHipsWithProxy` saw the
proxy volume (Part 0) about a quarter of a meter from its target for one tick.

- **Cause:** the two-point branch of `FindFaceEdgePrism` (`0x82ad0cc0`) replaces the contact
  normal with `cross(dir, cross(delta, dir))` (`0x82ad0fc8..0x82ad1040`): the part of the
  face-to-edge delta perpendicular to the edge. `face_edge_feature_prism` computed only
  `cross(delta, dir)`, a direction perpendicular to that.
- **Effect:** the upright proxy capsule is parallel to a vertical triangle edge. So the
  generic separating-axis candidates (`FindBestSepDir_FindCandidates`, `0x82aca230`) reduce to
  the triangle's face normal, and the prism takes the two-point branch.
  - The wrong replacement normal failed the orientation test, and the face normal stayed with
    a separation of about the capsule radius.
  - The contact's positional impulse then moved Part 0 that far within one step, with no
    change of velocity.
  - The next tick the hook drive pulled it back.
- **Measured:** at a planter corner, a front-face contact with normal `(-0.087, 0, 0.996)` and
  separation `-0.285` moved the proxy 0.234 m (limit 0.3); the same corner's side face gave the
  correct normal `(0.968, 0.04, 0.246)` and no depth. With the native normal the one-tick jump is
  gone.
- The other replacement normals match the reference: the face/face fallback
  (`0x82ad2b98..0x82ad2c48`) and the single-point normal of `Sk8ComputeContactPoints`.

## Unresolved boundary

The following are not parity claims and must not be filled with plausible
ragdoll defaults:

- Values after a user-selected debug INI, generic cVar UI/console mutation, or
  another executable build are unresolved. The exact supplied-build compiled
  seeds and ordinary startup snapshot do not freeze those later live values.
- Human-friendly meanings, types, and effective values for unresolved hashed
  wipeout, drag, restitution, mass/inertia, drive, settle, and recovery
  attributes are unknown unless explicitly identified above.
- Word 564 bit 18 has exact offset, mask, branch polarity, and tests, but its
  original name, producer, and gameplay meaning are unknown.
- The scoped `SkaterBodyPres`, `OffBoard`, and `OnBoard` `phys_tpose` records
  are byte-identical. A proof excluding an additional mounted playback-data
  overlay remains unresolved.
- Effective body dimensions and the prepared inverse-mass/inertia values still
  require the complete shape-scalar and mass-property calculation. Joint
  backend mutation and drive strengths also remain outside this slice. No
  generic ragdoll approximation is acceptable.
- NaN result payload/sign, subnormal handling, and PPC FPSCR versus host
  floating-point behavior in the final spherical reciprocal still require a
  differential trace; the proven comparison branches must not be broadened to
  a blanket bitwise-arithmetic claim.
- The outer manager call trace and per-frame reference trace needed to turn the
  `high` cross-frame handshake into an exact externally visible latency claim
  have not been captured.
- OffBoard Jump state selection, state-specific locomotion/air dynamics,
  settle/recovery, teleport, remount, and a complete native ragdoll solver are
  outside the accepted Rust slice documented here.

Any implementation that substitutes generic ragdoll constraints, guessed
enable defaults, partial collision filtering, or friendly names for unknown
hashes remains a `hypothesis`, not a reconstruction.
