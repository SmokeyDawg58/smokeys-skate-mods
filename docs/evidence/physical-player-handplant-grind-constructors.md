# HandPlantManager, GrindManager and World Predictor construction

Date: 2026-09-12. Author ground_path. Root independent
review and integration gates pending at authorship. No actual handplant/grind
update, launched trajectory query or successful prediction is implied.

## Actual allocation ownership

HiLOD allocates HandPlantManager0xa00 under Sim at82e0408c, constructs4098,
publishes+2cc40a8. It constructs GrindManager later at82e042bc, after Wipeout
and before FootPlant. Root separately verified Sim OR0x100 zero-fill policy;
these selected parents therefore have known zero in untouched cells. Their
separate PhysicsSystems/PhysicsGrinds children are not Sim allocations.
Unknown child payload is not promoted to zero merely because its parent is.
Root independently checked PhysicsGrinds category43 row82ff0d90 has ORflags0,
and PhysicsSystems category41 likewise has ORflags0.

The World Predictor is a World.Init dependency, not a per-skater fallback.
World.Init allocates size0x320 under PhysicsSystems827b114c, reads actual
global physics layout gravity (World+40 -> physics+10)1158..1170, calls
Predictor ctor1174 and publishes the completed owner to World+34 at11a8.
Static HandPlant selector predictor is assigned from World+34 at1248;
World destroy clears it827ab31c. Host retains manager authority and the
actual generated physics layout owner alongside four-lane gravity. Source
ctor82e4b6c8 initializes its Futex, stores world+20 and gravity+30, and
creates an empty fixed vector of90 InternalQuery entries at+40..31f.
No InternalQuery is manufactured before a launch. Root owns session slot
registration and the explicit World-init schedule before Actor construction.

## HandPlant hierarchy

HandPlantManager ctor82dc1748 executes position-selector attributes, embedded
HandPlantTrajectorySelector, position result, outgoing trajectory result,
manager attributes, four trajectories, IK data, animation info, then Reset.
Raw generated constructors prove these classes, all using default collection
`d7edbd362d7d2152`:

- positionselector `55b5940075f78842`,82db7324..733c;
- manager `0f38084037cb9175`,828b143c..1454;
- trajectory `2656db32bef0b68d`,82dbeef0..ef0c.

Embedded selector PDB size0x5b0: attributes+0, six prediction futures+10
(stride0xb0), winning pointer+430, trajectories+440/+480, primitive+4c0,
hand position+4f0, reflect normal+500, landing normal+510, landing position
+520, surface+530, time+534, outgoing result+540, SurfaceQuery pointer+5a0,
readiness+5a4. Six future constructors set trajectories to zero/zero/zero/-1
and readinessfalse. Other embedded fields are known from the actual Sim
allocation, and still do not indicate a completed prediction.

Selector ctor82dbeed0 separately allocates SurfaceQuery0xc0 at82dbf008,
constructorf020 receives the actual static World predictor binding. A null
allocation publishes null. SurfaceQuery ctor82dbee50 stores the supplied
nullable predictor, constructs its trajectory and false readiness, then
Reset82dcd038 only clears readiness again. Its separate nonzero-fill
allocation leaves collision result, normal, surface, radius and error cells
unknown; host reuses the explicit unknown prediction layout.

Selector Reset82db5eb0 preserves winning pointer, six futures/readiness,
trajectories, result and primitive. Raw stack stores prove landing normal
UP and reflect normal UP, hand/landing position zero. Stores are time0 at
5f38, surface0 at5f40, readyfalse5f44, landing normal5f50, hand5f5c,
reflect5f60, landing position5f64.

Manager PDB size0xa00: selector+10, animation+5c0, position result+5e0,
trajectory result+640, attributes+6a0, four trajectories+6b0/+6f0/+730/+770,
primitive+7b0, three up vectors+7e0..800, four matrices+810..8d0,
Bezier vectors+910..940, grind normal+950, six time/scalar vectors+960..9b0,
ready+9c0, surface+9c4, IK+9d0. IK blend0, distance[-1;4], flags clear high2.
Animation position0, estimatedFLT_MAX and high flags masked0x4fffffff.
Raw constructor/reset stack stores establish all vector W lanes.

Manager Reset82db5c78 clears readiness first, masks animation flags while
preserving bit30, sets animation estimateFLT_MAX/positionzero, resets found
position/dirVsUp0. It writes four time vectors[-1;4] at960..990, two full
zero vectors at9a0/9b0, surface0, then grind normalzero at950. It preserves
the embedded selector, all four trajectories, IK data, primitive, matrices,
up vectors and Bezier storage. HandPlantDirVsUp NONE=0 confirmed PDB enum.

## Grind hierarchy and partial construction

GrindManager ctor82dd36e0 first captures same Processed/board/statechanger,
constructs GrindGeometry then immediately zeroes the entire0x190-byte
GrindInvestigation via Reset82b39088. The earlier geometry initialization is
overwritten. Consequently investigation grindType=0 (5050), not NONE=-1.

Seven separate allocations and publications in source order:

| Child | Size | Alloc | Publish | Constructor effects |
| --- | --- | --- | --- | --- |
| Collision | 7a0 | 3740 | 3764 | same Processed, resultCount0;40 primitives unknown |
| Materials | 14 | 3778 | 37a8 | same Processed/board/collision/trajectory, wasTargetingfalse |
| Investigator | 40 | 37bc | 3808 | same Processed/board/collision, five counters0, hangfalse, lastVelocity0 |
| Balance | 40 | 381c | 3898 | same Processed, UP normal, RIGHT high-side, timers0, fresh cvars |
| Engagement | c | 38ac | 38d0 | same Processed/Wipeout, tipslideGrabTimer0 |
| Controls | c | 38e4 | 3904 | same Processed, LR/UD0 |
| Jumper | 70 | 3918 | 3938 | ctor82dc70c0 described below |

Addresses in table share prefix82dd. Null children are published null and
later constructor allocations continue. Materials/Investigator retain the
same Collision Arc when nonnull, including actual null on allocation failure.
Host keeps explicit unattempted/null/constructing/published child phases.
Balance candidate is retained separately until publication, so a later cvar
read failure cannot erase its allocation or prior counter store.

GrindBalance source does NOT capture both cvar targets together. Raw384c and
3864 merely form reference addresses.3874 loads lean target,3878 reads int,
387c adds1,3880 writes counter.3884 then loads curb target,3888 reads float,
388c writes exitStartTime. Host uses separate fresh getters around the
retained counter store, with wrapping integer addition. Initializers:
82f99dd0 Physics_Grinds_Balance/kLeanMinNonGrindFrames Int20 range0..100;
82f99e28 same section/kCurbExitStartTime Float2 range0..4. These are live
cvars with display ranges, not enforced simulation clamps.

Jumper ctor82dc70c0: justJumpedfalse, avoidanceFrames0, geometryType0 thin
rail, lastGrindType3 (5_O), energyFraction1, highSideRIGHT, grindNormalUP,
grindDirRIGHT, mostVerticalNormalUP, primitivePositionzero. PDB enums verify
these values. Final Manager stores reset time/gravity relief/flicker state,
investigation grindPossible, forbid/flicker flags, score and avoid frames.

The new constructor owners do not resolve the separate live grind
investigation, collision query, material choice, balance or jump updates.
Root controls all registrations, allocation scheduling, actual corpus
checks, and Cargo gates.
