# Actor owner mounting for the gameplay listener

2026-09-08. Read-only matched original IDA/PDB audit and current repository comparison. No Cargo or Rust edits. Builds on local-skater-spawn-path.md, physical-player-owner-next-source.md, animation-physics-handoff.md, and the new game-input-actor-listener-runtime-plan.md rather than replacing their recovered leaves.

## Ownership conclusion

The real Actor is an outer Entity owner constructed BEFORE its listener, graphs, animation, and physical player. PhysicalPlayerHiLOD retains that SAME Entity; it is not the Entity issuer. The current PhysicalPlayerAssemblyOwner is a meaningful, retained paired-physics projection with actual assets and simulation-manager authority, but its identity currently stands in only for equality within the selected pair. It cannot be renamed Actor, and an ActorGraphAuthority issued after a pair has already been admitted does not reconstruct the native constructor order.

Smallest honest mounting direction: create a retained Actor construction owner with actual player ID, simulation flags, spawn input and world/services binding, then execute its selected Entity/interface prefix and create ONE existing ActorGraphAuthority issuance there. The same issuance backs the listener and both graph facades, and is retained by the later physical-pair construction. This is justified ownership when the record owns real source fields and completed constructor stages; it is not a free Arc<()> labelled Actor. Keep incomplete construction unpublished. Actual source constructor completion, manager-list publication, graph loading, and observer/local-player readiness must remain separate states.

There is no ready native Actor owner in Rust to borrow today. Existing ActorGraphAuthority is the correct reusable issuance mechanism, not proof that all native construction services exist. Existing listener may remain the selected component on this new meaningful outer owner while the constructor is stopped at a specifically missing stage. Do not call that outer owner spawned/initialized merely because its listener can receive messages.

## Source caller and publication

Player registration/presentation recipe completion is separate from spawn; player.rs implements that protocol and CharacterScene owns presentation assets. Neither is an Actor factory. Existing local-skater-spawn-path.md recovers command3: SkaterManager::OnCommand82600920 decodes PlayerSpawnData, AdjustSpawnPosition825f1a20, DestroyPlayer, SpawnPlayer, then UpdateLocalPlayer. Do not bypass this and claim the real command scheduler; a selected constructor entry can explicitly accept the already adjusted source transform.

SpawnPlayer82600630 allocates Actor size260 at0664. Raw066a0..066bc supplies SAME manager Simulation PhysicsWorld, requested position and index, flags1 (FLAG_Simulation), with optional formal service arguments zero where explicitly set. The actual constructor call is066bc; decompiler comment066c0 is the result move. Constructor result is appended to SkaterManager list at066ec/066f4 or overflow06704, then EntityManager list at06750/58 or overflow06764, then skater sort06780, InitPlayer0678c. A checked failure in EntityManager publication must not erase the earlier skater-list publication. Allocation-null branches are native assert/crash-prone paths, not evidence for successful empty Actors.

SkaterManager constructor825fb300 retains actual Simulation atb3c0, constructs empty skater list and four update groups, sets ghost/local null and local index-1 b454..45c, then constructs GestureTrickMapping, SkaterAnimShared and RumbleManager before actual CommandManager registrationb4b4. There is no existing Rust SkaterManager owner implementing those stages. A selected actor slot may be useful before implementing the complete manager, but it must not fake native list membership or local-player readiness.

## Actor constructor order and real source fields

Actor825fe850..825ff6c0:

1. UCOM interface list constructor at e888 (expected count0), Entity cached interfaces IMotionGraph/IAnimatable/IChannelAnimatable null e898/e8a8/e8b0. Construct ICollidable e8bc, IActor e8c8, targetable e8f0, Chasee e8fc, protectee e908 around SAME UCOM object (Actor+4); cTriggerTester and IActorQuery vtables interleave. Actor flags+1dc storee914, player ID+1d8 storee91c, graph/animation/AI pointers and counters/defaults follow. Player index unsigned>8 asserts e9d8..f4. Source matrix validity and position-length assertion gates e9f8..ec20 occur BEFORE listener allocation; preserve actual VMX numeric policy if implementing them. This audit does not reduce their IDA VMX register-alias output to guessed host is_finite checks.
2. Listener allocation1c0 ec38, ctor825f9f28 call ec4c with Actor pointer and actual index, store+1f0 ec58. Its three source registrations happen before any ActionGraph is constructed. Existing selected listener/route semantics are reusable here.
3. ActionGraph allocationa68 ec6c, ctor82cea340 ec84 with SAME Entity and debug-level/flush0, store+1f4 ec90. SkaterMotionGraph allocation178c eca4, ctor825fdda8 ecb4 with SAME Entity; retain ISkaterMotionGraph subobject+ a78 in Actor+1f8 ecc8. ActionGraph constructor registers IActionGraph and creates a controller/context around that Entity; SkaterMotionGraph constructs MotionGraph base and ISkaterMotionGraph, own intent/attribute output and reset. Existing motion_graph_entity_authority.rs and state_graph_context.rs explicitly recover sharing semantics and are reusable; current generated module only has descriptors/selection, not ticks.
4. ExternalPlayerReset allocation follows. Actual local-player/network/ghost values determine the animation construction mode. PlayersBackEnd selects character identity/male or female/CAC versus actual characters_marquee collection. SkaterAnim allocation61d0 edc0; ctor call eee4 uses SAME Entity, supplied PhysicsWorld, TRANSIENT, selected local flag, actual selected name, and actual FaceAnimDatabaseManager, AnimPlaybackManager and AnimCommandManager. Later CAC settings and body/left-hand/right-hand playback initialization remain source work. They cannot become successful no-op services simply because a physics-only corpus already has the required files.
5. AnimOutPhysIn allocation2ab0 and ctor with actual SkaterAnim bone count; retained at Actor+204 f10c. Actual animation hierarchy call f120; form exact name phys_Tpose then actual playback GetPhysicsParam f168; retain original PhysicsParam in Init_IPhysical. Fresh Simulation/world reads gather grab-spline manager, network query, physical-world services, LivingWorld and SAME Actor query (Actor+34), supplied PhysicsWorld, actual bouncy/human decisions. Bindpose call f2b4 and hat lookup f2c4 precede HiLOD allocation340 f2d8, ctor82e03c50 f2f4 with SAME Entity, inputs, spawn and hat; result store Actor+200 f300.
6. Breadcrumb owner and actual LW mesh/store/query dependencies follow, then external impulse, AIPhysicsInput, SubmittableSkater and three animation jobs. Job input retains Actor pointer f524 and source initial dt1/60 f52c (not a scheduler cadence substitute). Finally UCOM finds IMotionGraph f614/storef618, IAnimatable f644/storef648, IChannelAnimatable f674/storef678, each followed by nonnull assertion; temporary bindpose storage and attribute instance are released before constructor returns.

IActor ctor825ef5c8 is a useful minimal native interface rule: retain owner f5f0, require nonnull, Find duplicate f620/assert, Add f650. UCOM list ctor8298f170 begins null/end/capacity then reserve(expected count). This is an interface registry with real ownership, not a map of invented native addresses. Chasee ctor825f6990 registers IChasee, makes empty chaser vector, constructs a null attribute instance with default layout12, then calls ChaseeReset82de2310. Recover its reset before claiming the entire Entity prefix complete; do not demand a live pursuit service for an empty constructor unless source needs one.

## Existing physics and animation reuse without duplicate owners

physical_player_assembly.rs owns actual Vault, hierarchy, PhysicsParam, pose hierarchy/matrices, spawn/hat/bouncy/human and SAME simulation manager, plus retained Reckoning/Processed/PhysOut/surface/controller components. Its constructor/teardown phases already preserve real body admission prefixes. Use that factory INSIDE Actor's HiLOD stage, with an explicit parent Entity binding supplied at construction; do not build a second pair or replace an admitted pair's identity. Board/skeleton external Entity comparisons should eventually resolve this parent issuance while their physical construction occurrence remains the same inner owner.

HiLOD82e03c50 registers IPhysical into the supplied Entity UCOM and retains owner+8. The selected inner chronology is already documented: Processed then PhysOut, conditioners, SurfacePhysics, GrindAirAdjust, Reckoning, selectors/handplant/pumping/AnimOut, board, skeleton. Board and skeleton receive SAME parent Entity and same processed/Reckoning. Existing corpus construction has explicitly scoped omitted stages; mounting must not silently upgrade its phase Complete into full native HiLOD/Actor completion.

character.rs owns presentation graph/skins, not SkaterAnim. Original animation playback/physics assets are already parsed and owned; physical_player_current_animation.rs and skeleton_current_animation.rs implement selected producers with explicit supplied animation data. Those are inputs to the future animation owner, not evidence that native SkaterAnim constructor, its interface registration, or animation jobs have happened. animation-physics-handoff.md already recovers Actor::SetUpPhysics825ff6c0: reset owned handoff, gather animation output, gather motion graph output, merge attributes, apply external inputs/settings, submit to SAME physical player. Reuse these existing types and consumers rather than allocating a parallel RawPhysIn path.

## Init is after constructor AND list publication

InitPlayer826000b0 gets actual StateCache, samples determinism/generated cvars, selects Action and Motion generated settings. It verifies cached ActionGraph against SAME Entity UCOM lookup, loads Action controller at02a4; verifies cached IMotionGraph against UCOM, loads Motion at0350; notifies actual skater observers036c; UpdateLocalPlayer0374. Graph-load failure retains already completed construction/list publication and any earlier Action load. A successful metadata parse is not either load. The source-selected generated path must not fall back to dynamic execution merely because dynamic helpers exist.

Thus useful readiness states are EntityPrefix, ListenerRegistered, GraphFacadesConstructed, AnimationConstructed, PhysicalConstructed, ConstructorComplete, SkaterListPublished, EntityListPublished, ActionLoaded, MotionLoaded, ObserversNotified, LocalAssociationUpdated. These are proposed host bookkeeping names, not extra native fields. A selected implementation may stop at an earlier stage, retaining actual components and naming the exact missing collaborator; it must not issue later ready tokens.

## Teardown and lifetime boundary

DestroyPlayer825fff80 finds actual index; missing index no-op. For found Actor: remove SAME Entity from EntityManager first (00008..2c), erase skater list00030, sort0004c, clear ghost if same00058/5c, UpdateLocalPlayer00064, notify destruction observers00080, then virtual Actor destructor000a0. It does not destroy first and remove stale pointers afterward.

Actor destructor825f9668 first Chasee/Protectee cull notifications96f0/f8; destroy/free jobs1/2/3 and clear each; destroy Submittable, AI/external/reset allocations; destroy listener97f4 then clear+1f0 at9804; destroy ActionGraph9820, MotionGraph9844; breadcrumbs985c; owned AnimOutPhysIn buffers/map/object987c..9894; SkaterAnim98b8; Physical98dc, then AI controller98e8; remove remaining UCOM interfaces in source order, require list empty, destroy list99a8. Listener destructor825f72d8 unregisters HELD first7300/04, PATTERN second7314/18, INPUT third7328/2c. Current selected route's grouped teardown is a scoped semantic removal; native owner teardown should preserve this staged order if failures/callback effects become observable.

Borrowed listener's current lifetime cannot become a self-referential movable Actor struct. Prefer an outer stable Actor storage/identity object with a separate construction state, or extend the existing authority with a shallow SAME-issuance handle (never call new again to solve a borrow). Arc-cloning a real retained authority is valid ownership plumbing; minting an equal-value authority for each graph/listener/physical child is wrong. Teardown must remove subscriptions before dropping listener/Actor state; the route's explicit failed-delivery recovery already preserves queued message and completed-handler prefix.

## Smallest concrete next implementation recommendation

Implement Actor Entity/interface construction prefix and owner storage first, not a fake completed Actor facade. It should retain real ID/flags/world/spawn and existing logical authority once, enforce one attempt, preserve completed interface/field prefixes, and allow the existing listener to be constructed at the actual source point. Add source-backed ChaseeReset and position-validation numeric work needed to reach that point. This mounts recognition under an actual retained constructor component without claiming manager publication or full spawn.

Then add the Action/Motion facade constructors and exact interface publication using existing authority/context/controller-selection pieces; constructor ownership can be recovered independently of generated ticking. Next build/retain the actual SkaterAnim selected data/interface owner from original playback sources, with mandatory managers supplied by real owners and no successful placeholders. Only after that stage invoke the existing paired physics construction, carrying the same parent Entity. Keep full Actor constructor publication blocked until mandatory later jobs/interfaces complete. This sequence gives each new component a native owner immediately and avoids perpetual isolated leaf wrappers, while not pretending the still missing constructor stages have run.

Do not retrofit the currently admitted diagnostic corpus pair with an invented earlier Actor. Start this new staged construction path before pair admission; existing corpus remains a useful independent original-data check. No direct recognizer→jump path is added: the actual listener Fill/generated graph/animation handoff still produces the event consumed by existing jump logic.

Fresh SHA256, end exclusive:

## Follow-up: constructor prefix blockers resolved

Original VX128 words were independently decoded using the [published instruction bit-field definitions](https://raw.githubusercontent.com/xenia-project/xenia/master/src/xenia/cpu/ppc/ppc_instr.h), solely as encoding documentation. VA is low5 bits16..20 plus bit5 and instructionbit10 shifted to registerbit6. The original game words, rather than IDA's incorrect high VA aliases, establish the behavior:

- ea00 word1bbef065 compares v62 with ITSELF; ea18 1b7ce065 compares v60 with itself; ea30 1b3ad065 compares v58 with itself. The following Y/Z/translation vector checks repeat the same pattern. All twelve XYZ scalars are checked for self-equality; W is not tested. NaN fails, signed zeros and infinities pass this first gate. Do not strengthen it to all-lanes-finite or orthonormality validation.
- Each source vector loads and tests its components, skipping remaining components of that vector on failure; then processes subsequent vectors. Combined four vector flags are ANDed eb54..5c before assertioneb68..7c. Numeric length is not called when checked-host handling stops on this failure.
- eb88 word148631b5 is dot3(v38,v38), actual translation. ebc0/ebc4 copy squared lengthv36 (not unknownv68). ebfc word1801f964 compares actual thresholdv33 against lengthv31. Threshold source bits47c35000 are100000. Strict ordered threshold>length is required; equality and unordered fail.
- Dot3, rsqrt estimateebbc, two original correction iterations, squared*inverseebf4, and exact-zero selectebf8 match existing reckoning_numeric::try_length/length_from_squared_and_estimate. Reuse that graph. Even zero executes estimate/refinement before select. Unknown input W can use an explicitly unobserved scratch lane ONLY at dot3; do not publish a fabricated source W. Infinite basis XYZ is accepted by these checks when translation passes. Infinite translation usually reaches the numeric exceptional graph and fails its final ordered predicate; preserve the numeric port rather than prechecking it.

ChaseeReset82de2310 raw write/read order is complete: clear chaser end2330, flank enum-1 2334; reacquire actual ChaseMan2338, min getter2348 then store+164234c; reacquire2350, max getter2360 then store+1682364; reacquire2368, velocity scalar getter2378; reread already retained min2380 and store blockRadius+1702388 BEFORE scalar+16c2390; zero timers+174/+178239c/23a0; read Sk8::globalKey23a4, construct OWN livingworld_entities_chase instance23a8, assign+18023b4, destroy temporary23bc, final end-reason-1+17c23c0. PDB chaser list is fixed_vector<tChaserInfo,5,false>; no uninitialized entries become observations.

GetChaseMan82e4edf8 requires actual gpChaseMan, not a fabricated null-success port. Manager embeds its ChaseMan at+11b80 (ctorcall82721f00); InitLWMan82703100 publishes that SAME subobject through InitChaseMan82e4f2e8/storef2ec. ChaseMan ctor82715770 has three empty capacity256 collections, null own attribute collection with layout fallback12, zero numchasers, and source remaining flags/defaults. Manager Reset invokes ChaseManReset at8271d430; that method binds its OWN global chase instance and requires collection nonnull before subsequent flags. ChaseIsOn826f3b48 is simply numchasers!=0, hence source constructor-empty Reset does not require invented pursuit callbacks. ChaseMan Init827185f8 separately registers allow/disallow callbacks; it is not the getter implementation and must not be claimed by a selected formation-settings owner.

Class is1477b0d5343d29fd (generated constructor826fc7c8, layout fallback12). Both Chasee and manager use collection Attrib::StringToKey("global"), but retain SEPARATE attribute instances. Sk8::globalKey initialization82f9d03c..4c and manager translation-unit globalKey82f64d84..94 independently prove the literal. Reuse formats::attrib::string_hash64("global"), not RenderWare hash or physics default collection. Three manager getters read its same retained instance at+1834, each first checking null collection (null returns positive zero), then fresh key lookup index0, missing-pointer fallback4, scalar load:

| Getter | Full key | Lookup/read |
|---|---|---|
| Min radius826f3f08 |09c7ba0759e13dde|3f44/3f58|
| Max radius826f3ea0 |4cf39d0d047c8cc0|3edc/3ef0|
| Velocity scalar826f3f70 |fdcd97d058875e31|3fac/3fc0|

No runtime tuning constant is needed. A selected constructor/Reset-backed ChaseMan formation component can retain actual Vault+instance and constructor-empty counts, while explicitly not claiming LivingWorld registration/event initialization. Actor must be bound to that one retained component; it must not construct another provider for each getter or substitute its own chase instance. Actual full-manager mounting still supplies the published subobject boundary.

Entity registry rules are now concrete. IList::Find8298dfb0 does lower-bound by _IName FUNCTION identity (not returned string); Add8298f470 inserts by upper-bound. Each derived interface ctor first retains SAME owner, rejects null, checks duplicate through Find, then Add. Order is ICollidable→IActor→ITargetable→IChasee, Chasee initialization/reset, then IProtectee. IActorQuery and cTriggerTester at this prefix are direct subobjects, not extra UCOM registrations. Static interface tags can encode original identities without guest pointers: IActor825ebe28, ICollidable825ebe38, IPhysical825ebe48, IChasee825ebe58, ITargetable825ebe68, IProtectee825ebe78, IActionGraph825ebe88, IMotionGraph825ec090. Preserve source insertion/comparison ordering if exposing registry traversal; lookup can use a typed equivalent. Failed ChaseeReset leaves IChasee already registered and earlier component fields intact, and prevents IProtectee/Actor fields/position checks/listener.

Implementation proposal: ActorConstruction owns actual gameplay Arc, requested world-manager authority, flags/spawn and one existing ActorGraphAuthority issuance. A Mutex-held prefix state tracks one attempted construction and completed interface/Chasee/Actor-field/assertion writes. End phase is ListenerRequired, NOT ActorComplete. Expose authority only after prefix checks; retain immutable authority outside the mutable prefix so subsequent methods can borrow &self and listener can borrow the SAME authority without a self-referential Rust structure. The caller's construction collection retains listener separately; later graph and pair startup consume that same owner identity. Player ID derives from the supplied constructed gameplay owner rather than a caller-created integer alias. Staged errors retain source prefixes; no automatic retry, rollback, full-Actor ready token, or retroactive rebinding of an admitted diagnostic pair.

Additional hashes: ChaseeReset82de2310..82de23dc ef7587164329840dbcf432e1648bb517c9fbb0abc450e17492a54740c96c933c; Chasee ctor825f6990..825f6a20 fa99c45e83e2fbef4885f5a137909515ef52bd64e2c3073b176ffc70811a973e; min getter826f3f08..826f3f6c c326e7fdc4c3b4843e972fe2afaea439e63f50e0d507777679ba0e0192766ff9; max getter826f3ea0..826f3f04 26317cae37f4c7322ac57d585c777e99bdfea1f915cff0ff014a20a3a67111ad; velocity getter826f3f70..826f3fd4 ed41cc257e92ecd23c7d85cf22144c5be3e2a0baaa9f05c6a43120d41dab534c.

- Actor825fe850..825ff6c0: 000a6b1806f82d590b2ee088127b6b9a9ce0c16ef6f46459bd8de730c346a392
- SpawnPlayer82600630..82600798: 473a784d38c8862b0b8275d64bed6372424f8993fbefe8c32964b893baac7d88
- InitPlayer826000b0..82600380: 2e0b2bc7faf1a6c7a100199ff511ca768f24c63e7a502a85debcb3b3b7c95b03
- DestroyPlayer825fff80..826000ac: a4b7b311f94eaa470c0787605a75a623b4e4e72b6a8a281639f66d85bd9e93c0
- Actor destructor825f9668..825f99b4: ec27f422df638ffd3f55898ed7d093cfef92cbc5762dea6691a3b6624bd87ed1
- Listener destructor825f72d8..825f7348: a1258fd31684a16972a5969a45da536fe821932df8bf454fbb48710e9d502c45

### Implemented selected prefix, awaiting root checks

`actor_construction.rs` now retains the constructor occurrence, its existing
`ActorGraphAuthority<ActorIdentity>`, actual constructed gameplay owner, host
simulation-manager authority, source arguments, interface membership, Chasee
reset fields and selected Actor initial fields. `new` captures the actual
gameplay player index as the immutable constructor argument; it does not run
the native range assertion. `try_construct_prefix` consumes one attempt and
retains failed prefixes. The ID assertion runs after Chasee Reset and Actor
field writes, before the position checks. `authority_for_listener` succeeds
only at `ListenerRequired`, and borrows the same issuance on every call.
The caller can retain the listener separately while keeping this owner alive;
no self-referential Rust owner or substitute Actor issuance is necessary.

`ActorChaseFormationOwner::new(actual_vault)` creates selected null binding
storage; `try_bind_selected_reset` assigns the actual `chase/global` instance
and then enforces the source non-null assertion. A failed assignment/assertion
consumes the attempt. The Actor's three getters use this same retained instance
and actual full keys, decoding each scalar only at its getter. The later Chasee
assignment constructs a distinct instance from the same Vault. Generated
fallback-layout allocation and unused chase-list storage are represented only
at their selected empty/null boundary; no full ChaseMan constructor, global
publication, events, allocator parity, or full Chasee operational API is claimed.

The interface list retains original `_IName` identities in source sort order.
Unknown low 25 packed bits remain unknown through an explicit known-zero mask;
there is no invented initial packed word. The matrix gate rejects XYZ NaN,
allows basis infinity, ignores W, and calls existing `try_length` even for a
zero position. The final comparison is ordered `100000 > length`, not `<=`.
Its numerical policy is the existing caller-supplied Reckoning numeric port.

The `SimulationManagerContactAuthority` is explicitly a host owner binding,
not proof of a constructed native PhysicsWorld or the world/services needed
by the remainder of Actor startup. The endpoint does not publish an Entity,
construct the Action/Motion graphs, mount animation or an actual physical pair,
run InitPlayer, or claim a native Actor-ready token. Those remain subsequent
stages on this same owner. Six focused tests cover partial attribute failure,
binding failure, source defaults/fields, W/NaN/zero/strict-length behavior,
late numeric failure, and native ID-assertion timing. Files were formatted;
Cargo and independent review are separate root-owned gates, not claimed here.
