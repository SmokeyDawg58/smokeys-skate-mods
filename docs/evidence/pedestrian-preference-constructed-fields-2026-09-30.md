# Pedestrian interaction preference field admission

Runtime failed after4068 ticks with unsupported constructed value
f89323e420a6aaa3. Existing DecideToUseBehaviourSource consumer reads this
preference array; no second interaction implementation is needed.

IDA82e9ac50 fetches the entry, copies its RefSpec and reads probability at24.
Type registry82ff0f48 index13 maps e4aac2eabc053d0a to singleton82ff12a0,
vtable82167b38. Construct and Destruct point to825187c0 (no-op). Assign828b5658
copies RefSpec and the float24, confirming the authored key/probability are
not transformed by construction. Existing evidence records original Vault
stride32 for this type.

Shared typed array field admission now includes this exact type/stride and
only key8..16/probability24..28. Cache16..20 and whole-record reads remain
rejected; relocation, inherited-owner, type and stride checks are retained.
Extended existing regression case to this type rather than duplicating tests.

Builds and runtime diagnostics now run for the full pedestrian
goal. Release build and runtime verification are in progress; this source fix
alone does not prove visible, moving, collidable pedestrians. Optional
SKATE2_TRACE_PEDESTRIANS logs presentation count/mesh count/positions every300
camera updates; absent that variable it produces no extra logs.

## Runtime follow-up

Release build passed (7m27, exit0). Two bounded4500-completion runs passed
(exit0): offboard-street-look and offboard-city-walk at Waterfront. Both
reported zero presentation pedestrians/meshes at every300-camera-update sample.
Artifacts pedestrian-waterfront-run.log / pedestrian-walk-run.log and matching
4500.png captures. Therefore full goal remains incomplete. These runs do NOT
prove the interaction preference branch was exercised. Census/model residency
diagnostics were added behind the same opt-in environment variable; the next
release build is artifacts/pedestrian-census-build.log. No population tuning
or forced spawn workaround has been applied.

## Census diagnostic result

Diagnostic release build passed. Waterfront1200 walking run exited0.
artifacts/pedestrian-census-run.log shows enable0xfe, max12, reserve0,
roster0, valid population candidates, Load_Model=true, but every sampled
recipe ID is NULL_ASSET_ID01678c4862139059 and resident=false.
This narrows missing pedestrians to recipe selection/loading, before rendering.
Original skater Vault confirms required parent c281eaeaad42219b has two leaf
variants and optional parent1bf4c3c172a6f81a has three; sampled parents and
leaves all have authored Load_Model byte1. No tuning workaround applied.
Additional RecipeMan.Update diagnostics print initialized/delay/load count,
three queue lengths and actual registered recipe records every300 calls.
Rebuild running in artifacts/pedestrian-recipes-build.log; tool session94756.
Next: poll that exact session, then run1200 Waterfront offboard-city-walk with
SKATE2_TRACE_PEDESTRIANS=1 and inspect queue/record output. Goal remains active.

## Admission capture correction / next step

Recipe diagnostic run1200 exited0. RecipeMan is initialized, queues start
[15,0,10], and models DO load (male/female skaters, then tourists). Earlier
all-null samples were incomplete evidence: at candidate2280 female tourist
modelcb2c143b9929a70a had resident=true but roster stayed0. No loader fix made.
Admission diagnostic release compiled; first final-exe replacement hit Windows
AccessDenied with no running game, retry succeeded.2400 walking run exited0.
artifacts/pedestrian-admission-run.log shows first16 admitted model candidates
and later samples all rejected at nav-absent across Waterfront coordinates.
IDA82e9a510 confirms this is a real spawn rejection, not to be bypassed.
IDA82e580c8/82e57ff8 and PDB pegasus::NavMesh2::tCell confirm layout56, centroid36,
fixed-cost48. Shared lookup diagnostics now report tile count/IDs and whether
a matching tile returns a cell (opt-in only). Build running session6993 ended;
retry session68956 succeeded; no games remain from these captures. New build
is artifacts/pedestrian-nav-build.log: poll latest returned session and run
2400 offboard-city-walk with SKATE2_TRACE_PEDESTRIANS=1. Need determine absent
registration vs lookup failure. Full goal STILL unverified/incomplete.
