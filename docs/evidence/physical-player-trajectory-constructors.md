# Retained trajectory selector constructors

2026-09-12 root implementation; independent input/PhysOut agent review. Confidence
high for selected constructors and ownership, no predictor execution claim.

HiLOD82e04030 constructs TrajectorySelector82dc4af8 (size0x23a0), then
82e04068 constructs OffboardTrajectorySelector82dc0bc0 (size0x14e0). They follow
Reckoning and precede Pumping/board/skeleton. Both use Sim flags1, category55
row82ff0eb0 maskFFFFFFFF/OR100. Constructor-unwritten residual fields therefore
remain known zero, unlike a general stack or non-cleared allocation.

The new physical_player_trajectory_owners.rs retains the same physical owner by
Weak and the actual shared Processed Arc. Factory publishes each selected owner
in source order. It launches no queries and does not infer prediction readiness.
Original-data assertions verify the shared binding, actual Vault instances and
unready results. Tests/gates are pending for this new batch.

Onboard: seven future<PredictionResults> of stride0xb0. Each result has collision
payload+0, trajectory+50; future ready+a0=false. Trajectory has three zero vectors
and time[-1;4]. Winning result pointer null, winning buffer same trajectory
construction. TrajectoryLaunchInfo82d83cf8 size0x110 initializes two affine frames
I/J/K/full zero column, eight zero vectors, floats/flags/count0. Seven four-vector
trajectory records and winning record zero. Initial/local/start vectors zero;
suggested landing and most-vertical grind normals UP, even with validityfalse.
Fixed list starts empty with capacity40; future insertion must enforce the bound.
Grind query count0 and zero parent primitive storage remain retained. Signed
i16 pass/count/best all0. This differs from the older trace-fed helper's default
suggested normal and Option best index; that helper is not this constructor.

Onboard's nested PossibleGrindOrientations allocation82dc4e3c is0x50 bytes in
PhysicsSystems category41. Row82ff0d60 maskFFFFFFFF/OR0 does not clear it. There
is no constructor call: geometry enum, garbage flag and all four direction
vectors remain unknown. The owner distinguishes allocated-but-unwritten from
NULL. No zero-filled orientation is synthesized for either case.

Offboard: source raw82dc0be4..c00 classCDD926B35EF0F6C4 and default collection
D7EDBD362D7D2152. Nested generated physics827b2660 class04293D0F2ADCFBB4 with
same default collection, cached layout or DefaultDataArea0x20 on later use.
Hex-Rays miscombines these64-bit arguments; raw register assembly was checked.
Sixteen unready futures; winning pointer null; launch two zero vectors/angles
but numTrajectories=1. Sixteen0x80-byte data records plus winning record construct
trajectory zero/-1 and three zero landing vectors, collisionFoundfalse. Their
indexShift/collisionIndex are unwritten by constructor but zero from parent Sim.
Winning COM and raw trajectories zero/-1, offset zero; score16, counters and
blend values zero. No scored candidate is published by those defaults.

Independent reviewer freshly verified both constructors and PDB layouts, launch
matrix W, count defaults, class keys, and nested allocation distinction. Missing
HiLOD owners still prevent full Actor+200 publication; native trajectory
Launch/Update need subsequent integration. GrindAirAdjust and HandPlant are now
mounted in source order; see hilod-motion-constructor-integration.md.

Bail/trajectory checkpoint passed3648 units, six docs, all16 original-data tests,
strict Clippy and fmt. Logs: %TEMP%\skate2-bail-trajectory-workspace3.log,
skate2-bail-trajectory-original2.log, skate2-bail-trajectory-clippy2.log; formatting
was applied and rechecked after that strict log's formatting-only failure.
