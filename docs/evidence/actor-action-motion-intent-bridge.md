# Actual Action to Motion intent callbacks

Fresh IDA audit 2026-09-12. This packet closes the callback implementation,
not the generated Action scheduling around it. The four root occurrences
identified in `character-gesture-first-tick-map.md` must execute in the actual
generated controller before the Motion gesture consumer. Action and Motion
retain separate maps on the same Actor; no temporary maps or input aliases.

## Source and callback lifetime

Constructor82cba338 is already represented by the actual generated
`BehaviourPayload::TransferIntent`: MGIntent, AGIntent, optional default,
scale1, first filter (filter1 overrides filter), second/fakie/mirror filters,
and onUpdate defaulttrue. The new callback consumes that existing payload.

CreateInstance82cba730 allocates0xc from StateGraph, initializes intrusive
refcounter then byte8=true at82cba798 and replaces the vtable with the shared
UpdateRidingFakie::Data vtable at82cba7a0. Null allocation remains null.
Native refcounter/token storage is represented by retained Rust ownership;
per-occurrence `TransferIntentInstance` contains the callback byte and an
opaque identity used by the real shallow Context instance overlay.

Begin82cd6290 calls CreateIntent82cd62a4, then writes byte8=true82cd62b0.
Update82cd62c8 loads onUpdate and the actual context instance. If onUpdate is
false and byte8 is false, RemoveIntent82cd62f8. It reads onUpdate again;
if true, CreateIntent82cd6310. Sole suffix82cd6318 writes byte8=false.
Thus onUpdate=false survives the first Update after Begin and is erased by
the second. End82cd4288 is a tail branch to RemoveIntent, not a no-op; it
does not clear or otherwise change byte8.

Host checked failures retain already completed side effects and omit the
unreached suffix. The bridge first rejects a foreign Actor issuance,
instance token, or controller identity against the actual registered Action
wrapper. This host domain guard uses a short released prefix lock before
callback work. Generated scheduling must retain/overlay the actual instance;
this module cannot authorize an arbitrary caller to select occurrences.

## Actual map and animation ownership

CreateIntent82cd52d0 resolves IActionGraph, then IMotionGraph, checking each
cached pointer against the actual UCOM lookup and requiring both. It obtains
Motion destination map at82cd5424 and Action source map at82cd543c. Local
value starts at mDefaultValue; HasIntent82cd5450 writes it when the key exists.
An existing zero value is present. Absent source plus no default erases only
mMgIntent at82cd5474. Otherwise fmuls82cd5494 computes scale*value;
ApplyFilters82cd5498 follows, then destination operator[]/store82cd54ac/b0.
RemoveIntent82cc1e58 checks only IMotionGraph and erases only mMgIntent at
82cc1f1c. Source Action is never cleared by these callbacks.

ApplyFilters82c98fa0 resolves ISkaterAnim and checks actual UCOM equality.
A nonnull concrete Actor with null cached animation asserts at82c98ff8;
the null-interface bypass is reachable for a null entity, outside this
same-Actor bridge's accepted context domain. Existing retained animation
occurrence is checked, and its live fields supply the two virtual queries.
First filter+58 then second+5c execute82c99060/68. GetIsRidingFakie virtual+c
is called82c99080; if true apply+60 filter. GetIsMirrored virtual+1c follows
82c990b0, then optional+64 filter. Fresh getters82cb7530 and82cb7598 read
ISkaterAnim+e4c masks20000000 and40000000 respectively. These are existing
constructor-known player flags, not AnimOutPhysIn Reset flags or inferred
values. The bridge reads each at its proper query point.

## Filters and erase

ApplyFilter82c8bfd8 raw confirms eight existing enum meanings. Negate is
fmuls by -1, Abs fabs. OneMinus is signed: fsel(value,1-value,-1-value),
so negativezero selects1. Clamp performs fsel(-value,0,value), then
fsel(1-low,low,1), yielding positivezero for either zero and1 for NaN.
AngleFlip similarly chooses PI-value or -PI-value by value>=0. Rot90
subtracts HALF_PI and adds TAU only if below -PI; RotN90 adds HALF_PI and
subtracts TAU only if above PI. These perform a single wrap, not modulo.
PI/HALF_PI/TAU use source float bits40490fdb/3fc90fdb/40c90fdb.

Erase82cc01a0 hashes all six key words, finds the matching collision-chain
link, redirects it to the successor, frees the node and decrements count.
The map is unique-key, so its count is0/1. Host Vec index compaction preserves
survivor bucket-chain traversal exactly; freed capacity is reusable and
reinsertion prepends to its bucket, like the existing insertion. Native
freelist addresses are not exposed by this host map API.

Tests cover erase of head/middle/tail in a full colliding map, surviving
order and capacity reuse; filter tests cover signedzero, negative signed
OneMinus, NaN clamp and single-wrap limits. Root runs Cargo and supplies
independent source review and same-Actor generated dispatch/corpus validation.

## Fresh caller audit and remaining execution boundary

Update82955c78 clears all five condition-active words82...5ca0..5cb0,
stores dt5cb8, then calls UpdateLists5cc4, UpdateExitStates5cd4,
UpdateTransition5ce4, UpdateEnterStates5cf0, UpdateBehaviours5cf8. The
decompiler misassigns the float ABI and stack set arguments; raw call setup
and the already recorded generated-controller evidence take precedence.
GetNextState82951138 initial currentFFFF calls Evaluate(root0,PreCond) at
82951168. Evaluate82929c08 validates activation, recursively tries active
children in descriptor order and returns the first valid leaf; a state with
children all failing returnsFFFF, not that parent. The fresh initial graph
must therefore select its actual leaf before any root callback is entered.

UpdateEnterStates829527f8 iterates enter states in retained set order, stores
their timers0, then iterates each descriptor's behaviour interval. Enabled
behaviours call CreateInstance8295295c and publish the behaviour/instance pair
into the current IndexSet829529b8 before optional Begin. Creation/publication
also happens when mCallBehaviours=false. With calls enabled, all six source
context fields copy829529d4..e8; only instance overlays82952a00 before Begin
82952a10. A failure must preserve this already published occurrence.

UpdateBehaviours829290d0 first validates every packed active instance's
behaviour identity against the static declaration indexed by CurrentBehaviours.
Only then, if mCallBehaviours, it calls the selected state's generated update
function829291cc with the original retained context and packed instances.
Root StateUpdate_Action_0 at828ecbc0 writes context.instance before every
direct callback, in this order: JuiceHook0; transfers1..10. The first four
are GestureDown/Left/Right/Up; remaining six are NoInput, GrabWorld,
PhysGrindTranslation, PhysGrindStabilityNudge, PhysGrindUpDown, PhysBodySpin.
The new generic transfer callback covers all ten actual payloads.

The preceding JuiceHook is now implemented in this same bridge: CreateInstance82cbab70
allocates0x20 and calls DataCtor82cbaaf8, which initializes trickName from
Andale::nullStringName. **This is literal `nullString`, not an empty name**:
dynamic initializer82fabfd0 loads the string at823afc6c and assigns the
FastString30 global8312a84c; DataCtor82cbab50 extends that packed name to36.
The raw file's zero global bytes precede this initializer and cannot supply
the constructor default. Begin/End bind shared no-op825187c0. Update82cd4290
tailcalls RemoveIntent82cc2398: check same actual Motion interface, erase
instance.trickName from its map, then assign nullStringName to the instance.
The erase call is82cc2454, followed by temporary name conversion2464 and six
retained name stores248c..24a0. `JuiceHookInstance` retains its actual separate
token and name; the callback cannot receive a synthetic map. Generated instance
mounting, all selected child-state callbacks, condition evaluation and source
IndexSet lifecycle remain required before claiming generated Action Update.
Calling these transfer methods directly is only callback-level validation.

Initial evaluation's first source conditions are concrete and ordered:
root0 expressionFFFF, child1 OnBoardGeneral expressionFFFF, then its child4
ForceBrake expression37 = condition0 HasAGIntent(ForceBrake) AND NOT
condition1 PhysFilteredState(teleport). If false, child5 NoInputBrake
expression38 = condition2 TimeSinceLastInput(greater10) AND NOT the same
cached condition1. TimeSinceLastInput82c98b78 queries actual IPhysical,
GetPhysOut virtual+28, then PhysOut+38 Animation+80 float.
The existing retained Animation.time_since_last_input is that field; querying
the input listener timer or wall clock would be wrong. If neither matches,
child6 OnBoard expressionFFFF tries child16 Grinding expression3 (condition6
IsGrinding), child17 InAir expression6 (condition36 PhysFilteredState), skips
inactive Coffin18, then child19 OnGround expression56 = condition74
PhysFilteredState AND NOT condition75 HasAGIntent. These fields must come from
the actual retained PhysOut and Action map, and callbacks/cache stores only run
on branches reached by the short-circuit expression.

GetStateActivation<1>82929568 first asserts evaluations enabled and bounds,
returns true directly for expressionFFFF, otherwise supplies the actual state
parent semantic index to the direct condition callback. IndexToState829282c0
uses native complemented-index pointer tags; native Rust should retain a typed
state index instead of fake pointers. Direct condition wrappers assign this
parent before ActivationFunc, so shared static condition parent mutation must
not be silently omitted. Evaluate82929c08 uses Sustain only for a descendant
of current state; otherwise it uses PreCond. IsStateActive82928e60 requires
the actual table active bit and returns false forFFFF.

UpdateLists82952068 enables evaluation, calls GetNextState, disables evaluation,
requires a valid selected state, resolves any transition's target and computes
ancestor differences. FindCommonAncestors829292f0 builds selected/current
ancestor IndexSets, intersects with the pivot, and obtains uncommon suffixes
from the common count. Existing state times increment by f32 dt before
last=current/current=selected publication. On an initial successful selection
there is no transition index, so UpdateTransition82928ff8 executes no hooks;
the actual enter-state lifecycle still follows. This read-only audit does not
implement or force a selected leaf and does not claim the ground candidate is
the actual result before the source PhysOut conditions are evaluated.
