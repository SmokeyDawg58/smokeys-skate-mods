# DMOs D1: LivingWorld locator manager and the live kind-31 pass (2026-09-28)

Scope (2026-09-27): DMOs must spawn at their proper places, cast dynamic
shadows like the skater, and be movable with RB exactly as in the real game.
This supersedes the 2026-09-08 DMO parking; its retained code
(DynamicPool candidates, source backend, query batches, DMO bank catalogs)
stays and is reused by later batches.

Status: source written and statically reviewed against IDA (debug
`sk82_na_m.xex`). rustfmt PASS on the new files and every own hunk; line
endings of every touched file unchanged (all LF). NOT compiled, NOT tested.

## Native map (what later batches follow)

- World collections register with userdata 0 (`cAssetStreamSystem::LoadComplete`
  0x824dc208). `SimThreadMsgMan::ReceiveRegCollectionMsg` 0x8277e988 runs
  `Simulation::RegArena` (plus `cLocationManager::RegArena`) over every arena
  only when the message activates, then ALWAYS runs `Simulation::RegDMOArena`
  0x828a0420 over every arena (0x8277ea3c branches straight to the DMO loop).
- `RegDMOArena`, per kind-31 `tDMOData` asset: with `kLoadDmoLocatorFromStream`
  (DMO, default true) userdata & 4 goes to `cChallengeDMOManager::AddChallengeDMOs`,
  otherwise `LWDynamicObjectMan::AddStreamedLocatorData(asset, userdata)`;
  userdata & 7 == 7 also calls `Dmo::SimManager::RegDmoLocator`. World DMOs
  therefore reach the LivingWorld dynamic-object manager.
- `ProcessDMOData` 0x82d5f4f0 publishes each kind-31 asset with
  `AddSimRegistryAsset(id, 0x1f, object)` (first full-ID winner, `AddRegAsset`
  0x82d64890) before the registration message.
- Census chain for D2: `Manager::Update1` 0x82727ee0, `CensusMan::EvaluateCensus`
  0x82727a38, `EvaluateDynamicObjectCensus` 0x82726930, `EvaluativeSpawnDynamicObjects`
  0x8271c690, `CensusMan::SpawnDynamicObject` 0x82726c08,
  `LWDynamicObjectCensusMan::Spawn` 0x82d31ba0, `DynamicObject::DynamicObject`
  0x82d3dc58. `SimUpdate1` 0x8289f410 brackets the skater animation with
  `BeginDMOUpdate`/`EndDMOUpdate`.
- Roadmap: D2 census; D3 models (`ModelMan::AddDMOModel`,
  `LWDynamicObjectModel::Create` from the DMO bank) and `DynamicObject` creation
  with `AddModelsToWorld`; D4 DMO physics in the frame; D5 presentation and CSM
  casters; D6 RB grab and move; D7 DMO grind splines.

## `LWDynamicObjectLocatorMan` (`living_world_dmo_locators.rs`)

- Construction: `LWDynamicObjectMan::Init` 0x82d3e030 (called on the
  `LWComponentMan` subobject, so +0x3aa8 there is `mpLocatorMan` +0x3aac)
  allocates 0x1c178 bytes, constructs at 0x82d3ac40 with
  `LWDynamicObjectMemoryMan` store 3 (`Init` 0x82d29dcc..dde0: 1024 x 0xa0,
  "DynamicObject Locators MemStore") and initialises the KD tree's own store
  (0x82d3e114..e124: 1024 x 0x50, "Entity KDTree MemStore"). Ported at the point
  the retained LivingWorld owner initialises its DMO entity store.
  `MemStore::RequestBlock` 0x82a6e450 returns null when exhausted.
- `AddStreamedLocatorData` 0x82d3d548: only with `kEnableDMOLocatorLoading`
  (AI_DynamicObject, default true); after a successful chunk the chunk ID is
  appended once (full 64-bit compare, 0x82d3d5b0..b4).
  `LWDynamicObjectMan::AddStreamedLocatorData` 0x82d3d6d0 passes (1, 1, 0).
- `AddLocatorsForStreamedLocatorChunk` 0x82d3be90: static = flags bit 4 |
  treatAsStatic, persistence = allow & !bit 5, priority = allow & !bit 6; a
  missing registry row asserts (line 209). Per 128-byte record: locator ID +0x60,
  model ID +0x70, both data transforms from the record matrix +0 through
  `tLocatorData::SetTransform` 0x82d29720 / `SetInitialStreamTransform`
  0x82d29a40 (NaN asserts lines 27/30), attachment time 0. With persistence an
  existing locator is not recreated; a missing one first takes
  `GetLocatorPersistenceData` 0x82d33de8. Created locators get flag byte +0x98 =
  `(old & 0x97) | static<<3 | priority<<5 | persistence<<6` (0x82d3c064..c08c,
  rlwimi decoded), then every created or kept locator gets bit 7.
- `CreateLocator` 0x82d3ad38: `kEnableTheHackToRemoveBadLocators`
  (DMO_Debug_Locators, default TRUE) rejects five full IDs (0x82d3ad80..ae08,
  `insrdi` pairs); asserts lines 388 (pos xyz), 389 (quaternion, 4 lanes),
  390 (`GetTransform` rows). A new block gets flags `(old & 7) | 0xe0`
  (0x82d3af7c..af8c) and user object 0; an attached locator (`mUserObjectID`)
  returns untouched; otherwise transform, initial stream transform
  (`DynamicObjectLocator::SetInitialStreamTransform` 0x82d29608, line 24),
  model and attachment time are rewritten, a new ID enters `mLocatorMap`, an
  existing one leaves the KD tree at its OLD XZ (captured before `SetTransform`),
  and the new XZ (`VectorPermuteConstant<0,6,0,0>` of the w axis) is inserted.
  The kNullAssetID/kInvalidAssetID placeholders (hashes of "Null Asset ID" /
  "InvalidStringHash") are always overwritten before use and are not modelled.
- Locator transform math: `QuatFromMat` in `SetTransform` is the same unnormalised
  candidate graph as `try_simulation_quaternion_candidate` (sign vectors from
  the dynamic initializers 0x82f7eec8/ef28/ef58: x [+,+,-,-], y [+,-,+,-],
  z [+,-,-,+]); `GetTransform` 0x82d29990 is the `gSqrt2s` (0x3fb504f3)
  expansion of `drive_basis_host`, including the noncanonical axis W lanes (an
  identity locator comes back with x-axis W = 1). The initial-stream twins
  (0x82d29a40, 0x82d29cb0) differ only in register allocation.
- KD tree `KDTree<cAssetID, Vector2, 2>`: `Insert` 0x82d2f000 with
  `KDNode::Insert` 0x82d2d3f0 (ties and NaN go left, dims alternate from 0);
  `KDNode::FindNear` 0x82d39670 iterative walk (left while `coord >= p - r`,
  right while `p + r > coord` and only after returning from the left child,
  stops at `min(n, 256)` held nodes, distance `x*rsqrt(x)` with the two fused
  Newton rounds and the zero select); `KDTree::FindNear` 0x82d3a3b8 asserts
  n <= 256 (kdtree.h 723), sorts through a multiset whose `>=`-right insertion
  keeps walk order for equal distances; `KDTree::Remove` 0x82d3a180 searches
  0.1 with 4 neighbours, matches |d| <= FLT_EPSILON per lane plus the full ID,
  derives the node dimension from its depth; `KDNode::Remove` 0x82d2d550 (lone
  left subtree moves right, right-subtree minimum copied down recursively) and
  `FindMin` 0x827eab38 (later equal minima win). IDA's VMX128 VA register
  display swap was decoded raw for every vector operand.
- `GetPassiveLocatorsWithinRange` 0x82d3ad20 clamps to 0x400 before the tree.
- Not modelled: `mLocatorMap`/`mPersistentLocatorMap` EASTL iteration order
  (only find/insert here); NaN distances (refused as a typed error).

## Live wiring

- Cvars 827..829: kLoadDmoLocatorFromStream (DMO, true), kEnableDMOLocatorLoading
  (AI_DynamicObject, true), kEnableTheHackToRemoveBadLocators
  (DMO_Debug_Locators, true); accessors in `input_cvar_living_world_dmo.rs`.
- `SelectedTerrainStartup` now runs the DMO pass after the normal pass (initial
  collection and every streamed focus collection) through the existing
  `SimulationDmoArenaRegistrationAdapter` with a live policy (cvar per asset)
  and a dispatch into the retained LivingWorld owner. The dispatch publishes
  every arena's kind-31 rows first (first winner) and latches the first typed
  failure; challenge/registry/removal routes are explicit refusals (the selected
  session registers userdata 0 and never deregisters).
- The grind corpora mount terrain without LivingWorld Init and pass no DMO route.

## Remaining

Nothing spawns yet: the census consumes these locators next.

- The selected session never unloads collections, so the 1024-block locator
  store can fill where native `RemoveStreamedLocatorData` 0x82d3d618 would
  have freed unloaded chunks; locator removal must arrive with collection
  unloading.
- Census prerequisites found while mapping `EvaluativeSpawnDynamicObjects`
  0x8271c690: the LivingWorld `ModelMan` (`AddDMOModel` 0x82ce9c60,
  `LWDynamicObjectModel::Create` 0x82d39f90, bounds via `GetDimensions` +0x20
  and `GetBBoxTransform` 0x82e4cd60) fed by the main DMO bank (hull "DMO",
  `cPres_Global`, userdata 0x0a, 93 kind-11 models in the last of 125
  arenas), which the live session does not load yet.
