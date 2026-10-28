# Retained pedestrian targeting

Source inspection only. No compilation, tests, or game execution.
Standalone Python syntax parsing was unavailable (tree_sitter module absent);
no parser installation or build was attempted. New files were checked for
trailing whitespace directly.

`pedestrian_targeting.rs` retains the existing road/no-road iterator owners and
the constructor-backed fixed/random/flee/vehicle/chase state on each actual
pedestrian. Separate mesh/physics selections represent the native pointer on
each navigation object. SetCurrentTargetIterator82e6bb68 is only pointer
assignment; it does not reset targets or path costs.

SwitchTargetingMode82e6d548 uses enum0 fixed,1 waypoint,2 road,3 chase,4 flee,
5 vehicle-flee,6 no-road. Road selects the random iterator when RAIN has no
chunks or kDoRAINPedIteration is disabled. IsRAINRead8271b4b8 checks the actual
RAINChunks element count, so the implementation uses that existing owner.
GetMaxTurnThreshold is90 degrees for both road and no-road (their vtables
point to82e77b38); the other original iterators use base180 at82e6a6c8.
The entity exposes this through its actual current-iterator selection.
Road/chase set arrival radius2, flee/no-road radius4 and Auto. No-road performs
Seek(true) before the common Seek(false). Invalid mode still performs the final
Seek. PhysicsNavigation.Seek82e6c308 only refreshes actual entity position and
heading; it does not call the iterator.

The existing MeshNavigation Seek/ResolvePath/Update kernels now accept unsized
iterator references. Their algorithms were not copied or changed. Retained
road and no-road frame adapters are selected by the entity and supplied to
these same kernels. RestartTargetTimer82e9aed0 writes the actual AI timer29.
Road state retains the original four-node lookahead/reservations; there is no
new RAIN navigation topology or random owner.

RandomDirection.Increment82e6cf30 consumes Manager's existing Sk8::Random
GetRandom(0,1), truncates value*8, chooses +X,-X,+Z,-Z,-X-Z,+X-Z,-X+Z,+X+Z,
then computes position+direction*40. Diagonals are not normalized. Raw
82e6d11c loads42200000. The out-of-range switch default retains zero direction.
Fixed and vehicle-flee IncrementTarget slots resolve to native noop825187c0;
they are implemented as such, not missing callbacks disguised as success.
Vehicle-flee constructor82e783a0 initializes vehicleID-1 and path threshold0.3;
switching updates only its ID from actual honking_vehicle_id.

Flee82e77e98 resolves wants10 through the real targetable owner supplied to the
operation. An unresolved target uses native +X. Resolved velocity/delta are
flattened toXZ, NormalizeXZSafe82de2290 uses a zero fallback, and direction
uses the velocity-facing dot, side cross and away-vector half blend. Increment
82e7f330 applies distance3 using FMA. Cost reset82e77e80 sets0.3.

Chase constructor82e78140/reset82e6ce50 keeps threshold1 and clears only
alternate-target presence. The original vtable823a4648+c points to
82e6ce60: raw instructions return a zero vector. This must not be replaced by
the chasee position; explicit targeting is a separate producer. Chase
GetAllowIncompletePath82e781c0 requires its cvar, a bound pedestrian, an actual
resolved chasee, and positive CompareDistanceXZ. Failure82e782b0 cases2/3 set
AI bytecac bit20 when that comparison is<=0. CompareDistanceXZ82707440 uses
XZ squared distance minus squared radius, verified by raw fmuls/vmsum3fp/vsub.

The waypoint mode now uses the sole typed PropMan and its concrete waypoint
group/point reservations. GetTarget82e77be0/Increment82e77cc0 preserve the
independent iterator index and entity current-waypoint pointer. Increment
unlocks the point at the iterator index, wraps through the actual group,
then locks and assigns the first available point. Missing live references
remain errors; null current pointers retain native zero/no-op branches.
See pedestrian-waypoints-2026-09-30.md for construction and stream limits.
The targeting world requires real Mesh/Rain/Manager-RNG/world-avoid/Prop
owners and actual resolved targetable facts. This change alone is not a
claim of live end-to-end AI execution.
