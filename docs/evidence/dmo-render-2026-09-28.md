# DMO presentation audit

Status: opacity, ghost passes and DMO culling bounds connected after live IDA
and original-asset checks. SOLO. No compilation, GPU shader validation, Rust
test execution or gameplay test. Full render/runtime parity is unproven.
Earlier default-shader module comments record the prior session's shader work;
this continuation does not independently validate every default-shader opcode.

## Existing implementation to reuse

apps/skate2-pc/src/dmo_render.rs retains the main DMO presentation bank,
per-model uploads and per-entity GPU instances. It reads material bindings and
textures from the bank and removes GPU entities absent from the core snapshot.
gpu_renderer/dynamic_object.rs implements dynamicobject and
alphatestdynamicobject default shading and shadow casters. The existing
SelectedDynamicObjectVisual supplies entity ID, bank asset, RModel entry and
the solved Part transform. Do not create another object/model/texture manager.

Current explicit omissions: world-to-object shadow map is a dummy texel with
zero matrix rows; replay transform interpolation is absent. Shader half
precision and host anisotropy also differ as documented in the module.
Material LOD's original update remains unconnected, but the shipped DMO shader
constant-name census below finds no consumer for it. The presentation snapshot
samples live inputs after a completed host simulation tick; it is not the full
native previous/current replay-data presentation schedule.

## Source flow confirmed this continuation

- CreatePresDynamicObject82811100 resolves RecipeID in the model map, allocates
  the DMO presentation entity and inserts its entity ID. Shared
  CreateLivingWorldEntity82813298 handles entity type5 and registers it in
  render views. Do not substitute another model for failed resolution.
- cPresDynamicObject ctor8280bb78 calls the simple presentation constructor,
  binds constants, and adjusts its local bounds. The raw VMX audit and host
  correction are recorded below.
- cLivingWorldSimplePresEntity.Update82812168 calls UpdatePartTransforms for
  its current LOD parts, then cLivingWorldPresEntity.Update82811bd8.
- Update82811bd8 reads camera matrix1, entity position and bounds centre;
  updates material LOD, spawn opacity, distance opacity, ghost flag and the
  default/ghost technique. These are presentation operations, not DMO forces.
- cLivingWorldPresEntity ctor82807a20 initializes mInstanceParams.x=1 and
  mSpawnTransparency=1 (raw stfs82807ab0, offset0x210). Do NOT automatically
  invent a30-frame fade for every newly seen DMO. The traced DMO constructor,
  CreatePresDynamicObject, shared CreateLivingWorldEntity and manager creation
  path do not reset this value. Generic fade support alone is not evidence
  that a DMO starts at zero opacity.
- UpdateLivingWorldPresentationEntities828133e0 matches previous/current
  records by entity ID; creates missing current entities, destroys absent
  ones and interpolates retained entities. New entities receive previous=
  current and percent1. cPresDynamicObject.UpdateFromReplayData8280a040
  interpolates Part transforms via the native matrix SLerp. Reuse existing
  quaternion/matrix helpers after tracing their exact conventions.
- LivingWorldPresentation.DoSimRestore82813a90 publishes the per-category
  SimCenterVelocities from current replay data; the DMO slot is+0x7f40.
  Update82811bd8 uses this velocity's magnitude times3.6 when evaluating
  opacity distances. It does NOT use the individual object's velocity.
  DoSimSubmit8280e8b0 supplies this category velocity immediately before the
  DMO count (virtual call8280f4dc, SubmitData8280f4f0). Vtable+78 is
  Manager.GetSimulationCenterVelocity826f4e48, using Actor.GetVelocity825ed338.
  The existing local_skater_census_centre port is reused. Its stationary branch
  was corrected to raw BLE (dot(v,v)-1e-4 <=0), preserving unordered fallthrough.

## Opacity data and connected behaviour

PDB tLWOpacityDistance is12 bytes: FullyOpaqueDistance+0,
FullyTransparentDistance+4, Speed+8. EvaluateOpacityDistance828060f8
interpolates these using the two authored speed endpoints. Raw r5=low, r6=high,
r7=output; the floating argument shifts the decompiler's apparent ABI.
The implementation follows raw registers and branches, not those parameter names.

livingworld_models class hash4e740a6670f241a7 confirmed in skater.vlt. Four
attributes read by Update (raw64-bit lis/ori/insrdi, index0, default area12):

- 9fcfdbea56ba4733 and73b6874c7b46c7c6, evaluated first into far distances.
- 4e5e99c0ab48f75f and8098b1f07fd28112, evaluated second into near distances.

All four concrete DMO values were checked in the January17 skater.vlt/bin.
livingworld_entities class60d703c77cab1631 / dynamicobjects20f04d295272ee46
cached layout+8 contains ModelKey20f04d295272ee46. SpawnDynamicObject82726c08
(raw82726d74) publishes this into LWCommonSpawnInfo+10; Spawn82d31ba0 carries
it into LWCommonCreationInfo+10; DynamicObject ctor82d3de80 retains it in
LWCommonCoreData+100. DoSimSubmit8280f6e4 publishes that key to presentation.
The existing DynamicObject owner now retains it; no second model owner added.
livingworld_models was absent from the selected Vault class list and is now
included. Do not substitute the physics-characteristics key.

For that ModelKey, far-max and far-min are both [45,55,0] (BIN590448/590460);
near-max and near-min are [0,0,0] (BIN590424/590436). These are evidence values,
not hardcoded runtime tuning. The reader uses AttributeRuntimeInstance and
its inheritance/default behaviour; missing data uses the native zero default.

living_world_presentation_opacity.rs ports EvaluateOpacityDistance828060f8
and the opacity portion of Update82811bd8:

- If low.speed>=high.speed, copy low. Otherwise saturate the source positive
  speed delta divided by the endpoint range and use FMAs for all three fields.
- Update spawn opacity by saturate((1/fadeFrames)+spawnOpacity), even when
  distance opacity is disabled. Initial spawnOpacity is1, as traced above.
- Enabled distance uses category velocity length times float bits40666666 and
  camera1-to-bounds-centre distance. Reuses try_length's two-refinement numeric
  helper (host estimate limitations still apply).
- Beyond the far opaque endpoint, use1-saturate(positive distance/width).
  A nonpositive width leaves opacity1: do not invent an immediate zero.
- Below the near opaque endpoint, use the native near ramp; its nonpositive
  width also leaves opacity1. Native BLE/BGE unordered fallthrough and FSEL
  operand order are retained, including the final minimum with spawn opacity.
- Disabled sets opacity1/ghostfalse; enabled ghost is opacity<1.

The core visual snapshot supplies parameters/settings/category velocity; the
existing DmoRender per-entity owner retains spawn opacity and publishes actual
i_params.x/ghost selection. Snapshot reads are at the host completed-tick
boundary, not a claim of original replay scheduling. No alpha-zero admission
skip was added: the native IsEnabled828060e0 does not perform one.

Material LOD uses rendering collection51416546b54f421d, scalar key
f412fc7b191f16c8 (raw82811c60..78 and82811cc8..e4). Its subsequent scalar
double/single rounding and all opacity FSELs still need detailed raw review.

Cvars added to the existing registry at894..895 (896 total):

- kRenderLivingWorldOpacityEnabled, Render_Settings, title
  `Enable Livingworld Distance Based Opacity`, defaulttrue,
  reference83054f04 / initializer82f6eb00 / table82fd264c.
- kLivingWorldTransparancyFadeinFrames, same section, title
  `Living World Transparancy Fadein Frames`, default30, GUI1..120,
  reference83054f28 / initializer82f6eb50 / table82fd2650.
  Raw82f6eb8c sets callback r10=0; HexRays' extra callback arguments are wrong.

## Ghost rendering

Original render.big data/render/shaders/final/dynamicobject.xml:

- ghost pass0: same defaultVS/defaultPS, FRONT culling, colour writes0,
  default enabled Z test/write with LESSEQUAL, no blending.
- ghost pass1: same programs, FRONT, RGB writes, SRCALPHA/INVSRCALPHA ADD,
  alpha test GREATEREQUAL1, stencil ref/mask/write-mask255 and front/back
  PASS=REPLACE (default ALWAYS/KEEP/KEEP); Z write stays enabled.
- alphatestdynamicobject has no ghost technique. SetCurrentEffect82e1c210
  falls back to default, retaining diffuse alpha and threshold30/255.

The DMO pipeline implements both passes. The host ghostPS entry implements
the native fixed-function alpha test; defaultPS already outputs i_params.x.
Integration audit caught a required attachment change: the old main scene
used Depth32Float with stencil_ops=None. XBox2GraphicsMan.Init8282c3d8 sets
PIXELFORMAT_D24S8; RenderModeScene82857c50 clears depth/stencil. The scene now
uses Depth24PlusStencil8 and writable cleared stencil, with the same format
shared by every scene pipeline and the existing MSAA capability check. Depth
bias's 24-bit unit conversion is unchanged. No camera-relative precision fix
was removed. Actual native rasterization remains backend-dependent.
The pinned local wgpu-core0.19.4 command/render.rs1535..1541 rejects pipelines
that write stencil in a read-only pass; a depth-only texture cannot supply the
new pass's stencil aspect. This was found statically, not by a GPU test.
RenderMeshes82850670 draws sorted single-pass meshes first, then multi-pass
meshes pass-major. DMO ghost meshes now share that ordering with character
meshes rather than being drawn separately. Existing DMO shadow casters remain.

## Culling bounds correction

PDB PresentationEntity: m_BB+c0, m_localBB+e0, m_localBC+100, m_globalBC+110,
m_MatLtoW+160. The original single-part CalculateBoundsInfo827f2708 sets
m_localBC to the RModel midpoint, W1. SetLocalToWorldMatrix827efd08 transforms
that centre in X product, Y FMA, Z FMA, W FMA order. bounds_center() now uses
that operation order for opacity.

DMO ctor8280bbec..bdb4 separately computes extent=max(0,max.x-min.x,
max.y-min.y,max.z-min.z) using sequential strict ordered comparisons. It
overwrites localBB.xyz with [-extent,+extent], without shifting to the mesh
centre or halving extent. It preserves W and does not modify localBC.
Swizzle table83091fc0 is runtime-initialized: initializer82f7efe8 confirms
offset0 replaces X, offset40 replaces Y, offset80 replaces Z. Static zeros
in the database are not its runtime permutation masks.

SceneRenderView.Cull828501c0..1e0 adds m_MatLtoW.wAxis to both localBB endpoints,
without applying rotation. ShadowView.Cull82850c30 consumes that m_BB.
DynamicObjectDraw.world_bounds now follows this path; the old tight eight-corner
mesh transform was incorrect. Opacity's mesh-derived centre remains distinct.

## Validation / outstanding work

- Static constructor/caller inventory: one DynamicObject constructor, one
  SelectedDynamicObjectVisual producer, one DynamicObjectDraw producer; live
  view and the two renderer draw sites carry the new inputs/pass index.
- Scoped rustfmt parser checks passed for13 changed Rust files. This is syntax
  evidence only, not type checking or shader validation.
- Registry:896 definitions,896 registration indices, a complete permutation.
  Historical title-order unit-test oracle remains stale (544 expected labels);
  it must not be cited as a passing test. No Rust tests ran.
- Two unexecuted opacity tests cover authored45..55 distance behaviour,
  equal-speed endpoint selection, speed interpolation and near opacity.
- Still needed: compilation and live load/collision/grab/push/pull/release
  evidence, plus remaining world-shadow/replay interpolation work.
  Full DMO/render parity is not established; the active goal is not complete.

## Continuation: shipped bank coverage and remaining shader inputs

Read-only probes used the CURRENT January17 installation, not the older
November28 path embedded in the historical scratch dmo_materials.py:
`<game folder>`.
No Cargo command or gameplay test was run.

`data/content/worldmisc.big`, entry
`data/content/world/dmo/DMO/cPres_Global.xsf`, is3,092,992 decoded bytes and
contains125 arenas. Following each of the93 instance records' RModel and
CModel references through the arena dictionary gives:

- All93 have both referenced models and CModel.numMeshes=0xffffffff.
- All93 CModel+8 references select serialized Assembly objects(type0x40006).
- All93 Assembly+4 references select AssemblyDefinition objects(type0x40007).
- Both the serialized Assembly count(+28) and definition count(+4) are1
  for every model. Their part volumes all have arena type0x80001.

These checks cover the structural admission gates in
living_world_dmo_models.try_create_model (CreateFromMesh82d337b0), including
the one-part requirement. They do NOT execute the Rust loader, validate every
volume child, or prove that every model spawns. None of the93 requires the
UnportedFallback/UnportedProceduralParts/UnportedSphereDefinition branches.
Do not add replacement model implementations merely because those error
variants remain declared.

The eight shipped `final/*dynamicobject_*.vpo/.fpo` programs in render.big
were independently read and their embedded constant names enumerated:
defaultVS/defaultPS/simpleVS/shadowPS for both dynamicobject and
alphatestdynamicobject. None lists `i_material_lod`. Both defaultPS programs
list WorldShadow_MatRow and shadowTexture_World. The opaque defaultPS lists
i_params; the alpha-tested variant does not. Both defaultVS programs list
i_partArray and i_detailNormalUVScale; both simpleVS list i_partArray.
Therefore the unconnected base-entity material-LOD update is not evidence
of a missing input to these shipped DMO programs. World-shadow input IS a
confirmed missing producer; it currently receives the documented dummy map.
This is an asset binding census, not GPU execution or an ALU parity test.

The transform path was also rechecked against live IDA:
DoSimSubmit8280e8b0 publishes the entity transform AND separate Part transforms.
DynamicObject.GetPartTransform82d2a9a0 is already exposed by the existing world
service. UpdatePostTransforms copies Part0 into target then entity transform;
the selected visual is taken after that completed tick. With the93 single-part
models this explains the existing direct Part0 path; it does not implement
native replay interpolation. UpdateFromReplayData8280a040 converts the retained
previous/current quaternions to matrices and calls matrix SLerp82808ff8.
UpdatePartTransforms827efa10 copies each Part matrix into its mesh bone slots.
Do not add independent DMO wall-clock smoothing: the live host currently
updates the map, camera, skater palette and DMOs together per completed tick.

## World-shadow implementation anchors (not yet implemented)

Additional live IDA checks narrow the required producer:

- PostFX.RenderShadows_CSM82855888 performs this pass AFTER its two dynamic
  cascades. It reuses the512x512 depth/stencil target, clearing depth/stencil.
- Raw82856d54..74 selects scale key0x9c8706768cfcbb4a from m_CSMAttribs
  (rendering collection0xb9a92367e7ac68b9). Raw82856d94..9c multiplies by
  ScaleLevel[3], which is16. This is NOT the dynamic cascade scale attribute.
- Raw82856d98..e18 selects offset key0x0fb8d6f785615ebe, a12-byte vector.
  The repeated reads select its components; HexRays drops those loads from
  its apparent expressions. Missing values use original DefaultDataArea.
- Enable gate: kEnableWorldOntoObjectShadow, NIS override when applicable,
  and render_locations cached-layout byte+2 (true without a collection).
- WorldShadowView.Cull82850ec0 first admits registered entities through
  DetermineVisibilityFromGameState, then their m_BB through the existing
  vectorized frustum culler. Surviving entities run ShadowCullMeshes_VMX with
  argument8; Sort82851368 compacts their shadow-visible islands in mesh order.
- Cull's final height/depth bounds and RenderShadows' subsequent depth-row
  replacement must be traced in raw VMX before implementing the matrix.
  A generic light-space AABB fit would not establish equivalence.
- RenderShadowMesh82824440 sets the supplied simple vertex-program state,
  binds the original mesh index/vertex streams and draws only
  m_uiShadowVisibleIslandIndices. It does not invoke the material's full
  default pixel shader. WorldShadowView.Render82829ef8 clears the pixel
  program; the caller installs Z-write/stencil255 with alpha test disabled.
- The final depth/stencil resolve is processed by depthsampler_PS82130890
  into m_pWorldShadowMap and published with the three WorldShadow_MatRow
  vectors. Reuse the existing GPU mesh buffers and ShadowMaps owner; do not
  create a second world geometry store or substitute the dynamic CSM atlas.

No world-shadow code was added during this audit. Matrix/cull admission,
depth/coverage filtering and shared bindings still need a complete connected
implementation and GPU validation. Compilation/playtest of the already
connected DMO load/collision/grab/move/release flow remains outstanding.

## Authored island bounds and CPU shadow culling continuation

The later continuation added prerequisites, not a connected GPU world-shadow
pass. Next is the first functional DMO build.
World-shadow work must not delay that compile/playtest checkpoint.

- RenderModelData now retains all authored island AABBs (32 bytes each),
  transferred through own_presentation_models into PresentationModel.
  PDB tRModelData: numIslands+0x36, pointer+0x38. Fixup829c70f4..7108 adds
  the containing RModel base, not the field address. The existing parser and
  model owner are reused. W lanes are preserved.
- WorldPresEntityOptimesh827d41dc assigns each mesh its cumulative island
  subrange; 827d423c binds the model table and 827d4248 sets mAreaList=null.
  Read-only Python corpus probes found no invalid bounds tables in all93 DMO
  models, and in the largest8 worldbam XSFs (289 models/11,961 islands) and
  largest8 worldproxy XSFs (7 models/157 islands). These did not run Rust.
- presentation_shadow_culling.rs specializes DoFrustumCulling828589c0 for
  those no-area/no-antifrustum world-shadow calls. ComputeFrustum8282bd28
  extracts w+x,w-x,w-y,w+y,w+z,w-z; NormalizePlane8282bc60 uses its ordered
  Y-square/X-FMA/Z-FMA and two rsqrt refinements, reusing existing numerics.
- The culler uses centre=(min+max)*.5, half=max-centre, radius=min.W+extra.
  Distance uses Z-square/Y-FMA/X-FMA, bare rsqrt then reciprocal estimates;
  the reciprocal of distance-radius has exactly ONE Newton step. Projected
  size uses mViewH bits44ba3d71, full view height when gap<radius, and an
  inclusive minimum-pixel comparison. Host estimate/VMX exceptional selection
  boundaries remain explicit; bit-identical Xenon arithmetic is not proven.
- WorldShadowView.Cull82851244..ec updates only Y/Z from admitted ENTITY
  boxes, independent of subsequent island admission. 82851304..48 expands
  the resulting extents by bits3e19999a (15%). No generic light-space fit.
- Raw82857244..64 passes the preliminary shadow matrix's snapped translation
  as ComputeFrustum's camPosition. It is NOT the gameplay camera position.
  World shadow scale key9c8706768cfcbb4a=1.5, multiplied by ScaleLevel[3]=16;
  offset key0fb8d6f785615ebe=[0,0,0] in current January17 Vault.
- The final depth replacement at82857318..3d8 uses the padded Y range and
  max.Y, then rescales XY and snaps translation. Degenerate range selects
  depth=z (row0.z=0,row1.z=0,row2.z=1,translation.z=0). GPU integration still
  needs the full matrix publication, native caster admission, depthsampler
  resolve and shared bindings. Reuse ShadowMaps and existing geometry buffers.
- WorldPresentation ctor827e9d10 registers eSceneView and eWorldShadowView
  (calls827e9da4/dbc). Proxy draw-order exclusion remains as previously traced.
- tools/vault_dump.py incorrectly checked serialized entry_flags+15 for
  inline data. Corrected to node_flags+14, already used by production Rust.
  Collection ctor raw8297be90/8297bef0 confirms this; runtime Node stores
  flags at+15 after construction, a distinct layout. Read-only reprobe now
  correctly returns world scale1.5. No production Vault-reader change needed.

New Rust culler and parser tests are present but unexecuted. Scoped rustfmt
parsing passed; no Cargo, shader validation or gameplay was run. Latest
playtest preparation keeps once-per-model render admission errors visible in
--quiet; this is host diagnostics only and does not alter original gameplay.
