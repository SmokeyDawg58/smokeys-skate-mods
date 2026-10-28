# Generated Action controller load proof

Confidence: **high** for recovered constructor semantics; native descriptor bytes are **exact** for the inspected reference image. Callback execution, allocation ABI and full skating parity are not claimed.

Implementation: `generated_action_load.rs`, `generated_action_load_construct.rs`, `generated_action_load_data.rs`, `generated_action_load_tests.rs`. Root owns registration, wrapper attachment, original-data integration and Cargo gates.

## Native entry and ownership

`StateGraph::HierarchicalController::Load82cef180` deletes its old controller through virtual slot0 with delete argument1, stores NULL at wrapper+1c, calls `AbstractHierarchicalController::Create829513f0`, and stores the returned pointer. If nonnull it calls GetRootState(slot14), GetStateName(slot30), hashes through GetHashValue3282a74490 and stores wrapper+18. Null return preserves the previous root hash.

Raw82cef1bc..1d4 establishes call_behaviours=true, require_generated=false, allow_generated=true, debugFlushWhen=3, debugLevel=2 and the actual inline wrapper Context pointer. The public wrapper Load allowGenerated argument is ignored in this release function.

Create strips the last forward slash if any, otherwise the last backslash, then performs case-sensitive generated basename selection. Selected Action requires the actual gEnabled8305a778 byte and allocates2e4, alignment16, debug name"unknown". Null allocation returns NULL without substituting a dynamic graph. The constructor is82927fc0. Abstract constructor82cef238 contains only context-pointer store+4 and vtable store; its debug arguments are unused.

The host controller binds the wrapper/controller and Entity identities at load. Callback entry receives the actual wrapper Context anew and rejects a different issuance. This preserves the source pointer relationship instead of retaining an obsolete context snapshot. Per-behaviour copied contexts belong to later recovered callbacks.

The actual enable producer is SkaterManager::InitPlayer826000b0. It obtains the Simulation workspace StateGraph cache, reads kbTestDeterminism, and either enables each graph iff a separately sampled Actor player id is8, or rereads kbUseGeneratedStateGraph for each graph. It passes kagFile82fdacb0→"state/ActionGraph_OnBoard.xml" and kmgFile82fdacb4→"state/MotionGraph_OnBoard.xml" to EnableGenerated828ec858. It then resolves the same Actor Action interface (cross-checking cached pointer and requiring mSkaterAnim), loads Action, resolves/loads Motion, notifies observers, and updates the local player. gEnabled's original image byte is0; the runtime boot producer is necessary.

Relevant source cvars: init82f56a48/ref83048068 State/kbUseGeneratedStateGraph/"Use Generated State Graph"/true/NULL; init82f56a98/ref8304808c State/kbTestDeterminism/"Use Ghost To Test Determinism"/false/NULL. These need actual linked targets at the caller, not fixed booleans.

## Exact complete tables

All table sizes were recovered independently from IDA/PDB array type information. All integer reads use IDA's big-endian get_dword/get_word/get_byte. Names are decoded by walking bytes to NUL, because IDA string-item APIs omit some empty strings. No executable or game asset is embedded; Rust retains semantic declarations.

| Table | Base | Count | Stride | Raw SHA256 |
|---|---|---:|---:|---|
| Attributes |82174cd8|457|8|25f14d82787d5dee37759918fde03ecf85520b31dd0b8206e2af53f917f6a37e|
| Behaviours |8217a8b0|311|16|34e2b52cdb23c1831a71659f99c46302a367858d07caeb460f2998631323d1e8|
| Conditions |8217c460|146|8|3d41afcb921867cee762589feb239a170f993578d9abdc939ba28512964de37e|
| States |821766b0|224|32|d410c2994ad587e734155bf0cd438338d39e520239b318b1477f4f72a2295d70|
| Transitions |821782b0|129|16|887c4b39dff266342fe6236cb0031c294b2a0aab3c7e4320056eb428ee64ff27|
| Index array |82175b20|129|2|b05a736deeba2b721a33628dcad6658ab9e37ea8a5afbf500871f18920a940a7|

Layouts:

- Attribute: key pointer0, value pointer4.
- Behaviour: class name pointer0, parent state u16+4, attributes Section+6, parameters Section+a.
- Condition: class name pointer0, attributes Section+4.
- State: name pointer0, interrupt type u8+4, active u8+5, interrupt target u16+6, parent+8, expression+a, children Section+c, behaviours Section+10, transitions Section+14, update function address+18, trailing word+1c.
- Transition: name pointer0, priority u8+4, target state u16+6, expression u16+8, hooks Section+a.
- Section: count u16 then index u16. Children/behaviours are contiguous intervals; transition sections index the separate u16 IndexArray. Zero-count sections may carry indexFFFF and must not be dereferenced.
- All311 behaviour parameter counts are0. All129 transition hook counts are0. HookDesc8217c8f0[1] is eight zero bytes. The empty placeholder does not cause construction or callback execution.

The extraction artifact remained external at %TEMP%/skate2-generated-action-load-data.json for root's independent full-row comparison. Source callback binding evidence is %TEMP%/skate2-action-bindings.json.

## Real constructor families and native binding identity

Factories are separate Behaviour/State and Condition/ExpressionOperator domains. Static initializer registrations resolve ClassHelper functions; for example82f82d80 registers CreateConstMGIntent→82cb6138,82f82d40 registers CreateMGIntentFromAGIntent→82cbef68,82f82f00 registers JuiceHook→82c8f2a8,82f82b40 registers HasAGIntent→82cb6018,82f832f0 registers PhysicsSpeedAndSlopeCompare→82c8f668,82f83030 registers HasAnimAttribute→82cac2a0.

Each host object retains a distinct typed class, base constructor fields and its actual class payload. A Plain condition payload means the source class adds no fields beyond Condition; it is not a successful empty execution callback.

| Behaviour class | Count | ClassHelper | Size | Vtable | CreateInstance / Begin / Update / End |
|---|---:|---|---:|---|---|
|JuiceHook|1|0x82c8f2a8|0x1c|0x822fef50|0x82cbab70 / 0x825187c0 / 0x82cd4290 / 0x825187c0|
|CreateMGIntentFromAGIntent|177|0x82cbef68|0x68|0x82302ea8|0x82cba730 / 0x82cd6290 / 0x82cd62c8 / 0x82cd4288|
|PrintText2D|1|0x82c8fc08|0x428|0x82300930|0x82bb3da8 / 0x825187c0 / 0x825187c0 / 0x825187c0|
|CreateConstMGIntent|115|0x82cb6138|0x3c|0x82301ed8|0x827126f8 / 0x82cd6258 / 0x82cd4230 / 0x82cd4280|
|BodyFlippingSignal|1|0x82c8f248|0x1c|0x822fef14|0x82caec70 / 0x82c94840 / 0x82cd5cd8 / 0x825187c0|
|CreateMGTimeIntentFromAGIntent|3|0x82cb6198|0x4c|0x82301f20|0x82712148 / 0x825187c0 / 0x82cd5928 / 0x82cc2028|
|CreateTrickIntentFromGesture|7|0x82cbefc8|0x3c|0x82302e18|0x82cba2b8 / 0x82cd4da8 / 0x82cd51b0 / 0x82cc1d58|
|BoardAdjust|6|0x82cbf028|0x54|0x82302f1c|0x82cbaa40 / 0x82cbaa20 / 0x82cd54c0 / 0x82cc1f38|

| Condition class | Count | ClassHelper | Size | Vtable | ActivationFunc |
|---|---:|---|---:|---|---|
|HasAGIntent|60|0x82cb6018|0x40|0x82301df4|0x82cc1748|
|PhysFilteredState|10|0x82c8f368|0x20|0x822ff05c|0x82c99618|
|TimeSinceLastInput|1|0x82c8f068|0x28|0x822fee04|0x82c98b78|
|PhysicsSpeedCompare|6|0x82c8f6c8|0x2c|0x822ff0d4|0x82c99808|
|InParentStateForTime|3|0x82c8f788|0x28|0x822ff21c|0x82c8c950|
|IsGrinding|35|0x82cb61f8|0x34|0x82301f68|0x82caa310|
|IsMirrored|2|0x82c8f848|0x1c|0x822ff288|0x82c9a210|
|IsHandPlanting|1|0x82c8f9c8|0x24|0x822ff448|0x82c9abf0|
|IsFootPlanting|1|0x82c92428|0x1c|0x82301140|0x82ca7830|
|CurrentState|2|0x82cac3c0|0x2c|0x82301be8|0x82c94ae0|
|TimeToLand|1|0x82c90508|0x28|0x822ffa54|0x82c9cc40|
|DisablePushBrake|2|0x82c8f608|0x1c|0x822ff3d4|0x82c9a8f0|
|PhysicsSpeedAndSlopeCompare|1|0x82c8f668|0x28|0x822ff090|0x82c99710|
|IsRidingFakie|1|0x82c8f8a8|0x1c|0x822ff2bc|0x82c9a308|
|HasGestureIntent|7|0x82c8f008|0x20|0x822fedd0|0x82cc1870|
|PhysicsRequestsDismount|1|0x82c8f0c8|0x1c|0x822fee3c|0x82c98c70|
|IsGrabbingObject|1|0x82c8fb48|0x1c|0x822ff4e8|0x82c9aef0|
|IsDismounting|1|0x82c8fae8|0x28|0x822ff4b0|0x82c9ad78|
|HasAnimAttribute|4|0x82cac2a0|0x44|0x82301b44|0x82caa1d0|
|IsInLocomotion|2|0x82c8f188|0x1c|0x822feea4|0x82c98e78|
|IsLandingOnBoard|1|0x82c8f128|0x1c|0x822fee70|0x82c98d70|
|OBTimeToLand|1|0x82c8fba8|0x28|0x822ff51c|0x82c9aff0|
|PhysicsWantsRunout|1|0x82c8f488|0x1c|0x822ff1e8|0x82c99f98|
|IsDoneAirDismount|1|0x82c90808|0x28|0x822ffc64|0x82caefe0|

Common Base82cf6f88 sets refcount0, pooled name, NULL parent, enabledtrue and castParenttrue; path shares the name's pooled storage. All generated factory invocations pass NULL parent. Host Arc represents retained object/name ownership, not source allocator, global string-pool identity or ABI layout.

GenAttributeMap HasAttribute828f8c08 and GetString828f8cf8 special-case name and enabled before scanning the actual attributes linearly, exact-case first match. name is the descriptor class name; enabled is"true". Base therefore constructs the actual class-name path, not its eventual parent-state path. State parent membership remains separately in BehaviourDesc and is installed during execution.

Condition82cf7470 then initializes its mask through ParseMask82ceefa8: missing/precond1, sustain2, always3, postcond4; invalid values assert (checked host error). All selected original masks default to precondition. Comparison82cf7ae8 sets reference0/absolutefalse, accepts comparison+value with Equals/NotEqual/GreaterThan/LessThan/GreaterEqual/LessEqual, otherwise checks equal,notEqual,greater,less,greaterEqual,lessEqual in that order, or NOP if none. abs is sampled last.

Concrete payload constructors:
- CreateConstMGIntent82cae880: packed MGIntent required, value optional0, onUpdate defaulttrue.
- CreateMGTimeIntentFromAGIntent82cae9f8: both names required, writes MG then AG.
- CreateMGIntentFromAGIntent82cba338: both names required, default0 with separate presencefalse, scale1, onUpdatetrue, four NONE filters; optional default presence/value, scale, legacy filter then optional filter1 override, filter2, fakieFilter, mirrorFilter, onUpdate in source order.
- ParseFilter82cb9c98 compares packed FastString30 names: negate,abs,oneMinus,clamp,angleFlip,angleRot90,angleRotN90; unknown names map NONE.
- CreateTrickIntentFromGesture82cb9eb0 and HasGestureIntent82c8c410 use exact-case groups Square,Nose,Tail,90Nose,90Tail,N90Nose,N90Tail; behaviour additionally initializes false override flag and empty FastString36.
- BoardAdjust82cba7f0 reads negateOnMirror defaulttrue before required MGIntentMag/MGIntentAngle; angleFilter must be NONE/ANGLE_FLIP/ROT90/ROTN90.
- PrintText2D82c8cbb8 requires x,y,numFrames floats, converts each by fctiwz, then strncpy1024 of text. Full-length text need not terminate; shorter text is zero padded.
- PhysFilteredState82c8c5e8 accepts ground,air,grind,wipeout,teleport,invalid,offboard,offboardair. PhysicsSpeedCompare82c8c790 extends Comparison with alongSkateZ defaultfalse.
- IsHandPlanting82c8ca58 requires air/ground state; dirFS/BS selects direction, anything else leaves NONE.
- HasAnimAttribute82caa110 extends Comparison with required FastString30 attribute and sequenceid float default-1 converted toint. CurrentState82caa710 constructs an owned string from required state.
- HasAGIntent82cae360 extends Comparison with required FastString36 intent. IsGrinding82caecf0 separately retains grindName presence and packed defaultempty name.
- JuiceHook/BodyFlippingSignal and the nine plain conditions execute their real Base/Condition constructors. The other seven comparison-family condition classes execute their real Comparison constructor. Runtime callback work is not performed during those constructions.

GenAttributeMap float getters828f9280/828f9518 call CRT atof (double), then frsp. Host construction admits exactly the original finite decimal grammar, performs f64 parse then f32 conversion, and rejects unsupported broader CRT grammar instead of claiming equivalence. The generated literals include fractional/negative values and are retained text, not replaced by tuning constants. FastString parsing uses the existing packed codec and rejects unsupported malformed/overcapacity names at the checked host boundary. All original selected names fit the supported alphabet/capacity.

## Static and per-controller initialization ordering

TemplateHC constructor82927fc0 sets current/lastFFFF, deltaTime+0, empty fixed_map capacity8, empty IndexSet42, 42 null BehaviourInstance pairs, one null hook, five zero condition-active words, call_behaviours argument and allowEvaluationsfalse. Condition result values are uninitialized until active; host represents that unknown state with Option.

Shared behaviours[311] and condition[146] are retained across loaded controllers. Source behaviours sInit store is *inside* the loop: raw82928244 stores1 before count comparison82928248 and backwards branch8292824c. Source condition sInit store is *after* the complete loop: branch828ff964 precedes store828ff96c. The host preserves these distinct partial prefixes on checked host errors. It does not roll them back or silently reconstruct a skipped behaviour suffix. Source ClassHelper null allocation results are retained NULL and do not stop the loops. A NULL is diagnosed when actually dereferenced for a callback.

The constructor does not create any behaviour Instance. It does not enter a state, evaluate a condition, call Begin, run an update function, or write Motion intents. The first instance and activation APIs return a checked error with the reached native callback address until its body and collaborators have been recovered.

## Execution graph and next implementation boundary

Wrapper Update825ecb30 calls TemplateHC Update82955c78. Update clears condition active words, stores dt, then executes UpdateLists82952068, UpdateExitStates82929a08, UpdateTransition82928ff8, UpdateEnterStates829527f8, UpdateBehaviours829290d0.

UpdateLists allows evaluations, calls GetNextState82951138, disables evaluations, builds ancestor difference sets, advances current state times, then writes last/current. Initial currentFFFF evaluates root0 through Evaluate82929c08 with precondition mask. Activation uses GetStateActivation<1>82929568→delegate EvaluateExpression<1>828ff990. Sustain variant uses829294b8→82906048. Expression evaluation is ordered and short-circuited with per-condition caching; it cannot be replaced by choosing a named default path.

A candidate first ground path is Action0→OnBoardGeneral1→OnBoard6→OnGround19→IdleTurning51→IdleTurning94, but is admitted only after all preceding predicates fail or pass as appropriate. In particular ForceBrake state4 expression37 and NoInputBrake state5 expression38 precede OnBoard. Expression37 checks HasAGIntent condition0 ForceBrake, then NOT PhysFilteredState condition1 teleport. The next no-input brake expression uses TimeSinceLastInput condition2 greater10 and the teleport test. Actual intent, physical filtered state, elapsed input time, speed/slope, animation and disable-push/brake collaborators are required.

The candidate ground path has31 active behaviours: root0..10; OnBoard42..49; OnGround68..79. They include actual Action→Motion transfers, mutable timer instances, BodyFlippingSignal and JuiceHook cleanup. Source callbacks require same Action and Motion maps and actual physical/animation collaborators. No successful substitute callback or scripted trick selection is provided.

UpdateEnter installs state timers0, creates each enabled behaviour's real instance in descriptor order and then calls Begin if enabled. UpdateExit calls End in reverse active-behaviour order, removes/releases instances, and erases exit-state timers. UpdateBehaviours dispatches the selected generated state's real update function after identity checks. These lifecycle operations remain the next execution scope.

## Validation status and limitations

All seven generated-load tests passed in root workspace gate16282. That workspace's sole failure was an older cvar label-order expectation missing the newly registered Jobs label; its expectation was corrected before the next gate. Tests cover complete original declaration construction, family counts and base identity, full hierarchy/transition interval validation, representative constructor fields and exact float bits, shared object identity across distinct controllers, native NULL callback boundaries, distinct partial initialization semantics, missing factory behavior, and precise unrecovered callback addresses.

Root independently re-read all six tables from live IDA, compared every row with the external extraction, then separately parsed every Rust declaration literal and compared that complete result with the fresh IDA rows. Counts457/311/146/224/129/129 all matched; HookDesc's eight bytes were zero. Independent checkpoint5489b5, external fresh read %TEMP%/skate2-root-action-tables.json. Root also independently checked all32 ClassHelper allocation sizes and all32 virtual callback bindings, derived concrete constructors from helper BL targets, read every unique constructor/Base/Condition/Comparison, and reviewed the two different shared-init store positions. No mismatches were found within the shipped declaration domain. The binding field is named factory_helper to distinguish the allocator helper from the concrete C++ constructor.

Root's actor_graph_load.rs attachment was independently reviewed against raw82cef180 and approved: old owner released then NULL, real selected Action constructor, publication before root hash, old hash preserved on allocationNULL or checked unsupported selection, and no statics construction on the NULL allocation branch. This is a load owner, not completed graph execution.

The host factory scope is the32 actual selected Skater registrations. It is not a reconstructed global factory singleton allocator or a general arbitrary-class registration API. Likewise statics must be owned once by the real boot domain and shared across controllers; making one new statics owner per Actor would break the source relationship. Source debug registration output and allocators are outside the current native host claim.
