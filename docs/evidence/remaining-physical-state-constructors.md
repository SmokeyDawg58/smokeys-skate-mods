# Remaining physical state constructor packet

Implemented in physical_player_assembly::remaining_states (new source file plus fields/simple/skitch/pushing children). Registration and mounted execution remain root integration responsibility. No Cargo or tests run by author. Independent review required before parity claim.

## Exact constructor domain and ownership

The 22 missing state slots interleave the already implemented Sleeping, PhysicsAir, WipeoutGround and PhysicsGround. Every state allocation is a PhysicsSkater allocation with no source OR0x100 clearing. Typed Option cells preserve every named PDB field not written by that constructor. Pointer cells use Option<Option<StateOwnerLink>>: outer None is uninitialized storage; Some(None) is an explicitly copied NULL pointer. Same-parent weak links identify an exact immutable collaborator member, distinct from copied data or fabricated services. No Update/Enter/Exit behavior is implemented by these owners.

StateConstructorBindings::capture takes the actual existing body binding, retains the real Processed Arc and copies the current PhysicsSystems collaborator pointer snapshot. Actual GrindManager must exist before dereferencing its separately allocated GrindJumper; a NULL jumper itself is preserved. Source PhysicsSystems includes Reckoning,Wipeout,TrajectorySelector,GrindAirAdjust,FootPlant,HandPlant,PlayerGrabSpline,OffboardTrajectorySelector,SkateboardController,FeetIK,LandingDeck,Pumping,LocationTest,JumpingTest,SkeletonController,GrindJumper. Controller presence follows constructor completion, not host inline storage existence. Actual conditioners are separately retained for Teleporting.

RemainingStates::try_construct preserves the partial allocation in its slot before any live attribute/cvar reads. No replay is allowed after an attempt. A native allocation NULL publishes NULL and proceeds; a failed constructor preserves an unpublished prefix. StateKind::ORDER orders only these22; root must interleave its existing four at the native callsites.

| State | HiLOD constructor call | PDB allocation bytes |
|---|---|---|
| SlideGround | 82e045bc | 48 |
| RevertGround | 82e04610 | 96 |
| GroundAnimation | 82e0464c | 96 |
| KnownAir | 82e046c4 | 224 |
| GrindBoardslide | 82e04704 | 240 |
| Grind5050 | 82e04754 | 208 |
| GrindTipslide | 82e047a4 | 208 |
| Grind5O | 82e047f4 | 208 |
| GrindBackslash | 82e04844 | 208 |
| Nonspecific | 82e048d0 | 64 |
| Teleporting | 82e04928 | 112 |
| FollowPath | 82e04968 | 32 |
| Skitching | 82e049b4 | 1312 |
| HandPlant | 82e049f0 | 80 |
| FootPlant | 82e04a2c | 52 |
| Boneless | 82e04a68 | 96 |
| OffBoard | 82e04aa4 | 144 |
| OffBoardLocomotion | 82e04ae0 | 92 |
| OffBoardPushing | 82e04b1c | 1136 |
| LandingOnDeck | 82e04b58 | 92 |
| OffBoardAir | 82e04bd4 | 192 |
| OffBoardJump | 82e04c10 | 176 |

## Simple constructor proof

PhysicsState82d83aa8 copies board/skeleton and five actual system references plus Processed. SlideGround inline82e045c4/c8 writes targetSpeed0/BoardIsFlippedfalse only; pushScalar/dampedTurn stay unknown. Revert82d912a8 binds actual generated physics_revert then Pumping; revertingfalse,direction0,scalar1,time0; velStart and hasStarted untouched. GroundAnimation82d83dd0 constructs JumpInfo zero/newvelocity/comheight0/doJumpfalse but leaves jumpedThisFrame/jumpVel unknown. Raw83e00/08/10/14 confirms every vector lane, omitted in HexRays.

KnownAir82d8b578 binds physics_airstates, GrindAirAdjust and FootPlant. LandingNormalUP; apex/collision/start/error/targetCom vectorsZERO; six booleansfalse, time/maxY/collTime/timeToApex/bodyFlipSpeed0,index0. LandingHeading,collSpeed,startY,comMaxY remain unknown. Five Grind subclasses share82d84fa8 and install their distinct derived identity: grindDir/normal/skewUP, precontact I/J/K/ZERO matrix, grindSubstateInvalid0, flagsfalse,crouch/frames/invexit0,jumpVelZERO, actual GrindJumper. Boardslide PDB240 adds wipeoutImpulse/applyWipeoutImpulse/preparingToJump, all untouched by its inline constructor; other four subclasses PDB208 add no fields. Nonspecific inline82e048e4/e8 copies GrindJumper only; jumpedThisFrame/jumpVel unknown. FollowPath has base fields only.

Teleport82d911b8 copies actual conditioners, preserves resetTransform unknown; bitfield raw100=(old&1fffffff)|20000000 means teleportToOnBoardtrue/dataAcquiredfalse/readyfalse. HandPlant82d972c0 binds actual attributes/HandPlantManager, descriptorInvalid0/object0; queryManager,grabObject,groundBlend,timeToApex,state remain unknown. FootPlant82d975d8 binds attributes and actual FootPlantManager only. Boneless82d977e0 binds attrs, landingToeIndex-1,positionZERO,rightLegfalse; raw97890..b0 proves zero vector including W.

OffBoard82d916d8 copies actual GrabSpline,FeetIK,BoardController,Location/JumpTest; two vectorszero,5flagsfalse,descriptorInvalid0,groundBlend/autoTurn0. Locomotion82d91b18 copies GrabSpline,FeetIK,BoardController,LandingDeck; descriptor/groundBlend/booleans/delay0. LandingDeckState82d97f48 binds BOTH offboard-air and landing attributes, actual LandingDeckManager; four booleansfalse, justHippyJumpedint0 and scalar landing/turn/angle0. OffAir82d94040 binds offboard-air attributes and actual selectors/controllers, scalar bodyspin/angles/frame0, two flagsfalse; OffboardTrajectoryResult.Reset82d88138 zeroes all five vectors/time/frame/valid, raw88160..1c8 proves all lanes. OffJump82d98428 binds actual jump attrs, FeetIK/BoardController/LandingDeck/JumpTest, trajectoryZERO/time-1, eight scalar metrics0, two booleansfalse. Shared-stack scalar loads are -1, not uninitialized.

## Full Skitch constructor and Reset

Skitch82d9d908 binds its actual class, PlayerGrabSpline and embedded shared GrabSpline::in_uninitialized_allocation() (root implementation); descriptorInvalid0. Reset82d887f0 initializes seven matrices to I/J/K/ZERO, preserves mGrabSplineToIKHandSpaceTransform unknown, and sets actual self-relative matrix selectors: general spline/Future, IK/Future, physical/Current. These selectors refer to the same retained struct's matrix slots. All14 vectorsZERO and all named scalar fields0 except centrifugalAccelerationSign untouched and delayUntilHandsOffMatters read late. GripNone0, centrifugalExceededByGrip0, FollowingVelocity0, skitchEntering0; four hand states OFF1. Raw flags at1296 preserve bit2 mApplySideForce, clear other7; byte1297 clears inAir/steering then sets firstEval only AFTER actual delay attribute read. Low unnamed storage remains unclaimed.

Late GetAttributePointer82d88c28 uses raw keyCC257F85657759A1 (assembled82d88bc4/bcc/bdc), scalar fallback DefaultDataArea4 means0. All reset prefix fields remain observable if that read fails. Embedded GrabSpline retains unknown lengthUnit and low status bits from its noncleared parent.

## Full Pushing constructor and Reset

Pushing82d9c980 binds pushing and offboard attribute instances, actual GrabSpline/FeetIK/BoardController, descriptor and embedded shared non-Sim GrabSpline. Native vector PID control read82d9ca9c keyDF79539DBDA006EE, then four control stores and vector Reset82e48ec8; angular PID read82d9cafc keyB46764285AD1DC5F then four controls and scalar Reset82e48ef0. Both use DefaultDataArea16 only for real absent data. A failure in second PID read retains the first complete PID and earlier allocation prefix.

Reset82d91d70 writes three I/J/K/ZERO matrices, descriptor, calls SAME embedded GrabSpline.Reset preserving its unknowns, zeroes22 vectors and preserves three unwritten vectors targetPushNormal/lastPushNormal/targetXDirection. Fresh raw91e68..91f48 proves temporary vectors zero before loads despite HexRays reordering; no use of uninitialized vector constants. It resets PID histories without resetting their actual controls. Scalars splineBlendCoeff/Adjust=-1, animCrouch=.5, objectMass/forceMassScale/objectYTensor/torqueScale=1, remaining written scalars0.

Then raw82d921b8 loads actual kEnableRepushingTimeLimit target,bc reads Boolean current. If true only, GetAttributePointer82d921e4 keyB6730894F9DFE0B3 supplies delay, absent DefaultDataArea4=>0. Then delay stored and final7flags cleared. On cvar/attribute failure, delay and finalflags remain unknown while all earlier reset prefix is retained. Shared GrabSpline.Reset implemented by root preserves lengthUnit plus low5 status bits/mask.

## Actual Vault class keys

All collection keys are D7EDBD362D7D2152. These are raw assembled 64-bit keys, not malformed decompiler arguments.

- Revert018E8A2E5028AB3F: generated wrapper82d91230 raw50/54/5c.
- KnownAir28ABB872CB8CB9ED: generated wrapper828b0c30 raw50/54/5c.
- Skitch032CD3DE521B4ECD: raw82d9d96c/974/984.
- HandPlant4AB4B3C7411F583A: raw82d97324/32c/33c.
- FootPlant172D553937D8E07C: raw82d9763c/644/654.
- BonelessCCB95A83C78B4FF9: raw82d97838/840/850.
- Offboard492026E0F096F283: raw82d9173c/744/754.
- Pushing3EDA5B140604613D: raw82d9c9dc/9e4/9f4.
- OffAirCDD926B35EF0F6C4: raw82d940a4/ac/bc; also LandingDeckState.
- Landing2377CEF0FF59C4D2: raw82d98000/7ffc/8008.
- OffJump118B9162E0262FBF: raw82d9848c/494/4a4.

Selected original Vault loader must actually include these classes before testing nonNULL instances. No verified tuning payload is hardcoded.

## Cvar44 and validation hook

Initializer82f93170/table82fd59e4/ref830d9e68: kEnableRepushingTimeLimit, sectionPhysics_States_OffBoard_Pushing, titleEnable Repushing Time Limit, Booleantrue, callbackNULL. Registry43->44, source registration43 afterwipeout34/beforehanddrives6. Getter samples live target independently. Focused test checks NULL,defaulttrue,INI0,fresh-manager relinktrue. Root owns Cargo and strict checks.

check_original(&RemainingStates,&Arc<PhysicalPlayerAssemblyOwner>) verifies22 completed allocations, same physical/Processed/collaborator identities, source unknown fields, UP normal, actual Skitch/Pushing attributes, actual PID control data, preserved nested GrabSpline unknowns, and key reset values. It is intended for the actual paired original corpus owner after root mounts these stages.

