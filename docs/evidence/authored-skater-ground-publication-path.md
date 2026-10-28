# Authored board publication to original skater-ground collision

2026-09-07. Initial next-path audit after the2379 spawn checkpoint, followed by
independently reviewed authored-publication/ground implementation. Confidence
**high** for the scoped source ordering and consumed semantic XYZ path. No
native per-tick trace or raw packed-W parity claim. Actual SkaterPool membership
and the full scheduler remain open; the connected component is detailed below.

## Outcome

Next connected path: real SkaterPool admission -> authored collision Create/
Update and bounds -> existing resident GroundPool job/Contact producer -> the
same live Joint/Drive/Contact compiler and Horus/integration. The actual board
already owns seven bodies and19 primitive occurrences (4wheels,2trucks,13deck).
Shared wheel allocation does not remove repeated occurrences. Disabled children
still participate in the source Create/bounds traversal where prescribed.

## Original ownership blocker (now bridged for the component)

`serialised_assembly_template.rs` template/inertia/culling/VolumeDataSet requires
Arc<SimulationNormalArenaPlan> and arena ObjectIds. `serialised_assembly_create.rs`
PreparedSerialisedAssembly adds arena/packed-word image authority, and Create/
Update requires SimulationRigidBodyAddInertiaOwner. Pool refresh shares those
requirements. `simulation_ground_contact.rs` and `simulation_assembly_ground_job.rs`
accept that prepared serialized owner and SimulationPackedBodyWordImageDomain.

The authored board instead retains real collision definitions and Arc inertia.
Do not manufacture arena resources, serialized inertia backing, ObjectIds or
virtual Xenon addresses to satisfy these APIs. Preserve the existing genuine
serialized/packed-word route while creating a source-owned authored branch.

## Original producer and scheduler

`Island::AddSkaterBody 0x827b66d0`: if Body.mAssembly exists, construct serialized
Params, select normal-bending threshold (enabled0.99900001, disabled-1), call
SkaterPool Insert at827b6748, then assign Body.mIsland. Static state gets BodyID0
and static membership; other state consumes separate Island mNextID and dynamic
membership. This BodyID is NOT manager Assembly world index. The board must
consume the existing shared manager counter BEFORE HookPool insertion.
Earlier board recovery supplies Assembly group4, every Part userData4 and49
self-culling bits; current direct session attachment does not publish these.

`InitWorld 0x827b42c0`: SkaterPool capacity16 when flags&1, otherwise2; serialized
records enabled; one assembly per update job; parallel-update setting sampled
from live kSkaterPoolUpdateInParallel (initializer82f6a490 defaultfalse). A distinct
SkaterGroundCollisionPipeline receives this pool, actual GroundPool and resident
collections. Its overrides are allowMultipleContactWrites=true, assembliesPerJob2,
triangle-cache count from CURRENT kSkaterGroundTrianglesToAllocate (initializer
82f69d98 default0/range0..10000). applyObjectFixes=false and null debug provider
are Params defaults827c7f68. These constants are source evidence, not permission
to freeze live cvar services.

`Step_Collision1 0x827b3818` completes SkaterPool EndUpdate before ground Run.
Pipeline Run827cab00 supplies pipeline culling, GroundPool collision group and
configured object-fix setting. Board self-culling is NOT the ground-pipeline
culling table; neither may substitute for the other.

CRITICAL: flags==3 asserts EVEN SkaterPool count and submits adjacent-member
pairs, ordered by native Assembly pointers, BEFORE ground Run. Full offline
semantics therefore require the actual skater counterpart and pair producer.
Do not fabricate a second Assembly or pointer. A bounded single-board/static-
ground consumer may use another source-supported flag profile after auditing
its settings; it cannot silently skip this branch while claiming full flags3.
Distinct static-ground collision need not await generic dynamic pair collision,
but no restricted test proves the complete source scheduler.

## Full-path implementation plan and acceptance

1. Manager-bound real SkaterPool membership owns/protects the actual session,
   consumes shared Assembly world ID before Add and publishes source metadata.
   Preserve explicit host safety divergence from native failed Insert behavior.
2. Authored Create/Update traverses actual wheel/truck/deck owners in source
   order, consuming live Part transforms and seven checked body observations.
   Preserve original26-record capacity, padding0.05, flags/material/userData/
   culling ownership, header selection and bounds ordering. Reuse scalar record
   kernels where appropriate; no arbitrary-record publication constructor.
3. Sealed authored ground publication retains definition/culling ownership,
   body-pool authority, occurrences and refresh freshness. Adapt existing ground
   consumers without weakening serialized image validation.
4. Refresh bounds before exclusive collision phase; run existing resident mesh
   queries, triangle ripping, Sk8 contact generation and role-qualified buffer.
5. Replace managed frame's empty Contact begin/seal with actual ground append,
   retaining current same-phase real Joint/Drive Build and Horus/integration.

Suggested bounded additions: dynamic_assembly_skater_pool.rs and
skateboard_collision_publication.rs, plus targeted shared Create/Update/ground
interfaces. Inspect current call sites before committing this design.

The mandatory numerical audit is now complete for the scoped consumed XYZ
path below. This does not permit canonical W throughout all source subsystems
or a raw packed-W parity claim.
Existing contact output already uses sealed observations and checked inert
slot+1 tokens, not native addresses; do not add a redundant addressing layer.

Tests must establish19 ordered occurrences, seven unchanged body identities,
actual group/material/flags/culling, board-before-hook IDs, foreign/stale refusal,
source refresh each frame, real resident-terrain contacts and continued solving
after live spawn/integration. Full player/input/tricks, flags3 counterpart/pairs,
native per-tick traces and presentation remain separate unfinished requirements.

## Implemented connected component and independent audit

`skateboard_collision_publication.rs` is a child of the actual assembly session,
reading private attached body handles. It retains the exact template/definition
Arcs, body-pool authority and seven checked allocation occurrences. Create
`0x82e0c850` emits wheel occurrences0..3, trucks4..5, then every retained deck
child. Shared volume Arcs do not deduplicate occurrences. The source cap is26
emissions, not a universal19-child requirement; the shipped five-subdivision
deck contributes13 children. Original volume data, flags, material and surface,
padding bits3d4ccccd, distinct Assembly group4 and Part userdata4 are retained.
Create calculates and unions each primitive immediately. Update `0x82e0cf30`
changes only enabled/material/transform/group/frozen; its separate bounds job
`0x827cdf78` calculates every primitive before union and Header publication.
Disabled, static and frozen Parts are not filtered from these bounds traversals.
Header static count and all-frozen reduction use actual live state bits.

Refresh invalidates both stamps first, refuses foreign pools, changed definition
owners or allocation occurrences, then publishes current volume/bounds stamps.
Host failures may retain a completed write prefix but no fresh bounds stamp.
This is explicit checked-host failure policy, not native allocator semantics.
Same-frame mutation is prevented by refreshing immediately before acquiring the
exclusive collision phase; a frame stamp alone is not a general pose version.
Multi-member all-Update/all-bounds/all-publish scheduling is still unimplemented
for authored owners, and this component does not claim SkaterPool membership.

The ground consumers now accept a closed private publication enum: original
prepared serialized owner plus its genuine packed image, or the real authored
board owner. No arbitrary records, fake arena IDs, serialized inertia addresses
or invented Xenon pointers can establish authored publication authority. The
serialized branch retains its image/epoch/pool checks unchanged. Authored
headers retain source early skips before authority/freshness checks, then feed
the same Extra, primitive bounds, resident-mesh query, triangle ripping, narrow
phase, contact composition and checked body-role bindings as serialized inputs.

Independent live source audit covers Create/Update, Part transforms, swept
CalculatePartBBox `0x82e0b760..0x82e0b96c`, GP generation, ground AABB/velocity
padding and overlap. Run overlap `0x827cd42c` and BoxTest `0x827cda08` discard W
before their all-equal reduction. Triangle GP `0x82ad97c4..0x82ad97d8` explicitly
canonicalizes parent affine W. Capsule/cylinder fourth-lane arithmetic remains
lane-local. Existing body semantic XYZ plus canonical affine W and zero link W
therefore preserve the consumed XYZ/control path here. Original definition and
inverse-transform W are retained; actual inverse mass, state, kine/cooldown and
body roles are never substituted. Genuine packed-image processing is unchanged.
Exceptional Xenon arithmetic and raw GP/Contact W are outside this confidence.

`skateboard_ground_frame.rs` refreshes the real publication inside the retained
manager owner after StartFrame/hook exclusion/drive normalization, before the
exclusive phase. Original resident-ground contacts replace the empty-contact
begin/seal path through the existing validated Contact append and Build. The
same phase then feeds existing Joint/Drive Build, ten-island serial Horus and
integration. No alternative solver, narrow phase or caller-made contact rows.

### Validation and limits

The original mounted board now reaches original resident Rob's Park mesh0
(asset a114c7540da18d0e, mesh signature a463b42229b8fb97) through four retained
frames. Controlled placement uses the actual wheel0 radius near terrain point
[588.8617,-54.431,996.5048], not a recovered player spawn location. Contact
capacity256, ground group0/no pipeline table, multiple writes, dt3c888889 and
fifty iterations are explicit component inputs, not live cvar/World services.
The actual source bend-threshold argument is3f7fbe77; it is not a new runtime
tuning default. Current flags3 pair construction is not bypassed or claimed.

Actual C/J/D counts are6/6/3,6/6/3,6/6/3,4/6/3:22 ground contacts through the
shared solver. Each frame processes one assembly, one resident mesh, one range,
one triangle and one intersection run, with no unavailable run. All nine body
identities/occurrences, seven Active/two Static roles, constraint topology,
definition/geometry/culling ownership and immutable record fields persist.
Static hook state remains unchanged, board state stays finite and actual RF
clears. Pose changes are observed but not isolated as a contact-only effect;
gravity and existing constraints also act. Teardown and retained owner checks
still pass. This is four-frame component coverage, not long-term stability.

Mounted checks verify19 source-ordered occurrences, actual metadata/live pose,
foreign pool/definition refresh refusal and stale-stamp predicate behavior.
Direct authored stale-consumer rejection remains additional coverage debt.
Five new ordinary tests cover real kine/cool retention, actual static/frozen
header bits, disabled/static/frozen bounds inclusion, and five primitive types
under five poisoned W patterns. Bbox/GP XYZ remain unchanged while a real XYZ
velocity change does alter swept bounds. These are synthetic discriminators,
not original geometry fixtures or native numerical oracles. Independent
remapped_source_audit source/production/test review: PASS.

Next: source-owned SkaterPool membership consuming the shared manager index
before HookPool; real skater counterpart/player construction and pair producer;
whole-pool scheduling and actual cvar/configuration lifecycle. Current direct
board attachment means the hook still has world index0 in this component. No
fake index burn or phantom skater substitutes for the missing registration.

The next bounded owner/source plan is now recorded in
`skater-pool-owner-source.md`, including independently checked Insert/Remove,
nonparallel refresh chronology and full-member ground iteration. It is evidence
for the next implementation, not a claim that membership is already connected.

Later update: that owner is now implemented. Managed board runs insert the
actual detached board before HookPool, yielding real shared indices0/1 in a
pristine component manager. Pool-owned publications feed the same ground job
after all-member nonparallel refresh. The earlier direct-hook-index0 and
single-publication scheduling limits above describe the2384 checkpoint, not
the current2392 owner. Full Island/player/counterpart/pairs remain open. Direct
stale-ground refusal is now tested with real owned members; see the owner packet.
