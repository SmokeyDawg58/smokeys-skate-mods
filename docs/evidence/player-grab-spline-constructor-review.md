# Independent PlayerGrabSpline constructor review

Reviewed root draft physical_player_grab_spline.rs and physical_player_grab_query_manager.rs against fresh IDA on2026-09-12. Reviewer owns no implementation. Confidence high for selected constructors; live query admission/execution remains unimplemented.

One mismatch: PlayerGrabSpline.interactable_object_id was i32; PDB InteractableObject.mObjectID is unsigned int, requiring u32. GrabSplineDescriptor.mObjectID is int and remains i32. Root notified before integration.

## Source proof

HiLOD82e04310 requests0x2ca0=11424 bytes, Sim category, flags1. Native ctor82dd1c10 runs only after nonNULL allocation and result publication is82e0433c; NULL manager is dereferenced at the tail after all earlier initialization. The draft retains the allocation prefix and delays returned=true until actual manager check succeeds.

PDB PlayerGrabSpline: datafuture0, alternate272, queryfuture544, pending8768, validated10064, interactablefuture11360, positions11376/11392, manager11408,linebatch11412,world11416,flags11420. GrabSpline256 bytes, futures272; querybuffer32 is8208 bytes, querybuffer5 is1296. Draft exact counts and separate readiness flags match. Interactable count/cursor and low flag bits are known zero from actual Sim parent allocation even though constructors leave those cells untouched.

GrabSpline82d98ce8 initializes descriptor and inertia then Reset827ada10. Fresh raw Reset confirms all temporary vector lanes explicitly zero (notably P2 first lanes827ada58/5c omitted by HexRays). Transform uses I/J/K with ZERO translation W-column. P1/P2/normal/velocity/bodyposition zero; length/half/sort zero; descriptorInvalid0/object0, four pointers NULL, mFlags0, status upper bits cleared. mLengthUnit+0x70 is untouched, but known zero from actual parent Sim allocation. Inertia has inverse tensor four lanes1, inverse mass1,spherical1,max velocity/omega f32MAX,drags0. Draft shared SimulationInertia::native_default plus retained inverse_tensor_w1 matches. This in_sim_allocation constructor must not be reused for non-Sim callers without preserving unknown residuals.

Buffers82dd1468 and82dd13b8 initialize every one of32/5 embedded splines then result count/cursor0. Player has both data futures, queryfuture, pending/validated buffers, interactableobject0/readinessfalse, zero positions, manager/world references, NULL linebatch, clears2 high flag bits. Its final HasDeferred call receives the SAME queryfuture allocation.

Manager827afa90 PDB3520 bytes initializes lock and four actual empty fixed_vectors capacities16/16/16/1, plus NULL line batch. HasDeferred827ac080 locks, walks mQueries in order comparing mOutput pointer identity, unlocks on either return. Draft Mutex and ordered Arc::ptr_eq search reflect these contents and do not return an unconditional miss. Future producers must retain full128-byte GrabSplineInternalQuery semantics and enforce original fixed capacities; no query-admission implementation is claimed now.

World Initialize827b125c allocates0xdc0;1268 invokes ctor on nonNULL;1278 loads priorWorld+0x3c, compares to new pointer, destroys old nonNULL at129c, stores new at12a0, then GrindData.Init12a4. A selected initial World with no prior manager can construct a real empty owner. Repeated initialization cannot silently discard a populated existing manager without implementing source destruction. Shared World manager mounting remains root integration responsibility.

No Cargo or corpus tests run by reviewer. Root owns source correction, mount and validation.

## Next constructor dependency audit

Fresh HiLOD callsite scan identifies26 state slots: Sleeping0456c and Slide045bc use PhysicsState82d83aa8 plus inline derived setup; Revert04610 ->82d912a8; GroundAnimation0464c ->82d83dd0; PhysicsAir04688 ->82d8a9e0; KnownAir046c4 ->82d8b578; five grind constructors04704/754/7a4/7f4/844 ->82d84fa8 plus respective inline derived vtables; Wipeout04890 ->82d9fcb8; Nonspecific048d0 and FollowPath04968 use base82d83aa8 plus inline setup; Teleport04928 ->82d911b8; Skitch049b4 ->82d9d908; HandPlant049f0 ->82d972c0; FootPlant04a2c ->82d975d8; Boneless04a68 ->82d977e0; OffBoard04aa4 ->82d916d8; Locomotion04ae0 ->82d91b18; Pushing04b1c ->82d9c980; LandingDeck04b58 ->82d97f48; Ground04b98 ->82d8cef0; OffAir04bd4 ->82d94040; OffJump04c10 ->82d98428.

Existing retained owner has Sleeping/Ground/Air/Wipeout;22 slots remain. Most constructors bind actual PhysicsSystems collaborators and initialize fields without live physics callbacks. State base reads board,skeleton,Processed,Wipeout,Reckoning,TrajectorySelector,SkeletonController. GrindCommon additionally retains GrindJumper. Teleport retains actual conditioners. Offboard family uses newly constructed PlayerGrabSpline,FeetIK,LandingDeck,Location/Jump owners; no fake replacements. Pushing is the major constructor with live generated attribute reads: two actual PID control data fetches before final Reset. Skitch constructs GrabSpline then Reset. Raw keys and full nested PDB must be reviewed before implementing any remaining state.

Tail after26 states: actual LivingWorld resource allocator -> CreateCollisionCapsule82e4afa0(radius1,length1) at04c60, despite parked LivingWorld gameplay. Three surface results/positions/batch pointers explicitly reset04c80..cfc. Sleeping pointer selected04d0c then actual SetPhysicsState(Ground100)04d10. Wipeout invincibility float bits3d23d70a stored04d24; vector+110zero. Optional actor query at+314 determines collision-world-only toggling with exact currentflag branches; board/controller calls04d70/78 versus04d9c/a4. Mandatory world services+318 virtual+8 callback at04dbc precedes jphy-state+174=0 and actual ctor return04dd8. Therefore cached Actor publication still requires these tail callbacks/capsule, not merely all state structs.

Proposed bounded next dependency: a concrete retained22-state constructor module, reusing actual state bindings and owned collaborators, followed by full tail producer/callback integration under root. No implementation started by reviewer.
