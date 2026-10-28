# Same-owner HiLOD and Motion constructor integration

2026-09-12. Confidence high for inspected constructor source; no complete
Actor constructor, running graph Tick, trajectory result or gameplay parity claim.

The assembly retains PhysOutConditioners after PhysOut at82e03f58, then
SurfacePhysics82e03f90, GrindAirAdjust82e03fc4, Reckoning82e03ff8,
onboard/offboard selectors82e04030/4068 and HandPlantManager82e04098 before
Pumping. GrindManager82e042bc follows Wipeout and precedes FootPlant82e042f4.
Each candidate is retained before fallible initialization; child publication
and allocator NULL paths remain represented by the constructor contracts.
The selected successful host route is not an allocator-failure oracle.

GrindAirAdjust uses28 fresh Vault reads. Raw9ce8/9cec stack alias proves that
standalone Sin receives deck angle times3c8efa35, not a literal radians value.
Both truck offset sums82dc9dcc/82dca200 are fmadds and use fused mul_add.
All target index and floating-point multiplicand ordering survived independent
review. Sim category55 row82ff0eb0 OR100 proves residual zero storage. Separate
PhysicsSystems41 and PhysicsGrinds43 rows have OR0; their unknown payloads
remain unknown. Root verified rows82ff0d60/82ff0d90 and Predictor82e4b6c8.

World predictor initialization occurs explicitly before Actor construction in
the combined Ground-startup corpus, retaining the actual manager and physics
gravity. HandPlant SurfaceQuery keeps that same Arc. No query is launched and
no fake collision miss is supplied. Other controlled partial-world fixtures
can retain their actual absent predictor; these do not establish World.Init.

Conditioners bind the same completed PhysOut. ScoreModule's first and second
Reset write the actual Scoring component; current Other points to the same
retained collector allocation. Independent source review is recorded in
phys-out-conditioners-independent-review.md. Cvars are carried by the physical
inputs and shared with Actor input/animation/graph setup in the combined corpus.
No private registry is created or relinked inside a physical constructor.

The concrete Actor wrapper now retains either a generated Action or Motion
controller. Actual startup enablement selects each route. Motion constructors
load all2469 behaviours,18 hooks and406 conditions against the same Vault and
cvar owners, then publish the returned controller and root-name hash. Prior
controller release, NULL result and failed-constructor hash preservation follow
wrapper Load82cef180. Motion controller fields match8292a1b0; root independently
reviewed the eager loop and per-behaviour versus final-condition sInit stores.
Callback instances remain absent until real CreateInstance execution.

Validation so far: workspace and six doc tests pass in
%TEMP%\skate2-motion-mounted-workspace.log; all16 original-data tests pass in
skate2-motion-mounted-original.log. Strict Clippy and fmt pass in
skate2-motion-mounted-clippy2.log. The subsequently rebuilt combined original
test also passes in skate2-motion-mounted-original-final.log, including the
new Ground/OffBoard output-accessor assertions. git diff --check passes.
The earlier constructor-only run also passed16/16. Negative fixtures moved their
deliberately malformed byte to board-only joint enable, since GrindAir now
correctly reads truck geometry earlier; minimal synthetic frame-count settings
are confined to those controlled failure tests.

Remaining full constructor gaps include AnimOut and subsequent offboard/grab
owners, complete26-state construction and native constructor tail publication.
Action and Motion graph evaluation, instance lifecycle, animation output and
the complete input/physics/render frame remain subsequent work.
