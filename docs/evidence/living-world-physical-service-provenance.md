# Selected LivingWorld physical-service ownership

Fresh IDA/PDB/raw review, 2026-09-12. Gameplay remains parked. This packet and
`living_world_physical_services.rs` do not claim full Manager construction.

Actor raw825ff244 loads Simulation+e0 (Workspace.mLivingMan), compares nonnull,
asserts atff25c/DebugBreakff260 if absent, reloadsff264 and copies to
Init_SimulationData.ilwmanager atff274. The producer is Workspace::InitWorkspace
828a7fb4 aligned workspace storage, Manager82721dc0 call7fc8, publication7fd8.
Manager is PDB112096 bytes, ILWManager base offset0. It is not an empty service.
HiLOD stores the same interface at+31c at82e03ec8. Skeleton does not own it.

The selected projection retains actual Sk8::Random Reset/SetSeed fields,
actual debug attributes, empty waypoint key collections and explicit source
scalar/null-presence fields, plus the actual capsule allocation result.
Debug generated wrapper826fcae0 classFF1968A5C467BCF9, collection supplied by
82721f38..58 isF97B9086CAA50C53, cached-layout fallback132. Random Reset82a6dba8
copies kInitSeed8220554c=[f22d0e56,883126e9,c624dd2f,0702c49c,9e353f7d,6fdf3b64].
SetSeed82a6db50 writes seed,index0 and seed+each constant with wrapping arithmetic.
The external rw::math::Random call82722018 must execute after prefix scalar
stores, before actual CreateCollisionCapsule82722074. Narrow effects preserve
that ordering and partial failure. Parked nested constructors remain explicitly
Unrecovered; selected_prefix_complete is not full Manager completion.

HiLOD +31c operand audit across named methods found only constructor,
CheckPlayerInsideVehicle82dfe690 and UpdateCollisionStatus82e016a0. The former
calls IsPositionInsideAnyVehicle at82dfe6dc. The latter conditionally calls
pedestrian sphere82e018e8, BuildSkaterTestCapsule1a9c, BuildSkaterTestAABBox1abc,
dynamic capsule/AABB1ae0 and vehicle sphere1b6c. No initial Ground constructor
service query was found. Exact implementations:82703600,827041a0,82704488,
82725930,82710760 and insideVehicle82703a58. Pedestrian/vehicle queries resolve
actual global submanager aliases; dynamic queries use this->mDynamicObjectMan.
The decompiled apparent constant false still contains required spatial calls.

Three component Init entry points82d29248,82d3e030,82e9ef00 call shared82d2af28.
It creates/stores MemoryMan, ExecutionMan, EntityMan (storeaf80), Factory,
CensusMan, then initializes the same five owners in that order. Entity factories:
vehicle82d27908 allocation0x24c/ctor82d26b48; dynamic82d37778 allocation0x454/
ctor82d366d0; pedestrian82e93678 allocation0x38c/ctor82e92c40. All use actual
LivingWorld category, flags1. Entity ctor establishes empty fixed map, MemStore
and NULL KDTree root. These are meaningful source-owned empty collections.
Entity Init82d26bc0/82d36748/82e92cb8 initializes node size80,count24/50/40.

MemStore ctor82a6dd08 sets size/count0 and managed/free-list pointersNULL;
freeListIdx remains untouched. Init82a6dd98 sets size/count, allocates managed
size*count then indices4*count, zero-fills managed storage, writes indices
count-i-1 and finally freeListIdx=count-1. Existing host slot allocator alone
does not prove these actual storage allocations occurred.

KDTree FindNear82d28c00 writes output count0, checks maxItems<=256, then checks
actual root. A retained NULL root produces no traversal and leaves result arrays
unchanged. GetWithinRange82d29030/82d3be78/82e9ab88 requires actual EntityMan;
constructor-only component storage is not sufficient. A future bounded query
implementation must inspect this actual tree and reject populated or unrecovered
nodes until traversal is implemented. No unconditional query-miss service is
provided by the selected projection.
