# Selected LivingWorld dynamic grind provider

This is the selected EntityMan/query-owner closure only. The historic
LivingWorldPhysicalServices file was unregistered when this work began; its
comment about a workspace occurrence was not proof of a live owner. Root owns
mounting a single retained LivingWorldGrindServices occurrence. The selected
projection now lives separately in living_world_grind_services.rs; my temporary
fields/methods were removed from the old physical-services draft. A future full
Manager must retain this same Arc, not create replacement entity stores. No complete LivingWorld Manager or
Simulation.Init/Reset execution is claimed, and the historical parked_submanagers remains
Unrecovered. Existing physics IK empty collections are DIFFERENT native owners.

## Owner proof

Manager.QueryGrindSplinesWithBoundingBox82703ff0 invokes DMO then vehicle with the
same result even when static count is40. The submanager provider base is305c;
both methods load provider-304c, i.e. submanager+10 = component.mpEntityMan.
The queried map is EntityMan+4. No Physics assembly membership substitutes here.

Vehicle CreateEntityMan82d27908 allocates24c, then ctor82d26b48 creates the
capacity24 fixed hash map and null KD root. DMO CreateEntityMan82d37778 allocates
454, then ctor82d366d0 creates capacity50 map/nullroot. ComponentInit82d2af28
publishes these after its virtual CreateEntityMan, separately from its other
memory/execution/factory/census children. EntityMan.Init82d26bc0/82d36748
initializes MemStore element80/count24 or50 and adds no entity.

The selected host owner allocates map storage and retains a distinct constructor
state before MemStore Init. MemStore retains uninitialized80byte blocks and the
existing source free-list owner. Membership payload decoding/publication is
unimplemented; no production insertion API is exposed. Generic absence is not
interpreted as empty. Unrelated ComponentMan/Manager initialization stays pending.

## Exact empty query prefixes

DMO82d30fb0: read Feed target0fd4/value0fd8; false branches0fe0 toreturn. True
reads EntityMan0ff0 before center1030. Center is (maximum+minimum)*0.5 forALL4
lanes. Capture mapbegin/end before size. One size load1058, separate square105c;
compare captured iterators1074 and emptyreturn1078. MissingEntityMan must precede
late size failure. Nonempty produces explicit pending with computed bounds data.

Vehicle82d237a0: read EntityMan37b8, compute center380c, capture end3838 and
compare3854 BEFORE size load3858/square385c; branch on captured relation386c.
There is no feedtoggle. Never skip this invocation because the static result is
full or DMO feed isfalse. Empty owners preserve the existing result exactly.

## Live references

All callbacks are null (raw bool r8=0; floats r10=0). Float GUI1..100 does not
clamp runtime values. Defaults size15 =41700000.

|API index|Ref|Initializer|Static table|Section / label / title / default|
|---|---|---|---|---|
|67|830a6df8|82f8b5a8|82fd53ec|AI_LivingWorld / kMaxVechileObjectSize / Max Vechicle test size /15|
|68|830a7284|82f8c1b0|82fd5498|AI_DynamicObject / kFeedDMOGrindSplines / Feed DMO Grind Splines /true|
|69|830a75b8|82f8cff8|82fd5550|AI_LivingWorld / kMaxDynamicObjectSize / Max DMO test size /15|

## Validation boundary

Focused tests cover separate constructor/Init lifetime, repeated Init rejection,
missing cvar before missingowner, DMO-beforevehicle, disabled feed skipping DMO,
actual constructed empty maps preserving result, parkedstate remainingunknown,
and live default/relink/unclamped cvars. Root owns compilation, original kind6
corpus, full-result preservation and actual shared workspace bootstrap checks.
No Cargo was run by the implementing child agent.

## Selected initialization order correction

Fresh Manager.Init8271f6f0 raw f8bc invokes Vehicle component Init; f8d0 invokes
Prop; f8e4 invokes DMO. Selected bootstrap now performs Vehicle EntityMan
constructor plus its MemStore Init BEFORE DMO constructor plus its Init. This is
a selected subsequence, not execution of the intervening Prop or other component
constructors. Query order remains DMO then Vehicle. The initial draft's separate
DMO-first constructors and later Init methods were removed before integration.
`try_initialize_selected_entities` retains published prefixes on errors and
refuses replay; `selected_entities_initialized` is host admission evidence only,
not a manufactured native full-Manager completion field.

## Independent session integration review

The independent reviewer checked PhysicalGrindServices and the single retained
field in SimulationAssemblyConstraintSession. Static initialization, selected
Vehicle/DMO initialization, and provider installation are separate calls. Both
accessors return the existing Arcs, allowing future complete LivingWorld owners
to retain these same entity stores. Fresh raw8289f0e4..f0fc and828a6f24..6f3c
confirm native installation of Manager+4, or null for an absent Manager. The
host replay guard explicitly admits only the first selected installation.

Review found and root corrected an eager dynamic-owner lock: it originally ran
before count clearing, extent gating and static output writes. The retained
provider now acquires its mutex inside the actual dynamic query callback, after
those source stages. The null-provider path never acquires the LivingWorld owner
or samples its cvars. Re-review accepts this corrected ordering.

Two focused regression tests poison the actual retained LivingWorld mutex while
retaining a real arena-backed static spline. A degenerate query still clears
the prior count and returns false; a valid query publishes its static result
before the dynamic lock error. With the native provider still null, a valid
query succeeds despite the poisoned dynamic owner and unlinked dynamic cvars.
Tests reuse the existing source fixture rather than duplicating arena setup.
They are authored but await root's compilation and test gate; no Cargo was run
by the independent reviewer.
