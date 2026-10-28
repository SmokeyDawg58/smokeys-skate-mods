# PROXY world rendering (2026-09-27)

Scope: the entire proxy rendering flow, so that as chunks load and unload around the
player every proxy renders accordingly, with every flow 1:1 against the IDA reference.

Status: source written and statically reviewed; rustfmt PASS on every new
hunk. NOT compiled, NOT tested, NOT run.

Reference: IDA on the debug `sk82_na_m.xex`. Data: `worldproxy.big`,
`worldbam.big`, `render.big` and the Vault (`db.big`) of the Nov-28-2008
build; the Jan-17-2009 Vault gives the same values.

## 1. Boot: load order and proxy map

`cProxyWorldManager::SyncLoadProxyWorldStream` 0x824df648 (called through
`cAssetStreamSystem` 0x824e0710 from `GameModule::OneOffLoad`, see
`proxy-world-boot-order-2026-09-23.md`):

- asserts the stream ID is invalid, `BuildStreamPath(kStreamType_Pres,
  "data\content\world\stream", "PROXY")`, `LoadStream`, spins `Update` until
  loaded (0x824df6b8..714);
- synchronously loads `<name>.xss` (0x824df72c..780); the row count is XSS+16
  (0x824df788); `mProxyCollectionID` is resized to it;
- rows start at +24 with stride 80: ID at +0, hull min at +16, hull max at +32
  (0x824df860..9e4). Centre = `(max - min) * 0.5 + min` (vmaddfp), then
  `x = (int)floor(cx + 0.5)`, `z = (int)floor(cz + 0.5)` and
  `tProxySort::mOrder = z / 10 + 10000 * x` (0x824df9d0);
- `merge_sort_buffer` with `ProxyCollectionOrderCompare` 0x824d6888
  (`lwz +8`, `cmpw`, `blt`: signed `a.mOrder < b.mOrder`): a stable ascending
  sort;
- per sorted row: `mProxyCollectionMap` insert (ID, 0) at 0x824dfa64,
  `cAssetStreamManager::LoadCollection` (WorldStream category, the load
  callback) at 0x824dfa78, `mProxyCollectionID[i] = ID` at 0x824dfa88;
- spins until `mCollectionsLoaded` reaches the count;
  `cLoadProxyWorldRequestCallback::OnComplete` 0x824d68a8 increments it.

`SyncActivateProxyWorld` 0x824d9ef8 asserts every load completed, then per
`mProxyCollectionID` entry, in order, sends `cMsgAssetPresRegCollection`
(kind 1280779474, userdata 0, activate 1) and a callback message (kind
50695555) carrying a `cProxyWorldActCallback`, and spins until
`mCollectionsActivated` reaches the count (`OnComplete` 0x824d7d38 increments
it atomically).

## 2. Mapping: `AddHighCollection`

`cStreamFile::Update` 0x824e0ba8, state 2 (the stream's XSS loaded), for every
row: skip when the byte at +8 (LOD) is nonzero (0x824e0d64) or
`cAssetStreamSystem::IsCollectionValid` is true (0x824e0d84); record the
octree info; when `mStreamType` is 0 (Presentation), call
`cProxyWorldManager::AddHighCollection` at 0x824e11c8 with the collection ID
and `floor(cx + 0.5)`, `floor(cz + 0.5)` of the same hull centre.

`AddHighCollection` 0x824dc678: `sprintf(buf, "cPres_%d_%d_high_proxy", x, z)`
(r5, r6 untouched), `rw::RwHash64String(buf, RWHASH64_INIT)`, `find` in
`mProxyCollectionMap` (this+0x3890); only on a hit `DoInsertValue` of
(high ID, proxy hash) into `mProxyInfoMap` (this+0x44). The port's
`world_stream_adapter` already emits these calls with the same rounding
(`processed_position_and_order`).

`GetProxyCollection` 0x824daef8: the `mProxyInfoMap` value, else
`kInvalidCollectionID`.

## 3. Swap: high chunk registration hides its proxy

- `cAssetStreamSystem::LoadComplete` 0x824dc208: looks up the collection's
  octree info (asserting it exists), increments its loaded count, and only
  when it is not yet registered (+117) marks it and calls
  `NotifyCollectionReg(collection, GetProxyCollection(collection))`
  (0x824dc3b0, or 0x824dc3cc without a proxy).
- `NotifyCollectionReg` 0x82d5c920 -> `NotifyCollectionSimReg`, then
  `NotifyCollectionPresReg` 0x82d5c680: `UpCollectionLoadedCount`, stream type
  lookup, then either the deferred row (when `kbEnableCollectionRegDefer`, the
  manager's defer word, not `forceImmediateReg` and a stream size above
  0x7D000) or, directly: register the high collection (kind 1280779474,
  activate 1, 0x82d5c84c), deregister the proxy when valid (kind 1096347160,
  0x82d5c888), then the callback (0x82d5c8b4).
- Deferred rows: `UpdatePresCollectionDefer` 0x82d5b5f0 (from
  `GameModule::Render` 0x8273216c) sends `cMsgAssetPresRegArena` for the
  active row's arenas a few per frame, and only after the last one sends the
  collection registration (activate 0), the proxy deregistration and the
  callback. A deferred chunk therefore never leaves a hole.
- `UnloadComplete` 0x824dc3e8: decrements, and when registered clears it and
  calls `NotifyCollectionDReg` (0x824dc5b8 / 0x824dc5d0).
  `NotifyCollectionPresDReg` 0x82d580d0 is never deferred: deregister the
  high collection (0x82d58124), register the proxy with activate 1 when valid
  (0x82d58168), then the callback (0x82d58194).

## 4. Presentation side

- `PresThreadMsgMan::ReceiveRegCollectionMsg` 0x82780958 returns unless
  `fActivate`; walks the collection's container registry arenas;
  `cAssetRefCounter::AddAsset` (0x82780a3c) and, on first use only,
  `Presentation::RegArena` (0x82780ab0).
- `ReceiveDRegCollectionMsg` 0x8277fff8: `RemAsset` per arena, and on last
  use `Presentation::DRegArena` (0x82780128).
- `RegArena` 0x827f1430 dispatches processed kinds: 0 `AddTexture`, 2
  `AddMaterial`, 10 `UnlockableLocationMan::AddEnviroInstance` 0x828b02d8 ->
  0x828b0068 (unless its unlock context holds it back) -> vtable `AddToWorld`
  -> `PresUnlockableLocationMan::AddToWorld` 0x827e5360 ->
  `WorldPresentation::AddRenderInstance` 0x827e4f00. `DRegArena` 0x827f1680
  mirrors it (`RemFromWorld` 0x827dfdd0 -> `RemRenderInstance` 0x827df4c0).
- `AddRenderInstance` 0x827e510c..5128: the first mesh context's draw-order
  byte (`+4 -> +0x6C -> +0xC`) compared with `m_iProxyOrderGuid` (+0x1D0)
  selects `AddEntityToRenderViews(entity, eSceneView)` for a match and
  `eRenderViewAll` otherwise. The `WorldPresentation` constructor
  (0x827e9d80..9e18) adds views 1 (`eSceneView`) and 4 (`eWorldShadowView`)
  and sets `m_iProxyOrderGuid` to field 0 of `Attrib::Gen::pass`
  0xC9480BAF95B5238F. `RenderPresentation::AddEntityToRenderViews` 0x827f2998
  adds to every view for type 0, else only to views of that type. Proxies
  are in the scene view only; no world-shadow casting.

## 5. Data census (read-only scratch probes)

- `PROXY_Pres.xss`: 147 rows, all LOD 0. Every row's own hull centre names
  its ID (147/147). First sorted row: 0xb2ad17415ee8147a =
  `RwHash64String("cPres_-500_-100_high_proxy")`, key -5000010.
- `BAM_Pres.xss`: 284 LOD-0 rows. 147 map to a proxy, 137 have none, and no
  proxy is claimed twice.
- XSF (`worldproxy.big`): 148 collections, the 147 spatial ones and
  `cPres_Global_proxy` (93 single-texture arenas). A spatial collection:
  one main arena (1 model, 1-6 optimeshes, 1 material object, 1 instance
  list with one Environment instance) and 0-3 texture arenas; 35 texture
  arena IDs appear in two collections.
- Materials: 482, all `AttribulatorMaterialName = proxyworld.default`, channels
  `Name`, `AttribulatorMaterialName`, `diffuse` (a `proxyquadrantN` texture).
- Vertex declarations (482): POSITION FLOAT3 at 0, TEXCOORD0 SHORT2N at 12.
  The port decodes SHORT2N to v/32767 floats (`render.rs`), so `upload_mesh`
  gives UV0. Island draws: 1121, all primitive type 4 (triangle list).
- Vault `material_proxyworld/default`: `effect` -> `"proxyworld"`;
  `DrawOrder` -> RefSpec `pass`/0xC9480BAF95B5238F whose layout field 0 is 6
  (so `m_iProxyOrderGuid` = 6 = the proxy draw order); `m_params` has 3
  elements, `[0] = (0, 0.4, 0, 0)`.
- `evidence/render-queue.md` said `proxyworld.default` (and four others)
  resolves to `default`; the Vault shows real effect strings for all five
  (correction appended there).

## 6. `proxyworld.fx` (shipped microcode)

`proxyworld.xml`: technique `default`, one pass, `CULLMODE FRONT`, no sampler
block. `tools/xenos_disasm.py` on the shipped programs:

- `proxyworld_defaultVS.vpo`: oPos = four dp4 of the fetched position with
  c0-c3 (`g_matVP`, world space, no instance matrix); UV0 passes through;
  `r0 = P - c4` (`g_vViewPos`), dp3, sqrt, `mad_sat` with c5.x/c5.y
  (`g_FogK1`), log/`mulsc` c5.z/exp (pow), fog rgb = f1 * c6.xyz
  (`g_FogColour`), fog a = 1 + c6.w * f1.
- `proxyworld_defaultPS.fpo`: texture fetch; rgb squared, a kept; all four
  lanes x c3.y (`m_params[0].y`); `r0 = col * fog.a + fog`; tonemap with c2.x
  (`g_envattributes[2].x`) and literals 0.75/0.5/0.25/1; sqrt of every lane
  to oC0.
- `.updb` source (`proxyworld.fx`) agrees: `GetDiffuseTextureValue`,
  `PS_Fog_Bloom_Tone(outcol * m_params[0].y, In.Fog, g_envattributes[2].x)`.
  `g_proxyClipXZ` is declared but unused.
- Sampler: no manifest block, so `cFxParser::EndElement("Pass")`
  0x82e1fb64..fbc4 binds the slot's reset `cSamplerStateDefinition`
  (0x82e1fd70..fdac, fields from `cSamplerStateHandle::SetState` 0x82e176e0):
  WRAP u/v/w, bias 0, max level 0, min level 13, anisotropy 1 (enum 0),
  trilinear immediate, LINEAR mag/min, POINT mip.

## 7. Port

- `crates/skate2-core/src/streaming/proxy_world_manager.rs` (new):
  `proxy_sort_order`, `ProxyWorldManager` (`mProxyCollectionID`, proxy map,
  `mProxyInfoMap`, `AddHighCollection`, `GetProxyCollection`) and
  `ProxyWorld`: activation registers every proxy in order; a high
  registration deregisters its mapped proxy; its deregistration registers
  it again; arena counts use `ArenaRegistrationRefCounter`
  (`cAssetRefCounter`). Seven unit tests, one anchored on the shipped
  first-row hash.
- `crates/skate2-core/src/rendering/presentation.rs`: `PresentationScene.proxies`
  (`Arc`, shared, never edited), chained after `loose` in
  `resident_collections()` / `resident_collection()`; excluded from
  `resident_collections_mut()`.
- `apps/skate2-pc/src/proxy_world.rs` (new): the shared owner. The manager is
  read from `PROXY_Pres.xss` (`StreamPaths::derive`, the `BuildStreamPath`
  reconstruction) before the session boots; `ProxyHighCollections` is the
  world stream's `WorldStreamEffects` (it keeps the old log line) and feeds
  `AddHighCollection`; a registration fault latches and stops the city on
  the next revision read.
- `apps/skate2-pc/src/proxy_texture_world.rs`: loads the collections in the
  manager's order (textures as before) and turns each spatial collection's
  non-texture arenas into a Presentation collection (`own_presentation_arena`,
  Attribulator bindings) with its container arena list.
- `apps/skate2-pc/src/city_streaming.rs`: `activate_proxy_world` (load,
  activate, register the initial high chunks); each generation applies its
  executed unloads, then its loads (the queue drains unloads first), and the
  scene takes the registered proxies. A revision change without a focus
  change rebuilds the draw list.
- `apps/skate2-pc/src/render_inspection.rs`: initial activation and draw list;
  `proxyworld` binds `i_diffuse` and `m_params[0]`.
- `apps/skate2-pc/src/gpu_renderer/environment_shading.rs`:
  `EnvironmentEffect::ProxyWorld`, its layout, the pass-reset sampler, the
  pipeline (CULLMODE FRONT, depth write, no blend, all channels, no TEXCOORD1
  stream) and the WGSL of section 6 through the shared `world_clip`,
  `compute_fog_factor` and `fog_bloom_tone`.
- `apps/skate2-pc/src/gpu_renderer/map_draw.rs`: `push_environment` binds the
  lighting stream only for effects that read TEXCOORD1. The GPU test's effect
  loop includes `ProxyWorld`.
- `apps/skate2-pc/src/live_skating_client.rs`: creates the owner, hands it to
  the city and to the session (`StreamLog` replaced).

## 8. Deviations and open items

- Boot timing: the collections load at renderer init (texture uploads need
  the device), after the initial high chunks, and `AddHighCollection` can
  arrive after those chunks registered. Activation still precedes every high
  registration the port applies; a late mapping deregisters the proxy at
  once (the state the original reaches). This settling is a port rule, not
  an original path.
- Granularity: a high chunk registers whole at the generation swap (the
  direct batch); the deferred per-frame arena registration is not modeled.
- Proxy textures stay resident; drawing is the same because a registered
  proxy's textures are always registered.
- No world-shadow pass exists yet, so the scene-view-only routing has no
  consumer; unlock contexts are not modeled (as for high chunks);
  `kEnableProxyWorldStream` is always on (authored default), so the disabled
  `DIST_ProxyWorld` loose path is not implemented.
- Effect/material scope IDs are rebuilt per generation (existing deviation);
  proxies sort by draw order 6 among them.

## 9. How to check

Log lines: `PROXY world: 147 collections in load order`, `PROXY retained
collections=148 unique texture uploads=145 spatial geometry=147`, `PROXY world
activated: N registered proxies beside M high collections`, and `ProxyWorld`
in `original environment shading draws`. In game: the distant city fills in
beyond the streamed chunks; walking or skating should swap a proxy for its
high chunk as it streams in, with no gap and no doubled geometry.
