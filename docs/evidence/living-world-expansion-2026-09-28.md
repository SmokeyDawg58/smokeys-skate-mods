# LivingWorld expansion: DMO shadows, pedestrians, vehicles, guards

## September 29: live vehicle population, frame, collision and presentation

SOLO. Changes UNCOMPILED/UNRUN. No Cargo,
build, check, clippy, test or game command was run. Scoped rustfmt and read-only
source/assets audits do not establish type correctness or successful traffic.
This section supersedes older missing-frame/presentation statements below.

Existing-owner inventory: reused CensusState/WorldPainter/RecipeMan selection,
VehicleSpawnFrame and retained entity factory, EntityMan and execution groups,
Vehicle.xml graph runtime, RAIN, DmoModelMan Character models, Island/Part pools,
collision solver/contact lists, PresentationRecipeModels, CharacterScene,
Andale SQT operations, CharacterPalette and the dynamic-object GPU passes.
No second traffic world, entity registry, physics solver, road planner, palette
algorithm or model loader. New adapters connect these owners to the live loop.

Population/frame/culling:
- CensusMan.Evaluate82727a38, vehicle Evaluate82720a98, SpawnVehicle8271c188,
  SpawnVehicleEvaluative8271ed50, factory82d25670/InitModels82d1d5f0 rechecked.
  Regular two attempts/one success; initial6000/600 with radius8..80. Original
  initial cap15 and enable masks retained. Initial and regular share one caller.
  Recipe loading is still original ordinary queue cadence one/121frames;
  LoadAllRequired82727330 + InitialSpawn82727400 were found at ReloadEverything
  827274d0/BackToSinglePlayer8273dce8, not assumed ordinary boot operations.
- Vehicle.UpdatePositionalData82d1afa0, PreStateUpdate82d28488,
  UpdateState82d1aee0 run on same core, driver attributes and graph. TestLOSim
  82e50180 and LOSim switch82d21cf0 use source distance sorting/execution groups.
  RAIN target information/transform is updated each frame independently of
  lower-frequency behavior execution. Borrowed CurrentVehicle provides self
  queries without copying entity state or maintaining a second registry.
- Sim.Update1 8289f410 places LivingWorld.Update2 between SkaterAnim1 and Anim2.
  Split the EXISTING actor selected prefix at that boundary; no duplicate tick.
  MotionForcePhysicsWriter owns only borrowed world/skeleton/ground, so scoped
  writers preserve state. Existing host serial scheduling is otherwise retained.
- RAIN.UpdateOnRoadPoints8271b4e8/VehiclesShouldAvoidSkater826f38c0 now read
  actual skater PhysOut and scoring state; DMO corners82d2c7d0, PosRAIN82d2a7d0,
  IsOnRoad82d36330, AddPoints82d363d8 use actual transforms/dimensions/mass.
  Cvars927..929 use original avoidance defaults. No fabricated pedestrian list.
- Active RequestCull ends graph behaviors, removes road/execution/entity
  membership, clears light bias, then deferred destruction reuses same
  Collideable/Part/model/allocator cleanup as rejected spawns. RAIN streamed
  removal also uses this path. RequestExplicitCull82d25920/CullVehicle82d277e8
  both set core AllowCulling and request Census cull (source inspected).
- MemStore.Init82a6dd98 AND FreeBlock82a6df20 zero allocation bytes; Vehicle
  ctor82d1cb00 preserves low flag bits. Actual factory-allocated vehicle motion
  now initializes preserved breaking-rules=false, cornering=0 and skitch delta=0
  from this evidence. Standalone ctor-only helper still distinguishes unwritten
  fields. This avoids an artificial uninitialized flag at first intersection.

Ground/physics/interactions:
- Three HiFi ground probes Add82d1d938, Receive82d17718, Process82d1add0,
  Create82d22a08 retain source all-three-nonzero hit gate and basis arithmetic:
  z=normalize(b-c), y=normalize(cross(b-a,z)), x=normalize(cross(y,z)),
  translation=(a+c)/2. BF_Ground1 uses existing line-query kernel. Host currently
  computes a temporary synchronous batch per car and applies results postphysics;
  native uses five persistent batches. Timing/performance parity is NOT proven.
- Vehicle collisions use existing group10 dynamic pool/Island and original
  target drive. No visual-only dummy cars. Skater/vehicle and DMO/vehicle jobs
  were already implemented and are now populated by real admitted car bodies.
- World.Update3 827af8f0 calls ResolveContactSpyList827b3a90 AFTER physics output,
  not at CreateContactSpyList827ba710. cLivingWorldCollisionBody.GetContacts
  82e50e60 stops after first accepted callback; Vehicle.OnEntityCollision
  82d1d390 always accepts, so only FIRST contact per car is consumed. Parked
  impact speed > driver543475921fd9e04a resets timers/starts alarm. Actual skater
  owner sets contact flag/index bit. ForceStop uses dot(contactPos-corePos,
  GetVelocity), NOT facing; primary vtable82326cac+10 resolves82d36158.
  Collision controller part-flag setters are source no-ops; no invented flags.
- Update0CollisionStatus82e016a0, CheckPlayerInsideVehicle82dfe690,
  IsPositionInsideAnyVehicle82703a58 and sphere query82710760 now use actual KD
  candidates/source local-box tests and cvars930..935. Reuses existing Actor
  appropriate teleport/breadcrumb recovery before input Update1.
- ResetSkitch82d22778/Evaluate82d225a0 and light-bias register8271b470/clear
  8271b4d0 connected; source stopped timer attribute4727cf785ef735c8. Wheels
  Update82d22940/82d215d0 run after target/ground/DMO position updates.

Presentation/assets/shaders:
- selected_living_world_vehicle_visual.rs reads finalized SAME recipe scenes
  and actual transforms, color attributes df76d7d773857edb/12026e2eed18cc8d,
  source DEFAULT SH probe render_locations/ec763034105b866b keyf45dfcab6ec195cd.
- InitVehiclesDB82810b18, pose map82810860, animation commands82809610 and
  wheel pack8280f1cc/unpack82807e88 checked. Uses raw8-bone VehiclesDB pose,
  spinX/turnY, front compose(turn,spin), rear spin, procedural+bind AddSQTtrue.
  VMX82808370..b4 confirms quaternion operand/cross order. Reuses hierarchy IBP
  and CharacterPalette; default bone Chassis (skater Hips remains unchanged).
- Exact wheel pack quantization constants: spin4622f8e0/u16/unpack38c910a4;
  turn4321b390/i8/unpack3bcaa525. Normal fixed60 frame uses current samples.
- Camera LOD82826a10/GetLODDistances82821a48/Switch82806528: authored rendering
  collection51416546b54f421d attr5d6975b42dace3a7 gives65,80,85. Preserve source
  fallback to lower available level; shipped vehicles commonly have0 and2.
- Bounds Calculate827f2708 unions ALL LOD0 model boxes; Cull828501c0 translates
  vehicle bounds only. DMO expanded-origin cube behavior kept separate. Uses
  same combined bounds for model opacity and GPU frustum/shadow culling.
- GPU shares dynamic-object instance/shadow passes, extends actual car vertex
  format (one bone from UBYTE4.W), body/glass/light effects, source car colors,
  SH9, normal/environment maps, alpha handling, fog/CSM/shadows and wheel palette.
  Body/glass source shader UPDB AND compiled Xenos shader disassemblies read.
  Compiled shadow PCF radius uses charattributes[0].z (UPDB stale literal).
  Original shader blobs live under render/data/render/shaders/final.
- Read-only Jan17 asset audit:70 vehicle model arenas (36 body,34 glass), all
  have environment/diffuse/normal channels. Body UV1 float2; glass lacks UV1.
  All16 ordinary recipe vehicle model names match raw poses in VehiclesDB.abin
  (data/big/anim.big,6656bytes); all shipped Vehicle.xml entities lack contained
  prop refs. No vehiclelight materials in these70 arenas. No asset substitution.
- Correct Vault is the Jan 17 build's (vault_jan17), NOT the Nov 28 build's (vault_nov28). Array headers differ:
  scalarLOD header8/data8, material float4 header8/data16, SHfloat3 header8/data8.
  Actual material_params collection already handles car classes; reused it.

Final static review: scoped rustfmt succeeded on25 files in the final review
pass, including the split Actor phase caller, actual contact callback, common
palette, renderer and vehicle frame. Manual visibility/initializer/call-site
review performed. This remains UNCOMPILED and UNRUN.

Known boundaries / next validation:
- Compile with build-release.bat and run the quiet controller test. Traffic
  should populate authored nearby roads as recipes become available; this has
  not been observed in runtime yet. Startup skating success predates these edits.
- No promise of numerical bit parity: existing host reciprocal/normalization
  primitives remain approximate. Broad renderer parity needs visual comparison.
- FrameSaver adaptive population changes are not yet connected (initial cap15
  works); source ReloadEverything/explicit asset unload user notifications still
  pending. Ordinary queued recipe loads work, but asset-cache retirement is not
  complete. Invalid road admission's unreachable-for-valid-road cull callback
  remains explicit Pending; do not silently drop partially allocated bodies.
- DMO RAIN producer currently follows existing every-frame host DMO schedule;
  original execution-group cadence not fully audited. Native async ground/job
  scheduling is not reproduced exactly by this host serial pipeline.
- Pedestrians/security population and audio playback are still separate unfinished
  work; car horn/alarm state is connected but no missing audio engine invented.
- New cvar definition/reference total936; read-only count/order/unique-label
  audit passed. Existing gameplay, terrain streaming and quiet error logs retained.


## September 29: vehicle population selection and generated graph shutdown

The preceding factory/positional/perception work was confirmed to compile and
can spawn/skate. This supersedes its UNCOMPILED status below; its unit tests and
NPC runtime remain unvalidated. This NEW continuation is SOLO, UNCOMPILED and
UNRUN. No Cargo/check/build/test/game commands. Scoped rustfmt check passes all
11 changed Rust files; three new regressions added but not executed. Confidence:
high source control-flow/data ownership; numeric primitives retain documented
host approximation, not Xenon bit parity. No claim of working live traffic.

Existing implementation inventory before edits: CensusState and WorldPainter
target selection, RecipeMan.select_sub_model/asset_id_for_filename, shared
DmoModelMan (both Character and DMO variants), VehicleSpawnFrame and retained
request_spawn_vehicle, NativeSkaterRandom and reciprocal-sqrt refinement already
existed. New callers use these owners/algorithms; no second factory, random
generator, model table, entity map or physics world was introduced.

- CensusMan.SpawnVehicle8271c188, raw PPC checked to recover full64 keys and
  ABI arguments (HexRays falsely reports IDynamicObject and splits arguments):
  allow-LivingWorld bit80; entity collection null -> null; parent model key from
  entities layout+8; GetRandomSubModelKey; optional Juice override; model null
  -> null; Load_Model; GenerateModelAssetID; only when Load_Model is true,
  reject null asset or missing ANY model in the same ModelMan map. Then initial
  cvar or retained frame-saver allowance, positive/count cap, and the existing
  RequestSpawn. forceSpawn does NOT override the population cap. Native Juice
  flag is separate and bypasses that cap; its GetModelName(reset)/SetModelName
  side effects remain an explicit unsupported route, not silently emulated.
  Common allowWorldCollision packet bit is ignored by lower vehicle Spawn.
- EvaluativeSpawnVehicles8271ed50/raw8271ee58..ef5c: capture current entity
  count ONCE before gate/debugmanager layout+121, ghost and network. Ordinary
  pass: two attempts, one success. Initial:6000 attempts,600 successes, radii
  8..80 irrespective of passed radii. Initial allowance normally15 remains a
  separate per-attempt gate; don't optimize away exhausted attempts/RNG draws.
  Each attempt: Manager random position, existing GetCensusTargetInfo using
  captured count, identity orientation, SpawnVehicle(initial,true,false,true).
  New VehicleCensusFrame composes the actual existing factory/admission code;
  the selected live DMO census caller has NOT enabled this vehicle category.
- LW.GetRandomPos82e4e898 raw/VMX decoded through82e4e9cc: first draw is Z,
  unsigned-f32*bits2f800000 then subtract.5; second is X using fmsubs. Unsafe
  normalize with dot3/rsqrt and two Newton refinements; third Manager draw is
  radius, after rsqrt estimate. Reuse try_living_world_range: rounded
  word*(outer-inner), fused product*2^-32+inner. Position is fused normal*radius
  +center, including W. This is square-direction normalization with linear
  radius, not uniform disk/annulus sampling. Two UNRUN regressions cover draw
  count/order, horizontal placement/height/W, and two-draw failure prefix.
- RecipeMan.GetRandomSubModelKey8271afc8 and GenerateModelAssetID82712b18/
  key overload82715360 live checked: add forced-category prefix to the existing
  selector, category0/3 only, same forced RefSpec keys. Source missing RefSpec
  yields collection0. Other raw categories fall through. Filename-only helper
  avoids imposing enum parsing on paths that don't read category. Existing
  select_sub_model still filters loaded models only while a load queue has
  work and consumes global rw::math random for nonempty selection. Asset lookup
  still uses original lowercase filename hash/status-independent record lookup;
  ModelMan presence is the separate spawn gate. Recipe IO timing unchanged.
- CensusMan ctor827208d0 at82720a0c captures kInitialSpawnVehicleCount before
  Manager constructor reaches MeshMan/RAIN (xref82721e4c). Added linked cvar925:
  initializer82f5de70/table82fd1640/ref8304ee24, AI_Spawner, label
  kInitialSpawnVehicleCount, title FrameSaver Initial Spawn Vehicle Allowance,
  default15/range0..24/null callback. Real selected initialization samples it;
  constructor-only query fixtures retain None and cannot invent a cap. Static
  audit:926 definitions/926 registration entries/926 unique indices0..925.
  Existing label aliases remain; this newly appended label is not a duplicate.
  Adaptive allowance updates are not yet connected; max_vehicles is NOT proof
  that Increment/DecrementFrameSaverTargets are integrated.
- Generated Vehicle.EndAllBehaviours8292e2e8 (base dispatcher82e4beb8),
  UpdateExitStates8292e408, UpdateEnterStates82952d38, Update82955fc8 and
  UpdateBehaviours8292cf40 traced together. Existing VehicleGraph is the correct
  generated controller, not DynamicHierarchicalController. Added separate
  active leaf behaviour; entered instance is published BEFORE Begin, End precedes
  erasure. EndAll leaves selected/last state, clocks and delta time intact and
  is idempotent. Counterintuitively, generated UpdateBehaviours still calls
  kStates[current].Update after its active-instance list is emptied. Preserve
  that; a later transition must not call End twice. One UNRUN regression covers
  this lifecycle. Vehicle.end_state_behaviours shares the same existing
  VehicleBehaviourFrame construction as update_state; not an entire cull.

Next cull proof (traced, NOT implemented as a full lifecycle):

- EvaluativeCullVehicles8271f210 copies source map order, checks core+f0 bit80,
  uses actual GetPosition and FULL XYZ squared distance (DMO census uses XZ,
  so its distance loop cannot be copied). Boundary inclusive; unordered also
  selects cull via negated comparison. Calls shared RequestCull8271a5e8.
- RequestCull suppresses duplicate pointer membership, invokes OnCullRequested,
  DetachFromManagers82d26f70 (execution group Remove BEFORE EntityMan.Remove),
  then inserts into pointer-keyed cull map. Update82e67b10 destroys later.
  Vehicle.RequestExplicitCull82d25920 sets allow-cull before RequestCull.
- Vehicle.OnCullRequested82d23bf0 calls Basic82e502a8: sink82e4c500 first
  (RequestAllSinksDeRegister82e4c478 -> DeRegisterAllSinks82e50318 contained
  props; DeInitContainedProps; clean/end/remove plugin), then graph EndAll.
  Raw this is LWBasicEntity-adjusted; +6c virtual+30 resolves IVehicle vtable
  82326b38+30 -> RemoveFromRoadNetwork82d24488. It is NOT a model virtual as
  HexRays suggests. Clear RAIN light bias only if ownID matches, then broadcast
  cMsgRequestCull kind3bdc78bc/size20/port0/id0/entityID. Presentation message
  consumption and native pointer-map order still need concrete ownership.
- Destructor82d25138: notify ped head trackers, RemoveFromRoadNetwork again,
  Collideable.DeInit, core destruction/free, common proxy DeInit, lane passage,
  attributes/grind/grab, Basic destructor82e614e0 (model/cache/RAIN null, graph
  delete, history delete, plugin remove, contained-prop deinit). Existing rejected
  spawn cleanup only covers an entity before attachment/plugins/contained props;
  don't relabel it a complete active cull or skip presentation notifications.

Remaining live prerequisites: outer component census schedule and ranges,
adaptive cap updates, RAIN skater/entity point producer, full pre/state/post
and target/physics frame tasks, active cull/stream-removal/presentation, recipe
references. Then real pedestrian payloads and security behavior. Cars remain
disabled until their producers and consumers are connected.

## September 29: positional tasks, front perception and skitch detection

SOLO, source-verified before writing; UNCOMPILED/UNEXECUTED.
These methods are not a completed NPC frame/population
pipeline and do not enable live cars/peds. Seven additional regressions UNRUN
(twelve including the factory/scheduler tests below). Scoped rustfmt only.
Scoped `rustfmt --check --edition 2024 --config skip_children=true`
passed for all22 files, then the additional state-frame module and its parent,
across these two September29 continuations (23 distinct files).
An initial recursive formatting check also traversed three unrelated existing
files with formatting differences; those were left untouched. No compiler or
test execution took place, so Rust types and runtime behavior remain unverified.

- Added shared core EntityDistanceStack and Vehicle.UpdatePositionalData from
  82e50040/Set82e4ca18, full VMX decoded. Player minus actual entity position,
  dot3, TWO separate rsqrt estimates before refinements, unsafe normal and
  zero-selected length. Set stores magnitude first, then length of
  (previousVector-newVector)*60, then vector and normal. Not radial delta or
  divided by LOSim timestep. Reused reckoning_numeric and existing two-Newton
  kernels, retaining their explicit host numeric limits. Native constructor
  82e4cb38 leaves distance bytes untouched; checked MemStore.Init82a6dd98 at
  82a6de38 and FreeBlock82a6df20 at82a6df68: BOTH clear all bytes. RequestBlock
  82a6e450 leaves payload untouched. Therefore zero fresh/reused state is proven
  by allocator lifecycle; no per-slot stale-data cache is needed.
- Vehicle.GetPosition82d36140 (Entity vtable82326cac+8) reads core+40, actual
  transform translation. BasicEntity positional trailing virtuals+dc/+e0 in
  vtable82326bc8 both resolve true no-op825187c0 for Vehicle.
- Completed source target sort for normal24-car capacity: FillPreFrame82d279f0
  -> quick_sort82e8df98 -> insertion82d1e9a8 (cutoff28, quick prefix inert at24).
  Comparator82e64cf0 rawVMX uses Manager.GetSimulationCenterPosition826f4dd0,
  actual GetPosition, dot3 squared distances, ordered strict less; source ties
  preserved. More than24 targets reports invalid group membership rather than
  applying this restricted insertion path outside its valid source domain.
  Added positional task82d1afa0, with source UpdatePreFrame82d28488 null-local
  zero fallback. This task uses LOCAL ACTOR ROOT, not simulation-center COM.
  Sorting/positional functions still await complete live frame orchestrator.
- RAIN's previous uninhabited RainPayload replaced by actual RainPoint (PDB:
  position0/radiusVecFloat10/compulsoryStop20/signedPedID24); same retained lists.
  CheckEntitiesAgainstVision82d245e0 consumes entity list before skater list,
  clears only active flag, nearest MAX, strictly closer wins (ties first).
  Raw full64 driver keys e939890e1f14dc0b and6367f3054cefb534; speed conversion
  bits40666666. Raw bgt gate means NaN enters avoidance flag/WillRunRedLight arm.
  Reuses existing will_run_red_light. Geometry uses core+80 TARGET translation,
  source all-lane radius/gap comparisons, minDistance+speed, cylinder first only
  for positive radius, then point query. Retains old nearest payload if inactive.
- Vision geometry82d217c8 tests exactly THREE SameSide calls82e4e510, flattening
  tested point Y/W. No fourth plane. Reused cross_full4 and dot3 in source order.
  Cylinder82d24018 tries edges0->2,2->3,3->1, then point. Selected null-output
  CircleLineIntersection82e55578 raw decoded: flatten XZ, assert positive edge
  lengthSquared, reciprocal estimate before projection dot, two refinements,
  reject t<0 or t>1 (no clamping), FMA closest point, strict radius intersection
  (tangency false). Does NOT implement unused non-null intersection-output arms.
- EvaluateSkitchingIntents82d23cc0 rawVMX/PDB checked: clear rear flags, speed0,
  distance-1; already being skitched returns before actor count. Manager order,
  skip PHYSICS_STATE_SKITCHING104 (state, not category); two distinct position
  reads; distance to TARGET translation minus half length; characteristics key
  33466832d8178eaf; rear quad; effective deck Z dot TARGET forward. Writes last
  qualifying candidate's distance/speed even when unaligned, but flags are only
  set/replaced by aligned candidates. Thus later unaligned does NOT clear prior
  aligned flags. Actual intent is PhysOut_Intents+7 has_world_grab_intent.
  BorrowedSkitchActors uses SAME Actor physical allocation and existing PhysOut
  getters, no copied simulation roster. Actor.GetPosition825f9df0 = skeleton
  root translation+1a0; GetVelocity825ed338 = SystemReckoning dampedCOMVelocity+0
  WITHOUT census small-speed threshold; EffectiveDeck825ed448 = board+40.
- Added Vehicle.pre_state_graph_vehicle_suffix and Vehicle.update_state in
  living_world_vehicle_state_frame.rs: compose EXISTING decision helpers,
  checked perception/skitch, existing horn/light helpers, then original graph
  through existing VehicleBehaviourFrame. No second graph/route/motion state.
  Source global RNG for decisions, Manager RNG supplied separately to graph.
  Horn82d18870 rechecked: nearest geometry uses TARGET position+80; compulsory
  point suppresses horn observation. BasicEntity.UpdateState82e4c1d0 graph-null
  branch retained. Suffix explicitly requires caller to run BasicPre first;
  plugin-removal prefix and debug-history Post remain unfinished. These methods
  still have NO live frame caller; do not mark complete pre/graph/post scheduling.
- Tests UNRUN: fixed60 tangential distance delta/W retention and zero unsafe
  normal; three-sided open far end; circle tangency/outside-projection rejection;
  actual-vs-target perception/ties/stale inactive payload; multi-skater ordered
  flags/read laziness; scheduler explicit flag rules/distance ties/separate local
  position. Factory fixture reused, no production surrogate actor implementation.

Next source traces already collected (NOT implemented or connected):

- RAIN.UpdateOnRoadPoints8271b4e8: network games return WITHOUT clearing lists.
  Otherwise clear skater points; manager-order actors; VehiclesShouldAvoidSkater
  826f38c0 gates; deck position on road yields radius0, compulsory=isWipingOut,
  pedestrianID-1. Wiping-out actors additionally test hips+40,head+20,leftfoot+90,
  rightfoot+a0 in that order; each on-road point radius0/compulsorytrue/-1.
  Then clear entity list and Manager.AddAllEntitiesRAINPoints826f5218 in category
  order ped,vehicle,prop,DMO. Vehicle82d21ef0 and Prop82e66368 iterate/assert
  nonnull payloads but add NO points (actual compiled no-op per member).
- VehiclesShouldAvoidSkater reads cvars lazily: disableAll ->false; if neither
  disableAnyBail nor disableAirborneBail ->true; otherwise HoM scoring bail state
  and current-air-drop duration, NOT physics wiping-out/grounded substitution.
  Three cvars NOT ADDED YET: initializers82f64ad8/4b28/4b78, table82fd1ca0/a4/a8,
  refs8304fbcc/fbf0/fc14, section Debug_Scoring_Wipeouts, exact labels
  kDisableVehicleAvoidanceOfSkaterDuringAnyBail(false),
  kDisableVehicleAvoidanceOfSkaterDuringAirborneBail(true),
  kDisableVehicleAvoidanceOfSkater(false). Null callbacks, label=title.
- DMO.AddRAINTestPoints82d363d8: IsOnRoadNetwork first, physics_skeleton_default
  attribute mass threshold (FULL64 key still to decode), !handProp and mass>=
  threshold, actual model bbox transform center, radius=min(X,Y,Z)*0.5, compulsory
  true/ped-1; append center then four mFarthestCorners with SAME radius. Need
  source producer for corners/RAIN data and existing DMO fields before wiring.
- Vehicle.PreStateGraphUpdate82d252c0: BasicPre82e4c248 (plugin removal) ->
  reevaluate actions -> forced new behavior decisions -> vision -> skitch ->
  horn -> lights. Existing route.force_new_behaviours already owns that flag;
  DO NOT add a second one. Basic.UpdateState82e4c1d0 invokes existing graph when
  nonnull. Post82e62858 only cvar-gated in-memory debug state history8, removes
  first whenfull BEFORE querying graph, append only changed name; disabled
  deletes history. Debug cvar/default and plugin lifecycle still need connection.
- Cull82d1aec8 calls factory DestroyEntity. Census.Update82e67b10 drains pointer
  hash-map order and clears map after all destroys; RequestCull/Detach timing and
  active destructor remaining. Do not enable cars while RAIN removal still has
  populated-vehicle pending path. Population/recipe/spawn-point caller and
  vehicle presentation remain incomplete; then pedestrians/security behaviors.

## September 29: vehicle factory, spawn admission and scheduling owners

The previous continuation compiled after two build fixes (RAIN curve module
crate visibility; RenderResult propagates shadow failures while preserving surface
timeout/lost/outdated recovery). Boot/map/skating confirmed in a playtest. This is NOT a
test of the new work below. All new work SOLO, uncompiled/unexecuted; no Cargo,
tests or gameplay launched. Scoped rustfmt parses only; five regressions UNRUN.

Live IDA checked before implementation:

- CreateEntityFactory82d25670 installs Regular82d1eeb8. Factory vtable82326ec4:
  Init82e8a030 calls InitCallbackMap at+18, resolved to clear82d24cd8. GetCallback
  +20 resolves82e66c28: keyed override, including null, otherwise default.
  AddCallback82d26eb0 uses operator[] overwrite. Create82e66bb8 requests object
  storage BEFORE callback lookup; null callback does not free that block. Vehicle
  ctor82d1cab0 consumes LW ID in BasicEntity82e61400 before core RequestBlock
  82d1cd94. MemoryMan.Init82d15e38: stores0/1/2 sizes1130/120/2c, all24 blocks.
  Implemented using existing LivingWorldMemStoreSlots, not a new allocator/roster.
  InitModels now requests collision slot at CreateLivingWorldCollision82d1d290;
  deinit frees it only after body destruction82d1d310. Two factory regressions
  exercise capacities/ID consumption/full64 null override/independent slot reuse.
- Spawn82d27de0 rechecked in full: same road selection, model-key cache before
  factory; model validation before Manager RNG lane draw; road placement before
  DMO overlap; then attributes (global rw::math RNG), AddModels(true,4,4), allowCull,
  InitContainedProps. VehicleSpawnFrame connects the EXISTING helpers in that order.
  Retained LivingWorld exposes spawn_vehicle/request_spawn_vehicle, borrowing the
  actual vault/recipes/physical world. No population producer calls this yet.
  A diagnostic failure keeps the partially built Vehicle in vehicle_spawn_unfinished,
  including any real admitted body; normal rejection runs source cleanup.
- Destructor82d25138: NotifyOfVehicleCulled -> road departure -> Collideable.DeInit
  -> core free -> grind then grab teardown -> BasicEntity -> object FreeBlock
  (Factory.Destroy82d24b90). Rejected-spawn path implements this selected lifecycle.
  ILWManager vtable820d4150+20 resolves folded NotifyOfPropCulled826f5500 ->
  NotifyPedsAboutCulls82e84d90, a head-tracker notification loop. Empty ped roster
  is exact early completion; populated roster remains explicit unsupported here.
  Grab teardown82e62c00 erases each local then global spline in ascending ID;
  grind82e62da0 erases local entries. BasicEntity82e614e0 clears model/RAIN/graph.
  Rejection precedes plugins/contained props/Attach; this is NOT full live culling.
- InitContainedProps82e615a0: default-true cvar checked before Vault; count<=8
  assertion, separate length reread before first loop. Exact empty branch hooked;
  nonempty SpawnProp/parent-attachment remains explicit error (shipped11 vehicle
  entity collections audited absent previously). Added cvar920 from82fa37b8,
  table82fd671c/ref83120218: AI_Prop/kAllowContainedProps, true/null callback.
- RequestSpawn826f5890 attaches then calls OnSpawnComplete. Attach82d26ef0 writes
  existing EntityMan first, then ExecutionGroup[current_losim].Add. Vehicle
  BasicEntity vtable82326bc8+50 resolves825187c0, raw blr (not an invented no-op).
  SAME EntityMan.add_retained preserves an unpublished payload on failure;
  existing add delegates to it. Published map/KD prefixes stay published on error.
  Duplicate wrapped ID remains explicit unsupported after original map/KD prefix.
- Found EXISTING living_world_state_graph_preload.rs and ActorGraphStartupOwners
  delegate store. Refactored existing controller prefix into one checked helper,
  original full preloader wraps SAME helper; no duplicated loop/table. Rechecked
  PreLoadStateGraphs82724280 source: first branch and first path read share captured
  target; later paths reacquire; case-sensitive independent substring writes.
  Actual selected graph-owner startup now runs this prefix once. Cvar921 source
  82f5d320/table82fd15b8/ref8304e8dc AI_LivingWorld/kbUseGeneratedStateGraphForLW
  true/null callback. Disabled-generated dynamic load remains explicit pending.
  Descriptor loop/InitPropData still UNCONNECTED, not marked complete. Vehicle
  spawn consumes existing table delegate2, not per-spawn cvar or fabricated bool.
- Vehicle Init82d29248 captures HiFi/LoFi/NoFi cvars then CreateGroups82d29158.
  Cvars922/923/924 initializers82f8b450/4a8/500 table82fd53dc/e0/e4, refs
  830a6f24/830a6e74/830a6f60. Defaults30/10/3, GUI0..60. Original display titles
  LoFi/NoFi are swapped; preserved. Definition count/order925 and section+label
  uniqueness audited read-only Python. All references registered exactly once.
- Group.Init82d27050 unsigned60/rate, assert buckets>1, rawfdivs1/rate at270c4.
  Source bucket enum PreFrame0/Current1/Idle2, initial pre/currentindex0, bucket0
  PreFrame. Add82d28b08 sorts retained proxy array by entity-list size each call,
  skips CurrentFrame then push_back (no duplicate suppression), listcapacity256.
  Remove82d35500 removes all matches in all buckets. Advance82711608 oldcurrent
  Idle -> oldpre Current -> increment/wrap pre -> newpre PreFrame.
- Sort82d27b08 cutoff28, quick82d27630 median/partition82d33548 right-recursion
  then left; insertion82e86610/688. Important raw-confirmed quirk in heap fallback:
  adjust82e86700 child selection compares actual bucket pointers (cmplw86724),
  whereas promote82e86598 compares sizes. Contiguous bucket indices preserve
  that pointer ordering without guessed guest addresses. makeheap82d34a88 and
  sortheap82e8a4e0 checked. Three UNRUN scheduling regressions cover retained
  equal-count order/current exclusion, 60 equal bucket permutation, source rates.
- GetPreFrameBucketContents82d271d8 appends explicit requests from NON-PreFrame
  buckets, clearing flags, then all scheduled PreFrame members WITHOUT clearing
  their explicit flags. Implemented unsorted collection only; caller MUST still
  sort execution targets by native distance before tasks. FillPreFrame82d279f0,
  UpdatePreFrame82d28488, positional82d1afa0, graph82d1aee0 are next integration.
  Target comparator82e64cf0 returns ordered distance-left < distance-right; raw
  fcmpu/blt64da4/da8 checked (Hex-Rays falsely returns literal0). Full VMX operands
  and distance sort still need final trace/connection.

Remaining before enabling traffic: census population/RAIN spawn-point/recipe
request caller; graph descriptors/props; full frame tasks and perception/recovery;
active cull/RAIN removal and recipe references; wheel/model presentation. Request-
spawn entry is deliberately not invoked by gameplay yet. No live cars or peds.

## September 28: vehicle interaction and target integration continuation

SOLO, UNCOMPILED/UNEXECUTED. No Cargo/build/check/tests/gameplay run. Scoped
rustfmt parsed changed Rust files. No live NPC factory, spawning or presentation
is enabled by this continuation; vehicle/pedestrian rosters remain empty.

- Rechecked LWVehicleMan.QueryGrindSplinesWithBoundingBox82d237a0 in live IDA
  and raw VMX238cc..3908/399c..3a40. Replaced populated-roster refusal with the
  SAME DMO segment kernel: target transform, ordered radius >= rejection,
  inclusive AABB overlap, head..tail stride0x90, FMA endpoints. Both categories
  consume actual registered maps. Existing zero policy for the native
  uninitialized vertical flag remains a documented host boundary.
- Spawn82d27de0 and GetRoadData827267a8 checked live: new VehicleSpawnRoad
  uses existing road-only RAIN triangle search, segment part/2, source
  aggregate-length arithmetic, inclusive15m end clearance and existing
  CanPlaceVehicle lanes. Override branch forces lane0; otherwise nonempty
  eligible lanes are retained. Manager random lane draw happens only AFTER
  successful InitModels/model check (separate choose_lane call). No extra RNG
  draw or new point/road search. Spawn-point producer/factory still missing.
- Authored vehicle GrabSplineQuery82d22e80 and refresh82d1ed40 now connected
  through existing World deferred grab manager. Query keeps feed gate/map
  order, (maxVehicleSize+radius)^2, and first-to-last spline CHORD sphere test
  (not DMO's polyline/CanGrabSpline box filtering). Raw VMX2300c..23150 verified.
  Requires nonempty point/approach arrays and attached skater Body.mIsland;
  capacity32 skips append but continues traversal. Refresh independently uses
  owner map then active roster, leaves output untouched on failed lookup, and
  has no feed-cvar gate. Disabled-authored legacy populated query remains an
  explicit unsupported path, as does vehicle box-query dispatch.
- Vehicle.GetVelocity82d36158 raw core+90 confirmed; source PDB confirms
  collision+18 skater body, Body+24 Island and +18 Assembly. Query uses core
  velocity, not rigid-body velocity. SAME GrabSpline.Init82d1a988 now accepts
  descriptor type1/2; DMO wrapper delegates without changing its geometry.
  SAME DynamicPool/Part grab reader accepts constructed Character memberships,
  reads retained model inertia including actual packed W, actual body identity
  and body pose. Source-routed branch preserved. Constructed pose/motion retain
  the existing semantic host W convention, not a fabricated Xenon pointer.
- NotifyLocal827ac428 -> IVehicle+12882d1d1c8 and NotifyGrabbing827ac568 ->
  IVehicle+12482d1d1b0 rechecked live. Resolve current spline owner/entity for
  each deferred message; set actual motion.local_player_grabbing / being_skitched.
  No invented reset timing: native per-frame behavior reset still awaits caller.
- Read-only shipped livingworld.big audit: seek individual entries only,
  70 vehicle model arenas/70 instances,32 embedded instances,194 grind and32
  grab references. Embedded IDs direct section0; all226 spline references
  section2. Grind typeeb0004,grab typeeb001f; grabs have12/16/20/24 points,
  one approach vector. No duplicate embedded instance GUID within an arena;
  checked inline grab header stride0x50/backing point bounds. This audits data
  shape, not execution of Rust decoder. No original files modified/extracted.
- Concrete Vehicle target frame now borrows retained core/wheels/vision/nav
  fields, reusing existing route and geometry functions. Rechecked source
  AddToRoadNetwork82d17428, UpdateTargetTransform82d26848, vision82d1deb0,
  nav82d20f18 and RequestExplicitCull82d25920. Pass current motion/route state
  to vision AFTER initial speed-limit/route writes, preserving lane insertion
  -> target/wheels -> vision -> nav -> registered flag. Cull callback sets
  allow_culling before delegating to actual caller; owns no private queue.
  Factory/census caller remains missing. Null collision model is allowed by
  vision early return but explicitly refused at subsequent native nav deref.
- Extended existing real-pool fixture (UNRUN) for constructed/source grab Part
  identity, model inertia/W, removed-handle refusal and retained detached
  inertia. New variants/callback signatures were inspected, not type checked.
- Vehicle Spawn DMO-clearance method now uses SAME DMO spatial tree/model bbox
  center. Raw82d281bc..283d8 checked live: LWCommonSpawnInfo.mbForceSpawn+62
  skips this check (separate from Juice override-lane0). Vehicle radius is
  half max(X,Z), DMO radius half max(X,Z,Y), via ordered source fsel operations.
  Query asks64, original manager clamps50. No static-object exclusion. Vehicle
  target position vs DMO current-transform bbox center; native vtables resolve
  GetCenterPosition82d361c8 -> GetTransform82d1cef8/core+10 -> GetBBoxTransform.
  CompareDistanceXZ82707440 uses XYZ dot with Y=0 minus separately squared
  combined radius; raw ble also rejects unordered, equality rejects touching.
  New cvar919 from82f8b658/table82fd53f4/ref830a6e48: Vehicle_Spawner /
  kSpawnVehicleDMOCheckRadius,20(default),.1..1000(GUI),raw r10=null callback.
  Read-only structural audit:920 definitions and full registration permutation.
  Geometry branch regression added UNRUN. Native debug overlay is not rendered.

Factory next-trace discoveries: CreateRegular82d1eec4 forwards Vehicle ctor;
CreateEntity82e66bb8 allocates block before entity-key callback lookup. Spawn
uses CensusTargetCache.GetCensusTarget82e62b70 before factory, keyed by the
authored model; fixed_map256 holds LWAttribCache0x60. PDB fields:hi_lodradius+10,
lo_lodradius+20,hi_losim_radius+30,lo_losim_radius+40,modelAttribKey+50. Refresh
82e4fd00 reads livingworld_models, four scalar attributes splatted into vectors.
Shared cache now implemented in living_world_attrib_cache.rs after confirming
no existing equivalent. Full64 keys from raw ori/insrdi:99dd8f0f82269171,
f918c4555e23b93a,1023caadfc90b0a1,1b9d7cdec5bad784; shipped Vault confirms
all four4-byte fields. SAME retained LivingWorld owner holds one fixed-capacity
256 model-key cache, returned Arc identity reused; checked Refresh failure keeps
native operator[]'s null slot for retry. Vehicle.construct_fields receives that
cache, as LWBasicEntity ctor82e61400 stores creationInfo.mpAttribCache at+54
before consuming its entity ID. Fixture for sharing/capacity/splats added UNRUN.
Factory caller still must obtain cache BEFORE its actual allocation/constructor.

ContainedProps full64 key3eb77fb8ca81c99b confirmed raw615d8..5ec. Read-only
Vault traversal through actual Vehicle.xml graph references found11 entity
collections, all with ABSENT contained-props attribute (24-byte RefSpec array
definition present). This supports the native empty branch for shipped cars;
still implement the cvar/count/branch in caller, never silently omit a nonempty
attribute for other ambient entities. LWCommonSpawnInfo fields confirmed PDB:
entity+8/model+10/recipe+18/transform+20;initial+60/allowCull+61/forceSpawn+62/
allowWorldCollisions+63/loadModel+64. Vehicle Spawn uses ACTIVE4/ACTIVE4 and
allow-world=true independently of that general spawn-info allowWorld field.

Next: actual vehicle factory/census, generated delegate preload, authored spawn
points, overlap admission CALLER, ContainedProps, AttachToManagers and live frame/
presentation/cull. InitContainedProps82e615a0 was read: cvar gate, <=8 assertion,
length rereads and SpawnProp/parent attachment, not safe to silently omit.
Pedestrian/security implementation remains outstanding. Do not claim this is
a traffic-ready build or full parity without compilation and runtime evidence.

## September 28: vehicle model, collision and spline lifecycle

SOLO; all new work remains UNCOMPILED/UNEXECUTED. No live car or pedestrian
has been spawned. No Cargo/check/build/test/game commands were run. Scoped
rustfmt parses the changed files only. Existing regression cases were extended,
and one inherited array-length regression was added, but none were executed.

- VehicleCollision: native ctor82d1a810/base82e571e8, InitBodies82d28eb8,
  AddModels82e5cd40 and UpdateTransforms82e5cf28 now use existing actual Parts,
  same vehicle pool/Island. Model assignment precedes InitBodies; vehicle has
  ONLY skater body(group10), even if fallback model carries a world Definition.
  DoWorld/Skater full64 keys af6dee43dcf73377 / 3bcac7a14c4ebf5f, collection
  8be1d151f9da2d29 checked in live IDA. Source target helper82e5f8d8 and
  shift82e50ae0 raw VMX confirm target.up*(modelHeight*.5) for non-pipe models,
  then velocity FMA at fixed1/60(bits3c888889), all4 lanes. Same helper now
  serves the existing pipe DMO zero-velocity placement. Character candidate's
  actual Assembly handle survives Island removal/re-admission; BodyID renews.
- Vehicle.initialize_models follows82d1d5f0: set recipe, model colour-array
  length/read global rw::math draw/store u8 one at a time, ModelMan.GetModel,
  Collideable.Init82e605a8, model field, wheel radius(or1), AI filename then
  InitSplines82d17340. Colour full64 keys df76d7d773857edb/12026e2eed18cc8d;
  graph key ca4d0ad4ea45965b. Read-only shipped Vault inspection confirmed
  16-byte colour-array elements and11 entity collections selecting the original
  state/livingworldentities/vehicle/Vehicle.xml path (inline pointer fixups).
- Attribute.GetLength8297a0a0/Node.GetCount82979ec0/GetArray82979b78 verified
  live, raw flags lbz/+15. Shared Vault node search, scalar count1 without data
  read, absent0, array big-endian count at+2. Layout uses original INSTANCE
  layout even for inherited nodes; layout then class-static then pointer order.
  Class-static array storage remains an explicit unsupported boundary. Existing
  census reference-array decoder reuses this SAME array storage range helper.
- LoadAIGraph82e4c340 -> Hierarchical.Load82cef180 -> Create829513f0 verified:
  last slash chosen before backslash fallback, case-sensitive Vehicle.xml,
  call_behaviours=true. initialize_models receives the Manager's generated
  delegate setting; dynamic graph branch remains explicit unsupported. Native
  kbUseGeneratedStateGraphForLW init82f5d320 defaults true (ref8304e8dc,
  table82fd15b8); actual manager delegate preload still must be connected.
- SAME GetTInstanceWithCollisionVolume82d10f10 selector now serves model and
  spline setup. Original component/model/LOD ordering; nonzero CModel count
  includes Assembly(-1), with unsupported assembly handling kept in caller.
  Existing ambient load retains embedded grab/grind data while geometry is
  already available; no second asset read/cache/registry. Common embedded
  decoder was extracted from DMO code, preserving section2/header validation.
  GrindSplineSource can retain actual character CollisionArena identity,
  operation_index=None rather than inventing a normal-arena operation.
- GetNextGrab/Grind82e4d368/388 use global .data83025058/5c both1. Lifted
  existing DMO grab counter into ONE LivingWorldSplineIds alongside grind.
  AddGrab82e638b0 has local64 then manager512 gates; AddGrind82e62ea0 local64;
  IDs still consumed on full maps. Shared registration implementation, distinct
  original category owner maps. DMO grind queries now consume registered map
  instead of bypassing local64; IDs allocated after grabs, before active roster.
- CollideableObjectMan Register82e5faf8/GetOwner82e5ceb8/DeInit82e5e9c0:
  one implementation, native per-category128-entry map, real Assembly handle
  independent of active EntityMan roster. Replaced DMO owner scan, so mapping
  survives explicit cull request until DeInit. DMO existing combined creation/
  Island admission remains its host boundary; map becomes visible before roster
  admission, not an exact interleaving claim for native detached DMO construction.
  Vehicle registers before road/Island admission. DeInit removes map then calls
  real collision teardown82d1d310/82e57220. Safe-host partial-insert recovery
  explicitly detaches backend bodies; dropping attached sessions is forbidden.

Next: complete vehicle factory/census Spawn82d27de0 and destruction/notification,
including actual generated-delegate preload, shared model lifecycle, RAIN spawn
point/lane selection, target-control provider, attributes, DMO overlap check,
ContainedProps and AttachToManagers. Then live vehicle state/pre/post update,
collision recovery, perception/skitch, rendering, and population spawn/cull.
Pedestrians/security remain unconnected. Do NOT enable a placeholder traffic
loop or fabricate straight-line movement to claim completion.


## September 28: live DMO Island ownership

September28 live DMO Island registration continuation (SOLO, UNCOMPILED/UNEXECUTED):
Closed the older LivingWorld DMO admission gap found while wiring vehicles.
Actual entity ID now flows DmoFrame.spawn -> DmoWorld.try_add_body -> existing
candidate/pool. After Insert, SAME Island allocator/list/spies register source
Parts, with group14..16. Dynamic Body ID is separate from manager Assembly ID;
STATIC gets0. Source candidates from terrain locators keep ownerNone and retain
their separate existing path. Vehicle and DMO now share ONE LivingWorld Island
membership implementation/metadata vector; no copied registration/cleanup loops.
DynamicIslandBody is a borrowed view of existing source or Character Parts, no
new body/geometry owner. Typed contact owners and lazy skeleton group lookup now
include Dmo. Native AddToWorld82e50cb0/AddDMO827b6798/CleanUpDMO827b6b90 rechecked
live. Removal: spies, contacts, four original query job Syncs, same pool detach/
swap-last/meshes, static then dynamic lists; partial admission returns real handle.
Both DMO and vehicle BeginUpdate now sync existing batch jobs before mesh rebuild.
One real source-Part identity/spy regression added UNRUN; previous vehicle Island
regressions adapted to common membership. Eleven latest lifecycle regressions
remain UNRUN. No Cargo, compile, tests or gameplay run; scoped rustfmt syntax only.
Supersedes older notes that LivingWorld DMO outer Island IDs/spies are missing.
Actual NPC factory/census/behavior/presentation still NOT connected; no cars/peds
spawned. Continue vehicle InitModels/Collideable lifecycle and full factory, then
pedestrians/security. Detailed prior collision frame/source evidence below.


## September 28: shared vehicle collision frame

September28 vehicle collision scheduling continuation (SOLO, UNCOMPILED/UNEXECUTED):
Vehicle BeginUpdate now synchronizes the existing four query batch job families
and rebuilds that same DynamicPool after DMO BeginUpdate. Vehicle publication is
refreshed in the existing serial frame before the exclusive body-pool phase;
native launches DMO jobs before vehicle EndUpdate, whereas this host prepares
both immutable publications before jobs (no intervening body movement). DMO-
vehicle jobs follow DMO-ground/DMO-DMO; skater-vehicle follows skater-DMO, using
shared contact budgets, pairs, thaw and solver. Pedestrian pipeline slots remain
empty. Native InitWorld827b42c0 uses SAME SkaterDMOCollisionJob for skater-vehicle;
MeshCollisionTarget borrows actual source/Character publications and their real
per-Part CollisionArena backing. Ordinary DMO-vehicle collision extends SAME
AssemblyAssembly worker: 10 A/job, padding0, multiple writes; old DMO-DMO remains
5/job,padding0.15. Rechecked Run82e0bef8 and raw job827cc410 (decompiler fails):
cross-pool B starts at0, no same-ordinal self-collision, A/B thaw order retained.
Corrected existing primitive worker flush to every A (827ccaf0) and unordered
material comparisons to source bge/ble; existing numeric regression updated,
UNRUN. Cvars917/918 true; all919 definitions/order statically audited permutation.
VehicleRoute.AddToRoadNetwork82d17428 now ports full64 road ID, fields, native
lane insertion, target update(dt0), registered flag LAST; concrete factory still
missing. No live NPC spawned/rendered, no Cargo/build/test/game run. Scoped
rustfmt parses only; NOT a compiler/type/runtime verification. Older missing
vehicle frame scheduling notes below superseded. Remaining: actual vehicle
factory/model/collision/behavior/presentation/census, ped/security. DMO outer
Island Body/mpOwner/spy registration remains unfinished older work. Next source
trace: Vehicle.InitModels82d1d5f0, LWCollideable.Init82e605a8, AddModels82e5cd40,
collision transform update82e5cf28; reuse existing DMO owners and Part/backend.


## September 28: common vehicle/skater contact ownership and cleanup

September28 latest contact-owner continuation (SOLO, UNCOMPILED/UNEXECUTED):
SAME Island contact lists now retain typed Skater/Vehicle handles, original Part
tags, and actual source mpOwner identity (physical construction Arc or same-world
LivingWorld entity ID). CreateContactSpyList827ba710 rechecked live: both target
lookups/owner reads, same-body OR same-owner, A-before-B append, original16-cap
and independent opposite-before-this velocity reads preserved. Common numeric
kernels untouched. New living_world_contact_owners.rs validates live pool/member/
spy occurrence before returning owner/group. Board surface + post-contact prefix
and SkeletonCollision.Update now accept these typed owners/groups; Skeleton lazy
group lookup receives same existing vehicle pool, no cached guessed group10.
Legacy skater contact APIs wrap same generalized storage. Vehicle GetCollisions
and removal reuse SAME contact entries. Cleanup now source order: spy removal,
contacts erase, four batch-job Syncs, pool detach/swap-last/owned meshes, static
then dynamic-list removal. IDs never refunded. Admission regression expanded to
owner/group/GetCollisions and cleanup, still UNRUN. No actual NPC factory calls
admission yet. Remaining: vehicle per-frame pool/contact pipeline scheduling,
actual factory/census/behavior/presentation, then pedestrians/security. DMO outer
Island registration is also older unfinished work; do not assume its live pool
membership already registers DMO mpOwner/Body IDs/spies. No live NPC or compile/
test/game run; latest scoped rustfmt parsing only. Prior contact-owner-pending
notes below superseded (old physical-only diagnostic API still rejects Vehicle,
but shared contact creation now uses island_body_target and typed owner path).


## September 28: shared Island vehicle admission

September28 latest Island continuation (SOLO, UNCOMPILED/UNEXECUTED):
Vehicle detached placement -> SAME DynamicPool -> SAME Island Body ID/static/
dynamic lists -> SAME spy-target map now implemented as an admission entry point
in living_world_vehicle_island.rs (child of physical_player_island). Actual
body-only Parts reuse existing spy-state backend operations. Candidate retains
entity ID and native Body ID separately from manager Assembly world index.
No live vehicle factory calls this yet. Contact-list construction/board/skeleton
consumers STILL assume skater owners; physical_body_target explicitly rejects a
Vehicle target (LivingWorldContactOwnerPending) rather than dropping it. NEXT:
extend SAME physical_player_contact_list.rs + board surface/prefix and skeleton
collision owner/group lookup to typed Island bodies. Preserve source owner reads,
A/B order, 16-entry cap and current sealed spy frame. Then vehicle cleanup (IDA
traced but not implemented), collision frame scheduling and actual factory.
Shared Island lists now SimulationIslandBodyHandle; legacy skater same_as and
physical diagnostics retain their existing behavior. Vehicle registration tests
added UNRUN (ten latest tests total); scoped Rust syntax parsing only.
Live IDA checked Body.SetPosition82de8090, LWCollisionBody ctor82e5ea98
(mpOwner=entity), InitVehicleCollision82d28eb8, CleanUpVehicle827b6a98,
RemoveVehicle827b75b8 and CreateContactSpyList827ba710. Original contact-list
numeric code stays untouched; typed owner integration is still pending. No NPC
spawned/rendered and no Cargo, compile, tests or gameplay run by the agent.


## September 28: constructed Character collision pool integration

September28 latest DynamicPool continuation (SOLO, UNCOMPILED/UNEXECUTED):
Constructed Character collision publication now uses shared semantic Part pose/
motion helpers, original serialized child selection and existing swept bbox.
AuthoredCollisionPublication accepts Character; pair identity uses actual owner.
SAME DynamicPool now stores source or Character members, shares native capacity,
manager Assembly IDs, swap-last removal and mesh queries. Mesh backing retains
actual source Arc<CollisionArena> for constructed models, no invented DMO plan.
Source and Character refresh share ONE BeginUpdate -> all bounds -> EndUpdate
implementation in serialised_assembly_pool_update.rs; legacy source API wraps it.
Two mixed/foreign-binding regressions added UNRUN (eight latest total). Scoped
Rust syntax parsing only. Vehicle Island BodyID/list/spy registration, collision
frame dispatch, actual factory/census/presentation remain UNCONNECTED; no NPC
spawned. Next reuse/extend physical_player_island.rs SAME Island ID/list/target
owner, not a second vehicle registry. Live IDA checked AddVehicle827b6840,
AddToWorld82e50cb0, Insert82e0dbb0, Begin827c93f0, AddVolume827c86d0,
End82e0bb10, RegisterSpy827b7830, RemoveSpy827b5610 (state store827b5660).


Order: DMO leftovers first, then complete source-backed ambient
LivingWorld behavior. SOLO. No Cargo or
runtime validation has been performed for this continuation.

## Latest collision integration after account switch (uncompiled)

Subsequent binding step (also uncompiled/unexecuted): CharacterModel's existing
collision definitions now have shared mutable storage and one cached attachment
binding. Create82e5cff0 / PhysicsUtility.CreateAssembly82e4a540 reuse the existing
DynamicAssemblyBodyOnlyPartSession and same backend; no separate physics pool.
Repeated SetMasses updates the SAME retained SimulationMutableInertia cells,
through an exclusive matching backend. Cached body inertia fields are not
eagerly rewritten (source only edits Definition). Actual native Part-init
I/J/K/zero transforms, group and Part user data, and shared culling table retained.
Detached Part transform store now shared by full and body-only sessions.

Vehicle InitCollisionBodies82d28eb8 reread: group10, one skater body, materials
zero because callback825187c0 is no-op; fallback world Definition does NOT mean
this method creates a world body. No live InitModels/collision admission caller
yet. Definition layout/material changes after first binding are explicit host
refusals until live Part-pointer propagation exists; ordinary repeated mass
updates share ownership correctly. Two additional UNRUN regression tests cover
same-model/shared-inertia/separate body occurrences and cross-backend refusal.

SerialisedAssembly.Create82e0c850 / Update82e0cf30 reread. Their mapped-child
selection is now one common visitor in the EXISTING template implementation,
also used by CharacterCollisionAssembly.try_volume_sources. Child mesh wrappers
are emitted; child SimpleMappedArray contributes nonaggregate grandchildren;
KD arrays are skipped here (unlike mass-integral traversal). Cap26 and order
unchanged. No fabricated outer arena object. Existing top-level Mesh Create-only
asymmetry remains. AssemblyPoolUpdateJob827cdf78 reread: Aggregate bounds flag0,
all primitive bounds flag1 independently of enabled/frozen. Next is actual
publication and Island admission; all newly constructed NPC paths remain
unconnected to live gameplay. Scoped Rust syntax parsing passed, six regressions
authored in total but NOT RUN.

This supersedes the older pending math/density/body-drive notes later in this
file. No NPC has spawned: body admission, pool publication/contact integration,
factory/census scheduling and NPC rendering are still incomplete. SOLO.
Confidence: high for the traced control/value
graphs, not exact runtime parity. Numeric estimate/exception behavior retains
the existing host boundaries; no reference per-tick trace has validated these
new paths.

- Cvars914..916 added: vehicle density500 (82f8b758, ref830a6edc), LW velocity
  clamp=true (82fa39e8, ref83120300), max60 (82fa3a38, ref8311fd4c). Registry
  audit:917 definitions,917 unique registration indices, no missing/extra index.
- Matrix33.OrthoNormalize82b24f88 and RotationVectorFromMatrix33829b0a50 now
  share existing numeric, acos and XMVectorSin services. The Matrix33 routine
  is distinct from the existing Matrix44 OrthoNormalize3x3; branch/tie and
  reciprocal/refinement ordering were checked against decoded VMX operands.
- UpdateCollisionBodyPartsFromTransform82e506d8 numeric prefix and actual
  per-Part session loop now use the SAME body-only session/backend/pool.
  Source scalar1/dt, epsilon37800000, old-transpose times target, linear clamp,
  world angular velocity. Velocity/omega write XYZ only; ring-link W remains.
  AddForce82de8218 WORLD_SPACE wakes Frozen and clears cooldown before adding
  inverse-mass-scaled force. Gravity cancellation is 411ccccd/inverseMass then
  AddForce's multiply, not simplified. Final wake/cooldown remains after stores.
  New method has no live caller until actual vehicle Island admission exists.
- Reused BodyMassState, existing primitive mass, mapped-array integral and
  Jacobi diagonalisation. ComputeVolumeIntegral82ae9878 now resolves existing
  decoded SimpleMappedArray/KDTreeMappedArray recursively in source child order,
  ignoring enabled flags. Recognized empty mapped arrays return success, unlike
  Null. Malformed cyclic host assets are explicitly rejected.
- ClusteredMesh mass fallback is the original header-bbox approximation:
  raw82ae995c passes the Volume itself as parent; AggregateVolume.GetBBox
  82adc9c8 composes it with its own transform. Existing affine/bbox helpers
  reused. No triangle union or guessed mesh inertia. Vehicle outer two children
  remain original model Volume, then extra box, including identity outer
  TransformIntegral rather than optimizing that operation away.
- CharacterModel's existing definition now implements material writes
  (82e4c818), exact-count ordered SetMasses82e50c10 and vehicle density sampling
  82d285f8. These are definition operations, not fabricated serialized DMO
  assets. Mutable shared-definition lifetime/body attachment still needs its
  model/physics adapter before the operations are called in gameplay.

Validation: scoped rustfmt syntax parsing passed. Four regression tests added
for nested/disabled mass geometry, empty/Null/cycle handling, Frozen activation
with unchanged pose/no torque, and zero-inverse-mass arithmetic; UNRUN. No Cargo,
compiler, unit test or gameplay executed. No new playtest readiness claim.

## DMO world-shadow producer

Connected the existing authored-bound culler to existing map GPU geometry and
ShadowMaps. This replaces dynamicobject's dummy world texture/zero row buffer.
The two dynamic cascades are unchanged; the world pass reuses their D24S8
target afterwards and resolves into its own 512x512 RG map. No second world
geometry store or DMO-specific shadow caster implementation was introduced.

Live IDA checks beyond dmo-render-2026-09-28.md:

- RenderShadows_CSM82855888 raw82856d54..828576bc: scale key9c8706768cfcbb4a
  times ScaleLevel[3]=16, offset key0fb8d6f785615ebe. Same inverse light basis
  as the dynamic pass. Preliminary Z scale bits3a83126f; all translation lanes
  snapped floor(x*256)/256. Frustum position is that translation.
- WorldShadowView fields +4ec0=maxDimension,+4ed0=minDimension. Final depth
  is -worldY/(maxY-minY)+maxY/(maxY-minY); f12's negative multiplier at
  82857334 was initialized to -1 at82855e28/3c. Bounds were padded15% by
  Cull. Degenerate range uses worldZ. XY is unchanged, translation snapped.
- ViewMan ctor8282aed8 rawaf28..50 binds Frustum rendering collection
  51416546b54f421d. Shipped minShadowPixelSize=0,extraRadius=5. The CSM and
  default collections instead have minShadowPixelSize45: do NOT use them.
- render_locations::_LayoutStruct+2 is use_world_onto_object_shadow (true
  in this location). Current gameplay path uses default enabled source toggle;
  NIS/debug render-toggle plumbing is not added by this batch.
- WorldPresentation.AddRenderInstance827e4f00 compares FIRST mesh's sort-order
  byte with pass collectionc9480baf95b5238f layout GUID (shipped6), excluding
  proxies from WorldShadowView. Bounds come from RModel, not tight vertices.
- CalculateBoundsInfo827f2708/827f28d8..958 puts half the four-lane diagonal
  length into min.W. Existing shared numeric policy/refinement reused.
  SetLocalToWorldMatrix827efd08 does not transform m_BB.
- Old-format WorldPresEntity vtable82110f48+24/+34 returns full meshes and
  has no-op shadow cull; AddLODModelInstances827fa1c0 initializes all islands.
  Optimesh uses authored island boxes and original nonzero admission.
- Sky ctor827d433c/436c sets null shadow indices and count0. Its admitted
  entity still contributes to fitting bounds, but draws no shadow islands.
- depthsampler_PS82130890 (container1a8 bytes, code+118 size90): source
  rounds decoded depth bytes then dots with (1,65536,256)/16777215. Coverage
  is UNcovered stencil fraction times X/Y edge fade:
  (1-stencil/255)*(1-20*saturate(abs(u-.5)-.449999988)) and same V.
  This differs from dynamic CSM coverage/blur. Port uses the existing native
  depth/stencil-aspect adaptation, RG16Float instead of Xenon G16R16.

Host boundaries: same existing f32/reciprocal policy, no Xenon bit-equality
claim; empty admitted lists publish neutral rows rather than empty-bound
infinities; retained scene submission ordering replaces native hash iteration
for opaque depth writes. Current normal gameplay visibility is reused; this
does not add photo-NIS visibility or character world-shadow receivers.
Scoped Rust parsing passed initially; regression test added, NOT executed.
GPU/shader/type-check and visual behavior remain UNVERIFIED pending a build.

## Ambient-world audit (in progress)

Reuse LivingWorldGrindServices: it owns pedestrian EntityMan40 and vehicle
EntityMan24 already. They are initialized but empty; do not construct parallel
managers. DMO objects/locators/models are populated and live. MeshMan/MeshStore
are present, but tile admission/removal and populated nav queries stop at
explicit pending errors. RAIN constructor and empty road query exist; traffic
init/update and actual road records do not. RecipeMan.Init mapping/queues and
state-graph preload exist; asynchronous recipe completion is not implemented
there. These are larger missing producers, not just unhandled intent IDs.

Next: trace streamed nav/RAIN admission, original manager update/spawn order,
and the pedestrian/vehicle/security owners before enabling a populated census.

## Navigation and traffic continuation

MeshStore now binds the SAME selected registry before terrain registration.
Add82e63968 links cardinal neighbours reciprocally and inserts last; low32
neighbour-ID test preserved. Remove82e5ecb0 unlinks then erases. Destructor
82e4d3a0 only clears data/neighbours and frees: it does NOT notify pedestrians.
Host weak cell references expire with the tile, avoiding departed arena retention.
NavMesh2 Fixup829c7a20: relative point/edge/cell arrays and arena object IDs for
quadtree/dictionary. Shared WorldPainter resolver/quadtree reused.
GetMeshCell82e580c8: first sorted tile's inclusive XZ bounds, authored dictionary
candidate order (maximum64), raw PointInTris_XZ82e4e1a0, nearest centroid height,
strict y tolerance. GetCellScore82e517f0 preserves .99 threshold and1000 multiplier.
Breadcrumb82ce14d0 now uses its already-implemented score attribute for valid cells.
Tests added but not run; scoped parser checks passed; no Cargo/type validation.

Read-only tools/inspect_living_world_navigation.py over all284 city Sim collections:
NO NavMesh2 records there;75 RAIN records,88 intersection records,464 road records,
8110 segments,493 passages,446 super-road records (shared IDs may repeat).
Validated serialized offsets against original record extents. Zero-count arrays
may contain uninitialized pointer words: do not dereference them in host readers.

TrafficLight AddState82e5fea8, AddSync82e600a8/60170, Update82e54678,
GetCurrent82e54578/545f8 and Sync82e547f0/54838 ported in the EXISTING RAIN owner.
Init8271332c..13514 scalar keys, verified raw insrdi and original Vault:
green3836077defb6f008=7, yellowcfca94d67e0b234e=1,
sync2c0ce3766a0074cd=.5, brisk5e41c959d17527cc=.4.
Even/odd original sequences retained. Pedestrian green splits into walk/brisk-walk.
Strict timer<0, at most one transition/update, vehicle wrap discards overrun and
resynchronizes pedestrians. Raw SyncToGreen82e54844/64/70 uses vehicle +138 index
for BOTH state indices, despite separate +13c pedestrian index; preserved.
Traffic Init now runs before entity Init and clock advances after collision reset,
before census in the existing selected LivingWorld tick. No second timer owner.
Full RAIN storage/admission/spatial lookup and biased-vehicle producer still pending.

Source inventory for next continuation: RAINMemory.Init826f3820 capacities
intersection72x0xbf0,road144x0xe0,super-road144x0x98,KD items3168x8.
RAIN.Init spatial memory288x80 intersection points,2880x80 road points.
AddRAINData827202d8: super-roads dedup, roads, intersections, PopulatePedData in
road/intersection order, chunk insertion/bounds, pending spatial insertion.
Remove82727510: optionalKD removal, roads (notify vehicles/remove super membership,
disconnect/deinit),intersections, empty super-roads,chunk erase,bounds, all entity
RAIN-cache invalidations; when KD removals disabled reset trees and repopulate ALL survivors.
RAIN ctorRoad82e61020,SuperRoad82e63b60,Intersection82e64170.
RoadPopulatePedData82e4dff8;IntersectionPopulatePedData82e52c68.
RAIN asset fixups829c8558/80e0/82b8: nested pointers are relative to EACH record.
Curve CalculateParamFromArcLength82e5b8d0; point82e4de80 calls Hermite829b0f70,
tangent82e4deb8 calls Hermite829b10b0. No existing Hermite helper found in core.

No ambient NPC spawned yet. End-to-end pedestrians/vehicles/security remains
IN PROGRESS; these changes do not certify a playable populated world.

## Streamed RAIN network connected (still uncompiled)

Existing RainMan now owns real road/intersection/super-road records; constructor
placeholders replaced, not supplemented with another road manager. Source capacities
above retained, pool exhaustion preserves the original admission prefix. Super-road
records are copied (native ctor); roads/intersections retain their source arenas.
Curve arc-length upper_bound and Hermite evaluation/tangent use the shared numerical
policy. Hermite basis from initializer82f76448; final VMX W lane is X. Source corner
normal82e54478 uses cross(V3-V0,V2-V1), NormalizeSafe(+Y).
Road/intersection pedestrian adjacency nodes now populated from authored pins.
ConnectRoad82701d68's destination-road branch really reads the ORIGIN ID and writes
that neighbour's destination validity; raw82701e20..48 checked, asymmetry retained.

Native chunk hash iteration is represented explicitly: ctor8271e210 loadfactor1,
growth2, one initial bucket; insert82d62af8 hashes high32+low32 (wrapping), prepends;
rehash82d623e8 walks old buckets/head chains and prepends again. Prime table822852f0
and policy82bd2438 retained. Pending range assignment82715dd8 reserves the entire
surviving count before inserting in source order. This avoids BTreeMap's different
one-chunk-per-frame publication order and preserves initial bounding-box W lanes.
Empty bounding-box uninitialized stack lanes use host zero; no equality claimed there.

Populate82713700 uses two centroids per road segment, four outer intersection
corners sharing two tRAINItemData records. tRAINItemData is POINTER+partIndex,8bytes,
not a serialized ID. Host uses pool slot handles and typed item identity.
Shared LocatorKdTree owns both RAIN trees; no duplicate KD implementation. Added
numeric-only FindNear/nearest and coordinate-only Remove82725b68 overload, reusing
existing walk/removal. EPSILON_23=34000000, radius.1, first4 candidate limit.
GetIntersectionData82726650 searches16 within40; GetRoadData827267a8 searches16
within10. Both sort by distance then use the SAME PointInTris_XZ helper as MeshStore.
GetPointRAINData82702118 prefers containing intersection, then containing road.

Remove82727510 uses SuperRoad.RemoveRoad82e62208, Intersection.Deinit82e61b30,
SuperRoad.Deinit82e61da8. Vehicle callbacks are RemoveFromRoadNetwork THEN CullVehicle;
full64-bit comparison verified82e6235c. Selected vehicle roster is still empty:
occupied lane callbacks remain explicit errors until real vehicles are integrated.
IMPORTANT correction to earlier notes: component vtable+38 is InvalidateAllRAINData,
NOT UpdateCullMaps. Source82d2f8a8/82e82f00 clears mpRAINData on each entity. DMO owner
now has this field and invalidation; current ped/vehicle rosters checked empty.
kEnableKDTreeRemovals8304fb58 initializer82f649d8 defaultsFALSE; new cvar904,
registry905. Default removal resets both KD pools/item pool and queues all survivors.
Optional incremental source removal also implemented; intersection's duplicate
corner pointers are freed once per pair after all four coordinate removals.

Selected normal arena adapter now dispatches Rain only when it has the real retained
LivingWorld route; existing corpus exclusions remain explicit without that owner.
Both admission and removal use typed errors. Same registry bound before terrain
admission. Manager.Init storage precedes traffic, then EntityMan initialization.
Existing selected Update1 now publishes exactly one pending chunk before traffic.
BiasLightsGreen82702190/IsLightBiased826f3ac8 implemented; vehicle bias producer and
UpdateOnRoadPoints still need connection with actual NPCs.

Scoped rustfmt parsing passed for edited modules. KD removal/capacity regression
and pool recycling tests added but NOT RUN. No Cargo, shader or runtime checks.
Original source root roadmap remains: recipes/models, census, actual entities,
navigation targets/movement, behavior graphs/guards, collision and presentation.
No pedestrians or vehicles are spawned by this continuation yet.

## Ambient recipes, models and navigation targets (uncompiled)

RecipeMan ctor82727140, limits820c856c [50,2,1,25,1], Info ctor82701ba8,
Load827239e0/LoadByKey82726210, GetActiveCount8270f540, Update82728380,
OnLoaded82712f30/OnUnloaded82713178, Release82715278/UnloadIterator82712918.
New living_world_recipe_runtime retains the SAME existing mapping/queue builders.
Native status1..5, filename-lowercase GetHashValue32 (not RwHash), reference counts,
SIM-before-PRES notifications, null-asset special case, reverse queue pops and
120-frame throttle retained. Delays freeze while loads are outstanding. Category
limits count every retained record including unloading. GetRandomSubModelKey
8271afc8 filters exactly status3 while any queue is nonempty, uses all authored
entries once queues empty, consumes Random even for one candidate, falls back to
parent without consuming Random when empty. Forced-selection branch still needs
connection; non-forced selection takes caller's actual random producer.

RecipeAttributes reads the SAME loaded Vault. Added strict sorted leaf descendants
and stored RefSpec collection read (+8) by extending existing ClassRefSpec helper.
Native leaf seed itself is excluded (IsDescendant82ab8be8). New cvars905..907:
initial15 range0..64, secondary10 range0..64, forcedfalse. Source initializers
82f64738/82f64790/82f64848. All908 registry indices statically checked once each.

Existing DmoModelMan is the original common ModelMan owner despite its historical
host name. Extended its existing map with CharacterModel entries (shared256 cap),
recipe count and original update flags. No second model manager. AddRecipe82ce9b00,
RemoveRecipe82ce8cd0, Init82ce9dc8, Update82ce6c30. Existing Update1 now calls it
between DMO collision reset and RAIN, source82728204.
LWModel.Create82e51010, Ped.Create82e73890, Ped world capsule82e73988/three-body
skater82e73a40, Vehicle.Create82d23ae0/mesh82d1efd0/worldbox82d1eed0/basebox82e578c0,
BBox82e51218, WheelRadius82d1b1e8. Ped body/arms/world capsule fields read actual
12-byte Vault nodes (definition flags4, verified shipped Vault), not tuned values.
Collision definitions reuse fresh_volume, BodyMassState::native_default, existing
DMO box helper, and shared culling table. Original material .5/.3/.4 from native
PartDefinition.Initialize82ae9058. Mass assignment belongs to entity later.
VMX arm matrix and bounds W lanes checked raw. Vehicle chooses first collidable
component/model/LOD in original ordered maps; uses source volume plus shifted
extra box in that order; wheel radius=abs(LeftFront_wheel inverse-bind Y), default0.
Pipe branch has no world-collision definition. All16 shipped vehicle CModels have
one nonassembly root; assembly branch stays explicit error. Host mapped-array
owner preserves original CollisionArena backing and new box, not fake object IDs;
entity physics lowering is still required.

Character::GetCategoryString82cfd288 and ExtractAssetList82d0c58c confirm vehicle
entry namespace but livingworld.big archive. Shared format path builder corrected.
Shared mounted character decoder captures SIM collision geometry during its existing
read; no second archive pass. FinalizedRecipeScene now accepts native NoClone0 as
well as existing RopaClone3 (AddtInstance82d04994, Rebind82d08fd8) and exposes its
existing resolved ordering to vehicle collision selection.

New living_world_mounted_recipe is a CHILD of existing MountedRecipeRegistry.
It extends that SAME registry/tracking vector with references/finalized ambient
assets, not a parallel CharacterAssetManager. Native LW loader82814900 accepts
categories1/2 and NoClone0. GetRecipeFileName82cfdfc0 maps both to livingworld.big.
OnComplete82d0a220..230 skips FaceMan for those two categories. Existing local
marquee path preserved. Cache hits retain same scene; zero-reference release
requires notification completion before registry removal and returns ownership
hold for subsequent arena release. Actual unload/GC completion must still be
connected. SimCharacterRequests now shares its256 slots/counter with ambient loads
and Unload82817910; OnUnloaded828154a0 releases slot AFTER consumer callback.
MountedPlayerRecipeLoad lends that allocator; no separate NPC allocator.

Intersection NavTargetList ctor82e6426c initializes16 IDs/occupied=true; positions
and entity IDs are native-unwritten until produced. Generate82e5a2b0 checks16 grid
candidates, root+.5*(from+to), step1.4 (3fb33333), source fused operations and loop
order, accepts valid mesh cells with score exactly0. Request82e4dfa8 picks first
unoccupied target, marks occupied, does not write entity ID. Intersection generation
82e5d628 first checks inner-corner centroid; missing tile leaves it retryable. Corners
use unsafe two-refinement normalization through EXISTING shared numeric policy.
GetIntersection827155e0 lazily generates targets. Shared MeshStore extended with
cell validity predicate. Host reciprocal seed/VMX rounding limitations unchanged.

Read-only all284 BAM SIM audit contains no NavMesh1(eb0012) or NavMesh2(eb0022).
Types observed: eb0008,0d,0a,0b,10,11,0f,1d,20,09,1a,04,13,14,1f,19.
RAIN totals88 intersections/464 roads/8110 segments/493 passages/446 super records.
All named BIG headers searched: no separate navigation-named asset found; this
remains an asset-location/registration investigation, not permission to fake cells.

Duplicate-owner audit: DMO-only raw NextEntityID removed. Existing shared
LivingWorldGrindServices now retains existing LivingWorldEntityIdAllocator at1;
DmoFrame borrows it. NPC constructors must use this SAME sequence. Grab spline
counter remains DMO-owned until another actual grabbable kind is admitted.

Current limits: no live ambient RecipeServices implementation or startup/tick
wiring yet; no actual ped/vehicle creation, behavior backend, security chase,
NPC collision or rendering. Existing empty EntityMan owners must be extended.
No Cargo/build/runtime checks. Scoped rustfmt parser checks passed; lifecycle,
queue/model selection, request ownership, vehicle path and nav-target regression
cases added but UNEXECUTED. Do not report populated-world gameplay as complete.

## Live recipe admission and shared census continuation (uncompiled)

Supersedes the preceding no-live-RecipeServices status. SelectedLivingWorldRecipes
now owns the RecipeMan runtime and original distinct PRES recipe membership128,
referencing SAME MountedRecipeRegistry assets. Startup initializes mappings/queues;
SelectedRecipeFrame lends existing registry/SimCharacterRequests at SimUpdate1
8289f580, before LivingWorld.Update1. Load completion is deferred until Load stores
its request ID; slot remains occupied through SIM-before-PRES OnLoaded callbacks.
Character model generation consumes the SAME finalized recipe/collision assets.
Host IO synchronous, native worker timing NOT claimed; periodic initial load
hitches remain possible. Release/unload completion still explicit pending until
actual arena-GC completion is connected. Populated entity notifications not faked.

CensusState moved from DmoObjectMan to LivingWorldGrindServices (same singleton),
all DMO borrowers updated. FE slider/reservation fields/update ported; actual NPC
census scheduling not connected yet. GetPointCensusInfo82702a28 uses painter2/3,
authored class1b7ba2772f011bcd, group/classd4dd1d82b9981bc5 and category
adcb4d6bd3b4f85e. GetCensusTargetInfo8271e738 preserves captured count, group
weights, 1..100 draw and second Manager RNG trunc(fraction*100)%memberCount.
No normalized probabilities or last-item fallback. Cvars908/909 ignoreWorldPainter
and ignoreFE sliders defaultfalse; full910 registration permutation audited.

RefSpec/group arrays now read through existing AttributeRuntime node resolution,
retaining inherited lookup/null semantics and shared Vault storage. Array header
GetData8297b7e8/GetCount82979ec0/GetPointer82979ac8/ctor82979c00 checked:8bytes,
optional8-byte alignment padding,24/32-byte elements, collectionID at+8 and
group probability+24. Narrow typed reader, not generic unresolved pointer support.

Random sources are distinct in original: rw::math::RandomImpl82bc33a8 has its
own global six-word carry chain; Manager has Sk8::Random seeded from that global
at82722018. Reused existing Sk8 seed constants/gate; no second Manager. Global
host seed substitutes clock for InitApp82734a64 hardware timebase; full application
RNG call-sequence parity not claimed. GetRandom82e4e838 uses separate multiply
then fmadds(product,2^-32,min). Both original discrete choices preserved.

Vehicle GetCurvePosUpdate82d171a8, UpdateAcceleration82d18310, UpdateSpeed82d18700
implemented as entity fields/helpers: per-call acceleration ramp, source skitch
braking branches, skid/stop handling, rounded/FMA order and no speed-limit clamp.
Still no actual vehicle entity creation or driving tick. Pedestrian ctor correction:
82e97748 is actual Pedestrian;82e7c900 is PedestrianMan. Native ctor initializes
both navigation implementations and selects PhysicsNavigation at82e97e1c. Missing
BAM NavMesh2 is therefore not sufficient evidence to block all pedestrian movement.

No Cargo/compilation/gameplay tests. Scoped Rust syntax checks only; new regression
cases UNEXECUTED. Actual ped/car/security roster, AI behavior, NPC physics and live
presentation remain to implement. No claim of complete ambient gameplay parity.

## Vehicle graph, attributes and lane geometry continuation (uncompiled)

Vehicle.xml tables821bd130/250/550/6a0 supply9 states,13 transitions,11 conditions
and11 hooks. Existing authored graph is immutable/all-enabled/noninterruptible,
all state expressionsFFFF and all priorities0. Specialized evaluator preserves
ordered first-match transitions, ancestor times and SAME-state exit/hook/enter
clock reset. Controller82955fc8 and child lifecycle functions traced. Behavioral
callbacks still need entity backend; null CreateInstance82bb3da8 does not create
fake state instances. Regression cases written, NOT run.

All11 predicates82d163e0..82d1c5e8 read shared RAIN/attributes. Register checks
last vehicle in ascending lane; unregister checks origin intersection. Float
comparisons and separate-versus-fused operations checked from PPC/decoded VMX.
UpdateAttributes82d25968 reads original driver class02aa3f538d6c188a and original
characteristics: probabilities, speed deltas, length-scaled clearances and random
acceleration. Global vector choice uses ordered>=, distinct from census scalarble.

Road/segment transforms82e5bfa8/82e5d7d8, position/tangent/normal, width82e4e0c0,
super-road lookup82e5c2c8 reuse existing RAIN assets and numerical services.
LookAtUpEye829b0860 added to shared math: normalize Z/X, cross Y without final
normalization. No alternative road representation. Lane and parking registration
82e5db10/82e5dc98 maintain source-stable ordering and total lengths; removals,
front/rear indices, clearance82e5a720 and parking search82e5a990 retain source
asymmetry/capacities. Missing membership becomes host error at native invalid
sentinel access. Vehicle creation, actual driving callbacks, NPC bodies/rendering
and pedestrian/security brains still unfinished. No Cargo or runtime checks.

## Vehicle obstacles, intersections and driving updates (uncompiled)

FrontVehicle82d18d20 and FrontEntity82d18ef8 now share retained VehicleMotion
fields. Driver waiting-speed hash40540e1a0a5d447a recovered from raw64-bit args.
Traffic queue stop propagates Intersection rather than Vehicle impatience;
entity waiting requires travel intention TRUE, unlike vehicle waiting.
StayParked82d162d0 clears speed/acceleration and resets parking time during alarm.
WillRunRedLight82d1a768 checks driver speed f142abbfbeda71e2 before local-grab.

Intersection CheckIfPassage82e59270, CheckIfPassageToLane82e595c0 and
PassageApproved82e59918 ported: full64-bit destination equality, proximity/ID
tie-breaks, accumulated occupancy rule-breaker promotion, original lane ordering,
light time remaining, exit capacity, turn-conflict order and state-output writes.
Same RAIN lists and vehicle provider, no duplicate traffic simulation. Cvar910
kDebugOverrideIntersectionRules and911 kCheckVehiclesInParkingLaneBeforePullingOver
read from actual default-false initializers82fa43e0/82f8b818; total912.

ContinueFollowingLane82d1f310, PassingIntersection82d20038, Impatience82d1b298,
ChangingLane82d1b680, PullingOver82d1bc38, PullingOut82d1c2e0 and StayingParked
update bodies implemented; graph's actual live entity backend still absent.
Preserved source one-first-intersection-car per passage, pileup occupancy choice,
Manager RNG trunc(fraction*100)%count, lane removal halfway after vehicle rear
clears, and distinct quarter/half/full minimum distances. Hex-Rays incorrectly
shows0.5 where VMX overwrote a stack slot with summed vehicle half-lengths:
raw reloads82d1b5d8/82d1c528/82d1ba50 checked. Old-lane check specifically reads
OTHER vehicle's DynPassage.valid10f1, not parked/alarming flags.

DynCurve82e5fcb8/82e5ba48 shares existing Hermite/arc interpolation. Sampling
uses numPoints+1 values and original four-lane length, including native W.
Invalid count<2 returns0 but leaves prior length/samples. DynPassage82e5bbb0
constructor leaves length uninitialized; Deinit82e591f0 retains rows/length/capacity.
Still missing transition hook backend, wheel-adjusted curve generation, target
RAIN updates, actual vehicle/ped/security entities/physics/rendering. No Cargo,
Rust tests or runtime executed; scoped rustfmt syntax checks only. Regression
cases for waiting/alarm/curve lifecycle added but UNEXECUTED.

## Vehicle graph hooks and route advancement (uncompiled)

All11 transition hooks now implemented:82d16fa0/16fe0/170e0/17190,
82d1c870/c990,82d20930/20ba8/20d20,82d241f0/25738. Existing RAIN owns lane
and intersection queues. Intersection departure uses native pop_back, registration
push_front with capacity16; no erase-by-ID substitution. Hook callbacks and all7
driving bodies now connected to VehicleBehaviourFrame, not yet a live NPC frame.

Wheel positions82d17228/172b8, super-road wheels82d17900 and chassis adjustment
82d18108 preserve source boundary arithmetic and unchanged Y axis. Lane-change
curve builder82d1dc80 uses adjusted Z, RAIN tangentFactor d2c11cc10c9e0e49 and
signed-byte parameter count e35079bdd5bae286; shared Hermite/numeric services.

UpdateTargetRAINInfo82d25d98 advances at most one node per call; collision blocks
the update. Dynamic curve position uses refined reciprocal and fused interpolation.
Road/intersection crossings update native queues and core explicit-update requests.
Forward missing-road lookup requests culling; backward missing predecessor still
publishes its ID/position, matching original asymmetry. VehicleRouteControl requires
the actual owner to write core f0 bit40 (mExplicitUpdateRequested) or run
RequestExplicitCull82d25920 (f0 bit80 mAllowCulling then census RequestCull).
It does not create a second cull queue. Actual entity owner remains unfinished.
Only scoped rustfmt parsing; no compilation, Rust tests or gameplay performed.
# Account-switch continuation: vehicle geometry, signals and owner (uncompiled)

Further vehicle pool integration (still no spawned NPC): WorldLineQueries'
ConstructorVehiclePool replaced IN PLACE with SimulationDmoAssemblyPool's
shared DynamicPool implementation, parameterized capacity. DMO remains50,
vehicle24; corresponding mesh capacity now follows that owner. Line, trajectory
and StepSelector vehicle consumers read the same live pool collection. Names
of existing shared handle/mesh types retain Dmo for now; no duplicate algorithm
or registry was added. Source InitWorld827b479c..47b8, DynamicPool constructor
827caf20 and shared mesh source already documented in dynamic-trajectory-providers.
Cvar913 kVehiclePoolUpdateInParallels from82f6a530/table82fd2240/ref8305271c,
defaultfalse/null callback. Total914 definitions/indices audited. Unsupported
parallel mode remains an explicit host refusal, as with the DMO pool.
All config fixtures supply this field; updated empty-pool tests remain UNRUN.
Entity registry regression added for map-only removal followed by the separate
clear/rebuild phase, native colliding bucket order and initialization validity.

Next collision sources traced, NOT implemented yet:

- cLWVehicleCollision.InitCollisionBodies82d28eb8 allocates only skater body,
  group10. It does not spawn an independent DMO or world body.
- FillSkaterCollisionMasses82d285f8 uses ((dimensions.X*Y)*Z)*density, where
  kVehicleWorldCollisionDensity ref830a6edc/init82f8b758 is500(default), GUI
  .001..10000. This cvar is NOT YET registered. Material callback is source
  no-op825187c0, leaving caller-initialized friction/restitution allzero.
- cLivingWorldCollisionBody.Create82e5cff0 writes definition materials, computes
  mass properties per part with density1, then shares existing assembly creation.
  Init82e605a8 registers actual body assemblies with collideable-object manager.
- AddModelsToWorld82e5cd40 sets core transform first; AddToWorld82e50cb0 group10
  calls Island.AddVehicleBody827b6840 (original DynamicPool plus body list/IDs).
- UpdateCollisionBodyPartsFromTransform82e506d8 derives linear/angular velocities
  from target/current part matrices, optional clamp and gravity cancellation,
  preserves velocity/omega W and wakes state2, resets cooling. Do not replace
  this with a per-frame body teleport. Math.RotationVectorFromMatrix33829b0a50,
  Matrix33.OrthoNormalize82b24f88 need the remaining numeric graph audit.

Vehicle pool admission, body drives, skater-vehicle contact pipeline and per-frame
pool updates are still missing; the new query owner remains empty in gameplay.

Work continues SOLO, with no Cargo/check/test/game runs. Rustfmt is syntax checking
only. No vehicle or pedestrian has been spawned or rendered by this work.

Source additions since the route handoff:

- Target transforms82d17e60/82d18010/82d1db38/82d26848, wheels82d17cc8/82d17b38,
  UpdateWheelAdjustments82d215d0, vision82d1deb0 and navrig82d20f18. Raw VMX
  recheck corrected the intersection normal to cross(vertex3-vertex1,
  vertex2-vertex0). Preserve original mileage addition and asymmetric wheel
  baseline/heading, four vision lines/spheres and all source coefficients.
- MakeNewBehaviourDecision82d19718, MakeNewActionDecision82d190c8,
  ReevaluateActionDecisions82d1d850, adjacent lane choice82e62048. Shared global
  RNG occurrence/order retained, including probability0/1 and one-choice draws.
- Road teardown82d24488/82d17590/82d24340: original two-lane/parking membership,
  initialized-vs-valid dynamic passage distinction, first-match passage removal.
- UpdateLights82d18b20 and UpdateHornState82d18870. Same route light byte;
  braking delay0.5, blink reset/toggle without catch-up. Cvar912 initializer
  82f8bfa8/table82fd5480/ref830a6fb4, GUI0..10, raw r10=0 callback. Registry913
  definitions and913 unique registration indices, no omissions/extras.
  PDB IPedestrian vtable+104 is SetHonkingVehicle, not an audio-only event.
  Target82e6ed98 writes vehicle ID. Authored driver keys checked in accessor
  instructions82d15c30/82d15c78/82d15d50, blink key82d18c9c.
- EntityMan now accepts typed payloads in the SAME owner and uses the existing
  LocatorKdTree. Source AddEntity82d26dc0 publishes map before tree; remove
  82d26d48 only removes map membership; Clear and UpdateSpatialLookups82d26c28
  remain separate. Map traversal reuses the DMO EntityOrder helper with bucket
  count parameter: vehicle23, ped41, existing DMO47. Verified constructors
  82d253e8/82e8ded0 and GetPrevBucketCountOnly82bd2278/table822852f0.
- Vehicle field owner uses ctor82d1cab0, full64-bit RefSpec collection+8,
  existing attribute instances and shared global entity-ID allocator (type3).
  Existing vehicle registry is now typed; no separate map. RAIN/driver getters
  read this owner, matching source82d1d140..1d1a8 and constructor field mapping.
- Shared common-core selected fields from82e4cb38/82e50f48. Raw instructions
  show entity/model/recipe keys0 (decompiler produced misleading64-bit values).
  SetTargetTransform82e4cbf8 checks input XYZ; SetTransform82e577a8 checks OLD
  transform XYZ then writes input and XZ position. Original NaN-only validity
  ignores W. LWBasicEntity.UpdateTransform82e5b358 velocity was already in DMO;
  extracted that identical math so ambient code reuses it. DMO fields/owner
  remain intact.

Remaining: actual factory allocation/admission, model/physics/grab/grind owners,
spawn/cull scheduling and execution, perception/skitching, live presentation,
then pedestrians/security. Vehicle.construct_fields intentionally does not
claim or perform the constructor's unported manager registration side effects.
Empty-provider guards still reject populated flows that lack their consumers.
No gameplay-readiness or full parity claim; numerical host boundaries remain.
