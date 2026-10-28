# Motion wipeout expression420

Independent read-only IDA audit, 2026-09-22. No Cargo or registered source edits.

Pre evaluator8290aa50 table8290aa90 entry420 at8290b120 contains8291ad2c.
Raw casead2c..ae98 is lazy `condition2 || condition3 || condition4 || condition5`.
True exits atae8c; false reachesae90. The same condition cache word0 uses
bits4,8,10,20; each miss passes the original parent to its direct condition.

Condition2 is HasIntent with attribute2432 intent WipeOutRequest. Condition3
is plain IsPhysicsWiping, condition4 plain PhysicsWantsWipeOut, and condition5
PhysFilteredState with attribute2433 state wipeout. Existing retained HasIntent
and filtered-state runtime paths already implement2/5; only plain3/4 and the
expression dispatch require additions. Transition declarations2 and326 both
use expression420, priority2, target4. This is graph selection, not permission
to skip the target lifecycle or complete Wipeout physics.

IsPhysicsWiping Activation82c99bd8 validates actual cached IPhysical against
UCOM, samples PhysOut+1c State+32 at9cb8, then reads the live
kDebugWipeoutConditions cvar at9cc0. Its debug branch requires the overlay
singleton once and calls the folded no-op at9d44. Return the sampled byte.

PhysicsWantsWipeOut Activation82c99d58 validates the same IPhysical, calls
GetPhysOut separately for State+36 at9e48 and State+38 at9e60, then reads
the debug cvar at9e68. These are retained phys_state_wants_wipeout and
physics_requests_wipeout, respectively. Both reads occur before boolean OR;
do not short-circuit the second source read when the first is true.

In that debug branch, singleton is loaded ONCE into retained r31 at9eac.
First nonnull assertion precedes no-op call9f00. The second assertion at9f1c
checks the SAME r31, then no-op9f60. Singleton reloads9edc/9f3c exist only
after failed native assertions; host errors terminate there. Thus one successful
host require models this branch, not two fresh singleton queries.

The existing check_motion_runtime_context enforces entity and controller
identity. cached_physical_allocation additionally validates cached allocation,
published interface Arc identity, same entity issuance and registered
PHYSICAL_INTERFACE (actor_physical_mount.rs101..125). Together these preserve
the native IPhysical identity requirements; reuse them and the existing actual
PhysOut State access rather than another owner or shadow booleans.

Sustain evaluator8291e218 table8291e254 entry420 at8291e8e4 contains829221c8.
That target is `li r3,1`: unconditional true, with no predicate or state reads.
This proof admits only Sustain420, not neighboring default-table entries.

Reviewed boundary: source control flow and existing producer/consumer identities.
Root implementation and runtime verification are separate from this audit.

## Adjacent selected Idle transitions

The authored current1170 ancestry is Default1170 -> Idle1025 -> Turning719 ->
Riding277 -> RidingIdle89 -> OnGround24 -> OnBoard3 -> Motion0. Transition
sections index the separate INDICES array; they are not direct declaration
indices. OnBoard's transition is326, the expression420 reviewed above.
After false420, Idle's priority0 transition1551 targets Push1028: ValidTarget
reads that target's Pre253 before checking its Ground ancestor. Root's next
original run confirmed PendingExpression253, matching this bounded prediction.

Fresh raw Pre253 table target82911434 is `!condition18` (DisablePushBrake),
condition pointer+48/cache word0 bit40000. Existing actor_action_disable_push
implements that exact native82c9a8f0 predicate; root extracted its source body
for reuse after separate Action/Motion context validation rather than copying
the numeric or attribute graph.

Next adjacent Idle target Manual1027 uses Pre4298291bb74:
`c345 && (!c346 || !c347 || !c348)` in exactly that lazy order. Pointers564,
568,56c,570 name HasIntent, LastState, ComVelCompare, SurfaceSlope. Shared
false exit82910254 loads0. Idle transition1555 later uses Pre4148291a6d4:
`c14 && (c337 || c338)`; pointers38/544/548 name IsRidingFakie,
IsRidingSwitch, InParentStateForTime. Missing runtime predicates337/338 and
347/348 remain explicit pending if reached; branch admission does not invent
their results. Existing HasIntent/LastState and same shared condition cache
are reused.

Sustain table8291e254 entries253,429,414 all independently point829221c8:
constant true, no reads. Root's four-file implementation and focused lazy/error
tests were independently reviewed against this source and approved. No reviewer
Cargo execution; root owns runtime validation.

## Push selection and shared condition construction

Independent fresh IDA review on 2026-09-22 confirms Pre3 table entry8290aa9c
points8290b2a0: condition14 IsRidingFakie, pointer+38, cacheword0 bit4000.
Sustain3 entry8291e260 points829221c8, constant true without predicate reads.
The actual factory first tries shared ConditionClass names. Thus condition14 is
Shared(IsRidingFakie,Plain), and condition18 DisablePushBrake is likewise
Shared(DisablePushBrake,Plain). A Motion-only runtime arm for18 was unreachable;
the corrected shared arm reuses the original DisablePush source kernel. Explicit
factory-shape regression tests now construct the real condition declarations.

After target Push1028 passes validation, its first child PushFromFakie1178 uses
Pre3. Its children1276/1277 use Pre254/255. If Pre3 is false, sibling PushStart1179
has no predicate, but its children1278/1279 use Pre416/417. These are authored
possible paths, not proof that a runtime selected any specific child.

Fresh native targets and exact short-circuit graphs:

- Pre254829114a8: c362 (RightPush), pointer5a8, cacheword11 bit400.
- Pre25582911508: c363 (LeftPush), pointer5ac, cacheword11 bit800.
- Pre4168291a89c: c362 || c364 || c365.
- Pre4178291a9c0: c363 || c364 || c365.

Both latter graphs reuse c364 pointer5b0/bit1000 and c365 pointer5b4/bit2000.
All four Sustain table entries point829221c8 (true, no reads). Conditions362/363
are Motion HasIntent payloads with authored RightPush/LeftPush attributes2692/2693.
Conditions364/365 are Shared CurrentState(String), constructed from attributes
2694/2695 naming PushFromFakie/Switch. They are not Motion plain predicates.

Fresh CurrentState.ActivationFunc82c94ae0 reads the condition name and parent,
then calls GetSubState virtual+2c at82c94b3c, CurrentState virtual+18 at82c94b74,
and StateHasDescendent virtual+38 at82c94bb4. The last call receives current leaf
as first argument and resolved target as second. LastState82c94a00 uses the same
name-resolution/ancestry operation with its last-state getter+1c. The implementation
therefore reuses the existing state_matches helper while supplying current_state
or last_state from the already-held Motion controller; no second controller lock
or copied authority is introduced. Name lookup remains before ancestry traversal.

The six-file root batch was independently approved for source/code scope: lazy
OR success/error prefixes, direct leaf reads, constant Sustain no-read tests, and
real constructor-shape tests. Reviewer ran no Cargo; root owns integration gates.

September22 Pre441 continuation: fresh independent raw audit table8290b174
points8291d374. Exact lazy c371||c372||c373||c374||c375, pointers5cc..5dc,
cacheword11 masks00080000..00800000. Motion HasIntent attributes2702..2706
name OllieAnticipation, PopShuvitAnticipation, FSPopShuvitAnticipation,
360PopShuvitAnticipation, FS360PopShuvitAnticipation. Existing handler reused.
Sustain table8291e938 points829221c8 true/no reads. Root registered ordered
success/error-prefix tests and no-read Sustain coverage; independent code review
approved. Runtime gate pending for this snapshot.

Pre442 sibling: table8290b178?8291d548, lazy c376||c377||c378||c379||c380.
Sustain table8291e93c?829221c8 is true/no reads. All existing Motion HasIntent:
attrs2707..2711 NollieAnticipation, N_PopShuvitAnticipation,
N_FSPopShuvitAnticipation, N_360PopShuvitAnticipation,
N_FS360PopShuvitAnticipation. Pointers5e0..5f0, cacheword11 masks01000000
through10000000. Native exits true8291d70c/false8291d710. Authored leaves1286
AnticTail441 and1287AnticNose442 share1185AnticInto. Original fresh startup
17/18 now actually reaches442 after441 false. Source approved; root registered
lazy/error-prefix and Sustain no-read tests, original rerun pending.
