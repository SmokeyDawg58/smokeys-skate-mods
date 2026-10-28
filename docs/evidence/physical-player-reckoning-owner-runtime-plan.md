# One physical-player Reckoning owner and its native phase

2026-09-08. Read-only original XEX/PDB and repository audit.
No code or Cargo. Coordinated with the GroundAnimation author, who owns jump
history/type0 dispatch research rather than this Reckoning ownership packet.

## Native occurrence and construction order

PhysicalPlayerHiLOD::PhysicalPlayerHiLOD82e03c50..82e04ddc SHA256
3acf66b57c929cabff86c71671972c9eb254fa93716205e8d9d91ff0ebbd8ded.
One allocation size0x370 at82e03fe8, constructor82dd7ab8 called3ff8 with the
actual ProcessedPhysIn pointer loaded from HiLOD+2b03ff4. Result stored in
HiLOD+2bc4008. Null allocation takes the native null branch4004; successful
Rust allocation/constructor is a bounded supported path, not debugger recovery.

Relevant actual constructor chronology:

| Stage | Call/store |
|---|---|
| ProcessedPhysIn ctor |3ef0, HiLOD+2b0 store3efc|
| PhysOut ctor |3f20, HiLOD+2ac store3f38|
| PhysOutConditioners |3f58|
| SurfacePhysics, same input/output pointers |3f90, +308 store3f9c|
| GrindAirAdjust |3fc4, +30c store3fd4|
| Reckoning, same processed pointer |3ff8, +2bc store4008|
| TrajectorySelector/Offboard/HandPlant/Pumping/AnimOut |4030/4068/4098/40c8/40f8|
| Skateboard |413c, +2f8 store414c|
| Skeleton |41c8, +2fc store41d4|

Board constructor arguments load SAME HiLOD+2bc at4130 and processed+2b0 at4138.
Skateboard ctor82b38918 retains r7 inr26 at38930, then stores that pointer at
outer+10438b1c; processed r6 retainedr28 then outer+11038b34. Skeleton similarly
loads SAME +2bc41bc asr6 and +2b041c4 asr5 before its constructor. There is no
second per-board or per-skeleton Reckoning. FillPhysOut reads board+104fe00/fed4;
HiLOD later reads its own+2bc4f44 before Reckoning::FillPhysOut4f48.

Reckoning ctor82dd7ab8..82dd7e5c SHA256
55194bba2121d7e4054e41890ce08dcede3d8f37b4be391b1f6832c6bb46ecbb.
Generated own physics_reckoning ctor7aec; retains processed pointer+107af0;
allocates/constructs BodySpin7b0c/7b1c, pointer+147b2c; then matrices/vectors,
PID controls/values and final scalars/flags. Own generated attribute instance
is not the global singleton used by some FETCH/graphs. Reuse existing
ReckoningVaultAttributes owning both scopes; do not copy constructor tuning.

Six matrices Ground20/Transform60/NoFlipA0/InverseE0/Flip120/BodySpin160 start
I/J/K/P0, all W0. DynamicUp1a0,Up1b0,UpTarget1c0,GroundNormal1f0 start J;
Heading1e0 and FlipAxis200 start I, not K. Speed1d0/tilt210/tiltspeed220 zero.
PID Ground230(control4c0)7de4,Slow290(control4a0)7e04,Fast2f0(control4b0)7e24,
initial value/target J. Final350..368 scalars and36c/36d flags zero.
Existing Reckoning::try_new reproduces selected successful construction, but
its fallible host API returns no partial owner. Do not call this eager helper
and claim full native allocation/PID failure-prefix preservation without an
explicit selected-construction contract or retained staged constructor.

## Source phase, not a new simultaneous pipeline

HiLOD Update1_ProcessInput82e01f98 copies prior output into SAME processed
owner, runs subsequent processing; native output latchtrue82e03ae8 remains
late. Update2_SystemLogic82e01c40 calls CheckForStateChange1c54, then clears
wipeout requests1c5c. Update3_State82dfe708 calls selected state virtual Update
through HiLOD+2a8 at e7c0..e7d0, after prior per-state/board preparation. It
then updates controllere7d8, board pre-physics e808 and force displacemente820.
The raw virtual call identifies the selected state's update slot; full state
selection/Actor scheduling is still a separate production prerequisite.

GroundAnimation::Update82d98d68..8dd0 SHA256
deb930e1eccf6d7dbe933c97e6473d48ddc144620686b58b326b440bfbdcf569:
clear jumpedThisFrame8d80; **FixedHeading**82ddd468 at8d94 using processed
GroundNormal+1d0/DynamicUp+210; UpdateAnimatedSkateboard82b5bc58 at8da4;
skeleton add-error flag8db4; UpdateSkateboard82d8a7b0 at8db8. The animated
board path calls GeneralUpdatec3a4 after root/animation/body-matrix work.
GroundAnimation's state call is therefore BEFORE its GeneralUpdate and six
board forces, not a post-solve recomputation inserted for Fill.

IMPORTANT distinction: UpdateGroundStates82de0408..0424 (SHA256
8c6b90c6655eaf59d01f3c85c1aefbb3b35aa31f1915a6caa03618e5b95ecc61)
stores DynamicUp+1a00414 and Heading+1e0041c, then tailcalls FixedHeading0420.
GroundAnimation calls FixedHeading directly and does NOT execute these wrapper
heading stores. PhysicsGround82da0de8 calls the wrapper0e94, then mandatory
UpdateHandplants0e9c and UpdateGroundSkateboard0ea8. Do not choose either
wrapper solely because a method is named grounded.

Within FixedHeading, existing bodyspin/filter/attribute/normal graph leads to
CalculateTransform82dd3da0 at82dde198, then CalculateDynamicLean e19c, then
CalculateTilt e1b0 and debug gates. A matrix store is not whole-update success:
late failures must preserve numerical prefix but withhold completed-phase
provenance. Existing calculate_transform updates ordinary/flip/inverse and
then ground basis on the SAME owner. Its ground_transform is not transform().

Following engine solve, HiLOD Update4_PostEngineAdjust82e01c78 ends skeleton
line tests1c94 then actual board UpdatePostPhysics1cac, with subsequent state/
collision/hand adjustments. This does not imply another grounded Reckoning
update. Update5_ProcessOutput82e04ea8 resets SAME output4ed0, fills skeleton
line results4ed8, Board4ef8, Skeleton4f18, Pumping4f24, GrindManager4f30,
Trajectory4f3c, THEN SAME Reckoning4f48, then later component fills. Board
surface projection consumes the retained state established in its state phase;
Reckoning's own later Fill is output publication, not state recomputation.

## Current code and smallest honest integration

Reckoning<A> already owns actual matrices, BodySpin, filters and attributes.
Its update_ground_normal and calculate_transform are actual retained producers;
ground_normal() is exposed, ground_transform needs a narrow borrowed getter.
Production roots/grounded methods accept &Reckoning; they do not retain an
independent one. RootCorpus currently constructs the only corpus instance and
PostRecordsCorpus forwards references to it. Corpus ground inputs/cvars are
controlled, not actual same-pair runtime scheduling. Moving ownership must
REMOVE that competing corpus construction and have it borrow the paired owner.

Current Rust try_update_grounded takes ground,dynamic,heading and unconditionally
writes heading before its fixed-heading graph. It correctly projects the wrapper,
but cannot represent GroundAnimation direct FixedHeading as-is. Extract the
existing body into a same-owner fixed-heading entry and make wrapper retain its
existing two leading stores then delegate. Preserve all existing lazy reads,
failure stores and tests; do not duplicate the grounded numerical implementation
or overwrite heading from a constant/current guess to satisfy its signature.

Minimal host ownership projection: add one optional retained Reckoning to the
actual PhysicalPlayerAssemblyOwner (shared Arc already identifies board and
skeleton), or a dedicated HiLOD component owner held once by that pair. Provide
scoped mutable update and immutable consumer borrows; consumers must validate
same pair and phase occurrence. Board/Skeleton do not each own a clone. Retain
same processed occurrence alongside it because native Reckoning+10 points to
HiLOD+2b0; current selected processed owner must not silently be replaced by
another equal-valued input struct. No global singleton owns this per-player state.

Initialize at an explicit constructor stage after actual processed/output and
preceding components, before native board/skeleton constructors when that full
path exists. If current pair admission still constructs board earlier, permit
only clearly labelled selected late component initialization; it is not exact
HiLOD chronology. Host storage allocation may precede native readiness.
Construction completion must be distinct from completed state-update provenance.

Next bind the direct FixedHeading input to the SAME retained current processed
owner after prior-copy/processing, with original BodySpin and Reckoning cvar
services and runtime Vault. Unknown native W in selected processed inputs is
an unresolved integration constraint because current ReckoningGroundInput uses
full arrays; do not force W0 to manufacture readiness. Extend shared typed math
only after source producer proof or explicit unknown-W propagation design.

On successful selected state call, issue host completion scoped to pair and
that state's input generation; invalidate on next attempted update/reset even
if it fails after partial writes. Do not blindly stamp it with post-solve contact
frame: state update precedes solve and Fill must read that SAME retained result
across solve. A source phase generation or sealed phase continuation is needed,
with later board contact-frame ownership checked separately.

Then existing roots/forces and selected Fill borrow the same state. No extra
UpdateGroundStates immediately before surface Fill; no Body-contact-normal
replacement and no fresh Reset between these consumers. Native Reset82dd3c50
is selective and preserves matrices/PIDs; ResetSystems82dfff20 calls it then
fresh board transform and UpdateGroundStates(J,J,board.Z)82dfff78. Placement
and reset scheduling are independent from first frame publication.

Tests: one constructor shared by board/skeleton/Fill, foreign equal-value owner
refusal, no second corpus Reckoning, direct fixed-heading preserves heading,
wrapper updates heading, late numeric/debug failure retains fields but no new
phase completion, state result stays bound across solve, reset preserves native
filters/matrices until ResetSystems recomputation, actual published normal and
ground transform match that retained occurrence. Controlled component inputs
remain explicitly labelled until current processed/Actor producer chain closes.
Confidence exact for raw listed ownership/call order, high for bounded host
integration design; full HiLOD constructor/state scheduler remains unfinished.

## Concrete factory migration recommendation

Inspection of PhysicalPlayerAssemblyConstruction::new confirms it currently
executes SkateboardForceRuntime::new immediately, before try_construct_board.
Adding Reckoning at the start of try_construct_board without moving that call
would reverse native selected outer constructor ordering. Recommended migration:

1. Keep factory.new as host storage allocation only: owner retained optional
   Reckoning, stable selected processed binding, unknown PhysOut storage, and
   board_force_runtime=None. Host bookkeeping allocation is not native readiness.
2. Add an explicit selected component initialization stage on that SAME factory
   before board construction. It binds the actual selected processed occurrence,
   initializes the one Reckoning with actual Vault, then constructs the selected
   outer ForceRuntime. Require both before existing Body Reset/routing/Init.
   If making this an internal try_construct_board prefix is more ergonomic,
   expose its selected scope and consumed/failed phase explicitly; do not retry
   a failed stage by allocating another equal-valued Reckoning.
3. Prefer this retained factory over a fallible factory.try_new: failed later
   constructor work can remain on the same owner, and existing source routing
   failure prefixes stay inspectable. A fallible try_new is acceptable only as
   a clearly bounded successful-construction projection, ideally with failure
   carrying its partial factory/owner. Neither form executes the omitted native
   Processed/conditioner/grindair/trajectory constructors by implication.
4. A selected processed binding is an occurrence token tied to the actual
   SkeletonProcessedContact storage, not proof its full native constructor ran.
   Allocate/retain that storage once before selected Reckoning initialization;
   pass its identity through same-pair bridges. Keep separate readiness for
   selected fields, complete prior copy, toolkit and any native whole-component
   constructor. Do not bind a later corpus temporary by equal values.
5. RootCorpus stops constructing Reckoning in new. It holds the pair Arc and
   borrows the one retained instance for controlled updates and consumers.
   Returning bare references through a Mutex is not possible: use scoped
   closures/guards, or a paired session borrow that splits mutable world access
   from retained component access. Do not clone state to preserve old signatures.
   Existing controlled input/cvar fixture may remain until actual input migration,
   explicitly labelled as such; ownership migration alone does not make it live.

This separates two independent corrections: selected native relative order
(Reckoning before ForceRuntime) and full HiLOD chronology (still includes omitted
stages). It prevents the former regression without claiming the latter complete.
