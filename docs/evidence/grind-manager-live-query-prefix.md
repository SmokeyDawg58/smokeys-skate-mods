# Actual GrindManager prefix through nearby query

Implementation in physical_player_grind_update_prefix.rs and
physical_player_grind_find_nearby.rs stops immediately before Materials.Update
82de1830. It does not claim whole GrindManager.Update or Update1 completion.
Root owns mounting the same actual paired physical owner and original corpus.

## Retained ownership and call order

PhysicalPlayerAssemblyOwner.grind() exposes the existing Mutex<Option<GrindManager>>.
Its constructor binds the exact processed Arc, weak physical owner, and actual
board handle. GrindCollision remains the SAME Arc shared by Manager, Materials,
Investigator. Its old disconnected count/primitive fields were replaced with one
Mutex<GrindDataQueryResult>; count0 and unknown slots/lowflags are constructor
state. clear_count preserves old slots and flags. No shadow collision results.

Update82de1798: Reset investigation17b0; MonitorFlickering17b8;
UpdateGrindPermission17bc; category8ec400 or state8e4=701 selects dt93c + previous
time1c4 (source fadds17f8), elsepositivezero; store180c precedes live Enable target
1814/value1818; false1820 skips FindNearby WITHOUT clearing previous query result.
True calls retained Collision1828. Each earlier mutation survives a later error.

Monitor82dd74c8: capture current8e4 once. State changes400/402/404 sample DeckScore;
403 samples TruckScore. Addition wraps signed32, then publish flickerState.
Always decrement score wrapping, signed clamp0, publish score, then sample
TooMuch threshold and publish boolean using signedgreater. No wholesale cvar
snapshot or GUI clamping.

Permission82dd7580: actual tooMuch selects live AvoidFrames, else wrapping
counterminus1/clamp0; publish avoidcounter75c4 first. Capture8c4 and shortcircuit
10000000 then08000000; capture8c8 at75e4 and test20,10000; then live8cc01000000;
then avoidcounter>0; actual StateChanger761c; finally ORIGINAL captured8c8/10
with latecategory8ec!=400. Unknown8c4 is checked only for consumed bits.
StateChanger lock is acquired after releasing processed lock, preventing recursive
acquisition. Its same processed occurrence is then supplied to the source helper.

WouldPreventGrind82dd3a68: nonAir category!=200 returnsfalse without cvars. Air
reads stateTimer974, then live minimum; raw3a90=40990024 is NOT_GT, includingNaN,
and returnstrue. Only remaining branch reads actual +28 postGrindJumpCounter then
live GrindJumpAirFrames; signedcount<=threshold returns true. Constructor bindings
are required, but unrelated unknown currentState+10 is not consumed.

## FindNearby82cfa500

Capture flags8cca528 BEFORE full decktranslation+70a530. Read radius ONCE a540;
construct radiusXYZ/W0; sourceflag04000000 clears only counta558. Captured unknown
translationW is not demanded on this skipped arithmetic path. Active branch
forms position-radius / position+radius as fourlane separate operations, then
reads DebugNearby a598. Source BoxTest82cf97e0's DoDebugDraw parameter has no
observable body consumer in decomp; retain its caller read, do not add debugdraw
or singleton requirements. Root independently reviews this raw omission.

Callback executes actual world try_box_test(bounds,result,false,cvars); boolreturn
is unused by native caller. Count exactly40 then requires actual DebugOverlay
at a5b4..e4, even when debug toggle isfalse. Overlay failure preserves queryresults.
No exception path resets the result or substitutes misses.

## Live cvar source packet

Null callbacks confirmed by constructor forms; root raw review remains the gate
for claims. Float GUI range is metadata; integer targets also remainunclamped.

|Index|Label|Ref|Initializer|Table|Default|Section / title|
|---|---|---|---|---|---|---|
|70|kGrindableDetectionRadius|830a6a80|82f8a820|82fd530c|3f99999a|Debug_Physics_GrindDetector / Grind Detection Radius|
|71|kDebugDrawNearbyGrindPrimatives|830a6888|82f8a880|82fd5310|false|Debug_Physics_GrindDetector / Do Nearby Grind Debug|
|72|kEnableUsualGrinds|830dca54|82f9baf0|82fd6024|true|Physics_Grinds / kEnableUsualGrinds|
|73|kDeckFlickerScore|830dd514|82f9bb40|82fd6028|50|Physics_Grinds / Deck Grind Flicker Score|
|74|kTruckFlickerScore|830dcb70|82f9bb98|82fd602c|20|Physics_Grinds / Truck Grind Flicker Score|
|75|kTooMuchFlickerThresh|830dc7c4|82f9bbf0|82fd6030|151|Physics_Grinds / Too Much Flicker Thresh|
|76|kStateFlickerAvoidGrindFrames|830dc13c|82f9bc48|82fd6034|90|Physics_Grinds / State Flicker Avoid Grind Frames|
|77|kGrindJumpAirFrames|830dc318|82f9bdb0|82fd6044|10|Physics_States_StateChanger / Grind Jump Air Frames|
|78|kStateChangerMinTimeInAir|830dcfc0|82f9beb8|82fd6050|3da3d70a|Physics_States_StateChanger / Min Time In Air|

## Selected bootstrap order

Simulation calls World.Initialize828a6cec, whose GrindData.Init is827b12a4;
LivingWorld.Init follows828a6d40; installedproviderstore828a6f3c comes afterward.
Root selectedsession owns staticdata and LivingWorld Arc allocations across
registration/query/removal. Selected ownership is distinct from full execution of
all these native functions. See living-world-empty-grind-provider.md.

## Validation scope

Focused tests exercise actual factory-issued StateChanger constructor identity,
nonAir skipping unlinked cvars, AirNaN preserving NOT_GT earlyreturn beforemissing
counter, and mandatory minimumread. Cvar tests cover all9 defaults and unclamped
live values. Existing original constructorcheck verifies shared CollisionArc
aliases and all40 unknown flag slots. Root owns actual original query mounting,
all Cargo/clippy gates, and independent source review. No Cargo run by child.
