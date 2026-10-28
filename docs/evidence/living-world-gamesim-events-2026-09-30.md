# LivingWorld GameSim events, September 30

Source-only implementation; rustfmt syntax parsing passed. No compile, tests,
or runtime were executed. This does not establish subscriber completeness.

Simulation ctor828a8728..873c constructs ThreadSafeSinglePortBroadcaster with
literal port GameSimulationBroadcaster, queue4. EventManager.Init829826a4
allocates24576 bytes for queue4, separately from queue7's65536 bytes.
InitLWMan82702eb8 binds Simulation's existing broadcaster through
InitEventBroadcaster, rather than constructing a private LivingWorld queue.

SelectedSessionWorldStartup now retains that one GameSimEventRoute Arc and
binds the SAME Arc to LivingWorldGrindServices before its initialization.
Source header kind/port/size/id and full entity32/GUID64 payloads are retained.

The existing Challenge queue's envelope and EmbedField storage moved to
hermes_queue_storage.rs. Both queues call that one allocation algorithm:
16-byte envelope, message allocation rounded to16, prefix writes retained on
the original overflow boundary. Existing Challenge producer API and snapshot
shape remain intact. This is a refactor, not a second queue algorithm.

Queue.Pop829827b8 delivers before advancing deletion cursor and resets both
cursors only when deletion reaches creation. Flush82982968 keeps popping,
including messages appended during callbacks. Dispatch824eccf8 increments
dispatch tick after Flush completes. Host callback failures retain the message
and deletion cursor. Concurrent/reentrant Pop currently reports an explicit
host boundary; no claim of native recursive/concurrent dispatcher parity.

Pedestrian.OnSpawnComplete82e85f18 reads attribute79d412ca0b043f37 from actual
entity attributes then sends SecuritySpawned(-994188055, size20). Raw PPC
82e85f30..4c forms that full64 key. Callback this is LWBasicEntity+20, so
its +1940 access is actual Pedestrian.mAttribulatorData+1960. The host helper
requires the entity already admitted to the sole EntityMan roster.

OnCullRequested82e85fd0 performs base/animation/handprop/chase operations
before unconditional RequestCull(1004304572,size20), then reads the same
security attribute and optionally emits SecurityCulled(-141156312,size20).
The new pedestrian_cull_messages method implements ONLY this event suffix,
while the concrete lifecycle caller remains responsible for earlier work.

RegisterWaypointData8271a088..a0ac emits WaypointGroupAdded(480852204,size24)
with full source LOCAL GUID+56 when requested. Deregister827222d0 emits
WaypointGroupRemoved(1452361618,size24). Prop.OnCullRequested82e65bf0 always
emits RequestCull with its entity ID. These typed messages use the same route.

Actual subscriber trace: FrontEnd.StartInGame82640904/74 registers security
handlers826063e8/82606410; these call MapBackEnd.AddSecurityIcon825658e0 and
RemoveSecurityIcon82565a60. Add uses actual pedestrian position/heading,
duplicates suppressed and cap10; removal shifts retained icon/entity arrays.
ChallengeCharacterManager ctor825a9038/90c8 registers waypoint add/remove.
Added825a5798 sets UpdatePendingSpawnsNextFrame then consults its authored
WaypointGUIDMap, actual entity roster and LW group to set the first point.
BindAllMessageHandlersToLua828eb5a4/828eb898/828eb8bc also registers RequestCull
and waypoint handlers. Generated Lua SecuritySpawned/Culled bindings exist.

GameSimEventRoute.dispatch requires a real GameSimSubscribers owner. Those
FE-map/challenge/Lua consumers are NOT replaced by no-op callbacks or private
duplicate rosters. Actual selected frame dispatch remains to be connected to
those concrete owners. The producer is wired; consumer completeness is not
claimed. Synchronous waypoint arena registration still needs the scoped
RecipeMan callback extension documented in pedestrian-waypoints evidence.


Freeroam subscription audit: FrontEndManager.StartInGame82640450 registers
SecuritySpawned82640908, SecurityCulled82640978, ChaseStart826409e8,
ChaseEnd82640a58, and ChaseState82640ac8 unconditionally. IsPartyPlayMode
only gates phonelist setup. The GM_ProfileFreeSkate branch82640b94 follows
these registrations and only changes HUD visibility. Native freeroam therefore
has these FE subscribers after StartInGame; their missing selected host owners
cannot be represented as a native zero-handler route.
