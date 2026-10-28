# Camera dynamic obstacle evaluation (2026-09-23)

`Iris::PathEvaluator::Update` (`0x82e46420`) resets current collision time to
`FLT_MAX` and valid to false, runs the retained static query, runs the dynamic
query, then copies current collision time to last-valid time only if valid.
`UpdateDynamicEvaluator` (`0x82e45f00`) constructs a stack-local
`fixed_vector<Iris::Obstacle,50>`, calls its obstacle server, calls
`GetCollisionTime` (`0x82e42048`) on that list, and merges a result only when
it is strictly below input lookahead. Its `fsel` keeps the lesser of current
and dynamic collision time, selecting dynamic on an equal value.

The selected server is `Sk8::Camera::PathEvaluatorObstacleServer::FindObstacles`
(`0x82e3a7a0`). It stores the camera position, velocity, and radius, then
samples `kbPedestrianAvoidance` before `kbSkaterAvoidance`; both default true.
`AddPedestrianObstacles` (`0x82e39eb0`) calls
`LW::Man::GetPedestrianMan()->GetPedestriansWithinRange` with
`kActorSearchRadius` (default 10) and a maximum of 20 returned pedestrians,
then tests each candidate and appends up to three vertically separated spheres.
`AddSkaterObstacles` (`0x82e3a198`) gets local and local-sim indices from
`ISkaterManager`, walks its actual roster in order, skips either local ID,
checks radius and approach, then appends up to three spheres of radius 0.75
at base, +0.7, and +1.4 metres. `AddObstacle` (`0x82e39d90`) returns false
at the fixed list's 50-element cap, stopping that actor's appends. Pedestrians
consume capacity before skaters.

`SphereTest` (`0x82e3c9b0`) uses relative position and velocity. It reports
time zero for strict initial overlap. Otherwise it rejects a nonclosing pair,
computes time of closest approach `-dot(position,velocity) /
dot(velocity,velocity)`, and accepts only strict intersection at that time.
Thus its time is closest approach, not first surface contact. `GetCollisionTime`
iterates list order and retains the earliest time strictly below lookahead.
PPC `fcmpu`/`bge` takes the branch for an unordered comparison because the
LT bit is clear. The Rust helper therefore requires each relevant value to be
strictly less before accepting a collision; the branch bit interpretation is
defined by the [PowerPC User ISA](https://powerpc.dev/general/PPC_Vers202_Book1_public.pdf).
Host `host_dot3` uses the repo's
existing reduction policy, so Xenon SIMD bit parity is **medium** pending a
reference runtime trace. Control flow, list capacity, and update ordering are
**high** confidence from the named debug build and disassembly.

Rust now contains the source-shaped fixed list, sphere test, dynamic minimum,
and full PathEvaluator update method for a list supplied by the actual server.
The live camera still uses the explicit pending prefix. There is no owned
`LW::ILWPedestrianMan::GetPedestriansWithinRange` producer in this first-skater
runtime, and the source-default pedestrian cvar is true. Consequently a
constructed empty list would fabricate a query miss. The next integration
boundary is the retained pedestrian manager/query result, followed by the
source roster scan and live server call. Existing `CameraObstacles` and
`SkaterManagerLocalIdentity` remain their sole owners; no competing copy was
introduced.

Focused validation: `cargo test -p skate2-core camera_prediction::dynamic_obstacle_tests --lib`
checks strict overlap/tangency, closest-approach time, earlier obstacle
selection, static minimum, strict lookahead, NaN rejection, and the 50-element cap: **3/3
passed**. The Rust crate compiled cleanly for this gate (with existing dead-code
warnings); no live pedestrian query or original-runtime parity is claimed.
