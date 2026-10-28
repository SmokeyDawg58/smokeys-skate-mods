# Generated Motion first evaluation and retained callback closure

Confidence: high for the source control flow below. Runtime integration is
unfinished; the condition audit is not a claim of a working Motion update.

The production caller is `Actor::UpdatePhase1`825f41a8. It calls the retained
Action controller at825f4374, clears Motion attributes at825f43a0, then calls
the retained Motion controller at825f43cc. Motion `TemplateHC::Update`82955e20
calls `GetNextState`829511b0; initial current stateffff evaluates root0 using
`Evaluate`8292bf58. The generated Pre evaluator8290aa50 uses a447-entry switch
at8290aa8c/table8290aa90. Hex-Rays does not decompile this large evaluator;
the case control flow must be checked from the raw branch targets.

Original state declarations are at82195fa0, stride32. Root0 tries OffBoard1
before OnBoard3, regardless of the apparent names. With every reached
predicate false, the inspected traversal reaches Default6 after expressions
0,283,433,423,39,43,434,125,133,134,200,418,271. This is an audit oracle only:
production must evaluate the actual retained conditions, preserve cache and
parent writes on failed branches, and never select Default directly.

Initial expressions from the raw switch are:

- 0 at8290b18c: condition6, `HasIntent`.
- 283 at829126a0: c76 && c9.
- 433 at8291c398: `(c76 && c77 && ((c48 && c78) || (c78 && !c48)))
  || (c26 && (c79 || c80 || c81 || c82))`. Repeated references remain in
  source order; the controller cache suppresses repeat activation bodies.
- 423 at8291b2cc: `c76 && c77 && ((c48 && !c78) || (!c48 && c78))`.
- 39 at8290bffc: c87, `PhysicsWantsRunout`.
- 43 at8290c17c: c94, `PhysFilteredState`.
- 434 at8291c770: c139 && c140 && c141 && (c89 || c13) && !c138.
- 125 at8290e08c and133 at8290e3a4: c212/c222, `IsHandPlanting`.
- 134 at8290e404,200 at8290fe80,271 at82911b20: c223/c202/c402.
- 418 at8291aae4: c394 && c395 && c396.

`PhysicsWantsRunout::ActivationFunc`82c99f98 resolves actual physical UCOM,
reads PhysOut+1c State+44, then samples the live `kDebugWipeoutConditions`
reference. A true debug cvar reaches the original debug singleton; its folded
no-op method does not justify inventing the required singleton. Constructor
defaultfalse must not replace the runtime cvar query.

`IsHandPlanting::ActivationFunc`82c9abf0 first reads actual PhysOut+8 Air
handplant flags at+134, then the air alternative reads a second PhysOut
State+10 and compares with600. The ground alternative tests80000000; the
frontside direction tests20000000, backside its inverse. Direction None
does not require either orientation bit after the state test succeeds.

Root `StateUpdate`828edd40 calls CharacterGesture0, UpdateTimeSinceTeleport1,
UpdateRidingFakie2, FakieHeadChannel3, in order. Default6 `StateUpdate`828ee1a8
calls root, PrintText2D39, PlayAnimation40. Original PlayAnimation40 has
anim=`KeepDefaultAnim`, transType=`play`, no parameters. The retained actual
animation owner, transition override and dispatch route must service Begin;
a recording backend does not close this path.

TimeSinceTeleport Update82ca5148 resolves physical then Motion graph, samples
filtered enum5, resets through825fe138 or adds controller dt through825fe148.
RidingFakie CreateInstance82ca2be8 allocates0x10 and initializes+8/+c to zero;
Update82ca2808 compares the OLD teleport timer before incrementing it and
uses source filtered/slide/velocity inputs. SetIsRidingFakie82cb7540 changes
only animation player flag20000000. FakieHead CreateInstance82cbc3f0 allocates
0x10, initializes float+8 and clears only bit80000000 at+c; the lower31 bits
must not be fabricated as known initialized storage. Update82caf338 requires
physical, animatable, channel animatable, skater anim and Motion interfaces.

Live cvar registrations now added in `input_cvar_runtime.rs`:

| Label | Initializer / constructor | Reference | Table | Default |
|---|---|---|---|---|
| kTimeToLandToWipeout |82f80d88 /82f80dc8|830a4c98|82fd48dc|1.4, bits3fb33333|
| kMinTimeInAirToWipeOut |82f80de8 /82f80e28|830a2884|82fd48e0|0.8, bits3f4ccccd|
| kDebugWipeoutConditions |82f82f90 /82f82fbc|830a402c|82fd4d10|false|

The two floats have section/title `Animation_Locomotion`/their exact labels;
GUI bounds0..5 are metadata and do not clamp startup INI overrides. Debug
uses section `Animation_Wipeouts`, title `Debug Wipeout Conditions`. Native
table order is LowBlend48d4, HighBlend48d8, landing48dc, minimum48e0,
debug4d10, gestures4ec8. GetPhysUpdateData consumes minimum first82c977a8,
then conditionally landing82c977c0, with no captured target caching.

The focused cvar test checks unlinked errors, exact default float bits,
unclamped INI overrides and fresh target resolution after manager relinking.
Root owns compilation and independent review; no new Cargo process was run
by this worker. Source packet with full symbols, bytes and decompilations
is external at `%TEMP%/skate2-motion-initial-source.json`.

## Actual construction-map lookup dependency

`PlayAnimation::Begin`82cc9818 unconditionally writes construction attribute
`ProSkater` before dispatch. The prior native animation tree implementation
rejected every nonempty map, so even Default could not reach its real lookup.
Fresh source `PlaybackData::GetAnimTree`82d7e468 forwards the map through each
database, and `DataBase::GetAnimTree`82d7def0 does not inspect it in Clip2 or
PhaseBlend7. Phase children retain the same database and map recursively.

Type8 at82d7e100..e21c performs the actual conditional selection: payload+0
is packed30 key, +20 names-relative-offset, +24 values-relative-offset, +28
count. Child names have24-byte stride and option values20-byte stride. Missing
map/key/value uses child0. First matching option index selects the SAME child
index, without adding1. It allocates no alias node, recursing in the same
database. The source default path does not read option values when its key is
absent. Sequence9 and Selector10 also consume specific construction keys but
their previously explicit unsupported-record boundaries remain intact.

`actor_animation_tree_database.rs` now forwards actual maps through Clip/Phase
and implements type8 selection. Focused serialized-payload test distinguishes
default0, first duplicate winner, no +1 and absent-key lazy value reads. The
existing original105-gesture-tree corpus now passes a real nonempty ProSkater
map through all phase and clip lookups. Independent root review/tests pending.

## Initial retained lifecycle now connected

`try_update_motion_initial` uses the same retained generated controller and
context. It evaluates original data, publishes last/current in UpdateLists
order, then enters sorted numeric ancestors (capacity11). Enter82952a98 stores
time0 before reading each state's behaviours; enabled occurrences allocate
even when callback calls are disabled, then enter the sorted packed active
set (capacity40). Each Begin gets its own six-slot context copy. Final generated
StateUpdate mutates the original context projection; the Actor session writes
its reached instance back on failure as well as success.

Default6 Enter now invokes actual PlayAnimation lifecycle against the actual
animation, physical and Motion owners. The ProSkater construction write and
transition-override read/clear precede actual body dispatch. Root callbacks
then execute CharacterGesture, UpdateTimeSinceTeleport, UpdateRidingFakie,
FakieHeadChannel; PrintText2D39 is the audited no-op, PlayAnimation40 Update
uses its real first-update latch. Original callback layout is validated against
the active indices `[0,1,2,3,39,40]`, not assumed for other selected states.

RidingFakie implements normal allocated-instance branches using actual
filtered state, conditional powerslide/grind fields and direct Motion+a8
Zspeed. Native read82ca2b44 does not sample the write latch, so the narrow const
getter verifies construction and reads the retained projected scalar directly.
The old teleport timer controls eligibility BEFORE its increment. Aged-false
sets ridingfalse before clearing slow timer; aged-true but ineligible still
reads velocity and clears slow timer without changing the riding flag.

Raw low-speed branch82ca2b8c is40980038 (BO4 BI24, NOT CR6.LT), so unordered
resets the slow timer. Slow-duration branch82ca2ba4 is40990024 (BO4 BI25,
NOT CR6.GT), so unordered skips the true setter, matching strict `>` in Rust.
FakieHead reads actual manual flag10000000 first, powerslide04000000 only when
manual is false, then physical ground state and ridingflag20000000. Its initial
inactive path completes all required reads. Active new-channel creation remains
an explicit boundary at82caf774; existing-channel exit and torso updates retain
the source writes, including two FSEL clamps and normalizedfalse/sequence-1.

Other remaining boundaries: other selected state lifecycle/expressions,
optional PlayAnimation filtered intent/tree attribute
and switch-name queries, and null root-instance Update bodies. These paths do
not silently return a default state, success or invented owner value.

Initial runtime checkpoint reviewed by root: original6 all17 tests passed,
including one actual initial Update naturally selecting Default6 and entering
its six behaviours; workspace6 passed3719 unit tests and6 doc tests; clippy10
passed. These results precede the continuous-update additions below.

## Subsequent retained Motion update

The controller now runs `GetNextState` for retained current states as well.
Source-specialized graph semantics use actual1724 transition declarations and
1737 indirect indices, with capacity11 ancestry (actual deepest chain1342,
1298,1232,1145,989,686,250,78,21,3,0). `SearchTransitions`8292c370 invokes
priority overload8292c1a8 for3,2,1,0, restarting current-to-root each time.
Priority match precedes activation, ValidTarget8292ba00, and target subtree
evaluation8292bf58; failed candidates retain their condition cache effects.

`EvaluateState`8292c0a0 implements interrupt types0/1/2. The selected current
ancestry uses Sustain; siblings use Pre. Unsupported nonempty Sustain
expressions stop at that exact masked evaluation boundary; Default and root
both have expressionffff and invoke no condition body for Sustain.

Original Default6 has zero transitions. Root0's indirect transition0 has
priority3, targetTeleport5, expression273. Raw case82911be0..1ca4 is condition0
PhysFilteredState AND NOT condition1 CurrentState, with lazy current-state
query. Actual Invalid physical state makes c0 false. The graph then reevaluates
normal root children and naturally retains Default6. No Default is forced.

`UpdateLists`829521e8 uses sorted ancestors8292b528 and exclusive-pivot
intersection8292b008/8292b640, including self/descendant target pivot behavior.
Unchanged Default gives empty Exit/Enter, retains instances/body playback and
increments existing time entries using dt+old before callbacks. Exit8292bd58
walks packed behaviours backwards, runs End on its own context copy before
removing each slot, then erases timers in exit-set order. UpdateTransition
8292b3d0 has no callBehaviours guard; currently nonempty hook sections stop
after Exit at the hook boundary. Original root transition0 has zero hooks.

New focused tests verify the real Default second selection/root transition
order, no ancestor reentry, transition-query failure before fallback, actual
depth11 support and lazy273 operands. Root owns the subsequent-update original
corpus checks and final gate results.
