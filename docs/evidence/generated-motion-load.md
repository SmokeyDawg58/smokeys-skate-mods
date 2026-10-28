# Generated Motion load inventory and staged construction

2026-09-12. Fresh IDA/PDB audit. Source declarations are exact for inspected bytes. Draft constructors now cover all 91 behaviour, four hook and 75 condition families. Cargo validation and independent source review are pending; callback execution and graph Tick remain incomplete.

## Source path

Constructor `8292a1b0` initializes40 instance slots,18 hooks,13 condition-active words, fixed time map11, current/lastFFFF, positive-zero dt, call-behaviours argument and allow-evaluations false. It eagerly visits2469 shared behaviours in table order, including ordered parameters. Shared behaviour sInit is stored INSIDE the loop after each iteration; null ClassHelper results remain null and continue unless AddParam dereferences them. Next come18 per-controller hook factories, then delegate Init `8290a918` constructs406 shared conditions and sets its separate sInit only after that loop.

Update `82955e20`: clear13 active words; dt; UpdateLists `829521e8`; Exit `8292bd58`; Transition `8292b3d0`; Enter `82952a98`; UpdateBehaviours `8292ac08`. Initial stateFFFF uses GetNextState `829511b0` -> Evaluate `8292bf58` root0 PreCond. Child order is increasing original state indices and first-success short circuit; choosing a named default would change behavior.

Root direct update `828edd40` sets Context.instance before CharacterGesture, UpdateTimeSinceTeleport, UpdateRidingFakie, FakieHeadChannel. Their actual CreateInstance/Begin lifecycle precedes Update.

## Table evidence

| Table | Base | Count | Stride | SHA256 |
|---|---|---:|---:|---|
| attributes | 8218bf08 | 2816 | 8 | b6213c7b014572d8363cbff8db1e02d4a8447262e98c0b745d0b4f83ba5d4fd4 |
| indices | 82191708 | 1737 | 2 | d2b27ea6caf240d1d6b814cb741d93da064eee25e289162aa709760e1710429c |
| states | 82195fa0 | 1354 | 32 | a070d6e419c8b34ec08cb50e27f0947826005678cad29b98b96c0563ce6d526f |
| transitions | 821a08e0 | 1724 | 16 | 774ccea0264a3d6d1f61205cf7b35f859186d685047273981d0e7dfe14337e77 |
| behaviours | 821b0fb8 | 2469 | 16 | f72ee4f0de985ace89dde5a4c2364a61215f6cff6ec1aa79bf1a46977cd8b9b7 |
| conditions | 821bc118 | 406 | 8 | 1bf01b8ab9870477fbd3584fd1a30b095dfaade2cb82c31c46d424bb1161d65e |
| hooks | 821bcfa0 | 18 | 8 | 7a174b66555d6bffc4ee1d36f9d112f522fff756eefc3bbdacfa1c0e015ea18e |
| parameters | 821bd030 | 18 | 4 | 9fd19c12018d55895186819986ae6718cabebe150c78c0fe3c6f89b7b52af063 |

Behaviour attributes/parameters begin at offsets6/10; Transition hooks at10. Sections are count then index as big-endian16 fields. PDB definitions were inspected directly; semantic Rust arrays contain no executable code or runtime guest pointers.

## Initial recovered constructors

| Class | Helper | Constructor | Size | Semantics |
|---|---|---|---:|---|
| CharacterGesture |82c91468|inline base|1c|Behaviour base only, then class vtable|
| UpdateTimeSinceTeleport |82c90bc8|inline base|1c|Behaviour base only, then class vtable|
| UpdateRidingFakie |82c91ac8|82c8dac0|2c|Four ordered GetFloat reads, defaults -1,-0.5,0.2,3; original root overrides last to1|
| FakieHeadChannel |82cbf388|82cbc368|34|Base then packed36 channel fakie|
| AttachIntent |82cbf1a8|82cbe4f0|4c|intent string0 -> packed36; attr string0 -> packed30; set boolfalse|

Shared BaseObject and explicit-attribute constructors reuse the actual Action implementation without resolving Motion sections against Action arrays. Host allocation errors remain checked boundaries. Existing parsing limits are not broader native CRT/FastString parity claims.

## Factory inventory

### behaviours

91 distinct families.

| Family | Occurrences |
|---|---:|
| CharacterGesture | 1 |
| UpdateTimeSinceTeleport | 1 |
| UpdateRidingFakie | 1 |
| FakieHeadChannel | 1 |
| AttachIntent | 93 |
| ToggleBoard | 1 |
| CreateAttribute | 333 |
| PlayAnimation | 807 |
| SettingBodyTilt | 1 |
| PowerSliding | 1 |
| BodySpin | 1 |
| UpdateTimeSinceKickturn | 1 |
| StoreLandingData | 1 |
| SetScoreAugmentation | 24 |
| ResetSkaterAnimation | 1 |
| ResetToGivenStance | 1 |
| PrintText2D | 1 |
| ControlAirLegExtension | 16 |
| OBAir | 3 |
| ForcePhysics | 11 |
| CreateGrindAttributes | 1 |
| ControlGrindCrouch | 1 |
| HandBusy | 168 |
| EndGesture | 90 |
| DisallowPumping | 21 |
| IsLanding | 1 |
| SetLandingData | 1 |
| SetDistComToBoard | 26 |
| DisableTricks | 1 |
| SetTurning | 3 |
| ScoringGrabs | 39 |
| ApplyingBodyTilt | 8 |
| LandOnBoard | 1 |
| Wipeout | 1 |
| PlayWipeoutRecoverAnim | 1 |
| AirDismounting | 3 |
| LandingOffBoardAir | 2 |
| RunoutFront | 1 |
| ScoringTrick | 187 |
| GrindControlFade | 36 |
| FilterMotionGraphIntent | 25 |
| ScoringHandPlants | 2 |
| EnterSkitchingBehaviour | 1 |
| SkitchingBehaviour | 1 |
| EndShimmy | 5 |
| HippyJumpAntic | 2 |
| Dismount | 2 |
| InitMovingObjects | 1 |
| IsDoingTrick | 41 |
| SetTrickAttr | 29 |
| SetHandPlantAnticLength | 4 |
| ClearTrickAttr | 1 |
| SetDeckPitchAndYaw | 6 |
| ChooseRandomSketchyLanding | 1 |
| SetBumpCoefficients | 1 |
| Crouching | 1 |
| UpdateManualOutTimer | 1 |
| ResetTimeSinceKickturn | 11 |
| KickTurnSteering | 12 |
| MovingObject | 58 |
| SteppingBoardNoBoard | 4 |
| StandingJumpLandingCOG | 1 |
| StandingJumpDescent | 1 |
| Locomotion | 1 |
| SetGrabType | 44 |
| SetTrickHeight | 133 |
| SetPushCoefs | 31 |
| TweakProject | 2 |
| PowerSlideManualAtt | 1 |
| IsPowerSliding | 1 |
| JumpInto | 64 |
| InitPush | 11 |
| ComputeTargetCoefsFromSpeedAndStrength | 20 |
| FingerFlipOut | 8 |
| SkitchShimmyingBehaviour | 2 |
| Pumping | 1 |
| IsManualing | 1 |
| SetManualOutTimer | 1 |
| IsAnticipating | 2 |
| SetSpeed | 1 |
| ComputeFirstPushStrength | 4 |
| FootPlantAbsorb | 8 |
| CreateSlide | 2 |
| PowerSlideDecel | 2 |
| InCandidateSlidingState | 13 |
| UpdateIsWeightOnNose | 3 |
| ComputeRepushDeadline | 2 |
| PushCycle | 2 |
| PushOut | 2 |
| PowerSlideSpin | 2 |
| SetManualAngle | 2 |

### conditions

75 distinct families.

| Family | Occurrences |
|---|---:|
| PhysFilteredState | 13 |
| CurrentState | 14 |
| HasIntent | 159 |
| IsPhysicsWiping | 1 |
| PhysicsWantsWipeOut | 1 |
| WillExpire | 22 |
| IsDismounting | 2 |
| IsLanding | 1 |
| IsRunningOut | 1 |
| IsRidingFakie | 1 |
| ComVelCompare | 4 |
| LastState | 9 |
| DisablePushBrake | 1 |
| OBTimeToLand | 6 |
| IsGrabbingObject | 2 |
| IsMovingObject | 9 |
| AngleAdjustByAnimationEndLessThan | 1 |
| LocomotionValid | 1 |
| IsLocomotionQuitNotAllowed | 2 |
| IsMirrored | 1 |
| InStateForTime | 4 |
| IsInJumpAir | 2 |
| IsLocomotionFootDown | 2 |
| DoSprintMount | 1 |
| IsRetrievingSkateboard | 1 |
| IsDroppingSkateboard | 1 |
| IsAirMountRequested | 1 |
| IsPastLocomotionApex | 1 |
| HasAnimAttribute | 8 |
| PhysicsWantsRunout | 1 |
| InTimeWindow | 1 |
| IsTransitioning | 1 |
| IsGrinding | 44 |
| CurrentGrabType | 4 |
| IsGrindBluntingBackslash | 1 |
| AllowedToTrick | 1 |
| OkToDoTrickOnStairs | 1 |
| IsProSkater | 10 |
| TimeToLand | 5 |
| IsBodyFlipping | 1 |
| ManualOutTimerIsActive | 1 |
| IsHandPlanting | 4 |
| ShouldPlayHandPlantAnim | 3 |
| IsRidingGoofy | 2 |
| TrucksOrDeckInContact | 1 |
| HasTweak | 4 |
| HasAGIntent | 2 |
| ShouldPrepareOneFootAirForFootplant | 1 |
| IsFootPlanting | 1 |
| HasFilteredMotionGraphIntent | 3 |
| HasTiltToLargeForPreland | 1 |
| HasLandingType | 2 |
| InParentStateForTime | 4 |
| IsEnteringSkitch | 2 |
| IsSkitching | 1 |
| SkitchingPosition | 3 |
| IsSkitchingWithAbsorb | 1 |
| IsSkitchShimmying | 2 |
| PhysicsSpeedCompare | 5 |
| SkateSlope | 1 |
| RandomCond | 1 |
| IsBumped | 1 |
| SurfaceSlope | 2 |
| CanEnterSlide | 3 |
| ShouldLeaveSlide | 3 |
| HasNewHandPlantPos | 1 |
| IsRidingSwitch | 1 |
| MongoPushFootToFar | 1 |
| HasGestureIntent | 1 |
| BreakOutOfPush | 1 |
| ApexReached | 1 |
| CanLandOnBoard | 1 |
| WipeoutTimeToLand | 1 |
| IsDoneWipingOut | 1 |
| IsInDebugAnimationsMode | 1 |

### hooks

4 distinct families.

| Family | Occurrences |
|---|---:|
| OverideNextAnimTransitionHook | 10 |
| InitializeLocomotion | 4 |
| GrabSlide | 2 |
| MongoPushToAntic | 2 |

## Validation

Owned files formatted. Root alone runs Cargo. The retained partial-load tests distinguish native null continuation, partial sInit preservation, real root overrides and pooled name identity. A declaration sweep now constructs every original non-Vault behaviour with all ordered parameters, every hook and every condition with the actual linked cvar manager. It has not yet run. Full actual Vault corpus integration is still required. `controller::GeneratedMotionController<K>` now retains the bound entity and wrapper authority and the exact constructor state listed above, returning only after the eager factory load. A constructed controller still cannot execute: condition evaluation, callbacks and Update remain separate work.

## Expanded constructor proof packet

Local source evidence files (outside repository, captured from current IDA) are `%TEMP%/skate2-motion-constructor-evidence.json` (91 behaviour helpers and available named constructors), `%TEMP%/skate2-motion-condition-evidence.json` (75 condition helpers/constructors/vtables), and `%TEMP%/skate2-motion-all-bindings.json`. Rust source-binding addresses are evidence labels only. The condition plain table distinguishes 36 Condition-base-only helpers from ten ComparisonCondition-base-only helpers. All are inspected 16-line helpers; the latter call the shared actual comparison parser, never a plain substitute. The remaining shared Action classes call their exact existing constructor implementation with Motion's own attribute slice.

Source-specific details preserved:

- CreateAttribute `82cab718`: `always` enables all three phases but raw store `82cab89c` writes UPDATE `+38` a second time; END `+3c` remains zero. Both the always path and no-phase path return before reading `set`.
- SetScoreAugmentation `82c8eb68`: mirrored `Switching` raw `82c8ef44` writes PRIMARY `+1c`, leaving mirror `+20` uninitialized. Host stores unknown mirror as `None`, not an invented fallback.
- FilterMotionGraphIntent: actual Curve::Insert `82c8d630` appends at most ten points and asserts strictly increasing x. Rising/falling defaults perform the same fresh blend lookup. Filter2 and filter1 override order retained through shared Action filter parser.
- EnterSkitching `82c95d70`, Skitching `82c95c08`, SkitchShimmying `82c95e78`: raw PPC proves class `07D39B41AACEE50F`, collection `D7EDBD362D7D2152`. Hex-Rays incorrectly combines argument halves here. MovingObject `82cb3488` raw proves class `97B0272A2A7E54F8` with the same collection. Each retains the actual Vault instance; null collections remain null without a generated fallback.
- HippyJumpAntic `82cb5418` invokes Gen::anim_motion `828b1590`: class `90C3DECF0105631E`, collection `3D79DAB33E8BD2BE`; retains the actual cached 0x5c0-byte layout or generated fallback. DefaultDataArea `828b5340` returns `gDefaultDataArea83059d30`, PDB byte[2580], first0x5c0 bytes all zero, only direct xref its return. This is represented explicitly by `AnimationMotionLayout::DefaultZero`.
- SetManualOutTimer `82c8e1f0` default is exact bits `3e29fbe7`; IsMovingObject condition scales by original `DEGREES_TO_RADIANS_116` bits `3c8efa35` using ordinary f32 multiplication.
- PDB verified GrabType FS1/BS2/Nose3/Tail4; foot Right0/Left1; landing Straight0/Sketchy1/Spin2; Motion object directions0..12; handplant Antic0/Into1/Out2. Unknown source foot fields are retained as `None`, not assigned a default.
- AngleAdjustByAnimationEndLessThan reads live `kMOAngleMaxDelta` before GetFloat even when an explicit MaxDeltaAngle exists. Motion statics retain the real InputCvarReferences and call `try_mo_angle_max_delta` on each constructor occurrence. Input agent independently recovered its initializer and added the real registry entry.

Hooks call TransitionHook base `82cf7a88` with null parent. Helpers: GrabSlide `82c92068` size1c, OverideNextAnimTransitionHook `82c920c8` size28, InitializeLocomotion `82c92128` size20, MongoPushToAntic `82cbf4a8` size30. Hook binding callback labels are Base::Process/AddParam/Hook::Process/OnTransition (vtable offsets36/40/44/48). Override preserves unknown transType=>Blend and reads time/current-frame/under/from-weights in order. InitializeLocomotion converts startingFoot toward zero then requires unsigned<2; Mongo requires actual anim string. All18 hook objects belong to the load instance, not shared statics.
