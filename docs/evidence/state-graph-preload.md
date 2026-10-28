# Living-world StateGraph preload

## Scope and confidence

`LivingWorld::Manager::PreLoadStateGraphs` is reconstructed at **exact**
confidence for its cVar access pattern, generated-delegate writes, ordered
StateGraph/BehaviourGraph collaborator calls, returned behaviour ownership,
and final prop-initialization call. Cache-front ownership and compiled wire
deserialization are now reconstructed separately in
`graph-cache-binary-payloads.md`; dynamic controller construction, generated
controller bodies, XML parsing, and `InitPropData` internals remain open.

The principal executable authorities are:

- `LivingWorld::Manager::PreLoadStateGraphs`, `0x82724280..0x82725538`;
- `AbstractHierarchicalController::EnableGeneratedStateGraph`, `0x828ec858`;
- `StateGraph::Cache::Load`, `0x82cf8af0`;
- `BehaviourGraph::BehaviourMan::Load`, `0x82cf6ee0`;
- `BehaviourGraph::Cache::Load`, `0x82cf6ca0`;
- `TransferableBehaviour::Process(StateGraph::Cache&)`, `0x82cf0a40`;
- `LivingWorld::Manager::InitPropData`, `0x8271d610`.

## Recovered control flow

The initial `kbUseGeneratedStateGraphForLW` target byte chooses one complete
branch. The target pointer used for that branch read is retained for the first
generated item and its byte is read again. Items 2 through 18 each re-resolve
the target pointer, read its current raw byte, and pass that byte without
boolean normalization. A later zero changes delegate bytes but does not switch
the already chosen branch. The false branch calls `StateGraph::Cache::Load`
for all 18 paths with `flushCache=false`; it does not clear any previously
enabled generated delegates.

After either state loop, the manager loads ten behaviour descriptors in fixed
order. Each returned intrusive handle is released before the next descriptor.
`InitPropData` follows all ten. The readable core models this native void
suffix as infallible; it does not invent an error-driven early return.

The exact 18 state and ten descriptor strings live in
`living_world_state_graph_preload.rs` and are frozen by full-order tests. Paths
reach this boundary without a `data\` prefix and with forward slashes.

## Generated delegates

`EnableGeneratedStateGraph` performs 20 independent, ordered, case-sensitive
`strstr` checks. It stores the supplied low byte for every match and does not
return after the first. A null name stores all 20. The checked basenames are:

```text
ActionGraph_OnBoard.xml
MotionGraph_OnBoard.xml
Vehicle.xml
Big_Black.xml
MotionGraph_Pedestrian.xml
Pedestrian.xml
actortracker_spectate.xml
conversation.xml
lookat.xml
sit.xml
spectate.xml
useatm.xml
usenewspaperbox.xml
usetrashbin.xml
usevendingmachine.xml
usewaterfountain.xml
marquee.xml
motiongraph_marquee.xml
Motiongraph_Photographer.xml
Photographer.xml
```

Independent tests preserve four observable overlap sequences:

- `MotionGraph_Pedestrian.xml` writes both the motion and generic pedestrian
  delegates;
- `actortracker_spectate.xml` also writes `spectate.xml`;
- `motiongraph_marquee.xml` writes `marquee.xml` first, then its specific
  delegate;
- `Motiongraph_Photographer.xml` also writes `Photographer.xml`.

The Rust string helper's exact claim is deliberately limited to the supplied
NUL-free UTF-8/ASCII caller paths. Native `strstr` additionally accepts
arbitrary C-byte strings.

## Backend and corpus facts

The StateGraph cache conditionally adds `data\`, converts `/` to `\`, and keys
only by `GetHashValue32(normalized_path)`. The BehaviourGraph cache always adds
`data\`. Both caches return a hash hit immediately when `flushCache=false`,
replace on a true flush, and negative-cache a null parse result. Hash
collisions therefore alias; no source string is retained for disambiguation.

Both parsers replace the final extension with `.stategraph`, synchronously try
the compiled sibling first with flags zero/alignment `0x10`, and fall back to
XML on a compiled miss. State XML uses flags zero; Behaviour XML uses flags
one. The path/cache front and compiled wire readers now implement these facts;
the actual loader call sequence and XML fallback remain unwired collaborators.

All 29 XML assets and all 29 compiled siblings—including the camera graph,
the 18 state paths, and ten descriptors—resolve from archive index 11,
`data\big\miscload.big`. `MountSet` can read and RefPack-expand them. Presence
does not prove semantic deserialization, and a byte-only mounted backend must
not report graph-load completion.

On the ordinary default-true path, the ten compiled Behaviour descriptors are
read and processed. Their `file=` attributes name the matching pedestrian
plugin state paths. `TransferableBehaviour::Process` creates a hierarchical
controller; enabled delegates normally select generated plugin controllers,
while unavailable/disabled delegates fall back through dynamic creation to the
shared StateGraph cache.

## Remaining boundary

There is no application implementation of graph semantics yet. The hash-only
cache front and both compiled payload readers are present and pass the shipped
corpus. Required next pieces are XML fallback, a shared StateGraph semantic
cache used by camera and behaviour processing, generated controller bodies or
faithful dynamic fallback, and actor-tracker behavior inside `InitPropData`.
Resolving structured records alone is not semantic completion.

## Evidence hashes

- extractor `A1C650885D811E8FD91B3B7481797238BDF124C123D1EEC0EC41EB9C44E7FC6C`;
- StateGraph backend packet `5D6EF39D4EB9566C743176BC8089DA3D47C675A3B2FE3590CFA912E75CD24073`;
- transferable-vtable extractor `93460340859D5AB0AA34D919B52CC68697EF354EF5A2523183731653BD5C3429`;
- transferable-vtable packet `5CCA9DBF98CA2C524DFAA4E9329727E4E7A25527EC117A0CE18EA4EC5DC87AEA`;
- physical asset probe source `4D4D3F7FFF42E3D61B863F31DEB83C63F4188CFB533F0B0587EFCB9D87CFDC59`;
- physical asset probe output `0D8BC9D242262B3D9C7F630BEB4108030ED85F408470BBE5F2F35660909CE516`;
- supplied `miscload.big` `0512157AAADD88A6E1992509944D0AAFF49EFAD2DB7BDD8C32AA4774A92F7F13`.
