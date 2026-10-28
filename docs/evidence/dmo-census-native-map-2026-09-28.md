# DMO census and model native map (2026-09-28)

Historical research map for the DMO census, ModelMan and main bank. All
addresses are the debug `sk82_na_m.xex` in IDA. The original research-only
status is superseded: implementations now exist in living_world_dmo_models.rs,
living_world_dmo_objects.rs and selected_living_world_dmo.rs, using the retained
bank and locator owners. Read dmo-grab-runtime-2026-09-28.md for
the connected flow; do not implement another census or model manager from
this older map. Compilation and gameplay of this continuation remain unverified.

## Schedule
- Manager::Update1 0x82727ee0: EvalCensusTargets, ped/veh/DMO ResetCollisionFlags,
  ModelMan::Update, Waypoint/RAIN/CensusMan::Update, hand props, then if local
  skater (vtable+4 on +48 iface true and !(skater+600 bit29)):
  EvaluateCensus(GetSimulationCenterPosition, GetSimulationCenterVelocity, false);
  then each entity census Update (ped, veh, prop, DMO); ClearSpatialLookups;
  UpdateSpatialLookups.
- Initial spawn: Flow::StateLoading::CharacterPrepareState 0x82760058 (enter,
  loading to game, both stream focuses): Random::msLocked=0, ClearSpatialLookups,
  EvaluateCensus(stream focus, zero velocity, true), UpdateSpatialLookups,
  msLocked=1 (skipped if challenge LW manager SkipInitialSpawn).
- EvaluateCensus 0x82727a38: assert pos xyz; needs byte+0xa8 bit7
  (mAllowLivingWorld); if !(+0xa9 bit7) CensusMan::Update first. cull=kEnableCull;
  spawn=kEnableSpawn && +0xa8 bit6 (mbEnableSpawner) && (FE in game || initial)
  && debugmanager+121 && !ghost && !IsNetworkGame. One-per-frame mode
  (kEvaluateCensusForOneComponentPerFrame && !initial): cull all components if
  kEvaluateCullForAllComponentsPerFrame else current index; CensusMan::Update;
  spawn current index; index=(index+1)%4. Else: cull all 4, Update, spawn all 4.
  eLWComponent: 0 peds, 1 vehicles, 2 DMOs, 3 props. Ctor index 0.
- CensusMan ctor 0x827208d0: +0xa8 = 0xFE (all enable bits, mFirstActorCreated 0),
  +0xa9 &= 0x7f; keys pedestrians/vehicles/props/dynamicobjects (StringToKey).
  Init 0x82715908: bit7 = Simulation::IsLWSpawnEnabled (!network && !ghost).
- Bitfield +0xa8 from LSB: FirstActorCreated 1, PropSpawn 2, DynamicObjectSpawn 4,
  VehicleSpawn 8, PropDrivenPedSpawn 0x10, PedSpawn 0x20, Spawner 0x40,
  AllowLivingWorld 0x80. +0xa9 bit7 mbHasSampledFEToggleSettings.
- CensusMan::Update 0x82702780: reserve countdown, FE sliders (free skate only),
  +0xa9 |= 0x80, each entity census man Update.

## DMO census
- EvaluateDynamicObjectCensus 0x82726930: livingworld_census_ranges
  (class 0xc779d2e21a4f4f35, key +0x80 "dynamicobjects" 0x20f04d295272ee46,
  layout 0x28: two tLWCensusCircle {NoSpawn, Spawn, Cull, Shift, Speed}); vault:
  both {0, 60, 75, 0, 0}. speed = |vel| (dot3 + rsqrt, zero select) * 3.6f.
  EvaluateCensusCircle 0x826f46f0: if min.Speed >= max.Speed copy min; else
  t = upper(lower(upper(lower(s-minS,0),|s-minS|)/(maxS-minS),0),1) (fsel chain),
  out = fma(max-min, t, min) per field and Speed. center = fma(NormalizeSafe(vel),
  splat Shift, pos). Cull: EvaluativeCullDynamicObjects(center, Cull, true).
  Spawn if bDoSpawn && kEnableSpawn_DynamicObjects && +0xa8 bit2:
  EvaluativeSpawnDynamicObjects(center, NoSpawn, Spawn, Cull, initial).
- Cull 0x82724090: every DMO entity ID; object (iface-140); coreData+240 bit7;
  d=center-pos, y zeroed, dot3 - cull^2 (fmuls); cull-outside: request
  RequestExplicitRecursiveCull when !(d2-r2 < 0).
- Spawn 0x8271c690: budget kMAXDYNAMICOBJECTS (100 if initial). xz(center) valid
  (line 1448). GetPassiveLocatorsWithinRange(xz, Spawn, 256). If !initial:
  results with dist < NoSpawn -> kNullAssetID. Candidates (fixed 256): non-null
  IDs in order. Expansion loop over candidates (end re-read, vector grows):
  loc=GetLocator; model=ModelMan::GetModel(loc.model +0x88); bbox=model
  GetBBoxTransform(loc.transform) (w = fma chain of (min+max)*0.5); r1=|dims|
  (rsqrt); close = GetPassiveLocatorsWithinRange(xz(bbox.w),
  kCloseByLocatorSearchRadius, 50); for each close id2 while candidates<256:
  skip self (+0x80); loc2; |loc2.pos-censusCenter|^2 (3D dot3) - cull^2 > 0 skip;
  skip if already candidate; model2; bbox2; r2=|dims2|; d=(bbox2.w-bbox1.w) xz,
  dot3; if dist2 - ((r1+r2)*0.5)^2 < 0 push id2.
- Main loop per candidate while budget>0: loc (bit7, user 0); M=loc.transform;
  model id != kNullAssetID; LivingMan+0xF4B0 vfunc+8 true; dmoMan=LivingMan+0x5780;
  GeneratePriorityIndexForLocator(locID,&prio); if prio && kMAXDYNAMICOBJECTS <=
  entityMan(+0x5790)->+0x10 count: IsLocatorHigherPriority... ? lowest=
  GetLowestPriorityObjectID : reject. BuildModelOverlapIDList(dmo, model, M,
  -0.05): each overlap obj vfunc+0x5c fill prio; reject if (prio.+8+prio.+4) <=
  (o.+8+o.+4)+0.001; else cull overlaps (iface+0x8c vfunc+0xc). Vehicle overlaps
  (pad 0) RequestExplicitCull. Cull lowest; LivingMan+0x5798 vfunc+0x10;
  SpawnDynamicObject(key "dynamicobjects", &locID, &model, &M, initial, 1, 0,
  stack 1) -> budget-- on success.

## Models
- ModelMan::AddDMOModel 0x82ce9c60 (kEnableDMOModelLoading): key =
  GetSimModelIDFromInstanceID 0x82d2ee20 (kEnableDMOBankIDLookup: tInstance
  +0x68 m_uiGuidLocal); new LWDynamicObjectModel (224) Create(inventory, id, 0);
  map[key]; ++count. GetModel 0x82ce7970 map lookup.
- Create 0x82d39f90: tInstance via GetRegAsset (assert 49); FindCharacteristic;
  CreateFromMesh 0x82d337b0 (needs RModel && CModel && numMeshes != 0; bbox from
  RModel +0/+0x10; dims=max-min; -1 meshes: CModel mesh ptr -> serialized
  Assembly, its +4 definition if >= 0x3e8 else sphere(1.0) def; ApplyAttribulator
  Offsets) else LWModel::Create 0x82e51010 (always 1; livingworld_models dims or
  (1,1,1)); InitializeHotpoints.
- Model vtable (RefCounter): +0x10 GetDimensions (+0x20), +0x14 bbox min (+0x30),
  +0x18 bbox max (+0x40), +0x2c GetBBoxTransform.
- Main bank: cDMOBankManager::LoadMainDMOBank 0x82e0e628 from
  GameModule::SecondaryLoad01; LoadHull("DMO", flags 8) -> userdata 0x0a.
  Evidence dmo-bank-processed-asset.md: 125 arenas, last d3096f13cf1c0aab holds
  1 material + 93 kind-11; all m_iNumMeshes == -1.

## Existing port pieces to audit before reuse
- world.rs: DynamicObjectPriorityIndex, plan_dynamic_object_admission,
  dynamic_object_priority_index (GeneratePriorityIndexForLocator), entity
  allocator/lifetime, reference cull order (host math: audit vs IDA).
- app main.rs RuntimeDmoCensus/RuntimeDmoCatalog: approximation-grade
  (nearest-first candidates) - do not reuse its selection.
- Core: game_stream::ManifestOnlyStream + mounted_stream_backend::MountedStreamBackend
  load the DMO manifest; dmo_bank_processed_asset builder sessions build arenas.
