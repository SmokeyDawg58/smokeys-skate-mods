# Neutral and Antic continuation proposal

## September24: directional anticipation dispatch correction

A bottom-right hold reached PendingStateUpdate(2190400456), i.e.
0x828edbc8. Existing implementation admitted only83/88 despite the recovered
descriptors already naming the same callback for164..179. No new intent or
animation implementation was needed: extend the existing checked dispatcher.

Fresh IDA inspection of kStates821766b0, 224 descriptors of32bytes, confirms
mParent+8, mBehaviours+16 (count/index), and mUpdateFunc+24. Exactly83,88 and
164..179 point to828edbc8. The sixteen directional regular/switch leaves have
parents84..87 and89..92; their two behaviors start249,251,...279. Their inherited
prefix is the same0..10,42..49,68..79,119 as83/88.

Raw828edbe4 calls OnGround19; dbf8 executes packed31 Transfer; dc0c packed32
Const; dc20 packed33 Transfer. Each installs the corresponding retained instance
at context+8 first. Runtime now selects the existing handler by this verified
function address and still validates state, address and the entire active list.
No generic success fallback and no new parallel implementation.

Regression covers all18 descriptors, ancestry and leaf callback classes. An
original-data replay holds/releases all four stick corners. Both dispatch tests
PASS;448frame original-data replay PASS46.04s, selecting164,166,172,174. Logs
artifacts/diagonal-antic-replay-20260924.log. Workspace5015core plus other suites
and fmt PASS. Optimized release exited0 in4m19s, executable updated23:36:40.
Strict Clippy failed on existing core backlog1037 diagnostics.
No interactive launch; controller validation remains next.
Confidence high for source dispatch equivalence; no console numeric-trace claim.

--- Historical proposal below ---

2026-09-12: NEW unregistered files only. No Cargo or original-graph execution
claim. Parent owns shared integration and independent source review.

## Condition98

Fresh raw PhysicsSpeedAndSlopeCompare82c99710: GetPhysOut97e4, outer+4
pointer97f0, lfs Motion+c0 at97f4, existing EvalCompare97f8. PDB names
PhysOut_SkateboardMotion.mHeadingAdjustFactor at+c0. The existing
MotionRemainder.heading_adjust_factor is the same destination. This is
not a request to recompute slope or substitute deck speed.

Proposed files/registrations:

* physical_player_phys_out_heading_condition.rs under PhysOut.construct:
  direct retained read, no write-latch check. Two tests cover current NaN
  bit preservation despite locked output, and absent construction.
* actor_action_heading_condition.rs under Actor.action_motion_intent_bridge:
  actual context and cached physical identity checks, then fresh output read.
* generated_action_runtime_heading_condition.rs under generated Action runtime:
  invoke the existing conditions::compare only after the actual field read.

Proposed condition dispatch arm is
`(ConditionClass::PhysicsSpeedAndSlopeCompare, ConditionPayload::Comparison(c))`
calling heading_condition::evaluate_heading_condition(actor, context, *c).
This keeps the existing parent-tag write before activation and cached result
publication only after success. Original condition98 has less0.1. Neutral
absent KickTurn then stops at condition99 before the other speed/fakie tests.

## Held anticipation generated dispatch

Fresh raw 828edbc8..dc38 is shared by original state83 OllieAnticipation and
state88 NollieAnticipation. It calls OnGround828ed1f0 first, then reads packed
slot31 instance+fc/behaviour+f8 and invokes Transfer.Update atdbf8; then
slot32 instance+104/behaviour+100 and Const.Update atdc0c; then slot33
instance+10c/behaviour+108 and Transfer.Update atdc20. Context+8 is stored
before every invocation. The new generated_action_runtime_antic_dispatch.rs
checks the actual state/address and exact active occurrence list, then reuses
the existing dispatch_callback machinery in this order.

Common indices0..10,42..49,68..79 precede Antic119 (AnticMag->Compression).
Ollie then owns157 Const OllieAnticipation and158 AnticMag->AnticMag;
Nollie owns159 Const NollieAnticipation and160 AnticMag->AnticMag.
Other anticipation/shuvit leaves are not admitted by this bounded dispatcher.
The existing grounded dispatch test and new exact-slot test remain separate
from actual callback implementation and original corpus coverage.

## First remaining actual callback

CreateConstMGIntent is loaded but not executable in current runtime.
CreateInstance827126f8 allocates0xc/alignment16; successful native shared
data constructor sets byte8=false. Allocation failure retains null.
Begin82cd6258 calls CreateIntent82cd4cd8 BEFORE storing byte8=true.
CreateIntent resolves actual IMotionGraph/UCOM identity, requires nonnull,
gets its real intent map and inserts payload mValue under mMgIntent.
The loaded Ollie/Nollie declarations default value0 and onUpdate=true;
zero is a present intent, not an absent intent.

Raw Update82cd4230 reads behaviour+38 onUpdate first, captures instance,
and only when onUpdate=false reads instance+8. If both flags are false,
it calls RemoveIntent82cc1c90; only after that returns does4264 clear
instance+8. Update NEVER reinserts. End82cd4280 tailcalls RemoveIntent.
RemoveIntent still verifies cached/UCOM identity, but a genuinely null
Motion interface is permitted after that check; only nonnull gets the map
and erases the name. Null-instance failures must occur at source byte-read
or byte-store positions, preserving any prior insert/remove side effect.
This callback is the next necessary actual held-Antic closure, not another
generic constant-intent summary. Existing transfer callbacks remain reused.

Follow-up authorized draft now implements this lifecycle in
actor_action_const_intent.rs and actor_action_const_intent_tests.rs, proposed
intent-bridge child. Four focused tests distinguish null-after-insert failure,
failed insertion before byte publication, pulse first/second Update, and
persistent Update with no map access. Actual Actor adapter reuses the retained
Motion intent map. The removal path admits only a truly absent graph plus
absent published interface; partial/foreign owners remain checked errors.
Parent must add the typed ConstIntent instance variant and exact allocation
request0xc/16, preserving NULL in the existing occurrence slot. Unlike the
older generic missing-instance branch, dispatch must pass None into these
callbacks so Begin can insert before its late byte-store failure, and End
can complete without ever dereferencing an instance.

## Repeated held and released input

Correction after the original grounded corpus exposed transition search:
the state transition section is an index into data::INDICES, not directly
into TRANSITIONS. Antic46 section(index52,count1) resolves transition88,
priority0, target Trick20, expressionFFFF, no hooks. The prior version of
this packet incorrectly used declaration52 and claimed LateFlips21; that
claim is retracted. The existing runtime already uses the correct indirection.
The existing template transition search therefore
must test target Pre expression20 before Antic sustain, even on a still-held
stick. Fresh Pre828ff990 case14 hex is condition107 HasAGIntent("Trick").
Add that exact expression to the proposed grounded evaluator; do not skip
the transition because current diagnostic samples have no Trick intent.

The unregistered grounded evaluator now includes Pre20=c107. Fresh
Sustain82906048 has no case20 and returns true through its native default;
the draft preserves Sustain20=true without inventing a repeated intent read.

When the real Trick intent is absent, that target fails, and the actual
Sustain58 tests AnticMag presence only (not the entry >0.9 threshold).
When AnticMag is genuinely removed, fallback can return grounded neutral
after the original exit callbacks. This is distinct from successful flick
trick execution. If real Trick is present, transition88 can search Trick20's
original gesture children. Success must not be asserted from an input label.
Original Trick20's first child uses expression91/HasGestureIntent,
and CreateTrickIntentFromGesture remains a concrete pending callback.

## Full transition preflight on the selected grounded route

Default3 has only ancestor0 and no transitions. After that first update has
selected neutral94, subsequent neutral/Ollie83/Nollie88 updates have the
following ordered transition search (source8292a020/82929e58, actual INDICES):

1. Priority2 ancestor OnBoardGeneral1 section0 -> transition0 -> target31
   AccidentalDismount, target Pre35. This is c144 PhysicsWantsRunout.
2. Priority0 leaf ancestor IdleTurning51 section54 -> transition90, or
   Antic46 section52 -> transition88: target20 Trick, target Pre20=c107.
3. OnGround19 section28/29 -> transitions85/86 -> targets8/7,
   target Pre31 and Pre29 respectively (existing lazy expressions).
4. OnBoard6 section3..5 -> transitions3/4/5 -> ForceBrake4 Pre37,
   NoInputBrake5 Pre38, HippyJump28 Pre74.

All these declarations have expressionFFFF; target validation makes the
predicates observable. Empty intent maps do not justify skipping the targets.
Fresh Pre35=c144; Sustain35 has no case and native defaulttrue.
Fresh Pre74=c142 && c134; Sustain74=c134 only. Condition134 is the loaded
PhysFilteredState offboard. Thus even current Ground must first execute142
before its later offboard test can reject the target. These35/74 were the
remaining missing expression cases on neutral->Ollie/Nollie held->release;
Pre20,31,29,37,38 already exist. Offboard33/34 become relevant only if the
actual earlier physical predicates admit their subtree.

c144 PhysicsWantsRunout82c99f98 reuses the already implemented actual Motion
physical query: source reads State+44 mPhysicsRequestsRunout at82c9a078,
then existing kDebugWipeoutConditions and conditional overlay/display, returns
the captured byte. Parent owns reuse; no duplicate leaf proposed here.

c142 IsLandingOnBoard82c98d70: raw82c98e48 GetPhysOut,8e4c pointer+1c,
8e50 loadState+10,8e54 subtract505/cntlzw equality. PDB mPhysicsState,
existing try_state_output().physics_state. It is NOT animation's
can_land_on_skateboard flag. No debug/cvar access occurs in this predicate.

Root acceptance: workspace42=3,972units+6docs;original39=17/17;Clippy67/fmt33/diff33PASS. Controlled original Actor boundary verifies neutral94 ->Ollie83/Nollie88 ->heldsameinstance ->released94, with zero-valued anticipation present until End. Root fresh raw verifies35/74,State505getter, sharedrunoutread ordering; independent reviewer agrees. This is graph-boundary testing, not actual full-frame or gestureproducer completion. DebugWipeoutConditions=true still stops at existing explicitMissingOwner.
