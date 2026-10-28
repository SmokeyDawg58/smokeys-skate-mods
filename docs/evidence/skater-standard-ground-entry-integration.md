# Actual SetStandard integration, 2026-09-08

Accepted strict Clippy 51289/d9a638; workspace tests45269/28c7d1 pass3467 units
(3224 core,202 formats,38 app,3 symbols),6 docs. Original91383/2a334c twelve PASS;
format/diff487c25 PASS. Initial Clippy doc/test-style findings were corrected
before the accepted strict run. Independent source/code review
approves leaf, material ownership, actual bridge, publication/inertia and corpus.

Source Skateboard.SetStandard82b355a0 -> Body.SetStandard82b32d70 ->
SetStandardAngularDamping82b2b818 and EnableCollisionVolumes82b241a8, detailed in
physics-ground-enter-runtime-plan.md. Native store/read ordering is high
confidence from fresh raw review; whole-game per-tick parity remains unproven.

The live world entry validates the registered physical owner and SAME attached
construction/session template. It clears existing free_wheel_enabled, reads the
continuing GLOBAL physicsdeck.default key8d3065ced858a7df at runtime, resolves
actual Deck6 and captures its SAME mutable inertia before the angular-drag store.
Native multiplication uses literal bits426fffff; original data yields26.999998.
The host captured-inertia setter checks both allocation occurrence and retained
owner identity; it changes only angular_drag.

Seven Body materials now have private stable Arc storage from constructor.
Source eleven attribute reads precede original stores through unique ownership;
no material ownership can escape before Complete/HandedOff. SetStandard captures
deck/wheel/truck owners at the source stage and installs those exact Arcs in
Part order6,0,1,2,3,4,5. Original templates remain immutable construction evidence.

SAME CollisionDefinition owns live volume flags initialized once from original
components: top0 deck aggregate,top1 shared trucks,top2 shared wheels, plus deck
children. The leaf ORs1 in top order and ascending children, rereading signed
count and preserving native16-bit child address indexing. Revisions are host
freshness certificates, not native fields. Every successful flag write invalidates
older bounds through revision comparison. Actual serialized Update samples live
enabled flags while preserving existing primitive geometry; refresh does not
replace the definition or publication identity. Controlled original fixture
disables/re-enables shared wheels to prove staleness and refreshed behavior.

Actual SetStandard corpus validates runtime damping against the supplied Vault,
unchanged other inertia fields, exact material identities for all7 Parts,
3+childcount flag writes, stale bounds, then same-publication enabled/material
records after real refresh. Leaf tests cover staged refusal and floating-point
edge cases; actual captured-inertia test rejects an unrelated live owner.

This completes only PhysicsGround.Enter's first board effect. Existing normal
Skeleton collision can be reused next; retained SkeletonController, IK/truck
tilt, hook/manual/speed/Pumping/Wipeout and processed category branches remain
separate requirements. Actor/generated-state scheduling and playable presentation
remain open. No full ground entry, native frame trace or playable parity claim.
